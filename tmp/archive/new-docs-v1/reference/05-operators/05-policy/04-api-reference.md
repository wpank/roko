# Policy — API Reference

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## Trait

```rust
// source: crates/roko-core/src/policy.rs

pub trait Policy: Send + Sync {
    fn evaluate(
        &mut self,
        outcome: &LoopOutcome,
        score: &Score,
    ) -> Result<PolicyDecision, PolicyError>;
}
```
<!-- source: crates/roko-core/src/policy.rs -->

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `outcome` | `&LoopOutcome` | The result of the agent's action this tick. |
| `score` | `&Score` | The Score produced by the Scorer this tick. |

### Return value

`Ok(PolicyDecision)` — the loop executes the decision.  
`Err(PolicyError)` — the loop defaults to `PolicyDecision::Continue` (fail-open).

---

## `PolicyDecision` variants

```rust
// source: crates/roko-core/src/policy.rs

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Continue,
    Escalate { reason: String },
    CircuitBreak { reason: String, cooldown_secs: u64 },
    SafetyOverride { blocked_response: String },
}
```
<!-- source: crates/roko-core/src/policy.rs -->

| Variant | Loop effect |
|---|---|
| `Continue` | Store outcome, proceed to LEARN. |
| `Escalate { reason }` | Serialise loop state → `EscalationPacket` → publish to `agent.escalation` topic. Loop pauses. |
| `CircuitBreak { reason, cooldown_secs }` | Loop pauses for `cooldown_secs`, then enters half-open. |
| `SafetyOverride { blocked_response }` | Block response, substitute safe response, log for audit. |

---

## `PolicyError`

```rust
// source: crates/roko-core/src/policy.rs

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("policy evaluation failed: {0}")]
    Computation(String),
}
```
<!-- source: crates/roko-core/src/policy.rs -->

`PolicyError::Computation` is the only variant. When `evaluate` returns this error, the
runtime logs it and falls through to `Continue`. See [Failure Modes](./06-failure-modes.md).

---

## `CircuitBreakerConfig` fields

```rust
// source: crates/roko-core/src/policy.rs

pub struct CircuitBreakerConfig {
    pub window_size: usize,           // Default: 10
    pub failure_threshold: f32,       // Default: 0.6  (60%)
    pub cooldown_secs: u64,           // Default: 30
    pub escalation_threshold: f32,    // Default: 0.9
    pub escalation_streak: usize,     // Default: 3
}
```
<!-- source: crates/roko-core/src/policy.rs -->

| Field | Default | Effect |
|---|---|---|
| `window_size` | 10 | Number of past ticks in the rolling window. |
| `failure_threshold` | 0.6 | Failure rate [0.0–1.0] that opens the circuit. |
| `cooldown_secs` | 30 | Seconds the loop pauses when the circuit opens. |
| `escalation_threshold` | 0.9 | Prediction error magnitude that starts the escalation streak. |
| `escalation_streak` | 3 | Consecutive ticks above `escalation_threshold` before `Escalate` fires. |

---

## `LoopOutcome` variants visible to Policy

Policy receives the full `LoopOutcome`. The variants it acts on:

| `LoopOutcome` variant | Policy treatment |
|---|---|
| `LoopOutcome::Ok { .. }` | Success — adds `true` to rolling window. |
| `LoopOutcome::Rejected` | Failure — adds `false` to rolling window. |
| `LoopOutcome::LlmError(_)` | Failure — adds `false` to rolling window. |
| `LoopOutcome::SafetyViolation { .. }` | Triggers `SafetyOverride` immediately. |

---

## `EscalationPacket` (published to Bus on `Escalate`)

<!-- ADDED: Inferred from escalation semantics in source architecture docs -->

```rust
// source: crates/roko-core/src/policy.rs

#[derive(Debug, serde::Serialize)]
pub struct EscalationPacket {
    pub agent_id: AgentId,
    pub tick: u64,
    pub reason: String,
    pub last_outcome: LoopOutcome,
    pub last_score: Score,
    pub circuit_state: String,  // "open" | "half_open" | "closed"
}
```
<!-- source: crates/roko-core/src/policy.rs -->

Published to the `agent.escalation` Bus topic. Downstream consumers (human dashboards,
supervisor agents) subscribe to this topic to handle escalations.

---

## See Also

- [Semantics](./02-semantics.md)
- [Invariants](./05-invariants.md)
- [Failure Modes](./06-failure-modes.md)
