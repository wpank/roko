# Scorer Semantics

> What each of the 7 score axes means, how the `prior` accumulation works, and what
> the effective score formula is.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md), [Score Type](../../10-types/score.md)
**Last reviewed**: 2026-04-19

---

## The 7 Axes in Depth

### Stable Axes (Required)

**`confidence`** — How certain is the information? Derived from the source's accuracy history,
the internal consistency of the `Engram`'s body, and cross-referencing with related memories.
A fresh observation from a reliable source starts near 1.0; inference from a noisy sensor
starts near 0.3.

**`novelty`** — How new is this information relative to what the agent already knows? Derived
by comparing the `Engram`'s HDC fingerprint distance to the nearest stored memories. A
memory with no close neighbours scores 1.0 (entirely new); a memory identical to an existing
one scores 0.0.

**`utility`** — How useful is this information for the current task? Utility is context-
dependent: it is derived from the semantic similarity between the `Engram` and the current
task context, and from historical outcome correlation (if this type of memory led to good
outcomes in the past, its utility is higher).

**`reputation`** — How trustworthy is the source that produced this `Engram`? Derived from the
`Provenance` attestation chain: a `LocalAgent` source has neutral reputation (0.5); a
`PeerWitness` or `ChainWitness` source can have higher or lower reputation based on track
record.

### Extended Axes (Optional, Default 0.5)

**`precision`** — How specific is the information? A highly specific fact scores near 1.0;
a vague general statement scores near 0.0.

**`salience`** — How attention-capturing is this information? Derived from the `Daimon` affect
cross-cut: information that triggers high arousal scores higher.

**`coherence`** — How internally consistent is the `Engram`'s body with itself and with
related memories? An `Engram` that contradicts stored facts scores low.

---

## Accumulation: The `prior` Parameter

The `prior` score allows a scorer chain to build a composite score without any scorer
needing to know what the others did.

**Rule**: A scorer SHOULD only modify the axes it is responsible for. It SHOULD return
the `prior` value unchanged for axes it does not set.

```rust
// source: crates/roko-core/src/scorer.rs
// A confidence-only scorer:
fn score(&self, engram: &Engram, prior: Score) -> Result<Score, ScorerError> {
    let confidence = self.compute_confidence(engram);
    Ok(Score {
        confidence,
        ..prior  // preserve all other axes from prior
    })
}
```
<!-- source: crates/roko-core/src/scorer.rs -->

This means scorer chains are **additive by default**: each scorer specialises in one or two
axes.

---

## The Effective Score Formula

When all scorers have run, the effective (composite) score used by downstream operators is:

```
effective(s) = w_conf * s.confidence
             + w_novl * s.novelty
             + w_util * s.utility
             + w_repu * s.reputation
```

Default weights: `w_conf = 0.4, w_novl = 0.2, w_util = 0.3, w_repu = 0.1`.

The extended axes (`precision`, `salience`, `coherence`) are available to specific operators
(e.g., `Gate` uses `coherence` for contradiction detection) but are not included in the
default effective score formula.

Weights are configurable per-agent in the configuration schema. See
[Configuration](../../operations/configuration/README.md).

---

## Score Clamping

All axes must remain in [0.0, 1.0]. Scorers that compute values outside this range must
clamp:

```rust
// source: crates/roko-core/src/scorer.rs
let raw = /* computation */;
let clamped = raw.clamp(0.0_f32, 1.0_f32);
```
<!-- source: crates/roko-core/src/scorer.rs -->

NaN and infinite values must not be returned — the invariant checker will return
`ScorerError::InvalidValue`.

---

## See Also

- [Score Type](../../10-types/score.md)
- [Invariants](./05-invariants.md)
- [Composition Patterns](./09-composition-patterns.md)
