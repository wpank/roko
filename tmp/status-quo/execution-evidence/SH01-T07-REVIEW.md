# SH01-T07 independent review — rejected

- Reviewed candidate: `79cf3ececb5c51f2b0a372b257343384135adff6`
- Reviewed integrated base: `main` at `7b35442c67000c79d7dcc4ffff548400f7bdcc37`
- Review date: 2026-07-21
- Verdict: **REJECTED**

## Verification

The task-prescribed command completed successfully:

```text
cargo test -p roko-cli runner::event_loop
53 passed; 0 failed
```

The command emitted the existing `plan_validation` missing-documentation warning,
but no test failure.

## Findings

1. The SH01-T07 description and issue 46 require exact cancelled and orphaned
   categories. `PlanRunSummary`, `RunTotals`, and `build_plan_report` expose
   only completed, failed, blocked, skipped, active, and pending, so cancelled
   and orphaned tasks cannot be reported truthfully.
2. `build_plan_report` hard-codes `tasks_active` to zero. A live task in
   `RunState` is therefore reported as pending rather than active.
3. `task_status_is_terminal` considers the declared status `skipped` terminal,
   and `build_plan_report` then counts that task as completed. It neither
   reports the task as skipped nor supplies a reason.
4. The candidate tests cover aggregate arithmetic and all-done plans, but do
   not cover active, cancelled, orphaned, declared-skipped, or transitively
   skipped dependency cases.

## Required correction

Derive each task's category from lifecycle/attempt state with mutually
exclusive passed, failed/exhausted/timed-out, cancelled, and orphaned terminal
categories; separately report blocked and skipped tasks with reasons; derive
active tasks from live state; and map declared skipped tasks to skipped with a
declared reason. Extend the backward-compatible event payload and add regression
coverage for every category plus the per-plan/global invariant.
