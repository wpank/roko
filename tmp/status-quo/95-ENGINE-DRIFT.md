# ENGINE DRIFT — historical hollow default and current lifecycle split

> **Superseded runtime note (2026-08-15):** Runner v2 remains the default and complete
> lifecycle owner. `roko plan run --engine graph` now injects a live `TaskExecutorCell`
> dispatcher that reaches the shared provider runtime; unconfigured registries fail closed.
> The Graph path still lacks Runner-v2 gates/replan, attempt worktrees/merge, durable
> state/episodes/efficiency, CLI resume, and cancellation/cleanup parity. The historical
> dry-run findings below are retained as audit history, not current behavior.

> Status-quo audit · created 2026-07-08 @ HEAD `5852c93c05` on `main` · **updated 2026-08-13** @ HEAD on `main` · **navigation-layer reference.** Centralizes the single most important cross-cutting theme in the codebase: three coexisting plan-execution generations, the projection seam that tries to unify their output, and the historical default-engine bug (now fixed). Sources: `36-ORCHESTRATION-RUNNERS.md`, `37-RUNNER-V2-AND-GRAPH.md`, `31-GRAPH-CELLS-ENGINE.md`, `92-RUNNER-V2-MODULE-FAMILY.md`, and the second-pass execution traces `96-TRACE-RUNNER-V2-EXECUTION.md`, `97-TRACE-SERVE-LIFECYCLE.md`, `98-TRACE-SELF-HOSTING-LOOP.md`, `99-TRACE-AGENT-TURN.md`, `101-TRACE-GATE-PIPELINE.md`; code re-checked at HEAD.
>
> **2026-08-13 update:** The two P0 engine-routing bugs documented below (graph-as-default, `roko resume` hardcoding Graph) have been **fixed** on `main` as of commits `996c7a08d` (E01-T01) and `8bf311c2f` (E01-T02), landed 2026-07-11. The three engines still coexist, and the Graph engine's `TaskExecutorCell` is still a dry-run stub, but the CLI now correctly defaults to Runner v2.

## The one thing to know

> **FIXED (2026-07-11).** The default-engine bug described below was fixed by E01-T01 (`996c7a08d`). `roko plan run <dir>` now defaults to `--engine runner-v2` (`main.rs:1382`). The Graph engine still exists and its `TaskExecutorCell` is still a dry-run stub (`task_executor.rs:32`), but it is no longer the default.

~~`roko plan run <dir>` **with no flags does no real work.**~~ Historically, it defaulted to the Graph Engine, which mapped every task to a dry-run stub, printed `SUCCESS`, spawned no agent, spent $0, changed no code, and wrote no episodes/snapshot. Real execution required `--engine runner-v2`. Empirical proof from the original audit: daeji Run 1 `plan run --fresh` reported "8 nodes, 8 output signals, SUCCESS" in ~2 s with **0 agents / $0.00 / no code changed**; after `--engine runner-v2` the same plan ran 8 real agents, $2.75, 621 s.

## Three generations, side by side

| | **orchestrate.rs (v1)** | **Runner v2** | **Graph Engine** |
|---|---|---|---|
| Location | `roko-cli/src/orchestrate.rs` (23,676 LOC) | `roko-cli/src/runner/` (17,090 LOC, 19 files) | `crates/roko-graph/` |
| Compiled by default? | ❌ gated behind `legacy-orchestrate` (`Cargo.toml:15`) | ✅ always compiled | ✅ always compiled |
| Selected by | `--features legacy-orchestrate` (off) | ~~`--engine runner-v2`~~ **now the default** (`main.rs:1382`); also implicit for `do`/`serve`/`prd`/`worker` | ~~**default**~~ `--engine graph` (was default until E01-T01, 2026-07-11) |
| Real agent work? | yes (older path) | ✅ yes — streaming dispatch, gates, merge, resume | ❌ **no** — `TaskExecutorCell` dry-run stub |
| State / lock / resume | partial | ✅ lock + `executor.json` + auto-resume | lock only; `--resume-plan` ignored and no execution snapshot |
| Status | 🕰️ legacy, dead-by-default | ✅ **the live engine + now the default** | 🟡 hollow (no longer default since E01-T01) |

## Why the former default was hollow (historical mechanism)

