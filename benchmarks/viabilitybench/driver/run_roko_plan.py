"""The roko_plan arm runner, harness `roko`, runner `run_roko_plan` (3319, S09 §4.9/E12): a frontier planner
(`claude -p`, claude-opus-5-5, through 3305's egress allowlist) writes a multi-task plan from a plan-slice
feature's description, under the authoring constraints (no `depends_on_plan`, no `read_files`, scalars before
sub-tables, as `planemit.py`'s module docstring states them); the cheap-model ladder then executes it
(`roko plan run`, the arm's own rungs, up to `[roko] max_parallel` tasks at once); and the whole-plan gate
(`[meta] verify`, gap-60233f, done) checks the integrated tree once every task has passed. One record per feature,
scored with `slicekit.census` (VF); `task.ladder = null`, as `vb.feature/1` requires.

Unlike `planemit.emit` (one task, one plan, never edited after `emit` writes it), the planner itself writes
`plans/<slug>/tasks.toml` here: this module only writes `roko.toml` (the ladder's providers and models, reusing
`planemit`'s own table templates) and the authoring prompt. `ctx.workdir` is the feature's own materialized repo
(`families/plan_slice/slicekit.materialize`), not an F1-F8 one.

Cost classes (S09 §4.9): `plan` is the planner's own session, priced like `run_cli`'s direct arm (U' from its
`result` event's `modelUsage`, source `cli_usage`); `execute` is each plan task's first attempt, `retry` a later
attempt on the same model, `escalate` a later attempt on another (a rung up the ladder, 3312's rule, applied here
without its full cross-record consistency checks: this arm is exploratory, S09 §4.6); `integrate` is the
whole-plan gate Roko itself runs as part of `plan run` (gap-60233f) — a Python check with no model call, so its
cost is always $0, recorded so every successful run shows that it ran.

Reuses, never reimplements: `run_cli.CliConfig`/`build_invocation`/`run_session`/`Meter`/`parse_result` for the
planner's own session (the same machinery the direct fd_claude arm uses); `run_roko.binary_path`/`_build`/`_head`/
`_roko`/`_start_egress`/`network_rule`/`_meter_from_verdict` for the ladder execution's subprocess, sandbox and
per-attempt pricing; `planemit.PROVIDER_TABLE`/`MODEL_TABLE` for roko.toml's provider and model tables.

API:
    run_task(ctx: harness.TaskContext) -> harness.TaskOutcome
"""

from __future__ import annotations

import dataclasses
import json
import shlex
import time
from pathlib import Path

import agent_env
import caps
import harness
import ledger
import planemit
import provider
import run_cli
import run_roko
from common import sandbox

PLANNER_PROMPT = """\
Write a Roko plan for this feature, as the file plans/{slug}/tasks.toml in the repository root. Follow these rules
exactly; Roko's `plan validate --strict --dag` will reject the file otherwise.

Feature description:
<description>
{description}
</description>

Rules for tasks.toml (TOML):
- `[meta]`: `plan = "{slug}"`, `total` = the number of `[[task]]` entries, `done = 0`, `status = "ready"`,
  `max_parallel = {max_parallel}`.
- `[[meta.verify]]`: at least one step, each `phase = "test"`, `command = "{visible}"`,
  `fail_msg = "the feature's own check failed on the integrated tree"`. This runs once, after every task has
  passed, on the whole feature; it is the only check for the feature as a whole.
- One `[[task]]` per unit of work small enough for a cheap model to implement and verify on its own. Each needs:
  `id` (short, unique, e.g. "T01"), `title`, `description` (the work, in enough detail that a cheap model needs
  nothing else), `status = "ready"`, `role = "implementer"`, `files` (a non-empty list of real paths this task
  writes or edits, no globs), `depends_on` (a list of other tasks' `id`s this one needs first, or `[]`),
  `max_retries` (an integer, 0-5), and at least one `[[task.verify]]` with `phase = "test"`, a `command` that is a
  real shell command this task's own change makes pass, and a `fail_msg`.
- Never write `depends_on_plan`, `read_files`, `model_hint` or a hidden command: every command you write is one a
  human could read and run themselves.
- Write scalars before any sub-table in each TOML table.
- Prefer several small, independently verifiable tasks over one large one: give the ladder's cheap models a real
  chance to execute each one.

Write the file now, then stop.
"""
EGRESS_LOG = run_roko.EGRESS_LOG
CLI_CACHE_WRITE_TTL = run_roko.CLI_CACHE_WRITE_TTL
MAX_PARALLEL_DEFAULT = 3


