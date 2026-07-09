# M125 — Behavioral state Score Cell with hysteresis

## Objective
Add a continuous archetype-scoring function and `RoutingModulation` struct on top of the existing behavioral state classification. The 6 behavioral states, `BehavioralStateTracker` with hysteresis and 10-tick dwell minimum, and `BehavioralStateThresholds` with asymmetric entry/exit already exist. This batch adds the scoring dimension (distance from archetype centers) and the routing modulation mapping.

## Scope
- Crates: `roko-daimon`
- Files:
  - New: `crates/roko-daimon/src/behavioral_scorer.rs`
  - `crates/roko-daimon/src/phase2_stubs.rs` (reference — read existing `BehavioralStateTracker`, `BehavioralStateThresholds`, `classify_with_hysteresis()`)
  - `crates/roko-daimon/src/lib.rs` (module decl, re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/19-behavioral-states-and-routing.md`

## Existing types reference

The `BehavioralState` enum is in `crates/roko-core/src/affect.rs` with 6 variants:
```rust
pub enum BehavioralState { Engaged, Struggling, Coasting, Exploring, Focused, Resting }
impl BehavioralState {
    pub fn classify(pad: PadVector, confidence: f64) -> Self { ... }  // memoryless
}
```

The `BehavioralStateTracker` already exists in `crates/roko-daimon/src/phase2_stubs.rs`:
```rust
pub struct BehavioralStateThresholds {
    pub struggling_entry_confidence: f64,  // 0.30
    pub struggling_exit_confidence: f64,   // 0.40
    pub struggling_entry_dominance: f64,   // -0.25
    pub struggling_exit_dominance: f64,    // -0.15
    pub coasting_entry_pleasure: f64,      // 0.35
    pub coasting_exit_pleasure: f64,       // 0.25
    pub resting_entry_arousal: f64,        // -0.20
    pub resting_exit_arousal: f64,         // -0.10
}

pub fn classify_with_hysteresis(state: &AffectState, current: BehavioralState, thresholds: &BehavioralStateThresholds) -> BehavioralState { ... }

pub struct BehavioralStateTracker {
    pub current_state: BehavioralState,
    pub entered_at: u64,
    pub min_dwell_ticks: u64,  // default 10
    pub thresholds: BehavioralStateThresholds,
}
impl BehavioralStateTracker {
    pub fn update(&mut self, state: &AffectState, current_tick: u64) -> BehavioralState { ... }
}
```

The `DaimonState` already has `pub behavioral_tracker: BehavioralStateTracker`.

`PadVector` is defined in `roko-primitives/src/pad.rs`, re-exported by roko-core.

## Steps
1. Discover the full existing API:
   ```bash
   grep -rn 'BehavioralState\|BehavioralStateTracker\|BehavioralStateThresholds\|classify_with_hysteresis' crates/roko-daimon/src/phase2_stubs.rs | head -20
   grep -rn 'BehavioralState' crates/roko-core/src/affect.rs | head -10
   grep -rn 'behavioral_tracker' crates/roko-daimon/src/lib.rs | head -5
   ```

2. Create `crates/roko-daimon/src/behavioral_scorer.rs`:
   ```rust
   use roko_core::BehavioralState;
   use roko_primitives::PadVector;
   use serde::{Deserialize, Serialize};

   /// Archetype PAD center for each behavioral state (P, A, D, confidence).
   pub fn archetype_center(state: BehavioralState) -> (f64, f64, f64, f64) {
       match state {
           BehavioralState::Engaged => (0.0, 0.0, 0.0, 0.50),
           BehavioralState::Struggling => (-0.4, 0.5, -0.4, 0.20),
           BehavioralState::Coasting => (0.5, -0.1, 0.2, 0.75),
           BehavioralState::Focused => (0.3, 0.1, 0.5, 0.60),
           BehavioralState::Resting => (0.0, -0.4, 0.0, 0.50),
           BehavioralState::Exploring => (0.1, 0.2, -0.1, 0.45),
       }
   }
   ```

3. Add `BehavioralScorer` that wraps `BehavioralStateTracker` with archetype scoring:
   ```rust
   pub struct BehavioralScorer {
       tracker: BehavioralStateTracker,
   }
   impl BehavioralScorer {
       pub fn new() -> Self { Self { tracker: BehavioralStateTracker::default() } }

       /// Score the current PAD against all 6 state archetypes.
       /// Returns (state, distance_score) pairs sorted by distance (closest first).
       pub fn score_all_states(&self, pad: &PadVector, confidence: f64) -> Vec<(BehavioralState, f64)> {
           // For each state, compute Euclidean distance from pad to archetype center
           // Convert to a similarity score: score = 1.0 / (1.0 + distance)
       }

       /// Classify with hysteresis + dwell time (delegates to tracker).
       pub fn classify(&mut self, state: &AffectState, tick: u64) -> (BehavioralState, bool) {
           let prev = self.tracker.current_state;
           let next = self.tracker.update(state, tick);
           (next, next != prev)
       }
   }
   ```

4. Add `RoutingModulation` struct:
   ```rust
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct RoutingModulation {
       /// Multiplier on model cost tolerance. < 1.0 = prefer expensive (Struggling), > 1.0 = prefer cheap (Coasting).
       pub cost_multiplier: f64,
       /// Bonus for exploratory/epistemic routing choices.
       pub epistemic_bonus: f64,
       /// Maximum retry budget for this behavioral state.
       pub retry_budget: u8,
   }
   impl RoutingModulation {
       pub fn from_state(state: BehavioralState) -> Self {
           match state {
               BehavioralState::Engaged => Self { cost_multiplier: 1.0, epistemic_bonus: 0.0, retry_budget: 3 },
               BehavioralState::Struggling => Self { cost_multiplier: 0.80, epistemic_bonus: 0.0, retry_budget: 5 },
               BehavioralState::Coasting => Self { cost_multiplier: 1.10, epistemic_bonus: 0.0, retry_budget: 2 },
               BehavioralState::Focused => Self { cost_multiplier: 0.95, epistemic_bonus: 0.0, retry_budget: 3 },
               BehavioralState::Resting => Self { cost_multiplier: 1.20, epistemic_bonus: 0.0, retry_budget: 1 },
               BehavioralState::Exploring => Self { cost_multiplier: 1.0, epistemic_bonus: 0.2, retry_budget: 4 },
           }
       }
   }
   ```

5. Add module to `lib.rs`:
   ```rust
   pub mod behavioral_scorer;
   pub use behavioral_scorer::{BehavioralScorer, RoutingModulation, archetype_center};
   ```

6. Add tests:
   - `score_all_states()` ranks Struggling highest near (-0.4, 0.5, -0.4)
   - `score_all_states()` ranks Engaged highest near (0, 0, 0)
   - Classify with hysteresis prevents oscillation
   - Dwell minimum enforced (no transition before 10 ticks)
   - `RoutingModulation::from_state(Struggling).cost_multiplier == 0.80`
   - `RoutingModulation::from_state(Coasting).cost_multiplier == 1.10`

## Verification
```bash
cargo check -p roko-daimon
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-daimon -- behavioral_scorer
cargo test -p roko-daimon -- routing_modulation
cargo test -p roko-daimon -- archetype
```

## What NOT to do
- Do NOT modify the existing `BehavioralStateTracker` or `classify_with_hysteresis()` in phase2_stubs.rs
- Do NOT modify the `BehavioralState` enum in roko-core — it already has all 6 variants
- Do NOT wire into CascadeRouter or orchestrate.rs — that is M129
- Do NOT add Bus publication — just state classification and scoring
- Do NOT add threshold learning here — that is a follow-up
