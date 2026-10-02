"""The Roko arm's runner, harness `roko` (S08 §4.9 `roko_fixed`, T11): one task through `roko plan run` on one
pinned model, which is checked on every attempt.

Before the first task, `preflight` emits the plan of a stand-in task into a scratch workspace and runs the
`plan validate --strict --dag` of step 2 on it, so `vb run` refuses a binary that rejects the plans this arm emits
(bug-a05c53) instead of ending every task `infra_error`. For each (task, seed), `run_task`:

1. emits a one-task plan and the workspace roko.toml into the task's workdir, which is Roko's workspace
   (`planemit`);
2. runs `roko --repo <ws> --model <m> plan validate --strict --dag <ws>/plans`; a plan that fails is `infra_error`;
3. reserves every attempt Roko may make on the run's budget line (`Ledger.reserve`: `max_retries` + 1 attempts, each
   an equal share of the task's worst case, $0 when unbilled), because Roko retries without the driver. A refusal
   ends the task `aborted_cap` (reason `budget`) before any model call. Then it runs
   `roko --repo <ws> --model <m> plan run <plan> --no-tui` (never `roko run`), in its own session, killed at the
   task's wall-clock cap (status `timeout`);
4. reads Roko's records (`read_evidence`), makes one attempt per dispatch and checks each (`settle`);
5. appends one ledger row per attempt, which releases its reservation, and releases the reservations of the attempts
   Roko did not make. It copies Roko's records to `<run_dir>/s01/<key>/`, and removes `roko.toml`,
   `plans/` and `.roko/` from the workdir, so c_i holds only the agent's tree.

**Environment.** Roko gets the task's agent environment (`agent_env`), plus `ROKO_CONFIG` naming the emitted
roko.toml (`plan run` ignores `--config`) and a placeholder for the key that roko.toml names, because Roko refuses a
provider without one. The real key never reaches Roko's process tree, whose tools run the agent's commands
(bug-979a06): Roko reaches the provider only on a loopback URL, the metering proxy's (which sends the key) or a
stub's, and a network endpoint is refused before Roko starts. The agent environment gives it a fresh HOME, so no
`~/.roko/.env` loads and no learned state crosses seeds, along with PATH, TMPDIR, locale and a git identity. Nothing
else of the driver's environment reaches Roko. At 33e107da1, Roko needed nothing under HOME and wrote nothing there.

**Network** (gap-0bd49a, 3304). Roko's tools run the agent's commands inside Roko's process tree, so on macOS every
roko process of a task (`--version`, `plan validate`, `plan run`) runs through `common.sandbox` with the rule
`loopback:<port>`, the port of the loopback endpoint Roko calls (`network_rule`): the metering proxy's, or a stub's.
No other connection can open, to another loopback port, to DNS or to the internet. Unix sockets stay open under the
workspace, where Roko keeps its StateHub and per-run `inject` sockets (`.roko/runtime/`). The tree is also denied
`ctx.deny`, the files and directories no agent may touch. The run record names the rule
(`provenance.network_policy`); off macOS its `sandbox` is "none" and the network stays open (gap-29ac83). The
preflight's `plan validate`, which runs before any task, gets the rule "none". Builds need no network for the Python
families; a family that fetches its dependencies needs them vendored first (F7, 3324).

**The model check** (W10 rec 5, bug-35379d, bug-31438d). Every record Roko writes must name the pinned model and the
arm's provider, as the model it dispatched and, when the provider reported one, as the model that served:
- each episode (`.roko/episodes.jsonl`, one per dispatch): `model` and `backend`, and in `extra` the served
  `model_reported`, Roko's own `model_mismatch` mark and a failover's `substituted_from`;
- the cost rows (`.roko/learn/costs.jsonl`), the agent run's and each helper call's (role `helper`);
- every model-call and efficiency row (`.roko/learn/efficiency.jsonl`), helper calls included;
- S01's `roko.verdict/1` lines (`.roko/runs/*/attempts.jsonl`, gap-528762): `executed.model_requested`,
  `model_dispatched`, `model_reported`, `models_reported`, `model_mismatch` and `failover_chain`.

A null `model_reported` is no evidence either way, and a dated snapshot of the pin is the pin (`records.same_model`).
A task that a `model_swap` disturbance covers declares the model the proxy serves in place of the pin
(`ctx.model_swap`, gap-8bdf5e): that model is accepted wherever a record names a served model, Roko's own mismatch
mark included, and the attempts it served are marked `model_swapped`. The proxy's log names each swap it made
(`model_swap`), and a swap no disturbance declared is a mismatch. Roko's failover is never a declared swap.
Each row belongs to the attempt its S01 attempt key names (`<run>:<plan>:<task>:<n>`; a helper call's efficiency row
is `<key>/helper-<i>`), and attempts are numbered by their episodes' keys, whatever order the rows are in. An older
Roko's rows carry no key: its episodes and dispatch cost rows count in file order, its efficiency rows by `/a<n>`.

The failure kinds:
- `model_mismatch`: a record names another model or provider, or a failover.
- `model_unverified`: an attempt has no episode, Roko left no records or unreadable ones, or its episodes' attempt
  keys repeat or name only some attempts.
- `extra_attempts`: Roko made more attempts than the plan allows.

Any of them makes the task `infra_error`. The report excludes and counts it, and the census never labels it a
success. Roko's records live in the agent's workdir, so a hostile agent could edit them. Only the proxy's log is out
of the agent's reach.

**What Roko records of an attempt.** Checked at b0ede92d7 against a loopback fake:
- its turns, the tool loop's model calls. `extra.turns_unknown` marks a count the agent never reported, and the
  attempt's `turns` is then null, not 0 (bug-55fd84);
- the helper calls it makes after a failed gate (quality rating, error diagnosis, reflection): `extra.helper_calls`,
  and a cost row and an efficiency row each (bug-62e3f4). The attempt waits for them before it writes its episode;
- the served model: the helper calls' rows name it, but the agent run's streamed calls leave it null. So an
  attempt's `model_reported` is null and its cost unknown (null) unless S01 verdicts or the metering proxy supply
  them. Roko's own token counts stay in each attempt's `roko_usage`, for diagnosis only. Roko's USD is never used.
  When S01's verdict meters an attempt and says its usage was `estimated` (`cost.source`: usage a call streamed
  before Roko cut it off), the attempt's cost is `estimated` too, in its record and its ledger row, and the report
  counts it apart (gap-288e38). The proxy, when it meters an attempt, saw what the provider billed.
- its gate verdict: S01's outcome, which each episode carries in `extra.outcome`. `passed`, `already_satisfied`,
  `unverified` and `forced_accept` are verdict tags, and any other outcome has none. An older Roko's episode says
  only whether it succeeded, read as `passed`.

An attempt's `roko_calls` is its turns plus its helper calls, None when either is unknown. Without the proxy it is
also the attempt's `calls`; with the proxy, `calls` is the proxy's count and `roko_calls` stays beside it, so the
record shows where the two disagree (a helper call still running when the attempt settled counts only in the proxy).

**The proxy** (gap-e003ec, `faultproxy.py`). `vb run` routes Roko through it on every billed network run, and on any
run with `--proxy`. Roko gets the proxy's loopback URL and a placeholder key, and the proxy sends the real one, from
the driver's key file. When `<run_dir>/proxy.jsonl` exists, its rows for this task are the meter. The driver sets
the proxy's active task to the task key before each task (`proxy.configure(task=key)`); rows without that key never
match, so the attempts fail as `no_proxy_traffic`.
- Requests are assigned to attempts by `ts`, in `ordinal` order. An attempt owns the requests up to its episode, its
  helper calls included, since Roko waits for them before it writes the episode; the last attempt owns the rest. The
  proxy stamps each request to the microsecond, as Roko stamps its episodes, so attempts that end within one second
  keep their own requests (bug-09fac4). With whole-second stamps on either side (an older proxy log), an attempt that
  ended in the same second as the one before cannot be told apart from it. Task totals are exact either way.
- Every request's `model_requested` and `model_reported` must be the pin.
- An attempt's usage is the sum of its billed requests (`usage_source` other than `none`), in run-record shape. It
  is priced from the snapshot by the model the provider reported. A billed request without usage leaves the
  attempt's cost unknown, never 0; an attempt whose requests were all unbilled (refused, faulted or failed) cost $0.
- An attempt whose window saw no request is `no_proxy_traffic`, which makes the task `infra_error`.
- Roko has no per-attempt input cap, so the proxy holds each attempt to the arm's `input_tokens_per_attempt`
  (`PROXY_CAPS`; the proxy draws attempts as conversations, gap-806e37). An attempt whose calls it refused there ends
  `attempt_input_tokens`, the direct loop's name for that cap.

**Status.**
- `completed`: the Graph checkpoint says the plan succeeded with a `passed` gate verdict. This is Roko's reported
  pass, so a census VS = 0 counts as a false green.
- `failed`: the gates failed through every retry, or Roko stopped the plan itself. Also a plan that succeeded with
  an `already_satisfied` verdict (gap-9eb1e1): Roko changed nothing, and its checks passed on the tree as it was.
  S05 §0.1 counts only a `passed` verdict as Roko's completion, so the census gives the run 0, and the report counts
  it apart, never as a reported pass.
- `timeout`: the wall-clock cap.
- `infra_error`: a failed validation, a failed check above, or a run Roko did not finish.

**Process measures** (S09 §4.9, gap-04e8e2). Each attempt records its plan task (`T01`), its cost class and its busy
time, and `records.py` sums them into the record:
- the class: `execute` for the first attempt, then `retry`, or `escalate` for one that dispatched another model. A
  one-task plan has no planner call (the driver writes the plan) and no integration step, so `plan` and
  `integrate` cost $0;
- the busy time: S01's verdict timing from the attempt's start to its settlement when Roko writes it, else the
  episode's dispatch window, which ends at `completed_at` and lasts `duration_secs`;
- the queue wait: a one-task plan never waits for a dispatch slot, so it is the time the task waited after a rate
  limit (a 429 in the proxy's log) until its next request. It is known only when the proxy metered the run, and
  null otherwise. Roko records neither when a task became ready nor when it was dispatched, so a multi-task plan's
  slot waits need the Graph engine to record them.

API:
    run_task(ctx: harness.TaskContext) -> harness.TaskOutcome
    preflight(arm, model, endpoint, limits, snapshot) -> None       # raises RunnerError
    network_rule(endpoint: provider.Endpoint) -> str                # the roko processes' network rule
    read_evidence(workspace: Path, slug: str, *, proxy_rows: list[dict] | None = None) -> Evidence
    settle(evidence, *, chain_key, model, provider, snapshot, reserved_usd, max_attempts, roko_build=None,
           swap=None, rungs=()) -> (list[RokoAttempt], list[str])
    RokoAttempt(harness.Attempt); Evidence; binary_path(arm) -> Path; PROMPT_VERSION, PROMPT_SHA256
"""

