# 01 — Signal and Pulse

> Two mediums: Signal (durable) in Store, Pulse (ephemeral) on Bus. Graduation converts Pulse → Signal. Everything that flows through Roko is one or the other.

**Subsumes**: Engram, Pulse/Envelope, Artifact, Knowledge Entry, Pheromone, Evidence, Feed event, Finding.

---

## 1. Two Mediums

The system has two data shapes because reality has two timescales: things that persist and things that flow. The v1 spec claimed "one noun" (Signal) but the code already had two — `Engram` for durable data and `Envelope<E>` in `roko-runtime::event_bus` for ephemeral messages. This spec makes both first-class.

| Property | Signal (durable) | Pulse (ephemeral) |
|---|---|---|
| **Identity** | Content hash (SHA-256 of payload) | `(topic, seq)` tuple |
| **Durability** | Store (`.roko/signals.jsonl`, knowledge store) | Ring buffer on Bus (~4,096 entries default) |
| **Lineage** | Full `Vec<SignalRef>` provenance DAG | Optional `lineage_hint: Option<ContentHash>` |
| **Scoring** | 5-dimensional Score | None |
| **Retention** | Demurrage (Gesell 1916): balance decays unless actively used | Ring buffer eviction |
| **HDC fingerprint** | 10,240-bit binary vector (1,280 bytes, Kanerva 2009) | None (too transient) |
| **Typical rate** | 1 Hz – 1 kHz | 1 Hz – 1 MHz |
| **Typical lifetime** | Minutes to permanent | Milliseconds to seconds |

They are **siblings, not parent-child**. A Signal is not "a Pulse that grew up." The only bridges are explicit:

- **Graduation**: `Pulse::graduate(provenance, initial_balance, score, tags) → Signal` — the ONLY path from transport into the audit DAG.
- **Projection**: `Signal::to_pulse(topic, seq) → Pulse` — lossy broadcast of stored Signals.

---

## 2. Signal — The Durable Medium

```rust
pub struct Signal {
    // ── Identity ──────────────────────────────────────────────────
    pub id: SignalId,                    // ULID, globally unique
    pub content_hash: ContentHash,       // SHA-256 of canonical payload bytes
    pub kind: Kind,                      // discriminant (see §4)

    // ── Content ───────────────────────────────────────────────────
    pub payload: Value,                  // serde_json::Value, schema-validated
    pub schema: TypeSchema,              // structural type

    // ── Scoring ───────────────────────────────────────────────────
    pub score: Score,                    // 5-axis quality rating
    pub confidence: f64,                 // 0.0..=1.0

    // ── Demurrage ─────────────────────────────────────────────────
    pub balance: f64,                    // starts at 1.0, decays via demurrage
    pub demurrage_paid: f64,             // cumulative tax paid (monotonic)
    pub last_touched_at: DateTime<Utc>,  // last retrieval, citation, or gate-pass
    pub tier: Tier,                      // Transient | Working | Consolidated | Persistent
    pub created_at: DateTime<Utc>,

    // ── Lineage ───────────────────────────────────────────────────
    pub source: Vec<SignalRef>,          // upstream Signals (provenance DAG)
    pub provenance: Provenance,          // generation metadata, citations, sources

    // ── Embedding ─────────────────────────────────────────────────
    pub hdc_fingerprint: HdcVector,      // 10,240-bit binary vector (1,280 bytes)

    // ── Authorship ────────────────────────────────────────────────
    pub author: Author,                  // agent ID, wallet address, or system
    pub tags: Vec<String>,               // topic tags for discovery
}
```

**Mapping to code**: `Signal` maps 1:1 to `roko-core::Engram`. The Rust struct remains `Engram`; new code bridges with `type Signal = Engram;`.

---

## 3. Pulse — The Ephemeral Medium

```rust
pub struct Pulse {
    pub seq: u64,                        // monotonic per Bus instance
    pub topic: Topic,                    // hierarchical string (OpenTelemetry-style)
    pub kind: Kind,                      // reused from Signal
    pub body: Value,                     // payload
    pub emitted_at_ms: i64,              // Unix ms, server clock
    pub source: PulseSource,             // who emitted
    pub lineage_hint: Option<ContentHash>, // back-reference to Signal context
    pub trace_id: Option<TraceId>,       // distributed tracing
}

pub enum PulseSource {
    Agent(AgentId),
    Cell(CellRef),
    Graph(GraphRef),
    System,
    External(String),
}
```

**Mapping to code**: Pulse replaces `Envelope<E>` in `roko-runtime::event_bus`.

