# Adaptive Thresholds -- EMA, CUSUM, EWMA Control Charts

> Depth file for [07-GATES.md](../../07-GATES.md) section 4.
> Source: `crates/roko-gate/src/adaptive_threshold.rs`, `crates/roko-gate/src/spc.rs`,
> `crates/roko-gate/src/hotelling.rs`, `crates/roko-gate/src/pelt.rs`

---

## 1. Overview

Adaptive thresholds tune verification behavior based on historical pass
rates. They use exponential moving averages (EMA) per gate rung to track
how often each rung passes, and derive two advisory signals:

1. **Retry budget:** How many retries should a rung get? High pass rate
   means fewer retries. Low pass rate means more retries.
2. **Skip advisory:** Should a rung be skipped? If it has passed 20+
   consecutive times, it is probably always passing and could be skipped.

Both signals are advisory -- the orchestrator may override them.

**Persistence:** `.roko/learn/gate-thresholds.json` (atomic write via
temp-file-then-rename).

---

## 2. Per-Rung Statistics

```rust
pub struct RungStats {
    pub ema_pass_rate: f64,       // EMA of pass rate [0.0, 1.0]
    pub total_observations: u64,  // Total gate runs
    pub consecutive_passes: u32,  // Reset on any failure
}
```

Fresh rungs start with `ema_pass_rate = 0.5` (neutral prior),
`total_observations = 0`, `consecutive_passes = 0`.

---

## 3. The EMA Algorithm

### 3.1 The Formula

```
EMA(t) = alpha * x(t) + (1 - alpha) * EMA(t-1)
```

Where `alpha = 0.1` (the `EMA_ALPHA` constant). This gives an effective
memory window of ~1/alpha = 10 observations.

### 3.2 Update Rule

```rust
pub fn update(&mut self, rung: u32, passed: bool) {
    let stats = self.rungs.entry(rung).or_default();
    let value = if passed { 1.0 } else { 0.0 };

    if stats.total_observations == 0 {
        stats.ema_pass_rate = value;
    } else {
        // EMA formula: alpha * new_value + (1 - alpha) * old_value
        stats.ema_pass_rate = EMA_ALPHA.mul_add(
            value,
            (1.0 - EMA_ALPHA) * stats.ema_pass_rate
        );
    }

    stats.total_observations += 1;

    if passed {
        stats.consecutive_passes += 1;
    } else {
        stats.consecutive_passes = 0;
    }
}
```

### 3.3 Alpha Parameter Selection

| alpha | Effective window | Behavior |
|-------|-----------------|----------|
| 0.01 | ~100 observations | Very stable, slow to adapt |
| **0.10** | **~10 observations** | **Balanced (current default)** |
| 0.30 | ~3 observations | Responsive, potentially noisy |

The choice of 0.1 balances responsiveness with stability. A gate that
fails once should not immediately triple the retry budget, but a gate that
fails 5 times in a row should trigger adjustment.

### 3.4 Why EMA Over Simple Average

Gate pass rates change over time:

- New project with many issues: low pass rates initially
- As issues are fixed: pass rates climb
- Major refactor: temporarily drops pass rates before recovery

A simple average (total passes / total observations) would be slow to
respond. EMA adapts within ~10 observations.

---

## 4. Retry Budget Suggestion

```rust
pub fn suggested_max_retries(&self, rung: u32) -> u32 {
    let Some(stats) = self.rungs.get(&rung) else {
        return 3; // Default for unknown rungs
    };

    if stats.total_observations < 5 {
        return 3; // Not enough data
    }

    // Linear mapping: high pass rate -> low retries, low -> high
    let retries = stats.ema_pass_rate.mul_add(-range, max).round() as u32;
    retries.clamp(MIN_RETRIES, MAX_RETRIES)
}
```

Linear mapping:

| Pass rate | Retries | Rationale |
|-----------|---------|-----------|
| 1.0 | 1 | Almost always passes; one attempt suffices |
| 0.5 | 3 | Coin flip; give it a few tries |
| 0.0 | 5 | Almost never passes; maximize attempts |

Constants: `MIN_RETRIES = 1`, `MAX_RETRIES = 5`.

### Cold Start

For unknown rungs or rungs with fewer than 5 observations, default is 3
retries. Avoids extreme behavior early in a project's lifecycle.

---

## 5. Skip Advisory

```rust
pub fn should_skip_rung(&self, rung: u32) -> bool {
    self.rungs
        .get(&rung)
        .is_some_and(|s| s.consecutive_passes >= SKIP_STREAK_THRESHOLD)
}
```

