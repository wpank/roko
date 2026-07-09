# Runner v2 — the live execution engine (`crates/roko-cli/src/runner/`)

> Status-quo audit · created 2026-07-08 @ HEAD `5852c93c05` on `main` · sources: all 19 files of `crates/roko-cli/src/runner/` (17,090 LOC), `commands/plan.rs`, `main.rs`, `serve_runtime.rs`, `commands/do_cmd.rs`, `worker/cloud.rs`, `prd.rs`, live `.roko/state/`. Companion to `36-ORCHESTRATION-RUNNERS.md` (three-generation orchestration audit) and `37-RUNNER-V2-AND-GRAPH.md` (operator decision doc). This file is the **module-family evidence ledger** for the runner crate itself — 36/37 cover *which* engine is live and why; this covers *what each runner file does*.
>
> Status vocab: ✅ wired · 🔌 built-not-wired · 🟡 partial · ❌ missing · 🕰️ legacy/superseded

## Why this doc exists

The status-quo pack was originally written around `orchestrate.rs` (the v1 `PlanRunner`, now feature-gated **off** behind `legacy-orchestrate`, `roko-cli/Cargo.toml:15`). The real execution engine moved to `crates/roko-cli/src/runner/` — an event-driven `tokio::select!` loop — and **every live execution entry point except the default `plan run` calls it directly**. No single doc enumerated the runner module family; 36 is orchestration-wide and 43 covers it as a *surface*. This is that ledger.

## TL;DR

- **Runner v2 is the production plan engine.** It is invoked directly (no `--engine` plumbing) by: `roko do` (`commands/do_cmd.rs:616`), `roko serve` runs (`serve_runtime.rs:304`), PRD auto-execution (`prd.rs:956`), the cloud worker (`worker/cloud.rs:462`), and `roko plan run --engine runner-v2` (`commands/plan.rs:654`). The module doc-comment states its purpose plainly: *"replaces the batch-only `orchestrate.rs` plan runner with a streaming architecture"* (`runner/mod.rs:2-9`).
- **The default `plan run` does NOT reach it.** `--engine` defaults to `"graph"` (`main.rs:1361`), which routes to the dry-run Graph Engine. Runner v2 is the *implicit* engine everywhere else. See the ENGINE DRIFT doc (`95`) for the full three-engine picture.
- **19 files, 17,090 LOC.** `event_loop.rs` alone is ~6.7K LOC (16× the ~400-line `docs/v2/27-ORCHESTRATOR.md` spec) — the god-object problem is regrowing in generation 2 (`types.rs` 1.9K, `output_sink.rs` 1.4K).
- **Wraps roko-orchestrator's pure `ParallelExecutor`** as the only state machine (executor-pure / runner-does-I/O split, P6). Rebuilds its own lightweight `task_dag.rs` rather than using `roko-orchestrator`'s `UnifiedTaskDag`.
- **Real capabilities**: streaming stream-json dispatch, parallel tasks + parallel plans, gates, auto-resume from `.roko/state/executor.json`, merge queue, budgets, learning sinks, process-group teardown.
- **Regressions vs v1**: no worktree isolation (in-place, `merge.rs:145-150`); replan is prompt-context-injection only, no tasks.toml rewrite; no gate oracles; no hash-chained event log.

## Module-family census (per-file, file:line evidence)

