# Timeout snapshot precedes terminal state

- Severity: critical
- Area: persistence / resume

`handle_plan_timeout` saves the snapshot before emitting the terminal run event. The final snapshot therefore says phase `implementing`, `last_error=null`, `started_at_ms=0`, and no assigned agents even though the run failed by timeout.

It also omits failed task IDs, blocked/skipped task IDs, DAG running state, and active/superseded attempts. Only aggregate `tasks_failed=5` and completed IDs survive.

Persist terminal outcome and task-attempt reconciliation first, then atomically write a snapshot that can explain and resume the failure.