- ~~`PlanCmd::Run` clap field: `#[arg(long, default_value = "graph", value_enum)]` (`main.rs:1361`).~~ **FIXED (E01-T01, 2026-07-11).** The clap `default_value` is now `"runner-v2"` (`main.rs:1382`), matching the `PlanEngine` enum's `#[default]` of `RunnerV2` (`main.rs:1322`).
- Graph path (`cmd_plan_run_engine`, `commands/plan.rs:1567-1715`) converts each task to a `task-executor` node (`convert.rs:63`), runs `roko_graph::default_registry()`.
- That registry binds `task-executor` → `TaskExecutorCell::default()` (`engine.rs:356-358`), whose `dry_run` field is `true` (`task_executor.rs:31-34`). It emits a synthetic `task-output:dry-run:<title>` engram and returns `Complete` (`:70-79`). The `dry_run:false` branch is unimplemented and *also* falls back to dry-run with a warning (`:80-92`).
- The engine path now holds the same command-lifetime workspace lock as runner v2. It still emits no episodes/signals/events and writes no snapshot.
- **Built-not-registered**: `roko-graph` already has a real `AgentCell` (`cells/agent.rs`, full model/provider/tools config) plus `ComposeCell`/`GraduationCell` — but `default_registry()` never binds them. The live-dispatch cell exists in the tree; the default path can't reach it.

## The `roko resume` bug (P0) -- FIXED

> **FIXED (E01-T02, 2026-07-11).** `roko resume` now builds `PlanCmd::Run { engine: PlanEngine::RunnerV2, .. }` (`main.rs:2782`).

~~`roko resume` locates a snapshot then builds `PlanCmd::Run { engine: PlanEngine::Graph, resume_plan: Some(snapshot), .. }` (`main.rs:2697-2709`). Because the Graph path prints "snapshots will be ignored" (`commands/plan.rs:260-264`), the snapshot is discarded and the run dry-runs.~~ This was the original bug. Runner v2 auto-resumes from `.roko/state/executor.json`.

## The feature-flag façade

`legacy-runner-v2` is a **default** cargo feature (`roko-cli/Cargo.toml:15`) whose comment claims it gates the Runner v2 path — but **no `#[cfg(feature = "legacy-runner-v2")]` exists anywhere in `src/`** (0 matches). It gates only 4 test files. Runner v2 always compiles. The feature that actually gates code is `legacy-orchestrate` (non-default, ~39 `cfg` sites in `run.rs`). So the "runner-v2 is legacy/optional" framing in the Cargo comment and GAPS.md Task 102 is **false**.

## Where each engine writes state

| Engine | `.roko/state/` writes | Episodes | Lock |
|---|---|---|---|
| Runner v2 | `executor.json`, `orchestrator.json`, `run-state.json`, `state-snapshot.json`, `run-ledger.jsonl` | ✅ `episodes.jsonl` | ✅ `.workspace.lock` |
| Graph (default) | **nothing** | ❌ | ❌ |
| orchestrate.rs | v1 snapshots (dead) | ✅ (when compiled) | ✅ |

The live workspace currently has only `state-snapshot.json` + 20 orphaned `state-snapshot.json.bak.*` + `run-ledger.jsonl` — confirming **Runner v2 was the last engine to actually run here**, and `--fresh` archives (not deletes) prior snapshots. See `60-STATE-PERSISTENCE-LEDGER.md`.

## The projection seam (attempted unification of engine output)

`crates/roko-cli/src/projection/` (`mod.rs`, `cli_progress.rs`, `dashboard.rs`) is the **single seam** that turns runner events into the three consumer shapes, fixing a *separate* drift: *"TUI showed one set of fields, HTTP showed another, non-TUI CLI rendered a third"* (`projection/mod.rs:1-24`). It wraps (does not duplicate) the runner-local `runner::projection::Projection` facade (`mod.rs:22-24,36`):

- `ProjectionEvent` — normalized event every consumer subscribes to via one broadcast channel; consumers never branch on payload (`mod.rs:12-16`).
- `dashboard::DashboardProjection` — bridges the broadcast into `StateHub`/`DashboardEvent` (`mod.rs:17-18`).
- `cli_progress::CliProgressPrinter` — non-TUI CLI rendering (`mod.rs:19-20`).

This only normalizes **Runner v2** output — the Graph default emits `eprintln` only and never reaches the projection seam, so the drift-fix does not help the default path. Status: 🟡 new / partial (see `43-SURFACES-DEMO-UX.md`).

## Consequences for the navigation layer

