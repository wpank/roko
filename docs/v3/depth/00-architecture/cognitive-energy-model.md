# Cognitive Energy Model

> **v3 depth file** -- `/docs/v3/depth/00-architecture/cognitive-energy-model.md`
> Canonical source: v1 `docs/v1/00-architecture/29-cognitive-energy-model.md`
> Status: **Wired** -- `roko-daimon::CognitiveEnergy` provides activity-based depletion
> with affect coupling, fatigue tracking, behavioral phase constraints, and recovery modes
> (Gamma/Theta/Delta) wired into dispatch-time modulation via `GraphFeedbackContext`.
> `roko-runtime/src/energy.rs` provides a standalone USD-denominated energy model for
> contexts without DaimonState (benchmarks, isolated tool loops).

---

## 1. The Problem: Unbounded Cognitive Work

Without an energy model, Roko agents have no concept of fatigue or diminishing returns.
Real cognitive systems exhibit energy dynamics: periods of high performance followed by
necessary recovery. The energy model introduces this dynamic, creating natural rhythms of
work and rest that:

1. **Prevent burnout**: Sustained high-intensity work degrades output quality
2. **Enable recovery**: Rest periods consolidate learning (Dreams) and improve future work
3. **Create adaptivity**: Energy level modulates risk tolerance, creativity, and delegation
4. **Align incentives**: The agent naturally paces itself

> "The adaptive regulation of effort is central to effective goal pursuit."
> -- Karl Friston, "A free energy principle for the brain,"
> *Journal of Physiology-Paris* 100(1-3):70-87 (2006).

Friston's free energy principle provides the theoretical grounding: an agent minimizes
surprise (free energy) by allocating metabolic resources -- cognitive energy -- to actions
that reduce uncertainty about the world. When energy is depleted, the agent's capacity for
surprise-minimization degrades, producing the behavioral signatures of fatigue: conservative
strategy, reduced exploration, and increased error rates.

---

## 2. The Energy Pool

### 2.1 Core Type

```rust
/// Cognitive energy pool for a single agent.
///
/// Energy is a dimensionless scalar in [0.0, max_energy].
/// 1.0 energy ~ cost of one Gamma tick at T1 inference.
///
/// Energy differs from attention tokens:
///   - AT are spent and gone (like money)
///   - Energy depletes AND recovers (like biological stamina)
///   - AT buy specific operations; energy modulates capability level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveEnergy {
    pub current: f64,
    pub max_energy: f64,
    pub base_recovery_rate: f64,
    pub depletion_rate: f64,
    pub fatigue: f64,
    pub session_peak: f64,
    pub session_spent: f64,
}

impl Default for CognitiveEnergy {
    fn default() -> Self {
        Self {
            current: 100.0,
            max_energy: 100.0,
            base_recovery_rate: 0.5,  // 0.5 energy/second during idle
            depletion_rate: 0.0,
            fatigue: 0.0,
            session_peak: 100.0,
            session_spent: 0.0,
        }
    }
}
```

### 2.2 Energy vs. Attention Tokens vs. Affect

| Dimension | Attention Tokens | Cognitive Energy | Affect (Daimon) |
|---|---|---|---|
| **Nature** | Currency (spent) | Stamina (depletes + recovers) | Emotional state (continuous) |
| **Replenishment** | Per session / Delta cycle | Continuous (faster during rest) | Driven by outcomes |
| **Depletion cause** | Inference, context, gates | Sustained effort, difficult tasks | Failures, frustration |
| **Effect when low** | Cannot afford expensive ops | Reduced capability, forced conservation | Risk aversion, caution |
| **Recovery mechanism** | Budget allocation | Delta sleep cycles + idle recovery | Successes, novelty |
| **Time constant** | Session-scale (hours) | Minutes to hours | Ticks to sessions |

### 2.3 Implementation in roko-runtime

The `roko-runtime/src/energy.rs` module provides a standalone USD-denominated energy model:

```rust
/// The type of cognitive operation consuming energy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OperationKind {
    LlmCall,           // $0.01
    ToolCall,          // $0.001
    GateRun,           // $0.005
    ContextAssembly,   // $0.002
    KnowledgeQuery,    // $0.001
    Research,          // $0.02
    Planning,          // $0.015
    Checkpoint,        // $0.0005
}

pub struct CognitiveMetabolism {
    pub rates: HashMap<OperationKind, f64>,
    pub global_multiplier: f64,
}
```

This module is superseded by `roko_daimon::CognitiveEnergy` for runtime use, which adds
affect coupling, behavioral phase constraints, and dispatch-time modulation.

---

## 3. Depletion Functions

### 3.1 Per-Operation Energy Costs

