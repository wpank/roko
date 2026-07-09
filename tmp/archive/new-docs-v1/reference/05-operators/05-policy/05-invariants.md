# Policy — Invariants

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md), [Semantics](./02-semantics.md)
**Last reviewed**: 2026-04-19

---

These invariants hold for all correct Policy implementations. Violating them is a bug.

---

## I1 — Called exactly once per tick, after OBSERVE

Policy is called in the LEARN phase, after the OBSERVE step has produced a `LoopOutcome`
and after the Scorer has produced a `Score`. It is never called before the action, never
called mid-action, and never called more than once per tick.

```
tick N:
  SENSE → RECALL → SCORE → GATE → ROUTE/ACT → OBSERVE → [Policy.evaluate()] → STORE
```

Calling Policy before OBSERVE would mean no outcome data is available. The trait
signature makes this impossible: `evaluate` requires `&LoopOutcome`.

---

## I2 — `PolicyError` must default to `Continue`

If `evaluate` returns `Err(PolicyError)`, the loop **must** proceed as if the return value
were `Ok(PolicyDecision::Continue)`. Policy is a control circuit, not a safety gate. A
failed policy evaluation must not lock the loop — that would turn a transient bug into a
permanent denial of service.

This is the **fail-open** principle. Contrast with Gate, which can safely fail-closed
(see [Gate Invariants](../02-gate/05-invariants.md)).

<!-- ADDED: fail-open vs fail-closed distinction inferred from architecture safety principles -->

---

## I3 — `&mut self` is permitted but mutation must be bounded

Policy is the only operator trait that takes `&mut self`. Implementations are permitted
to mutate internal state (rolling window, streak counters, circuit state). However:

- Mutation must be **O(1)** per call with bounded memory (use ring buffers, not growing vecs).
- State must not be shared across threads without a lock; the loop calls Policy from a
  single async task and the `Send + Sync` bound is satisfied by ownership, not by interior
  mutability.

---

## I4 — `SafetyOverride` takes priority over all other decisions

When both a safety violation and a circuit breaker threshold are detected in the same
tick, `SafetyOverride` must be returned. The priority order is:

1. `SafetyOverride` (highest)
2. `Escalate`
3. `CircuitBreak`
4. `Continue` (lowest)

The `CircuitBreakerPolicy` implementation encodes this order explicitly. Custom
implementations must replicate it.

<!-- ADDED: Priority ordering inferred from safety-first architecture principle -->

---

## I5 — `CircuitBreak` cooldown is enforced by the loop, not by Policy

`CircuitBreakerPolicy` returns `CircuitBreak { cooldown_secs }` and immediately changes
internal state to `CircuitState::Open`. It does **not** sleep. The loop runtime is
responsible for sleeping `cooldown_secs` and then transitioning the circuit to `HalfOpen`
by calling `evaluate` again.

This separation ensures Policy remains a pure decision function that can be unit-tested
without real-time waits.

---

## I6 — Stacking policies

When safety and circuit-breaking are both required but implemented in separate types,
stack them with the `ComposedPolicy` wrapper:

```rust
// source: crates/roko-core/src/policy.rs

/// Evaluates an ordered list of policies. Returns the first non-Continue decision.
pub struct ComposedPolicy {
    inner: Vec<Box<dyn Policy>>,
}

impl Policy for ComposedPolicy {
    fn evaluate(
        &mut self,
        outcome: &LoopOutcome,
        score: &Score,
    ) -> Result<PolicyDecision, PolicyError> {
        for policy in &mut self.inner {
            let decision = policy.evaluate(outcome, score)?;
            if decision != PolicyDecision::Continue {
                return Ok(decision);
            }
        }
        Ok(PolicyDecision::Continue)
    }
}
```
<!-- source: crates/roko-core/src/policy.rs -->

Place `SafetyPolicy` first in the `inner` vec to preserve the I4 priority order.

---

## I7 — Policy does not modify `LoopOutcome` or `Score`

Policy receives references, not ownership. It observes but does not mutate the outcome or
score. Any modifications to loop state are expressed only through the returned
`PolicyDecision`.

---

## Open Questions

- Should `ComposedPolicy` short-circuit on `Err` (current behaviour) or continue to the
  next policy? The current implementation propagates the error upward, which satisfies I2
  at the loop level but not within the composed stack.
