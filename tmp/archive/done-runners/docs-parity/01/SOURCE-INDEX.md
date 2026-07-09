# Source Index — Orchestration Spot Checks

This file is a verification aid for the parity pack.

These anchors are spot checks for the main claims in `tmp/docs-parity/01/`. They are not proof that a concept is fully runtime-owned.

---

## A — Core Orchestration

| Claim | Source |
|-------|--------|
| `orchestrate.rs` is the main runtime hotspot | `crates/roko-cli/src/orchestrate.rs` (`17,087` LOC) |
| atomic snapshot write helper | `crates/roko-cli/src/orchestrate.rs:668` |
| `WatcherRunner` | `crates/roko-cli/src/orchestrate.rs:1890` |
| `PlanRunner` struct | `crates/roko-cli/src/orchestrate.rs:2567` |
| `TaskTracker` | `crates/roko-cli/src/orchestrate.rs:2681` |
| `PlanRunner::from_plans_dir()` | `crates/roko-cli/src/orchestrate.rs:3609` |
| runtime `discover_plans(...)` call | `crates/roko-cli/src/orchestrate.rs:3623` |
| `PlanRunner::from_snapshot()` | `crates/roko-cli/src/orchestrate.rs:3858` |
| `PlanRunner::from_snapshots()` | `crates/roko-cli/src/orchestrate.rs:3992` |
| `run_conductor_check()` | `crates/roko-cli/src/orchestrate.rs:4731` |
| `PlanRunner::snapshot()` | `crates/roko-cli/src/orchestrate.rs:5178` |
| `PlanRunner::event_log_snapshot()` | `crates/roko-cli/src/orchestrate.rs:5192` |
| `PlanRunner::save_state()` | `crates/roko-cli/src/orchestrate.rs:5205` |
| `PlanRunner::save_state_to()` | `crates/roko-cli/src/orchestrate.rs:5247` |
| `PlanRunner::run_all()` | `crates/roko-cli/src/orchestrate.rs:5605` |
| main dispatch loop reaches `dispatch_action()` | `crates/roko-cli/src/orchestrate.rs:5704` |
| `dispatch_action()` | `crates/roko-cli/src/orchestrate.rs:6139` |
| `discover_plans()` | `crates/roko-orchestrator/src/plan_discovery.rs:161` |
| `rank_plans()` | `crates/roko-orchestrator/src/plan_discovery.rs:270` |
| `UnifiedTaskDag` | `crates/roko-orchestrator/src/dag.rs:228` |
| `critical_path()` | `crates/roko-orchestrator/src/dag.rs:358` |
| `waves()` | `crates/roko-orchestrator/src/dag.rs:434` |
| `apply_mutation()` | `crates/roko-orchestrator/src/dag.rs:621` |
| `IncrementalDag` | `crates/roko-orchestrator/src/dag.rs:925` |
| `ParallelExecutor` | `crates/roko-orchestrator/src/executor/mod.rs:144` |
| `register_speculative_execution()` | `crates/roko-orchestrator/src/executor/mod.rs:218` |
| `resolve_speculative_execution()` | `crates/roko-orchestrator/src/executor/mod.rs:270` |
| `ExecutorAction` enum | `crates/roko-orchestrator/src/executor/action.rs:19` |
| `PlanStateMachine` | `crates/roko-orchestrator/src/executor/state_machine.rs:92` |
| `transition()` | `crates/roko-orchestrator/src/executor/state_machine.rs:106` |
| `next_action()` | `crates/roko-orchestrator/src/executor/state_machine.rs:227` |
| `PlanPhase` enum | `crates/roko-core/src/phase.rs:157` |

## B — Isolation & Merge

| Claim | Source |
|-------|--------|
| `WorktreeConfig` | `crates/roko-orchestrator/src/worktree.rs:36` |
| `WorktreeHandle` | `crates/roko-orchestrator/src/worktree.rs:57` |
| `WorktreeHealth` | `crates/roko-orchestrator/src/worktree.rs:73` |
| `WorktreeManager` | `crates/roko-orchestrator/src/worktree.rs:134` |
| `ensure_for_plan()` | `crates/roko-orchestrator/src/worktree.rs:281` |
| `touch()` | `crates/roko-orchestrator/src/worktree.rs:354` |
| `check_health()` | `crates/roko-orchestrator/src/worktree.rs:373` |
| `reclaim_idle()` | `crates/roko-orchestrator/src/worktree.rs:420` |
| `prune()` | `crates/roko-orchestrator/src/worktree.rs:512` |
| runtime `ensure_for_plan(...)` call | `crates/roko-cli/src/orchestrate.rs:12757` |
| runtime `touch(plan_id)` call | `crates/roko-cli/src/orchestrate.rs:12766` |
| `cleanup_plan_worktree()` | `crates/roko-cli/src/orchestrate.rs:12770` |
| `cleanup_tracked_plan_worktrees()` | `crates/roko-cli/src/orchestrate.rs:12780` |
| `MergeRequest` | `crates/roko-orchestrator/src/merge_queue.rs:28` |
| `MergeQueue` | `crates/roko-orchestrator/src/merge_queue.rs:131` |
| `next_mergeable()` | `crates/roko-orchestrator/src/merge_queue.rs:163` |
| `mark_merging()` | `crates/roko-orchestrator/src/merge_queue.rs:198` |
| `mark_complete()` | `crates/roko-orchestrator/src/merge_queue.rs:223` |
| `mark_failed()` | `crates/roko-orchestrator/src/merge_queue.rs:239` |
| `PostMergeRunner` | `crates/roko-orchestrator/src/post_merge.rs:106` |
| no `MergeQueue` runtime call site found in `orchestrate.rs` | spot check via `rg -n "MergeQueue|next_mergeable|mark_merging|mark_complete|mark_failed" crates/roko-cli/src/orchestrate.rs` |

