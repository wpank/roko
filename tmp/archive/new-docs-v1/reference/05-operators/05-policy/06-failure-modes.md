# Policy — Failure Modes

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Invariants](./05-invariants.md)
**Last reviewed**: 2026-04-19

---

## FM-1 — Policy returns `Err(PolicyError)`

**Cause**: Internal computation error — corrupt state, arithmetic overflow in the failure
rate calculation, or a custom implementation panicking and returning an error.

**Effect**: The loop logs the error with `tracing::error!` and substitutes
`PolicyDecision::Continue`. The tick completes normally.

**Risk**: If the error is persistent (broken implementation), the circuit breaker silently
stops working. Failures accumulate without triggering `CircuitBreak`.

**Mitigation**:
- Monitor the `policy.error` metric (incremented each time this fallback fires).
- Alert if `policy.error` fires more than N times per minute.
- Use `PassPolicy` in tests; use `CircuitBreakerPolicy` in production.

<!-- ADDED: Fail-open fallback and monitoring guidance inferred from architecture docs -->

---

## FM-2 — Rolling window is too small

**Cause**: `window_size` set to 1 or 2.

**Effect**: A single failure immediately triggers `CircuitBreak` (100% failure rate >
60% threshold). The loop oscillates: break → cooldown → half-open → one tick → break.

**Mitigation**: Keep `window_size ≥ 5`. Default is 10.

---

## FM-3 — `cooldown_secs` is zero or very small

**Cause**: `cooldown_secs = 0` or `cooldown_secs = 1`.

**Effect**: The circuit opens and immediately re-enters half-open, then re-opens on the
first failure. The loop thrashes: opens and closes on every failure, providing no actual
protection.

**Mitigation**: Keep `cooldown_secs ≥ 10`. Default is 30.

<!-- ADDED: Thrashing failure mode inferred from circuit breaker patterns -->

---

## FM-4 — `failure_threshold` too high

**Cause**: `failure_threshold = 1.0` (100% failure required).

**Effect**: Circuit never opens. The loop keeps sending failing requests indefinitely.

**Mitigation**: Default 0.6 is appropriate for most workloads. Do not raise above 0.8.

---

## FM-5 — Escalation packet fails to publish

**Cause**: The Bus is unavailable when `Escalate` is returned.

**Effect**: The escalation is lost. The loop may still pause (depending on loop
implementation), but the human operator / supervisor agent never receives the packet.

**Mitigation**:
- The Bus publish call must be retried with exponential backoff.
- If the Bus is persistently unavailable, fall back to writing the `EscalationPacket` to
  local disk (configurable path).
- Log `escalation.publish_failed` metric.

<!-- ADDED: Bus unavailability path inferred from distributed system failure patterns -->

---

## FM-6 — State loss on restart

**Cause**: The agent process restarts (crash or deploy). `CircuitBreakerPolicy` holds
its rolling window and circuit state in memory only.

**Effect**: On restart, the circuit starts fresh in `Closed` state with an empty window.
Previously detected failure trends are lost. If the underlying issue persists, the loop
will fail-fast again after another `window_size` ticks, but there is a gap.

**Mitigation**:
- Persist circuit state to the Substrate on each tick (optional, adds latency).
- Accept the gap: the circuit re-opens after `window_size` failures, which is bounded.

---

## FM-7 — Safety classifier false positive

**Cause**: `SafetyPolicy` classifier flags a benign response as a violation.

**Effect**: `SafetyOverride` is returned. The response is blocked. The user receives the
safe-substitution message.

**Mitigation**:
- Log all blocked responses to an audit table.
- Run offline review of audit logs to tune classifier thresholds.
- Provide a human-in-the-loop review path for false-positive reports.

<!-- ADDED: False positive handling inferred from safety system design principles -->

---

## Summary table

| FM | Symptom | Default mitigation |
|---|---|---|
| FM-1 | Policy silently stops working | `policy.error` metric, alert |
| FM-2 | Loop oscillates | `window_size ≥ 5` |
| FM-3 | Loop thrashes | `cooldown_secs ≥ 10` |
| FM-4 | Circuit never opens | `failure_threshold ≤ 0.8` |
| FM-5 | Escalation lost | Bus retry + disk fallback |
| FM-6 | State lost on restart | Accept gap or persist state |
| FM-7 | Safe response shown for benign output | Audit log + offline tuning |

---

## See Also

- [Invariants](./05-invariants.md)
- [Performance](./07-performance.md)
- [Failure Modes — Bus](../../04-bus/11-failure-modes.md)
