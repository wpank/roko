# Emergent Goal Structures

> **v3 depth file** -- `/docs/v3/depth/00-architecture/emergent-goal-structures.md`
> Canonical source: v1 `docs/v1/00-architecture/28-emergent-goal-structures.md`
> Status: **Complete (E23 10/10)** -- GoalTree, GoalSeed, SlotManager, lifecycle type-state,
> behavioral vitality, CorticalState energy fields, energy accounting, adaptive timescales,
> energy/affect coupling, EFE routing, revisioned mode owners, and phase-aware runner dispatch
> are live in `roko-daimon`. Native Agent-to-E33 observation publication remains broader
> integration scope.

---

## 1. The Problem: Static Goals in a Dynamic System

Roko currently operates with externally assigned goals: requests define what to build, plans
define how to build it, and tasks define the individual steps. The agent executes -- it does
not *want*. This creates three limitations:

1. **No initiative**: The agent cannot identify that a test suite is degrading and
   spontaneously create a "fix flaky tests" goal.
2. **No curiosity**: The agent cannot notice an unexplored API and generate a
   "investigate this for potential utility" goal.
3. **No self-maintenance**: The agent cannot sense its own knowledge decay and generate
   a "consolidate and verify stale knowledge" goal.

Emergent goal structures solve these by allowing goals to arise naturally from the agent's
internal state.

---

## 2. Three Sources of Goal Emergence

### 2.1 The Emergence Triangle

```
           AFFECT (Daimon)
          "What do I want?"
              /      \
             /        \
            /   GOAL   \
           /  EMERGENCE \
          /              \
KNOWLEDGE (Neuro)  <->  EXPERIENCE (Learn)
"What do I know?"    "What have I done?"
```

| Source Pair | Emergence Pattern | Example |
|---|---|---|
| Affect x Knowledge | Desire meets opportunity | "I'm frustrated AND I know the tests are flaky -> Goal: fix flaky tests" |
| Affect x Experience | Desire meets capability | "I'm curious AND I've explored APIs before -> Goal: investigate new API" |
| Knowledge x Experience | Opportunity meets capability | "I know docs are stale AND I've written docs -> Goal: update docs" |
| All three | Full convergence | Affect + knowledge + experience -> Goal: refactor auth module |

### 2.2 E23 Implementation: GoalTree and GoalSeed

The E23 implementation in `roko-daimon/src/goals.rs` uses a `GoalSeed` / `GoalTree` model:

```rust
/// A nascent behavioral pattern that may become a goal.
pub struct GoalSeed {
    pub id: String,
    pub pattern: String,
    pub observation_count: u64,
    pub score: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl GoalSeed {
    pub fn observe(&mut self, weight: f64) {
        self.observation_count += 1;
        self.score += weight;
        self.last_seen = Utc::now();
    }

    pub fn is_promotable(&self, min_observations: u64, min_score: f64) -> bool {
        self.observation_count >= min_observations && self.score >= min_score
    }

    pub fn should_prune(&self, prune_threshold: f64) -> bool {
        self.score < prune_threshold
    }

    pub fn decay(&mut self, decay_factor: f64) {
        self.score *= decay_factor;
    }
}
```

### 2.3 Goal Lifecycle

```
GoalSeed (nascent pattern)
    | observe() -- pattern detected again
    | is_promotable() -- enough evidence accumulated
    v
GoalNode in GoalTree (active goal)
    | progress tracked via episodes
    | sub-goals decomposed via SlotManager
    v
Achieved | Abandoned | Merged
```

The E23 lifecycle type-state ensures compile-time correctness of state transitions.

---

## 3. Goal Emergence Engine

### 3.1 Built-in Detectors

The specification defines five detector types. In the E23 implementation, these emerge
from behavioral vitality monitoring and CorticalState energy fields:

