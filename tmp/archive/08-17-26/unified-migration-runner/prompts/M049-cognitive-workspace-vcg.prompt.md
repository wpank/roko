# M049 — CognitiveWorkspace with VCG Auction

## Objective
Implement the CognitiveWorkspace: context assembly via Vickrey-Clarke-Groves (VCG) auction. Eight or more bidders (Neuro, Task, Research, Heuristic, Episode, Pheromone, Affect, System) bid for context window slots. VCG ensures truthful bidding: each bidder's payment equals the marginal social cost of their inclusion. The budget constraint is the model's context window size. This replaces the current greedy context assembly with an economically optimal mechanism.

## Scope
- Crates: `roko-compose`
- Files: `crates/roko-compose/src/workspace.rs` (new or refactor existing), `crates/roko-compose/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.6
- Spec ref: `tmp/unified/07-AGENT-RUNTIME.md` SS7 (CognitiveWorkspace)

## Steps
1. Check what exists for context assembly and VCG:
   ```bash
   grep -rn 'vcg\|VCG\|Vcg\|CognitiveWorkspace\|context.*assembly\|AttentionBidder' crates/roko-compose/src/ --include='*.rs' | head -15
   grep -rn 'vcg_allocate\|AttentionBidder' crates/roko-cli/src/ --include='*.rs' | head -10
   ```

2. Read the existing context assembly code:
   ```bash
   grep -rn 'compose\|Compose\|build_prompt\|system_prompt' crates/roko-compose/src/ --include='*.rs' | head -15
   ```

3. Define the bidder trait and workspace in `crates/roko-compose/src/workspace.rs`:
   ```rust
   pub trait ContextBidder: Send + Sync {
       /// Name of this bidder (for logging and diagnostics).
       fn name(&self) -> &str;
       /// Produce candidate context sections with bid values.
       fn bid(&self, context: &BidContext) -> Vec<ContextBid>;
   }

   pub struct ContextBid {
       pub section: ContextSection,
       pub value: f64,        // How much value this section adds
       pub token_cost: usize, // How many tokens this section requires
       pub bidder: String,
   }

   pub struct CognitiveWorkspace {
       bidders: Vec<Box<dyn ContextBidder>>,
       context_budget: usize,  // Max tokens
   }
   ```

4. Implement VCG allocation:
   - Collect all bids from all bidders
   - Solve the knapsack problem (value maximization subject to token budget)
   - For each winning bid, compute VCG payment: the reduction in total value that others experience due to this bid's inclusion
   - Return the winning sections ordered by priority

5. Implement the 8 builtin bidders (stubs that can be filled in later):
   - `NeuroBidder` -- bids knowledge Signals relevant to current task
   - `TaskBidder` -- bids the task description and requirements
   - `ResearchBidder` -- bids research artifacts and citations
   - `HeuristicBidder` -- bids extracted heuristic Signals
   - `EpisodeBidder` -- bids relevant past episodes
   - `PheromoneBidder` -- bids pheromone coordination Signals
   - `AffectBidder` -- bids somatic marker context
   - `SystemBidder` -- bids system prompt and capability declarations

6. Add `assemble(task: &TaskContext) -> Vec<ContextSection>` that runs the auction and returns ordered sections.

7. Write tests:
   - VCG selects higher-value sections over lower-value ones
   - Budget constraint is respected (total tokens <= context_budget)
   - Removing a bidder changes others' payments correctly
   - With surplus budget, all bids win

## Verification
```bash
cargo check -p roko-compose
cargo clippy -p roko-compose --no-deps -- -D warnings
cargo test -p roko-compose -- workspace
cargo test -p roko-compose -- vcg
```

## What NOT to do
- Do NOT replace the existing compose pipeline yet -- CognitiveWorkspace is a parallel implementation
- Do NOT implement full neuro/knowledge retrieval in bidders -- use stub implementations
- Do NOT solve the knapsack optimally for large inputs -- a greedy approximation is acceptable
- Do NOT wire into orchestrate.rs yet -- that is a follow-up integration task
