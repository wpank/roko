# M128 — Contagion accumulator and attenuation Functor

## Objective
Extend the existing contagion system with structured `AttenuationFunctor`, `ContagionAccumulator` with half-life decay, and `ContagionReceiver` with mix weight. The existing codebase already has `ContagionEvent`, `ContagionTrigger`, `BorrowedAffect`, `SomaticField`, `contagion()`, and `contagion_susceptibility()` in phase2_stubs.rs — this batch adds the structured accumulator/receiver pattern on top.

## Scope
- Crates: `roko-daimon`
- Files:
  - New: `crates/roko-daimon/src/contagion.rs`
  - `crates/roko-daimon/src/phase2_stubs.rs` (reference — existing `ContagionEvent`, `ContagionTrigger`, `BorrowedAffect`, `SomaticField`, `contagion()`, `contagion_susceptibility()`)
  - `crates/roko-daimon/src/lib.rs` (module decl, re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/21-collective-contagion.md`

## Existing types reference

Already in `crates/roko-daimon/src/phase2_stubs.rs`:
```rust
pub enum ContagionTrigger { GateFailure, TaskBlocked, HighArousal, DreamOutcome }

pub struct ContagionEvent {
    pub source_agent: String,
    pub trigger: ContagionTrigger,
    pub source_pad: PadVector,
}

pub struct BorrowedAffect {
    pub source_agent: String,
    pub pad: PadVector,
    pub received_at: DateTime<Utc>,
}

pub struct SomaticField {
    pub markers: Vec<SomaticMarker>,
    pub accuracy_weights: HashMap<String, f64>,
}
impl SomaticField {
    pub fn accept_contribution(&mut self, marker: SomaticMarker, contributor: &str, accuracy: f64) { ... }
    pub fn query(&self, coords: &[f64; STRATEGY_DIMENSIONS], k: usize) -> Vec<SomaticMarker> { ... }
}

/// Existing maturity-based susceptibility (tick count based)
pub fn contagion_susceptibility(tick_count: u64) -> f64 { ... }

/// Existing contagion function
pub fn contagion(my_affect: &PadVector, peer_affects: &[PadVector], tick_count: u64) -> PadVector { ... }
```

DaimonState already has `pub borrowed_affect: Vec<BorrowedAffect>` and `fn apply_contagion(&mut self, event: ContagionEvent)`.

## Steps
1. Discover the full existing contagion API:
   ```bash
   grep -rn 'ContagionEvent\|ContagionTrigger\|contagion\|BorrowedAffect\|SomaticField' crates/roko-daimon/src/phase2_stubs.rs | head -25
   grep -rn 'apply_contagion\|borrowed_affect' crates/roko-daimon/src/lib.rs | head -10
   ```

2. Create `crates/roko-daimon/src/contagion.rs` with structured types:
   ```rust
   use chrono::{DateTime, Utc};
   use roko_primitives::PadVector;
   use roko_core::BehavioralState;
   use serde::{Deserialize, Serialize};

   /// Attenuation functor for dampening PAD at group boundaries.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AttenuationFunctor {
       pub pleasure_coeff: f64,   // default 0.3
       pub arousal_coeff: f64,    // default 0.3
       pub dominance_coeff: f64,  // default 0.0
       pub arousal_cap: f64,      // default 0.3 — hard cap on arousal after attenuation
   }
   impl Default for AttenuationFunctor {
       fn default() -> Self { Self { pleasure_coeff: 0.3, arousal_coeff: 0.3, dominance_coeff: 0.0, arousal_cap: 0.3 } }
   }
   impl AttenuationFunctor {
       pub fn attenuate(&self, pad: &PadVector) -> PadVector {
           PadVector::new(
               pad.pleasure * self.pleasure_coeff,
               (pad.arousal * self.arousal_coeff).clamp(-self.arousal_cap, self.arousal_cap),
               pad.dominance * self.dominance_coeff,
           )
       }
   }
   ```

3. Add `ContagionAccumulator`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ContagionAccumulator {
       accumulated: PadVector,
       last_update: DateTime<Utc>,
       half_life_hours: f64,  // default 6.0
   }
   impl ContagionAccumulator {
       pub fn absorb(&mut self, attenuated_pad: &PadVector, now: DateTime<Utc>) {
           // Apply half-life decay to existing accumulated, then add new
       }
       pub fn current(&self, now: DateTime<Utc>) -> PadVector {
           // Apply decay to accumulated based on elapsed time
       }
   }
   ```

4. Add `ContagionReceiver`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ContagionReceiver {
       pub accumulator: ContagionAccumulator,
       pub attenuation: AttenuationFunctor,
       pub mix_weight: f64,  // default 0.1
   }
   impl ContagionReceiver {
       pub fn receive(&mut self, peer_pad: &PadVector, now: DateTime<Utc>) -> PadVector {
           let attenuated = self.attenuation.attenuate(peer_pad);
           self.accumulator.absorb(&attenuated, now);
           // Return delta: mix_weight * accumulated_current
       }
   }
   ```

5. Add behavioral-state-based susceptibility (complement to existing tick-based):
   ```rust
   pub fn behavioral_susceptibility(state: BehavioralState) -> f64 {
       match state {
           BehavioralState::Focused => 0.7,
           BehavioralState::Engaged => 0.8,
           BehavioralState::Struggling => 1.2,
           BehavioralState::Exploring => 1.1,
           BehavioralState::Resting => 0.5,
           BehavioralState::Coasting => 1.0,
       }
   }
   ```

6. Add privacy stripping:
   ```rust
   pub fn strip_for_sharing(marker: &SomaticMarker) -> SomaticMarker {
       let mut stripped = marker.clone();
       stripped.episodes.clear();
       stripped
   }
   ```

7. Add module to `lib.rs`:
   ```rust
   pub mod contagion;
   pub use contagion::{AttenuationFunctor, ContagionAccumulator, ContagionReceiver, behavioral_susceptibility};
   ```

8. Add tests:
   - Attenuation produces expected dampened values (D component is zeroed)
   - Arousal is hard-capped at 0.3 after attenuation
   - Accumulator decays to half after 6 hours
   - Anti-cascade: cascade of 5 agents results in < 0.01 effect on the 5th
   - Susceptibility is higher for Struggling (1.2) than Focused (0.7)
   - Privacy strip clears episodes vector

## Verification
```bash
cargo check -p roko-daimon
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-daimon -- contagion
cargo test -p roko-daimon -- attenuation
cargo test -p roko-daimon -- accumulator
```

## What NOT to do
- Do NOT modify the existing `contagion()` / `contagion_susceptibility()` functions in phase2_stubs.rs
- Do NOT modify the existing `SomaticField` in phase2_stubs.rs — it is already implemented
- Do NOT wire into Bus relay infrastructure — just the data types and algorithms
- Do NOT add Group coordination — that requires roko-runtime
- Do NOT make this depend on roko-serve — pure daimon types
