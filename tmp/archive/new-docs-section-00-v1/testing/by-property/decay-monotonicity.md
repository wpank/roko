# Decay Monotone Non-Increasing

> An Engram's decay score at time t+Δ is always ≤ its decay score at time t. Decay only goes down, never up.

**Crate**: `roko-core`
**Test type**: Property-based (proptest)
**Enforcement**: All `Decay` variant implementations
**Last reviewed**: 2026-04-19

---

## Statement

For all decay models D, all initial values v₀ in (0, 1], and all time points t₁ < t₂:

`D.value_at(t₁) ≥ D.value_at(t₂)`

The four decay variants must all satisfy this: `Exponential`, `Linear`, `Step`, `None`.

---

## Why It Matters

Decay drives the memory management of the Roko substrate:
- Engrams with score below the GC threshold are candidates for collection.
- The GC scheduler assumes it can compute a monotone time-to-zero estimate.
- Retrieval scoring gives higher weight to newer or less-decayed Engrams.

If decay were non-monotone (sometimes increasing), the GC scheduler would miss collection windows, retrieval scores would oscillate unpredictably, and the "living" vs. "dead" Engram distinction would become undefined.

---

## Variant Specifications

| Variant | Value at t=0 | Value as t→∞ | Monotone? |
|---|---|---|---|
| `Exponential(λ)` | v₀ | 0 | Yes: v(t) = v₀ × e^(-λt) |
| `Linear(lifetime)` | v₀ | 0 at t=lifetime | Yes: v(t) = max(0, v₀ × (1 - t/lifetime)) |
| `Step(step_time)` | v₀ | 0 after step_time | Yes: v(t) = v₀ if t < step_time, else 0 |
| `None` | v₀ | v₀ (constant) | Trivially yes: non-decreasing and non-increasing |

---

## Property Test

```rust
proptest! {
    #[test]
    fn decay_monotone_nonincreasing(
        initial_value in 0.0f64..=1.0,
        t1 in 0.0f64..1000.0,
        dt in 0.0f64..100.0,
        decay in arb_decay_params(),
    ) {
        let t2 = t1 + dt;
        let v1 = decay.value_at(initial_value, t1);
        let v2 = decay.value_at(initial_value, t2);

        prop_assert!(
            v1 >= v2 - f64::EPSILON * 10.0, // allow small floating point error
            "Decay must be non-increasing: v({}) = {} > v({}) = {}",
            t1, v1, t2, v2
        );
    }
}
```

**File**: `crates/roko-core/src/decay.rs` (test module)

The floating-point epsilon allows for tiny rounding errors in the exponential computation; the semantic monotonicity is still enforced.

---

## Related Properties

- [decay-exponential-asymptote.md](decay-exponential-asymptote.md) — exponential reaches 0
- [decay-linear-terminus.md](decay-linear-terminus.md) — linear reaches exactly 0 at lifetime
- [substrate-gc-preserves-living.md](substrate-gc-preserves-living.md) — GC depends on monotone decay

## See also

- [../by-subsystem/subsystem-core.md](../by-subsystem/subsystem-core.md)
