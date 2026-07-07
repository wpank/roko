# Policy — Trait Surface

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

```rust
// source: crates/roko-core/src/policy.rs

/// Reactive post-action control operator.
///
/// Called after the agent observes the outcome of its action. Returns a
/// [`PolicyDecision`] that the loop executes.
pub trait Policy: Send + Sync {
    fn evaluate(
        &mut self,  // &mut self: policy may update internal state (circuit breaker window)
        outcome: &LoopOutcome,
        score: &Score,
    ) -> Result<PolicyDecision, PolicyError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    /// Continue the loop normally. Store and learn.
    Continue,
    /// Escalate to a human operator or higher-tier agent.
    Escalate { reason: String },
    /// Open the circuit breaker. Pause the loop for `cooldown_secs`.
    CircuitBreak { reason: String, cooldown_secs: u64 },
    /// Block the outgoing response. Substitute a safe response.
    SafetyOverride { blocked_response: String },
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("policy evaluation failed: {0}")]
    Computation(String),
}
```
<!-- source: crates/roko-core/src/policy.rs -->

---

Note: `&mut self` (not `&self`) because the circuit breaker maintains a rolling window of
outcomes. This makes `Policy` the only operator trait that requires mutability.

## See Also

- [Semantics](./02-semantics.md)
- [Invariants](./05-invariants.md)
