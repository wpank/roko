# Learning Loop: Built But Disconnected

## Current Data Flow (What Actually Happens)

```
                            RUNTIME (orchestrate.rs)
                            ========================
    dispatch_agent_with()           run_gate_pipeline()          record_completed_run()
         |                               |                              |
         v                               v                              v
 +------------------+          +------------------+          +---------------------+
 | CascadeRouter    |          | GatePipeline     |          | LearningRuntime     |
 | .select_model()  |          | compile/test/    |          | (runtime_feedback)  |
 |                  |          | clippy/diff      |          |                     |
 +--------+---------+          +--------+---------+          +----------+----------+
          |                             |                               |
          v                             v                               v
  .roko/learn/                  gate_passed: bool                WRITES ALL OF:
  cascade-router.json           (used inline)                    - episodes.jsonl
  routing-decisions.jsonl                                        - efficiency.jsonl
                                                                 - costs.jsonl
                                                                 - playbooks/ (outcome)
                                                                 - cascade-router.json
                                                                 - experiments.json
                                                                 - gate-thresholds.json
                                                                 - section-effects.json
                                                                 - post-gate-reflections.json
                                                                 - provider-model-outcomes.jsonl

                              WHAT READS BACK?
                              ================

  +--- CascadeRouter reads cascade-router.json on startup (model stats) -----+
  |                                                                           |
  +--- PlaybookStore queries playbooks/ dir for prompt enrichment -----------+
  |                                                                           |
  +--- ExperimentStore reads experiments.json for variant assignment ---------+
  |                                                                           |
  +--- PostGateReflection loads reflections for retry context ---------------+
  |                                                                           |
  +--- CalibrationTracker CAN load from routing-decisions.jsonl, but --------+---> NEVER CALLED
  |    no runtime code invokes load_from_routing_log()                       |     AT DISPATCH
  |                                                                           |
  +--- SectionOutcomeStore writes section-outcomes.jsonl, but ---------------+---> NEVER READ
  |    no bandit or policy consumes these records                             |     BACK
  |                                                                           |
  +--- event_subscriber.rs run_learning_subscriber() processes events -------+---> NEVER
       but has ZERO non-test callers in the runtime                                STARTED
```

## Complete Disconnection Inventory

### Break 1: Playbook Outcomes Correctly Wired (REVISED from original)

**Status**: WIRED (was broken, now fixed)

**What happens now**: `LearningRuntime::record_completed_run()` at
`crates/roko-learn/src/runtime_feedback.rs:2230-2238` calls
`playbook_store.record_outcome(&playbook_id, success)` when the episode's
`extra` map contains `"playbook_id"`. The `playbook_id` is populated by the
orchestrator when a playbook match is found at dispatch time.

**Data flow**:
- Source: `episode.extra["playbook_id"]` set by `orchestrate.rs`
- Sink: `PlaybookStore::record_outcome()` updates `playbook.successes`/`failures`
- Persistence: individual playbook JSON files under `.roko/learn/playbooks/`
- Read-back: `PlaybookStore::query()` ranks playbooks by score at next dispatch

**Remaining gap**: Playbook *rule* confidence is updated
(`playbook_rules.record_outcome` at line 2244-2248), but the
`playbook_rules.toml` confidence values do not yet feed back into the
cascade router's model selection. The rule confidence is persisted but only
affects playbook step ordering, not model routing.

### Break 2: CalibrationPolicy Corrections Discarded (Task 031)

**What should happen**: `CalibrationPolicy` predicts model success probability
before dispatch. After the gate result, it computes a `CalibrationCorrection`
containing the mean bias and a correction factor. This correction should adjust
the cascade router's confidence statistics or the next prediction's prior.

**What actually happens in `event_subscriber.rs:98-105`**:
```rust
if let Some(correction) = calibration_policy.process_event(&event) {
    tracing::info!(
        model = %correction.model,
        category = %correction.category,
        bias = correction.mean_bias,
        "calibration correction triggered"
    );
    // That's it. No router update. Logged and discarded.
}
```

**No `apply_calibration_correction()` method exists on `CascadeRouter`.**

**Additionally**, `run_learning_subscriber()` itself has zero non-test callers.
The function is never spawned from the runtime event loop. This means:
- `CalibrationPolicy` never processes events in production
- `VerdictHistory` in the subscriber never records verdicts for routing penalty
- The `ProviderHealthRegistry` and `LatencyRegistry` updates in the subscriber
  are duplicated by `LearningRuntime::record_completed_run()` anyway

