# 01 — Signal and Pulse

> Two mediums: Signal (durable) in Store, Pulse (ephemeral) on Bus. Graduation converts Pulse → Signal. Everything that flows through Roko is one or the other.

**Subsumes**: Engram, Pulse/Envelope, Artifact, Knowledge Entry, Pheromone, Evidence, Feed event, ModuleOutput, Finding.

---

## 1. Two Mediums

The system has two data shapes because reality has two timescales: things that persist and things that flow.

| Property | Signal (durable) | Pulse (ephemeral) |
|---|---|---|
| **Identity** | Content hash (SHA-256 of payload) | (topic, seq) tuple |
| **Durability** | Store (`.roko/signals.jsonl`, knowledge store) | Ring buffer on Bus (~64K entries) |
| **Lineage** | Full `Vec<SignalRef>` provenance chain | Optional `lineage_hint: Option<ContentHash>` |
| **Scoring** | Multi-dimensional Score (5 axes) | None |
| **Demurrage** | Balance decays unless actively used | N/A (expires with buffer) |
| **HDC fingerprint** | 10,240-bit binary vector | None (too transient to encode) |
| **Rate** | 1 Hz – 1 kHz | 1 Hz – 1 MHz |
| **Typical lifetime** | Minutes to permanent | Milliseconds to seconds |

They are siblings, not parent-child. A Signal is not "a Pulse that grew up." They serve different purposes and carry different structure. The only bridge between them is explicit:

- **Graduation**: `Pulse::graduate(provenance, decay, score, tags) → Signal` — the ONLY path from transport into the audit DAG.
- **Projection**: `Signal::to_pulse(topic, seq) → Pulse` — lossy broadcast of stored Signals for real-time consumers.

---

## 2. Signal — The Durable Medium

A **Signal** is a content-addressed, typed, scored, decaying, lineage-tracked data unit with an HDC fingerprint. Signals are the durable noun of the system — the 9 protocols (Store, Score, Verify, Route, Compose, React, Observe, Connect, Trigger) are verbs that operate on them.

```rust
pub struct Signal {
    // ── Identity ──────────────────────────────────────────────────
    pub id: SignalId,                    // ULID, globally unique
    pub content_hash: ContentHash,       // SHA-256 of payload bytes
    pub kind: Kind,                      // discriminant (see §4)

    // ── Content ───────────────────────────────────────────────────
    pub payload: Value,                  // serde_json::Value, schema-validated
    pub schema: TypeSchema,              // structural type of the payload

    // ── Scoring ───────────────────────────────────────────────────
    pub score: Score,                    // multi-dimensional quality signal
    pub confidence: f64,                 // 0.0..=1.0

    // ── Demurrage ─────────────────────────────────────────────────
    pub balance: f64,                    // starts at 1.0, decays via demurrage
    pub demurrage_paid: f64,             // monotonic, for observability
    pub last_touched_at: DateTime<Utc>,  // last retrieval, citation, or gate-pass
    pub tier: Tier,                      // Transient | Working | Consolidated | Persistent
    pub created_at: DateTime<Utc>,

    // ── Lineage ───────────────────────────────────────────────────
    pub source: Vec<SignalRef>,          // upstream Signals (provenance graph)
    pub provenance: Provenance,          // generation metadata, citations

    // ── Embedding ─────────────────────────────────────────────────
    pub hdc_fingerprint: HdcVector,      // 10,240-bit binary vector (1,280 bytes)

    // ── Authorship ────────────────────────────────────────────────
    pub author: Author,                  // agent ID, wallet address, or system
    pub tags: Vec<String>,               // topic tags for filtering and discovery
}
```

### Mapping to existing code

`Signal` maps directly to `roko-core::Engram`. The Rust struct remains `Engram` for backward compatibility; `type Signal = Engram;` bridges.

---

## 3. Pulse — The Ephemeral Medium

