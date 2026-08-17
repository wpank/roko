# M124 — Appraisal Pipeline and ALMA temporal model

## Objective
Implement the 8-step appraisal pipeline as a sequence of functions operating on PAD state, and the ALMA 3-layer temporal model. This structures the existing `DaimonState::appraise()` logic into a composable pipeline with temporal smoothing. The existing appraisal is monolithic; this adds a layered alternative.

## Scope
- Crates: `roko-daimon`
- Files:
  - New: `crates/roko-daimon/src/appraisal.rs`
  - New: `crates/roko-daimon/src/alma.rs`
  - `crates/roko-daimon/src/lib.rs` (module declarations, re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/18-affect-as-functor.md`

## Existing types reference

The daimon already has appraisal types (`crates/roko-daimon/src/lib.rs`):

```rust
// AffectEvent — the input enum (already has 7+ variants)
pub enum AffectEvent {
    GateResult { plan_id, task_id, passed: bool, rung: u32 },
    TaskOutcome { task_id, succeeded: bool },
    Blocked { task_id, blocker_count: usize },
    TimePressure { task_id, deadline_proximity: f64 },
    QueueWait { task_id, wait_hours: f64 },
    DreamFailure { task_type, failure_count: usize },
    DreamCompletion { consolidated_episodes, new_playbooks, arousal_delta },
    // ... more variants
}

// AffectState — the current PAD + confidence snapshot
pub struct AffectState {
    pub pad: PadVector,
    pub confidence: f64,
    pub updated_at: DateTime<Utc>,
    // ... more fields
}

// DaimonState has fn appraise(&mut self, event: AffectEvent) -> PadVector
// which does the full appraisal in a single method (monolithic)
```

`PadVector` is defined in `roko-primitives/src/pad.rs` (not roko-core).

## Steps
1. Discover the full existing appraisal API:
   ```bash
   grep -rn 'fn appraise\|AffectEvent\|AffectState' crates/roko-daimon/src/lib.rs | head -20
   grep -rn 'pub struct AffectState' crates/roko-daimon/src/lib.rs | head -5
   # Check for ALMA or multi-layer code
   grep -rn 'alma\|emotion.*layer\|mood.*layer\|personality.*layer' crates/roko-daimon/src/ --include='*.rs' | head -10
   # Check for EmotionalTag (already has mood_snapshot field)
   grep -rn 'mood_snapshot' crates/roko-core/src/affect.rs
   ```

2. Create `crates/roko-daimon/src/appraisal.rs` with the 8-step pipeline as standalone functions. Do NOT create a new `AppraisalEvent` enum — use the existing `AffectEvent`:
   ```rust
   use roko_primitives::PadVector;
   use super::AffectEvent;

   /// Contextual grounding for an affect event.
   #[derive(Debug, Clone)]
   pub struct AppraisalGrounding {
       pub rung_scale: f64,
       pub blocked_count: u32,
       pub time_proximity: f64,
   }

   /// Delta to apply to PAD state.
   #[derive(Debug, Clone)]
   pub struct PadDelta {
       pub pleasure: f64,
       pub arousal: f64,
       pub dominance: f64,
       pub confidence: f64,
   }

   /// Result of the full appraisal pipeline.
   #[derive(Debug, Clone)]
   pub struct AppraisalResult {
       pub pad: PadVector,
       pub delta: PadDelta,
       pub should_emit: bool,
   }

   // 8-step pipeline functions:
   pub fn ground(event: &AffectEvent) -> AppraisalGrounding { ... }
   pub fn scale(event: &AffectEvent, ground: &AppraisalGrounding) -> PadDelta {
       // Prospect theory: failure deltas are 2x magnitude of success deltas
   }
   pub fn compute(alma: &mut AlmaState, delta: &PadDelta) { ... }
   pub fn decay(alma: &mut AlmaState, elapsed_hours: f64, half_life: f64) { ... }
   pub fn apply(alma: &AlmaState) -> PadVector { ... }
   pub fn persist(alma: &AlmaState) -> serde_json::Value { ... }
   pub fn should_emit(before: &PadVector, after: &PadVector, threshold: f64) -> bool {
       before.distance(after) > threshold  // uses distance() from M123
   }
   pub fn run_appraisal(alma: &mut AlmaState, event: &AffectEvent, half_life: f64) -> AppraisalResult { ... }
   ```

3. Create `crates/roko-daimon/src/alma.rs`:
   ```rust
   use chrono::{DateTime, Utc};
   use roko_primitives::PadVector;
   use serde::{Deserialize, Serialize};

   /// ALMA 3-layer temporal affect model.
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct AlmaState {
       pub emotion: PadVector,      // fast layer, tau=0.1
       pub mood: PadVector,         // slow layer, tau=0.5
       pub personality: PadVector,  // near-static, tau=0.9
       pub confidence: f64,
       pub last_decay: DateTime<Utc>,
       pub tick_count: u64,
   }
   impl AlmaState {
       pub fn new() -> Self { ... }
       pub fn effective_pad(&self) -> PadVector { /* 0.5*emotion + 0.3*mood + 0.2*personality */ }
       pub fn update_emotion(&mut self, stimulus: &PadVector) { /* EMA: e = tau*stimulus + (1-tau)*e, tau=0.1 */ }
       pub fn update_mood(&mut self) { /* EMA of emotion into mood, tau=0.5 */ }
       pub fn update_personality(&mut self) { /* EMA of mood into personality, tau=0.9 */ }
       pub fn decay_toward_baseline(&mut self, factor: f64) { /* multiplicative decay via PadVector::decay_by_factor */ }
   }
   ```

4. Add `ema_blend` and `decay_pad` helper functions:
   ```rust
   /// Use PadVector::new3() (from M123) for backward compat — confidence defaults to 0.5.
   /// If M123 is already applied, use new3() or the 4-arg new() with blended confidence.
   fn ema_blend(current: &PadVector, target: &PadVector, tau: f64) -> PadVector {
       PadVector {
           pleasure: current.pleasure * (1.0 - tau) + target.pleasure * tau,
           arousal: current.arousal * (1.0 - tau) + target.arousal * tau,
           dominance: current.dominance * (1.0 - tau) + target.dominance * tau,
           ..Default::default()  // confidence defaults via serde or Default
       }
   }
   ```

5. Wire the new modules into `lib.rs`:
   ```rust
   pub mod alma;
   pub mod appraisal;
   pub use alma::AlmaState;
   pub use appraisal::{AppraisalGrounding, AppraisalResult, PadDelta, run_appraisal};
   ```

6. Add tests:
   - GateFail delta is ~2x GatePass delta magnitude (prospect theory)
   - Effective PAD is weighted blend of 3 layers
   - Emotion layer reacts quickly (tau=0.1), mood slowly (tau=0.5)
   - Decay moves PAD toward neutral over time
   - `should_emit()` fires when distance > threshold (default 0.15)
   - AlmaState serializes/deserializes correctly

## Verification
```bash
cargo check -p roko-daimon
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-daimon -- appraisal
cargo test -p roko-daimon -- alma
```

## What NOT to do
- Do NOT modify the existing `DaimonState::appraise()` method — add parallel implementation
- Do NOT create a new `AppraisalEvent` enum — use the existing `AffectEvent`
- Do NOT add Bus subscription — just define the pipeline functions
- Do NOT wire into orchestrate.rs — that is M126
- Do NOT implement the full Cell trait on each step — they are plain functions
