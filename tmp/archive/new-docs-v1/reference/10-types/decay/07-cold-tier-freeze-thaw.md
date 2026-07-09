# Decay — Cold-Tier Freeze / Thaw

> The mechanism by which low-weight Engrams are frozen into a cold storage tier, suspending decay, and thawed back to warm storage on access.

**Status**: Shipping  
**Crate**: `roko-core`, `roko-fs`  
**Depends on**: [Overview](00-overview.md), [Tier Matrix](08-tier-matrix.md)  
**Used by**: [Substrate](../../../subsystems/substrate/)  
**Last reviewed**: 2026-04-19

---

## TL;DR

When an Engram's effective weight drops below `COLD_TIER_THRESHOLD` (default: `0.1`), the
Substrate moves it from the warm tier to the cold tier. In the cold tier, **decay is
suspended** — the balance is not updated by the idle tax. When a cold Engram is retrieved,
it is thawed back into warm storage and its balance is restored to `THAW_RESTORE_BALANCE`
(default: `0.3`). The goal is to preserve rarely-used knowledge cheaply without losing it.

---

## The Idea

Without a cold tier, Engrams would decay toward zero and be garbage-collected even if they
hold valid knowledge that is rarely but legitimately needed. This is analogous to archiving
documents: you move old files to cheap archival storage rather than shredding them.

The freeze-thaw cycle solves this:

1. **Freeze**: When balance crosses below `COLD_TIER_THRESHOLD`, the Substrate moves the
   Engram to cold storage and records `frozen_at_ms`. Decay stops applying.
2. **Thaw**: When a query retrieves the Engram, the Substrate moves it back to warm storage
   and sets `balance = THAW_RESTORE_BALANCE`. Normal decay resumes from that point.
3. **GC after cold dwell time**: If the Engram stays cold longer than `MAX_COLD_DWELL_SECS`,
   it is garbage-collected (deleted). This is the true TTL.

The system therefore has two expiry paths: continuous decay to zero (warm, unchecked) and
cold dwell timeout (cold, infrequently checked).

<!-- ADDED: rationale — Without a cold tier, extremely rarely accessed knowledge would 
slowly disappear even if still valid. The cold tier separates "low usage" from "no value". -->

---

## Constants

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub const COLD_TIER_THRESHOLD: f64 = 0.1;
/// Balance restored on thaw — gives the Engram a second chance at warm life.
pub const THAW_RESTORE_BALANCE: f64 = 0.3;
/// How long an Engram can dwell in the cold tier before GC. Default: 1 year.
pub const MAX_COLD_DWELL_SECS: u64 = 365 * 24 * 3600;
```

---

## State Machine

An Engram moves through three states with respect to the cold tier:

```
              balance < COLD_TIER_THRESHOLD
   [WARM] ─────────────────────────────────► [COLD]
            (freeze; stop decay)

              query retrieves Engram                     dwell > MAX_COLD_DWELL_SECS
   [COLD] ─────────────────────────────────► [WARM]     [COLD] ──────────────────► [GC]
            (thaw; balance = THAW_RESTORE)
```

---

## Freeze Operation

```rust
<!-- source: crates/roko-fs/src/substrate.rs -->

/// Move Engram to cold tier. Suspend decay. Record freeze time.
pub fn freeze(&self, id: &ContentHash, now_ms: i64) -> Result<(), SubstrateError> {
    let mut engram = self.get_warm(id)?;
    // Snapshot current balance before freezing.
    // Decay will not be applied while in cold storage.
    self.cold_store.put(id, &engram, now_ms)?;
    self.warm_store.delete(id)?;
    Ok(())
}
```

The compaction cycle runs `freeze` for every warm Engram whose current `weight_at()` is
below `COLD_TIER_THRESHOLD`.

---

## Thaw Operation

```rust
<!-- source: crates/roko-fs/src/substrate.rs -->

