# 39 — ACP Learning-Pipeline Parity

**Priority**: P2 — IDE-driven sessions (Cursor) contribute weaker learning signal than CLI sessions, causing model routing to lag behind the CLI path over time
**Size**: M (1-2 days)
**Crates**: `crates/roko-acp/` (primary), `crates/roko-learn/`, `crates/roko-daimon/`
**Depends on**: None

---

## Background

Roko has three learning subsystems that shape model selection and accumulate feedback: the `CascadeRouter` (learned model routing), the `DaimonPolicy` (affect-based dispatch modulation), and the `ExperimentStore` (canonical section A/B assignment). When work arrives via the CLI runner-v2 path, all three subsystems are fully exercised. When the same work arrives via the ACP path — the protocol used by Cursor and other IDE integrations — the subsystems are partially wired and the pipeline dispatch path bypasses all learning code.

The ACP entry point for prompts is the `acp_dispatch_prompt` function in `crates/roko-acp/src/bridge_events.rs`, which is approximately 8,773 lines long. When `pipeline_template.is_some()` (the `is_pipeline_dispatch` branch), the function calls `run_with_workflow_engine` and returns at line 2125 (`return Ok(())`), bypassing the entire block that runs `acp_dispatch_succeeded`, `record_acp_experiment_outcome`, and `record_cascade_observation` (lines 2314-2388). This means pipeline dispatches — the subset of ACP calls that actually run multi-task plans — produce zero learning signal.

For single-agent ACP dispatches (the non-pipeline path), the learning code runs but with two gaps: the `DaimonPolicy` is constructed from only two fields of the daimon state file (`confidence` and `behavioral_state`, lines 726-738), rather than loading the full `roko_daimon::DaimonPolicy` struct. The full struct loads from disk via `DaimonState::load_or_new` and calls `modulate_dispatch` with tier_bias, turn_limit_factor, and exploration_rate derived from the behavioral state — none of which are present in the ACP coarse parse. The `CalibrationTracker` feedback path is also absent: after `record_cascade_observation` is called, the CLI runner feeds back actual token counts and latency against the pre-dispatch estimate, but ACP does not.

## Current State

1. `crates/roko-acp/src/bridge_events.rs` line 28 — imports `roko_core::DaimonPolicy` (the lightweight two-field struct from `crates/roko-core/src/affect.rs` lines 67-89). The full-fidelity `roko_daimon::DaimonPolicy` (which wraps `DaimonState` and implements `AffectPolicy`) is **not** imported or used.

2. `bridge_events.rs` lines 714-744 — `acp_routing_context` constructs a `DaimonPolicy` by parsing two JSON fields (`state.confidence` and `state.behavioral_state`) from the daimon file on disk. This discards `vitality_tracker.last_phase`, `cognitive_energy.current`, and the full `DispatchModulation` that `roko_daimon::DaimonPolicy::modulate_dispatch` would produce.

3. `bridge_events.rs` line 1004 — `cascade_select_model` is called for single-agent dispatches and returns a model selection when `ROKO_ACP_CASCADE_SELECT=1` is set and the router file exists.

4. `bridge_events.rs` line 1138 — `record_cascade_observation` records the model, routing context, success, wall time, and output tokens into the cascade router file.

5. `bridge_events.rs` lines 798-916 — `assign_acp_experiment` and `record_acp_experiment_outcome` are called for single-agent dispatches. `record_acp_experiment_outcome` records a binary `success = acp_dispatch_succeeded(...)` (line 2354). `acp_dispatch_succeeded` (line 766) returns `true` only when: no task error, no stream error, and stop reason is `EndTurn`. For pipeline dispatches this code is never reached (early return at line 2125).

6. `bridge_events.rs` lines 2364-2388 — the `if !is_pipeline_dispatch` guard explicitly skips `record_cascade_observation` for pipeline dispatches. Comment: "Workflow services own their own provider feedback and recording them again would train the wrong arm."

7. `crates/roko-learn/src/prediction.rs` line 140 — `CalibrationTracker` struct. Has `record_residual(model, category, residual)`, `record_routing_decision(record)`, and `from_routing_logs(records)` methods. Is **not** imported in `bridge_events.rs`.

8. `crates/roko-daimon/src/policy.rs` — `roko_daimon::DaimonPolicy` wraps `DaimonState` and provides `pre_dispatch`, `modulate_dispatch`, `behavioral_state` via `AffectPolicy`. The `DaimonState::load_or_new(path)` function loads from disk and returns a full state struct.

## Implementation Plan

### Change A: Full DaimonPolicy load in `acp_routing_context`

In `bridge_events.rs`, the `acp_routing_context` function (around line 680) currently builds a two-field `roko_core::DaimonPolicy`. Replace the ad-hoc JSON parse at lines 714-744 with a `roko_daimon::DaimonState::load_or_new(&daimon_path)` call and extract the `DaimonPolicy` fields from the full state:

```rust
use roko_daimon::DaimonState;

let daimon_policy = {
    let canonical = workdir.join(".roko").join("daimon").join("affect.json");
    let daimon_path = if canonical.exists() {
        canonical
    } else {
        workdir.join(".roko").join("state").join("daimon.json")
    };

    if daimon_path.exists() {
        let state = DaimonState::load_or_new(&daimon_path);
        let affect = state.query();
        roko_core::DaimonPolicy::new(
            affect.pad.pleasure.clamp(0.0, 1.0),  // use pleasure as confidence proxy
            affect.behavioral_state,
        )
    } else {
        roko_core::DaimonPolicy::default()
    }
};
```

Note: the ACP path remains read-only — it does not call `state.appraise()` or `state.persist()`. The orchestrator (runner-v2) is the sole writer.

Add `roko-daimon` to `crates/roko-acp/Cargo.toml` as a dependency if it is not already present.

Estimated: ~25 lines changed. Risk: low (read-only operation, fallback to default).

### Change B: CalibrationTracker feedback after `record_cascade_observation`

After the `record_cascade_observation` call at line 2377, construct a residual and feed it to `CalibrationTracker`:

```rust
use roko_learn::prediction::CalibrationTracker;

// Only for direct-dispatch turns (already guarded by !is_pipeline_dispatch).
if let Some(sr) = stream_result_ref {
    let actual_output_tokens = sr.usage.as_ref().map(|u| u.output_tokens).unwrap_or(0);
    // Load or default the calibration tracker from the router log.
    let router_path = workdir_for_logging.join(".roko").join("learn").join("cascade-router.json");
    let model_slugs = cascade_router_model_slugs(&roko_config_for_logging, &resolved_for_logging.slug);
    let router = CascadeRouter::load_or_new(&router_path, model_slugs);
    if let Some(recent) = router.most_recent_log_for_model(&model_key_for_logging) {
        let mut tracker = CalibrationTracker::default();
        tracker.record_routing_decision(&recent);
        // tracker now contains the residual; log or persist as needed.
        let _ = tracker; // extend: persist calibration state or pass to future predictions
    }
}
```

Alternatively, thread `actual_output_tokens` and `dispatch_started.elapsed()` into `record_cascade_observation` as a new `calibration_hint: Option<CalibrationHint>` parameter and have the blocking task apply it.

Estimated: ~40 lines. Risk: low (additive, no existing behavior changed).

### Change C: Record learning signal for pipeline dispatches

The pipeline branch currently returns at line 2125 without recording any learning signal. Before the `return Ok(())`, record the experiment outcome using `report.success`:

```rust
let report = run_with_workflow_engine(...).await?;

// Record experiment outcome using the workflow engine's pass/fail result.
if let Some(assignment) = experiment_assignment_for_pipeline.as_ref() {
    let _ = record_acp_experiment_outcome(&experiment_path, assignment, report.success);
}

// Record cascade observation using the model that was selected.
if !model_selection_explicit && acp_cascade_selection_enabled() {
    let model_slugs = cascade_router_model_slugs(&roko_config, &resolved.slug);
    let routing_ctx = acp_routing_context(...);
    let wall_ms = dispatch_started.elapsed().as_millis() as u64;
    drop(record_cascade_observation(
        workdir.join(".roko").join("learn").join("cascade-router.json"),
        model_key_for_dispatch.clone(),
        routing_ctx,
        report.success,
        wall_ms,
        None, // token counts not available from workflow engine report
        model_slugs,
    ));
}

return Ok(());
```

This requires that `experiment_assignment`, `model_key_for_dispatch`, `dispatch_started`, and the routing context are available at this point. Audit the control flow to confirm they are in scope or thread them down.

Estimated: ~50 lines. Risk: medium (requires auditing variable lifetimes across the early-return boundary).

## Acceptance Criteria

1. `grep -n 'DaimonState' crates/roko-acp/src/bridge_events.rs` returns a `DaimonState::load_or_new` call (not just the two-field JSON parse).
2. `grep -n 'CalibrationTracker\|record_routing_decision' crates/roko-acp/src/bridge_events.rs` returns at least one call site.
3. After a pipeline dispatch completes (workflow engine runs), the cascade router file is updated with the model and outcome.
4. After a pipeline dispatch, the experiment outcome is recorded using `report.success` (not `dispatch_succeeded` from the transport layer).
5. `cargo test -p roko-acp` passes with zero failures.
6. `cargo clippy -p roko-acp -- -D warnings` is clean.

## Verification Checklist

- [ ] `grep -n 'DaimonState' crates/roko-acp/src/bridge_events.rs` shows `load_or_new` call
- [ ] `grep -n 'CalibrationTracker' crates/roko-acp/src/bridge_events.rs` shows at least one line
- [ ] Run a pipeline dispatch via ACP; check `.roko/learn/cascade-router.json` is updated
- [ ] Run a pipeline dispatch; check `.roko/learn/efficiency.jsonl` has a new entry
- [ ] `cargo test -p roko-acp 2>&1 | tail -5` shows all tests passed
- [ ] `cargo clippy -p roko-acp -- -D warnings 2>&1 | grep error` is empty

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` | Replace two-field JSON parse with `DaimonState::load_or_new`; add CalibrationTracker feedback; add learning signal recording for pipeline path |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/Cargo.toml` | Add `roko-daimon` dependency if not already present |
