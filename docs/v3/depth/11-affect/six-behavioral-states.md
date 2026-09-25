# Six Behavioral States

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/04-six-behavioral-states.md`

---

## Overview

The PAD vector is a continuous three-dimensional signal. But agents need discrete
decisions: which model tier to use, how aggressively to explore, whether to escalate
or conserve. The six behavioral states bridge the continuous affect space and the
discrete decision space. Each state is a named region of PAD space with a specific
behavioral profile -- a set of parameters that modulate tier routing, exploration
rate, retry limits, and proactive maintenance.

The critical design constraint is **cyclicality**. There is no terminal state. An
agent in the Struggling state will eventually recover through successful task
outcomes or through dream depotentiation. An agent in the Coasting state will
eventually encounter a harder problem. The state machine is a loop, not a directed
graph with a sink node.

---

## The Six States

| State | PAD Profile | Description |
|---|---|---|
| **Engaged** | Balanced (near origin) | Normal operation -- making progress at a sustainable rate |
| **Struggling** | Low P, High A | Failing and under pressure -- gate failures, blocked tasks |
| **Coasting** | High P, Low A | Succeeding without difficulty -- routine tasks, clean passes |
| **Exploring** | Low D | Unfamiliar territory -- low confidence regardless of other signals |
| **Focused** | High D, High P | Succeeding in well-understood territory -- exploit mode |
| **Resting** | Low A, Low D | Idle or low-demand phase -- time for offline learning |

### Classification Algorithm

The behavioral state is computed by `BehavioralState::classify(pad, confidence)`
and stored on `AffectState`:

```rust
pub fn classify(pad: PadVector, confidence: f64) -> BehavioralState {
    let p = pad.pleasure;
    let a = pad.arousal;
    let d = pad.dominance;
    let c = confidence.clamp(0.0, 1.0);

    if pad == PadVector::neutral() { return BehavioralState::Engaged; }

    // Struggling: clearly failing under pressure
    if c < 0.30 || d < -0.25 || (p < -0.30 && a > 0.30) {
        return BehavioralState::Struggling;
    }
    // Coasting: succeeding without effort
    if p > 0.35 && c > 0.65 { return BehavioralState::Coasting; }
    // Focused: high confidence, high pleasure -- exploit mode
    if d > 0.30 && p > 0.25 { return BehavioralState::Focused; }
    // Resting: low urgency -- maintenance mode
    if a < -0.20 { return BehavioralState::Resting; }
    // Exploring: low dominance but not failing
    if d < 0.10 && p > -0.20 { return BehavioralState::Exploring; }
    // Default
    BehavioralState::Engaged
}
```

**Priority order**: Struggling is checked first because protective measures should
not be delayed. Invalid PAD values (NaN/Inf) default to Engaged as a safe fallback.

---

## Behavioral Modulation Parameters

### Engaged (Default)

```rust
strategy:                Balanced
exploration_rate:        0.20
prefer_proven_playbooks: true
model_tier_escalation:   0
extra_retries:           0
trigger_dream_cycles:    false
run_maintenance_tasks:   false
```

20% exploration means one in five strategy choices tries something new. No model
escalation. No extra retries.

### Struggling (Conservative / Escalating)

Two sub-profiles depending on the specific PAD signature:

**Low confidence (C < 0.30) or very low dominance (D < -0.25)**:

```rust
strategy:   Escalating
turn_limit: +10 turns
model:      promote (haiku -> sonnet -> opus)
```

**Low pleasure with high arousal (P < -0.30, A > 0.30)**:

```rust
strategy:   Conservative
turn_limit: -3 turns
model:      demote (opus -> sonnet -> haiku)
```

The choice depends on the failure mode. Frustrated-and-fighting (-P, +A, +D) uses
Escalating -- the agent believes it can solve the problem but needs more resources.
Anxious-and-unsure (-P, +A, -D) uses Conservative -- the agent is failing and does
not know why, so it falls back to known-good approaches.

### Coasting (Exploratory)

```rust
strategy:   Exploratory
turn_limit: -5 turns
model:      demote
exploration_rate: 0.35
```

When things are going well, use cheaper models, reduce turn limits, and increase
exploration to 35%. Successful streaks pay for experimentation.

### Exploring

Triggered by low dominance (D < 0.10) regardless of pleasure or arousal. Maps to
Balanced defaults with T2 routing for research tasks and T1 for breadth queries.

### Focused

High dominance with high pleasure. Exploit known patterns with cheap models.
Maximum speed, reduced overhead, cached strategies preferred.

### Resting (Proactive)

```rust
strategy:                Proactive
turn_limit:              +5 turns
trigger_dream_cycles:    true
run_maintenance_tasks:   true
exploration_rate:        0.25
```

No urgent work. Time for dream cycles, knowledge pruning, index rebuilding.

---

## Dispatch Strategy Labels

Each state maps to a dispatch strategy with an effort label for cost tracking:

```rust
pub enum DispatchStrategy {
    Conservative,   // effort: "low"
    Balanced,       // effort: "medium"
    Exploratory,    // effort: "medium"
    Escalating,     // effort: "high"
    Proactive,      // effort: "medium"
}
```

The effort label is written to `.roko/learn/efficiency.jsonl` for cost-outcome
correlation analysis.

---

## Cyclicality: No Terminal State

```
           +----------------------------------+
           |                                  |
    Engaged --> Struggling --> Resting         |
       ^            |              |          |
       |            v              v          |
    Focused <-- Exploring    (Dream cycles)   |
       ^                           |          |
       |                           v          |
       +-------- Coasting <--------+          |
                    |                          |
                    +--------------------------+
