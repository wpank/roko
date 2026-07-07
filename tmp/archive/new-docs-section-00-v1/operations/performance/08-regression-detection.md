# Regression Detection

> How Roko's learning subsystem detects performance regressions across runs and surfaces
> them to operators.

**Status**: Built (code exists and is tested; not yet wired to CI gate)
**Crate**: `roko-learn`
**Depends on**: [07-benchmarks-reference.md](07-benchmarks-reference.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`roko-learn` includes a regression detector that compares episode-derived performance
metrics (token counts, gate pass rates, task duration) across a rolling window of
executions. When a metric degrades beyond a threshold, a `PerformanceRegression` Pulse
is emitted. **Status: Built** — the detector is implemented and tested; it is not yet
wired to the main execution path or CI.

---

## What Is Detected

The regression detector monitors two categories of metric:

### Episode-level metrics (per task)

| Metric | Window | Regression trigger |
|--------|--------|--------------------|
| `gate_pass_rate` | 50 tasks | Drop > 10 percentage points |
| `avg_retries` | 50 tasks | Increase > 0.5 retries |
| `avg_llm_tokens` | 100 tasks | Increase > 20% |
| `avg_task_duration_s` | 100 tasks | Increase > 30% |
| `avg_cost_usd` | 100 tasks | Increase > 25% |
| `first_attempt_pass_rate` | 100 tasks | Drop > 15 percentage points |

### Benchmark-level metrics (per benchmark run)

| Metric | Window | Regression trigger |
|--------|--------|--------------------|
| Benchmark p99 for any hot-path op | Last 5 CI runs | Increase > 25% |
| Benchmark p50 for any hot-path op | Last 5 CI runs | Increase > 15% |

---

## Detection Algorithm

The detector uses a two-sample Welch's t-test comparing the current window against
the reference window:

1. **Reference window**: the last N tasks before the suspected change point (or the
   historical baseline if no change point is detected).
2. **Current window**: the last M tasks.
3. **Test statistic**: Welch's t-test with `p < 0.05` as the significance threshold.
4. **Effect size**: Cohen's d > 0.5 required (medium effect) to avoid noise-triggered
   alerts.

Both the p-value and the effect size must cross their thresholds to emit a regression
alert. This prevents alerting on statistically significant but practically irrelevant
changes (e.g. 1 ms slower at p < 0.001).

---

## How Alerts Surface

When a regression is detected, the `PerformanceRegression` Pulse is emitted on the
event bus:

```rust
pub struct PerformanceRegressionPulse {
    pub metric: MetricName,
    pub reference_value: f64,
    pub current_value: f64,
    pub percent_change: f64,
    pub p_value: f64,
    pub effect_size: f64,
    pub window_size: usize,
    pub detected_at: Instant,
}
```

This Pulse:
1. Is logged as a `warn!` event (visible with `ROKO_LOG=roko=warn`).
2. Is persisted as an Engram with kind `PerformanceRegression`.
3. Can be subscribed to by any Policy operator (e.g. to trigger an automatic alert or
   to pause new task dispatch pending investigation).

**Status**: The Pulse is emitted correctly. The Policy hook for automatic response
(pause dispatch, send notification) is not yet implemented.

---

## Manual Regression Check

Run the regression check manually against your episode store:

```bash
roko learn regression-check
```

Output:

```
Checking performance metrics over last 100 tasks...

✓ gate_pass_rate: 91.2% (reference: 90.8%) — no regression
✓ avg_retries: 1.2 (reference: 1.1) — no regression
⚠ avg_task_duration_s: 28.4s (reference: 19.1s) — +48.7% REGRESSION
  p-value: 0.003, Cohen's d: 0.72
  Likely cause: gate.timeout_seconds increased, or test suite grew

Recommendation: check gate.timeout_seconds, recent gate.pipeline changes,
and whether new slow tests were added.
```

---

## Integrating with CI

To run the regression check as a CI gate:

```bash
# In your CI workflow, after running Roko on a representative task set:
roko learn regression-check --fail-on-regression

# Exit code 0 = no regression
# Exit code 1 = regression detected (details on stdout)
```

This is **planned** as an optional CI gate (`"regression"` in `gate.pipeline`). Not yet
implemented.

---

## See Also

- [07-benchmarks-reference.md](07-benchmarks-reference.md) — micro-benchmarks vs episode metrics
- [operations/error-handling/08-observability.md](../error-handling/08-observability.md) — where `PerformanceRegression` Pulses surface

## Open Questions

- The regression detector is not yet wired to the CI gate pipeline.
- Change-point detection (PELT / BOCPD algorithm) is planned to improve accuracy of regression detection when the reference window is mixed (pre- and post-regression data).
- Per-task-category regression tracking (so a regression in "architectural" tasks doesn't dilute the "config" task baseline) is planned.
