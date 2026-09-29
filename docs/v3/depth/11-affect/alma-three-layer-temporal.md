# ALMA Three-Layer Temporal Model

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/02-alma-three-layer-temporal.md`

---

## Overview

The ALMA (A Layered Model of Affect) architecture, developed by Gebhard (2005),
provides a three-layer temporal model for affect processing. In the Daimon, these
three layers operate at different timescales and serve different cognitive functions:

| Layer | Timescale | Function | Implementation |
|---|---|---|---|
| **Emotion** | Seconds | Immediate reaction to discrete events | PAD delta from appraisal rules |
| **Mood** | Hours | Accumulated emotional trajectory | EMA-smoothed PAD vector in `AffectState` |
| **Personality** | Lifetime | Stable baseline attractor | Neutral [0, 0, 0] (configurable per agent) |

This three-layer decomposition prevents two failure modes: **emotional volatility**
(if only the emotion layer existed, the agent would whipsaw between states on every
event) and **emotional inertia** (if only the mood layer existed, the agent would
respond too slowly to urgent events). The emotion layer provides fast reactivity,
the mood layer provides stability, and the personality layer provides a gravitational
center that prevents permanent drift.

---

## Theoretical Foundation: Gebhard (2005)

Gebhard's ALMA model ("ALMA -- A Layered Model of Affect," *Proceedings of the
Fourth International Joint Conference on Autonomous Agents and Multiagent Systems
(AAMAS)*, 2005, pp. 29-36) proposed that affective processing in virtual agents
should operate across three temporal layers, each with distinct dynamics:

1. **Emotions** are short-lived, intense, object-directed responses triggered by
   specific events. They have rapid onset and rapid decay.
2. **Moods** are longer-lasting, lower-intensity, diffuse states that bias
   perception and behavior over extended periods. They accumulate from repeated
   emotions and decay slowly.
3. **Personality** represents stable individual differences that determine the
   baseline mood and the sensitivity to different emotion-triggering events.

Gebhard demonstrated that this layered approach produces more naturalistic and
coherent affective behavior than single-layer models, because it captures both
the fast reactivity of emotions and the sustained influence of moods.

In Roko's implementation, the ALMA layers map as follows: the emotion layer is the
delta applied to the PAD vector in each appraisal event, the mood layer is the
EMA-smoothed PAD vector stored in `AffectState`, and the personality layer is the
neutral baseline that the mood decays toward.

---

## Layer 1: Emotion (Reactive, Seconds-Scale)

The emotion layer captures **immediate reactions to discrete events**. Each event
triggers an appraisal (see `occ-scherer-appraisal.md`) that produces a PAD delta --
a signed change vector applied to the current mood state.

### Characteristics

| Property | Value | Rationale |
|---|---|---|
| **Timescale** | Seconds to minutes | Single task outcome, single gate evaluation |
| **Trigger** | Discrete `AffectEvent` | Every event is grounded in a concrete metric |
| **Duration** | Immediate | Applied as delta, then absorbed into mood layer |
| **Decay** | Not applicable -- absorbed into mood | Emotions do not persist independently |
| **Function** | Fast reactivity to changing conditions | Ensures the agent responds to urgent events |

### Appraisal Deltas

Every `AffectEvent` maps to a specific PAD delta. These deltas are the emotion
layer -- the immediate emotional response to what just happened:

| Event | P delta | A delta | D delta | Conf delta | Character |
|---|---|---|---|---|---|
| Gate pass (rung r) | +0.05 x rs | -0.01 x rs | +0.03 x rs | +0.03 x rs | Satisfaction, slight relief |
| Gate fail (rung r) | -0.10 x rs | +0.04 x rs | -0.08 x rs | -0.08 x rs | Disappointment, increased urgency |
| Task success | +0.10 | 0.00 | +0.10 | +0.08 | Achievement, confidence boost |
| Task failure | -0.20 | 0.00 | -0.15 | -0.15 | Significant setback |
| Blocked (n blockers) | 0.00 | +n x 0.05 | -n x 0.08 | -0.02 x n | Frustration, loss of control |
| Time pressure (prox) | 0.00 | +prox x 0.40 | 0.00 | 0.00 | Pure urgency signal |
| Queue wait (>24h) | 0.00 | scaled | 0.00 | 0.00 | Increasing urgency for stale work |
| Dream failure | 0.00 | 0.00 | 0.00 | -0.07 x n | Confidence erosion from pattern review |

Where `rs = 1.0 + min(rung, 3) x 0.15` is a rung scale factor that makes
higher-rung gate results more emotionally significant.

### Grounded Appraisal Constraint

Every emotion in the Daimon has a trigger, and every trigger is grounded in a
concrete metric. No emotion is generated without a triggering event. This is the
central design constraint from OCC theory (Ortony, Clore, & Collins 1988):
emotions are appraisals of events relative to goals, not random fluctuations.

This constraint prevents **affective hallucination** -- the risk that an agent
"feels" something without justification. The Daimon avoids this by requiring every
PAD update to trace back to a specific `AffectEvent` with measurable inputs.

---

## Layer 2: Mood (Accumulated, Hours-Scale)

The mood layer captures **accumulated emotional trajectory** over hours. It is the
EMA-smoothed PAD vector -- the "how have things been going?" signal that drives
behavioral state classification.

### Characteristics

| Property | Value | Rationale |
|---|---|---|
| **Timescale** | Hours | Captures multi-task trajectory |
| **Update mechanism** | Each emotion delta absorbed into current PAD | Smooth accumulation, no sudden jumps |
| **Decay** | Exponential toward personality baseline, half-life 4 hours | Prevents permanent affect drift |
| **Persistence** | Survives agent restart (`.roko/daimon/affect.json`) | Agent "wakes up" with residual mood |
| **Function** | Stable behavioral state classification | Determines behavioral state |

### Mood Update Rule

When an emotion delta is applied, the mood layer absorbs it through addition with
clamping:

```
mood.pleasure  = clamp(mood.pleasure  + delta.pleasure,  -1.0, 1.0)
mood.arousal   = clamp(mood.arousal   + delta.arousal,   -1.0, 1.0)
mood.dominance = clamp(mood.dominance + delta.dominance, -1.0, 1.0)
```

This is equivalent to an EMA with alpha = 1.0 for the delta (full immediate
impact) combined with exponential decay over time. The decay provides the smoothing:

```
After 4 hours with no events:  mood.pleasure *= 0.5  (halved)
After 8 hours with no events:  mood.pleasure *= 0.25 (quartered)
```

### Mood Sampling Stability

The mood state requires a minimum sample count (10 appraisal events) before the
mood classification is considered meaningful. Before the minimum sample count is
reached, the agent uses its personality baseline as the mood state.

This prevents early-life transient emotions from triggering behavioral changes
before the EMA has stabilized. An agent that fails its very first task should not
immediately enter the Struggling state -- it needs enough history for the mood
trajectory to be meaningful.

### Mood Persistence

The mood layer persists to disk at `.roko/daimon/affect.json`:

```json
{
  "state": {
    "pad": { "pleasure": -0.15, "arousal": 0.22, "dominance": -0.08 },
    "confidence": 0.42,
    "updated_at": "2026-04-12T14:30:00Z"
  },
  "half_life_hours": 4.0
}
```

When the agent restarts, it loads the persisted mood and applies decay for the
elapsed time since `updated_at`. An agent shut down 8 hours ago in a negative
mood will resume with that mood at 25% intensity -- enough residual context to
remember "yesterday was rough" without being trapped in yesterday's state.

---

## Layer 3: Personality (Stable, Lifetime-Scale)

The personality layer provides the **baseline attractor** that the mood decays
toward. In the current implementation, personality is the neutral vector [0, 0, 0].
Future extensions could configure per-agent personality profiles.

### Characteristics

| Property | Value | Rationale |
|---|---|---|
| **Timescale** | Agent lifetime | Does not change during operation |
| **Configuration** | Currently neutral [0, 0, 0] | Future: per-agent profiles in `roko.toml` |
| **Function** | Gravitational center for mood decay | Ensures long-term emotional stability |
| **Biological analog** | Big Five traits (Costa & McCrae 1992) | Stable individual differences |

### Personality as Baseline Disposition

The personality baseline maps to the Dominance axis:

- **High baseline dominance**: agent defaults to confident, exploratory behavior
- **Low baseline dominance**: agent defaults to cautious, conservative behavior
- **Neutral baseline**: agent starts with no disposition and adapts from experience

### Future: Learned Personality

Over many tasks, an agent's personality could be learned from its long-term mood
trajectory. An agent that consistently operates in the Confident/Focused octant
could have its personality baseline shifted toward positive Dominance. This creates
a second-order learning loop: experience -> mood -> personality -> behavioral
default -> experience.

---

## Layer Interactions

The three layers interact in a specific temporal cascade:

```
Event occurs (gate fail, task success, blocker, ...)
  |
  v
