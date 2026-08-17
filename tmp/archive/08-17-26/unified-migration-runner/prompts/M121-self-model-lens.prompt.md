# M121 — Self-model accuracy Lens and Yerkes-Dodson pressure Score Cell

## Objective
Implement the SelfModelLens that tracks five accuracy metrics and extend the existing `YerkesDodson` with a multi-input pressure index computation. These are the observation layer for adaptive supervision.

## Scope
- Crates: `roko-conductor`
- Files:
  - New: `crates/roko-conductor/src/self_model_lens.rs`
  - `crates/roko-conductor/src/yerkes_dodson.rs` (extend existing `YerkesDodson` struct)
  - `crates/roko-conductor/src/threshold_learner.rs` (extend existing `ThresholdLearner` and `AdaptiveThreshold`)
  - `crates/roko-conductor/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/17-adaptive-supervision-loop.md`

## Existing types reference

The `YerkesDodson` struct already exists (`crates/roko-conductor/src/yerkes_dodson.rs`):
```rust
pub struct YerkesDodson {
    pub pressure: f64,      // [0.0, 1.0]
    pub optimal: f64,       // default 0.5
    pub width: f64,         // Gaussian width, default 0.25
}
impl YerkesDodson {
    pub fn new(optimal: f64, width: f64) -> Self
    pub fn set_pressure(&mut self, pressure: f64)
    pub fn performance_multiplier(&self) -> f64  // Gaussian curve
    pub fn intervention_aggressiveness(&self) -> f64  // 1 - performance
    // plus: pressure_adjustment(), decision_bias()
}
```

The `ThresholdLearner` already exists (`crates/roko-conductor/src/threshold_learner.rs`):
```rust
pub struct AdaptiveThreshold {
    pub ema: f64,
    pub observations: u64,
    pub effective_count: u64,
    pub ineffective_count: u64,
}
pub struct ThresholdLearner {
    pub watcher_thresholds: HashMap<String, AdaptiveThreshold>,
    pub intervention_history: VecDeque<InterventionOutcome>,  // flat ring buffer, not per-watcher
    pub alpha: f64,  // EMA smoothing, default 0.1
    pub default_restart: f64,
    pub default_fail: f64,
}
impl ThresholdLearner {
    pub fn new() -> Self
    pub fn load_or_new(path: &Path) -> Self  // returns Self, not io::Result
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error>
}
```

## Steps
1. Discover the full existing API:
   ```bash
   grep -rn 'pub fn\|pub struct\|pub enum' crates/roko-conductor/src/yerkes_dodson.rs | head -20
   grep -rn 'pub fn\|pub struct' crates/roko-conductor/src/threshold_learner.rs | head -20
   grep -rn 'SelfModel\|BrierScore\|self_model\|FlowIndicator' crates/roko-conductor/src/ --include='*.rs' | head -10
   ```

2. Create `crates/roko-conductor/src/self_model_lens.rs`:
   ```rust
   use serde::{Deserialize, Serialize};
   use std::collections::HashMap;

   /// Five accuracy metrics, each an f64 in [0.0, 1.0].
   #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
   pub struct SelfModelAccuracy {
       pub intervention_effectiveness: f64,
       pub stuck_detection_precision: f64,
       pub diagnosis_accuracy: f64,
       pub gate_brier_score: f64,
       pub threshold_calibration_error: f64,
   }

   /// Rolling EMA tracker for Brier score of gate pass predictions.
   #[derive(Debug, Clone, Default, Serialize, Deserialize)]
   pub struct BrierScoreTracker {
       pub ema: f64,
       pub observations: u64,
       alpha: f64,  // default 0.1
   }
   impl BrierScoreTracker {
       pub fn new(alpha: f64) -> Self { Self { ema: 0.0, observations: 0, alpha } }
       pub fn observe(&mut self, predicted_prob: f64, actual_passed: bool) {
           let actual = if actual_passed { 1.0 } else { 0.0 };
           let error_sq = (predicted_prob - actual).powi(2);
           if self.observations == 0 { self.ema = error_sq; } else { self.ema = self.alpha * error_sq + (1.0 - self.alpha) * self.ema; }
           self.observations += 1;
       }
       pub fn score(&self) -> f64 { self.ema }
   }

   /// Per-watcher intervention tracking.
   #[derive(Debug, Clone, Default, Serialize, Deserialize)]
   pub struct InterventionTracker { pub successes: u64, pub total: u64 }
   impl InterventionTracker {
       pub fn observe(&mut self, succeeded: bool) { self.total += 1; if succeeded { self.successes += 1; } }
       pub fn effectiveness(&self) -> f64 { if self.total == 0 { 0.5 } else { self.successes as f64 / self.total as f64 } }
   }

   /// Composite self-model lens.
   #[derive(Debug, Clone, Default, Serialize, Deserialize)]
   pub struct SelfModelLens {
       pub interventions: HashMap<String, InterventionTracker>,
       pub stuck_tracker: InterventionTracker,
       pub brier: BrierScoreTracker,
   }
   impl SelfModelLens {
       pub fn observe_intervention(&mut self, watcher: &str, succeeded: bool) { ... }
       pub fn observe_stuck_detection(&mut self, was_genuine: bool) { ... }
       pub fn observe_gate_prediction(&mut self, predicted_prob: f64, actual_passed: bool) { ... }
       pub fn accuracy(&self) -> SelfModelAccuracy { ... }
   }
   ```

