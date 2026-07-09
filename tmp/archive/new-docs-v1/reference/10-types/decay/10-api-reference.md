# Decay — API Reference

> Complete signatures for every public method and associated function in the `Decay` type family.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

This page is a mechanical reference. Every public method on `Decay`, `DemurrageParams`,
`ExponentialDecayParams`, `StepDecayParams`, `LinearDecayParams`, and `CustomDecayParams`
is listed here with its signature, parameters, return type, and a one-sentence description.
For concepts and worked examples, see the individual variant pages.

---

## `enum Decay`

```rust
<!-- source: crates/roko-core/src/decay.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Decay {
    Demurrage(DemurrageParams),
    Exponential(ExponentialDecayParams),
    Step(StepDecayParams),
    Linear(LinearDecayParams),
    Custom(CustomDecayParams),
}
```

### Methods

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl Decay {
    /// Return the effective weight at `now_ms`.
    /// `created_at_ms` and `last_retrieved_ms` are sourced from Substrate metadata.
    pub fn weight_at(
        &self,
        now_ms: i64,
        created_at_ms: i64,
        last_retrieved_ms: i64,
    ) -> f64;

    /// Apply reinforcement after a successful retrieval.
    /// No-op for Exponential, Step, and Linear variants.
    pub fn reinforce(&mut self);

    /// Return true if the Engram is eligible for GC (weight below floor).
    pub fn is_gc_eligible(&self, now_ms: i64, created_at_ms: i64, last_retrieved_ms: i64) -> bool;

    /// Return true if reinforcement is supported by this variant.
    pub fn supports_reinforcement(&self) -> bool;

    /// Return the variant name as a static string for logging/metrics.
    pub fn variant_name(&self) -> &'static str;
}
```

---

## `struct DemurrageParams`

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub struct DemurrageParams {
    pub balance: f64,
    pub idle_tax_per_day: f64,
    pub reinforcement_per_use: f64,
}
```

| Method | Signature | Description |
|---|---|---|
| `weight_at` | `(&self, now_ms: i64, last_retrieved_ms: i64) -> f64` | Balance after applying idle tax since last retrieval |
| `apply_idle_tax` | `(&mut self, idle_days: f64)` | Mutate balance by applying idle tax for `idle_days` |
| `reinforce` | `(&mut self)` | Add `reinforcement_per_use` to balance, capped at 1.0 |
| `half_life_days` | `(&self) -> f64` | Days until balance halves with no retrieval |
| `equilibrium_balance` | `(&self, retrievals_per_day: f64) -> f64` | Balance at steady-state for given retrieval frequency |
| `Default::default` | `() -> Self` | `{balance: 1.0, idle_tax_per_day: 0.01, reinforcement_per_use: 0.05}` |

---

## `struct ExponentialDecayParams`

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub struct ExponentialDecayParams {
    pub half_life_secs: u64,
}
```

| Method | Signature | Description |
|---|---|---|
| `weight_at` | `(&self, now_ms: i64, created_at_ms: i64) -> f64` | `e^(-ln(2)/half_life × elapsed)` |
| `time_to_gc_ms` | `(&self, created_at_ms: i64, gc_floor: f64) -> i64` | Millisecond timestamp when weight crosses `gc_floor` |
| `Default::default` | `() -> Self` | `{half_life_secs: 2_592_000}` (30 days) |

---

## `struct StepDecayParams`

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub struct StepDecayParams {
    pub balance: f64,
    pub epoch_secs: u64,
    pub step_multiplier: f64,
}
```

| Method | Signature | Description |
|---|---|---|
| `weight_at` | `(&self, now_ms: i64, created_at_ms: i64) -> f64` | Balance after applying all full epochs since creation |
| `apply_epochs` | `(&mut self, epochs: u32)` | Mutate balance by applying `epochs` full steps |
| `epochs_elapsed` | `(&self, now_ms: i64, created_at_ms: i64) -> u32` | Number of full epochs since creation |
| `next_step_ms` | `(&self, now_ms: i64, created_at_ms: i64) -> i64` | Timestamp of the next epoch boundary |
| `Default::default` | `() -> Self` | `{balance: 1.0, epoch_secs: 604_800, step_multiplier: 0.5}` |