### Topic taxonomy

```
orchestration.plan.started           Plan lifecycle
orchestration.task.ready             Task readiness
agent:{id}.heartbeat                 Agent heartbeat ticks
agent:{id}.output                    Streaming LLM output
agent:{id}.turn.completed            Turn completed
gate.verdict.emitted                 Gate results (graduates)
safety.approval.requested            Safety events (graduates)
conductor.circuit.tripped            Health events (graduates)
prediction.{operator}                Operator predictions (for calibration)
outcome.{operator}                   Operator outcomes (for calibration)
calibration.{operator}.updated       Error signals
pheromone.{location_hash}            Stigmergic coordination
cost.charged                         Budget tracking
ui.refresh.requested                 UI-only (does not graduate)
heartbeat.tick                       Clock infrastructure (does not graduate)
```

### Graduation policy

| Topic | Graduate? | Rationale |
|---|---|---|
| `gate.verdict.emitted` | Yes | Audit-critical |
| `agent.*.turn.completed` | Yes (batch) | Episodes feed learning |
| `safety.approval.requested` | Yes | Safety must be auditable |
| `conductor.circuit.tripped` | Yes | Health events are forensic |
| `cost.charged` | Yes | Accounting record |
| `agent.*.output` (chunks) | Batch on stream close | Individual chunks are noise; full response is artifact |
| `heartbeat.tick` | No | Latest is all that matters |
| `ui.refresh.requested` | No | UI-local |
| `pheromone.*` | No (on-chain only) | Ephemeral by design |

---

## 4. Kind System

Every Signal and Pulse has a `Kind` determining schema, demurrage behavior, and Cell interaction.

```rust
#[non_exhaustive]
pub enum Kind {
    // ── Core data ──────────────────────────────────────
    Text, Markdown, Json, Toml,
    Code { language: String },
    Diff, Binary { mime: String }, Image { format: String },

    // ── Artifacts ──────────────────────────────────────
    File { path: PathBuf },
    Artifact { kind: ArtifactKind },

    // ── Knowledge ──────────────────────────────────────
    Insight,                             // observed pattern + evidence
    Heuristic,                           // when/then + mandatory falsifier + calibration
    Warning,                             // transient danger flag
    CausalLink,                          // cause → effect
    StrategyFragment,                    // reusable strategy component
    AntiKnowledge,                       // known-bad (repels similar entries)

    // ── Coordination ───────────────────────────────────
    Pheromone { ptype: PheromoneType },  // stigmergic: location + intensity
    Heartbeat,
    Presence { event: PresenceEvent },

    // ── Execution ──────────────────────────────────────
    Evidence { kind: EvidenceKind },     // typed verification evidence (19 kinds)
    Finding { severity: Severity },      // verification finding
    Verdict,                             // pass/fail + reward + evidence
    Episode,                             // recorded agent turn
    CostReport,

    // ── Observation ────────────────────────────────────
    Observation, Alert { level: AlertLevel }, Trend, Anomaly,

    // ── User-defined ───────────────────────────────────
    Custom { name: String },
}
```

### Kind::Heuristic — first-class learned rule

A Heuristic is a testable prediction with a mandatory falsifier and a live calibration track record grounded in episode outcomes (not LLM self-report). Heuristics are richer than playbooks (sequences of actions) and more formal than rules of thumb.

```rust
pub struct HeuristicPayload {
    pub when: Vec<Predicate>,            // preconditions (matchable)
    pub then: String,                    // action or prediction
    pub falsifier: String,               // "what would prove this wrong?"
    pub calibration: Calibration,        // live track record
    pub receipts: Vec<SignalRef>,        // episodes where tested
}

pub struct Calibration {
    pub trials: u32,
    pub confirmations: u32,
    pub violations: u32,
    pub brier_score: f64,                // calibration quality
    pub confidence_interval: (f64, f64), // Wilson score CI
}
```

Heuristics are live-calibrated from Bus events (gate verdicts, agent outcomes). Confidence CI decays via demurrage if unchallenged. **Worldviews** emerge as coherent clusters of co-citing heuristics with high calibration scores (see [Doc-11](11-MEMORY-AND-KNOWLEDGE.md)). Multiple worldviews are maintained deliberately: main + challenger + niche specialists. (Cf. Quinlan ID3 on a live stream for heuristic evolution.)

---

## 5. Scoring

Every Signal carries a 5-dimensional `Score`:

```rust
pub struct Score {
    pub relevance:  f64,     // 0.0..=1.0
    pub quality:    f64,     // 0.0..=1.0
    pub confidence: f64,     // 0.0..=1.0
    pub novelty:    f64,     // 0.0..=1.0  (attenuated: 1/(1+ln(freq)))
    pub utility:    f64,     // 0.0..=1.0
}
```

Score Cells produce these. Route Cells consume them. Compose uses them for budget-constrained assembly. **Novelty attenuation**: `novelty = 1/(1+ln(freq))` — habituation that never reaches zero, so even highly familiar Signals retain a nonzero novelty floor.

---

## 6. Demurrage Model

Signals decay via **demurrage** (Gesell 1916) — an attention-weighted holding cost replacing pure time-based Ebbinghaus. Every Signal has a `balance` that starts at 1.0 and decreases unless actively reinforced.

### Rate law

```
balance(t+Δt) = balance(t) − r·Δt − β·balance(t)·Δt
```

- `r` = flat tax per day (default 0.01) — constant drain
- `β` = exponential decay rate per day (default 0.02) — keeps value bounded

### Reinforcement

Active usage restores balance, weighted by **novelty** (anti-hoarding mechanism):

```rust
pub enum ReinforceKind {
    Retrieved,      // returned in a query
    Cited,          // in another Signal's source[] lineage
    GatePassed,     // in context pack when gate passed
    Surprised,      // high prediction error in context (Shannon surprise as economic bonus)
    AgentQuoted,    // agent referenced in output
}
```

`balance += bonus(kind) × novelty(signal)` where `novelty = 1 − max_similarity` against top-K HDC neighbors. Citing a common Signal → small bump. Citing a rare Signal → large bump.

### Per-Kind default rates

| Kind | Flat tax (r) | Exp decay (β) | Rationale |
|---|---|---|---|
| Core data (Text, Code) | 0.001 | 0.001 | Data artifacts are inherently stable |
| Insight | 0.01 | 0.02 | Observations need ongoing confirmation |
| Heuristic | 0.005 | 0.01 | Behavioral rules are durable once proven |
| Warning | 0.10 | 0.20 | Danger signals are deliberately short-lived |
| StrategyFragment | 0.02 | 0.03 | Strategies go stale in evolving codebases |
| AntiKnowledge | 0.01 | 0.02 | What-not-to-do stays relevant |
| Episode | 0.005 | 0.01 | Episodes feed learning loops |

### Why demurrage instead of Ebbinghaus

Ebbinghaus is the special case where no interactions occur. Demurrage is strictly more expressive:
- **Self-trimming**: duplicates get fewer citations → faster decay. Unique insights get cited → stay warm.
- **Usage-based**: a Signal retrieved daily stays fresh; one never accessed fades.
- **Compounding**: the retrieval → gate-pass → reinforcement loop is superlinear.
- **Observable**: balance is a first-class field — visible in TUI, queryable via API.
- **Economically grounded**: Gesell's insight is that idle value is a social cost; same applies to idle knowledge.

### Tiers

```rust
pub enum Tier {
    Transient,     // 0.1× multiplier — decays 10× faster
    Working,       // 0.5× — decays 2× faster
    Consolidated,  // 1.0× — base rate
    Persistent,    // 5.0× — decays 5× slower
}
```

Progression: Transient → Working (3+ gate passes) → Consolidated (5+ across distinct contexts) → Persistent (consortium approval or freeze).

### Cold threshold

When balance drops below `COLD_THRESHOLD` (default 0.01), the Signal enters cold storage. Body moves to slower storage; content hash stays valid; lineage preserved. **Thaw** restores balance to a starter value and is itself a Bus event (`knowledge.thawed`). Frozen Signals skip demurrage entirely — they are bedrock knowledge.

---

## 7. Content Addressing

Signals are content-addressed via SHA-256:

```rust
impl Signal {
    pub fn compute_hash(payload: &Value) -> ContentHash {
        let canonical = serde_json::to_vec(payload).expect("serializable");
        ContentHash(sha2::Sha256::digest(&canonical).into())
    }
}
```

Enables: deduplication, integrity verification, lineage chain validation, semantic caching (5× cost reduction via content-addressed reuse across Flows), and on-chain commitments (hash on-chain, content off-chain).

---

## 8. Lineage and Provenance

### Lineage (structural)

```rust
pub source: Vec<SignalRef>  // upstream Signals
```

Walking `source[]` recursively produces a DAG. Enables `roko replay <hash>`, artifact queries ("what produced this?"), blame attribution ("which upstream caused this failure?").

### Provenance (metadata)