from __future__ import annotations

import datetime as dt
import json
import os
import re
import shutil
import signal
import subprocess
import tempfile
import time
import urllib.parse
from collections.abc import Iterable
from dataclasses import dataclass, field, replace
from pathlib import Path

import agent_env
import archive
import caps
import harness
import layout
import ledger
import planemit
import provider
import records
from common import repo, sandbox

PROXY_CAPS = ("input_tokens_per_attempt",)  # arm caps that `vb run` has the metering proxy hold for this runner
PROMPT_VERSION = planemit.TEMPLATE_VERSION  # Roko's own prompt is in its binary, recorded per attempt as roko_build
PROMPT_SHA256 = planemit.TEMPLATE_SHA256
DEFAULT_BINARY = "target/debug/roko"  # relative to the repository root, like the arm file's [roko] binary
OFFLINE_KEY = "vb-offline-placeholder"
VALIDATE_TIMEOUT_S = 120.0
# The stand-in task `preflight` emits a plan for: a family task's shape (one source file, visible tests run by
# unittest), with nothing of any task in it.
PREFLIGHT_SPEC = ("# Preflight\n\nA stand-in task: before any task runs, the driver checks that Roko accepts the "
                  "plan this arm emits.\n")
PREFLIGHT_TREE = {"src/stand_in.py": "", "tests/visible/test_stand_in.py": "import unittest\n"}
PREFLIGHT_VISIBLE = "python3 -m unittest discover -s tests/visible"
OUTPUT_CHARS = 20_000  # of each of Roko's stdout and stderr kept in the transcript, head and tail
EVIDENCE_MAX_BYTES = 50_000_000
BUILD = re.compile(r"\bgit ([0-9a-f]{7,40})\b")
ATTEMPT_ID = re.compile(r"/a(\d+)(?:/|$)")  # an older Roko's attempt ids, from 0
VERDICT_SCHEMA = "roko.verdict/1"
VERDICT_TAGS = ("passed", "already_satisfied", "unverified", "forced_accept")  # S01 outcomes that are verdict tags
HELPER_ROLE = "helper"  # the role of a helper call's cost and efficiency rows (bug-62e3f4)


class RunnerError(RuntimeError):
    """The arm cannot run this task: the binary, the plan or the environment is wrong."""


@dataclass
class RokoAttempt(harness.Attempt):
    model_dispatched: str | None = None  # the model Roko's records say it dispatched
    gate_verdict: str | None = None  # S01's tag (VERDICT_TAGS), or None when the gate failed
    roko_usage: dict | None = None  # Roko's own token counts, for diagnosis only: never priced
    roko_build: str | None = None
    checks: list[str] = field(default_factory=list)  # failed check kinds
    calls_known: bool = False  # only the proxy counts every call
    task_id: str | None = None  # the plan task, from the episode
    cost_class: str | None = None  # S09 §4.9: execute, then retry, or escalate on another model
    started_at: str | None = None  # the attempt's busy time (module docstring, "Process measures")
    finished_at: str | None = None
    queue_wait_s: float | None = None  # rate-limit waits in its proxy window; None without the proxy
    helper_calls: int | None = None  # helper model calls after its gate (bug-62e3f4); None when Roko does not say
    roko_calls: int | None = None  # its model calls by Roko's records: turns plus helper calls; None if unknown
    model_swapped: bool = False  # a record says the declared swap's model served it (model_swap, gap-8bdf5e)
    usage_estimated: bool = False  # S01's verdict metered it from usage a call streamed (`cost.source` estimated)

    def as_record(self) -> dict:
        record = super().as_record()
        record.update(model_dispatched=self.model_dispatched, gate_verdict=self.gate_verdict,
                      roko_usage=self.roko_usage, roko_build=self.roko_build, checks=list(self.checks),
                      task_id=self.task_id, cost_class=self.cost_class, started_at=self.started_at,
                      finished_at=self.finished_at, queue_wait_s=self.queue_wait_s, helper_calls=self.helper_calls,
                      roko_calls=self.roko_calls)
        if self.model_swapped:
            record["model_swapped"] = True
        if not self.calls_known:
            record["calls"] = None
        return record


