# Energy Accounting: CognitiveEnergy, RecoveryModes, and Yerkes-Dodson

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- new for v3 (E23 cognitive autonomy)

---

## Overview

Cognitive energy accounting tracks how much computational and attentional
budget the agent has available at any given moment. Unlike the PAD vector
(which tracks emotional quality and direction) or the VitalityTracker (which
tracks budget-derived lifecycle phase), the `CognitiveEnergy` pool models a
**depletable, recoverable resource** that gates what kinds of cognitive
operations the agent can attempt.

Every cognitive operation has a cost. A T0 deterministic tick costs almost
nothing (0.01). A T2 deep-reasoning tick costs fifteen times more (0.15). A
dream consolidation cycle is unusual -- it has a *negative* cost (-0.30),
meaning it restores energy. The pool tracks current energy, maximum capacity,
accumulated fatigue, and an affect-derived depletion multiplier that makes
operations more expensive when the agent is emotionally stressed.

The Yerkes-Dodson law (1908) provides the theoretical ceiling: performance
peaks at moderate arousal. Very low arousal means the agent is understimulated
and unmotivated. Very high arousal means the agent is overstimulated and can
only handle simple tasks safely. The inverted-U curve maps arousal to a
maximum complexity ceiling that constrains which tasks the agent should
attempt.

---

## Theoretical Foundation

### Yerkes-Dodson Law (1908)

Yerkes and Dodson's original finding was that the optimal arousal level for
learning depends on task difficulty. Simple tasks benefit from high arousal
(increased drive). Complex tasks are impaired by high arousal (anxiety
disrupts working memory and analytical reasoning). The relationship forms an
inverted U:

```text
Performance
  1.0  ---------.         .--------
                 \       /
  0.5             \     /
                   \   /
  0.0  ---------.--'-'--.---------
      -1.0    0.0    0.35   1.0  Arousal
```

For LLM agents, this translates directly: high arousal (from time pressure,
consecutive failures, many blockers) degrades performance on complex reasoning
tasks because the agent routes to shorter turn budgets, more conservative
strategies, and proven playbooks rather than novel analysis. The Yerkes-Dodson
ceiling prevents the agent from attempting complex work when it is
overstimulated.

**Citation**: Yerkes, R.M. & Dodson, J.D. (1908). "The relation of strength
of stimulus to rapidity of habit-formation." *Journal of Comparative Neurology
and Psychology*, 18(5), 459-482.

### Resource-Depletion Models

The cognitive energy model draws from Baumeister's ego depletion hypothesis
(Baumeister et al. 1998) -- the idea that self-regulation draws from a limited
pool that can be exhausted through use and restored through rest. While the ego
depletion literature has been contested on replication grounds, the
computational analogy is concrete and unambiguous: LLM inference has real
costs, turn budgets are finite, and sustained high-effort operation should be
balanced with recovery.

The three recovery modes (Gamma, Theta, Delta) map to the three temporal
layers of the ALMA model:

| Recovery Mode | ALMA Layer | Timescale | Recovery Amount | Fatigue Reset |
|---|---|---|---|---|
| **Gamma** | Emotion (reactive) | Seconds-minutes | +0.05 (5%) | No |
| **Theta** | Mood (accumulated) | Minutes-hours | +0.15 (15%) | No |
| **Delta** | Personality (deep) | Hours (dream cycle) | Full (100%) | Yes |

The naming follows neuroscience convention where gamma oscillations (30-100 Hz)
correspond to active cognition, theta (4-8 Hz) to relaxed attention and memory
consolidation, and delta (0.5-4 Hz) to deep sleep.

---

## CognitiveEnergy Struct

