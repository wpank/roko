# M101 — Immune Verify Pipeline and Memetic Fitness Score

## Objective
Implement the knowledge immunity system as a Verify pipeline: incoming knowledge is checked against existing AntiKnowledge entries, scored for memetic fitness (novelty, consistency, source diversity), and either admitted, quarantined, or rejected. This replaces the scattered ad-hoc confidence discounting in `knowledge_store.rs` with a structured verification pipeline.

## Scope
- Crates: `roko-neuro`
- Files: `crates/roko-neuro/src/admission.rs` (existing), `crates/roko-neuro/src/knowledge_store.rs`, new file `crates/roko-neuro/src/immune_pipeline.rs`
- Phase ref: depth doc 11-memory/04-antiknowledge-and-immunity.md
- Depth doc: `tmp/unified-depth/11-memory/04-antiknowledge-and-immunity.md`

## Steps
1. Discover existing anti-knowledge and admission types:
   ```bash
   grep -n 'pub struct\|pub fn\|pub enum' crates/roko-neuro/src/admission.rs | head -20
   grep -rn 'AntiKnowledge\|anti_knowledge\|refut' crates/roko-neuro/src/ --include='*.rs' | head -15
   grep -n 'ANTI_KNOWLEDGE' crates/roko-neuro/src/knowledge_store.rs | head -10
   ```

2. **Current admission system** (in `crates/roko-neuro/src/admission.rs`):
   ```rust
   pub struct LightAdmissionGate { ... }
   pub struct KnowledgeAdmissionStore { ... }
   pub struct KnowledgeAdmissionPolicy { ... }
   pub enum KnowledgeAdmissionOutcome { Admitted, Rejected }
   pub enum AdmissionGateOutcome { Accepted, Rejected { reason: String } }
   ```
   `KnowledgeAdmissionStore` is the main admission controller, NOT `KnowledgeAdmissionController`.

3. **KnowledgeEntry anti-knowledge fields** (in `crates/roko-neuro/src/lib.rs`):
   ```rust
   pub refuted_insight_id: Option<String>,
   pub refutation_evidence: Option<String>,
   pub hdc_vector: Option<Vec<u8>>,
   ```

4. Create `crates/roko-neuro/src/immune_pipeline.rs`:
   ```rust
   use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeStore};
   use serde::{Deserialize, Serialize};

   /// Result of the immune verification pipeline.
   #[derive(Debug, Clone)]
   pub enum ImmuneVerdict {
       /// Knowledge admitted -- passes all checks.
       Admit { fitness_score: f64 },
       /// Knowledge quarantined -- conflicts detected, needs review.
       Quarantine { conflicts: Vec<ConflictRecord>, fitness_score: f64 },
       /// Knowledge rejected -- strong AntiKnowledge match.
       Reject { reason: String, anti_match_id: String },
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ConflictRecord {
       pub existing_id: String,
       pub conflict_type: ConflictType,
       pub similarity: f64,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ConflictType {
       AntiKnowledgeMatch,
       ContradictoryHeuristic,
       DuplicateInsight,
   }
   ```

5. Implement the memetic fitness score:
   ```rust
   /// Compute memetic fitness score for a knowledge entry.
   /// Axes: novelty (HDC distance from nearest neighbor), consistency
   /// (agreement with existing heuristics), source diversity (distinct episode sources).
   pub fn memetic_fitness(
       entry: &KnowledgeEntry,
       existing_entries: &[KnowledgeEntry],
   ) -> f64 {
       let novelty = compute_novelty(entry, existing_entries);       // 0.0-1.0
       let consistency = compute_consistency(entry, existing_entries); // 0.0-1.0
       let diversity = compute_source_diversity(entry);                // 0.0-1.0
       // Weighted sum
       (novelty * 0.4 + consistency * 0.35 + diversity * 0.25).clamp(0.0, 1.0)
   }

   fn compute_novelty(entry: &KnowledgeEntry, existing: &[KnowledgeEntry]) -> f64 {
       // If entry has hdc_vector, find nearest neighbor and return 1.0 - similarity
       // If no hdc_vector, return 0.5 (neutral)
   }

   fn compute_consistency(entry: &KnowledgeEntry, existing: &[KnowledgeEntry]) -> f64 {
       // Check if entry's tags overlap with existing heuristic tags
       // Higher overlap = higher consistency
   }

   fn compute_source_diversity(entry: &KnowledgeEntry) -> f64 {
       // Count distinct source episodes
       // More sources = higher diversity score
       let count = entry.source_episodes.len();
       (count as f64 / 5.0).clamp(0.0, 1.0) // normalize: 5+ sources = 1.0
   }
   ```

6. Implement the three-stage immune pipeline:
   ```rust
   pub struct ImmunePipeline {
       /// HDC similarity threshold for anti-knowledge rejection (default: 0.85)
       pub reject_threshold: f64,
       /// HDC similarity threshold for quarantine (default: 0.65)
       pub quarantine_threshold: f64,
       /// Minimum fitness score for admission (default: 0.3)
       pub min_fitness: f64,
   }

   impl ImmunePipeline {
       pub fn verify(
           &self,
           entry: &KnowledgeEntry,
           existing: &[KnowledgeEntry],
       ) -> ImmuneVerdict {
           // Stage 1: Check against AntiKnowledge entries (by HDC similarity)
           // Stage 2: Check for contradictory heuristics
           // Stage 3: Compute memetic fitness and decide
       }
   }
   ```

7. Wire the pipeline into `KnowledgeAdmissionStore` (in `admission.rs`) as an additional verification step:
   ```bash
   grep -n 'pub fn submit_candidate\|pub fn evaluate_only' crates/roko-neuro/src/admission.rs | head -5
   ```
   Add `ImmunePipeline` as an optional verifier in the admission flow.

8. Register in `crates/roko-neuro/src/lib.rs`:
   ```rust
   pub mod immune_pipeline;
   ```

9. Write tests:
   - Entry matching AntiKnowledge above reject_threshold is rejected
   - Entry with minor conflict is quarantined
   - Entry with high novelty and no conflicts is admitted with high fitness
   - Memetic fitness components are in [0.0, 1.0] range
   - Empty existing entries always admits

## Verification
```bash
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro -- immune_pipeline
```

## What NOT to do
- Do NOT remove existing anti-knowledge thresholds from knowledge_store.rs -- the pipeline replaces the logic but constants may still be referenced
- Do NOT implement cross-domain immunity (federation) -- that is M102
- Do NOT add LLM calls for contradiction detection -- use HDC similarity only
- Do NOT modify the KnowledgeEntry struct