@dataclass
class Evidence:
    episodes: list[dict]  # this plan's episodes in file order: one per dispatch
    cost_rows: list[dict]  # this plan's rows of learn/costs.jsonl
    efficiency: list[dict]  # model-call and efficiency rows of learn/efficiency.jsonl
    verdicts: list[dict]  # S01 roko.verdict/1 lines for this plan
    checkpoint: dict | None  # the Graph checkpoint
    proxy_rows: list[dict] | None  # None when no proxy metered the run
    unreadable: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class Ran:
    argv: list[str]
    returncode: int | None
    stdout: str
    stderr: str
    timed_out: bool
    seconds: float

    def event(self, name: str) -> dict:
        return {"event": name, "argv": self.argv, "returncode": self.returncode, "timed_out": self.timed_out,
                "seconds": round(self.seconds, 3), "stdout": _clip(self.stdout), "stderr": _clip(self.stderr)}


def run_task(ctx: harness.TaskContext) -> harness.TaskOutcome:
    started, clock = harness.utc_now(), time.monotonic()
    settings = ctx.arm.get("roko", {})
    max_retries = int(settings.get("max_retries", 2))
    bound = _worst_task_usd(ctx.arm, ctx.snapshot, ctx.caps, ctx.price_row)
    task_bound = ctx.caps.usd_per_task if bound is None else bound
    network = network_rule(ctx.endpoint)
    jail = sandbox.command([], deny=ctx.deny, network=network, sockets=[ctx.workdir])  # every roko process's prefix
    transcript: list[dict] = []
    attempts: list[RokoAttempt] = []
    emitted = build = None
    status, reason, dispatched = "infra_error", "", False
    reserved: list[str] = []
    try:
        binary = binary_path(ctx.arm)
        spec = _plan_spec(ctx.arm, ctx.model, ctx.endpoint, ctx.caps, ctx.price_row, task_bound, key=ctx.key,
                          spec_text=ctx.spec_text, files=ctx.files_in_scope, visible=ctx.visible_verify,
                          verify_wrapper=_wrapper_command(ctx), snapshot=ctx.snapshot)
        emitted = planemit.emit(spec, ctx.workdir)
        transcript.append({"event": "emit", "slug": emitted.slug, "tasks_toml": emitted.tasks_text,
                           "roko_toml": emitted.config_text})
        env = _roko_env(ctx, {spec.api_key_env, *(rung.api_key_env for rung in spec.rungs)}, emitted.config_path)
        build = _build(binary, env, settings.get("build") or None, transcript, jail)
        head = _head(binary, ctx.workdir, ctx.model)
        checked = _roko([*head, "plan", "validate", "--strict", "--dag", str(ctx.workdir / "plans")], ctx.workdir,
                        env, min(VALIDATE_TIMEOUT_S, _left(ctx, clock)), jail)
        transcript.append(checked.event("validate"))
        if checked.returncode != 0:
            raise RunnerError(f"the emitted plan failed `plan validate --strict --dag` (exit {checked.returncode}): "
                              f"{(checked.stdout + checked.stderr).strip()[-300:]}")
        keys = [f"{ctx.chain_key}:{number}" for number in range(1, max_retries + 2)]
        _reserve(ctx.ledger, keys, task_bound / (max_retries + 1) if ctx.billed else 0.0)
        reserved = keys
        dispatched = True
        ran = _roko([*head, "plan", "run", str(emitted.plan_dir), "--no-tui"], ctx.workdir, env, _left(ctx, clock),
                    jail)
        transcript.append(ran.event("run"))
        evidence = read_evidence(ctx.workdir, emitted.slug, proxy_rows=_proxy_rows(ctx))
        attempts, problems = settle(evidence, chain_key=ctx.chain_key, model=ctx.model,
                                    provider=ctx.endpoint.provider, snapshot=ctx.snapshot,
                                    reserved_usd=task_bound / (max_retries + 1), max_attempts=max_retries + 1,
                                    roko_build=build, swap=ctx.model_swap, rungs=spec.rungs)
        transcript += [_attempt_event(attempt, episode) for attempt, episode in zip(attempts, evidence.episodes)]
        transcript.append({"event": "check", "problems": problems})
        status, reason = _status(ran, evidence, problems)
    except ledger.BudgetError as err:  # no room on the budget line: `plan run` never started
        status, reason = "aborted_cap", "budget"
        transcript.append({"event": "stop", "kind": status, "reason": str(err)})
    except (RunnerError, planemit.PlanEmitError, OSError, subprocess.SubprocessError) as err:
        reason = f"{type(err).__name__}: {err}"[:500]
        transcript.append({"event": "error", "error": reason})
    finally:
        if emitted is not None:
            _save_evidence(ctx.workdir / ".roko", ctx.ledger.path.parent / "s01" / ctx.key, transcript)
            for name in planemit.SCAFFOLDING:
                archive.remove_tree(ctx.workdir / name)
        if dispatched and not attempts:  # Roko may have called the model without recording an attempt
            attempts = [RokoAttempt(number=1, attempt_key=f"{ctx.chain_key}:1", model_requested=ctx.model,
                                    provider=ctx.endpoint.provider, reserved_usd=task_bound, usage_unknown=True,
                                    ended_by="no_attempt_record", roko_build=build, checks=["model_unverified"])]
        if attempts:
            try:
                attempts[-1].tree = repo.tree_hash(ctx.workdir)
            except (OSError, repo.RepoError):
                attempts[-1].tree = None
        for attempt in attempts:
            ctx.ledger.append(attempt_key=attempt.attempt_key, provider=attempt.provider,
                              model_reported=attempt.model_reported, usage=attempt.reported_usage(),
                              cost=attempt.cost or ledger.Cost(None, None, "unknown"), billed=ctx.billed,
                              reserved_usd=attempt.reserved_usd)
        for key in reserved:  # the attempts Roko did not make; a key whose row was just written is already free
            ctx.ledger.release(key)
    saved = (ctx.ledger.path.parent / "s01" / ctx.key).is_dir()  # Roko's records, copied by _save_evidence
    return harness.TaskOutcome(status=status, reason=reason, attempts=list(attempts), transcript=transcript,
                               started_at=started, finished_at=harness.utc_now(),
                               s01_run_dir=f"s01/{ctx.key}" if saved else None,
                               network_policy={"network": network, "sandbox": sandbox.kind(ctx.deny, network),
                                               "unix_sockets": "workspace"})


def network_rule(endpoint: provider.Endpoint) -> str:
    """The network rule of a task's roko processes: the loopback port of the endpoint Roko calls, the metering
    proxy's or a stub's; "none" for a network endpoint, which `_roko_env` refuses before Roko starts."""
    if not endpoint.offline:
        return sandbox.NETWORK_NONE
    parts = urllib.parse.urlsplit(endpoint.base_url)
    return sandbox.loopback(parts.port or (443 if parts.scheme == "https" else 80))


