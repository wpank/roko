"""The bash-only direct loop, harness `mini-loop`: the runner of the cheap_direct and fd_api arms (S08 §4.9).

One model, one bash tool, no Roko prompt. Each reply must hold a short THOUGHT and exactly one ```bash block. The
loop runs the command in the task's workdir, in a new shell with the agent environment and a time limit, and sends
back its exit code and output; long output keeps its head and tail. The agent ends its session by printing
`VB_SUBMIT` as the first line of a command's output (the prompt tells it to run `echo VB_SUBMIT`). A reply without
exactly one bash block gets a format reminder, which costs a turn. The shape follows mini-swe-agent (S08 decision 3)
with the standard library only; running pinned mini-swe-agent itself is out of this module's scope.

`caps.Governor` is asked before every model call and every command. An attempt that reaches its turn or input-token
cap ends, and the next attempt starts from a fresh context whose task message says the tree holds the earlier
attempts' changes. The task stops `aborted_cap` at a task cap or a runaway signal, and `timeout` at its wall-clock
limit. A provider error is retried twice, each retry a model call under the same caps; the third failure, or any
error that retrying cannot fix, ends the task `infra_error`.

Every attempt that made a model call gets one ledger row when it ends, even when the loop fails. Its usage and cost
are the provider's reported usage priced by the reported model's row (`ledger.price`); an attempt with a call whose
usage never came back has an unknown cost.

API:
    run_task(ctx: harness.TaskContext) -> harness.TaskOutcome
    parse_command(reply: str) -> str | None
    run_command(command, *, cwd, env, timeout_s, observation_chars) -> CommandResult
    PROMPT_VERSION, PROMPT_SHA256, SUBMIT
"""

from __future__ import annotations

import hashlib
import os
import re
import shutil
import signal
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

import caps
import harness
import layout  # noqa: F401 (puts families/ on sys.path for common)
import ledger
import provider
from common import repo

PROMPT_VERSION = "mini-loop-1"
SUBMIT = "VB_SUBMIT"
RETRIES = 2
MESSAGE_OVERHEAD_TOKENS = 16  # chat-template tokens a message may add beyond its bytes
PREAMBLE_TOKENS = 256  # what a provider may inject before the first message (e.g. a harmony system header)
BASH_BLOCK = re.compile(r"```bash[ \t]*\r?\n(.*?)\r?\n[ \t]*```", re.DOTALL)

SYSTEM_PROMPT = """\
You are a software engineer working on a task in a git repository, through a bash shell.

Every reply must contain a short THOUGHT paragraph, then exactly one bash code block:

```bash
<one command; join steps with && if you need several>
```

Rules:
- Each command runs in a new shell in the repository root, so `cd` and variables do not persist.
- Commands cannot be interactive: never open an editor or a pager. Edit files with tools such as
  `cat <<'EOF' > file`, `sed -i`, or a python3 script.
- Each command has a time limit of {command_timeout_s:g} seconds; long output is cut in the middle.
- When the task is done, reply with exactly this command, alone in its block:

```bash
echo {submit}
```

You cannot continue after submitting.
"""

TASK_MESSAGE = """\
<task>
{spec}
</task>

The repository is your current working directory. Complete the task, then submit.
"""

RETRY_NOTE = """
Note: this is attempt {number}. An earlier attempt stopped at its turn or token limit, and the repository still
holds its changes; `git status` and `git diff` show them.
"""

FORMAT_ERROR = """\
Format error: your reply must contain exactly one ```bash code block (it had {count}). Reply with a short THOUGHT
and one bash block. To finish, run `echo {submit}` alone.
"""

PROMPT_SHA256 = hashlib.sha256("\0".join([SYSTEM_PROMPT, TASK_MESSAGE, RETRY_NOTE, FORMAT_ERROR]).encode()).hexdigest()


@dataclass(frozen=True)
class CommandResult:
    returncode: int | None
    output: str
    timed_out: bool
    submitted: bool
    observation: str


class _Conversation:
    """One attempt's messages, and an upper bound on the input tokens of the next call."""

    def __init__(self, messages: list[dict]) -> None:
        self.messages: list[dict] = []
        self.known_tokens: int | None = None  # the last reported prompt size
        self.pending = PREAMBLE_TOKENS  # tokens added since then, bounded by bytes
        for message in messages:
            self.add(message)

    def add(self, message: dict) -> None:
        self.messages.append(message)
        self.pending += len(message["content"].encode("utf-8")) + MESSAGE_OVERHEAD_TOKENS

    def input_bound(self) -> int:
        return (self.known_tokens or 0) + self.pending

    def reported(self, prompt_tokens: int | None) -> None:
        if prompt_tokens is not None:
            self.known_tokens, self.pending = prompt_tokens, 0


