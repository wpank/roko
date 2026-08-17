# 06 — Memory and Knowledge

> Current local memory is `roko_neuro::KnowledgeStore`, implementing the
> `NeuroStore` lifecycle over durable `KnowledgeEntry` records. A unified
> Store-protocol `MemoryCell`, universal Signal storage, and distributed memory
> are target architecture.

**Current distinction and target unification**: `KnowledgeEntry` is the current
`roko-neuro` persistence record and remains distinct from core `Engram`/`Signal`.
The target vocabulary unifies their lifecycle concepts and models Pheromone as
Pulse; InsightStore, PheromoneRegistry, resonator networks, and the full temporal
knowledge graph remain target surfaces.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality, demurrage, Kind system, HDC fingerprints), [02-CELL](02-CELL.md) (Store protocol, Verify redesign, Score protocol, React protocol), [03-GRAPH](03-GRAPH.md) (Graph, Loop), [05-AGENT](05-AGENT.md) (CognitiveWorkspace, somatic markers, 9-step pipeline)

> **Implementation status (2026-08): PARTIALLY SHIPPED.** The local, file-backed
> runtime lifecycle described below is implemented; the complete Signal/Cell and
> distributed protocol remains roadmap.
>
> **Shipped now:** `roko-neuro::KnowledgeStore` persists typed `KnowledgeEntry`
> records (including Heuristic and AntiKnowledge), confidence decay, fixed-per-sweep
> balance demurrage, fixed reinforcement (`0.05/0.10/0.15/0.08/0.12` for
> Retrieved/Cited/Gated/Surprised/AgentQuoted), seven-day depleted-balance freezing,
> configurable four-tier promotion/demotion, falsifier survival/immunity/discrediting,
> optional Allen-interval indexing, and feature-gated HDC role/filler queries and
> cross-domain resonance detection. Runtime gate completions distill passed patterns
> as Heuristics and failures as AntiKnowledge, deduplicate/reinforce prior entries,
> inject the top three matches into later prompts, and run progression/demurrage at
> successful plan completion. Idle dream cycles now merge groups of three or more
> same-kind entries sharing at least two tags, freeze their sources, promote the
> merged entry by one tier, then run decay, demurrage, and freeze-before-delete GC.
>
> **Still roadmap:** a `MemoryCell` implementing the unified Store protocol over
> canonical `Signal`/`Kind`, the full Gesell-Shannon ODE and per-Kind rate table,
> retrieval-count novelty accounting, validator-quorum freeze/challenge and Bus
> events, Worldview/co-citation and resonator networks, durable temporal validity
> epochs, distributed/on-chain synchronization, and the HTTP/consortium interfaces.
> Falsifiers are attached automatically by the runtime distiller and dream merger,
> but are not yet enforced for every externally constructed Heuristic.

### Current implementation sources and reading contract

| Current surface | Authoritative source |
|---|---|
| Knowledge kinds, tiers, entries, and `NeuroStore` | `crates/roko-neuro/src/lib.rs` |
| Durable store, decay, demurrage, reinforcement, and lifecycle | `crates/roko-neuro/src/knowledge_store.rs` |
| Optional epochs, millisecond intervals, and Allen relations | `crates/roko-neuro/src/temporal.rs` |
| Idle dream-cycle orchestration and reports | `crates/roko-dreams/src/cycle.rs` |
| Runtime distillation and prompt retrieval | `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs` |

Unless a section is explicitly labeled **Current implementation**, its Rust,
Solidity, TOML, endpoint, formula, and acceptance snippets are normative target
design. They preserve the research rationale; they do not define a second shipped
API beside the sources above.

---

## 1. The Knowledge Problem

Agent frameworks treat memory as a bag of text chunks. Append to a vector store, retrieve by cosine similarity, stuff into the next prompt. Nothing decays. Nothing consolidates. Nothing gets shared across agents.

Four consequences compound over time:

1. **Noise floor rises.** Without temporal pressure, stale information dilutes retrieval. A cached heuristic from session #12, invalidated by session #47, still appears in results.
2. **No compression.** Five episodes demonstrating the same pattern remain five separate chunks. No process distills raw episodes into compact, reusable knowledge.
3. **No quality signal.** A hallucinated claim and a gate-validated insight sit at the same confidence level. Without provenance tracking, unreliable knowledge contaminates reliable knowledge.
4. **No sharing.** A thousand agents solving related problems independently discover the same patterns. The hundredth agent to learn "run clippy before committing Rust code" pays the same discovery cost as the first.

The current store treats knowledge as a living substrate instead of a dead archive:
entries decay, receive usage reinforcement, progress through tiers, and can be
consolidated by dream cycles. Peer validation and network-wide stigmergic flow are
target extensions.

---

## 2. Memory as Specialization

### Current implementation

`KnowledgeStore` implements `NeuroStore`, whose storage lifecycle is
`init / query / query_similar / ingest / decay / gc`, with confidence and usage
updates. `KnowledgeEntry` supports the six current `KnowledgeKind` variants
(`Insight`, `Heuristic`, `AntiKnowledge`, `Warning`, `CausalLink`, and
`StrategyFragment`) and four `KnowledgeTier` variants (`Transient`, `Working`,
`Consolidated`, and `Persistent`).

The current local store manages this lifecycle:

1. **Ingest** -- New knowledge entries enter the durable local store.
2. **Retrieve** -- HDC similarity search + multi-dimensional scoring.
3. **Demurrage** -- Balance decays unless actively used (retrieved, cited, gate-passed, surprised).
4. **Promote/Demote** -- Based on gate validation results and balance thresholds.
5. **Consolidate** -- Dream cycles compress episodes into durable knowledge (section 9).
6. **Garbage collect** -- Low-confidence records are removed after the store's
   freeze-before-delete checks.

### Target specialization

The unified architecture models Memory as a Store-protocol Cell over canonical
Signals, with demurrage, tier progression, dream consolidation, and HDC retrieval.
That `MemoryCell` and its proposed `put / get / query / query_similar / prune`
surface are not the current `NeuroStore` API.

---

## 3. Demurrage Model

**Demurrage replaces Ebbinghaus decay** (Gesell 1916). Instead of passive time-based forgetting, Signals pay a holding cost for occupying store space. Active use -- retrieval, citation, gate-pass, surprise -- restores balance. This is economic selection pressure on knowledge: unique, actively-useful insights stay warm; redundant or stale entries fade.

### Current balance state

Canonical `Engram` carries balance and cumulative demurrage state. The local
knowledge runtime separately persists this subset on `KnowledgeEntry`:

```rust
pub struct KnowledgeEntry {
    // ...
    pub balance: f64,
    pub frozen: bool,
    pub balance_depleted_at: Option<DateTime<Utc>>,
    pub frozen_at: Option<DateTime<Utc>>,
    // ...
}
```

### Current runtime balance model

`KnowledgeEntry::apply_demurrage(elapsed_hours)` deducts
`DEMURRAGE_RATE_PER_HOUR * elapsed_hours`. `KnowledgeStore::demurrage(tax_rate)`
also supports a fixed sweep: the first depletion halves `half_life_days`, and an
entry left depleted for more than seven days is frozen. The current
`ReinforcementSignal` variants are `Retrieved`, `Cited`, `Gated`, `Surprised`, and
`AgentQuoted`, with respective base values `0.05`, `0.10`, `0.15`, `0.08`, and
`0.12`. This is not the complete ODE below.

### Target rate law (Gesell-Shannon)

The demurrage ODE combines a flat tax with an exponential decay term:

```
dB/dt = -r - beta * B(t)
```

Where:
- `B(t)` is the balance at time t
- `r` is the flat tax per day (constant drain regardless of current balance)
- `beta` is the exponential decay rate per day (proportional drain)

The discrete update applied each demurrage interval:

```
balance(t+dt) = balance(t) - r*dt - beta*balance(t)*dt
```

This ODE has the closed-form solution:

```
B(t) = (B_0 + r/beta) * exp(-beta * t) - r/beta
```

The balance reaches zero at time:

```
t_zero = -ln(r / (beta * B_0 + r)) / beta
```

