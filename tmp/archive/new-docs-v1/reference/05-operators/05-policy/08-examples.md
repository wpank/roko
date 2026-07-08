# Policy — Examples

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Implementation](./03-implementation.md), [API Reference](./04-api-reference.md)
**Last reviewed**: 2026-04-19

---

## Example 1 — Minimal circuit breaker setup

The most common production pattern: create a `CircuitBreakerPolicy` with default config
and register it with the loop.

```rust
// source: crates/roko-core/src/policy.rs

use roko_core::policy::{CircuitBreakerConfig, CircuitBreakerPolicy, Policy};

let config = CircuitBreakerConfig {
    window_size: 10,
    failure_threshold: 0.6,
    cooldown_secs: 30,
    escalation_threshold: 0.9,
    escalation_streak: 3,
};

let policy: Box<dyn Policy> = Box::new(CircuitBreakerPolicy::new(config));
```
<!-- source: crates/roko-core/src/policy.rs -->

---

## Example 2 — Loop executing a `PolicyDecision`

This is how the loop runtime handles the decision returned by `policy.evaluate`:

```rust
// source: crates/roko-runtime/src/loop_tick.rs

match policy.evaluate(&outcome, &score) {
    Ok(PolicyDecision::Continue) => {
        // Normal path: store and learn.
        substrate.put(&observation_key, &outcome.to_bytes()).await?;
    }
    Ok(PolicyDecision::CircuitBreak { reason, cooldown_secs }) => {
        tracing::warn!("circuit breaker opened: {reason}");
        metrics::increment_counter!("policy.circuit_break");
        tokio::time::sleep(Duration::from_secs(cooldown_secs)).await;
        // Next tick will be in half-open state.
    }
    Ok(PolicyDecision::Escalate { reason }) => {
        let packet = EscalationPacket {
            agent_id: ctx.agent_id,
            tick: ctx.tick,
            reason,
            last_outcome: outcome.clone(),
            last_score: score.clone(),
            circuit_state: "open".to_string(),
        };
        bus.publish(Topic::new("agent.escalation"), &packet).await?;
        tracing::error!("escalating: {:?}", packet);
        // Loop pauses here; external resolution resumes it.
    }
    Ok(PolicyDecision::SafetyOverride { blocked_response }) => {
        tracing::warn!("safety override: blocking response");
        audit_log.write(&blocked_response).await?;
        // Substitute safe response in outgoing message.
        let safe = ctx.safe_response_template.clone();
        ctx.outgoing_response = safe;
    }
    Err(e) => {
        // Fail-open: log and continue.
        tracing::error!("policy error (fail-open): {e}");
        metrics::increment_counter!("policy.error");
    }
}
```
<!-- source: crates/roko-runtime/src/loop_tick.rs -->

---

## Example 3 — Safety + circuit breaker stacked

Combine `SafetyPolicy` and `CircuitBreakerPolicy` so that both checks run each tick.

```rust
// source: crates/roko-core/src/policy.rs

use roko_core::policy::{ComposedPolicy, SafetyPolicy, CircuitBreakerPolicy};

let policy = ComposedPolicy::new(vec![
    // Safety check runs first (highest priority per I4).
    Box::new(SafetyPolicy::new(vec![
        Box::new(KeywordClassifier::new(&["harmful", "illegal"])),
        Box::new(RegexClassifier::new(r"(?i)\bviolence\b")),
    ])),
    // Circuit breaker runs second.
    Box::new(CircuitBreakerPolicy::new(CircuitBreakerConfig::default())),
]);
```
<!-- source: crates/roko-core/src/policy.rs -->

---

## Example 4 — Tight circuit breaker for a fragile external API

When the agent is calling an external API known to rate-limit aggressively, use a smaller
window and longer cooldown.

```rust
// source: crates/roko-core/src/policy.rs

let config = CircuitBreakerConfig {
    window_size: 5,
    failure_threshold: 0.4,   // Open at 2/5 failures (40%)
    cooldown_secs: 120,        // 2-minute cooldown to let rate limit reset
    escalation_threshold: 0.8,
    escalation_streak: 2,
};
```
<!-- source: crates/roko-core/src/policy.rs -->

<!-- ADDED: Tight-breaker pattern inferred from rate-limit use case in architecture docs -->

---

## Example 5 — Unit test with `PassPolicy`

```rust
// source: crates/roko-core/src/policy.rs

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::policy::PassPolicy;

    #[test]
    fn pass_policy_always_continues() {
        let mut policy = PassPolicy;
        let outcome = LoopOutcome::Ok { response: "hello".into() };
        let score = Score { value: 0.8, prediction_error: 0.1 };
        assert_eq!(
            policy.evaluate(&outcome, &score).unwrap(),
            PolicyDecision::Continue,
        );
    }
}
```
<!-- source: crates/roko-core/src/policy.rs -->

---

## Example 6 — Observing circuit state in telemetry

<!-- ADDED: Telemetry pattern inferred from metrics usage across codebase -->

`CircuitBreakerPolicy` exposes a read-only method for dashboards:

```rust
// source: crates/roko-core/src/policy.rs

// In your metrics export loop:
let state = policy.circuit_state(); // Returns "closed" | "half_open" | "open"
metrics::gauge!("policy.circuit_state", state_to_f64(state));
```
<!-- source: crates/roko-core/src/policy.rs -->

---

## See Also

- [Implementation](./03-implementation.md)
- [Failure Modes](./06-failure-modes.md)
- [Policy vs. Calibrator](./09-policy-vs-calibrator.md)
