# Roko Gaps Tracker

Canonical list of unfinished items. Check before starting new work.

## Tasks 101-103 (Wave 5: Migration + Hot Graphs)

### Task 101: Plan-to-Graph Converter
- **TaskExecutorCell live dispatch**: The `dry_run: false` path in `TaskExecutorCell.execute()` falls back to dry-run behavior with a warning. The real implementation should delegate to the Runner v2 agent dispatch path (or the new Engine dispatch path when it replaces Runner v2). Subsystem: `roko-graph/src/cells/task_executor.rs`.
- **Graph Engine snapshot/resume**: The `--resume-plan` flag is not yet supported on the Graph Engine path. Implementing this requires state serialization between graph executions. Subsystem: `roko-cli/src/commands/plan.rs`.

### Task 102: Engine as Default
- **Runner v2 feature gate coverage**: Only the `PlanCmd::Run` dispatch in `commands/plan.rs` is gated. Other callers of Runner v2 internals (e.g., `runner::plan_loader`) remain compiled unconditionally because they serve other commands too (plan list, plan show). A future cleanup pass should assess whether runner internals need tighter gating.
- **Graph Engine parallel execution**: The current Engine executes nodes sequentially in topological order. The `max_parallel` metadata from plans is stored but not used for parallel node dispatch. Subsystem: `roko-graph/src/engine.rs`.

### Task 103: Hot Graphs + Cognitive Loop
- **Real cell implementations**: All 7 cognitive loop cells (`signal-reader`, `relevance-scorer`, `system-prompt-builder`, `claude-agent`, `gate-pipeline`, `store-writer`, `event-publisher`) use `PassthroughCell` stubs. Each needs a real implementation. Subsystem: `roko-graph/src/cells/`.
- **Hot Graph state persistence**: `HotPolicy.persist_tick_state` is defined but not implemented. The tick loop does not save/restore cell outputs between ticks. Subsystem: `roko-graph/src/hot.rs`.
- **TOML `[graph.policy.hot]` parsing**: The loader does not parse `[graph.policy.hot]` sections from TOML files. HotPolicy must be constructed programmatically. Subsystem: `roko-graph/src/loader.rs`.
- **Conditional edge evaluation**: Edges in cognitive-loop.toml note conditions (e.g., "only proceed if relevance above threshold") but the Engine treats all edges as unconditional. Subsystem: `roko-graph/src/engine.rs`.
