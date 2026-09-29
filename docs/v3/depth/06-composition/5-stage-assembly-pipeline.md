# The 5-Stage Assembly Pipeline

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/context_assembler.rs`, `crates/roko-compose/src/prompt.rs`
> v1 source: `docs/v1/03-composition/08-5-stage-assembly-pipeline.md`

---

## Overview

The 5-stage assembly pipeline transforms a task description into a
cache-aligned, budget-fitted, U-shaped prompt. The five stages -- Query, Score,
Deduplicate, Budget, Format -- execute in order for every agent spawn. The
pipeline bridges raw context sources (knowledge store, episodes, file content,
signals) and the final assembled prompt. Each stage is independently testable
and replaceable.

---

## 1. Pipeline Overview

```
Task description + metadata
         |
         v
+-------------------------+
| Stage 1: QUERY          |  HDC fingerprint + keyword search
| Candidate retrieval     |  Top-50 candidates with similarity scores
+------------+------------+
             |
             v
+-------------------------+
| Stage 2: SCORE          |  Composite score per candidate
| Rank by relevance       |  track_record * belief_change / uncertainty
+------------+------------+
             |
             v
+-------------------------+
| Stage 3: DEDUPLICATE    |  Remove near-duplicates
| Diversity enforcement   |  Hamming distance < 0.15 = duplicate
+------------+------------+
             |
             v
+-------------------------+
| Stage 4: BUDGET         |  Fit to token budget
| Priority-based dropping |  800-1,200 tokens for knowledge context
+------------+------------+
             |
             v
+-------------------------+
| Stage 5: FORMAT         |  U-shaped placement
| Cache-aligned output    |  Most relevant at start + end
+------------+------------+
             |
             v
       Agent execution
```

---

## 2. Stage 1: Query (Candidate Retrieval)

### Sources

| Source | Content | Query Method | Typical Candidates |
|--------|---------|-------------|-------------------|
| Knowledge Store | Insights, heuristics, warnings | HDC + keyword | 5-15 entries |
| Episode Store | Past task execution records | Category + crate + file overlap | 3-5 episodes |
| File Context | Source code from target files | Direct file read | 2-8 files |
| Signal Log | Recent plan signals | Plan ID + recency | 2-5 signals |

### Hybrid Search

Knowledge retrieval uses Reciprocal Rank Fusion (RRF):

```
RRF_score = SUM_{search_mode} 1 / (K + rank_in_mode)

where K = 60 (standard RRF constant)
```

A result ranked first in both keyword and HDC search scores
`1/61 + 1/61 = 0.033`. A result only in one list at rank 5 scores
`1/66 = 0.015`.

### Implementation

```rust
impl ContextAssembler {
    pub fn gather(
        &self,
        workdir: impl AsRef<Path>,
        task: &TaskInput,
        plan_id: &str,
        signals_path: impl AsRef<Path>,
    ) -> Vec<ContextChunk> {
        let task_text = task_query_text(task);
        let mut chunks = Vec::new();
        chunks.extend(self.gather_knowledge(&task_text));
        chunks.extend(self.gather_episodes(task, plan_id, &task_text));
        chunks.extend(self.gather_read_files(workdir, task));
        chunks.extend(self.gather_recent_signals(plan_id, signals_path));
        self.rank(&task_text, &mut chunks);
        self.compress(chunks)
    }
}
```

---

## 3. Stage 2: Score (Ranking)

### Current Scoring (Static)

```rust
fn score_chunk(task_text: &str, chunk: &ContextChunk, affect: Option<&PadState>) -> f64 {
    let base = source_priority(&chunk.source)
        + chunk.relevance * 0.4
        + chunk.track_record.unwrap_or(0.0) * 0.3
        + chunk.confidence.unwrap_or(0.5) * 0.2
        + chunk.recency.unwrap_or(0.5) * 0.1;

    let affect_modifier = match affect {
        Some(pad) if pad.arousal >= 0.35 => chunk.recency.unwrap_or(0.0) * 0.2,
        Some(pad) if pad.pleasure <= -0.35 => {
            if matches!(chunk.source, ContextSource::AntiPattern) { 0.3 } else { 0.0 }
        }
        _ => 0.0,
    };

    base + affect_modifier
}
```

### Source Priority Weights

| Source Type | Weight | Rationale |
|------------|--------|-----------|
| AntiPattern | 1.0 | Critical safety information |
| Verification | 0.9 | Verification commands |
| TaskBrief | 0.8 | Direct task context |
| InlineFile | 0.7 | Source code |
| KnowledgeEntry | 0.6 | Relevant knowledge |
| Episode | 0.5 | Past experience |
| SymbolSignature | 0.4 | Type signatures |
| RecentSignal | 0.3 | Plan signals |
| SiblingTasks | 0.2 | Other tasks |

### Canonical Composite Formula

```
score = (hdc_similarity * 0.4)
      + (weight_decay * 0.3)
      + (pf_utility * 0.2)
      + (freshness * 0.1)
