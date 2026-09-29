# Knowledge Store: NeuroStore Details

> **v3 depth -- 09-memory** | NeuroStore internals.

The `KnowledgeStore` is the primary implementation of the `NeuroStore` trait,
providing durable JSONL-backed storage with in-memory indexing, HDC
similarity search, tier progression, worldview clustering, and
backup/restore.

---

## Storage Backend

### JSONL format

Knowledge entries are stored as append-only JSONL at
`.roko/neuro/knowledge.jsonl`:

```jsonl
{"id":"ke_001","kind":"insight","content":"Rust borrow checker errors...","confidence":0.8,...}
{"id":"ke_002","kind":"heuristic","content":"Always run clippy...","confidence":0.9,...}
```

Append-only guarantees crash safety: a partial write at the end of the file
is detectable and recoverable. The store loads all entries into memory at
init time, maintaining both the in-memory index and the on-disk log.

### In-memory representation

```rust
pub struct KnowledgeStore {
    entries: Vec<KnowledgeEntry>,
    id_index: HashMap<String, usize>,    // id -> position in entries vec
    tag_index: HashMap<String, Vec<usize>>,  // tag -> positions
    kind_index: HashMap<KnowledgeKind, Vec<usize>>,
    #[cfg(feature = "hdc")]
    memory_index: MemoryIndex,
    temporal_index: Option<TemporalIndex>,
    worldview_clusters: Vec<WorldviewCluster>,
}
```

---

## KnowledgeEntry Fields

The full entry struct carries all state needed for the memory economy:

| Field | Type | Purpose |
|-------|------|---------|
| `id` | String | Unique identifier |
| `kind` | KnowledgeKind | Semantic category (6 types) |
| `tier` | KnowledgeTier | Validation depth (4 tiers) |
| `content` | String | The actual knowledge text |
| `confidence` | f64 | 0.0--1.0 evidence score |
| `balance` | f64 | Demurrage freshness reserve |
| `frozen` | bool | Cold storage flag |
| `falsifier` | Option\<Falsifier\> | Testable predicate |
| `hdc_vector` | Option\<Vec\<u8\>\> | 10,240-bit BSC fingerprint |
| `confirmation_count` | u32 | Successful use count |
| `distinct_contexts` | Vec\<String\> | Contexts where confirmed |
| `activation_conditions` | Vec\<ActivationCondition\> | Context gates |
| `catalytic_score` | u32 | How many entries this helped create |
| `access_count` | u64 | Total access count |
| `last_accessed` | Option\<DateTime\> | Last access timestamp |

### Source channel and trust

```rust
pub enum SourceChannel {
    UserInput,           // 1.00x discount
    GateVerdict,         // 0.95x discount
    AgentOutput,         // 0.80x discount
    ExternalApi,         // 0.60x discount
    DreamConsolidation,  // 0.50x discount
}
```

Each channel carries a security label that propagates monotonically through
the entry's lifetime.

---

## Core Operations

### Ingest

1. Assign unique ID (if not set)
2. Timestamp with `created_at` (if not set)
3. Set default `half_life_days` from `KnowledgeKind`
4. Compute HDC fingerprint if not present
5. Apply source channel confidence discount
6. Run reactive AntiKnowledge check
7. Append to JSONL file
8. Add to in-memory indexes

### Query

Two query paths:

**Keyword query** (`query(topic, limit)`):
- Tag matching: entries whose tags contain query words
- Content matching: substring or keyword match
- Results sorted by composite retrieval score

**Similarity query** (`query_similar(fingerprint, limit)`):
- Brute-force Hamming distance scan against stored fingerprints
- Returns entries with similarity scores
- Threshold: 0.526 for cross-domain, 0.51 for within-domain

### Decay

```rust
pub fn decay(&mut self) -> Result<usize> {
    // For each entry:
    //   new_confidence = confidence * 2^(-elapsed_days / half_life_days)
    //   AntiKnowledge: max(0.3, new_confidence)
    //   Apply demurrage to balance
}
```

### Confirm

```rust
pub fn confirm(&mut self, entry_id: &str, episode_id: &str, positive: bool) {
    // Create KnowledgeConfirmationRecord
    // Positive: confidence *= CONFIRMATION_BOOST (1.5)
    // Negative: confidence *= 0.5
    // Check tier promotion/demotion criteria
}
```

---

## KnowledgeStats

```rust
pub struct KnowledgeStats {
    pub total_entries: usize,
    pub entries_by_kind: HashMap<KnowledgeKind, usize>,
    pub entries_by_tier: HashMap<KnowledgeTier, usize>,
    pub mean_confidence: f64,
    pub mean_balance: f64,
    pub frozen_count: usize,
    pub entries_above_threshold: usize,
}
```

Accessible via `roko knowledge stats`.

---

## Worldview Clustering

```rust
pub struct WorldviewCluster {
    pub id: String,
    pub representative_tags: Vec<String>,
    pub entry_count: usize,
}
```

Union-find by tag overlap. Two entries share a cluster when they have at
least `min_tag_overlap` tags in common. During GC, sole cluster
representatives are preserved.

---

## Demurrage Constants

```rust
pub const DEMURRAGE_RATE_PER_HOUR: f64 = 0.005;
pub const BALANCE_GC_FLOOR: f64 = 0.05;
pub const THAW_STARTER_BALANCE: f64 = 0.3;
pub const DEFAULT_GC_MIN_CONFIDENCE: f64 = 0.05;
pub const CONFIRMATION_BOOST: f64 = 1.5;
```

---

## Reinforcement Signals

```rust
pub enum ReinforcementSignal {
    Retrieved,     // 0.05 base bump
    Cited,         // 0.10 base bump
    Gated,         // 0.15 base bump
    Surprised,     // 0.08 base bump
    AgentQuoted,   // 0.12 base bump
}
```

Each bump is novelty-weighted: `bump = base * (1.0 + novelty)` where
novelty = 1.0 - max_hdc_similarity to top-K neighbors. Balance capped
at 5.0.

---

## Falsifier Lifecycle

```rust
pub struct Falsifier {
    pub predicate: String,
    pub observations: u32,
    pub violations: u32,
    pub last_checked: DateTime<Utc>,
    pub active: bool,
}
```

A single observed violation deactivates the falsifier and discredits the
entry. Surviving observations increase evidentiary standing. A Heuristic
without a falsifier cannot be frozen at Consolidated tier.

---

## HDC Migration

```rust
// crates/roko-primitives/src/hdc_migration.rs
pub struct HdcMigration {
    pub from_version: u32,
    pub to_version: u32,
}
```

When the HDC encoding scheme changes, the migration framework re-encodes
entries with the new version. The `hdc_encoder_version` field on each entry
tracks which encoder produced its fingerprint.

---

## Verification Commands

```bash
cargo test -p roko-neuro -- knowledge_store
cargo test -p roko-neuro -- tier_progression
cargo test -p roko-neuro -- temporal
cargo test -p roko-primitives -- hdc
cargo test -p roko-primitives -- codebook
roko knowledge stats
roko knowledge query "test topic"
```

---

## Cross-References

- `six-knowledge-types.md` -- the six KnowledgeKind variants
- `four-validation-tiers.md` -- the four KnowledgeTier variants
- `ebbinghaus-decay-with-tier.md` -- decay and demurrage formulas
- `knowledge-query-api.md` -- the NeuroStore trait and query flow
- `temporal-query-gc.md` -- Allen relations and GC details