| Detector | Pattern | Trigger |
|---|---|---|
| Knowledge gap | Systematic query misses on referenced topics | Goals to fill knowledge gaps |
| Quality degradation | Gate pass rate trending downward | Goals to fix declining quality |
| Curiosity | High arousal + high-novelty unexplored knowledge | Goals to investigate |
| Self-maintenance | Consolidated knowledge unvalidated > 7 days | Goals to re-validate |
| Frustration recovery | Low pleasure + 3+ consecutive gate failures | Goals to change approach |

### 3.2 SlotManager

The `SlotManager` constrains concurrent active goals by energy zone:

| Energy Zone | Max Active Goals |
|---|---|
| Peak (80-100%) | 5 |
| Normal (50-80%) | 3 |
| Conserving (25-50%) | 2 |
| LowPower (10-25%) | 1 |
| Critical (0-10%) | 0 |

---

## 4. Intrinsic Motivation Scoring

### 4.1 The Motivation Function (Schmidhuber 2010; Colas et al. 2022)

```rust
/// IM = alpha * learning_progress + beta * competence_match + gamma * affect_alignment
pub fn intrinsic_motivation(
    goal: &EmergentGoal,
    agent_competence: f64,
    learning_potential: f64,
    affect_alignment: f64,
) -> f64 {
    const ALPHA: f64 = 0.4;   // learning progress weight
    const BETA: f64 = 0.35;   // competence match weight
    const GAMMA: f64 = 0.25;  // affect alignment weight

    let difficulty = goal.estimated_cost.value() / 1000.0;
    let competence_gap = (difficulty - agent_competence).abs();
    let zpd_score = (-competence_gap * competence_gap / 0.5).exp();

    let competence_score = goal.sources.experience.historical_success_rate;
    let affect_score = affect_alignment.max(0.0);

    ALPHA * learning_potential * zpd_score
        + BETA * competence_score
        + GAMMA * affect_score
}
```

### 4.2 Zone of Proximal Development (Vygotsky 1978, Colas et al. 2022)

Goals are most motivating at the boundary of the agent's competence:

```
Motivation(goal)
      |
  1.0 |         /\
      |        /  \
  0.5 |       /    \
      |      /      \
  0.0 |-----/--------\------
      +----------------------- Difficulty
            |    |    |
          Easy   ZPD   Hard
        (boring)     (frustrating)
```

---

## 5. Goal Competition and Selection

### 5.1 Expected Free Energy (EFE) Ranking (Friston 2010)

Goals are ranked by expected free energy reduction, capturing both epistemic value (how much
the agent will learn) and pragmatic value (how much the agent will accomplish):

```rust
pub fn expected_free_energy(
    goal: &EmergentGoal,
    current_knowledge_entropy: f64,
    predicted_post_knowledge_entropy: f64,
    task_completion_probability: f64,
) -> f64 {
    let epistemic = current_knowledge_entropy - predicted_post_knowledge_entropy;
    let pragmatic = task_completion_probability * goal.priority;
    let cost_penalty = (goal.estimated_cost.value() / 10_000.0).min(1.0);
    epistemic + pragmatic - cost_penalty
}
```

### 5.2 Goal Selection Algorithm

```
ALGORITHM: GoalSelection(engine, budget)

1. Run all detectors -> collect new Nascent goals
2. For each new Nascent goal:
   a. Compute intrinsic_motivation
   b. If IM < nascent_threshold: discard
   c. Check for duplicates via HDC similarity
   d. If duplicate (similarity > merge_threshold): merge
   e. Else: add as Nascent

3. For existing Nascent goals:
   a. Re-evaluate IM (conditions may have changed)
   b. If IM dropped below threshold: remove
   c. If reinforced: increment reinforcement_count
   d. If reinforcement_count >= threshold: promote to Candidate

4. For Candidate goals:
   a. Compute EFE
   b. If slot available: activate highest-EFE Candidate
   c. Convert to plan/task and submit

5. For Active goals:
   a. Check progress (gate verdicts, episodes)
   b. If completed: mark Achieved
   c. If stalled (5+ ticks): evaluate Abandon
   d. If conditions invalidated: Abandon
```

