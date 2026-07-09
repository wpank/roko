# orchestrate.rs — Structural Analysis & Refactor Plan

## The Problem

`crates/roko-cli/src/orchestrate.rs` is **21,478 lines**. `PlanRunner` has **250 methods
across 15,059 lines in 3 separate impl blocks**. It's a god object.

## File Metrics

| Metric | Value |
|--------|-------|
| Total lines | 21,478 |
| Code lines (before tests) | 19,042 |
| Test lines | 2,436 (11.3%) |
| Import statements | 100 |
| Struct definitions | 32 |
| Enum definitions | 2 |
| Trait implementations | 6 |
| Private functions | 127 |
| Public functions | 1 |

## Type Breakdown

### PlanRunner: The God Object
- **250 methods** across 3 impl blocks
- **15,059 lines** total
- Block 1: lines 3587-17619 (14,033 lines, 206 methods) — core execution
- Block 2: lines 17620-18343 (724 lines, 31 methods) — git/worktree ops
- Block 3: lines 18344-18645 (302 lines, 13 methods) — dashboard/observability

### Other Large Types (self-contained, easy to extract)
- `ReviewDriftReport`: 99 methods, 2,833 lines (lines 18646-21478)
- `StaticCFactorSource`: 33 methods, 644 lines (lines 344-987)
- `TaskTracker`: 30 methods, 574 lines (lines 3013-3586)
- `WatcherRunner`: 12 methods, 564 lines (lines 1990-2112+)
- `EnrichmentPhaseSummary`: 9 methods, 446 lines (lines 1473-1512+)
- `CrateFamiliarityTracker`: 10 methods, 302 lines (lines 1095-1360+)
- `ReplanLedger`: 12 methods, 269 lines (lines 2577-2634+)

## Crate Boundaries: Actually Sound

orchestrate.rs does NOT reimplement crate logic. It correctly uses:

| Crate | API Used | Duplication? |
|-------|----------|-------------|
| roko-orchestrator | ParallelExecutor, ExecutorAction/Event | None |
| roko-conductor | Conductor, HealthMonitor, StuckDetector | None |
| roko-runtime | ProcessSupervisor, CancelToken | None |
| roko-learn | LearningRuntime, CascadeRouter, EpisodeLogger | None |
| roko-compose | RoleSystemPromptSpec, EnrichmentPipeline | None |
| roko-gate | run_rung(), GatePipeline | Thin wrappers (appropriate) |
| roko-agent | TaskRunner, SafetyLayer, ToolDispatcher | None |

The problem is NOT wrong boundaries — it's that the runtime harness piled up in one file.

## Proposed Module Decomposition

Split orchestrate.rs into ~14 focused files. Zero behavior change — just file moves.

### Phase 1: Extract self-contained types (easy, no PlanRunner changes)

| New File | Type | Lines | Risk |
|----------|------|-------|------|
| `review_drift.rs` | ReviewDriftReport | ~2,900 | Zero — standalone type |
| `cfactor_source.rs` | StaticCFactorSource + SectionEffectCatalystSource | ~700 | Zero — trait impls |
| `task_tracker.rs` | TaskTracker + PreAgentRemediation* + ReplanLedger | ~900 | Zero — standalone |
| `enrichment.rs` | EnrichmentPhaseSummary + EnrichmentRuntimeClient | ~500 | Zero — standalone |
| `watcher.rs` | WatcherRunner | ~600 | Zero — standalone |
| `crate_familiarity.rs` | CrateFamiliarityTracker + ContextAttribution* | ~400 | Zero — standalone |
| `oracle_adapters.rs` | PerplexitySearchOracle + AgentJudgeOracle | ~100 | Zero — trait impls |

**Phase 1 total**: ~6,100 lines extracted. orchestrate.rs drops to ~15,400 lines.

### Phase 2: Extract PlanRunner methods into focused modules

