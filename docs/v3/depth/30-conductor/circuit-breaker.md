# Circuit Breaker -- Three-State Model

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 3.
> Source: `crates/roko-conductor/src/circuit_breaker.rs`,
>         `crates/roko-learn/src/provider_health.rs`
>
> **Status (2026-09-29, at `7c556bc0a`):** the plan circuit breaker is built but not
> enforced: nothing on the Graph path evaluates the Conductor (see
> [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 3). The provider-health breaker in
> `provider_health.rs` is a separate mechanism, and plan-run routing does use it.

---

## 1. The Problem It Solves

Without a circuit breaker, a fundamentally broken plan enters an infinite retry
loop:

```
Plan fails -> orchestrator retries -> plan fails the same way ->
orchestrator retries -> plan fails again -> orchestrator retries -> ...
```

Each retry costs tokens. Each retry burns wall-clock time that could be spent on
plans that might succeed. Each retry produces the same failure output, adding noise
to the signal stream without adding information.

This was Issue #7 from production (circuit breaker for repeated failures). The
circuit breaker enforces a hard budget: two failures per plan. After that, the plan
is marked as requiring human intervention and is never automatically retried.

> "In the absence of a circuit breaker, every service can become a victim."
> -- Nygard (2007), *Release It!*

---

## 2. Implementation

```rust
use dashmap::DashMap;

pub const MAX_PLAN_FAILURES: u32 = 2;

pub struct CircuitBreaker {
    failures: DashMap<String, FailureRecord>,
}

struct FailureRecord {
    count: u32,
    // Additional metadata: timestamps, failure reasons
}
```

### 2.1 Thread Safety

The `DashMap` provides lock-free concurrent reads and sharded writes. Two plans with
different IDs almost always hit different shards, enabling true parallel access.
This is preferable to a `Mutex<HashMap>` which would serialize all failure record
access.

### 2.2 API

```rust
impl CircuitBreaker {
    pub fn new() -> Self { Self { failures: DashMap::new() } }

    /// Record a failure. Returns true if the plan is now tripped.
    pub fn record_failure(&self, plan_id: &str) -> bool {
        let mut entry = self.failures.entry(plan_id.to_string())
            .or_insert(FailureRecord { count: 0 });
        entry.count += 1;
        entry.count >= MAX_PLAN_FAILURES
    }

    /// Check if a plan has exceeded its failure budget.
    pub fn is_tripped(&self, plan_id: &str) -> bool {
        self.failures.get(plan_id)
            .map(|r| r.count >= MAX_PLAN_FAILURES)
            .unwrap_or(false)
    }

    /// Reset failure count (e.g., after manual intervention).
    pub fn reset(&self, plan_id: &str) { self.failures.remove(plan_id); }
}
```

---

## 3. Three-State Model

The circuit breaker implements a classic three-state pattern (Nygard, 2007). The
plan-level conductor uses a simplified two-state model (tripped / not tripped). The
full three-state model, implemented in the provider health tracker
(`roko-learn/src/provider_health.rs`), provides additional granularity.

### 3.1 State Transitions

```
Closed (Healthy)
  |
  | consecutive failures >= threshold
  v
Open (Tripped)
  |
  | cooldown period expires
  v
HalfOpen (Probing)
  |
  +-- probe succeeds -> Closed
  |
  +-- probe fails -> Open (reset cooldown)
```

**Closed**: Normal operation. Failures are counted but requests proceed. Initial
state for every plan.

**Open**: All requests are blocked. The plan has exceeded its failure budget. In the
conductor's simplified model, this is the terminal state. In the provider health
model, the system waits for a cooldown period before transitioning to HalfOpen.

**HalfOpen**: One probe request is permitted. If the probe succeeds, the breaker
returns to Closed. If the probe fails, the breaker returns to Open with a fresh
cooldown. This state exists in the provider health tracker but not in the
conductor's plan-level breaker -- because plans do not benefit from automatic
probing (a plan that failed twice needs a different approach, not another attempt at
the same approach).

### 3.2 Error-Type-Specific Cooldowns

The provider health tracker classifies errors to set cooldown durations:

| Error Class | Cooldown | Rationale |
|------------|----------|-----------|
| RateLimit | 5 seconds | Transient; provider will accept again soon |
| Timeout | 10 seconds | Might indicate temporary load |
| ServerError | 30 seconds | Likely operational issue |
| AuthFailure | 5 minutes | Likely persistent; manual fix needed |
| ContentPolicy | 5 minutes | Likely persistent |
| ContextOverflow | N/A | Not retryable; needs model switch |

---

## 4. Integration with the Conductor

The circuit breaker is checked at the start of every `evaluate()` call:

```rust
impl Conductor {
    pub fn evaluate(&self, plan_id: &str, stream: &[Signal], ctx: &Context)
        -> ConductorDecision
    {
        // 1. Check circuit breaker FIRST
        if self.circuit_breaker.is_tripped(plan_id) {
            return ConductorDecision::Fail {
                reason: format!("plan {} tripped after {} failures",
                    plan_id, MAX_PLAN_FAILURES),
            };
        }

        // 2. Run watchers
        let watcher_outputs = self.check_all(stream, ctx);

        // 3. Apply intervention policy
        let decision = self.policy.evaluate(&watcher_outputs, ctx);

        // 4. Record failures
        if matches!(decision, ConductorDecision::Fail { .. }) {
            self.circuit_breaker.record_failure(plan_id);
        }

        decision
    }
}
```

The check happens before watcher evaluation. If a plan is already tripped, there is
no point running watchers -- the decision is predetermined.

---

## 5. Why Two Failures

`MAX_PLAN_FAILURES = 2` is derived from production data:

**First failure**: Often caused by transient issues -- API rate limit, cold start,
missing context. Retrying with a fresh agent and different context frequently
succeeds.

**Second failure**: The same plan failing twice usually indicates a structural
problem -- the task is beyond the agent's capability, the acceptance criteria are
contradictory, or the codebase has changed in a way that makes the task impossible
as specified.

**Third failure (never reached)**: The probability of success is negligible. The two
previous attempts have already tried the obvious approaches.

The math: if each attempt has a 30% success rate (typical for complex plans that
fail the first time), the probability of failing twice is (0.7)^2 = 49%. But
failures are correlated (same root cause), so the conditional probability of a third
failure given two failures is much higher than 70%. The expected cost of a third
attempt almost always exceeds its expected value.

---

## 6. Hard Guarantee Relationship

The circuit breaker implements two hard guarantees:

**Hard Guarantee 3: Hard Iteration Cap.** Each plan attempt includes up to 3
implementation iterations. With 2 plan-level failures:

```
2 plan attempts x 3 iterations each = 6 total implementation cycles
```

**Hard Guarantee 7: Circuit Breaker.** Direct implementation. The plan can fail a
maximum of 2 times. After 2 failures, it is permanently marked as requiring human
intervention.

```
MAX_PLAN_FAILURES (2) x MAX_ITERATION_LOOP (3) = 6 max attempts ever
```

This prevents infinite retry loops (max 2 failures), token burn on doomed plans (6
attempts max), and silent stuck plans (tripped state is surfaced prominently).

---

## 7. Per-Plan Isolation

The circuit breaker is keyed by plan ID:

- Plan A hitting its failure budget does not affect Plan B
- Resetting Plan A does not reset Plan B
- The breaker can track hundreds of plans concurrently

This per-plan isolation is critical for batch runs where 20+ plans execute in
parallel. A single broken plan should not cascade to affect healthy plans.

---

## 8. Predictive Circuit Breaking

The current breaker is reactive: it counts failures after they happen. The
predictive extension trips the circuit *before* failures cascade, based on leading
indicators.

### 8.1 Holt-Winters Forecasting

`HoltForecaster` extends EWMA with a trend component, enabling forward projection:

```rust
pub struct HoltForecaster {
    level: f64,           // smoothed value
    trend: f64,           // smoothed rate of change
    alpha: f64,           // level smoothing (default: 0.3)
    beta: f64,            // trend smoothing (default: 0.1)
    observations: usize,
}

impl HoltForecaster {
    pub fn forecast(&self, h: usize) -> f64 {
        self.level + self.trend * h as f64
    }
}
```

### 8.2 Proactive Trip Signal

`ProactiveTripSignal` projects the error rate forward and trips preemptively when
the projected rate exceeds 60% and the slope exceeds 5% per cycle. The
`min_observations` guard prevents false trips with fewer than ~10 data points.

---

## 9. Partial Circuit Breaking

A full circuit trip halts all work on a plan. Partial circuit breaking degrades
individual capabilities while keeping core execution running.

### 9.1 Feature-Level Breakers

Each plan capability has its own circuit:

| Feature | Fallback 1 | Fallback 2 |
|---------|-----------|-----------|
| Clippy gate | WarnAndContinue | Skip |
| Context enrichment | UseCached | Skip |
| Review cycle | Downgrade (Haiku reviewer) | Skip |
| Compile gate | (no fallback -- always required) | -- |
| Test gate | (no fallback -- always required) | -- |

Compile and test gates have no fallback because they enforce correctness. Everything
else -- linting, enrichment, review -- is valuable but not essential.

### 9.2 AIMD Adaptive Concurrency

AIMD (Additive Increase, Multiplicative Decrease) self-tunes to the optimal
concurrency:

- On success: `concurrency += 1 / concurrency` (additive increase)
- On failure: `concurrency *= 0.9` (multiplicative decrease)

The additive increase is inversely proportional to the current limit. At
concurrency 2, each success adds 0.5. At concurrency 8, each success adds 0.125.
This produces slow, cautious growth at high concurrency.

---

## 10. Persistence

The circuit breaker state is part of the executor snapshot. When the orchestrator
checkpoints to `.roko/state/`, failure records are included. On resume, the circuit
breaker is restored from the snapshot, preserving failure counts across restarts.
This prevents circumvention where restarting the orchestrator would reset all
breakers.

---

## 11. References

- Nygard, M. (2007). *Release It! Design and Deploy Production-Ready Software*.
  Pragmatic Bookshelf. -- Circuit breaker pattern, three-state model, cascading
  failure prevention.
- Netflix Hystrix -- rolling window metrics, health calculation
- Resilience4j -- sliding window, slow-call detection

---

## 12. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/circuit_breaker.rs` | CircuitBreaker, HoltForecaster, ProactiveTripSignal |
| `crates/roko-conductor/src/conductor.rs` | Integration point -- breaker checked in evaluate() |
| `crates/roko-learn/src/provider_health.rs` | Extended 3-state model for provider health |
| `crates/roko-core/src/agent.rs` | ConductorDecision enum consumed by orchestrator |