| File | LOC~ | Role | Status | Evidence |
|---|---|---|---|---|
| `mod.rs` | 56 | Module tree + primary re-exports (`run`, `RunConfig`, `load_plans`, `SseStreamClient`) | ✅ | `runner/mod.rs:32-55`; doc-comment "replaces orchestrate.rs" `:2-9` |
| `event_loop.rs` | 6,681 | The `tokio::select!` loop; drives `ParallelExecutor`; per-task dispatch, gate scheduling, resume, snapshot flush, `RunReport` | ✅ | `run(plans, config, state_hub, cancel)` entry; `ParallelExecutor` at `:198`; `classify_gate_failure` `:25` |
| `types.rs` | 1,905 | `RunConfig` (from `RokoConfig` via `from_roko_config`), `RunReport`, `PlanReport`, task/gate types | ✅ | `RunConfig::from_roko_config` referenced in `mod.rs:13-14` |
| `plan_loader.rs` | ~700 | `load_plans` (tasks.toml only, schema-validated), `scaffold_missing_crates` | ✅ | `runner/plan_loader.rs:49-107`; used by all live entry points |
| `agent_stream.rs` | ~600 | Spawns `claude … --output-format stream-json`; process groups + `kill_tree` teardown | ✅ | process-group `:17,99-105,172` |
| `agent_events.rs` | ~400 | Line-by-line parse of stream-json events (`MessageDelta`/`TokenUsage`) into typed events | ✅ | — |
| `gate_dispatch.rs` | ~500 | Background-task gate dispatch, rung selection, `cargo check/test/clippy`, failure classification | ✅ | `event_loop.rs:25` `classify_gate_failure`; global gate scope footgun (compiles whole workspace) |
| `merge.rs` | 777 | `PlanMerger` → `MergeQueue` + pluggable `GitMergeBackend` + post-merge `CargoCheckRegressionGate` | ✅ | `merge.rs:22,49-52,152,296`; **in-place, no worktree** `:145-150,169-194` |
| `persist.rs` | ~650 | Writes `executor.json`/`orchestrator.json`/`run-state.json`/`state-snapshot.json`/`run-ledger.jsonl`; stale-PID reaping | ✅ | `persist.rs:30-55`; stale-PID `:618` |
| `resume.rs` | ~400 | Auto-resume from `executor.json`, strict validation, drift detection, `--force-resume`, JSONL recovery | ✅ | `event_loop.rs:289-480`; `runner/resume.rs` |
| `snapshot_writer.rs` | ~350 | Async unified checksummed `state-snapshot.json` writer | ✅ | flush-per-task |
| `state.rs` | ~700 | Runner-local run state; replan-context storage | 🟡 | replan context `:664-671` (retry-prompt only) |
| `task_dag.rs` | 613 | Lightweight ready-set DAG: skip-propagation, deadlines, backoff — **does not** use `UnifiedTaskDag` | ✅ | `runner/task_dag.rs`; parallel duplicate of `roko-orchestrator/src/dag.rs` |
| `tui_bridge.rs` | ~500 | `TuiBridge` push seam → `StateHub` for real-time TUI updates | ✅ | `commands/plan.rs:552-612`; wired to projection |
| `output_sink.rs` | 1,399 | Output sinks (TUI / inline / SSE fan-out) | ✅ | — |
| `inline_output.rs` | ~400 | Non-TUI CLI progress rendering | ✅ | pairs with `projection/cli_progress.rs` |
| `projection.rs` | ~400 | `DashboardSnapshot` — bounded ring of last 200 `ProjectionEvent`s, 4 KB output truncation, `broadcast` channel + drop/coerce counters | 🟡 | `runner/projection.rs:123-127`; runner-local telemetry buffer (distinct from top-level `projection/`) |
| `sse_stream.rs` | ~300 | `SseStreamClient` — SSE consumer for remote/serve-driven runs | ✅ | re-exported `mod.rs:54` |
| `extension_loader.rs` | ~300 | Loads runner extensions/hooks | 🟡 | present; wiring depth unverified |

## Live entry points (who calls `runner::run`)

| Caller | Site | Path |
|---|---|---|
| `roko do` | `commands/do_cmd.rs:616` | `runner::event_loop::run(...)` |
| `roko serve` runs | `serve_runtime.rs:304` | `crate::runner::run(...)` |
| PRD auto-exec | `prd.rs:956` | `crate::runner::run(...)` |
| Cloud worker | `worker/cloud.rs:462` | `crate::runner::run(...)` |
| `roko plan run --engine runner-v2` | `commands/plan.rs:654` | `runner::event_loop::run(...)` |
| **`roko plan run` (default)** | ❌ routes to Graph dry-run | `--engine="graph"` `main.rs:1361` |

