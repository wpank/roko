"""The Codex CLI runner, harness `codex-cli`: the runner of the fd_codex arm (S08.T12; S09 §4.2, LOG1's "could" arm).

Codex runs non-interactively (`codex exec --json`, the prompt on stdin) in the task's workdir, with the task as its
only prompt and no Roko or system prompt, on the author's ChatGPT subscription. It mirrors the Claude Code runner and
reuses, never reimplements, its session machinery (`run_cli.run_session`, `Invocation`, `network_rule`, `CliAttempt`
and the task message), so both direct arms get the same prompt, kill rules and egress proxy. One session is the
task's one attempt, and it ends when Codex finishes its turn (status `completed`).

**Isolation.** Started from the author's machine, `codex exec` would load the author's `~/.codex`: its config.toml
(model, profiles, MCP servers, approval policy), AGENTS.md, execpolicy rules, skills and history, and the arm would
measure that setup rather than Codex (W10). Every session gets:
- a fresh `CODEX_HOME=$HOME/.codex` under the task's own HOME (`agent_env`), holding only the login: a private copy
  of the operator's `auth.json` (from `$CODEX_HOME`, else `~/.codex`; the scrubbed `vb run` keeps only the latter),
  with any API key in it dropped, so Codex can only use the subscription. A same-uid agent can read that copy, as it
  can Claude Code's `credentials_file` copy. Codex refreshes an expired login inside the copy; a refresh that
  rotates the refresh token can leave the operator's own login stale, which `codex login` then renews;
- `--ignore-user-config` and `--ignore-rules`: no config.toml and no execpolicy rules load, even if one appears;
- `-c web_search="disabled"` (gap-f253cf): no web search tool. The repository is public, so a search could return
  a truth suite, and Codex's search runs on OpenAI's side, where the egress proxy cannot see it. Codex names no
  tools up front, so a session cannot be stopped before its first turn as `run_cli` stops one: a `web_search` item
  in the stream makes the run `infra_error` (reason `web_tools`), which the report excludes;
- `--dangerously-bypass-approvals-and-sandbox`: Codex's own Seatbelt sandbox cannot start inside the driver's (macOS
  refuses a nested `sandbox_apply`), so the driver's is its only one, as for Claude Code's
  `--dangerously-skip-permissions`. The session's whole process tree runs under `common.sandbox`, which denies
  `ctx.deny` and, since Codex has no deny rules of its own (S08 §4.9, "where the CLI supports them"), what
  `run_cli.settings` denies Claude Code's file tools: the repository, `~/.roko`, the run's results and the default
  secret directory, plus the operator's Codex home (`denied`);
- `--cd <workdir>`, `--skip-git-repo-check`, `--color never` and `-c model_reasoning_effort=<[cli] effort>`, set
  explicitly as the Claude Code arm sets its effort. The runner refuses a workdir with an AGENTS.md above it;
- no `OPENAI_API_KEY` or `CODEX_API_KEY`: `agent_env` passes no key, so Codex never bills the API.
An npm-installed `codex` is a node script (`#!/usr/bin/env node`), and the session's PATH has only the system's
tools, so the runner links that one interpreter into the session's `.vb-bin` (`interpreter_link`). A run with
`flaky_verify` is refused: Codex has no shell prefix, like Claude Code's `CLAUDE_CODE_SHELL_PREFIX`, through which
the visible-verify wrapper could reach its shell commands.

**Network** (3305). As in `run_cli`: the session's own egress proxy admits `[cli] egress_allow` and logs every request
to `<run_dir>/egress.jsonl`, and the sandbox admits only the proxy's port. On the subscription Codex calls
`chatgpt.com/backend-api` and refreshes its login at `auth.openai.com`, the default allowlist (`DEFAULT_ALLOW`); a
host it needs besides those is refused until the arm file lists it, which a live probe should show first, as
gap-154f93 did for Claude Code. An offline run (a loopback `--provider-url`) admits no host at all: a real codex
cannot be pointed at a stub, so offline runs are for the tests' fake.

**Caps.** `codex exec` has no turn or budget flag, so the runner applies the two it can: the wall clock
(`wallclock_s`) and the dollar cap (`usd_per_task`) on the live meter (`Meter`). Codex reports usage only when its
turn completes, so the dollar cap acts at the end of a turn rather than inside one; a turn past it ends
`aborted_cap` (reason `usd`), as a capped Claude Code session does. Before codex starts, the attempt is reserved on
the run's budget line (`Ledger.reserve`, $0 on the subscription), and its ledger row releases the reservation.

**Cost** (S09 §4.1): U′ alone, since Codex reports no dollar figure (R, `vendor_usd`, stays null). Each
`turn.completed` event's `usage` is priced at the snapshot row of the model that served the turn, source
`cli_usage` (`price_turns`). OpenAI's `input_tokens` include the cached ones and its `output_tokens` the reasoning
(`reasoning_output_tokens`), so the uncached input is `input_tokens - cached_input_tokens` and reasoning adds
nothing where the row's `reasoning_in_output` holds (`turn_usage`, as Roko's adapter `codex_cli/stream.rs` prices
it). A session stopped before any turn completed reported no usage, so its cost is unknown.

**Model.** The stream names no model, so each turn's model is read from the session's own rollout file
(`$CODEX_HOME/sessions/**/rollout-*.jsonl`, its `turn_context` lines; `rollout`), which also holds the CLI version
Codex recorded (`session_meta`). `model_reported` is the first turn model that differs from the pin, else the pin;
a different one makes the run an `infra_error` (`records.final_status`). Without a rollout the turns are priced at
the pin (`codex exec --model` errs rather than switch models) and `cli.model_source` says `pinned`. The CLI version
also comes from `codex --version` before the session (`cli_version`).

Each attempt record carries `cli`: the exact argv and environment, the Codex home's digest at launch, the version,
each turn's raw usage and model, U′ and the stop. `PROMPT_SHA256` covers the task message and all that is fixed about
the invocation, so `vb run`'s `config_hash`, which holds it and the arm file, covers every flag.

API:
    run_task(ctx: harness.TaskContext) -> harness.TaskOutcome
    CodexConfig.from_table(table) -> CodexConfig                   # raises run_cli.CliError
    build_invocation(ctx, cli, proxy=None) -> run_cli.Invocation    # raises run_cli.CliError
    denied(ctx) -> tuple[Path, ...]; status_of(session) -> tuple[str, str]
    Meter(snapshot, model); turn_usage(raw) -> dict | None
    price_turns(usages, models, snapshot, pinned) -> tuple[dict | None, ledger.Cost, list[str]]
    rollout(codex_home) -> Rollout; cli_version(program, env) -> str | None
    operator_home() -> Path; home_digest(path) -> str; interpreter_link(program, env) -> Path | None
    ancestor_instructions(workdir) -> list[Path]
    PROMPT_VERSION, PROMPT_SHA256, DEFAULT_ALLOW
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, fields
from pathlib import Path

import agent_env
import caps
import egress
import harness
import layout
import ledger
import records
import run_cli
from common import repo, sandbox

PROMPT_VERSION = "codex-cli-1"
EFFORTS = ("minimal", "low", "medium", "high", "xhigh")  # Codex's model_reasoning_effort values
DEFAULT_ALLOW = ("chatgpt.com:443", "auth.openai.com:443")  # the subscription's API and its login refresh
AUTH_FILE = "auth.json"  # the login, in a Codex home
API_KEY_FIELD = "OPENAI_API_KEY"  # an API-key login inside auth.json; the copy drops it
FIXED_FLAGS = ("exec", "--json", "--color", "never", "--skip-git-repo-check", "--ignore-user-config", "--ignore-rules",
               "--dangerously-bypass-approvals-and-sandbox", "-c", 'web_search="disabled"')
WORKDIR_FLAG = "--cd"  # followed by the workdir
STDIN_PROMPT = "-"  # the prompt comes from stdin
WEB_ITEMS = ("web_search",)  # item types that mean the session searched the web
INSTRUCTION_FILES = ("AGENTS.md", "AGENTS.override.md")
ITEM_EVENTS = ("item.started", "item.updated", "item.completed")
VERSION_TIMEOUT_S = 30.0
VERSION = re.compile(r"\d+\.\d+\.\d+\S*")
SHEBANG_ENV = re.compile(r"#!\s*/usr/bin/env\s+(?:-S\s+)?([A-Za-z0-9._+-]+)")

PROMPT_SHA256 = hashlib.sha256(json.dumps([run_cli.TASK_MESSAGE, FIXED_FLAGS, WORKDIR_FLAG, STDIN_PROMPT, AUTH_FILE,
                                           API_KEY_FIELD, DEFAULT_ALLOW, agent_env.PROXY_NAMES, agent_env.NO_PROXY],
                                          sort_keys=True).encode()).hexdigest()


@dataclass(frozen=True)
class CodexConfig:
    program: str = "codex"
    effort: str = "high"
    egress_allow: tuple[str, ...] = DEFAULT_ALLOW  # the targets the session's egress proxy admits

    @classmethod
    def from_table(cls, table: Mapping) -> CodexConfig:
        unknown = sorted(set(table) - {item.name for item in fields(cls)})
        if unknown:
            raise run_cli.CliError(f"unknown [cli] key(s): {', '.join(unknown)}")
        table = dict(table)
        if "egress_allow" in table:
            try:
                table["egress_allow"] = egress.parse_allow(table["egress_allow"])
            except egress.EgressError as err:
                raise run_cli.CliError(f"[cli] egress_allow: {err}") from None
        config = cls(**table)
        if not isinstance(config.program, str) or not config.program:
            raise run_cli.CliError("[cli] program must name the codex executable")
        if config.effort not in EFFORTS:
            raise run_cli.CliError(f"[cli] effort must be one of {', '.join(EFFORTS)}, not {config.effort!r}")
        return config


@dataclass(frozen=True)
class Rollout:
    """What a session's rollout files say: each turn's model, in order, and the CLI version Codex recorded."""

    models: list[str]
    cli_version: str | None
    files: list[str]  # relative to the Codex home


class Meter:
    """Live API-equivalent spend of one session: its completed turns' usage at the pin's row, since the stream names
    no model. Codex reports usage only when a turn completes, so the meter moves then."""

    def __init__(self, snapshot: ledger.Snapshot, model: str) -> None:
        self.row = snapshot.row(model)
        self.turns: list[dict | None] = []

    def observe(self, event: Mapping) -> None:
        if event.get("type") == "turn.completed":
            self.turns.append(turn_usage(event.get("usage")))

    def spent_usd(self) -> float:
        """The priced part of the spend so far; a turn without token counts adds nothing."""
        costs = [ledger.price(usage, self.row) for usage in self.turns]
        return sum(cost.api_equiv_usd for cost in costs if cost.api_equiv_usd is not None)


def run_task(ctx: harness.TaskContext) -> harness.TaskOutcome:
    started = harness.utc_now()
    prompt = run_cli.TASK_MESSAGE.format(spec=ctx.spec_text.strip())
    transcript: list[dict] = [{"attempt": 1, "role": "user", "content": prompt}]
    try:
        cli = CodexConfig.from_table(ctx.arm.get("cli", {}))
        allow = () if ctx.endpoint.offline else cli.egress_allow  # offline: no host at all (module docstring)
        proxy = egress.EgressProxy(allow, log_path=ctx.ledger.path.parent / run_cli.EGRESS_LOG).start()
    except (run_cli.CliError, OSError) as err:
        return harness.TaskOutcome("infra_error", f"codex setup: {err}", [], transcript, started, harness.utc_now())
    try:  # the session's own egress proxy, for this task alone
        proxy.configure(task=ctx.key)
        outcome = _run_confined(ctx, cli, proxy, prompt, transcript, started)
    finally:
        proxy.close()
    network = run_cli.network_rule(ctx.endpoint, proxy.port)
    outcome.network_policy = {"network": network, "sandbox": sandbox.kind(denied(ctx), network),
                              "egress": proxy.summary(ctx.key)}
    return outcome


def _run_confined(ctx: harness.TaskContext, cli: CodexConfig, proxy: egress.EgressProxy, prompt: str,
                  transcript: list[dict], started: str) -> harness.TaskOutcome:
    """`run_task` behind the session's egress proxy."""
    try:
        above = ancestor_instructions(ctx.workdir)
        if above:
            raise run_cli.CliError(f"Codex would read {above[0]}, above the workdir")
        invocation = build_invocation(ctx, cli, proxy)
    except (run_cli.CliError, OSError, agent_env.AgentEnvError) as err:
        return harness.TaskOutcome("infra_error", f"codex setup: {err}", [], transcript, started, harness.utc_now())
    version = cli_version(invocation.argv[0], invocation.env)
    attempt = run_cli.CliAttempt(number=1, attempt_key=f"{ctx.chain_key}:1", model_requested=ctx.model,
                                 provider=ctx.endpoint.provider,
                                 reserved_usd=caps.worst_task_usd(ctx.caps, ctx.price_row) or ctx.caps.usd_per_task)
    try:  # a subscription bills nothing, so it reserves $0 against its budget line
        ctx.ledger.reserve(attempt.attempt_key, attempt.reserved_usd if ctx.billed else 0.0)
    except ledger.BudgetError as err:  # the line has no room for the session, which never starts
        transcript.append({"attempt": 1, "event": "stop", "kind": "aborted_cap", "reason": str(err)})
        return harness.TaskOutcome("aborted_cap", "budget", [], transcript, started, harness.utc_now())
    session = run_cli.run_session(invocation, prompt, cwd=ctx.workdir, wallclock_s=ctx.caps.wallclock_s,
                                  usd_cap=ctx.caps.usd_per_task, meter=Meter(ctx.snapshot, ctx.model))
    status, reason = status_of(session)
    transcript += [{"attempt": 1, "stream": event} for event in session.events]
    if session.stop:
        transcript.append({"attempt": 1, "event": "stop", "kind": session.stop.kind, "reason": session.stop.reason})
    if session.stderr:
        transcript.append({"attempt": 1, "stderr": session.stderr})
    if not session.started:  # no process, so no model call: nothing to book
        ctx.ledger.release(attempt.attempt_key)
        return harness.TaskOutcome(status, reason, [], transcript, started, harness.utc_now())
    attempt.ended_by = reason if status != "infra_error" else status
    _settle(ctx, attempt, session, invocation, version)  # its ledger row releases the reservation
    return harness.TaskOutcome(status=status, reason=reason, attempts=[attempt], transcript=transcript,
                               started_at=started, finished_at=harness.utc_now())


