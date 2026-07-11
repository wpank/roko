# Blocked plans are marked Implementing

- Severity: high
- Run: `run-1783779617962`

At startup the runner logged `dispatching plan` for SH01 through SH06 and persisted every plan with `current_phase=implementing`, even though cross-plan dependencies make only SH01 runnable. Assigned agents are empty for every plan.

This makes plan progress and active-plan counts false and can interact badly with plan-scoped timeout/phase logic. Plans should remain queued/blocked with explicit dependency reasons until their first task is eligible and capacity is reserved.