class RunnerError(RuntimeError):
    """The arm cannot run this feature: the planner, the binary or the environment is wrong."""


def run_task(ctx: harness.TaskContext) -> harness.TaskOutcome:
    started, clock = harness.utc_now(), time.monotonic()
    settings = ctx.arm.get("roko", {})
    planner_settings = ctx.arm.get("planner", {})
    slug = planemit.plan_slug(ctx.key)
    transcript: list[dict] = []
    attempts: list[run_roko.RokoAttempt] = []
    status, reason = "infra_error", ""
    try:
        binary = run_roko.binary_path(ctx.arm)
        plan_dir = ctx.workdir / "plans" / slug
        plan_dir.mkdir(parents=True)  # the planner writes tasks.toml here, as planemit.emit's own _write does for it
        plan_attempt, plan_cost_usd = _run_planner(ctx, settings, planner_settings, slug, transcript)
        attempts.append(plan_attempt)
        if not (plan_dir / "tasks.toml").is_file():
            raise RunnerError(f"the planner session ended without writing plans/{slug}/tasks.toml")
        config_path = ctx.workdir / "roko.toml"
        config_path.write_text(_roko_toml(ctx, settings), encoding="utf-8")
        api_key_envs = {table.get("api_key_env") for table in ctx.arm.get("providers", {}).values()
                        if table.get("api_key_env")}
        env = {**ctx.agent_env, "ROKO_CONFIG": str(config_path),
              **{name: run_roko.OFFLINE_KEY for name in api_key_envs}}
        network = run_roko.network_rule(ctx.endpoint)
        jail = sandbox.command([], deny=ctx.deny, network=network, sockets=[ctx.workdir])
        build = run_roko._build(binary, env, settings.get("build") or None, transcript, jail)
        head = [str(binary), "--repo", str(ctx.workdir), "--no-serve", "--color", "never"]
        checked = run_roko._roko([*head, "plan", "validate", "--strict", "--dag", str(ctx.workdir / "plans")],
                                 ctx.workdir, env, min(120.0, _left(ctx, clock)), jail)
        transcript.append(checked.event("validate"))
        if checked.returncode != 0:
            raise RunnerError(f"the planner's plan failed `plan validate --strict --dag` (exit "
                              f"{checked.returncode}): {(checked.stdout + checked.stderr).strip()[-300:]}")
        max_parallel = int(settings.get("max_parallel", MAX_PARALLEL_DEFAULT))
        ran = run_roko._roko([*head, "plan", "run", str(plan_dir), "--no-tui", "--max-parallel", str(max_parallel)],
                             ctx.workdir, env, _left(ctx, clock), jail)
        transcript.append(ran.event("run"))
        execution, problems = _settle_plan(ctx, slug, build)
        attempts += execution
        transcript.append({"event": "check", "problems": problems})
        status, reason = _status(ran, slug, problems, ctx.workdir)
        if status == "completed":
            attempts.append(_integration_attempt(ctx))
    except (ledger.BudgetError, RunnerError, OSError) as err:
        reason = f"{type(err).__name__}: {err}"[:500]
        transcript.append({"event": "error", "error": reason})
    if attempts:
        try:
            attempts[-1].tree = run_roko.repo.tree_hash(ctx.workdir)
        except (OSError, run_roko.repo.RepoError):
            attempts[-1].tree = None
    for attempt in attempts:
        # The planner's session is a $0 subscription call (module docstring), never billed, whatever the arm's
        # own billed-ness is for its (billed) executors.
        billed = ctx.billed and attempt.cost_class != "plan"
        ctx.ledger.append(attempt_key=attempt.attempt_key, provider=attempt.provider,
                          model_reported=attempt.model_reported, usage=attempt.reported_usage(),
                          cost=attempt.cost or ledger.Cost(None, None, "unknown"), billed=billed,
                          reserved_usd=attempt.reserved_usd)
    return harness.TaskOutcome(status=status, reason=reason, attempts=attempts, transcript=transcript,
                               started_at=started, finished_at=harness.utc_now(),
                               network_policy={"network": run_roko.network_rule(ctx.endpoint),
                                               "sandbox": sandbox.kind(ctx.deny, run_roko.network_rule(ctx.endpoint)),
                                               "unix_sockets": "workspace"})


