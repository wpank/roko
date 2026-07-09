# C-Factor Bounds

> The C-Factor collective intelligence metric is always in [0, 1].

**Crate**: `roko-learn`
**Test type**: Unit test + property test
**Enforcement**: `CFactor::compute`
**Last reviewed**: 2026-04-19

---

## Statement

For all sets of agent outputs O = {o₁, o₂, ..., oₙ}:
`0.0 ≤ CFactor::compute(O) ≤ 1.0`

---

## Why It Matters

The C-Factor is used as a routing signal and a quality metric in learning. Routing decisions that depend on C-Factor assume it is a probability-like quantity in [0, 1].

---

## Property Test

```rust
proptest! {
    #[test]
    fn c_factor_in_unit_interval(
        outputs in proptest::collection::vec(arb_agent_output(), 1..=20),
    ) {
        let cf = CFactor::compute(&outputs);
        prop_assert!(cf >= 0.0 && cf <= 1.0,
            "C-Factor {} must be in [0, 1]", cf);
    }
}
```

---

## See also

- [../by-subsystem/subsystem-learn.md](../by-subsystem/subsystem-learn.md)
