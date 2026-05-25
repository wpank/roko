# M033 — Implement Expected Free Energy (EFE) routing

## Objective
Implement the Expected Free Energy (EFE) computation for model routing and wire it into CascadeRouter as a replacement for LinUCB in the UCB stage. EFE balances epistemic value (information gain), pragmatic value (expected reward), and cost, with regime conditioning: Calm mode explores, Crisis mode exploits.

## Scope
- Crates: `roko-learn`, `roko-conductor`
- Files:
  - New: `crates/roko-learn/src/efe.rs` (EFE computation)
  - `crates/roko-learn/src/cascade_router.rs` (integration)
  - `crates/roko-learn/src/model_router.rs` (LinUCB — being replaced)
  - `crates/roko-conductor/src/` (regime detection — Calm/Normal/Volatile/Crisis)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.8
- Spec ref: `tmp/unified/02-CELL.md` §5 (Route protocol), `tmp/unified/10-LEARNING-LOOPS.md` §4
- Architecture ref: `tmp/architecture/05-learning.md`

## Steps
1. Understand current LinUCB routing:
   ```bash
   grep -n 'LinUCB\|select_arm\|ucb_score' crates/roko-learn/src/model_router.rs | head -20
   ```

2. Check existing regime/conductor state:
   ```bash
   grep -rn 'Calm\|Crisis\|Volatile\|Normal\|OperatingFrequency\|regime' crates/roko-conductor/src/ --include='*.rs' | head -15
   grep -rn 'OperatingFrequency' crates/roko-core/src/ --include='*.rs' | head -10
   ```

3. Check if active_inference module already has EFE concepts:
   ```bash
   grep -rn 'active_inference\|free_energy\|epistemic\|pragmatic' crates/roko-learn/src/ --include='*.rs' | head -15
   ```

4. Create `crates/roko-learn/src/efe.rs`:
   ```rust
   //! Expected Free Energy (EFE) computation for model routing.
   //!
   //! EFE(a) = -epistemic_value(a) - pragmatic_value(a) + cost(a)
   //!
   //! The agent selects the action minimizing EFE.
   //!
   //! See: tmp/unified/10-LEARNING-LOOPS.md §4

   /// Regime conditioning weights.
   #[derive(Debug, Clone)]
   pub struct RegimeWeights {
       pub epistemic: f64,  // weight on information gain
       pub pragmatic: f64,  // weight on expected reward
       pub cost: f64,       // weight on USD cost
   }

   impl RegimeWeights {
       /// Calm: prioritize exploration (high epistemic weight).
       pub fn calm() -> Self { Self { epistemic: 2.0, pragmatic: 0.5, cost: 0.5 } }
       /// Normal: balanced.
       pub fn normal() -> Self { Self { epistemic: 1.0, pragmatic: 1.0, cost: 1.0 } }
       /// Volatile: slightly exploit-oriented.
       pub fn volatile() -> Self { Self { epistemic: 0.5, pragmatic: 1.5, cost: 1.0 } }
       /// Crisis: prioritize exploitation (high pragmatic weight).
       pub fn crisis() -> Self { Self { epistemic: 0.2, pragmatic: 2.0, cost: 0.3 } }
   }

   /// Compute EFE for a candidate model action.
   pub fn compute_efe(
       epistemic_value: f64,  // information gain (KL divergence)
       pragmatic_value: f64,  // expected reward (0-1)
       cost_usd: f64,         // USD cost
       weights: &RegimeWeights,
   ) -> f64 {
       -weights.epistemic * epistemic_value
       - weights.pragmatic * pragmatic_value
       + weights.cost * cost_usd
   }
   ```

5. Implement epistemic value estimation:
   - Use model observation count and uncertainty as a proxy for information gain
   - Low-observation models have higher epistemic value (more to learn)
   ```rust
   pub fn epistemic_value(observation_count: u64, variance: f64) -> f64 {
       // KL divergence proxy: higher uncertainty = more information gain
       let count_factor = 1.0 / (1.0 + observation_count as f64).ln();
       count_factor * variance.sqrt()
   }
   ```

6. Wire into CascadeRouter's UCB stage:
   - When stage == UCB and regime data is available, use EFE instead of LinUCB
   - Fall back to LinUCB when no regime data is available
   - Add `OperatingFrequency` to `RoutingContext` if not already present

7. Add tests:
   ```rust
   #[test]
   fn calm_regime_prefers_uncertain_cheap_model() {
       let efe_uncertain = compute_efe(0.8, 0.5, 0.01, &RegimeWeights::calm());
       let efe_known = compute_efe(0.1, 0.8, 0.10, &RegimeWeights::calm());
       assert!(efe_uncertain < efe_known); // lower EFE = preferred
   }

   #[test]
   fn crisis_regime_prefers_known_good_model() {
       let efe_uncertain = compute_efe(0.8, 0.5, 0.01, &RegimeWeights::crisis());
       let efe_known = compute_efe(0.1, 0.9, 0.10, &RegimeWeights::crisis());
       assert!(efe_known < efe_uncertain); // lower EFE = preferred
   }
   ```

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- efe
cargo test -p roko-learn -- cascade_router
```

## What NOT to do
- Do NOT remove LinUCB — keep it as a fallback when regime data is unavailable
- Do NOT require roko-conductor as a hard dependency — accept regime as an enum parameter
- Do NOT implement full Bayesian inference — the epistemic proxy (observation count + variance) is sufficient
- Do NOT change the CascadeRouter's 3-stage architecture — EFE replaces LinUCB within stage 3 only