```rust
pub struct EnergyCosts {
    pub t0_probe: f64,              // 0.1
    pub t1_inference: f64,          // 1.0
    pub t2_inference: f64,          // 5.0
    pub context_per_kb: f64,        // 0.05
    pub gate_eval: f64,             // 0.3
    pub theta_reflection: f64,     // 3.0
    pub delta_consolidation: f64,  // 15.0
    pub goal_evaluation: f64,      // 0.5
}

impl EnergyCostModel {
    /// Tired agents spend more energy on the same task.
    pub fn actual_cost(&self, base_cost: f64, energy: &CognitiveEnergy) -> f64 {
        let energy_fraction = energy.current / energy.max_energy.max(f64::EPSILON);
        let fatigue_multiplier = 1.0 + self.fatigue_penalty * (1.0 - energy_fraction);
        base_cost * fatigue_multiplier
    }
}
```

### 3.2 Cognitive Difficulty Scaling

```rust
/// Returns a multiplier in [1.0, 5.0] applied to base energy cost.
pub fn cognitive_difficulty(task: &TaskContext) -> f64 {
    let mut difficulty = 1.0;
    difficulty += task.novelty_score * 1.5;
    let complexity = (task.files_involved as f64).log2().max(0.0) / 5.0;
    difficulty += complexity.min(1.0);
    difficulty += task.ambiguity_score * 1.0;
    difficulty.clamp(1.0, 5.0)
}
```

---

## 4. Recovery Mechanisms

### 4.1 Three Recovery Modes

```rust
pub struct EnergyRecovery {
    pub gamma_recovery_rate: f64,    // 0.05/sec (active work)
    pub theta_recovery_rate: f64,    // 0.3/sec (reflection)
    pub delta_recovery_rate: f64,    // 2.0/sec (deep rest)
    pub idle_recovery_rate: f64,     // 0.5/sec
    pub fatigue_recovery_rate: f64,  // 0.01/sec (slow)
}

impl EnergyRecovery {
    pub fn recover(
        &self,
        energy: &mut CognitiveEnergy,
        mode: RecoveryMode,
        elapsed_secs: f64,
    ) {
        let rate = match mode {
            RecoveryMode::Gamma => self.gamma_recovery_rate,
            RecoveryMode::Theta => self.theta_recovery_rate,
            RecoveryMode::Delta => self.delta_recovery_rate,
            RecoveryMode::Idle => self.idle_recovery_rate,
        };

        let effective_max = energy.max_energy - energy.fatigue;
        let recovery = rate * elapsed_secs;
        energy.current = (energy.current + recovery).min(effective_max);

        let fatigue_recovery = self.fatigue_recovery_rate * elapsed_secs;
        energy.fatigue = (energy.fatigue - fatigue_recovery).max(0.0);
    }
}
```

### 4.2 Delta Recovery Profile

```
Energy during Delta cycle:

100|-----\                    /-----
   |      \  consolidation  /
   |       \  cost (-15)   / recovery (+40)
 60|        \             /
   |         \           /
 45|          \---------/
   |     active    rest/sleep    refreshed
   +----------------------------------------
          Time during Delta cycle
```

---

## 5. Energy-Affect Coupling

### 5.1 Bidirectional Influence

Energy and affect form a bidirectional coupling. This is the core innovation of the E23
implementation -- both the DaimonState and CognitiveEnergy mutually modulate each other:

```rust
pub struct EnergyAffectCoupling {
    // Energy -> Affect direction
    pub energy_to_pleasure: f64,       // 0.3
    pub energy_to_dominance: f64,      // 0.2
    pub critical_energy_arousal: f64,  // 0.4

    // Affect -> Energy direction
    pub pleasure_cost_discount: f64,   // 0.15
    pub arousal_cost_premium: f64,     // 0.1
    pub dominance_recovery_bonus: f64, // 0.2
}

impl EnergyAffectCoupling {
    pub fn energy_to_pad(&self, energy: &CognitiveEnergy) -> PadVector {
        let fraction = energy.current / energy.max_energy.max(f64::EPSILON);

        let pleasure_delta = if fraction < 0.3 {
            -self.energy_to_pleasure * (1.0 - fraction / 0.3)
        } else { 0.0 };

        let dominance_delta = if fraction < 0.4 {
            -self.energy_to_dominance * (1.0 - fraction / 0.4)
        } else { 0.0 };

        let arousal_delta = if fraction < 0.15 {
            self.critical_energy_arousal * (1.0 - fraction / 0.15)
        } else { 0.0 };

        PadVector { pleasure: pleasure_delta, arousal: arousal_delta, dominance: dominance_delta }
    }
}
```

### 5.2 Energy Zones and Behavioral States

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyZone {
    Peak,       // 80-100%: Full capability. T2 allowed. Creative exploration.
    Normal,     // 50-80%: Standard operation. All tiers.
    Conserving, // 25-50%: Prefer T0/T1. Avoid novel tasks.
    LowPower,   // 10-25%: T0 only. Complete current, defer new.
    Critical,   // 0-10%: Shutdown non-essential. Trigger Delta.
}

impl EnergyZone {
    pub fn max_tier(&self) -> InferenceTier {
        match self {
            Self::Peak | Self::Normal => InferenceTier::T2,
            Self::Conserving => InferenceTier::T1,
            Self::LowPower | Self::Critical => InferenceTier::T0,
        }
    }

