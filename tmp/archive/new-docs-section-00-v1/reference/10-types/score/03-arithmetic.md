# Score — Arithmetic

> How axes combine into the effective score; the weighting formula; custom weight sets.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Stable Axes](01-axes-stable.md), [Constants](04-constants.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

The effective score is a weighted sum of the 4 stable axes. Extended axes are excluded
from the default formula but can be incorporated via `effective_weighted()` with a
custom `ScoreWeights`. The formula is transparent, deterministic, and fast.

---

## The Effective Score Formula

```
effective(s) = w_confidence × s.confidence
             + w_novelty    × s.novelty
             + w_utility    × s.utility
             + w_reputation × s.reputation
```

With default constants:

```
effective(s) = 0.35 × s.confidence
             + 0.20 × s.novelty
             + 0.30 × s.utility
             + 0.15 × s.reputation
```

**Range:** [0.0, 1.0] (since each axis is in [0.0, 1.0] and weights sum to 1.0).

---

## Implementation

```rust
<!-- source: crates/roko-core/src/score.rs -->

impl Score {
    /// Effective score using default weights.
    pub fn effective(&self) -> f64 {
        W_CONFIDENCE * self.confidence
            + W_NOVELTY    * self.novelty
            + W_UTILITY    * self.utility
            + W_REPUTATION * self.reputation
    }

    /// Effective score using custom weights.
    pub fn effective_weighted(&self, w: &ScoreWeights) -> f64 {
        let stable = w.confidence * self.confidence
            + w.novelty    * self.novelty
            + w.utility    * self.utility
            + w.reputation * self.reputation;

        let extended = w.precision   * self.precision.unwrap_or(0.5)
            + w.salience    * self.salience.unwrap_or(0.5)
            + w.coherence   * self.coherence.unwrap_or(0.5);

        // Total weight should sum to 1.0; normalize if not.
        let total_weight = w.confidence + w.novelty + w.utility + w.reputation
            + w.precision + w.salience + w.coherence;
        if total_weight.abs() < 1e-9 { return 0.0; }
        (stable + extended) / total_weight
    }
}
```

---

## ScoreWeights

```rust
<!-- source: crates/roko-core/src/score.rs -->

/// Weights for each score axis.
/// Must sum to 1.0 (or will be normalized).
#[derive(Clone, Debug)]
pub struct ScoreWeights {
    pub confidence: f64,   // default: W_CONFIDENCE = 0.35
    pub novelty: f64,      // default: W_NOVELTY = 0.20
    pub utility: f64,      // default: W_UTILITY = 0.30
    pub reputation: f64,   // default: W_REPUTATION = 0.15
    pub precision: f64,    // default: 0.0 (extended, excluded by default)
    pub salience: f64,     // default: 0.0 (extended, excluded by default)
    pub coherence: f64,    // default: 0.0 (extended, excluded by default)
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            confidence: W_CONFIDENCE,
            novelty: W_NOVELTY,
            utility: W_UTILITY,
            reputation: W_REPUTATION,
            precision: 0.0,
            salience: 0.0,
            coherence: 0.0,
        }
    }
}
```

---

## Gate Threshold Comparison

Gates compare `effective_score >= threshold`:

```rust
<!-- source: crates/roko-core/src/gate.rs -->

fn passes_score_gate(engram: &Engram, threshold: f64) -> bool {
    engram.score.effective() >= threshold
}
```

A gate with `threshold = 0.7` requires:
```
0.35c + 0.20n + 0.30u + 0.15r ≥ 0.7
```

An Engram with `confidence=0.95, novelty=0.5, utility=0.8, reputation=0.5`:
```
0.35 × 0.95 + 0.20 × 0.50 + 0.30 × 0.80 + 0.15 × 0.50
= 0.3325 + 0.10 + 0.24 + 0.075
= 0.7475  → passes threshold of 0.7 ✓
```

---

## Custom Weight Sets: Examples

### Trust-Sensitive Gate (reputation-heavy)

```rust
let weights = ScoreWeights {
    confidence: 0.20,
    novelty: 0.10,
    utility: 0.20,
    reputation: 0.50,  // heavy reputation weight
    ..Default::default()
};
```

### Knowledge-Freshness Gate (novelty-heavy)

```rust
let weights = ScoreWeights {
    confidence: 0.30,
    novelty: 0.40,  // heavy novelty weight
    utility: 0.20,
    reputation: 0.10,
    ..Default::default()
};
```

---

## Invariants

1. Default weights sum to exactly 1.0: `0.35 + 0.20 + 0.30 + 0.15 = 1.0`
2. `effective()` returns a value in [0.0, 1.0] (guaranteed by axis bounds and weight sum)
3. `effective_weighted()` normalizes by total weight if weights don't sum to 1.0

---

## See Also

- [`04-constants.md`](04-constants.md) — all weight constants with rationale
- [`01-axes-stable.md`](01-axes-stable.md) — what each axis means
- [`08-rationale.md`](08-rationale.md) — why this weighting was chosen