def preflight(arm: dict, model: str, endpoint: provider.Endpoint, limits: caps.Caps, snapshot: ledger.Snapshot) -> None:
    """Refuse a binary that rejects the plans this arm emits, before any task runs (bug-a05c53): every task's
    `plan validate --strict --dag` would fail, and every run would end `infra_error`. Emits the plan of a stand-in
    task (`PREFLIGHT_SPEC`) into a scratch workspace and validates it as `run_task` does. No model is called.
    Raises RunnerError."""
    binary = binary_path(arm)
    price_row = snapshot.row(model)
    bound = _worst_task_usd(arm, snapshot, limits, price_row)
    with tempfile.TemporaryDirectory(prefix="vb-roko-preflight-") as scratch:
        workspace = Path(scratch) / "workspace"
        for relpath, text in PREFLIGHT_TREE.items():
            (workspace / relpath).parent.mkdir(parents=True, exist_ok=True)
            (workspace / relpath).write_text(text, encoding="utf-8")
        spec = _plan_spec(arm, model, endpoint, limits, price_row, limits.usd_per_task if bound is None else bound,
                          key="preflight", spec_text=PREFLIGHT_SPEC, files=(next(iter(PREFLIGHT_TREE)),),
                          visible=(PREFLIGHT_VISIBLE,), snapshot=snapshot)
        try:
            emitted = planemit.emit(spec, workspace)
        except planemit.PlanEmitError as err:
            raise RunnerError(f"the arm cannot emit a plan: {err}") from None
        env = {**agent_env.build(home=Path(scratch) / "home"), "ROKO_CONFIG": str(emitted.config_path),
               **{name: OFFLINE_KEY for name in {spec.api_key_env, *(rung.api_key_env for rung in spec.rungs)}}}
        jail = sandbox.command([], deny=(), network=sandbox.NETWORK_NONE, sockets=[workspace])  # validate needs none
        checked = _roko([*_head(binary, workspace, model), "plan", "validate", "--strict", "--dag",
                         str(workspace / "plans")], workspace, env, VALIDATE_TIMEOUT_S, jail)
    if checked.returncode != 0:
        said = (checked.stdout + checked.stderr).strip()[-500:]
        raise RunnerError(f"{binary} rejects the plan this arm emits: `plan validate --strict --dag` "
                          f"{'timed out' if checked.timed_out else f'exited {checked.returncode}'}: {said}")


def _worst_task_usd(arm: dict, snapshot: ledger.Snapshot, limits: caps.Caps, price_row: dict | None) -> float | None:
    """The most one task can cost under `limits`: the most expensive of a routed arm's rungs (3312, 3313), so
    Roko's own plan budget (`PlanSpec.usd_cap`, below) is never priced off the cheap start rung alone and starved
    once the task escalates; `price_row`'s own worst case, unchanged, for a pinned arm (one models_allow entry)."""
    allowed = arm["arm"]["models_allow"]
    if len(allowed) <= 1:
        return caps.worst_task_usd(limits, price_row)
    worst = [caps.worst_task_usd(limits, snapshot.row(rung_model)) for rung_model in allowed]
    return None if any(one is None for one in worst) else max(worst)


def _plan_spec(arm: dict, model: str, endpoint: provider.Endpoint, limits: caps.Caps, price_row: dict | None,
               usd_cap: float, *, key: str, spec_text: str, files: tuple[str, ...], visible: tuple[str, ...],
               verify_wrapper: str | None = None, snapshot: ledger.Snapshot | None = None) -> planemit.PlanSpec:
    """The plan spec of one task on this arm: `run_task`'s, and `preflight`'s for its stand-in task. A routed arm
    (3312: more than one `models_allow` entry) emits planemit's ladder mode instead of one pinned model, with one
    rung per allowed model, cheapest first, and `model` as the start rung."""
    settings = arm.get("roko", {})
    api_key_env = endpoint.api_key_env or arm.get("providers", {}).get(endpoint.provider, {}).get("api_key_env")
    if not api_key_env:
        raise RunnerError(f"the arm names no api_key_env for {endpoint.provider}")
    allowed = arm["arm"]["models_allow"]
    rungs, start = (), None
    if len(allowed) > 1:
        if snapshot is None:
            raise RunnerError("a routed arm needs the price snapshot to build its ladder")
        rungs = tuple(_rung(arm, rung_model, snapshot, limits) for rung_model in allowed)
        start = model
    return planemit.PlanSpec(
        key=key, spec_text=spec_text, files=files, visible=visible, model=model, provider=endpoint.provider,
        base_url=endpoint.base_url, api_key_env=api_key_env, price_row=price_row, usd_cap=usd_cap,
        provider_kind=settings.get("provider_kind", "openai_compat"),
        context_window=int(settings.get("context_window", 128_000)), max_output=limits.max_output_tokens,
        max_retries=int(settings.get("max_retries", 2)), max_turns=limits.turns_per_attempt,
        tier=settings.get("tier", "focused"), skip_enrichment=bool(settings.get("skip_enrichment", True)),
        verify_timeout_s=int(limits.command_timeout_s), verify_wrapper=verify_wrapper,
        rungs=rungs, start=start, allow=tuple(allowed))


def _rung(arm: dict, model: str, snapshot: ledger.Snapshot, limits: caps.Caps) -> planemit.Rung:
    """One rung of a routed arm's ladder: `model`'s provider endpoint, from the price snapshot and the arm's
    `[providers.*]` tables. `base_url` is whatever the arm dict carries for that provider, the metering proxy's own
    URL in a proxied run (3311: `vb.py` rewrites it there before the runner ever sees the arm)."""
    row = snapshot.row(model)
    rung_provider = row["provider"] if row else None
    if rung_provider is None:
        raise RunnerError(f"{model} has no row in {snapshot.id}, so the ladder cannot price it")
    table = arm.get("providers", {}).get(rung_provider)
    if not table:
        raise RunnerError(f"the arm has no [providers.{rung_provider}] endpoint for {model}")
    api_key_env = table.get("api_key_env")
    if not api_key_env:
        raise RunnerError(f"the arm names no api_key_env for {rung_provider}")
    settings = arm.get("roko", {})
    return planemit.Rung(name=model, model=model, provider=rung_provider, base_url=table["base_url"],
                         api_key_env=api_key_env, price_row=row,
                         provider_kind=settings.get("provider_kind", "openai_compat"),
                         context_window=int(settings.get("context_window", 128_000)),
                         max_output=limits.max_output_tokens)


def _head(binary: Path, workspace: Path, model: str) -> list[str]:
    """Roko's command line up to its subcommand, pinned to `model`."""
    return [str(binary), "--repo", str(workspace), "--model", model, "--no-serve", "--color", "never"]


def binary_path(arm: dict) -> Path:
    """The roko binary the arm names (`[roko] binary`, relative to the repository root), which must be executable."""
    raw = Path(arm.get("roko", {}).get("binary") or DEFAULT_BINARY).expanduser()
    path = raw if raw.is_absolute() else layout.REPO_ROOT / raw
    if not (path.is_file() and os.access(path, os.X_OK)):
        raise RunnerError(f"no executable roko binary at {path}; build it or set [roko] binary in the arm file")
    return path


