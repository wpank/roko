# DaimonState Struct and Affect Engine

> Depth file for [11-AFFECT.md](../../11-AFFECT.md) -- new for v3 (E23 cognitive autonomy)

---

## Overview

`DaimonState` is the single entry point for all affect operations in the Roko
runtime. It owns the current PAD state, the somatic landscape, the strategy
space definition, per-crate confidence tracking, the contrarian retrieval
tracker, the error pattern model, the fatigue detector, borrowed peer affect,
the behavioral state tracker with hysteresis, the vitality tracker, the
cognitive energy pool, the emergent goal tree, and the novelty filter. Every
subsystem that needs to read or modify the agent's emotional state goes
through this struct.

The struct is persisted to disk at `.roko/daimon/affect.json` and restored on
resume. The persistence path is an `Option<PathBuf>` that is set at
initialization and is skipped during serialization (it is a runtime-only
field). The autosave mechanism writes the state after every significant
appraisal event, ensuring that the agent's emotional context survives
crashes and restarts.

---

## DaimonState Fields

The full struct definition in `crates/roko-daimon/src/lib.rs`:

```rust
pub struct DaimonState {
    /// Current affect snapshot (PAD, confidence, behavioral state, ALMA layers).
    pub state: AffectState,

    /// Half-life in hours for PAD state decay (default: 4.0).
    pub half_life_hours: f64,

    /// Situation-specific somatic markers (k-d tree over 8D strategy space).
    pub somatic_landscape: SomaticLandscape,

    /// Active strategy-space definition for interpreting 8D coordinates.
    pub strategy_space: StrategySpaceDefinition,

    /// Per-crate confidence hints for coding-domain integrations.
    pub crate_confidence_map: HashMap<String, f64>,

    /// Per-crate confidence and fatigue tracking (DAIM-03).
    pub crate_trackers: HashMap<String, CrateConfidence>,

    /// Rolling contrarian retrieval tracker (15% minimum opposite-valence).
    pub contrarian_tracker: ContrarianTracker,

    /// Familiarity model for error-category appraisal scaling.
    pub error_patterns: ErrorPatternTracker,

    /// Failure-streak tracker for fatigue detection.
    pub fatigue_detector: FatigueDetector,

    /// Borrowed peer affect awaiting accelerated decay.
    pub borrowed_affect: Vec<BorrowedAffect>,

    /// Behavioral state tracker with hysteresis and minimum dwell time.
    pub behavioral_tracker: BehavioralStateTracker,

    /// Budget-derived operational vitality phase.
    pub vitality_tracker: VitalityTracker,

    /// Cognitive energy and fatigue budget.
    pub cognitive_energy: CognitiveEnergy,

    /// Promoted emergent goals and their lifecycle state.
    pub goal_tree: GoalTree,

    /// Behavioral patterns accumulating evidence for promotion.
    pub goal_seeds: Vec<GoalSeed>,

    /// Novelty filter for suppressing repeated appraisal triggers (P1-04).
    pub novelty_filter: NoveltyFilter,

    /// Optional persistence path for best-effort autosaves.
    #[serde(skip)]
    persistence_path: Option<PathBuf>,
}
```

### Default Initialization

A freshly constructed `DaimonState` starts at emotional neutral:

- PAD vector: [0.0, 0.0, 0.0] (neutral)
- Confidence: 0.70 (slightly above midpoint -- optimistic default)
- Behavioral state: Engaged
- Half-life: 4.0 hours
- Somatic landscape: empty (no markers)
- Cognitive energy: 1.0/1.0 (full)
- Fatigue: 0.0
- Vitality phase: Thriving
- Goal tree: empty
- Novelty filter: window size 10

The optimistic default confidence (0.70 instead of 0.50) reflects the design
principle that an agent should start willing to attempt tasks. A 0.50 default
would place the agent closer to the Struggling threshold (0.30), meaning a
single bad task could immediately trigger defensive behavior before the agent
has enough history to assess its own capability.

---

## AffectState: The Affect Snapshot

`AffectState` is the core emotional snapshot within `DaimonState`:

```rust
pub struct AffectState {
    /// Current PAD vector.
    pub pad: PadVector,
    /// Motivational confidence in [0.0, 1.0].
    pub confidence: f64,
    /// Explicit behavioral state derived from PAD plus confidence.
    pub behavioral_state: BehavioralState,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Three-layer ALMA temporal model (Gebhard 2005).
    pub alma: AlmaLayers,
    /// Total appraisal ticks since creation.
    pub tick_count: u64,
}
```

The `behavioral_state` field is an explicit enum stored on the snapshot rather
than being re-derived every time it is needed. This makes the PAD-to-policy
bridge explicit: `BehavioralState::classify(pad, confidence)` is called once
per appraisal, and the result is persisted alongside the PAD values.

