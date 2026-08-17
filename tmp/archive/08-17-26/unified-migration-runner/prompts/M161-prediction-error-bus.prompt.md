# M161 — Wire Prediction Error Cycle on Bus

## Objective
Wire the full predict-publish-correct cycle as Bus events. When CascadeRouter selects a model, publish a `routing.prediction` Pulse with model_id, expected_quality, and context_hash. After the gate verdict is known, publish a `routing.outcome` Pulse. Create a `RoutingCalibrationCell` (React Cell) that subscribes to both topics, joins predictions to outcomes by lineage, computes prediction error, and feeds back to CascadeRouter via `record_observation()`.

## Scope
- Crates: `roko-learn`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` (emit prediction Pulse)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (emit outcome Pulse, wire cell)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs` (ensure topics available)
- Depth doc: `tmp/unified-depth/09-technical-analysis/` (prediction error loop)

## Steps
1. Read how the Bus/event system currently works:
   ```bash
   grep -n 'pub fn\|pub async fn\|pub enum RokoEvent\|publish\|emit\|send' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs | head -20
   ```

2. Read how CascadeRouter currently reports decisions:
   ```bash
   grep -n 'pub fn route\|pub async fn route\|fn select_model\|fn record_observation' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs | head -10
   ```

3. Read how orchestrate.rs calls the router and processes gate verdicts:
   ```bash
   grep -n 'cascade\|CascadeRouter\|route\|verdict\|gate.*result\|record_observation' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -15
   ```

4. Define Pulse payloads:
   ```rust
   /// Payload for routing.prediction Pulse.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct RoutingPredictionPulse {
       pub prediction_id: String,
       pub model_id: String,
       pub expected_quality: f32,
       pub confidence: f32,
       pub context_hash: u64,
       pub task_category: String,
       pub timestamp: chrono::DateTime<chrono::Utc>,
   }

   /// Payload for routing.outcome Pulse.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct RoutingOutcomePulse {
       pub prediction_id: String,
       pub actual_quality: f32,
       pub gate_passed: bool,
       pub cost_usd: f64,
       pub latency_ms: u64,
       pub timestamp: chrono::DateTime<chrono::Utc>,
   }
   ```

5. Emit prediction Pulse after CascadeRouter selects a model:
   ```rust
   // In orchestrate.rs, after route() returns:
   let prediction_id = uuid::Uuid::new_v4().to_string();
   bus.publish(RokoEvent::new(
       "routing.prediction",
       serde_json::to_value(&RoutingPredictionPulse {
           prediction_id: prediction_id.clone(),
           model_id: selected_model.clone(),
           expected_quality: route_result.expected_quality,
           confidence: route_result.confidence,
           context_hash,
           task_category: task.category.clone(),
           timestamp: Utc::now(),
       })?,
   ));
   ```

6. Emit outcome Pulse after gate verdict:
   ```rust
   // In orchestrate.rs, after gate pipeline returns:
   bus.publish(RokoEvent::new(
       "routing.outcome",
       serde_json::to_value(&RoutingOutcomePulse {
           prediction_id,
           actual_quality: gate_result.quality_score(),
           gate_passed: gate_result.passed(),
           cost_usd: dispatch_result.cost_usd,
           latency_ms: dispatch_result.latency.as_millis() as u64,
           timestamp: Utc::now(),
       })?,
   ));
   ```

7. Create `RoutingCalibrationCell`:
   ```rust
   /// React Cell that joins routing predictions to outcomes and feeds back.
   pub struct RoutingCalibrationCell {
       pending_predictions: HashMap<String, RoutingPredictionPulse>,
       cascade_router: Arc<Mutex<CascadeRouter>>,
       corrector: ResidualCorrector,
   }

   impl RoutingCalibrationCell {
       /// Handle a routing.prediction event.
       pub fn on_prediction(&mut self, pulse: RoutingPredictionPulse) {
           self.pending_predictions.insert(pulse.prediction_id.clone(), pulse);
       }

       /// Handle a routing.outcome event — join, compute error, feed back.
       pub fn on_outcome(&mut self, pulse: RoutingOutcomePulse) {
           if let Some(prediction) = self.pending_predictions.remove(&pulse.prediction_id) {
               let pred_error = (prediction.expected_quality - pulse.actual_quality).abs();
               self.corrector.update(prediction.expected_quality, pulse.actual_quality);

               // Feed back to CascadeRouter
               self.cascade_router.lock().record_observation(
                   &prediction.model_id,
                   &prediction.task_category,
                   pulse.gate_passed,
                   pulse.cost_usd,
                   pulse.latency_ms,
               );
           }
       }
   }
   ```

8. Wire the cell into the event loop:
   - Subscribe to `routing.prediction` and `routing.outcome` topics
   - On each event, dispatch to the appropriate handler

9. Write tests:
   - Prediction + outcome join produces correct error
   - ResidualCorrector bias elimination after multiple observations
   - Pending predictions are cleaned up (timeout stale entries)
   - CascadeRouter `record_observation` is called with correct values

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- routing_calibration
cargo test -p roko-learn -- prediction_error
cargo check -p roko-cli
```

## What NOT to do
- Do NOT modify the Bus trait or event system — use existing publish/subscribe API
- Do NOT make the calibration cell block the dispatch path — it reacts asynchronously
- Do NOT store unlimited pending predictions — add a TTL/max-size eviction
- Do NOT modify CascadeRouter's public API — use existing `record_observation()`
- Do NOT add real model calls in tests — mock the CascadeRouter