```

**Common transition patterns**:

1. **Recovery from struggle**: Struggling -> Resting -> Exploring -> Engaged
2. **Performance optimization**: Engaged -> Focused -> Coasting
3. **Challenge encounter**: Coasting -> Engaged -> Struggling
4. **Knowledge plateau**: Focused -> Exploring

Every state can transition to every other state through intermediate PAD changes.
No state is absorbing. The decay mechanism ensures extreme states eventually
moderate.

---

## Threshold Calibration

The thresholds derive from appraisal rule magnitudes:

- Single task failure: P: -0.20, D: -0.15, C: -0.15
- Starting from neutral confidence 0.70, two failures push to 0.40 (above threshold)
- Three consecutive failures push to 0.25 (below 0.30 threshold -> Struggling)

```
confidence_threshold = 0.70 - (2.5 x 0.15) = 0.30
dominance_threshold  = 0.00 - (2.0 x 0.15) = -0.25
```

### Hysteresis

Split thresholds with a dead zone prevent oscillation:

| State | Entry Threshold | Exit Threshold |
|---|---|---|
| Struggling (confidence) | < 0.30 | > 0.40 |
| Struggling (dominance) | < -0.25 | > -0.15 |
| Coasting (pleasure) | > 0.35 | < 0.25 |
| Resting (arousal) | < -0.20 | > -0.10 |

An agent must improve confidence by a meaningful margin before exiting Struggling.

### Dwell Time Minimum

Even with hysteresis, rapid PAD swings can produce state changes every few ticks.
A minimum dwell time of 10 ticks prevents flickering while keeping the system
responsive to sustained changes.

---

## Full State Transition Table

| From | To | Trigger Condition | Typical Cause |
|---|---|---|---|
| Engaged | Struggling | C < 0.30 OR D < -0.25 | 3+ consecutive failures |
| Engaged | Coasting | P > 0.35 AND C > 0.65 | Sustained easy successes |
| Engaged | Focused | D > 0.30 AND P > 0.25 | Success in familiar territory |
| Engaged | Resting | A < -0.20 | No tasks in queue |
| Engaged | Exploring | D < 0.10 AND P > -0.20 | New crate, unfamiliar API |
| Struggling | Engaged | C > 0.40 AND D > -0.15 | Successful task after struggle |
| Struggling | Resting | A < -0.20 | Arousal decay over time |
| Coasting | Engaged | P < 0.25 OR C < 0.65 | Harder problem encountered |
| Coasting | Struggling | C < 0.30 OR D < -0.25 | Sudden failure on "easy" task |
| Focused | Engaged | D < 0.30 OR P < 0.25 | Exhausted playbooks |
| Focused | Coasting | P > 0.35 AND C > 0.65 | Continued success |
| Focused | Struggling | C < 0.30 OR D < -0.25 | Unexpected failure |
| Resting | Engaged | A > -0.10 | New task arrives |
| Resting | Exploring | D < 0.10 AND A > -0.10 | Task in unfamiliar area |
| Exploring | Engaged | D > 0.10 OR P < -0.20 | Gained familiarity or failed |
| Exploring | Focused | D > 0.30 AND P > 0.25 | Built mastery |
| Exploring | Struggling | C < 0.30 OR D < -0.25 | Repeated exploration failure |

---

## Academic Foundations

- Mehrabian, A. (1996). *Current Psychology*, 14(4), 261-292.
- Russell, J.A. & Mehrabian, A. (1977). *Journal of Research in Personality*, 11(3).
- Gebhard, P. (2005). *AAMAS*, 29-36.
- Chen, L. et al. (2023). "FrugalGPT." arXiv:2305.05176.

---

## Cross-References

- `pad-vector.md` -- PAD vector structure and octant classification
- `alma-three-layer-temporal.md` -- temporal dynamics of state changes
- `behavioral-state-to-tier-routing.md` -- how states modulate model selection
- `integration-points.md` -- how states connect to dispatch, routing, and VCG