The `alma` field holds the three-layer ALMA model (see
`alma-three-layer-temporal.md`): emotion (immediate reaction), mood (EMA
smoothed), and temperament (stable baseline). The `tick()` method on
`AlmaLayers` updates mood and temperament at their respective intervals.

---

## Appraisal Pipeline

The central method on `DaimonState` is `appraise()`, which processes an
`AffectEvent` and updates all internal state:

```
AffectEvent arrives
  |
  v
Step 1: NOVELTY CHECK -- suppress repeated identical triggers
  |
  v
Step 2: COMPUTE APPRAISAL -- OCC/Scherer appraisal rules produce
        desirability, likelihood, coping_potential, trigger, novelty
  |
  v
Step 3: DECAY -- apply temporal decay to current mood before adding delta
  |
  v
Step 4: COMPUTE PAD DELTA -- from appraisal result
  |
  v
Step 5: APPLY DELTA -- add to current PAD with clamping to [-1, 1]
  |
  v
Step 6: CLASSIFY -- derive BehavioralState from updated PAD + confidence
  |
  v
Step 7: TICK ALMA -- update mood and temperament layers at their intervals
  |
  v
Step 8: UPDATE TRACKERS -- behavioral tracker, fatigue detector, error patterns
  |
  v
Step 9: AUTOSAVE -- persist to disk if path is configured
```

### AffectEvent Enum

The `AffectEvent` enum captures all triggers that produce emotional responses:

| Variant | Fields | PAD Effect |
|---|---|---|
| `GateResult` | plan_id, task_id, passed, rung | Pleasure +/- scaled by rung; arousal +/- |
| `TaskOutcome` | task_id, succeeded | Strong pleasure/dominance effect |
| `Blocked` | task_id, blocker_count | Arousal up, dominance down |
| `TimePressure` | task_id, deadline_proximity | Pure arousal signal |
| `QueueWait` | task_id, wait_hours | Ramped arousal after 24h |
| `DreamOutcome` | knowledge_entries, playbooks, regressions, hypotheses, episodes | Confidence via dream quality |
| `Shutdown` | vitality, total_episodes, graceful | Mortality-aware terminal appraisal |
| `PredictionError` | task_id, predicted, actual, magnitude | Dominance adjustment |
| `ToolSuccess` | task_id, tool_name | Mild positive pleasure |
| `ToolFailure` | task_id, tool_name, error | Mild negative pleasure and dominance |

The appraisal rules are deterministic: the same event always produces the
same PAD delta (given the same current state). No emotion is generated
without a grounded trigger -- this is the central OCC constraint that
prevents affective hallucination.

---

## Modulation: How Affect Changes Dispatch

The `modulate()` method on `DaimonState` takes a mutable `DispatchParams`
and adjusts it based on the current affect state:

```rust
fn modulate(&self, params: &mut DispatchParams) {
    let state = self.query();

    if state.confidence < 0.30 || state.pad.dominance < -0.25 {
        // Struggling -> Escalating
        params.strategy = DispatchStrategy::Escalating;
        params.turn_limit = params.turn_limit.saturating_add(10);
        params.model = promote_model(&params.model);
    } else if state.pad.pleasure > 0.35 && state.confidence > 0.65 {
        // Coasting -> Exploratory
        params.strategy = DispatchStrategy::Exploratory;
        params.turn_limit = params.turn_limit.saturating_sub(5);
        params.model = demote_model(&params.model);
    } else if state.pad.pleasure < -0.30 && state.pad.arousal > 0.30 {
        // Struggling -> Conservative
        params.strategy = DispatchStrategy::Conservative;
        params.turn_limit = params.turn_limit.saturating_sub(3);
        params.model = demote_model(&params.model);
    } else if state.pad.arousal < -0.20 {
        // Resting -> Proactive
        params.strategy = DispatchStrategy::Proactive;
        params.turn_limit = params.turn_limit.saturating_add(5);
    } else {
        // Engaged -> Balanced
        params.strategy = DispatchStrategy::Balanced;
    }

    params.effort = params.strategy.effort_label().to_string();
}
```

### Model Promotion and Demotion

Model promotion and demotion use string-based replacement:

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

This is a coarse heuristic operating on model name strings. The
CascadeRouter's bandit model provides fine-grained selection within the
promoted/demoted tier.

---

## Somatic Landscape

The `SomaticLandscape` within `DaimonState` is backed by a `kiddo::KdTree`
over 8 dimensions:

```rust
pub struct SomaticLandscape {
    tree: KdTree<f64, 8>,
    markers: Vec<SomaticMarker>,
}

pub struct SomaticMarker {
    pub strategy_coords: [f64; 8],
    pub valence: f64,      // positive = worked well, negative = went badly
    pub intensity: f64,    // [0, 1] strength of the feeling
    pub episodes: Vec<ContentHash>,
}
```

