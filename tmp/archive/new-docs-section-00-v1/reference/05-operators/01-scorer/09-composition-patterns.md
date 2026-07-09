# Scorer Composition Patterns

> Patterns for stacking Scorers, combining their outputs, and building specialised
> scoring chains.

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

## Pattern 1: Axis Specialisation

Each scorer in the chain owns one axis. The chain covers all axes without overlap:

```
RecencyScorer  → sets novelty
ConfidenceScorer → sets confidence (from provenance)
UtilityScorer → sets utility (from task similarity)
ReputationScorer → sets reputation (from provenance chain)
```

This is the recommended default chain. Each scorer is independently testable.

---

## Pattern 2: Refinement Chain

Each scorer refines an axis set by an earlier scorer. The second scorer may reduce or
increase an axis based on additional context:

```
DefaultScorer → sets all axes from basic heuristics
ContextualUtilityScorer → refines utility based on deep task analysis
```

The second scorer uses `prior.utility` as a baseline and adjusts:

```rust
// source: crates/roko-core/src/scorer.rs
let refined_utility = prior.utility * self.context_similarity(engram);
Ok(Score { utility: refined_utility.clamp(0.0, 1.0), ..prior })
```
<!-- source: crates/roko-core/src/scorer.rs -->

---

## Pattern 3: Gating by Score

Use a `ConstantScorer` as the first scorer to set a baseline, then use a custom scorer to
override specific axes for specific `Kind` values. Combine with a `Gate` that rejects on low
confidence.

---

## Anti-Pattern: Scorers That Override All Axes

A scorer that sets all 7 axes from scratch breaks the composition model — it silently
discards the output of all previous scorers. Use this only as the first (and only) scorer
in a chain.

---

## See Also

- [Trait Composition Model](../01-trait-composition-model.md)
- [Examples](./08-examples.md)
