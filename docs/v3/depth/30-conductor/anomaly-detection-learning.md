# Anomaly Detection and Learning Integration

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 12.
> Source: `crates/roko-learn/src/anomaly.rs`,
>         `crates/roko-learn/src/efficiency.rs`,
>         `crates/roko-learn/src/cascade_router.rs`

---

## 1. AnomalyDetector

The anomaly detector (`roko-learn/src/anomaly.rs`) provides statistical anomaly
detection complementing the Conductor's threshold-based watchers:

```rust
pub struct AnomalyDetector {
    prompt_hash_window: VecDeque<u64>,  // last 20 prompt hashes
    cost_ewma: EwmaState,               // exponentially weighted moving average
    quality_history: VecDeque<f64>,     // last 50 quality scores
    session_cost_usd: f64,
    session_start_ms: i64,
}

pub enum Anomaly {
    PromptLoop { repeated_count: usize },
    CostSpike { z_score: f64 },
    QualityDegradation { avg_drop: f64 },
    BudgetExhausted { used: f64, limit: f64 },
}
```

### 1.1 Prompt Loop Detection

Hashes each prompt and tracks in a sliding window of 20. Five identical hashes
trigger `PromptLoop`. This catches a broader class of loops than the stuck-pattern
watcher. The watcher looks at agent output; the anomaly detector looks at agent
input.

### 1.2 Cost Spike Detection (EWMA + z-score)

```rust
impl EwmaState {
    pub fn update(&mut self, value: f64) {
        let diff = value - self.mean;
        self.mean += self.alpha * diff;
        self.variance = (1.0 - self.alpha)
            * (self.variance + self.alpha * diff * diff);
    }

    pub fn z_score(&self, value: f64) -> f64 {
        let stddev = self.variance.sqrt();
        if stddev < 1e-10 { return 0.0; }
        (value - self.mean) / stddev
    }
}
```

A z-score above 3.0 triggers `CostSpike` -- the cost of the current turn is more
than 3 standard deviations above the running average.

### 1.3 Quality Degradation Detection

Compares recent quality scores (last 5) against earlier scores (turns 11-20). If
the recent average drops more than 0.15 AND the recent average is below 0.5, the
system is degrading. The dual condition prevents false positives on minor dips from
a high baseline.

---

## 2. Online Isolation Forest

The original Isolation Forest (Liu et al., 2008) builds random binary trees. Points
that isolate quickly (short average path length) are anomalies. The online variant
adapts to streaming data via a sliding window.

Anomaly score:

```
s = 2^(-E(depth) / c(window_size))
```

where `E(depth)` is the expected depth across all trees and `c(n)` is the average
path length normalization factor.

For agent monitoring, each turn becomes a multivariate point: latency (ms), tokens
consumed, tool calls made, error rate. A turn scoring above threshold is flagged as
anomalous -- even if no individual watcher fires.

Default parameters: 50 trees, 1000 window, 8 max leaf samples, 0.7 score threshold.

---

## 3. CUSUM for Change Detection

EWMA z-scores detect spikes. CUSUM (Cumulative Sum, Page 1954) detects sustained
shifts. A metric drifting upward by 0.5 sigma per turn will not trigger a z-score
alarm, but CUSUM catches it quickly.

```
C_t = max(0, C_{t-1} + (x_t - mu_0 - k))
```

where `mu_0` is baseline mean, `k` is allowance (typically delta/2). When `C_t`
exceeds decision threshold `h`, CUSUM raises an alarm.

```rust
pub struct CusumDetector {
    mu_0: f64,    // baseline mean
    k: f64,       // allowance parameter
    h: f64,       // decision threshold
    upper: f64,   // upper CUSUM statistic
    lower: f64,   // lower CUSUM statistic
}
```

**Operating characteristics**: With k = delta/2 and h = 4-5 sigma, ARL_0 (false
alarm) is ~500 samples. ARL_1 (1-sigma shift detection) is ~26 samples. CUSUM
detects sustained degradation 10-50x faster than EWMA z-scores.

**Application**: Attach a CUSUM detector to each watcher's numeric output. The
watcher catches spikes; CUSUM catches gradual worsening.

---

## 4. Conductor <-> Learning System Integration

### 4.1 Interventions as Learning Signals

Every conductor intervention produces data:

```
Conductor fires "compile-fail-repeat" for plan-42
    |
    v
AgentEfficiencyEvent {
    outcome: "conductor_intervention",
    gate_errors: [{ category: "TypeMismatch", count: 3 }],
}
    |
    v
Cascade Router records negative observation
    |
    v
Next similar task routed to a more capable model
```

### 4.2 Cascade Router Feedback

| Conductor Event | Router Signal |
|----------------|--------------|
| Continue | Positive (task progressing) |
| Restart (compile-fail) | Negative (model failed) |
| Restart (stuck-pattern) | Negative (model got stuck) |
| Restart (ghost-turn) | Strongly negative (model produced nothing) |
| Fail (iteration-loop) | Strongly negative (model did not converge) |

Over time, the router accumulates enough data to route tasks away from
model-context combinations that historically trigger interventions.

---

## 5. Feedback Loops

### Loop 1: Intervention -> Routing Improvement

```
Agent fails -> Conductor intervenes -> Negative routing signal ->
Router adjusts -> Future agents less likely to fail -> Fewer interventions
```

### Loop 2: Threshold -> Efficiency Data -> Threshold Tuning

```
Threshold fires -> Efficiency event records outcome ->
Effectiveness measured -> Threshold adjusted
```

### Loop 3: Error Classification -> Auto-Fix -> Pattern Library

```
Error classified -> Auto-fix attempted -> Success stored ->
Future similar errors auto-fixed faster
```

### Loop 4: Quality Degradation -> Escalation -> Quality Data

```
Quality drops -> Escalate to higher tier -> Better quality ->
Router learns tier requirements for this task type
```

---

## 6. Anomaly Detection in the Dispatch Pipeline

The anomaly detector runs BEFORE each agent turn:

```rust
if let Some(Anomaly::PromptLoop { .. }) =
    anomaly_detector.check_prompt(prompt_hash)
{
    return Err(DispatchError::PromptLoop);
}

if let Some(Anomaly::CostSpike { z_score }) =
    anomaly_detector.check_cost(turn_cost_usd)
{
    tracing::warn!("cost spike z={z_score:.1}");
}

if let Some(Anomaly::BudgetExhausted { used, limit }) =
    anomaly_detector.check_budget(budget_limit_usd)
{
    return Err(DispatchError::BudgetExhausted { used, limit });
}
```

Catching problems at input time rather than output time.

---

## 7. Provider Health Integration

A separate feedback loop for infrastructure-level anomalies:

| Breaker | Level | Trigger | Blocks |
|---------|-------|---------|--------|
| Provider health | API call | 3 consecutive errors | Requests to provider |
| Conductor | Plan | 2 plan failures | Retries of plan |

---

## 8. File Reference

| File | What |
|------|------|
| `crates/roko-learn/src/anomaly.rs` | AnomalyDetector, EWMA, prompt loop, cost spike |
| `crates/roko-learn/src/efficiency.rs` | AgentEfficiencyEvent (shared data format) |
| `crates/roko-learn/src/cascade_router.rs` | Cascade router (consumes signals) |
| `crates/roko-learn/src/provider_health.rs` | Provider health tracker |
| `crates/roko-gate/src/adaptive_threshold.rs` | Adaptive gate thresholds |
