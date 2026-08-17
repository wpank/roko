# M144 — Wire CorticalState Updates from Probes and Daimon

## Objective
The `CorticalState` struct already exists in `heartbeat.rs` with 20+ atomic fields and accessor methods. What is missing is the *update path*: nothing currently writes to CorticalState during runtime execution. Wire the gamma tick to update CorticalState from probe results (M143) and from the Daimon PAD (pleasure/arousal/dominance) state. Also add the `CorticalSnapshot` serialization so downstream systems (prediction error, tier gating) can read a consistent snapshot.

## Scope
- Crates: `roko-runtime`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs` (update logic)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (wire Daimon PAD → CorticalState)
- Depth doc: `tmp/unified-depth/05-heartbeat/` (cortical state update protocol)

## Steps
1. Read the existing CorticalState and its snapshot method:
   ```bash
   grep -n 'pub fn snapshot\|CorticalSnapshot\|pub struct Cortical' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs | head -10
   ```

2. Read how DaimonState is currently used in orchestrate.rs:
   ```bash
   grep -n 'DaimonState\|daimon\|pad\|PAD\|pleasure\|arousal\|dominance' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -15
   ```

3. Add `update_from_probes()` method to CorticalState:
   ```rust
   impl CorticalState {
       /// Update cortical signals from probe evaluation results.
       ///
       /// Called on every gamma tick after probes run. Updates:
       /// - aggregate_accuracy (weighted probe average)
       /// - accuracy_trend (EMA direction)
       /// - surprise_rate (anomaly count / total probes)
       /// - performance_trend (delta from previous tick)
       pub fn update_from_probes(&self, results: &[(String, f32)], weighted_agg: f32) {
           // ...
       }
   }
   ```

4. Add `update_from_pad()` method:
   ```rust
   impl CorticalState {
       /// Sync PAD vector from Daimon affect engine.
       pub fn update_from_pad(&self, pleasure: f32, arousal: f32, dominance: f32) {
           self.pleasure.store(pleasure.to_bits(), Ordering::Release);
           self.arousal.store(arousal.to_bits(), Ordering::Release);
           self.dominance.store(dominance.to_bits(), Ordering::Release);
       }
   }
   ```

5. Wire in orchestrate.rs: after each task dispatch, if DaimonState is loaded, call `cortical_state.update_from_pad(daimon.pleasure(), daimon.arousal(), daimon.dominance())`.

6. Ensure `CorticalSnapshot` is `Serialize + Deserialize` for logging/debugging:
   ```bash
   grep -n 'CorticalSnapshot' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs | head -5
   ```

7. Wire gamma tick handler: after probes evaluate, call `cortical_state.update_from_probes(results, aggregate)`.

8. Write tests:
   - `update_from_probes` correctly updates aggregate_accuracy
   - `update_from_pad` stores and retrieves correct f32 values
   - `snapshot()` returns consistent values after updates

## Verification
```bash
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- cortical
cargo check -p roko-cli
```

## What NOT to do
- Do NOT restructure CorticalState — add methods to the existing struct
- Do NOT add new atomic fields — the 20+ existing fields are sufficient
- Do NOT make CorticalState mutable (&mut self) — it uses atomics for lock-free concurrent access
- Do NOT remove any existing accessor methods
- Do NOT wire prediction error here — that is M145
