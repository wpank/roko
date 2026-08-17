# M129 — Wire affect-modulated routing into orchestrate.rs

## Objective
Wire the `BehavioralScorer` (M125), `RoutingModulation` (M125), `AffectFunctor` (M126), and `run_appraisal()` (M124) into the existing orchestrate.rs dispatch path. This is the integration batch that connects affect state to model tier selection and closes the feedback loop.

## Scope
- Crates: `roko-cli`, `roko-daimon`
- Files:
  - `crates/roko-cli/src/orchestrate.rs` (wire routing modulation at existing dispatch callsite)
  - `crates/roko-cli/src/runner/agent_events.rs` (emit affect events)
  - `crates/roko-daimon/src/behavioral_scorer.rs` (from M125 — use from orchestrate)
  - `crates/roko-daimon/src/affect_functor.rs` (from M126 — use from orchestrate)
- Depth doc: `tmp/unified-depth/07-agent-runtime/19-behavioral-states-and-routing.md`

## Existing types reference

Orchestrate.rs already integrates DaimonState extensively:
```rust
// PlanRunner struct already has:
//   daimon: DaimonState

// Already uses these at dispatch time:
use roko_daimon::{AffectEngine as _, AffectEvent, DaimonState, DispatchParams, SomaticSignal};

// Already queries affect: runner.daimon.query() -> AffectState { pad, confidence, behavioral_state, ... }
// Already creates: DaimonPolicy::new(affect.confidence, affect.behavioral_state)
// Already calls: roko_daimon::adjusted_thresholds(&affect.behavioral_state)
// Already has tier_thresholds in dispatch params
// Already queries somatic landscape: runner.daimon.somatic_landscape.query_nearest(...)
// Already appraises events: runner.daimon.appraise(AffectEvent::GateResult { ... })

// CascadeRouter is accessed via: self.learning.cascade_router()
// Already has routing bias: roko_learn::cascade_router::RoutingBias { ... }
```

New types from M124-M126:
- `BehavioralScorer` (M125) — archetype scoring + hysteresis-based classification
- `RoutingModulation` (M125) — `{ cost_multiplier, epistemic_bonus, retry_budget }`
- `AffectFunctor` (M126) — `pre_enrich()` and `post_stamp()`
- `AffectEnrichment` (M126) — `{ pad, behavioral_state, somatic_signal, contrarian_signal, vcg_modulation }`
- `VcgAffectModulation` (M126) — `{ urgency_weight, affect_weight, budget_scale }`
- `run_appraisal()` (M124) — 8-step pipeline function

## Steps
1. Discover the exact dispatch integration points:
   ```bash
   grep -n 'daimon.query\|DaimonPolicy\|adjusted_thresholds\|somatic\|tier_thresholds' crates/roko-cli/src/orchestrate.rs | head -15
   grep -n 'AffectEvent\|daimon.appraise' crates/roko-cli/src/orchestrate.rs | head -10
   grep -n 'cascade_router\|RoutingBias' crates/roko-cli/src/orchestrate.rs | head -10
   ```

2. At the existing dispatch site where `runner.daimon.query()` is called:
   - Use `RoutingModulation::from_state(affect.behavioral_state)` to get modulation
   - Pass `modulation.cost_multiplier` as a bias to the existing `RoutingBias`
   - Use `AffectFunctor::pre_enrich(...)` to compute `AffectEnrichment`
   - Include enrichment in the system prompt context section

3. Modify the existing `RoutingBias` construction to incorporate cost_multiplier:
   ```rust
   // Existing: roko_learn::cascade_router::RoutingBias { deprioritize, prefer_cheaper, reason }
   // Add affect influence:
   let modulation = RoutingModulation::from_state(affect.behavioral_state);
   let routing_bias = roko_learn::cascade_router::RoutingBias {
       prefer_cheaper: modulation.cost_multiplier > 1.0,  // Coasting/Resting prefer cheap
       // ... existing fields
   };
   ```

4. Wire somatic query at dispatch time (may already be partially wired):
   - Verify that somatic landscape is queried with strategy coordinates
   - Apply `somatic_confidence_bias()` to routing confidence if not already done
   - Include somatic signal in `AffectEnrichment`

5. Wire VCG modulation into system prompt builder:
   - Pass `AffectEnrichment.vcg_modulation.budget_scale` as a token budget hint
   - Log the modulation parameters in the efficiency event (already has efficiency event emission)

6. Wire appraisal feedback (may already be partially wired via `daimon.appraise()`):
   - After gate verdict, ensure `AffectEvent::GateResult` is apprised (verify existing)
   - After task completion, ensure `AffectEvent::TaskOutcome` is apprised (verify existing)
   - Optionally also call `run_appraisal()` from M124 alongside existing `appraise()`

7. Add a `behavioral_state` field to the episode log entry for observability (if not already present).

8. Add tests (in roko-daimon, not roko-cli, for unit testability):
   - `RoutingModulation::from_state(Struggling).cost_multiplier == 0.80`
   - `RoutingModulation::from_state(Coasting).cost_multiplier == 1.10`
   - `VcgAffectModulation::from_affect(...)` produces expected values

## Verification
```bash
cargo check -p roko-cli -p roko-daimon
cargo clippy -p roko-cli -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-daimon -- routing_modulation
cargo test -p roko-daimon -- affect_functor
```

## What NOT to do
- Do NOT refactor the CascadeRouter internals — just adjust the `RoutingBias` passed to it
- Do NOT add Bus infrastructure — use direct function calls at existing callsites
- Do NOT change the dispatch loop structure — inject at existing enrichment points where `daimon.query()` is already called
- Do NOT add multi-agent contagion wiring — single-agent only
- Do NOT duplicate existing appraisal calls — verify what's already wired before adding
