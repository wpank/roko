# Policy Semantics — Reactive Control

> Circuit breakers, escalation, and safety override: what each decision means and when
> the Policy triggers them.

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

## `PolicyDecision::Continue`

The loop is healthy. Store the outcome and proceed to LEARN. This is the default for
normal operation.

---

## `PolicyDecision::CircuitBreak`

The circuit breaker model treats the loop like an electrical circuit: too many failures in
a row "open" the circuit (stop the flow) to prevent cascading failures.

**Trigger condition** (default): failure rate > 60% in a 10-tick rolling window.

A "failure" is defined as:
- `LoopOutcome::Rejected` (gate rejection)
- LLM call returning an error
- Prediction error above `error_threshold`

On `CircuitBreak`:
1. The loop pauses for `cooldown_secs`.
2. The circuit moves to "half-open" state: allow one test tick.
3. If the test tick succeeds, the circuit closes (loop resumes).
4. If the test tick fails, the circuit opens again for another `cooldown_secs`.

---

## `PolicyDecision::Escalate`

Certain conditions warrant human attention or a higher-tier agent:

- Repeated failures on the same `ActionKind` (stuck in a failure loop).
- Safety concern detected in the outcome.
- Prediction error above `escalation_threshold` for N consecutive ticks.
- Explicit escalation request from the Composer output (model says "I can't help with this").

On `Escalate`:
1. The current loop state is serialised to a structured `EscalationPacket`.
2. The packet is published to the `agent.escalation` Bus topic.
3. The loop pauses until the escalation is resolved or times out.

---

## `PolicyDecision::SafetyOverride`

If the OBSERVE step detects a harmful or policy-violating response in `LoopOutcome`:
1. The original response is blocked.
2. `SafetyOverride { blocked_response }` is returned.
3. The loop substitutes a safe response (configured safe message or re-prompts with
   safety-explicit instructions).
4. The blocked response is logged (but not sent) for audit.

---

## Prediction Error Routing

Today, `Policy` also acts as the learning signal router: it computes the prediction error
(`expected_outcome - actual_outcome`) and publishes it to the `prediction.error` Bus topic,
which is consumed by the Delta-speed Dreams loop for offline consolidation.

In the target state, this learning-signal logic moves to `Calibrator`. See
[Policy vs. Calibrator](./09-policy-vs-calibrator.md).

---

## See Also

- [Circuit Breaker](./03-implementation.md) — `CircuitBreakerPolicy`
- [Policy vs. Calibrator](./09-policy-vs-calibrator.md)
- [Failure Modes](./06-failure-modes.md)
