# PAD Vector: The 3D Affect Coordinate System

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- v1 source: `docs/v1/09-daimon/01-pad-vector.md`

---

## Overview

The PAD (Pleasure-Arousal-Dominance) model is a three-dimensional framework for
representing emotional states. Originally developed by Mehrabian and Russell (1977)
and refined by Mehrabian (1996), it provides the mathematical foundation for the
Daimon affect engine. Each dimension occupies a continuous range of [-1.0, 1.0],
and the sign combination of all three dimensions defines one of 8 octant states.

For Roko agents, the PAD vector is not an emotional display -- it is a **control
signal**. The pleasure dimension tracks whether recent actions produce good outcomes.
The arousal dimension tracks urgency and cognitive load. The dominance dimension
tracks confidence in the current approach. Together, these three numbers control
which model is called, how many turns are allocated, whether the agent explores or
exploits, and whether it re-plans or persists.

---

## Theoretical Foundation

### Why PAD Over Alternatives?

The PAD model was chosen over several alternatives after evaluating four criteria:
continuous representation (gradual changes, not discrete jumps), orthogonal dimensions
(changes in one dimension do not force changes in others), computational efficiency
(three `f64` values, no embedding lookups), and bidirectional mapping to discrete
emotion labels when human-readable output is needed.

| Alternative | Limitation | Decision |
|---|---|---|
| Discrete emotion labels (Ekman, Plutchik) | Boundary problems: is this "fear" or "anxiety"? | Rejected as primary; used for display via octant mapping |
| Russell's Circumplex (2D: Valence x Arousal) | Lacks Dominance -- cannot distinguish "failing and I know what to do" from "failing and helpless" | Rejected -- Dominance is essential |
| Scherer's Component Process Model (5+ dimensions) | Computationally expensive; each event requires 5+ evaluations | Rejected as primary; used for appraisal rule design |
| Big Five personality dimensions | Measures stable traits, not transient states | Wrong timescale -- personality is Layer 3, not Layer 1 |

### Mehrabian (1996)

Mehrabian's key contribution in the 1996 paper ("Pleasure-arousal-dominance: A general
framework for describing and measuring individual differences in temperament,"
*Current Psychology*, 14(4), 261-292) was demonstrating that three dimensions explain
the variance in emotional experience more parsimoniously than discrete emotion
taxonomies. The three dimensions emerged from factor analysis of emotional response
scales and map consistently across cultures and stimulus types.

### Russell & Mehrabian (1977)

Russell and Mehrabian's 1977 paper ("Evidence for a three-factor theory of emotions,"
*Journal of Research in Personality*, 11, 273-294) provided the original empirical
evidence that emotional states cluster along three orthogonal factors. Their factor
analysis of semantic differential scales revealed three stable factors corresponding
to pleasure (evaluation), arousal (activity), and dominance (potency) -- the same
three factors identified independently by Osgood's semantic differential research.

---

## The Three Dimensions

### Pleasure [-1.0, 1.0]

Pleasure captures the **outcome quality trajectory** -- is the agent succeeding or
failing?

| Value Range | Agent State | Concrete Triggers |
|---|---|---|
| [0.6, 1.0] | Strong success trajectory | Multiple consecutive gate passes, tasks completing on first try |
| [0.2, 0.6] | Moderate success | Gate passes at moderate rungs, tasks completing with some iteration |
| [-0.2, 0.2] | Neutral | Mixed results, no clear trend |
| [-0.6, -0.2] | Moderate difficulty | Gate failures, tasks requiring multiple retries |
| [-1.0, -0.6] | Strong failure trajectory | Consecutive gate failures, tasks timing out, repeated errors |

**Appraisal rules** (from `roko-daimon`, `AffectEngine::appraise()`):

```
Gate pass:    pleasure += 0.05 * rung_scale
Gate fail:    pleasure -= 0.10 * rung_scale
Task success: pleasure += 0.10
Task failure: pleasure -= 0.20
```

The asymmetry (failure has 2x the pleasure impact of success) reflects prospect
theory (Kahneman & Tversky 1979): losses loom larger than gains. For agents, this
means a single failure is more disruptive than a single success is encouraging,
which matches engineering reality where a broken build demands more attention than
a clean build.

### Arousal [-1.0, 1.0]

Arousal captures **cognitive load and urgency** -- how much compute should the agent
invest in each decision?

| Value Range | Agent State | Concrete Triggers |
|---|---|---|
| [0.6, 1.0] | High urgency | Approaching deadlines, multiple blockers, consecutive failures |
| [0.2, 0.6] | Elevated load | Some time pressure, moderate complexity |
| [-0.2, 0.2] | Normal load | Routine tasks, no unusual pressure |
| [-0.6, -0.2] | Low load | Idle time, routine maintenance |
| [-1.0, -0.6] | Minimal load | No active tasks, consolidation opportunity |

**Appraisal rules**:

```
Time pressure (proximity in [0.0, 1.0]):  arousal += proximity * 0.40
Blocked (1-5 blockers):                   arousal += blockers * 0.05
Queue wait (>24 hours):                   arousal += scaled ramp 0.0 to 1.0 over 7 days
Gate fail:                                arousal += 0.04 * rung_scale
```

The arousal dimension is the primary input to the tier routing bias. High arousal
causes the agent to route to stronger models sooner. Low arousal keeps the agent
on cheaper T0/T1 models.

### Dominance [-1.0, 1.0]

Dominance captures **confidence in the current approach** -- does the agent feel in
control of the situation?

| Value Range | Agent State | Concrete Triggers |
|---|---|---|
| [0.6, 1.0] | High confidence | Known patterns, successful track record on this crate/task type |
| [0.2, 0.6] | Moderate confidence | Familiar territory with some uncertainty |
| [-0.2, 0.2] | Neutral | No strong signal about approach quality |
| [-0.6, -0.2] | Low confidence | Unfamiliar territory, novel APIs, first encounter with this code |
| [-1.0, -0.6] | Very low confidence | Repeated failures, blocked, no clear path forward |

**Appraisal rules**:

```
Gate pass:                     dominance += 0.03 * rung_scale
Gate fail:                     dominance -= 0.08 * rung_scale
Task success:                  dominance += 0.10
Task failure:                  dominance -= 0.15
Blocked (1-5 blockers):        dominance -= blockers * 0.08
```

Dominance drives the exploration/exploitation balance. Low dominance causes the
agent to explore (try new approaches, research mode, broader context retrieval).
High dominance causes the agent to exploit (use cached strategies, known patterns,
minimal context).

---

## The 8 Octant States

The sign of each PAD dimension defines one of eight octant states. These labels
provide human-readable names for dashboard display and logging, while the continuous
PAD values drive the actual behavioral modulation.

| Octant | P | A | D | Label | Agent Meaning | Behavioral Bias |
|---|---|---|---|---|---|---|
| +P+A+D | + | + | + | **Exuberant / Excited** | Succeeding under pressure, high confidence | Exploit aggressively, fast execution |
| +P+A-D | + | + | - | **Dependent / Surprised** | Unexpected success, not sure why it worked | Cautious continuation, seek understanding |
| +P-A+D | + | - | + | **Relaxed / Confident** | Calm, in control, succeeding | Steady execution, consider exploration |
| +P-A-D | + | - | - | **Docile / Relaxed** | Nothing urgent, things are fine | Low initiative, follow existing plans |
| -P+A+D | - | + | + | **Hostile / Angry** | Frustrated but still trying, attribution external | Escalate model, persist harder, add retries |
| -P+A-D | - | + | - | **Anxious** | Failing, pressured, no control | Conservative, proven playbooks, low exploration |
| -P-A+D | - | - | + | **Disdainful / Bored** | Nothing happening, agent idle | Proactive maintenance, dream cycles |
| -P-A-D | - | - | - | **Depressed / Bored** | Repeated failures, no agency | Trigger re-plan, escalate to stronger model |

The exact-zero vector (P=0, A=0, D=0) defaults to `Relaxed` for dashboard
readability at agent startup.

### Plutchik Emotion Mapping

Plutchik (1980) defined eight primary emotions in bipolar pairs. The PAD octants
map to Plutchik categories for human-readable logging:

| PAD Octant | Primary Plutchik Emotion | Intensity Variants |
|---|---|---|
| +P+A+D (Exuberant) | Joy | Ecstasy -> Joy -> Serenity |
| -P+A-D (Anxious) | Fear | Terror -> Fear -> Apprehension |
| -P+A+D (Hostile) | Anger | Rage -> Anger -> Annoyance |
| +P-A+D (Confident) | Trust | Admiration -> Trust -> Acceptance |
| -P-A-D (Depressed) | Sadness | Grief -> Sadness -> Pensiveness |
| +P+A-D (Surprised) | Surprise | Amazement -> Surprise -> Distraction |
| -P-A+D (Bored) | Disgust | Loathing -> Disgust -> Boredom |
| +P-A-D (Docile) | Anticipation | Vigilance -> Anticipation -> Interest |

The mapping is approximate -- Plutchik's model and Mehrabian's were developed
independently. The PAD values, not the Plutchik labels, drive all behavioral
modulation.

---

## Rust Implementation

### PadVector Struct (roko-core)

The shared canonical `PadVector` lives in `crates/roko-core/src/affect.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PadVector {
    pub pleasure: f64,   // [-1.0, 1.0]
    pub arousal: f64,    // [-1.0, 1.0]
    pub dominance: f64,  // [-1.0, 1.0]
}

impl PadVector {
    pub const fn new(pleasure: f64, arousal: f64, dominance: f64) -> Self { ... }
    pub const fn neutral() -> Self { ... }
    pub fn clamped(self) -> Self { ... }
    pub fn apply_delta(&mut self, pleasure: f64, arousal: f64, dominance: f64) { ... }
    pub fn decay_by_factor(&mut self, factor: f64) { ... }
}
```

