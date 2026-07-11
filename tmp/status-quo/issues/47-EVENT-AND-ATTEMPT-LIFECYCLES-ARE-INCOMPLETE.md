# Event and attempt lifecycles are incomplete

- Severity: high
- Area: event schema / replay

For this run, 13 task attempts started but only 10 completed. T08, T07 attempt 1, and T09 attempt 1 have no terminal attempt. There were 11 dispatch completions and agent completions but only 9 `agent.started` events. T11/T12 complete without starts.

T01 emits two starts and two completions both labeled attempt 1. T07/T09 jump from attempt 1 to attempt 3 with no attempt 2 lifecycle. No plan-failed, timeout, blocked-task, cancellation, or cleanup events exist.

Enforce lifecycle invariants at write time and test replay: every start has one terminal, attempt IDs are monotonic and unique, and every run/plan ends explicitly.