Note: `roko-acp` calls a *different* runner surface (`run_with_workflow_engine`/`run_workflow_pipeline`, `bridge_events.rs:65,1354`) — the one-shot WorkflowEngine, not the plan-DAG event loop.

## Persistence artifacts (Runner v2 writes)

All under `.roko/` (`persist.rs:30-55`):

| Artifact | Content |
|---|---|
| `state/executor.json` | `ParallelExecutor` snapshot (the file resume reads) |
| `state/orchestrator.json` | aggregate run state |
| `state/run-state.json` | schema-versioned cost/token/completed |
| `state/state-snapshot.json` | unified **checksummed** snapshot (the one the live workspace actually has on disk) |
| `state/run-ledger.jsonl` | typed task-start / task-complete / gate events |
| `events.jsonl` | untyped feed for TUI/serve (44 MB on disk as of last run) |
| `episodes.jsonl` | agent turns via `FeedbackFacade` sinks |
| `learn/{efficiency,cascade-router,experiments,gate-thresholds}.*` | learning state |

**Drift**: 4 overlapping state-file generations; only `state-snapshot.json` is current on the live workspace; `--fresh` **archives** (not deletes) the other three → 20 orphaned `state-snapshot.json.bak.*` accumulate (`commands/plan.rs:274-306`). See `60-STATE-PERSISTENCE-LEDGER.md`.

## What Runner v2 dropped vs orchestrate.rs (v1)

- **Worktree isolation** — v1 had `WorktreeManager`; v2 runs in-place, protected only by merge-queue file locks + post-merge cargo check (`merge.rs:145-150`). Parallel plans mutate the same tree.
- **Agent-driven replan** — v1 rewrote `tasks.toml` via `build_gate_failure_plan_revision`; v2 injects replan context into the retry prompt only (`state.rs:664-671`).
- **Gate oracles** — Perplexity-search + LLM-judge rungs live only in gated orchestrate.rs.
- **Hash-chained event log** — v1's tamper-detecting `EventLog`; v2 uses untyped `events.jsonl` + `run-ledger.jsonl`.
- **Conductor watcher loop / circuit breaker** — only `RecoveryEngine` + gate-failure classification ported.

## Correctness footguns (P1, from daeji real runs)

- **Preflight-verify agent-skip** (`event_loop.rs:3873 task_should_preflight_verify`, skip `:4089-4119`): on attempt 1, if a task's structural `verify` already passes, the agent is **never spawned** ("task verification already passes -- skipping agent"). Scaffold/implement tasks whose stub already compiles are silently skipped. No `--force`/`--no-preflight` escape hatch.
- **Crate-name extraction bug** (`task_helpers.rs:49 crate_name_for_path`): takes the 2nd path segment as the Cargo package → nested/renamed crates fail `cargo check -p <wrong-name>` (daeji Run 2: 0/8 tasks).
- **Global gate scope** (`gate_dispatch.rs`): compiles the whole workspace instead of the task's crate.
- **Model-hint has no provider fallback** (`provider/mod.rs`): a `model_hint` at a provider without creds fails the task instantly, no retry, no user-visible error.

## Drift list

- **Feature-flag façade**: `legacy-runner-v2` is a *default* cargo feature whose comment claims it gates the Runner v2 path, but **no `#[cfg(feature = "legacy-runner-v2")]` exists in `src/`** — it gates only 4 test files. Runner v2 always compiles.
- **`PlanEngine` enum `#[default]` = `RunnerV2`** (`main.rs:1301`) contradicts the clap `default_value = "graph"` (`main.rs:1361`); clap wins on the CLI.
- **`roko resume` broken**: hardcodes `engine: PlanEngine::Graph` (`main.rs:2699`) so the snapshot it locates is discarded and the run dry-runs. Real resume path is `roko plan run --engine runner-v2` (auto-resume).
- **CLAUDE.md is stale**: ~15 component rows + "Absolute paths" still name `orchestrate.rs` as the wired hub. None of that code is in the default binary.
- **`event_loop.rs` = 6.7K LOC**, 16× the spec; `types.rs` 1.9K, `output_sink.rs` 1.4K — god-object regrowth.
- **Two DAGs**: `runner/task_dag.rs` (613 LOC, live) vs `roko-orchestrator/src/dag.rs` `UnifiedTaskDag` (2,559 LOC, dead except `detect_cycle_nodes`).