The landscape stores situation-specific emotional memories. Before selecting
an action, the agent queries the landscape with the proposed strategy's 8D
coordinates. The query returns a `SomaticSignal` with the aggregate valence
and intensity of nearby markers, plus the mandatory 15% contrarian component.

Markers are created by significant live events (PAD delta > 0.15) and by
dream consolidation. Consolidation merges nearby markers (within Euclidean
distance 0.5) to prevent unbounded growth while preserving the aggregate
emotional signal.

For full details, see `somatic-markers-damasio.md` and
`8-dimensional-strategy-space.md`.

---

## Persistence and Resume

### File Layout

DaimonState persists to `.roko/daimon/affect.json`:

```json
{
  "state": {
    "pad": { "pleasure": -0.15, "arousal": 0.22, "dominance": -0.08 },
    "confidence": 0.42,
    "behavioral_state": "Struggling",
    "updated_at": "2026-04-12T14:30:00Z",
    "alma": { ... },
    "tick_count": 1547
  },
  "half_life_hours": 4.0,
  "somatic_landscape": { "markers": [...] },
  "strategy_space": { "domain": "coding", "dimensions": [...] },
  "crate_confidence_map": { "roko-core": 0.85, "roko-daimon": 0.35 },
  "contrarian_tracker": { "window": [...] },
  "cognitive_energy": {
    "current": 0.62,
    "max": 1.0,
    "fatigue": 0.28,
    "recovery_mode": "gamma",
    "depletion_multiplier": 1.0,
    "fatigue_intensity_factor": 1.0
  },
  "vitality_tracker": { ... },
  "goal_tree": { ... },
  "goal_seeds": [...]
}
```

### Resume Behavior

On resume (`DaimonState::load(path)`):

1. The JSON is deserialized into the full struct.
2. Temporal decay is applied for the elapsed time since `state.updated_at`:
   ```
   factor = 0.5 ^ (elapsed_hours / half_life_hours)
   pad.pleasure  *= factor
   pad.arousal   *= factor
   pad.dominance *= factor
   confidence = 0.5 + (confidence - 0.5) * factor
   ```
3. The behavioral state is reclassified from the decayed PAD values.
4. The somatic landscape is loaded as-is (markers do not decay with time --
   they are managed by dream consolidation).
5. Cognitive energy is loaded as-is (energy does not decay with time).
6. The contrarian tracker's ring buffer is loaded, and events outside the
   window are pruned.

This means an agent shut down 8 hours ago in a deeply negative mood (-0.60
pleasure) resumes at -0.15 pleasure (two half-lives of decay). The emotional
context is preserved but attenuated -- the agent "remembers" that yesterday
was rough without being trapped in yesterday's emotional state.

### Autosave

The autosave mechanism writes to disk after every significant appraisal
(PAD Euclidean delta > 0.15 from last save). This is best-effort -- if the
write fails, the agent continues operating and retries on the next save
opportunity. The goal is crash resilience, not transactional durability.

```rust
pub fn autosave(&self) -> Result<()> {
    if let Some(ref path) = self.persistence_path {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
    }
    Ok(())
}
```

---

## Integration with Runner

The plan runner loads `DaimonState` at startup and passes it through the
dispatch path:

```
1. PlanRunner::start()
   -> DaimonState::load(".roko/daimon/affect.json")
   -> Apply elapsed-time decay

2. Per-task dispatch:
   -> DaimonState::appraise(event)         // gate results, task outcomes
   -> DaimonState::modulate(&mut params)   // adjust model, turns, strategy
   -> DaimonState::query()                 // read current state for prompts
   -> SomaticLandscape::query(coords)      // pre-analytical heuristic
   -> Yerkes-Dodson ceiling check          // complexity constraint

3. Post-task:
   -> DaimonState::appraise(TaskOutcome)   // success/failure appraisal
   -> CognitiveEnergy::deplete(activity)   // energy cost
   -> CognitiveEnergy::recover(Gamma)      // inter-task recovery
   -> DaimonState::autosave()              // persist

4. Between waves:
   -> CognitiveEnergy::recover(Theta)      // extended recovery

5. Dream trigger (Resting state):
   -> CognitiveEnergy::recover(Delta)      // full reset
   -> SomaticLandscape::rebuild()          // consolidate markers
   -> DaimonState::autosave()
```

### SystemPromptBuilder Integration

The Daimon's state is injected into the agent's system prompt through the
9-layer `SystemPromptBuilder`. The `<daimon>` section includes:

```xml
<daimon>
  behavioral_state: Struggling
  confidence: 0.35
  pad: P:-0.40 A:0.25 D:-0.30
  energy: 0.62/1.00 (fatigue: 0.28)
  complexity_ceiling: standard
  recommendation: escalate to stronger model, consider re-planning
</daimon>
```

