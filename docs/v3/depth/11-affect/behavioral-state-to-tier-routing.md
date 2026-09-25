# Behavioral State to Tier Routing

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/05-behavioral-state-to-tier-routing.md`

---

## Overview

The tier routing system determines how much compute the agent spends on each
cognitive operation. The CascadeRouter (in `roko-learn`) uses a prediction error
scalar (0.0-1.0) from zero-cost probes to classify each operation into one of
three tiers: T0 (free, deterministic probes only), T1 (fast model, shallow
reasoning), or T2 (full model, deep reasoning). The Daimon's behavioral state
modulates the thresholds between these tiers, creating a feedback loop: emotional
state influences compute allocation, which influences task outcomes, which
influences emotional state.

This feedback loop is the concrete mechanism by which affect controls resource
efficiency. A Struggling agent spends more compute (lower T2 threshold -- routes
to deep reasoning sooner). A Coasting agent spends less compute (higher T2
threshold -- stays on cheap models longer).

---

## The Three-Tier Cognitive Architecture

The three-tier system is inspired by dual-process theory (Kahneman 2011) extended
with a zero-cost probe layer:

| Tier | Description | Cost | Latency | Usage |
|---|---|---|---|---|
| **T0** | 16 deterministic probes, no LLM | Zero | ~1ms | ~80% of cycles |
| **T1** | Fast model (haiku-class), shallow context | Low | ~2-5s | ~15% of cycles |
| **T2** | Full model (opus-class), deep context | High | ~10-30s | ~5% of cycles |

### Default Thresholds

Without Daimon modulation, the CascadeRouter uses fixed thresholds:

```
prediction_error < 0.2   ->  T0 (suppress, no LLM)
prediction_error < 0.6   ->  T1 (fast model)
prediction_error >= 0.6  ->  T2 (full model)
```

### Cost Differential

| Tier | Model Class | Relative Cost |
|---|---|---|
| T0 | None | 0x |
| T1 | Haiku | 1x |
| T2 | Opus | ~60x |

A 5% shift from T1 to T2 can increase total compute cost by approximately 3x.
The tier routing thresholds are economically significant -- the Daimon's modulation
directly affects the agent's burn rate.

**Citation**: FrugalGPT (Chen et al. 2023, arXiv:2305.05176) demonstrated that
cascade architectures can achieve substantial cost reduction while matching
top-model quality through intelligent routing.

---

## Daimon Modulation Mechanism

### Adjusted Thresholds per Behavioral State

```rust
fn adjusted_thresholds(state: &BehavioralState) -> TierThresholds {
    match state {
        Struggling => TierThresholds {
            t0_ceiling: 0.10,  // fewer cycles stay free
            t1_ceiling: 0.40,  // escalate to T2 sooner
        },
        Coasting => TierThresholds {
            t0_ceiling: 0.30,  // more cycles stay free
            t1_ceiling: 0.80,  // only escalate for major anomalies
        },
        Focused => TierThresholds {
            t0_ceiling: 0.25,
            t1_ceiling: 0.70,
        },
        Exploring => TierThresholds {
            t0_ceiling: 0.15,  // slightly lower
            t1_ceiling: 0.55,  // research benefits from T2
        },
        Resting => TierThresholds {
            t0_ceiling: 0.20,
            t1_ceiling: 0.90,  // almost never T2 during maintenance
        },
        Engaged => TierThresholds {
            t0_ceiling: 0.20,
            t1_ceiling: 0.60,
        },
    }
}
```

### Expected Tier Distributions

| State | T0 % | T1 % | T2 % | Relative Cost |
|---|---|---|---|---|
| Engaged | 80% | 15% | 5% | 1.0x (baseline) |
| Struggling | 60% | 25% | 15% | ~3.5x |
| Coasting | 90% | 8% | 2% | ~0.4x |
| Focused | 85% | 12% | 3% | ~0.6x |
| Exploring | 70% | 20% | 10% | ~2.2x |
| Resting | 80% | 19% | 1% | ~0.5x |

**Key insight**: Over a typical work session, an agent that transitions through
all states averages roughly 1.0x cost because expensive Struggling phases are
balanced by cheap Coasting and Resting phases. The affect system provides
**automatic cost regulation**.

---

## The Feedback Loop

### Closed-Loop Dynamics

```
Affect state -> Threshold modulation -> Tier selection -> Model quality -> Task outcome
      ^                                                                         |
      +---------------------------- Appraisal ---------------------------------+