/// Restore Engram from cold tier to warm tier on retrieval.
pub fn thaw(&self, id: &ContentHash, now_ms: i64) -> Result<Engram, SubstrateError> {
    let mut engram = self.cold_store.get(id)?;
    // Restore balance to THAW_RESTORE_BALANCE regardless of current balance.
    // This gives the Engram a meaningful runway before decaying again.
    match &mut engram.decay {
        Decay::Demurrage(ref mut p) => {
            p.balance = THAW_RESTORE_BALANCE;
        }
        Decay::Exponential(ref mut p) => {
            // Exponential decay does not track balance; reset last_retrieved_ms.
            // The weight_at() function will compute from this point.
        }
        _ => {}
    }
    self.warm_store.put(&engram)?;
    self.cold_store.delete(id)?;
    // Update last_retrieved_ms in meta layer.
    self.meta.record_retrieval(id, now_ms)?;
    Ok(engram)
}
```

---

## Cold Dwell GC

```rust
<!-- source: crates/roko-fs/src/substrate.rs -->

/// Scan cold tier; GC Engrams that have exceeded MAX_COLD_DWELL_SECS.
pub fn gc_cold_tier(&self, now_ms: i64) -> GcReport {
    let threshold_ms = now_ms - (MAX_COLD_DWELL_SECS as i64 * 1_000);
    let expired: Vec<ContentHash> = self.cold_store
        .scan_frozen_before(threshold_ms)
        .collect();
    for id in &expired {
        self.cold_store.delete(id).ok();
    }
    GcReport { cold_gc_count: expired.len() }
}
```

---

## Interaction with Decay Models

| Decay model | Freeze behaviour | Thaw behaviour |
|---|---|---|
| **Demurrage** | Balance frozen; no idle tax applied | Balance reset to `THAW_RESTORE_BALANCE` |
| **Exponential** | `weight_at()` would show near-zero; age not advanced | `last_retrieved_ms` reset to thaw time |
| **Step** | Step boundaries continue (time passes) but compaction ignores cold engrams | On thaw, epochs-since-creation still applies |
| **Linear** | Linear decay pauses at freeze (balance snapshot taken) | Balance restored to `THAW_RESTORE_BALANCE` |
| **Custom** | Custom handler's `weight_at()` is not called in cold tier | Custom handler's `on_retrieve()` called on thaw |

Note: Step decay is an edge case — epochs continue to pass in wall time, but the Substrate
does not apply them while the Engram is cold. On thaw, the Substrate applies missed epochs.

<!-- ADDED: edge case clarification — the interaction of Step decay with the cold tier
was not specified in the source docs and is inferred here. -->

---

## Invariants

1. An Engram in the cold tier has balance ≤ `COLD_TIER_THRESHOLD` at freeze time.
2. An Engram thawed from cold tier has balance = `THAW_RESTORE_BALANCE` (for
   balance-tracking models).
3. Decay is **not applied** to Engrams in the cold tier between freeze and thaw.
4. An Engram cannot be both in warm and cold storage simultaneously.
5. `frozen_at_ms` is recorded at freeze time and never updated while in cold storage.
6. Cold dwell GC only applies to the cold tier — warm Engrams are GC'd by hitting
   balance = 0.0.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Engram stuck in cold tier | GC job not running | Compaction cron must include `gc_cold_tier()` |
| Thaw creates duplicate in warm tier | Race between two concurrent retrievals | Thaw is idempotent: if already warm, return warm copy |
| Balance not restored on thaw | Bug in thaw path | Substrate integration test verifies balance = THAW_RESTORE after thaw |
| Cold tier grows unbounded | MAX_COLD_DWELL_SECS too large | Monitor cold tier size; tune dwell limit per deployment |

---

## Open Questions

- Should thawed Engrams get a higher restore balance if they were retrieved after a very
  long cold dwell? (Reward for survival.) Not currently implemented.
- Should the cold tier be a separate physical storage backend (e.g., S3 vs. local RocksDB)?
  Currently both are the same backend, differentiated by a key namespace prefix.

## See Also

- [`00-overview.md`](00-overview.md) — all decay variants compared
- [`08-tier-matrix.md`](08-tier-matrix.md) — which kinds are most likely to be frozen
- [`06-reinforcement.md`](06-reinforcement.md) — thaw also triggers reinforcement
