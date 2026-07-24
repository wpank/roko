# Roko Gaps Tracker

Canonical list of unfinished items. Check before starting new work.

Last updated: 2026-07-24 (Wave 7 cleanup pass).

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

## Wave 7+ cleanup findings (2026-07-24)

### Dead code and orphans (addressed)
- **Deleted**: `roko-core/src/state_hub.rs` and `pulse_bus.rs` — orphan files not declared in `lib.rs`, duplicates of wired copies in `roko-runtime`.
- **Fixed**: Broken doc-links in `bus_backends.rs`, `traits.rs`, `dashboard_snapshot.rs` referencing deleted modules.
- **Fixed**: Clippy `useless_conversion` in `roko-orchestrator/src/worktree.rs:3640`.

### Documentation gaps
- **`loop_tick` not wired**: `roko-core/src/loop_tick.rs` defines the universal loop but `runner/event_loop.rs` reimplements inline. Tracked under E01/E22.
- **VCG auction cold-start**: `CompositionStrategy::Auto` always resolves to `DensityGreedy` because the bidder observation registry starts empty. VCG activates only after all bidders reach 10 observations. Not a bug — by design.
- **`legacy-runner-v2` feature**: Cargo.toml comment was misleading (claimed it controlled binary behavior). Fixed: it only gates integration tests.

### Pre-existing issues (not yet addressed)
- **roko-cli compile errors**: 24 errors in `roko-cli` lib target (pre-existing, not from cleanup). Likely from recent refactoring that moved types. Needs investigation under E01/E03.
- **roko-orchestrator test failures**: 7 of 544 tests fail (pre-existing). Likely worktree/git-related tests that depend on repository state.
- **Phase-2 stubs in daimon/dreams**: `phase2_stubs.rs` has 4 `#[allow(dead_code)]` items and `replay.rs` has 1. These are intentional (not yet wired) — documented with module-level comments.
- **roko-chain Engram duplicate**: `identity_economy_markets.rs:653` defines a local `Engram` struct that duplicates `roko_core::Engram`. The gate modules use the canonical core version. The local stub is part of the phase-2 economy types — cleanup tracked under E03-T07.
