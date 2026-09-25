# Knowledge Query API

> **v3 depth -- 09-memory** | Source: v1/06-neuro/10

The NeuroStore trait defines storage and retrieval for Neuro's persistent
knowledge -- init, query, query_similar, ingest, decay, and gc -- with the
KnowledgeStore JSONL implementation as the primary backend.

---

## The NeuroStore Trait

```rust
pub trait NeuroStore: Sized {
    fn init(path: &Path) -> Result<Self>;
    fn query(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeEntry>>;
    fn query_similar(
        &self, fingerprint: &HdcVector, limit: usize,
    ) -> Result<Vec<(KnowledgeEntry, f32)>>;
    fn ingest(&mut self, entries: Vec<KnowledgeEntry>) -> Result<()>;
    fn decay(&mut self) -> Result<usize>;
    fn gc(&mut self, min_confidence: f64) -> Result<usize>;
}
```

### Core query flow

```
1. Compute HDC fingerprint for query text
2. Scan local NeuroStore for similarity matches
3. Filter by activation conditions (model, task type, domain, language)
4. Score: HDC similarity (40%) + keyword match (30%) + utility (20%) + freshness (10%)
5. Cross-domain bonus (+15% for matches from other domains above threshold)
6. Results enter the CognitiveWorkspace VCG auction as knowledge bidders
7. Winning entries injected into system prompt
```

### Retrieval scoring

```
retrieval_score = confidence * decay_weight * (1 + hdc_similarity_bonus)
```

Within-domain queries use threshold 0.51; cross-domain queries use 0.526.

---

## KnowledgeStore Implementation

### Storage format

Append-only JSONL at `.roko/neuro/knowledge.jsonl`:

```jsonl
{"id":"ke_001","kind":"insight","content":"Rust borrow checker...","confidence":0.8,"tags":["rust"],...}
```

### Key constants

```rust
pub const DEFAULT_GC_MIN_CONFIDENCE: f64 = 0.05;
pub const CONFIRMATION_BOOST: f64 = 1.5;
```

### HDC MemoryIndex

Feature-gated HDC-based similarity search alongside keyword matching:

```rust
#[cfg(feature = "hdc")]
pub struct MemoryIndex {
    vectors: Vec<HdcVector>,
    entry_ids: Vec<String>,
}
```

---

## ContextAssembler

Assembles context from knowledge and episode memory under a token budget:

1. **Query** knowledge store for relevant entries
2. **Query** episode store for recent relevant episodes
3. **Rank** by composite retrieval score including fingerprint similarity
4. **Budget** -- auction-style allocator weighs retrieval value against
   token cost, dampens repeated source families
5. **Format** -- render as structured text for the LLM prompt

The canonical implementation lives in `roko-neuro/src/context.rs`, with
PAD-based affect biasing wired into ranking via `PadState` and a mandatory
contrarian slice for affect-heavy retrieval.

---

## CLI Interface

```bash
roko knowledge query "borrow checker patterns"
roko knowledge stats
roko knowledge gc
roko knowledge export --output knowledge.json
roko knowledge import --input knowledge.json
roko knowledge backfill-hdc
roko knowledge dream run
roko knowledge custody list
roko knowledge archive
```

---

## Cross-References

- `ebbinghaus-decay-with-tier.md` -- how decay affects retrieval scoring
- `hdc-knowledge-encoding.md` -- HDC-based similarity search
- `4-tier-distillation-pipeline.md` -- distillation to ingestion pipeline
- `somatic-integration.md` -- PAD-biased retrieval