def read_evidence(workspace: Path, slug: str, *, proxy_rows: list[dict] | None = None) -> Evidence:
    """This plan's records from a Roko workspace's `.roko/`, read without following symlinks."""
    root = Path(workspace) / ".roko"
    unreadable: list[str] = []

    def rows(relpath: str) -> list[dict]:
        found, bad = _jsonl(root / relpath)
        unreadable.extend(f"{relpath}:{what}" for what in bad)
        return found

    def this_plan(row: dict) -> bool:
        return (row.get("extra") or {}).get("plan_id", row.get("plan_id")) == slug

    episodes = _attempt_order([row for row in rows("episodes.jsonl")
                               if this_plan(row) and row.get("task_id") == planemit.TASK_ID])
    verdicts = []
    runs = root / "runs"
    if runs.is_dir() and not runs.is_symlink():
        for path in sorted(runs.glob("*/attempts.jsonl")):
            verdicts += [row for row in rows(path.relative_to(root).as_posix())
                         if row.get("schema_version") == VERDICT_SCHEMA and row.get("plan_id") == slug]
    checkpoint = None
    path = root / "state" / "graph" / slug / "checkpoint.json"
    if path.is_file() and not path.is_symlink():
        try:
            checkpoint = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            unreadable.append("checkpoint.json")
    return Evidence(episodes=episodes, cost_rows=[row for row in rows("learn/costs.jsonl") if this_plan(row)],
                    efficiency=rows("learn/efficiency.jsonl"), verdicts=verdicts, checkpoint=checkpoint,
                    proxy_rows=proxy_rows, unreadable=unreadable)


def settle(evidence: Evidence, *, chain_key: str, model: str, provider: str, snapshot: ledger.Snapshot,
           reserved_usd: float, max_attempts: int, roko_build: str | None = None, swap: str | None = None,
           rungs: tuple[planemit.Rung, ...] = ()) -> tuple[list[RokoAttempt], list[str]]:
    """One attempt per episode, numbered by its attempt key, each checked against the pin and priced from a meter; the
    problems found. `swap` is the model a declared `model_swap` has the proxy serve in place of the pin: a record
    that says it served is marked `model_swapped`, not a mismatch. `rungs` (3312) is the arm's ladder, cheapest
    first, when it is a routed arm: an attempt may then dispatch any rung at or above the highest rung dispatched
    before it (an escalation, never a step down), each checked and priced against its own rung's model and
    provider; everything else (a failover, a foreign model or a lower rung) is still a mismatch, exactly as a
    foreign model is for a pinned arm."""
    rung_provider = {rung.model: rung.provider for rung in rungs}
    rung_index = {rung.model: position for position, rung in enumerate(rungs)}
    ordinals = [_ordinal((episode.get("extra") or {}).get("attempt_key")) for episode in evidence.episodes]
    keyed = bool(ordinals) and all(ordinals) and len(set(ordinals)) == len(ordinals)
    attempts = []
    for position, episode in enumerate(evidence.episodes, 1):
        extra = episode.get("extra") or {}
        number = ordinals[position - 1] if keyed else position
        verdict = _gate_verdict(episode)
        dispatched, before = episode.get("model") or None, attempts[-1].model_dispatched if attempts else None
        turns = None if extra.get("turns_unknown") is True or not _count(episode.get("turns")) else episode["turns"]
        # A keyed episode (bug-62e3f4's Roko) names its helper calls whenever it made some; an older one never does.
        helpers = extra["helper_calls"] if _count(extra.get("helper_calls")) else 0 if "attempt_key" in extra else None
        roko_calls = None if turns is None or helpers is None else turns + helpers
        attempts.append(RokoAttempt(
            number=number, attempt_key=f"{chain_key}:{number}", model_requested=model,
            provider=str(episode.get("backend") or provider), reserved_usd=reserved_usd, turns=turns,
            calls=roko_calls or 0, calls_known=roko_calls is not None, usage_unknown=True,
            ended_by="gate_passed" if verdict == "passed" else verdict or "gate_failed", model_dispatched=dispatched,
            gate_verdict=verdict, roko_usage=_roko_usage(episode), roko_build=roko_build,
            task_id=str(episode.get("task_id") or planemit.TASK_ID),
            cost_class="execute" if position == 1 else "escalate" if dispatched and before and dispatched != before
            else "retry", helper_calls=helpers, roko_calls=roko_calls, **_episode_span(episode)))
    by_number = {attempt.number: attempt for attempt in attempts}
    problems: list[str] = []

    def flag(kind: str, detail: str, number: int | None = None) -> None:
        problems.append(f"{kind}: {detail}")
        attempt = by_number.get(number)
        if attempt is not None:
            attempt.checks += [kind] if kind not in attempt.checks else []
            attempt.ended_by = kind

    def where(number: int | None) -> str:
        return f"attempt {number}" if number else "a run record"

    def expect(found: object, wanted: str, what: str, number: int | None) -> None:
        if found not in (None, "") and found != wanted:
            flag("model_mismatch", f"{where(number)}: {what} is {found!r}, not {wanted!r}", number)

    def expected_model(number: int | None) -> str:
        """What a record other than the episode itself is checked against: a routed attempt's own rung (3312),
        once its episode has named one of the arm's rungs, else the task's requested model. Degenerates to `model`
        for a pinned arm (no rungs), so this is a no-op there."""
        if not rungs:
            return model
        attempt = by_number.get(number)
        dispatched = attempt.model_dispatched if attempt else None
        return dispatched if dispatched in rung_index else model

    def expected_provider(number: int | None) -> str:
        return rung_provider.get(expected_model(number), provider) if rungs else provider

    def is_swap(reported: object, number: int | None) -> bool:
        """Whether `reported` is the declared swap's model, and not the one expected; marks the attempt it served."""
        return bool(swap and isinstance(reported, str) and records.same_model(swap, reported)
                    and not records.same_model(expected_model(number), reported))

    def served(row: dict, what: str, number: int | None) -> None:
        """The served-model fields a record carries (bug-31438d, bug-35379d): the model the provider reported, every
        model it named, Roko's own mismatch mark, and the planned model a failover replaced. A null report is no
        evidence either way, and the declared swap's model is a swap, not a mismatch."""
        wanted = expected_model(number)
        named = row.get("models_reported") if isinstance(row.get("models_reported"), list) else []
        for reported in dict.fromkeys([row.get("model_reported"), *named]):
            if is_swap(reported, number):
                if number in by_number:
                    by_number[number].model_swapped = True
            elif isinstance(reported, str) and reported and not records.same_model(wanted, reported):
                flag("model_mismatch", f"{where(number)}: {what} says the provider served {reported!r}, not "
                                       f"{wanted!r}", number)
        if row.get("model_mismatch") is True and not is_swap(row.get("model_reported"), number):
            flag("model_mismatch", f"{where(number)}: {what} marks the served model as another than the one "
                                   "launched", number)
        replaced = row.get("substituted_from") or row.get("failover_chain")
        if replaced:  # a failover is still a mismatch, even to another of the arm's own rungs (3312)
            flag("model_mismatch", f"{where(number)}: {what} records a failover from {replaced!r}", number)

    if evidence.episodes and not keyed and any(ordinals):
        flag("model_unverified", "Roko's episodes name attempt keys for only some attempts, or name one twice")
    climbed = -1  # the highest rung index dispatched so far (3312); -1 until the first valid one
    for attempt, episode in zip(attempts, evidence.episodes):
        if rungs:
            index = rung_index.get(attempt.model_dispatched)
            if attempt.model_dispatched not in (None, "") and index is None:
                flag("model_mismatch", f"{where(attempt.number)}: the dispatched model "
                     f"{attempt.model_dispatched!r} is not a rung of this arm", attempt.number)
            elif index is not None:
                if index < climbed:
                    flag("model_mismatch", f"{where(attempt.number)}: the dispatched model stepped down the "
                         "ladder, which only a failover does", attempt.number)
                else:
                    climbed = index
        else:
            expect(attempt.model_dispatched, model, "the dispatched model", attempt.number)
        expect(attempt.provider, expected_provider(attempt.number), "the provider", attempt.number)
        served(episode.get("extra") or {}, "its episode", attempt.number)
        if attempt.model_dispatched is None:
            flag("model_unverified", f"attempt {attempt.number}: its episode names no model", attempt.number)
    dispatch_rows = 0
    for row in evidence.cost_rows:
        helper = row.get("role") == HELPER_ROLE
        dispatch_rows += not helper
        number = _ordinal(row.get("attempt_key")) or (None if helper else dispatch_rows)  # unkeyed: one per attempt
        what = "a helper call's cost row" if helper else "the cost row"
        expect(row.get("model"), expected_model(number), f"{what}'s model", number)
        expect(row.get("provider"), expected_provider(number), f"{what}'s provider", number)
        served(row, what, number)
    for row in evidence.efficiency:
        match = ATTEMPT_ID.search(str(row.get("attempt_id") or ""))  # an older Roko's `.../a<n-1>/...`
        number = _ordinal(row.get("attempt_key")) or _ordinal(row.get("attempt_id")) or (
            int(match[1]) + 1 if match else None)
        what = "a helper call's efficiency row" if row.get("role") == HELPER_ROLE else "an efficiency row"
        for key in ("model", "resolved_model"):
            expect(row.get(key), expected_model(number), f"{what}'s {key}", number)
        expect(row.get("provider") or row.get("backend"), expected_provider(number), f"{what}'s provider", number)
        served(row, what, number)
    for verdict in evidence.verdicts:
        number = _ordinal(verdict.get("attempt_key")) or (
            verdict.get("attempt") if isinstance(verdict.get("attempt"), int) else None)
        executed = verdict.get("executed") or {}
        for key in ("model_requested", "model_dispatched"):
            expect(executed.get(key), expected_model(number), f"S01's executed {key}", number)
        expect(executed.get("provider"), expected_provider(number), "S01's executed provider", number)
        served(executed, "S01's verdict", number)
        if number in by_number:
            _meter_from_verdict(by_number[number], verdict)
    if evidence.unreadable:
        flag("model_unverified", f"unreadable Roko records: {', '.join(evidence.unreadable[:5])}")
    if not attempts:
        flag("model_unverified", "Roko recorded no attempt")
    if len(attempts) > max_attempts:
        flag("extra_attempts", f"Roko made {len(attempts)} attempts; the plan allows {max_attempts}")
    if evidence.proxy_rows is not None:
        _meter_from_proxy(attempts, evidence, model, flag, swap)
    for attempt in attempts:
        if attempt.cost is None:  # the proxy has already priced an attempt that billed nothing
            attempt.cost = ledger.price(attempt.reported_usage(), snapshot.row(attempt.model_reported))
            if attempt.usage_estimated and attempt.cost.source == "provider_usage":
                attempt.cost = replace(attempt.cost, source="estimated")
    return attempts, problems


