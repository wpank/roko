# Policy — Rationale

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Policy vs. Calibrator](./09-policy-vs-calibrator.md)
**Last reviewed**: 2026-04-19

---

## Why does the loop need a post-action control operator at all?

Without Policy, a failing agent keeps failing. If the LLM endpoint is down, the loop
fires 60 requests per minute, each returning an error, each consuming quota, each
generating noise logs. If a model produces harmful content, there is no systematic
interception — individual callsites must each handle it ad hoc.

Policy centralises these concerns: every tick's outcome passes through one choke point
where control decisions are made consistently.

---

## Why `&mut self` — why is Policy different from all other operators?

Gate, Scorer, Router, and Composer all take `&self`. Policy takes `&mut self`.

The reason is the circuit breaker rolling window. A circuit breaker is inherently
stateful: it must remember the last N outcomes to compute failure rate. That memory lives
inside the `Policy` implementation, not in an external store.

The alternative — storing the window in the Substrate — was considered and rejected:

| Approach | Tradeoff |
|---|---|
| Window in `Policy` (`&mut self`) | Simple, zero I/O, correct serialisation order within the loop |
| Window in Substrate (`&self` + async reads) | Forces async in a synchronous decision, adds Substrate dependency to a pure control operator |
| Window in `Arc<Mutex<_>>` (`&self` + interior mutability) | Adds lock overhead; no benefit because Policy is called from a single task |

`&mut self` is the honest, idiomatic choice for a stateful single-owner operator.

<!-- ADDED: Design alternatives analysis inferred from architecture principles -->

---

## Why four `PolicyDecision` variants?

Each variant maps to a distinct loop action with distinct severity:

| Variant | Severity | Why it exists |
|---|---|---|
| `Continue` | — | Default; loops must not stop without reason. |
| `CircuitBreak` | Medium | Transient failures need a pause, not human attention. |
| `Escalate` | High | Persistent failures need human or supervisor attention. |
| `SafetyOverride` | Critical | Harmful content must be blocked before it leaves the system. |

An earlier design used a single `PolicyDecision::Break(BreakKind)` enum, but callers then
had to match on `BreakKind` for every case. Flat variants are more ergonomic.

<!-- ADDED: Earlier enum design alternatives inferred from idiomatic Rust API patterns -->

---

## Why fail-open on `PolicyError`?

Policy protects the loop from failure cascades. If Policy itself fails, the loop must not
lock. Consider the sequence:

1. Policy implementation has a bug; every call returns `Err`.
2. If we fail-closed: the loop stops forever. The agent is dead. An operator must deploy a
   fix before the agent resumes.
3. If we fail-open: the loop continues without circuit-break protection. The agent may
   fire more failing requests, but it remains alive and observable.

Outcome 3 is strictly better: it preserves agent liveness and gives the engineering team
time to fix the bug without a production outage.

Gate uses fail-closed because a failed gate check is a safety uncertainty, not an
operational uncertainty. Policy is not a safety gate; it is a control governor.

---

## Why is Calibrator deferred?

Policy currently handles both control and learning because the two concerns were not
separated in the initial design. The split is conceptually clean but requires:

1. A new `Calibrator` trait and its implementations.
2. Changes to the loop config struct.
3. Migration of all existing agent configs.
4. Validation that `DefaultCalibrator` produces identical learning signals to the current
   inlined code.

The split is deferred until the Dreams offline loop is more mature. There is no point in
precisely routing learning signals to a consumer that does not yet use them.

See [Policy vs. Calibrator](./09-policy-vs-calibrator.md) for the planned design.

---

## Why not make Policy async?

`evaluate` is synchronous (`&mut self` → `Result`, no `async`). This was a deliberate
choice:

- Policy reads only data it already holds in memory (rolling window, config).
- No I/O is required for the decision itself.
- The loop is already in an async context; a sync call inside `loop_tick` is correct and
  avoids spurious `.await` points.

The only I/O that flows from a Policy decision (Bus publish for Escalate, cooldown sleep
for CircuitBreak) is performed by the loop runtime after Policy returns. Policy makes the
decision; the loop executes it.

---

## See Also

- [Trait Surface](./01-trait-surface.md)
- [Policy vs. Calibrator](./09-policy-vs-calibrator.md)
- [Gate Rationale](../02-gate/10-rationale.md) — fail-closed vs. fail-open contrast
- [Composer Rationale](../04-composer/10-rationale.md)
