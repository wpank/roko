# Event Log Replay Idempotence

> Replaying an event log twice produces the same state as replaying it once.

**Crate**: `roko-orchestrator`
**Test type**: Unit test
**Enforcement**: `EventLog::replay`
**Last reviewed**: 2026-04-19

---

## Statement

For all event logs L and initial states S₀:
`replay(replay(S₀, L), L) == replay(S₀, L)`

---

## Why It Matters

The crash recovery system replays the event log on startup. If replay were not idempotent, a double-replay (e.g., after a crash during the replay itself) would corrupt state.

---

## Test

```rust
#[test]
fn event_log_replay_idempotent() {
    let ctx = TestContext::new();
    let log = ctx.build_event_log(n_events = 20);

    let state_once = EventLog::replay(&log);
    let state_twice = EventLog::replay_on_state(state_once.clone(), &log);

    assert_eq!(state_once, state_twice,
        "Replaying an event log twice must produce the same state as once");
}
```

---

## See also

- [crash-recovery-consistency.md](crash-recovery-consistency.md)
- [../by-subsystem/subsystem-orchestrator.md](../by-subsystem/subsystem-orchestrator.md)