def _meter_from_verdict(attempt: RokoAttempt, verdict: dict) -> None:
    """S01's verdict meters the attempt when it reports the served model and every usage class, and times the whole
    attempt, its gate included, when it has the attempt's start and settlement."""
    timing = verdict.get("timing") or {}
    start, end = (_from_unix_ms(timing.get(key)) for key in ("attempt_started_at", "settled_at"))
    if start is not None and end is not None and start <= end:
        attempt.started_at, attempt.finished_at = _iso(start), _iso(end)
    reported = (verdict.get("executed") or {}).get("model_reported")
    usage = verdict.get("usage") or {}
    classes = ("tokens_in", "tokens_out", "tokens_cache_read")
    if reported and all(isinstance(usage.get(name), int) for name in classes):
        attempt.model_reported = reported
        attempt.usage = {**{name: usage[name] for name in classes},
                         "tokens_reasoning": usage.get("tokens_reasoning") or 0,
                         **{name: usage[name] for name in ("tokens_cache_write_5m", "tokens_cache_write_1h")
                            if isinstance(usage.get(name), int)}}
        attempt.usage_unknown = False
        attempt.usage_estimated = (verdict.get("cost") or {}).get("source") == "estimated"


def _meter_from_proxy(attempts: list[RokoAttempt], evidence: Evidence, model: str, flag,
                      swap: str | None = None) -> None:
    """Assign the proxy's requests to attempts by time and meter each attempt from them (module docstring).

    Times compare to the microsecond when every stamp has one. With a whole-second stamp anywhere, they compare in
    whole seconds, so an attempt that ended in the same second as the one before it cannot be told apart from it: its
    requests count toward the earlier attempt, and its empty window is not flagged.

    Each attempt's requests are checked against its own `model_dispatched` (3312: a routed attempt's rung, by then
    already held to the arm's rungs), falling back to `model` when it is unknown.
    """
    rows = sorted(evidence.proxy_rows or [], key=lambda row: row.get("ordinal") if isinstance(row.get("ordinal"), int)
                  else 0)
    stamps = [episode.get("completed_at") or episode.get("timestamp") for episode in evidence.episodes]
    clock = _instant if all(_subsecond(stamp) for stamp in [*stamps, *(row.get("ts") for row in rows)]) else _second
    ends = [clock(stamp) for stamp in stamps]
    windows: list[list[dict]] = [[] for _ in attempts]
    for row in rows:
        when = clock(row.get("ts"))
        index = next((i for i, end in enumerate(ends) if when is not None and end is not None and when <= end),
                     len(attempts) - 1)
        if 0 <= index < len(windows):
            windows[index].append(row)
    for position, (attempt, window) in enumerate(zip(attempts, windows)):
        # A routed attempt's own dispatched rung (3312), already held to the arm's rungs above; `model` otherwise,
        # exactly as before.
        wanted = attempt.model_dispatched or model
        for row in window:
            reported, sent = row.get("model_reported"), row.get("model_swap")
            if row.get("model_requested") and row["model_requested"] != wanted:
                flag("model_mismatch", f"attempt {attempt.number}: the proxy saw model_requested "
                                       f"{row['model_requested']!r}, not {wanted!r}", attempt.number)
            if sent and sent != swap:
                flag("model_mismatch", f"attempt {attempt.number}: the proxy sent {sent!r} in place of {wanted!r}, "
                                       "which no model_swap declared for this task", attempt.number)
            if not reported or records.same_model(wanted, reported):
                continue
            if swap and records.same_model(swap, reported):
                attempt.model_swapped = True
            else:
                flag("model_mismatch", f"attempt {attempt.number}: the proxy saw model_reported {reported!r}, not "
                                       f"{wanted!r}", attempt.number)
        if not window:
            if not rows or position == 0 or ends[position] is None or ends[position] != ends[position - 1]:
                flag("no_proxy_traffic", f"attempt {attempt.number}: the metering proxy saw no request",
                     attempt.number)
            else:
                attempt.queue_wait_s = 0.0  # its requests, and their waits, count toward the attempt before
            continue
        attempt.queue_wait_s = _rate_limit_waits(window, rows)
        attempt.usage_estimated = False  # the proxy saw what the provider billed
        billed = [row for row in window if row.get("usage_source") != "none"]  # none: a fault, refusal or error
        served = {row.get("model_reported") for row in billed}
        usages = [_proxy_usage(row.get("usage")) for row in billed]
        attempt.calls, attempt.calls_known = len(window), True
        if not attempt.checks and any(row.get("refused") == "attempt_input_cap" for row in window):
            attempt.ended_by = "attempt_input_tokens"  # the proxy refused its calls at the per-attempt cap
        attempt.model_reported = served.pop() if len(served) == 1 else None
        attempt.usage_unknown = any(usage is None for usage in usages)
        if not attempt.usage_unknown:
            total = ledger.vb_usage(0, 0)
            for usage in usages:
                total = ledger.add_usage(total, usage)
            attempt.usage = total
        if not billed:  # every request of the attempt was refused, faulted or failed: none was billed
            attempt.cost = ledger.Cost(0.0, 0.0, "provider_usage")


