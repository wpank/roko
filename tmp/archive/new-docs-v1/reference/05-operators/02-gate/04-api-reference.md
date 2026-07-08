# Gate API Reference

**Status**: Shipping
**Crate**: `roko-gate`
**Last reviewed**: 2026-04-19

---

## Trait

```rust
// source: crates/roko-gate/src/lib.rs
pub trait Gate: Send + Sync {
    fn evaluate(&self, engram: &Engram, score: &Score) -> Result<Verdict, GateError>;
}
```
<!-- source: crates/roko-gate/src/lib.rs -->

## `Verdict` Variants

| Variant | Meaning |
|---|---|
| `Verdict::Pass` | Gate approves |
| `Verdict::Reject(String)` | Gate rejects with reason |
| `Verdict::Abstain` | Gate abstains |

## `GateError` Variants

| Variant | Meaning |
|---|---|
| `Computation(String)` | Gate implementation failed |

## Shipping Implementations

| Type | Rejects when |
|---|---|
| `ConfidenceGate { min_confidence }` | `score.confidence < min_confidence` |
| `SafetyGate` | Body matches prohibited patterns |
| `CoherenceGate` | `score.coherence < threshold` |
| `PassAllGate` | Never |
| `RejectAllGate` | Always |
