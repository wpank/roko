# Gate Verdict Idempotence

> Evaluating a gate on the same input at the same threshold always returns the same verdict.

**Crate**: `roko-gate`
**Test type**: Property-based (proptest)
**Enforcement**: `Gate::evaluate` — must be a pure function of (input, threshold)
**Last reviewed**: 2026-04-19

---

## Statement

For all gates G, all valid inputs I, and all thresholds t:

`G.evaluate(I, t) == G.evaluate(I, t)` (evaluated twice, same result)

---

## Why It Matters

Gate evaluation is expensive (involves `cargo build`, `cargo test`, etc.). The gate pipeline relies on idempotence for:
- **Retry safety**: if a gate fails due to a transient infrastructure error, re-running it on the same input produces the same verdict (or a new verdict if the input changed).
- **Caching**: gate verdicts can be cached keyed by content hash of the input. Idempotence guarantees the cache is valid.
- **Forensic replay**: the causal replay system re-evaluates gates on historical inputs to explain verdicts. Non-idempotent gates would make replay unreliable.

---

## Property Test

```rust
proptest! {
    #[test]
    fn gate_verdict_idempotent(
        input in arb_gate_input(),
        threshold in 0.0f64..=1.0,
    ) {
        let gate = TestGate::default();
        let v1 = gate.evaluate(&input, threshold);
        let v2 = gate.evaluate(&input, threshold);

        prop_assert_eq!(
            v1, v2,
            "Gate evaluation must be idempotent for input {:?} at threshold {}",
            input.id(), threshold
        );
    }
}
```

**File**: `crates/roko-gate/src/tests/idempotence_tests.rs`

---

## Scope

This property applies to all gates with deterministic evaluation:
- `CompileGate`, `LintGate`, `TestGate`, `SymbolGate`, `GeneratedTestGate`, `PropertyTestGate`, `IntegrationGate`, `FormatGate`, `SecurityGate`.

The `SemanticGate` (LLM-based review) is excluded: its verdict depends on LLM output, which is non-deterministic in production. In tests, the `SemanticGate` is run with a fixed replay tape, making it locally idempotent.

---

## Non-Idempotent Infrastructure Errors

If the gate evaluator encounters an infrastructure error (disk full, compiler not found), it returns `Err(InfrastructureError)`, not `Verdict::Fail`. Infrastructure errors are not subject to the idempotence requirement — they may resolve between calls. The gate is idempotent for its application-level verdict logic.

---

## Related Properties

- [gate-verdict-monotonicity.md](gate-verdict-monotonicity.md) — threshold monotonicity depends on idempotence
- [pipeline-rung-ordering.md](pipeline-rung-ordering.md)

## See also

- [../by-subsystem/subsystem-gate.md](../by-subsystem/subsystem-gate.md)