**Data written**: `CalibrationCorrection { model, category, mean_bias, correction, sample_count }`

**Where it should flow**:
1. `CascadeRouter::adjust_confidence_prior(model, correction)` -- does not exist
2. `CalibrationTracker::adjust_prediction(model, category, raw_pred)` -- exists
   but is never called from the dispatch path

**Consuming code that exists but is not connected**:
- `CalibrationTracker::adjust_prediction()` at `prediction.rs:289-291` can
  subtract mean bias from raw predictions
- `CalibrationTracker::summary()` at `prediction.rs:367-382` can produce a
  `PredictionCalibrationSummary` that includes accuracy, coverage, mean_bias
- `load_predictive_calibration()` at `orchestrate.rs:383-388` loads a
  `CalibrationTracker` from `routing-decisions.jsonl` and builds a prompt
  section -- but this only inserts calibration data into the *prompt text*,
  it does not modify the router's scoring

**Exact code changes needed**:
1. Add to `CascadeRouter`:
   ```rust
   pub fn apply_calibration_correction(
       &self,
       model_slug: &str,
       correction: f64,  // negative = was overconfident
   ) {
       let mut stats = self.confidence_stats.lock();
       if let Some(entry) = stats.get_mut(model_slug) {
           // Adjust the confidence-stage pass rate by the correction
           // This feeds into stage 2 (Confidence) scoring
           entry.calibration_offset = correction;
       }
   }
   ```
2. In `event_subscriber.rs:98-105`, after the `tracing::info!`:
   ```rust
   router.apply_calibration_correction(&correction.model, correction.correction);
   ```
3. Spawn `run_learning_subscriber()` from the orchestrate.rs run loop, or
   move the calibration logic into `LearningRuntime::record_completed_run()`.

### Break 3: DemurrageConsumer Constructed But Never Ticked (Task 032)

**What should happen**: `DemurrageConsumer` governs the cadence of knowledge
decay with configurable validation intervals and domain-specific multipliers.

**What actually happens**:
```rust
let _consumer = DemurrageConsumer::new(...); // created and dropped
// Raw timer below, ignoring consumer's interval logic
```

**Partial credit**: `KnowledgeStore::apply_demurrage()` IS called on a 5-min
interval. Knowledge entries DO decay. But the `DemurrageConsumer`'s
configuration (validation_interval, domain multipliers) is dead code.

**Data written**: Demurrage values in neuro store entries

**Where it should flow**: The `DemurrageConsumer` should wrap the existing
5-min timer and apply domain-specific multipliers from config.

**Impact**: Low. Knowledge decay works, just with hardcoded intervals instead
of configurable ones. This is a configuration gap, not a feedback loop break.

### Break 4: Event Subscriber Has No Runtime Caller

**File**: `crates/roko-learn/src/event_subscriber.rs`

**Header**:
```rust
//! STATUS: NOT WIRED -- built but no non-test runtime caller.
```

**What it does when called (in tests)**: Subscribes to a `broadcast::Receiver<AgentEvent>` and:
1. Tracks `ActiveTurn` state across `TurnStarted`/`TurnCompleted` pairs
2. Records `ProviderHealth` success/failure
3. Records `CascadeRouter::record_confidence_outcome`
4. Creates and appends `AgentEfficiencyEvent` to JSONL
5. Processes `CalibrationPolicy` events (predict-publish-correct)
6. Records `VerdictHistory` entries from `GateResult` events
7. Records `LatencyRegistry` observations from `ToolCallExecuted` events
8. Checks `AnomalyDetector` on `CostRecorded` events

**Why it is not needed at the moment**: Most of these updates are already
performed by `LearningRuntime::record_completed_run()` and by the inline
event-bus draining in `publish_turn_learning_feedback()` in
`learning_helpers.rs`. The subscriber would be redundant EXCEPT for:
- CalibrationPolicy processing (not done elsewhere)
- VerdictHistory recording (not done elsewhere)

**Exact change needed**: Either:
(a) Spawn `run_learning_subscriber()` as a background task alongside the
    plan runner, OR
(b) Move CalibrationPolicy and VerdictHistory logic into
    `record_completed_run()` (preferred -- avoids dual-write races)

