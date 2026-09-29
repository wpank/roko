# Dual-Process Cognition: T0, T1, T2

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/08-dual-process-t0-t1-t2.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS9--11.
> Cite: Kahneman (2011), "Thinking, Fast and Slow".

---

## 1. Abstract

Daniel Kahneman's dual-process theory ("Thinking, Fast and Slow", 2011) distinguishes
two modes of cognition: System 1 (fast, automatic, effortless, heuristic-based) and
System 2 (slow, deliberate, effortful, analytical). In the brain, most processing is
System 1 -- you do not consciously decide to read these words, recognize a face, or
dodge a thrown object. System 2 engages only when System 1 detects something that
requires attention: a complex math problem, an unexpected event, a novel situation.

Roko implements this distinction literally. The three cognitive tiers -- T0, T1, T2
-- are not abstract labels. They correspond to concrete implementation choices with
dramatically different costs:

| Tier | Kahneman | Implementation | Cost per Call | Latency | Frequency |
|---|---|---|---|---|---|
| **T0** | System 1 (pure) | Deterministic probes + playbook rules. No LLM. | $0.00 | <10ms | ~80% |
| **T1** | System 1 -> System 2 | Fast LLM (Haiku-class). Reduced context. | $0.001-0.003 | 200-500ms | ~15% |
| **T2** | System 2 (deep) | Full LLM (Sonnet/Opus-class). Full Cognitive Workspace. | $0.01-0.25 | 1-5s | ~5% |

The 80/15/5 distribution is an **emergent property** of the gating mechanism, not a
target. The LLM-Last principle: the LLM is the last resort, not the first.

---

## 2. The LLM-Last Principle

Grounded in five frameworks:

1. **Kahneman (2011)** -- System 1 handles routine; System 2 handles exceptions.
2. **FrugalGPT** (Chen et al. 2023, arXiv:2305.05176; published 2024 TMLR) --
   Cascade architectures achieve up to 98% cost reduction.
3. **DPT-Agent** (Zhang et al. 2025, arXiv:2502.11882) -- Dual-process theory
   applied directly to LLM agent decision-making.
4. **CLARION** (Sun et al. 2005) -- Dual-level processing: implicit (fast) and
   explicit (slow).
5. **Talker-Reasoner** (Google Research, 2024) -- Separation of fast heuristic
   from slow analytical processing.

The economic argument: 8,640 ticks/day at $0.10/tick = $864/day without gating. With
80% T0 suppression: ~$2-50/day.

---

## 3. The InferenceTier Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InferenceTier {
    T0 = 0,  // Suppress. No LLM. ~80%. $0.00.
    T1 = 1,  // Analyze. Haiku-class. ~4K tokens. ~15%. $0.001-0.003.
    T2 = 2,  // Deliberate. Sonnet/Opus-class. ~32K tokens. ~5%. $0.01-0.25.
}
```

### 3.1 TierRouter

```rust
pub struct TierRouter;

impl TierRouter {
    pub fn select_model(tier: InferenceTier, resource_health: f32) -> Option<&'static str> {
        match tier {
            InferenceTier::T0 => None,
            InferenceTier::T1 => Some("claude-haiku-4-5"),
            InferenceTier::T2 => {
                if resource_health > T2_RESOURCE_THRESHOLD {
                    Some("claude-opus-4-6")
                } else {
                    Some("claude-sonnet-4-6")
                }
            }
        }
    }
}

pub const T2_RESOURCE_THRESHOLD: f32 = 0.3;
```

---

## 4. Prediction Error Computation

```rust
fn compute_prediction_error(
    probes: &[ProbeResult],
    predictions: &PredictionState,
    regime: &Regime,
) -> f32 {
    let mut error: f32 = 0.0;
    let anomaly_count = probes.iter().filter(|p| p.is_anomalous()).count();
    error += anomaly_count as f32 * 0.05;
    if regime.changed_since_last_tick() { error += 0.40; }
    error += predictions.compute_drift() * 0.30;
    let pending = predictions.pending_intervention_count();
    error += pending as f32 * 0.10;
    error.min(1.0)
}
```

| Component | Weight | Rationale |
|---|---|---|
| Probe anomaly | 0.05 each | 4+ anomalies cross T1 threshold |
| Regime change | 0.40 flat | Strongest indicator of stale world model |
| World model drift | 0.30 * drift | Continuous signal; max alone = 0.30 |
| Pending intervention | 0.10 each | Two interventions match the base threshold |

Per-probe anomaly detection uses rolling z-score:

```rust
pub struct ProbeResult {
    pub probe_id: &'static str,
    pub value: f64,
    pub rolling_mean: f64,
    pub rolling_stddev: f64,
    pub z_threshold: f64,  // default: 2.0
}

