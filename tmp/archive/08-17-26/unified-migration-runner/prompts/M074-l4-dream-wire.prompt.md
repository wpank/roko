# M074 — Wire L4 into Dream Cycle

## Objective
Wire L4 structural proposal generation into the dream consolidation cycle. During the dream Integration phase, the system reviews episode patterns and proposes structural improvements. Proposals include evidence from the episodes that motivated them. This connects the offline consolidation process (dreams) to the online evolution process (structural proposals).

## Scope
- Crates: `roko-dreams`
- Files: `crates/roko-dreams/src/structural.rs` (new), `crates/roko-dreams/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.3
- Spec ref: `tmp/unified/10-LEARNING-LOOPS.md` SS5.3

## Steps
1. Read the existing dream cycle code:
   ```bash
   grep -rn 'dream\|Dream\|consolidat\|integration' crates/roko-dreams/src/ --include='*.rs' | head -20
   ls crates/roko-dreams/src/
   ```

2. Read the proposal types from M072:
   ```bash
   grep -rn 'StructuralProposal\|ProposalKind' crates/roko-learn/src/structural.rs | head -10
   ```

3. Implement structural proposal generation in `crates/roko-dreams/src/structural.rs`:
   ```rust
   pub struct StructuralProposalGenerator {
       proposal_store: Arc<Mutex<ProposalStore>>,
       min_pattern_frequency: usize,  // min episodes showing same pattern
       min_confidence: f64,
   }

   impl StructuralProposalGenerator {
       pub fn new(store: Arc<Mutex<ProposalStore>>) -> Self;

       /// Analyze episodes and generate structural proposals.
       pub async fn generate_proposals(
           &self,
           episodes: &[Episode],
           current_config: &Config,
       ) -> Vec<StructuralProposal>;
   }
   ```

4. Pattern detection heuristics:
   - **Recurring Verify failures**: if the same Verify Cell fails > N times on similar inputs, propose modifying its config
   - **Consistent model escalation**: if a model is consistently escalated from T0 to T1 for a task type, propose changing the default
   - **Unused Cells**: if a Cell has not been invoked in M episodes, propose removal
   - **Bottleneck nodes**: if a node consistently has the longest execution time, propose parallelization or splitting

5. Wire into the dream cycle's Integration phase:
   ```rust
   // During dream integration:
   let proposals = generator.generate_proposals(&dream_episodes, &config).await;
   for proposal in proposals {
       proposal_store.lock().unwrap().submit(proposal)?;
       // Submission emits Bus Pulse -> Inbox notification (via M073)
   }
   ```

6. Write tests:
   - 10 episodes with same Verify failure pattern -> generates ModifyGraph proposal
   - Episodes without recurring patterns -> no proposals generated
   - Generated proposals include evidence (Signal references from episodes)
   - Proposals respect min_confidence threshold

## Verification
```bash
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-dreams -- structural
```

## What NOT to do
- Do NOT apply proposals automatically -- generation only, approval is via M073
- Do NOT generate proposals for safety-critical components -- those are protected by RecursiveSafetyMonitor
- Do NOT run proposal generation outside the dream cycle -- it requires consolidated episode data
- Do NOT add complex ML-based pattern detection -- simple frequency/threshold heuristics are sufficient
