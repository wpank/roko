# Policy — Built-in Implementations

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Trait Surface](./01-trait-surface.md), [Semantics](./02-semantics.md)
**Used by**: [Loop integration](../../02-loop/00-overview.md)
**Last reviewed**: 2026-04-19

---

Three concrete types ship with `roko-core`. All implement `Policy`.

---

## `CircuitBreakerPolicy`

The primary production implementation. Maintains a rolling window of the last N tick
outcomes and opens a circuit breaker when the failure rate exceeds a configurable
threshold.

```rust
// source: crates/roko-core/src/policy.rs

/// Configures the rolling-window circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// How many past ticks to consider. Default: 10.
    pub window_size: usize,
    /// Failure rate [0.0, 1.0] that opens the circuit. Default: 0.6.
    pub failure_threshold: f32,
    /// How many seconds the loop pauses on CircuitBreak. Default: 30.
    pub cooldown_secs: u64,
    /// Prediction error magnitude that triggers Escalate. Default: 0.9.
    pub escalation_threshold: f32,
    /// Consecutive ticks above escalation_threshold before Escalate fires. Default: 3.
    pub escalation_streak: usize,
}

pub struct CircuitBreakerPolicy {
    config: CircuitBreakerConfig,
    /// Ring buffer of recent outcomes. true = success, false = failure.
    window: VecDeque<bool>,
    /// Count of consecutive ticks above escalation_threshold.
    escalation_streak: usize,
    /// Current circuit state.
    state: CircuitState,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    Closed,   // Normal operation.
    Open,     // Paused; waiting for cooldown.
    HalfOpen, // Allowing one test tick.
}

impl Policy for CircuitBreakerPolicy {
    fn evaluate(
        &mut self,
        outcome: &LoopOutcome,
        score: &Score,
    ) -> Result<PolicyDecision, PolicyError> {
        let is_failure = Self::classify_failure(outcome, score, &self.config);

        // Update rolling window.
        if self.window.len() == self.config.window_size {
            self.window.pop_front();
        }
        self.window.push_back(!is_failure);

        // Update escalation streak.
        if score.prediction_error > self.config.escalation_threshold {
            self.escalation_streak += 1;
        } else {
            self.escalation_streak = 0;
        }

        // Safety override takes priority.
        if matches!(outcome, LoopOutcome::SafetyViolation { .. }) {
            return Ok(PolicyDecision::SafetyOverride {
                blocked_response: outcome.raw_response().unwrap_or_default(),
            });
        }

        // Escalation check.
        if self.escalation_streak >= self.config.escalation_streak {
            return Ok(PolicyDecision::Escalate {
                reason: format!(
                    "prediction error above threshold for {} consecutive ticks",
                    self.escalation_streak
                ),
            });
        }

        // Circuit breaker check.
        let failure_rate = self.failure_rate();
        if failure_rate > self.config.failure_threshold {
            self.state = CircuitState::Open;
            return Ok(PolicyDecision::CircuitBreak {
                reason: format!("failure rate {:.0}% exceeds threshold", failure_rate * 100.0),
                cooldown_secs: self.config.cooldown_secs,
            });
        }

        Ok(PolicyDecision::Continue)
    }
}

impl CircuitBreakerPolicy {
    fn failure_rate(&self) -> f32 {
        if self.window.is_empty() { return 0.0; }
        let failures = self.window.iter().filter(|&&ok| !ok).count();
        failures as f32 / self.window.len() as f32
    }

    fn classify_failure(
        outcome: &LoopOutcome,
        score: &Score,
        config: &CircuitBreakerConfig,
    ) -> bool {
        matches!(outcome, LoopOutcome::Rejected | LoopOutcome::LlmError(_))
            || score.prediction_error > config.escalation_threshold
    }
}
```
<!-- source: crates/roko-core/src/policy.rs -->

### Half-open recovery sequence

```
Circuit: Closed ──[failure_rate > threshold]──► Open
                                                  │
                          cooldown_secs elapses   │
                                                  ▼
                                              HalfOpen ──[test tick succeeds]──► Closed
                                                  │
                                          [test tick fails]
                                                  │
                                                  ▼
                                               Open (restart cooldown)
```

<!-- ADDED: State-machine diagram inferred from circuit breaker standard semantics -->

---

## `PassPolicy`

A no-op policy for tests and single-turn agents that have no control requirements.
Always returns `Continue`.

```rust
// source: crates/roko-core/src/policy.rs

pub struct PassPolicy;

impl Policy for PassPolicy {
    fn evaluate(
        &mut self,
        _outcome: &LoopOutcome,
        _score: &Score,
    ) -> Result<PolicyDecision, PolicyError> {
        Ok(PolicyDecision::Continue)
    }
}
```
<!-- source: crates/roko-core/src/policy.rs -->

Use `PassPolicy` in unit tests to isolate the component under test from policy effects.
Never use in production; it will not protect the loop from failure cascades.

---

## `SafetyPolicy`

<!-- ADDED: Described from architecture context; specific implementation details inferred -->

A policy implementation that focuses exclusively on the `SafetyOverride` path. It wraps
a configurable list of safety classifiers and blocks responses that match any of them.
Does not maintain a rolling window, so `&mut self` is a no-op mutation.

```rust
// source: crates/roko-core/src/policy.rs

pub struct SafetyPolicy {
    classifiers: Vec<Box<dyn SafetyClassifier>>,
}

pub trait SafetyClassifier: Send + Sync {
    fn is_violation(&self, response: &str) -> bool;
}

impl Policy for SafetyPolicy {
    fn evaluate(
        &mut self,
        outcome: &LoopOutcome,
        _score: &Score,
    ) -> Result<PolicyDecision, PolicyError> {
        if let Some(raw) = outcome.raw_response() {
            for classifier in &self.classifiers {
                if classifier.is_violation(&raw) {
                    return Ok(PolicyDecision::SafetyOverride {
                        blocked_response: raw,
                    });
                }
            }
        }
        Ok(PolicyDecision::Continue)
    }
}
```
<!-- source: crates/roko-core/src/policy.rs -->

`SafetyPolicy` can be composed with `CircuitBreakerPolicy` using the stacking pattern
described in [Invariants](./05-invariants.md).

---

## Choosing an Implementation

| Need | Use |
|---|---|
| Production agent loop | `CircuitBreakerPolicy` |
| Safety-only constraint (no circuit break) | `SafetyPolicy` |
| Safety + circuit break | Stack both (see Invariants) |
| Unit tests | `PassPolicy` |

---

## Open Questions

- Should `SafetyPolicy` also count safety violations towards the circuit breaker window?
  Currently it does not, because it is a separate implementation.
- Should classifiers run before or after the circuit break check? Current order: safety
  override takes priority (checked first inside `CircuitBreakerPolicy`).
