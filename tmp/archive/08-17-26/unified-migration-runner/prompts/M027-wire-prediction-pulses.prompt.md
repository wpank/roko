# M027 — Wire prediction Pulses for Score, Route, and Compose

## Objective
When a Score cell rates a Signal, a Route cell selects a model, or a Compose cell assembles context, publish prediction Pulses on the Bus. After the gate verdict comes back, publish outcome Pulses. This completes the predict-publish-correct data pipeline that CalibrationReact (M026) consumes.

## Scope
- Crates: `roko-learn`, `roko-compose`, `roko-gate`, `roko-cli`
- Files:
  - `crates/roko-learn/src/cascade_router.rs` (Route predictions)
  - `crates/roko-compose/src/` (Compose predictions)
  - `crates/roko-gate/src/` (outcome after verdict)
  - `crates/roko-cli/src/orchestrate.rs` (wiring point)
  - `crates/roko-core/src/topics.rs` (topic constants)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.5
- Spec ref: `tmp/unified/10-LEARNING-LOOPS.md` §2.1-2.3

## Steps
1. Identify where routing decisions are made:
   ```bash
   grep -n 'explain_routing\|select_model\|route_for\|fn route' crates/roko-learn/src/cascade_router.rs | head -10
   ```

2. Identify where context assembly happens:
   ```bash
   grep -rn 'fn compose\|fn assemble\|fn build_prompt\|SystemPromptBuilder' crates/roko-compose/src/ --include='*.rs' | head -10
   ```

3. For Route predictions — after CascadeRouter selects a model, publish:
   ```rust
   // In the routing code path (or orchestrate.rs wrapper):
   let prediction_pulse = Pulse::builder(seq_next(), Topic::new(
       &format!("{}.cascade", topics::PREDICTION_ROUTE)), Kind::Metric)
       .body(Body::json(json!({
           "model": selected_model,
           "expected_quality": confidence,
           "task_id": task_id,
       })))
       .lineage_hint(task_signal_ref)
       .build();
   bus.publish(prediction_pulse)?;
   ```

4. For Compose predictions — after prompt assembly, publish:
   ```rust
   let prediction_pulse = Pulse::builder(seq_next(), Topic::new(
       &format!("{}.sections", topics::PREDICTION_COMPOSE)), Kind::Metric)
       .body(Body::json(json!({
           "included_sections": section_ids,
           "total_tokens": token_count,
           "task_id": task_id,
       })))
       .build();
   ```

5. For outcomes — after gate verdict in orchestrate.rs, publish outcome Pulses:
   ```rust
   // After gate results:
   let outcome_pulse = Pulse::builder(seq_next(), Topic::new(topics::OUTCOME_ROUTE), Kind::Metric)
       .body(Body::json(json!({
           "model": used_model,
           "verdict": verdict_str,
           "task_id": task_id,
       })))
       .lineage_hint(task_signal_ref)  // matches the prediction
       .build();
   bus.publish(outcome_pulse)?;
   ```

6. The key linking mechanism is `lineage_hint` — both the prediction and outcome Pulses for the same task should share the same lineage_hint (typically the task signal ref or task ID), enabling CalibrationReact to join them.

7. This should be wired primarily in `orchestrate.rs` where the full dispatch→gate flow is orchestrated, wrapping the existing function calls.

8. Add a test in orchestrate.rs or a new integration test:
   ```rust
   #[tokio::test]
   async fn prediction_outcome_pulses_emitted_during_dispatch() {
       // Setup: mock bus, mock agent, mock gate
       // Execute: dispatch a task
       // Verify: bus received prediction.route.* and outcome.route.* pulses
       // Verify: lineage_hints match between prediction and outcome
   }
   ```

## Verification
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
# Verify prediction pulses are published:
grep -rn 'prediction\.\|PREDICTION_' crates/roko-cli/src/orchestrate.rs
# Should see prediction + outcome publishing
```

## What NOT to do
- Do NOT modify the CascadeRouter's selection logic — just add Pulse emission around it
- Do NOT make Bus a required parameter for routing/composition — use Option and skip emission if None
- Do NOT add complex correlation logic here — CalibrationReact (M026) handles correlation
- Do NOT publish Pulses from deep inside library code — emit from the orchestration layer where the full context is available
