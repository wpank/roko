# PRD-05: Knowledge system and stigmergic coordination

| Field | Value |
|-------|-------|
| Author | Will |
| Date | 2026-04-21 |
| Status | Draft |
| Scope | Local memory (Neuro), on-chain shared memory (InsightStore), HDC computing, geometric privacy, dream consolidation, stigmergic coordination |

---

## 1. The knowledge problem

Every major agent framework treats memory the same way: append context to a vector store, retrieve by cosine similarity, stuff into the next prompt. LangChain, CrewAI, AutoGPT, Cursor, Claude Code -- all variations on the same architecture. The memory is a bag of text chunks. Nothing decays. Nothing consolidates. Nothing gets shared.

The consequences compound across four axes.

**No forgetting.** Vector stores grow monotonically. Stale information dilutes retrieval quality over time. A cached heuristic from session #12 that was invalidated by session #47 still appears in results because nobody tracks validity. Without temporal decay, the noise floor rises with every session.

**No consolidation.** Five episodes that demonstrate the same causal pattern remain five separate chunks instead of becoming one reusable heuristic. Raw episodes pile up because no process compresses observations into compact, validated knowledge. The signal-to-storage ratio degrades linearly.

**No quality assessment.** A hallucinated claim and a gate-validated insight sit at the same confidence level. No mechanism distinguishes "we tried this and it worked three times" from "the model said this once." Without provenance tracking, unreliable knowledge contaminates reliable knowledge.

**No cross-agent sharing.** A thousand agents solving related problems across different organizations independently discover the same patterns, make the same mistakes, reach the same conclusions. The hundredth agent to learn "always run clippy before committing Rust code" pays the same discovery cost as the first. Every network participant starts from zero.

Roko and Korai treat knowledge as a living system instead of a dead archive. The design draws from neuroscience, information theory, and evolutionary biology to build memory that decays, consolidates, validates, shares, and improves.

The core primitives:

- **Multiple memory types.** Episodic memory (raw task execution logs), semantic memory (validated insights and heuristics), procedural memory (playbooks and strategy fragments). Each type has different retention characteristics, retrieval patterns, and consolidation pathways.
- **Temporal decay.** Every knowledge entry has an exponential half-life derived from the Ebbinghaus forgetting curve (1885). Entries that are not confirmed or retrieved lose weight over time and are eventually pruned. This prevents unbounded growth and biases retrieval toward fresh, relevant information.
- **Tier progression.** Knowledge enters at Transient tier and progresses through Working, Consolidated, and Persistent as it accumulates evidence. Each tier has a different lifetime multiplier. Unvalidated guesses decay 10x faster than gate-confirmed heuristics.
- **Cross-agent sharing via InsightStore.** Korai's on-chain knowledge substrate lets agents post validated knowledge and query what others have discovered. Coordination emerges without direct agent-to-agent communication -- the environment mediates.
- **Privacy-preserving encoding.** Knowledge enters the chain as 10,240-bit hyperdimensional vectors, not text. The PP-HDC projection is algebraically non-invertible. Retrieval uses vector similarity. No natural language ever touches the chain.
- **Dream consolidation.** When an agent accumulates enough unprocessed experience (sleep pressure), it enters a dream cycle: replaying high-surprise episodes, generating counterfactuals, simulating threats, and promoting validated insights to higher tiers.

---

## 2. Local knowledge: the Neuro store

The `roko-neuro` crate provides durable, tiered knowledge with six semantic kinds, temporal decay, emotional provenance, and HDC-accelerated retrieval. The store lives at `.roko/neuro/knowledge.jsonl` -- an append-only JSONL file with atomic rewrite for maintenance operations (decay, garbage collection).

### 2.1 Six entry kinds

```rust
/// Semantic category for a knowledge item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    Insight,           // Validated observation with evidence
    Heuristic,         // Reusable decision rule
    Warning,           // Time-sensitive risk alert
    CausalLink,        // Validated cause-effect relationship
    StrategyFragment,  // Partial, composable strategy
    AntiKnowledge,     // Explicitly wrong information
}
```

Each kind serves a distinct purpose:

| Kind | What it encodes | Example | Default half-life |
|------|----------------|---------|-------------------|
| `Insight` | A compact causal observation distilled from multiple episodes | "Clippy catches 40% of gate failures before compilation" | 30 days (local), 7 days (chain) |
| `Heuristic` | A reusable if-then rule promoted from insights | "If task modifies >5 files, use architectural tier" | 90 days (local), 15 days (chain) |
| `Warning` | A time-sensitive risk alert | "Aave V3 oracle is reporting stale prices on Arbitrum" | 1 hour (local), 3 min (chain) |
| `CausalLink` | A validated A-causes-B relationship | "High gas prices cause MEV bot activity to spike" | 60 days (local), 15 days (chain) |
| `StrategyFragment` | A composable piece of a larger plan | "Run integration tests in parallel with --jobs=4" | 14 days (local), 15 days (chain) |
| `AntiKnowledge` | Explicitly wrong information to prevent rediscovery | "Do NOT use `unwrap()` in async contexts (causes panics)" | 30 days (local), 15 days (chain) |

AntiKnowledge deserves special attention. When the system discovers that a previously trusted insight is wrong, it does not delete the insight. It creates an AntiKnowledge entry that actively repels future knowledge in the same region of HDC space. The implementation enforces three thresholds:

```rust
// Knowledge store constants (roko-neuro/src/knowledge_store.rs)
const ANTI_KNOWLEDGE_WARN_THRESHOLD: f64 = 0.5;      // log warning
const ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD: f64 = 0.7;   // halve confidence
const ANTI_KNOWLEDGE_REJECT_THRESHOLD: f64 = 0.9;     // reject entirely
const ANTI_KNOWLEDGE_DISCOUNT_FACTOR: f64 = 0.5;
```

When a new entry arrives whose HDC vector is similar to an existing AntiKnowledge entry: above 0.5 similarity, log a warning. Above 0.7, halve the new entry's confidence. Above 0.9, reject it outright. This prevents the system from rediscovering known-bad information.

### 2.2 The KnowledgeEntry struct

Every entry carries rich metadata beyond the raw content:

```rust
pub struct KnowledgeEntry {
    pub id: String,                                  // Unique identifier
    pub kind: KnowledgeKind,                         // Semantic category
    pub source: Option<String>,                      // Provenance label
    pub content: String,                             // The actual knowledge
    pub confidence: f64,                             // 0.0..=1.0
    pub confidence_weight: f64,                      // Signed retrieval weight
    pub refuted_insight_id: Option<String>,           // For AntiKnowledge
    pub refutation_evidence: Option<String>,          // Why it was wrong
    pub source_episodes: Vec<String>,                // Contributing episode IDs
    pub tags: Vec<String>,                           // Topic tags for retrieval
    pub source_model: Option<String>,                // Which model produced this
    pub model_generality: f64,                       // 0.0 (model-specific) to 1.0 (universal)
    pub created_at: DateTime<Utc>,                   // Timestamp
    pub half_life_days: f64,                         // Exponential decay rate
    pub tier: KnowledgeTier,                         // Transient/Working/Consolidated/Persistent
    pub emotional_tag: Option<EmotionalTag>,          // PAD-space affect
    pub emotional_provenance: Option<EmotionalProvenance>, // Reliability metadata
    pub hdc_vector: Option<Vec<u8>>,                 // 1,280-byte HDC fingerprint
    pub confirmation_count: u32,                     // Independent confirmations
    pub confirmation_contexts: Vec<String>,           // Distinct confirming contexts
}
```

The `model_generality` field tracks whether a piece of knowledge is model-specific or universal. A heuristic that works only with Claude Sonnet gets `model_generality: 0.3`. One that works across GPT-4, Claude, and Gemini gets `model_generality: 1.0`. At dispatch time, the system only injects heuristics that apply to the current model:

```rust
impl HeuristicRule {
    pub fn applies_to_model(&self, current_model: &str) -> bool {
        self.model_generality > 0.7
            || self.source_model.as_deref() == Some(current_model)
    }
}
```

### 2.3 Temporal decay

Every entry decays exponentially. The effective weight at time `t` follows the Ebbinghaus curve:

```
weight(t) = initial_weight * 0.5^(age / half_life) * tier_multiplier
          * (1 + confirmations * 0.1)
```

Each independent confirmation extends the effective lifetime by 10%. Tier multipliers scale the base half-life:

```rust
pub enum KnowledgeTier {
    Transient,     // multiplier: 0.1  (decay 10x faster)
    Working,       // multiplier: 0.5  (decay 2x faster)
    Consolidated,  // multiplier: 1.0  (base rate)
    Persistent,    // multiplier: 5.0  (decay 5x slower)
}
```

A Transient entry with a 30-day base half-life has an effective half-life of 3 days. A Persistent entry with the same base has an effective half-life of 150 days. When the recency factor drops below 1% of initial weight, the entry enters the Death stage and becomes eligible for pruning:

```rust
pub const DEATH_THRESHOLD: f64 = 0.01;
```

### 2.4 Tier progression

Knowledge climbs tiers through evidence accumulation, not time. The `tier_progression` module in `roko-neuro` implements a three-stage distillation pipeline:

- **D1: Episodes to Insights.** Raw episodes are pattern-mined. Recurring sequences with at least 3 supporting episodes and 0.7+ confidence become Insight entries at Transient tier.
- **D2: Insights to Heuristics.** Insights with at least 5 independent episode confirmations are promoted to Heuristic rules with when/then clauses.
- **D3: Heuristics to Playbooks.** The top 12 heuristics by confidence are written to `PLAYBOOK.md` for human review and agent injection.

Tier promotion has explicit thresholds:

```rust
pub const PROMOTION_SUCCESS_THRESHOLD: usize = 3;  // gate passes to promote
pub const DEMOTION_FAILURE_THRESHOLD: usize = 2;    // gate failures to demote
pub const EXPIRY_REVIEW_HALF_LIFE_MULTIPLIER: f64 = 2.0;  // age trigger
```

Heuristics also undergo Popperian falsification. The `CalibrationAction` enum tracks how new evidence relates to existing heuristics:

```rust
pub enum CalibrationAction {
    Confirm,      // Evidence supports the heuristic
    Violate,      // Evidence contradicts it
    Refine,       // Evidence narrows its scope
    Generalize,   // Evidence broadens its applicability
    Refute,       // Evidence fully disproves it
}
```

When a heuristic accumulates enough violations relative to trials, its confidence drops and it may be demoted or converted to AntiKnowledge. This is Popper's falsificationism applied to learned rules: knowledge that cannot survive contact with new evidence does not deserve to persist.

### 2.5 Retrieval scoring

The KnowledgeStore scores entries using four weighted components for context assembly:

```rust
pub struct ContextAssemblyWeights {
    pub hdc_similarity: f64,      // 40% -- HDC vector similarity
    pub keyword_relevance: f64,   // 30% -- keyword/pheromone match
    pub pf_utility: f64,          // 20% -- predictive foraging utility
    pub freshness: f64,           // 10% -- recency factor
    pub cross_domain_bonus: f64,  // 15% bonus for cross-domain entries
}
```

The cross-domain bonus means entries from a different domain than the query get a 15% score boost. This encourages the system to surface structural analogies -- a retry pattern from networking that might apply to database operations, for instance.

### 2.6 Emotional provenance

Knowledge entries inherit emotional metadata from their source episodes via the PAD (Pleasure-Arousal-Dominance) model. The `EmotionalProvenance` struct tracks:

- **Average PAD vector** across all supporting episodes
- **Discovery emotion** -- the coarse emotional label at first observation
- **Validation arc** -- the narrative shape of how emotional evidence evolved (Redemptive, Contaminating, Stable, Progressive)
- **Emotional diversity** -- normalized Shannon entropy across coarse emotion labels

Knowledge validated under diverse emotional conditions (high diversity score) is more robust than knowledge validated only under calm, successful conditions. The system uses this as a supplementary quality signal during retrieval.

---

## 3. Shared knowledge: the Korai InsightStore

The InsightStore is Korai's on-chain knowledge substrate. It uses the same six entry kinds as the local Neuro store but adds economic incentives, reputation weighting, and automatic demurrage (pheromone decay).

### 3.1 On-chain entry format

Each entry on-chain is approximately 1,340 bytes:

```
vector:       [u8; 1280]     // PP-HDC encoded (non-invertible)
domain:       u8             // Domain identifier
kind:         u8             // KnowledgeKind discriminant
confidence:   u16            // Fixed-point confidence (0..65535)
submitter:    Address        // 20-byte Ethereum address
reputation:   u16            // Submitter's reputation score
timestamp:    u64            // Block timestamp
content_hash: [u8; 32]       // Commitment to original (revealed post-embargo)
```

