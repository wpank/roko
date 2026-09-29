# Good Regulator and the Self-Model

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 9.
> Source: `crates/roko-conductor/src/conductor.rs`,
>         `crates/roko-conductor/src/stuck_detection.rs`,
>         `crates/roko-conductor/src/diagnosis.rs`

---

## 1. The Theorem

The Good Regulator Theorem (Conant & Ashby, 1970) states that any system that
successfully regulates another system must contain a model of that system. This is
not a design recommendation -- it is a mathematical proof. A regulator that does not
model the system it controls cannot be an optimal regulator.

> "Every good regulator of a system must be a model of that system."
> -- Conant, R.C. & Ashby, W.R. (1970). *International Journal of Systems Science*,
>    1(2), 89-97.

For the Conductor: to regulate agent execution, the Conductor must model what
healthy agent execution looks like. Every threshold, every heuristic, every error
pattern is a component of this model.

---

## 2. Components of the Self-Model

### 2.1 Behavioral Norms (Watcher Thresholds)

Each watcher threshold encodes an expectation about normal behavior:

| Threshold | Expectation |
|-----------|------------|
| `MAX_GHOST_TURNS = 3` | Healthy agent produces meaningful output every turn |
| `MAX_COMPILE_FAIL_REPEAT = 3` | Healthy agent does not repeat same compile error |
| `MAX_ITERATION_LOOP = 3` | Healthy plan converges within 3 gate-fail cycles |
| `MAX_REVIEW_CYCLES = 3` | Healthy plan passes review within 3 cycles |
| `MAX_SPEC_DRIFT_RATIO = 0.25` | Healthy agent modifies at most 25% unexpected files |
| `MAX_STUCK_REPEATS = 4` | Healthy agent does not repeat identical actions |
| `MIN_FAILURE_INCREASE = 1` | Healthy agent does not increase test failures |
| `ALERT_THRESHOLD = 0.80` | Healthy task completes within 80% of timeout |
| `MAX_CONTEXT_USAGE_RATIO = 0.80` | Healthy agent uses at most 80% of context |
| `MAX_PLAN_FAILURES = 2` | Recoverable plan succeeds within 2 attempts |

These define the "normal region" of execution space. When execution leaves this
region, the Conductor intervenes.

### 2.2 Failure Taxonomy (Error Categories)

The 20 error categories in the diagnosis engine model the system's failure modes.
Each category represents the system's understanding of a distinct way things can go
wrong. The intervention mapping represents the understanding of how to recover.

### 2.3 Process Patterns (Stuck Heuristics)

Six stuck kinds model pathological execution patterns:

- OutputLoop -- doing the same thing repeatedly
- NoProgress -- doing things that produce no results
- GateLoop -- oscillating between two broken states
- CompileLoop -- toggling between incompatible fixes
- EmptyOutput -- producing text without action
- ExcessiveRetries -- retrying without changing approach

Each pattern is a mode of execution that LOOKS like progress but IS NOT progress.

### 2.4 Infrastructure Expectations (Health Checks)

The health monitor models infrastructure requirements: agents should be running,
agents should be responsive, specifications should be current, quality should be
maintained.

---

## 3. Model Accuracy

The self-model's accuracy determines the Conductor's effectiveness.

### 3.1 False Positives (Model Too Strict)

The model considers healthy behavior to be pathological:
- `MAX_GHOST_TURNS = 1` would kill agents taking one turn to read context
- `MAX_SPEC_DRIFT_RATIO = 0.05` would flag agents updating a mod.rs file

False positives waste resources -- healthy agents killed unnecessarily.

### 3.2 False Negatives (Model Too Lenient)

The model considers pathological behavior to be healthy:
- `MAX_GHOST_TURNS = 10` would let a stuck agent burn tokens for 10 turns
- `MAX_ITERATION_LOOP = 10` would let non-converging plans retry 10 times

False negatives waste resources -- pathological agents run unchecked.

### 3.3 The Tuning Challenge

The current thresholds are derived from production experience during batch runs in
March-April 2026. As model versions, codebase complexity, and task types change, the
model drifts.

---

## 4. Static vs. Adaptive Models

### 4.1 Current: Static Model

All thresholds are compile-time constants or constructor parameters. The model does
not update based on observed behavior.

**Advantage**: Predictable, easy to reason about, no drift.
**Disadvantage**: Cannot adapt to changing conditions.

### 4.2 Adaptive Model Infrastructure

The learning system provides the infrastructure:

- **Adaptive gate thresholds** (`roko-gate/src/adaptive_threshold.rs`): EMA-based
  threshold adjustment per gate rung. Already wired.
- **Efficiency events**: Per-turn metrics. Already collected.
- **Cascade router observations**: Model-task outcomes. Already recorded.

An adaptive Conductor model would record which threshold triggered each
intervention, track whether the intervention improved the outcome, and adjust
thresholds toward values that maximize intervention effectiveness.

---

## 5. Self-Model Accuracy Metrics

```rust
pub struct SelfModelAccuracy {
    /// Fraction of interventions that improve outcomes.
    pub intervention_effectiveness: f64,
    /// Fraction of stuck detections that were genuine.
    pub stuck_detection_precision: f64,
    /// Fraction of diagnoses with correct category.
    pub diagnosis_accuracy: f64,
    /// RMSE between predicted and actual completion duration.
    pub completion_time_rmse_ms: f64,
    /// Brier score: mean((predicted_pass_prob - actual_pass)^2).
    pub gate_pass_brier_score: f64,
    /// Harmonic mean of component accuracies.
    pub composite_accuracy: f64,
}
```

