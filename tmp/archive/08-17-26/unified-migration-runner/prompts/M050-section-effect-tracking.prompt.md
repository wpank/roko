# M050 — Beta Posterior Section Effect Tracking

## Objective
Implement section effect tracking using Beta-distribution posteriors. After each Verify verdict, update the posterior for each context section that was present: `Beta(alpha + successes, beta + failures)`. Use the posterior mean as the section's bidding weight in future VCG auctions. This creates a feedback loop where context sections that correlate with success get higher bids and those that correlate with failure get lower bids.

## Scope
- Crates: `roko-compose`
- Files: `crates/roko-compose/src/section_effects.rs` (new), `crates/roko-compose/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.6
- Spec ref: `tmp/unified/07-AGENT-RUNTIME.md` SS7.2 (Section Effects)

## Steps
1. Check for existing section tracking or Beta distribution code:
   ```bash
   grep -rn 'section_effect\|SectionEffect\|Beta\|posterior\|alpha.*beta' crates/roko-compose/src/ --include='*.rs' | head -10
   grep -rn 'section_effect\|Beta.*posterior' crates/roko-learn/src/ --include='*.rs' | head -10
   ```

2. Define the section effect tracker in `crates/roko-compose/src/section_effects.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct SectionEffectTracker {
       effects: HashMap<String, BetaPosterior>,
       persistence_path: Option<PathBuf>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct BetaPosterior {
       pub alpha: f64,  // successes + prior
       pub beta: f64,   // failures + prior
   }

   impl BetaPosterior {
       pub fn new() -> Self { Self { alpha: 1.0, beta: 1.0 } }  // Uniform prior
       pub fn mean(&self) -> f64 { self.alpha / (self.alpha + self.beta) }
       pub fn update_success(&mut self) { self.alpha += 1.0; }
       pub fn update_failure(&mut self) { self.beta += 1.0; }
       pub fn confidence(&self) -> f64 {
           // Higher with more observations
           1.0 - 1.0 / (1.0 + self.alpha + self.beta)
       }
   }
   ```

3. Implement the tracker:
   ```rust
   impl SectionEffectTracker {
       pub fn new(persistence_path: Option<PathBuf>) -> Self;
       pub fn update(&mut self, sections: &[String], verdict: VerifyVerdict);
       pub fn weight(&self, section_type: &str) -> f64;  // posterior mean
       pub fn save(&self) -> Result<()>;
       pub fn load(path: &Path) -> Result<Self>;
   }
   ```

4. On Verify pass: `update_success()` for each section type in context.
   On Verify fail: `update_failure()` for each section type in context.

5. Integrate with CognitiveWorkspace (from M049): each bidder's bid value is multiplied by the section effect weight for its type.

6. Persist effects to `.roko/learn/section-effects.json` for durability across sessions.

7. Write tests:
   - Section consistently present during passes develops high posterior mean (> 0.7 after 10 passes)
   - Section consistently present during failures develops low posterior mean (< 0.3 after 10 failures)
   - New/unseen section types start with uniform prior (mean = 0.5)
   - Persistence round-trip: save, load, verify posteriors match

## Verification
```bash
cargo check -p roko-compose
cargo clippy -p roko-compose --no-deps -- -D warnings
cargo test -p roko-compose -- section_effect
```

## What NOT to do
- Do NOT use a complex statistical library -- Beta distribution is simple enough to implement inline
- Do NOT track individual sections by content -- track by section TYPE (Neuro, Task, Research, etc.)
- Do NOT implement causal inference -- correlation-based tracking is sufficient for bidding weights
- Do NOT reset posteriors on session boundaries -- they are persistent learning state
