# M105 — NREM Replay Cells

## Objective
Implement the NREM (replay) phase of the dream cycle as composable Cell functions: episode selection via Score, temporal ordering, pattern extraction, and consolidation candidate ranking. These cells form the NREM sub-pipeline within the dream Loop Graph (M103). The existing `replay.rs` logic is refactored into discrete stages.

## Scope
- Crates: `roko-dreams`
- Files: `crates/roko-dreams/src/replay.rs` (existing), new file `crates/roko-dreams/src/nrem_cells.rs`
- Phase ref: depth doc 11-memory/07-replay-and-counterfactual-cells.md
- Depth doc: `tmp/unified-depth/11-memory/07-replay-and-counterfactual-cells.md`

## Steps
1. Discover existing replay code and available deps:
   ```bash
   grep -n 'pub fn\|pub async fn\|pub struct' crates/roko-dreams/src/replay.rs | head -20
   wc -l crates/roko-dreams/src/replay.rs
   grep -n 'pub struct Episode' crates/roko-learn/src/episode_logger.rs | head -3
   grep 'roko-primitives\|roko-neuro\|roko-learn' crates/roko-dreams/Cargo.toml
   ```
   **Available deps**: roko-dreams depends on roko-learn, roko-neuro (KnowledgeStore), and roko-primitives (HdcVector).
   **Import path for Episode**: `use roko_learn::episode_logger::Episode;` (NOT re-exported from `roko_learn` root).
   **Existing replay types**: `ReplayUtility`, `DreamReplayPolicy`, `DreamReplayBatch`, `select_replay_episodes`.

2. Create `crates/roko-dreams/src/nrem_cells.rs` with four NREM stages:
   ```rust
   /// Stage 1: Episode Selection Score Cell.
   /// Scores episodes by surprise (prediction error), recency, and emotional salience.
   /// Returns top-k episodes sorted by consolidation priority.
   pub struct EpisodeSelectionCell {
       pub max_episodes: usize,
       pub surprise_weight: f64,
       pub recency_weight: f64,
       pub salience_weight: f64,
   }

   impl EpisodeSelectionCell {
       pub fn select(&self, episodes: &[Episode]) -> Vec<ScoredEpisode> { ... }
   }

   #[derive(Debug, Clone)]
   pub struct ScoredEpisode {
       pub episode: Episode,
       pub score: f64,
       pub surprise: f64,
       pub recency: f64,
       pub salience: f64,
   }
   ```

3. Implement Stage 2: Temporal Sequence Cell:
   ```rust
   /// Stage 2: Temporal ordering and sequence detection.
   /// Groups episodes by temporal proximity, detects causal chains.
   pub struct TemporalSequenceCell {
       pub max_gap_seconds: u64,
   }

   impl TemporalSequenceCell {
       pub fn sequence(&self, episodes: &[ScoredEpisode]) -> Vec<EpisodeSequence> { ... }
   }

   #[derive(Debug, Clone)]
   pub struct EpisodeSequence {
       pub episodes: Vec<ScoredEpisode>,
       pub span_seconds: u64,
       pub causal_score: f64,
   }
   ```

4. Implement Stage 3: Pattern Extraction Cell:
   ```rust
   /// Stage 3: Extract recurring patterns from episode sequences.
   /// Uses HDC fingerprint bundling to detect structural similarity across sequences.
   pub struct PatternExtractionCell {
       pub min_occurrences: usize,
       pub similarity_threshold: f64,
   }

   impl PatternExtractionCell {
       pub fn extract(&self, sequences: &[EpisodeSequence]) -> Vec<ExtractedPattern> { ... }
   }

   #[derive(Debug, Clone)]
   pub struct ExtractedPattern {
       pub pattern_fingerprint: Vec<u8>,  // HDC fingerprint of the pattern
       pub supporting_episodes: Vec<String>,  // episode IDs
       pub occurrences: usize,
       pub confidence: f64,
   }
   ```

5. Implement Stage 4: Consolidation Ranker Cell:
   ```rust
   /// Stage 4: Rank patterns by consolidation priority.
   /// High-priority patterns are novel (far from existing knowledge)
   /// and well-supported (many occurrences).
   pub struct ConsolidationRankerCell;

   impl ConsolidationRankerCell {
       pub fn rank(&self, patterns: &[ExtractedPattern], store: &KnowledgeStore) -> Vec<RankedPattern> { ... }
   }
   ```

6. Compose into a NREM pipeline:
   ```rust
   pub struct NremPipeline {
       pub selection: EpisodeSelectionCell,
       pub sequencing: TemporalSequenceCell,
       pub extraction: PatternExtractionCell,
       pub ranking: ConsolidationRankerCell,
   }

   impl NremPipeline {
       pub fn run(&self, episodes: &[Episode], store: &KnowledgeStore) -> NremResult { ... }
   }
   ```

7. Register in `crates/roko-dreams/src/lib.rs`:
   ```rust
   pub mod nrem_cells;
   ```

8. Write tests:
   - Episode selection respects max_episodes limit
   - Episodes scored by surprise/recency/salience
   - Temporal sequencing groups nearby episodes
   - Pattern extraction finds recurring structures
   - Consolidation ranker prioritizes novel, well-supported patterns

## Verification
```bash
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-dreams -- nrem_cells
```

## What NOT to do
- Do NOT modify existing `replay.rs` -- add alongside it
- Do NOT implement REM cells -- that is M106
- roko-dreams already depends on roko-primitives, so HdcVector can be used directly via `roko_primitives::HdcVector` -- but `Option<Vec<u8>>` is also acceptable for compatibility with KnowledgeEntry's `hdc_vector` field
- Do NOT implement actual LLM calls for pattern extraction -- use structural/HDC similarity