### Break 5: Predict-Publish-Correct Not Fully Implemented (Task 100)

**What should happen**: Before dispatching an agent, record a prediction:
"model X will succeed on task type Y with probability P." After the gate
result, compare prediction to reality and update confidence.

**What partially exists**:
- `PredictionRecord::register()` at `prediction.rs:67-92` -- creates
  an unresolved prediction record
- `PredictionRecord::resolve()` at `prediction.rs:95-104` -- fills in
  actuals and computes residuals
- `CalibrationTracker::record_prediction()` at `prediction.rs:166-174` --
  ingests resolved predictions
- `CalibrationTracker::from_routing_logs()` at `prediction.rs:196-202` --
  replays routing-decision JSONL into calibration state
- `CalibrationTracker::adjust_prediction()` at `prediction.rs:289-291` --
  bias-corrects raw predictions
- `ResidualCorrector` at `prediction.rs:527-663` -- O(1) streaming bias
  correction with interval calibration
- `CalibrationPolicy::process_event()` at `calibration_policy.rs:89-162` --
  matches TurnStarted predictions to TurnCompleted outcomes

**What is missing from the runtime dispatch path**:
1. No `register_prediction()` call before `dispatch_agent_with()` in
   orchestrate.rs. The `CalibrationPolicy` uses a hardcoded 0.7 prior
   instead of the router's actual confidence score.
2. No `resolve_prediction()` call after gate result in orchestrate.rs.
   The resolution only happens inside the unwired `event_subscriber.rs`.
3. The `RoutingDecisionLogStore` is written to (outcomes are appended),
   but `CalibrationTracker::load_from_routing_log()` is only called at
   startup for the prompt calibration section -- it never feeds corrections
   back into the router's scoring.

**Data written**:
- `.roko/learn/routing-decisions.jsonl` -- RoutingDecisionLog records
  with outcome fields populated after task completion
- `.roko/learn/cascade-router.json` -- confidence stats (trials/successes)

**Where it should flow**:
1. At dispatch: `CalibrationTracker::adjust_prediction(model, category, raw_score)`
   should modify the cascade router's candidate scoring in `select_model()`
2. After gate: `CalibrationTracker::record_residual(model, category, residual)`
   should update the tracker that adjusts future predictions

**Exact code changes needed**:
1. In `CascadeRouter::select_model()`, after computing candidate scores,
   call `calibration.adjust_prediction(slug, category, score)` to correct
   for historical bias
2. In `record_completed_run()`, after appending the routing decision outcome,
   call `calibration_tracker.record_routing_decision(&completed_routing_log)`
3. Persist the CalibrationTracker alongside the CascadeRouter snapshot

### Break 6: Section Outcome Data Not Consumed (Task 034)

**What should happen**: Track which prompt sections led to success/failure so
a contextual bandit can learn which sections to include for which task types.

**What exists**:
- `SectionOutcomeRecord` at `section_outcome.rs:119-173` -- rich per-section
  observation record with gate outcomes and review verdicts
- `SectionOutcomeStore` at `section_outcome.rs:362-442` -- append-only JSONL
  persistence with read_all
- `summarize_section_outcomes()` at `section_outcome.rs:514-539` -- aggregation
  by action_id with pass rates
- `SectionEffectivenessRegistry` at `section_effect.rs` -- tracks lift per
  section (included pass rate vs. excluded pass rate)
- `FeedbackService` at `feedback_service.rs` -- records section effectiveness

**What is wired**:
- `SectionEffectivenessRegistry` IS loaded, updated, and saved via
  `FeedbackService` and `LearningRuntime::record_completed_run()` (through
  `append_derived_episode_feedback`). Section effectiveness data IS persisted
  to `.roko/learn/section-effects.json`.

**What is NOT wired**:
1. The `SectionEffectivenessRegistry` IS persisted but prompt assembly does
   not read it back. The `SectionEffect::budget_weight()` method exists and
   returns multiplicative adjustments, but `SystemPromptBuilder` does not
   call it.
2. `SectionOutcomeStore` writes to `.roko/learn/section-outcomes.jsonl` but
   no runtime code reads the summaries to adjust section priorities.
3. The `contextual_bandit.rs` module exists with a `ContextualBanditPolicy`
   but it is never instantiated with section outcome data.

**Where it should flow**:
- `section_effect.rs` lift values --> `SystemPromptBuilder` section priority
- `section_outcome.rs` pass rates --> contextual bandit arm selection

