# Decay — Demurrage

> The Demurrage model: idle tax + reinforcement on retrieval. The primary decay model for most Engrams.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Demurrage is the "use it or lose it" model. An Engram starts with `balance = 1.0`.
Each day of non-retrieval, the balance decreases by `idle_tax_per_day`. Each retrieval
increases the balance by `reinforcement_per_use`, capped at 1.0. The current balance
is the effective weight. An Engram that is retrieved daily can remain at full weight
indefinitely.

---

## The Idea

The name comes from economics: *demurrage* is a holding cost charged on currency or
goods to encourage circulation. In Roko, knowledge that sits in the substrate unused
incurs a cost. Knowledge that is actively retrieved and used is reinforced.

This model is better than pure exponential decay for knowledge because:
1. Frequently-used knowledge stays warm without requiring special configuration.
2. Occasionally-used knowledge decays slowly enough to remain accessible.
3. Never-used knowledge decays to zero and is GC'd, keeping the substrate trim.

---

## Specification

```rust
<!-- source: crates/roko-core/src/decay.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DemurrageParams {
    /// Current balance in [0.0, 1.0].
    /// Starts at 1.0 on creation. Decays with idle time.
    pub balance: f64,

    /// Fraction of balance lost per day of non-retrieval.
    /// Default: 0.01 (1% per day).
    pub idle_tax_per_day: f64,

    /// Fraction added to balance per retrieval (capped at 1.0).
    /// Default: 0.05 (5% per retrieval).
    pub reinforcement_per_use: f64,
}
```

---

## Weight Function

The effective weight at time `t` is simply the current `balance`. The balance is not
a continuous function of time — it is updated discretely by the Substrate:

- **On each day boundary** (or on GC / compact): `balance *= (1 - idle_tax_per_day)`
- **On each retrieval**: `balance = (balance + reinforcement_per_use).min(1.0)`

In practice, the Substrate applies the idle tax lazily at retrieval time:

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl DemurrageParams {
    /// Compute the weight at `now_ms`, applying idle tax since `last_retrieved_ms`.
    /// Does NOT mutate self — call `apply_idle_tax()` to persist.
    pub fn weight_at(&self, now_ms: i64, last_retrieved_ms: i64) -> f64 {
        let idle_days = (now_ms - last_retrieved_ms) as f64 / (86_400_000.0);
        let decayed = self.balance * (1.0 - self.idle_tax_per_day).powf(idle_days);
        decayed.max(0.0)
    }

    /// Apply idle tax for elapsed days and return new balance.
    pub fn apply_idle_tax(&mut self, idle_days: f64) {
        self.balance *= (1.0 - self.idle_tax_per_day).powf(idle_days);
        self.balance = self.balance.max(0.0);
    }

    /// Reinforce on retrieval.
    pub fn reinforce(&mut self) {
        self.balance = (self.balance + self.reinforcement_per_use).min(1.0);
    }
}
```

---

## Half-Life Analysis

With default params (`idle_tax_per_day = 0.01`):
- **Half-life**: ~68.7 days without any retrieval
- **At 1 retrieval/week**: balance stabilizes around 0.72 (reinforcement compensates)
- **At 1 retrieval/day**: balance stabilizes around 0.99 (fully warm)

With aggressive params (`idle_tax_per_day = 0.1`):
- **Half-life**: ~6.6 days
- Appropriate for short-lived tool traces or session-scoped data

---

## Default Parameters

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl Default for DemurrageParams {
    fn default() -> Self {
        DemurrageParams {
            balance: 1.0,
            idle_tax_per_day: 0.01,      // 1% idle tax per day
            reinforcement_per_use: 0.05,  // 5% reinforcement per retrieval
        }
    }
}
```

---

## Invariants

1. `balance ∈ [0.0, 1.0]` always
2. `idle_tax_per_day ∈ (0.0, 1.0)` (0 = no decay, 1 = lose everything daily)
3. `reinforcement_per_use ∈ (0.0, 1.0]`
4. `weight_at(t)` is non-increasing between retrievals
5. `reinforce()` is idempotent when balance is already 1.0

---

## See Also

- [`06-reinforcement.md`](06-reinforcement.md) — how reinforcement interacts with the substrate
- [`08-tier-matrix.md`](08-tier-matrix.md) — which Engrams use Demurrage by default
- [`00-overview.md`](00-overview.md) — comparison to other decay models
