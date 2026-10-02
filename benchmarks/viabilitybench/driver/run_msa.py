"""mini-swe-agent as the pinned candidate direct loop, harness `mini-swe-agent` (S08 decision 3, 3317): a second
"without Roko" baseline, so the field's own direct loop (`mini_loop.py`) is checked against a real, independently
maintained one before either becomes every direct arm's loop (Pilot B's optional block, 3308).

mini-swe-agent (MIT, github.com/SWE-agent/mini-swe-agent) is pinned in `requirements-msa.lock` and installed into its
own venv, `.venv-msa/`, next to the driver's own `.venv/`; this module never imports it, only runs its `mini` CLI as
a subprocess (`binary_path`, `[msa] binary` in the arm file, default `.venv-msa/bin/mini`), under 3303's sandbox with
loopback to the arm's endpoint only (the metering proxy's port in a proxied run), exactly as Roko's process tree is
confined (`run_roko.network_rule`): unlike `mini_loop.py`, whose driver makes every model call itself, mini-swe-agent
makes its own, so its whole process tree needs that one loopback port, not none.

Our own prompt (`_config`) replaces mini-swe-agent's bundled one, so the agent sees exactly the spec text every arm
gets (A4, module docstring of `planemit.py`) and nothing SWE-bench-specific; its completion marker stays
mini-swe-agent's own (`echo COMPLETE_TASK_AND_SUBMIT_FINAL_OUTPUT`, checked by its `LocalEnvironment`), not
`mini_loop.py`'s `VB_SUBMIT`. The arm's caps map onto its own limits: `agent.step_limit` from `caps.turns_per_task`
and its `--cost-limit` disabled (0), since litellm's cost tracking does not know our custom models even with
`model.cost_tracking: ignore_errors` (it always reports $0 for one) — the metering proxy's input-token cap and this
module's wallclock kill are what actually bound a task, as they are for Roko. `MSWEA_CONFIGURED=1` skips its
interactive first-run setup, which would otherwise block on a fresh HOME (every task's `agent_env.build`).

One attempt per task: mini-swe-agent manages its own turns internally and the driver does not restart it. Its
trajectory file (`--output`, read back after it exits) says how the session ended (`info.exit_status`: `"Submitted"`
a completed one, `"LimitsExceeded"` or `"TimeExceeded"` a governed stop, anything else an error) and how many model
calls it made (`info.model_stats.api_calls`); its own cost is never used. The attempt is priced from the metering
proxy's meter when one ran (`vb.py` sets it up exactly as for any other billed network run); without the proxy its
usage, and so its cost, is unknown, as an un-proxied `mini_loop.py` call's is.

Records keep `arm = "cheap_direct"`, the same arm id as the bash-only loop's (S08 decision 3 compares them on equal
footing), with `provenance.network_policy.harness` naming `"mini-swe-agent"` so `analysis/metrics.py` never pools
the two loops' runs: a cell is (arm, harness, model), not (arm, model) alone, once more than one harness has run
that arm.

API:
    run_task(ctx: harness.TaskContext) -> harness.TaskOutcome
    binary_path(arm: dict) -> Path                        # raises RunnerError
    network_policy(deny, endpoint) -> dict
    HARNESS, PROMPT_VERSION, PROMPT_SHA256
"""

from __future__ import annotations

import hashlib
import json
import os
import signal
import subprocess
import tempfile
import time
import urllib.parse
from pathlib import Path

import caps
import harness
import layout
import ledger
from common import repo, sandbox

HARNESS = "mini-swe-agent"
PROMPT_VERSION = "run-msa-1"
DEFAULT_BINARY = ".venv-msa/bin/mini"  # relative to the benchmark tree (layout.VB_ROOT), like the Roko arm's binary
OFFLINE_KEY = "vb-offline-placeholder"  # the proxy drops it and sends the real key; a stub ignores it
STEP_LIMIT_FLOOR = 4  # mini-swe-agent counts a step as one model call; never ask for fewer than this
WALLCLOCK_GRACE_S = 10.0  # given to mini-swe-agent to write its trajectory after its own session ends