def _left(ctx: harness.TaskContext, clock: float) -> float:
    return max(ctx.caps.wallclock_s - (time.monotonic() - clock), 1.0)


def _run_planner(ctx: harness.TaskContext, settings: dict, planner_settings: dict, slug: str,
                 transcript: list[dict]) -> tuple[run_roko.RokoAttempt, float | None]:
    """Claude-opus-5-5, through this task's own egress proxy, writes `plans/<slug>/tasks.toml`; priced as `run_cli`
    prices the direct arm's sessions (U' from `modelUsage`, source `cli_usage`)."""
    planner_model = planner_settings.get("model", "claude-opus-5-5")
    table = ctx.arm.get("providers", {}).get("anthropic", {})
    if not table:
        raise RunnerError("the arm has no [providers.anthropic] endpoint for the planner")
    planner_endpoint = provider.Endpoint(provider="anthropic", base_url=table["base_url"])
    planner_caps = dataclasses.replace(ctx.caps, turns_per_task=int(planner_settings.get("turns", 30)),
                                       wallclock_s=float(planner_settings.get("wallclock_s", 600)),
                                       usd_per_task=float(planner_settings.get("usd_cap", 3.0)))
    planner_ctx = dataclasses.replace(ctx, model=planner_model, endpoint=planner_endpoint, caps=planner_caps)
    cli = run_cli.CliConfig.from_table(ctx.arm.get("cli", {}))
    egress = run_roko._start_egress(ctx, settings)
    try:
        invocation = run_cli.build_invocation(planner_ctx, cli, egress)
        prompt = PLANNER_PROMPT.format(slug=slug, description=ctx.spec_text.strip(),
                                       max_parallel=int(settings.get("max_parallel", MAX_PARALLEL_DEFAULT)),
                                       visible=shlex.join(ctx.visible_verify) if len(ctx.visible_verify) == 1
                                       else " && ".join(f"( {c} )" for c in ctx.visible_verify))
        meter = run_cli.Meter(ctx.snapshot, cache_write_ttl=cli.cache_write_ttl)
        session = run_cli.run_session(invocation, prompt, cwd=ctx.workdir, wallclock_s=planner_caps.wallclock_s,
                                      usd_cap=planner_caps.usd_per_task, meter=meter)
    finally:
        egress.close()
    transcript += [{"event": "planner", **event} for event in session.events]
    attempt = run_roko.RokoAttempt(number=0, attempt_key=f"{ctx.chain_key}:plan", model_requested=planner_model,
                                   provider="anthropic", reserved_usd=planner_caps.usd_per_task, cost_class="plan",
                                   usage_unknown=True)
    if session.result:
        run_roko._meter_from_cli(attempt, session.result, ctx.snapshot)
    if not session.started or session.error:
        raise RunnerError(f"the planner could not start: {session.error or 'no session'}")
    return attempt, attempt.cost.api_equiv_usd if attempt.cost else None