`SKIP_STREAK_THRESHOLD = 20`. If a rung has passed 20 consecutive times,
the system suggests skipping it.

### Why Advisory Only

Even 100 consecutive passes can fail unexpectedly. Making the skip
advisory enables the "mostly skip, periodically verify" pattern:

- Skip the gate 90% of the time for speed
- Every Nth run, still run it to check
- A failure resets the consecutive pass counter, re-enabling the gate

---

## 6. The EMA Feedback Loop

```
Gate pipeline executes
    -> Verdict(s) produced
    -> For each (rung, verdict):
         thresholds.update(rung, verdict.passed)
    -> thresholds.save(path)
    -> Next execution:
         suggested_max_retries(rung) may differ
         should_skip_rung(rung) may change
```

---

## 7. Statistical Process Control: CUSUM

**CUSUM (Cumulative Sum)** detects small, sustained changes that EMA might
smooth over. Tracks cumulative departures from target in both directions.

### 7.1 The Algorithm

```
C+(t) = max(0, C+(t-1) + z(t) - k)     # upward shift (improving)
C-(t) = max(0, C-(t-1) - z(t) - k)     # downward shift (degrading)
z(t) = (x(t) - mu_0) / sigma            # standardized observation
```

Signal when `C+ > h` or `C- > h`.

### 7.2 Parameters

| Parameter | Default | Effect |
|-----------|---------|--------|
| `k` (reference value) | 0.25 | Lower = more sensitive to small shifts |
| `h` (decision interval) | 4.0 | Lower = faster detection, more false alarms |
| FIR reset | `h/2` | Halves accumulator on signal, enabling re-detection |

### 7.3 ARL (Average Run Length)

With `k=0.25, h=4.0`:

- ARL_0 ~ 168 observations before false alarm (in-control)
- For a 0.5-sigma shift: detection in ~20 observations on average

### 7.4 Binary Data Considerations

For binary pass/fail data:

```
sigma = sqrt(p * (1 - p))
```

At baseline p=0.85: sigma ~ 0.357. A shift from 85% to 70% (delta=0.15)
represents ~0.42 sigma, which CUSUM with k=0.25 detects within 25-30
observations.

---

## 8. EWMA Control Chart

Extends the existing EMA with formal upper/lower control limits (UCL/LCL).

### 8.1 Control Limit Formulas

```
UCL = mu_0 + L * sigma_z
LCL = mu_0 - L * sigma_z

sigma_z = sigma * sqrt( (lambda / (2 - lambda)) * (1 - (1 - lambda)^(2n)) )
```

Where:

- `lambda = 0.10` (smoothing parameter, same as EMA_ALPHA)
- `L = 2.814` (limit width in sigma units)
- `n` = number of observations (time-varying limits)

### 8.2 Time-Varying Limits

Limits are wider early (few observations) and converge to steady-state as
n grows. The term `(1 - (1 - lambda)^(2n))` converges to 1.0 as n
increases, giving the asymptotic limit:

```
sigma_z_asymptotic = sigma * sqrt(lambda / (2 - lambda))
```

For lambda=0.10: sigma_z_asymptotic ~ 0.229 * sigma.

### 8.3 ARL Tuning

| lambda | L | ARL_0 (in-control) | ARL_1 (1-sigma shift) |
|--------|---|-------|-------|
| 0.05 | 2.625 | ~500 | ~26 |
| **0.10** | **2.814** | **~500** | **~31** |
| 0.20 | 2.962 | ~500 | ~41 |

ARL_0 ~ 500 means one false alarm per ~500 observations. ARL_1 ~ 31
means a true 1-sigma shift is detected in ~31 observations on average.

### 8.4 Out-of-Control Detection

```rust
pub fn is_in_control(&self) -> bool {
    let (lcl, ucl) = self.control_limits();
    self.z >= lcl && self.z <= ucl
}
```

When out-of-control: flag gate in dashboard, notify conductor.

---

## 9. BOCPD -- Bayesian Online Change Point Detection

Answers "did the gate's fundamental behavior change?" probabilistically.

> **Citation:** Adams & MacKay, "Bayesian Online Changepoint Detection"
> (arXiv:0710.3742, 2007).

### 9.1 How It Works

Maintains a posterior distribution over run lengths (time since last change
point). When P(run_length = 0) spikes above `changepoint_threshold = 0.5`,
a regime change is declared.

### 9.2 The Algorithm

