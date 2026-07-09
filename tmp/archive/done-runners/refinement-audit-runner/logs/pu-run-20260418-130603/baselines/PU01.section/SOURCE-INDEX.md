# Source Index — Orchestration Spot Checks

This file is a verification aid for the parity pack.

These anchors are spot checks for the main claims in `tmp/docs-parity/01/`. They are not proof that a concept is fully runtime-owned.

---

## A — Core Orchestration

| Claim | Source |
|-------|--------|
| `orchestrate.rs` is the main runtime hotspot | `crates/roko-cli/src/orchestrate.rs` (`17,087` LOC) |
| atomic snapshot write helper | `crates/roko-cli/src/orchestrate.rs:668` |
| `PlanRunner` struct | `crates/roko-cli/src/orchestrate.rs:2567` |
| `TaskTracker` | `crates/roko-cli/src/orchestrate.rs:2681` |
| runtime `discover_plans(...)` call | `crates/roko-cli/src/orchestrate.rs:3623` |
| `PlanRunner::from_snapshot()` | `crates/roko-cli/src/orchestrate.rs:3858` |
| `PlanRunner::from_snapshots()` | `crates/roko-cli/src/orchestrate.rs:3992` |
| `run_conductor_check()` | `crates/roko-cli/src/orchestrate.rs:4731` |
| state snapshot save surface | `crates/roko-cli/src/orchestrate.rs:5178-5260` |
| main runtime loop | `crates/roko-cli/src/orchestrate.rs:5608-5704` |
| background watcher task spawn | `crates/roko-cli/src/orchestrate.rs:6065-6095` |
| `dispatch_action()` | `crates/roko-cli/src/orchestrate.rs:6139` |
| `discover_plans()` | `crates/roko-orchestrator/src/plan_discovery.rs:161` |
| `rank_plans()` | `crates/roko-orchestrator/src/plan_discovery.rs:270` |
| `UnifiedTaskDag` | `crates/roko-orchestrator/src/dag.rs:228` |
| `critical_path()` | `crates/roko-orchestrator/src/dag.rs:358` |
| `waves()` | `crates/roko-orchestrator/src/dag.rs:434` |
| `fuse_linear_chains()` | `crates/roko-orchestrator/src/dag.rs:529` |
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
| runtime worktree cleanup | `crates/roko-cli/src/orchestrate.rs:12770-12780` |
| `MergeRequest` | `crates/roko-orchestrator/src/merge_queue.rs:28` |
| `MergeQueue` | `crates/roko-orchestrator/src/merge_queue.rs:131` |
| `next_mergeable()` | `crates/roko-orchestrator/src/merge_queue.rs:163` |
| `mark_merging()` | `crates/roko-orchestrator/src/merge_queue.rs:198` |
| `mark_complete()` | `crates/roko-orchestrator/src/merge_queue.rs:223` |
| `mark_failed()` | `crates/roko-orchestrator/src/merge_queue.rs:239` |
| `PostMergeRunner` | `crates/roko-orchestrator/src/post_merge.rs:106` |

## C — Persistence & Recovery

| Claim | Source |
|-------|--------|
| `ExecutorSnapshot` | `crates/roko-orchestrator/src/executor/snapshot.rs:28` |
| snapshot JSON helpers | `crates/roko-orchestrator/src/executor/snapshot.rs:73-83` |
| `RecoveryEngine` | `crates/roko-orchestrator/src/executor/recovery.rs:127` |
| `recover_from_snapshot()` | `crates/roko-orchestrator/src/executor/recovery.rs:147` |
| `recover_from_event_log()` | `crates/roko-orchestrator/src/executor/recovery.rs:193` |
| `merge_recovery()` | `crates/roko-orchestrator/src/executor/recovery.rs:252` |
| `validate_recovery()` | `crates/roko-orchestrator/src/executor/recovery.rs:294` |
| `EventKind` enum | `crates/roko-orchestrator/src/event_log.rs:28` |
| `EventLog` | `crates/roko-orchestrator/src/event_log.rs:179` |
| `verify_integrity()` | `crates/roko-orchestrator/src/event_log.rs:245` |
| `snapshot()` | `crates/roko-orchestrator/src/event_log.rs:281` |
| shared runtime bus `RokoEvent` | `crates/roko-runtime/src/event_bus.rs:103-123` |

## D — Monitoring & Conductor

| Claim | Source |
|-------|--------|
| `WatcherRunner` | `crates/roko-cli/src/orchestrate.rs:1890-1935` |
| learned conductor policy imported by runtime | `crates/roko-cli/src/orchestrate.rs:83-88` |
| background watcher loop calls `check_all()` | `crates/roko-cli/src/orchestrate.rs:1925` |
| local event log records `InterventionFired` | `crates/roko-cli/src/orchestrate.rs:4707` |
| runtime reads conductor routing bias | `crates/roko-cli/src/orchestrate.rs:11219` |
| `Conductor` | `crates/roko-conductor/src/conductor.rs:53` |
| `check_all()` | `crates/roko-conductor/src/conductor.rs:109` |
| `routing_bias()` | `crates/roko-conductor/src/conductor.rs:146` |
| `evaluate()` | `crates/roko-conductor/src/conductor.rs:156` |
| conductor depends on `roko-learn` | `crates/roko-conductor/Cargo.toml:13-21` |
| watcher imports `AgentEfficiencyEvent` | `crates/roko-conductor/src/watchers/context_window_pressure.rs:7,94` |

## E — Deferred Coordination / Domains

| Claim | Source |
|-------|--------|
| `Kind::Pheromone` exists | `crates/roko-core/src/kind.rs:92` |
| pheromone decay constants exist | `crates/roko-core/src/decay.rs:97-105` |
| `FleetCFactor` is real reporting, not a stigmergy runtime | `crates/roko-learn/src/efficiency.rs:427,585` |
| cross-domain hypothesis generation is background work | `crates/roko-dreams/src/cycle.rs:494,1622` |
| T0/T1/T2 tier routing exists | `crates/roko-primitives/src/tier.rs:69-71` |

## Overstated Source Docs

| Claim To Re-read Carefully | Source |
|----------------------------|--------|
| doc `12` reads more concrete than the runtime really is | `docs/01-orchestration/12-stigmergy-niche.md:12-19,60-89` |
| doc `13` is broader than the current shipped domain runtime | `docs/01-orchestration/13-cross-domain-orchestration.md:12-19,64-77,149-157` |

## Not Found As Live Batch-01 Runtime

- formal stigmergy API
- orchestrator-owned pheromone model
- cross-domain chain execution path
- template system
- saga coordinator
- semantic merge engine
- dedicated plan repair engine
