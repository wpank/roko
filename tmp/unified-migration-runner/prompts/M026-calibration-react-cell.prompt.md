# M026 — Implement CalibrationReact Cell

## Objective
Create a CalibrationReact policy that subscribes to `prediction.*` and `outcome.*` topics on the Bus, joins prediction-outcome pairs by `lineage_hint`, computes calibration error (Brier score), and publishes `calibration.*.updated` Pulses. This is the core of the predict-publish-correct loop.

## Scope
- Crates: `roko-learn`
- Files:
  - `crates/roko-learn/src/calibration_policy.rs` (existing file — extend or replace)
  - `crates/roko-core/src/topics.rs` (topic constants)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.5
- Spec ref: `tmp/unified/02-CELL.md` §9 (Predict-Publish-Correct), `tmp/unified/10-LEARNING-LOOPS.md` §2

## Steps
1. Check existing calibration code:
   ```bash
   grep -rn 'CalibrationPolicy\|CalibrationReact\|calibration_policy' crates/roko-learn/src/ --include='*.rs' | head -10
   cat crates/roko-learn/src/calibration_policy.rs | head -40
   ```

2. Check what prediction/outcome types exist:
   ```bash
   grep -rn 'Prediction\|prediction' crates/roko-learn/src/prediction.rs | head -20
   ```

3. Design the CalibrationReact struct:
   ```rust
   /// Watches prediction/outcome pairs and maintains calibration state.
   ///
   /// See: tmp/unified/10-LEARNING-LOOPS.md §2
   pub struct CalibrationReact {
       /// Per-operator calibration state (operator_id -> CalibrationState).
       state: HashMap<String, CalibrationState>,
       /// Path for persistence.
       state_path: PathBuf,
   }

   pub struct CalibrationState {
       /// Operator/cell identifier.
       pub operator_id: String,
       /// Exponential moving average of Brier scores.
       pub brier_ema: f64,
       /// Total predictions observed.
       pub total_predictions: u64,
       /// Total correct predictions (within threshold).
       pub correct_predictions: u64,
       /// Last updated timestamp.
       pub updated_at: DateTime<Utc>,
   }
   ```

4. Implement the core calibration logic:
   - `join_prediction_outcome(prediction: &Pulse, outcome: &Pulse) -> Option<(f64, f64)>` — extracts predicted probability and actual outcome
   - `update_calibration(state: &mut CalibrationState, predicted: f64, actual: f64)` — updates Brier EMA
   - Brier score: `(predicted - actual)^2`

5. Implement `Policy` (or `decide_with_pulses`) for CalibrationReact:
   - Scan incoming pulses for `prediction.*` topics — buffer them
   - Scan for `outcome.*` topics — match with buffered predictions via lineage_hint
   - On match: compute calibration error, update state, produce a calibration Pulse

6. Add persistence: load/save calibration state from `.roko/learn/calibration-state.json`.

7. Add tests:
   ```rust
   #[test]
   fn calibration_updates_on_prediction_outcome_pair() {
       // Publish prediction pulse, then outcome pulse
       // Verify CalibrationReact produces calibration.updated pulse
       // Verify Brier score is correct
   }
   ```

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- calibration
```

## What NOT to do
- Do NOT replace the existing calibration_policy.rs if it contains useful logic — extend it
- Do NOT implement full Bus subscription here — the CalibrationReact processes Pulses handed to it via decide_with_pulses, not direct Bus subscription (that wiring happens at the orchestrator level)
- Do NOT add complex windowing or buffering — a simple HashMap of recent predictions keyed by lineage_hint is sufficient
- Do NOT block on missing predictions — if an outcome arrives without a matching prediction, log and skip
