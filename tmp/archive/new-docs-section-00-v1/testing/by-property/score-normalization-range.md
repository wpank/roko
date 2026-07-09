# Score Normalization Range

> All Score axes are bounded to their defined ranges. Setting an axis outside its range is either rejected or clamped.

**Crate**: `roko-core`
**Test type**: Property-based (proptest)
**Enforcement**: `Score` field setters
**Last reviewed**: 2026-04-19

---

## Statement

For axes with range [0, 1]: `∀ v: score.set_axis(v).get_axis() ∈ [0.0, 1.0]`

For `valence` with range [-1, 1]: `∀ v: score.set_valence(v).get_valence() ∈ [-1.0, 1.0]`

---

## Property Test

```rust
proptest! {
    #[test]
    fn score_axes_in_range(
        novelty in any::<f32>(),
        valence in any::<f32>(),
    ) {
        let mut s = Score::default();
        s.set_novelty(novelty);
        s.set_valence(valence);

        prop_assert!(s.novelty() >= 0.0 && s.novelty() <= 1.0,
            "Novelty {} out of [0, 1]", s.novelty());
        prop_assert!(s.valence() >= -1.0 && s.valence() <= 1.0,
            "Valence {} out of [-1, 1]", s.valence());
    }
}
```

---

## See also

- [score-axis-independence.md](score-axis-independence.md)
- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
