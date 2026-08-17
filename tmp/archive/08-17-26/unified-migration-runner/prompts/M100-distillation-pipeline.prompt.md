# M100 — Distillation as Pipeline Graph

## Objective
Refactor the three sequential distillation stages (D1 `extract_insights`, D2 `promote_heuristics`, D3 `compile_playbook`) in `roko-neuro` from a monolithic `TierProgression::analyze` method into composable pipeline stages. Each stage becomes a standalone struct with clear input/output types, enabling the distillation pipeline to be composed, extended, and run incrementally rather than as a single batch job.

## Scope
- Crates: `roko-neuro`, `roko-learn`
- Files: `crates/roko-neuro/src/tier_progression.rs`, `crates/roko-neuro/src/distiller.rs`, new file `crates/roko-neuro/src/distill_pipeline.rs`
- Phase ref: depth doc 11-memory/03-knowledge-lifecycle-loop.md
- Depth doc: `tmp/unified-depth/11-memory/03-knowledge-lifecycle-loop.md`

## Steps
1. Discover the current distillation code and types:
   ```bash
   grep -n 'pub fn extract_insights\|pub fn promote_heuristics\|pub fn compile_playbook\|pub fn analyze' crates/roko-neuro/src/tier_progression.rs | head -10
   grep -n 'pub struct InsightRecord\|pub struct HeuristicRule\|pub struct TierProgression\|pub struct TierProgressionReport' crates/roko-neuro/src/tier_progression.rs | head -10
   grep -n 'pub struct\|pub fn\|pub async fn' crates/roko-neuro/src/distiller.rs | head -10
   grep -n 'pub struct Episode' crates/roko-learn/src/episode_logger.rs | head -5
   ```

2. **Current distillation flow** (in `crates/roko-neuro/src/tier_progression.rs`):
   ```rust
   pub struct TierProgression {
       min_support: usize,
       min_heuristic_support: usize,
       min_confidence: f64,
       playbook_limit: usize,
   }

   impl TierProgression {
       /// D1 -> D2 -> D3 all in one call
       pub fn analyze(&self, episodes: &[Episode]) -> TierProgressionReport;

       /// D1: Extract insights from episodes
       pub fn extract_insights(&self, episodes: &[Episode], patterns: &[roko_learn::pattern_discovery::Pattern]) -> Vec<InsightRecord>;

       /// D2: Promote insights to heuristics
       pub fn promote_heuristics(&self, insights: &[InsightRecord]) -> Vec<HeuristicRule>;

       /// D3: Compile playbook from heuristics
       pub fn compile_playbook(&self, heuristics: &[HeuristicRule], insight_count: usize) -> PlaybookCompilation;
   }
   ```
   Also: `Distiller` in `distiller.rs` does async LLM-based distillation via `Distiller::distill(&self, episodes: &[Episode]) -> Result<Vec<KnowledgeEntry>>`.

3. Create `crates/roko-neuro/src/distill_pipeline.rs` with three pipeline stage structs:
   ```rust
   use crate::tier_progression::{InsightRecord, HeuristicRule, PlaybookCompilation};
   use roko_learn::episode_logger::Episode;  // Episode is NOT re-exported from roko_learn root
   use roko_learn::pattern_discovery::Pattern;

   /// D1: Extract insights from episodes.
   /// Mines recurring patterns, emits InsightRecords.
   pub struct ExtractInsightsStage {
       /// Minimum episode support count to trigger pattern extraction
       pub min_support: usize,
       /// Minimum confidence for pattern inclusion
       pub min_confidence: f64,
   }

   impl ExtractInsightsStage {
       pub fn run(&self, episodes: &[Episode], patterns: &[Pattern]) -> Vec<InsightRecord> { ... }
   }

   /// D2: Promote insights to heuristics.
   /// Clusters insights, emits HeuristicRules with when/then clauses.
   pub struct PromoteHeuristicsStage {
       /// Minimum supporting episodes for promotion
       pub min_support: usize,
   }

   impl PromoteHeuristicsStage {
       pub fn run(&self, insights: &[InsightRecord]) -> Vec<HeuristicRule> { ... }
   }

   /// D3: Compile playbook from top heuristics.
   /// Ranks by confidence, renders PlaybookCompilation.
   pub struct CompilePlaybookStage {
       /// Maximum heuristics to include in playbook
       pub max_entries: usize,
   }

   impl CompilePlaybookStage {
       pub fn run(&self, heuristics: &[HeuristicRule], insight_count: usize) -> PlaybookCompilation { ... }
   }
   ```

4. Create a `DistillationPipeline` that composes the three stages:
   ```rust
   pub struct DistillationPipeline {
       pub d1: ExtractInsightsStage,
       pub d2: PromoteHeuristicsStage,
       pub d3: CompilePlaybookStage,
   }

   pub struct DistillationResult {
       pub insights: Vec<InsightRecord>,
       pub heuristics: Vec<HeuristicRule>,
       pub playbook: PlaybookCompilation,
   }

   impl DistillationPipeline {
       /// Run all three stages in sequence.
       pub fn run_full(&self, episodes: &[Episode]) -> DistillationResult { ... }

       /// Run only on new episodes (incremental mode).
       pub fn run_incremental(
           &self,
           new_episodes: &[Episode],
           existing_insights: &[InsightRecord],
       ) -> DistillationResult { ... }
   }
   ```

5. Wire the pipeline into the existing `TierProgression` as a delegate (do NOT break existing callers):
   ```rust
   // In TierProgression::analyze, optionally delegate to the pipeline:
   // The pipeline stages reuse the same logic from extract_insights/promote_heuristics/compile_playbook
   ```

6. Register in `crates/roko-neuro/src/lib.rs`:
   ```rust
   pub mod distill_pipeline;
   ```

7. Write tests:
   - D1 extracts insights from episodes with sufficient support
   - D2 promotes insights with sufficient evidence
   - D3 produces a playbook from promoted heuristics
   - DistillationPipeline::run_full produces all three outputs
   - Incremental run reuses existing insights

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- distill_pipeline
```

## What NOT to do
- Do NOT delete the existing distillation functions in `tier_progression.rs` or `distiller.rs` -- delegate to the new pipeline
- Do NOT change the InsightRecord, HeuristicRule, or PlaybookCompilation struct fields
- Do NOT implement predict-publish-correct feedback -- that is M112
- Do NOT add async to the pipeline stages -- keep them synchronous (TierProgression methods are sync)