A **Pulse** is a sequence-numbered, topic-scoped, ring-buffered message on the Bus. Pulses carry events, heartbeats, streaming output, coordination signals, and any data that matters for seconds, not days.

```rust
pub struct Pulse {
    pub seq: u64,                        // monotonic per Bus instance
    pub topic: Topic,                    // hierarchical string
    pub kind: Kind,                      // reused from Signal
    pub body: Value,                     // payload
    pub emitted_at_ms: i64,              // Unix ms, server clock
    pub source: PulseSource,             // who emitted
    pub lineage_hint: Option<ContentHash>, // back-reference to Signal context
    pub trace_id: Option<TraceId>,       // distributed tracing
}

pub enum PulseSource {
    Agent(AgentId),
    Block(BlockRef),
    Graph(GraphRef),
    System,
    External(String),
}
```

### Topic taxonomy

```
signal:{kind}                     All Signals of a Kind
block:{id}:input                  Input to a specific Block
block:{id}:output                 Output from a specific Block
graph:{id}:events                 Graph lifecycle events
agent:{id}:heartbeat              Agent heartbeat ticks
agent:{id}:output                 Agent streaming output
pheromone:{location_hash}         Pheromones at a location
lens:{id}:observations            Lens observation output
prediction:{operator}             Operator predictions (for calibration)
outcome:{operator}                Operator outcomes (for calibration)
calibration:{operator}:updated    Calibration error signals
system                            System-wide events
```

---

## 4. Kind System

Every Signal (and Pulse) has a `Kind` that determines schema, demurrage behavior, and how Blocks interact with it.

```rust
#[non_exhaustive]
pub enum Kind {
    // ── Core data ──────────────────────────────────────
    Text,
    Markdown,
    Json,
    Toml,
    Code { language: String },
    Diff,
    Binary { mime: String },
    Image { format: String },

    // ── Artifacts ──────────────────────────────────────
    File { path: PathBuf },
    Artifact { kind: ArtifactKind },

    // ── Knowledge ──────────────────────────────────────
    Insight,                             // observed pattern with supporting evidence
    Heuristic,                           // behavioral rule: when/then + mandatory falsifier
    Warning,                             // transient danger signal
    CausalLink,                          // cause → effect relationship
    StrategyFragment,                    // reusable strategy component
    AntiKnowledge,                       // known-bad information (repels similar entries)

    // ── Coordination ───────────────────────────────────
    Pheromone { ptype: PheromoneType },  // stigmergic signal with location + intensity
    Heartbeat,                           // agent health tick
    Presence { event: PresenceEvent },   // join/leave/supersede

    // ── Execution ──────────────────────────────────────
    Evidence { kind: EvidenceKind },     // verification evidence (typed, separate from Criterion)
    Finding { severity: Severity },      // verification finding
    Verdict,                             // gate result: passed + reward + evidence + findings
    Episode,                             // recorded agent turn
    CostReport,                          // cost/latency/quality metrics

    // ── Observation ────────────────────────────────────
    Observation,                         // Lens output
    Alert { level: AlertLevel },         // threshold breach notification
    Trend,                               // statistical trend (slope, EMA)
    Anomaly,                             // statistical outlier

    // ── User-defined ───────────────────────────────────
    Custom { name: String },
}
```

### Kind::Heuristic — first-class learned rule

A Heuristic Signal carries a when/then clause with a mandatory falsifier:

```rust
pub struct HeuristicPayload {
    pub when: Vec<Predicate>,            // preconditions
    pub then: String,                    // action or prediction
    pub falsifier: String,               // "what would prove this wrong?"
    pub calibration: Calibration,        // track record
    pub receipts: Vec<SignalRef>,        // episodes where tested
}

pub struct Calibration {
    pub trials: u32,
    pub confirmations: u32,
    pub violations: u32,
    pub brier_score: f64,
    pub confidence_interval: (f64, f64), // Wilson score CI
}
```

