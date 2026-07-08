# Crash Recovery Consistency

> After a crash and resume, the plan state is identical to what it would have been with no crash. No task is executed twice; no completed task is re-executed.

**Crate**: `roko-orchestrator`
**Test type**: Property-based (proptest) + integration test
**Enforcement**: `EventLog::replay`, `ParallelExecutor::resume`
**Last reviewed**: 2026-04-19

---

## Statement

For all plans P and all crash points C (where 0 ≤ C ≤ len(P)):

Let S_clean = final state of executing P without crashes.
Let S_crash = final state of executing P with a crash at C, then resuming.

`S_clean == S_crash`

And additionally: each task in P was executed exactly once.

---

## Why It Matters

The self-hosting loop runs long multi-agent plans (hours). Crashes are expected. The recovery guarantee is that no work is lost and no work is duplicated. The event log is a hash-chained append-only record; replaying it exactly reconstructs the pre-crash state.

---

## Enforcement

The `ParallelExecutor` writes a `TaskStarted` event before dispatching a task. On resume, tasks with a `TaskStarted` event but no `TaskCompleted` event are re-dispatched. Tasks with `TaskCompleted` are skipped.

```rust
enum Event {
    PlanStarted { plan_id, timestamp },
    TaskStarted { task_id, timestamp },
    TaskCompleted { task_id, verdict, timestamp },
    TaskFailed { task_id, error, timestamp },
    PlanCompleted { plan_id, timestamp },
}
```

Each event's hash covers the previous event's hash (hash chain), making tampering detectable.

---

## Property Tests

```rust
proptest! {
    #[test]
    fn crash_recovery_idempotent(
        plan in arb_plan_dag(max_tasks = 10),
        crash_at_task in 0..10usize,
    ) {
        let (clean_state, _) = run_plan_clean(&plan);
        let (crashed_state, _) = run_plan_with_crash_and_resume(&plan, crash_at_task);

        prop_assert_eq!(clean_state, crashed_state,
            "Plan state after crash+resume must equal clean execution state");
    }
}
```

---

## Related Properties

- [event-log-replay-idempotence.md](event-log-replay-idempotence.md)
- [plan-dag-acyclicity.md](plan-dag-acyclicity.md)

## See also

- [../by-subsystem/subsystem-orchestrator.md](../by-subsystem/subsystem-orchestrator.md)
- [../tiers/05-end-to-end-tests.md](../tiers/05-end-to-end-tests.md)
