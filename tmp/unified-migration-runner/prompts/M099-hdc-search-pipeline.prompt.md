# M099 — Three-Tier HDC Search Pipeline

## Objective
Implement the three-tier search pipeline described in the depth doc: (1) HDC fingerprint pre-filter using Hamming distance for candidate selection, (2) keyword/tag overlap re-ranking, (3) optional dense-embedding re-scoring for top-k results. This replaces the current ad-hoc query logic in `KnowledgeStore::query` with a structured pipeline that can be composed as a Graph.

## Scope
- Crates: `roko-neuro`
- Files: `crates/roko-neuro/src/knowledge_store.rs` (existing query methods), new file `crates/roko-neuro/src/search_pipeline.rs`
- Phase ref: depth doc 11-memory/02-hdc-algebra-and-retrieval.md
- Depth doc: `tmp/unified-depth/11-memory/02-hdc-algebra-and-retrieval.md`

## Steps
1. Discover current query implementation:
   ```bash
   grep -n 'pub fn query\|pub fn search\|pub fn query_similar\|pub fn query_hits' crates/roko-neuro/src/knowledge_store.rs | head -15
   grep -n 'hamming\|similarity\|hdc\|fingerprint\|HdcVector' crates/roko-neuro/src/knowledge_store.rs | head -15
   grep -n 'struct\|fn ' crates/roko-neuro/src/hdc.rs | head -15  # Note: types in hdc.rs are pub(crate), not pub
   ```

2. **Current query methods** (in `crates/roko-neuro/src/knowledge_store.rs`):
   ```rust
   pub fn query(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeEntry>>;
   pub fn query_similar(&self, fingerprint: &[u8], limit: usize) -> Result<Vec<KnowledgeSimilarityHit>>;
   pub fn query_hits(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeQueryHit>>;
   pub fn query_kind(&self, topic: &str, kind: KnowledgeKind, limit: usize) -> Result<Vec<KnowledgeEntry>>;
   pub fn search(&self, query: &str, limit: usize) -> Vec<MemoryHit>;  // in MemoryIndex impl block (behind #[cfg(feature = "hdc")])
   ```
   The store already has HDC-based `query_similar` and text-based `query`. The pipeline unifies these into a staged retrieval process.

3. **KnowledgeEntry HDC field** (in `crates/roko-neuro/src/lib.rs`):
   ```rust
   pub hdc_vector: Option<Vec<u8>>,  // Optional HDC fingerprint
   ```

4. Create `crates/roko-neuro/src/search_pipeline.rs`:
   ```rust
   use crate::{KnowledgeEntry, KnowledgeStore};

   /// Three-tier search pipeline for knowledge retrieval.
   ///
   /// Tier 1: HDC Hamming pre-filter (fast, 10K entries in <50ms)
   /// Tier 2: Keyword/tag overlap re-rank (medium, on tier-1 candidates)
   /// Tier 3: Optional dense-embedding re-score (slow, on top-k) — stubbed
   pub struct SearchPipeline {
       /// Maximum candidates from tier-1 HDC pre-filter
       pub hdc_candidate_limit: usize,
       /// Minimum Hamming similarity for tier-1 inclusion (0.0-1.0)
       pub hdc_similarity_floor: f64,
       /// Number of results to pass to tier-3
       pub rerank_top_k: usize,
   }
   ```

5. Implement tier-1: HDC pre-filter
   ```rust
   impl SearchPipeline {
       /// Tier 1: Compute Hamming similarity against all entries with HDC vectors.
       /// Returns (entry_index, similarity) sorted by similarity descending.
       fn hdc_prefilter(
           &self,
           query_fingerprint: &[u8],
           entries: &[KnowledgeEntry],
       ) -> Vec<(usize, f64)> {
           // For each entry with hdc_vector, compute similarity
           // Filter by hdc_similarity_floor
           // Sort descending, take hdc_candidate_limit
       }
   }
   ```

6. Implement tier-2: keyword/tag re-ranking
   ```rust
   /// Tier 2: Re-rank candidates by keyword overlap, tag match, and recency.
   fn keyword_rerank(
       &self,
       query_tags: &[String],
       query_keywords: &[String],
       candidates: &[(usize, f64)],
       entries: &[KnowledgeEntry],
   ) -> Vec<(usize, f64)> {
       // Combine HDC similarity with tag overlap score
       // Weight: 0.6 * hdc_similarity + 0.3 * tag_overlap + 0.1 * recency
   }
   ```

7. Implement the combined pipeline:
   ```rust
   /// Run the full three-tier search pipeline.
   pub fn search(
       &self,
       query: &SearchQuery,
       entries: &[KnowledgeEntry],
   ) -> Vec<SearchResult> {
       // Tier 1: HDC pre-filter
       // Tier 2: Keyword re-rank
       // Tier 3: Stub (pass-through for now)
       // Return sorted results up to query.max_results
   }
   ```

8. Add query/result types:
   ```rust
   pub struct SearchQuery {
       pub text: String,
       pub tags: Vec<String>,
       pub hdc_fingerprint: Option<Vec<u8>>,
       pub max_results: usize,
   }

   #[derive(Debug, Clone)]
   pub struct SearchResult {
       pub entry_id: String,
       pub score: f64,
       pub tier_scores: [f64; 3],  // [HDC, keyword, dense_stub]
       pub entry: KnowledgeEntry,
   }
   ```

9. Register in `crates/roko-neuro/src/lib.rs`:
   ```rust
   pub mod search_pipeline;
   ```

10. Write tests:
    - HDC pre-filter returns entries sorted by similarity
    - Keyword re-rank boosts entries with matching tags
    - Pipeline respects `max_results` limit
    - Empty entries slice returns empty results
    - Entries without hdc_vector are skipped in tier-1 but can still appear via tier-2

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- search_pipeline
```

## What NOT to do
- Do NOT remove the existing `query` or `query_similar` methods on `KnowledgeStore` -- add the pipeline alongside them
- Do NOT implement tier-3 dense embedding re-scoring (requires external model) -- stub it with a pass-through
- Do NOT change the KnowledgeStore persistence format
- Do NOT add heavy dependencies (no ML libraries) -- tier-3 is a stub
