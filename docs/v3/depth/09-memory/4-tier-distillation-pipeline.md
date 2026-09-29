# Four-Tier Distillation Pipeline

> **v3 depth -- 09-memory** | Source: v1/06-neuro/12

Episodes are distilled into Insights, Insights are promoted into Heuristics,
Heuristics are compiled into PLAYBOOK.md, and violations spawn refined
children. Durable knowledge keeps its receipts; explicit falsifier records,
worldview history, and demurrage-governed freshness are wired.

---

## Four Stages

| Stage | Input | Output | Criteria |
|-------|-------|--------|----------|
| **D1** | Recurring episode patterns | Insight at Transient tier | 3+ supporting episodes |
| **D2** | Confirmed insights | Heuristic with when/then + falsifier | 5+ independent confirmations |
| **D3** | Top heuristics | `PLAYBOOK.md` for human review | Top 12 by calibration score |
| **D4** | Heuristic violations | Refined children with narrower conditions | Falsifier triggered above threshold |

---

## Stage D1: Episodes -> Insights

When an episode finishes, `spawn_episode_distillation()` triggers async
extraction:

```rust
pub fn spawn_episode_distillation(
    episode: Episode,
    distiller: Arc<dyn DistillationBackend>,
    store: Arc<Mutex<KnowledgeStore>>,
) -> JoinHandle<Result<()>>;
```

The `DistillationBackend` uses an LLM to extract structured knowledge:

```rust
pub trait DistillationBackend: Send + Sync {
    async fn distill(&self, episode: &Episode) -> Result<Vec<KnowledgeEntry>>;
}
```

`InsightRecord` requires support from at least 3 episodes:

```rust
pub struct InsightRecord {
    pub pattern: String,
    pub support: usize,
    pub confidence: f64,
    pub source_episodes: Vec<String>,
}
```

Distilled entries enter at Transient tier with initial confidence 0.3--0.6,
receive HDC fingerprints at ingestion, and carry receipts back to originating
episodes.

---

## Stage D2: Insights -> Heuristics

### Pattern mining

1. Collect all Insights with confidence >= 0.5
2. Cluster by HDC fingerprint similarity
3. Filter clusters with 5+ members and mean confidence >= 0.7
4. Extract the common pattern from qualifying clusters
5. Emit a durable Heuristic with calibration metadata and receipts

```rust
pub struct HeuristicRule {
    pub rule: String,
    pub support: usize,
    pub confidence: f64,
    pub source_insights: Vec<String>,
}
```

### Promotion criteria

| Criterion | Threshold | Rationale |
|-----------|----------|-----------|
| Minimum support | 5 Insights | Pattern is robust |
| Minimum confidence | 0.7 | High reliability |
| Cross-validation | 2+ distinct contexts | Prevents overfitting |
| No contradictions | No active falsifier/AntiKnowledge | No contested knowledge promoted |

### Heuristic calibration

Once promoted, each later episode can:
- **Confirm** -- heuristic predicted correctly
- **Violate** -- heuristic predicted wrong, loses weight
- **Refine** -- directionally right but too broad, spawn narrower child
- **Generalize** -- worked in broader context, lift claim upward
- **Refute** -- repeated contradictions create falsifier record

---

## Stage D3: Heuristics -> PLAYBOOK.md

```rust
pub struct PlaybookCompilation {
    pub title: String,
    pub rules: Vec<HeuristicRule>,
    pub markdown: String,
}
```

Compiled to `.roko/neuro/PLAYBOOK.md`. The playbook is the user-facing
surface; the heuristic record is the source of truth. Rules that are
retrieved often keep their balance higher. Rules never used eventually cool.

---

## Stage D4: Violations -> Refined Children

When a falsifier accumulates violations above threshold, the system spawns
refined children with narrower activation conditions. Example: "When
refactoring code, run clippy" that fails for JavaScript files spawns "When
refactoring Rust code, run clippy."

---

## Worldview Clustering

Target-state: a worldview is a cluster of heuristics that repeatedly
co-occur in successful episodes. The cluster is observed from the co-citation
graph and episode overlap.

Worldviews matter because they let Neuro reason above a single heuristic.
Cold-tier preservation prevents good but quiet worldviews from being
forgotten.

---

## TierProgression Orchestrator

```rust
pub struct TierProgression {
    knowledge_store: Arc<Mutex<KnowledgeStore>>,
    pattern_miner: Arc<PatternMiner>,
}

impl TierProgression {
    pub fn analyze(&self, episodes: &[Episode]) -> Result<Vec<InsightRecord>>;
    pub fn extract_insights(&self, records: Vec<InsightRecord>) -> Result<Vec<KnowledgeEntry>>;
    pub fn promote_heuristics(&self) -> Result<Vec<HeuristicRule>>;
    pub fn compile_playbook(&self) -> Result<PlaybookCompilation>;
    pub fn replay_heuristics(&mut self) -> Result<Vec<HeuristicAdjustment>>;
}
```

---

## Integration with Dreams

1. NREM replay re-processes recent episodes (Mattar-Daw utility)
2. Consolidation runs D1 and D2 on replayed episodes
3. Pruning charges demurrage, freezes cold knowledge
4. Playbook update runs D3 to recompile

---

## Pipeline Flow

```
Episodes (raw agent turns)
    |
    v
D1: Episode Distillation
    - LLM extracts observations, records receipts
    - min_support = 3 episodes
    - Output: KnowledgeEntry (Insight), Transient tier
    |
    v
D2: Heuristic Calibration
    - HDC clusters repeated Insight sets
    - Promote to durable Heuristic records
    - Contradictions become falsifier records
    |
    v
D3: Heuristic -> PLAYBOOK.md
    - Compile validated Heuristics
    - Output: .roko/neuro/PLAYBOOK.md
    |
    v
D4: Violation -> Refined Children
    - Falsifier violations spawn narrower children
    - Original preserved with updated calibration
```

---

## Academic Foundations

- McClelland, J. L. et al. (1995). "Complementary learning systems."
  *Psychological Review*, 102(3).
- Mattar, M. G. & Daw, N. D. (2018). "Prioritized memory access explains
  planning and hippocampal replay." *Nature Neuroscience*, 21.
- Festinger, L. (1957). *A Theory of Cognitive Dissonance*. Stanford.

---

## Cross-References

- `hdc-vsa-foundations.md` -- HDC algebra behind fingerprint similarity
- `hdc-knowledge-encoding.md` -- encoding pipeline
- `knowledge-query-api.md` -- native similarity query surface