def build_invocation(ctx: harness.TaskContext, cli: CodexConfig,
                     proxy: egress.EgressProxy | None = None) -> run_cli.Invocation:
    """The argv and environment of one session, with its fresh Codex home (made here). Behind `proxy`, the session
    gets the proxy's variables and the sandbox prefix (`denied`, `run_cli.network_rule`)."""
    if ctx.verify_wrapper is not None:
        raise run_cli.CliError("flaky_verify: Codex has no shell prefix to route its commands through the visible-"
                               "verify wrapper")
    program = shutil.which(cli.program)
    if program is None:
        raise run_cli.CliError(f"{cli.program} is not an executable on the driver's PATH")
    program_path = Path(program).absolute()
    codex_home = _seed_codex_home(Path(ctx.agent_env["HOME"]) / ".codex")
    argv = [str(program_path), *FIXED_FLAGS, "-c", f'model_reasoning_effort="{cli.effort}"', WORKDIR_FLAG,
            str(ctx.workdir), "--model", ctx.model, STDIN_PROMPT]
    env = {**ctx.agent_env, "CODEX_HOME": str(codex_home)}
    interpreter_link(program_path, env)
    jail, network = (), None
    if proxy is not None:
        env.update(agent_env.proxy_env(proxy.url))
        network = run_cli.network_rule(ctx.endpoint, proxy.port)
        jail = tuple(sandbox.command([], deny=denied(ctx), network=network))
    agent_env.check(env)
    return run_cli.Invocation(argv=argv, env=env, config_dir=codex_home, config_dir_sha256=home_digest(codex_home),
                              jail=jail, network=network)


