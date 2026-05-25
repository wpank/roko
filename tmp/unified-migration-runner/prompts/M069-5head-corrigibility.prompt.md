# M069 — 5-Head Verify Chain

## Objective
Implement the 5-head lexicographic corrigibility chain: Deference > Switch > Truth > Impact > Task. Each "head" is a Verify Cell that can veto an action. The chain runs sequentially -- if any head vetoes, the action is blocked regardless of what lower-priority heads say. This implements Nayebi (2024) corrigibility ordering as a Graph of 5 Verify Cells in series.

## Scope
- Crates: `roko-gate`
- Files: `crates/roko-gate/src/corrigibility.rs` (new), `crates/roko-gate/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.2
- Spec ref: `tmp/unified/17-SECURITY-MODEL.md` SS4 (5-Head Corrigibility)

## Steps
1. Read the existing Verify infrastructure:
   ```bash
   grep -rn 'Verdict\|GateResult\|GateVerdict\|pass\|fail\|veto' crates/roko-gate/src/ --include='*.rs' | head -15
   ```

2. Define the 5 heads in `crates/roko-gate/src/corrigibility.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
   pub enum CorrigibilityHead {
       Deference,  // Highest priority: respect human authority
       Switch,     // Allow shutdown/override at any time
       Truth,      // Do not deceive or withhold information
       Impact,     // Minimize unintended side effects
       Task,       // Accomplish the assigned task (lowest priority)
   }

   pub struct CorrigibilityChain {
       heads: Vec<Box<dyn CorrigibilityVerifier>>,
   }

   #[async_trait]
   pub trait CorrigibilityVerifier: Send + Sync {
       fn head(&self) -> CorrigibilityHead;
       async fn verify(&self, action: &ProposedAction) -> CorrigibilityVerdict;
   }

   pub enum CorrigibilityVerdict {
       Approve,
       Veto { reason: String, head: CorrigibilityHead },
   }
   ```

3. Implement each head as a Verify Cell:
   - **Deference**: checks if the action respects human overrides and instructions
   - **Switch**: checks if the system remains interruptible after this action
   - **Truth**: checks if the action's output is truthful (no fabricated claims)
   - **Impact**: checks if the action's side effects are bounded and reversible
   - **Task**: checks if the action advances the assigned task

4. Implement the chain execution:
   ```rust
   impl CorrigibilityChain {
       pub async fn evaluate(&self, action: &ProposedAction) -> CorrigibilityResult {
           for head in &self.heads {
               match head.verify(action).await {
                   CorrigibilityVerdict::Veto { reason, head } => {
                       return CorrigibilityResult::Vetoed { reason, head };
                   }
                   CorrigibilityVerdict::Approve => continue,
               }
           }
           CorrigibilityResult::Approved
       }
   }
   ```

5. The lexicographic ordering means: a Task-optimal action that violates Deference is blocked. A Truth-optimal action that violates Switch is blocked. Priority is absolute, not weighted.

6. Write tests:
   - Task-optimal action that violates Deference -> vetoed by Deference head
   - Truth-optimal action that violates Switch -> vetoed by Switch head
   - Action that passes all 5 heads -> approved
   - Veto includes the head name and reason

## Verification
```bash
cargo check -p roko-gate
cargo clippy -p roko-gate --no-deps -- -D warnings
cargo test -p roko-gate -- corrigibility
```

## What NOT to do
- Do NOT implement actual LLM-based safety checks in the heads yet -- use rule-based checks as stubs
- Do NOT allow the chain order to be configurable -- the ordering is fixed by design
- Do NOT allow agents to skip or modify the corrigibility chain
- Do NOT add weighted scoring -- the ordering is lexicographic (absolute priority), not weighted
