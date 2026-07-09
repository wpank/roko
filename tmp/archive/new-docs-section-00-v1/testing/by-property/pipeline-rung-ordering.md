# Pipeline Rung Ordering

> The 7-rung gate pipeline always evaluates rungs in order 1 → 7. A rung is never evaluated before all lower-numbered rungs have completed.

**Crate**: `roko-gate`
**Test type**: Unit test
**Enforcement**: `GatePipeline::run`
**Last reviewed**: 2026-04-19

---

## Statement

For all pipeline runs: if rung R₂ has been evaluated, then rung R₁ (R₁ < R₂) has already been evaluated and passed.

In other words: rung evaluation is strictly sequential in the ordering 1 → 2 → 3 → 4 → 5 → 6 → 7.

---

## Test

```rust
#[test]
fn pipeline_evaluates_rungs_in_order() {
    let mut evaluation_log: Vec<u8> = Vec::new();
    let pipeline = GatePipeline::new_with_spy(|rung| evaluation_log.push(rung));

    let input = GateInput::valid_fixture();
    let _ = pipeline.run(&input);

    // Rungs must appear in strictly ascending order
    for window in evaluation_log.windows(2) {
        assert!(window[0] < window[1],
            "Rung {} was evaluated before rung {}", window[1], window[0]);
    }
}
```

---

## Short-Circuit Behaviour

When a rung fails, subsequent rungs are NOT evaluated. This is tested by:

```rust
#[test]
fn pipeline_short_circuits_on_rung_failure() {
    let pipeline = GatePipeline::new_with_forced_failure(at_rung = 3);
    let mut log: Vec<u8> = Vec::new();
    // …
    pipeline.run_with_spy(&input, |rung| log.push(rung));

    assert_eq!(log, vec![1, 2, 3], "Must stop at rung 3");
    assert!(!log.contains(&4), "Rung 4 must not be evaluated after rung 3 failure");
}
```

---

## See also

- [gate-verdict-monotonicity.md](gate-verdict-monotonicity.md)
- [../by-subsystem/subsystem-gate.md](../by-subsystem/subsystem-gate.md)