For an Insight with `r = 0.01`, `beta = 0.02`, `B_0 = 1.0`:
- At t=0: B = 1.0
- At t=7d: B ~ 0.82
- At t=30d: B ~ 0.38
- At t=60d: B ~ 0.09 (approaching cold threshold)

The flat tax `r` ensures that even Signals with very low balance continue to lose value (preventing "zombie" Signals that hover near zero indefinitely). The exponential term `beta*B(t)` makes high-balance Signals pay more, creating progressive taxation that prevents knowledge hoarding.

### Target reinforcement policy

Active use restores balance. Each reinforcement kind has a different weight, and reinforcement is novelty-weighted (anti-hoarding mechanism):

```rust
pub enum ReinforcementSignal {
    Retrieved,      // returned in a query
    Cited,          // in another Signal's source[] lineage
    Gated,          // in context pack when gate passed
    Surprised,      // high prediction error in context (Shannon surprise as economic bonus)
    AgentQuoted,    // agent referenced in output
}
```

| Reinforcement Kind | Balance Restored | Condition |
|---|---|---|
| **Retrieved** | `+0.05 * novelty` | Signal was returned in a Memory query |
| **Cited** | `+0.10 * novelty` | Signal included in composed context that passed a gate |
| **GatePassed** | `+0.15 * novelty` | Signal in the context pack of a successful gate evaluation |
| **Surprised** | `+0.20 * novelty` | Signal relevant to a high-PE observation (PE > 0.40) |
| **AgentQuoted** | `+0.08 * novelty` | Signal's content was directly referenced in an agent's output |

### Target novelty-weighted reinforcement

```
novelty = 1 / (1 + ln(retrieval_count))
```

The first retrieval restores full balance (`novelty = 1.0`). The 10th restores ~0.30x. The 100th restores ~0.18x. This prevents a popular-but-mediocre Signal from staying warm purely through high retrieval frequency. Genuinely novel use (new context, new agent, new domain) resets the retrieval counter.

### Target per-Kind default rates (canonical)

> **These are the canonical demurrage constants for the entire specification.** All other documents (including [07-LEARNING](07-LEARNING.md)) MUST reference this table rather than redeclare values. Any discrepancy is a bug in the referencing document.

| Kind | Flat tax (r) | Exp decay (beta) | Effective lifetime (no reinforcement) |
|---|---|---|---|
| Core data (Text, Code) | 0.001 | 0.001 | ~1000 days |
| Insight | 0.01 | 0.02 | ~30 days |
| Heuristic | 0.005 | 0.01 | ~90 days |
| Warning | 0.10 | 0.20 | ~1 hour |
| CausalLink | 0.007 | 0.017 | ~60 days |
| StrategyFragment | 0.02 | 0.03 | ~14 days |
| AntiKnowledge | 0.01 | 0.02 | ~30 days |
| Episode | 0.005 | 0.01 | ~90 days |
| Worldview | 0.005 | 0.01 | ~90 days (half standard; see SS6) |

### Target cold-storage protocol

When balance drops below `COLD_THRESHOLD` (default 0.05), the Signal enters cold storage. Body moves to slower storage; content hash stays valid; lineage preserved. **Thaw** restores balance to a starter value and is itself a Bus event (`knowledge.thawed`).

### Current freezing and target quorum lifecycle

Current `KnowledgeEntry` records have `frozen`, `frozen_at`, and
`balance_depleted_at`; `freeze()` and `thaw(starter_balance)` update that local
state, and fixed demurrage skips frozen entries. The validator/quorum/evidence,
challenge, CLI, Bus, and consortium lifecycle below is target design.

```rust
pub struct FreezeState {
    pub is_frozen: bool,
    pub frozen_at: Option<DateTime<Utc>>,
    pub frozen_by: Vec<AgentId>,       // 3+ required
    pub freeze_evidence: Vec<SignalRef>, // gate-pass receipts or validator attestations
    pub thaw_reason: Option<String>,
}
```

**Freeze criteria** -- A Signal can be frozen when ALL of the following hold:

1. **Consensus**: 3+ validators from distinct contexts (different agents, different episodes) have independently confirmed the Signal. Validator attestations are stored as `freeze_evidence`.
2. **Tier**: The Signal is at Consolidated or Persistent tier. Transient and Working Signals cannot be frozen (they lack sufficient validation history).
3. **Calibration**: For Heuristic Signals, the calibration score must be >= 0.80 with >= 20 receipts.

**Freeze triggers**:

| Trigger | Who | When |
|---|---|---|
| Automatic | Dream Integration (Phase 4) | D2 distillation produces a Heuristic with 3+ independent validators |
| Manual | Human operator | `roko knowledge freeze <signal-id>` CLI command |
| Consortium | Validator quorum | 3+ validators call `IInsightStore.freeze(entryId)` on-chain |

**Thaw protocol** -- Frozen Signals can be thawed (unfrozen) through the challenge flow:

1. **Challenge**: Any agent or human can challenge a frozen Signal by submitting evidence via `POST /api/knowledge/challenge/:id` or `IInsightStore.challenge(entryId, reason)`.
2. **Review window**: The challenge opens a 72-hour review window. During this window, the Signal remains frozen but is marked `under_review`.
3. **Quorum vote**: 3+ validators (distinct from the original freezers) must vote. Majority rules:
   - **Challenge upheld** (majority agrees Signal is wrong): Signal is thawed, demoted to Working tier, balance reset to 0.5. An AntiKnowledge Signal MAY be created if the refutation is strong.
   - **Challenge rejected** (majority disagrees): Signal stays frozen. The challenger's reputation takes a small hit (TraceRank -0.01) to discourage frivolous challenges.
4. **Automatic thaw**: If 3+ consecutive gate evaluations produce failures when this Signal is in the context pack, the Signal is automatically thawed and demoted to Working tier. This handles the case where the environment has changed and the frozen knowledge is no longer valid.

```rust
pub enum ThawTrigger {
    ChallengeUpheld { challengers: Vec<AgentId>, votes: u32 },
    ConsecutiveGateFailures { count: u32, episodes: Vec<SignalRef> },
    ManualOverride { operator: AgentId, reason: String },
}
```

**Bus events**: Freeze and thaw operations publish Pulses on Bus topic `knowledge.frozen` and `knowledge.thawed` respectively. The Pulse payload includes the Signal hash, trigger, and new state.

### Ebbinghaus as special case

Pure time-based decay (Ebbinghaus) is recovered when no interactions occur: a Signal that is never retrieved, cited, or gate-passed decays by the demurrage rate. Demurrage generalizes Ebbinghaus by adding an economic mechanism -- use restores value.

---

## 4. Four-Tier System

### Current implementation

`KnowledgeEntry` progresses through four `KnowledgeTier` values. The multiplier
scales effective half-life, so lower values decay faster:

```rust
pub enum KnowledgeTier {
    Transient,     // 0.1x multiplier -- decays 10x faster
    Working,       // 0.5x -- decays 2x faster
    Consolidated,  // 1.0x -- base rate
    Persistent,    // 5.0x -- decays 5x slower
}
```

Default `TierProgressionConfig` uses these thresholds:

| From | To | Requirement |
|---|---|---|
| Transient | Working | `confirmation_count >= 2` and confidence `>= 0.5` |
| Working | Consolidated | at least 3 distinct contexts and confidence `>= 0.6` |
| Consolidated | Persistent | age `>= 30` days and confidence `>= 0.8` |

With demotion enabled, current defaults are:

| From | To | Requirement |
|---|---|---|
| Persistent | Consolidated | entry is explicitly deprecated |
| Consolidated | Working | confidence `< 0.3` |
| Working | Transient | confidence `< 0.15` |

The gate-count, validator-quorum, challenge, and consortium progression rules
elsewhere in this chapter are target policy layered above this current local
configuration.

---

## 5. Heuristic as First-Class Kind

A **Heuristic** is a first-class Signal kind with structured `when/then` clauses, a mandatory falsifier, and a calibration track record grounded in episode outcomes (not LLM self-report). Heuristics are the system's actionable knowledge -- behavioral rules that predict outcomes.

