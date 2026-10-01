# 05 -- Cognitive Autonomy (E23)

> **Implementation status (2026-09):** Complete (E23 10/10 manifest).
> Lifecycle type-state, five-phase VitalityTracker, CorticalState
> energy fields, energy/affect coupling, EFE routing, GoalTree,
> SlotManager, revisioned mode owners, and phase-aware runner dispatch
> are all live. Native Agent-to-E33 observation publication remains
> broader integration scope.

---

## Theoretical Foundations

The cognitive autonomy subsystem gives agents biologically-inspired
lifecycle and energy dynamics. It draws on two theoretical pillars:

### Free Energy Principle (Friston, 2006)

The Free Energy Principle (Friston, 2006, "A free energy principle for
the brain," *Journal of Physiology - Paris*, 100(1-3), 70-87) proposes
that biological agents minimize the difference between their internal
model of the world and incoming sensory evidence. This difference is
measured as *free energy* (or equivalently, *surprise*).

In Roko, the Expected Free Energy (EFE) framework drives routing
decisions. An agent selects actions that minimize expected surprise:

- **Epistemic value** -- exploring uncertain states reduces future
  surprise (maps to trying new models or strategies).
- **Pragmatic value** -- achieving known objectives directly reduces
  surprise (maps to using proven strategies).
- **Precision** -- the confidence threshold acts as the agent's
  precision parameter, controlling how much surprise it tolerates.

The EFE decomposition into epistemic and pragmatic value provides a
principled basis for the explore-exploit tradeoff that the CascadeRouter
implements (Friston et al., 2015, "Active inference and epistemic
value," *Cognitive Neuroscience*).

### Developmental Maturation (Gesell, 1916)

Arnold Gesell's developmental maturation theory (*The Mental Growth of
the Pre-School Child*, 1916, Yale University Press) proposes that
organisms pass through invariant developmental stages driven by
internal biological maturation rather than external training alone.
Key principles:

1. **Invariant sequence** -- stages cannot be skipped. An agent in
   Critical phase cannot jump directly to Thriving.
2. **Intrinsic pacing** -- development follows an internal clock. In
   Roko, this is the resource budget depletion rate.
3. **Progressive differentiation** -- capabilities emerge from simple
   to complex. Thriving agents have the broadest capability set;
   Terminal agents have the narrowest.
4. **Interweaving** -- periods of stability alternate with periods of
   rapid change. The hysteretic phase transitions implement this
   principle, preventing oscillation at boundaries.

In Roko, these principles map to the five `BehavioralPhase`s: an agent
matures from Thriving through Conserving to Terminal as its resources
deplete, and the transitions are hysteretic (interweaving principle)
to prevent oscillation at boundaries.

---

## VitalityTracker

The VitalityTracker monitors an agent's resource budget and maps it to
behavioral phases:

```rust
// crates/roko-daimon/src/lib.rs

pub struct VitalityTracker {
    pub initial_budget: f64,
    pub remaining_budget: f64,
    pub last_phase: BehavioralPhase,
    pub last_transition_at: DateTime<Utc>,
}
```

### Vitality computation

```
vitality = remaining_budget / initial_budget
```

A 5-percentage-point hysteresis band (`HYSTERESIS = 0.05`) prevents
phase oscillation at boundaries. An agent must cross the boundary
*plus* the hysteresis band before transitioning.

### Five BehavioralPhases

```rust
pub enum BehavioralPhase {
    Thriving,   // vitality >= 0.8
    Stable,     // vitality >= 0.5
    Conserving, // vitality >= 0.2
    Critical,   // vitality >= 0.05
    Terminal,   // vitality < 0.05
}
```

| Phase | Vitality | Tier access | Max active goals | Behavior |
|-------|----------|-------------|-----------------|----------|
| Thriving | >= 0.8 | All tiers | Unlimited | Full exploration available |
| Stable | >= 0.5 | All tiers | 5 | Standard operation |
| Conserving | >= 0.2 | Standard + Fast | 3 | Reduced exploration |
| Critical | >= 0.05 | Fast only | 1 | Essential tasks only |
| Terminal | < 0.05 | None | 0 | Shutdown preparation |

### Phase transition example

```
Initial state: Thriving (vitality = 1.0)

Budget consumption over time:
  vitality = 0.85 -> still Thriving (above 0.8)
  vitality = 0.78 -> still Thriving (below 0.8 but within hysteresis)
  vitality = 0.74 -> transition to Stable (0.8 - 0.05 = 0.75, crossed)

To return to Thriving:
  vitality must exceed 0.85 (0.8 + 0.05)
```

---

## CorticalState

The CorticalState is a shared perception surface providing lock-free
concurrent access to cognitive signals:

```rust
// crates/roko-runtime/src/heartbeat.rs

pub struct CorticalState {
    // PAD affect dimensions (Pleasure-Arousal-Dominance)
    pleasure: AtomicU32,
    arousal: AtomicU32,
    dominance: AtomicU32,

    // Plutchik primary emotion label
    primary_emotion: AtomicU8,

    // Prediction accuracy tracking
    aggregate_accuracy: AtomicU32,
    accuracy_trend: AtomicI8,
    category_accuracies: [AtomicU32; 16],

    // Environmental awareness
    surprise_rate: AtomicU32,
    universe_size: AtomicU32,
    active_count: AtomicU16,
    pending_predictions: AtomicU32,

    // Creative/exploratory state
    creative_mode: AtomicU8,
    fragments_captured: AtomicU32,
    last_novel_prediction_tick: AtomicU64,

    // Environmental regime
    regime: AtomicU8,

    // Resource signals
    gas_gwei: AtomicU32,
    resource_health: AtomicU32,
    knowledge_health: AtomicU32,
    performance_trend: AtomicU32,

    // Behavioral classification
    behavioral_state: AtomicU8,

    // Energy dynamics
    compounding_momentum: AtomicU32,
    cognitive_energy: AtomicU32,
    fatigue_penalty: AtomicU32,
    efe_last_tier: AtomicU8,
}
```

### Why atomics?

All fields use atomic types for lock-free concurrent access. Multiple
producers (heartbeat ticks, task completions, prediction evaluations)
and consumers (the runner, routing decisions, TUI display) access the
CorticalState simultaneously. Atomics avoid the latency and contention
of mutex locks on a hot path.

The PAD dimensions use `AtomicU32` to store `f32` bit patterns. The
boundary conversion uses `f32::to_bits()` for storage and
`f32::from_bits()` for reads, narrowing from the canonical `f64`
`PadVector` at the read/write boundary.

### PersonalityPreset

Initial CorticalState values are determined by a personality preset:

```rust
pub enum PersonalityPreset {
    Cautious,   // PAD: (-0.1, 0.1, -0.2)
    Balanced,   // PAD: (0.0, 0.0, 0.0)
    Aggressive, // PAD: (0.1, 0.3, 0.2)
    Custom(PadVector),
}
```

### BehavioralState

```rust
pub enum BehavioralState {
    Engaged,    // Balanced exploration and exploitation
    Struggling, // Repeated failures, low confidence
    Coasting,   // Success with low arousal (complacency risk)
    Exploring,  // Actively searching, tolerating uncertainty
    Focused,    // Deep-work state, high arousal + dominance
    Resting,    // Low-arousal pre-consolidation state
}
```

---

## Energy/Affect Coupling

Cognitive energy depletion and recovery modulate affect state. The
coupling ensures that:

- **Low energy** produces conservation behavior: reduced exploration,
  tier restrictions, negative affect shift.
- **High energy** enables creative exploration: broader tool access,
  positive affect, willingness to try premium models.

The energy fields in CorticalState (`cognitive_energy`,
`fatigue_penalty`, `compounding_momentum`) track resource dynamics:

- `cognitive_energy` -- current available energy for cognitive work.
- `fatigue_penalty` -- accumulated penalty from sustained effort.
- `compounding_momentum` -- bonus from consecutive successful tasks.

---

## EFE Routing

Expected Free Energy (Friston, 2006) drives routing decisions. The
agent selects actions that minimize expected surprise between its
internal model and observed outcomes:

```
EFE(action) = epistemic_value(action) + pragmatic_value(action)
```

This manifests in the CascadeRouter's confidence cascade:

- **High-confidence tasks** route to cheap models (low EFE -- the
  outcome is predictable, so epistemic value is low and pragmatic value
  dominates).
- **Uncertain tasks** route to expensive models (high epistemic value
  justifies the cost -- reducing uncertainty about the right approach).

The `efe_last_tier` field in CorticalState records which tier the last
EFE-driven routing decision selected, providing continuity across
turns.

---

## GoalTree

Goals emerge from repeated patterns in agent behavior rather than
being explicitly programmed:

```rust
// crates/roko-daimon/src/goals.rs

pub struct GoalTree {
    seeds: Vec<GoalSeed>,
    nodes: HashMap<String, GoalNode>,
    min_observations: u64,
    min_score: f64,
    completion_threshold: f64,
    prune_threshold: f64,
}
```

### Lifecycle

```
observation -> GoalSeed -> (evidence accumulates) -> GoalNode
                              | pruned if score < threshold
```

### GoalSeed

```rust
pub struct GoalSeed {
    pub id: String,
    pub pattern: String,       // Observed behavioral pattern
    pub observation_count: u64,
    pub score: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub tags: Vec<String>,
}
```

Seeds are created when a recurring pattern is detected (e.g., the
agent repeatedly fixes compile errors before running tests). Each seed
tracks observation frequency and accumulates a score.

**Promotion:** A seed promotes to a `GoalNode` when
`observation_count >= min_observations` and `score >= min_score`.

**Decay:** Seeds undergo time-based score decay via
`seed.decay(factor)`. This prevents stale patterns from persisting
indefinitely.

### GoalNode

```rust
pub struct GoalNode {
    pub id: String,
    pub description: String,
    pub priority: f64,      // [0, 1]
    pub progress: f64,      // [0, 1]
    pub status: GoalStatus, // Active | Completed | Suspended | Pruned
    pub success_count: u64,
    pub failure_count: u64,
    pub children: Vec<String>,
    pub parent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum GoalStatus {
    Active,
    Completed,
    Suspended,
    Pruned,
}
```

Nodes form a hierarchy through `parent`/`children` links. A node
is completed when `progress >= completion_threshold` and pruned when
its effective priority drops below `prune_threshold`.

---

## SlotManager

Manages concurrent agent instances with validated capacity:

```rust
// crates/roko-agent/src/lifecycle.rs

pub struct SlotManager {
    slots: BTreeMap<String, SlotState>,
    max_slots: usize,
}
```

- Slot names must be unique.
- Capacity must be non-zero.
- Revisioned mode owners ensure that only one mode-change per slot is
  active at any time, preventing race conditions in concurrent
  lifecycle transitions.

---

## Adaptive Clock and Timescales

The `AdaptiveClock` (`crates/roko-runtime/src/heartbeat.rs`) provides
regime-adaptive timing for cognitive loops:

```rust
pub struct AdaptiveClock {
    timescale: CognitiveTimescale,  // Gamma | Theta | Delta
    base_interval: Duration,
    current_interval: Duration,
    last_fired: Option<Instant>,
    gamma_ticks_since_theta: u32,
}
```

### Regime-based adaptation

```rust
pub enum Regime {
    Calm,      // 4x slower (Gamma), 2x slower (Theta)
    Normal,    // 1x (baseline)
    Volatile,  // 2x faster
    Crisis,    // 4x faster (Gamma/Theta), 2x faster (Delta)
}
```

In Crisis regime, the Gamma loop runs 4x faster than baseline,
increasing the agent's reactive perception cadence. In Calm regime,
it slows by 4x to conserve resources.

---

## Yerkes-Dodson Complexity Ceiling

The Yerkes-Dodson law (Yerkes & Dodson, 1908) states that performance
peaks at moderate arousal. This is encoded as a complexity ceiling
function:

```rust
pub fn yerkes_dodson_complexity_ceiling(arousal: f64) -> f64 {
    let optimal = 0.35;
    let width = 0.6;
    let deviation = (arousal - optimal) / width;
    (1.0 - 0.5 * deviation * deviation).clamp(0.25, 1.0)
}
```

At dispatch time, the ceiling maps to complexity bands:

| Ceiling | Band |
|---------|------|
| >= 0.85 | complex |
| >= 0.6 | standard |
| >= 0.4 | simple |
| < 0.4 | trivial |

This prevents over-aroused agents from taking on complex tasks and
under-aroused agents from wasting capacity on trivial work.

---

## Prospect Theory Integration

The `prospect_value` function maps realized P&L to subjective value
using Kahneman and Tversky's prospect theory (1979):

```rust
pub fn prospect_value(pnl: f64) -> f64 {
    const LOSS_AVERSION: f64 = 2.25;
    const CURVATURE: f64 = 0.88;

    if pnl > 0.0 {
        pnl.powf(CURVATURE)
    } else {
        -LOSS_AVERSION * pnl.abs().powf(CURVATURE)
    }
}
```

Losses are weighted 2.25x more than equivalent gains. This produces
risk-averse behavior when resources are scarce.

---

## Phase-Aware Runner Dispatch

The plan runner consults the VitalityTracker before dispatching each
task:

1. **Check vitality** -- compute `remaining_budget / initial_budget`.
2. **Determine phase** -- apply hysteretic phase transitions.
3. **Apply restrictions** -- filter available model tiers and limit
   active goals based on the current phase.
4. **Route through CascadeRouter** -- the phase-restricted tier set
   is passed as a constraint to the routing decision.
5. **Record feedback** -- after task completion, update the
   VitalityTracker with consumed budget.

In Terminal phase, the runner initiates graceful shutdown rather than
dispatching new tasks.

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-daimon/src/lib.rs` | VitalityTracker, BehavioralPhase, prospect_value, affect_size_multiplier, yerkes_dodson |
| `crates/roko-daimon/src/goals.rs` | GoalTree, GoalSeed, GoalNode, GoalStatus |
| `crates/roko-runtime/src/heartbeat.rs` | CorticalState, AdaptiveClock, Regime, PersonalityPreset, BehavioralState |
| `crates/roko-agent/src/lifecycle.rs` | SlotManager, AgentCoreManifest, lifecycle type-state |
| `crates/roko-learn/src/` | CascadeRouter (EFE routing integration) |

---

## Citations

1. Friston, K. (2006). "A free energy principle for the brain." *Journal of Physiology - Paris*, 100(1-3), 70-87.
2. Friston, K. et al. (2015). "Active inference and epistemic value." *Cognitive Neuroscience*, 6(4), 187-214.
3. Gesell, A. (1916). *The Mental Growth of the Pre-School Child.* Yale University Press.
4. Yerkes, R. M. & Dodson, J. D. (1908). "The relation of strength of stimulus to rapidity of habit-formation." *Journal of Comparative Neurology and Psychology*, 18(5), 459-482.
5. Kahneman, D. & Tversky, A. (1979). "Prospect Theory: An Analysis of Decision under Risk." *Econometrica*, 47(2), 263-291.
6. Plutchik, R. (1980). *Emotion: A Psychoevolutionary Synthesis.*
7. `crates/roko-daimon/src/lib.rs` -- VitalityTracker, BehavioralPhase.
8. `crates/roko-runtime/src/heartbeat.rs` -- CorticalState,
   AdaptiveClock.
9. `crates/roko-daimon/src/goals.rs` -- GoalTree, GoalSeed, GoalNode.
