# M107 — Hypnagogia Pipeline

## Objective
Refactor the monolithic `HypnagogiaEngine` in `roko-dreams` into a four-cell pipeline: AntiCorrelation (invert query vector to find dissimilar knowledge), NoveltyScore (score fragments by distance from existing knowledge), LooseCompose (assemble fragments with relaxed coherence), and Retention Verify (filter by triple-axis threshold). This makes creative fragment generation composable and its components individually replaceable.

## Scope
- Crates: `roko-dreams`
- Files: `crates/roko-dreams/src/hypnagogia.rs` (existing engine), new file `crates/roko-dreams/src/hypnagogia_pipeline.rs`
- Phase ref: depth doc 11-memory/08-hypnagogia-and-creativity.md
- Depth doc: `tmp/unified-depth/11-memory/08-hypnagogia-and-creativity.md`

## Steps
1. Discover existing hypnagogia types and methods:
   ```bash
   grep -n 'pub fn\|pub async fn\|pub struct\|pub enum' crates/roko-dreams/src/hypnagogia.rs | head -25
   wc -l crates/roko-dreams/src/hypnagogia.rs
   grep -n 'pub fn\|pub struct' crates/roko-dreams/src/phase2/hypnagogia.rs | head -15
   ```
   **Existing types** (in `crates/roko-dreams/src/hypnagogia.rs`):
   - `ThalamicGate` -- sensory gating threshold
   - `ExecutiveLoosener` -- relaxes coherence constraints
   - `DaliInterrupt` -- random creative perturbation
   - `HomuncularObserver` -- post-hoc quality filtering
   - `HypnagogiaEngine` -- main engine composing the above, with `run(...)` method

2. Identify the four conceptual stages in the existing code:
   ```bash
   grep -n 'anti_correl\|novelty\|compose\|retain\|filter\|threshold\|loosener\|gate\|observer' crates/roko-dreams/src/hypnagogia.rs | head -20
   ```

3. Create `crates/roko-dreams/src/hypnagogia_pipeline.rs`:
   ```rust
   /// Cell 1: Anti-Correlation Query.
   /// Inverts the query HDC vector to find knowledge that is DISSIMILAR
   /// to recent patterns -- the opposite of normal retrieval.
   pub struct AntiCorrelationCell {
       /// Number of anti-correlated fragments to retrieve
       pub max_fragments: usize,
       /// Minimum dissimilarity (1.0 - similarity) to include
       pub min_dissimilarity: f64,
   }

   impl AntiCorrelationCell {
       pub fn query(
           &self,
           seed_fingerprint: &[u8],
           store: &KnowledgeStore,
       ) -> Vec<Fragment> { ... }
   }

   #[derive(Debug, Clone)]
   pub struct Fragment {
       pub source_id: String,
       pub content_summary: String,
       pub dissimilarity: f64,
       pub hdc_fingerprint: Option<Vec<u8>>,
   }
   ```

4. Implement Cell 2: Novelty Score:
   ```rust
   /// Cell 2: Score fragments by genuine novelty.
   /// Higher score = more distant from the agent's existing knowledge base.
   pub struct NoveltyScoreCell {
       /// Weight for HDC distance component
       pub hdc_weight: f64,
       /// Weight for kind diversity (different from recent kinds)
       pub kind_diversity_weight: f64,
   }

   impl NoveltyScoreCell {
       pub fn score(&self, fragments: &[Fragment], store: &KnowledgeStore) -> Vec<ScoredFragment> { ... }
   }
   ```

5. Implement Cell 3: Loose Compose:
   ```rust
   /// Cell 3: Assemble fragments with relaxed coherence constraints.
   /// Unlike normal composition which maximizes coherence, this allows
   /// high-novelty fragments even when they don't perfectly fit.
   pub struct LooseComposeCell {
       /// Temperature parameter: higher = more random composition (0.0-2.0)
       pub temperature: f64,
       /// Maximum fragments to compose together
       pub max_compose: usize,
   }

   impl LooseComposeCell {
       pub fn compose(&self, scored: &[ScoredFragment]) -> Vec<ComposedFragment> { ... }
   }
   ```

6. Implement Cell 4: Retention Verify:
   ```rust
   /// Cell 4: Filter composed fragments by triple-axis threshold.
   /// Axes: (1) novelty floor, (2) coherence floor, (3) relevance floor.
   pub struct RetentionVerifyCell {
       pub novelty_floor: f64,
       pub coherence_floor: f64,
       pub relevance_floor: f64,
   }

   impl RetentionVerifyCell {
       pub fn verify(&self, fragments: &[ComposedFragment]) -> Vec<RetainedFragment> { ... }
   }
   ```

7. Compose into a pipeline:
   ```rust
   pub struct HypnagogiaPipeline {
       pub anti_correlation: AntiCorrelationCell,
       pub novelty_score: NoveltyScoreCell,
       pub loose_compose: LooseComposeCell,
       pub retention_verify: RetentionVerifyCell,
   }

   impl HypnagogiaPipeline {
       pub fn run(&self, seed: &[u8], store: &KnowledgeStore) -> Vec<RetainedFragment> { ... }
   }
   ```

8. Register in `crates/roko-dreams/src/lib.rs`:
   ```rust
   pub mod hypnagogia_pipeline;
   ```

9. Write tests:
   - Anti-correlation returns fragments dissimilar to seed
   - Novelty scoring ranks distant fragments higher
   - Loose compose respects temperature parameter
   - Retention verify filters below threshold
   - Full pipeline produces fragments from a store with entries

## Verification
```bash
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-dreams -- hypnagogia_pipeline
```

## What NOT to do
- Do NOT modify or remove `HypnagogiaEngine` in `hypnagogia.rs`
- Do NOT implement the executive loosener or thalamic gate from the existing code -- those are wrappers around the four cells
- Do NOT use floating-point embeddings -- HDC fingerprints only
- Do NOT add LLM calls -- this is purely structural/computational