```
1. Compute predictive probability for each run length
2. Growth probabilities: P(r_{t+1} = r_t + 1) = P(r_t) * pred * (1 - hazard)
3. Change-point mass: P(r_{t+1} = 0) = sum of P(r_t) * pred * hazard
4. Normalize posterior
5. Update sufficient statistics (Normal-Gamma conjugate)
6. Detect: P(r=0) > threshold => change point
```

### 9.3 Parameters

| Parameter | Default | Effect |
|-----------|---------|--------|
| `hazard_rate` | 1/200 | Prior on change frequency |
| `max_run_length` | 300 | Truncation depth (memory vs accuracy) |
| `changepoint_threshold` | 0.5 | Sensitivity to regime changes |

### 9.4 When to Recalibrate

When BOCPD detects a change point:

1. Reset the CUSUM accumulators to zero
2. Update the EWMA target mean (mu_0) to the post-changepoint EMA
3. Log a regime-change event to efficiency telemetry
4. Notify the dashboard

---

## 10. PELT -- Offline Change-Point Detection

Provides retrospective change-point detection for historical gate data.

> **Citation:** Killick et al., "Optimal Detection of Changepoints with
> a Linear Computational Cost" (arXiv:1101.1438, 2012).

### 10.1 Algorithm

O(n) expected complexity with pruning. Finds all points where the gate's
statistical properties changed using a penalized cost function:

```
Minimize: sum(cost(segment)) + penalty * num_changepoints
```

Cost function: Bernoulli negative log-likelihood for binary pass/fail
data, Gaussian negative log-likelihood for continuous scores.

### 10.2 Retrospective Report Example

```
PELT analysis of Test gate pass rates (last 500 observations):

Change points detected at: [47, 183, 312]

Segment 1 (0-47):    mean 0.92 +/- 0.04  [stable, healthy]
Segment 2 (48-183):  mean 0.71 +/- 0.08  [regression]
Segment 3 (184-312): mean 0.88 +/- 0.05  [recovery]
Segment 4 (313-500): mean 0.82 +/- 0.06  [current regime]
```

---

## 11. Hotelling's T-Squared -- Multi-Gate Detection

Monitors the joint distribution of gate metrics across all rungs
simultaneously, detecting correlated anomalies that per-gate monitors miss.

### 11.1 The Statistic

```
T-squared = (x - mu)^T * Sigma^{-1} * (x - mu)
```

When T-squared exceeds the chi-squared critical value (alpha=0.01, p=7
gates, threshold ~ 18.48), a multi-gate anomaly is flagged.

### 11.2 Per-Gate Attribution

On anomaly detection, the `JointAnomalyResult` provides per-gate
contribution scores identifying which gates drive the anomaly.

### 11.3 Coordination Policies

When multi-gate anomaly detected:

- **Sympathetic tightening:** Downstream gate degrades -> tighten upstream
  gates (test failures -> stricter compile/lint)
- **Compensatory:** One gate relaxes -> neighbors tighten to maintain
  overall verification strength

---

## 12. Persistence

```rust
pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(self)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load_or_new(path: &Path) -> Self {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
```

Atomic write pattern: write to temp file, then rename. If the process
crashes between write and rename, the old file remains intact.

`load_or_new()` returns a fresh `AdaptiveThresholds` if the file is
missing or corrupt. Graceful degradation -- the system always starts
correctly.

Storage: `.roko/learn/gate-thresholds.json`.

---

## 13. SPC Hierarchy

All four methods work together:

```
Per gate observation (pass/fail)
    |
    +-- EMA update (existing) --- smoothed pass rate
    |
    +-- CUSUM update ----------- sustained shift detection
    |    +-- shift? -> adjust retry budget aggressively
    |
    +-- EWMA control chart ----- formal anomaly detection
    |    +-- out of control? -> flag gate in dashboard
    |
    +-- BOCPD update ----------- regime change detection
         +-- change point? -> recalibrate all detectors
```

---

## 14. Dashboard Reporting

```
Gate Thresholds:
  Rung 0 (Compile):  98.2% pass, 142 obs, 31 consecutive [SKIP]
  Rung 1 (Lint):     87.5% pass, 130 obs, 8 consecutive
  Rung 2 (Test):     72.1% pass, 118 obs, 3 consecutive
  Rung 3 (Symbol):   95.0% pass, 45 obs, 15 consecutive
```

---

## Verification

```bash
cargo test -p roko-gate -- adaptive_threshold
cargo test -p roko-gate -- spc
cargo test -p roko-gate -- hotelling
cargo test -p roko-gate -- pelt
cargo run -p roko-cli -- learn gates
```
