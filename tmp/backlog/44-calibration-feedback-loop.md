# 44 — Calibration Feedback Loop Closure

**Priority**: P2 — Two of three learning feedback channels are wired but never affect routing or prompt decisions
**Size**: M (2-3 days)
**Crates**: `roko-learn` (`crates/roko-learn/`), `roko-cli` (`crates/roko-cli/`)
**Depends on**: None

---

## Background

Roko has a learning pipeline with three subsystems that compute useful signal after each agent turn and gate result. The data is produced and persisted, but two of the three signals never feed back into the decisions they are meant to improve.

The three subsystems are:

1. **Calibration residuals** (`roko-learn/src/calibration_policy.rs`): tracks how wrong cost and quality predictions were for each model. When a task runs, the actual gate pass rate is compared to the predicted pass rate; the residual is a `CalibrationCorrection` value that quantifies how over- or under-confident the predictions were.

2. **Section effectiveness** (`roko-learn/src/section_effect.rs`): tracks which prompt sections (e.g., `"knowledge"`, `"playbooks"`, `"episode_knowledge"`) correlate with gate passes and which correlate with gate failures. This is persisted to `.roko/learn/section-effects.json` and is read back into the `DispatchPromptBuilder` when building prompts. The read path is wired; the write path is not.

3. **Per-task efficiency grades** (`roko-learn/src/efficiency.rs`): a letter grade (A/B/C/D) computed from signal-per-token ratios after each turn. A model that consistently earns grade D is using tokens without producing useful output. These grades are appended to `.roko/learn/efficiency.jsonl` but are never fed back into routing.

All three produce real data. Only calibration corrections are forwarded to the router (via `apply_calibration_correction`), and even that path has a gap: the corrections modify `confidence_stats` but the router's scoring stage uses `linucb` arm scores and the Pareto frontier, neither of which incorporates the confidence correction.

## Current State

1. **`CascadeRouter::apply_calibration_correction` at `crates/roko-learn/src/cascade_router.rs:1326`**: modifies `self.confidence_stats` (a `Mutex<HashMap<String, ModelStats>>`) by injecting synthetic trials. The event subscriber calls this at `event_subscriber.rs:124` when a `CalibrationCorrection` fires.

2. **The routing/scoring path at `cascade_router.rs:1728`**: uses `self.linucb.score_candidates_with_alpha_adjuster(ctx, candidates, ...)` — a LinUCB arm scorer — and the Pareto frontier. It does NOT read `self.confidence_stats` when computing scores. The confidence stats are used in stage 2 (early exploration: see the `score_candidates` path for stage 2), but once the router has enough observations to reach stage 3, the LinUCB path ignores `confidence_stats`. A model with 100 injected synthetic failures from calibration corrections will score identically in LinUCB to one with none.

3. **`SectionEffectivenessRegistry` at `crates/roko-learn/src/section_effect.rs:114`**: has `record_outcome(section_name, role, included, passed)` to record a gate outcome for a prompt section, and `save(path)` to persist to disk. Read path: `crates/roko-cli/src/dispatch/prompt_cache.rs:39` loads it at startup; `crates/roko-cli/src/dispatch/prompt_builder.rs:1088` reads it as `section_effectiveness`. The write path is in `FeedbackService::persist_score_snapshots` at `crates/roko-learn/src/feedback_service.rs:516` and is only called from `roko-serve` routes. The runner's learning subscriber (`run_learning_subscriber` at `event_subscriber.rs`) does NOT call any method on `SectionEffectivenessRegistry` when observing gate outcomes.

4. **`AgentEfficiencyEvent` and `Grade` at `crates/roko-learn/src/efficiency.rs`**: `Grade` is an `A/B/C/D` enum with `numeric()` values 4/3/2/1. The event subscriber appends `AgentEfficiencyEvent` to `.roko/learn/efficiency.jsonl` at `event_subscriber.rs:163-202`. `CascadeRouter` has no field for efficiency weights and no method to accept a `Grade`. The efficiency JSONL is never read back.

5. **`PromptCache` stale check at `crates/roko-cli/src/runner/event_loop.rs:4858`**: `prompt_cache.is_stale()` is already checked in the runner's main loop. If the `section-effects.json` file is updated during a run, the next stale check will reload it. The reload mechanism works; the file just never gets updated during a run.

6. **`CascadeSnapshot` in `cascade_router.rs`**: the persisted JSON format. Any new field on `CascadeRouter` (e.g., efficiency weights) must be added to `CascadeSnapshot` and the serialization/deserialization paths.

## Implementation Plan

### Change A: apply calibration corrections in the LinUCB scoring stage

The calibration corrections are stored in `confidence_stats` as synthetic trial injections. To make them affect LinUCB scoring, add a calibration bias multiplier to the LinUCB reward when a model has strong calibration evidence.

In `cascade_router.rs`, inside the LinUCB routing branch (around line 1728), after computing `arm_scores`, apply a per-model multiplier derived from `confidence_stats`:

