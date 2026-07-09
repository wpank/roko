# Score — API Reference

> Full public API for the Score type.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md), [Constants](04-constants.md)  
**Last reviewed**: 2026-04-19

---

## `Score` Struct

```rust
<!-- source: crates/roko-core/src/score.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub confidence: f64,
    pub novelty: f64,
    pub utility: f64,
    pub reputation: f64,
    pub precision: Option<f64>,
    pub salience: Option<f64>,
    pub coherence: Option<f64>,
}
```

---

## `Score` Methods

```rust
<!-- source: crates/roko-core/src/score.rs -->

impl Score {
    /// Default score: all stable axes at 0.5; no extended axes.
    pub fn default() -> Self;

    /// All stable axes at the given value; no extended axes.
    pub fn uniform(value: f64) -> Self;

    /// Effective score using default weights.
    /// effective = 0.35c + 0.20n + 0.30u + 0.15r
    pub fn effective(&self) -> f64;

    /// Effective score using custom weights (normalized if they don't sum to 1.0).
    pub fn effective_weighted(&self, weights: &ScoreWeights) -> f64;

    /// Returns Ok(()) if all present axis values are in [0.0, 1.0].
    pub fn validate(&self) -> Result<(), ScoreError>;

    /// Clamp all axis values to [0.0, 1.0].
    pub fn clamp(&mut self);

    /// Returns a new Score with `confidence` replaced.
    pub fn with_confidence(self, v: f64) -> Self;

    /// Returns a new Score with `novelty` replaced.
    pub fn with_novelty(self, v: f64) -> Self;

    /// Returns a new Score with `utility` replaced.
    pub fn with_utility(self, v: f64) -> Self;

    /// Returns a new Score with `reputation` replaced.
    pub fn with_reputation(self, v: f64) -> Self;

    /// Returns a new Score with `precision` replaced.
    pub fn with_precision(self, v: Option<f64>) -> Self;

    /// Returns a new Score with `salience` replaced.
    pub fn with_salience(self, v: Option<f64>) -> Self;

    /// Returns a new Score with `coherence` replaced.
    pub fn with_coherence(self, v: Option<f64>) -> Self;

    /// True if effective() >= threshold.
    pub fn passes(&self, threshold: f64) -> bool;
}
```

---

## `ScoreWeights` Struct

```rust
<!-- source: crates/roko-core/src/score.rs -->

#[derive(Clone, Debug)]
pub struct ScoreWeights {
    pub confidence: f64,
    pub novelty: f64,
    pub utility: f64,
    pub reputation: f64,
    pub precision: f64,
    pub salience: f64,
    pub coherence: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self;  // Returns W_CONFIDENCE, W_NOVELTY, W_UTILITY, W_REPUTATION, 0, 0, 0
}

impl ScoreWeights {
    /// Normalize weights so they sum to 1.0.
    pub fn normalize(&mut self);

    /// Returns true if weights sum to approximately 1.0.
    pub fn is_normalized(&self) -> bool;
}
```

---

## `ScoreError` Type

```rust
<!-- source: crates/roko-core/src/score.rs -->

#[derive(Debug)]
pub enum ScoreError {
    AxisOutOfRange { axis: &'static str, value: f64 },
}
```

---

## See Also

- [`03-arithmetic.md`](03-arithmetic.md) — how the API computes effective()
- [`06-examples.md`](06-examples.md) — usage examples
