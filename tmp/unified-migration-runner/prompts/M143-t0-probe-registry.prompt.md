# M143 — Create T0 Probe Registry

## Objective
Create a formal `Probe` trait and register two universal probes (WorldModelDrift, CausalConsistency) as zero-cost T0 checks. The existing `HeartbeatProbeRegistry` in `heartbeat_probes.rs` uses an `EngineState`-based evaluation model but lacks a trait-based interface for extensibility. Define the `Probe` trait with `evaluate()`, `weight()`, `name()`, `domain()` methods and wire probe evaluation into the gamma tick handler so probes run every reactive cycle without LLM calls.

## Scope
- Crates: `roko-runtime`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat_probes.rs` (extend)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat.rs` (wire into gamma tick)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs` (re-export)
- Depth doc: `tmp/unified-depth/05-heartbeat/` (probe architecture)

## Steps
1. Read the existing probe infrastructure:
   ```bash
   grep -n 'pub struct.*Probe\|pub trait\|pub fn evaluate' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat_probes.rs | head -20
   grep -n 'EngineState\|ProbeDomain\|ProbeResult' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat_probes.rs | head -15
   ```

2. Define the `Probe` trait in `heartbeat_probes.rs`:
   ```rust
   /// A zero-cost T0 probe that evaluates system health without LLM calls.
   ///
   /// Probes are the cheapest possible health check — they read atomic state
   /// and return a normalized score in [0.0, 1.0] where 0.0 = healthy, 1.0 = critical.
   pub trait Probe: Send + Sync {
       /// Evaluate the probe against current cortical state.
       fn evaluate(&self, state: &CorticalState) -> f32;
       /// Relative weight for aggregation (default 1.0).
       fn weight(&self) -> f32 { 1.0 }
       /// Human-readable probe name.
       fn name(&self) -> &str;
       /// Domain this probe belongs to (for selective evaluation).
       fn domain(&self) -> ProbeDomain;
   }
   ```

3. Implement `WorldModelDrift` probe:
   ```rust
   /// Detects drift between predicted and observed world state.
   /// Reads `prediction_accuracy` and `performance_trend` from CorticalState.
   /// Score = 1.0 - accuracy + abs(trend_delta).
   pub struct WorldModelDrift;
   ```

4. Implement `CausalConsistency` probe:
   ```rust
   /// Detects causal inconsistencies in recent observations.
   /// Reads `surprise_rate` and `knowledge_health` from CorticalState.
   /// Score = surprise_rate * (1.0 - knowledge_health).
   pub struct CausalConsistency;
   ```

5. Create `ProbeRegistry` that holds `Vec<Box<dyn Probe>>`:
   ```rust
   pub struct ProbeRegistry {
       probes: Vec<Box<dyn Probe>>,
   }

   impl ProbeRegistry {
       pub fn new() -> Self { ... }
       pub fn with_universal_probes() -> Self { ... }
       pub fn register(&mut self, probe: Box<dyn Probe>) { ... }
       pub fn evaluate_all(&self, state: &CorticalState) -> Vec<(String, f32)> { ... }
       pub fn weighted_aggregate(&self, state: &CorticalState) -> f32 { ... }
   }
   ```

6. Wire into gamma tick: after each gamma tick fires on the bus, evaluate all registered probes against the current `CorticalState` snapshot. Store results for prediction error calculation (M145).

7. Write tests:
   - `WorldModelDrift` returns 0.0 when accuracy=1.0 and trend=0.0
   - `CausalConsistency` returns 0.0 when surprise_rate=0.0
   - `ProbeRegistry::weighted_aggregate` correctly weights multiple probes
   - Probes are `Send + Sync` (compile-time test)

## Verification
```bash
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- probe
```

## What NOT to do
- Do NOT replace `HeartbeatProbeRegistry` or `StatefulProbeRegistry` — the new `Probe` trait is a parallel abstraction that takes `CorticalState` instead of `EngineState`
- Do NOT add LLM calls — probes are pure computation over atomic state
- Do NOT implement domain-specific probes here — coding probes are M160
- Do NOT wire into orchestrate.rs directly — M142 handles the spawn, this batch just adds the evaluation logic
- Do NOT add rolling stats to individual probes — `StatefulProbeRegistry` already handles that concern