### 5.1 Brier Score for Calibration

The Brier score measures whether the model's confidence matches reality. When the
model says "80% chance of gate pass," do 80% of attempts actually pass?

`BS = (1/N) * sum((p_i - o_i)^2)` where p_i is predicted probability and o_i is
outcome (0 or 1). Perfect calibration: BS = 0. Random guessing: BS = 0.25.

### 5.2 Bayesian Threshold Adaptation

Each watcher threshold is a belief tracked as a Beta distribution:

```rust
pub struct ThresholdPosterior {
    pub threshold: f64,
    pub alpha: f64,   // successful interventions
    pub beta: f64,    // unsuccessful interventions
    pub discount: f64, // for non-stationarity (default: 0.995)
    pub min_samples: f64,
}
```

- Intervention fires and restarted agent succeeds: alpha += 1
- Intervention fires and restarted agent fails: beta += 1
- Threshold well-calibrated when alpha / (alpha + beta) approximates target
  precision (e.g., 0.8)

The discount factor causes old observations to decay, so the posterior tracks
non-stationary behavior.

### 5.3 Kalman Filter for State Estimation

A scalar Kalman filter provides online estimation of slowly-drifting system
parameters (baseline error rate, typical cost):

```rust
pub struct ScalarKalman {
    pub estimate: f64,
    pub uncertainty: f64,
    pub process_noise: f64,
    pub measurement_noise: f64,
}

impl ScalarKalman {
    pub fn update(&mut self, observation: f64) {
        self.uncertainty += self.process_noise;
        let gain = self.uncertainty / (self.uncertainty + self.measurement_noise);
        self.estimate += gain * (observation - self.estimate);
        self.uncertainty *= 1.0 - gain;
    }
}
```

When uncertainty is high relative to measurement noise, the filter trusts new
observations more. When uncertainty is low, it trusts the existing estimate.

---

## 6. Precision-Weighted Prediction Errors

The framework connects to active inference theory:

**Prediction**: The model predicts what healthy execution looks like.
**Prediction error**: The difference between predicted and observed behavior.
**Precision weighting**: Errors from reliable sources (many historical episodes)
drive large model updates. Errors from noisy sources (novel tasks) drive small
updates.

Precision is derived from the cascade router's observation count per context:

```rust
pub struct PrecisionWeightedUpdater {
    context_precision: HashMap<String, f64>,
    min_precision: f64,  // default: 0.1
    max_precision: f64,  // default: 10.0
}
```

The min/max bounds prevent two failure modes: novel contexts producing zero-weight
updates, and familiar contexts over-dominating.

---

## 7. The Internal Model Principle

The Internal Model Principle (Francis & Wonham, 1976) strengthens Conant-Ashby:
the controller must contain a copy of the dynamics generating the signals it must
track. The conductor's model of gate outcomes must mirror the actual gate pipeline's
logic. If a new gate is added and the conductor's model does not update, regulation
degrades.

This is testable: when the gate pipeline changes and the model does not update, the
Brier score on gate pass prediction should worsen.

---

## 8. Recursive Self-Modeling

```
Level 0: Agent executes task
Level 1: Watchers model agent execution
Level 2: MetaCognitionHook models watcher effectiveness
```

Level 2 asks: "Am I stuck?" -- a second-order question about the effectiveness of
first-order monitoring. In practice two levels suffice; the law of diminishing
returns applies.

---

## 9. The Model Gap

The self-model is always incomplete. The six stuck kinds do not cover all possible
stuck modes. The 20 error categories do not cover all possible errors. The practical
response:

1. **Default handling**: Unknown errors fall to generic categories
2. **Error logging**: Every unmatched error is logged for pattern discovery
3. **Model expansion**: New patterns added as encountered
4. **Learning integration**: Clustering of unclassified errors reveals new
   categories

---

## 10. References

- Conant, R.C. & Ashby, W.R. (1970). "Every good regulator of a system must be a
  model of that system." *International Journal of Systems Science*, 1(2), 89-97.
- Francis, B.A. & Wonham, W.M. (1976). "The Internal Model Principle of Control
  Theory." *Automatica*, 12(5), 457-465.
- Ashby, W.R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. Law of
  Requisite Variety.
- Friston, K. (2010). "The free-energy principle: a unified brain theory?" *Nature
  Reviews Neuroscience*, 11(2), 127-138.
- Kalman, R.E. (1960). "A New Approach to Linear Filtering and Prediction
  Problems." *Journal of Basic Engineering*, 82(1), 35-45.

---

## 11. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/conductor.rs` | Self-model instantiation (Conductor::new()) |
| `crates/roko-conductor/src/stuck_detection.rs` | Process pattern model (6 heuristics) |
| `crates/roko-conductor/src/diagnosis.rs` | Failure taxonomy (20 categories, 34 patterns) |
| `crates/roko-conductor/src/health.rs` | Infrastructure expectation model (4 checks) |
| `crates/roko-learn/src/efficiency.rs` | Data source for model calibration |
| `crates/roko-gate/src/adaptive_threshold.rs` | Adaptive model precedent |