def denied(ctx: harness.TaskContext) -> tuple[Path, ...]:
    """What the session's sandbox denies: `ctx.deny`, what `run_cli.settings` keeps from Claude Code's file tools
    (the repository, `~/.roko`, the run's results, the default secret directory), and the operator's Codex home."""
    home = Path.home()
    return (*ctx.deny, layout.REPO_ROOT, home / ".roko", ctx.ledger.path.parent, home / ".config" / "viabilitybench",
            operator_home())


def operator_home() -> Path:
    """The operator's Codex home, where the subscription login lives: `$CODEX_HOME`, else `~/.codex`."""
    return Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")


def status_of(session: run_cli.Session) -> tuple[str, str]:
    """The runner status of a finished session, and why (module docstring)."""
    if session.error:
        return "infra_error", session.error
    if session.stop:
        return session.stop.kind, session.stop.reason
    if any(_item_type(event) in WEB_ITEMS for event in session.events):
        return "infra_error", "web_tools"
    failed = next((event for event in session.events if event.get("type") == "turn.failed"), None)
    if failed is not None:
        return "infra_error", f"codex reported turn.failed: {_message(failed)[:200]}"
    if not any(event.get("type") == "turn.completed" for event in session.events):
        errors = [_message(event) for event in session.events if event.get("type") == "error"]
        detail = (errors[-1] if errors else session.stderr.strip())[-300:]
        return "infra_error", f"codex exited {session.exit_code} without turn.completed" + (
            f": {detail}" if detail else "")
    return "completed", "ended"