```rust
pub struct HeuristicPayload {
    // -- Rule --------------------------------------------------------
    pub when: Vec<Predicate>,            // preconditions (matchable)
    pub then: String,                    // action or prediction
    pub confidence: f64,                 // 0.0..=1.0

    // -- Falsifier (mandatory) ---------------------------------------
    pub falsifier: String,               // "what would prove this wrong?"
    pub falsifier_checked_count: u64,
    pub falsifier_triggered_count: u64,

    // -- Calibration -------------------------------------------------
    pub calibration: CalibrationRecord,
    pub receipts: Vec<CalibrationReceipt>,

    // -- Lineage -----------------------------------------------------
    pub source_episodes: Vec<SignalRef>,
    pub parent_heuristic: Option<SignalRef>,
    pub children: Vec<SignalRef>,
}
```

### Predicate enum

```rust
pub enum Predicate {
    DomainIs(String),                    // "rust", "typescript", "trading"
    TaskContains(String),                // substring match
    FilePathGlob(String),               // "*.rs", "src/api/**"
    RegimeIs(Regime),                    // Calm, Normal, Volatile, Crisis
    VitalityAbove(f64),                  // vitality > threshold
    VitalityBelow(f64),                  // vitality < threshold
    ModelIs(String),                     // specific model
    TagPresent(String),                  // tag exists on task
    Custom(String),                      // freeform (LLM-evaluated)
}
```

### Calibration

```rust
pub struct CalibrationRecord {
    pub predictions: u64,
    pub correct: u64,
    pub score: f64,                      // correct / predictions (running)
    pub brier_score: f64,               // Brier score (lower = better calibrated)
    pub confidence_interval: (f64, f64), // Wilson score CI
    pub last_calibrated_at: DateTime<Utc>,
}

pub struct CalibrationReceipt {
    pub episode_ref: SignalRef,
    pub predicted: bool,
    pub actual: bool,
    pub timestamp: DateTime<Utc>,
}
```

The Brier score (Brier 1950) measures calibration quality: `mean((predicted_prob - actual)^2)`. A well-calibrated heuristic has a Brier score close to 0.0. The Wilson score confidence interval provides a range that accounts for sample size -- a heuristic with 5 receipts has a wide CI; one with 500 has a narrow CI.

### Mandatory falsifier

Every Heuristic MUST carry a falsifier -- a concrete condition under which the heuristic should be considered wrong. This is Popper's falsificationism applied to learned rules. A heuristic without a falsifier cannot be created.

The falsifier serves two purposes:

1. **Epistemic hygiene**: Forces the system to articulate *how* the heuristic could be wrong, preventing unfalsifiable belief accumulation.
2. **Automatic retirement**: When the falsifier fires enough times (calibration score drops below threshold), the heuristic is automatically retired or refined.

### Heuristic lifecycle

```
Birth --> Test --> Calibrate --> Retire/Evolve
```

**Birth**: Heuristics are born from L3 dream consolidation (section 9): when 5+ confirmed Insights cluster around the same when/then pattern, the distillation stage produces a Heuristic Signal with an auto-generated falsifier. Heuristics can also be created manually via `roko knowledge heuristic create`.

**Test**: Every time the Heuristic's `when` predicates match the current context and the Heuristic is included in the CognitiveWorkspace context pack ([05-AGENT](05-AGENT.md) SS16), the system records whether the `then` prediction was correct. The gate verdict determines correctness.

**Calibrate**: The CalibrationRecord updates on each receipt. Calibration score affects the heuristic's bid in the CognitiveWorkspace VCG auction: poorly calibrated heuristics lose prompt space to better-calibrated ones. This is the **heuristic calibration loop**.

**Retire/Evolve**: When a heuristic's falsifier fires above the retirement threshold, violations spawn refined children with narrower when-clauses (cf. Quinlan ID3 for decision tree refinement on a live stream). A heuristic "When refactoring code, run clippy" that fails for JavaScript files might spawn: "When refactoring Rust code, run clippy" and "When refactoring TypeScript code, run eslint." Children carry a `parent_heuristic` reference for lineage tracking.

---

## 6. Target: Worldviews

The target architecture has **Worldviews** emerge from co-citation clusters of
heuristics with high calibration scores. The current `KnowledgeKind` enum has no
`Worldview` variant, and `KnowledgeStore` does not yet persist this representation.

### How worldviews form

When multiple heuristics are frequently cited together in successful gate evaluations, they form a co-citation cluster. The system identifies these clusters during L3 dream consolidation:

```
Heuristic A: "When building APIs, use typed schemas"
Heuristic B: "When deploying, run integration tests"
Heuristic C: "When refactoring, maintain backward compatibility"

Co-citation frequency: A+B (47 times), A+C (38 times), B+C (42 times)
All three: 34 times

-> These three form a worldview: "careful API engineering"
```

### Worldview storage

Worldviews are stored as **Signals with `Kind::Worldview`** at Consolidated tier. They follow the standard Store protocol -- they are content-addressed, lineage-tracked, and HDC-fingerprinted like any other Signal.

**Demurrage**: Worldviews decay at **half the standard rate** for their tier (flat tax `r = 0.005`, exp decay `beta = 0.01`, giving ~90-day effective lifetime without reinforcement). This slower decay reflects their role as aggregate knowledge -- worldviews are more durable than individual insights because they represent validated patterns across many heuristics.

**Rival worldviews**: Each Worldview Signal carries a `rival_worldviews: Vec<SignalRef>` field containing references to competing worldviews for the same domain. During the RETRIEVE step (SS12), the 15% mandatory contrarian retrieval slot is populated from the highest-calibration rival worldview. This ensures that the agent always considers at least one alternative perspective.

### Worldview representation

```rust
pub struct Worldview {
    pub id: SignalId,
    pub name: String,
    pub heuristics: Vec<SignalRef>,
    pub co_citation_matrix: BTreeMap<(SignalRef, SignalRef), u64>,
    pub avg_calibration: f64,
    pub domain: String,
    /// References to competing worldviews. The highest-calibration rival
    /// is served to the 15% contrarian retrieval slot during RETRIEVE.
    pub rival_worldviews: Vec<SignalRef>,
}

/// Worldviews are stored as Signals:
///   kind: Kind::Worldview
///   tier: Consolidated (default)
///   demurrage: half standard rate (r=0.005, beta=0.01)
///   body: serde_json::to_value(Worldview)
```

### Multiple worldviews deliberately

The system maintains multiple worldviews for each domain to prevent cognitive monoculture:

| Role | Purpose | Example |
|---|---|---|
| **Main** | Highest avg calibration, used by default | "careful API engineering" |
| **Challenger** | Second-highest, used for 15% contrarian retrieval | "move-fast API engineering" |
| **Niche specialists** | High calibration for specific sub-domains | "high-throughput streaming APIs" |

The 15% mandatory contrarian retrieval from somatic markers ([05-AGENT](05-AGENT.md) SS13) naturally consults challenger worldviews, preventing the dominant worldview from becoming an unchallenged orthodoxy.

### Worldview swap mechanism

When a challenger worldview's avg calibration exceeds the main worldview's for 20+ consecutive evaluations, they swap roles. The former main becomes the new challenger. This mechanism allows the system's collective beliefs to shift in response to changing environments without catastrophic forgetting.

---

## 7. AntiKnowledge

When the system discovers that a previously trusted Signal is wrong, it creates an **AntiKnowledge** Signal that actively repels future Signals in the same HDC region. Popper's falsificationism applied to learned rules.

### Repulsion thresholds

```rust
const ANTI_KNOWLEDGE_WARN_THRESHOLD: f64 = 0.5;
const ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD: f64 = 0.7;
const ANTI_KNOWLEDGE_REJECT_THRESHOLD: f64 = 0.9;
const ANTI_KNOWLEDGE_DISCOUNT_FACTOR: f64 = 0.5;
```

When a new Signal arrives whose HDC vector is similar to an existing AntiKnowledge Signal:

| Similarity | Action |
|---|---|
| Above 0.5 | Log a warning: "New entry resembles known anti-knowledge" |
| Above 0.7 | Halve the new Signal's initial balance (discount factor 0.5) |
| Above 0.9 | Reject the Signal outright -- it is not stored |

### AntiKnowledge lifecycle