The mutable cognitive energy pool is owned by `DaimonState` and persisted
across sessions:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveEnergy {
    /// Current available energy in [0.0, max].
    pub current: f64,
    /// Maximum energy capacity (default: 1.0).
    pub max: f64,
    /// Accumulated fatigue in [0.0, 1.0].
    pub fatigue: f64,
    /// Most recent recovery cadence.
    pub recovery_mode: RecoveryMode,
    /// Affect-derived cost multiplier (default: 1.0).
    /// Higher values make all operations more expensive.
    pub depletion_multiplier: f64,
    /// How strongly fatigue amplifies depletion (default: 1.0).
    pub fatigue_intensity_factor: f64,
}
```

**Default state**: current = 1.0, max = 1.0, fatigue = 0.0,
recovery_mode = Gamma, depletion_multiplier = 1.0, fatigue_intensity_factor = 1.0.

A freshly created agent starts at full energy with no fatigue. The depletion
multiplier is 1.0 (neutral). As the agent's affect state shifts (e.g., high
arousal from stress), the multiplier can increase, making every operation more
costly -- a computational analog of cognitive load theory (Sweller 1988).

---

## EnergyActivity Costs

Every cognitive activity has a base cost drawn from the E23 cognitive-energy
specification:

```rust
pub enum EnergyActivity {
    T0Tick,          // 0.01 -- deterministic probes, no LLM
    T1Tick,          // 0.05 -- lightweight model inference
    T2Tick,          // 0.15 -- deep model inference
    ToolCall,        // 0.03 -- external tool invocation
    ContextAssembly, // 0.02 -- VCG auction and context building
    GateCheck,       // 0.04 -- corrigibility gate evaluation
    DreamCycle,      // -0.30 -- dream consolidation (RESTORES energy)
}
```

### Cost Rationale

The cost ratios reflect actual compute and latency differences:

| Activity | Base Cost | Rationale |
|---|---|---|
| T0Tick | 0.01 | Deterministic probes only, no LLM call, ~1ms |
| T1Tick | 0.05 | Fast model (haiku-class), ~2-5s, low token cost |
| T2Tick | 0.15 | Full model (opus-class), ~10-30s, high token cost |
| ToolCall | 0.03 | File system, git, or MCP invocation |
| ContextAssembly | 0.02 | VCG auction, knowledge retrieval, prompt building |
| GateCheck | 0.04 | Compile/test/clippy/diff/judge evaluation |
| DreamCycle | -0.30 | **Negative cost**: dreaming restores energy |

The dream cycle's negative cost implements the biological observation that
sleep consolidation is restorative. In the computational model, a dream cycle
reviews past episodes, consolidates patterns, and resets the agent's energy
pool -- the agent emerges with more capacity than when it started the dream.

### Effective Cost Formula

The actual energy charged is not the base cost alone. Fatigue and the
depletion multiplier amplify it:

```
effective_cost = base_cost * (1.0 + fatigue * fatigue_intensity_factor) * depletion_multiplier
```

At zero fatigue and neutral multiplier, effective cost equals base cost. At
fatigue = 0.5 and multiplier = 1.0, effective cost is 1.5x base. At fatigue =
1.0 and multiplier = 1.5, effective cost is 3.0x base. This means a fatigued,
stressed agent burns through its energy pool much faster -- exactly the
behavior we want to model.

### Fatigue Accumulation

Fatigue accumulates with each operation and decays slowly:

```
fatigue = (fatigue * 0.95 + base_cost * 0.1).clamp(0.0, 1.0)
```

The 0.95 decay factor means fatigue naturally decreases by 5% per operation
(a slow leak). The 0.1 accumulation factor means each operation adds 10% of
its base cost to fatigue. Heavy operations (T2Tick at 0.15) contribute more
fatigue than light ones (T0Tick at 0.01). Fatigue is only fully reset by
Delta recovery (dream cycles).

---

## Recovery Modes

```rust
pub enum RecoveryMode {
    Gamma,  // fast partial recovery: +0.05
    Theta,  // moderate partial recovery: +0.15
    Delta,  // full recovery with fatigue reset
}
```

### Gamma Recovery (Between Tasks)

Gamma recovery adds 0.05 to current energy, capped at max. It does **not**
reduce fatigue. This represents the brief pause between task executions --
enough to top off a small deficit but not enough to address accumulated
exhaustion.

**When it fires**: between sequential task dispatches in the plan runner. The
agent finishes one task, enters a brief idle state, and recovers a small
amount before the next dispatch.

### Theta Recovery (Extended Pause)

Theta recovery adds 0.15 to current energy, capped at max. Like Gamma, it
does not reset fatigue. This represents longer idle periods -- the agent has
been waiting for a dependency, or the plan runner has paused between waves.

**When it fires**: during extended idle periods detected by the vitality
tracker, or after a plan wave completes and before the next wave begins.

### Delta Recovery (Dream Cycle)

Delta recovery is qualitatively different: it restores energy to maximum
**and** resets fatigue to zero. This is the only way to clear accumulated
fatigue. It represents the dream consolidation cycle where the agent
processes past episodes, consolidates knowledge, and emerges refreshed.

**When it fires**: during dream consolidation (triggered by the Resting
behavioral state or by schedule). Delta recovery also applies the
affect-neutralizing effect -- PAD dimensions are multiplied by 0.7,
dampening both positive and negative extremes:

```rust
pub fn recover_cognitive_energy(&mut self, mode: RecoveryMode) {
    self.cognitive_energy.recover(mode);
    if mode == RecoveryMode::Delta {
        self.state.pad = PadVector::new(
            self.state.pad.pleasure * 0.7,
            self.state.pad.arousal * 0.7,
            self.state.pad.dominance * 0.7,
        );
    }
}
```

This couples energy recovery with emotional dampening. An agent that enters
a dream cycle while highly aroused (stressed) emerges with both more energy
and reduced emotional intensity -- it has "slept off" the stress.

---

## Yerkes-Dodson Complexity Ceiling

The Yerkes-Dodson function maps the agent's current arousal to a maximum task
complexity ceiling:

```rust
pub fn yerkes_dodson_complexity_ceiling(arousal: f64) -> f64 {
    let a = arousal.clamp(-1.0, 1.0);
    let optimal = 0.35;
    let width = 0.6;
    let deviation = (a - optimal) / width;
    (1.0 - 0.5 * deviation * deviation).clamp(0.25, 1.0)
}
```

### Curve Parameters

- **Optimal arousal**: 0.35 (moderate activation). The peak of the
  inverted-U is not at zero (bored) or at 1.0 (panicked) but at moderate
  activation -- the agent performs best when moderately engaged.

- **Width**: 0.6. The Gaussian-like curve degrades gracefully. At arousal =
  -0.25 or 0.95 (one width from optimal), the ceiling drops to about 0.5.

- **Floor**: 0.25. Even at extreme arousal, the agent can still attempt
  trivial tasks. The ceiling never reaches zero.

### Complexity Band Mapping

The ceiling maps to discrete complexity labels used at dispatch time:

```rust
pub fn yerkes_dodson_max_complexity(arousal: f64) -> &'static str {
    let ceiling = yerkes_dodson_complexity_ceiling(arousal);
    if ceiling >= 0.85 { "complex" }
    else if ceiling >= 0.6 { "standard" }
    else if ceiling >= 0.4 { "simple" }
    else { "trivial" }
}
```

| Arousal | Ceiling | Max Complexity | Meaning |
|---|---|---|---|
| -1.0 | 0.25 | trivial | Understimulated -- only routine work |
| -0.3 | ~0.72 | standard | Low activation -- standard tasks ok |
| 0.0 | ~0.83 | standard | Calm -- approaching full capability |
| 0.35 | 1.0 | complex | Optimal -- any complexity is fine |
| 0.7 | ~0.72 | standard | Elevated -- complex tasks degraded |
| 1.0 | ~0.44 | simple | Overstimulated -- only simple tasks |

### Integration with Dispatch

The complexity ceiling is consulted at dispatch time alongside the behavioral
state modulation:

```
1. DaimonState.query() -> current arousal
2. yerkes_dodson_max_complexity(arousal) -> max band
3. Task complexity estimate from strategy space -> task band
4. If task_band > max_band: defer task, select simpler alternative
5. If no simpler alternative: escalate model tier to compensate
```

This creates a dynamic task selection mechanism: an overstimulated agent
(high arousal from many failures) is steered toward simpler tasks, while a
moderately aroused agent can tackle anything. The agent self-regulates its
workload based on its emotional state.

---

## Affect-Energy Coupling

The bidirectional coupling between affect and energy is a key E23 design:

### Energy -> Affect

When energy drops below critical thresholds, it produces affect events:

| Energy Level | Effect |
|---|---|
| current < 0.3 | Arousal increases (urgency to conserve) |
| current < 0.1 | Pleasure decreases (frustration at exhaustion) |
| current = 0.0 | Behavioral phase shifts to Conservation or Declining |

### Affect -> Energy

The depletion multiplier on `CognitiveEnergy` is derived from the PAD state:

| Affect Condition | Multiplier Effect |
|---|---|
| High arousal (A > 0.5) | Multiplier increases -- stressed operations cost more |
| Low pleasure (P < -0.3) | Multiplier increases -- frustration wastes energy |
| High dominance (D > 0.5) | Multiplier decreases slightly -- confidence is efficient |
| Neutral | Multiplier = 1.0 |

This creates the feedback loop: stress makes operations more expensive, which
depletes energy faster, which increases stress. The circuit breaker is
recovery -- Gamma, Theta, or Delta recovery breaks the depletion spiral by
restoring energy without requiring successful task outcomes.

---

## VitalityTracker Integration

The `VitalityTracker` operates at a higher level than `CognitiveEnergy`. It
tracks the agent's overall budget-derived lifecycle phase:

```rust
pub enum BehavioralPhase {
    Thriving,      // vitality >= 0.8: all tiers, full exploration
    Stable,        // vitality >= 0.5: all tiers, max 5 goals
    Conservation,  // vitality >= 0.3: T0/T1 only, reduced exploration
    Declining,     // vitality >= 0.1: T0 only, 1 goal, no exploration
    Terminal,      // vitality < 0.1: no dispatch, prepare shutdown
}
```

The VitalityTracker uses five-percentage-point hysteresis bands around each
boundary to prevent oscillation. Crossing from Thriving to Stable requires
dropping below 0.75 (not 0.80), and returning to Thriving requires rising
above 0.85 (not 0.80).

### Interaction Between Vitality and Energy

- **VitalityTracker** gates *which operations are permitted* (tier limits,
  goal count, exploration allowed). It is a hard policy constraint.
- **CognitiveEnergy** gates *how many operations remain affordable*. It is
  a soft budget signal.
- Together they implement a two-layer resource model: vitality determines
  what the agent *may* do, energy determines what it *can* do within those
  bounds.

---

## Exhaustion and Conservation

When `CognitiveEnergy.current` reaches zero, the agent enters a conservation
state:

```rust
if self.current < 0.0 {
    tracing::warn!(
        effective_cost = effective,
        "cognitive energy exhausted; entering conservation"
    );
    self.current = 0.0;
}
```

Conservation means:
- No new T2 dispatches (too expensive)
- Prefer T0 deterministic probes (cheapest)
- Trigger dream cycle if possible (Delta recovery)
- Signal the plan runner to pause the current wave

The agent does not halt entirely -- it can still process T0 ticks and
respond to critical events. But it cannot perform expensive reasoning until
energy is restored through recovery.

---

## Persistence

`CognitiveEnergy` is serialized as part of `DaimonState` and persisted to
`.roko/daimon/affect.json`. On resume, the agent loads its previous energy
level, fatigue, and recovery mode. Time-based decay is applied to the
PAD state (not to energy -- energy does not decay with time, only with use).

This means an agent that was shut down at 40% energy and resumed 8 hours
later still has 40% energy (not depleted by the passage of time). Its
PAD state, however, has decayed toward neutral during the elapsed time.
The agent wakes up with the same resource budget but with a calmer
emotional state.

---

## Academic Foundations

- Yerkes, R.M. & Dodson, J.D. (1908). "The relation of strength of stimulus
  to rapidity of habit-formation." *Journal of Comparative Neurology and
  Psychology*, 18(5), 459-482.
- Baumeister, R.F., Bratslavsky, E., Muraven, M., & Tice, D.M. (1998).
  "Ego depletion: Is the active self a limited resource?" *Journal of
  Personality and Social Psychology*, 74(5), 1252-1265.
- Sweller, J. (1988). "Cognitive load during problem solving: Effects on
  learning." *Cognitive Science*, 12(2), 257-285.
- Gebhard, P. (2005). "ALMA -- A Layered Model of Affect." *AAMAS*, 29-36.
- Walker, M.P. & van der Helm, E. (2009). "Overnight therapy? The role of
  sleep in emotional brain processing." *Psychological Bulletin*, 135(5),
  731-748.

---

## Cross-References

- `pad-vector.md` -- PAD state that drives the depletion multiplier
- `alma-three-layer-temporal.md` -- three temporal layers that name the recovery modes
- `six-behavioral-states.md` -- behavioral states that gate dispatch decisions
- `daimon-state-and-affect-engine.md` -- DaimonState struct that owns CognitiveEnergy
- `behavioral-state-to-tier-routing.md` -- tier routing constrained by energy budget
