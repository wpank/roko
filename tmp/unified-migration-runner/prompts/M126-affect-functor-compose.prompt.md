# M126 — Affect Functor for Compose enrichment

## Objective
Implement the `AffectFunctor` that wraps Compose calls with pre/post affect enrichment. Pre-enrichment injects PAD context, somatic markers, contrarian retrieval, behavioral state, and VCG bid adjustments. Post-enrichment stamps output with PAD provenance. Wire into orchestrate.rs dispatch path.

## Scope
- Crates: `roko-daimon`, `roko-cli`
- Files:
  - New: `crates/roko-daimon/src/affect_functor.rs`
  - `crates/roko-cli/src/orchestrate.rs` (wire functor at existing dispatch callsite)
  - `crates/roko-daimon/src/lib.rs` (module decl, re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/18-affect-as-functor.md`

## Existing types reference

Orchestrate.rs already uses DaimonState extensively:
```rust
// In orchestrate.rs:
use roko_daimon::{AffectEngine as _, AffectEvent, DaimonState, DispatchParams, SomaticSignal};

// PlanRunner already has: daimon: DaimonState
// Already queries affect: runner.daimon.query() -> returns AffectState { pad, confidence, behavioral_state, ... }
// Already creates DaimonPolicy::new(affect.confidence, affect.behavioral_state)
// Already calls roko_daimon::adjusted_thresholds(&affect.behavioral_state)
```

Key existing types:
- `DaimonState` in `crates/roko-daimon/src/lib.rs` (has `.query()`, `.appraise()`, `.somatic_landscape`, `.behavioral_tracker`)
- `AffectState` — snapshot with `.pad: PadVector`, `.confidence: f64`, etc.
- `SomaticSignal` — query result with `.valence`, `.intensity`, `.neighbor_count`, `.contrarian_count`
- `BehavioralState` in `roko-core/src/affect.rs` (6 variants)
- `BehavioralScorer` and `RoutingModulation` from M125
- `PadVector` defined in `roko-primitives/src/pad.rs`

## Steps
1. Discover the current dispatch site:
   ```bash
   grep -rn 'daimon\|DaimonState\|SomaticSignal\|DaimonPolicy' crates/roko-cli/src/orchestrate.rs | head -25
   grep -rn 'dispatch_agent_with\|enrich_rung_config\|somatic_confidence_bias' crates/roko-cli/src/orchestrate.rs | head -10
   grep -rn 'SomaticOracleContext\|somatic_confidence_bias\|retrieve_somatic' crates/roko-daimon/src/ --include='*.rs' | head -10
   ```

2. Create `crates/roko-daimon/src/affect_functor.rs`:
   ```rust
   use roko_core::BehavioralState;
   use roko_primitives::PadVector;
   use serde::{Deserialize, Serialize};
   use std::collections::HashMap;
   use super::{AffectState, SomaticSignal};

   /// Pre-computed affect context for dispatch enrichment.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AffectEnrichment {
       pub pad: PadVector,
       pub behavioral_state: BehavioralState,
       pub somatic_signal: Option<SomaticSignal>,
       pub contrarian_signal: Option<SomaticSignal>,
       pub vcg_modulation: VcgAffectModulation,
   }

   /// VCG bid modulation parameters derived from affect state.
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct VcgAffectModulation {
       pub urgency_weight: f64,   // 1.0 + arousal * 0.5
       pub affect_weight: f64,    // 1.0 + 0.3 * |pleasure - 0.5|
       pub budget_scale: f64,     // from RoutingModulation::cost_multiplier
   }

   impl VcgAffectModulation {
       pub fn from_affect(pad: &PadVector, state: BehavioralState) -> Self {
           use super::behavioral_scorer::RoutingModulation;
           let routing = RoutingModulation::from_state(state);
           Self {
               urgency_weight: 1.0 + pad.arousal * 0.5,
               affect_weight: 1.0 + 0.3 * (pad.pleasure - 0.5).abs(),
               budget_scale: routing.cost_multiplier,
           }
       }
   }

   /// Affect functor: pre-enrichment and post-stamping for agent dispatch.
   pub struct AffectFunctor;
   impl AffectFunctor {
       pub fn pre_enrich(affect: &AffectState, somatic: Option<SomaticSignal>, contrarian: Option<SomaticSignal>) -> AffectEnrichment { ... }
       pub fn post_stamp(pad: &PadVector) -> HashMap<String, serde_json::Value> { ... }
   }
   ```

3. In `orchestrate.rs`, at the dispatch site where `runner.daimon.query()` is already called:
   - Call `AffectFunctor::pre_enrich(&affect, somatic, contrarian)` to get `AffectEnrichment`
   - Include enrichment data in the system prompt context section (alongside existing `DaimonPolicy`)
   - After agent response, call `AffectFunctor::post_stamp(&affect.pad)` and attach metadata to episode

4. The VCG modulation should be passed to the system prompt builder:
   - `budget_scale` adjusts the token budget for this dispatch
   - `urgency_weight` prioritizes task context over exploratory content
   - Log the modulation parameters in the efficiency event

5. Add module to `lib.rs`:
   ```rust
   pub mod affect_functor;
   pub use affect_functor::{AffectEnrichment, AffectFunctor, VcgAffectModulation};
   ```

6. Add tests:
   - `VcgAffectModulation::from_affect` returns budget_scale 0.80 for Struggling
   - `VcgAffectModulation::from_affect` returns budget_scale 1.10 for Coasting
   - Pre-enrichment includes somatic signal when provided
   - Post-stamp produces valid PAD metadata keys
   - Urgency weight increases with arousal

## Verification
```bash
cargo check -p roko-daimon -p roko-cli
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-daimon -- affect_functor
cargo test -p roko-daimon -- vcg_modulation
```

## What NOT to do
- Do NOT refactor orchestrate.rs dispatch loop — just inject enrichment at the existing `daimon.query()` callsite
- Do NOT add Bus Pulse subscription — functor is called directly by the orchestrator
- Do NOT implement the full CrossCutFunctor trait from the spec — use a simple struct with static methods
- Do NOT touch the VCG auction in roko-compose — pass modulation as context hints
- Do NOT duplicate DaimonState — the AffectFunctor takes `&AffectState` parameters
