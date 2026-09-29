"""The Roko arm's runner, harness `roko` (S08 §4.9 `roko_fixed`, T11): one task through `roko plan run` on one
pinned model, which is checked on every attempt.

For each (task, seed), `run_task`:

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
roko.toml (`plan run` ignores `--config`) and the provider key that roko.toml names. The agent environment gives it
a fresh HOME, so no `~/.roko/.env` loads and no learned state crosses seeds, along with PATH, TMPDIR, locale and a
git identity. Nothing else of the driver's environment reaches Roko. Roko scrubs the key from its gates' and tools'
children (bug-7d7200). A loopback `--provider-url` (a stub, later the metering proxy) gets a placeholder key,
because Roko refuses a provider without one. At 33e107da1, Roko needed nothing under HOME and wrote nothing there.

**The model check** (W10 rec 5, bug-35379d). Every record Roko writes must name the pinned model and the arm's
provider:
- each episode (`.roko/episodes.jsonl`, one per dispatch, with the model Roko dispatched);
- the cost rows (`.roko/learn/costs.jsonl`);
- every model-call and efficiency row (`.roko/learn/efficiency.jsonl`), auxiliary calls included;
- S01's `roko.verdict/1` lines (`.roko/runs/*/attempts.jsonl`, gap-528762), once Roko writes them. These carry the
  executed and provider-reported model and the failover chain.

The failure kinds:
- `model_mismatch`: a record names another model or provider.
- `model_unverified`: an attempt has no episode, or Roko left no records or unreadable ones.
- `extra_attempts`: Roko made more attempts than the plan allows.

Any of them makes the task `infra_error`. The report excludes and counts it, and the census never labels it a
success. Roko's records live in the agent's workdir, so a hostile agent could edit them. Only the proxy's log is out
of the agent's reach.

**What Roko does not record.** Checked at 33e107da1 against a loopback fake:
- the model the provider reports: the fake answered glm-4.7, and every record still said gpt-oss-120b;
- the three auxiliary calls it makes after each failed gate (quality rating, error diagnosis, reflection). Episodes
  and the Graph `costs.json` leave these out, and `efficiency.jsonl` missed the last one.

So `model_reported` is null and the cost is unknown (null) unless S01 verdicts or the metering proxy supply them.
Roko's own token counts stay in each attempt's `roko_usage`, for diagnosis only. Roko's USD is never used.

**The proxy** (gap-e003ec, `faultproxy.py`). Route Roko through it with the proxy's base URL for the provider as
`--provider-url`. Roko then gets a placeholder key, and the proxy sends the real one. When `<run_dir>/proxy.jsonl`
exists, its rows for this task are the meter. The driver must set the proxy's active task to the task key
(`proxy.configure(task=key)`); rows without that key never match, so the attempts fail as `no_proxy_traffic`.
- Requests are assigned to attempts by `ts`, in whole seconds, in `ordinal` order. An attempt owns the requests up
  to its episode, so its auxiliary calls, made after that, usually count toward the next attempt; the last attempt
  owns the rest. An attempt that ended in the same second as the one before cannot be told apart from it. Task
  totals are exact either way.
- Every request's `model_requested` and `model_reported` must be the pin.
- An attempt's usage is the sum of its billed requests (`usage_source` other than `none`), in run-record shape. It
  is priced from the snapshot by the model the provider reported. A billed request without usage leaves the
  attempt's cost unknown, never 0.
- An attempt whose window saw no request is `no_proxy_traffic`, which makes the task `infra_error`.

**Status.**
- `completed`: the Graph checkpoint says the plan succeeded with a `passed` gate verdict. This is Roko's reported
  pass, so a census VS = 0 counts as a false green.
- `failed`: the gates failed through every retry, or Roko stopped the plan itself.
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
    read_evidence(workspace: Path, slug: str, *, proxy_rows: list[dict] | None = None) -> Evidence
    settle(evidence, *, chain_key, model, provider, snapshot, reserved_usd, max_attempts, roko_build=None)
        -> (list[RokoAttempt], list[str])
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
import time
from dataclasses import dataclass, field
from pathlib import Path

import archive
import caps
import harness
import layout
import ledger
import planemit
from common import repo

PROMPT_VERSION = planemit.TEMPLATE_VERSION  # Roko's own prompt is in its binary, recorded per attempt as roko_build
PROMPT_SHA256 = planemit.TEMPLATE_SHA256
DEFAULT_BINARY = "target/debug/roko"  # relative to the repository root, like the arm file's [roko] binary
OFFLINE_KEY = "vb-offline-placeholder"
VALIDATE_TIMEOUT_S = 120.0
OUTPUT_CHARS = 20_000  # of each of Roko's stdout and stderr kept in the transcript, head and tail
EVIDENCE_MAX_BYTES = 50_000_000
BUILD = re.compile(r"\bgit ([0-9a-f]{7,40})\b")
ATTEMPT_ID = re.compile(r"/a(\d+)(?:/|$)")
VERDICT_SCHEMA = "roko.verdict/1"


class RunnerError(RuntimeError):
    """The arm cannot run this task: the binary, the plan or the environment is wrong."""


@dataclass
class RokoAttempt(harness.Attempt):
    model_dispatched: str | None = None  # the model Roko's records say it dispatched
    gate_verdict: str | None = None  # S01's tag: "passed", or None when the gate failed
    roko_usage: dict | None = None  # Roko's own token counts, for diagnosis only: never priced
    roko_build: str | None = None
    checks: list[str] = field(default_factory=list)  # failed check kinds
    calls_known: bool = False  # only the proxy counts every call
    task_id: str | None = None  # the plan task, from the episode
    cost_class: str | None = None  # S09 §4.9: execute, then retry, or escalate on another model
    started_at: str | None = None  # the attempt's busy time (module docstring, "Process measures")
    finished_at: str | None = None
    queue_wait_s: float | None = None  # rate-limit waits in its proxy window; None without the proxy

    def as_record(self) -> dict:
        record = super().as_record()
        record.update(model_dispatched=self.model_dispatched, gate_verdict=self.gate_verdict,
                      roko_usage=self.roko_usage, roko_build=self.roko_build, checks=list(self.checks),
                      task_id=self.task_id, cost_class=self.cost_class, started_at=self.started_at,
                      finished_at=self.finished_at, queue_wait_s=self.queue_wait_s)
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
    bound = caps.worst_task_usd(ctx.caps, ctx.price_row)
    task_bound = ctx.caps.usd_per_task if bound is None else bound
    transcript: list[dict] = []
    attempts: list[RokoAttempt] = []
    emitted = build = None
    status, reason, dispatched = "infra_error", "", False
    reserved: list[str] = []
    try:
        binary = binary_path(ctx.arm)
        api_key_env = ctx.endpoint.api_key_env or ctx.arm.get("providers", {}).get(
            ctx.endpoint.provider, {}).get("api_key_env")
        if not api_key_env:
            raise RunnerError(f"the arm names no api_key_env for {ctx.endpoint.provider}")
        emitted = planemit.emit(planemit.PlanSpec(
            key=ctx.key, spec_text=ctx.spec_text, files=ctx.files_in_scope, visible=ctx.visible_verify,
            model=ctx.model, provider=ctx.endpoint.provider, base_url=ctx.endpoint.base_url, api_key_env=api_key_env,
            price_row=ctx.price_row, usd_cap=task_bound, provider_kind=settings.get("provider_kind", "openai_compat"),
            context_window=int(settings.get("context_window", 128_000)), max_output=ctx.caps.max_output_tokens,
            max_retries=max_retries, max_turns=ctx.caps.turns_per_attempt, tier=settings.get("tier", "focused"),
            skip_enrichment=bool(settings.get("skip_enrichment", True)),
            verify_timeout_s=int(ctx.caps.command_timeout_s)), ctx.workdir)
        transcript.append({"event": "emit", "slug": emitted.slug, "tasks_toml": emitted.tasks_text,
                           "roko_toml": emitted.config_text})
        env = _roko_env(ctx, api_key_env, emitted.config_path)
        build = _build(binary, env, settings.get("build") or None, transcript)
        head = [str(binary), "--repo", str(ctx.workdir), "--model", ctx.model, "--no-serve", "--color", "never"]
        checked = _roko([*head, "plan", "validate", "--strict", "--dag", str(ctx.workdir / "plans")], ctx.workdir,
                        env, min(VALIDATE_TIMEOUT_S, _left(ctx, clock)))
        transcript.append(checked.event("validate"))
        if checked.returncode != 0:
            raise RunnerError(f"the emitted plan failed `plan validate --strict --dag` (exit {checked.returncode}): "
                              f"{(checked.stdout + checked.stderr).strip()[-300:]}")
        keys = [f"{ctx.chain_key}:{number}" for number in range(1, max_retries + 2)]
        _reserve(ctx.ledger, keys, task_bound / (max_retries + 1) if ctx.billed else 0.0)
        reserved = keys
        dispatched = True
        ran = _roko([*head, "plan", "run", str(emitted.plan_dir), "--no-tui"], ctx.workdir, env, _left(ctx, clock))
        transcript.append(ran.event("run"))
        evidence = read_evidence(ctx.workdir, emitted.slug, proxy_rows=_proxy_rows(ctx))
        attempts, problems = settle(evidence, chain_key=ctx.chain_key, model=ctx.model,
                                    provider=ctx.endpoint.provider, snapshot=ctx.snapshot,
                                    reserved_usd=task_bound / (max_retries + 1), max_attempts=max_retries + 1,
                                    roko_build=build)
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
    return harness.TaskOutcome(status=status, reason=reason, attempts=list(attempts), transcript=transcript,
                               started_at=started, finished_at=harness.utc_now())


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

    episodes = [row for row in rows("episodes.jsonl") if this_plan(row) and row.get("task_id") == planemit.TASK_ID]
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
           reserved_usd: float, max_attempts: int, roko_build: str | None = None
           ) -> tuple[list[RokoAttempt], list[str]]:
    """One attempt per episode, each checked against the pin and priced from a meter; the problems found."""
    attempts = []
    for number, episode in enumerate(evidence.episodes, 1):
        passed = episode.get("success") is True
        dispatched, before = episode.get("model") or None, attempts[-1].model_dispatched if attempts else None
        attempts.append(RokoAttempt(
            number=number, attempt_key=f"{chain_key}:{number}", model_requested=model,
            provider=str(episode.get("backend") or provider), reserved_usd=reserved_usd,
            turns=episode.get("turns") if isinstance(episode.get("turns"), int) else None, usage_unknown=True,
            ended_by="gate_passed" if passed else "gate_failed", model_dispatched=dispatched,
            gate_verdict="passed" if passed else None, roko_usage=_roko_usage(episode), roko_build=roko_build,
            task_id=str(episode.get("task_id") or planemit.TASK_ID),
            cost_class="execute" if number == 1 else "escalate" if dispatched and before and dispatched != before
            else "retry", **_episode_span(episode)))
    problems: list[str] = []

    def flag(kind: str, detail: str, number: int | None = None) -> None:
        problems.append(f"{kind}: {detail}")
        if number is not None and 1 <= number <= len(attempts):
            attempt = attempts[number - 1]
            attempt.checks += [kind] if kind not in attempt.checks else []
            attempt.ended_by = kind

    def expect(found: object, wanted: str, what: str, number: int | None) -> None:
        if found not in (None, "") and found != wanted:
            where = f"attempt {number}" if number else "a run record"
            flag("model_mismatch", f"{where}: {what} is {found!r}, not {wanted!r}", number)

    for attempt in attempts:
        expect(attempt.model_dispatched, model, "the dispatched model", attempt.number)
        expect(attempt.provider, provider, "the provider", attempt.number)
        if attempt.model_dispatched is None:
            flag("model_unverified", f"attempt {attempt.number}: its episode names no model", attempt.number)
    for number, row in enumerate(evidence.cost_rows, 1):
        expect(row.get("model"), model, "the cost row's model", number)
        expect(row.get("provider"), provider, "the cost row's provider", number)
    for row in evidence.efficiency:
        match = ATTEMPT_ID.search(str(row.get("attempt_id") or ""))
        number = int(match[1]) + 1 if match else None
        for key in ("model", "resolved_model"):
            expect(row.get(key), model, f"an efficiency row's {key}", number)
        expect(row.get("provider") or row.get("backend"), provider, "an efficiency row's provider", number)
    for verdict in evidence.verdicts:
        number = verdict.get("attempt") if isinstance(verdict.get("attempt"), int) else None
        executed = verdict.get("executed") or {}
        for key in ("model_requested", "model_reported"):
            expect(executed.get(key), model, f"S01's executed {key}", number)
        expect(executed.get("provider"), provider, "S01's executed provider", number)
        if executed.get("failover_chain"):
            flag("model_mismatch", f"S01 records a failover through {executed['failover_chain']}", number)
        if number and number <= len(attempts):
            _meter_from_verdict(attempts[number - 1], verdict)
    if evidence.unreadable:
        flag("model_unverified", f"unreadable Roko records: {', '.join(evidence.unreadable[:5])}")
    if not attempts:
        flag("model_unverified", "Roko recorded no attempt")
    if len(attempts) > max_attempts:
        flag("extra_attempts", f"Roko made {len(attempts)} attempts; the plan allows {max_attempts}")
    if evidence.proxy_rows is not None:
        _meter_from_proxy(attempts, evidence, model, flag)
    for attempt in attempts:
        attempt.cost = ledger.price(attempt.reported_usage(), snapshot.row(attempt.model_reported))
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


def _meter_from_proxy(attempts: list[RokoAttempt], evidence: Evidence, model: str, flag) -> None:
    """Assign the proxy's requests to attempts by time and meter each attempt from them (module docstring).

    The proxy stamps whole seconds, so an attempt that ended in the same second as the one before it cannot be told
    apart from it: its requests count toward the earlier attempt, and its empty window is not flagged.
    """
    rows = sorted(evidence.proxy_rows or [], key=lambda row: row.get("ordinal") if isinstance(row.get("ordinal"), int)
                  else 0)
    ends = [_second(episode.get("completed_at") or episode.get("timestamp")) for episode in evidence.episodes]
    windows: list[list[dict]] = [[] for _ in attempts]
    for row in rows:
        when = _second(row.get("ts"))
        index = next((i for i, end in enumerate(ends) if when is not None and end is not None and when <= end),
                     len(attempts) - 1)
        if 0 <= index < len(windows):
            windows[index].append(row)
    for position, (attempt, window) in enumerate(zip(attempts, windows)):
        for row in window:
            for key in ("model_requested", "model_reported"):
                if row.get(key) and row[key] != model:
                    flag("model_mismatch", f"attempt {attempt.number}: the proxy saw {key} {row[key]!r}, not "
                                           f"{model!r}", attempt.number)
        if not window:
            if not rows or position == 0 or ends[position] is None or ends[position] != ends[position - 1]:
                flag("no_proxy_traffic", f"attempt {attempt.number}: the metering proxy saw no request",
                     attempt.number)
            else:
                attempt.queue_wait_s = 0.0  # its requests, and their waits, count toward the attempt before
            continue
        attempt.queue_wait_s = _rate_limit_waits(window, rows)
        billed = [row for row in window if row.get("usage_source") != "none"]  # none: a fault, refusal or error
        served = {row.get("model_reported") for row in billed}
        usages = [_proxy_usage(row.get("usage")) for row in billed]
        attempt.calls, attempt.calls_known = len(window), True
        attempt.model_reported = served.pop() if len(served) == 1 else None
        attempt.usage_unknown = not billed or any(usage is None for usage in usages)
        if not attempt.usage_unknown:
            total = ledger.vb_usage(0, 0)
            for usage in usages:
                total = ledger.add_usage(total, usage)
            attempt.usage = total


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
    if state == "succeeded" and (verdicts.get("verdicts") or {}).get(planemit.TASK_ID) == "passed":
        return "completed", "gate_passed"
    if state == "failed":
        last = str(evidence.episodes[-1].get("failure_reason") or "") if evidence.episodes else ""
        return "failed", "gate_failed" if last.startswith("verify:") else f"roko: {last[:200] or 'plan failed'}"
    return "infra_error", f"roko exited {ran.returncode} with the plan {state or 'unrecorded'}"


def _roko_env(ctx: harness.TaskContext, api_key_env: str, config_path: Path) -> dict[str, str]:
    key = OFFLINE_KEY if ctx.endpoint.offline else os.environ.get(api_key_env, "")
    if not key:
        raise RunnerError(f"set {api_key_env} for {ctx.endpoint.provider}")
    return {**ctx.agent_env, "ROKO_CONFIG": str(config_path), api_key_env: key}


def _build(binary: Path, env: dict[str, str], pinned: str | None, transcript: list[dict]) -> str | None:
    """The binary's git sha from `roko --version`; a mismatch with the arm's `[roko] build` stops the task."""
    version = subprocess.run([str(binary), "--version"], env=env, capture_output=True, text=True, timeout=30,
                             check=False, stdin=subprocess.DEVNULL)
    match = BUILD.search(version.stdout)
    transcript.append({"event": "version", "stdout": version.stdout.strip()[:300]})
    build = match[1] if match else None
    if pinned and not (build and (build.startswith(pinned) or pinned.startswith(build))):
        raise RunnerError(f"the arm pins roko build {pinned}, but {binary} is {build or 'unknown'}")
    return build


def _roko(argv: list[str], cwd: Path, env: dict[str, str], timeout_s: float) -> Ran:
    """Run Roko in its own session and kill the whole session when it ends or times out."""
    start = time.monotonic()
    process = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
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


def _roko_usage(episode: dict) -> dict | None:
    usage = episode.get("usage")
    if not isinstance(usage, dict):
        return None
    return {name: usage.get(name) for name in ("input_tokens", "output_tokens", "cache_read_tokens",
                                               "cache_write_tokens")}


def _attempt_event(attempt: RokoAttempt, episode: dict) -> dict:
    return {"event": "attempt", "attempt": attempt.number, "model_dispatched": attempt.model_dispatched,
            "provider": attempt.provider, "success": episode.get("success"), "checks": attempt.checks,
            "failure_reason": _clip(str(episode.get("failure_reason") or ""))}


def _second(value: object) -> int | None:
    """An ISO 8601 UTC time as whole Unix seconds, the proxy's resolution."""
    if not isinstance(value, str) or not value:
        return None
    try:
        return int(dt.datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp())
    except ValueError:
        return None


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