1. **Creation**: A knowledge Signal is refuted through the challenge flow (3+ challenges, consortium review upholds).
2. **Conversion**: The refuted Signal's Kind is changed to `AntiKnowledge`. Content preserved; role inverts.
3. **Demurrage**: AntiKnowledge decays via demurrage like other Signals (~30-day effective rate at Consolidated tier). Old mistakes eventually stop blocking new discoveries.
4. **Override**: If overwhelming evidence contradicts an AntiKnowledge entry, the challenge flow can convert it back or archive it.

---

## 8. HDC Operations and Target Resonator Networks

Feature-gated HDC encoding and similarity retrieval are current. The resonator
network types and factorization policy below remain target design.

The knowledge system encodes structured information as 10,240-bit binary vectors (Kanerva 2009). No floating point. No matrix multiply. No GPU.

### The vector

```rust
pub struct HdcVector {
    bits: [u64; 160],  // 160 words * 64 bits = 10,240 bits = 1,280 bytes
}
```

Implementation lives in `crates/roko-primitives/src/hdc.rs`.

### Core operations

| Operation | What | Cost | Reference |
|---|---|---|---|
| **Bind** (XOR) | Role-filler binding. Dissimilar to both inputs. Self-inverse. | O(n) | Rachkovskij 2001 |
| **Bundle** (majority vote) | Aggregation. Similar to all inputs. | O(n*k) | Kanerva 2009 |
| **Permute** (bit rotation) | Positional encoding. Cyclic left shift. | O(n) | Plate 2003 |
| **Similarity** (Hamming) | Overlap via XOR + POPCNT. | O(vector words); benchmark per platform | Hardware |
| **Resonate** | Factorize: recover constituents from bundle. | O(n*k*iter) | Frady et al. 2020 |

### HDC algebra

The operations compose to encode structured information:

**Role-filler binding**: To encode "language=Rust", bind the role vector with the filler vector: `bind(V_language, V_Rust)`. The result is dissimilar to both inputs but can recover either given the other (since XOR is self-inverse).

**Bundling for sets**: To encode multiple role-fillers, bundle them: `bundle(bind(V_language, V_Rust), bind(V_domain, V_systems))`. The result is similar to all inputs.

**Positional encoding**: To encode sequences, permute by position: `bundle(permute(V_word1, 0), permute(V_word2, 1), permute(V_word3, 2))`.

### Target: Resonator Networks

**Resonator Networks** factorize bundled HDC vectors to recover their constituent parts (Frady et al. 2020). Given a bundle `B = bundle(bind(R1,F1), bind(R2,F2), bind(R3,F3))`, recover the original role-filler pairs.

```rust
pub struct ResonatorNetwork {
    pub codebooks: BTreeMap<String, Vec<HdcVector>>,
    pub max_iterations: usize,
    pub similarity_threshold: f32,
}

impl ResonatorNetwork {
    pub fn factorize(
        &self,
        bundle: &HdcVector,
        roles: &[String],
    ) -> Vec<(String, HdcVector, f32)>;  // (role, filler_estimate, confidence)
}
```

### Why Resonator Networks matter

1. **Knowledge deduplication**: Factorization reveals whether two similar bundles encode the same structured content (true duplicates) or merely similar content.
2. **Constituent extraction**: A complex episode fingerprint can be factorized to identify which tool sequences, error patterns, or domain contexts contributed.
3. **Cross-domain transfer**: Factorization reveals shared sub-structure across domains. An "API retry pattern" in networking and a "retry pattern" in database operations share the same abstract structure.
4. **HDC cleanup**: During L3 consolidation (section 9), Resonator Networks factorize to identify patterns learned independently at higher tiers. Redundant bundles are pruned.

### Unverified performance targets

These are research goals, not measured guarantees in the current repository.
Acceptance requires a reproducible benchmark on named hardware and corpus layout.

| Operation | Target | Notes |
|---|---|---|
| Encode one vector | Benchmark required | Seed + splitmix64 expansion |
| Similarity (two vectors) | Benchmark required | XOR + POPCNT, cache-friendly |
| Search 1K patterns | Benchmark required | Brute-force, no index needed |
| Search 10K patterns | Benchmark required | Corpus layout and hardware dominate |
| Resonator factorization (3 roles) | Target benchmark required | Target type; iteration count is design-dependent |

### Why HDC instead of float embeddings

| Property | HDC (10,240-bit binary) | Float (1536-d float32) |
|---|---|---|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity cost | XOR + POPCNT; platform-dependent | Dot product; platform-dependent |
| Compositionality | Native (bind/bundle/permute/resonate) | Requires learned operations |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder |
| Determinism | Identical seeds produce identical vectors | Depends on model version |

At 10,240 bits, raw fingerprint storage is 1,280 bytes per vector before
container/index overhead. Brute-force search latency must be measured for the
deployment corpus and hardware. (Cf. Levy & Gayler 2008 for VSA survey;
Olshausen & Field 1996 for biological precedent.)

---

## 9. Dream Consolidation

Dream consolidation is the offline process where Agents compress raw episodes into durable knowledge. It runs when an Agent accumulates enough unprocessed experience -- "sleep pressure." Dream consolidation is a **Loop** specialization: a Graph that feeds output back to input on the delta timescale.

### Four phases

```rust
pub enum DreamPhase {
    NremReplay,            // priority replay of high-surprise episodes
    HindsightRelabeling,   // relabel failed trajectories for achieved sub-goals
    RemImagination,        // counterfactual generation
    Integration,           // promote validated insights to higher tiers
}
```

**Phase 1: NREM Replay** -- Select episodes with highest prediction error, cluster by HDC similarity, extract patterns into Insight Signals at Transient tier.

**Phase 2: Hindsight Relabeling** -- Failed trajectories are decomposed into sub-goals. Sub-goals that were achieved are relabeled as positive episodes and fed back into NREM replay. Recovers useful learning signal from at least 45% of otherwise-discarded episodes.

**Phase 3: REM Imagination** -- Generate counterfactual scenarios from high-value Insights. Useful counterfactuals that would have passed gates become StrategyFragment Signals. **Threat rehearsal** runs as a sub-phase: enumerate plausible threat scenarios, generate Warning Signals (ephemeral, published on Bus with short TTL).

**Phase 4: Integration** -- Promote validated Insights and StrategyFragments through tiers. Three-stage distillation:

| Stage | Input | Output | Criteria |
|---|---|---|---|
| **D1** (episodes to insights) | Recurring patterns | Insight Signals at Transient tier | 3+ supporting episodes |
| **D2** (insights to heuristics) | Confirmed insights | Heuristic Signals with when/then + falsifier | 5+ independent confirmations |
| **D3** (heuristics to playbooks) | Top heuristics | `PLAYBOOK.md` for human review | Top 12 by calibration score |

### Dream triggers

| Trigger | Default | Description |
|---|---|---|
| `idle_timeout` | 5 minutes | Agent has been idle for this duration |
| `episode_threshold` | 50 | Unprocessed episodes exceed this count |
| `scheduled_cron` | Off | Optional seven-field cron cadence; due work waits for the next idle boundary |
| `manual` | N/A | Explicit `roko knowledge dream run` command |
| `bus_signal` | Off | Signal on Bus topic triggers at delta timescale |

---

## 10. Temporal Knowledge Graph

### Current implementation

`roko_neuro::temporal` already supplies an optional in-memory `TemporalIndex`,
ordered `KnowledgeEpoch` records, half-open `TemporalInterval` values in epoch
milliseconds, and all 13 `AllenRelation` variants. The index associates entry IDs
with intervals and caches pairwise relations. It is not yet a durable, propagating
temporal constraint graph.

### Target extension

The full temporal knowledge graph adds three components:

1. Allen's 13 interval relations as a **constraint network** stored in Store.
2. Event calculus (Kowalski-Sergot 1986) as **Cells**: HoldsAt, Initiates, Terminates.
3. A 3-tier temporal memory as **three Memory specializations** at different timescales.

### 10.1 Allen's Interval Relations

Allen (1983) defined 13 mutually exclusive relations between time intervals. Every pair of temporal intervals satisfies exactly one. The relations form a JEME (jointly exhaustive, mutually exclusive) partition.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AllenRelation {
    Before, After, Meets, MetBy, Overlaps, OverlappedBy, Contains,
    During, Starts, StartedBy, Finishes, FinishedBy, Equal,
}