1. **CLAUDE.md engine-default claim: partially fixed.** The self-hosting workflow (step 5 `plan run plans/`) now does real work by default since the engine default was flipped to `runner-v2` (E01-T01). `roko resume` also works correctly now (E01-T02). However, CLAUDE.md still names `orchestrate.rs` in some component descriptions (orchestrate.rs is dead-by-default, gated behind `legacy-orchestrate`).
2. **"Self-host loop works" claims no longer require specifying the engine.** The default `plan run` now routes to Runner v2. Prior 55-task and 8-node runs reporting `agent_outcomes: 0` / $0.00 were Graph no-ops under the old default; this is no longer a risk.
3. **Two DAGs, two projection layers, four state-file generations** — the god-object problem (orchestrate.rs = 23,676 LOC, ~3.3% of the whole tree) is regrowing in Runner v2 (`event_loop.rs` = 6.7K LOC, 16× spec). orchestrate.rs anchors a ~52K-LOC **dead legacy island** (orchestrate.rs + roko-orchestrator + roko-conductor + roko-plugin), gated off by `legacy-orchestrate`. The worst remaining layering violation is `roko-runtime → roko-gate` (V1, `Cargo.toml:27`), and `roko-index` reimplements HDC with no `roko-primitives` dep. See [16-CODEBASE-INVENTORY.md](16-CODEBASE-INVENTORY.md), [11-DEPENDENCY-GRAPH.md](11-DEPENDENCY-GRAPH.md), [03-CRATE-AUDIT.md](03-CRATE-AUDIT.md), [102](102-SPEC-DEBT-LEDGER.md), [104](104-DEAD-CODE-AND-FACADE-CENSUS.md).

## The engine-drift consequence set (second pass, docs 96–101)

Engine drift is not just "the default engine is a stub." Because the **live** capabilities were built for `orchestrate.rs` and only partially re-hosted, the runner-v2 path itself is missing whole subsystems that CLAUDE.md/docs claim are wired. Each of the following is a *direct consequence* of the same split, verified file-by-file this pass:

| Consequence | What the docs imply | What the live runner-v2 path actually does | Evidence |
|---|---|---|---|
| **No live task DAG** | Plan compiled to `UnifiedTaskDag`, `max_concurrent_tasks` parallelizes tasks | Flat `task_index` HashMap + per-plan FSM; concurrency is **per-plan** (`max_concurrent_plans=4`, one agent/plan); `max_concurrent_tasks` only sizes the gate semaphore; `runner/task_dag.rs`/`UnifiedTaskDag` are dead/legacy-only | [96](96-TRACE-RUNNER-V2-EXECUTION.md) §4-5 (`event_loop.rs:62,473,484-497`; `defaults.rs:313`) |
| **Gate path is shallow** | Adaptive thresholds, oracles 4-6, SPC/ratchet, VerdictPublisher, gate-failure→replan | `gate_dispatch::run_gate_once` uses `RungExecutionInputs::default()`, **never calls `enrich_rung_config`**; rungs 3-6 stub-pass `Verdict::pass`; EMA only updates rung 2; `GateThresholds::save` has zero callers; replan is prompt-append only. All the adaptive apparatus lives only on the dead `PlanRunner`. | [101](101-TRACE-GATE-PIPELINE.md) (`gate_dispatch.rs:104,140`; `event_loop.rs:1128,1206`; `rung_dispatch.rs:290`) |
| **Compose bypasses the canonical builder** | 9-layer `SystemPromptBuilder`/12-slot/`RoleSystemPromptSpec`/VCG is the live prompt path | Runner-v2 prompts built by CLI-side `PromptAssembler` (`dispatch/prompt_builder.rs:717`); the 9-layer builder + VCG run only on non-default paths and are reachable-but-cold | [103](103-DUPLICATE-TYPES-CENSUS.md) row 12, [102](102-SPEC-DEBT-LEDGER.md) |
| **Safety funnel bypassed on default provider** | roko safety-gates every tool call | The `ToolDispatcher`→`SafetyLayer` 9-policy pre-check runs only on the OpenAI-compat `ToolLoop`; the **default Claude-CLI (and Codex) subprocess loop never touches roko `SafetyLayer` per tool call** (`--dangerously-skip-permissions:true`) | [99](99-TRACE-AGENT-TURN.md) §7 (`dispatch_v2.rs:1202`; `openai_compat.rs:388`) |

