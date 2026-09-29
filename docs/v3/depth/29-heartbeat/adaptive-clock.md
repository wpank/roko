# Adaptive Clock

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/07-adaptive-clock.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS8.

---

## 1. Abstract

The adaptive clock is the runtime policy that publishes the three heartbeat tick
Pulses on the Bus. It dynamically adjusts each frequency based on environmental
regime, resource constraints, and agent behavioral state.

It does not orchestrate cognition directly. A `HeartbeatPolicy` emits
`heartbeat.gamma.tick`, `heartbeat.theta.tick`, and `heartbeat.delta.tick` at
cadence, and the speed-specific consumers subscribe by topic. That makes the
clock a Bus producer, not a special control-flow mechanism.

The adaptive clock draws from Friston's (2010) free energy principle: the agent
should sample its environment more frequently when prediction error is high and
less frequently when prediction error is low. Clark (2013) extends this into the
predictive brain framework. Buzsaki (2006) establishes that oscillatory hierarchies
enable simultaneous processing at different temporal resolutions.

---

## 2. Configuration

```toml
[clock]
gamma_min_interval_secs = 5
gamma_max_interval_secs = 15
gamma_base_interval_secs = 10
theta_min_interval_secs = 15
theta_max_interval_secs = 120
theta_base_interval_secs = 75
theta_gamma_count = 5
delta_episode_threshold = 50
delta_idle_timeout_secs = 300
daily_budget_usd = 50.0
throttle_at_percent = 80
hard_stop_at_percent = 95
```

All intervals have minimum and maximum bounds. The adaptive logic operates within
these bounds -- it can never make gamma faster than 5 seconds or slower than 15
seconds.

---

## 3. Regime Detection

| Regime | Chain Domain | Coding Domain | Research Domain | Universal |
|---|---|---|---|---|
| **Calm** | Low volatility, stable | All tests passing | Low citation churn | Prediction error < 0.1 |
| **Normal** | Moderate movement | Some flakiness | Moderate discovery | Prediction error 0.1-0.3 |
| **Volatile** | High swings, gas spikes | Build failures, regressions | Major findings | Prediction error 0.3-0.6 |
| **Crisis** | Flash crash, exploit | Critical outage | Paradigm challenge | Prediction error > 0.6 |

Regime is stored in CorticalState (`regime: AtomicU8`) and read by `HeartbeatPolicy`.

---

## 4. Frequency Adjustment Rules

### 4.1 Gamma Adaptation

```rust
fn compute_gamma_interval(violations: &[Violation], config: &ClockConfig) -> Duration {
    let base = Duration::from_secs(config.gamma_max_interval_secs);
    let adjusted = base.mul_f64(1.0 / (1.0 + violations.len() as f64 * 0.3));
    adjusted
        .max(Duration::from_secs(config.gamma_min_interval_secs))
        .min(Duration::from_secs(config.gamma_max_interval_secs))
}
```

| Anomaly Count | Interval | Ticks/Hour |
|---|---|---|
| 0 | 15.0s | 240 |
| 1 | 11.5s | 313 |
| 2 | 9.4s | 383 |
| 3 | 7.9s | 456 |
| 5 | 6.0s | 600 |
| 7+ | 5.0s (floor) | 720 |

### 4.2 Theta Adaptation

```rust
fn compute_theta_interval(regime: Regime, config: &ClockConfig) -> Duration {
    let multiplier = match regime {
        Regime::Calm => 1.6,
        Regime::Normal => 1.0,
        Regime::Volatile => 0.4,
        Regime::Crisis => 0.2,
    };
    let base = Duration::from_secs(config.theta_base_interval_secs);
    Duration::from_secs_f64(base.as_secs_f64() * multiplier)
        .max(Duration::from_secs(config.theta_min_interval_secs))
        .min(Duration::from_secs(config.theta_max_interval_secs))
}
```

### 4.3 Delta Timing

Delta fires based on triggers, not a periodic timer. During volatile periods,
the episode threshold drops to 30. During calm periods, it rises to 80.

---

## 5. Budget-Aware Throttling

```rust
fn apply_budget_throttle(
    interval: Duration,
    budget_pct: f64,
    config: &ClockConfig,
) -> Duration {
    if budget_pct >= config.hard_stop_at_percent as f64 / 100.0 {
        Duration::from_secs(config.theta_max_interval_secs)
    } else if budget_pct >= 0.90 {
        interval.mul_f64(4.0)
            .min(Duration::from_secs(config.theta_max_interval_secs))
    } else if budget_pct >= config.throttle_at_percent as f64 / 100.0 {
        interval.mul_f64(2.0)
            .min(Duration::from_secs(config.theta_max_interval_secs))
    } else {
        interval
    }
}
```

| Budget Usage | Effect |
|---|---|
| < 80% | No throttling |
| 80-90% | Theta intervals 2x |
| 90-95% | Theta intervals 4x, T2 restricted to crisis |
| > 95% | T2 stopped, theta at maximum interval |

**Gamma T0 probes always run** regardless of budget. They cost $0.00. Even at 100%
utilization, the agent maintains perception.

---

## 6. Event-Driven Wakeup

```rust
pub enum WakeupCondition {
    UserIntervention,
    SafetyAlert,
    PheromoneAlert { intensity: f32 },
    BudgetAlert,
    ScheduledEvent(EventId),
}
```

When a wakeup condition fires, the policy skips remaining sleep time and immediately
emits a new `heartbeat.gamma.tick` Pulse.

---

## 7. The Frequency Scheduler

```rust
pub struct FrequencyScheduler {
    clock: AdaptiveClock,
    cortical: Arc<CorticalState>,
}

impl FrequencyScheduler {
    pub async fn run(&self) {
        loop {
            let snapshot = self.cortical.snapshot();
            let gamma_interval = self.clock.compute_gamma_interval(
                &snapshot.recent_anomalies);
            let theta_interval = self.clock.compute_theta_interval(snapshot.regime);
            if self.clock.should_enter_delta(
                snapshot.idle_duration, snapshot.episodes_since_delta) {
                self.clock.emit_signal(CognitiveSignal::Resume);
            }
            let throttled_theta = apply_budget_throttle(
                theta_interval, budget_pct, &self.clock.config);
            self.clock.set_gamma_interval(gamma_interval);
            self.clock.set_theta_interval(throttled_theta);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
```

---

## 8. References

- **Buzsaki 2006** -- "Rhythms of the Brain" (Oxford University Press).
- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience 11(2)).
- **Clark 2013** -- "Whatever Next?" (Behavioral and Brain Sciences 36(3)).
- **Sims 2003** -- "Implications of rational inattention" (Journal of Monetary
  Economics 50(3)).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/three-cognitive-speeds-t0-t1-t2.md` -- Three speeds
- `docs/v3/depth/29-heartbeat/gamma-reactive-loop.md` -- Gamma loop
- `docs/v3/depth/29-heartbeat/theta-reflective-loop.md` -- Theta loop
- `docs/v3/depth/29-heartbeat/delta-consolidation-loop.md` -- Delta loop
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
