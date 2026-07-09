# PAD Vector Bounds

> All three axes of a PAD vector (Pleasure, Arousal, Dominance) are always in [-1.0, 1.0].

**Crate**: `roko-daimon`
**Test type**: Unit test + property test
**Enforcement**: `PadVector::new`, `PadVector::set_axis`
**Last reviewed**: 2026-04-19

---

## Statement

For all valid PAD vectors V:
- `-1.0 ≤ V.pleasure ≤ 1.0`
- `-1.0 ≤ V.arousal ≤ 1.0`
- `-1.0 ≤ V.dominance ≤ 1.0`

---

## Enforcement

Axis setters clamp to [-1, 1]. Construction from out-of-range values returns `Err(OutOfRange)`.

---

## Property Test

```rust
proptest! {
    #[test]
    fn pad_vector_axes_in_range(
        p in -1.0f32..=1.0,
        a in -1.0f32..=1.0,
        d in -1.0f32..=1.0,
    ) {
        let v = PadVector::new(p, a, d).unwrap();
        prop_assert!(v.pleasure() >= -1.0 && v.pleasure() <= 1.0);
        prop_assert!(v.arousal() >= -1.0 && v.arousal() <= 1.0);
        prop_assert!(v.dominance() >= -1.0 && v.dominance() <= 1.0);
    }
}
```

---

## See also

- [../by-subsystem/subsystem-daimon.md](../by-subsystem/subsystem-daimon.md)
- [daimon-no-terminal-state.md](daimon-no-terminal-state.md)
