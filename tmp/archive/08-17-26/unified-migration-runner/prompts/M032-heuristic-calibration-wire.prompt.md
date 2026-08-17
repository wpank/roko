# M032 — Wire heuristic calibration from Verify verdicts

## Objective
Subscribe to `outcome.verify.*` events on the Bus. When a Verify verdict references a Heuristic Signal in its context, update that Heuristic's calibration (increment trials, confirmations/violations based on verdict, recompute Brier score and Wilson CI). Publish `calibration.heuristic.{id}.updated` on the Bus.

## Scope
- Crates: `roko-learn`, `roko-neuro`
- Files:
  - New: `crates/roko-learn/src/heuristic_calibration.rs`
  - `crates/roko-learn/src/lib.rs` (module declaration)
  - `crates/roko-core/src/kind.rs` (HeuristicCalibration from M031)
  - `crates/roko-core/src/topics.rs` (topic constants)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.7
- Spec ref: `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §6.2
- Depends on: M031 (Kind::Heuristic), M024 (Bus wired)

## Steps
1. Verify M031 types exist:
   ```bash
   grep -rn 'HeuristicPayload\|HeuristicCalibration' crates/roko-core/src/ --include='*.rs'
   ```

2. Check how gate verdicts are structured:
   ```bash
   grep -rn 'pub struct Verdict' crates/roko-core/src/ --include='*.rs'
   grep -rn 'context\|evidence\|engram_refs\|signal_refs' crates/roko-core/src/ --include='*.rs' | grep -i verdict
   ```

3. Create `crates/roko-learn/src/heuristic_calibration.rs`:
   ```rust
   //! Automatic calibration of Heuristic signals from Verify verdicts.
   //!
   //! When a gate verdict references a Heuristic in its context, this module
   //! updates the Heuristic's calibration state and publishes a calibration
   //! Pulse on the Bus.
   //!
   //! See: tmp/unified/11-MEMORY-AND-KNOWLEDGE.md §6.2

   use std::collections::HashMap;
   use std::path::{Path, PathBuf};

   /// Tracks heuristic calibration state across sessions.
   pub struct HeuristicCalibrationTracker {
       /// In-memory calibration state per heuristic ID.
       calibrations: HashMap<String, HeuristicCalibration>,
       /// Persistence path.
       state_path: PathBuf,
   }
   ```

4. Implement the core update logic:
   ```rust
   impl HeuristicCalibrationTracker {
       /// Process a Verify verdict that may reference Heuristic signals.
       ///
       /// Returns Pulses to publish on the Bus for any updated heuristics.
       pub fn process_verdict(
           &mut self,
           verdict_passed: bool,
           referenced_heuristic_ids: &[String],
       ) -> Vec<Pulse> {
           let mut pulses = Vec::new();
           for heuristic_id in referenced_heuristic_ids {
               let cal = self.calibrations
                   .entry(heuristic_id.clone())
                   .or_default();
               cal.trials += 1;
               if verdict_passed {
                   cal.confirmations += 1;
               } else {
                   cal.violations += 1;
               }
               // Update Brier score and Wilson CI
               cal.update_brier_score(verdict_passed);
               cal.update_wilson_ci();

               pulses.push(Pulse::builder(
                   0, // seq assigned by bus
                   Topic::new(&format!("{}.{}", topics::CALIBRATION_HEURISTIC, heuristic_id)),
                   Kind::Metric,
               )
               .body(Body::json(serde_json::to_value(cal).unwrap()))
               .build());
           }
           pulses
       }
   }
   ```

5. Add persistence (load/save from `.roko/learn/heuristic-calibration.json`).

6. Wire into orchestrate.rs or the gate evaluation path — after a gate verdict, check if any Heuristic signals were in the agent's context, and if so, call `process_verdict()`.

7. Also wire into Compose context assembly (cross-reference with M031): well-calibrated heuristics (high Wilson CI lower bound) should bid higher for context slots.

8. Add tests:
   ```rust
   #[test]
   fn verdict_updates_heuristic_calibration() {
       let mut tracker = HeuristicCalibrationTracker::new(PathBuf::from("/tmp/test"));
       let pulses = tracker.process_verdict(true, &["heur-1".into()]);
       assert_eq!(pulses.len(), 1);
       let cal = &tracker.calibrations["heur-1"];
       assert_eq!(cal.trials, 1);
       assert_eq!(cal.confirmations, 1);
   }
   ```

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- heuristic_calibration
```

## What NOT to do
- Do NOT modify the Verify/Gate pipeline — this is a subscriber, not part of the gate logic
- Do NOT assume every verdict references heuristics — most won't; handle the empty case gracefully
- Do NOT couple tightly to a specific store — accept heuristic IDs as strings, let the caller resolve them
- Do NOT add Bus subscription directly — implement as a Policy that receives Pulses via decide_with_pulses