```

**Positive feedback (self-correcting)**: A Struggling agent routes more to T2.
Stronger models produce better outcomes. Better outcomes increase pleasure and
confidence. The agent transitions from Struggling to Engaged and reduces T2 usage.

**Negative feedback (self-regulating)**: A Coasting agent routes almost everything
to T0/T1. Cheaper models may produce lower-quality results on harder tasks. Lower
quality decreases pleasure. The agent transitions from Coasting to Engaged,
restoring default thresholds.

### Stability Analysis

The feedback loop is stable because:

1. **Decay provides a restoring force**: PAD values decay toward zero with a
   4-hour half-life. Without reinforcing events, every state returns to Engaged.

2. **Model quality has diminishing returns**: Promoting from haiku to sonnet
   produces a larger improvement than promoting from sonnet to opus for most tasks.
   This prevents runaway compute escalation.

3. **Tier thresholds are bounded**: Even a Struggling agent still routes 60% of
   cycles to T0. Even a Coasting agent still uses T2 for 2% of cycles.

4. **Asymmetric appraisal prevents oscillation**: Gate failures produce 2x the
   pleasure impact of gate passes. The agent cannot rapidly oscillate between
   Struggling and Coasting.

---

## TierBias Integration

The behavioral state applies a bias to the CascadeRouter:

```rust
fn apply_tier_bias(router: &mut CascadeRouter, state: BehavioralState) {
    let bias = match state {
        Struggling => TierBias { t0_delta: -0.1, t1_delta: -0.2 },
        Coasting   => TierBias { t0_delta:  0.1, t1_delta:  0.2 },
        Focused    => TierBias { t0_delta:  0.1, t1_delta:  0.0 },
        _          => TierBias::ZERO,
    };
    router.set_tier_bias(bias);
}
```

The CascadeRouter considers four signals:
1. Prediction error from probes (primary)
2. Daimon-adjusted thresholds (behavioral bias)
3. Task features (domain, complexity)
4. Historical model performance (LinUCB posterior)

The Daimon bias shifts thresholds, but the CascadeRouter can override if its
bandit model strongly suggests a different tier.

---

## Model Promotion/Demotion

Within a tier, the Daimon also affects specific model selection:

```rust
fn promote_model(model: &str) -> String {
    if model.contains("haiku") { model.replacen("haiku", "sonnet", 1) }
    else if model.contains("sonnet") { model.replacen("sonnet", "opus", 1) }
    else { model.to_string() }
}

fn demote_model(model: &str) -> String {
    if model.contains("opus") { model.replacen("opus", "sonnet", 1) }
    else if model.contains("sonnet") { model.replacen("sonnet", "haiku", 1) }
    else { model.to_string() }
}
```

This is a coarse heuristic operating on model name strings. The CascadeRouter's
bandit model provides fine-grained selection within the promoted/demoted tier.

---

## Turn Limit Modulation

| State | Turn Limit Adjustment | Rationale |
|---|---|---|
| Struggling (Escalating) | +10 turns | More attempts with the stronger model |
| Struggling (Conservative) | -3 turns | Fail fast on a failing approach |
| Coasting | -5 turns | Tasks are easier -- complete faster |
| Resting | +5 turns | Maintenance tasks can take longer |
| Engaged | 0 (default) | Standard turn budget |

The Struggling/Escalating combination (+10 turns with a promoted model) is the
most expensive configuration -- reserved for situations where the agent is
genuinely stuck.

---

## Cost Impact Summary

| Transition | Cost Change | When It Happens |
|---|---|---|
| Engaged -> Struggling | ~3.5x increase | Sustained failures lower confidence below 0.30 |
| Engaged -> Coasting | ~0.4x decrease | Sustained successes raise pleasure above 0.35 |
| Engaged -> Resting | ~0.5x decrease | Low arousal during idle periods |
| Struggling -> Engaged | ~0.3x decrease | Recovery through successful outcomes |
| Coasting -> Engaged | ~2.5x increase | Encountering harder problems |

---

## Academic Foundations

- Kahneman, D. (2011). *Thinking, Fast and Slow*. Farrar, Straus and Giroux.
- Chen, L. et al. (2023). "FrugalGPT." arXiv:2305.05176.
- Li, L. et al. (2010). "A contextual-bandit approach to personalized news article
  recommendation." *WWW*, 661-670.
- Mehrabian, A. (1996). *Current Psychology*, 14(4), 261-292.

---

## Cross-References

- `six-behavioral-states.md` -- behavioral state definitions and PAD thresholds
- `integration-points.md` -- how tier routing connects with VCG bidding and dispatch
- `energy-accounting.md` -- cognitive energy budget interacts with tier selection
- `daimon-state-and-affect-engine.md` -- the DaimonState modulate() method