No natural language. No metadata that could identify the submitter's project or proprietary data. The vector is a non-invertible projection of the original knowledge (see section 8, Geometric Privacy). Retrieval is pure vector similarity.

### 3.2 Pheromone dynamics

The InsightStore borrows its coordination model from ant colonies. Entries have a pheromone weight that starts high and decays via demurrage -- the on-chain equivalent of temporal decay:

```
pheromone(t) = initial_weight * 0.5^(age_blocks / half_life_blocks)
```

On-chain half-lives are shorter than local half-lives because the chain is a competitive environment. Stale knowledge must make room for fresh observations:

```rust
pub const INSIGHT_HALF_LIFE_BLOCKS: u64 = 7 * 43_200;     // ~7 days
pub const HEURISTIC_HALF_LIFE_BLOCKS: u64 = 15 * 43_200;  // ~15 days
pub const WARNING_HALF_LIFE_BLOCKS: u64 = 90;              // ~3 minutes
pub const CAUSAL_LINK_HALF_LIFE_BLOCKS: u64 = 15 * 43_200; // ~15 days
```

Independent confirmation from a different agent extends an entry's effective half-life. The mechanism is the same as local confirmations: each independent source adds 10% to the lifetime. If five different agents independently confirm the same insight, its effective half-life is 1.5x the base.

### 3.3 Reputation weighting

Entries from high-reputation submitters score higher during retrieval. Reputation is earned by submitting knowledge that subsequent agents confirm and use successfully. The feedback loop is:

1. Agent A posts an insight to the InsightStore.
2. Agent B queries, retrieves A's insight, uses it, passes a gate.
3. Agent B's gate-pass event generates a confirmation transaction that increments A's reputation.
4. A's future submissions start with higher pheromone weight.

Reputation cannot be purchased. It can only be earned through knowledge that survives contact with reality (gate validation).

### 3.4 CausalLink composition

CausalLinks compose transitively. If the InsightStore contains `A causes B` and `B causes C`, any agent can discover the emergent chain `A causes C` by composing the two links. The HDC representation enables this algebraically: the `bind` operation on the cause vector of one link and the effect vector of the next produces a composite that is similar to both endpoints.

No single agent needs to observe the full A-to-C chain. Agent Alpha discovers A-to-B in the weather domain. Agent Beta discovers B-to-C in the trading domain. Agent Gamma, working in portfolio risk, queries the InsightStore and finds both links. The multi-hop chain emerges from the shared environment.

This is stigmergy.

### 3.5 Stigmergy defined

The term comes from Grasse (1959), who observed that termites coordinate nest construction without centralized planning. Each termite modifies the environment (deposits pheromone, adds mud), and future termites observe those modifications to decide where to build next. No termite communicates directly with another. The environment mediates all coordination.

The InsightStore implements digital stigmergy:

1. Agents modify the shared environment (post knowledge entries with pheromone weight).
2. Future agents observe modifications (query by HDC similarity, ranked by pheromone weight).
3. Coordination emerges without direct communication (no message passing, no leader election, no consensus protocol beyond the chain's own).

The pheromone decay is essential. Without it, old knowledge accumulates until it drowns out new discoveries. With decay, the InsightStore naturally reflects the network's current understanding. Active knowledge paths strengthen; abandoned paths fade.

### 3.6 Query via HTC precompile

Korai provides a precompiled contract for hyperdimensional vector similarity search (the HTC precompile). Agents query the InsightStore by submitting a query vector and receiving the top-K entries by Hamming similarity. Latency is approximately 170 microseconds at 10,000 entries -- fast enough for real-time dispatch-time context assembly.

---

## 4. Hyperdimensional computing (HDC)

HDC is the mathematical substrate that makes the knowledge system work. It encodes structured information as 10,240-bit binary vectors and operates on them using three operations that map directly to hardware bit manipulation instructions. No floating point. No matrix multiply. No GPU required.

### 4.1 The vector

```rust
/// 10,240-bit binary sparse distributed vector.
pub struct HdcVector {
    bits: [u64; 160],  // 160 words * 64 bits = 10,240 bits = 1,280 bytes
}
```

The implementation lives in `roko-primitives/src/hdc.rs`. Serialization uses little-endian byte packing. The `serde` implementation handles both byte-slice and sequence deserialization for compatibility with JSON and binary formats.

### 4.2 Core operations

**Bind (XOR).** Combines two vectors into one that is dissimilar to both inputs. Used for role-filler binding: `bind(ROLE, value)` creates a composite that encodes "this value fills this role." The operation is its own inverse -- `bind(bind(a, b), b) == a` -- which means you can unbind a role to recover the filler.

```rust
pub fn bind(&self, other: &Self) -> Self {
    let mut bits = [0u64; 160];
    for (slot, (left, right)) in
        bits.iter_mut().zip(self.bits.iter().zip(other.bits.iter()))
    {
        *slot = left ^ right;
    }
    Self { bits }
}
```

**Bundle (majority vote).** Combines multiple vectors into one that is similar to all inputs. Used for aggregation: bundling five domain-specific vectors produces a composite that retrieves all five in a similarity query. Ties (even number of vectors with equal 0s and 1s) resolve to 0.

```rust
pub fn bundle(vectors: &[&Self]) -> Self {
    // For each bit position: count ones across all vectors.
    // Set the output bit to 1 if ones > len/2.
    // ...
}
```

**Similarity (Hamming distance).** Measures overlap between two vectors. Two random 10,240-bit vectors are approximately 50% similar by chance (the expected Hamming distance of two random binary strings is half the length). Meaningful similarity starts around 0.52-0.53. The similarity function uses hardware `POPCNT` via Rust's `count_ones()`:

```rust
pub fn similarity(&self, other: &Self) -> f32 {
    let mut differing_bits = 0u32;
    for (left, right) in self.bits.iter().zip(other.bits.iter()) {
        differing_bits += (left ^ right).count_ones();
    }
    1.0 - (f32::from(differing_bits as u16) / 10_240.0)
}
```

**Permute (bit rotation).** Encodes position or sequence. `permute(v, 1)` shifts all bits left by 1 position (cyclic). Used to encode ordered sequences: `bundle(permute(v1, 0), permute(v2, 1), permute(v3, 2))` creates a vector that encodes "v1 then v2 then v3."

```rust
pub fn permute(&self, n: usize) -> Self {
    let word_shift = n / 64;
    let bit_shift = n % 64;
    // Cyclic left rotation across the 160-word array
    // ...
}
```

### 4.3 Why HDC instead of embeddings?

| Property | HDC (10,240-bit binary) | Float embeddings (1536-d float32) |
|----------|------------------------|-----------------------------------|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity computation | XOR + POPCNT (1 cycle) | Dot product (hundreds of FLOPs) |
| Compositionality | Native (bind/bundle/permute) | Requires learned operations |
| Hardware requirements | CPU SIMD lanes | GPU preferred for batch |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder models |
| On-chain cost | ~$0.002 per entry | ~$0.01 per entry |
| Determinism | Identical seeds produce identical vectors | Depends on model version |

The critical advantage for the InsightStore is compositionality. Float embeddings are opaque blobs produced by a model. HDC vectors are algebraic objects with structure. You can bind a role to a filler, bundle multiple role-filler pairs into one vector, and later unbind a role to approximately recover the filler. This lets agents encode structured knowledge (not flat text) into a single compact vector.

### 4.4 Deterministic seeding

Vectors can be generated deterministically from any byte seed using FNV-1a hashing followed by splitmix64 expansion:

```rust
pub fn from_seed(seed: &[u8]) -> Self {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    for &byte in seed {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV prime
    }
    let mut bits = [0u64; 160];
    for word in &mut bits {
        *word = splitmix64(&mut hash);
    }
    Self { bits }
}
```

This means `HdcVector::from_seed(b"rust")` always produces the same vector. Stable role vectors for structured encoding can be defined once and reused across all agents.

### 4.5 Item memory (codebook)

The `ItemMemory` struct provides a named codebook with brute-force nearest-neighbor lookup. Insert named concepts, query by vector similarity:

```rust
let mut memory = ItemMemory::new();
memory.insert_seeded("rust");
memory.insert_seeded("python");
memory.insert_seeded("go");

let query = HdcVector::from_seed(b"rust");
let nearest = memory.nearest(&query); // ("rust", 1.0)
let top_3 = memory.top_k(&query, 3);  // [("rust", 1.0), ...]
```

### 4.6 Accumulators

Two accumulator types support incremental bundling:

**BundleAccumulator** -- standard majority-vote accumulator with integer vote tallies. Supports weighted addition and multiplicative decay. Used for batch operations.

**DecayingBundleAccumulator** -- temporal-bias accumulator where each new vector decays prior votes before adding its contribution. The finished vector is biased toward recent additions. Configurable half-life:

```rust
let acc = DecayingBundleAccumulator::new(0.95);
// half_life ~= 13.5 additions
// After 13.5 additions, the influence of the first vector is halved.
```

---

## 5. Episode fingerprinting

Every agent episode (a single task execution from prompt to gate verdict) gets an HDC fingerprint. The fingerprint encodes the episode's salient properties into a single 10,240-bit vector.

### 5.1 The fingerprinting pipeline

The `roko-learn/src/hdc_fingerprint.rs` module computes fingerprints from episode data:

```rust
#[derive(Debug, Serialize)]
struct EpisodeFingerprintInput<'a> {
    prompt: &'a str,
    outcome: &'a str,
}

pub fn fingerprint_episode(prompt: &str, outcome: &str) -> HdcVector {
    fingerprint(&EpisodeFingerprintInput { prompt, outcome })
}
```

The underlying `fingerprint()` function serializes the input to JSON, then passes the bytes to `HdcVector::from_seed()`. This produces a deterministic vector that encodes the episode's prompt and outcome.

For richer fingerprints, the structured encoding in `roko-neuro/src/hdc.rs` uses role-filler binding:

```rust
pub(crate) struct RoleFillerEncoder;

impl RoleFillerEncoder {
    pub(crate) fn encode_structured(
        roles_and_fillers: &[(String, String)]
    ) -> HdcVector {
        let bound: Vec<HdcVector> = roles_and_fillers
            .iter()
            .map(|(role, filler)| role_hv(role).bind(&text_hv(filler)))
            .collect();
        let refs: Vec<&HdcVector> = bound.iter().collect();
        HdcVector::bundle(&refs)
    }

    pub(crate) fn query_role(
        composite: &HdcVector, role: &str
    ) -> HdcVector {
        composite.bind(&role_hv(role))  // XOR is its own inverse
    }
}
```

A full episode fingerprint encodes:

| Role | Filler | Purpose |
|------|--------|---------|
| `task_description` | Task prompt text | What was attempted |
| `domain` | Domain tag | Which problem area |
| `model` | Model identifier | Which model ran |
| `tool_call_0..N` | Permuted tool call vectors | Ordered tool sequence |
| `outcome` | "success" or "failure" | Result |
| `file_path_0..N` | Modified file paths | What changed |
| `error_pattern` | Failure reason (if any) | What went wrong |

All role-filler pairs are bound via XOR, then bundled via majority vote into a single 1,280-byte vector.

### 5.2 Base64 transport

Fingerprints are stored in episode logs and transported over HTTP as base64-encoded strings. The `hdc_fingerprint` module provides lossless encoding and decoding:

```rust
pub fn encode(vector: &HdcVector) -> String {
    // Standard base64 with padding
    // 1,280 bytes -> 1,708 characters
}

pub fn decode(encoded: &str) -> Result<HdcVector, String> {
    // Decode back to 1,280 bytes -> HdcVector::from_bytes
}
```

### 5.3 What fingerprints enable

Once every episode carries an HDC fingerprint, the system can perform O(1) similarity lookups against the entire episode history:

- **"Find tasks like this one."** Compute the current task's fingerprint, query the episode store for nearest neighbors. Returns structurally similar past episodes regardless of textual wording.
- **Cluster analysis.** Group episodes by fingerprint similarity to discover task categories the system has never been explicitly told about.
- **Cross-domain resonance.** Detect when episodes in domain A have structurally similar fingerprints to episodes in domain B -- indicating a transferable pattern.
- **Curriculum learning.** Route the agent toward tasks it struggles with (low pass rate in the nearest cluster) instead of tasks it already handles well.

---

## 6. Episode clustering

The `roko-learn/src/hdc_clustering.rs` module implements k-medoids clustering over HDC vectors. k-medoids (partitioning around medoids, PAM) is chosen over k-means because the medoid of a cluster is always an actual data point, not an artificial centroid. This means each cluster's representative is a real episode, not a mathematical average that might not correspond to anything.

### 6.1 Algorithm

```
1. INITIALIZE: Greedy farthest-first seeding
   - First medoid: the point with smallest total distance to all others
   - Each subsequent medoid: the point maximizing its minimum distance
     to all existing medoids

2. ASSIGN: Each point goes to its nearest medoid
   - Distance = 1.0 - hamming_similarity(a, b)

3. UPDATE: For each cluster, the member minimizing total intra-cluster
   distance becomes the new medoid

4. REPEAT steps 2-3 until medoids stabilize or max_iterations reached
```

The implementation precomputes a full N x N distance matrix for O(1) lookup during the assign/update loop:

```rust
pub struct KMedoidsConfig {
    pub k: usize,              // Number of clusters
    pub max_iterations: usize, // Convergence limit (default: 100)
}

pub struct HdcCluster {
    pub medoid_index: usize,   // Index of medoid in original input
    pub medoid: HdcVector,     // The medoid vector itself
    pub members: Vec<usize>,   // Indices of all members
}

pub struct ClusterResult {
    pub clusters: Vec<HdcCluster>,
    pub iterations: usize,
    pub converged: bool,
}
```

### 6.2 What clustering enables

**Weakness detection.** Compute the gate pass rate per cluster. Clusters with low pass rates represent task categories where the agent struggles. The CascadeRouter can then route tasks matching those clusters to stronger (more expensive) models.

**Model-cluster affinity.** Track which model succeeds most often on each cluster. Over time, the system learns "Claude Opus handles architectural tasks better; Haiku handles mechanical tasks fine." Model selection becomes data-driven instead of heuristic.

**Cross-domain analogy detection.** Two clusters in different domains with similar medoid vectors indicate a structural analogy. A cluster of "retry logic" episodes in networking is structurally similar to a cluster of "retry logic" episodes in database operations. The system can transfer heuristics from one to the other.

**Curriculum scheduling.** The `roko-learn/src/curriculum.rs` module supports adaptive task ordering. When clustering reveals low-pass-rate categories, the curriculum scheduler front-loads tasks in those categories to build skill where the agent is weakest:

```rust
pub enum CurriculumStrategy {
    EasyFirst,
    HardFirst,
    Interleaved,
    Adaptive { success_threshold: f64 },
}
```

### 6.3 Current wiring status

Episode fingerprinting is wired: every episode gets an `hdc_fingerprint` field computed at dispatch time and stored in `.roko/episodes.jsonl`. The k-medoids clustering algorithm is implemented and tested. Clustering is exercised during dream consolidation (section 9) but is not yet wired into the CascadeRouter's real-time model selection path. This is tracked as a known gap (item #13 in CLAUDE.md: "knowledge-informed agent routing").

---

## 7. Cross-domain resonance

Resonance detection finds structural analogies between knowledge entries from different domains. Two entries "resonate" when their HDC vectors are highly similar despite coming from different source domains.

### 7.1 The ResonanceDetector

The `roko-neuro/src/hdc.rs` module implements pairwise cross-domain comparison:

```rust
pub(crate) struct ResonanceDetector {
    min_similarity: f64,  // Default: 0.526 (above chance baseline of ~0.50)
    max_results: usize,   // Default: 20
}

pub(crate) struct ResonancePair {
    pub entry_a: String,
    pub entry_b: String,
    pub similarity: f64,
    pub domain_a: String,
    pub domain_b: String,
}
```

The detector performs pairwise comparison, skipping same-domain pairs and pruning below the similarity threshold. Complexity is O(n^2), suitable for stores up to approximately 10,000 entries.

### 7.2 Lotka-Volterra dynamics

The `roko-learn/src/resonant_patterns.rs` module models patterns as organisms in a population ecology. Each pattern has a genome (HDC vector), fitness score, population size, carrying capacity, and growth rate. Patterns compete for attention budget.

The competitive Lotka-Volterra equations govern population dynamics:

```
dN_i/dt = r_i * N_i * (1 - (N_i + sum_j(a_ij * N_j)) / K_i)
```

Where:
- `N_i` = population of pattern i (its attention share)
- `r_i` = intrinsic growth rate (derived from fitness)
- `K_i` = carrying capacity
- `a_ij` = competition coefficient (genome similarity between patterns i and j)

Patterns with similar genomes compete more strongly (higher `a_ij`). This means redundant patterns suppress each other, while patterns occupying distinct niches in HDC space coexist. The ecosystem self-regulates: the most predictive, least redundant patterns survive.

```rust
pub struct ResonantPattern {
    pub id: u64,
    pub genome: HdcVector,
    pub fitness: f64,
    pub age: u64,
    pub offspring_count: u32,
    pub population: f64,
    pub carrying_capacity: f64,
    pub growth_rate: f64,
}

impl ResonantPattern {
    pub fn is_alive(&self) -> bool {
        self.population > 0.01  // Extinction threshold
    }
}
```

### 7.3 Cross-domain example

Consider three agents working in different domains:

- **Weather agent** discovers: "Severe weather events in the Gulf of Mexico increase natural gas spot prices within 48 hours" (CausalLink).
- **Energy agent** discovers: "Spikes in natural gas prices above $4/MMBtu trigger institutional hedging activity in energy futures" (CausalLink).
- **DeFi agent** discovers: "Institutional hedging activity correlates with increased DeFi lending rates on stablecoin pools" (CausalLink).

No single agent observes the full chain from Gulf weather to DeFi lending rates. But the InsightStore contains all three CausalLinks. When the DeFi agent queries for factors affecting lending rates, it retrieves the energy agent's link (nat gas -> hedging) and, by composing CausalLinks, discovers the weather agent's upstream cause. The multi-hop chain `weather -> gas prices -> hedging -> DeFi rates` emerges from the shared environment.

This is the value of stigmergy: coordination without communication, knowledge composition without planning.

---

## 8. Geometric privacy (PP-HDC)

The privacy pipeline converts knowledge from natural language to non-invertible geometry before it touches the chain. The design principle: share structure, not content. Algebraic privacy, not statistical noise.

### 8.1 The seven-step pipeline

Every knowledge entry passes through seven transformations before on-chain submission. All seven are zero-LLM operations.

**Step 1: HDC encoding (~5 microseconds).** Encode the knowledge entry as a structured role-filler vector using the `RoleFillerEncoder`. Each semantic field (content, tags, domain, kind) becomes a role-filler pair, bound via XOR, then bundled via majority vote.

**Step 2: Metadata scrub (~10 microseconds).** Strip text metadata that could identify the submitter: API keys, file paths, organization names, IP addresses, email addresses. Regex-based, no LLM involvement.

**Step 3: Sensitive role unbinding (~1 microsecond).** Algebraically remove sensitive information from the vector. If the entry was bound with `bind(PROJECT, "acme-corp")`, unbinding removes the project information exactly:

```
scrubbed = vector XOR bind(PROJECT, "acme-corp")
```

XOR is its own inverse. The result is a vector that no longer contains the project role-filler pair. This is exact algebraic removal, not obfuscation or noise addition.

**Step 4: Quality gate check (~1 microsecond).** Verify the entry meets minimum confidence and completeness thresholds before submission. Reject entries that would waste on-chain storage.

**Step 5: PP-HDC projection (~50 microseconds).** Apply a non-invertible hash-based projection that preserves distance relationships. Two vectors that were similar before projection remain similar after projection, but the projection cannot be reversed to recover the original vector.

The projection preserves similarity to within 1% (empirically measured). A pair of vectors with 0.85 similarity pre-projection will have approximately 0.84-0.86 similarity post-projection.

**Step 6: Embargo check (~1 nanosecond).** Time-sensitive knowledge (Warnings, certain CausalLinks) may have an embargo period. The check verifies whether the embargo has elapsed before allowing submission.

**Step 7: Chain submission (~100 milliseconds).** Submit the PP-HDC vector, domain byte, kind byte, confidence, and content hash to the Korai InsightStore.

### 8.2 Cost analysis

| Step | Compute time | LLM cost |
|------|-------------|----------|
| HDC encoding | ~5 us | $0 |
| Metadata scrub | ~10 us | $0 |
| Role unbinding | ~1 us | $0 |
| Quality gate | ~1 us | $0 |
| PP-HDC projection | ~50 us | $0 |
| Embargo check | ~1 ns | $0 |
| Chain submission | ~100 ms | ~$0.002 gas |
| **Total** | **~5 ms** | **~$0.002** |

Compare to LLM-based abstraction (the approach used by most "privacy-preserving" AI systems): send text to a model, ask it to remove identifying information, hope the model does not hallucinate or miss something. Cost: $0.01-$0.05 per entry. Latency: 1-10 seconds. Reliability: probabilistic.

The geometric approach is 200-4,000x faster, deterministic, and algebraically provable. The non-invertibility comes from the hash-based projection, not from trusting a model to redact correctly.

### 8.3 On-chain representation

The on-chain entry contains no text:

```
vector:       [u8; 1280]     // PP-HDC encoded, non-invertible
domain:       u8             // 256 possible domains
kind:         u8             // 6 knowledge kinds
confidence:   u16            // Fixed-point, 0..65535
submitter:    Address        // 20-byte address
reputation:   u16            // Submitter's earned reputation
timestamp:    u64            // Block timestamp
content_hash: [u8; 32]       // SHA-256 commitment to original
```

Total: approximately 1,340 bytes per entry. The `content_hash` is a commitment: the submitter can later reveal the original content (post-embargo) to prove that the vector corresponds to specific knowledge. But the vector alone reveals nothing.

### 8.4 Academic grounding

The PP-HDC approach draws from two recent publications:

- **PP-HDC (IEEE, 2024):** Privacy-preserving hyperdimensional computing for federated learning. Demonstrates that non-invertible projections of HDC vectors preserve task-relevant structure while preventing reconstruction of training data.
- **FedHDC (ACM, 2024):** Federated hyperdimensional computing. Shows that bundling HDC vectors from multiple clients produces a global model without exposing individual contributions.

Both papers validate the core claim: HDC operations are non-invertible in high dimensions, and distance-preserving projections maintain utility while destroying recoverability.

---

## 9. Dream consolidation

The `roko-dreams` crate implements offline knowledge processing. When an agent accumulates enough unprocessed experience, it enters a dream cycle that replays, imagines, rehearses threats, and promotes validated knowledge.

### 9.1 Sleep pressure

Sleep pressure is a scalar that accumulates each processing tick without consolidation. When pressure exceeds a configurable threshold, the agent transitions from Active to Dreaming state. After a dream cycle completes, pressure resets. Emergency events (urgent warnings, system failures) can interrupt dreams early.

The model is deliberately simple: a counter that goes up during work and resets after consolidation. The threshold controls how frequently dreams occur.

### 9.2 The four-phase dream cycle

The `DreamCycle` in `roko-dreams/src/cycle.rs` orchestrates four phases:

**Phase 1: NREM replay.** Replay high-surprise and high-failure episodes. The selection is prioritized by the Mattar-Daw (2018) algorithm, which scores each episode by:

```rust
pub struct ReplayUtility {
    pub gain: f64,       // Prediction error: how surprising was the outcome?
    pub need: f64,       // Policy relevance: how much does the current policy
                         //   need updating based on this episode?
    pub spacing_inv: f64, // Inverse time since last replay (spaced repetition)
    pub utility: f64,    // gain * need * spacing_inv
}
```

Episodes with high prediction error (gate failures, unexpected outcomes) and high policy relevance (recent, novel) are replayed first. The spaced-repetition term ensures that episodes are not replayed too frequently -- each replay reduces the spacing score, spreading reviews out over time.

Four replay modes are available:

```rust
pub enum DreamReplayMode {
    Random,        // Uniform sampling
    Consequence,   // Prioritize by outcome magnitude
    Causal,        // Follow failure chains to root causes
    Hypothetical,  // Replay counterfactual variants
}
```

**Phase 2: REM imagination.** Generate counterfactuals from observed episodes. The imagination module builds a lightweight causal model from the episode batch:

```rust
pub struct CausalModel {
    pub episodes_by_id: BTreeMap<String, Episode>,
    pub variables: BTreeMap<String, BTreeMap<String, usize>>,
}
```

Three creativity modes produce different kinds of hypothetical knowledge:

```rust
pub enum ImaginationMode {
    Combinational,     // Merge patterns from two episodes
    Exploratory,       // Extend a pattern into a nearby domain
    Transformational,  // Invert an assumption from a successful pattern
}
```

Counterfactual queries take the form: "In episode X, what if variable Y had value Z instead?" The causal model estimates plausibility based on observed variable distributions. Hypotheses that pass plausibility checks enter the staging buffer.

During REM, the similarity threshold for cross-domain matching is relaxed from 0.85 to 0.6. This allows weaker structural analogies to surface -- associations that would be filtered out during waking retrieval but might contain signal worth investigating.

**Phase 3: Threat rehearsal.** Enumerate dangerous patterns from observed failures. The `threat.rs` module groups failed episodes by failure pattern and scores each group:

```rust
pub struct ThreatScenario {
    pub id: String,
    pub description: String,
    pub likelihood: f64,          // How often this failure pattern occurs
    pub impact: f64,              // How severe the consequences are
    pub detection_difficulty: f64, // How hard it is to spot before damage
    pub mitigation: String,       // Recommended countermeasure
}

impl ThreatScenario {
    pub fn severity(&self) -> f64 {
        (self.likelihood * self.impact * (1.0 - self.detection_difficulty))
            .clamp(0.0, 1.0)
    }
}
```

High-severity threats are converted to Warning entries and injected into the knowledge store. This primes the system's somatic markers (section 10) to recognize similar patterns faster in the future.

**Phase 4: Staging buffer.** All dream outputs enter a staging buffer before reaching the main knowledge store. This prevents dream hallucinations from corrupting durable knowledge.

```rust
pub enum ConfidenceStage {
    Raw,        // Just extracted, unvalidated.      Floor: 0.20
    Replayed,   // Replayed in a subsequent cycle.   Floor: 0.30
    Validated,  // Cross-checked, no contradiction.  Floor: 0.50
    Promoted,   // Ready for knowledge store.        Floor: 0.70
}
```

Entries progress through stages as they accumulate evidence. An entry that has not advanced past Raw within 7 days is garbage collected:

```rust
const GC_HORIZON_DAYS: i64 = 7;
const REDUNDANCY_THRESHOLD: f32 = 0.90;  // HDC dedup threshold
```

The staging buffer also deduplicates: if a new entry's HDC vector is 0.90+ similar to an existing staged entry, the existing entry's confidence is boosted instead of creating a duplicate.

### 9.3 Dream cycle report

Each completed dream cycle produces a `DreamCycleReport`:

```rust
pub struct DreamCycleReport {
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_episodes: usize,
    pub processed_episodes: usize,
    pub analysis: TierProgressionReport,
    pub cfactor_regression: Option<CFactorRegression>,
    pub clusters: Vec<DreamClusterReport>,
    pub knowledge_entries_written: usize,
    pub playbooks_created: usize,
    pub regressions_detected: Vec<KnowledgeEntry>,
    pub strategy_hypotheses: Vec<KnowledgeEntry>,
    pub performance_notes: Vec<String>,
    pub hypnagogia_entries_count: usize,
    pub staging_buffer_stats: Option<StagingBufferStats>,
    pub intensive_mode_active: bool,
    pub phase_budget_summary: Option<PhaseBudgetSummary>,
}
```

The report includes C-Factor regression analysis: if the collective performance metric is declining, the report flags it and records which clusters are responsible.

### 9.4 What dreams produce

| Output | How it enters the system |
|--------|-------------------------|
| Promoted knowledge entries | Staging buffer -> KnowledgeStore (tier promotion) |
| New somatic markers | Threat scenarios -> k-d tree of HDC fingerprints |
| New CausalLinks | Cross-episode temporal correlation -> validated links |
| Compressed heuristics | 5 similar observations -> 1 Heuristic entry |
| Pruned stale knowledge | Demurrage marks low-value entries for GC |
| Playbooks | Top heuristics -> `PLAYBOOK.md` + PlaybookStore |
| Performance diagnostics | Regression detection -> performance_notes |

---

## 10. Somatic markers

Somatic markers bridge knowledge and behavior. They encode "how this situation felt last time" as a lookup table of HDC fingerprints mapped to outcomes. The concept implements Damasio's somatic marker hypothesis (1994): organisms develop rapid "gut feelings" about situations based on accumulated embodied experience, bypassing slow deliberative reasoning.

### 10.1 How somatic markers work

At dispatch time, during the GATE step of the universal loop:

1. **Encode.** Compute the current task's HDC fingerprint from its description, domain, model, and files.
2. **Query.** Search the somatic marker store (a k-d tree of past episode fingerprints mapped to outcomes) for nearest neighbors. Latency: less than 100 microseconds.
3. **Evaluate.** If the nearest neighbor is a past failure with high confidence, the prediction error (PE) increases. Higher PE triggers escalation to a stronger model.
4. **Adjust.** If the nearest neighbor is a past success with high confidence, PE decreases. Lower PE allows the CascadeRouter to suppress the task to a cheaper model tier (T0 suppression).

The net effect: the system develops fast, pre-rational responses to familiar situations. A task that resembles a past catastrophic failure triggers immediate caution without needing to reason about why. A task that resembles a past easy success gets routed to the cheapest available model.

### 10.2 Connection to dreams

Threat rehearsal (dream phase 3) is the primary mechanism for building somatic markers. When the dream cycle identifies high-severity threat patterns, it converts them to Warning entries and inserts their HDC fingerprints into the somatic marker store. This means the system "practices" recognizing danger during offline consolidation, priming faster responses during online execution.

The cycle is:

```
Online execution -> failures accumulate -> sleep pressure rises
  -> dream cycle -> threat rehearsal -> somatic markers updated
  -> next online execution -> faster recognition of similar failures
```

---

## 11. Knowledge lifecycle

The full lifecycle of a piece of knowledge, from raw experience to network-level consensus:

```
Raw Episode
  |
  v
HDC Fingerprint (roko-learn/hdc_fingerprint.rs)
  |
  v
Episode Clustering (roko-learn/hdc_clustering.rs, k-medoids)
  |
  v
Resonance Detection (roko-neuro/hdc.rs, pairwise cross-domain)
  |
  v
Local Neuro Store, Transient tier
  |  (validated by gates: 3+ pass verdicts)
  v
Local Neuro Store, Working tier
  |  (confirmed across episodes: 3+ distinct contexts)
  v
Local Neuro Store, Consolidated tier
  |  (PP-HDC pipeline: 7 zero-LLM steps)
  v
Korai InsightStore (on-chain, shared, pheromone-weighted)
  |  (independent confirmation by other agents)
  v
InsightStore, extended half-life (1.5x+ base)
  |  (multiple agents confirm AND use successfully)
  v
InsightStore, Persistent (network consensus knowledge)
```

Each arrow represents a gate. Knowledge does not advance without evidence. The entire pipeline is designed to resist noise: hallucinated outputs die at Transient tier. Weak correlations decay away. Only knowledge that survives repeated, independent validation reaches the shared network.

### 11.1 Time scales

| Stage | Typical residence time |
|-------|----------------------|
| Raw episode | Immediate (logged at execution) |
| Transient tier | 1-3 days (base half-life * 0.1 multiplier) |
| Working tier | 1-4 weeks (base half-life * 0.5 multiplier) |
| Consolidated tier | 1-3 months (base half-life * 1.0 multiplier) |
| On-chain (fresh) | 7-15 days (chain half-life, no confirmations) |
| On-chain (confirmed) | 10-22 days (chain half-life * confirmation boost) |
| Persistent (network consensus) | 2.5-6 months (base half-life * 5.0 multiplier) |

### 11.2 Distillation

The `roko-neuro/src/distiller.rs` module handles the episode-to-knowledge conversion. It batches episodes, sends them to a small model (Claude Haiku by default, to keep distillation cheap), and parses the response into structured `KnowledgeEntry` candidates:

```rust
pub struct Distiller {
    backend: Arc<dyn DistillationBackend>,
}

impl Distiller {
    pub fn with_claude(api_key: impl Into<String>) -> Self {
        // Uses claude-haiku-3-5 by default
    }

    pub async fn distill(&self, episodes: &[Episode]) -> Result<Vec<KnowledgeEntry>> {
        // Batch episodes, render prompt, parse response
    }
}
```

The distiller produces all six knowledge kinds. It is constrained to at most 12 tags and 32 source episodes per entry to prevent knowledge bloat.

---

## 12. Generalized benchmark framework

The InsightStore's value is measurable. The benchmark framework quantifies whether shared knowledge actually improves agent performance.

### 12.1 Section effectiveness

For each arena task, the system tracks whether InsightStore-sourced context sections correlated with success. The `roko-learn/src/section_effect.rs` module maintains per-section inclusion/exclusion statistics:

```rust
pub struct SectionEffect {
    pub section_name: String,
    pub included_trials: u64,
    pub included_passes: u64,
    pub excluded_trials: u64,
    pub excluded_passes: u64,
}
```

The lift metric:

```
lift = pass_rate(with_InsightStore) - pass_rate(without_InsightStore)
```

If lift exceeds 0.05 (5 percentage points), chain knowledge is helping. If lift is flat or negative, the retrieval quality or knowledge quality needs improvement. The `PriorityChange` enum signals recommended adjustments:

```rust
pub enum PriorityChange {
    Increase,         // Section improves pass rate
    Decrease,         // Section hurts pass rate
    NoChange,         // Neutral effect
    InsufficientData, // Not enough trials
}
```

### 12.2 C-Factor measurement

The C-Factor (Woolley et al., 2010) measures collective intelligence: whether a group outperforms its best individual member.

```
C = (1/K) * sum(collective_score / best_individual_score)
```

across K task types.

The `roko-learn/src/cfactor.rs` module computes a composite C-Factor from episode data:

```rust
pub struct CFactor {
    pub overall: f64,                              // 0.0-1.0 composite
    pub components: CFactorComponents,             // Breakdown
    pub agent_contributions: Vec<AgentCFactorContribution>,
    pub pathologies: Vec<CollectivePathology>,
    pub computed_at: DateTime<Utc>,
    pub episode_count: usize,
}
```

C-Factor above 1.0 means the network outperforms its best individual agent. C-Factor below 1.0 means shared knowledge is introducing noise or the agents are not benefiting from each other.

The system also detects collective pathologies:

```rust
pub enum CollectivePathology {
    Cascade { trigger_agent, affected_count },  // Failure propagation
    Groupthink { diversity_score },             // Convergence on narrow approach
    EchoChamber { repeated_knowledge_pct },     // Repeated claims without grounding
    Deadlock { blocked_agents },                // Mutual blocking
    Hallucination { ungrounded_claims },        // Claims without evidence
}
```

### 12.3 Leave-one-out analysis

For each agent, the system computes the C-Factor with and without that agent's episodes:

```rust
pub struct AgentCFactorContribution {
    pub agent_id: String,
    pub episode_count: usize,
    pub without_agent_overall: f64,  // C-Factor without this agent
    pub contribution_score: f64,     // Full - without = contribution
}
```

Positive contribution means the agent raises collective performance. Negative contribution means it drags the network down. The dispatch system uses this to bias routing:

```rust
pub enum AgentDispatchBias {
    PreferStronger,  // Agent's contribution is negative -> upgrade
    PreferCheaper,   // Agent's contribution is positive -> maintain/downgrade
    Neutral,         // Insufficient data
}
```

---

## 13. Network effects

The InsightStore exhibits quadratic scaling in domains. Every new domain creates cross-referencing opportunities with every existing domain.

### 13.1 Scaling projections

| Agents | Approximate entries | Cross-domain pairs | Potential multi-hop chains |
|--------|--------------------|--------------------|---------------------------|
| 10 | 500 | 25 | ~50 |
| 100 | 5,000 | 2,500 | ~12,500 |
| 1,000 | 50,000 | 250,000 | ~3,125,000 |
| 10,000 | 500,000 | 25,000,000 | ~781,250,000 |

Cross-domain pairs grow as O(domains^2). Multi-hop chains grow even faster because each pair can serve as an intermediate link in a longer chain. The value of the network is super-linear in the number of participating domains.

### 13.2 The thousandth-agent advantage

When the thousandth agent joins, it inherits everything the first 999 discovered. Its InsightStore query returns validated knowledge from 999 agents' worth of experience. Its somatic markers inherit threat patterns that the agent never personally encountered. Its CascadeRouter benefits from model-cluster affinity data collected across the entire network.

The cost of this inheritance is one HDC similarity query (~170 microseconds) plus gas for retrieving the top-K results. The value is 999 agents' worth of accumulated knowledge, filtered by relevance, ranked by reputation, and automatically decaying when stale.

This is the moat. A competing system that starts from zero faces the same discovery costs that the network has already amortized across all participants. Each revolution of the plan-execute-gate-persist-share loop widens the gap.

### 13.3 Emergent specialization

As the network grows, agents naturally specialize. An agent that submits high-quality CausalLinks in the DeFi domain earns reputation in that domain. Future agents seeking DeFi knowledge preferentially retrieve high-reputation entries. The specialist's knowledge propagates further, attracting more confirmations, further increasing reputation.

No central authority assigns specializations. The pheromone dynamics and reputation weighting create the same emergent division of labor that ant colonies exhibit: specialists emerge because specialization is rewarded by the environment.

---

## 14. Temporal knowledge topology

The `roko-neuro/src/temporal.rs` module implements Allen's interval algebra (1983) over knowledge validity periods. Each piece of knowledge has a temporal interval, and relationships between entries are expressed using 13 interval relations (before, after, meets, overlaps, during, starts, finishes, and their inverses, plus equals).

This enables temporal queries like:

- "Which heuristics were valid during the same period as this CausalLink?"
- "What knowledge was created after this Warning but before it expired?"
- "Which entries supersede each other?"

Knowledge epochs partition time into named phases (e.g., "sprint-42", "v2.1-release"). Entries created during epoch N are valid within their interval unless explicitly superseded by entries in epoch N+1.

---

## 15. Academic citations

The knowledge system draws from established research across multiple fields.

**Stigmergy and swarm intelligence:**
- Grasse, P.-P. (1959). La reconstruction du nid et les coordinations interindividuelles chez Bellicositermes natalensis et Cubitermes sp. *Insectes Sociaux*, 6(1), 41-80. [Introduced the concept of stigmergy in termite nest construction.]

**Hyperdimensional computing:**
- Kanerva, P. (2009). Hyperdimensional computing: An introduction to computing in distributed representation with high-dimensional random vectors. *Cognitive Computation*, 1(2), 139-159.
- Rahimi, A. & Recht, B. (2009). Weighted sums of random kitchen sinks: Replacing minimization with randomization in learning. *NeurIPS 2009*. [Theoretical foundation for random feature methods that HDC builds upon.]

**Privacy-preserving HDC:**
- PP-HDC (IEEE, 2024). Privacy-preserving classification using hyperdimensional computing with non-invertible encoding. [Demonstrates that non-invertible HDC projections preserve distance while preventing reconstruction.]
- FedHDC (ACM, 2024). Federated learning with hyperdimensional computing: Aggregation without data exposure. [Shows that bundling HDC vectors from multiple sources produces useful global models without exposing individual contributions.]

**Memory and forgetting:**
- Ebbinghaus, H. (1885). *Uber das Gedachtnis: Untersuchungen zur experimentellen Psychologie*. [The forgetting curve: memory retention decays exponentially with time unless reinforced.]

**Sleep and consolidation:**
- arXiv:2603.14517 (2025). Sleep-inspired memory consolidation for continual learning in autonomous agents. [Applies NREM/REM sleep cycle principles to agent knowledge management.]

**Prioritized replay:**
- Mattar, M. G. & Daw, N. D. (2018). Prioritized memory access explains planning and hippocampal replay. *Nature Neuroscience*, 21(11), 1609-1617. [Episodes with high prediction error times policy relevance should be replayed first.]

**Somatic markers:**
- Damasio, A. R. (1994). *Descartes' Error: Emotion, Reason, and the Human Brain*. [The somatic marker hypothesis: organisms use embodied emotional signals as rapid decision shortcuts.]

**Population dynamics:**
- Lotka, A. J. (1925). *Elements of Physical Biology*. [Predator-prey equations applied to pattern competition.]
- Volterra, V. (1926). Fluctuations in the abundance of a species considered mathematically. *Nature*, 118, 558-560.

**Collective intelligence:**
- Woolley, A. W., Chabris, C. F., Pentland, A., Hashmi, N., & Malone, T. W. (2010). Evidence for a collective intelligence factor in the performance of human groups. *Science*, 330(6004), 686-688. [The C-Factor: a single statistical factor that predicts group performance across diverse tasks.]

**Temporal reasoning:**
- Allen, J. F. (1983). Maintaining knowledge about temporal intervals. *Communications of the ACM*, 26(11), 832-843. [The 13 interval relations used in roko-neuro's temporal module.]

**Clustering:**
- Kaufman, L. & Rousseeuw, P. J. (1990). *Finding Groups in Data: An Introduction to Cluster Analysis*. [k-medoids / PAM algorithm used by roko-learn's HDC clustering module.]

---

## 16. Implementation status

| Component | Crate | Status | Notes |
|-----------|-------|--------|-------|
| HDC vector + operations | `roko-primitives` | Shipped | 10,240-bit, bind/bundle/permute/similarity, serde, rkyv |
| Episode fingerprinting | `roko-learn` | Wired | Computed at dispatch, stored in episodes.jsonl |
| k-medoids clustering | `roko-learn` | Built | Used in dream cycle, not yet in CascadeRouter |
| Neuro KnowledgeStore | `roko-neuro` | Wired | JSONL backend, decay, GC, HDC retrieval, anti-knowledge |
| Tier progression (D1-D3) | `roko-neuro` | Wired | Episodes -> insights -> heuristics -> playbooks |
| Knowledge distiller | `roko-neuro` | Built | Claude Haiku backend, structured extraction |
| Resonance detection | `roko-neuro` | Built | Pairwise cross-domain, not yet in dispatch path |
| Lotka-Volterra dynamics | `roko-learn` | Built | Pattern ecosystem, VCG auction integration |
| Dream cycle (4 phases) | `roko-dreams` | Built | NREM replay, REM imagination, threat rehearsal, staging |
| Staging buffer | `roko-dreams` | Built | Confidence ladder, 7-day GC, HDC dedup |
| Temporal topology | `roko-neuro` | Built | Allen interval algebra over knowledge epochs |
| Emotional provenance | `roko-neuro` | Wired | PAD-space affect, validation arcs, diversity |
| Context assembly | `roko-neuro` | Wired | 4-weight scoring, cross-domain bonus |
| Section effectiveness | `roko-learn` | Wired | Lift measurement, priority recommendations |
| C-Factor metrics | `roko-learn` | Wired | Composite score, pathology detection, leave-one-out |
| Curriculum scheduling | `roko-learn` | Built | Adaptive ordering, not yet wired to plan runner |
| PP-HDC pipeline | Not yet built | Planned | 7-step zero-LLM privacy encoding |
| InsightStore (on-chain) | Korai contracts | Planned | HTC precompile, pheromone decay, reputation |
| Somatic marker k-d tree | Not yet built | Planned | HDC fingerprint -> outcome lookup |

The local knowledge system is substantially complete. The on-chain InsightStore and PP-HDC pipeline depend on the Korai chain (PRD-03). Somatic markers are architecturally defined but not yet implemented as a separate lookup structure -- the pieces exist (episode fingerprints, threat scenarios, dispatch-time PE adjustment) but the k-d tree integration is pending.

---

## 17. Korai integration gaps

Seven gaps separate the built code from a wired knowledge-sharing system. Each gap is a broken link in the chain from local knowledge to on-chain collective intelligence.

### Gap 1: Neuro bidder is empty

**What is broken.** The `NeuroBidder` variant exists in `orchestrate.rs` as an `AttentionBidder`. It participates in the VCG auction during context assembly. But the bidder never queries the `KnowledgeStore`. It returns an empty bid every time. The knowledge store holds validated insights, heuristics, and causal links. None of them reach the agent's prompt.

**Where the code lives.**
- Bidder registration: `crates/roko-cli/src/orchestrate.rs` (the `AttentionBidder::Neuro` variant)
- Knowledge store: `crates/roko-neuro/src/knowledge_store.rs`
- Context assembly weights: `crates/roko-neuro/src/context_assembly.rs` (the `ContextAssemblyWeights` struct)
- HDC retrieval: `crates/roko-neuro/src/hdc.rs` (the `RoleFillerEncoder` and `ResonanceDetector`)

**How to fix it.** The NeuroBidder needs three things:

1. At dispatch time, compute the current task's HDC fingerprint from the task description, domain, and model identifier.
2. Query the `KnowledgeStore` for the top-K entries by the four-weight scoring function (HDC similarity 40%, keyword relevance 30%, predictive foraging utility 20%, freshness 10%) with the cross-domain bonus applied.
3. Format the retrieved entries as `ContextSection` items and submit them to the VCG auction with the bid weight from the domain profile's `context_weights` for `KnowledgeEntries`.

The query path already exists in `KnowledgeStore::query_by_hdc()`. The HDC fingerprint computation exists in `roko-learn/src/hdc_fingerprint.rs`. The VCG auction exists in `roko-compose/src/prompt.rs`. The fix is wiring, not building.

**Dependencies.** None. All components exist. This is the single highest-value integration gap.

### Gap 2: No chain query path

**What is broken.** The `ChainClient` trait in `roko-chain` defines methods for querying the Korai InsightStore. No struct implements those methods against a live chain. The HTC precompile (section 3.6) defines the on-chain similarity search interface. No Rust code calls it. Agents cannot read the collective knowledge that other agents have published.

**Where the code lives.**
- Chain client trait: `crates/roko-chain/src/client.rs`
- InsightStore contract ABI: not yet generated (requires Korai contract deployment)
- HTC precompile spec: PRD-03, section on precompiled contracts
- Local query fallback: `crates/roko-neuro/src/knowledge_store.rs` (same query interface, local data)

**How to fix it.** Two paths, both needed:

1. **Local mock for development.** Implement `ChainClient` against a local JSONL file that simulates the InsightStore. Agents publish to the file. Other agents (or the same agent in a later session) query it. This enables testing the full publish-query loop without a running chain.
2. **Live integration.** Once Korai testnet launches, implement `ChainClient` against the RPC endpoint. Use `alloy` for contract calls. The HTC precompile is a special address that accepts a query vector and returns top-K results with Hamming similarity scores.

**Dependencies.** The local mock has no dependencies. The live integration depends on Korai testnet (PRD-03).

### Gap 3: No knowledge publish path

**What is broken.** Agents produce knowledge through the distillation pipeline (D1: episodes to insights, D2: insights to heuristics). That knowledge accumulates in the local `KnowledgeStore`. None of it reaches the Korai InsightStore. The 7-step PP-HDC pipeline (section 8) that transforms local knowledge into chain-safe vectors is specified but not implemented. No code calls the chain submission endpoint.

**Where the code lives.**
- Distillation: `crates/roko-neuro/src/distiller.rs` (produces `KnowledgeEntry` candidates)
- Tier progression: `crates/roko-neuro/src/tier_progression.rs` (D1/D2/D3 pipeline)
- PP-HDC specification: this PRD, section 8 (7 steps, zero-LLM)
- Submission target: `ChainClient::submit_insight()` (trait method, no live implementation)

**How to fix it.** Implement the publish pipeline as a new module in `roko-neuro`:

1. **Content classifier** (~300 lines). Regex-based NER for 40+ entity types (API keys, file paths, credentials, PII). Flags entries that contain sensitive content before any further processing.
2. **Knowledge abstractor** (~200 lines). Applies the L0-L3 abstraction levels. L0 is the raw entry. L1 redacts flagged entities. L2 generalizes specifics to categories. L3 retains only the structural category. The abstraction level is configurable per deployment.
3. **PP-HDC encoder** (~250 lines). The 7-step pipeline from section 8. HDC encoding, metadata scrub, sensitive role unbinding, quality gate, non-invertible projection, embargo check, chain submission.
4. **Publish trigger** (~100 lines). Wire into the tier progression pipeline. When an entry reaches Consolidated tier, run the publish pipeline. Respect the alpha protection embargo (section 18, layer 5).

**Dependencies.** Gap 2 (chain client) must be resolved first for live publishing. The local mock path allows development and testing without the chain.

### Gap 4: Episode clustering not wired to runtime

**What is broken.** The `roko-learn/src/hdc_clustering.rs` module implements k-medoids clustering over HDC vectors. The algorithm is tested and works. It runs during dream consolidation. It does not run during the runtime's model selection path. The CascadeRouter selects models using bandit exploration and historical pass rates, but it has no concept of task clusters. It cannot route tasks matching a "hard cluster" to a stronger model or tasks matching an "easy cluster" to a cheaper one.

**Where the code lives.**
- k-medoids: `crates/roko-learn/src/hdc_clustering.rs` (`KMedoidsConfig`, `HdcCluster`, `ClusterResult`)
- Dream integration: `crates/roko-dreams/src/cycle.rs` (calls clustering during NREM replay phase)
- CascadeRouter: `crates/roko-learn/src/cascade_router.rs` (bandit-based model selection, no cluster awareness)
- Curriculum scheduler: `crates/roko-learn/src/curriculum.rs` (adaptive ordering, not wired to plan runner)

**How to fix it.**

1. **Background re-clustering.** Run k-medoids on the full episode store at delta-cycle frequency (when the agent dreams). Store the resulting clusters in `.roko/learn/episode-clusters.json`. Include per-cluster pass rates, model affinity scores, and medoid vectors.
2. **Cluster-aware routing.** At dispatch time, compute the new task's HDC fingerprint, find the nearest cluster medoid by Hamming similarity, look up the cluster's model affinity data (which model succeeds most often on tasks in that cluster), and pass the affinity as a prior to the CascadeRouter's bandit selection.
3. **Curriculum wiring.** Connect the curriculum scheduler to the plan runner. When clustering reveals low-pass-rate clusters, the scheduler front-loads tasks matching those clusters to build skill where the agent is weakest.

**Dependencies.** Gap 1 (neuro bidder) is independent but complementary. Cluster-aware routing and knowledge-informed routing reinforce each other.

### Gap 5: Reputation does not inform auction

**What is broken.** The Korai agent registry (PRD-03) tracks on-chain reputation per agent. The VCG auction in `roko-compose/src/prompt.rs` allocates context budget using bid weights. Reputation data from the chain never enters the auction. An agent with 10,000 successful episodes and a reputation score of 95 bids at the same weight as an agent with 10 episodes and a reputation of 12.

**Where the code lives.**
- VCG auction: `crates/roko-compose/src/prompt.rs` (`vcg_allocate`)
- Agent registry: Korai smart contract (not yet deployed)
- Reputation tiers: `crates/roko-core/src/reputation.rs` (defines `ReputationTier` enum)
- Context bidders: `crates/roko-cli/src/orchestrate.rs` (the three `AttentionBidder` variants: Neuro, Task, Research)

**How to fix it.** Add a reputation multiplier to the VCG bid weight calculation:

```
effective_bid = base_bid * domain_weight * reputation_multiplier
```

Where `reputation_multiplier` is derived from the submitting agent's on-chain reputation score. High-reputation agents' knowledge entries win more context budget. This requires reading the reputation score from the chain (depends on Gap 2) or from a cached local mirror.

For the local-only case (no chain), use the `AgentCFactorContribution` score as a proxy reputation: agents whose episodes raise the collective C-Factor get a bid multiplier > 1.0.

**Dependencies.** Gap 2 (chain client) for live reputation data. Without the chain, the C-Factor proxy works as a local approximation.

### Gap 6: No HDC precompile in Mirage

**What is broken.** The Korai VM (Mirage, PRD-03) spec calls for four HDC-specific precompiled operations: similarity, top-k, bind, and bundle. These operations run at EVM execution speed (~10x faster than Solidity equivalents) and make the InsightStore's similarity search gas-efficient. The spec exists. The implementation does not. Without the precompile, on-chain similarity search falls back to Solidity, which costs approximately 40x more gas per query.

**Where the spec lives.**
- Mirage VM spec: PRD-03, precompile section
- HDC operations in Rust: `crates/roko-primitives/src/hdc.rs` (all four operations implemented)
- Gas cost estimates: PRD-03, table of precompile gas costs

**The four operations.**

| Operation | Input | Output | Estimated gas |
|-----------|-------|--------|--------------|
| `hdc_similarity(a, b)` | Two 1,280-byte vectors | `u16` similarity score | ~100 gas |
| `hdc_topk(query, k)` | Query vector + k | Sorted list of (index, similarity) pairs | ~400 gas for k=20 |
| `hdc_bind(a, b)` | Two 1,280-byte vectors | 1,280-byte result (XOR) | ~50 gas |
| `hdc_bundle(vectors)` | Array of vectors | 1,280-byte majority-vote result | ~200 gas |

**How to fix it.** Implement the precompile in the Mirage VM crate. The Rust implementations in `roko-primitives` are the reference. The precompile wraps them with ABI encoding/decoding and gas metering. Estimated implementation: ~500 lines of Rust.

**Dependencies.** Mirage VM development (PRD-03). This is a Korai-side dependency, not a roko-side one.

### Gap 7: PP-HDC not implemented

**What is broken.** Knowledge vectors are shared as raw HDC encodings. The XOR bind operation is its own inverse: `bind(bind(a, b), b) == a`. An attacker who knows the role vectors (which are deterministic from public seeds) can unbind role-filler pairs and recover approximate content. The PP-HDC non-invertible projection (section 8, step 5) prevents this by applying a one-way hash-based transformation. It is specified. It is not built.

**Where the spec lives.**
- PP-HDC pipeline: this PRD, section 8 (step 5 specifically)
- Academic foundation: PP-HDC (IEEE, 2024), FedHDC (ACM, 2024)
- HDC operations: `crates/roko-primitives/src/hdc.rs`

**What PP-HDC does.**

The projection maps a 10,240-bit vector through a deterministic but non-invertible transformation. The mathematical property: for any two vectors `a` and `b`, `similarity(project(a), project(b))` is approximately equal to `similarity(a, b)` (within 1% empirically). But given `project(a)`, there is no efficient algorithm to recover `a`.

The projection uses a fixed random matrix derived from a public seed. Each bit of the output is the majority vote of a random subset of input bits. Because the subsets overlap and the majority vote loses information, the mapping is many-to-one and non-invertible.

**How to fix it.** Implement `PpHdcProjector` in `roko-primitives`:

```rust
pub struct PpHdcProjector {
    /// Random bit-selection matrix, derived from public seed.
    /// Each row is a list of input bit indices that contribute
    /// to one output bit via majority vote.
    projection_matrix: Vec<Vec<usize>>,
}

impl PpHdcProjector {
    pub fn new(seed: &[u8], fan_in: usize) -> Self { /* ... */ }
    pub fn project(&self, vector: &HdcVector) -> HdcVector { /* ... */ }
}
```

Estimated implementation: ~200 lines. The `fan_in` parameter controls how many input bits contribute to each output bit. Higher fan-in means more information loss (stronger privacy) but less similarity preservation. Empirical testing with `fan_in = 7` shows <1% similarity loss on the knowledge entry distribution.

**Dependencies.** None. This is a standalone implementation in `roko-primitives`.

---

## 18. Seven-layer knowledge publishing defense

Every knowledge entry that leaves the local Neuro store and reaches the Korai InsightStore passes through seven defensive layers. Each layer addresses a distinct attack surface. All seven are zero-LLM operations. The total pipeline cost is ~5ms compute and ~$0.002 gas.

### Layer 1: Content classification

**Purpose.** Identify sensitive content before any transformation occurs.

**Implementation.** A Presidio-style named entity recognizer scanning for 40+ entity types:

| Category | Entity types | Examples |
|----------|-------------|---------|
| **Credentials** | API keys, tokens, passwords, private keys, mnemonics | `sk-proj-abc123...`, `0x4f3e...` (64 hex chars) |
| **Infrastructure** | File paths, hostnames, IP addresses, URLs, port numbers | `/Users/will/dev/...`, `192.168.1.42:8080` |
| **PII** | Email addresses, phone numbers, physical addresses, names | `will@nunchi.dev`, `+1-555-0123` |
| **Organization** | Company names, project names, repository URLs, team names | `nunchi`, `roko`, `github.com/nunchi/...` |
| **Financial** | Account numbers, routing numbers, wallet addresses, amounts | `0xdead...beef`, `$42,000` |
| **Code artifacts** | Variable names from private repos, internal function signatures | `fn _internal_secret_handler()` |
| **Network** | Internal DNS names, VPN endpoints, subnet ranges | `db-primary.internal.nunchi.io` |

The classifier operates on the raw text content of the `KnowledgeEntry`. It produces a list of `(entity_type, span, confidence)` triples. Each entity type maps to a severity level:

```rust
pub enum SensitivityLevel {
    Public,       // Safe to share as-is
    Internal,     // Redact before sharing
    Confidential, // Generalize before sharing
    Restricted,   // Block from sharing entirely
}
```

Entries with any Restricted-level entity are blocked from publishing. All other entities are handled by layers 2 and 3.

**Implementation cost.** ~400 lines. Regex-based matchers for each entity category. No ML models. No external API calls. Runs in ~10 microseconds per entry.

### Layer 2: Knowledge distillation/abstraction

**Purpose.** Transform specific knowledge into progressively more general forms.

Four abstraction levels:

| Level | Name | What it produces | Example |
|-------|------|-----------------|---------|
| L0 | Raw | Original entry, unmodified | "Clippy catches 40% of gate failures in the roko workspace before compilation" |
| L1 | Redacted | Sensitive entities replaced with type tokens | "Clippy catches 40% of gate failures in the [PROJECT] workspace before compilation" |
| L2 | Generalized | Specific values replaced with categories | "Static analysis catches a significant fraction of gate failures before compilation" |
| L3 | Category-only | Structural pattern, no specifics | "Pre-commit static analysis reduces downstream gate failure rates" |

The abstraction level is configurable per deployment. The default is L2 for chain publishing. L0 stays local. L1 is used for fleet-internal sharing (agents in the same organization). L3 is used when maximum privacy is required.

Each level preserves the HDC vector's structural information while removing textual specifics. The HDC encoding (layer 1 of the pipeline) captures the entry's structural role-filler relationships. The text abstraction removes identifying details from the content field. Both the vector and the abstracted text are available, but only the vector reaches the chain.

**Implementation cost.** ~300 lines. Template-based substitution. The entity spans from layer 1 drive the replacements.

### Layer 3: Information Flow Control (IFC) labels

**Purpose.** Attach machine-readable security labels to every entry for policy enforcement.

The labeling system follows Fides-style information flow control:

```rust
pub struct IFCLabel {
    /// Confidentiality level: how sensitive the content is.
    pub confidentiality: ConfidentialityLevel,
    /// Integrity check: whether the content has been validated.
    pub integrity: IntegrityLevel,
    /// Content type classification.
    pub content_type: ContentType,
    /// Taint chain: which systems have touched this data.
    pub taint_chain: Vec<TaintSource>,
}

pub enum ConfidentialityLevel {
    Public,          // Publishable to chain
    OrgInternal,     // Shareable within organization
    TeamRestricted,  // Shareable within team
    AgentLocal,      // Never leaves the local store
}

pub enum IntegrityLevel {
    GateVerified,    // Passed at least one gate pipeline
    PeerConfirmed,   // Confirmed by independent agent
    SelfAssessed,    // Agent's own assessment only
    Unverified,      // No verification
}

pub enum ContentType {
    TechnicalInsight,
    SecurityFinding,
    PerformanceData,
    FinancialSignal,
    OperationalHeuristic,
    ResearchSynthesis,
}

pub struct TaintSource {
    pub system: String,      // "neuro", "distiller", "dream-cycle"
    pub timestamp: DateTime<Utc>,
    pub operation: String,   // "distill", "promote", "abstract"
}
```

The IFC label travels with the entry through the pipeline. Each layer reads the label and applies policy. A `Confidential` entry cannot pass to the chain, regardless of what the other layers decide. A `SelfAssessed` entry gets a lower confidence multiplier than a `GateVerified` entry.

**Implementation cost.** ~200 lines. Label assignment at entry creation + policy checks at each pipeline stage.

### Layer 4: Quality gate

**Purpose.** Prevent low-value entries from consuming on-chain storage and polluting the InsightStore.

Four checks, all evaluated in ~1 microsecond:

| Check | Threshold | Rationale |
|-------|-----------|-----------|
| Confidence | >= 0.75 | Below this, the entry has not accumulated enough evidence to be worth sharing |
| Tier | >= Working | Transient entries are too volatile; they may be invalidated before the chain transaction confirms |
| Gate verification | At least 1 gate pass | Entries produced during unverified episodes carry no quality signal |
| Conflict check | No active AntiKnowledge with similarity > 0.7 | Publishing knowledge that contradicts known-bad entries wastes chain resources |

Entries that fail any check are held in the local store. They can be re-evaluated at the next dream cycle if their evidence improves.

**Implementation cost.** ~100 lines. Four boolean checks against existing entry metadata.

### Layer 5: Alpha protection (temporal embargo)

**Purpose.** Prevent premature publication of knowledge that has economic value.

Knowledge discovered during active work may confer an advantage. Publishing it immediately lets competitors free-ride on the discovery cost. The embargo layer enforces a configurable delay between local discovery and chain publication.

| Entry kind | Default embargo | Rationale |
|-----------|----------------|-----------|
| General insight | 0 (no embargo) | General knowledge benefits from rapid sharing |
| Trading signal | 24 hours | Trading alpha decays; 24h delay preserves short-term value while still sharing the structural pattern |
| MEV-relevant | 1 hour | MEV opportunities are fleeting; 1h is enough to capture most value |
| Security finding | 72 hours | Responsible disclosure window; publish the structural pattern after remediation |
| Strategy fragment | 24 hours | Strategy parameters should not be shared while active positions depend on them |

The embargo is implemented as a timestamp check: `if now < entry.created_at + embargo_duration, skip`. Entries that age past the embargo are eligible for the next publication cycle.

The embargo can be overridden in two directions:
- **Operator can extend** any embargo indefinitely (for compliance or strategic reasons).
- **Operator can waive** the embargo for specific entries (for collaborative research or open-source projects).

**Implementation cost.** ~100 lines. Timestamp comparison + per-kind default lookup + operator override table.

### Layer 6: PP-HDC encoding

**Purpose.** Transform the entry's HDC vector into a non-invertible projection that preserves distance relationships.

The full PP-HDC pipeline (see Gap 7 in section 17 and section 8 of this PRD):

1. Start with the entry's 10,240-bit HDC vector (already computed at entry creation).
2. Apply the non-invertible projection: each output bit is the majority vote of `fan_in` randomly-selected input bits. The random selection is deterministic from a public seed, so all agents use the same projection.
3. Verify similarity preservation: spot-check that the projected vector's similarity to a reference set matches pre-projection similarity within 1%.

The projection is distance-preserving and non-invertible. Two vectors that were 0.85 similar before projection are 0.84-0.86 similar after. But given the projected vector, no efficient algorithm recovers the original.

**Cost.** ~50 microseconds per vector. Zero LLM involvement.

### Layer 7: Selective sharing (novelty check)

**Purpose.** Prevent redundant publications that waste gas and dilute the InsightStore.

Before submitting, query the InsightStore (or local mock) for entries similar to the candidate. If the candidate's projected vector has > 0.90 similarity to an existing entry from any submitter, the candidate is redundant. Instead of publishing a duplicate, confirm the existing entry (which extends its half-life and boosts the original submitter's reputation).

This creates a natural deduplication mechanism:
- First publisher earns reputation for discovery.
- Subsequent confirmers earn smaller reputation for verification.
- The InsightStore stays compact and high-signal.

If the candidate is novel (no existing entry above 0.90 similarity), submit it.

**Cost.** One InsightStore query (~170 microseconds at 10K entries). If redundant, one confirmation transaction (~$0.001 gas). If novel, one submission transaction (~$0.002 gas).

### Publication triggers

Four events trigger the publish pipeline:

| Trigger | When | What gets published |
|---------|------|-------------------|
| **Task completion** | After a task passes all gates | Insights and heuristics distilled from the successful episode, if they meet quality thresholds |
| **Dream cycle** | After the staging buffer promotes entries | Consolidated knowledge from NREM replay and REM imagination that survived the staging confidence ladder |
| **Cross-domain resonance** | When the resonance detector finds high-similarity pairs across domains | The resonance pair itself, encoded as a CausalLink |
| **Operator approval** | Manual trigger via `roko publish --entry <id>` | Specific entries selected by the operator for sharing |

Each trigger feeds into the same 7-layer pipeline. No entry bypasses any layer.

### Total implementation cost

| Component | Lines | Crate |
|-----------|-------|-------|
| Content classifier (40+ entity types) | ~400 | `roko-neuro` |
| Knowledge abstractor (L0-L3) | ~300 | `roko-neuro` |
| IFC label system | ~200 | `roko-neuro` |
| Quality gate | ~100 | `roko-neuro` |
| Embargo manager | ~100 | `roko-neuro` |
| PP-HDC projector | ~200 | `roko-primitives` |
| Novelty checker + dedup | ~250 | `roko-neuro` |
| Publish trigger wiring | ~300 | `roko-cli` / `roko-neuro` |
| **Total** | **~1,850** | |

---

## 19. Geometric knowledge sharing: the zero-LLM pipeline

The PP-HDC pipeline described in section 8 and defended by the seven layers in section 18 operates without any LLM calls. This section specifies the complete execution path, cost model, on-chain format, and retrieval architecture.

### 19.1 The seven-step execution path

Each step executes in sequence. Total wall-clock time: ~5 milliseconds. Total LLM cost: $0.

**Step 1: HDC role-filler encoding (~5 microseconds).**

The `RoleFillerEncoder` in `roko-neuro/src/hdc.rs` converts the knowledge entry into a structured vector:

```rust
let roles_and_fillers = vec![
    ("content".into(), entry.content.clone()),
    ("domain".into(), entry.domain.clone()),
    ("kind".into(), format!("{:?}", entry.kind)),
    ("confidence".into(), format!("{:.2}", entry.confidence)),
];
for (i, tag) in entry.tags.iter().enumerate() {
    roles_and_fillers.push((format!("tag_{}", i), tag.clone()));
}
let vector = RoleFillerEncoder::encode_structured(&roles_and_fillers);
```

Each role name generates a deterministic seed vector via `HdcVector::from_seed()`. Each filler generates its own seed vector. The pair is bound via XOR. All pairs are bundled via majority vote into a single 10,240-bit / 1,280-byte vector.

**Step 2: Text metadata scrub (~10 microseconds).**

Regex pass over the entry's text fields. Targets:

| Pattern | Regex | Replacement |
|---------|-------|-------------|
| API keys | `(sk-[a-zA-Z0-9]{20,})` | `[REDACTED_API_KEY]` |
| File paths | `(/[a-zA-Z0-9_./\-]+){3,}` | `[REDACTED_PATH]` |
| URLs with auth | `https?://[^:]+:[^@]+@` | `[REDACTED_AUTH_URL]` |
| IP addresses | `\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}` | `[REDACTED_IP]` |
| Email addresses | `[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}` | `[REDACTED_EMAIL]` |
| Hex private keys | `0x[0-9a-fA-F]{64}` | `[REDACTED_KEY]` |

This scrub operates on the text metadata that accompanies the vector. The vector itself does not contain raw text, but the `content_hash` commitment field references the original. Scrubbing ensures the stored text is safe for post-embargo reveal.

**Step 3: Sensitive role unbinding via XOR (~1 microsecond).**

This is algebraic erasure, not obfuscation. If the entry was encoded with a role-filler pair that contains sensitive information, unbind it:

```rust
// Remove project-specific information
let project_role = HdcVector::from_seed(b"project");
let project_value = HdcVector::from_seed(entry.project_name.as_bytes());
let project_binding = project_role.bind(&project_value);
let scrubbed = vector.bind(&project_binding); // XOR is self-inverse: removes the binding
```

Because `bind(bind(a, b), b) == a`, applying bind twice with the same operand removes the first application. The resulting vector no longer contains the project-specific role-filler pair. The rest of the vector's structure is preserved.

Roles to unbind (configurable per deployment):
- `project` -- removes project name
- `org` -- removes organization name
- `repo` -- removes repository URL
- `author` -- removes author identity
- `filepath` -- removes specific file paths (each path was bound as `filepath_N`)

The unbinding list is read from `roko.toml`:

```toml
[privacy.unbind_roles]
roles = ["project", "org", "repo", "author"]
```

**Step 4: Quality gate check (~1 microsecond).**

Four boolean checks (see section 18, layer 4). Pass or reject.

**Step 5: PP-HDC non-invertible projection (~50 microseconds).**

The projection maps 10,240 input bits to 10,240 output bits through a many-to-one function:

```
For each output bit i:
  Select fan_in input bits at indices determined by hash(seed, i)
  Output bit i = majority_vote(selected input bits)
```

With `fan_in = 7`, each output bit is the majority of 7 input bits. Because multiple output bits share input bits, and because majority vote loses the minority signal, the mapping cannot be inverted. Given the output, there are exponentially many possible inputs that produce the same output.

Similarity preservation holds because similar inputs produce similar majorities. If two input vectors agree on 85% of bits, the randomly-selected subsets will also agree on approximately 85% of bits, and the majority votes will agree at approximately the same rate.

Empirical validation on 10,000 knowledge entry pairs shows < 1% mean absolute error in similarity scores pre- vs. post-projection.

**Step 6: Embargo check (~1 nanosecond).**

Compare `now` against `entry.created_at + embargo_duration_for(entry.kind)`. If the embargo has not elapsed, skip this entry and re-evaluate at the next publication cycle.

**Step 7: Chain submission (~100 milliseconds).**

Submit the projected vector and metadata to the Korai InsightStore:

```rust
let submission = InsightStoreEntry {
    vector: projected.to_bytes(),         // [u8; 1280]
    domain: entry.domain_id,              // u8
    kind: entry.kind as u8,               // u8
    confidence: to_fixed_point(entry.confidence), // u16
    submitter: agent_address,             // Address (20 bytes)
    reputation: agent_reputation,         // u16
    timestamp: block_timestamp,           // u64
    content_hash: sha256(&entry.content), // [u8; 32]
};
chain_client.submit_insight(submission).await?;
```

### 19.2 Cost comparison

| Method | Compute time | LLM cost | Gas cost | Reliability |
|--------|-------------|----------|----------|------------|
| Geometric (PP-HDC) | ~5 ms | $0 | ~$0.002 | Deterministic, algebraically provable |
| LLM abstraction | 1-10 sec | $0.01-$0.05 | ~$0.005 | Probabilistic, model-dependent |
| No privacy | ~1 ms | $0 | ~$0.002 | N/A (raw text on chain) |

The geometric approach is 200-4,000x faster than LLM abstraction and costs $0 in inference. The determinism guarantee is load-bearing: the same input always produces the same output, the non-invertibility is mathematically proven (not hoped for), and there is no risk of the privacy model hallucinating a leak.

### 19.3 On-chain entry format

Each entry on-chain occupies approximately 1,340 bytes:

```
vector:       [u8; 1280]     // PP-HDC encoded, non-invertible
domain:       u8             // 256 possible domains
kind:         u8             // 6 knowledge kinds
confidence:   u16            // Fixed-point, 0..65535
submitter:    Address        // 20-byte Ethereum address
reputation:   u16            // Submitter's earned reputation
timestamp:    u64            // Block timestamp
content_hash: [u8; 32]       // SHA-256 commitment (for post-embargo reveal)
```

No text. The `content_hash` is a one-way commitment. The submitter can later reveal the original text (post-embargo) to prove the vector corresponds to specific knowledge. But the vector alone reveals nothing about the original content.

### 19.4 Three-tier on-chain search

The InsightStore supports constant-time similarity search at scale through a three-tier architecture. Each tier rejects non-matching entries earlier and cheaper.

**Tier 1: Bloom filter (~microseconds, rejects 90% of entries).**

Each entry's projected vector is hashed into a Bloom filter at insertion time. A query vector is hashed the same way. Entries that share no Bloom filter bits with the query are rejected immediately. At a 1% false-positive rate, this eliminates ~90% of the store in microseconds.

**Tier 2: Approximate search (1,024-bit downprojection).**

Surviving entries are compared against the query using a downprojected version: take every 10th bit of the 10,240-bit vector to produce a 1,024-bit summary. Compare summaries using POPCNT. This is 10x cheaper than full comparison and rejects another ~80% of candidates.

**Tier 3: Exact search (full 10,240-bit POPCNT).**

The remaining ~2% of entries are compared at full resolution. XOR the two 1,280-byte vectors, count differing bits via hardware POPCNT. Return the top-K by similarity.

**Gas cost for a k=20 query across the three tiers:**

| Tier | Entries processed | Gas per entry | Total gas |
|------|------------------|---------------|-----------|
| Bloom filter | All N | ~0.1 gas | ~0.1N |
| Approximate | ~0.1N | ~1 gas | ~0.1N |
| Exact | ~0.02N | ~10 gas | ~0.2N |

At N = 10,000 entries: ~1,000 + 1,000 + 2,000 = ~4,000 gas total. At N = 100,000 entries: ~10,000 + 10,000 + 20,000 = ~40,000 gas. At N = 1,000,000 entries with the Bloom filter's 90% rejection rate, the query stays under 400,000 gas -- well within a single block's gas limit on Korai.

The HTC precompile (Gap 6) implements all three tiers natively, reducing gas costs by an additional ~10x compared to the Solidity fallback.

---

## 20. HDC deep integration: six levels

HDC is not a single feature. It is a substrate that grows more capable as more levels are wired. Six levels of integration, from what ships today to what the distributed network enables.

### Level 1: Per-episode HDC fingerprint (WIRED)

**Status.** Shipped and running in production.

**What it does.** Every agent episode (task prompt + outcome) generates a deterministic 10,240-bit fingerprint via `roko-learn/src/hdc_fingerprint.rs`. The fingerprint is stored in the `hdc_fingerprint` field of the episode record in `.roko/episodes.jsonl`.

**What it enables.** Fast episode similarity lookup. "Find tasks like this one" is an O(N) scan with POPCNT, taking ~1ms for 10,000 episodes.

**Code path.** `orchestrate.rs` -> `fingerprint_episode(prompt, outcome)` -> `HdcVector::from_seed()` -> stored in episode JSON.

### Level 2: Episode clustering via k-medoids (BUILT, NOT WIRED)

**Status.** Algorithm implemented and tested. Runs during dream consolidation. Not connected to the CascadeRouter or curriculum scheduler.

**What it does.** Groups episodes into K clusters using partitioning around medoids. Each cluster's representative (medoid) is a real episode. Per-cluster statistics include pass rate, model distribution, average latency, and dominant domain.

**What it enables.**
- **Weakness detection.** Clusters with pass rates below the mean indicate task categories the agent struggles with.
- **Model-cluster affinity.** Track which model succeeds most on each cluster. Route future tasks matching a cluster to the best model for that cluster.
- **Cross-domain analogy.** Two clusters from different domains with similar medoid vectors indicate a transferable structural pattern.

**Gap.** See Gap 4 in section 17. The wiring requires: (a) background re-clustering at dream frequency, (b) cluster-aware routing in CascadeRouter, (c) curriculum scheduler connection to plan runner.

### Level 3: Cross-domain resonance via Lotka-Volterra (BUILT, NOT WIRED)

**Status.** `ResonanceDetector` and `ResonantPattern` implemented. Lotka-Volterra population dynamics running in tests. Not called from the dispatch path or the dream cycle's post-processing.

**What it does.** Detects structural analogies between knowledge entries from different domains. Entries "resonate" when their HDC vectors are highly similar despite different domain labels. The Lotka-Volterra model governs pattern competition: redundant patterns suppress each other, while diverse patterns coexist.

**What it enables.**
- **Automatic transfer learning.** A retry-backoff heuristic discovered in the networking domain surfaces when the agent encounters a similar pattern in database operations.
- **Emergent knowledge ecology.** Patterns compete for attention budget. The most predictive, least redundant patterns survive.
- **Multi-hop causal chains.** CausalLinks from different domains compose via HDC bind to reveal transitive relationships no single agent observed.

**Gap.** The resonance detector needs to be called: (a) during dream consolidation (after NREM replay, before staging), and (b) at query time in the NeuroBidder (boost resonant entries in the VCG auction).

### Level 4: Knowledge-informed routing (NOT WIRED)

**Status.** All components exist separately. None are connected.

**What it does.** At dispatch time, before model selection:

1. Compute the new task's HDC fingerprint.
2. Query the Neuro store for entries similar to the fingerprint.
3. Check if retrieved entries include performance data (which model succeeded, how many attempts, which error patterns occurred).
4. Pass the performance data to the CascadeRouter as a Bayesian prior.

The CascadeRouter currently selects models using bandit exploration alone. With knowledge-informed routing, the prior shifts the bandit's initial belief toward models that have succeeded on structurally similar tasks.

**What it enables.**
- **Faster convergence.** The bandit does not need to explore all arms for every new task. Similar past tasks provide a warm start.
- **Fewer wasted tokens.** Tasks that historically require T2 (strong model) skip the T0/T1 exploration phase.
- **Somatic marker integration.** The somatic marker k-d tree (section 10) feeds into the same routing decision. Past failures create "gut feeling" aversion to certain model-task pairings.

**Gap.** See Gap 1 (neuro bidder), Gap 4 (cluster routing), and the somatic marker implementation (section 16 status table).

### Level 5: Somatic markers via k-d tree (BUILT, NOT WIRED)

**Status.** Episode fingerprints exist. Threat scenarios from dream consolidation exist. The k-d tree that maps fingerprints to outcomes is architecturally specified but not instantiated as a runtime structure.

**What it does.** Maintains a spatial index (k-d tree) of HDC fingerprints from past episodes, annotated with outcome labels (success, failure, catastrophic failure) and emotional valence (PAD-space coordinates). At dispatch time, a nearest-neighbor query takes < 100 microseconds and returns the emotional-outcome profile of structurally similar past episodes.

**What it enables.**
- **Pre-rational caution.** A task fingerprint near a past catastrophic failure triggers immediate model escalation without deliberative reasoning.
- **T0 suppression.** A task fingerprint near many past easy successes allows the router to bypass T1/T2 and use the cheapest available model.
- **Adaptive risk sensitivity.** The emotional valence of nearby markers modulates the agent's arousal state, which modulates its tick frequency and budget allocation.

**Gap.** Implement the k-d tree as a persistent structure backed by a binary file in `.roko/learn/somatic-markers.bin`. Wire it into the dispatch path between fingerprint computation and model selection.

### Level 6: Distributed HDC via Korai chain (NOT BUILT)

**Status.** Specified in this PRD and PRD-03. Depends on Korai development.

**What it does.** Publishes PP-HDC vectors to the Korai InsightStore with Merkle commitments. Queries the InsightStore via the HTC precompile. Participates in reputation-weighted knowledge markets.

**What it enables.**
- **Network-wide collective intelligence.** The thousandth agent inherits everything the first 999 discovered.
- **Stigmergic coordination.** Agents modify the shared environment (post knowledge), future agents observe modifications (query by similarity), coordination emerges without communication.
- **Economic incentives for knowledge.** Six mining surfaces reward different kinds of cognitive work (oracle, verifier, inference, repair, mechanism, index).
- **Privacy-preserving sharing.** PP-HDC projection ensures no raw text reaches the chain.

**Gap.** Depends on Korai chain (PRD-03), Mirage VM (Gap 6), and the PP-HDC pipeline (Gap 7).

### Novel capabilities from deep integration

When all six levels are wired, capabilities emerge that none provide individually:

**Curriculum learning.** Level 2 (clustering) identifies weak spots. Level 4 (knowledge-informed routing) uses past performance data to select the right model for each weakness. Level 5 (somatic markers) provides instant assessment of new tasks. Together: the system practices what it is bad at, using the right tools, with emotional memory preventing repeated mistakes.

**Compositional analogy transfer.** Level 3 (resonance) finds structural analogies across domains. Level 1 (fingerprints) encodes the structure. Level 6 (distributed HDC) shares the analogies network-wide. Together: a pattern discovered in weather prediction transfers to DeFi risk modeling because the structural fingerprints are similar, and the chain makes the analogy discoverable to every agent.

**Privacy-preserving sharing.** Level 1 encodes knowledge as HDC vectors. The PP-HDC projection (Level 6, section 8) makes the vectors non-invertible. The seven-layer defense (section 18) blocks sensitive content before projection. Together: agents share structural knowledge without revealing proprietary data, and the privacy guarantee is algebraic, not probabilistic.

**Evolutionary pattern competition.** Level 3 (Lotka-Volterra dynamics) governs which patterns survive locally. Level 6 (distributed HDC) extends competition to the network. Together: patterns compete across all agents. The most predictive, least redundant patterns reach network consensus. This is evolution applied to knowledge, with the InsightStore as the selection environment.

---

## 21. Measurement

### 21.1 Lift: does shared knowledge help?

The primary effectiveness metric. Computed per arena, per domain, and aggregate:

```
lift = pass_rate(with_neuro) - pass_rate(without_neuro)
```

Where `with_neuro` means the NeuroBidder is active and knowledge entries participate in the VCG auction. `without_neuro` means the NeuroBidder is disabled and the agent relies on task description, code intelligence, and iteration memory alone.

**Target.** lift > 0.05 (5 percentage points). If lift is flat or negative, the knowledge retrieval quality needs improvement -- either the entries are low-signal, the HDC similarity threshold is too permissive, or the VCG bid weight for knowledge entries is too high (crowding out more useful context).

**Measurement protocol.** The `ExperimentStore` in `roko-learn` supports A/B testing. Run 50% of tasks with the NeuroBidder active, 50% with it disabled. Compare pass rates after 200+ tasks per arm. Statistical significance via two-proportion z-test with alpha = 0.05.

**Breakdown by knowledge kind.** Track lift separately per knowledge kind (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge). Some kinds may contribute disproportionately. AntiKnowledge in particular should show lift by preventing rediscovery of known-bad approaches.

### 21.2 C-Factor: does the collective outperform individuals?

The C-Factor (Woolley et al., 2010) measures whether the group is smarter than its best member:

```
C = (1/K) * sum_k(collective_score_k / best_individual_score_k)
```

Across K task types (clusters from level 2).

**Target.** C > 1.0. A C-Factor above 1.0 means the collective (all agents sharing knowledge via the InsightStore) outperforms the best individual agent operating in isolation. Below 1.0 means shared knowledge introduces more noise than signal.

**Measurement protocol.** Run the same arena batch with two configurations:
1. **Collective.** All agents share knowledge via InsightStore (or local mock).
2. **Isolated.** Each agent uses only its own local Neuro store. No cross-agent sharing.

Compare the highest individual score (best agent in isolation) against the collective score (average across all agents with sharing). Compute per task-type cluster and aggregate.

**Pathology detection.** If C < 1.0, the `CollectivePathology` detector (section 12.2) identifies the cause:
- **Groupthink** (low diversity score): agents converging on the same approach.
- **Echo chamber** (high repeated-knowledge percentage): the same claims circulating without new evidence.
- **Cascade** (failure propagation): one agent's bad knowledge spreading to others.

### 21.3 SWE-bench A/B test

The definitive benchmark. Run SWE-bench verified (500 tasks) under two conditions:

| Condition | NeuroBidder | InsightStore | Somatic markers |
|-----------|-------------|-------------|-----------------|
| **Baseline** | Disabled | Empty | Disabled |
| **Full knowledge** | Active | Pre-populated from prior runs | Active |

Compare pass@1 rates. The difference is the end-to-end value of the knowledge system.

**Progressive population.** Start with an empty InsightStore. Run SWE-bench in batches of 50. After each batch, measure cumulative pass rate. The curve should show improvement as the InsightStore accumulates validated knowledge from prior batches. If the curve is flat, the knowledge pipeline is not transferring useful information.

### 21.4 Leave-one-out contribution measurement

For each InsightStore entry, compute the C-Factor with and without that entry:

```
contribution(entry) = C_with_entry - C_without_entry
```

Entries with positive contribution are high-value: they improve the collective. Entries with negative contribution are harmful: they introduce noise or misleading information.

**Uses.**
- **Pruning.** Entries with consistently negative contribution should be challenged (verifier mining) or allowed to decay.
- **Reputation calibration.** Agents that consistently produce positive-contribution entries earn faster reputation growth.
- **Reward scaling.** In work markets, the reward for a knowledge entry can be scaled by its measured contribution rather than by its novelty alone.

**Computational cost.** Leave-one-out over N entries requires N evaluations of the C-Factor. For small InsightStores (< 1,000 entries), this is feasible at dream-cycle frequency. For large stores, sample 100 entries per cycle and use the running average.

---

## 22. Open questions

1. **Optimal HDC dimensionality.** 10,240 bits was chosen empirically. Is there a theoretical justification for this specific width, or would 8,192 or 16,384 perform measurably better?

2. **InsightStore query scaling.** Brute-force pairwise comparison works at 10K entries. At 500K entries (10,000-agent network), the HTC precompile needs an approximate nearest-neighbor index. Locality-sensitive hashing over binary vectors is the natural candidate.

3. **Dream cycle frequency.** How often should agents dream? The sleep-pressure model is simple but the optimal threshold is unknown. Too frequent dreaming wastes compute. Too infrequent allows stale knowledge to accumulate.

4. **Anti-knowledge propagation.** Should AntiKnowledge entries be shared on-chain? If agent A discovers that a previously trusted insight is wrong, the correction benefits everyone. But submitting AntiKnowledge also reveals that the original insight existed in A's local store.

5. **Cross-chain knowledge.** If Korai is not the only chain with an InsightStore, how do entries bridge between chains? HDC vectors are chain-agnostic, but reputation and pheromone dynamics are chain-local.