**Exact code changes needed**:
1. In `SystemPromptBuilder::build()` or the prompt composition path in
   orchestrate.rs, load `SectionEffectivenessRegistry` and call
   `effect.budget_weight()` to adjust per-section token budgets
2. Wire `SectionOutcomeStore::read_all()` into the section bandit
   initialization so it can use historical pass rates as priors

## What Actually Works in the Learning System

Despite the breaks above, these learning components ARE wired and functional:

| Component | Status | Data Path | Read-back |
|-----------|--------|-----------|-----------|
| `EpisodeLogger` | WIRED | `.roko/episodes.jsonl` | Yes: dream consolidation, pattern mining |
| `EfficiencyTracker` | WIRED | `.roko/learn/efficiency.jsonl` | Yes: efficiency signals injected into conductor checks, cost overrun detection |
| `CascadeRouter` (basic) | WIRED | `.roko/learn/cascade-router.json` | Yes: loaded at startup, updated per-episode, persisted immediately |
| `ExperimentStore` | WIRED | `.roko/learn/experiments.json` | Yes: variant assignment at dispatch, concluded winners update static routing table |
| `AdaptiveGateThresholds` | WIRED | `.roko/learn/gate-thresholds.json` | Yes: EMA per rung, loaded at startup, updated per-gate |
| `PostGateReflection` | WIRED | `.roko/learn/post-gate-reflections.json` | Yes: lessons from failures loaded into retry context |
| `PlaybookStore` | WIRED | `.roko/learn/playbooks/*.json` | Yes: queried at dispatch, outcomes recorded after gate |
| `PlaybookRules` | WIRED | `.roko/learn/playbook-rules.toml` | Yes: confidence updated per-outcome, persisted |
| `SkillLibrary` | WIRED | `.roko/learn/skills.json` | Yes: extracted from episodes, matched at dispatch, outcomes recorded |
| `LocalRewardFunction` | WIRED | `.roko/learn/local-rewards.json` | Yes: Optimas-style (local_decision, global_outcome) recorded per subsystem |
| `ProviderHealth` | WIRED | In-memory + persisted | Yes: circuit breaker state affects routing |
| `ProviderModelOutcomes` | WIRED | `.roko/learn/provider-model-outcomes.jsonl` | Partial: written but only read by dashboard routes |
| `LatencyRegistry` | WIRED | `.roko/learn/latency-stats.json` | Yes: affects routing latency SLA |
| `CostsDb / CostsLog` | WIRED | `.roko/learn/costs.jsonl` | Yes: cost tracking, regression detection |
| `SectionEffectiveness` | PARTIALLY | `.roko/learn/section-effects.json` | Written but not read by prompt assembly |
| `RoutingDecisionLog` | PARTIALLY | `.roko/learn/routing-decisions.jsonl` | Written with outcomes, read only at startup for prompt calibration section |
| `CalibrationTracker` | PARTIALLY | Derived from routing-decisions.jsonl | Loaded at startup for prompt text, never modifies scoring |

## Complete Learning Loop Architecture

### Data Ingestion

#### Events Captured

| Event | Where Captured | Format | File |
|-------|---------------|--------|------|
| Agent turn completion | `orchestrate.rs` -> `record_completed_run()` | `Episode` JSON | `.roko/episodes.jsonl` |
| Per-turn cost/tokens | `orchestrate.rs` -> `flush_efficiency_events()` | `AgentEfficiencyEvent` JSON | `.roko/learn/efficiency.jsonl` |
| Model routing decision | `cascade_router.rs` -> `RoutingLogger` | `RoutingDecisionLog` JSON | `.roko/learn/routing-decisions.jsonl` |
| Gate outcome | `record_completed_run()` -> `append_derived_episode_feedback()` | Gate outcome JSON | `.roko/learn/gate-outcomes.jsonl` |
| Cost record | `record_completed_run()` | `CostRecord` JSON | `.roko/learn/costs.jsonl` |
| Provider model outcome | `record_completed_run()` | `ProviderModelOutcomeRecord` JSON | `.roko/learn/provider-model-outcomes.jsonl` |
| Section effectiveness | `FeedbackService` | `SectionEffect` JSON snapshot | `.roko/learn/section-effects.json` |
| Prompt experiment outcome | `record_completed_run()` | Experiment stats | `.roko/learn/experiments.json` |