Layer 1: Emotion -- compute PAD delta from appraisal rules
  |
  v
Layer 2: Mood -- apply delta to current PAD vector (with clamping)
  |                 +-----------------------------+
  +---------------->| Decay toward Layer 3 baseline|
  |                 +-----------------------------+
  v
Layer 3: Personality -- static attractor (currently [0,0,0])
```

### Temporal Dynamics Example

Consider an agent that fails three consecutive tasks, then succeeds:

```
t=0: Neutral mood [P:0.0, A:0.0, D:0.0]
t=1: Task failure -> delta [P:-0.20, A:0.00, D:-0.15]
     Mood: [P:-0.20, A:0.00, D:-0.15]  State: Anxious/Depressed region
t=2: Task failure -> delta [P:-0.20, A:0.00, D:-0.15]
     Mood: [P:-0.40, A:0.00, D:-0.30]  State: Deep Struggling
t=3: Task failure -> delta [P:-0.20, A:0.00, D:-0.15]
     Mood: [P:-0.60, A:0.00, D:-0.45]  State: Struggling, may trigger re-plan
t=4: Task success -> delta [P:+0.10, A:0.00, D:+0.10]
     Mood: [P:-0.50, A:0.00, D:-0.35]  State: Still negative, but recovering
t=5: (4 hours pass, no events) -> decay factor 0.5
     Mood: [P:-0.25, A:0.00, D:-0.175] State: Fading toward neutral