```rust
pub struct Provenance {
    pub source_files: Vec<SourceFileRange>,       // file, line range, commit
    pub generation: Option<GenerationProvenance>, // model, prompt hash, temperature, seed
    pub web_fetch: Option<WebFetchProvenance>,    // URL, timestamp, HTTP status
    pub citations: Vec<Citation>,                 // claimed sources
}
```

---

## 9. HDC Fingerprint

Every Signal carries a 10,240-bit binary HDC vector (Kanerva 2009) for similarity search and cross-domain pattern discovery.

### Encoding

Structured information enters a single vector through role-filler binding:

```rust
pub fn encode_signal(signal: &Signal) -> HdcVector {
    let pairs = vec![
        ("kind", signal.kind.to_string()),
        ("tags", signal.tags.join(",")),
        ("author", signal.author.to_string()),
        // ... kind-specific fields
    ];
    HdcVector::encode_structured(&pairs)
}
```

Deterministic across deployments via BLAKE3-seeded `WordMemory`. Encoder version tracked to prevent drift.

### Operations

| Operation | What | Cost | Reference |
|---|---|---|---|
| **Bind** (XOR) | Role-filler binding | O(n) | Rachkovskij 2001 |
| **Bundle** (majority) | Consensus: similar to all inputs | O(n×k) | Kanerva 2009 |
| **Permute** (rotation) | Positional encoding | O(n) | Plate 2003 |
| **Similarity** (Hamming) | Overlap via POPCNT | <1 μs | Hardware |
| **Resonate** | Factorize: recover constituents | O(n×k×iter) | Frady et al. 2020 |

### Cross-domain resonance

When Signals from different domains have similar HDC fingerprints, they share structural properties despite surface differences. A retry pattern from networking might apply to database operations. Retrieval gives cross-domain matches a **15% bonus** (additive when domains differ).

### Why HDC instead of float embeddings

| Property | HDC (10,240-bit binary) | Float (1536-d float32) |
|---|---|---|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity cost | XOR + POPCNT (~1 ns) | Dot product (hundreds FLOPs) |
| Compositionality | Native (bind/bundle/permute/resonate) | Requires learned operations |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder |
| Determinism | Identical seeds → identical vectors | Depends on model version |

At 10,240 bits, **800K fingerprints fit in 1 GB RAM**; brute-force SIMD comparison is **<1 ms** for the full set. No external vector store needed. (Cf. Levy & Gayler 2008 for VSA survey; Olshausen & Field 1996 for biological precedent.)

---

## 10. Bus — Ephemeral Transport

The **Bus** is the ephemeral transport fabric — a kernel-level pub/sub system alongside Store. Every real-time behavior runs through Bus: heartbeats, event streaming, learning feedback (predict-publish-correct), pheromone sensing, coordination.

```rust
#[async_trait]
pub trait Bus: Send + Sync {
    async fn publish(&self, pulse: Pulse) -> Result<u64>;
    fn subscribe(&self, filter: TopicFilter) -> PulseStream;
    async fn replay_since(&self, since: u64, filter: &TopicFilter) -> Result<Vec<Pulse>>;
    async fn current_seq(&self) -> Result<u64>;
    fn ring_capacity(&self) -> usize;
}

pub enum TopicFilter {
    Exact(Topic),
    Glob(String),           // e.g., "agent:*:heartbeat"
    AnyOf(Vec<Topic>),
    All,
    And(Box<TopicFilter>, Box<TopicFilter>),
    Or(Box<TopicFilter>, Box<TopicFilter>),
    Not(Box<TopicFilter>),
}
```

Bus is **broadcast**: every subscriber sees every matching Pulse. No queuing or redelivery — subscribers that fall behind lose Pulses. For critical data, graduate the Pulse to a Signal and persist via Store.

### Backpressure

| Strategy | Used for | Behavior |
|---|---|---|
| Coalesce | Heartbeats | Buffer, send latest per interval |
| Drop-oldest | Streaming output | Ring buffer, slow consumers miss old |
| Lossless | Gate results | Queue with flow control |
| Sample | Feed data | Every Nth update |

### Backends

| Backend | Scope | Status |
|---|---|---|
| `BroadcastBus` (`tokio::sync::broadcast`) | In-process | Ships immediately |
| `MemoryBus` | Testing | Ships immediately |
| `NatsBus` / `KafkaBus` | Multi-process | Phase 2 |
| `ChainBus` | On-chain events | Phase 2+ |

The `BroadcastBus` replaces the current `EventBus<E>` in `roko-runtime`. The Bus trait moves to `roko-core` (L0 kernel); implementations live in `roko-std`.