### Data Storage

| File | Schema | Rotation | Current Size Mgmt |
|------|--------|----------|-------------------|
| `.roko/episodes.jsonl` | Append-only JSONL, one `Episode` per line | `jsonl_rotation.rs` size-based (10MB default) | Tolerable; malformed lines skipped on read |
| `.roko/learn/efficiency.jsonl` | Append-only JSONL, one `AgentEfficiencyEvent` per line | None (grows unbounded) | Read as Vec, could OOM on huge runs |
| `.roko/learn/cascade-router.json` | Atomic-write JSON snapshot | N/A (overwritten each update) | Small (model count * stats) |
| `.roko/learn/experiments.json` | Atomic-write JSON snapshot | N/A (overwritten each update) | Small (experiment count * variant stats) |
| `.roko/learn/gate-thresholds.json` | Atomic-write JSON snapshot | N/A (overwritten each update) | Small (rung count * EMA) |
| `.roko/learn/routing-decisions.jsonl` | Append-only JSONL | None | Grows with task count |
| `.roko/learn/section-effects.json` | Atomic-write JSON snapshot | N/A | Small |
| `.roko/learn/post-gate-reflections.json` | Atomic-write JSON, capped at 1000 records + 256 candidates | N/A | Self-limiting |
| `.roko/learn/playbooks/*.json` | One JSON file per playbook | N/A | Grows with playbook count |

### Data Analysis (What Should Be Computed)

| Analysis | Input | Output | Consumer |
|----------|-------|--------|----------|
| Pass rate per model/category | `routing-decisions.jsonl` | `CalibrationTracker` residuals | Router scoring adjustment |
| Section lift per role | `section-effects.json` | `PriorityChange` enum per section | Prompt assembly token budgets |
| Efficiency grade per turn | `efficiency.jsonl` | `Grade` A-D | Conductor intervention policy |
| Role cost profile | `efficiency.jsonl` | `RoleCostProfile` per role | Dashboard, budget alerts |
| Fleet C-Factor | `efficiency.jsonl` | `FleetCFactor` composite score | Run summary, trend tracking |
| Regression detection | `task-metrics.jsonl` | `RegressionReport` | Alerting, conductor abort |
| Pattern discovery | Episodes | Recurring action sequences | Skill extraction |
| Playbook effectiveness | Playbook outcomes | Score ranking | Prompt enrichment priority |

### Feedback Application (How Analysis Modifies Future Behavior)

#### 1. Episode -> Playbook Extraction -> Dispatch Enrichment

**Current state**: WIRED

```
Episode logged
  |
  v
record_completed_run()
  |
  +---> playbook_store.record_outcome(playbook_id, success)  [WIRED]
  |        updates playbook.successes / .failures
  |
  +---> skill_library.extract(&episode, &generator)          [WIRED]
  |        extracts reusable skills from successful episodes
  |        (every 10th episode via update_frequency)
  |
  +---> pattern_miner.ingest_episode(&actions)               [WIRED]
           mines recurring action sequences (every 20th episode)

Next dispatch:
  |
  +---> playbook_store.query(&QueryContext)                   [WIRED]
  |        returns top-scored playbooks for this task type
  |        injected into system prompt via playbook_query_context()
  |
  +---> skill_library.get(&skill_id)                         [WIRED]
           matched skills injected into prompt
```

**Call sites in orchestrate.rs**:
- `playbook_query_context()` in `learning_helpers.rs:513-530`
- `build_task_playbook()` in `learning_helpers.rs:532-557`
- `load_or_create_playbook_store()` in `learning_helpers.rs`
- `record_completed_run()` in `runtime_feedback.rs:2230-2238`

#### 2. Efficiency -> Model Routing Adjustments

**Current state**: PARTIALLY WIRED

```
AgentEfficiencyEvent written to .roko/learn/efficiency.jsonl
  |
  v
load_efficiency_signals_sync()                                [WIRED]
  |   reads efficiency.jsonl, builds conductor signals
  |   (cost overrun detection, efficiency grades)
  v
Conductor checks: high cost -> abort/pause decision           [WIRED]

BUT:
  |
  v
compute_role_profiles() / compute_fleet_cfactor()             [WIRED for summary]
  |   called at end of run for fleet C-Factor summary
  v
  DOES NOT feed back into CascadeRouter model scoring         [BROKEN]
  (efficiency-based model demotion/promotion not implemented)
```