/// A half-open interval `[start, end)` in epoch milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemporalInterval {
    pub start: i64,
    pub end: i64,
}

impl TemporalInterval {
    pub const fn new(start: i64, end: i64) -> Self { Self { start, end } }

    pub fn allen_relation(&self, other: &Self) -> AllenRelation {
        AllenRelation::compute(self, other)
    }
}
```

The complete method set, including duration, containment, overlap, intersection,
and hull helpers, is authoritative in `crates/roko-neuro/src/temporal.rs`.

### Target: constraint network

Allen's algebra supports constraint propagation: if A is Before B and B Overlaps C, we can infer the possible relations between A and C. This is stored in Store as a constraint network over Signal validity intervals.

```rust
/// Temporal constraint network stored in Store.
pub struct TemporalConstraintNetwork {
    /// Adjacency: (hash_a, hash_b) -> set of possible Allen relations.
    constraints: HashMap<(ContentHash, ContentHash), AllenRelationSet>,
    /// All known intervals.
    intervals: HashMap<ContentHash, TemporalInterval>,
}

/// Compact set of Allen relations (13 bits, one per relation).
/// Intersection is bitwise AND. Union is bitwise OR.
/// Empty set means temporal contradiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllenRelationSet(u16);

impl AllenRelationSet {
    pub const ALL: Self = Self(0x1FFF);   // all 13 bits set
    pub const EMPTY: Self = Self(0);

    pub fn singleton(r: AllenRelation) -> Self { Self(1 << r as u16) }
    pub fn contains(&self, r: AllenRelation) -> bool { self.0 & (1 << r as u16) != 0 }
    pub fn intersect(&self, other: Self) -> Self { Self(self.0 & other.0) }
    pub fn is_empty(&self) -> bool { self.0 == 0 }
}
```

### Target: constraint propagation Cell

```rust
/// Cell: Allen constraint propagation.
///
/// When a new temporal relation is asserted between two Signals,
/// propagates the constraint through the network using the 13x13
/// composition table (Allen 1983, Table 2).
///
/// Returns Err if inconsistency detected (temporal contradiction).
pub struct AllenPropagationCell;

impl Cell for AllenPropagationCell {
    fn name(&self) -> &str { "allen-constraint-propagation" }

    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let assertion = extract_temporal_assertion(&input[0])?;
        let network = ctx.store().get_temporal_network().await?;

        let (a, b, relation) = (assertion.signal_a, assertion.signal_b, assertion.relation);
        network.add(a, b, AllenRelationSet::singleton(relation));

        // Propagate using worklist algorithm
        let mut worklist = vec![(a, b)];
        while let Some((x, y)) = worklist.pop() {
            for z in network.neighbors_of(x).chain(network.neighbors_of(y)) {
                if z == x || z == y { continue; }

                let r_xy = network.get(x, y);
                let r_yz = network.get(y, z);
                let r_xz_composed = allen_compose(r_xy, r_yz);

                let r_xz_old = network.get(x, z);
                let r_xz_new = r_xz_old.intersect(r_xz_composed);

                if r_xz_new.is_empty() {
                    // INCONSISTENCY: temporal contradiction detected
                    return Ok(vec![Signal::new(
                        Kind::Finding,
                        ThreatFinding {
                            class: ThreatClass::LineageMismatch,
                            affected_signals: vec![x, y, z],
                            confidence: 1.0,
                            severity: 0.7,
                            recommended_action: ContainmentAction::Reverify,
                        },
                    )]);
                }

                if r_xz_new != r_xz_old {
                    network.set(x, z, r_xz_new);
                    worklist.push((x, z));
                }
            }
        }

        ctx.store().put_temporal_network(&network).await?;
        Ok(vec![])
    }
}

/// The 13x13 Allen composition table.
/// compose(R1, R2) returns the set of Allen relations between A and C
/// given R1(A, B) and R2(B, C). 169 entries, each an AllenRelationSet.
fn allen_compose(r1: AllenRelationSet, r2: AllenRelationSet) -> AllenRelationSet {
    let mut result = AllenRelationSet::EMPTY;
    for i in 0..13 {
        if r1.contains(AllenRelation::from_index(i)) {
            for j in 0..13 {
                if r2.contains(AllenRelation::from_index(j)) {
                    result = result.union(COMPOSITION_TABLE[i][j]);
                }
            }
        }
    }
    result
}
```

**Key property**: Allen's algebra is qualitative. It reasons about temporal ordering without requiring exact timestamps. This matters because many temporal facts are qualitative ("the refactor happened before the release") rather than exact.

### 10.2 Target: Event Calculus as Cells

The event calculus (Kowalski & Sergot 1986) models how the truth of properties
("fluents") changes in response to events. This design proposes three Cells; the
`Fluent`, `TemporalEvent`, HoldsAt, Initiates, and Terminates Cell types below are
not current runtime types.

```rust
/// A fluent: a time-varying property of the system.
/// Fluents are stored as Signals with Kind::Fluent.
pub struct Fluent {
    pub id: FluentId,
    pub name: String,
    pub value: serde_json::Value,
    pub valid: TemporalInterval,
    pub initiated_by: Option<EventId>,
    pub terminated_by: Option<EventId>,
}

/// An event: a point-in-time occurrence that initiates or terminates fluents.
/// Events are stored as Signals with Kind::TemporalEvent.
pub struct TemporalEvent {
    pub id: EventId,
    pub timestamp: i64,
    pub description: String,
    pub signal_hash: Option<ContentHash>,
    pub initiates: Vec<FluentId>,
    pub terminates: Vec<FluentId>,
    pub caused_by: Vec<EventId>,
}
```

**HoldsAt Cell (Score protocol)**: Given a fluent and a time, determine whether the fluent is true. A fluent holds at time T if there exists an event E at E.timestamp <= T that initiates the fluent, AND there is no event E' at E.timestamp < E'.timestamp <= T that terminates the fluent. This is the law of inertia -- fluents persist until terminated, solving the frame problem.

**Initiates Cell (React protocol)**: When an event occurs, start the fluent. Subscribes to event Signals and emits fluent-start Signals.

**Terminates Cell (React protocol)**: When an event occurs, end the fluent. Closes the open interval on the current fluent value.

### 10.3 Target: Three-Tier Temporal Memory

Inspired by Rasmussen et al. (2025, Zep/Graphiti), the temporal knowledge graph has three tiers, each a **Memory specialization** operating at a different timescale.

#### Tier 1: Episode Memory (Minutes to Hours)

Raw Signal sequences with bundled HDC fingerprints. "What happened."

```rust
pub struct EpisodeMemory {
    pub episodes: Vec<TemporalEpisode>,
    pub max_episodes: usize,  // FIFO eviction when exceeded
}

pub struct TemporalEpisode {
    pub id: Uuid,
    pub interval: TemporalInterval,
    pub signal_hashes: Vec<ContentHash>,
    /// Bundle of member Signal fingerprints.
    pub fingerprint: Option<HdcVector>,
    pub summary: Option<String>,
    pub causal_links: Vec<Uuid>,
}
```

Demurrage: standard rate. Episodes decay at the normal Signal rate.

#### Tier 2: Entity Memory (Hours to Weeks)

Extracted entities with temporal properties and evolving HDC centroids. "What exists."

```rust
pub struct EntityMemory {
    pub entities: HashMap<EntityId, TemporalEntity>,
}

pub struct TemporalEntity {
    pub id: EntityId,
    pub name: String,
    pub kind: EntityKind,
    /// Properties modeled as fluents (time-varying via event calculus).
    pub properties: Vec<FluentId>,
    /// Relationships with temporal validity.
    pub relationships: Vec<TemporalRelationship>,
    /// Running centroid from supporting Signal fingerprints.
    pub fingerprint: Option<HdcVector>,
    pub created: i64,
    pub last_seen: i64,
}

