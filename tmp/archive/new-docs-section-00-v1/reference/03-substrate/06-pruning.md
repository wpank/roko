# Pruning

> Pruning removes `Engram`s from the store to reclaim capacity. Roko supports two eviction
> strategies: decay-driven (records whose `Decay` schedule has expired) and
> capacity-driven (records evicted to stay within a size budget).

**Status**: Shipping
**Crate**: `roko-core` (trait), `roko-fs` (file backend implementation)
**Depends on**: [Decay Variants](../10-types/decay.md), [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`prune()` is a `Substrate` method that removes expired or low-priority records. Two
strategies drive eviction: decay expiry (a record's balance reaches zero) and capacity
pressure (total record count exceeds a configured ceiling). The two strategies compose —
decay runs first, then capacity-based eviction if the store is still over budget.

---

## Why Prune?

Agents accumulate memories continuously. Without pruning:
- Storage grows without bound.
- `query` and `query_similar` slow down as scan sizes grow.
- Low-value, stale memories "crowd out" high-value ones in recall results.

Pruning is how Roko implements forgetting — a deliberate design decision that mirrors
biological memory, where disuse reduces recall probability.

---

## The Two Eviction Strategies

### 1. Decay-Driven Eviction

Each `Engram` has a [`Decay`](../10-types/decay.md) schedule. The `balance` field decreases
over time according to the decay variant (exponential, step, freeze/thaw, etc.). When an
`Engram`'s effective balance reaches zero (or falls below a configured floor `decay_floor`),
it is eligible for pruning.

During `prune()`:
1. Compute the current effective balance for every stored `Engram`.
2. Mark those with `balance <= decay_floor` as expired.
3. Remove expired records.

The `decay_floor` is a backend configuration parameter (default: `0.0`).

### 2. Capacity-Driven Eviction

If after decay eviction the store still holds more than `max_capacity` records, additional
records are evicted by ascending score rank:

1. Sort remaining records by `Score::confidence * Score::utility` (a composite priority).
2. Evict the lowest-ranked records until `len() <= max_capacity`.

This ensures that even if decay schedules haven't expired, the store doesn't overflow a
deployment's storage budget.

---

## Prune Invocation

`prune` is a method on `Substrate` and can be called:

1. **On a schedule** — the runtime calls `prune` on a timer (default: every 5 minutes in
   production, every 60 seconds in tests).
2. **Lazily on `put`** — backends MAY trigger an amortised prune when the store crosses a
   high-water mark (default: 110% of `max_capacity`).
3. **Explicitly by the caller** — useful in tests and the Delta (consolidation) speed loop.

```rust
// source: crates/roko-runtime/src/agent.rs
// Explicit prune call from the Delta-speed consolidation loop:
let removed = substrate.prune()?;
tracing::debug!(removed, "substrate pruned");
```
<!-- source: crates/roko-runtime/src/agent.rs -->

---

## Return Value

`prune() -> Result<usize, SubstrateError>` returns the number of records removed. The
runtime logs this as a metric (`substrate.prune.removed_count`). A return of `0` is normal
and not an error — it means no records were eligible for eviction.

---

## Configuration

<!-- ADDED: inferred from codebase patterns -->

| Parameter | Default | Description |
|---|---|---|
| `decay_floor` | `0.0` | Records with effective balance ≤ this are decay-expired |
| `max_capacity` | `usize::MAX` (no limit) | Trigger capacity eviction above this record count |
| `prune_interval_secs` | `300` (5 min) | How often the runtime calls `prune` automatically |
| `high_water_ratio` | `1.1` | Lazy prune trigger: prune when `len > max_capacity * ratio` |

---

## Invariants

- `prune` must never remove a record that has `balance > decay_floor` unless capacity
  eviction requires it.
- After `prune` returns, `len() <= max_capacity`.
- A record removed by `prune` must not be returned by subsequent `get` or `query` calls.
- `prune` is not called concurrently with reads — the runtime holds the write lock.

---

<!-- ADDED -->
## Failure Modes

| Failure | Behaviour |
|---|---|
| I/O error mid-prune | Returns `SubstrateError::Io`. Partial removals are acceptable — the store may still be over capacity. The runtime will retry on the next schedule tick. |
| `balance` computation overflow | Saturating arithmetic prevents panics; affected records are treated as balance = 0 (eligible for eviction). |
| `max_capacity = 0` | Treated as "no limit" (same as `usize::MAX`) to prevent accidentally emptying the store. |

---

## See Also

- [Decay Variants](../10-types/decay.md) — the `balance` field and decay schedules
- [Performance](./13-performance.md) — prune cost and scheduling
- [Invariants](./11-invariants.md)

## Open Questions

- Should `prune` accept a `budget: usize` parameter (max records to remove per call) to
  bound its latency?
- Should the runtime support a "dry run" prune that returns the candidates without actually
  removing them?
