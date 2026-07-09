# 09 -- Knowledge and pheromones

> On-chain knowledge registry, temporal decay, HDC embeddings, stigmergic coordination, and dream consolidation.

---

## The knowledge problem

Agent frameworks treat memory as a bag of text chunks. Append to a vector store, retrieve by cosine similarity, stuff into the next prompt. Nothing decays. Nothing consolidates. Nothing gets shared across agents.

Four consequences compound over time:

1. **Noise floor rises.** Without temporal decay, stale information dilutes retrieval. A cached heuristic from session #12, invalidated by session #47, still appears in results.
2. **No compression.** Five episodes demonstrating the same pattern remain five separate chunks. No process distills raw episodes into compact, reusable knowledge.
3. **No quality signal.** A hallucinated claim and a gate-validated insight sit at the same confidence level. Without provenance tracking, unreliable knowledge contaminates reliable knowledge.
4. **No sharing.** A thousand agents solving related problems independently discover the same patterns. The hundredth agent to learn "run clippy before committing Rust code" pays the same discovery cost as the first.

The knowledge system treats memory as a living substrate instead of a dead archive. Entries decay, consolidate, get validated by peers, and flow across the network through pheromone-weighted stigmergy.

---

## InsightStore (on-chain knowledge registry)

The InsightStore is Korai's shared knowledge substrate. Agents publish validated knowledge on-chain, other agents query and consume it, and economic incentives keep the store healthy.

### Knowledge entry structure

Each on-chain entry is approximately 1,340 bytes. No natural language touches the chain -- content is stored off-chain, with an on-chain hash commitment.

```rust
pub struct OnChainEntry {
    pub id: EntryId,                    // 32-byte unique identifier
    pub kind: KnowledgeKind,            // Insight | Heuristic | Warning | CausalLink | StrategyFragment | AntiKnowledge
    pub content_hash: [u8; 32],         // SHA-256 of off-chain content
    pub confidence: u16,                // Fixed-point 0..65535
    pub tier: KnowledgeTier,            // Transient (T0) | Working (T1) | Consolidated (T2) | Persistent (T3)
    pub tags: Vec<String>,              // Topic tags for filtering
    pub author_wallet: Address,         // 20-byte Ethereum address
    pub created_at: u64,                // Block timestamp
    pub validated_count: u32,           // Independent confirmations
    pub challenged_count: u32,          // Active disputes
    pub hdc_fingerprint: [u8; 1280],    // PP-HDC encoded (non-invertible)
    pub frozen: bool,                   // Promoted by consensus, never decays
}
```

Off-chain content lives in JSONL files at `.roko/neuro/knowledge.jsonl`. The on-chain record stores the commitment hash. After an optional embargo period, authors can reveal the full content for network consumption.

### Solidity interface

```solidity
interface IInsightStore {
    // ── Entry lifecycle ─────────────────────────────────────────────

    /// Publish a new knowledge entry. Caller becomes the author.
    /// @param kind        Entry type discriminant (0-5)
    /// @param contentHash SHA-256 of the off-chain content
    /// @param confidence  Fixed-point confidence (0..65535)
    /// @param tier        Initial retention tier (0-3)
    /// @param tags        ABI-encoded tag list
    /// @param hdcVector   1280-byte PP-HDC fingerprint
    function publish(
        uint8 kind,
        bytes32 contentHash,
        uint16 confidence,
        uint8 tier,
        bytes calldata tags,
        bytes calldata hdcVector
    ) external returns (uint256 entryId);

    /// Validate an existing entry. Increments validated_count,
    /// boosts confidence, extends effective half-life.
    /// @param entryId  The entry to validate
    /// @param evidence Optional hash of supporting evidence
    function validate(
        uint256 entryId,
        bytes32 evidence
    ) external;

    /// Challenge an existing entry. Increments challenged_count,
    /// triggers consortium review if threshold reached.
    /// @param entryId The entry to challenge
    /// @param reason  Hash of the counter-evidence
    function challenge(
        uint256 entryId,
        bytes32 reason
    ) external;

    /// Freeze an entry. Requires consortium approval (3+ validators).
    /// Frozen entries skip decay entirely.
    function freeze(uint256 entryId) external;

    // ── Queries ─────────────────────────────────────────────────────

    /// Fetch a single entry by ID.
    function getEntry(uint256 entryId)
        external view returns (
            uint8 kind, bytes32 contentHash, uint16 confidence,
            uint8 tier, address author, uint64 createdAt,
            uint32 validatedCount, uint32 challengedCount,
            bool frozen
        );

    /// Query by HDC similarity via the HTC precompile.
    /// Returns top-K entry IDs ranked by Hamming similarity.
    /// @param queryVector  1280-byte query HDC vector
    /// @param topK         Number of results to return
    function querySimilar(
        bytes calldata queryVector,
        uint8 topK
    ) external view returns (uint256[] memory entryIds, uint16[] memory scores);

    // ── Events ──────────────────────────────────────────────────────

    event EntryPublished(uint256 indexed entryId, address indexed author, uint8 kind);
    event EntryValidated(uint256 indexed entryId, address indexed validator);
    event EntryChallenged(uint256 indexed entryId, address indexed challenger);
    event EntryFrozen(uint256 indexed entryId);
}
```

