# Engram — Decay Fields

> The `decay: Decay` field governs how an Engram's effective weight decreases over time. It is not part of the identity hash.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Decay type](../10-types/decay/00-overview.md)  
**Used by**: Substrate GC, retrieval ranking, pheromone evaporation  
**Last reviewed**: 2026-04-19

---

## TL;DR

Every Engram carries a `Decay` model that determines how its weight evolves over time.
Four models are supported: Demurrage (the primary model), Exponential, Step, and Linear.
Custom functions are available. Decay is excluded from the identity hash so substrates
can change the model without changing the Engram's id.

---

## The Idea

Information has a shelf life. An agent output produced five minutes ago is likely more
relevant than one from three months ago. A pheromone signal deposited by a failed attempt
should evaporate quickly. A foundational knowledge entry should persist for months.

Decay models encode this intuition mathematically. The Substrate multiplies an Engram's
effective score by its decay weight at retrieval time. An Engram at weight 0.0 is
effectively invisible and becomes a GC candidate.

---

## Specification

The `Decay` type is specified in full in
[`../10-types/decay/`](../10-types/decay/README.md). This page covers the attachment
of Decay to Engram.

### The Decay Field

```rust
<!-- source: crates/roko-core/src/engram.rs -->

/// Decay model governing how this Engram's effective weight changes over time.
/// Not included in the identity hash — substrates can adjust without changing id.
pub decay: Decay,
```

### Default Decay

`EngramBuilder::build()` uses `Decay::default()` if no decay is specified:

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl Default for Decay {
    fn default() -> Self {
        Decay::Demurrage(DemurrageParams::default())
    }
}
```

The default Demurrage params impose a 1% idle tax per day and grant 5% reinforcement per
retrieval.

### Changing Decay at Runtime

The Substrate supports updating the decay model for an existing Engram:

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

trait Substrate {
    /// Replace the decay model for an existing Engram.
    /// Does not change the Engram's id.
    fn update_decay(&self, id: &ContentHash, decay: Decay) -> Result<(), SubstrateError>;
}
```

This is used for:
- **Cold-tier migration**: promoting an Engram to `Decay::ColdTier` when it has not been
  accessed for the substrate's cold threshold.
- **Decay model upgrades**: migrating a fleet of Engrams from Exponential to Demurrage.

---

## The Five Decay Models

| Model | When to use |
|-------|------------|
| `Decay::Demurrage` | Primary model for most Engrams; rewards active use, taxes idleness |
| `Decay::Exponential` | Classic half-life decay; pheromones, short-lived signals |
| `Decay::Step` | Epoch-based drops; gate verdicts, time-boxed plans |
| `Decay::Linear` | Simple time-linear fade; metrics, observations |
| `Decay::Custom` | User-defined function; domain-specific use cases |

For full specifications, see the individual files in
[`../10-types/decay/`](../10-types/decay/README.md).

### Demurrage (Primary Model)

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub struct DemurrageParams {
    /// Current balance (0.0–1.0). Starts at 1.0. Never goes negative.
    pub balance: f64,
    /// Fraction of balance lost per day of non-retrieval.
    pub idle_tax_per_day: f64,    // typical: 0.01 (1% per day)
    /// Fraction of balance gained per retrieval.
    pub reinforcement_per_use: f64, // typical: 0.05 (5% per use)
}
```

Demurrage is the primary model because it implements the "use it or lose it" principle:
frequently-accessed knowledge stays warm; idle knowledge degrades. When an Engram is
retrieved, `balance += reinforcement_per_use`, capped at 1.0. Each day of non-access,
`balance *= (1 - idle_tax_per_day)`.

### Exponential Decay

```rust
pub struct ExponentialDecayParams {
    /// Half-life in seconds.
    pub half_life_secs: f64,
}

// weight(t) = 0.5^(elapsed_secs / half_life_secs)
```

### Step Decay

```rust
pub struct StepDecayParams {
    /// Epoch duration in seconds.
    pub epoch_secs: f64,
    /// Weight multiplier applied at each epoch boundary.
    pub step_multiplier: f64,  // e.g. 0.5 = halved each epoch
}
```

### Linear Decay

```rust
pub struct LinearDecayParams {
    /// Decay rate per second.
    pub rate_per_sec: f64,
}
// weight(t) = max(0.0, 1.0 - rate_per_sec * elapsed_secs)
```

---

## How Decay Affects the Substrate

### Effective Weight at Retrieval

```rust
let effective_weight = decay.weight_at(now_ms, engram.created_at_ms);
```

The Substrate multiplies the Engram's `score.effective()` by `effective_weight` when
ranking retrieval results.

### GC Eligibility

An Engram with `effective_weight < substrate.gc_threshold` (typically 0.01) is a GC
candidate. The GC runs during idle periods and removes the lowest-weighted Engrams first.

### Reinforcement on Use

For Demurrage Engrams, the Substrate calls `decay.reinforce()` on every retrieval:

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

fn on_retrieve(&self, id: &ContentHash) {
    if let Some(mut engram) = self.get_mut(id) {
        engram.decay.reinforce();
    }
}
```

---

## Invariants

1. `decay` is not included in the identity hash
2. `decay.weight_at(t)` is in [0.0, 1.0] for all t
3. Decay weight is monotonically non-increasing over time (except for Demurrage reinforcement)
4. `Demurrage::balance` is in [0.0, 1.0]

---

## Failure Modes

| Failure | Cause | Recovery |
|---------|-------|----------|
| Weight goes to 0 prematurely | Idle tax too high | Tune `idle_tax_per_day`; cold-tier migration before GC |
| Reinforcement overflow | Multiple simultaneous retrievals | Capped at 1.0; not an error |
| GC removes needed Engram | Decay without reinforcement | Pre-retrieve before GC window; monitor GC logs |

---

## See Also

- [`../10-types/decay/00-overview.md`](../10-types/decay/00-overview.md) — decay type folder
- [`../10-types/decay/01-demurrage.md`](../10-types/decay/01-demurrage.md) — the primary model
- [`../10-types/decay/08-tier-matrix.md`](../10-types/decay/08-tier-matrix.md) — which model for which kind