def turn_usage(raw: object) -> dict | None:
    """A `turn.completed` event's usage in the run-record shape; None without token counts (module docstring)."""
    if not isinstance(raw, dict):
        return None
    tokens_in, tokens_out = _count(raw.get("input_tokens")), _count(raw.get("output_tokens"))
    if tokens_in is None or tokens_out is None:
        return None
    cached = min(_count(raw.get("cached_input_tokens")) or 0, tokens_in)
    return ledger.vb_usage(tokens_in, tokens_out, cached, _count(raw.get("reasoning_output_tokens")) or 0)


def price_turns(usages: Sequence[dict | None], models: Sequence[str], snapshot: ledger.Snapshot,
                pinned: str) -> tuple[dict | None, ledger.Cost, list[str]]:
    """The turns' summed usage, U′ (source `cli_usage`) and the model each turn was priced at: its own when the
    rollout names one per turn, else the one model the rollout names, else the pin. The cost is unknown when a turn
    had no token counts, a model has no row, or the rollout names several models for fewer turns; the usage is None
    when a turn had no token counts."""
    if not usages or None in usages:
        return None, ledger.Cost(None, None, "unknown"), []
    usage: dict = {}
    for one in usages:
        usage = ledger.add_usage(usage, one)
    distinct = list(dict.fromkeys(models))
    if len(models) == len(usages):
        priced_at = list(models)
    elif len(distinct) <= 1:
        priced_at = [distinct[0] if distinct else pinned] * len(usages)
    else:
        return usage, ledger.Cost(None, None, "unknown"), []
    costs = [ledger.price(one, snapshot.row(model)) for one, model in zip(usages, priced_at)]
    if any(cost.source == "unknown" for cost in costs):
        return usage, ledger.Cost(None, None, "unknown"), priced_at
    return usage, ledger.Cost(sum(cost.api_equiv_usd for cost in costs),
                              sum(cost.without_cache_usd for cost in costs), "cli_usage"), priced_at


