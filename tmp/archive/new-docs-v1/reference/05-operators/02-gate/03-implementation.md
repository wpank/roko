# Gate Implementations

**Status**: Shipping
**Crate**: `roko-gate`
**Last reviewed**: 2026-04-19

---

## Shipping Implementations

### `ConfidenceGate`

Rejects `Engram`s whose `score.confidence` is below a configurable threshold.

```rust
// source: crates/roko-gate/src/lib.rs
pub struct ConfidenceGate {
    pub min_confidence: f32, // default: 0.4
}

impl Gate for ConfidenceGate {
    fn evaluate(&self, _engram: &Engram, score: &Score) -> Result<Verdict, GateError> {
        if score.confidence < self.min_confidence {
            Ok(Verdict::Reject(format!(
                "confidence {:.2} < threshold {:.2}",
                score.confidence, self.min_confidence
            )))
        } else {
            Ok(Verdict::Pass)
        }
    }
}
```
<!-- source: crates/roko-gate/src/lib.rs -->

### `SafetyGate`

Checks the `Engram` body against a list of prohibited patterns (regex or literal). Returns
`Reject` on a match.

### `CoherenceGate`

Uses `score.coherence` (from the extended axes) to reject `Engram`s that contradict stored
facts. Requires the extended axis to be populated by a `Scorer`.

### `PassAllGate`

Always returns `Pass`. Useful as a no-op placeholder in tests.

### `RejectAllGate`

Always returns `Reject("test rejection")`. Useful for testing rejection handling.

---

## See Also

- [Trait Surface](./01-trait-surface.md)
- [Gate Composition](./09-gate-composition.md)