### Validation flow

When agent B retrieves an entry published by agent A, uses it during a task, and passes a gate:

1. Agent B's gate-pass event generates a confirmation transaction.
2. The confirmation increments `validated_count` on A's entry.
3. A's confidence increases: `new_confidence = old_confidence + 0.05 * (1.0 - old_confidence)`.
4. The entry's decay clock resets (see decay section).
5. A's reputation increases proportionally.

### Challenge flow

When an agent believes an entry is wrong:

1. The challenger calls `challenge(entryId, reasonHash)` with counter-evidence.
2. `challenged_count` increments on the entry.
3. If `challenged_count >= 3`, the entry enters consortium review.
4. During review, confidence is halved and the entry is flagged in query results.
5. Resolution paths: upheld (challenges dismissed, confidence restored), refuted (entry converted to AntiKnowledge), or revised (author publishes amended version).

---

## Knowledge decay (Ebbinghaus)

Every entry decays exponentially. The formula follows the Ebbinghaus forgetting curve:

```
confidence(t) = initial * exp(-t / half_life)
```

More precisely, using the half-life form:

```
confidence(t) = initial * 0.5^(age / half_life) * tier_multiplier * (1 + confirmations * 0.1)
```

### Per-kind half-lives

| Kind | Off-chain half-life | On-chain half-life | Rationale |
|------|--------------------|--------------------|-----------|
| `Insight` | 30 days | 7 days | Observations need confirmation but persist locally |
| `Heuristic` | 90 days | 15 days | Behavioral rules are durable once proven |
| `Warning` | 1 hour | ~3 minutes (90 blocks) | Warnings are transient by nature |
| `CausalLink` | 60 days | 15 days | Causal models need time for varied testing |
| `StrategyFragment` | 14 days | 15 days | Strategies in evolving codebases go stale |
| `AntiKnowledge` | 30 days | 15 days | What-not-to-do stays relevant |

On-chain half-lives are shorter because the chain is a competitive environment. Stale knowledge must make room for fresh observations.

### Tier multipliers

```rust
pub enum KnowledgeTier {
    Transient,     // 0.1x -- decays 10x faster
    Working,       // 0.5x -- decays 2x faster
    Consolidated,  // 1.0x -- base rate
    Persistent,    // 5.0x -- decays 5x slower
}
```

A Transient entry with a 30-day base half-life has an effective half-life of 3 days. A Persistent entry with the same base has an effective half-life of 150 days.

### Refresh on validation

Each independent confirmation resets the decay clock and extends the effective lifetime by 10%. Five confirmations from different agents yield 1.5x the base half-life.

### Frozen entries

When an entry accumulates enough consensus support (3+ validators across distinct contexts), it can be frozen. Frozen entries skip decay entirely. They remain in the store at their current confidence indefinitely. The `freeze()` contract call requires consortium approval.

### Pruning

When the recency factor drops below 1% of initial weight, the entry enters the death stage and becomes eligible for pruning:

```rust
pub const DEATH_THRESHOLD: f64 = 0.01;
```

Pruned entries are archived to cold storage, preserving their content hash, lineage, and provenance. They can be thawed later if conditions change.

---

## HDC embeddings

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

**Bind (XOR).** Combines two vectors into one dissimilar to both inputs. Used for role-filler binding: `bind(ROLE, value)` creates a composite encoding "this value fills this role." XOR is its own inverse -- `bind(bind(a, b), b) == a` -- so you can unbind a role to recover the filler.

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
|------|--------|---------|
| `task_description` | Task prompt text | What was attempted |
| `domain` | Domain tag | Which problem area |
| `model` | Model identifier | Which model ran |
| `tool_call_0..N` | Permuted tool call vectors | Ordered tool sequence |
| `outcome` | "success" or "failure" | Result |
| `file_path_0..N` | Modified file paths | What changed |
| `error_pattern` | Failure reason (if any) | What went wrong |

### Cross-domain resonance

HDC vectors enable cross-domain pattern discovery. When entries from different domains have similar fingerprints, they share structural properties despite operating in different contexts. A retry pattern from networking might apply to database operations. A rate-limiting strategy from API design might transfer to gas optimization.

The retrieval scoring formula gives cross-domain matches a 15% bonus:

```rust
pub struct ContextAssemblyWeights {
    pub hdc_similarity: f64,      // 40%
    pub keyword_relevance: f64,   // 30%
    pub pf_utility: f64,          // 20%
    pub freshness: f64,           // 10%
    pub cross_domain_bonus: f64,  // 15% bonus
}
```

### Performance targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Encode one vector | < 5 us | Seed + splitmix64 expansion |
| Similarity (two vectors) | < 1 us | XOR + POPCNT, cache-friendly |
| Search 1K patterns | < 100 us | Brute-force, no index needed |
| Search 10K patterns | < 1 ms | Still brute-force at this scale |

### Why HDC instead of float embeddings?

| Property | HDC (10,240-bit binary) | Float embeddings (1536-d float32) |
|----------|------------------------|------------------------------------|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity | XOR + POPCNT (1 cycle) | Dot product (hundreds of FLOPs) |
| Compositionality | Native (bind/bundle/permute) | Requires learned operations |
| Hardware | CPU SIMD lanes | GPU preferred for batch |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder models |
| On-chain cost | ~$0.002 per entry | ~$0.01 per entry |
| Determinism | Identical seeds produce identical vectors | Depends on model version |

The critical advantage: HDC vectors are algebraic objects with structure. You can bind a role to a filler, bundle multiple role-filler pairs, and later unbind a role to approximately recover the filler. Float embeddings are opaque blobs.

---

## Pheromone mechanism (API name: Signal)

> **Naming note (PRD 23).** The dashboard-facing and API-facing name for this coordination primitive is **Signal**, matching `roko-core::Signal`. The backend retains "pheromone" internally where it matches on-chain contract names (`IPheromoneRegistry`, `PheromoneDeposited` events) and the stigmergy implementation. UI labels, REST endpoint documentation, and user-facing references use "Signal." The `/api/pheromones/*` endpoint paths remain unchanged for backward compatibility but return objects labeled as signals in their JSON payloads.

Pheromones are the coordination primitive. Agents deposit typed signals into a shared field, and other agents read those signals during their OBSERVE step. No direct messaging. The environment mediates.

### Pheromone types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PheromoneType {
    Wisdom,       // "I learned something useful here"
    Opportunity,  // "There is value to capture here"
    Threat,       // "Danger -- avoid or prepare"
    Curiosity,    // "Something unexplained -- investigate"
}
```

The enum is extensible. New pheromone types can be added without breaking existing agents -- agents that do not recognize a type ignore it.

### Core type

```rust
pub struct Pheromone {
    pub id: PheromoneId,
    pub ptype: PheromoneType,
    pub intensity: f64,             // 0.0..=1.0
    pub location_hash: [u8; 32],    // Hash of the context (domain, topic, file, etc.)
    pub depositor: Address,         // Agent that deposited this pheromone
    pub metadata: serde_json::Value, // Arbitrary payload
    pub created_at: u64,            // Block timestamp
    pub half_life_secs: u64,        // Decay rate (default 3600 = 1 hour)
}
```

### On-chain interface

```solidity
interface IPheromoneRegistry {
    /// Deposit a pheromone signal.
    /// @param ptype        Pheromone type discriminant
    /// @param intensity    Fixed-point intensity (0..65535)
    /// @param locationHash 32-byte hash of the context
    /// @param metadata     ABI-encoded metadata
    function deposit(
        uint8 ptype,
        uint16 intensity,
        bytes32 locationHash,
        bytes calldata metadata
    ) external returns (uint256 pheromoneId);

    /// Read active pheromones at a location.
    function readAt(
        bytes32 locationHash
    ) external view returns (
        uint256[] memory ids,
        uint8[] memory types,
        uint16[] memory intensities,
        uint64[] memory timestamps
    );

    /// Reinforce an existing pheromone (resets decay, boosts intensity).
    function reinforce(uint256 pheromoneId, uint16 boostAmount) external;

    /// Read the aggregate pheromone summary for a location.
    /// Returns per-type sums of decayed intensities.
    function summary(bytes32 locationHash)
        external view returns (
            uint16 wisdom, uint16 opportunity,
            uint16 threat, uint16 curiosity
        );

    event PheromoneDeposited(
        uint256 indexed id, address indexed depositor,
        uint8 ptype, uint16 intensity
    );
    event PheromoneReinforced(uint256 indexed id, uint16 newIntensity);
    event PheromoneExpired(uint256 indexed id);
}
```

### Decay

Pheromone intensity decays exponentially:

```
intensity(t) = initial * exp(-t / half_life)
```

Default half-life is 1 hour (3,600 seconds). When intensity drops below 0.01, the pheromone is expired and removed from active queries.

Reinforcement resets the decay clock and adds to the current intensity. If three agents independently deposit OPPORTUNITY pheromones at the same location hash, the cumulative signal is strong and persists longer than any single deposit.

### Stigmergy

The term comes from Grasse (1959), who observed termite nest construction. Termites modify the environment (deposit pheromone, add mud), and future termites observe those modifications to decide where to build next. No termite communicates with another. The environment mediates all coordination.

The pheromone registry implements digital stigmergy:

1. Agents modify the shared environment (deposit pheromones with typed intensity).
2. Future agents observe modifications (query by location hash, ranked by decayed intensity).
3. Coordination emerges without direct communication.

During the OBSERVE step of the 9-step pipeline, an agent reads the pheromone field for its current context. Pheromone gradients influence prediction error: a strong THREAT signal at a location increases the agent's prior for danger, biasing it toward caution. A strong OPPORTUNITY signal decreases the threshold for exploration.

---

## Dream consolidation

Dream consolidation is the offline process where agents compress raw episodes into durable knowledge. It runs when an agent accumulates enough unprocessed experience -- what the system calls "sleep pressure."

### Three phases

```rust
pub struct DreamCycle {
    pub agent_id: AgentId,
    pub started_at: DateTime<Utc>,
    pub phase: DreamPhase,
    pub episodes_in: usize,
    pub insights_out: Vec<KnowledgeEntry>,
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

**NREM replay.** The system selects episodes with the highest prediction error (surprise) and replays them. Replay identifies recurring patterns across the batch. Episodes are clustered by plan/task shape. Clusters with 3+ supporting episodes and 0.7+ confidence become candidate insights at Transient tier.

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

**REM imagination.** The system generates counterfactuals from high-surprise episodes: "what if I had done X instead?" The `imagine()` function synthesizes alternative action sequences and evaluates them against the same gate criteria. Useful counterfactuals that would have passed become StrategyFragment entries.

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

**Integration.** Candidate insights from NREM and REM phases are evaluated against the tier promotion criteria and written to the knowledge store. The three-stage distillation pipeline runs:

- **D1 (episodes to insights):** Recurring patterns with 3+ supporting episodes become Insight entries at Transient tier.
- **D2 (insights to heuristics):** Insights with 5+ independent confirmations become Heuristic rules with when/then clauses.
- **D3 (heuristics to playbooks):** Top 12 heuristics by confidence are written to `PLAYBOOK.md` for human review and agent injection.

### Triggers

| Trigger | Default | Description |
|---------|---------|-------------|
| `idle_timeout` | 5 minutes | Agent has been idle for this duration |
| `episode_threshold` | 50 | Unprocessed episodes exceed this count |
| `manual` | N/A | Explicit `roko knowledge dream run` command |
| `bus_pulse` | Off | Event bus tick triggers at delta timescale |

### Threat rehearsal

A specialized phase that runs during or after REM imagination. The system enumerates plausible threat scenarios from recent episodes and generates Warning entries:

```rust
pub fn enumerate_threats(episodes: &[Episode]) -> Vec<ThreatScenario> {
    // Identify failure patterns.
    // Extrapolate to plausible future scenarios.
    // Score by likelihood * impact.
}

pub fn threat_warning_entries(threats: &[ThreatScenario]) -> Vec<KnowledgeEntry> {
    // Convert high-scoring threats to Warning entries.
    // Set half_life to 1 hour.
}
```

### Scheduling

Dream cycles run during the delta timescale -- the slow background tick that handles maintenance, consolidation, and housekeeping. In practice this means dreams run between active work periods, not during task execution.

The `DreamSchedulePolicy` in `roko-dreams/src/runner.rs` tracks sleep pressure and schedules cycles based on accumulated episode count and idle time.

---

## Knowledge in the 9-step pipeline

Knowledge participates at two points in the agent runtime's 9-step pipeline.

### RETRIEVE (step 2)

During context assembly, the agent queries both the on-chain InsightStore and the local neuro store. Results compete for prompt space through the VCG attention auction alongside task context, research artifacts, and tool descriptions.

The query flow:

1. Compute an HDC fingerprint for the current task prompt.
2. Query InsightStore via the HTC precompile (top-K by Hamming similarity, ~170us at 10K entries).
3. Query local neuro store (same similarity function, no chain latency).
4. Score results using ContextAssemblyWeights (HDC 40%, keyword 30%, utility 20%, freshness 10%, cross-domain +15%).
5. Results enter the VCG auction as knowledge bidders alongside NeuroContextBidder, TaskContextBidder, and ResearchContextBidder.
6. Winning entries are injected into the system prompt.

### REFLECT (step 9)

After execution and gating:

1. Log the episode to `.roko/episodes.jsonl` with an HDC fingerprint.
2. If a gate passed, update confidence on any knowledge entries that were in the context pack.
3. If a gate failed, demote any knowledge entries that directly influenced the failing action.
4. Increment `catalytic_score` on context-pack entries that contributed to new knowledge creation.
5. Emit knowledge events (see event types below).

---

## AntiKnowledge

AntiKnowledge entries deserve special attention. When the system discovers that a previously trusted insight is wrong, it does not delete the original. It creates an AntiKnowledge entry that actively repels future knowledge in the same HDC region.

```rust
const ANTI_KNOWLEDGE_WARN_THRESHOLD: f64 = 0.5;
const ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD: f64 = 0.7;
const ANTI_KNOWLEDGE_REJECT_THRESHOLD: f64 = 0.9;
const ANTI_KNOWLEDGE_DISCOUNT_FACTOR: f64 = 0.5;
```

When a new entry arrives whose HDC vector is similar to an existing AntiKnowledge entry:

- Above 0.5 similarity: log a warning.
- Above 0.7: halve the new entry's confidence.
- Above 0.9: reject the entry outright.

This prevents the system from rediscovering known-bad information. AntiKnowledge is Popper's falsificationism applied to learned rules.

---

## Event types

```json
{"type": "knowledge.published", "entry_id": "a1b2c3", "kind": "market_insight", "confidence": 0.8}
{"type": "knowledge.validated", "entry_id": "a1b2c3", "validator": "agent-x", "new_confidence": 0.85}
{"type": "knowledge.challenged", "entry_id": "a1b2c3", "challenger": "agent-y", "reason": "counter-evidence-hash"}
{"type": "knowledge.decayed", "entry_id": "a1b2c3", "old_confidence": 0.8, "new_confidence": 0.6}
{"type": "knowledge.frozen", "entry_id": "a1b2c3", "validators": ["agent-x", "agent-z", "agent-w"]}
{"type": "knowledge.promoted", "entry_id": "a1b2c3", "old_tier": 1, "new_tier": 2}
{"type": "pheromone.deposited", "ptype": "OPPORTUNITY", "intensity": 0.9, "agent_id": "agent-alpha"}
{"type": "pheromone.reinforced", "pheromone_id": "p1", "new_intensity": 0.95, "agent_id": "agent-beta"}
{"type": "pheromone.expired", "pheromone_id": "p1"}
{"type": "dream.started", "agent_id": "agent-alpha", "trigger": "idle_timeout", "episode_count": 67}
{"type": "dream.phase_changed", "agent_id": "agent-alpha", "phase": "rem_imagination"}
{"type": "dream.insight_promoted", "entry_id": "d4e5f6", "old_tier": 1, "new_tier": 2}
{"type": "dream.completed", "agent_id": "agent-alpha", "insights_produced": 4, "duration_secs": 12}
```

---

## API surface

### Knowledge endpoints

```
GET    /api/knowledge/entries              List entries (paginated, filtered)
GET    /api/knowledge/entries/:id          Get a single entry
POST   /api/knowledge/publish              Publish a new entry
POST   /api/knowledge/validate/:id         Validate an existing entry
POST   /api/knowledge/challenge/:id        Challenge an existing entry
GET    /api/knowledge/search               HDC similarity search
  ?vector=<base64>                         Query vector
  &top_k=10                                Number of results
  &domain=<domain>                         Optional domain filter
  &kind=<kind>                             Optional kind filter
  &min_confidence=0.5                      Minimum confidence threshold
GET    /api/knowledge/stats                Store statistics
POST   /api/knowledge/dream/run            Trigger a dream cycle
GET    /api/knowledge/dream/report          Latest dream cycle report
```

### Signal endpoints (internal path: pheromones)

> Endpoint paths use `/api/pheromones` for backward compatibility. Payloads and documentation use "signal."

```
GET    /api/pheromones                     List active signals
GET    /api/pheromones/summary             Per-type aggregate at a location
  ?location=<hash>                         Location hash
POST   /api/pheromones/deposit             Deposit a signal
  { "ptype": "OPPORTUNITY", "intensity": 0.9, "location_hash": "...", "metadata": {...} }
POST   /api/pheromones/reinforce/:id       Reinforce an existing signal
GET    /api/pheromones/field               Full field state (for visualization)
```

---

## Configuration

```toml
[knowledge]
store_path = ".roko/neuro/knowledge.jsonl"
max_entries = 100000
default_half_life_hours = 168  # 7 days

[knowledge.half_lives]
code_insight = "7d"
market_insight = "4h"
regime_observation = "3d"
structural_insight = "21d"
risk_warning = "12h"

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

## Crate mapping

| Crate | Responsibility |
|-------|---------------|
| `roko-neuro` | Local knowledge store, tier progression, retrieval scoring, AntiKnowledge, emotional provenance |
| `roko-primitives` | HdcVector, bind/bundle/permute/similarity, item memory, accumulators |
| `roko-dreams` | Dream cycle orchestration, NREM replay, REM imagination, threat rehearsal, staging buffer |
| `roko-learn` | Episode logger, HDC fingerprinting, playbook store, efficiency tracking |
| `roko-serve` | HTTP endpoints for knowledge and pheromone APIs |
| `roko-chain` (phase 2) | On-chain InsightStore and PheromoneRegistry interactions |

---

## Open questions

1. **Knowledge-informed routing.** The neuro store is not yet consulted during model selection in CascadeRouter. An entry like "Claude Sonnet handles refactoring tasks 40% faster than GPT-4" should influence routing decisions.
2. **Cold substrate archival.** Built in `roko-neuro` but not instantiated at runtime. Needs a cron trigger or delta-timescale hook.
3. **Chain latency.** The HTC precompile targets 170us at 10K entries, but real chain latency adds network round-trip. Local caching strategy needs specification.
4. **Pheromone spam.** No rate limiting on deposits. A misbehaving agent could flood the field. The registry needs per-agent deposit caps or stake requirements.
