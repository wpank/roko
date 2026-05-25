# M112 — Distillation Calibration Receipts and Predict-Publish-Correct

## Objective
Wire the predict-publish-correct feedback loop into the knowledge lifecycle: each distillation stage publishes a prediction (expected outcome), the system observes the actual outcome, and a calibration receipt records the error. This enables the distillation pipeline to self-calibrate over time, learning which episode patterns reliably produce good heuristics and which are false positives.

**Important**: A `CalibrationReceipt` struct already exists in `crates/roko-neuro/src/tier_progression.rs` (tracks `episode_id`, `CalibrationAction`, `timestamp_ms` for heuristic tier transitions). This batch creates a **separate** `DistillCalibrationReceipt` type for the predict-publish-correct loop. Do NOT rename or modify the existing `CalibrationReceipt`.

## Scope
- Crates: `roko-neuro`, `roko-learn`
- Files: `crates/roko-neuro/src/distill_pipeline.rs` (from M100), new file `crates/roko-neuro/src/distill_calibration.rs`, `crates/roko-learn/src/runtime_feedback.rs`
- Phase ref: depth doc 11-memory/03-knowledge-lifecycle-loop.md
- Depth doc: `tmp/unified-depth/11-memory/03-knowledge-lifecycle-loop.md`

## Steps
1. Discover existing calibration and feedback code:
   ```bash
   grep -rn 'CalibrationReceipt\|CalibrationAction\|CalibrationTable' crates/roko-neuro/src/ --include='*.rs' | head -10
   grep -rn 'calibration\|predict\|receipt' crates/roko-learn/src/ --include='*.rs' | head -15
   grep -n 'pub struct CalibrationReceipt' crates/roko-neuro/src/tier_progression.rs | head -3
   ```

2. **Existing CalibrationReceipt** (in `crates/roko-neuro/src/tier_progression.rs`):
   ```rust
   pub struct CalibrationReceipt {
       pub episode_id: String,
       pub action: CalibrationAction,      // Promote, Demote, Confirm, etc.
       pub timestamp_ms: i64,
   }
   ```
   This is for heuristic tier transitions. Do NOT modify or rename it.

3. Create `crates/roko-neuro/src/distill_calibration.rs` with a separate receipt type:
   ```rust
   use chrono::{DateTime, Utc};
   use serde::{Deserialize, Serialize};
   use std::path::{Path, PathBuf};

   /// A calibration receipt for the predict-publish-correct distillation loop.
   /// Distinct from `CalibrationReceipt` in tier_progression.rs which tracks
   /// heuristic tier transitions.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct DistillCalibrationReceipt {
       /// Unique ID for this receipt
       pub id: String,
       /// Which distillation stage made the prediction
       pub stage: DistillStage,
       /// What was predicted
       pub prediction: DistillPrediction,
       /// What actually happened (filled in after observation)
       pub outcome: Option<DistillOutcome>,
       /// Calibration error (|predicted - actual| / predicted, once outcome is recorded)
       pub error: Option<f64>,
       /// Timestamp of prediction
       pub predicted_at: DateTime<Utc>,
       /// Timestamp of outcome observation
       pub observed_at: Option<DateTime<Utc>>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum DistillStage {
       D1ExtractInsights,
       D2PromoteHeuristics,
       D3CompilePlaybook,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct DistillPrediction {
       /// Expected count of outputs from this stage
       pub expected_count: usize,
       /// Expected confidence range (low, high)
       pub expected_confidence: (f64, f64),
       /// Predicted quality score
       pub predicted_quality: f64,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct DistillOutcome {
       /// Actual count of outputs
       pub actual_count: usize,
       /// Actual average confidence
       pub actual_confidence: f64,
       /// Quality score from downstream verification
       pub actual_quality: f64,
   }
   ```

4. Implement receipt management:
   ```rust
   /// Store for distillation calibration receipts.
   /// Persisted to `.roko/learn/distill-calibration.jsonl`.
   pub struct DistillCalibrationStore {
       path: PathBuf,
       receipts: Vec<DistillCalibrationReceipt>,
   }

   impl DistillCalibrationStore {
       pub fn load(path: &Path) -> Result<Self> { ... }
       pub fn save(&self) -> Result<()> { ... }

       /// Publish a prediction (before running a stage). Returns receipt ID.
       pub fn publish_prediction(
           &mut self,
           stage: DistillStage,
           prediction: DistillPrediction,
       ) -> String { ... }

       /// Record the outcome (after running a stage).
       pub fn record_outcome(
           &mut self,
           receipt_id: &str,
           outcome: DistillOutcome,
       ) -> Result<()> { ... }

       /// Get calibration error statistics per stage.
       pub fn stage_stats(&self, stage: &DistillStage) -> DistillCalibrationStats { ... }
   }

   #[derive(Debug, Clone)]
   pub struct DistillCalibrationStats {
       pub total_receipts: usize,
       pub mean_error: f64,
       pub count_error_trend: f64,       // positive = predictions getting worse
       pub confidence_error_trend: f64,   // positive = confidence estimates off
   }
   ```

5. Wire into the distillation pipeline (from M100):
   ```rust
   // In distill_pipeline.rs, modify DistillationPipeline::run_full:
   // Before D1: publish prediction from recent stats
   // After D1: record outcome
   // Repeat for D2, D3
   ```
   If `distill_pipeline.rs` does not exist yet (M100 not run), create the receipt infrastructure standalone with clear integration points documented.

6. Register in `crates/roko-neuro/src/lib.rs`:
   ```rust
   pub mod distill_calibration;
   ```

7. Write tests:
   - publish_prediction returns a receipt ID
   - record_outcome fills in receipt fields and computes error
   - Calibration error is computed correctly
   - stage_stats aggregates over multiple receipts
   - Receipts persist to JSONL and reload correctly
   - DistillCalibrationReceipt does not conflict with CalibrationReceipt in tier_progression

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- distill_calibration
```

## What NOT to do
- Do NOT modify the existing `CalibrationReceipt` in tier_progression.rs -- it is a different type for a different purpose
- Do NOT modify the existing `CalibrationReceipt` in roko-neuro/src/tier_progression.rs -- the new `DistillCalibrationReceipt` is a separate type for a different purpose
- Do NOT implement temperature scaling or Bayesian calibration -- just track error
- Do NOT make the receipt store async -- keep it synchronous file I/O
- Do NOT remove or modify the distillation pipeline from M100 -- add calibration as an overlay
