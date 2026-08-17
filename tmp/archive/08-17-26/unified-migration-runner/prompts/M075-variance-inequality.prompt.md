# M075 — Variance Inequality Enforcement

## Objective
Implement the Variance Inequality enforcement for L4 structural proposals. L4 pauses when the generator (the system proposing changes) improves faster than the verifier (the system validating changes). If the verifier is not keeping pace, structural proposals are held until verification catches up. This prevents runaway self-modification where the system evolves faster than it can verify its own changes.

## Scope
- Crates: `roko-learn`
- Files: `crates/roko-learn/src/structural.rs` (modify), `crates/roko-learn/src/variance_inequality.rs` (new)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.3
- Spec ref: `tmp/unified/10-LEARNING-LOOPS.md` SS5.4

## Steps
1. Read the existing structural proposal code from M072:
   ```bash
   cat crates/roko-learn/src/structural.rs 2>/dev/null | head -40
   ```

2. Implement the Variance Inequality tracker in `crates/roko-learn/src/variance_inequality.rs`:
   ```rust
   pub struct VarianceInequalityTracker {
       generator_improvement_rate: f64,  // EMA of proposal quality improvement
       verifier_accuracy: f64,           // EMA of verifier's detect rate
       window_size: usize,
       history: VecDeque<InequalityObservation>,
   }

   struct InequalityObservation {
       generator_quality: f64,
       verifier_accuracy: f64,
       timestamp: DateTime<Utc>,
   }

   impl VarianceInequalityTracker {
       pub fn new(window_size: usize) -> Self;

       /// Record a generator improvement observation.
       pub fn record_generator(&mut self, quality: f64);

       /// Record a verifier accuracy observation.
       pub fn record_verifier(&mut self, accuracy: f64);

       /// Check if L4 should be paused.
       /// Returns true if generator is improving faster than verifier.
       pub fn should_pause(&self) -> bool {
           self.generator_improvement_rate > self.verifier_accuracy
       }

       /// Get the current inequality gap.
       pub fn gap(&self) -> f64 {
           self.generator_improvement_rate - self.verifier_accuracy
       }
   }
   ```

3. Integrate with ProposalStore: before submitting a proposal, check the inequality:
   ```rust
   impl ProposalStore {
       pub fn submit_if_allowed(&mut self, proposal: StructuralProposal, tracker: &VarianceInequalityTracker) -> Result<SubmitOutcome> {
           if tracker.should_pause() {
               return Ok(SubmitOutcome::Held { reason: "Variance inequality: verifier lagging generator" });
           }
           self.submit(proposal)?;
           Ok(SubmitOutcome::Submitted)
       }
   }
   ```

4. Measurement:
   - Generator quality: success rate of previously applied proposals (did they improve metrics?)
   - Verifier accuracy: detection rate of intentionally-injected test failures

5. Persist the tracker state to `.roko/learn/variance-inequality.json`.

6. Write tests:
   - When verifier accuracy > generator improvement rate -> proposals submitted normally
   - When generator improves faster than verifier -> proposals held
   - Artificially degrading verifier accuracy causes L4 to pause
   - Recovery: improving verifier accuracy resumes L4

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- variance_inequality
```

## What NOT to do
- Do NOT implement spectral analysis -- use simple EMA-based rate comparison
- Do NOT permanently block proposals -- they are held, not rejected
- Do NOT add manual override to bypass the inequality check -- it is a safety mechanism
- Do NOT tie this to specific Verify Cells -- it measures aggregate system capability