def _roko_toml(ctx: harness.TaskContext, settings: dict) -> str:
    """The ladder's providers and models (planemit's own tables, module docstring); no task or gate rungs of its
    own, since the planner's tasks.toml already carries every task's verify step and the whole-plan `[meta.verify]`
    gate."""
    allowed = ctx.arm["arm"]["models_allow"]
    rows = [ctx.snapshot.row(model) for model in allowed]
    rungs = [run_roko._rung(ctx.arm, model, ctx.snapshot, ctx.caps) for model in allowed]
    providers = {}
    for rung in rungs:
        providers.setdefault(rung.provider, planemit.PROVIDER_TABLE.format(
            key=planemit._key(rung.provider), kind=planemit._s(rung.provider_kind),
            base_url=planemit._s(rung.base_url), api_key_env=planemit._s(rung.api_key_env)))
    models = "".join(planemit.MODEL_TABLE.format(
        key=planemit._key(rung.model), provider=planemit._s(rung.provider), model=planemit._s(rung.model),
        context_window=rung.context_window, max_output=rung.max_output, rates=planemit._rates(row))
        for rung, row in zip(rungs, rows))
    rung_lines = "".join(f"  {{ name = {planemit._s(rung.name)}, model = {planemit._s(rung.model)} }},\n"
                         for rung in rungs)
    start = "{ " + ", ".join(f"{tier} = {planemit._s(ctx.model)}" for tier in planemit.TIERS) + " }"
    usd_cap = run_roko._worst_task_usd(ctx.arm, ctx.snapshot, ctx.caps, ctx.price_row) or ctx.caps.usd_per_task
    return (
        "config_version = 2\nschema_version = 2\n\n[agent]\ndefault_model = "
        f"{planemit._s(ctx.model)}\n{''.join(providers.values())}{models}\n"
        f"[routing]\nfallback_models = []\n\n[routing.ladder]\nenabled = true\nrungs = [\n{rung_lines}]\n"
        f"start = {start}\n\n[gates]\ncargo_fix_enabled = false\nmax_review_cycles = 0\n"
        f"adaptive_min_retries = {int(settings.get('max_retries', 2))}\n"
        f"adaptive_max_retries = {int(settings.get('max_retries', 2))}\n\n"
        "[[gates.rungs]]\nname = \"visible\"\ncommand = \"true\"\ntimeout_secs = 5\nrequired = true\n\n"
        f"[pipeline.{planemit._key(settings.get('tier', 'focused'))}]\nmax_turns = {ctx.caps.turns_per_attempt}\n\n"
        f"[conductor]\nmax_agents = {int(settings.get('max_parallel', MAX_PARALLEL_DEFAULT))}\n\n"
        "[runner]\nworktree_per_task = false\n\n"
        f"[budget]\nmax_plan_usd = {round(usd_cap, 6)}\nmax_task_retry_usd = {round(usd_cap, 6)}\n"
        f"max_turn_usd = {max(round(usd_cap / 10, 6), 1e-06)}\n\n"
        "[learning]\nauto_playbook_refresh = false\nreplan_on_gate_failure = false\ndream_on_completion = false\n\n"
        "[learning.dreams]\ntrigger_on_plan_complete = false\n")