pub struct TemporalRelationship {
    pub source: EntityId,
    pub target: EntityId,
    pub relation_type: RelationType,
    pub valid: TemporalInterval,
    pub confidence: f64,
    pub evidence: ContentHash,
}
```

Demurrage: half the standard rate. Entities persist longer than episodes.

#### Tier 3: Community Memory (Weeks to Months)

HDC-backed clusters of related entities. "What patterns exist."

```rust
pub struct CommunityMemory {
    pub communities: Vec<TemporalCommunity>,
}

pub struct TemporalCommunity {
    pub id: Uuid,
    pub entities: Vec<EntityId>,
    /// Temporal intersection: when all entities co-exist.
    pub active_interval: TemporalInterval,
    /// Bundle-center for similarity-driven promotion.
    pub fingerprint: Option<HdcVector>,
    pub summary: Option<String>,
    /// Stability score: how long unchanged.
    pub stability: f64,
}
```

Demurrage: quarter the standard rate. Communities are the most durable tier.

### HDC-guided tier progression

Tier progression uses HDC fingerprints to decide when episodes should be promoted to entities and entities to communities:

```rust
pub fn tier_progression(
    episodes: &EpisodeMemory,
    entities: &mut EntityMemory,
    communities: &mut CommunityMemory,
    similarity_threshold: f64,     // default: 0.7
    stability_threshold: f64,      // default: 0.8
    min_community_size: usize,     // default: 3
) {
    // Episode -> Entity: cluster temporally overlapping, HDC-similar episodes
    let clusters = cluster_by_temporal_overlap_and_hdc(
        &episodes.episodes, similarity_threshold
    );

    for cluster in clusters {
        if cluster.len() < 2 { continue; }

        // Compute centroid of the cluster's fingerprints
        let centroid = hdc_bundle(
            &cluster.iter()
                .filter_map(|ep| ep.fingerprint.as_ref())
                .collect::<Vec<_>>()
        );

        let entity_id = entities.find_or_create_by_centroid(
            &centroid, similarity_threshold
        );
        entities.update_centroid(entity_id, &centroid);
    }

    // Entity -> Community: cluster stable, co-temporal entities
    let entity_clusters = cluster_entities_by_overlap_and_hdc(
        &entities.entities, similarity_threshold
    );

    for cluster in entity_clusters {
        if cluster.len() < min_community_size { continue; }

        let stability = compute_cluster_stability(&cluster);
        if stability >= stability_threshold {
            let community = TemporalCommunity {
                id: Uuid::new_v4(),
                entities: cluster.iter().map(|e| e.id).collect(),
                active_interval: compute_intersection(&cluster),
                fingerprint: Some(hdc_bundle(
                    &cluster.iter()
                        .filter_map(|e| e.fingerprint.as_ref())
                        .collect::<Vec<_>>()
                )),
                summary: None,
                stability,
            };
            communities.communities.push(community);
        }
    }
}
```

### Target: temporal decay modulation

Demurrage interacts with temporal validity. Active-interval Signals get slower demurrage; expired-interval Signals get faster demurrage.

```rust
pub fn temporal_demurrage(
    signal: &Signal,
    now: i64,
    base_tax: f64,
    community_stability: f64,
) -> f64 {
    let interval = signal.temporal_interval();

    let temporal_modifier = if interval.end == TemporalInterval::ONGOING {
        0.5  // ongoing: half the normal tax rate
    } else if interval.end > now {
        0.7  // active but with known end: reduced tax
    } else {
        let staleness = (now - interval.end) as f64 / 86400_000_000_000.0; // days
        1.0 + (staleness * 0.1).min(2.0) // up to 3x normal rate
    };

    let stability_modifier = 1.0 - (community_stability * 0.3);
    base_tax * temporal_modifier * stability_modifier
}
```

The interaction creates a natural lifecycle:
- **Fresh, active Signals**: low demurrage, high balance, prominent in retrieval.
- **Active but aging Signals**: moderate demurrage, gradually losing prominence.
- **Expired Signals**: high demurrage, rapidly losing balance, moving toward cold tier.
- **Expired but in stable community**: moderate demurrage (community structure supports the Signal).

### Target: temporal consistency (Verify Cell)

New Signals that contradict the established timeline are flagged. The `TemporalConsistencyVerify` Cell checks whether adding a Signal's interval creates an inconsistency (empty AllenRelationSet after propagation). Inconsistency means the asserted timeline is self-contradictory -- a signal of memory poisoning or data corruption.

---

## 11. Target: Pheromone Mechanism

The unified design represents pheromones as **Pulses** with a typed
`PheromoneKind`, location hash, and intensity (Grasse 1959, first description of
stigmergy in termite construction; Dorigo 1992, Ant Colony Optimization). The
current code has a `ContextSource::Pheromone` label but no typed pheromone runtime,
registry, or Pulse retention/reinforcement policy.

### Pheromone types

```rust
pub enum PheromoneKind {
    Wisdom,        // "I learned something useful here"
    Opportunity,   // "There is value to capture here"
    Threat,        // "Danger -- avoid or prepare"
    Curiosity,     // "Something unexplained -- investigate"
}
```

### Stigmergy

The pheromone mechanism implements digital stigmergy:

1. **Agents modify the shared environment** -- deposit pheromone Pulses with typed intensity at a location hash.
2. **Future Agents observe modifications** -- query by location hash, ranked by decayed intensity.
3. **Coordination emerges** -- without direct communication.

### Pheromone demurrage

In the target policy, pheromone intensity decays via a Bus-backed bounded field
rather than durable Store retention. Its proposed default half-life is one hour,
and independent deposits at the same location reinforce the field. Current
`BroadcastBus` is live-only and `MemoryBus` replay capacity is caller-supplied;
neither backend implements this pheromone retention policy by itself.

### Pipeline integration

During the **Observe step** (step 1) of the 9-step pipeline ([05-AGENT](05-AGENT.md) SS8), an Agent reads the pheromone field for its current context:

- **THREAT** signal: increases prior for danger, biases toward caution.
- **OPPORTUNITY** signal: decreases threshold for exploration.
- **WISDOM** signal: boosts confidence in related knowledge Signals.
- **CURIOSITY** signal: increases prediction error, biases toward investigation.

---

## 12. Current Retrieval/Distillation and Target 9-Step Pipeline

The runner currently retrieves local knowledge during prompt assembly and distills
terminal gate outcomes into Heuristic or AntiKnowledge entries. The numbered
nine-step flow and its Bus/on-chain/VCG details below describe the broader target.

### RETRIEVE (Step 2)

During context assembly, the Agent queries the Memory store and assembles context via the CognitiveWorkspace VCG auction ([05-AGENT](05-AGENT.md) SS16).

**Query flow**:

1. Compute an HDC fingerprint for the current task prompt.
2. Query local neuro store (similarity search, ~170us at 10K entries).
3. Optionally query InsightStore on-chain (same similarity function, chain latency).
4. Score results using ContextAssemblyWeights (HDC 40%, keyword 30%, utility 20%, freshness 10%, cross-domain +15%).
5. Results enter the VCG auction as knowledge bidders alongside NeuroBidder, TaskBidder, HeuristicBidder, and others.
6. Winning entries are injected into the system prompt.
7. Heuristic bidders are weighted by calibration score -- poorly calibrated heuristics bid lower.

### REFLECT (Step 9)

After execution and gating:

1. Log the episode to `.roko/episodes.jsonl` with an HDC fingerprint.
2. If a gate passed, reinforce balance on any knowledge Signals that were in the context pack.
3. If a gate failed, do NOT reinforce -- demurrage continues uninterrupted.
4. Check heuristic falsifiers: if any heuristic in the context pack had its falsifier condition met, record a calibration receipt.
5. Emit knowledge events on the Bus.

---

## 13. Target: On-Chain Integration

### InsightStore (Solidity)

```solidity
interface IInsightStore {
    function publish(
        uint8 kind,
        bytes32 contentHash,
        uint16 confidence,
        uint8 tier,
        bytes calldata tags,
        bytes calldata hdcVector
    ) external returns (uint256 entryId);

    function validate(uint256 entryId, bytes32 evidence) external;
    function challenge(uint256 entryId, bytes32 reason) external;
    function freeze(uint256 entryId) external;
    function querySimilar(
        bytes calldata queryVector,
        uint8 topK
    ) external view returns (uint256[] memory entryIds, uint16[] memory scores);

