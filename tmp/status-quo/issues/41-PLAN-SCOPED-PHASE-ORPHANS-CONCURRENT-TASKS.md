# Plan-scoped phase machine orphans concurrent tasks

- Severity: critical
- Area: executor state machine

`apply_agent_completion` consults only the current plan phase (`event_loop.rs:2312-2346`). With multiple tasks active, the first completion moves the shared plan to Gating. Later sibling completions are logged as `agent completion ignored for phase` and lose their lifecycle transition.

The run logged ignored completions at 15:32:17, 15:37:45, 15:38:12, 15:39:57, and 15:43:08 local-time windows. Gate results likewise write into one plan-level collection.

Task phase, attempt, gate, and completion state must be keyed by plan/task/attempt. Until then, concurrency within one plan is unsafe and should be disabled.