def parse_command(reply: str) -> str | None:
    """The command in a reply's one bash block; None unless there is exactly one."""
    blocks = BASH_BLOCK.findall(reply)
    return blocks[0].strip() if len(blocks) == 1 and blocks[0].strip() else None


def run_command(command: str, *, cwd: Path, env: dict[str, str], timeout_s: float,
                observation_chars: int) -> CommandResult:
    """Run `command` with bash in its own session; kill the whole session when it ends or times out."""
    bash = shutil.which("bash", path=env.get("PATH")) or "/bin/bash"
    try:
        process = subprocess.Popen([bash, "-c", command], cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
    except OSError as err:
        text = f"could not start bash: {err}"
        return CommandResult(None, text, False, False, _observation(None, text, False, timeout_s, observation_chars))
    timed_out = False
    try:
        raw, _ = process.communicate(timeout=max(timeout_s, 0.1))
    except subprocess.TimeoutExpired:
        timed_out = True
        _kill_session(process.pid)
        try:
            raw, _ = process.communicate(timeout=5)
        except subprocess.TimeoutExpired:  # a process outside the session still holds the pipe
            process.kill()
            raw = b""
    finally:
        _kill_session(process.pid)  # background jobs the command left behind
    text = raw.decode("utf-8", "replace")
    lines = text.lstrip().splitlines()
    submitted = not timed_out and bool(lines) and lines[0].strip() == SUBMIT
    code = None if timed_out else process.returncode
    return CommandResult(code, text, timed_out, submitted,
                         _observation(code, text, timed_out, timeout_s, observation_chars))


def run_task(ctx: harness.TaskContext) -> harness.TaskOutcome:
    started = harness.utc_now()
    governor = caps.Governor(ctx.caps, price_row=ctx.price_row)
    system = SYSTEM_PROMPT.format(command_timeout_s=ctx.caps.command_timeout_s, submit=SUBMIT)
    attempts: list[harness.Attempt] = []
    transcript: list[dict] = []
    status = reason = None
    while status is None:
        governor.start_attempt()
        number = governor.attempts
        attempt = harness.Attempt(number=number, attempt_key=f"{ctx.chain_key}:{number}", model_requested=ctx.model,
                                  provider=ctx.endpoint.provider, reserved_usd=governor.reserve_attempt_usd())
        task = TASK_MESSAGE.format(spec=ctx.spec_text.strip())
        task += RETRY_NOTE.format(number=number) if number > 1 else ""
        conversation = _Conversation([{"role": "system", "content": system}, {"role": "user", "content": task}])
        transcript += [{"attempt": number, **message} for message in conversation.messages]
        try:
            ctx.ledger.reserve(attempt.attempt_key, attempt.reserved_usd if ctx.billed else 0.0)
            status, reason = _run_attempt(ctx, governor, attempt, conversation, transcript)
        except ledger.BudgetError as err:  # the budget line has no room for this attempt; it made no call
            status, reason, attempt.ended_by = "aborted_cap", "budget", "budget"
            transcript.append({"attempt": number, "event": "stop", "kind": status, "reason": str(err)})
        except Exception as err:  # a harness bug must still leave a record and a ledger row
            status, reason = "infra_error", f"{type(err).__name__}: {err}"
            attempt.ended_by = attempt.ended_by or "infra_error"
        finally:
            if attempt.calls:
                _settle(ctx, attempt)
                attempts.append(attempt)
            else:
                ctx.ledger.release(attempt.attempt_key)
        if status is None and not attempt.calls:  # an attempt that could not make a single call
            status, reason = "aborted_cap", attempt.ended_by or "no_progress"
    return harness.TaskOutcome(status=status, reason=reason, attempts=attempts, transcript=transcript,
                               started_at=started, finished_at=harness.utc_now())


def _run_attempt(ctx: harness.TaskContext, governor: caps.Governor, attempt: harness.Attempt,
                 conversation: _Conversation, transcript: list[dict]) -> tuple[str | None, str | None]:
    """Play one attempt. (None, None) means it hit an attempt cap and another attempt may follow."""
    failures = 0
    while True:
        bound = conversation.input_bound()
        stop = governor.before_call(bound)
        if stop:
            attempt.ended_by = stop.reason
            transcript.append({"attempt": attempt.number, "event": "stop", "kind": stop.kind, "reason": stop.reason})
            return (None, None) if stop.kind == "end_attempt" else (stop.kind, stop.reason)
        attempt.calls += 1  # counted before the request, so a call that raises is still in the ledger row
        try:
            completion = ctx.provider.complete(conversation.messages, model=ctx.model,
                                               max_tokens=ctx.caps.max_output_tokens,
                                               timeout_s=min(ctx.endpoint.timeout_s, governor.remaining_s()))
        except Exception as raised:  # every failed call is accounted for; a non-ProviderError is a provider bug
            err = raised if isinstance(raised, provider.ProviderError) else provider.ProviderError(
                f"{type(raised).__name__}: {raised}", retryable=False, billed_unknown=True)
            attempt.usage_unknown |= err.billed_unknown
            governor.after_call(input_tokens=None if err.billed_unknown else 0, input_bound=bound,
                                cost_usd=None if err.billed_unknown else 0.0, completed=False)
            transcript.append({"attempt": attempt.number, "event": "provider_error", "error": str(err)[:500]})
            failures += 1
            if not err.retryable or failures > RETRIES:
                attempt.ended_by = "provider_error"
                return "infra_error", f"provider error: {str(err)[:200]}"
            time.sleep(max(min(err.retry_after or 2.0 ** failures, governor.remaining_s()), 0.0))
            continue
        failures = 0
        usage = completion.usage
        cost = None
        if usage is None:
            attempt.usage_unknown = True
        else:
            call_usage = ledger.vb_usage(usage.prompt_tokens, usage.completion_tokens, usage.cached_tokens,
                                         usage.reasoning_tokens)
            attempt.usage = ledger.add_usage(attempt.usage, call_usage)
            cost = ledger.price(call_usage, ctx.price_row).api_equiv_usd
        governor.after_call(input_tokens=None if usage is None else usage.prompt_tokens, input_bound=bound,
                            cost_usd=cost, completed=True)
        attempt.turns += 1
        attempt.model_reported = attempt.model_reported or completion.model_reported
        conversation.reported(None if usage is None else usage.prompt_tokens)
        reply = {"role": "assistant", "content": completion.content}
        conversation.add(reply)
        transcript.append({"attempt": attempt.number, **reply})
        command = parse_command(completion.content)
        if command is None:
            note = FORMAT_ERROR.format(count=len(BASH_BLOCK.findall(completion.content)), submit=SUBMIT)
            conversation.add({"role": "user", "content": note})
            transcript.append({"attempt": attempt.number, "role": "user", "content": note})
            continue
        stop = governor.before_tool(command)
        if stop:
            attempt.ended_by = stop.reason
            transcript.append({"attempt": attempt.number, "event": "stop", "kind": stop.kind, "reason": stop.reason,
                               "command": command})
            return stop.kind, stop.reason
        result = run_command(command, cwd=ctx.workdir, env=ctx.agent_env,
                             timeout_s=min(ctx.caps.command_timeout_s, max(governor.remaining_s(), 0.0)),
                             observation_chars=ctx.caps.observation_chars)
        if result.submitted:
            attempt.ended_by = "submitted"
            transcript.append({"attempt": attempt.number, "event": "submitted", "command": command})
            return "completed", "submitted"
        conversation.add({"role": "user", "content": result.observation})
        transcript.append({"attempt": attempt.number, "role": "user", "content": result.observation,
                           "command": command, "returncode": result.returncode, "timed_out": result.timed_out})


def _settle(ctx: harness.TaskContext, attempt: harness.Attempt) -> None:
    """Price the attempt by its reported model, note its tree, and append its ledger row."""
    try:
        attempt.tree = repo.tree_hash(ctx.workdir)
    except (OSError, repo.RepoError):
        attempt.tree = None
    attempt.cost = ledger.price(attempt.reported_usage(), ctx.snapshot.row(attempt.model_reported))
    ctx.ledger.append(attempt_key=attempt.attempt_key, provider=attempt.provider, model_reported=attempt.model_reported,
                      usage=attempt.reported_usage(), cost=attempt.cost, billed=ctx.billed,
                      reserved_usd=attempt.reserved_usd)


def _observation(code: int | None, text: str, timed_out: bool, timeout_s: float, limit: int) -> str:
    if len(text) > limit:
        half = limit // 2
        text = (f"{text[:half]}\n\n[... {len(text) - 2 * half} characters omitted; use a more selective command ...]"
                f"\n\n{text[-half:]}")
    head = (f"<returncode>timeout</returncode>\n<note>The command ran past its {timeout_s:g} s limit and was "
            f"killed.</note>") if timed_out else f"<returncode>{code}</returncode>"
    return f"{head}\n<output>\n{text}</output>"


def _kill_session(pid: int) -> None:
    try:
        os.killpg(pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass

