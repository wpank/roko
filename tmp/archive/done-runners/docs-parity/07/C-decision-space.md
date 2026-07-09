# C — Decision Space

Refresh of the circuit-breaker and intervention parity brief.

Generated: 2026-04-18

---

## Bottom Line

The conductor decision space is already shipped and intentionally small:

- `Severity` is `Info | Warning | Critical`
- `ConductorDecision` is `Continue | Restart | Fail`
- `WorstSeverityPolicy` is the default mapping layer
- the plan-level circuit breaker is live with `MAX_PLAN_FAILURES = 2`

The parity refresh should document that clearly and avoid turning this file
into a speculative breaker redesign.

---

## Shipped Decision Contract

### Severity and decision mapping

- `Severity` lives at `crates/roko-conductor/src/interventions.rs:22-44`.
- `WorstSeverityPolicy` lives at
  `crates/roko-conductor/src/interventions.rs:107-121`.
- `Conductor::evaluate()` consumes watcher output and returns a
  `ConductorDecision` at `crates/roko-conductor/src/conductor.rs:156-186`.

This is the actual contract readers should use when reasoning about the live
conductor.

### Plan-level breaker

- `MAX_PLAN_FAILURES = 2` is defined at
  `crates/roko-conductor/src/circuit_breaker.rs:11`.
- `CircuitBreaker` is the live plan-level breaker at
  `crates/roko-conductor/src/circuit_breaker.rs:28-44`.
- The orchestrator refuses dispatch for tripped plans at
  `crates/roko-cli/src/orchestrate.rs:4718-4727`.

### Orchestrator integration

- The orchestrator runs conductor checks at
  `crates/roko-cli/src/orchestrate.rs:4729-4775`.
- Non-continue outcomes publish diagnosis/alert summaries from the same
  runtime path.

---

## Important Clarifications

### `Restart` and `Fail` are not the same thing

Only `Fail` records breaker failures in the conductor:

- `crates/roko-conductor/src/conductor.rs:175-184`

Parity docs should not say that every restart increments the breaker.

### Use the real emitted kinds

The decision surface is observable, but with the actual live names:

- `conductor:alert:<watcher>`
- `conductor.decision`
- `conductor.circuit_breaker`

That is enough for parity. A cleaner unified signal taxonomy is later work.

### Keep breaker persistence honest

`ExecutorSnapshot` currently stores:

- `plan_states`
- `queue_order`
- `speculative_executions`
- `timestamp_ms`

See `crates/roko-orchestrator/src/executor/snapshot.rs:27-44`.

So this parity refresh should **not** restate any claim that breaker state is
already persisted across restart. That is a real follow-up, but outside this
docs-only pass.

---

## Defer

Do not expand this file into:

- a new breaker type hierarchy,
- cooldown or debounce redesign,
- event-bus rewiring,
- or a learned-policy replacement.

The current parity task is smaller: document the live decision contract
correctly, keep the known caveats explicit, and stop there.