This context block is assembled by `roko-compose` and injected at a position
determined by the VCG auction. Under high arousal, the Daimon section bids
higher, making the emotional context more likely to be included in the
limited context window.

---

## Trait Implementations

`DaimonState` implements several important traits:

### Serialize / Deserialize

Full serde support with `#[serde(default)]` on all optional fields. This
ensures backward compatibility when new fields are added -- an older
persisted state can be deserialized into a newer struct without errors,
with new fields taking their defaults.

The `persistence_path` field is `#[serde(skip)]` because it is a runtime
configuration, not persisted state.

### Clone

`DaimonState` is `Clone`, which is necessary for snapshot-based state
management in the plan runner. The runner can clone the state before a
speculative operation and restore it if the operation fails.

### PartialEq

`DaimonState` derives `PartialEq` for testing. This enables assertions
like `assert_eq!(state_before, state_after)` in unit tests that verify
appraisal idempotency and serialization round-trips.

### Default

`DaimonState::default()` delegates to `DaimonState::new()`, producing the
neutral initial state described above.

---

## Sub-Component Summary

| Component | Type | Purpose | See Also |
|---|---|---|---|
| `state` | `AffectState` | PAD + confidence + behavioral state + ALMA | `pad-vector.md`, `alma-three-layer-temporal.md` |
| `somatic_landscape` | `SomaticLandscape` | k-d tree of emotional markers | `somatic-markers-damasio.md` |
| `strategy_space` | `StrategySpaceDefinition` | 8D axis definitions | `8-dimensional-strategy-space.md` |
| `crate_trackers` | `HashMap<String, CrateConfidence>` | Per-crate affect | `coding-agent-integration.md` |
| `contrarian_tracker` | `ContrarianTracker` | 15% opposite-valence floor | `15-percent-contrarian-retrieval.md` |
| `error_patterns` | `ErrorPatternTracker` | Error familiarity scaling | `coding-agent-integration.md` |
| `fatigue_detector` | `FatigueDetector` | Consecutive-failure detection | `coding-agent-integration.md` |
| `borrowed_affect` | `Vec<BorrowedAffect>` | Peer contagion entries | `collective-emotional-contagion.md` |
| `behavioral_tracker` | `BehavioralStateTracker` | Hysteresis + dwell time | `six-behavioral-states.md` |
| `vitality_tracker` | `VitalityTracker` | Budget-derived lifecycle phase | `energy-accounting.md` |
| `cognitive_energy` | `CognitiveEnergy` | Depletable energy pool | `energy-accounting.md` |
| `goal_tree` | `GoalTree` | Emergent goal lifecycle | (E23 specification) |
| `goal_seeds` | `Vec<GoalSeed>` | Pattern-to-goal promotion | (E23 specification) |
| `novelty_filter` | `NoveltyFilter` | Suppress repeated triggers | (P1-04) |

---

## Error Handling

| Error | Cause | Response |
|---|---|---|
| Persistence path not set | Agent created without persistence | Autosave silently skips |
| Deserialization failure | Corrupt or incompatible JSON | Start with default state, log warning |
| NaN in PAD values | Buggy appraisal arithmetic | Clamp to neutral, log warning |
| Non-finite energy | Depletion overflow | Clamp to 0.0, enter conservation |
| Missing fields in JSON | Older schema version | `#[serde(default)]` fills in defaults |

---

## Academic Foundations

- Mehrabian, A. (1996). "Pleasure-arousal-dominance: A general framework for
  describing and measuring individual differences in temperament."
  *Current Psychology*, 14(4), 261-292.
- Gebhard, P. (2005). "ALMA -- A Layered Model of Affect." *AAMAS*, 29-36.
- Ortony, A., Clore, G.L., & Collins, A. (1988). *The Cognitive Structure
  of Emotions*. Cambridge University Press.
- Damasio, A.R. (1994). *Descartes' Error*. Putnam.
- Scherer, K.R. (2001). "Appraisal considered as a process of multilevel
  sequential checking." In Scherer, Schorr, & Johnstone (Eds.), *Appraisal
  Processes in Emotion*. Oxford University Press.

---

## Cross-References

- `pad-vector.md` -- PAD vector structure and octant classification
- `alma-three-layer-temporal.md` -- ALMA three-layer temporal model
- `occ-scherer-appraisal.md` -- appraisal rules that drive the pipeline
- `six-behavioral-states.md` -- behavioral state classification
- `energy-accounting.md` -- CognitiveEnergy, RecoveryModes, Yerkes-Dodson
- `somatic-markers-damasio.md` -- somatic landscape details
- `integration-points.md` -- four integration points for PAD state
