# Gate Examples

**Status**: Shipping
**Crate**: `roko-gate`
**Last reviewed**: 2026-04-19

---

## Example 1: Single Confidence Gate

```rust
// source: crates/roko-gate/src/lib.rs
use roko_gate::{Gate, ConfidenceGate, Verdict};
use roko_core::Score;

let gate = ConfidenceGate { min_confidence: 0.6 };
let verdict = gate.evaluate(&engram, &score)?;
match verdict {
    Verdict::Pass => println!("passed"),
    Verdict::Reject(reason) => println!("rejected: {reason}"),
    Verdict::Abstain => println!("abstained"),
}
```
<!-- source: crates/roko-gate/src/lib.rs -->

---

## Example 2: Gate Pipeline

```rust
// source: crates/roko-gate/src/lib.rs
let gates: Vec<Box<dyn Gate>> = vec![
    Box::new(ConfidenceGate { min_confidence: 0.4 }),
    Box::new(SafetyGate::default()),
    Box::new(CoherenceGate { min_coherence: 0.3 }),
];

for gate in &gates {
    match gate.evaluate(&engram, &score)? {
        Verdict::Reject(reason) => {
            return Ok(LoopOutcome::Rejected(reason));
        }
        Verdict::Pass | Verdict::Abstain => continue,
    }
}
// All gates passed or abstained — proceed to routing.
```
<!-- source: crates/roko-gate/src/lib.rs -->

---

## See Also

- [Gate Composition](./09-gate-composition.md)
- [Semantics](./02-semantics.md)
