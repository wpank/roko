# Score — Overview

> Score is a 7-axis quality assessment attached to every Engram. It governs gate thresholds, retrieval ranking, and GC priority.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Engram](../../01-engram/00-overview.md)  
**Used by**: Gate pipeline, Substrate retrieval, Router, GC  
**Last reviewed**: 2026-04-19

---

## TL;DR

Every Engram has a `Score` with 4 stable axes (confidence, novelty, utility, reputation)
and 3 optional extended axes (precision, salience, coherence). The effective score is a
weighted sum of the stable axes. Scorers compute scores; the Substrate stores them but
does not compute them. Score is excluded from the identity hash.

---

## The Idea

Quality is multi-dimensional. An Engram might be highly confident but not novel. A tool
trace might be high-utility but from a low-reputation source. A prediction might be
precise but not salient to the current task. A single number cannot capture these
distinctions.

The 7-axis model allows different downstream consumers to weight axes differently:
- A Gate that enforces factual accuracy weights `confidence` heavily.
- A Router that wants diverse perspectives weights `novelty` heavily.
- The Substrate GC weights `utility` — Engrams that have proven useful stay warm.
- A trust-sensitive gate weights `reputation` — low-trust sources get extra scrutiny.

The **effective score** is a single number derived from the weighted combination of
stable axes. It is used when a single-number comparison is needed (gate thresholds,
GC priority, default ranking).

---

## The 7 Axes

### Stable Axes (always present, always in [0.0, 1.0])

| Axis | Meaning | 1.0 means… |
|------|---------|-----------|
| `confidence` | How certain is the source that this is true? | Certainty |
| `novelty` | How new is this information to the substrate? | Completely new |
| `utility` | Has this Engram proven useful in past retrievals? | Always led to success |
| `reputation` | How trustworthy is the author? | Chain-witnessed |

### Extended Axes (optional, Some(f64) in [0.0, 1.0])

| Axis | Meaning | 1.0 means… |
|------|---------|-----------|
| `precision` | How specific and accurate is the claim? | Exact and precise |
| `salience` | How relevant to the current task context? | Perfectly relevant |
| `coherence` | How internally consistent is the content? | Fully coherent |

---

## Effective Score Formula

```
effective = w_confidence × confidence
          + w_novelty   × novelty
          + w_utility   × utility
          + w_reputation × reputation
```

Default weight constants (see [`04-constants.md`](04-constants.md)):

```
w_confidence = 0.35
w_novelty    = 0.20
w_utility    = 0.30
w_reputation = 0.15
```

Sum: 1.0. The extended axes do not enter the default effective score formula; they are
used by specialized gates and Scorers that opt in.

---

## Score Struct

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

impl Score {
    /// Default: all stable axes at 0.5; no extended axes.
    pub fn default() -> Self;

    /// Compute the effective score using default weights.
    pub fn effective(&self) -> f64;

    /// Compute effective score with custom weights.
    pub fn effective_weighted(&self, weights: &ScoreWeights) -> f64;

    /// Returns true if all axis values are in [0.0, 1.0].
    pub fn validate(&self) -> Result<(), ScoreError>;
}
```

---

## See Also

- [`01-axes-stable.md`](01-axes-stable.md) — stable axes in depth
- [`02-axes-extended.md`](02-axes-extended.md) — extended axes
- [`03-arithmetic.md`](03-arithmetic.md) — effective score formula
- [`reference/01-engram/08-scoring-fields.md`](../../01-engram/08-scoring-fields.md) — how Score attaches to Engram