**What should happen additionally**:
- Models with consistently low efficiency grades (D-grade) should receive
  a negative routing bias in the cascade router
- Models with high cache hit rates should receive an affinity bonus
- This requires wiring `compute_role_profiles()` output into
  `CascadeRouter::RoutingBias`

#### 3. Gate Outcomes -> Threshold Adaptation

**Current state**: WIRED

```
Gate result (passed/failed, rung index)
  |
  v
AdaptiveGateThresholds.update(rung, passed)                   [WIRED]
  |   EMA update of per-rung pass rate
  v
gate-thresholds.json persisted                                [WIRED]
  |
  v
Next gate evaluation loads thresholds                         [WIRED]
  |   threshold for each rung adapts based on historical pass rate
  v
Higher-rung gates can be skipped when historical pass rate    [WIRED]
  is very high (adaptive skip threshold)
```

**Call site**: `orchestrate.rs` gate pipeline, `enrich_rung_config()`

#### 4. Section Outcomes -> Prompt Template Selection

**Current state**: BROKEN (data written, not read back)

```
SectionOutcomeRecord written to section-outcomes.jsonl         [WIRED]
SectionEffectivenessRegistry written to section-effects.json   [WIRED]
  |
  v
SectionEffect.lift() computes included vs excluded pass rate   [EXISTS]
SectionEffect.budget_weight() returns multiplicative factor    [EXISTS]
  |
  v
SystemPromptBuilder does NOT call budget_weight()              [BROKEN]
contextual_bandit.rs is never instantiated with outcomes       [BROKEN]
```

**What should happen**:
1. `SystemPromptBuilder::build()` should load the `SectionEffectivenessRegistry`
2. For each prompt section, call `registry.get(section_name, role)`
3. Apply `effect.budget_weight()` as a multiplier on the section's token budget
4. Sections with negative lift get reduced budgets; positive lift gets more budget

**Exact integration point**:
```rust
// In orchestrate.rs, when building the system prompt:
let section_registry = SectionEffectivenessRegistry::load_or_new(
    &learning.paths().section_effects_json
);
for section in &mut prompt_sections {
    if let Some(effect) = section_registry.get(&section.name, &role) {
        section.budget = (section.budget as f64 * effect.budget_weight()) as usize;
    }
}
```

#### 5. Calibration Predictions -> Accuracy Tracking -> Router Updates

**Current state**: BROKEN (the full predict-publish-correct loop)

```
DESIRED FLOW:

  CascadeRouter.select_model()
       |
       v
  PredictionRecord::register(
       task_id, model, category,
       predicted_success_prob,   <-- from router score
       predicted_cost_usd,
       predicted_duration_ms)
       |
       v
  [Agent executes task]
       |
       v
  Gate result: passed/failed
       |
       v
  prediction.resolve(actual_success, cost, duration)
       |
       v
  CalibrationTracker.record_prediction(&prediction)
       |
       v
  CalibrationTracker.adjust_prediction(model,cat,raw)
       |  returns bias-corrected score
       v
  CascadeRouter uses corrected score next time

ACTUAL FLOW:
  CascadeRouter.select_model()
       |
       v
  [Agent executes task]
       |
       v
  record_completed_run() -> update_cascade_router()
       |
       v
  router.record_confidence_outcome(slug, success)   [WIRED: binary only]
  router.observe(context_vec, model_idx, reward)     [WIRED: LinUCB bandit]
       |
       v
  cascade-router.json updated                        [WIRED]

  BUT: No prediction registered, no residual tracked,
  no bias correction applied to future scores.
```

## Integration Design: Closing All Loops

### Phase 1: Wire CalibrationTracker into Dispatch (Highest Impact)

**Scope**: Make the cascade router's scoring benefit from historical accuracy data.

```rust
// --- crates/roko-learn/src/cascade_router.rs ---

impl CascadeRouter {
    /// Apply a calibration adjustment to candidate scoring.
    ///
    /// Called during `select_model()` after computing base candidate scores.
    /// The calibration tracker's mean bias for each candidate's model/category
    /// pair is subtracted from the score, correcting systematic over/under-
    /// confidence.
    pub fn apply_calibration(
        &self,
        candidates: &mut [CascadeRoutingCandidate],
        calibration: &CalibrationTracker,
        task_category: &str,
    ) {
        for candidate in candidates.iter_mut() {
            let bias = calibration.mean_bias(&candidate.model, task_category);
            if bias.abs() > 0.05 && calibration.sample_count(&candidate.model, task_category) >= 10 {
                candidate.score -= bias;
                tracing::debug!(
                    model = %candidate.model,
                    category = task_category,
                    bias,
                    adjusted_score = candidate.score,
                    "applied calibration correction to candidate score"
                );
            }
        }
    }
}
```

