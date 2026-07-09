# Gate Verdict Monotonicity

> A gate that passes input I at threshold t must also pass input I at any threshold t' < t. Lowering the bar cannot turn a pass into a fail.

**Crate**: `roko-gate`
**Test type**: Property-based (proptest)
**Enforcement**: `Gate::evaluate`, threshold comparison logic
**Last reviewed**: 2026-04-19

---

## Statement

For all gates G, all valid inputs I, and all thresholds t, t' where 0 ≤ t' < t ≤ 1:

`G.evaluate(I, t) == Verdict::Pass → G.evaluate(I, t') == Verdict::Pass`

Equivalently: the pass/fail boundary is monotone in the threshold parameter. An input that passes at a stricter threshold must also pass at a looser threshold.

---

## Why It Matters

Monotonic ratcheting is a core gate design principle: the gate pipeline can only become stricter over time, never looser (without an explicit override). This property underpins the monotonic ratcheting behaviour:

- If a codebase passes Rung 3 (TestGate) at threshold 0.95, it must still pass at threshold 0.90.
- If the threshold is raised to 0.98 (ratchet tightening), new code must meet the higher bar.
- A threshold regression (lowering the bar) is a deliberate override requiring a justification comment.

Without monotonicity, a ratchet "tightening" would have unpredictable effects on inputs near the boundary.

---

## Property Test

```rust
proptest! {
    #[test]
    fn gate_verdict_monotone_in_threshold(
        input in arb_gate_input(),
        threshold in 0.0f64..=1.0,
        lower_threshold in 0.0f64..=1.0,
    ) {
        let gate = CompileGate::default(); // most predictable gate for this test
        let high_t = threshold.max(lower_threshold);
        let low_t = threshold.min(lower_threshold);

        let verdict_high = gate.evaluate(&input, high_t);
        let verdict_low = gate.evaluate(&input, low_t);

        match verdict_high {
            Ok(Verdict::Pass { .. }) => {
                // If it passes at the higher threshold, must also pass at the lower
                prop_assert!(
                    matches!(verdict_low, Ok(Verdict::Pass { .. })),
                    "Passed at threshold {} but failed at lower threshold {}",
                    high_t, low_t
                );
            }
            Ok(Verdict::Fail { .. }) | Err(_) => {
                // Failing at higher threshold says nothing about lower threshold
            }
        }
    }
}
```

**File**: `crates/roko-gate/src/tests/monotonicity_tests.rs`

---

## Scope

This property is tested for gates that have a numeric quality metric: `CompileGate` (0 errors = metric 1.0), `LintGate` (warning count → metric), `TestGate` (pass rate → metric), `PropertyTestGate`.

Gates with binary verdicts (`FormatGate`: formatted / not formatted) are excluded — their verdicts do not vary with numeric thresholds.

---

## Enforcement in Production

The `EmaThresholdManager` enforces monotonic ratcheting in production:
- Threshold can only decrease (loosen) in response to repeated failures.
- Threshold can only increase (tighten) in response to repeated passes.
- A manual override to lower the threshold below the ratchet floor requires a signed justification record in the Engram substrate.

---

## Related Properties

- [gate-verdict-idempotence.md](gate-verdict-idempotence.md) — same input, same verdict
- [gate-adaptive-threshold-bounds.md](gate-adaptive-threshold-bounds.md) — threshold stays within valid range
- [pipeline-rung-ordering.md](pipeline-rung-ordering.md) — rungs are evaluated in order

## See also

- [../by-subsystem/subsystem-gate.md](../by-subsystem/subsystem-gate.md) — gate coverage overview
