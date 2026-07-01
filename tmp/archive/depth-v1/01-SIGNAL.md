# 01 — Signal

> The universal data unit. Everything that flows through Roko is a Signal.

**Subsumes**: Engram, Pulse, Artifact, Knowledge Entry, Pheromone, Evidence, Feed event, ModuleOutput, Finding.

---

## 1. Definition

A **Signal** is a content-addressed, typed, scored, decaying, lineage-tracked data unit with an HDC fingerprint. Signals are the one noun of the Roko system — the six protocols (Store, Score, Verify, Route, Compose, React) and three new protocols (Observe, Connect, Trigger) are the verbs that operate on them.

Two variants exist based on lifetime:

| Variant | Lifetime | Where | Use |
|---|---|---|---|
| **Ephemeral** | Transient, lives on Bus | In-memory Bus topics | Events, heartbeats, pheromones, streaming output |
| **Persisted** | Durable, lives in Store | `.roko/signals.jsonl`, knowledge store, artifact store | Artifacts, knowledge entries, episodes, findings |

Both variants share the same type. The difference is whether a Store-protocol Block has persisted it.

---

## 2. Core Structure

```rust
pub struct Signal {
    // ── Identity ──────────────────────────────────────────────────
    pub id: SignalId,                    // ULID, globally unique
    pub content_hash: ContentHash,       // SHA-256 of payload bytes
    pub kind: Kind,                      // discriminant (see §3)

    // ── Content ───────────────────────────────────────────────────
    pub payload: Value,                  // serde_json::Value, schema-validated against kind
    pub schema: TypeSchema,              // structural type of the payload

    // ── Scoring ───────────────────────────────────────────────────
    pub score: Score,                    // multi-dimensional quality signal
    pub confidence: f64,                 // 0.0..=1.0, decays over time

    // ── Temporal ──────────────────────────────────────────────────
    pub created_at: DateTime<Utc>,
    pub decay: DecayConfig,              // half-life, tier multiplier, frozen flag
    pub tier: Tier,                      // Transient | Working | Consolidated | Persistent

    // ── Lineage ───────────────────────────────────────────────────
    pub source: Vec<SignalRef>,          // upstream Signals (provenance graph)
    pub provenance: Provenance,          // generation metadata, citations, source files

    // ── Embedding ─────────────────────────────────────────────────
    pub hdc_fingerprint: HdcVector,      // 10,240-bit binary vector (1,280 bytes)

    // ── Authorship ────────────────────────────────────────────────
    pub author: Author,                  // agent ID, wallet address, or system
    pub tags: Vec<String>,               // topic tags for filtering and discovery
}
```

### Mapping to existing code

The `Signal` struct maps directly to `roko-core::Engram`:

| Signal field | Engram field | Notes |
|---|---|---|
| `id` | `id` | Same ULID type |
| `content_hash` | `content_hash` | Same ContentHash type |
| `kind` | `kind` | Same Kind enum (extended) |
| `payload` | `body` | Renamed for clarity |
| `score` | `score` | Same multi-dimensional Score |
| `confidence` | `confidence` | Same f64 |
| `created_at` | `timestamp` | Renamed |
| `decay` | `decay` | Same DecayConfig |
| `tier` | `tier` | Same KnowledgeTier (renamed Tier) |
| `source` | `lineage` | Renamed for clarity |
| `provenance` | `provenance` | Same Provenance struct |
| `hdc_fingerprint` | `hdc_fingerprint` | Same HdcVector |
| `author` | `author` | Same Author enum |
| `tags` | `tags` | Same Vec<String> |

**No new type is needed.** The unified spec names the concept "Signal"; the Rust implementation remains `Engram` for backward compatibility. New code should use `type Signal = Engram;` to bridge.

---

## 3. Kind System

Every Signal has a `Kind` that determines its schema, decay behavior, and how Blocks interact with it.