SYSTEM_TEMPLATE = "You are a software engineer working on a task in a git repository, through a bash shell."
INSTANCE_TEMPLATE = """\
<task>
{{task}}
</task>

The repository is your current working directory. Complete the task, then submit by running exactly this command,
alone, with no other command:

    echo COMPLETE_TASK_AND_SUBMIT_FINAL_OUTPUT
"""
# A minimal config of our own (module docstring): no SWE-bench system prompt, no few-shot examples. Placeholders are
# plain tokens, never `.format()` braces, since the templates above hold mini-swe-agent's own Jinja `{{...}}`.
CONFIG_TEMPLATE = """\
agent:
  system_template: __SYSTEM__
  instance_template: __INSTANCE__
  step_limit: __STEP_LIMIT__
  cost_limit: 0
  mode: yolo
model:
  model_name: __MODEL__
  model_kwargs:
    api_base: __API_BASE__
    api_key: __API_KEY__
  cost_tracking: ignore_errors
environment:
  cwd: __CWD__
  timeout: __COMMAND_TIMEOUT__
"""
PROMPT_SHA256 = hashlib.sha256("\0".join([SYSTEM_TEMPLATE, INSTANCE_TEMPLATE, CONFIG_TEMPLATE]).encode()).hexdigest()


class RunnerError(RuntimeError):
    """The arm cannot run this task: the binary or the environment is wrong."""


def run_task(ctx: harness.TaskContext) -> harness.TaskOutcome:
    started, clock = harness.utc_now(), time.monotonic()
    binary = binary_path(ctx.arm)
    if not ctx.endpoint.offline:
        raise RunnerError(f"{ctx.endpoint.provider} is a network provider, which mini-swe-agent reaches only "
                          "through the metering proxy: run it with `vb run` (which proxies every billed network "
                          "run, and any run with --proxy) and a key file (--key-file)")
    network = _network_rule(ctx.endpoint)
    jail = sandbox.command([], deny=ctx.deny, network=network, sockets=[ctx.workdir])
    bound = caps.worst_task_usd(ctx.caps, ctx.price_row)
    task_bound = ctx.caps.usd_per_task if bound is None else bound
    attempt = harness.Attempt(number=1, attempt_key=f"{ctx.chain_key}:1", model_requested=ctx.model,
                              provider=ctx.endpoint.provider, reserved_usd=task_bound, usage_unknown=True)
    transcript: list[dict] = []
    status, reason = "infra_error", ""
    reserved = False
    with tempfile.TemporaryDirectory(prefix="vb-msa-") as scratch_name:
        scratch = Path(scratch_name)
        config_path = scratch / "mini.yaml"
        config_path.write_text(_config(ctx, task_bound), encoding="utf-8")
        output_path = scratch / "trajectory.json"
        env = {**ctx.agent_env, "MSWEA_CONFIGURED": "1", "MSWEA_SILENT_STARTUP": "1",
              "MSWEA_COST_TRACKING": "ignore_errors", "MSWEA_GLOBAL_CONFIG_DIR": str(scratch / "global-config")}
        argv = [str(binary), "-m", _litellm_model(ctx.model), "-t", ctx.spec_text.strip(), "-y",
               "--exit-immediately", "-c", str(config_path), "-l", "0", "-o", str(output_path)]
        try:
            ctx.ledger.reserve(attempt.attempt_key, task_bound if ctx.billed else 0.0)
            reserved = True
            ran = _run(jail + argv, ctx.workdir, env, ctx.caps.wallclock_s, transcript)
            trajectory, problem = _read_trajectory(output_path)
            if problem:
                raise RunnerError(problem)
            attempt.turns = _calls(trajectory)
            attempt.calls = attempt.turns
            attempt.ended_by = ((trajectory.get("info") or {}).get("exit_status")) or ("timeout" if ran.timed_out
                                                                                        else "unknown")
            status, reason = _status(ran, trajectory)
        except ledger.BudgetError as err:  # no room on the budget line: mini-swe-agent never started
            status, reason = "aborted_cap", "budget"
            attempt.ended_by = "budget"
            transcript.append({"event": "stop", "kind": status, "reason": str(err)})
        except (RunnerError, OSError, subprocess.SubprocessError) as err:
            reason = f"{type(err).__name__}: {err}"[:500]
            transcript.append({"event": "error", "error": reason})
        finally:
            if reserved:
                meter = _task_meter(ctx)
                attempt.cost = meter.cost if meter else None
                if meter:
                    attempt.model_reported = meter.model_reported
                    attempt.usage = meter.usage
                    attempt.usage_unknown = meter.usage_unknown
                    attempt.calls = meter.calls
                attempt.tree = _tree_hash(ctx.workdir)
                ctx.ledger.append(attempt_key=attempt.attempt_key, provider=attempt.provider,
                                  model_reported=attempt.model_reported, usage=attempt.reported_usage(),
                                  cost=attempt.cost or ledger.Cost(None, None, "unknown"), billed=ctx.billed,
                                  reserved_usd=attempt.reserved_usd)
    return harness.TaskOutcome(status=status, reason=reason, attempts=[attempt], transcript=transcript,
                               started_at=started, finished_at=harness.utc_now(),
                               network_policy=network_policy(ctx.deny, ctx.endpoint))