    pub fn should_trigger_delta(&self) -> bool {
        matches!(self, Self::Critical)
    }

    pub fn max_active_goals(&self) -> usize {
        match self {
            Self::Peak => 5, Self::Normal => 3, Self::Conserving => 2,
            Self::LowPower => 1, Self::Critical => 0,
        }
    }
}
```

---

## 6. Capacity Growth (Fitness)

Over time, successful work and effective consolidation increase the agent's energy capacity:

```rust
pub struct EnergyCapacityModel {
    pub base_capacity: f64,      // 100.0
    pub growth_per_delta: f64,   // 0.1
    pub growth_per_task: f64,    // 0.02
    pub capacity_ceiling: f64,   // 200.0
    pub disuse_decay: f64,       // 0.001 per hour
}
```

Output quality degrades gracefully with energy:
```
Output quality ~ base_quality * energy_fraction^0.3

At full energy: quality = base * 1.0
At 50% energy: quality = base * 0.81
At 25% energy: quality = base * 0.66
At 10% energy: quality = base * 0.50
```

---

## 7. Configuration

```toml
[energy]
enabled = true

[energy.pool]
base_capacity = 100.0
capacity_ceiling = 200.0
idle_recovery_rate = 0.5

[energy.costs]
t0_probe = 0.1
t1_inference = 1.0
t2_inference = 5.0
context_per_kb = 0.05
gate_eval = 0.3
theta_reflection = 3.0
delta_consolidation = 15.0
goal_evaluation = 0.5

[energy.fatigue]
fatigue_penalty = 0.5
fatigue_recovery_rate = 0.01

[energy.recovery]
gamma_recovery = 0.05
theta_recovery = 0.3
delta_recovery = 2.0

[energy.zones]
peak_threshold = 0.80
normal_threshold = 0.50
conserving_threshold = 0.25
low_power_threshold = 0.10

[energy.coupling]
energy_to_pleasure = 0.3
energy_to_dominance = 0.2
critical_energy_arousal = 0.4
pleasure_cost_discount = 0.15
arousal_cost_premium = 0.1
dominance_recovery_bonus = 0.2

[energy.capacity]
growth_per_delta = 0.1
growth_per_task = 0.02
disuse_decay = 0.001
```

---

## 8. Theoretical Foundations

### 8.1 Free Energy Principle (Friston 2006)

Friston's free energy principle models agents as minimizing variational free energy -- a
quantity that bounds surprise. Energy allocation follows naturally: actions that reduce
surprise are metabolically efficient; actions that increase surprise are costly. The
cognitive energy model operationalizes this: T2 inference (high surprise-reduction capacity)
costs more energy than T0 probes (habitual processing), creating natural pressure toward
efficient cognitive strategies.

### 8.2 Resource Theory of Attention (Kahneman 1973)

Kahneman modeled attention as drawing from a limited "effort supply" that replenishes over
time. The cognitive energy model extends this to general cognitive capacity.

### 8.3 Compensatory Control Model (Hockey 2011)

Three fatigue levels map directly:
- **Performance protection**: fatigue penalty (spend more energy on same task)
- **Strategy adjustment**: energy zone degradation (switch to cheaper strategies)
- **Goal disengagement**: Critical zone (abandon non-essential goals)

### 8.4 Energy-Based World Models (Gladstone et al. 2024, EBWM)

EBWM demonstrated that energy scalars effectively model state plausibility and guide
adaptive computation allocation.

---

## 9. Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_energy_depletion_t2` | T2 inference depletes 5.0 base energy | Unit |
| `test_fatigue_penalty_increases_cost` | At 20% energy, T2 costs 7.0 | Unit |
| `test_recovery_idle_rate` | Idle recovery at 0.5/sec over 10s = +5.0 | Unit |
| `test_recovery_delta_net_positive` | Delta cycle costs -15 but recovers net positive | Unit |
| `test_zone_peak_allows_t2` | At 90% energy, T2 is allowed | Unit |
| `test_zone_conserving_blocks_t2` | At 40% energy, T2 is blocked | Unit |
| `test_zone_critical_triggers_delta` | At 5% energy, Delta auto-triggered | Unit |
| `test_energy_affect_coupling_low_energy` | Low energy reduces pleasure and dominance | Unit |
| `test_pleasure_reduces_cost` | High pleasure -> 15% cost discount | Unit |
| `test_capacity_growth_from_tasks` | Successful tasks increase max_energy | Unit |
| `test_capacity_ceiling_enforced` | max_energy cannot exceed 200.0 | Unit |
| `test_max_goals_by_zone` | Peak=5, Normal=3, Conserving=2, LowPower=1, Critical=0 | Unit |

---

## Cross-References

- [Attention as Currency](./attention-as-currency.md) -- AT and energy as dual constraints
- [Emergent Goal Structures](./emergent-goal-structures.md) -- energy zones limit goal count
- v3 Section 9 -- Daimon, bidirectional energy-affect coupling
- v3 Section 10 -- Dreams, Delta consolidation as primary recovery mechanism