```

The `pf_utility` component (Predictive Foraging) ensures entries that actually
improved outcomes in verified predictions rank higher than merely popular ones.

---

## 4. Stage 3: Deduplicate (Diversity Enforcement)

### Near-Duplicate Detection

```
For each candidate (in score order, highest first):
    If Hamming_distance(candidate.fingerprint, any_selected.fingerprint) < 0.15:
        Skip candidate (near-duplicate)
    Else:
        Select candidate
```

The 0.15 threshold removes functionally identical entries while preserving
genuinely distinct perspectives.

### Why Deduplication Matters

Without deduplication, a query for "proxy deployment" might return 15
near-identical entries about UUPS patterns, leaving no room for the
chain-specific gas warning that prevents the most common failure.

### Current Implementation

The `compress` method summarizes lower-ranked chunks to short heads:

```rust
fn compress(&self, mut chunks: Vec<ContextChunk>) -> Vec<ContextChunk> {
    let split_at = chunks.len() / 2;
    for (idx, chunk) in chunks.iter_mut().enumerate() {
        if idx >= split_at { continue; }
        chunk.content = summarize_content(&chunk.content);
    }
    while total_tokens > self.max_context_tokens { chunks.pop(); }
    chunks
}
```

HDC-based deduplication via `with_hdc_dedup()` on the PromptComposer is the
more precise replacement.

---

## 5. Stage 4: Budget (Token Fitting)

### Budget Targets

| Context Category | Token Budget |
|-----------------|-------------|
| Knowledge context (from Neuro) | 800-1,200 tokens |
| File context (source code) | Up to 8,000 tokens |
| Episode summaries | 500-1,000 tokens |
| Signal context | 200-500 tokens |
| **Total** | Per context tier: 4K / 12K / 24K |

### No-Truncation Policy

Entries are never truncated; an entry either fits whole or is skipped entirely.
This preserves semantic coherence within each chunk.

---

## 6. Stage 5: Format (U-Shaped Placement)

### Ordering Rule

```
Position 1-3:    Highest-scoring entries    -> Beginning (highest attention)
Position 4..N-3: Medium-scoring entries     -> Middle (lowest attention)
Position N-2..N: Second-highest entries     -> End (second-highest attention)
```

### Entry Format

```
[Type: Insight] [Age: 3d] [Weight: 0.82] [Confirmations: 7]
{Content text}
```

Metadata allows the agent to assess provenance at a glance.

---

## 7. Performance

The pipeline executes in under 5ms total:

| Stage | Latency |
|-------|---------|
| Query (HDC search) | <2ms (sub-50ns per comparison) |
| Score | <0.5ms |
| Deduplicate | <0.5ms |
| Budget | <0.5ms |
| Format | <0.5ms |
| **Total** | **<5ms** |

HDC fingerprint search uses Hamming distance (XOR + popcount), O(1) per
comparison on modern CPUs with POPCNT instructions.

---

## 8. Academic Foundations

- **RAG** [Lewis et al. 2020]. Foundational RAG paper. The pipeline is an
  advanced RAG implementation with scoring, dedup, and attention-aware format.
- **Modular RAG** [Gao et al. 2023]. Composable retrieval/generation modules.
- **Reciprocal Rank Fusion** [Cormack et al. 2009]. RRF for hybrid search.
- **Liu et al. (2023)**. U-shaped attention motivating Stage 5.
- **Sufficient Context** [Joren et al., ICLR 2025]. Insufficient context
  makes models 6x worse. Motivates no-truncation policy.
- **RAGAS** [Shahul Es et al., EACL 2024]. Three evaluation dimensions.

---

## 9. Implementation Status

| Stage | Status |
|-------|--------|
| Stage 1: Query (gather methods) | **Shipped** |
| Stage 2: Score (static) | **Shipped** |
| Stage 2: Score (active inference) | **Designed** |
| Stage 3: Dedup (compression) | **Shipped** |
| Stage 3: Dedup (HDC-based) | **Shipped** (PromptComposer) |
| Stage 4: Budget | **Shipped** |
| Stage 5: Format (Placement enum) | **Shipped** |
| Stage 5: Format (metadata annotations) | **Partial** |

---

## Cross-References

- [composer-trait.md](composer-trait.md) -- Compose trait consuming output
- [prompt-composer.md](prompt-composer.md) -- PromptComposer assembly
- [lost-in-the-middle-u-shape.md](lost-in-the-middle-u-shape.md) -- Stage 5
- [active-inference-context-selection.md](active-inference-context-selection.md) -- Stage 2
- [predictive-foraging-mvt.md](predictive-foraging-mvt.md) -- pf_utility
- `crates/roko-compose/src/context_assembler.rs` -- Stages 1-3
- `crates/roko-compose/src/prompt.rs` -- Stages 4-5