def binary_path(arm: dict) -> Path:
    """The `mini` CLI the arm names (`[msa] binary`, relative to the benchmark tree), or the default venv's."""
    raw = Path(arm.get("msa", {}).get("binary") or DEFAULT_BINARY).expanduser()
    path = raw if raw.is_absolute() else layout.VB_ROOT / raw
    if not (path.is_file() and os.access(path, os.X_OK)):
        raise RunnerError(f"no executable mini binary at {path}; pip install -r requirements-msa.lock into "
                          f"{layout.VB_ROOT / '.venv-msa'}")
    return path


def network_policy(deny: tuple[Path, ...], endpoint) -> dict:
    """The run record's `provenance.network_policy`: the loopback rule mini-swe-agent's whole process tree ran
    under, the confinement that applied it, and the harness marker `analysis/metrics.py` groups runs by (module
    docstring), since this arm's `arm` id is shared with `mini_loop.py`'s."""
    network = _network_rule(endpoint)
    return {"network": network, "sandbox": sandbox.kind(deny, network), "harness": HARNESS}


def _network_rule(endpoint) -> str:
    """mini-swe-agent's own process makes the model call (unlike `mini_loop.py`'s driver-made ones), so its tree
    needs the loopback port it calls, exactly as Roko's does (`run_roko.network_rule`)."""
    if not endpoint.offline:
        return sandbox.NETWORK_NONE
    parts = urllib.parse.urlsplit(endpoint.base_url)
    return sandbox.loopback(parts.port or (443 if parts.scheme == "https" else 80))


def _litellm_model(model: str) -> str:
    """litellm routes on a provider prefix; `openai` sends the bare model name to any OpenAI-compatible `api_base`,
    which is all the metering proxy and every arm's upstream are (module docstring)."""
    return f"openai/{model}"


def _config(ctx: harness.TaskContext, task_bound: float) -> str:
    step_limit = max(STEP_LIMIT_FLOOR, int(ctx.caps.turns_per_task))
    text = CONFIG_TEMPLATE
    for token, value in (("__SYSTEM__", SYSTEM_TEMPLATE), ("__INSTANCE__", INSTANCE_TEMPLATE),
                        ("__MODEL__", _litellm_model(ctx.model)), ("__API_BASE__", ctx.endpoint.base_url),
                        ("__API_KEY__", OFFLINE_KEY), ("__CWD__", str(ctx.workdir))):
        text = text.replace(token, json.dumps(value))
    for token, value in (("__STEP_LIMIT__", step_limit), ("__COMMAND_TIMEOUT__", ctx.caps.command_timeout_s)):
        text = text.replace(token, json.dumps(value))
    return text