3. Extend `yerkes_dodson.rs` with a `PressureIndex` struct (do NOT modify existing `YerkesDodson`):
   ```rust
   /// Multi-dimensional pressure computation for conductor threshold scaling.
   #[derive(Debug, Clone, Default, Serialize, Deserialize)]
   pub struct PressureIndex {
       pub time_remaining_fraction: f64,
       pub budget_remaining_fraction: f64,
       pub error_rate: f64,
       pub stuck_count_norm: f64,
       pub gate_fail_streak_norm: f64,
   }
   impl PressureIndex {
       pub fn compute(&self) -> f64 {
           (0.3 * (1.0 - self.time_remaining_fraction)
            + 0.3 * (1.0 - self.budget_remaining_fraction)
            + 0.2 * self.error_rate
            + 0.1 * self.stuck_count_norm
            + 0.1 * self.gate_fail_streak_norm).clamp(0.0, 1.0)
       }
   }

   /// Maps pressure to a watcher threshold sensitivity modifier.
   pub fn pressure_to_sensitivity_modifier(pressure: f64) -> f64 {
       match pressure {
           p if p < 0.3 => 1.0,
           p if p < 0.7 => 1.2,
           p if p < 0.9 => 1.5,
           _ => 2.0,
       }
   }
   ```

4. Add `FlowIndicators` to `yerkes_dodson.rs`:
   ```rust
   #[derive(Debug, Clone, Default)]
   pub struct FlowIndicators {
       pub consistent_file_changes: bool,
       pub improving_gate_scores: bool,
       pub diverse_tool_usage: bool,
       pub moderate_context: bool,
   }
   impl FlowIndicators {
       pub fn flow_score(&self) -> f64 {
           [self.consistent_file_changes, self.improving_gate_scores,
            self.diverse_tool_usage, self.moderate_context]
               .iter().filter(|&&b| b).count() as f64 / 4.0
       }
       pub fn is_in_flow(&self) -> bool { self.flow_score() >= 0.75 }
   }
   ```

5. Extend `ThresholdLearner` with predict/correct methods. The existing API uses EMA; add Beta-distribution methods alongside:
   ```rust
   impl ThresholdLearner {
       /// Predict the effective threshold for a watcher, incorporating learned corrections.
       pub fn predict_threshold(&self, watcher: &str) -> f64 { ... }
       /// Correct the threshold based on whether the intervention helped.
       pub fn correct_threshold(&mut self, watcher: &str, intervention_helped: bool) { ... }
   }
   ```

6. Add `self_model_lens` module to `crates/roko-conductor/src/lib.rs` and re-export `SelfModelLens`, `SelfModelAccuracy`, `BrierScoreTracker`, `InterventionTracker`. Re-export `PressureIndex`, `FlowIndicators`, `pressure_to_sensitivity_modifier` from `yerkes_dodson`.

7. Add tests:
   - BrierScore is 0 for perfect predictions, ~0.25 for random
   - PressureIndex increases when budget decreases
   - FlowIndicators fires when all 4 indicators are true
   - ThresholdLearner predict/correct adjusts after outcomes

## Verification
```bash
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- self_model
cargo test -p roko-conductor -- yerkes_dodson
cargo test -p roko-conductor -- pressure
cargo test -p roko-conductor -- flow
cargo test -p roko-conductor -- threshold
```

## What NOT to do
- Do NOT modify the existing `YerkesDodson` struct or its methods — add new types alongside
- Do NOT modify the existing `ThresholdLearner::update()` / `get_threshold()` — add new methods
- Do NOT wire into the orchestrator — observation only
- Do NOT add Bus integration — Lens Cells are polled, not subscribed
- Do NOT add triple-loop learning — just single-loop correction here