```rust
#[non_exhaustive]
pub enum Kind {
    // ── Core data ──────────────────────────────────────
    Text,                    // plain text content
    Markdown,                // markdown document
    Json,                    // structured JSON data
    Toml,                    // TOML configuration
    Code { language: String }, // source code with language tag
    Diff,                    // unified diff
    Binary { mime: String }, // opaque binary with MIME type
    Image { format: String }, // image data

    // ── Artifacts ──────────────────────────────────────
    File { path: PathBuf },  // file system artifact
    Artifact { kind: ArtifactKind }, // versioned, lineage-tracked output

    // ── Knowledge ──────────────────────────────────────
    Insight,                 // observed pattern with supporting evidence
    Heuristic,               // behavioral rule (when/then)
    Warning,                 // transient danger signal
    CausalLink,              // cause → effect relationship
    StrategyFragment,        // reusable strategy component
    AntiKnowledge,           // known-bad information (repels similar entries)

    // ── Coordination ───────────────────────────────────
    Pheromone { ptype: PheromoneType }, // stigmergic signal with location + intensity
    Heartbeat,               // agent health tick
    Presence { event: PresenceEvent }, // join/leave/supersede

    // ── Execution ──────────────────────────────────────
    Evidence { kind: EvidenceKind }, // verification evidence
    Finding { severity: Severity },  // verification finding
    Verdict,                 // gate pass/fail result
    Episode,                 // recorded agent turn
    CostReport,              // cost/latency/quality metrics

    // ── Observation ────────────────────────────────────
    Observation,             // Lens output (see doc-09)
    Alert { level: AlertLevel }, // threshold breach notification
    Trend,                   // statistical trend (slope, EMA)
    Anomaly,                 // statistical outlier

    // ── User-defined ───────────────────────────────────
    Custom { name: String }, // extensible for user Blocks
}
```

The `#[non_exhaustive]` attribute ensures new kinds can be added without breaking downstream.

### Kind → Decay mapping

Each Kind has a default decay profile. Blocks can override these defaults.

| Kind group | Default half-life | Default tier | Rationale |
|---|---|---|---|
| Core data (Text, Json, Code) | ∞ (no decay) | Persistent | Data artifacts don't decay |
| Artifacts | ∞ (no decay) | Persistent | Versioned outputs are permanent |
| Insight | 30 days | Transient | Observations need confirmation |
| Heuristic | 90 days | Working | Behavioral rules proven by use |
| Warning | 1 hour | Transient | Danger signals are ephemeral |
| CausalLink | 60 days | Working | Causal models need varied testing |
| StrategyFragment | 14 days | Transient | Strategies go stale in evolving codebases |
| AntiKnowledge | 30 days | Working | What-not-to-do stays relevant |
| Pheromone | 1 hour | Transient | Stigmergic signals are deliberately ephemeral |
| Heartbeat | 5 seconds | (ephemeral only) | Latest heartbeat is all that matters |
| Evidence | 7 days | Working | Evidence ages but persists for audit |
| Episode | 90 days | Working | Episodes feed learning loops |
| Observation | 1 hour | Transient | Lens output is consumed, not archived |
| Alert | 24 hours | Transient | Alerts are actionable, not historical |

---

## 4. Content Addressing

Signals are content-addressed via SHA-256 of their payload bytes:

```rust
impl Signal {
    pub fn compute_hash(payload: &Value) -> ContentHash {
        let canonical = serde_json::to_vec(payload).expect("payload serializable");
        ContentHash(sha2::Sha256::digest(&canonical).into())
    }
}
```

Content addressing enables:
- **Deduplication**: Two Blocks producing identical output reference the same Signal.
- **Integrity**: A Signal's hash verifies its content hasn't been tampered with.
- **Lineage verification**: Walking `source[]` and checking hashes proves an unbroken provenance chain.
- **On-chain commitment**: The content hash is the on-chain record; full content lives off-chain.

---

## 5. Scoring

Every Signal carries a multi-dimensional `Score`:

```rust
pub struct Score {
    pub relevance:  f64,     // 0.0..=1.0 — how relevant to current context
    pub quality:    f64,     // 0.0..=1.0 — how well-formed / correct
    pub confidence: f64,     // 0.0..=1.0 — how certain we are
    pub novelty:    f64,     // 0.0..=1.0 — how new / surprising
    pub utility:    f64,     // 0.0..=1.0 — how useful for downstream work
}
```

Score-protocol Blocks produce these dimensions. Route-protocol Blocks consume them to select among candidates. The Compose protocol uses them for budget-constrained assembly (highest-utility Signals win prompt space).

---

## 6. Decay Model

Signals decay following the Ebbinghaus forgetting curve:

```
confidence(t) = initial × 0.5^(age / half_life) × tier_multiplier × (1 + confirmations × 0.1)
```

### Tiers

```rust
pub enum Tier {
    Transient,     // 0.1× — decays 10× faster than base
    Working,       // 0.5× — decays 2× faster
    Consolidated,  // 1.0× — base rate
    Persistent,    // 5.0× — decays 5× slower
}
```

