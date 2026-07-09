# Decay — Exponential Decay

> Classic half-life decay: weight halves every `half_life_secs` seconds.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Exponential decay is the simplest model: weight = 0.5^(elapsed / half_life). No state
beyond the half-life parameter. Use for pheromones, short-lived signals, and any Engram
where recency dominates.

---

## Specification

```rust
<!-- source: crates/roko-core/src/decay.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExponentialDecayParams {
    /// Time for weight to halve, in seconds.
    pub half_life_secs: f64,
}

impl ExponentialDecayParams {
    pub fn weight_at(&self, elapsed_secs: f64) -> f64 {
        0.5_f64.powf(elapsed_secs / self.half_life_secs)
    }
}
```

---

## Common Half-Life Values

| Use case | Suggested half_life |
|----------|-------------------|
| Pheromone (ACO-style) | 3600 s (1 hour) |
| AgentOutput | 86400 s (1 day) |
| Short-term Observation | 1800 s (30 minutes) |
| Metric sample | 300 s (5 minutes) |

---

## Properties

- Pure function of elapsed time; no mutable state.
- Weight is always positive (never exactly 0.0).
- At `t = 0`: weight = 1.0. At `t = half_life`: weight = 0.5.
- Asymptotically approaches 0.0 but never reaches it.

For GC: the Substrate uses `weight < gc_threshold` (typically 0.01) as the cutoff.
At `gc_threshold = 0.01`, an Engram with a 1-hour half-life is GC-eligible after
≈ 6.6 hours (since `0.5^(6.6) ≈ 0.01`).

---

## See Also

- [`01-demurrage.md`](01-demurrage.md) — primary model (use-aware)
- [`00-overview.md`](00-overview.md) — when to choose exponential