| New File | Methods | Lines | Dependencies |
|----------|---------|-------|-------------|
| `dispatch.rs` | dispatch_agent_with + model routing + budget | ~2,000 | PlanRunner fields |
| `enrichment_phase.rs` | handle_enriching + run_enrichment_pipeline | ~800 | PlanRunner fields |
| `gate_phase.rs` | run_gate_pipeline + rung config + verdicts | ~1,200 | PlanRunner fields |
| `learning_feedback.rs` | enrich_completed_run + record_and_check_learning | ~1,000 | PlanRunner fields |
| `context.rs` | build_relevant_context_layer + knowledge queries | ~1,500 | PlanRunner fields |
| `replan_phase.rs` | build_gate_failure_plan_revision + replan logic | ~800 | PlanRunner fields |
| `git_ops.rs` | ensure_plan_exec_dir + commit + merge | ~700 | PlanRunner fields |
| `events.rs` | emit_server_event + publish_dashboard_event | ~600 | PlanRunner fields |

**Pattern**: Each module gets `impl PlanRunner` methods in its own file.
Rust allows this natively — `impl Foo` blocks can live in any file in the crate.

**Phase 2 total**: ~8,600 lines extracted. Core `plan_runner.rs` drops to ~5,000 lines
(the state machine loop, shutdown, field definitions).

### Phase 3: Move helpers to standalone modules

| New File | Functions | Lines |
|----------|-----------|-------|
| `helpers.rs` | Path resolution, scrubbing, config loading | ~800 |
| `types.rs` | DispatchOutcome, OrchestrationReport, AgentRunConfig, etc. | ~500 |
| `run_prepared.rs` | run_prepared_agent() free function | ~250 |

### Final State

```
crates/roko-cli/src/orchestrate/
  mod.rs              — re-exports, PlanRunner struct def (~500 lines)
  plan_runner.rs      — run_all, run_task_plans, shutdown (~3,000 lines)
  dispatch.rs         — dispatch_agent_with + routing (~2,000 lines)
  enrichment_phase.rs — handle_enriching (~800 lines)
  gate_phase.rs       — run_gate_pipeline (~1,200 lines)
  learning_feedback.rs — episode recording (~1,000 lines)
  context.rs          — context assembly (~1,500 lines)
  replan_phase.rs     — replan logic (~800 lines)
  git_ops.rs          — git operations (~700 lines)
  events.rs           — dashboard/server events (~600 lines)
  task_tracker.rs     — TaskTracker (~900 lines)
  review_drift.rs     — ReviewDriftReport (~2,900 lines)
  cfactor_source.rs   — CFactorSource impls (~700 lines)
  enrichment.rs       — EnrichmentPhaseSummary (~500 lines)
  watcher.rs          — WatcherRunner (~600 lines)
  helpers.rs          — free functions (~800 lines)
  types.rs            — shared types (~500 lines)
  oracle_adapters.rs  — gate oracle impls (~100 lines)
  tests.rs            — all tests (~2,400 lines)
```

~21,500 lines across 19 files. Average: ~1,130 lines/file. Largest: plan_runner.rs at ~3,000.

## Should We Rewrite From Scratch?

**No.** The architecture is sound:
- Crate boundaries are correct — no logic duplication
- The unified spec (tmp/unified/) defines a clear 4-phase migration
- ParallelExecutor already provides the pure state machine
- PlanRunner is the correct runtime harness pattern

The problem is file organization, not architecture. Module extraction is mechanical
and risk-free. A rewrite would lose 21K lines of battle-tested edge case handling
(budget pressure, cost anomaly recovery, stuck detection, replan logic, etc.) that
took months to build.

## Relationship to Unified Architecture

The unified spec (tmp/unified-migration/) defines 4 phases:
- **Phase 0** (current): Wire dead code, fix gaps → what we're doing now
- **Phase 1**: Kernel rename (Engram→Signal, etc.) + Predict-Publish-Correct
- **Phase 2**: Graph engine + Flow executor + Type-state agents → THIS replaces the
  imperative dispatch loop with declarative cell composition
- **Phase 3**: Economy + on-chain registries

Phase 2 is when PlanRunner's dispatch loop becomes a Graph executor. Until then,
module extraction keeps the code maintainable without architectural changes.
