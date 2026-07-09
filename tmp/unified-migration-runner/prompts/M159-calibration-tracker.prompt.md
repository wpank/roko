# M159 — Create Calibration Tracker

## Objective
Create a `CalibrationTracker` in `roko-learn` that tracks per-(model, category) prediction accuracy using Brier score decomposition (reliability - resolution + uncertainty). Record outcomes from oracle predictions and gate verdicts. Wire into episode logging so calibration metrics are recorded alongside gate verdicts for offline analysis.

## Scope
- Crates: `roko-learn`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/oracles/` (new calibration.rs or extend)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs` (wire calibration data)
- Depth doc: `tmp/unified-depth/09-technical-analysis/` (calibration theory)

## Steps
1. Check if calibration infrastructure already exists:
   ```bash
   grep -rn 'calibration\|Calibration\|brier\|Brier' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/ --include='*.rs' | head -15
   ```

2. Read the episode logger to understand how to wire in:
   ```bash
   grep -n 'pub fn\|pub async fn\|pub struct' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs | head -15
   ```

3. Create `CalibrationTracker`:
   ```rust
   /// Tracks prediction calibration per (model, category) pair.
   ///
   /// Uses Brier score decomposition: BS = REL - RES + UNC
   /// - REL (reliability): how close predictions are to observed frequencies
   /// - RES (resolution): how much predictions differ from base rate
   /// - UNC (uncertainty): base-rate entropy (constant for a dataset)
   pub struct CalibrationTracker {
       buckets: HashMap<CalibrationKey, CalibrationBucket>,
       global_outcomes: Vec<bool>,  // for UNC calculation
   }

   #[derive(Debug, Clone, Hash, Eq, PartialEq)]
   pub struct CalibrationKey {
       pub model: String,
       pub category: String,
   }

   #[derive(Debug, Clone)]
   pub struct CalibrationBucket {
       /// Binned confidence levels and their observed success rates.
       bins: [ConfidenceBin; 10],  // 10 bins: [0.0-0.1), [0.1-0.2), ..., [0.9-1.0]
       total_observations: u64,
   }

   #[derive(Debug, Clone, Default)]
   pub struct ConfidenceBin {
       pub predicted_sum: f64,  // sum of predicted confidences
       pub actual_sum: f64,     // sum of actual outcomes (0 or 1)
       pub count: u64,
   }
   ```

4. Implement core methods:
   ```rust
   impl CalibrationTracker {
       /// Record an outcome observation.
       pub fn record_outcome(
           &mut self,
           model: &str,
           category: &str,
           predicted_confidence: f32,
           actual_outcome: bool,
       ) {
           let key = CalibrationKey { model: model.to_string(), category: category.to_string() };
           let bucket = self.buckets.entry(key).or_insert_with(CalibrationBucket::new);
           let bin_idx = (predicted_confidence * 10.0).min(9.0) as usize;
           bucket.bins[bin_idx].predicted_sum += predicted_confidence as f64;
           bucket.bins[bin_idx].actual_sum += if actual_outcome { 1.0 } else { 0.0 };
           bucket.bins[bin_idx].count += 1;
           bucket.total_observations += 1;
           self.global_outcomes.push(actual_outcome);
       }

       /// Get calibration stats for a specific model+category.
       pub fn get_calibration(&self, model: &str, category: &str) -> Option<CalibrationStats> { ... }

       /// Compute Brier score decomposition.
       pub fn brier_decomposition(&self, model: &str, category: &str) -> Option<BrierDecomposition> { ... }
   }

   #[derive(Debug, Clone)]
   pub struct CalibrationStats {
       pub observations: u64,
       pub mean_confidence: f32,
       pub mean_accuracy: f32,
       pub brier_score: f32,
       pub reliability: f32,
       pub resolution: f32,
       pub uncertainty: f32,
   }

   #[derive(Debug, Clone)]
   pub struct BrierDecomposition {
       pub reliability: f64,  // lower is better
       pub resolution: f64,   // higher is better
       pub uncertainty: f64,  // constant
       pub brier_score: f64,  // REL - RES + UNC
   }
   ```

5. Implement Brier decomposition formula:
   ```rust
   fn compute_brier_decomposition(bucket: &CalibrationBucket, base_rate: f64) -> BrierDecomposition {
       let n = bucket.total_observations as f64;
       let mut rel = 0.0;
       let mut res = 0.0;

       for bin in &bucket.bins {
           if bin.count == 0 { continue; }
           let nk = bin.count as f64;
           let fk = bin.predicted_sum / nk;  // mean predicted confidence in bin
           let ok = bin.actual_sum / nk;     // observed frequency in bin

           rel += nk * (fk - ok).powi(2);
           res += nk * (ok - base_rate).powi(2);
       }

       rel /= n;
       res /= n;
       let unc = base_rate * (1.0 - base_rate);

       BrierDecomposition { reliability: rel, resolution: res, uncertainty: unc,
                            brier_score: rel - res + unc }
   }
   ```

6. Wire into episode logging:
   - After gate verdict, call `calibration_tracker.record_outcome(model, category, confidence, passed)`
   - Add `calibration_snapshot` field to episode record (periodic, not every episode)

7. Write tests:
   - Perfect calibration (predicted 0.7, observed 70%) → REL ≈ 0.0
   - Overconfident model (predicted 0.9, observed 50%) → REL > 0
   - Brier decomposition identity: BS = REL - RES + UNC
   - Empty tracker returns None

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- calibration
```

## What NOT to do
- Do NOT add visualization/plotting — only computation and storage
- Do NOT add online recalibration (Platt scaling) — that is future work
- Do NOT store every observation individually — use binned aggregates
- Do NOT modify episode_logger's core struct — add calibration as an optional enrichment
- Do NOT make this async — calibration is pure computation
