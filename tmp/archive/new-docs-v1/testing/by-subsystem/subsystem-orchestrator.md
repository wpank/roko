# roko-orchestrator — Test Coverage

> 158 tests for the L4 orchestration layer: plan DAG scheduling, crash recovery, and git worktree isolation.

**Status**: Shipping
**Crate**: `roko-orchestrator`
**Section**: 01 — Orchestration
**Last reviewed**: 2026-04-19

---

## Test Count: 158

Source: implementation status audit, 2026-04-17.

| Module | Approx. tests | Focus |
|---|---|---|
| `parallel_executor` | ~50 | DAG scheduling, task dispatch, dependency ordering |
| `plan_dag` | ~30 | DAG construction, cycle detection, topological sort |
| `crash_recovery` | ~30 | Event-log replay, checkpoint restore, resume semantics |
| `worktree` | ~20 | Git worktree creation, isolation, conflict queue |
| `merge_queue` | ~18 | File-conflict-aware merge serialization |
| `event_log` | ~10 | Hash-chained event log append/replay |

---

## Key Test Focus Areas

### ParallelExecutor

- Tasks with no dependencies dispatch immediately.
- Tasks with dependencies dispatch only after all dependencies complete.
- A task that fails does not block independent tasks.
- The executor terminates cleanly when all tasks complete or when max failures are reached.
- Cancellation propagates: cancelling the executor stops all in-flight tasks.

### Plan DAG

- Construction: duplicate task IDs are rejected.
- Cycle detection: a plan DAG with a cycle returns `Err(CycleDetected)`.
- Topological sort: tasks are always dispatched in a valid topological order.
- Empty plan: a plan with zero tasks completes immediately.

### Crash Recovery

- Event log replay: after a crash, replaying the event log restores the exact pre-crash state.
- Resume: `plan resume` continues from the last checkpoint without re-executing completed tasks.
- Idempotent replay: replaying the same event log twice produces the same result as replaying it once.
- Partial completion: a plan that completed 3/7 tasks before crash resumes from task 4.

### Git Worktrees

- Each task gets an isolated git worktree in a temp directory.
- Two tasks working on different files do not conflict.
- Worktrees are cleaned up after task completion (success or failure).

### Merge Queue

- Two tasks modifying the same file are serialized through the conflict queue.
- The merge order matches the task completion order.
- A merge conflict failure quarantines the task without blocking the queue.

---

## Integration Tests

The orchestrator integration tests (in `roko-orchestrator/tests/`) assemble the orchestrator with:
- `roko-agent` (LLM calls replayed from tape).
- `roko-gate` (gate pipeline with permissive config).
- `roko-fs` (JSONL substrate in a temp dir).
- `roko-learn` (episode logger).

Key integration scenarios:
- Full plan execution with 3 tasks, 2 in parallel, 1 sequential dependency.
- Plan execution where gate rejects task 2; tasks 3 and 4 are still attempted if independent.
- Crash-and-resume across the orchestrator ↔ agent boundary.

---

## Property Tests

| Property | Test name |
|---|---|
| Plan DAG acyclicity | `plan_dag_never_has_cycles` |
| Topological order is valid | `topo_sort_respects_dependencies` |
| Crash recovery idempotence | `event_log_replay_idempotent` |
| Task execution completeness | `all_tasks_eventually_dispatched` |

See [../by-property/plan-dag-acyclicity.md](../by-property/plan-dag-acyclicity.md).

---

## Known Gaps

- No stress test for plans with > 100 tasks.
- Merge queue correctness under 3+ concurrent conflicting tasks is tested only for 2.
- `roko-runtime` (process supervisor) has 0 unit tests; its correctness is tested only through orchestrator integration tests.

## See also

- [../by-property/plan-dag-acyclicity.md](../by-property/plan-dag-acyclicity.md)
- [../by-property/crash-recovery-consistency.md](../by-property/crash-recovery-consistency.md)
- [../tiers/05-end-to-end-tests.md](../tiers/05-end-to-end-tests.md) — orchestrator E2E
