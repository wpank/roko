# Event-Log Replay

> Roko's hash-chained event log provides a tamper-evident record of all state transitions.
> This record enables point-in-time recovery: replaying the log reconstructs any prior
> system state without retaining full snapshots.

**Status**: Built (event log exists and is hash-chained; replay recovery not yet wired to the CLI)
**Crate**: `roko-runtime`, `roko-orchestrator`
**Depends on**: [00-overview.md](00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Every state transition in Roko is recorded in a hash-chained log. The chain means:
tampering with any entry is detectable. Replaying the log from the beginning reconstructs
the state at any point. **Status: Built** — the log is written and the hash chain is
verified; replay-based recovery is not yet exposed in the CLI.

---

## What the Event Log Is

The event log is an append-only sequence of `EventRecord` entries:

```rust
pub struct EventRecord {
    pub seq:        u64,          // Monotonic sequence number
    pub timestamp:  SystemTime,   // Wall-clock time of the event
    pub kind:       EventKind,    // What type of event this is
    pub payload:    Bytes,        // Serialised event payload
    pub prev_hash:  [u8; 32],     // BLAKE3 hash of the previous record
    pub hash:       [u8; 32],     // BLAKE3 hash of (seq + timestamp + kind + payload + prev_hash)
}
```

The `hash` field commits to the entire record, including the `prev_hash` of the
preceding record. This creates a hash chain where:

- The first record's `prev_hash` is the zero hash `[0u8; 32]`.
- Each subsequent record's `prev_hash` equals the previous record's `hash`.
- Modifying any record (including re-ordering) breaks the chain.

---

## Events Recorded

| Event kind | Description |
|-----------|-------------|
| `PlanStarted` | A plan execution began |
| `TaskStarted` | A task began executing |
| `TaskCompleted` | A task completed successfully |
| `TaskFailed` | A task failed (all retries exhausted) |
| `GateVerdict` | A gate produced a verdict (Pass or Fail) |
| `AgentTurn` | An agent turn completed (LLM response received) |
| `EngramPersisted` | An Engram was written to the Substrate |
| `ExecutorSnapshot` | A full executor state snapshot was written |
| `ProcessStart` | The Roko process started |
| `ProcessShutdown` | The Roko process shut down gracefully |

---

## Log Location and Format

The event log is written to `.roko/logs/events.log` in binary format (using `rkyv`
for zero-copy serialisation).

A human-readable export is available:

```bash
roko events show --format json
roko events show --format table --since "2026-04-19 14:00" --until "2026-04-19 15:00"
```

---

## Hash Chain Verification

To verify the chain integrity (detect tampering or corruption):

```bash
roko events verify
```

Output on success:

```
Event log verification: PASS
  Entries: 4,217
  Last hash: a3f7b2c1...
  Chain: intact
```

Output on failure:

```
Event log verification: FAIL
  Entries checked: 892 of 4,217
  Broken link at seq=892: expected prev_hash a3f7..., found 00000...
  This may indicate log corruption or tampering.
  Entries before seq=892 are trustworthy.
```

---

## Replay-Based Recovery

**Status: Built, not yet CLI-accessible.**

The replay recovery reconstructs the executor state at any point in time by:

1. Reading the event log from the beginning.
2. Applying each event to an in-memory state machine.
3. Stopping at the target sequence number or timestamp.
4. Producing an `executor.json` snapshot from the replayed state.

This is useful when the current `executor.json` snapshot is corrupted or inconsistent
(e.g. the process was killed during snapshot write).

**Planned CLI:**

```bash
# Reconstruct executor state at a specific point in time
roko events replay --until "2026-04-19 14:32:00" --output .roko/state/executor.json

# Then resume from the reconstructed state
roko plan run plans/ --resume .roko/state/executor.json
```

---

## Log Rotation

The event log grows indefinitely unless rotated. Rotation is not yet automated. Manual
rotation:

```bash
# Archive the current log
mv .roko/logs/events.log .roko/logs/events-$(date +%Y%m%d).log

# Roko will create a new events.log on the next write
```

After rotation, the hash chain in the new log starts fresh. The archived log retains
its own chain integrity independently.

---

## See Also

- [04-crash-recovery.md](04-crash-recovery.md) — snapshot-based recovery (the primary recovery mechanism)
- [07-forensic-replay.md](07-forensic-replay.md) — using the event log to reproduce failures

## Open Questions

- The `roko events replay` command is planned but not yet implemented.
- Automatic log rotation (size-based or time-based) is planned.
- Event log encryption (for deployments where task descriptions are sensitive) is planned.