Heuristics are live-calibrated from Bus events (gate verdicts, agent outcomes). Confidence CI decays via demurrage if unchallenged. Worldviews emerge as coherent clusters of co-citing heuristics with high calibration scores.

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

Score-protocol Blocks produce these dimensions. Route-protocol Blocks consume them. Compose uses them for budget-constrained assembly. Novelty is attenuated by frequency: `novelty = 1/(1+ln(freq))` — habituation that never reaches zero.

---

## 6. Demurrage Model

Signals decay via **demurrage** — an attention-weighted holding cost that replaces pure time-based Ebbinghaus decay. Every Signal has a `balance` that starts at 1.0 and decreases over time unless actively reinforced.

### The rate law

```
balance(t+Δt) = balance(t) - r·Δt - β·balance(t)·Δt
```

Where:
- `r` = flat tax per day (default 0.01)
- `β` = exponential decay rate per day (default 0.02)

### Reinforcement

Active usage restores balance:

```rust
pub enum ReinforceKind {
    Retrieved,      // queried and returned to an agent
    Cited,          // explicitly cited in another Signal's lineage
    GatePassed,     // was in context pack when gate passed
    Surprised,      // high prediction error in context of this Signal
    AgentQuoted,    // agent referenced in output
}
```

Each reinforcement event adds a bonus weighted by **novelty**: `balance += bonus(kind) * novelty(signal)` where `novelty = 1 - max_similarity` against top-K HDC neighbors. Citing a common Signal → small bump. Citing a rare Signal → large bump. This is the anti-hoarding mechanism.

### Why demurrage instead of Ebbinghaus

Ebbinghaus is the special case where no interactions occur. Demurrage is strictly more expressive:
- **Self-trimming**: duplicates get fewer citations → faster decay. Unique insights get cited → stay warm.
- **Usage-based**: a Signal that is retrieved daily stays fresh. One that is never accessed fades.
- **Compounding**: the retrieval → gate-pass → reinforcement loop is superlinear.
- **Observable**: balance is a first-class field, visible in TUI and queryable via API.

### Tier multipliers

```rust
pub enum Tier {
    Transient,     // 0.1× — decays 10× faster than base
    Working,       // 0.5× — decays 2× faster
    Consolidated,  // 1.0× — base rate
    Persistent,    // 5.0× — decays 5× slower
}
```

Tier progression (same criteria as v1):
- Transient → Working: 3+ independent gate passes where this Signal was in context
- Working → Consolidated: 5+ confirmations across distinct contexts
- Consolidated → Persistent: consortium approval (3+ validators) or explicit freeze

### Cold threshold

When balance drops below `COLD_THRESHOLD` (default 0.01), the Signal enters cold storage. Body moves to slower storage; hash stays valid; lineage preserved. Thaw restores balance to a starter value and is itself a Bus event.

---

## 7. Content Addressing

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
- **Deduplication**: identical outputs reference the same Signal
- **Integrity**: hash verifies content hasn't been tampered with
- **Lineage verification**: walking `source[]` and checking hashes proves unbroken provenance
- **Semantic caching**: content-addressed Signals maximize cache reuse across Flows (5× cost reduction)
- **On-chain commitment**: content hash is the on-chain record; full content lives off-chain

---

## 8. Lineage and Provenance

### Lineage (structural)

```rust
pub source: Vec<SignalRef>  // upstream Signals that contributed to this one
```

Walking `source[]` recursively produces a DAG. Enables `roko replay <hash>`, artifact lineage queries, and blame attribution.

### Provenance (metadata)

```rust
pub struct Provenance {
    pub source_files: Vec<SourceFileRange>,
    pub generation: Option<GenerationProvenance>,
    pub web_fetch: Option<WebFetchProvenance>,
    pub citations: Vec<Citation>,
}
```

---

## 9. HDC Fingerprint

Every Signal carries a 10,240-bit binary HDC vector for similarity search and cross-domain pattern discovery.

### Operations