### Why Bus is kernel-level

The event bus already existed but was architecturally invisible — no trait, no doc chapter, not mentioned in the five-layer taxonomy. This caused the `roko-conductor → roko-learn` layer violation (both needed to share gate verdict events, creating a compile-time cross-layer dependency). With Bus as L0, both subsystems subscribe to `gate.verdict.emitted` independently — no compile-time coupling.

---

## 11. Store — Persisted Storage

```rust
pub trait Store: Cell {
    async fn put(&self, signal: Signal) -> Result<SignalRef>;
    async fn get(&self, id: &SignalId) -> Result<Option<Signal>>;
    async fn query(&self, query: StoreQuery) -> Result<Vec<Signal>>;
    async fn query_similar(
        &self, fp: &HdcVector, radius: f32, limit: usize,
    ) -> Result<Vec<(SignalRef, f32)>>;
    async fn prune(&self, threshold: f64) -> Result<PruneReport>;
}
```

`query_similar` is native HDC similarity over stored Signals. No external vector store. At 10,240 bits and 800K entries, brute-force SIMD is <1 ms.

### Storage layout

```
.roko/
├── signals.jsonl          # primary Signal log (append-only)
├── neuro/
│   └── knowledge.jsonl    # knowledge Signals (demurrage, tiers)
├── episodes.jsonl         # episode Signals
├── runs/<run-id>/
│   ├── artifacts/
│   └── events.jsonl       # graduated Pulse snapshots
└── learn/
    ├── reflexes.jsonl     # promoted T0 reflex Signals
    └── efficiency.jsonl
```

---

## 12. AntiKnowledge

When a previously trusted Signal is proven wrong, an **AntiKnowledge** Signal actively repels future Signals in the same HDC region. Popper's falsificationism applied to learned rules.

```rust
const ANTI_KNOWLEDGE_WARN_THRESHOLD: f64 = 0.5;      // log warning
const ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD: f64 = 0.7;  // halve initial balance
const ANTI_KNOWLEDGE_REJECT_THRESHOLD: f64 = 0.9;    // reject outright
```

AntiKnowledge itself decays via demurrage (30-day effective rate). Old mistakes eventually stop blocking new discoveries.

---

## 13. Signal Lifecycle

```
Created (by Cell or external source)
    │
    ├── Pulse path ──► Bus topic ──► consumed by subscribers
    │                                    │
    │                       graduate() if graduation policy says yes
    │                                    ▼
    └── Signal path ──► Store.put() ──► scored ──► routed ──► composed
                             │
                             ├── retrieved ──► balance ↑ (reinforcement)
                             ├── gate passed ──► balance ↑, tier ↑
                             ├── challenged ──► balance ↓, tier ↓
                             ├── demurrage ──► balance ↓ over time
                             ├── cold ──► balance < 0.01 ──► archive
                             └── frozen ──► permanent (skip demurrage)
```

---

## 14. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Signal struct compiles with `balance`, `demurrage_paid`, `last_touched_at` | Compile check |
| Pulse struct compiles with `seq`, `topic`, `kind`, `body`, `source`, `lineage_hint` | Compile check |
| Content hash deterministic: same payload → same hash | Unit test |
| Demurrage: balance decreases over time, increases on reinforcement | Unit test with mock clock |
| Novelty-weighted reinforcement: rare Signals get larger bonus | Unit test (two Signals at different HDC distances) |
| Tier progression: 3 gate-passes promote Transient → Working | Integration test |
| AntiKnowledge: >0.9 HDC similarity rejected | Unit test |
| Bus: Pulse published to topic received by subscriber | Integration test |
| Bus replay: reconnecting subscriber receives missed Pulses within ring capacity | Integration test |
| Graduation: `Pulse.graduate()` produces valid Signal with provenance | Unit test |
| Projection: `Signal.to_pulse()` produces valid Pulse | Unit test |
| Store round-trip: put + get returns identical Signal | Integration test |
| `Store.query_similar`: returns Signals ranked by HDC similarity | Integration test |
| Cold threshold: balance < 0.01 triggers archive | Unit test |
| Heuristic kind: `when` + `then` + `falsifier` + `calibration` fields present | Compile check |
| Lineage walk: `source[]` recursion produces correct DAG | Integration test |
| HDC fingerprint determinism: same inputs → same fingerprint | Unit test |
| Bus `TopicFilter::Glob` matches expected topics | Unit test |
| Graduation policy: `gate.verdict.emitted` graduates, `heartbeat.tick` does not | Integration test |
