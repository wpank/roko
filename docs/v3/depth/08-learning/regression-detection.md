# 08-learning/08 -- Regression Detection

> Baseline-relative performance comparison, per-slice analysis, configurable
> thresholds, alert severity levels, and corrective feedback to the routing
> and planning layers.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/regression.rs`

**Cross-references:** [task-metrics-and-baselines](task-metrics-and-baselines.md),
[cascade-router](cascade-router.md)

---

## 1. Purpose

The regression detector answers a critical question: "Did this configuration
change make things worse?" It compares a fresh batch of `TaskMetric` records
against a previously computed `Baseline` and fires alerts when key indicators
breach configurable thresholds. This closes the feedback loop between system
changes (prompt modifications, model routing updates, playbook rule changes)
and their observable impact on task outcomes.

Without regression detection, the learning system could silently degrade: a
bandit might converge on a model that worked well last week but performs poorly
after a provider update, or a playbook rule might be promoted despite
introducing regressions in edge cases.

---

## 2. Threshold Configuration

```rust
pub struct RegressionThresholds {
    pub pass_rate_drop: f64,       // default: 0.15 (15%)
    pub cost_increase: f64,        // default: 0.20 (20%)
    pub duration_increase: f64,    // default: 0.30 (30%)
    pub iterations_increase: f64,  // default: 0.25 (25%)
    pub min_records: usize,        // default: 5
}
```

| Metric | Threshold | Severity | Rationale |
|--------|-----------|----------|-----------|
| Pass rate drop | > 15% | **Alert** | Direct impact on task completion |
| Cost increase | > 20% | **Alert** | Budget impact |
| Duration increase | > 30% | Warning | May be acceptable for higher quality |
| Iterations increase | > 25% | Warning | May reflect harder tasks |

---

## 3. Detection Algorithm

```
Baseline (from historical TaskMetric records)
    |
    v
Current batch (recent N task metrics)
    |
    v
For each (role, complexity_band) slice:
    |
    +-- Pass rate regression:
    |     change = (baseline.pass_rate - current.pass_rate) / baseline.pass_rate
    |     if change > pass_rate_drop -> Alert
    |     if change < -pass_rate_drop -> Improvement
    |
    +-- Cost regression:
    |     change = (current.avg_cost - baseline.avg_cost) / baseline.avg_cost
    |     if change > cost_increase -> Alert
    |
    +-- Duration regression:
    |     change = (current.avg_duration - baseline.avg_duration) / baseline
    |     if change > duration_increase -> Warning
    |
    +-- Iterations regression:
          change = (current.avg_iterations - baseline.avg_iterations) / baseline
          if change > iterations_increase -> Warning
```

### 3.1 Per-Slice Analysis

Regressions are detected per `(role, complexity_band)` slice, not just in
aggregate. This prevents a scenario where a severe regression in
"Implementer/complex" tasks is masked by improvements in "Reviewer/standard"
tasks.

---

## 4. Alert Schema

```rust
pub enum AlertSeverity {
    Alert,        // Key metric breached
    Warning,      // Secondary metric breached
    Improvement,  // Metric improved
}

pub struct RegressionAlert {
    pub metric_name: String,
    pub severity: AlertSeverity,
    pub baseline_value: f64,
    pub current_value: f64,
    pub change_fraction: f64,
    pub threshold: f64,
    pub description: String,
    pub slice: Option<(String, String)>,
}

pub struct RegressionReport {
    pub alerts: Vec<RegressionAlert>,
    pub has_regressions: bool,
    pub sufficient_data: bool,
    pub current_records: usize,
    pub baseline_records: usize,
}
```

---

## 5. LearningRuntime Integration

```rust
pub struct RegressionConfig {
    pub thresholds: RegressionThresholds,
    pub current_window: usize,  // default: 20
}
```

The runtime:

1. Reads all `TaskMetric` records from `.roko/learn/task-metrics.jsonl`.
2. Splits into baseline (all except latest `current_window`) and current.
3. Computes baselines for both sets.
4. Calls `detect_regressions(baseline, current, thresholds)`.
5. If `has_regressions`, logs alerts and optionally triggers corrective
   actions.

### 5.1 Current Window

`current_window = 20` balances responsiveness vs stability:

- Too small (< 10): noisy, a single outlier triggers false alerts.
- Too large (> 50): sluggish, a real regression takes many tasks to surface.
- 20 provides statistical stability while catching regressions within a
  single plan execution.

---

## 6. C-Factor Regression

C-Factor regression detects systemic decline: when the composite score drops
against a trailing history window, it indicates that the system as a whole is
performing worse, even if individual metrics have not breached their
thresholds. This catches subtle multi-dimensional regressions where pass rate
drops slightly, cost rises slightly, and speed decreases slightly -- none
individually alarming, but collectively significant.

---

## 7. Adaptive Gate Thresholds

Gate thresholds (pass/fail criteria) are adjusted via EMA:

```
Gate threshold = EMA(pass_rates, alpha=0.1)
```

When the regression detector fires a pass_rate Alert, it signals that
thresholds may need recalibration.

---

## 8. Practical Example

### 8.1 Baseline (tasks 1-130)

```
Slice (Implementer, standard):
    pass_rate: 0.75, avg_cost: $0.78, avg_iterations: 1.3
```

### 8.2 Current (tasks 151-170, after template change)

```
Slice (Implementer, standard):
    pass_rate: 0.50, avg_cost: $1.05, avg_iterations: 2.0
```

### 8.3 Report

```
ALERT: pass_rate regression in (Implementer, standard)
    Baseline: 0.75, Current: 0.50, Change: -33.3% (threshold: 15%)

ALERT: cost regression in (Implementer, standard)
    Baseline: $0.78, Current: $1.05, Change: +34.6% (threshold: 20%)

WARNING: iterations regression in (Implementer, standard)
    Baseline: 1.3, Current: 2.0, Change: +53.8% (threshold: 25%)
```

---

## 9. False Positive Management

Mitigation strategies:

1. **min_records threshold**: Don't fire with fewer than 5 records per slice.
2. **Per-slice analysis**: Detect whether slice-specific or systemic.
3. **Improvement tracking**: Report improvements alongside regressions.
4. **Config hash correlation**: Flag specific changes as likely causes.
