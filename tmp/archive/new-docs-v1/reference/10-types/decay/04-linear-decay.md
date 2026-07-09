# Decay — Linear Decay

> The Linear model: balance decreases at a constant rate per second until it reaches zero.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Used by**: [Tier Matrix](08-tier-matrix.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Linear decay reduces the balance by a fixed `rate_per_sec` each second. The balance reaches
zero after `initial_balance / rate_per_sec` seconds and does not go negative. It is the
simplest possible time-based decay: no logarithm, no exponent, no epoch boundary — just a
straight line to zero. Use it when the lifetime of an Engram is well-defined and predictable
in advance (e.g., "this context is valid for exactly the next 30 minutes").

---

## The Idea

Linear decay is the "best-before date" model. Given `rate_per_sec = 1/1800` the Engram
has exactly 30 minutes of full weight, then zero. There is no tail — unlike exponential
decay, it hits zero exactly at the end of its configured lifetime.

This predictability is its main advantage and main limitation:

**Advantage**: the GC scheduler can pre-compute exactly when an Engram will be GC-eligible
without running any weight calculation. The expiry is `created_at_ms + (balance / rate_per_sec * 1000)`.

**Limitation**: there is no way to extend the lifetime by retrieval. An Engram that is used
heavily still expires at the same wall-clock time.

For Engrams that should be extended by use, prefer [Demurrage](01-demurrage.md).

---

## Specification

```rust
<!-- source: crates/roko-core/src/decay.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinearDecayParams {
    /// Current balance in [0.0, 1.0].
    pub balance: f64,

    /// Balance reduction per second.
    /// An Engram with balance=1.0 and rate_per_sec=1/3600 lives exactly 1 hour.
    pub rate_per_sec: f64,
}
```

---

## Weight Function

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl LinearDecayParams {
    /// Compute weight at `now_ms` given last update at `last_updated_ms`.
    /// Returns 0.0 when the Engram has expired.
    pub fn weight_at(&self, now_ms: i64, last_updated_ms: i64) -> f64 {
        let elapsed_secs = ((now_ms - last_updated_ms) as f64 / 1_000.0).max(0.0);
        let decayed = self.balance - self.rate_per_sec * elapsed_secs;
        decayed.max(0.0)
    }

    /// Apply elapsed time and persist balance.
    pub fn apply_elapsed(&mut self, elapsed_secs: f64) {
        self.balance = (self.balance - self.rate_per_sec * elapsed_secs).max(0.0);
    }

    /// Return the remaining lifetime in seconds (0 if already expired).
    pub fn remaining_secs(&self) -> f64 {
        if self.rate_per_sec <= 0.0 {
            f64::INFINITY
        } else {
            (self.balance / self.rate_per_sec).max(0.0)
        }
    }
}
```

---

## Lifetime Table

Common rate configurations:

| `rate_per_sec` | Lifetime at balance=1.0 |
|---|---|
| `1.0 / 60.0` | 1 minute |
| `1.0 / 300.0` | 5 minutes |
| `1.0 / 1_800.0` | 30 minutes |
| `1.0 / 3_600.0` | 1 hour |
| `1.0 / 86_400.0` | 1 day |
| `1.0 / 604_800.0` | 1 week |

---

## Default Parameters

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl Default for LinearDecayParams {
    fn default() -> Self {
        LinearDecayParams {
            balance: 1.0,
            rate_per_sec: 1.0 / 3_600.0,  // 1 hour lifetime
        }
    }
}
```

---

## Semantics

Linear decay is **stateless with respect to retrieval**. Retrieving an Engram never changes
its `rate_per_sec` or restores its balance. The only operation that mutates a
`LinearDecayParams` is `apply_elapsed`.

The Substrate calls `apply_elapsed` either:
- Lazily at retrieval time (compute elapsed since last update, apply, update timestamp).
- Eagerly in the compaction cycle (scan all linear-decayed Engrams, apply elapsed, GC zeros).

Roko uses the **lazy** strategy to avoid scanning on every compaction tick.

---

## Invariants

1. `balance ∈ [0.0, 1.0]` always.
2. `rate_per_sec > 0.0` — non-positive rate is immortal; use `Decay::Demurrage` with
   `idle_tax_per_day = 0.0` for that instead.
3. `weight_at(t)` is a strictly decreasing linear function.
4. `weight_at(t)` reaches exactly 0.0 and never goes negative.
5. `remaining_secs()` is a monotonically decreasing function of time.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Engram expires too early | `rate_per_sec` set too high | Use `remaining_secs()` to validate before storing |
| Engram never expires | `rate_per_sec = 0.0` stored accidentally | Validate on construction; reject ≤ 0.0 |
| GC misses an expired Engram | Lazy eviction not triggered (no retrieval) | Compaction cycle must scan and evict zero-balance Engrams |
| Balance underflows to negative | Bug in caller computing elapsed | `max(0.0)` floor in `apply_elapsed` and `weight_at` prevents this |

---

## Interactions

- **Substrate GC**: Linear decay enables deterministic GC scheduling. The expiry
  time is `now + remaining_secs()`. The Substrate can build a min-heap of expiry times
  and GC in deadline order.
- **Exponential vs. Linear**: Exponential decay has a long tail (never reaches zero);
  Linear decay has a hard deadline. For short-lived ephemeral data, Linear is the
  correct choice. For knowledge intended to have long-term residual value,
  use Exponential or Demurrage.
- **Custom decay**: If you need a linear-decay-like model with a retrieval reset,
  implement a [Custom](05-custom-decay.md) decay that resets `last_updated_ms` on
  retrieval.

---

## Open Questions

- Should `LinearDecayParams` support a configurable floor above 0.0 (i.e., asymptotic
  decay that approaches but never hits a non-zero floor)? Not currently implemented;
  that would be a Custom variant.
- Should the GC min-heap be pre-built during Substrate startup or maintained
  incrementally? Currently rebuilt on each compaction pass.

## See Also

- [`00-overview.md`](00-overview.md) — all decay variants compared
- [`02-exponential-decay.md`](02-exponential-decay.md) — continuous decay with a tail
- [`05-custom-decay.md`](05-custom-decay.md) — for non-standard decay shapes
- [`08-tier-matrix.md`](08-tier-matrix.md) — default models per Engram kind
