# 11 — Memory and Knowledge

> Memory = Store-protocol Block with decay, tier progression, HDC-based retrieval, and dream consolidation.

**Subsumes**: Knowledge Entry (now Signal), Pheromone (now Signal), InsightStore, PheromoneRegistry, dream consolidation, AntiKnowledge, knowledge decay.

**Source**: Refactored from `tmp/architecture/09-knowledge.md` using unified vocabulary.

---

## 1. The Knowledge Problem

Agent frameworks treat memory as a bag of text chunks. Append to a vector store, retrieve by cosine similarity, stuff into the next prompt. Nothing decays. Nothing consolidates. Nothing gets shared across agents.

Four consequences compound over time:

1. **Noise floor rises.** Without temporal decay, stale information dilutes retrieval. A cached heuristic from session #12, invalidated by session #47, still appears in results.
2. **No compression.** Five episodes demonstrating the same pattern remain five separate chunks. No process distills raw episodes into compact, reusable knowledge.
3. **No quality signal.** A hallucinated claim and a gate-validated insight sit at the same confidence level. Without provenance tracking, unreliable knowledge contaminates reliable knowledge.
4. **No sharing.** A thousand agents solving related problems independently discover the same patterns. The hundredth agent to learn "run clippy before committing Rust code" pays the same discovery cost as the first.

Memory treats knowledge as a living substrate instead of a dead archive. Signals decay, consolidate, get validated by peers, and flow across the network through stigmergic coordination.

---

## 2. Memory as Specialization

In the unified vocabulary, **Memory** is a Store-protocol Block with decay, tier progression, dream consolidation, and HDC-based retrieval (see [doc-04, section 7](04-SPECIALIZATIONS.md#7-memory)).

```rust
pub struct MemoryConfig {
    pub store_path: PathBuf,
    pub max_entries: usize,
    pub default_half_life: Duration,
    pub tier_config: TierConfig,
    pub anti_knowledge: AntiKnowledgeConfig,
    pub dream_config: DreamConfig,
}
```

A Memory Block manages the knowledge lifecycle:

1. **Ingest** — New Signals enter at Transient tier
2. **Retrieve** — HDC similarity search + multi-dimensional scoring
3. **Decay** — Ebbinghaus curve with tier multipliers
4. **Promote/Demote** — Based on gate validation results
5. **Consolidate** — Dream cycles compress episodes into durable knowledge
6. **Prune** — Below 1% threshold, archive to cold storage

The Memory Block implements the Store protocol, meaning it conforms to `put / get / query / prune` — the same interface as FileStore or MemoryStore, but with decay semantics layered on top.

---

## 3. Knowledge Signal Kinds

Knowledge is stored as Signals with specific Kinds. In the unified vocabulary, what was previously "Knowledge Entry" is now **Signal (persisted, knowledge Kind)**.

```rust
pub enum KnowledgeKind {
    /// Observation with evidence. "Tests run 30% faster with parallel execution."
    Insight,
    /// Behavioral rule with when/then clause. "When refactoring, run clippy first."
    Heuristic,
    /// Transient danger flag. "API rate limit approaching."
    Warning,
    /// Causal relationship. "Increasing batch size causes OOM above 512."
    CausalLink,
    /// Partial strategy. "For TypeScript migration, start with shared types."
    StrategyFragment,
    /// Explicitly falsified knowledge. Repels similar future entries.
    AntiKnowledge,
}
```

Each Kind has different decay characteristics, promotion criteria, and retrieval behavior. The Kind discriminant on the Signal determines how the Memory Block handles it.

### On-chain representation

Each on-chain entry is approximately 1,340 bytes. No natural language touches the chain — content is stored off-chain with an on-chain hash commitment.

```rust
pub struct OnChainKnowledgeSignal {
    pub id: SignalId,                    // 32-byte content-addressed identifier
    pub kind: KnowledgeKind,             // Discriminant (0-5)
    pub content_hash: [u8; 32],          // SHA-256 of off-chain content
    pub confidence: u16,                 // Fixed-point 0..65535
    pub tier: KnowledgeTier,             // Transient (T0) | Working (T1) | Consolidated (T2) | Persistent (T3)
    pub tags: Vec<String>,              // Topic tags for filtering
    pub author_wallet: Address,          // 20-byte Ethereum address
    pub created_at: u64,                 // Block timestamp
    pub validated_count: u32,            // Independent confirmations
    pub challenged_count: u32,           // Active disputes
    pub hdc_fingerprint: [u8; 1280],     // PP-HDC encoded (non-invertible)
    pub frozen: bool,                    // Promoted by consensus, never decays
}
```

Off-chain content lives in JSONL files at `.roko/neuro/knowledge.jsonl`. The on-chain record stores only the commitment hash.

---

## 4. Decay Model (Ebbinghaus)

Every knowledge Signal decays exponentially following the Ebbinghaus forgetting curve:

```
confidence(t) = initial * 0.5^(age / half_life) * tier_multiplier * (1 + confirmations * 0.1)
```

### Per-Kind half-lives

| Kind | Off-chain half-life | On-chain half-life | Rationale |
|---|---|---|---|
| `Insight` | 30 days | 7 days | Observations need confirmation but persist locally |
| `Heuristic` | 90 days | 15 days | Behavioral rules are durable once proven |
| `Warning` | 1 hour | ~3 min (90 blocks) | Warnings are transient by nature |
| `CausalLink` | 60 days | 15 days | Causal models need time for varied testing |
| `StrategyFragment` | 14 days | 15 days | Strategies in evolving codebases go stale |
| `AntiKnowledge` | 30 days | 15 days | What-not-to-do stays relevant |

On-chain half-lives are shorter because the chain is a competitive environment. Stale knowledge must make room for fresh observations.

### Refresh on validation

Each independent confirmation resets the decay clock and extends the effective lifetime by 10%. Five confirmations from different agents yield 1.5x the base half-life.

### Frozen Signals

When a knowledge Signal accumulates enough consensus support (3+ validators across distinct contexts), it can be frozen. Frozen Signals skip decay entirely. They remain in the store at their current confidence indefinitely. The `freeze()` contract call requires consortium approval.

### Pruning threshold

When confidence drops below 1% of initial weight, the Signal enters the death stage:

```rust
pub const DEATH_THRESHOLD: f64 = 0.01;
```

Pruned Signals are archived to cold storage, preserving their content hash, lineage, and provenance. They can be thawed later if conditions change.

---

## 5. Tier System

Knowledge Signals progress through four tiers. Each tier applies a multiplier to the base half-life.

### Tiers

```rust
pub enum KnowledgeTier {
    /// T0: New, unvalidated. Decays 10x faster.
    Transient,     // multiplier: 0.1x
    /// T1: Survived initial validation. Decays 2x faster.
    Working,       // multiplier: 0.5x
    /// T2: Repeatedly validated. Decays at base rate.
    Consolidated,  // multiplier: 1.0x
    /// T3: Consensus-backed. Decays 5x slower.
    Persistent,    // multiplier: 5.0x
}
```

**Example**: An Insight with a 30-day base half-life:

| Tier | Effective half-life |
|---|---|
| Transient (0.1x) | 3 days |
| Working (0.5x) | 15 days |
| Consolidated (1.0x) | 30 days |
| Persistent (5.0x) | 150 days |

### Tier Progression

#### Promotion criteria

| From | To | Requirement |
|---|---|---|
| Transient | Working | 3+ gate passes where this Signal was in the context pack |
| Working | Consolidated | 5+ independent confirmations from different Agents or contexts |
| Consolidated | Persistent | Consortium approval (3+ validators) OR manual freeze |

#### Demotion criteria

| From | To | Requirement |
|---|---|---|
| Persistent | Consolidated | Unfreezing (manual or challenge upheld) |
| Consolidated | Working | 2+ gate failures where this Signal was in the context pack |
| Working | Transient | 3+ consecutive gate failures OR confidence below 0.3 |
| Transient | Pruned | Confidence below DEATH_THRESHOLD (0.01) |

### Validation flow

When Agent B retrieves a knowledge Signal published by Agent A, uses it during a task, and passes a gate:

1. The gate-pass event generates a confirmation.
2. The confirmation increments `validated_count` on A's Signal.
3. Confidence increases: `new_confidence = old_confidence + 0.05 * (1.0 - old_confidence)`.
4. The decay clock resets.
5. A's reputation increases proportionally.

### Challenge flow

When an Agent believes a knowledge Signal is wrong:

1. The challenger submits a challenge with counter-evidence.
2. `challenged_count` increments.
3. If `challenged_count >= 3`, the Signal enters consortium review.
4. During review, confidence is halved and the Signal is flagged in query results.
5. Resolution paths: **upheld** (challenges dismissed, confidence restored), **refuted** (Signal converted to AntiKnowledge), or **revised** (author publishes amended version).

---

## 6. HDC Embeddings

The knowledge system encodes structured information as 10,240-bit binary vectors. No floating point. No matrix multiply. No GPU.

### The vector

```rust
/// 10,240-bit binary sparse distributed vector.
pub struct HdcVector {
    bits: [u64; 160],  // 160 words * 64 bits = 10,240 bits = 1,280 bytes
}
```

Implementation lives in `roko-primitives/src/hdc.rs`. Serialization uses little-endian byte packing. Transport uses base64 encoding (1,280 bytes become 1,708 characters).

### Core operations

**Bind (XOR).** Combines two vectors into one dissimilar to both inputs. Used for role-filler binding: `bind(ROLE, value)` creates a composite encoding "this value fills this role." XOR is its own inverse — `bind(bind(a, b), b) == a` — so you can unbind a role to recover the filler.

**Bundle (majority vote).** Combines multiple vectors into one similar to all inputs. Used for aggregation: bundling five domain-specific vectors produces a composite that retrieves all five in a similarity query.

**Permute (bit rotation).** Encodes position and sequence. `permute(v, 1)` shifts all bits left by 1 (cyclic). Ordered sequences: `bundle(permute(v1, 0), permute(v2, 1), permute(v3, 2))` encodes "v1 then v2 then v3."

**Similarity (Hamming distance).** Measures overlap via hardware POPCNT. Two random 10,240-bit vectors are ~50% similar by chance. Meaningful similarity starts around 0.52-0.53.

```rust
pub fn similarity(&self, other: &Self) -> f32 {
    let mut differing_bits = 0u32;
    for (left, right) in self.bits.iter().zip(other.bits.iter()) {
        differing_bits += (left ^ right).count_ones();
    }
    1.0 - (differing_bits as f32 / 10_240.0)
}
```

### Role-filler encoding

Structured knowledge enters a single vector through role-filler binding:

```rust
pub fn encode_structured(roles_and_fillers: &[(String, String)]) -> HdcVector {
    let bound: Vec<HdcVector> = roles_and_fillers
        .iter()
        .map(|(role, filler)| role_hv(role).bind(&text_hv(filler)))
        .collect();
    HdcVector::bundle(&bound.iter().collect::<Vec<_>>())
}
```

An episode fingerprint encodes:

| Role | Filler | Purpose |
|---|---|---|
| `task_description` | Task prompt text | What was attempted |
| `domain` | Domain tag | Which problem area |
| `model` | Model identifier | Which model ran |
| `tool_call_0..N` | Permuted tool call vectors | Ordered tool sequence |
| `outcome` | "success" or "failure" | Result |
| `file_path_0..N` | Modified file paths | What changed |
| `error_pattern` | Failure reason (if any) | What went wrong |

### Cross-domain resonance

HDC vectors enable cross-domain pattern discovery. When Signals from different domains have similar fingerprints, they share structural properties despite operating in different contexts. A retry pattern from networking might apply to database operations. A rate-limiting strategy from API design might transfer to gas optimization.

The retrieval scoring formula gives cross-domain matches a **15% bonus**:

```rust
pub struct ContextAssemblyWeights {
    pub hdc_similarity: f64,      // 40%
    pub keyword_relevance: f64,   // 30%
    pub pf_utility: f64,          // 20%
    pub freshness: f64,           // 10%
    pub cross_domain_bonus: f64,  // 15% bonus (additive when domains differ)
}
```

### Performance targets

| Operation | Target | Notes |
|---|---|---|
| Encode one vector | < 5 us | Seed + splitmix64 expansion |
| Similarity (two vectors) | < 1 us | XOR + POPCNT, cache-friendly |
| Search 1K patterns | < 100 us | Brute-force, no index needed |
| Search 10K patterns | < 1 ms | Still brute-force at this scale |

### Why HDC instead of float embeddings?

| Property | HDC (10,240-bit binary) | Float embeddings (1536-d float32) |
|---|---|---|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity | XOR + POPCNT (1 cycle) | Dot product (hundreds of FLOPs) |
| Compositionality | Native (bind/bundle/permute) | Requires learned operations |
| Hardware | CPU SIMD lanes | GPU preferred for batch |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder models |
| On-chain cost | ~$0.002 per entry | ~$0.01 per entry |
| Determinism | Identical seeds produce identical vectors | Depends on model version |

The critical advantage: HDC vectors are algebraic objects with structure. You can bind a role to a filler, bundle multiple role-filler pairs, and later unbind a role to approximately recover the filler. Float embeddings are opaque blobs.

---

## 7. AntiKnowledge

When the system discovers that a previously trusted insight is wrong, it does not delete the original. It creates an AntiKnowledge Signal that actively repels future knowledge in the same HDC region. This is Popper's falsificationism applied to learned rules.

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
| Above 0.7 | Halve the new Signal's confidence (discount factor 0.5) |
| Above 0.9 | Reject the Signal outright — it is not stored |

### AntiKnowledge lifecycle

1. **Creation**: A knowledge Signal is refuted through the challenge flow (3+ challenges, consortium review upholds).
2. **Conversion**: The refuted Signal's Kind is changed to `AntiKnowledge`. Its content is preserved but its role inverts.
3. **Decay**: AntiKnowledge decays like other Signals (30-day base half-life). Old mistakes eventually stop blocking new discoveries.
4. **Override**: If overwhelming evidence contradicts an AntiKnowledge entry, the challenge flow can convert it back or archive it.

---

## 8. Dream Consolidation

Dream consolidation is the offline process where Agents compress raw episodes into durable knowledge. It runs when an Agent accumulates enough unprocessed experience — what the system calls "sleep pressure." Dream consolidation is a **Loop** specialization: a Graph that feeds output back to input on the delta timescale.

### Three phases

```rust
pub struct DreamCycle {
    pub agent_id: AgentId,
    pub started_at: DateTime<Utc>,
    pub phase: DreamPhase,
    pub episodes_in: usize,
    pub insights_out: Vec<Signal>,     // knowledge-Kind Signals produced
    pub report: Option<DreamCycleReport>,
}

pub enum DreamPhase {
    /// NREM replay: priority replay of high-surprise episodes.
    NremReplay,
    /// REM imagination: counterfactual generation.
    RemImagination,
    /// Integration: promote validated insights to higher tiers.
    Integration,
}
```

### Phase 1: NREM Replay

The system selects episodes with the highest prediction error (surprise) and replays them. Replay identifies recurring patterns across the batch.

```rust
pub fn select_replay_episodes(
    episodes: &[Episode],
    budget: &DreamBudget,
) -> Vec<&Episode> {
    // Sort by prediction_error descending.
    // Take up to budget.max_replay_episodes.
    // Filter out already-processed episodes.
}
```

Episodes are clustered by plan/task shape using HDC similarity. Clusters with **3+ supporting episodes** and **0.7+ confidence** become candidate Insight Signals at Transient tier.

### Phase 2: REM Imagination

The system generates counterfactuals from high-surprise episodes: "what if I had done X instead?"

```rust
pub struct CounterfactualQuery {
    pub original_episode: Episode,
    pub alternative_action: String,
    pub expected_outcome: String,
}

pub fn imagine(query: &CounterfactualQuery) -> ImaginationOutcome {
    // Generate alternative episode.
    // Evaluate against gate criteria.
    // Return outcome with confidence.
}
```

The `imagine()` function synthesizes alternative action sequences and evaluates them against the same gate criteria. Useful counterfactuals that would have passed become StrategyFragment Signals.

### Phase 3: Integration

Candidate Signals from NREM and REM phases are evaluated against tier promotion criteria and written to the knowledge store. The three-stage distillation pipeline runs:

| Stage | Input | Output | Criteria |
|---|---|---|---|
| **D1** (episodes to insights) | Recurring patterns | Insight Signals at Transient tier | 3+ supporting episodes |
| **D2** (insights to heuristics) | Confirmed insights | Heuristic Signals with when/then clauses | 5+ independent confirmations |
| **D3** (heuristics to playbooks) | Top heuristics | `PLAYBOOK.md` for human review | Top 12 by confidence |

### Threat rehearsal

A specialized sub-phase that runs during or after REM imagination. The system enumerates plausible threat scenarios from recent episodes and generates Warning Signals:

```rust
pub fn enumerate_threats(episodes: &[Episode]) -> Vec<ThreatScenario> {
    // Identify failure patterns.
    // Extrapolate to plausible future scenarios.
    // Score by likelihood * impact.
}

pub fn threat_warning_signals(threats: &[ThreatScenario]) -> Vec<Signal> {
    // Convert high-scoring threats to Warning-kind Signals.
    // Set half_life to 1 hour.
}
```

### Dream triggers

| Trigger | Default | Description |
|---|---|---|
| `idle_timeout` | 5 minutes | Agent has been idle for this duration |
| `episode_threshold` | 50 | Unprocessed episodes exceed this count |
| `manual` | N/A | Explicit `roko knowledge dream run` command |
| `bus_signal` | Off | Signal on Bus topic triggers at delta timescale |

### Scheduling

Dream cycles run during the **delta timescale** — the slow background tick that handles maintenance, consolidation, and housekeeping. In practice this means dreams run between active work periods, not during task execution.

---

## 9. Pheromone Mechanism (Signal-based)

In the unified vocabulary, pheromones are **ephemeral Signals** with a typed `PheromoneKind`, location hash, and intensity. They are not a separate primitive — they are Signals published to the Bus that happen to carry pheromone semantics.

### Pheromone Signal types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PheromoneKind {
    /// "I learned something useful here"
    Wisdom,
    /// "There is value to capture here"
    Opportunity,
    /// "Danger -- avoid or prepare"
    Threat,
    /// "Something unexplained -- investigate"
    Curiosity,
}
```

The enum is extensible. New pheromone types can be added without breaking existing Agents — Agents that do not recognize a type ignore it.

### Pheromone Signal structure

```rust
pub struct PheromoneSignal {
    /// Standard Signal fields (id, kind, content_hash, lineage, etc.)
    pub signal: Signal,
    /// Pheromone-specific fields
    pub ptype: PheromoneKind,
    pub intensity: f64,              // 0.0..=1.0
    pub location_hash: [u8; 32],     // Hash of the context (domain, topic, file, etc.)
    pub depositor: AgentId,          // Agent that deposited this pheromone
    pub half_life_secs: u64,         // Decay rate (default 3600 = 1 hour)
}
```

### Stigmergy

The term comes from Grasse (1959), who observed termite nest construction. Termites modify the environment (deposit pheromone, add mud), and future termites observe those modifications to decide where to build next. No termite communicates with another. The environment mediates all coordination.

The pheromone mechanism implements digital stigmergy:

1. **Agents modify the shared environment** — deposit pheromone Signals with typed intensity at a location hash.
2. **Future Agents observe modifications** — query by location hash, ranked by decayed intensity.
3. **Coordination emerges** — without direct communication.

### Decay

Pheromone intensity decays exponentially:

```
intensity(t) = initial * exp(-t / half_life)
```

Default half-life is **1 hour** (3,600 seconds). When intensity drops below 0.01, the pheromone Signal expires and is removed from active queries.

### Reinforcement

When multiple Agents independently deposit pheromone Signals at the same location hash, the cumulative signal is strong and persists longer than any single deposit. Reinforcement resets the decay clock and adds to the current intensity.

### Pipeline integration

During the **OBSERVE step** (step 1) of the 9-step pipeline, an Agent reads the pheromone field for its current context. Pheromone gradients influence prediction error:

- A strong **THREAT** signal at a location increases the Agent's prior for danger, biasing toward caution.
- A strong **OPPORTUNITY** signal decreases the threshold for exploration.
- A strong **WISDOM** signal boosts confidence in related knowledge Signals.
- A strong **CURIOSITY** signal increases prediction error, biasing toward investigation.

---

## 10. Knowledge in the 9-Step Pipeline

Knowledge participates at two points in the Agent's 9-step pipeline.

### RETRIEVE (Step 2)

During context assembly, the Agent queries both the on-chain InsightStore and the local neuro store. Results compete for prompt space through the VCG attention auction.

**Query flow**:

1. Compute an HDC fingerprint for the current task prompt.
2. Query InsightStore via the HTC precompile (top-K by Hamming similarity, ~170us at 10K entries).
3. Query local neuro store (same similarity function, no chain latency).
4. Score results using ContextAssemblyWeights (HDC 40%, keyword 30%, utility 20%, freshness 10%, cross-domain +15%).
5. Results enter the VCG auction as knowledge bidders alongside `NeuroContextBidder`, `TaskContextBidder`, and `ResearchContextBidder`.
6. Winning entries are injected into the system prompt.

### REFLECT (Step 9)

After execution and gating:

1. Log the episode to `.roko/episodes.jsonl` with an HDC fingerprint.
2. If a gate passed, update confidence on any knowledge Signals that were in the context pack.
3. If a gate failed, demote any knowledge Signals that directly influenced the failing action.
4. Increment `catalytic_score` on context-pack Signals that contributed to new knowledge creation.
5. Emit knowledge events on the Bus.

### Learning Loop 3: Knowledge Consolidation

Knowledge consolidation is one of the four learning loops (see [doc-10, Learning Loops](10-LEARNING-LOOPS.md)). It operates at the delta timescale:

```
Episodes accumulate -> sleep pressure builds -> dream cycle fires ->
NREM replay -> REM imagination -> Integration ->
new knowledge Signals at Transient tier -> validation through use ->
tier promotion -> durable knowledge
```

This loop feeds back into RETRIEVE: newly consolidated knowledge appears in future context assemblies, improving Agent performance over time.

---

## 11. On-Chain Integration

### InsightStore (Solidity)

```solidity
interface IInsightStore {
    /// Publish a new knowledge Signal. Caller becomes the author.
    function publish(
        uint8 kind,
        bytes32 contentHash,
        uint16 confidence,
        uint8 tier,
        bytes calldata tags,
        bytes calldata hdcVector
    ) external returns (uint256 entryId);

    /// Validate an existing Signal. Increments validated_count.
    function validate(uint256 entryId, bytes32 evidence) external;

    /// Challenge an existing Signal. Increments challenged_count.
    function challenge(uint256 entryId, bytes32 reason) external;

    /// Freeze a Signal. Requires consortium approval (3+ validators).
    function freeze(uint256 entryId) external;

    /// Fetch a single Signal by ID.
    function getEntry(uint256 entryId)
        external view returns (
            uint8 kind, bytes32 contentHash, uint16 confidence,
            uint8 tier, address author, uint64 createdAt,
            uint32 validatedCount, uint32 challengedCount,
            bool frozen
        );

    /// Query by HDC similarity via the HTC precompile.
    function querySimilar(
        bytes calldata queryVector,
        uint8 topK
    ) external view returns (uint256[] memory entryIds, uint16[] memory scores);

    event EntryPublished(uint256 indexed entryId, address indexed author, uint8 kind);
    event EntryValidated(uint256 indexed entryId, address indexed validator);
    event EntryChallenged(uint256 indexed entryId, address indexed challenger);
    event EntryFrozen(uint256 indexed entryId);
}
```

### PheromoneRegistry (Solidity)

```solidity
interface IPheromoneRegistry {
    /// Deposit a pheromone Signal.
    function deposit(
        uint8 ptype,
        uint16 intensity,
        bytes32 locationHash,
        bytes calldata metadata
    ) external returns (uint256 pheromoneId);

    /// Read active pheromone Signals at a location.
    function readAt(bytes32 locationHash)
        external view returns (
            uint256[] memory ids,
            uint8[] memory types,
            uint16[] memory intensities,
            uint64[] memory timestamps
        );

    /// Reinforce an existing pheromone Signal.
    function reinforce(uint256 pheromoneId, uint16 boostAmount) external;

    /// Aggregate pheromone summary for a location (per-type sums of decayed intensities).
    function summary(bytes32 locationHash)
        external view returns (
            uint16 wisdom, uint16 opportunity,
            uint16 threat, uint16 curiosity
        );

    event PheromoneDeposited(uint256 indexed id, address indexed depositor, uint8 ptype, uint16 intensity);
    event PheromoneReinforced(uint256 indexed id, uint16 newIntensity);
    event PheromoneExpired(uint256 indexed id);
}
```

Detailed chain integration in [doc-18 (On-Chain Registries)](18-ON-CHAIN-REGISTRIES.md).

---

## 12. Event Types

Knowledge and pheromone events are published as ephemeral Signals on the Bus.

### Knowledge events

```json
{"type": "knowledge.published", "signal_id": "a1b2c3", "kind": "Insight", "confidence": 0.8}
{"type": "knowledge.validated", "signal_id": "a1b2c3", "validator": "agent-x", "new_confidence": 0.85}
{"type": "knowledge.challenged", "signal_id": "a1b2c3", "challenger": "agent-y", "reason_hash": "..."}
{"type": "knowledge.decayed", "signal_id": "a1b2c3", "old_confidence": 0.8, "new_confidence": 0.6}
{"type": "knowledge.frozen", "signal_id": "a1b2c3", "validators": ["agent-x", "agent-z", "agent-w"]}
{"type": "knowledge.promoted", "signal_id": "a1b2c3", "old_tier": "Transient", "new_tier": "Working"}
{"type": "knowledge.demoted", "signal_id": "a1b2c3", "old_tier": "Working", "new_tier": "Transient"}
{"type": "knowledge.pruned", "signal_id": "a1b2c3", "final_confidence": 0.008}
```

### Pheromone events

```json
{"type": "pheromone.deposited", "ptype": "Opportunity", "intensity": 0.9, "agent_id": "agent-alpha"}
{"type": "pheromone.reinforced", "pheromone_id": "p1", "new_intensity": 0.95, "agent_id": "agent-beta"}
{"type": "pheromone.expired", "pheromone_id": "p1"}
```

### Dream events

```json
{"type": "dream.started", "agent_id": "agent-alpha", "trigger": "idle_timeout", "episode_count": 67}
{"type": "dream.phase_changed", "agent_id": "agent-alpha", "phase": "RemImagination"}
{"type": "dream.insight_promoted", "signal_id": "d4e5f6", "old_tier": "Transient", "new_tier": "Working"}
{"type": "dream.completed", "agent_id": "agent-alpha", "insights_produced": 4, "duration_secs": 12}
```

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
  ?vector=<base64>                         Query vector
  &top_k=10                                Number of results
  &domain=<domain>                         Optional domain filter
  &kind=<kind>                             Optional kind filter
  &min_confidence=0.5                      Minimum confidence threshold
GET    /api/knowledge/stats                Store statistics
POST   /api/knowledge/dream/run            Trigger a dream cycle
GET    /api/knowledge/dream/report         Latest dream cycle report
```

### Pheromone endpoints

> Endpoint paths use `/api/pheromones` for backward compatibility. Payloads use unified Signal vocabulary.

```
GET    /api/pheromones                     List active pheromone Signals
GET    /api/pheromones/summary             Per-type aggregate at a location
  ?location=<hash>                         Location hash
POST   /api/pheromones/deposit             Deposit a pheromone Signal
  { "ptype": "Opportunity", "intensity": 0.9, "location_hash": "...", "metadata": {...} }
POST   /api/pheromones/reinforce/:id       Reinforce an existing pheromone Signal
GET    /api/pheromones/field               Full field state (for visualization)
```

---

## 14. TOML Configuration

```toml
[knowledge]
store_path = ".roko/neuro/knowledge.jsonl"
max_entries = 100000
default_half_life_hours = 168  # 7 days

[knowledge.half_lives]
Insight = "30d"
Heuristic = "90d"
Warning = "1h"
CausalLink = "60d"
StrategyFragment = "14d"
AntiKnowledge = "30d"

[knowledge.tiers]
promotion_success_threshold = 3    # Gate passes to promote
demotion_failure_threshold = 2     # Gate failures to demote
death_threshold = 0.01             # Prune below this weight

[knowledge.anti_knowledge]
warn_threshold = 0.5
discount_threshold = 0.7
reject_threshold = 0.9
discount_factor = 0.5

[pheromones]
default_half_life_secs = 3600     # 1 hour
max_active = 10000
expiry_threshold = 0.01

[dreams]
idle_timeout_mins = 5
episode_threshold = 50
max_replay_episodes = 200
counterfactual_budget = 20
promotion_confidence_floor = 0.7
```

---

## 15. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Memory Block implements Store protocol (put/get/query/prune) | Unit test: CRUD operations on knowledge Signals |
| Knowledge Signals decay following Ebbinghaus curve with tier multipliers | Unit test: compute confidence at t=0, t=half_life, t=2*half_life |
| Per-Kind half-lives applied correctly | Unit test: Insight vs Warning decay rates differ by 720x |
| Tier promotion on 3+ gate passes | Integration test: pass 3 gates, verify Transient -> Working |
| Tier demotion on 2+ gate failures | Integration test: fail 2 gates, verify Consolidated -> Working |
| Frozen Signals skip decay entirely | Unit test: freeze Signal, advance time, verify confidence unchanged |
| AntiKnowledge at 0.5 similarity logs warning | Unit test: insert AntiKnowledge, insert similar Signal, verify warning |
| AntiKnowledge at 0.7 similarity halves confidence | Unit test: verify confidence = initial * 0.5 |
| AntiKnowledge at 0.9 similarity rejects entry | Unit test: verify Signal not stored |
| HDC encode + similarity produces correct results | Unit test: encode two similar structures, verify similarity > 0.6 |
| Cross-domain bonus of 15% applied when domains differ | Unit test: score same-domain vs cross-domain, verify bonus |
| Dream NREM phase clusters episodes by HDC similarity | Integration test: 10 episodes, verify cluster formation |
| Dream REM phase generates counterfactuals | Integration test: high-surprise episode, verify alternatives produced |
| Dream Integration writes new Signals at Transient tier | Integration test: verify D1 output in store |
| D2 distillation produces Heuristic Signals with when/then | Integration test: 5+ confirmed Insights -> Heuristic |
| D3 distillation writes PLAYBOOK.md | Integration test: verify file written with top 12 |
| Pheromone Signals decay with 1-hour default half-life | Unit test: deposit, advance 1 hour, verify intensity ~50% |
| Pheromone reinforcement resets decay clock | Unit test: deposit, advance 30 min, reinforce, verify reset |
| Pheromone expiry at 0.01 threshold | Unit test: advance time, verify removal |
| RETRIEVE step queries both on-chain and local stores | Integration test: populate both, verify merged results |
| REFLECT step updates confidence on gate pass | Integration test: pass gate, verify context-pack Signals boosted |
| REFLECT step demotes on gate failure | Integration test: fail gate, verify context-pack Signals demoted |
| Pruned Signals archived to cold storage with provenance | Integration test: prune, verify archive contents |

---

## 16. Crate Mapping

| Crate | Responsibility |
|---|---|
| `roko-neuro` | Local knowledge store, tier progression, retrieval scoring, AntiKnowledge |
| `roko-primitives` | HdcVector (bind/bundle/permute/similarity), item memory, accumulators |
| `roko-dreams` | Dream cycle orchestration, NREM replay, REM imagination, threat rehearsal |
| `roko-learn` | Episode logger, HDC fingerprinting, playbook store, efficiency tracking |
| `roko-serve` | HTTP endpoints for knowledge and pheromone APIs |
| `roko-chain` (phase 2) | On-chain InsightStore and PheromoneRegistry interactions |
