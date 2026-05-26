# M169 — Wire Research-to-Runtime Pipeline as Loop

## Objective
Wire the research-to-runtime pipeline as a Loop in `roko-learn`. The `research_pipeline.rs` already has hypothesis/claim structures and paper processing, but there is no closed loop that promotes validated hypotheses to Heuristic Kind signals or creates AntiKnowledge signals on failure. Create `HypothesisSignal` Kind, wire the promotion/demotion lifecycle, and connect it to the `roko research` subcommand for systematic hypothesis testing.

## Scope
- Crates: `roko-learn`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/research_pipeline.rs` (extend with lifecycle)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/heuristics.rs` (promotion target)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/hypothesis_loop.rs` (new — the Loop Cell)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs` (re-export)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/research.rs` (wire hypothesis testing)
- Depth doc: `tmp/unified-depth/21-roadmap/08-research-to-runtime-bridge.md`

## Steps
1. Read existing research pipeline to understand current structures:
   ```bash
   grep -n 'pub struct\|pub fn\|pub enum\|hypothesis\|claim\|Claim' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/research_pipeline.rs | head -25
   ```

2. Read existing heuristics to understand the promotion target:
   ```bash
   grep -n 'pub struct\|pub fn\|Heuristic\|promote\|Kind' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/heuristics.rs | head -20
   ```

3. Check for existing AntiKnowledge or failure tracking:
   ```bash
   grep -rn 'AntiKnowledge\|anti_knowledge\|negative.*signal\|failed_hypothesis' /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/ --include='*.rs' | head -10
   ```

4. Define `HypothesisSignal` Kind in `hypothesis_loop.rs`:
   ```rust
   /// Signal Kind representing an active hypothesis under evaluation.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct HypothesisSignal {
       pub paper_id: String,
       pub claim_id: String,
       pub hypothesis: String,
       pub expected_effect: String,
       pub confidence: f64,           // 0.0..1.0
       pub status: HypothesisStatus,
       pub evidence: Vec<EvidenceRecord>,
       pub created_at: String,
   }

   #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
   pub enum HypothesisStatus {
       Proposed,           // Just created from research
       Testing,            // Active evaluation
       Validated,          // Passed — ready to promote
       Refuted,            // Failed — becomes AntiKnowledge
       Inconclusive,       // Not enough evidence either way
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct EvidenceRecord {
       pub run_id: String,
       pub result: EvidenceResult,
       pub measured_effect: Option<f64>,
       pub timestamp: String,
   }

   #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
   pub enum EvidenceResult {
       Supports,
       Contradicts,
       Neutral,
   }
   ```

5. Define `AntiKnowledgeSignal` for refuted hypotheses:
   ```rust
   /// Signal Kind for knowledge that has been actively disproven.
   ///
   /// AntiKnowledge prevents the system from re-testing failed hypotheses
   /// and warns agents away from approaches known to fail.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AntiKnowledgeSignal {
       pub paper_id: String,
       pub hypothesis: String,
       pub refutation_evidence: Vec<EvidenceRecord>,
       pub reason: String,
       pub created_at: String,
   }
   ```

6. Implement the hypothesis lifecycle Loop:
   ```rust
   /// Loop Cell that manages hypothesis lifecycle:
   /// Proposed → Testing → Validated/Refuted/Inconclusive
   pub struct HypothesisLoop {
       hypotheses: Vec<HypothesisSignal>,
       store_path: PathBuf,   // .roko/learn/hypotheses.json
   }

   impl HypothesisLoop {
       /// Add evidence to an active hypothesis.
       pub fn add_evidence(&mut self, claim_id: &str, evidence: EvidenceRecord) -> Result<(), HypothesisError> { ... }

       /// Evaluate a hypothesis based on accumulated evidence.
       /// Returns promotion/demotion action if threshold met.
       pub fn evaluate(&mut self, claim_id: &str) -> HypothesisAction { ... }

       /// Promote validated hypothesis to Heuristic Kind.
       pub fn promote_to_heuristic(&self, hypothesis: &HypothesisSignal) -> Heuristic { ... }

       /// Demote refuted hypothesis to AntiKnowledge Signal.
       pub fn demote_to_anti_knowledge(&self, hypothesis: &HypothesisSignal, reason: &str) -> AntiKnowledgeSignal { ... }
   }

   pub enum HypothesisAction {
       Continue,                    // Need more evidence
       Promote(Heuristic),          // Validated → Heuristic
       Refute(AntiKnowledgeSignal), // Failed → AntiKnowledge
       Defer,                       // Inconclusive, park it
   }
   ```

7. Wire evaluation thresholds:
   - Promote when: confidence >= 0.8 AND 3+ supporting evidence records
   - Refute when: 3+ contradicting evidence records OR confidence drops below 0.2
   - Defer when: 5+ evaluations with no clear signal

8. Wire into `roko research` CLI subcommand:
   ```bash
   grep -n 'research\|pub fn.*research\|SubCommand' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/research.rs 2>/dev/null | head -15
   grep -rn 'research' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs | head -10
   ```
   Add `roko research hypothesis list/add/evaluate/promote` subcommands.

9. Write unit tests:
   - HypothesisSignal creation from paper claim
   - Evidence accumulation updates confidence
   - Promotion fires when threshold met
   - Refutation creates AntiKnowledge signal
   - Persistence round-trips through JSON

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- hypothesis
cargo check -p roko-cli
```

## What NOT to do
- Do NOT modify existing research_pipeline.rs structures — extend with new module
- Do NOT require LLM calls for evaluation — use deterministic evidence-based thresholds
- Do NOT implement the actual experiment runner — this is the lifecycle/bookkeeping layer
- Do NOT add database dependencies — JSON file persistence is sufficient
- Do NOT make promotion automatic without thresholds — always require evidence accumulation