| Operation | What | Cost |
|---|---|---|
| **Bind** (XOR) | Role-filler binding: `bind(ROLE, value)` | O(n) |
| **Bundle** (majority vote) | Consensus: similar to all inputs | O(n×k) |
| **Permute** (bit rotation) | Positional encoding | O(n) |
| **Similarity** (Hamming) | Overlap via POPCNT | < 1 μs |
| **Resonator** | Factorize: recover constituents from bundle | O(n×k×iter) |

### Cross-domain resonance

When Signals from different domains have similar fingerprints, they share structural properties. Retrieval gives cross-domain matches a 15% bonus. A retry pattern from networking might apply to database operations.

### Why HDC instead of float embeddings

| Property | HDC (10,240-bit binary) | Float (1536-d float32) |
|---|---|---|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity cost | XOR + POPCNT (1 cycle) | Dot product (hundreds of FLOPs) |
| Compositionality | Native (bind/bundle/permute/resonate) | Requires learned operations |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder models |
| Determinism | Identical seeds → identical vectors | Depends on model version |

---

## 10. Bus (Ephemeral Transport)

The **Bus** is the ephemeral transport fabric — a kernel-level pub/sub system alongside Store. Every real-time behavior (heartbeat, event streaming, learning feedback, pheromone sensing, predict-publish-correct calibration) runs through Bus.

```rust
pub trait Bus: Send + Sync {
    /// Publish a Pulse to its topic.
    async fn publish(&self, pulse: Pulse) -> Result<u64>;

    /// Subscribe to topics matching a filter.
    fn subscribe(&self, filter: TopicFilter) -> PulseStream;

    /// Replay Pulses since a sequence number (for reconnection).
    async fn replay_since(&self, since: u64, filter: &TopicFilter) -> Result<Vec<Pulse>>;

    /// Current high-water sequence number.
    async fn current_seq(&self) -> Result<u64>;
}

pub enum TopicFilter {
    Exact(Topic),
    Glob(String),          // e.g., "agent:*:heartbeat"
    AnyOf(Vec<Topic>),
    All,
}
```

### Backpressure strategies

| Strategy | Used for | Behavior |
|---|---|---|
| Coalesce | Heartbeats | Buffer, send latest per interval |
| Drop-oldest | Streaming output | Ring buffer, slow consumers miss old |
| Lossless | Gate results, completions | Queue with flow control |
| Sample | Feed data | Every Nth update |

### Bus backends

- `BroadcastBus` (in-process, `tokio::sync::broadcast`) — ships immediately, default
- `MemoryBus` — testing
- `NatsBus` / `KafkaBus` — Phase 2 (multi-machine)
- `ChainBus` — Phase 2+ (on-chain event integration)

---

## 11. Store (Persisted Storage)

Persisted Signals live in a **Store** — a Block implementing the Store protocol.

```rust
pub trait Store: Block {
    async fn put(&self, signal: Signal) -> Result<SignalRef>;
    async fn get(&self, id: &SignalId) -> Result<Option<Signal>>;
    async fn query(&self, query: StoreQuery) -> Result<Vec<Signal>>;
    async fn query_similar(&self, fp: &HdcVector, radius: f32, limit: usize) -> Result<Vec<(SignalRef, f32)>>;
    async fn prune(&self, threshold: f64) -> Result<PruneReport>;
}
```

`query_similar` is native HDC similarity search over stored Signals — no external vector store. At 10,240 bits, 800K fingerprints fit in 1 GB RAM; brute-force SIMD is <1 ms.

### Storage layout

```
.roko/
├── signals.jsonl          # primary Signal log (append-only)
├── neuro/
│   └── knowledge.jsonl    # knowledge Signals (with demurrage, tiers)
├── episodes.jsonl         # episode Signals
├── runs/<run-id>/
│   ├── artifacts/         # artifact Signals
│   └── events.jsonl       # graduated Pulse snapshots
└── learn/
    ├── reflexes.jsonl     # promoted T0 reflex Signals
    └── efficiency.jsonl   # efficiency observation Signals
```

