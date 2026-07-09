# M145 — Wire Prediction Error to Tier Gating

## Objective
Wire the prediction error computation into the orchestrate.rs dispatch loop so that model tier selection is driven by real-time prediction error rather than static configuration. The `compute_prediction_error()` function and `PredictionErrorInput`/`PredictionErrorWeights` structs already exist in `heartbeat.rs`. The task is to compute prediction_error from CorticalState on each dispatch, compare against an adaptive threshold (modulated by affect/resource/arousal), and use the result to select inference tier (T0/T1/T2). This replaces the current static model selection path.

## Scope
- Crates: `roko-runtime`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs` (threshold computation)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (wire tier selection)
- Depth doc: `tmp/unified-depth/05-heartbeat/` (prediction error algorithms)

## Steps
1. Read the existing prediction error computation:
   ```bash
   grep -n 'compute_prediction_error\|PredictionErrorInput\|PredictionErrorWeights' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs | head -10
   ```

2. Read how tier selection currently works in orchestrate.rs:
   ```bash
   grep -n 'InferenceTier\|ModelTier\|tier\|select_tier\|CascadeRouter' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -20
   ```

3. Add `compute_adaptive_threshold()` to heartbeat.rs:
   ```rust
   /// Compute the adaptive prediction-error threshold.
   ///
   /// Base threshold = 0.20, modulated by:
   /// - Arousal: high arousal lowers threshold (more sensitive)
   /// - Resource health: low resources raises threshold (more conservative)
   /// - Dominance: high dominance lowers threshold (more decisive)
   pub fn compute_adaptive_threshold(state: &CorticalState) -> f32 {
       let base = 0.20;
       let arousal_mod = (state.arousal() - 0.5) * -0.05; // high arousal -> lower threshold
       let resource_mod = (1.0 - state.resource_health()) * 0.05; // low resources -> higher threshold
       let dominance_mod = (state.dominance() - 0.5) * -0.03;
       (base + arousal_mod + resource_mod + dominance_mod).clamp(0.10, 0.40)
   }
   ```

4. Add `select_tier_from_prediction_error()`:
   ```rust
   /// Map prediction error to inference tier.
   ///
   /// - pred_error < threshold → T0 (cheap, fast)
   /// - pred_error < 2×threshold → T1 (standard)
   /// - pred_error >= 2×threshold → T2 (expensive, high-quality)
   pub fn select_tier_from_prediction_error(pred_error: f32, threshold: f32) -> InferenceTier {
       if pred_error < threshold {
           InferenceTier::T0
       } else if pred_error < threshold * 2.0 {
           InferenceTier::T1
       } else {
           InferenceTier::T2
       }
   }
   ```

5. Wire into orchestrate.rs dispatch path:
   - Before agent dispatch, compute `PredictionErrorInput` from CorticalState snapshot
   - Call `compute_prediction_error(input, weights)`
   - Call `compute_adaptive_threshold(cortical_state)`
   - Call `select_tier_from_prediction_error(pred_error, threshold)`
   - Pass selected tier to CascadeRouter as a hint (do NOT override force_backend)

6. Preserve the existing static fallback: if CorticalState is not initialized (first tick), fall back to the current tier selection logic.

7. Write tests:
   - Threshold modulation: high arousal → lower threshold
   - Tier selection boundaries: exact boundary values
   - Integration: PredictionErrorInput from CorticalSnapshot → tier

## Verification
```bash
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- prediction_error
cargo check -p roko-cli
```

## What NOT to do
- Do NOT remove the existing CascadeRouter — prediction error provides a *hint*, not a replacement
- Do NOT override `force_backend` configuration — user overrides always win
- Do NOT make this blocking — prediction error computation is pure arithmetic, no I/O
- Do NOT add new dependencies — all math is stdlib f32 operations
- Do NOT modify the CascadeRouter internals — pass the tier hint via existing API