### AffectOctant Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AffectOctant {
    Excited,    // +P+A+D
    Surprised,  // +P+A-D
    Confident,  // +P-A+D
    Relaxed,    // +P-A-D
    Angry,      // -P+A+D
    Anxious,    // -P+A-D
    Bored,      // -P-A+D
    Depressed,  // -P-A-D
}

impl AffectOctant {
    pub const fn from_pad(pleasure: f64, arousal: f64, dominance: f64) -> Self {
        if pleasure == 0.0 && arousal == 0.0 && dominance == 0.0 {
            return Self::Relaxed;
        }
        let pp = !pleasure.is_sign_negative();
        let pa = !arousal.is_sign_negative();
        let pd = !dominance.is_sign_negative();
        match (pp, pa, pd) {
            (true, true, true)   => Self::Excited,
            (true, true, false)  => Self::Surprised,
            (true, false, true)  => Self::Confident,
            (true, false, false) => Self::Relaxed,
            (false, true, true)  => Self::Angry,
            (false, true, false) => Self::Anxious,
            (false, false, true) => Self::Bored,
            (false, false, false) => Self::Depressed,
        }
    }
}
```

---

## PAD Cosine Similarity

For mood-congruent memory retrieval and somatic landscape queries, PAD similarity
is computed as cosine similarity mapped to [0.0, 1.0]:

```rust
pub fn pad_cosine_similarity(a: &PadVector, b: &PadVector) -> f64 {
    let dot = a.pleasure * b.pleasure
        + a.arousal * b.arousal
        + a.dominance * b.dominance;
    let mag_a = (a.pleasure.powi(2) + a.arousal.powi(2) + a.dominance.powi(2)).sqrt();
    let mag_b = (b.pleasure.powi(2) + b.arousal.powi(2) + b.dominance.powi(2)).sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.5; // Neutral mood -> middle similarity
    }
    (dot / (mag_a * mag_b) + 1.0) / 2.0 // Map [-1, 1] -> [0, 1]
}
```

Why cosine instead of Euclidean: Euclidean distance conflates emotional direction
with intensity. Mild anxiety (P:-0.2, A:+0.1, D:-0.1) and strong anxiety
(P:-0.8, A:+0.5, D:-0.4) have high Euclidean distance (0.74) but high cosine
similarity (0.99). Both are anxious -- memories encoded under either state are
relevant to the other. Cosine captures the quality of emotion, not the magnitude.

---

## Decay Toward Baseline

The PAD vector decays toward neutral [0, 0, 0] with a configurable half-life
(default: 4 hours). This prevents permanent affect drift.

```
factor = 0.5 ^ (elapsed_hours / half_life_hours)
pad.pleasure  *= factor
pad.arousal   *= factor
pad.dominance *= factor
```

After 1 half-life (4h): 50% intensity. After 2 half-lives (8h): 25%. After 4
half-lives (16h): 6.25%. This matches the ALMA model's mood layer temporal
dynamics (see `alma-three-layer-temporal.md`).

Confidence decays toward 0.5 (neutral), not toward 0.0:

```
confidence = 0.5 + (confidence - 0.5) * factor
```

This ensures that an agent with no recent events settles at "uncertain" rather
than "no confidence."

---

## The Geometric Interpretation

The PAD space is a unit cube from [-1, -1, -1] to [+1, +1, +1]. Every possible
emotional state is a point in this cube. The origin (0, 0, 0) is the neutral
baseline. Movement away from the origin in any direction represents emotional
activation.

Key geometric properties:

- **Distance from origin**: emotional intensity (sqrt(P^2 + A^2 + D^2), max sqrt(3))
- **Direction from origin**: emotional quality (which octant, what kind of emotion)
- **Angle between two vectors**: emotional similarity (cosine similarity)
- **Octant membership**: discrete state classification (sign combination)

The decay function pulls the state vector toward the origin at a rate proportional
to its distance from origin. This means intense emotions decay faster in absolute
terms than mild ones, but all emotions reach the same fraction of their peak after
the same elapsed time (exponential decay is scale-invariant).

---

## Academic Foundations

- Mehrabian, A. (1996). "Pleasure-arousal-dominance: A general framework for
  describing and measuring individual differences in temperament."
  *Current Psychology*, 14(4), 261-292.
- Russell, J.A. & Mehrabian, A. (1977). "Evidence for a three-factor theory of
  emotions." *Journal of Research in Personality*, 11, 273-294.
- Plutchik, R. (1980). *Emotion: A Psychoevolutionary Synthesis*. Harper & Row.
- Kahneman, D. & Tversky, A. (1979). "Prospect Theory: An Analysis of Decision
  under Risk." *Econometrica*, 47(2), 263-291.

---

## Cross-References

- `alma-three-layer-temporal.md` -- three-layer temporal model for PAD dynamics
- `occ-scherer-appraisal.md` -- appraisal rules that produce PAD deltas
- `six-behavioral-states.md` -- PAD to behavioral state mapping
- `mood-congruent-memory.md` -- PAD cosine similarity in retrieval scoring
- `somatic-markers-damasio.md` -- PAD vs. situation-specific somatic markers
