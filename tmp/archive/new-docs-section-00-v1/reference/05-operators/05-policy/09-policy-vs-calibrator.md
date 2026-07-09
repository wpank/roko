# Policy vs. Calibrator

**Status**: Policy = Shipping · Calibrator = Specified
**Crate**: `roko-core` (Policy) · `roko-core` (Calibrator, planned)
**Depends on**: [Policy Semantics](./02-semantics.md)
**Last reviewed**: 2026-04-19

---

## Today: Policy does two jobs

`Policy` currently handles both **reactive control** and **learning signal routing**:

```
LoopOutcome + Score
       │
       ▼
  Policy.evaluate()
       │
       ├── PolicyDecision (Continue / CircuitBreak / Escalate / SafetyOverride)
       │            ▲ control: stops bad loops
       │
       └── Prediction error  ──► Bus topic: prediction.error
                    ▲ learning: feeds Dreams (offline consolidation)
```

Mixing control and learning in one trait violates the single-responsibility principle and
makes it impossible to replace the learning algorithm without touching the safety-critical
control code.

---

## Target state: Policy + Calibrator

The planned split introduces `Calibrator` as a separate operator called in the same
LEARN phase but after Policy:

```
LoopOutcome + Score
       │
       ├─► Policy.evaluate()     → PolicyDecision  (control only)
       │
       └─► Calibrator.calibrate() → CalibrationSignal (learning only)
                                          │
                                          ▼
                                   Bus: prediction.error, reward.signal
                                          │
                                          ▼
                                   Dreams (Delta-speed offline loop)
```

```rust
// source: crates/roko-core/src/calibrator.rs  [planned]

/// Learning signal router. Called once per tick after Policy.
/// Computes prediction error and routes it to the correct Bus topic.
pub trait Calibrator: Send + Sync {
    fn calibrate(
        &self,   // &self: calibrators are stateless routers
        outcome: &LoopOutcome,
        score: &Score,
    ) -> Result<CalibrationSignal, CalibratorError>;
}

pub struct CalibrationSignal {
    pub prediction_error: f32,
    pub reward: f32,
    pub target_topic: Topic,
}
```
<!-- source: crates/roko-core/src/calibrator.rs -->

---

## Comparison table

| Dimension | Policy (Shipping) | Calibrator (Specified) |
|---|---|---|
| Responsibility | Reactive loop control | Learning signal routing |
| Self mutability | `&mut self` | `&self` |
| Decisions returned | `PolicyDecision` | `CalibrationSignal` |
| Priority | Higher (runs first) | Lower (runs after Policy) |
| Fail behaviour | Fail-open → Continue | Fail-silent → no signal sent |
| Current status | Ships in every agent | Placeholder in loop; Dreams reads `prediction.error` directly for now |

---

## What moves from Policy to Calibrator

| Today in `Policy` | Target: moves to `Calibrator` |
|---|---|
| `prediction_error = expected - actual` computation | ✓ |
| Publishing to `prediction.error` Bus topic | ✓ |
| Publishing to `reward.signal` Bus topic | ✓ |
| Learning rate / discount factor calculation | ✓ |
| Circuit break logic | stays in Policy |
| Escalation logic | stays in Policy |
| Safety override logic | stays in Policy |

---

## Migration path

1. Add `Calibrator` trait to `roko-core` behind a `calibrator` feature flag.
2. Extract prediction error computation from `CircuitBreakerPolicy` into a
   `DefaultCalibrator` implementation.
3. Add `calibrator: Option<Box<dyn Calibrator>>` to the loop config struct.
4. Once `DefaultCalibrator` is validated, remove prediction error code from Policy.
5. Promote `Calibrator` status from Specified → Shipping.

This migration is backwards-compatible: agents that do not opt in to `Calibrator` keep
the current behaviour.

---

## See Also

- [Policy Semantics](./02-semantics.md) — current prediction error routing description
- [Bus Topics](../../04-bus/02-topics.md) — `prediction.error` topic
- [Rationale](./10-rationale.md) — why this split is deferred to a later release
