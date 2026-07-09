# Score Axis Independence

> Mutating one axis of a `Score` must not change the value of any other axis.

**Crate**: `roko-core`
**Test type**: Property-based (proptest)
**Enforcement**: `Score` struct field accessors
**Last reviewed**: 2026-04-19

---

## Statement

For all valid `Score` values S and for all axes A, B where A ≠ B:
`S.set_axis(A, v).get_axis(B) == S.get_axis(B)`

In words: setting axis A to any value v leaves axis B's value unchanged.

The 7 axes are: `novelty`, `relevance`, `confidence`, `valence`, `arousal`, `coherence`, `utility`.

---

## Why It Matters

The 7-axis Score is used to rank Engrams in retrieval, route tasks, modulate decay, and drive learning updates. If axes were not independent:
- A scoring operation targeting one axis would corrupt others.
- Scorer implementations would require global knowledge of all other axis values.
- The Score could not be composed from partial assessments (a `NoveltyScorer` setting novelty should not touch relevance).

---

## Implementation

The Score type stores each axis in an independent `f32` field:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    novelty: f32,      // [0.0, 1.0]
    relevance: f32,    // [0.0, 1.0]
    confidence: f32,   // [0.0, 1.0]
    valence: f32,      // [-1.0, 1.0] (positive/negative)
    arousal: f32,      // [0.0, 1.0]
    coherence: f32,    // [0.0, 1.0]
    utility: f32,      // [0.0, 1.0]
}
```

<!-- source: crates/roko-core/src/score.rs -->

Independence is structural: each axis has an independent storage location, and setters touch only their own field.

---

## Property Test

```rust
proptest! {
    #[test]
    fn score_axis_independence(
        novelty in 0.0f32..=1.0,
        relevance in 0.0f32..=1.0,
        new_novelty in 0.0f32..=1.0,
    ) {
        let mut s = Score::default();
        s.set_novelty(novelty);
        s.set_relevance(relevance);

        // Mutate novelty only
        s.set_novelty(new_novelty);

        // Relevance must be unchanged
        prop_assert_eq!(
            s.relevance(), relevance,
            "Setting novelty to {} must not change relevance from {}",
            new_novelty, relevance
        );
    }
}
```

**File**: `crates/roko-core/src/score.rs` (test module)

A full cross-product version tests all 42 axis pairs (7 × 6).

---

## Related Properties

- [score-normalization-range.md](score-normalization-range.md) — axes are bounded
- [score-aggregation-monotonicity.md](score-aggregation-monotonicity.md) — aggregation respects axis independence

## See also

- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