## C — Persistence & Recovery

| Claim | Source |
|-------|--------|
| `ExecutorSnapshot` | `crates/roko-orchestrator/src/executor/snapshot.rs:28` |
| `to_json()` | `crates/roko-orchestrator/src/executor/snapshot.rs:73` |
| `from_json()` | `crates/roko-orchestrator/src/executor/snapshot.rs:83` |
| `RecoveryEngine` | `crates/roko-orchestrator/src/executor/recovery.rs:127` |
| `recover_from_snapshot()` | `crates/roko-orchestrator/src/executor/recovery.rs:147` |
| `recover_from_event_log()` | `crates/roko-orchestrator/src/executor/recovery.rs:193` |
| `merge_recovery()` | `crates/roko-orchestrator/src/executor/recovery.rs:252` |
| `validate_recovery()` | `crates/roko-orchestrator/src/executor/recovery.rs:294` |
| `EventKind` enum | `crates/roko-orchestrator/src/event_log.rs:28` |
| `EventLog` | `crates/roko-orchestrator/src/event_log.rs:179` |
| `verify_integrity()` | `crates/roko-orchestrator/src/event_log.rs:245` |
| `snapshot()` | `crates/roko-orchestrator/src/event_log.rs:281` |
| shared runtime bus `RokoEvent` | `crates/roko-runtime/src/event_bus.rs:103` |
| shared bus `PlanRevision` variant | `crates/roko-runtime/src/event_bus.rs:105` |
| shared bus `PrdPublished` variant | `crates/roko-runtime/src/event_bus.rs:120` |

## D — Monitoring & Conductor

| Claim | Source |
|-------|--------|
| `WatcherRunner` | `crates/roko-cli/src/orchestrate.rs:1890` |
| background watcher loop calls `check_all()` | `crates/roko-cli/src/orchestrate.rs:1925` |
| local event log records `InterventionFired` | `crates/roko-cli/src/orchestrate.rs:4707` |
| runtime reads conductor routing bias | `crates/roko-cli/src/orchestrate.rs:11219` |
| `Conductor` | `crates/roko-conductor/src/conductor.rs:53` |
| `check_all()` | `crates/roko-conductor/src/conductor.rs:109` |
| `routing_bias()` | `crates/roko-conductor/src/conductor.rs:146` |
| `evaluate()` | `crates/roko-conductor/src/conductor.rs:156` |
| conductor depends on `roko-learn` | `crates/roko-conductor/Cargo.toml:15` |
| watcher imports `AgentEfficiencyEvent` | `crates/roko-conductor/src/watchers/context_window_pressure.rs:7` |
| watcher reads `AgentEfficiencyEvent` from payload | `crates/roko-conductor/src/watchers/context_window_pressure.rs:94` |

## E — Deferred Coordination / Domains

| Claim | Source |
|-------|--------|
| `Kind::Pheromone` exists | `crates/roko-core/src/kind.rs:92` |
| pheromone decay constants exist | `crates/roko-core/src/decay.rs:97`, `:101`, `:105` |
| `FleetCFactor` is reporting, not a stigmergy runtime | `crates/roko-learn/src/efficiency.rs:427`, `:585` |
| tier routing primitives exist | `crates/roko-primitives/src/tier.rs:67` |
| cross-domain strategy hypotheses are offline/background | `crates/roko-dreams/src/cycle.rs:1822` |

## Overstated Source Docs

| Claim To Re-read Carefully | Source |
|----------------------------|--------|
| doc `12` overview reads more concrete than the runtime really is | `docs/01-orchestration/12-stigmergy-niche.md:10`, `:23`, `:93` |
| doc `13` overview is broader than the current runtime | `docs/01-orchestration/13-cross-domain-orchestration.md:10`, `:17`, `:108` |
| doc `13` future sections are explicitly not batch-01 runtime | `docs/01-orchestration/13-cross-domain-orchestration.md:299`, `:347`, `:439`, `:524`, `:646` |

## Stale Claims Fixed In This Refresh

- `PlanRunner::from_plans_dir()` is at `:3609`, not the older inherited anchor.
- the runtime `discover_plans(...)` call is at `:3623`.
- state-save anchors are clearer as `snapshot() :5178`, `event_log_snapshot() :5192`, `save_state() :5205`, and `save_state_to() :5247`.
- the main runtime loop should anchor to `run_all() :5605`, not a loose range.
- `WorktreeHealth` is an enum at `worktree.rs:73`.
- the shared runtime bus evidence should point to `RokoEvent` and its 2 variants, not to a richer event taxonomy.
- local `InterventionFired` logging is the orchestrator event-log path, not the shared runtime bus.

## Not Found As Live Batch-01 Runtime

- formal stigmergy API
- orchestrator-owned pheromone model
- cross-domain chain execution path
- template system
- saga coordinator
- semantic merge engine
- dedicated plan repair engine
