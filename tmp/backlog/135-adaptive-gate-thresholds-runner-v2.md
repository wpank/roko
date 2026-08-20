# 135 — Adaptive Gate Thresholds in Runner-v2

**Priority**: P1 — `AdaptiveThresholds` exists in `roko-gate` and `gate-thresholds.json` is reserved, but runner-v2 never loads, updates, or saves threshold values; the gate pipeline is not self-calibrating despite the infrastructure being fully built.
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-gate/src/`, `crates/roko-cli/src/runner/gate_dispatch.rs`
**Depends on**: None (all infrastructure exists)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §H-1 (suggested 133), `tmp/backlog/_mori-old-gaps.md` MO-21

---

## Background

`roko-gate` contains an `AdaptiveThresholds` struct that tracks EMA (exponential moving average) pass rates per gate rung. The idea: if a rung consistently passes at a lower quality bar than required, the threshold is lowered to reduce false failures; if a rung is too lenient, the threshold is raised. This allows the gate pipeline to calibrate itself over time.

The infrastructure is complete:
- `AdaptiveThresholds` in `roko-gate`
- `.roko/learn/gate-thresholds.json` reserved by `LearningPaths`
- `AdaptiveThresholds::update(rung, passed: bool)` method

The gap is three missing steps in runner-v2:
1. Load `gate-thresholds.json` at startup.
2. Call `threshold.update(rung, passed)` on every gate completion.
3. Save the updated thresholds after each update and at shutdown.

Without these steps, every run starts with default thresholds, discards all observations, and the "adaptive" label is misleading.

## Current State

- `crates/roko-gate/src/` — `AdaptiveThresholds` struct with `update()` and `load()`/`save()`.
- `.roko/learn/gate-thresholds.json` — file path reserved; may or may not be created by the gate system.
- `crates/roko-cli/src/runner/gate_dispatch.rs` — dispatches gate rungs; does not reference `AdaptiveThresholds`.
- `crates/roko-cli/src/runner/event_loop.rs` — runner startup; does not load `AdaptiveThresholds`.

## Implementation Plan

1. **Load at startup**: In `event_loop.rs` during runner initialization:
   ```rust
   let mut adaptive_thresholds = AdaptiveThresholds::load_or_default(
       &config.learning_paths.gate_thresholds
   );
   ```

2. **Pass thresholds into gate dispatch**: Update the `build_rung_execution_inputs()` function in `gate_dispatch.rs` to accept `&AdaptiveThresholds` and include the per-rung threshold value in the `GateRungInput`.

3. **Update after each gate completion**: In the event handler for `RunnerEvent::GateCompleted`:
   ```rust
   adaptive_thresholds.update(gate_result.rung, gate_result.passed);
   adaptive_thresholds.save(&config.learning_paths.gate_thresholds)?;
   ```

4. **Include in `RunnerEvent::GateCompleted`**: Add `threshold_before: f64` and `threshold_after: f64` fields to `GateCompleted` so that the TUI and HTTP API can show threshold evolution.

5. **Save at shutdown**: In the runner's cleanup path, call `adaptive_thresholds.save()` to ensure the final state is persisted even if the last save during the run was skipped.

6. **Include in crash snapshot**: Add `adaptive_thresholds: Option<SerializedThresholds>` to `RunStateSnapshot`. On resume, restore from snapshot rather than re-reading the file (avoids races with concurrent runs on the same workspace).

7. **Proof**: Write a test that: (a) runs a plan where rung 1 always fails, (b) checks that `gate-thresholds.json` has a lower threshold for rung 1 after N runs compared to before.

## Acceptance Criteria

1. After running a plan, `.roko/learn/gate-thresholds.json` exists and has non-default values.
2. A rung that consistently fails has a lower threshold than its default after repeated runs.
3. `RunnerEvent::GateCompleted` includes `threshold_before` and `threshold_after` fields.
4. Runner startup loads and applies the existing threshold file.
5. Crash/resume preserves threshold state (no threshold reset on resume).
6. `cargo test -p roko-gate` passes with the new wiring.

## Verification Checklist

- [ ] Run a plan; check `.roko/learn/gate-thresholds.json` has content.
- [ ] Compare threshold values before and after two runs where compile gate fails consistently; verify the threshold decreases.
- [ ] Crash mid-run; resume; check that threshold values match the pre-crash state.
- [ ] Unit test: `AdaptiveThresholds::update(rung=0, passed=false)` five times; verify threshold decreased.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Load thresholds at startup; update and save on each gate completion |
| `crates/roko-cli/src/runner/gate_dispatch.rs` | Pass `AdaptiveThresholds` into rung inputs |
| `crates/roko-cli/src/runner/types.rs` | Add threshold fields to `GateCompleted` event; add to crash snapshot |
| `crates/roko-gate/src/` | Verify `AdaptiveThresholds::load_or_default` and `save` exist |
