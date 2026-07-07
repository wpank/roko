# Engram — Scoring Fields

> The `score: Score` field records a 7-axis quality assessment of the Engram. It is not part of the identity hash and can be recomputed at any time.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Score type](../10-types/score/00-overview.md)  
**Used by**: Gate pipeline, Router, Substrate GC priority  
**Last reviewed**: 2026-04-19

---

## TL;DR

Every Engram carries a `Score` with up to 7 axes: confidence, novelty, utility,
reputation (stable), and precision, salience, coherence (extended). The `score`
field is mutable — Scorers recompute it without changing the Engram's identity.
Gate thresholds and Substrate GC priority are driven by the effective score.

---

## The Idea

Score is intentionally separated from identity. A GateVerdict that was initially scored
with low confidence can be re-evaluated after the prediction resolves, updating only
the score without creating a new Engram. The substrate can then re-rank Engrams for
retrieval based on updated scores.

The score is also separable from provenance: a `KnowledgeEntry` written by a low-trust
author can be re-scored upward if chain-witnesses attest it. The Engram doesn't change;
only the score changes.

---

## Specification

The `Score` type is specified in detail in
[`../10-types/score/`](../10-types/score/README.md). This page covers only the
attachment of Score to Engram.

### The Score Field

```rust
<!-- source: crates/roko-core/src/engram.rs -->

/// Quality score at emission time.
/// Not included in the identity hash — Scorers can recompute without changing id.
pub score: Score,
```

### Score at Emission

`EngramBuilder::build()` sets `score` to either:

1. A provided `Score` (via `.score(s)` on the builder), or
2. `Score::default()` — all stable axes at 0.5, extended axes absent.

In production, a Scorer should run immediately after `build()` to replace the default
score with a meaningful one:

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

let raw_engram = EngramBuilder::new()
    .kind(Kind::AgentOutput)
    .body(/* ... */)
    .build()?;

// Run through the scorer pipeline
let scored_engram = scorer_pipeline.score(raw_engram, &context)?;
```

### Score After Emission

The Substrate's `update_score` method replaces an Engram's score in-place:

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

trait Substrate {
    /// Replace the score for an existing Engram.
    /// Does not change the Engram's id.
    fn update_score(&self, id: &ContentHash, score: Score) -> Result<(), SubstrateError>;
}
```

This is used by:
- **Outcome-driven Scorers**: update `utility` after a gate verdict resolves.
- **Reputation Scorers**: update `reputation` after chain-witness attestation.
- **Calibration loops**: update `confidence` based on prediction accuracy history.

---

## The 7 Axes at a Glance

For the complete specification of each axis, see
[`../10-types/score/01-axes-stable.md`](../10-types/score/01-axes-stable.md) and
[`../10-types/score/02-axes-extended.md`](../10-types/score/02-axes-extended.md).

| Axis | Range | Stable? | Typical use |
|------|-------|---------|------------|
| `confidence` | 0.0–1.0 | Yes | How certain is the source? |
| `novelty` | 0.0–1.0 | Yes | Is this information new? |
| `utility` | 0.0–1.0 | Yes | Did this Engram help? |
| `reputation` | 0.0–1.0 | Yes | How trusted is the author? |
| `precision` | 0.0–1.0 | No (extended) | How specific and accurate? |
| `salience` | 0.0–1.0 | No (extended) | How relevant to current task? |
| `coherence` | 0.0–1.0 | No (extended) | How internally consistent? |

---

## How Score Drives Other Subsystems

### Gate Pipeline

Gates check the effective score against configured thresholds:

```rust
effective_score >= gate.threshold  →  pass
effective_score <  gate.threshold  →  fail
```

The effective score formula is:
`effective = w_confidence × confidence + w_novelty × novelty + w_utility × utility + w_reputation × reputation`

For constants and extended-axis weighting, see
[`../10-types/score/03-arithmetic.md`](../10-types/score/03-arithmetic.md).

### Substrate GC

When the Substrate runs garbage collection, Engrams with low effective scores are
candidates for eviction ahead of Engrams with high scores. Combined with decay, this
means low-quality stale information is removed first.

### Retrieval Ranking

Similarity searches return Engrams ranked by a combination of HDC similarity and
effective score. A highly-similar but low-scored Engram ranks below a less-similar
but high-scored one.

---

## Invariants

1. All axis values are in [0.0, 1.0]
2. `score` is not included in the identity hash
3. `update_score` does not change `id`, `kind`, `body`, or any hash-included field

---

## Failure Modes

| Failure | Cause | Recovery |
|---------|-------|----------|
| Score not updated after emission | Scorer not wired | Default score (0.5 all axes) persists; gate thresholds may be too loose |
| Score axis out of range | Scorer bug | Substrate normalizes on `update_score`; logs warning |

---

## See Also

- [`../10-types/score/00-overview.md`](../10-types/score/00-overview.md)
- [`../10-types/score/03-arithmetic.md`](../10-types/score/03-arithmetic.md) — effective score formula
- [`09-decay-fields.md`](09-decay-fields.md) — decay interacts with score for GC priority