```

The key dynamics: emotion layer provides immediate reactivity (each failure has
impact), mood layer accumulates trajectory (three failures build up), and
personality-driven decay provides recovery (the agent gravitates back toward
neutral over time).

---

## Comparison with Alternative Models

### Why Not Discrete Emotion Labels?

Systems like OCC (Ortony, Clore, & Collins 1988) classify emotions into discrete
categories (joy, fear, anger). This creates boundary problems: is an event "fear"
or "anxiety"? What is the difference between "mild joy" and "satisfaction"? The
PAD model avoids boundary problems by using continuous dimensions. Discrete labels
are derived from PAD octants for human-readable output, but behavioral modulation
operates on continuous values.

### Why Not Full Appraisal-Only (Scherer)?

Scherer (2001) proposed evaluating events on multiple appraisal dimensions
(novelty, pleasantness, goal relevance, coping potential, norm compatibility) and
deriving emotion from the full appraisal profile. This is more theoretically
complete than PAD but computationally expensive. The Daimon uses a hybrid:
OCC/Scherer-inspired appraisal rules generate PAD deltas, combining theoretical
rigor with computational efficiency.

---

## Academic Foundations

- Gebhard, P. (2005). "ALMA -- A Layered Model of Affect." *Proceedings of the
  Fourth International Joint Conference on Autonomous Agents and Multiagent Systems
  (AAMAS)*, 29-36.
- Mehrabian, A. (1996). "Pleasure-arousal-dominance: A general framework."
  *Current Psychology*, 14(4), 261-292.
- Ortony, A., Clore, G.L., & Collins, A. (1988). *The Cognitive Structure of
  Emotions*. Cambridge University Press.
- Scherer, K.R. (2001). "Appraisal considered as a process of multilevel
  sequential checking." In Scherer, Schorr, & Johnstone (Eds.),
  *Appraisal Processes in Emotion*. Oxford University Press.
- Costa, P.T. & McCrae, R.R. (1992). *NEO PI-R Professional Manual*.
  Psychological Assessment Resources.

---

## Cross-References

- `pad-vector.md` -- PAD vector structure and octant classification
- `occ-scherer-appraisal.md` -- appraisal rules that generate emotion-layer deltas
- `six-behavioral-states.md` -- mood-layer to behavioral state mapping
- `energy-accounting.md` -- cognitive energy interacts with mood decay rates
