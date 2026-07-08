# Gate Adaptive Threshold Bounds

> The EMA-based adaptive threshold for any gate stays within [floor, 1.0] at all times.

**Crate**: `roko-gate`
**Test type**: Unit test
**Enforcement**: `EmaThresholdManager::update`
**Last reviewed**: 2026-04-19

---

## Statement

For all gates G, all threshold floors `floor_G` in (0, 1), and all update sequences:

At all times: `threshold_G ∈ [floor_G, 1.0]`

---

## Why It Matters

A threshold above 1.0 would make the gate unpassable (no quality metric can exceed 1.0). A threshold below the configured floor would defeat the purpose of the monotonic ratchet.

---

## Property Test

```rust
proptest! {
    #[test]
    fn adaptive_threshold_stays_in_bounds(
        floor in 0.1f64..=0.5,
        verdicts in proptest::collection::vec(arb_verdict(), 1..=50),
    ) {
        let mut mgr = EmaThresholdManager::new(initial = 0.9, floor = floor, alpha = 0.1);

        for verdict in &verdicts {
            mgr.update(verdict);
            prop_assert!(
                mgr.threshold() >= floor && mgr.threshold() <= 1.0,
                "Threshold {} must be in [{}, 1.0]",
                mgr.threshold(), floor
            );
        }
    }
}
```

---

## See also

- [gate-verdict-monotonicity.md](gate-verdict-monotonicity.md)
- [../by-subsystem/subsystem-gate.md](../by-subsystem/subsystem-gate.md)