**Call site in orchestrate.rs** (inside model selection):
```rust
// After building candidates but before final selection:
if let Some(ref calibration) = self.calibration_tracker {
    self.cascade_router.apply_calibration(&mut candidates, calibration, &task_category);
}
```

### Phase 2: Wire Section Effectiveness into Prompt Assembly

**Scope**: Make prompt sections that hurt pass rate get less token budget.

```rust
// --- crates/roko-cli/src/orchestrate.rs, in dispatch_agent_with() ---

// Load section effectiveness and adjust token budgets
let section_registry = SectionEffectivenessRegistry::load_or_new(
    &self.learning.paths().section_effects_json
);
for section in prompt_builder.sections_mut() {
    let weight = section_registry
        .get(&section.name, &role.label())
        .map(|effect| effect.budget_weight())
        .unwrap_or(1.0);
    section.max_tokens = ((section.max_tokens as f64) * weight) as usize;
}
```

### Phase 3: Move CalibrationPolicy into record_completed_run()

**Scope**: Avoid the unwired event subscriber entirely.

```rust
// --- crates/roko-learn/src/runtime_feedback.rs ---

impl LearningRuntime {
    pub async fn record_completed_run(&self, mut input: CompletedRunInput) -> Result<LearningUpdate> {
        // ... existing code ...

        // After recording the routing decision outcome:
        if let Some(routing_log) = self.latest_routing_decision.take() {
            let completed_log = routing_log.with_outcome(
                input.episode.success,
                cost_usd,
                duration_ms,
            );
            self.calibration_tracker.record_routing_decision(&completed_log);
        }

        // ... rest of existing code ...
    }
}
```

### Phase 4: Efficiency-Based Routing Bias

**Scope**: Models with consistently poor efficiency grades get demoted.

```rust
// --- In the periodic summary computation (every N episodes) ---

let role_profiles = compute_role_profiles(&efficiency_events);
for profile in &role_profiles {
    if profile.pass_rate < 0.3 && profile.observations >= 10 {
        // Model consistently failing for this role -- apply negative bias
        let bias = RoutingBias {
            model_slug: profile.role_model.clone(),
            bias: -0.2,
            reason: format!(
                "low pass rate ({:.0}% over {} obs)",
                profile.pass_rate * 100.0,
                profile.observations
            ),
        };
        cascade_router.apply_routing_bias(&bias);
    }
}
```

## Test Strategy for Verifying Loop Closure

### Unit Tests

1. **CalibrationTracker -> Router scoring**:
   ```rust
   #[test]
   fn calibration_correction_affects_model_selection() {
       let mut tracker = CalibrationTracker::default();
       // Record 20 overconfident predictions for model-a
       for _ in 0..20 {
           tracker.record_residual("model-a", "impl", 0.3); // 0.3 bias
       }
       let raw_score = 0.8;
       let adjusted = tracker.adjust_prediction("model-a", "impl", raw_score);
       assert!((adjusted - 0.5).abs() < 0.01);
   }
   ```

2. **Section effectiveness -> budget adjustment**:
   ```rust
   #[test]
   fn section_effectiveness_adjusts_token_budget() {
       let mut registry = SectionEffectivenessRegistry::new();
       // Section "workspace_map" has 80% pass when included, 40% when excluded
       let mut effect = SectionEffect::new("workspace_map");
       for _ in 0..80 { effect.record(true, true); }
       for _ in 0..20 { effect.record(true, false); }
       for _ in 0..40 { effect.record(false, true); }
       for _ in 0..60 { effect.record(false, false); }
       registry.insert("workspace_map", "implementer", effect);

       let weight = registry.get("workspace_map", "implementer")
           .map(|e| e.budget_weight())
           .unwrap_or(1.0);
       assert!(weight > 1.0, "positive lift should increase budget");
   }
   ```