def _proxy_usage(raw: object) -> dict | None:
    """A proxy row's usage, which is in run-record shape; null (no usage came back) stays unknown, never 0."""
    if not isinstance(raw, dict) or not all(isinstance(raw.get(name), int) for name in ("tokens_in", "tokens_out")):
        return None
    usage = {"tokens_in": raw["tokens_in"], "tokens_out": raw["tokens_out"],
             "tokens_cache_read": raw.get("tokens_cache_read") or 0,
             "tokens_reasoning": raw.get("tokens_reasoning") or 0}
    usage.update({name: raw[name] for name in ("tokens_cache_write_5m", "tokens_cache_write_1h")
                  if isinstance(raw.get(name), int)})
    return usage


def _rate_limit_waits(window: list[dict], rows: list[dict]) -> float:
    """The seconds the task waited after each rate-limited request of `window` (HTTP 429) until its next request,
    at the proxy's whole-second resolution less the 429's own time; `rows` are all the task's requests in order."""
    following = {id(row): after for row, after in zip(rows, rows[1:])}
    waits = []
    for row in window:
        after = following.get(id(row))
        start, end = _second(row.get("ts")), _second(after.get("ts")) if after else None
        if row.get("status") == 429 and start is not None and end is not None:
            elapsed = row.get("elapsed_ms") if isinstance(row.get("elapsed_ms"), (int, float)) else 0
            waits.append(max(0.0, end - start - elapsed / 1000))
    return round(sum(waits), 3)


def _episode_span(episode: dict) -> dict[str, str | None]:
    """The attempt's dispatch window from its episode: it ended at `completed_at` and lasted `duration_secs`. (The
    Graph path's own `started_at` is only when the episode was written.)"""
    end = _instant(episode.get("completed_at") or episode.get("timestamp"))
    seconds = episode.get("duration_secs")
    known = isinstance(seconds, (int, float)) and not isinstance(seconds, bool) and seconds >= 0
    return {"started_at": _iso(end - dt.timedelta(seconds=seconds)) if end and known else None,
            "finished_at": _iso(end) if end else None}


def _gate_verdict(episode: dict) -> str | None:
    """The attempt's verdict tag: S01's outcome from its episode, None for one that is not a tag (a failed gate);
    an older Roko's episode, which has no outcome, by whether it succeeded."""
    outcome = (episode.get("extra") or {}).get("outcome")
    if isinstance(outcome, str):
        return outcome if outcome in VERDICT_TAGS else None
    return "passed" if episode.get("success") is True else None


def _status(ran: Ran, evidence: Evidence, problems: list[str]) -> tuple[str, str]:
    """A substituted model outranks a timeout: that run is excluded, not counted as censoring."""
    invalid = [problem for problem in problems if not problem.startswith("model_unverified")]
    if invalid or (problems and not ran.timed_out):
        said = (ran.stderr.strip().splitlines() or [""])[-1] if not evidence.episodes else ""
        detail = f" (roko exited {ran.returncode}: {said[:300]})" if said else ""
        return "infra_error", ("; ".join(invalid or problems)[:500] + detail)
    if ran.timed_out:
        return "timeout", "wallclock"
    checkpoint = evidence.checkpoint or {}
    state = checkpoint.get("status")
    verdicts = ((checkpoint.get("extensions") or {}).get("roko.gate.verdict@1") or {}).get("value") or {}
    verdict = (verdicts.get("verdicts") or {}).get(planemit.TASK_ID)
    if state == "succeeded" and verdict == "passed":
        return "completed", "gate_passed"
    if state == "succeeded" and verdict == "already_satisfied":  # not Roko's completion (module docstring, Status)
        return "failed", "already_satisfied"
    if state == "failed":
        last = str(evidence.episodes[-1].get("failure_reason") or "") if evidence.episodes else ""
        return "failed", "gate_failed" if last.startswith("verify:") else f"roko: {last[:200] or 'plan failed'}"
    return "infra_error", f"roko exited {ran.returncode} with the plan {state or 'unrecorded'}"


def _wrapper_command(ctx: harness.TaskContext) -> str | None:
    """How the plan names the visible-verify wrapper: by name when the agent's PATH finds it, so no path of the host
    enters Roko's prompt (the verify command is rendered into it, A4); else by its path."""
    if ctx.verify_wrapper is None:
        return None
    wrapper = Path(ctx.verify_wrapper)
    found = shutil.which(wrapper.name, path=ctx.agent_env.get("PATH"))
    return wrapper.name if found and Path(found).absolute() == wrapper.absolute() else str(wrapper)


def _roko_env(ctx: harness.TaskContext, api_key_envs: str | Iterable[str], config_path: Path) -> dict[str, str]:
    """The agent environment, ROKO_CONFIG and a placeholder key for every env var the emitted roko.toml names: one
    name, or several (one per provider for a routed arm's rungs, 3312, not just the start rung's). A network
    endpoint is refused (module docstring)."""
    if not ctx.endpoint.offline:
        raise RunnerError(f"{ctx.endpoint.provider} is a network provider, which Roko reaches only through the "
                          "metering proxy, the one holder of its key: run it with `vb run` (which proxies every billed "
                          "network run, and any run with --proxy) and a key file (--key-file)")
    names = [api_key_envs] if isinstance(api_key_envs, str) else list(api_key_envs)
    return {**ctx.agent_env, "ROKO_CONFIG": str(config_path), **{name: OFFLINE_KEY for name in names}}