impl ProbeResult {
    pub fn is_anomalous(&self) -> bool {
        if self.rolling_stddev < f64::EPSILON { return false; }
        let z = (self.value - self.rolling_mean).abs() / self.rolling_stddev;
        z > self.z_threshold
    }
}
```

Drift metric: normalized Euclidean distance over a 5-dimension CorticalState vector
(regime, accuracy, resource_health, active_ratio, arousal).

---

## 5. Adaptive Threshold

All modulation is **additive** (not multiplicative, to prevent compounding). Clamped
to [0.05, 0.50]:

```rust
fn compute_adaptive_threshold(state: &AgentState) -> f32 {
    let base = 0.20;
    let dominance = state.cortical_state.pad().dominance;
    let affect_adj = if dominance < -0.2 { -0.05 }
                     else if dominance > 0.3 { 0.05 }
                     else { 0.0 };
    let budget_pct = state.budget_tracker.daily_usage_percent();
    let resource_adj = if budget_pct > 0.80 { 0.10 } else { 0.0 };
    let arousal = state.cortical_state.pad().arousal;
    let arousal_adj = if arousal > 0.5 { -0.05 } else { 0.0 };
    let confidence_adj = state.strategy_confidence * 0.05;
    (base + affect_adj + resource_adj + arousal_adj + confidence_adj)
        .clamp(0.05, 0.50)
}
```

| Condition | Adjustment | Justification |
|---|---|---|
| Low dominance (< -0.2) | -0.05 | Uncertain agents think more (Kahneman) |
| High dominance (> 0.3) | +0.05 | Confident agents coast on heuristics |
| Budget > 80% used | +0.10 | Conservation overrides curiosity |
| High arousal (> 0.5) | -0.05 | Surprise sharpens attention |
| Strategy confidence | +0.00 to +0.05 | Continuous: confidence * 0.05 |

---

## 6. Gating Decision

```rust
fn gate(prediction_error: f32, threshold: f32, state: &AgentState) -> InferenceTier {
    if state.has_forced_escalation() { return InferenceTier::T2; }
    if prediction_error < threshold { InferenceTier::T0 }
    else if prediction_error < threshold * 2.0 { InferenceTier::T1 }
    else { InferenceTier::T2 }
}
```

The 2x multiplier ensures moderate surprises get cheap T1 analysis while only
genuinely novel situations trigger expensive T2.

---

## 7. What Each Tier Does

### 7.1 T0: Suppress

No LLM. The 16 T0 probes execute, prediction error is computed, CorticalState is
updated, DecisionCycleRecord is written. T0 also checks playbook rules: if a known
situation matches, the agent acts without LLM involvement.

T0 playbook actions execute in a restricted sandbox -- they cannot invoke LLMs,
modify the plan DAG, or send external requests. They can update CorticalState,
emit CognitiveSignals, log observations, or adjust the adaptive clock.

### 7.2 T1: Analyze

Fast LLM (Haiku-class) with ~4,000 token context: system prompt (~1,200), top-5
Neuro entries (~1,500), active tasks (~800), critical warnings (~500). Uses layers
1-3 of the SystemPromptBuilder.

### 7.3 T2: Deliberate

Full LLM (Opus/Sonnet-class) with ~32,000 token Cognitive Workspace (Baddeley 2000):
invariants, strategy, playbook heuristics, retrieved episodes, insights, causal graph,
dream hypotheses, somatic landscape, pheromone summary, conversation tail.

---

## 8. The TierDecision Struct

```rust
pub struct TierDecision {
    pub tick_id: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tier: InferenceTier,
    pub prediction_error: f32,
    pub threshold: f32,
    pub anomaly_count: u32,
    pub regime_changed: bool,
    pub drift: f32,
    pub pending_interventions: u32,
    pub forced: bool,
    pub force_reason: Option<String>,
    pub pad: PadVector,
    pub budget_usage_pct: f32,
    pub strategy_confidence: f32,
    pub playbook_rule_id: Option<String>,
    pub model: Option<String>,
    pub resource_health: f32,
}
```

---

## 9. Configuration Parameters

| Parameter | Default | Range |
|---|---|---|
| `base_threshold` | 0.20 | [0.05, 0.50] |
| `t1_t2_multiplier` | 2.0 | [1.5, 4.0] |
| `anomaly_weight` | 0.05 | [0.01, 0.15] |
| `regime_change_weight` | 0.40 | [0.20, 0.60] |
| `drift_weight` | 0.30 | [0.10, 0.50] |
| `intervention_weight` | 0.10 | [0.05, 0.25] |
| `z_score_threshold` | 2.0 | [1.5, 3.0] |
| `t2_resource_threshold` | 0.30 | [0.10, 0.50] |

---

## 10. Error Handling

| Failure mode | Behavior |
|---|---|
| All probes fail | prediction_error = 0.50 (failsafe). Log warning. |
| PredictionState unavailable (first tick) | Skip drift term. |
| CorticalState read fails | Use base threshold (0.20). Log error. |
| Budget tracker unavailable | Skip resource adjustment. |
| Playbook rule action panics | Catch. Log error. Proceed to tier gating. |

---

## 11. Verification

| Test | Assertion |
|---|---|
| Zero anomalies, no regime change | `tier == T0` |
| 4 anomalies, no other signals | `tier == T1` (0.20 >= base 0.20) |
| 8 anomalies, no other signals | `tier == T2` (0.40 >= base * 2.0) |
| Regime change alone | `tier == T2` (0.40 >= 0.40) |
| Budget > 80% shifts threshold to 0.30 | 6 anomalies needed for T1 |
| Forced escalation flag | `tier == T2` regardless |
| All probes fail | prediction_error == 0.50, tier == T2 |
| Low resource health at T2 | Model is Sonnet, not Opus |

---

## 12. References

- **Kahneman 2011** -- "Thinking, Fast and Slow" (Farrar, Straus and Giroux).
- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience 11(2)).
- **Chen et al. 2023** -- FrugalGPT (arXiv:2305.05176; published 2024 TMLR).
- **Sun et al. 2005** -- CLARION (Psychological Review).
- **Baddeley 2000** -- Working memory model (Trends in Cognitive Sciences 4(11)).
- **Damasio 1994** -- "Descartes' Error" (Putnam). Somatic marker hypothesis.
- **Zhang et al. 2025** -- DPT-Agent (arXiv:2502.11882).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/gamma-reactive-loop.md` -- Per-tick tier selection
- `docs/v3/depth/29-heartbeat/16-t0-probes.md` -- Probes driving T0 suppression
- `docs/v3/depth/29-heartbeat/active-inference-compute-allocation.md` -- EFE theory
- `docs/v3/depth/29-heartbeat/attention-auction-and-gating.md` -- VCG context assembly
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