def rollout(codex_home: Path) -> Rollout:
    """The session's rollout (module docstring); empty when Codex wrote none, or wrote it in another shape."""
    home = Path(codex_home)
    paths = sorted(home.glob("sessions/**/rollout-*.jsonl"))
    models: list[str] = []
    version = None
    for path in paths:
        try:
            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError:
            continue
        for line in lines:
            try:
                item = json.loads(line)
            except ValueError:
                continue
            payload = item.get("payload") if isinstance(item, dict) else None
            if not isinstance(payload, dict):
                continue
            if item.get("type") == "turn_context" and isinstance(payload.get("model"), str):
                models.append(payload["model"])
            elif item.get("type") == "session_meta" and isinstance(payload.get("cli_version"), str):
                version = version or payload["cli_version"]
    return Rollout(models=models, cli_version=version, files=[path.relative_to(home).as_posix() for path in paths])


def cli_version(program: str, env: Mapping[str, str]) -> str | None:
    """`codex --version`'s version number ("codex-cli 0.152.0" -> "0.152.0"), or None if it cannot say."""
    try:
        done = subprocess.run([program, "--version"], capture_output=True, text=True, timeout=VERSION_TIMEOUT_S,
                              env=dict(env))
    except (OSError, subprocess.SubprocessError):
        return None
    found = VERSION.search(done.stdout) if done.returncode == 0 else None
    return found[0] if found else None


def home_digest(path: Path) -> str:
    """sha256 over a Codex home's paths and file contents; the login counts by name only, since it rotates."""
    entries = {}
    for item in sorted(Path(path).rglob("*")):
        name = item.relative_to(path).as_posix()
        if item.is_symlink():
            entries[name] = "link:" + os.readlink(item)
        elif item.is_dir():
            entries[name] = "dir"
        else:
            entries[name] = "login" if item.name == AUTH_FILE else hashlib.sha256(item.read_bytes()).hexdigest()
    return records.canonical_hash(entries)