def _build(binary: Path, env: dict[str, str], pinned: str | None, transcript: list[dict],
           jail: list[str]) -> str | None:
    """The binary's git sha from `roko --version`, run in its sandbox (`jail`, a `sandbox.command` prefix); a mismatch
    with the arm's `[roko] build` stops the task."""
    version = subprocess.run([*jail, str(binary), "--version"], env=env, capture_output=True, text=True, timeout=30,
                             check=False, stdin=subprocess.DEVNULL)
    match = BUILD.search(version.stdout)
    transcript.append({"event": "version", "stdout": version.stdout.strip()[:300]})
    build = match[1] if match else None
    if pinned and not (build and (build.startswith(pinned) or pinned.startswith(build))):
        raise RunnerError(f"the arm pins roko build {pinned}, but {binary} is {build or 'unknown'}")
    return build


def _roko(argv: list[str], cwd: Path, env: dict[str, str], timeout_s: float, jail: list[str]) -> Ran:
    """Run Roko in its sandbox (`jail`, a `sandbox.command` prefix; the record keeps `argv` without it) and in its
    own session, and kill the whole session when it ends or times out."""
    start = time.monotonic()
    process = subprocess.Popen([*jail, *argv], cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, start_new_session=True)
    timed_out = False
    try:
        out, err = process.communicate(timeout=max(timeout_s, 1.0))
    except subprocess.TimeoutExpired:
        timed_out = True
        _kill_session(process.pid)
        try:
            out, err = process.communicate(timeout=30)
        except subprocess.TimeoutExpired:  # a process outside the session still holds a pipe
            process.kill()
            out, err = b"", b""
    finally:
        _kill_session(process.pid)
    return Ran(argv=argv, returncode=None if timed_out else process.returncode,
               stdout=out.decode("utf-8", "replace"), stderr=err.decode("utf-8", "replace"), timed_out=timed_out,
               seconds=time.monotonic() - start)


def _proxy_rows(ctx: harness.TaskContext) -> list[dict] | None:
    """This task's rows of the run's proxy log, or None when no proxy ran."""
    path = ctx.ledger.path.parent / "proxy.jsonl"
    if not path.is_file():
        return None
    found, _ = _jsonl(path)
    return [row for row in found if row.get("task") == ctx.key]


def _save_evidence(root: Path, dest: Path, transcript: list[dict]) -> None:
    """Copy Roko's records out of the workdir before it is cleaned; symlinks are copied as links."""
    try:
        if not root.is_dir() or root.is_symlink():
            return
        dest.mkdir(mode=0o700, parents=True, exist_ok=True)
        for relpath in ("episodes.jsonl", "learn/costs.jsonl", "learn/efficiency.jsonl", "learn/run-metrics.jsonl",
                        "state/graph", "runs", *sorted(p.name for p in root.glob("roko.log*"))):
            source, target = root / relpath, dest / relpath
            target.parent.mkdir(parents=True, exist_ok=True)
            if source.is_dir() and not source.is_symlink():
                shutil.copytree(source, target, symlinks=True, dirs_exist_ok=True)
            elif source.is_symlink() or (source.is_file() and source.stat().st_size <= EVIDENCE_MAX_BYTES):
                shutil.copy2(source, target, follow_symlinks=False)
    except OSError as err:
        transcript.append({"event": "evidence_copy_failed", "error": str(err)[:300]})


def _jsonl(path: Path) -> tuple[list[dict], list[str]]:
    """The object rows of a JSONL file, and what could not be read (a symlink, or bad lines by number)."""
    if path.is_symlink():
        return [], ["symlink"]
    if not path.is_file():
        return [], []
    rows, bad = [], []
    for number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except ValueError:
            row = None
        if isinstance(row, dict):
            rows.append(row)
        else:
            bad.append(f"line {number}")
    return rows, bad


def _ordinal(key: object) -> int | None:
    """The attempt number of an S01 attempt key, `<run>:<plan>:<task>:<n>` with n from 1, or of a helper call's row id
    under it (`<key>/helper-<i>`); None for anything else."""
    if not isinstance(key, str):
        return None
    parts = key.split("/", 1)[0].split(":")
    if len(parts) != 4 or not all(parts[:3]) or not parts[3].isdigit() or int(parts[3]) < 1:
        return None
    return int(parts[3])


def _attempt_order(episodes: list[dict]) -> list[dict]:
    """Episodes by their attempt keys' numbers when each names its own, else in file order."""
    ordinals = [_ordinal((episode.get("extra") or {}).get("attempt_key")) for episode in episodes]
    if not all(ordinals) or len(set(ordinals)) != len(ordinals):
        return episodes
    return [episode for _, episode in sorted(zip(ordinals, episodes), key=lambda pair: pair[0])]


def _count(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def _roko_usage(episode: dict) -> dict | None:
    usage = episode.get("usage")
    if not isinstance(usage, dict):
        return None
    return {name: usage.get(name) for name in ("input_tokens", "output_tokens", "cache_read_tokens",
                                               "cache_write_tokens")}


def _attempt_event(attempt: RokoAttempt, episode: dict) -> dict:
    return {"event": "attempt", "attempt": attempt.number, "model_dispatched": attempt.model_dispatched,
            "provider": attempt.provider, "success": episode.get("success"), "checks": attempt.checks,
            "turns": attempt.turns, "helper_calls": attempt.helper_calls, "roko_calls": attempt.roko_calls,
            "calls": attempt.calls if attempt.calls_known else None,
            "failure_reason": _clip(str(episode.get("failure_reason") or ""))}


def _second(value: object) -> int | None:
    """An ISO 8601 UTC time as whole Unix seconds, the proxy's resolution."""
    if not isinstance(value, str) or not value:
        return None
    try:
        return int(dt.datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp())
    except ValueError:
        return None


def _subsecond(value: object) -> bool:
    """Whether an ISO 8601 time names a fraction of a second."""
    return isinstance(value, str) and "." in value.partition("T")[2]


def _instant(value: object) -> dt.datetime | None:
    """An ISO 8601 time as an aware datetime (UTC when it names no zone); None when missing or unreadable."""
    if not isinstance(value, str) or not value:
        return None
    try:
        moment = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    return moment if moment.tzinfo else moment.replace(tzinfo=dt.UTC)


def _from_unix_ms(value: object) -> dt.datetime | None:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        return None
    return dt.datetime.fromtimestamp(value / 1000, dt.UTC)


def _iso(moment: dt.datetime) -> str:
    return moment.astimezone(dt.UTC).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def _left(ctx: harness.TaskContext, clock: float) -> float:
    left = ctx.caps.wallclock_s - (time.monotonic() - clock)
    if left <= 0:
        raise RunnerError("no wall-clock time left for the task")
    return left


def _reserve(book: ledger.Ledger, keys: list[str], usd: float) -> None:
    """Reserve `usd` for each attempt key, all or none: raises BudgetError, holding nothing, if one does not fit."""
    held: list[str] = []
    try:
        for key in keys:
            book.reserve(key, usd)
            held.append(key)
    except ledger.BudgetError:
        for key in held:
            book.release(key)
        raise


def _clip(text: str) -> str:
    if len(text) <= OUTPUT_CHARS:
        return text
    half = OUTPUT_CHARS // 2
    return f"{text[:half]}\n[... {len(text) - 2 * half} characters omitted ...]\n{text[-half:]}"


def _kill_session(pid: int) -> None:
    try:
        os.killpg(pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass
