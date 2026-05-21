# 11 — Memory and Knowledge

> Memory = Store-protocol Cell with demurrage, tier progression, HDC-based retrieval, heuristic lifecycle, and dream consolidation. Knowledge is a living substrate: signals decay unless actively used, heuristics carry mandatory falsifiers, and worldviews emerge from co-citation.

**Subsumes**: Knowledge Entry (now Signal), Pheromone (now Pulse), InsightStore, PheromoneRegistry, dream consolidation, AntiKnowledge, knowledge decay, Heuristic lifecycle, Resonator Networks.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality, demurrage, Kind system), [02-CELL](02-CELL.md) (Store protocol, Verify redesign), [07-AGENT-RUNTIME](07-AGENT-RUNTIME.md) (CognitiveWorkspace, somatic markers), [10-LEARNING-LOOPS](10-LEARNING-LOOPS.md) (L3 dream consolidation, hindsight relabeling)

---

## 1. The Knowledge Problem

Agent frameworks treat memory as a bag of text chunks. Append to a vector store, retrieve by cosine similarity, stuff into the next prompt. Nothing decays. Nothing consolidates. Nothing gets shared across agents.

Four consequences compound over time:

1. **Noise floor rises.** Without temporal pressure, stale information dilutes retrieval. A cached heuristic from session #12, invalidated by session #47, still appears in results.
2. **No compression.** Five episodes demonstrating the same pattern remain five separate chunks. No process distills raw episodes into compact, reusable knowledge.
3. **No quality signal.** A hallucinated claim and a gate-validated insight sit at the same confidence level. Without provenance tracking, unreliable knowledge contaminates reliable knowledge.
4. **No sharing.** A thousand agents solving related problems independently discover the same patterns. The hundredth agent to learn "run clippy before committing Rust code" pays the same discovery cost as the first.

Memory treats knowledge as a living substrate instead of a dead archive. Signals decay via demurrage unless actively used, consolidate through dream cycles, get validated by peers, and flow across the network through stigmergic coordination.

---

## 2. Memory as Specialization

In the unified vocabulary, **Memory** is a Store-protocol Cell with demurrage, tier progression, dream consolidation, and HDC-based retrieval (see [doc-04](04-SPECIALIZATIONS.md)).

```rust
pub struct MemoryConfig {
    pub store_path: PathBuf,
    pub max_entries: usize,
    pub demurrage_config: DemurrageConfig,
    pub tier_config: TierConfig,
    pub anti_knowledge: AntiKnowledgeConfig,
    pub dream_config: DreamConfig,
    pub heuristic_config: HeuristicConfig,
}
```

A Memory Cell manages the knowledge lifecycle:

1. **Ingest** -- New Signals enter at Transient tier with initial balance 1.0
2. **Retrieve** -- HDC similarity search + multi-dimensional scoring
3. **Demurrage** -- Balance decays unless actively used (retrieved, cited, gate-passed, surprised)
4. **Promote/Demote** -- Based on gate validation results and balance thresholds
5. **Consolidate** -- Dream cycles compress episodes into durable knowledge ([doc-10 SS5](10-LEARNING-LOOPS.md))
6. **Prune** -- Below cold threshold, archive to cold storage

The Memory Cell implements the Store protocol, meaning it conforms to `put / get / query / query_similar / prune` -- the same interface as FileStore or MemoryStore, but with demurrage semantics layered on top.

---

## 3. Demurrage Model

**Demurrage replaces Ebbinghaus decay** (Gesell 1916). Instead of passive time-based forgetting, Signals pay a holding cost for occupying store space. Active use -- retrieval, citation, gate-pass, surprise -- restores balance. This is economic selection pressure on knowledge: unique, actively-useful insights stay warm; redundant or stale entries fade.

### The balance field

Every Signal carries a `balance` field (see [doc-01 SS2](01-SIGNAL.md)):

```rust
pub struct Signal {
    // ...
    pub balance: f64,                    // starts at 1.0, decays via demurrage
    pub demurrage_paid: f64,             // monotonic, for observability
    pub last_touched_at: DateTime<Utc>,  // last reinforcement event
    // ...
}
```

### Rate law

```
balance(t+dt) = balance(t) - r*dt - beta*balance(t)*dt
```

- `r` = flat tax per day (constant drain)
- `beta` = exponential decay rate per day (keeps value bounded)

### Reinforcement kinds

Active use restores balance. Each reinforcement kind has a different weight, and reinforcement is novelty-weighted (anti-hoarding mechanism):

```rust
pub enum ReinforceKind {
    Retrieved,      // returned in a query
    Cited,          // in another Signal's source[] lineage
    GatePassed,     // in context pack when gate passed
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

### Novelty-weighted reinforcement

```
novelty = 1 / (1 + ln(retrieval_count))
```

The first retrieval restores full balance (`novelty = 1.0`). The 10th restores ~0.30x. The 100th restores ~0.18x. This prevents a popular-but-mediocre Signal from staying warm purely through high retrieval frequency. Genuinely novel use (new context, new agent, new domain) resets the retrieval counter.

### Per-Kind default rates

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

### Cold threshold

When balance drops below `COLD_THRESHOLD` (default 0.05), the Signal enters cold storage. Body moves to slower storage; content hash stays valid; lineage preserved. **Thaw** restores balance to a starter value and is itself a Bus event (`knowledge.thawed`).

### Frozen Signals

When a knowledge Signal accumulates enough consensus support (3+ validators across distinct contexts), it can be frozen. Frozen Signals skip demurrage entirely -- they remain in the store at their current balance indefinitely. The `freeze()` operation requires consortium approval or explicit human action.

### Ebbinghaus as special case

Pure time-based decay (Ebbinghaus) is recovered when no interactions occur: a Signal that is never retrieved, cited, or gate-passed decays by the demurrage rate. Demurrage generalizes Ebbinghaus by adding an economic mechanism -- use restores value.

---

## 4. Heuristic as First-Class Kind

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

Heuristics use a `Predicate` enum for matchable preconditions:

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

**Birth**: Heuristics are born from L3 dream consolidation ([doc-10 SS5](10-LEARNING-LOOPS.md)): when 5+ confirmed Insights cluster around the same when/then pattern, the distillation stage produces a Heuristic Signal with an auto-generated falsifier. Heuristics can also be created manually via `roko knowledge heuristic create`.

**Test**: Every time the Heuristic's `when` predicates match the current context and the Heuristic is included in the CognitiveWorkspace context pack ([doc-07 SS11](07-AGENT-RUNTIME.md)), the system records whether the `then` prediction was correct. The gate verdict determines correctness.

**Calibrate**: The CalibrationRecord updates on each receipt. Calibration score affects the heuristic's bid in the CognitiveWorkspace VCG auction: poorly calibrated heuristics lose prompt space to better-calibrated ones. This is the **heuristic calibration loop** described in [doc-10 SS8.2](10-LEARNING-LOOPS.md).

**Retire/Evolve**: When a heuristic's falsifier fires above the retirement threshold, violations spawn refined children with narrower when-clauses (cf. Quinlan ID3 for decision tree refinement on a live stream). A heuristic "When refactoring code, run clippy" that fails for JavaScript files might spawn: "When refactoring Rust code, run clippy" and "When refactoring TypeScript code, run eslint." Children carry a `parent_heuristic` reference for lineage tracking.

---

## 5. Worldviews

**Worldviews** emerge from co-citation clusters of heuristics with high calibration scores. They are not explicitly created -- they are discovered patterns in how heuristics reinforce each other.

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

### Worldview representation

```rust
pub struct Worldview {
    pub id: SignalId,
    pub name: String,
    pub heuristics: Vec<SignalRef>,
    pub co_citation_matrix: BTreeMap<(SignalRef, SignalRef), u64>,
    pub avg_calibration: f64,
    pub domain: String,
    pub rival_worldviews: Vec<SignalRef>,
}
```

### Multiple worldviews deliberately

The system maintains multiple worldviews for each domain to prevent cognitive monoculture:

| Role | Purpose | Example |
|---|---|---|
| **Main** | Highest avg calibration, used by default | "careful API engineering" |
| **Challenger** | Second-highest, used for 15% contrarian retrieval | "move-fast API engineering" |
| **Niche specialists** | High calibration for specific sub-domains | "high-throughput streaming APIs" |

The 15% mandatory contrarian retrieval from somatic markers ([doc-07 SS10](07-AGENT-RUNTIME.md)) naturally consults challenger worldviews, preventing the dominant worldview from becoming an unchallenged orthodoxy.

### Worldview swap mechanism

When a challenger worldview's avg calibration exceeds the main worldview's for 20+ consecutive evaluations, they swap roles. The former main becomes the new challenger. This mechanism allows the system's collective beliefs to shift in response to changing environments without catastrophic forgetting.

---

## 6. AntiKnowledge

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

## 7. HDC Operations and Resonator Networks

The knowledge system encodes structured information as 10,240-bit binary vectors (Kanerva 2009). No floating point. No matrix multiply. No GPU.

### The vector

```rust
pub struct HdcVector {
    bits: [u64; 160],  // 160 words * 64 bits = 10,240 bits = 1,280 bytes
}
```

Implementation lives in `roko-primitives/src/hdc.rs`.

### Core operations

| Operation | What | Cost | Reference |
|---|---|---|---|
| **Bind** (XOR) | Role-filler binding. Dissimilar to both inputs. Self-inverse. | O(n) | Rachkovskij 2001 |
| **Bundle** (majority vote) | Aggregation. Similar to all inputs. | O(n*k) | Kanerva 2009 |
| **Permute** (bit rotation) | Positional encoding. Cyclic left shift. | O(n) | Plate 2003 |
| **Similarity** (Hamming) | Overlap via XOR + POPCNT. | < 1 us | Hardware |
| **Resonate** | Factorize: recover constituents from bundle. | O(n*k*iter) | Frady et al. 2020 |

### Resonator Networks

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
4. **HDC cleanup**: During L3 consolidation ([doc-10 SS8.3](10-LEARNING-LOOPS.md)), Resonator Networks factorize to identify patterns learned independently at higher tiers. Redundant bundles are pruned.

### Performance targets

| Operation | Target | Notes |
|---|---|---|
| Encode one vector | < 5 us | Seed + splitmix64 expansion |
| Similarity (two vectors) | < 1 us | XOR + POPCNT, cache-friendly |
| Search 1K patterns | < 100 us | Brute-force, no index needed |
| Search 10K patterns | < 1 ms | Still brute-force at this scale |
| Resonator factorization (3 roles) | < 10 ms | ~5-20 iterations |

### Why HDC instead of float embeddings

| Property | HDC (10,240-bit binary) | Float (1536-d float32) |
|---|---|---|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity cost | XOR + POPCNT (~1 ns) | Dot product (hundreds FLOPs) |
| Compositionality | Native (bind/bundle/permute/resonate) | Requires learned operations |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder |
| Determinism | Identical seeds produce identical vectors | Depends on model version |

At 10,240 bits, **800K fingerprints fit in 1 GB RAM**; brute-force SIMD comparison is **<1 ms** for the full set. No external vector store needed. (Cf. Levy & Gayler 2008 for VSA survey; Olshausen & Field 1996 for biological precedent.)

---

## 8. Tier System

Knowledge Signals progress through four tiers. Each tier applies a multiplier to the demurrage rate.

```rust
pub enum Tier {
    Transient,     // 0.1x multiplier -- decays 10x faster
    Working,       // 0.5x -- decays 2x faster
    Consolidated,  // 1.0x -- base rate
    Persistent,    // 5.0x -- decays 5x slower
}
```

### Progression criteria

| From | To | Requirement |
|---|---|---|
| Transient | Working | 3+ gate passes where this Signal was in the context pack |
| Working | Consolidated | 5+ independent confirmations from different Agents or contexts |
| Consolidated | Persistent | Consortium approval (3+ validators) OR manual freeze |

### Demotion criteria

| From | To | Requirement |
|---|---|---|
| Persistent | Consolidated | Unfreezing (manual or challenge upheld) |
| Consolidated | Working | 2+ gate failures where this Signal was in the context pack |
| Working | Transient | 3+ consecutive gate failures OR balance below 0.15 |
| Transient | Cold | Balance below `COLD_THRESHOLD` (0.05) |

---

## 9. Dream Consolidation

Dream consolidation is the offline process where Agents compress raw episodes into durable knowledge. It runs when an Agent accumulates enough unprocessed experience -- "sleep pressure." Dream consolidation is a **Loop** specialization: a Graph that feeds output back to input on the delta timescale. The full four-phase cycle is defined in [doc-10 SS5](10-LEARNING-LOOPS.md).

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
| `manual` | N/A | Explicit `roko knowledge dream run` command |
| `bus_signal` | Off | Signal on Bus topic triggers at delta timescale |

---

## 10. Pheromone Mechanism

In the unified vocabulary, pheromones are **Pulses** (ephemeral) with a typed `PheromoneKind`, location hash, and intensity (Grasse 1959, first description of stigmergy in termite construction; Dorigo 1992, Ant Colony Optimization). They are not a separate primitive -- they are Pulses on the Bus that carry pheromone semantics.

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

Pheromone intensity decays via the same demurrage mechanism, but as Pulses they live on the Bus ring buffer rather than in Store. Default half-life is 1 hour. Reinforcement resets the decay clock: when multiple Agents independently deposit pheromone Pulses at the same location hash, the cumulative signal is strong and persists longer.

### Pipeline integration

During the **Observe step** (step 1) of the 9-step pipeline ([doc-07 SS8](07-AGENT-RUNTIME.md)), an Agent reads the pheromone field for its current context:

- **THREAT** signal: increases prior for danger, biases toward caution
- **OPPORTUNITY** signal: decreases threshold for exploration
- **WISDOM** signal: boosts confidence in related knowledge Signals
- **CURIOSITY** signal: increases prediction error, biases toward investigation

---

## 11. Knowledge in the 9-Step Pipeline

### RETRIEVE (Step 2)

During context assembly, the Agent queries the Memory store and assembles context via the CognitiveWorkspace VCG auction ([doc-07 SS11](07-AGENT-RUNTIME.md)).

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

## 12. On-Chain Integration

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

Each on-chain entry is approximately 1,340 bytes. No natural language touches the chain -- content stored off-chain with on-chain hash commitment. Detailed chain integration in [doc-18](18-ON-CHAIN-REGISTRIES.md).

---

## 13. API Endpoints

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

## 14. TOML Configuration

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
idle_timeout_mins = 5
episode_threshold = 50
max_replay_episodes = 200
counterfactual_budget = 20
hindsight_enabled = true
hindsight_min_subgoals = 1
promotion_confidence_floor = 0.7
```

---

## 15. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| MK-1 | Memory Cell implements Store protocol (put/get/query/query_similar/prune) | Unit test |
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

---

## 16. Crate Mapping

| Crate | Responsibility |
|---|---|
| `roko-neuro` | Local knowledge store, tier progression, demurrage, retrieval scoring, AntiKnowledge, heuristic lifecycle |
| `roko-primitives` | HdcVector (bind/bundle/permute/similarity), Resonator Networks, item memory |
| `roko-dreams` | Dream cycle orchestration, NREM replay, hindsight relabeling, REM imagination, threat rehearsal |
| `roko-learn` | Episode logger, HDC fingerprinting, playbook store, efficiency tracking |
| `roko-serve` | HTTP endpoints for knowledge, heuristic, and pheromone APIs |
| `roko-chain` (phase 2) | On-chain InsightStore and PheromoneRegistry interactions |

---

## 17. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal/Pulse duality, demurrage field | [doc-01](01-SIGNAL.md) | SS2, SS6 |
| Store protocol | [doc-02](02-CELL.md) | SS3.1 |
| Verify redesign (continuous reward) | [doc-02](02-CELL.md) | SS3.3 |
| CognitiveWorkspace (VCG auction, section effects) | [doc-07](07-AGENT-RUNTIME.md) | SS11 |
| Somatic markers (contrarian retrieval) | [doc-07](07-AGENT-RUNTIME.md) | SS10 |
| L3 dream consolidation (4-phase) | [doc-10](10-LEARNING-LOOPS.md) | SS5 |
| Hindsight relabeling | [doc-10](10-LEARNING-LOOPS.md) | SS5 |
| Seven compounding feedback loops | [doc-10](10-LEARNING-LOOPS.md) | SS8 |
| DriftLens (knowledge health) | [doc-09](09-TELEMETRY.md) | SS4.6 |
| StateHub knowledge projection | [doc-09](09-TELEMETRY.md) | SS6 |
| On-chain registries | [doc-18](18-ON-CHAIN-REGISTRIES.md) | -- |