A Transient Signal with a 30-day base half-life has an effective half-life of 3 days. A Persistent Signal with the same base has 150 days.

### Tier progression

Signals promote through tiers based on validation:

```
Transient → Working:      3+ independent confirmations (gate passes using this Signal)
Working → Consolidated:   5+ confirmations across distinct contexts
Consolidated → Persistent: community consensus (3+ validators) or explicit freeze
```

Demotion reverses: gate failures using a Signal demote it one tier.

### Frozen Signals

Signals can be frozen — they skip decay entirely and remain at their current confidence indefinitely. Freezing requires consortium approval (3+ validators across distinct contexts). Frozen Signals are the system's bedrock knowledge.

### Pruning

When a Signal's decayed confidence drops below 1% of its initial value (`DEATH_THRESHOLD = 0.01`), it becomes eligible for pruning. Pruned Signals are archived to cold storage, preserving their content hash, lineage, and provenance. They can be thawed if conditions change.

---

## 7. Lineage and Provenance

Every Signal tracks where it came from:

### Lineage (structural)

```rust
pub source: Vec<SignalRef>  // upstream Signals that contributed to this one
```

Walking `source[]` recursively produces a DAG of all upstream Signals. This enables:
- `roko replay <hash>` — walk the Signal DAG by hash
- Artifact lineage queries — "what produced this?"
- Blame attribution — "which upstream Signal caused this failure?"

### Provenance (metadata)

```rust
pub struct Provenance {
    /// For ingested content: source file, line range, commit hash.
    pub source_files: Vec<SourceFileRange>,
    /// For LLM-generated content: model ID, prompt hash, temperature, seed.
    pub generation: Option<GenerationProvenance>,
    /// For web-fetched content: URL, timestamp, HTTP status, content hash.
    pub web_fetch: Option<WebFetchProvenance>,
    /// Citations the Signal claims to have used.
    pub citations: Vec<Citation>,
}
```

Provenance is metadata about how a Signal was produced. Lineage is the structural relationship between Signals. Both are queryable.

---

## 8. HDC Fingerprint

Every Signal carries a 10,240-bit binary HDC (Hyperdimensional Computing) vector for similarity search and cross-domain pattern discovery.

### Encoding

Structured information enters a single vector through role-filler binding:

```rust
pub fn encode_signal(signal: &Signal) -> HdcVector {
    let roles_and_fillers = vec![
        ("kind", signal.kind.to_string()),
        ("tags", signal.tags.join(",")),
        ("author", signal.author.to_string()),
        // ... kind-specific fields
    ];
    HdcVector::encode_structured(&roles_and_fillers)
}
```

### Operations

| Operation | What | Cost |
|---|---|---|
| **Bind** (XOR) | Combine two vectors into one dissimilar to both | O(n) |
| **Bundle** (majority vote) | Combine multiple vectors into one similar to all | O(n×k) |
| **Permute** (bit rotation) | Encode position / sequence | O(n) |
| **Similarity** (Hamming) | Measure overlap via POPCNT | < 1 μs |

### Cross-domain resonance

When Signals from different domains have similar HDC fingerprints, they share structural properties. A retry pattern from networking might apply to database operations. A rate-limiting strategy from API design might transfer to gas optimization.

Retrieval scoring gives cross-domain matches a 15% bonus:

```
final_score = hdc_similarity × 0.40
            + keyword_relevance × 0.30
            + utility × 0.20
            + freshness × 0.10
            + (cross_domain ? 0.15 : 0.0)
```

### Why HDC instead of float embeddings?

| Property | HDC (10,240-bit binary) | Float (1536-d float32) |
|---|---|---|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity cost | XOR + POPCNT (1 cycle) | Dot product (hundreds of FLOPs) |
| Compositionality | Native (bind/bundle/permute) | Requires learned operations |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder models |
| Determinism | Identical seeds → identical vectors | Depends on model version |

---

## 9. Bus (Ephemeral Transport)

Ephemeral Signals live on the **Bus** — an in-memory pub/sub transport organized by topics.

```rust
pub trait Bus: Send + Sync {
    /// Publish a Signal to a topic.
    async fn publish(&self, topic: &str, signal: Signal);

    /// Subscribe to a topic. Returns a stream of Signals.
    fn subscribe(&self, topic: &str) -> SignalStream;

    /// Subscribe with a filter expression.
    fn subscribe_filtered(&self, topic: &str, filter: Expr) -> SignalStream;
}
```