3. **Playbook outcome -> query ranking**:
   ```rust
   #[tokio::test]
   async fn playbook_outcome_affects_ranking() {
       let store = PlaybookStore::new(tempdir);
       let mut pb = Playbook::new("fix-imports", "Fix import errors");
       store.save(&pb).await.unwrap();
       // Record 10 successes
       for _ in 0..10 {
           store.record_outcome("fix-imports", true).await.unwrap();
       }
       let results = store.query(&ctx).await;
       assert!(results[0].score > 0.5);
   }
   ```

### Integration Tests

1. **Full dispatch -> gate -> learn -> next dispatch** roundtrip:
   - Dispatch with CascadeRouter
   - Run gate (mock)
   - Call `record_completed_run()`
   - Verify router state changed
   - Dispatch again and verify model selection differs

2. **Efficiency event -> conductor signal** roundtrip:
   - Write efficiency events with high cost
   - Load efficiency signals
   - Verify cost overrun signal present

3. **Experiment conclusion -> static table update**:
   - Register experiment with two variants
   - Record enough outcomes to conclude
   - Verify `on_experiment_concluded()` updates routing table
   - Verify next `select_model()` uses winner

## Metrics to Prove the Loop Is Working

### Operational Metrics (Can Be Measured Now)

| Metric | Source | Target | Meaning |
|--------|--------|--------|---------|
| Cascade router observation count | `cascade-router.json` | > 0 per model | Router is receiving outcomes |
| Playbook outcome count | `playbooks/*.json` | successes + failures > 0 | Playbook feedback flowing |
| Experiment convergence | `experiments.json` | At least one concluded | A/B tests reaching significance |
| Gate threshold drift | `gate-thresholds.json` | EMA != initial value | Thresholds adapting |
| Reflection record count | `post-gate-reflections.json` | > 0 after failures | Post-gate learning active |
| Efficiency event count | `efficiency.jsonl` | 1 per agent turn | Telemetry flowing |

### Learning Quality Metrics (Require Loop Closure)

| Metric | Source | Target | Meaning |
|--------|--------|--------|---------|
| Calibration Brier score | `CalibrationTracker::brier_score()` | < 0.15 | Predictions becoming accurate |
| Pass rate trend (7-day) | `provider-model-outcomes.jsonl` | Positive slope | System improving |
| Cost per pass trend | `efficiency.jsonl` | Decreasing | Routing becoming efficient |
| Section lift significance | `section-effects.json` | |lift| > 0.1 with 100+ trials | Sections evaluated with enough data |
| Playbook score variance | `playbooks/*.json` | > 0 across playbooks | Not all playbooks equally scored |
| Model selection diversity | `routing-decisions.jsonl` | Shannon entropy > 0.5 | Router exploring, not stuck |

### Dashboard Commands to Verify

```bash
# Check cascade router state
cargo run -p roko-cli -- learn all

# Check gate threshold adaptation
cargo run -p roko-cli -- learn tune gates

# Check efficiency telemetry
cargo run -p roko-cli -- learn efficiency

# Check experiment status
cargo run -p roko-cli -- learn experiments

# Check episode count
cargo run -p roko-cli -- learn episodes
```

## Summary

The learning system is a **data recorder with partial feedback**, not a
**complete feedback loop**. The major working loops are:

1. **Playbook outcome recording** -- WIRED end-to-end
2. **Cascade router binary outcome** -- WIRED (confidence stats update)
3. **Experiment A/B outcome** -- WIRED (conclusion updates routing table)
4. **Gate threshold adaptation** -- WIRED (EMA per rung)
5. **Post-gate reflection** -- WIRED (lessons stored for retry)
6. **Skill extraction and matching** -- WIRED (extract, match, record outcome)

The major broken loops are:

1. **Calibration bias correction** -- CalibrationTracker exists, corrections
   computed, but never applied to router scoring (Breaks 2, 4, 5)
2. **Section effectiveness -> prompt budgets** -- Data written but prompt
   assembly never reads it back (Break 6)
3. **Efficiency-based model demotion** -- Efficiency grades computed but
   never influence routing bias

Closing these three loops requires approximately:
- 1 new method on `CascadeRouter` (`apply_calibration`)
- 1 new read in prompt assembly (`SectionEffectivenessRegistry::load_or_new`)
- Moving CalibrationPolicy logic from the unwired event subscriber into
  `record_completed_run()`
- ~200 lines of integration code total
