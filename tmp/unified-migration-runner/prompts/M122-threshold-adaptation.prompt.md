# M122 — Threshold adaptation with predict-publish-correct

## Objective
Wire threshold adaptation into the conductor pipeline: each watcher threshold becomes a learnable parameter that adjusts via Beta-posterior predict-publish-correct. The `SelfModelLens` (M121) provides accuracy observations, the `ThresholdLearner` corrects thresholds, and the `PressureIndex` (M121) scales sensitivity at runtime.

## Scope
- Crates: `roko-conductor`, `roko-cli`
- Files:
  - `crates/roko-conductor/src/threshold_learner.rs` (extend — already has `ThresholdLearner`, `AdaptiveThreshold`, `InterventionOutcome`)
  - `crates/roko-conductor/src/pipeline.rs` (from M117 — integrate threshold reads)
  - `crates/roko-conductor/src/self_model_lens.rs` (from M121 — integrate accuracy observations)
  - `crates/roko-cli/src/orchestrate.rs` (wire learned thresholds at conductor callsite)
- Depth doc: `tmp/unified-depth/07-agent-runtime/17-adaptive-supervision-loop.md`

## Existing types reference

The `ThresholdLearner` already exists (`crates/roko-conductor/src/threshold_learner.rs`):
```rust
pub struct AdaptiveThreshold {
    pub ema: f64,                 // Current EMA of optimal intervention boundary
    pub observations: u64,
    pub effective_count: u64,
    pub ineffective_count: u64,
}

pub struct ThresholdLearner {
    pub watcher_thresholds: HashMap<String, AdaptiveThreshold>,
    pub intervention_history: VecDeque<InterventionOutcome>,  // flat ring buffer, not per-watcher
    pub alpha: f64,   // EMA smoothing factor, default 0.1
    pub default_restart: f64,
    pub default_fail: f64,
}

impl ThresholdLearner {
    pub fn new() -> Self
    pub fn load_or_new(path: &Path) -> Self  // returns Self, not io::Result
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error>
}
```

The `PressureIndex` and `pressure_to_sensitivity_modifier()` are from M121 in `yerkes_dodson.rs`.

The `Conductor` struct already has `threshold_learner: Mutex<ThresholdLearner>`.

## Steps
1. Discover the current state:
   ```bash
   grep -rn 'ThresholdLearner\|threshold_learner' crates/roko-conductor/src/ --include='*.rs' | head -15
   grep -rn 'ThresholdLearner\|threshold_learner\|conductor.*threshold' crates/roko-cli/src/orchestrate.rs | head -10
   grep -rn 'pipeline\|ConductorPipeline' crates/roko-conductor/src/ --include='*.rs' | head -10
   ```

2. Add `WatcherThreshold` as an extension alongside existing `AdaptiveThreshold`:
   ```rust
   // Add to threshold_learner.rs
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct WatcherThreshold {
       pub name: String,
       pub base_value: f64,         // original static threshold
       pub alpha: f64,              // Beta posterior (true positives), init 1.0
       pub beta: f64,               // Beta posterior (false positives), init 1.0
       pub pressure_modifier: f64,  // from PressureIndex, default 1.0
   }
   impl WatcherThreshold {
       pub fn effective(&self) -> f64 {
           let learned = self.base_value * (self.alpha / (self.alpha + self.beta));
           learned * self.pressure_modifier
       }
   }
   ```

3. Add Beta-distribution methods to `ThresholdLearner`:
   ```rust
   impl ThresholdLearner {
       /// Get or create a Beta-posterior WatcherThreshold.
       pub fn get_or_create_beta(&mut self, watcher: &str, base_value: f64) -> &mut WatcherThreshold { ... }

       /// Correct the Beta threshold based on intervention outcome.
       pub fn correct_beta(&mut self, watcher: &str, intervention_helped: bool) {
           if let Some(wt) = self.beta_thresholds.get_mut(watcher) {
               if intervention_helped { wt.alpha += 1.0; } else { wt.beta += 1.0; }
           }
       }

       /// Apply pressure modifier to all Beta thresholds.
       pub fn apply_pressure(&mut self, modifier: f64) {
           for wt in self.beta_thresholds.values_mut() {
               wt.pressure_modifier = modifier;
           }
       }
   }
   ```
   Add `beta_thresholds: HashMap<String, WatcherThreshold>` field to `ThresholdLearner`.

4. In the `ConductorPipeline` (M117), before each watcher evaluation:
   - Read `effective()` threshold from `ThresholdLearner::get_or_create_beta(watcher_name, default)`
   - Pass the effective threshold to the watcher (via a parameter or by setting it before evaluation)

5. At pipeline evaluation time:
   - Compute `PressureIndex` from current resource state (time/budget remaining, error rate, etc.)
   - Convert to modifier via `pressure_to_sensitivity_modifier(pressure)`
   - Call `threshold_learner.apply_pressure(modifier)`

6. Ensure the `save()`/`load()` persistence methods include the new `beta_thresholds` data. The persistence path is `.roko/learn/conductor-thresholds.json`.

7. In `orchestrate.rs`, after a gate verdict where the conductor intervened:
   - Record the intervention via `threshold_learner.correct_beta(watcher_name, next_attempt_passed)`
   - This closes the learning loop

8. Add re-export: `WatcherThreshold` from `lib.rs`.

9. Add tests:
   - `WatcherThreshold::effective()` converges downward after repeated true positives (alpha grows)
   - `WatcherThreshold::effective()` converges upward after repeated false positives (beta grows)
   - Pressure modifier raises effective thresholds (modifier > 1.0)
   - Persistence round-trips both EMA and Beta threshold data

## Verification
```bash
cargo check -p roko-conductor -p roko-cli
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- threshold
cargo test -p roko-conductor -- watcher_threshold
cargo test -p roko-conductor -- beta
```

## What NOT to do
- Do NOT remove or modify the existing `AdaptiveThreshold` / `update()` EMA path — add Beta alongside
- Do NOT change the default static thresholds — learned adjustments are multiplicative
- Do NOT require Bus infrastructure — use direct method calls in orchestrate.rs
- Do NOT add double-loop or triple-loop learning — just single-loop Beta correction
- Do NOT break the existing adaptive gate thresholds in roko-learn (separate system)
