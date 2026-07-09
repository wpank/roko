# Policy Overview

> `Policy` is the reactive control operator. It runs after the agent acts and observes the
> outcome. It decides whether the loop should continue, escalate, break, or adjust its
> parameters in response to what happened.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Score](../../10-types/score.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Policy::evaluate(outcome, score) -> PolicyDecision` takes the loop outcome and decides what
to do next: `Continue`, `Escalate`, `CircuitBreak`, or `SafetyOverride`. The shipping
implementation is `CircuitBreakerPolicy` — a window-based failure rate monitor.

---

## What Policy Does

An agent that acts without consequences is a random oracle. Policy is where consequence feeds
back into behaviour: if actions are failing (high prediction error, low outcome quality,
repeated gate rejections), Policy trips the circuit breaker and halts or escalates.

Policy is also the safety layer: if a `SafetyGate` missed something, Policy is the last line
of defence before a dangerous output reaches the user.

---

## Three Functions

### 1. Circuit Breaking

Monitors a rolling window of loop outcomes. If the failure rate exceeds a threshold (e.g.,
60% failures in the last 10 ticks), the circuit opens: the loop pauses and waits for manual
reset or automatic recovery after a cooldown.

### 2. Escalation

Certain events (safety concerns, high-confidence prediction errors, repeated failures on
a specific action kind) trigger escalation: the Policy returns `PolicyDecision::Escalate`,
which routes the current state to a human operator or a higher-tier agent.

### 3. Safety Override

If a response is flagged as potentially harmful by the outcome observer, Policy returns
`PolicyDecision::SafetyOverride`, which blocks the response and triggers an alternative
safe response.

---

## Where Policy Fits

```
ACT → OBSERVE → POLICY ← policy.evaluate(outcome, score)
                    │
      Continue?  ──→ STORE → LEARN
      Escalate?  ──→ human/escalation queue
      Break?     ──→ halt loop
      Safety?    ──→ replace response
```

Policy runs at step 6 of 7 (after OBSERVE, before STORE and LEARN). It is the guardian of
the loop's post-action phase.

---

## Today vs. Planned

Today, `Policy` does both reactive control (circuit breaking, escalation) and learning
signal routing (forwarding prediction errors to the Delta-speed loop). In the target state,
the learning logic is split into a separate `Calibrator` trait. See
[Policy vs. Calibrator](./09-policy-vs-calibrator.md).

---

## See Also

- [Semantics](./02-semantics.md)
- [Policy vs. Calibrator](./09-policy-vs-calibrator.md)
- [Gate Overview](../02-gate/00-overview.md) — the pre-action filter; Policy is post-action
