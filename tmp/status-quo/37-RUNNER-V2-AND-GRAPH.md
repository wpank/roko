# Runner V2 And Graph

> Status-quo audit · re-verified 2026-07-08 · git HEAD 5852c93c05 on `main`. Companion to `31-GRAPH-CELLS-ENGINE.md` (the full graph-crate audit). This file is the operator-facing decision doc: **which plan engine is live, why, and the path to one canonical engine.** All file:line refs re-checked against source on this date.

## TL;DR — there are (at least) three plan paths, and the default does no real work

| Path | Selected by | Backing crate | Does real agent work? | State/lock/resume? |
|---|---|---|---|---|
| **Graph Engine** | default (`--engine graph`) | `roko-graph` | **No** — every task → `TaskExecutorCell` dry-run | **No** — no lock, no episodes, no snapshot |
| **Runner v2** | `--engine runner-v2` | `roko-orchestrator` (via `orchestrate.rs`/`event_loop.rs`) | **Yes** — real dispatch, gates, merge queue | **Yes** — lock + `.roko/state/executor.json` + resume |
| **legacy-orchestrate** | `--features legacy-orchestrate` (OFF by default) | `run.rs` (pre-WorkflowEngine loop) | Yes (older path) | Partial |

`roko plan run plans/` with no flags runs the **Graph Engine**, which is a dry-run stub: it prints `SUCCESS`, spawns no agent, writes nothing. See `31-GRAPH-CELLS-ENGINE.md` for the full mechanism.

## Operational Rule Today

- Use `roko plan run ... --engine runner-v2` for real plan execution.
- Do **not** rely on default `roko plan run ...` for real work until the Graph default is fixed. It exits 0 with `SUCCESS` while doing nothing.
- `roko resume` is broken for real resumption: it hardcodes `engine: PlanEngine::Graph` (`main.rs:2699`), so the snapshot is found then discarded with `"snapshots will be ignored"` (`commands/plan.rs:260-263`) and the run re-executes as a dry-run. Resume real work with `roko plan run plans/ --engine runner-v2` (Runner v2 auto-resumes from `.roko/state/executor.json`, `plan.rs:359-376`).
- Use `roko graph validate/run/show` for graph authoring and graph-engine tests only, **not** as proof of agent work.

## Why the default is hollow

- The clap field for `PlanCmd::Run` sets `#[arg(long, default_value = "graph")]` (`main.rs:1361`). (Note: the `PlanEngine` enum's `#[derive(Default)]` marks `RunnerV2` as the type-level default — `main.rs:1301` — but the clap `default_value` overrides it for the CLI, so `graph` wins.)
- The Graph path (`cmd_plan_run_engine`, `plan.rs:1567-1715`) converts each task to a `task-executor` node (`convert.rs:63`) and runs `roko_graph::default_registry()` (`plan.rs:1644`).
- That registry maps `task-executor` → `TaskExecutorCell::default()` (`engine.rs:356-358`), whose `dry_run` field is `true` (`task_executor.rs:31-34`). It emits a synthetic `task-output:dry-run:<title>` engram and returns `Complete` (`task_executor.rs:70-79`). The `dry_run: false` branch is unimplemented and also falls back to dry-run with a warning (`task_executor.rs:80-92`).
- The engine path builds `CellContext::new()` (empty trace/run/budget, `plan.rs:1646`), acquires **no** workspace lock (the lock lives only in the Runner v2 branch, `plan.rs:272`), emits no episodes/signals/events, and writes no snapshot.

Runner v2 (`roko-orchestrator` driven from `orchestrate.rs`) is the only path with real agent dispatch, gates, snapshots, resume, feedback/replan, and merge behavior.

## Feature-flag façade (drift)

`legacy-runner-v2` is a **default** cargo feature (`roko-cli/Cargo.toml:15,20`) whose comment claims it gates the Runner v2 path — but **no `#[cfg(feature = "legacy-runner-v2")]` exists anywhere in `src/`** (verified 2026-07-08: 0 matches). Runner v2 always compiles. The feature that actually gates code is `legacy-orchestrate` (non-default), with ~39 `#[cfg(feature = "legacy-orchestrate")]` sites in `run.rs`. So the Cargo comment is misleading and the "runner-v2 is legacy/optional" framing is false.

## Migration Options

| Option | Pros | Cons |
|---|---|---|
| Flip default to Runner v2 now | Immediately restores honest default UX. | Graph as v2 target becomes opt-in again. |
| Keep Graph default but refuse non-live task execution | Forces visibility of the migration gap. | Users must pass `--engine runner-v2` for work. |
| Implement live Graph task execution now | Aligns CLI default with v2 target. | Requires dispatcher injection, persistence, gates, resume, and events to avoid partial parity. |

Recommended near-term path: **flip or refuse first** (P0), then implement live Graph parity behind proof gates.

## Minimum Graph Parity (before Graph can be the honest default)

- [ ] Live agent dispatch through injected dispatcher (reuse the `AgentDispatcher` pattern in `cells/agent.rs:134`; register a live `task-executor` factory from roko-cli where roko-agent is available, replacing `TaskExecutorCell::default()` at `plan.rs:1644`).
- [ ] Per-node gate execution or a real `gate-pipeline` cell (roko-gate).
- [ ] Snapshot/resume (`.roko/state/`), skipping `Complete` nodes on resume.
- [ ] Workspace lock + state persistence (episodes/signals) on the engine path.
- [ ] Budget/cost enforcement (build `BudgetTracker` from `GraphConfig`; populate `CellContext.budget_remaining`).
- [ ] Conditional edge evaluation (unify `EdgeCondition` and `condition::Condition`; evaluate in `GraphEngine::execute`).
- [ ] Parallel frontier execution honoring `max_parallel` (currently a metadata label only, `convert.rs:51`).
- [ ] Event/StateHub output (Bus pulses, dashboard visibility).
- [ ] Cross-plan dependency behavior (currently silently skipped, `convert.rs:91-99`) or an explicit unsupported status.

## Toward one canonical engine (ordered)

1. **[P0]** Make the default honest — flip `--engine` default to `runner-v2` **or** make the Graph path refuse/warn when `task-executor` resolves to a dry-run stub. Today it prints `SUCCESS` with no agent spawned.
2. **[P0]** Fix `roko resume` — stop hardcoding `PlanEngine::Graph` (`main.rs:2699`); route to `runner-v2` until graph snapshots exist.
3. **[P1]** Land the Minimum Graph Parity checklist above, each item behind a proof gate (CLI invocation + artifact assertion), so the Graph path can eventually reclaim the default.
4. **[P2]** Reconcile the `legacy-runner-v2` feature: either add the promised `cfg` gates or delete the feature + its Cargo comment.
5. **[P2]** Decide the fate of `legacy-orchestrate` (`run.rs`): keep as a fallback or retire it — three plan paths is one too many.
6. **[P3]** Once Graph is at parity and default, retire the Runner-v2-specific CLI surface and collapse to a single engine.

## Cross-references

- Full graph-crate mechanism, cell census, and per-file evidence: `31-GRAPH-CELLS-ENGINE.md`.
- Runner v2 executor internals (ParallelExecutor, MergeQueue, RecoveryEngine/snapshots) live in `crates/roko-orchestrator/` — see the "Undocumented: roko-orchestrator is the real Runner v2 engine" note in `31-GRAPH-CELLS-ENGINE.md`.