    event EntryPublished(uint256 indexed entryId, address indexed author, uint8 kind);
    event EntryValidated(uint256 indexed entryId, address indexed validator);
    event EntryChallenged(uint256 indexed entryId, address indexed challenger);
}
```

### PheromoneRegistry (Solidity)

```solidity
interface IPheromoneRegistry {
    function deposit(
        uint8 ptype,
        uint16 intensity,
        bytes32 locationHash,
        bytes calldata metadata
    ) external returns (uint256 pheromoneId);

    function readAt(bytes32 locationHash) external view returns (...);
    function reinforce(uint256 pheromoneId, uint16 boostAmount) external;
    function summary(bytes32 locationHash)
        external view returns (
            uint16 wisdom, uint16 opportunity,
            uint16 threat, uint16 curiosity
        );

    event PheromoneDeposited(uint256 indexed id, address indexed depositor, uint8 ptype);
    event PheromoneReinforced(uint256 indexed id, uint16 newIntensity);
}
```

Each on-chain entry is approximately 1,340 bytes. No natural language touches the chain -- content stored off-chain with on-chain hash commitment.

### Target: ChainConnectorCell (Connect protocol)

The proposed `ChainConnectorCell` bridges the local Memory Store and the on-chain
`IInsightStore`. Neither this connector nor the contracts below are current Rust
runtime surfaces.

```rust
/// Bridges local Store <-> on-chain IInsightStore.
/// Implements Cell + Connect protocol.
pub struct ChainConnectorCell {
    chain_client: Option<ChainClient>,
    /// Queue for chain writes when latency is high or chain is unavailable.
    write_queue: VecDeque<PendingChainWrite>,
    /// Maximum queue depth before dropping oldest entries.
    max_queue_depth: usize,  // default: 1000
    /// Chain write timeout.
    write_timeout: Duration, // default: 30s
}

pub struct PendingChainWrite {
    pub signal_hash: ContentHash,
    pub kind: u8,
    pub confidence: u16,
    pub tier: u8,
    pub tags: Vec<u8>,
    pub hdc_vector: Vec<u8>,
    pub queued_at: Instant,
    pub retries: u32,
}

impl Cell for ChainConnectorCell {
    fn name(&self) -> &str { "chain-connector" }
    fn protocols(&self) -> &[ProtocolId] { &[ProtocolId::Connect] }
}

#[async_trait]
impl ConnectProtocol for ChainConnectorCell {
    async fn connect(&mut self) -> Result<()> {
        self.chain_client = Some(ChainClient::new(/* config */).await?);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // Flush remaining queue before disconnect
        self.flush_queue().await?;
        self.chain_client = None;
        Ok(())
    }

    async fn health(&self) -> HealthStatus {
        match &self.chain_client {
            Some(client) => match client.block_number().await {
                Ok(_) => HealthStatus::Healthy,
                Err(_) => HealthStatus::Degraded,
            },
            None => HealthStatus::Disconnected,
        }
    }
}
```

**Put path** (local -> chain):

```
MemoryCell.put(signal)
  -> ChainConnectorCell.connect()
     -> IInsightStore.publish(kind, contentHash, confidence, tier, tags, hdcVector)
```

**Fail-open behavior**: When the chain is unavailable or latency exceeds `write_timeout`:

1. The Signal is stored in the **local Store immediately** -- local writes never block on chain availability.
2. The chain write is **queued** in `write_queue` with retry metadata.
3. A background flush task drains the queue when the chain becomes available, using exponential backoff (1s, 2s, 4s, ... up to 5min).
4. If the queue exceeds `max_queue_depth`, the oldest entries are dropped with a Bus warning Pulse on topic `chain.queue_overflow`.

```rust
impl ChainConnectorCell {
    /// Put path: local Store first, then queue chain write.
    pub async fn put_with_chain(
        &mut self,
        store: &mut dyn Store,
        signal: Signal,
    ) -> Result<ContentHash> {
        // Step 1: Always write to local Store (never blocks on chain)
        let hash = store.put(signal.clone()).await?;

        // Step 2: Queue chain write (fail-open)
        if self.chain_client.is_some() {
            let pending = PendingChainWrite {
                signal_hash: hash,
                kind: signal.kind as u8,
                confidence: (signal.scores.confidence * 1000.0) as u16,
                tier: signal.tier as u8,
                tags: encode_tags(&signal.tags),
                hdc_vector: signal.hdc_fingerprint.map(|v| v.to_bytes()).unwrap_or_default(),
                queued_at: Instant::now(),
                retries: 0,
            };

            match tokio::time::timeout(
                self.write_timeout,
                self.chain_client.as_ref().unwrap().publish_insight(&pending),
            ).await {
                Ok(Ok(_)) => { /* Chain write succeeded immediately */ }
                _ => {
                    // Chain slow or unavailable: queue for retry
                    if self.write_queue.len() >= self.max_queue_depth {
                        self.write_queue.pop_front(); // Drop oldest
                    }
                    self.write_queue.push_back(pending);
                }
            }
        }

        Ok(hash)
    }

    /// Background flush: called periodically or on reconnect.
    pub async fn flush_queue(&mut self) -> Result<usize> {
        let mut flushed = 0;
        while let Some(mut pending) = self.write_queue.pop_front() {
            match self.chain_client.as_ref()
                .ok_or(CellError::Disconnected)?
                .publish_insight(&pending).await
            {
                Ok(_) => { flushed += 1; }
                Err(_) => {
                    pending.retries += 1;
                    self.write_queue.push_front(pending);
                    break; // Stop flushing on first failure
                }
            }
        }
        Ok(flushed)
    }
}
```

**Query path** (chain -> local): During RETRIEVE (SS12), the agent can optionally query `IInsightStore.querySimilar()` for Signals not in the local Store. Results are cached locally with Transient tier and standard demurrage. Chain query latency (~200-500ms) is acceptable because it runs in parallel with the local Store query.

---

## 14. Target: API Endpoints

These routes are the desired HTTP surface, not a claim that `roko-serve` currently
mounts every route listed here.

### Knowledge endpoints

```
GET    /api/knowledge/entries              List Signals (paginated, filtered)
GET    /api/knowledge/entries/:id          Get a single knowledge Signal
POST   /api/knowledge/publish              Publish a new knowledge Signal
POST   /api/knowledge/validate/:id         Validate an existing Signal
POST   /api/knowledge/challenge/:id        Challenge an existing Signal
GET    /api/knowledge/search               HDC similarity search
  ?vector=<base64>&top_k=10&domain=<domain>&kind=<kind>&min_balance=0.1
GET    /api/knowledge/stats                Store statistics
POST   /api/knowledge/dream/run            Trigger a dream cycle
GET    /api/knowledge/dream/report         Latest dream cycle report
```

### Heuristic endpoints

```
GET    /api/knowledge/heuristics           List heuristics with calibration scores
POST   /api/knowledge/heuristics           Create a new heuristic (requires falsifier)
GET    /api/knowledge/heuristics/:id       Get heuristic with full calibration history
GET    /api/knowledge/heuristics/:id/receipts  Calibration receipts
GET    /api/knowledge/worldviews           List discovered worldviews
GET    /api/knowledge/worldviews/:id       Worldview with constituent heuristics
```

### Pheromone endpoints

```
GET    /api/pheromones                     List active pheromone Pulses
GET    /api/pheromones/summary?location=<hash>  Per-type aggregate
POST   /api/pheromones/deposit             Deposit a pheromone Pulse
POST   /api/pheromones/reinforce/:id       Reinforce an existing pheromone
GET    /api/pheromones/field               Full field state (for visualization)
```

---

## 15. Target: TOML Configuration

This is a proposed unified configuration shape. It is not the exact current
`roko-config` deserialization contract.

```toml
[knowledge]
store_path = ".roko/neuro/knowledge.jsonl"
max_entries = 100000

[knowledge.demurrage]
enabled = true
apply_interval = "1h"
cold_threshold = 0.05
novelty_reset_on_new_context = true

[knowledge.demurrage.base_rates]
Insight = 0.01
Heuristic = 0.005
Warning = 0.10
CausalLink = 0.007
StrategyFragment = 0.02
AntiKnowledge = 0.01