---

## 6. Goal Decomposition

```rust
pub struct GoalDecomposer {
    pub max_sub_goals: usize,           // default: 7 (Miller's number)
    pub decomposition_threshold: f64,   // default: 5000.0 AT
}
```

---

## 7. Somatic Marker Integration (Damasio 1994)

Somatic markers are HDC fingerprints of past goal outcomes that provide rapid pre-evaluation:

```rust
pub struct SomaticMarkerLibrary {
    pub markers: Vec<(Vec<u8>, f64)>,   // (fingerprint, valence)
    pub hdc_dim: usize,                 // default: 10240
    pub activation_threshold: f64,       // default: 0.75
}

impl SomaticMarkerLibrary {
    pub fn evaluate(&self, goal_fingerprint: &[u8]) -> Option<f64> {
        self.markers.iter()
            .filter_map(|(fp, valence)| {
                let similarity = hdc_cosine_similarity(fp, goal_fingerprint, self.hdc_dim);
                if similarity > self.activation_threshold {
                    Some(*valence * similarity)
                } else {
                    None
                }
            })
            .max_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap())
    }
}
```

---

## 8. Configuration

```toml
[goals]
enabled = true

[goals.emergence]
nascent_threshold = 0.3
reinforcement_threshold = 3
max_active_goals = 5
merge_threshold = 0.85

[goals.motivation]
learning_weight = 0.4
competence_weight = 0.35
affect_weight = 0.25

[goals.decomposition]
max_sub_goals = 7
decomposition_threshold = 5000.0

[goals.somatic]
activation_threshold = 0.75
max_markers = 1000

[goals.detectors]
knowledge_gap = true
quality_degradation = true
curiosity = true
self_maintenance = true
frustration_recovery = true
```

---

## 9. Theoretical Foundations

### 9.1 Autotelic Agents (Colas et al. 2022)

IMGEP (Intrinsically Motivated Goal Exploration Processes) establishes that agents which
generate their own goals explore more efficiently than externally directed agents. The key
insight: learning progress is a better intrinsic reward than novelty alone.

### 9.2 Free Energy Principle (Friston 2010)

Under active inference, agents minimize expected free energy, combining uncertainty reduction
(epistemic) and goal satisfaction (pragmatic).

### 9.3 Somatic Marker Hypothesis (Damasio 1994)

Emotional markers from past experiences speed up decision-making by pre-filtering options
before deliberate analysis. The somatic marker library provides sub-millisecond goal
evaluation via HDC fingerprint matching.

---

## 10. Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_knowledge_gap_detector_fires` | Systematic query misses trigger goal | Unit |
| `test_quality_degradation_detector` | Declining gate pass rate triggers goal | Unit |
| `test_curiosity_requires_arousal` | Low-arousal agent doesn't generate curiosity goals | Unit |
| `test_nascent_below_threshold_pruned` | IM < 0.3 goals are removed | Unit |
| `test_reinforcement_promotes_to_candidate` | 3 Theta reinforcements -> Candidate | Unit |
| `test_duplicate_goals_merged` | HDC similarity > 0.85 -> merge | Unit |
| `test_efe_ranks_correctly` | Higher EFE -> higher rank | Unit |
| `test_max_active_goals_enforced` | Cannot activate more than 5 goals | Unit |
| `test_somatic_marker_speeds_evaluation` | Matching marker returns instant valence | Unit |
| `test_goal_lifecycle_full_cycle` | Nascent -> Candidate -> Active -> Achieved | Integration |

---

## Cross-References

- [Cognitive Energy Model](./cognitive-energy-model.md) -- energy zones limit active goal count
- [Attention as Currency](./attention-as-currency.md) -- goals consume AT budget
- v3 Section 9 -- Daimon, affect source for goal emergence
