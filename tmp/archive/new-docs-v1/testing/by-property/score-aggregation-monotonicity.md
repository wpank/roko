# Score Aggregation Monotonicity

> Increasing any axis value in a Score, all else equal, must not decrease the aggregated weighted score.

**Crate**: `roko-core`
**Test type**: Unit test
**Enforcement**: `Score::aggregate`
**Last reviewed**: 2026-04-19

---

## Statement

For all `Score` values S and all axes A with non-negative weight w_A:
`S.aggregate() ≤ S.with_axis(A, S.get_axis(A) + δ).aggregate()` for all δ ≥ 0

---

## Property Test

```rust
proptest! {
    #[test]
    fn score_aggregate_monotone_in_axes(
        score in arb_score(),
        axis in arb_axis_with_positive_weight(),
        delta in 0.0f32..=0.5,
    ) {
        let original = score.aggregate();
        let new_value = (score.get_axis(axis) + delta).min(1.0);
        let increased = score.with_axis(axis, new_value);
        let new_agg = increased.aggregate();

        prop_assert!(
            new_agg >= original - f32::EPSILON * 10.0,
            "Increasing axis {:?} must not decrease aggregate score",
            axis
        );
    }
}
```

---

## See also

- [score-axis-independence.md](score-axis-independence.md)
- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