## Verification checklist

- [ ] `grep -n 'default_value = "graph"' crates/roko-cli/src/main.rs` → confirms default does NOT reach Runner v2 (`:1361`).
- [ ] `grep -rn "runner::run\|event_loop::run(" crates/ --include='*.rs' | grep -v runner/` → confirms 5 live callers (do_cmd, serve_runtime, prd, worker/cloud, plan.rs).
- [ ] `grep -rn "cfg(feature = \"legacy-runner-v2\")" crates/roko-cli/src/` → returns 0 (façade flag).
- [ ] `roko plan run <dir> --engine runner-v2` on a real plan → writes `.roko/episodes.jsonl` + spawns an agent (proves live dispatch).
- [ ] `wc -l crates/roko-cli/src/runner/*.rs` → 17,090 total; `event_loop.rs` ~6,681.
- [ ] `ls .roko/state/` → confirm which of the 4 state files are current (only `state-snapshot.json` expected).

## Roadmap (ordered)

1. **[P0]** Fix `roko resume` — route to `PlanEngine::RunnerV2` (honors snapshots) instead of hardcoded `Graph` (`main.rs:2699`). Verify: interrupted run + `roko resume` skips completed tasks.
2. **[P0]** Flip `roko plan run` default engine to `runner-v2` (or make `graph` refuse non-dry-run) so the honest default reaches this engine. Verify: default `plan run` spawns a real agent.
3. **[P1]** Fix preflight-verify agent-skip: add `--force`/`--no-preflight` and/or never preflight-skip scaffold/implement tasks (`event_loop.rs:3873,4089-4119`).
4. **[P1]** Fix `crate_name_for_path` to read `[package] name` from the nearest Cargo.toml / `cargo metadata` (`task_helpers.rs:49`).
5. **[P1]** Add model-hint provider fallback (`provider/mod.rs`); warn (don't silently default) on roko.toml parse error.
6. **[P1]** Reconcile the `legacy-runner-v2` feature — add the promised `cfg` gates or delete the feature + its Cargo comment.
7. **[P2]** Port agent-driven replan (tasks.toml rewrite) from orchestrate.rs `12329-13508` into the runner via `roko_orchestrator::replan::PlanRevisionRequest`.
8. **[P2]** Decide worktree strategy: wire `WorktreeManager` into runner dispatch or record in-place-is-the-design and delete `worktree.rs`.
9. **[P2]** Split `event_loop.rs` (6.7K) before it becomes the next 23K-line orchestrate.rs.
10. **[P3]** Collapse the 4 state files into `state-snapshot.json`; GC `.bak.*`.

## Cross-references

- `36-ORCHESTRATION-RUNNERS.md` — full three-generation orchestration audit (orchestrate.rs / Runner v2 / Graph handoff, decomposition map).
- `37-RUNNER-V2-AND-GRAPH.md` — operator decision doc (which engine, why the default is hollow).
- `95-ENGINE-DRIFT.md` — the cross-cutting three-engine drift the navigation layer must capture.
- `31-GRAPH-CELLS-ENGINE.md` — the Graph Engine (the dry-run default).
- `60-STATE-PERSISTENCE-LEDGER.md` — `.roko/state/` file map.
- `43-SURFACES-DEMO-UX.md` — Runner v2 as a *surface* (TuiBridge / projection).