def _settle_plan(ctx: harness.TaskContext, slug: str, build: str | None
                 ) -> tuple[list[run_roko.RokoAttempt], list[str]]:
    """One attempt per episode of this plan, across every task it ran (unlike `run_roko.settle`, which is one
    task's own); `cost_class` is execute/retry/escalate by the same rule, per task_id. Exploratory (module
    docstring): no cross-record consistency check beyond the model-truth fields a verdict itself carries."""
    root = Path(ctx.workdir) / ".roko"
    episodes, _ = run_roko._jsonl(root / "episodes.jsonl")
    episodes = [row for row in episodes if (row.get("extra") or {}).get("plan_id", row.get("plan_id")) == slug]
    verdicts: list[dict] = []
    runs_dir = root / "runs"
    if runs_dir.is_dir() and not runs_dir.is_symlink():
        for path in sorted(runs_dir.glob("*/attempts.jsonl")):
            found, _ = run_roko._jsonl(path)
            verdicts += [row for row in found if row.get("schema_version") == run_roko.VERDICT_SCHEMA
                        and row.get("plan_id") == slug]
    by_key = {verdict.get("attempt_key"): verdict for verdict in verdicts if verdict.get("attempt_key")}
    problems: list[str] = []
    attempts: list[run_roko.RokoAttempt] = []
    before_by_task: dict[str, str | None] = {}
    seen_by_task: dict[str, int] = {}
    for episode in episodes:
        extra = episode.get("extra") or {}
        task_id = str(episode.get("task_id") or "")
        key = extra.get("attempt_key")
        if not task_id or not key:
            problems.append(f"model_unverified: an episode of task {task_id or '?'} names no attempt_key")
            continue
        seen_by_task[task_id] = seen_by_task.get(task_id, 0) + 1
        position = seen_by_task[task_id]
        dispatched, before = episode.get("model") or None, before_by_task.get(task_id)
        before_by_task[task_id] = dispatched
        verdict = _gate_verdict(episode)
        attempts.append(run_roko.RokoAttempt(
            number=len(attempts) + 1, attempt_key=key, model_requested=ctx.model,
            provider=str(episode.get("backend") or ""), reserved_usd=0.0, usage_unknown=True,
            model_dispatched=dispatched, gate_verdict=verdict, roko_build=build, task_id=task_id,
            ended_by="gate_passed" if verdict == "passed" else verdict or "gate_failed",
            cost_class="execute" if position == 1 else "escalate" if dispatched and before and dispatched != before
            else "retry"))
    if not attempts:
        problems.append("model_unverified: Roko recorded no attempt for any plan task")
    for attempt in attempts:
        verdict = by_key.get(attempt.attempt_key)
        if verdict:
            run_roko._meter_from_verdict(attempt, verdict, ctx.snapshot)
        if attempt.cost is None:
            attempt.cost = ledger.price(attempt.reported_usage(), ctx.snapshot.row(attempt.model_reported))
    return attempts, problems


def _gate_verdict(episode: dict) -> str | None:
    outcome = (episode.get("extra") or {}).get("outcome")
    if isinstance(outcome, str):
        return outcome if outcome in run_roko.VERDICT_TAGS else None
    return "passed" if episode.get("success") is True else None


def _integration_attempt(ctx: harness.TaskContext) -> run_roko.RokoAttempt:
    """The whole-plan gate (gap-60233f) Roko itself runs as part of `plan run`: a Python check, no model call, $0."""
    return run_roko.RokoAttempt(number=0, attempt_key=f"{ctx.chain_key}:integrate", model_requested=ctx.model,
                                provider="", reserved_usd=0.0, cost_class="integrate", calls=0, calls_known=True,
                                usage_unknown=False, cost=ledger.Cost(0.0, 0.0, "provider_usage"),
                                ended_by="gate_passed")


def _status(ran, slug: str, problems: list[str], workdir: Path) -> tuple[str, str]:
    if problems:
        return "infra_error", "; ".join(problems)[:500]
    if ran.timed_out:
        return "timeout", "wallclock"
    path = Path(workdir) / ".roko" / "state" / "graph" / slug / "checkpoint.json"
    if not path.is_file():
        return "infra_error", f"roko exited {ran.returncode} with no checkpoint"
    try:
        checkpoint = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        return "infra_error", f"the checkpoint is unreadable: {err}"
    if checkpoint.get("status") == "succeeded":
        return "completed", "plan_succeeded"
    if checkpoint.get("status") == "failed":
        return "failed", "plan_failed"
    return "infra_error", f"roko exited {ran.returncode} with the plan {checkpoint.get('status') or 'unrecorded'}"
