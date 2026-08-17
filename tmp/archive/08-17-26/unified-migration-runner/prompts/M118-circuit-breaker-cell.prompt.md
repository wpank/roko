# M118 — Circuit breaker as state-machine Cell with AIMD

## Objective
Extend the existing `CircuitBreaker` in `roko-conductor` with an explicit Closed/Open/HalfOpen state machine, AIMD concurrency control, and snapshot/restore for persistence. The existing breaker already tracks per-plan failures via `DashMap<String, FailureRecord>` and has predictive tripping via `HoltForecaster` (COND-08) — this batch adds the three-state model and probe-based recovery on top.

## Scope
- Crates: `roko-conductor`
- Files:
  - `crates/roko-conductor/src/circuit_breaker.rs` (extend — DO NOT rewrite)
  - New: `crates/roko-conductor/src/aimd.rs`
  - `crates/roko-conductor/src/lib.rs` (re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/15-circuit-breaker-and-interventions.md`

## Existing types reference

The `CircuitBreaker` already exists:
```rust
// crates/roko-conductor/src/circuit_breaker.rs
pub struct CircuitBreaker {
    max_failures: u32,
    records: DashMap<String, FailureRecord>,       // per-plan failure records
    forecasters: DashMap<String, HoltForecaster>,   // per-plan Holt forecasters (COND-08)
    eval_counts: DashMap<String, (u32, u32)>,       // (successes, failures) per plan
    predictive: bool,
    forecast_trip_threshold: f64,
}
```

Already has: `HoltForecaster` (Holt exponential smoothing), `ProactiveTripSignal`, `FailureRecord`, `CircuitBreakerState` (for serialization with `max_failures` + `records: HashMap`), `with_predictive()`, `is_predictive()`.

Does NOT yet have: Closed/Open/HalfOpen state machine, probe-based recovery, AIMD.

## Steps
1. Discover the full existing API:
   ```bash
   grep -rn 'pub fn\|pub const fn\|pub struct\|pub enum' crates/roko-conductor/src/circuit_breaker.rs | head -30
   # Check what's re-exported
   grep -rn 'circuit_breaker' crates/roko-conductor/src/lib.rs
   ```

2. Add a `BreakerPhase` enum to `circuit_breaker.rs`:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub enum BreakerPhase {
       /// Normal operation — counting failures toward threshold.
       Closed,
       /// Tripped — all work for this plan is blocked until cooldown expires.
       Open {
           opened_at_ms: u64,
           cooldown_ms: u64,
           reason: String,
       },
       /// Recovery probe — allowing a fraction of work through to test recovery.
       HalfOpen {
           probe_fraction: f64,
           probes_sent: u32,
           probes_passed: u32,
       },
   }
   ```

3. Add per-plan phase tracking alongside the existing `records` DashMap:
   ```rust
   // Add field to CircuitBreaker
   phases: DashMap<String, BreakerPhase>,
   ```

4. Add transition methods:
   ```rust
   impl CircuitBreaker {
       /// Transition a plan to Open phase when failure threshold is hit.
       pub fn trip_open(&self, plan_id: &str, reason: &str, cooldown_ms: u64) { ... }
       /// Check if a plan's cooldown has expired and transition to HalfOpen.
       pub fn check_cooldown(&self, plan_id: &str, now_ms: u64) -> bool { ... }
       /// Record a probe result in HalfOpen phase.
       pub fn record_probe(&self, plan_id: &str, success: bool) { ... }
       /// Get the current phase for a plan (defaults to Closed).
       pub fn phase(&self, plan_id: &str) -> BreakerPhase { ... }
   }
   ```

5. The predictive tripping already exists. Integrate it with the new phases: when `check_proactive_trip()` fires a `ProactiveTripSignal`, call `trip_open()`.

6. Create `crates/roko-conductor/src/aimd.rs`:
   ```rust
   use serde::{Deserialize, Serialize};

   /// Additive-Increase Multiplicative-Decrease concurrency controller.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AimdController {
       pub concurrency: f64,
       pub max_concurrency: f64,
       pub additive_increase: f64,      // default 1.0
       pub multiplicative_decrease: f64, // default 0.5
   }

   impl Default for AimdController {
       fn default() -> Self {
           Self { concurrency: 4.0, max_concurrency: 16.0, additive_increase: 1.0, multiplicative_decrease: 0.5 }
       }
   }

   impl AimdController {
       pub fn on_success(&mut self) { self.concurrency = (self.concurrency + self.additive_increase).min(self.max_concurrency); }
       pub fn on_failure(&mut self) { self.concurrency = (self.concurrency * self.multiplicative_decrease).max(1.0); }
       pub fn allowed_concurrency(&self) -> usize { self.concurrency.floor() as usize }
   }
   ```

7. Extend the existing `CircuitBreakerState` (used for snapshot/restore) to include phase data:
   ```rust
   // Existing: pub struct CircuitBreakerState { pub max_failures: u32, pub records: HashMap<String, FailureRecord> }
   // Add:
   #[serde(default)]
   pub phases: HashMap<String, BreakerPhase>,
   ```

8. Add `aimd` module to `crates/roko-conductor/src/lib.rs` and re-export `AimdController`. Add `BreakerPhase` to the `circuit_breaker` re-exports.

9. Add tests:
   - Phase defaults to Closed for unknown plans
   - `trip_open()` sets phase to Open
   - `check_cooldown()` transitions Open -> HalfOpen after cooldown expires
   - Probe success in HalfOpen -> Closed
   - Probe failure in HalfOpen -> Open with doubled cooldown
   - AIMD increases on success, decreases on failure, floors at 1.0
   - Snapshot/restore preserves phases

## Verification
```bash
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- circuit_breaker
cargo test -p roko-conductor -- aimd
cargo test -p roko-conductor -- breaker_phase
```

## What NOT to do
- Do NOT remove or rewrite the existing `CircuitBreaker` fields or public API — extend it
- Do NOT remove `DashMap`-based concurrent access — add `phases: DashMap` alongside
- Do NOT wire AIMD into the orchestrator — that is integration work
- Do NOT add Bus Pulse publication yet
- Do NOT make the Cell depend on roko-core `Cell` trait — just add the state-machine pattern
