# M158 — Create Oracle Trait and Prediction Store

## Objective
Create the `Oracle` trait in `roko-learn/src/oracle/` with `predict()` and `evaluate()` methods. Create a `PredictionStore` that persists predictions with lineage to outcomes for later calibration. Create a `ResidualCorrector` that eliminates systematic bias via ~50ns EMA correction. Wire into the CascadeRouter as a feedback source so routing decisions improve over time.

## Scope
- Crates: `roko-learn`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/oracles/mod.rs` (trait definition)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/prediction.rs` (PredictionStore, if exists)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` (wire feedback)
- Depth doc: `tmp/unified-depth/09-technical-analysis/` (oracle architecture)

## Steps
1. Read the existing oracles module:
   ```bash
   cat /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/oracles/mod.rs
   grep -n 'pub trait Oracle\|trait Oracle' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/ -r --include='*.rs' | head -5
   ```

2. Read existing prediction infrastructure:
   ```bash
   grep -n 'pub struct\|pub fn\|Prediction' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/prediction.rs | head -20
   ```

3. Read how CascadeRouter currently accepts feedback:
   ```bash
   grep -n 'record_observation\|feedback\|observe\|record_outcome' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs | head -10
   grep -n 'record_observation\|feedback' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade/ -r --include='*.rs' | head -10
   ```

4. Define the Oracle trait (if not already in roko-core):
   ```rust
   /// A prediction oracle that produces calibrated predictions and tracks accuracy.
   pub trait Oracle: Send + Sync {
       /// Produce a prediction for the given query in context.
       fn predict(&self, query: &OracleQuery) -> Prediction;
       /// Evaluate a prediction against the actual outcome.
       fn evaluate(&self, prediction: &Prediction, outcome: &Outcome) -> Accuracy;
       /// Domain this oracle covers.
       fn domain(&self) -> &str;
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct OracleQuery {
       pub context_hash: u64,
       pub category: String,
       pub features: Vec<f32>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Prediction {
       pub id: String,
       pub predicted_model: String,
       pub expected_quality: f32,
       pub confidence: f32,
       pub timestamp: chrono::DateTime<chrono::Utc>,
       pub context_hash: u64,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Outcome {
       pub prediction_id: String,
       pub actual_quality: f32,
       pub gate_passed: bool,
       pub cost_usd: f64,
       pub latency_ms: u64,
   }

   pub struct Accuracy {
       pub absolute_error: f32,
       pub squared_error: f32,
       pub correct_direction: bool,
   }
   ```

5. Create `PredictionStore`:
   ```rust
   /// Persistent store for predictions and their outcomes.
   ///
   /// Stores predictions at creation time and links them to outcomes
   /// when available, enabling calibration analysis.
   pub struct PredictionStore {
       pending: HashMap<String, Prediction>,  // predictions awaiting outcome
       completed: VecDeque<(Prediction, Outcome, Accuracy)>,  // resolved pairs
       path: PathBuf,  // persistence path (.roko/learn/predictions.jsonl)
       max_history: usize,
   }

   impl PredictionStore {
       pub fn record_prediction(&mut self, prediction: Prediction) { ... }
       pub fn record_outcome(&mut self, outcome: Outcome) -> Option<Accuracy> { ... }
       pub fn recent_accuracy(&self, n: usize) -> f32 { ... }
       pub fn accuracy_by_category(&self, category: &str) -> f32 { ... }
   }
   ```

6. Create `ResidualCorrector`:
   ```rust
   /// EMA-based bias elimination for oracle predictions.
   ///
   /// Tracks the running residual (predicted - actual) and subtracts it
   /// from future predictions. Convergence is fast (~50 observations).
   pub struct ResidualCorrector {
       alpha: f32,           // EMA smoothing factor (default 0.05)
       residual_ema: f32,    // Current bias estimate
       observation_count: u64,
   }

   impl ResidualCorrector {
       pub fn new(alpha: f32) -> Self { ... }
       pub fn update(&mut self, predicted: f32, actual: f32) { ... }
       pub fn correct(&self, raw_prediction: f32) -> f32 {
           raw_prediction - self.residual_ema
       }
   }
   ```

7. Wire into CascadeRouter:
   - After routing decision, call `prediction_store.record_prediction(prediction)`
   - After gate verdict, call `prediction_store.record_outcome(outcome)`
   - Use `residual_corrector.correct()` on model quality estimates before ranking

8. Write tests:
   - ResidualCorrector eliminates constant bias within 50 observations
   - PredictionStore links predictions to outcomes correctly
   - Accuracy calculation is correct (absolute error, squared error)
   - Store persists and reloads from disk

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- oracle
cargo test -p roko-learn -- prediction
```

## What NOT to do
- Do NOT replace existing oracle implementations in `oracles/` — extend the module
- Do NOT add real LLM calls to oracle predictions — they are pure computation
- Do NOT modify CascadeRouter's core routing logic — add feedback as an overlay
- Do NOT make PredictionStore async — use synchronous file I/O (it writes infrequently)
- Do NOT add calibration visualization — that is a separate concern