---

## `struct LinearDecayParams`

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub struct LinearDecayParams {
    pub balance: f64,
    pub rate_per_sec: f64,
}
```

| Method | Signature | Description |
|---|---|---|
| `weight_at` | `(&self, now_ms: i64, last_updated_ms: i64) -> f64` | `balance - rate_per_sec × elapsed`, floored at 0.0 |
| `apply_elapsed` | `(&mut self, elapsed_secs: f64)` | Mutate balance by elapsed time |
| `remaining_secs` | `(&self) -> f64` | Seconds until balance reaches 0.0 |
| `expiry_ms` | `(&self, now_ms: i64) -> i64` | Wall-clock expiry time in milliseconds |
| `Default::default` | `() -> Self` | `{balance: 1.0, rate_per_sec: 1.0/3600.0}` (1-hour lifetime) |

---

## `struct CustomDecayParams`

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub struct CustomDecayParams {
    pub name: String,
    pub params: serde_json::Value,
}
```

| Method | Signature | Description |
|---|---|---|
| `new` | `(name: impl Into<String>, params: serde_json::Value) -> Self` | Construct a custom decay |
| `Default::default` | `() -> Self` | `{name: "unset".into(), params: Value::Null}` |

---

## `trait DecayHandler`

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub trait DecayHandler: Send + Sync + 'static {
    fn weight_at(
        &self,
        params: &serde_json::Value,
        now_ms: i64,
        created_at_ms: i64,
        last_retrieved_ms: i64,
    ) -> f64;

    fn on_retrieve(&self, params: &mut serde_json::Value) {}
}
```

| Method | Description |
|---|---|
| `weight_at` | Compute and return the weight for the given params. Must return a value in `[0.0, 1.0]`. |
| `on_retrieve` | Mutate params after a successful retrieval. Default implementation is a no-op. |

---

## Constants

```rust
<!-- source: crates/roko-core/src/decay.rs -->

/// Weight below which an Engram is moved to the cold tier.
pub const COLD_TIER_THRESHOLD: f64 = 0.1;

/// Balance assigned to a thawed Engram.
pub const THAW_RESTORE_BALANCE: f64 = 0.3;

/// Maximum cold dwell time before GC (in seconds). 1 year default.
pub const MAX_COLD_DWELL_SECS: u64 = 365 * 24 * 3_600;

/// Weight below which an Engram is GC-eligible (warm tier).
pub const GC_FLOOR: f64 = 0.001;
```

---

## Error Types

```rust
<!-- source: crates/roko-core/src/decay.rs -->

#[derive(Debug, thiserror::Error)]
pub enum DecayError {
    #[error("idle_tax_per_day must be in (0.0, 1.0), got {0}")]
    InvalidIdleTax(f64),

    #[error("reinforcement_per_use must be in (0.0, 1.0], got {0}")]
    InvalidReinforcement(f64),

    #[error("half_life_secs must be > 0")]
    ZeroHalfLife,

    #[error("epoch_secs must be > 0")]
    ZeroEpochLength,

    #[error("step_multiplier must be in (0.0, 1.0), got {0}")]
    InvalidStepMultiplier(f64),

    #[error("rate_per_sec must be > 0.0, got {0}")]
    InvalidLinearRate(f64),

    #[error("custom decay name must not be empty")]
    EmptyCustomName,
}
```

---

## Open Questions

- Should `Decay` implement `Ord` to allow sorting Engrams by decay urgency? Not currently done.
- Should `weight_at` be fallible (return `Result<f64, DecayError>`) to surface handler errors?
  Currently infallible; errors return `1.0` fallback.

## See Also

- [`00-overview.md`](00-overview.md) — all decay variants
- [`09-invariants.md`](09-invariants.md) — invariants for every method
- [`11-examples.md`](11-examples.md) — worked examples for every method