```rust
let confidence_bias: HashMap<String, f64> = {
    let stats = self.confidence_stats.lock();
    stats.iter()
        .map(|(slug, s)| (slug.clone(), s.pass_rate()))
        .collect()
};

let arm_scores: Vec<CandidateArmScore> = self
    .linucb
    .score_candidates_with_alpha_adjuster(ctx, candidates, |slug| {
        let bias = confidence_bias.get(slug).copied().unwrap_or(0.5);
        // Scale alpha: high-confidence models get slightly lower exploration bonus
        // (they're already known-good), while low-confidence models need more exploration.
        pareto_adjusted_alpha(base_alpha, slug, &frontier) * (1.0 + (bias - 0.5).clamp(-0.2, 0.2))
    });
```

Alternatively, after selecting a candidate, apply a post-selection confidence score re-ranking for models with strong negative calibration history (pass_rate significantly below 0.5).

Add a unit test verifying that a model with five synthetic failure injections (via `apply_calibration_correction`) scores lower than a model with five synthetic success injections.

Estimated: ~40 lines.

### Change B: wire gate outcomes into section effectiveness from the runner

In `crates/roko-learn/src/event_subscriber.rs`, inside the `AgentEvent::GateResult` handler at line 242, after recording the `VerdictRecord`, also update the `SectionEffectivenessRegistry`:

- The subscriber needs access to a `SectionEffectivenessRegistry` instance (currently not passed in). Add it as an argument to `run_learning_subscriber` or as a field on the subscriber state.
- When observing `AgentEvent::GateResult { gate_name, passed, .. }`, call `registry.record_outcome(gate_name, role, true, passed)`. The `role` is available from `active_turn.as_ref().map(|t| &t.model)` as a proxy, or from a dedicated role field if added to `ActiveTurn`.
- After recording, call `registry.save(section_effects_path)` to persist. This update triggers the existing stale-check reload in the runner at `event_loop.rs:4858`.

The `section_effects_path` should be `workdir.join(DEFAULT_SECTION_EFFECTS_PATH)` where `DEFAULT_SECTION_EFFECTS_PATH = ".roko/learn/section-effects.json"`.

Estimated: ~60 lines (subscriber argument, new field, match arm update, persist call, tests).

### Change C: efficiency grade → cascade router multiplier

Add an `efficiency_weights: Mutex<HashMap<String, f64>>` field to `CascadeRouter`. Add a method:

```rust
pub fn update_efficiency(&self, model: &str, grade: &Grade) {
    let multiplier = match grade {
        Grade::A => 1.0,
        Grade::B => 0.95,
        Grade::C => 0.85,
        Grade::D => 0.70,
    };
    let mut weights = self.efficiency_weights.lock();
    let w = weights.entry(model.to_owned()).or_insert(1.0);
    // EMA update: blend toward new multiplier
    *w = *w * 0.8 + multiplier * 0.2;
}
```

In the LinUCB scoring branch (after computing `arm_scores`), apply the efficiency weight as a final multiplier on each score:

```rust
let eff_weights = self.efficiency_weights.lock();
for score in &mut arm_scores {
    let eff = eff_weights.get(&score.slug).copied().unwrap_or(1.0);
    score.score *= eff;
}
```

In `crates/roko-learn/src/event_subscriber.rs`, after appending the `AgentEfficiencyEvent` to JSONL (line 202), compute the grade from the event:

```rust
let grade = efficiency_event.grade();
router.update_efficiency(&efficiency_event.model, &grade);
```

Add `efficiency_weights` to `CascadeSnapshot` for persistence. Add a unit test that verifies a model receiving three grade-D updates has a lower weight than one receiving three grade-A updates.

Estimated: ~80 lines (field, method, scoring integration, snapshot serialization, subscriber call, tests).

## Acceptance Criteria

1. After N plan tasks, a model with a negative calibration correction history (more synthetic failures than successes) ranks lower in LinUCB scoring than a model without any correction history. Verifiable by a unit test in `roko-learn`.
2. After a gate failure on a task, `.roko/learn/section-effects.json` is updated with an outcome entry for the failed gate. The next task's prompt builder reads this updated file.
3. After a task that earns `Grade::D`, `cascade-router.json` shows an `efficiency_weights` entry for that model with a value less than 1.0.
4. `cargo test -p roko-learn` passes with zero failures.
5. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] Unit test: model with negative calibration correction ranks lower than clean model in the LinUCB path
- [ ] Unit test: gate failure → `SectionEffectivenessRegistry::record_outcome` → `save` → file updated
- [ ] Unit test: `update_efficiency(model, &Grade::D)` results in weight < 1.0 in `cascade-router.json`
- [ ] `cargo test -p roko-learn` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` is clean
- [ ] Run one plan task that fails a gate; verify `.roko/learn/section-effects.json` is modified

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-learn/src/cascade_router.rs` | Apply calibration confidence bias in LinUCB scoring branch (~line 1728); add `efficiency_weights` field and `update_efficiency` method; update `CascadeSnapshot` |
| `crates/roko-learn/src/event_subscriber.rs` | Wire `SectionEffectivenessRegistry` updates in `GateResult` handler (line 242); call `router.update_efficiency(...)` after efficiency event append (line 202) |
| `crates/roko-learn/src/section_effect.rs` | No structural changes; verify `record_outcome` and `save` API is sufficient |
| `crates/roko-cli/src/dispatch/prompt_cache.rs` | No changes needed; existing stale-check reload handles the updated file |
