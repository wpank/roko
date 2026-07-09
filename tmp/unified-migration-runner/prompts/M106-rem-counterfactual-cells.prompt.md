# M106 — REM Counterfactual Cells

## Objective
Implement the REM (imagination) phase of the dream cycle as composable Cell functions: counterfactual scenario generation, heuristic testing against hypothetical inputs, and knowledge gap identification. These cells form the REM sub-pipeline within the dream Loop Graph. The existing `imagination.rs` logic is decomposed into discrete testable stages.

## Scope
- Crates: `roko-dreams`
- Files: `crates/roko-dreams/src/imagination.rs` (existing), new file `crates/roko-dreams/src/rem_cells.rs`
- Phase ref: depth doc 11-memory/07-replay-and-counterfactual-cells.md
- Depth doc: `tmp/unified-depth/11-memory/07-replay-and-counterfactual-cells.md`

## Steps
1. Discover existing imagination code and types:
   ```bash
   grep -n 'pub fn\|pub async fn\|pub struct\|pub enum' crates/roko-dreams/src/imagination.rs | head -20
   wc -l crates/roko-dreams/src/imagination.rs
   grep 'roko-neuro\|roko-learn\|roko-primitives' crates/roko-dreams/Cargo.toml
   ```
   **Existing types** (in `crates/roko-dreams/src/imagination.rs`):
   - `CounterfactualQuery` -- query struct for counterfactual scenarios
   - `CausalModel` -- built from episodes via `from_episodes(&[Episode])`
   - `ImaginationOutcome` -- result of imagination pass
   - `imagine(...)` -- main imagination function
   - `synthesize_hypotheses(...)` -- hypothesis generation
   - `counterfactual_episode(base, query)` -- perturb a single episode

2. Create `crates/roko-dreams/src/rem_cells.rs`:
   ```rust
   /// Stage 1: Counterfactual Generator Cell.
   /// Given NREM patterns, generate "what-if" scenarios by perturbing
   /// key variables in the pattern.
   pub struct CounterfactualGeneratorCell {
       /// Number of counterfactuals to generate per pattern
       pub variants_per_pattern: usize,
       /// Perturbation magnitude (0.0-1.0)
       pub perturbation_strength: f64,
   }

   impl CounterfactualGeneratorCell {
       pub fn generate(
           &self,
           patterns: &[ExtractedPattern],
           store: &KnowledgeStore,
       ) -> Vec<Counterfactual> { ... }
   }

   #[derive(Debug, Clone)]
   pub struct Counterfactual {
       pub source_pattern_id: String,
       pub scenario: String,
       pub perturbed_variables: Vec<(String, String)>,  // (variable, new_value)
       pub hdc_fingerprint: Option<Vec<u8>>,
   }
   ```

3. Implement Stage 2: Heuristic Test Cell:
   ```rust
   /// Stage 2: Test existing heuristics against counterfactual scenarios.
   /// Identifies heuristics that would fail under the counterfactual.
   pub struct HeuristicTestCell;

   impl HeuristicTestCell {
       pub fn test(
           &self,
           counterfactuals: &[Counterfactual],
           heuristics: &[KnowledgeEntry],
       ) -> Vec<HeuristicTestResult> { ... }
   }

   #[derive(Debug, Clone)]
   pub struct HeuristicTestResult {
       pub counterfactual_id: String,
       pub heuristic_id: String,
       pub passed: bool,
       pub failure_mode: Option<String>,
   }
   ```

4. Implement Stage 3: Gap Identification Cell:
   ```rust
   /// Stage 3: Identify knowledge gaps from heuristic test failures.
   /// Gaps represent areas where the agent's knowledge is insufficient.
   pub struct GapIdentificationCell;

   impl GapIdentificationCell {
       pub fn identify(
           &self,
           test_results: &[HeuristicTestResult],
           counterfactuals: &[Counterfactual],
       ) -> Vec<KnowledgeGap> { ... }
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct KnowledgeGap {
       pub description: String,
       pub failed_heuristic_ids: Vec<String>,
       pub triggering_scenario: String,
       pub severity: f64,  // 0.0-1.0
   }
   ```

5. Compose into a REM pipeline:
   ```rust
   pub struct RemPipeline {
       pub generator: CounterfactualGeneratorCell,
       pub tester: HeuristicTestCell,
       pub gap_finder: GapIdentificationCell,
   }

   impl RemPipeline {
       pub fn run(
           &self,
           nrem_output: &NremResult,
           store: &KnowledgeStore,
       ) -> RemResult { ... }
   }
   ```

6. Register in `crates/roko-dreams/src/lib.rs`:
   ```rust
   pub mod rem_cells;
   ```

7. Write tests:
   - Generator produces correct number of counterfactuals per pattern
   - Heuristic tester detects failures when heuristic contradicts counterfactual
   - Gap identification finds gaps from test failures
   - Empty patterns produce empty results (no panics)

## Verification
```bash
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-dreams -- rem_cells
```

## What NOT to do
- Do NOT modify existing `imagination.rs` -- add alongside it
- Do NOT implement actual LLM-based counterfactual generation -- use structural perturbation
- Do NOT implement NREM cells (that is M105) or integration (that is M108)
- Do NOT add external ML dependencies