[knowledge.demurrage.reinforcement]
retrieved = 0.05
cited = 0.10
gate_passed = 0.15
surprised = 0.20
agent_quoted = 0.08

[knowledge.tiers]
promotion_success_threshold = 3
demotion_failure_threshold = 2

[knowledge.anti_knowledge]
warn_threshold = 0.5
discount_threshold = 0.7
reject_threshold = 0.9
discount_factor = 0.5

[knowledge.heuristic]
falsifier_required = true
retire_threshold = 0.30
min_calibration_score = 0.50
min_receipts_for_promotion = 10
auto_refine = true
max_children = 5

[pheromones]
default_half_life_secs = 3600
max_active = 10000
expiry_threshold = 0.01

[dreams]
auto_dream = true
idle_threshold_mins = 15
min_episodes_for_dream = 5
scheduled_cron = "0 0 */4 * * * *"
episode_count_trigger = 50
quality_gain = 0.75
quality_penalty = 1.25
max_replay_episodes = 200
counterfactual_budget = 20
hindsight_enabled = true
hindsight_min_subgoals = 1
promotion_confidence_floor = 0.7
```

---

## 16. Target Acceptance Catalog

This table is a roadmap verification catalog. It mixes already exercised local
store behavior with unimplemented distributed, temporal-graph, Cell, and
consortium targets; it is not a shipped-feature checklist.

| # | Criterion | Verification |
|---|---|---|
| MK-1 | `KnowledgeStore` implements the current `NeuroStore` lifecycle; target `MemoryCell` implements unified Store | Unit and integration tests |
| MK-2 | Knowledge Signals decay via demurrage with per-Kind rates | Unit test: compute balance at t=0, t=30d for Insight |
| MK-3 | Tier multipliers applied correctly (Transient decays 10x faster) | Unit test |
| MK-4 | Retrieval reinforces balance with novelty weighting | Unit test: retrieve 10 times, verify diminishing reinforcement |
| MK-5 | Gate-pass reinforcement restores balance | Integration test |
| MK-6 | Cold threshold triggers archival | Unit test: decay below 0.05, verify cold archival |
| MK-7 | Frozen Signals skip demurrage entirely | Unit test |
| MK-8 | Heuristic creation requires mandatory falsifier | Unit test: attempt create without falsifier, verify rejection |
| MK-9 | Heuristic calibration score updates on gate outcome | Integration test |
| MK-10 | Heuristic retirement when falsifier fires above threshold | Integration test |
| MK-11 | Retired heuristic spawns refined children | Integration test |
| MK-12 | Predicate matching: DomainIs, FilePathGlob, RegimeIs | Unit test |
| MK-13 | Wilson CI narrows with more calibration receipts | Unit test |
| MK-14 | Worldview forms from co-citation cluster | Integration test: co-cite 3 heuristics 20+ times |
| MK-15 | Worldview swap when challenger exceeds main calibration | Integration test |
| MK-16 | AntiKnowledge at 0.7 similarity halves initial balance | Unit test |
| MK-17 | AntiKnowledge at 0.9 similarity rejects entry | Unit test |
| MK-18 | HDC encode + similarity produces correct results | Unit test |
| MK-19 | Resonator Network factorizes bundle into role-fillers | Unit test |
| MK-20 | Cross-domain bonus of 15% applied when domains differ | Unit test |
| MK-21 | Dream NREM clusters episodes by HDC similarity | Integration test |
| MK-22 | Dream hindsight relabeling recovers sub-goals from failures | Integration test |
| MK-23 | Dream REM generates counterfactuals | Integration test |
| MK-24 | D2 distillation produces Heuristic with when/then + falsifier | Integration test |
| MK-25 | Pheromone Pulses decay with 1-hour default half-life | Unit test |
| MK-26 | Pheromone reinforcement resets decay clock | Unit test |
| MK-27 | Novelty attenuation: retrieval_count=10 yields ~0.30 novelty | Unit test |
| MK-28 | RETRIEVE step queries store and enters VCG auction | Integration test |
| MK-29 | REFLECT step reinforces balance on gate pass | Integration test |
| MK-30 | Cold Signals archived with provenance intact | Integration test |
| MK-31 | Allen relation computation correct for all 13 cases | Unit test: exhaustive endpoint comparison |
| MK-32 | Constraint propagation detects temporal contradiction | Unit test: assert Before(A,B) + After(A,B) -> error |
| MK-33 | HoldsAt Cell returns true for initiated, unterminated fluent | Unit test |
| MK-34 | Initiates/Terminates Cells create/close fluent intervals | Integration test |
| MK-35 | Episode -> Entity promotion via HDC clustering | Integration test |
| MK-36 | Entity -> Community promotion requires stability >= 0.8 | Unit test |
| MK-37 | Temporal demurrage modulation: ongoing Signals decay at 0.5x | Unit test |
| MK-38 | Temporal consistency Verify rejects contradictory intervals | Integration test |
| MK-39 | Freeze requires 3+ validators from distinct contexts | Unit test: attempt freeze with 2 validators, verify rejection |
| MK-40 | Thaw via challenge: 72-hour review window opens on challenge | Integration test |
| MK-41 | Thaw via challenge: quorum vote (3+ distinct from freezers) determines outcome | Integration test |
| MK-42 | Automatic thaw on 3+ consecutive gate failures with frozen Signal in context | Integration test |
| MK-43 | Freeze/thaw publish Bus events on `knowledge.frozen` / `knowledge.thawed` | Unit test |
| MK-44 | Worldviews stored as Signals with Kind::Worldview at Consolidated tier | Unit test |
| MK-45 | Worldview demurrage at half standard rate (r=0.005, beta=0.01) | Unit test |
| MK-46 | Contrarian retrieval slot (15%) populated from rival_worldviews | Integration test |
| MK-47 | ChainConnectorCell implements Connect protocol (connect/disconnect/health) | Unit test |
| MK-48 | ChainConnectorCell put path: local Store first, queue chain write | Integration test |
| MK-49 | ChainConnectorCell fail-open: local write succeeds when chain is unavailable | Integration test |
| MK-50 | ChainConnectorCell queue flush with exponential backoff on reconnect | Integration test |
| MK-51 | AgentQuoted reinforcement restores +0.08 * novelty balance | Unit test |

---

## 17. Current and Target Crate Mapping

| Crate | Responsibility |
|---|---|
| `roko-neuro` | Local knowledge store, tier progression, demurrage, retrieval scoring, AntiKnowledge, heuristic lifecycle |
| `roko-primitives` | Current HDC primitives; target resonator-network factorization |
| `roko-dreams` | Dream cycle orchestration, NREM replay, hindsight relabeling, REM imagination, threat rehearsal |
| `roko-learn` | Episode logger, HDC fingerprinting, playbook store, efficiency tracking |
| `roko-serve` | Target unified knowledge, heuristic, and pheromone HTTP surface |
| `roko-chain` (target phase 2) | On-chain InsightStore and PheromoneRegistry interactions, ChainConnectorCell |

---

## 18. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal/Pulse duality, demurrage field | [01-SIGNAL](01-SIGNAL.md) | -- |
| Store protocol | [02-CELL](02-CELL.md) | -- |
| Verify redesign (continuous reward) | [02-CELL](02-CELL.md) | -- |
| CognitiveWorkspace (VCG auction, section effects) | [05-AGENT](05-AGENT.md) | SS16 |
| Somatic markers (contrarian retrieval) | [05-AGENT](05-AGENT.md) | SS13 |
| 9-step pipeline | [05-AGENT](05-AGENT.md) | SS8 |
| Learning loops (L1-L4) | [07-LEARNING](07-LEARNING.md) | -- |
| Extension system | [12-EXTENSIONS](12-EXTENSIONS.md) | -- |
| Telemetry: DriftLens (knowledge health) | [15-TELEMETRY](15-TELEMETRY.md) | -- |
| On-chain registries | [22-REGISTRIES](22-REGISTRIES.md) | -- |
| Connect protocol (ChainConnectorCell) | [02-CELL](02-CELL.md) | SS9 (Connect) |
| Demurrage constants (canonical source) | This document | SS3 |
