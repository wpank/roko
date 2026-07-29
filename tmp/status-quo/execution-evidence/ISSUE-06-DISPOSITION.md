# Issue 06 disposition — timeout reports stale in-flight attempt

- Issue: tmp/status-quo/issues/06-timeout-reports-stale-in-flight-attempt.md
- Disposition: **RESOLVED** by SH01-T05
- Merged evidence: SH01-T05 (retry classification and attempt rollover)

## Resolution

SH01-T05 unified retry classification so that attempt state transitions are
atomic and exclusive. A completed task's attempt is terminalized before any
timeout reporting reads it. The `RetryDecision` type classifies failures as
Transient/Persistent/Fatal, and exhaustion clears all active/retrying state.

## Verification

```
cargo test -p roko-cli runner::state       # attempt lifecycle tests
cargo test -p roko-cli runner::gate_dispatch # gate terminalization tests
cargo test -p roko-cli runner::event_loop   # 76 pass, 0 fail
```

All SH01 tasks (28/28) are merged to main. The timeout reporting path now
reads only from the lifecycle projection, which is the single source of
truth for attempt state.