def _run(argv: list[str], cwd: Path, env: dict[str, str], wallclock_s: float, transcript: list[dict]):
    """Run mini-swe-agent in its own session; kill the whole session at the task's wallclock cap, plus a short
    grace so it can still write its trajectory file."""
    start = time.monotonic()
    process = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, start_new_session=True)
    timed_out = False
    try:
        out, _ = process.communicate(timeout=max(wallclock_s, 1.0))
    except subprocess.TimeoutExpired:
        timed_out = True
        try:
            out, _ = process.communicate(timeout=WALLCLOCK_GRACE_S)
        except subprocess.TimeoutExpired:
            _kill_session(process.pid)
            try:
                out, _ = process.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                out = b""
    finally:
        _kill_session(process.pid)
    text = out.decode("utf-8", "replace")
    transcript.append({"event": "mini", "returncode": process.returncode, "timed_out": timed_out,
                       "seconds": round(time.monotonic() - start, 3), "output": text[-20_000:]})
    return _Ran(process.returncode, timed_out)


class _Ran:
    def __init__(self, returncode: int | None, timed_out: bool) -> None:
        self.returncode, self.timed_out = returncode, timed_out


def _read_trajectory(path: Path) -> tuple[dict, str | None]:
    if not path.is_file():
        return {}, "mini-swe-agent left no trajectory file"
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except (OSError, ValueError) as err:
        return {}, f"the trajectory file is unreadable: {err}"


def _calls(trajectory: dict) -> int:
    calls = ((trajectory.get("info") or {}).get("model_stats") or {}).get("api_calls")
    return calls if isinstance(calls, int) and not isinstance(calls, bool) and calls >= 0 else 0


def _status(ran: _Ran, trajectory: dict) -> tuple[str, str]:
    exit_status = (trajectory.get("info") or {}).get("exit_status")
    if ran.timed_out and exit_status not in ("Submitted",):
        return "timeout", "wallclock"
    if exit_status == "Submitted":
        return "completed", "submitted"
    if exit_status in ("LimitsExceeded", "TimeExceeded", "RepeatedFormatError"):
        return "aborted_cap", exit_status
    return "infra_error", f"mini-swe-agent exited {ran.returncode} with status {exit_status or 'unrecorded'}"


def _task_meter(ctx: harness.TaskContext):
    """This task's totals in the metering proxy's meter, in `harness.Attempt` shape; None without a proxy."""
    path = ctx.ledger.path.parent / "proxy.jsonl"
    if not path.is_file():
        return None
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if isinstance(row, dict) and row.get("task") == ctx.key:
                rows.append(row)
    billed = [row for row in rows if row.get("usage_source") != "none"]
    served = {row.get("model_reported") for row in billed}
    model_reported = served.pop() if len(served) == 1 else None
    usage_unknown = any(not isinstance(row.get("usage"), dict) for row in billed)
    total = ledger.vb_usage(0, 0)
    for row in billed:
        if isinstance(row.get("usage"), dict):
            total = ledger.add_usage(total, row["usage"])
    if not billed:
        cost = ledger.Cost(0.0, 0.0, "provider_usage")
    elif usage_unknown:
        cost = None
    else:
        cost = ledger.price(total, ctx.snapshot.row(model_reported))
    return _Meter(model_reported=model_reported, usage=total, usage_unknown=usage_unknown, calls=len(rows), cost=cost)


class _Meter:
    def __init__(self, *, model_reported, usage, usage_unknown, calls, cost) -> None:
        self.model_reported, self.usage, self.usage_unknown = model_reported, usage, usage_unknown
        self.calls, self.cost = calls, cost


def _tree_hash(workdir: Path) -> str | None:
    try:
        return repo.tree_hash(workdir)
    except (OSError, repo.RepoError):
        return None


def _kill_session(pid: int) -> None:
    try:
        os.killpg(pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass
