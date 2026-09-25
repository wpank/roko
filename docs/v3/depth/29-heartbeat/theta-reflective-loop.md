# Theta: The Reflective Loop (~75s)

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/05-theta-reflective-loop.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS6.

---

## 1. Abstract

Theta is the breath. Every ~75 seconds (adapting between 30-120 seconds based on
environmental regime), the agent pauses its reactive gamma processing and asks:
*"Am I on the right track?"* Theta is not about reacting to individual observations
-- that is gamma's job. Theta is about reflecting on patterns across recent gamma
ticks, checking prediction calibration, updating the Daimon's behavioral state, and
re-evaluating the current plan.

The name comes from EEG theta oscillations (4-8 Hz), which Buzsaki (2006) associates
with navigation, memory encoding, and the hippocampal "indexing" of episodic memories.
In the brain, theta oscillations coordinate the binding of fast gamma events into
coherent episodes. Roko's theta loop serves the same function: it integrates the fast
gamma ticks into a coherent narrative.

Theta always invokes at least T1 -- it needs LLM reasoning to reflect meaningfully.
Most theta cycles use T1 ($0.005-0.01), with complex situations escalating to T2
($0.03-0.10).

---

## 2. Trigger Conditions

Theta fires based on two conditions, whichever comes first:

1. **Gamma count**: Every N=5 gamma cycles, theta fires.
2. **Episode completion**: When a logical unit of work completes, theta fires
   immediately.

```rust
fn should_fire_theta(
    gamma_since_last_theta: u32,
    episode_completed: bool,
    theta_config: &ThetaConfig,
) -> bool {
    gamma_since_last_theta >= theta_config.gamma_count_trigger  // default: 5
        || episode_completed
}
```

---

## 3. The Five Phases

### 3.1 Phase 1: Summarize Recent Gamma Work

Theta reads the DecisionCycleRecords from recent gamma ticks and extracts patterns:

- **Outcome distribution**: T0/T1/T2 counts, success/failure ratio.
- **Anomaly patterns**: Recurring probe alerts.
- **Action patterns**: What actions were taken, were they effective.
- **Cost accumulation**: Spending since last theta tick.

```rust
fn summarize_gamma_history(records: &[DecisionCycleRecord]) -> GammaSummary {
    let tier_counts = records.iter()
        .fold([0u32; 3], |mut acc, r| {
            acc[r.tier as usize] += 1;
            acc
        });
    let success_rate = records.iter()
        .filter(|r| r.outcome.as_ref().map_or(false, |o| o.passed))
        .count() as f32 / records.len().max(1) as f32;
    let total_cost: f64 = records.iter().map(|r| r.total_cost).sum();
    GammaSummary { tick_count: records.len() as u32, tier_distribution: tier_counts,
        success_rate, total_cost, .. }
}
```

### 3.2 Phase 2: Update Daimon State

Theta computes aggregate affect using the ALMA three-layer model (Gebhard 2005):

| Layer | Timescale | What It Represents | Decay Rate |
|---|---|---|---|
| **Emotion** | Seconds | Immediate reaction | Fast (alpha = 0.20) |
| **Mood** | Hours | Sustained state | Slow (alpha = 0.02, 4h half-life) |
| **Personality** | Permanent | Baseline disposition | None |

Six behavioral states derived from PAD:

| State | PAD Region | Effect |
|---|---|---|
| Engaged | P > 0.2, A > 0.1, D > 0.1 | Standard operation |
| Struggling | P < -0.2, A > 0.3 | More caution, lower risk |
| Coasting | P > 0.1, A < -0.1, D > 0.2 | May miss opportunities |
| Exploring | P ~ 0, A > 0.2, D < 0 | Higher T2 rate acceptable |
| Focused | P > 0, A > 0.3, D > 0.3 | Deep work, minimize distractions |
| Resting | A < -0.2 | Pre-delta state |

### 3.3 Phase 3: Check Predictions

CalibrationTracker aggregates prediction residuals per (model, task_category) pair.
Arithmetic correction: `adjusted = raw - mean_bias(model, category)` at ~50
nanoseconds. No LLM needed.

### 3.4 Phase 4: Re-Evaluate Plan

The core of theta. LLM (T1 or T2) reasons about: Is the plan still valid? Am I
making progress? Should I re-prioritize? Are there patterns in failures?

```rust
async fn reevaluate_plan(
    gamma_summary: &GammaSummary,
    calibration: &CalibrationReport,
    daimon_state: &DaimonState,
    current_plan: &Plan,
    composer: &dyn Composer,
    agent: &dyn Agent,
) -> Result<ThetaReflection> {
    let tier = if calibration.trend == Trend::Declining
        || daimon_state.behavioral_state() == BehavioralState::Struggling
        || gamma_summary.success_rate < 0.3
    { InferenceTier::T2 } else { InferenceTier::T1 };

    let context = composer.compose(
        &ThetaContextRequest { gamma_summary, calibration, .. },
        &ContextBudget::theta(),
    )?;

    agent.execute(&context, tier, &ThetaReflectionPrompt).await
}
```

### 3.5 Phase 5: Trigger Interventions

- **Stuck detection**: >3 retries on same task -> escalation.
- **Cost anomaly**: T2 rate > 20% -> tighten threshold.
- **Calibration collapse**: accuracy < 40% -> Struggling state.
- **Complacency detection**: Coasting + declining accuracy -> flag.

---

## 4. Adaptive Theta Interval

| Regime | Multiplier | Theta Interval | Ticks/Hour |
|---|---|---|---|
| Calm | 1.6x | 120s | 30 |
| Normal | 1.0x | 75s | 48 |
| Volatile | 0.4x | 30s | 120 |
| Crisis | 0.2x | 15s | 240 |

```rust
fn compute_theta_interval(regime: Regime, base: Duration) -> Duration {
    let multiplier = match regime {
        Regime::Calm => 1.6,
        Regime::Normal => 1.0,
        Regime::Volatile => 0.4,
        Regime::Crisis => 0.2,
    };
    Duration::from_secs_f64(base.as_secs_f64() * multiplier)
        .max(Duration::from_secs(15))
        .min(Duration::from_secs(120))
}
```

---

## 5. Theta's Role in the Hierarchy

Theta bridges individual ticks and long-term learning:

- **Upward (gamma -> theta)**: Gamma ticks are summarized into episode-level Signals
  for delta processing.
- **Downward (theta -> gamma)**: Threshold changes and state transitions immediately
  change gamma behavior.

```
Gamma ticks:  [T0] [T0] [T1] [T0] [T0]
                    ||||| aggregate
Theta:            [REFLECT]
                    | adjustments
Gamma ticks:  [T0] [T0] [T0] [T0] [T1]  <- threshold adjusted
```

Theta also increments **sleep pressure** toward the delta threshold.

---

## 6. References

- **Buzsaki 2006** -- "Rhythms of the Brain" (Oxford University Press).
- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience 11(2)).
- **Clark 2013** -- "Whatever Next?" (Behavioral and Brain Sciences 36(3)).
- **Gebhard 2005** -- "ALMA: A Layered Model of Affect" (AAMAS 2005).
- **Barrett 2017** -- "How Emotions Are Made" (Houghton Mifflin).
- **Mattar & Daw 2018** -- "Prioritized memory access" (Nature Neuroscience 21).
- **Scherer 2001** -- "Appraisal considered as a process of multilevel sequential
  checking" (Oxford University Press).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/gamma-reactive-loop.md` -- Fast loop theta summarizes
- `docs/v3/depth/29-heartbeat/delta-consolidation-loop.md` -- Slow loop theta feeds
- `docs/v3/depth/29-heartbeat/adaptive-clock.md` -- Clock managing all timescales
- `docs/v3/depth/29-heartbeat/dual-process-t0-t1-t2.md` -- Tier gating theta adjusts
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