Two more drift symptoms confirmed this pass: **`events.jsonl` is a 44 MB / 97% `feed_tick` write-only firehose nothing reads back** ([97](97-TRACE-SERVE-LIFECYCLE.md)), and **CascadeRouter LinUCB state is never durably persisted** (dual writers, resets toward identity, [96](96-TRACE-RUNNER-V2-EXECUTION.md)). Net: "runner-v2 is ~80% ported" is true by *hook count* but the missing 20% is load-bearing (DAG, gate adaptivity, canonical compose, provider-agnostic safety), and all of it traces to the same never-finished migration off `orchestrate.rs`.

## Roadmap (the P0 convergence path)

1. ~~**[P0]** Make the default honest — flip `--engine` default to `runner-v2`.~~ **DONE** (E01-T01, `996c7a08d`, 2026-07-11). `main.rs:1382` now reads `default_value = "runner-v2"`.
2. ~~**[P0]** Fix `roko resume` — route to `PlanEngine::RunnerV2`.~~ **DONE** (E01-T02, `8bf311c2f`, 2026-07-11). `main.rs:2782` now reads `engine: PlanEngine::RunnerV2`.
3. **[P0]** Wire `TaskExecutorCell` live mode — delegate to the Runner v2 dispatch facade, **or** register the already-built `AgentCell` (`cells/agent.rs`) in `default_registry()`. Verify: `plan run --engine graph` writes an episode. **STILL OPEN** -- `TaskExecutorCell::default()` still has `dry_run: true` (`task_executor.rs:32`), and the `dry_run:false` branch still falls back to dry-run with a warning.
4. **[P1]** Reconcile the `legacy-runner-v2` feature — add the promised `cfg` gates or delete it + fix Cargo comment + GAPS Task 102.
5. **[P1]** Update CLAUDE.md orchestration rows, self-host workflow flags, and executor.json claims (orchestrate.rs → `runner/` + `roko-graph`).
6. **[P2]** Land the Minimum Graph Parity checklist (`37-RUNNER-V2-AND-GRAPH.md`): resume, gate nodes, parallel dispatch, budgets, events.
7. **[P3]** Once Graph is at parity + default, collapse to one engine; delete orchestrate.rs after porting its ❌ rows (oracles, custody/attestation, dreams trigger).

## 2026-08-13 Status

- Graph dry-run default: **FIXED** (E01-T01, `996c7a08d`, 2026-07-11). `--engine` default is now `runner-v2` (`main.rs:1382`). The Graph engine's `TaskExecutorCell` still defaults to `dry_run: true`, but it is no longer reachable without explicit `--engine graph`.
- `roko resume` hardcodes Graph: **FIXED** (E01-T02, `8bf311c2f`, 2026-07-11). Now routes to `PlanEngine::RunnerV2` (`main.rs:2782`).
- Runner v2 is the real engine: **STILL TRUE** -- and now also the default.
- Graph engine live mode (P0 item 3): **STILL OPEN** -- `TaskExecutorCell::default()` is still `dry_run: true`; the `AgentCell` is still built-not-registered in `default_registry()`.
- ISFR in serve lifecycle: **STILL PRESENT but not deprecated** -- `roko-serve/src/lib.rs` still spawns `start_isfr_keeper` (line 409), `start_block_watcher` (line 410), and `start_isfr_relay_bridge` (line 434) as part of the serve boot sequence. ISFR is gated by `isfr.enabled` in config and is best-effort (failures are logged and swallowed). See `97-TRACE-SERVE-LIFECYCLE.md` rows 18-19, 23 for the original trace.

## Cross-references

- `92-RUNNER-V2-MODULE-FAMILY.md` — the live engine, file-by-file.
- `36-ORCHESTRATION-RUNNERS.md` — full three-generation audit + decomposition map.
- `37-RUNNER-V2-AND-GRAPH.md` — operator decision doc + Minimum Graph Parity checklist.
- `31-GRAPH-CELLS-ENGINE.md` — the Graph Engine crate + built-not-registered cells.
- `60-STATE-PERSISTENCE-LEDGER.md` — the 4 state-file generations.
- `43-SURFACES-DEMO-UX.md` — projection seam as a surface.
- `96-TRACE-RUNNER-V2-EXECUTION.md` — no live DAG, per-plan concurrency, replan/merge/dream holdouts.
- `97-TRACE-SERVE-LIFECYCLE.md` — the `events.jsonl` write-only firehose + empty panels.
- `99-TRACE-AGENT-TURN.md` — the default-provider safety bypass, 429-no-retry, tool-alias strip.
- `101-TRACE-GATE-PIPELINE.md` — the two gate engines; live one skips enrichment.
- `103-DUPLICATE-TYPES-CENSUS.md` — the two prompt-assembly surfaces (row 12).