---

## 12. Graduation and Projection

### Graduation: Pulse → Signal

The only path from ephemeral to durable. A graduation policy determines which Pulses are worth persisting:

```rust
impl Pulse {
    pub fn graduate(
        &self,
        provenance: Provenance,
        initial_balance: f64,
        score: Score,
        tags: Vec<String>,
    ) -> Signal { ... }
}
```

| Topic | Graduate? | Rationale |
|---|---|---|
| `gate.verdict.emitted` | Yes | Gate results are audit-critical |
| `agent.turn.completed` | Yes (batch) | Episodes feed learning |
| `heartbeat.tick` | No | Latest is all that matters |
| `agent.msg.chunk` | Batch on stream close | Full response is an artifact |
| `cost.charged` | Yes | Accounting record |
| `pheromone.deposited` | No (on-chain only) | Ephemeral by design |

### Projection: Signal → Pulse

Lossy broadcast for real-time consumers:

```rust
impl Signal {
    pub fn to_pulse(&self, topic: Topic, seq: u64) -> Pulse { ... }
}
```

---

## 13. AntiKnowledge

When a previously trusted Signal is proven wrong, an **AntiKnowledge** Signal actively repels future Signals in the same HDC region.

```rust
const ANTI_KNOWLEDGE_WARN_THRESHOLD: f64 = 0.5;
const ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD: f64 = 0.7;
const ANTI_KNOWLEDGE_REJECT_THRESHOLD: f64 = 0.9;
```

| Similarity to AntiKnowledge | Action |
|---|---|
| Above 0.5 | Log warning |
| Above 0.7 | Halve new Signal's initial balance |
| Above 0.9 | Reject outright — Signal not stored |

AntiKnowledge itself decays via demurrage (30-day base). Old mistakes eventually stop blocking new discoveries.

---

## 14. Signal Lifecycle

```
Created (by Block or external source)
    │
    ├── Pulse path ──► Bus topic ──► consumed by subscribers ──► expires from ring buffer
    │                                       │
    │                            graduate() if policy says yes
    │                                       │
    │                                       ▼
    └── Signal path ──► Store.put() ──► scored ──► routed ──► composed
                             │
                             ├── retrieved ──► balance ↑ (reinforcement)
                             ├── gate passed ──► balance ↑, tier ↑
                             ├── challenged ──► balance ↓, tier ↓
                             ├── demurrage ──► balance ↓ over time
                             ├── cold ──► balance < 0.01 ──► archive to cold storage
                             └── frozen ──► permanent (skip demurrage)
```

---

## 15. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Signal struct has all fields from this spec including `balance`, `demurrage_paid`, `last_touched_at` | Compile check |
| Pulse struct compiles with seq, topic, kind, body, source, lineage_hint | Compile check |
| Content hash is deterministic: same payload → same hash | Unit test |
| Demurrage formula: balance decreases over time, increases on reinforcement | Unit test |
| Novelty-weighted reinforcement: rare Signals get larger bonus | Unit test with two Signals at different HDC distances |
| Tier progression: 3 gate-passes promote Transient → Working | Integration test |
| AntiKnowledge repulsion: >0.9 HDC similarity rejected | Unit test |
| Bus pub/sub: Pulse published to topic received by subscriber | Integration test |
| Bus replay: reconnecting subscriber receives missed Pulses | Integration test |
| Graduation: Pulse.graduate() produces valid Signal with provenance | Unit test |
| Projection: Signal.to_pulse() produces valid Pulse | Unit test |
| Store round-trip: put + get returns identical Signal | Integration test |
| Store.query_similar: returns Signals by HDC similarity | Integration test |
| Cold threshold: balance < 0.01 triggers archive | Unit test |
| Heuristic kind: when/then + falsifier + calibration fields present | Compile check |
| Lineage walk: `source[]` recursion produces correct DAG | Integration test |
| HDC fingerprint determinism: same inputs → same fingerprint | Unit test |
