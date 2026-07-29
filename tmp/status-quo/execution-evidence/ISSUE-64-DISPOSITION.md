# Issue 64 disposition — task timing and exit reconciliation are wrong

- Issue: tmp/status-quo/issues/64-TASK-TIMING-AND-EXIT-RECONCILIATION-ARE-WRONG.md
- Disposition: **RESOLVED** by SH01-T03, SH01-T06C1, SH01-T07
- Merged evidence: full SH01 plan (28/28 done) + T07 fix at 88b3a31

## Resolution

SH01-T03 (atomic terminalization) ensures the durable task duration equals
the complete attempt lifecycle from dispatch through terminalization.

SH01-T06C1 records attempt, phase/effect, and agent-activity clocks in exact
ownership. Phase durations (dispatch, agent, gate) are preserved separately.

SH01-T07 fix (88b3a31) added `TaskPhaseDurations` struct with per-phase
timing (dispatch_ms, agent_ms, gate_ms, cleanup_ms) and `total_ms()` method.
The `TaskAttemptCompleted` event now carries `phase_durations` alongside
`duration_ms`, and `task_attempt_completed_with_timing` constructs the event
with both fields consistent.

Exit code reconciliation is handled by SH01-T06B2B2 (structured settle
failures remain owned and are durable failures). The agent completion pathway
normalizes raw `agent.exited` into structured `agent.completed` before
writing the terminal event.

## Verification

```
cargo test -p roko-cli runner::event_loop   # 76 pass, timing tests included
cargo test -p roko-cli runner::types        # 15 pass, TaskPhaseDurations tests
cargo test -p roko-cli --test e2e_self_host # phase duration assertions pass
```