### Topic naming convention

```
signal:{kind}                     All Signals of a Kind
block:{id}:input                  Input to a specific Block
block:{id}:output                 Output from a specific Block
graph:{id}:events                 Graph lifecycle events
agent:{id}:heartbeat              Agent heartbeat ticks
agent:{id}:output                 Agent streaming output
pheromone:{location_hash}         Pheromones at a location
lens:{id}:observations            Lens observation output
system                            System-wide events
```

### Backpressure

High-frequency Signal sources (heartbeats at 100ms, chain blocks at 2s) use per-topic strategies:

| Strategy | Used for | Behavior |
|---|---|---|
| Coalesce | Heartbeats | Buffer, send latest per interval |
| Drop-oldest | Streaming output | Ring buffer, slow consumers miss old |
| Lossless | Gate results, completions | Queue with TCP-level flow control |
| Sample | Feed data | Every Nth update |

---

## 10. Store (Persisted Storage)

Persisted Signals live in a **Store** — a Block implementing the Store protocol.

```rust
pub trait Store: Block {
    async fn put(&self, signal: Signal) -> Result<SignalRef>;
    async fn get(&self, id: &SignalId) -> Result<Option<Signal>>;
    async fn query(&self, query: StoreQuery) -> Result<Vec<Signal>>;
    async fn prune(&self, threshold: f64) -> Result<PruneReport>;
}
```

### Storage layout

```
.roko/
├── signals.jsonl          # primary Signal log (append-only)
├── neuro/
│   └── knowledge.jsonl    # knowledge Signals (with decay, tiers)
├── episodes.jsonl         # episode Signals
├── runs/<run-id>/
│   ├── artifacts/         # artifact Signals
│   └── events.jsonl       # ephemeral Signals snapshot
└── learn/
    ├── reflexes.jsonl     # promoted T0 reflex Signals
    └── efficiency.jsonl   # efficiency observation Signals
```

Different Stores specialize in different Signal kinds, but all implement the same protocol. The FileSubstrate in `roko-fs` is the default Store for local development.

---

## 11. AntiKnowledge

When the system discovers that a previously trusted Signal is wrong, it creates an **AntiKnowledge** Signal that actively repels future Signals in the same HDC region.

```rust
const ANTI_KNOWLEDGE_WARN_THRESHOLD: f64 = 0.5;      // log warning
const ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD: f64 = 0.7;  // halve confidence
const ANTI_KNOWLEDGE_REJECT_THRESHOLD: f64 = 0.9;    // reject outright
```

When a new Signal arrives whose HDC vector is similar to an existing AntiKnowledge Signal:
- Above 0.5 similarity: log a warning
- Above 0.7: halve the new Signal's confidence
- Above 0.9: reject the Signal outright

This prevents the system from rediscovering known-bad information.

---

## 12. Signal Lifecycle

```
Created (by Block or external source)
    │
    ├── Ephemeral path ──► Bus topic ──► consumed by subscribers ──► gone
    │
    └── Persisted path ──► Store.put() ──► scored ──► routed ──► composed
                               │
                               ├── validated ──► confidence ↑, tier ↑
                               ├── challenged ──► confidence ↓, tier ↓
                               ├── decayed ──► below threshold ──► pruned ──► cold storage
                               └── frozen ──► permanent (skip decay)
```

---

## 13. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Signal struct has all fields from this spec | Compile check on type alias `type Signal = Engram` |
| Content hash is deterministic: same payload → same hash | Unit test: two Signals with identical payloads have identical hashes |
| Decay formula matches Ebbinghaus: `initial × 0.5^(age/half_life) × tier_mult` | Unit test: confidence at t=half_life equals initial×tier_mult/2 |
| Tier progression: 3 confirmations promote Transient → Working | Integration test: validate a Signal 3 times, check tier |
| AntiKnowledge repulsion: new Signal with >0.9 HDC similarity is rejected | Unit test: create AntiKnowledge, submit similar Signal, verify rejection |
| Bus pub/sub: ephemeral Signal published to topic is received by subscriber | Integration test: publish + subscribe on same topic |
| Store round-trip: put + get returns identical Signal | Integration test: put Signal, get by ID, compare |
| Lineage walk: `source[]` recursion produces correct DAG | Integration test: create chain A→B→C, walk from C, verify [B, A] |
| HDC fingerprint determinism: same inputs → same fingerprint | Unit test: encode same Signal twice, compare vectors |