def interpreter_link(program: Path, env: Mapping[str, str]) -> Path | None:
    """For a script run through `#!/usr/bin/env <name>` whose interpreter the session's PATH lacks (npm's `codex`
    names node), a link to the driver's `<name>` in the session's `.vb-bin`; None when none is needed or found."""
    try:
        with open(program, "rb") as handle:
            first = handle.read(256).split(b"\n", 1)[0].decode("utf-8", "replace").strip()
    except OSError:
        return None
    match = SHEBANG_ENV.match(first)
    if not match or shutil.which(match[1], path=env["PATH"]):
        return None
    found = shutil.which(match[1], path=os.pathsep.join([str(program.parent), os.environ.get("PATH", "")]))
    if found is None:
        return None  # the session then fails to start, and says why
    link = Path(env["HOME"]) / ".vb-bin" / match[1]
    if not link.is_symlink():
        link.symlink_to(Path(found).absolute())
    return link


def ancestor_instructions(workdir: Path) -> list[Path]:
    """The AGENTS.md files in the directories above `workdir`."""
    return [parent / name for parent in Path(workdir).resolve().parents for name in INSTRUCTION_FILES
            if (parent / name).is_file()]


def _seed_codex_home(codex_home: Path) -> Path:
    """A fresh Codex home holding a private copy of the operator's subscription login, without any API key."""
    if codex_home.exists() and any(codex_home.iterdir()):
        raise run_cli.CliError(f"{codex_home} is not empty; every session gets a fresh Codex home")
    source = operator_home() / AUTH_FILE
    try:
        login = json.loads(source.read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        raise run_cli.CliError(f"the Codex arm needs the subscription login {source}: {err}") from None
    if not isinstance(login, dict) or not isinstance(login.get("tokens"), dict):
        raise run_cli.CliError(f"{source} holds no ChatGPT login (`codex login`); fd_codex runs on the subscription, "
                               "never an API key")
    if API_KEY_FIELD in login:
        login = {**login, API_KEY_FIELD: None}
    codex_home.mkdir(mode=0o700, parents=True, exist_ok=True)
    fd = os.open(codex_home / AUTH_FILE, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as handle:
        handle.write(json.dumps(login))
    return codex_home


def _settle(ctx: harness.TaskContext, attempt: run_cli.CliAttempt, session: run_cli.Session,
            invocation: run_cli.Invocation, version: str | None) -> None:
    """Price the attempt from its turns, fill in its record, and append its ledger row."""
    completed = [event for event in session.events if event.get("type") == "turn.completed"]
    usages = [turn_usage(event.get("usage")) for event in completed]
    found = rollout(invocation.config_dir)
    usage, cost, priced_at = price_turns(usages, found.models, ctx.snapshot, ctx.model)
    served = list(dict.fromkeys(found.models))
    attempt.model_reported = next((model for model in served if not records.same_model(ctx.model, model)),
                                  served[0] if served else ctx.model)
    attempt.turns = len(completed)  # Codex turns; the stream does not count the model calls inside one
    attempt.usage_unknown = usage is None
    attempt.usage = usage or attempt.usage
    attempt.cost = cost
    try:
        attempt.tree = repo.tree_hash(ctx.workdir)
    except (OSError, repo.RepoError):
        attempt.tree = None
    attempt.cli = {
        "argv": invocation.argv, "env": invocation.env, "codex_home_sha256": invocation.config_dir_sha256,
        "cli_version": version, "rollout_cli_version": found.cli_version, "rollout": found.files,
        "turn_usage": [event.get("usage") for event in completed], "turn_models": found.models,
        "model_source": "rollout" if found.models else "pinned", "priced_at": priced_at,
        "u_prime_usd": cost.api_equiv_usd, "r_usd": None,
        "errors": [_message(event) for event in session.events if event.get("type") in ("error", "turn.failed")],
        "exit_code": session.exit_code, "killed": session.stop.reason if session.stop else None}
    ctx.ledger.append(attempt_key=attempt.attempt_key, provider=attempt.provider, model_reported=attempt.model_reported,
                      usage=attempt.reported_usage(), cost=cost, billed=ctx.billed, reserved_usd=attempt.reserved_usd)


def _item_type(event: Mapping) -> str | None:
    item = event.get("item")
    return item.get("type") if event.get("type") in ITEM_EVENTS and isinstance(item, dict) else None


def _message(event: Mapping) -> str:
    """The message of an `error` or `turn.failed` event."""
    error = event.get("error")
    message = error.get("message") if isinstance(error, dict) else event.get("message")
    return str(message or "")


def _count(value: object) -> int | None:
    return value if isinstance(value, int) and not isinstance(value, bool) and value >= 0 else None
