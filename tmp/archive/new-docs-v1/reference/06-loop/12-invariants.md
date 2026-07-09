# Loop Invariants

> What must be true after every tick, regardless of outcome.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [loop\_tick()](09-loop-tick-code.md),
[PERSIST](07-stage-persist.md), [VERIFY](06-stage-verify.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The loop maintains a small set of invariants that hold after every tick. These
invariants are what make the loop tractable to reason about, test, and debug.
Violating any of them is a bug in `loop_tick()` or in a stage implementation.

---

## The Invariants

### INV-1: Every tick produces at least one Engram write

**Formal statement**: `result.provenance_id.is_valid()` is always `true`.

No tick may complete without writing a Provenance Engram to the Substrate. Even if
every stage fails, the Provenance Engram records that the tick was attempted, what
the stimulus was, and where it failed.

**Where enforced**: PERSIST stage. If `substrate.put(provenance_engram)` fails, the
tick returns `TickResult::persist_failed` — a critical error.

**Rationale**: Without this invariant, a sequence of failures becomes a black hole in
the audit trail. Debugging "why did the agent stop responding?" is impossible if tick
attempts are not recorded.

---

### INV-2: VERIFY always runs before PERSIST (for non-null output)

**Formal statement**: if `act_output.is_ok()`, then `verify_result` was computed from
that output before `persist` was called.

The Outcome Engram's `verified` field is set by VERIFY. An Outcome Engram with
`verified = true` that was not actually verified is a data corruption.

**Where enforced**: the `loop_tick()` function structure. VERIFY is called before
`persist_stage.persist()`. This ordering is structural, not conditional.

**Rationale**: If PERSIST could run before VERIFY, it would be possible for unverified
or policy-violating output to enter long-term memory. Future ticks that retrieve this
output would make decisions based on bad data.

---

### INV-3: A HardFail in VERIFY produces no Outcome Engram

**Formal statement**: if `verify_result.verdict == HardFail`, then
`result.outcome_id.is_none()`.

Output that fails a hard gate must not enter the Substrate, even partially.

**Where enforced**: PERSIST stage. If `VerifyResult.verdict` is HardFail, PERSIST
skips writing the Outcome Engram and writes only the Provenance + Failure Engrams.

**Rationale**: Partial persistence of failed output is dangerous. A future QUERY might
surface it, SCORE might rank it highly (it has valid HDC fingerprint), and COMPOSE
might include it — propagating the bad output through subsequent ticks.

---

### INV-4: ROUTE runs only if SCORE succeeded

**Formal statement**: if `scored.is_empty()` and the stimulus has no `route_hint`,
ROUTE must return `RouteTarget::Defer` or an error. ROUTE may not invent a target
from nothing.

**Where enforced**: the `CascadeRouter` — if neither static hint nor prior history
exists, it falls through to LinUCB with an empty feature vector, which returns low
confidence → deferral.

**Rationale**: Routing without evidence is guessing. A confident route to the wrong
target is worse than an honest deferral.

---

### INV-5: Every tick publishes exactly one predict.error Pulse

**Formal statement**: `react_result.pulses_published` contains exactly one Pulse with
`kind = PulseKind::PredictionError`.

**Where enforced**: REACT stage. The `predict.error` Pulse is published unconditionally,
even when prior stages failed (the error is large in that case).

**Rationale**: The active inference learning signal must be continuous. Missing ticks
in the free-energy signal create blind spots in adaptation.

---

### INV-6: Stage wall times are measured and recorded

**Formal statement**: `ctx.metrics.stage_time(stage)` is set for every stage that
ran, after that stage completes.

**Where enforced**: `loop_tick()` — each stage call is wrapped in a
`ctx.metrics.time_stage(name, || ...)` closure.

**Rationale**: Performance invariants (see [Performance](14-performance.md)) can only
be monitored if timing data is collected. The metrics record is also included in the
Provenance Engram for post-hoc analysis.

---

### INV-7: Budget consumption is monotonically non-decreasing

**Formal statement**: after each stage, `budget.consumed ≥ budget.consumed_before_stage`.

No stage may "refund" budget. Budget accounting is strictly additive.

**Where enforced**: `TickBudget` implementation. Each stage reports its cost;
`TickBudget::charge(cost)` adds to the running total and panics on underflow.

**Rationale**: Budget refunds would allow pathological stage implementations to appear
"cheaper" than they are, defeating the purpose of budget enforcement.

---

## Checking Invariants in Tests

The `TickInvariantChecker` struct in `roko-agent/tests/invariants.rs` checks all
seven invariants against any `TickResult`:

```rust
// source: crates/roko-agent/tests/invariants.rs
pub struct TickInvariantChecker;

impl TickInvariantChecker {
    pub fn check(result: &TickResult, ctx: &TickContext) -> Vec<InvariantViolation> {
        let mut violations = vec![];
        Self::check_inv1(result, &mut violations);
        Self::check_inv2(result, ctx, &mut violations);
        Self::check_inv3(result, &mut violations);
        // … etc.
        violations
    }
}

#[test]
fn all_invariants_hold_on_clean_tick() {
    let ctx = test_context();
    let result = tokio_test::block_on(loop_tick(&ctx));
    let violations = TickInvariantChecker::check(&result, &ctx);
    assert!(violations.is_empty(), "invariant violations: {:?}", violations);
}
```

Property-based tests (using `proptest`) run the checker against thousands of randomly
generated `TickContext` configurations to verify invariant coverage.

---

## See also

- [loop\_tick() reference](09-loop-tick-code.md) — the implementation that enforces these invariants
- [VERIFY](06-stage-verify.md) — INV-2 and INV-3
- [PERSIST](07-stage-persist.md) — INV-1 and INV-3
- [Active Inference](11-active-inference.md) — INV-5
- [Failure Modes](13-failure-modes.md) — what happens when invariants are at risk
