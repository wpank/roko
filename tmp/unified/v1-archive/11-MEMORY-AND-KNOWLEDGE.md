# 11 — Memory and Knowledge

> Memory = Store-protocol Block with demurrage, tier progression, HDC-based retrieval, heuristic lifecycle, and dream consolidation. Knowledge is a living substrate: signals decay unless actively used, heuristics carry mandatory falsifiers, and worldviews emerge from co-citation.

**Subsumes**: Knowledge Entry (now Signal), Pheromone (now Signal), InsightStore, PheromoneRegistry, dream consolidation, AntiKnowledge, knowledge decay, Heuristic lifecycle, Resonator Networks.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality, demurrage, Kind system), [02-BLOCK](02-BLOCK.md) (Store protocol, Verify redesign), [07-AGENT-RUNTIME](07-AGENT-RUNTIME.md) (CognitiveWorkspace, somatic markers), [10-LEARNING-LOOPS](10-LEARNING-LOOPS.md) (L3 dream consolidation, hindsight relabeling)

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

In the unified vocabulary, **Memory** is a Store-protocol Block with demurrage, tier progression, dream consolidation, and HDC-based retrieval (see [doc-04, section 7](04-SPECIALIZATIONS.md#7-memory)).

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

A Memory Block manages the knowledge lifecycle:

1. **Ingest** — New Signals enter at Transient tier with initial balance 1.0
2. **Retrieve** — HDC similarity search + multi-dimensional scoring
3. **Demurrage** — Balance decays unless actively used (retrieved, cited, gate-passed, surprised)
4. **Promote/Demote** — Based on gate validation results and balance thresholds
5. **Consolidate** — Dream cycles compress episodes into durable knowledge ([doc-10 §4](10-LEARNING-LOOPS.md))
6. **Prune** — Below cold threshold, archive to cold storage

The Memory Block implements the Store protocol, meaning it conforms to `put / get / query / prune` — the same interface as FileStore or MemoryStore, but with demurrage semantics layered on top.

---

## 3. Demurrage Model

**Demurrage replaces Ebbinghaus decay.** Instead of passive time-based forgetting, Signals pay a holding cost for occupying store space. Active use — retrieval, citation, gate-pass, surprise — restores balance. This is economic selection pressure on knowledge: unique, actively-useful insights stay warm; redundant or stale entries fade.

### The balance field

Every Signal carries a `balance` field (see [doc-01 §2](01-SIGNAL.md)):

```rust
pub struct Signal {
    // ...
    pub balance: f64,                    // starts at 1.0, decays via demurrage
    pub demurrage_paid: f64,             // monotonic, for observability
    pub last_touched_at: DateTime<Utc>,  // last reinforcement event
    // ...
}
```

### Demurrage rate

```
balance(t) = balance(t-1) - demurrage_rate * dt
```

Where `demurrage_rate` is per-Kind and per-Tier:

```
demurrage_rate = base_rate(kind) / tier_multiplier(tier)
```

| Kind | Base rate (balance/day) | Rationale |
|---|---|---|
| `Insight` | 0.033 | ~30 days to zero without reinforcement |
| `Heuristic` | 0.011 | ~90 days — behavioral rules are durable once proven |
| `Warning` | 24.0 | ~1 hour — warnings are transient by nature |
| `CausalLink` | 0.017 | ~60 days — causal models need varied testing |
| `StrategyFragment` | 0.071 | ~14 days — strategies in evolving codebases go stale |
| `AntiKnowledge` | 0.033 | ~30 days — what-not-to-do stays relevant |

### Tier multipliers

| Tier | Multiplier | Effect on base rate |
|---|---|---|
| Transient | 0.1x | Decays 10x faster |
| Working | 0.5x | Decays 2x faster |
| Consolidated | 1.0x | Base rate |
| Persistent | 5.0x | Decays 5x slower |

**Example**: An Insight at Transient tier has effective rate `0.033 / 0.1 = 0.33 balance/day`, reaching cold threshold in ~3 days. The same Insight at Persistent tier has rate `0.033 / 5.0 = 0.0066 balance/day`, lasting ~150 days.

### Reinforcement kinds

Active use restores balance. Each reinforcement kind has a different weight, and reinforcement is novelty-weighted:

| Reinforcement Kind | Balance Restored | Condition |
|---|---|---|
| **Retrieved** | `+0.05 * novelty` | Signal was returned in a Memory query |
| **Cited** | `+0.10 * novelty` | Signal was included in a composed context that passed a gate |
| **Gate-passed** | `+0.15 * novelty` | Signal was in the context pack of a successful gate evaluation |
| **Surprised** | `+0.20 * novelty` | Signal was relevant to a high-PE observation (PE > 0.40) |

### Novelty-weighted reinforcement

Repeated retrieval of the same Signal produces diminishing reinforcement:

```
novelty = 1 / (1 + ln(retrieval_count))
```

The first retrieval restores full balance (`novelty = 1.0`). The 10th restores ~0.30x. The 100th restores ~0.18x. This prevents a popular-but-mediocre Signal from staying warm purely through high retrieval frequency. Genuinely novel use (new context, new agent, new domain) resets the retrieval counter.

### Cold threshold

When balance drops below `cold_threshold` (default: 0.05), the Signal is a candidate for cold storage archival:

```rust
pub const COLD_THRESHOLD: f64 = 0.05;
```

Cold Signals are archived to `.roko/neuro/cold/`, preserving their content hash, lineage, and provenance. They can be thawed later if conditions change (e.g., a Resonator Network factorization reveals a useful constituent).

### Frozen Signals

When a knowledge Signal accumulates enough consensus support (3+ validators across distinct contexts), it can be frozen. Frozen Signals skip demurrage entirely. They remain in the store at their current balance indefinitely. The `freeze()` operation requires consortium approval or explicit human action.

### Ebbinghaus as special case

Pure time-based decay (Ebbinghaus) is recovered when no interactions occur: a Signal that is never retrieved, cited, or gate-passed decays linearly by the demurrage rate. Demurrage generalizes Ebbinghaus by adding an economic mechanism — use restores value.

---

## 4. Heuristic as First-Class Kind

A **Heuristic** is a first-class Signal kind with structured `when/then` clauses, a mandatory falsifier, and a calibration track record. Heuristics are the system's actionable knowledge — behavioral rules that predict outcomes.

```rust
pub struct HeuristicPayload {
    // ── Rule ────────────────────────────────────────────────
    pub when_clause: String,             // "When refactoring Rust code"
    pub then_clause: String,             // "Run clippy before committing"
    pub confidence: f64,                 // 0.0..=1.0

    // ── Falsifier (mandatory) ───────────────────────────────
    pub falsifier: String,               // "If clippy pass rate drops below 80%
                                         //  after following this heuristic"
    pub falsifier_checked_count: u64,    // times the falsifier was evaluated
    pub falsifier_triggered_count: u64,  // times the falsifier fired (heuristic wrong)

    // ── Calibration ─────────────────────────────────────────
    pub calibration: CalibrationRecord,
    pub receipts: Vec<CalibrationReceipt>,  // lineage to specific outcomes

    // ── Lineage ─────────────────────────────────────────────
    pub source_episodes: Vec<SignalRef>,
    pub parent_heuristic: Option<SignalRef>, // if refined from a violated parent
    pub children: Vec<SignalRef>,            // refined children spawned from this
}

pub struct CalibrationRecord {
    pub predictions: u64,                // total predictions made
    pub correct: u64,                    // predictions confirmed by gate
    pub score: f64,                      // correct / predictions (running)
    pub brier_score: f64,               // Brier score (lower = better calibrated)
    pub last_calibrated_at: DateTime<Utc>,
}

pub struct CalibrationReceipt {
    pub episode_ref: SignalRef,          // which episode tested this heuristic
    pub predicted: bool,                 // what the heuristic predicted (pass/fail)
    pub actual: bool,                    // what actually happened
    pub timestamp: DateTime<Utc>,
}
```

### Mandatory falsifier

Every Heuristic MUST carry a falsifier — a concrete condition under which the heuristic should be considered wrong. This is Popper's falsificationism applied to learned rules. A heuristic without a falsifier cannot be created.

The falsifier serves two purposes:

1. **Epistemic hygiene**: It forces the system to articulate *how* the heuristic could be wrong, preventing unfalsifiable belief accumulation.
2. **Automatic retirement**: When the falsifier fires enough times (calibration score drops below threshold), the heuristic is automatically retired or refined.

### Heuristic lifecycle

```
Birth ──► Test ──► Calibrate ──► Retire/Evolve
```

#### Birth

Heuristics are born from L3 dream consolidation ([doc-10 §4](10-LEARNING-LOOPS.md)): when 5+ confirmed Insights cluster around the same when/then pattern, the D2 distillation stage produces a Heuristic Signal with an auto-generated falsifier.

Heuristics can also be created manually via `roko knowledge heuristic create`.

#### Test

Every time the Heuristic's `when_clause` matches the current context and the Heuristic is included in the CognitiveWorkspace context pack ([doc-07 §10](07-AGENT-RUNTIME.md)), the system records whether the `then_clause` prediction was correct:

1. Heuristic says "when X, then Y will work"
2. Agent follows Y
3. Gate evaluates outcome
4. Gate pass → calibration receipt (predicted: true, actual: true)
5. Gate fail → calibration receipt (predicted: true, actual: false)

#### Calibrate

The CalibrationRecord updates on each receipt:

```
calibration.score = correct / predictions
calibration.brier_score = mean((predicted_prob - actual)^2)
```

A well-calibrated heuristic has a high calibration score (close to 1.0) and a low Brier score (close to 0.0).

Calibration score affects the heuristic's bid in the CognitiveWorkspace VCG auction: poorly calibrated heuristics lose prompt space to better-calibrated ones. This is the **heuristic calibration loop** described in [doc-10 §9.2](10-LEARNING-LOOPS.md).

#### Retire/Evolve

When a heuristic's falsifier fires enough times:

```
if falsifier_triggered_count / falsifier_checked_count > retire_threshold:
    retire heuristic (demote to cold storage)
    spawn refined children (narrower when-clauses)
```

Retirement does not delete the heuristic — it archives it and spawns refined children. A heuristic "When refactoring code, run clippy" that fails for JavaScript files might spawn two children: "When refactoring Rust code, run clippy" and "When refactoring TypeScript code, run eslint."

Children carry a `parent_heuristic` reference for lineage tracking.

### Heuristic configuration

```toml
[knowledge.heuristic]
falsifier_required = true              # cannot create heuristic without falsifier
retire_threshold = 0.30                # retire if falsifier fires >30% of checks
min_calibration_score = 0.50           # demote if calibration drops below this
min_receipts_for_promotion = 10        # need 10+ receipts before promoting
auto_refine = true                     # auto-spawn children on retirement
max_children = 5                       # max refined children per retirement
```

---

## 5. Worldviews

**Worldviews** emerge from co-citation clusters of heuristics with high calibration scores. They are not explicitly created — they are discovered patterns in how heuristics reinforce each other.

### How worldviews form

When multiple heuristics are frequently cited together in successful gate evaluations, they form a co-citation cluster. The system identifies these clusters during L3 dream consolidation:

```
Heuristic A: "When building APIs, use typed schemas"
Heuristic B: "When deploying, run integration tests"
Heuristic C: "When refactoring, maintain backward compatibility"

Co-citation frequency: A+B (47 times), A+C (38 times), B+C (42 times)
All three: 34 times

→ These three form a worldview: "careful API engineering"
```

### Worldview representation

```rust
pub struct Worldview {
    pub id: SignalId,
    pub name: String,                        // auto-generated or human-assigned
    pub heuristics: Vec<SignalRef>,           // constituent heuristics
    pub co_citation_matrix: BTreeMap<(SignalRef, SignalRef), u64>,
    pub avg_calibration: f64,                // mean calibration of constituents
    pub domain: String,
    pub rival_worldviews: Vec<SignalRef>,     // competing worldviews for same domain
}
```

### Multiple worldviews deliberately

The system maintains multiple worldviews for each domain:

| Worldview Role | Purpose | Example |
|---|---|---|
| **Main** | Highest avg calibration, used by default | "careful API engineering" |
| **Challenger** | Second-highest, used for 15% contrarian retrieval | "move-fast API engineering" |
| **Niche specialists** | High calibration for specific sub-domains | "high-throughput streaming APIs" |

The 15% contrarian retrieval from somatic markers ([doc-07 §9](07-AGENT-RUNTIME.md)) naturally consults challenger worldviews, preventing the dominant worldview from becoming an unchallenged orthodoxy.

### Worldview evolution

When a challenger worldview's avg calibration exceeds the main worldview's for 20+ consecutive evaluations, they swap roles. The former main becomes the new challenger. This mechanism allows the system's collective beliefs to shift in response to changing environments without catastrophic forgetting.

---

## 6. Knowledge Signal Kinds

Knowledge is stored as Signals with specific Kinds. In the unified vocabulary, what was previously "Knowledge Entry" is now **Signal (persisted, knowledge Kind)**.

```rust
pub enum KnowledgeKind {
    /// Observation with evidence. "Tests run 30% faster with parallel execution."
    Insight,
    /// Behavioral rule with when/then + mandatory falsifier + calibration.
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

Each Kind has different demurrage rates (§3), promotion criteria (§8), and retrieval behavior. The Kind discriminant on the Signal determines how the Memory Block handles it.

### On-chain representation

Each on-chain entry is approximately 1,340 bytes. No natural language touches the chain — content is stored off-chain with an on-chain hash commitment.

```rust
pub struct OnChainKnowledgeSignal {
    pub id: SignalId,
    pub kind: KnowledgeKind,
    pub content_hash: [u8; 32],
    pub confidence: u16,                 // Fixed-point 0..65535
    pub tier: KnowledgeTier,
    pub tags: Vec<String>,
    pub author_wallet: Address,
    pub created_at: u64,
    pub validated_count: u32,
    pub challenged_count: u32,
    pub hdc_fingerprint: [u8; 1280],     // PP-HDC encoded (non-invertible)
    pub frozen: bool,
}
```

Off-chain content lives in JSONL files at `.roko/neuro/knowledge.jsonl`. The on-chain record stores only the commitment hash.

---

## 7. HDC Embeddings and Resonator Networks

The knowledge system encodes structured information as 10,240-bit binary vectors. No floating point. No matrix multiply. No GPU.

### The vector

```rust
/// 10,240-bit binary sparse distributed vector.
pub struct HdcVector {
    bits: [u64; 160],  // 160 words * 64 bits = 10,240 bits = 1,280 bytes
}
```

Implementation lives in `roko-primitives/src/hdc.rs`.

### Core operations

**Bind (XOR).** Combines two vectors into one dissimilar to both inputs. Used for role-filler binding: `bind(ROLE, value)` creates a composite encoding. XOR is its own inverse — `bind(bind(a, b), b) == a`.

**Bundle (majority vote).** Combines multiple vectors into one similar to all inputs. Used for aggregation.

**Permute (bit rotation).** Encodes position and sequence. `permute(v, 1)` shifts all bits left by 1 (cyclic).

**Similarity (Hamming distance).** Hardware POPCNT. Two random 10,240-bit vectors are ~50% similar by chance. Meaningful similarity starts around 0.52-0.53.

### Role-filler encoding

Structured knowledge enters a single vector through role-filler binding:

| Role | Filler | Purpose |
|---|---|---|
| `task_description` | Task prompt text | What was attempted |
| `domain` | Domain tag | Which problem area |
| `model` | Model identifier | Which model ran |
| `tool_call_0..N` | Permuted tool call vectors | Ordered tool sequence |
| `outcome` | "success" or "failure" | Result |
| `file_path_0..N` | Modified file paths | What changed |
| `error_pattern` | Failure reason (if any) | What went wrong |

### Resonator Networks

**Resonator Networks** factorize bundled HDC vectors to recover their constituent parts. This is the inverse problem: given a bundle `B = bundle(bind(R1,F1), bind(R2,F2), bind(R3,F3))`, recover the original role-filler pairs.

```rust
pub struct ResonatorNetwork {
    pub codebooks: BTreeMap<String, Vec<HdcVector>>,  // per-role codebooks
    pub max_iterations: usize,                         // convergence limit
    pub similarity_threshold: f32,                     // match threshold
}

impl ResonatorNetwork {
    /// Factorize a bundle into role-filler estimates.
    pub fn factorize(
        &self,
        bundle: &HdcVector,
        roles: &[String],
    ) -> Vec<(String, HdcVector, f32)>;  // (role, filler_estimate, confidence)
}
```

### Why Resonator Networks matter

1. **Knowledge deduplication**: When two Signals have similar bundles, factorization can reveal whether they encode the same structured content (true duplicates) or merely similar content (false positives from HDC collision).

2. **Constituent extraction**: A complex episode fingerprint can be factorized to identify which specific tool sequences, error patterns, or domain contexts contributed to the pattern.

3. **Cross-domain transfer**: Factorization reveals shared sub-structure across domains. An "API retry pattern" in networking and a "retry pattern" in database operations share the same abstract structure when factorized — the `retry_strategy` role-filler pair is similar even though the `domain` role-fillers differ.

4. **HDC cleanup**: During L3 consolidation ([doc-10 §9.3](10-LEARNING-LOOPS.md)), Resonator Networks periodically factorize bundled vectors to identify constituent patterns that have been independently learned at higher tiers. When a bundle's constituents all exist separately, the bundle is pruned as redundant.

### Performance targets

| Operation | Target | Notes |
|---|---|---|
| Encode one vector | < 5 us | Seed + splitmix64 expansion |
| Similarity (two vectors) | < 1 us | XOR + POPCNT, cache-friendly |
| Search 1K patterns | < 100 us | Brute-force, no index needed |
| Search 10K patterns | < 1 ms | Still brute-force at this scale |
| Resonator factorization (3 roles) | < 10 ms | Iterative convergence, ~5-20 iterations |

### Why HDC instead of float embeddings?

| Property | HDC (10,240-bit binary) | Float embeddings (1536-d float32) |
|---|---|---|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity | XOR + POPCNT (1 cycle) | Dot product (hundreds of FLOPs) |
| Compositionality | Native (bind/bundle/permute) | Requires learned operations |
| Factorization | Resonator Networks | No native support |
| Hardware | CPU SIMD lanes | GPU preferred for batch |
| Privacy | Non-invertible after PP-HDC | Invertible via decoder models |
| On-chain cost | ~$0.002 per entry | ~$0.01 per entry |
| Determinism | Identical seeds produce identical vectors | Depends on model version |

---

## 8. Tier System

Knowledge Signals progress through four tiers. Each tier applies a multiplier to the demurrage rate, slowing or accelerating balance loss.

### Tiers

```rust
pub enum KnowledgeTier {
    /// T0: New, unvalidated. Demurrage 10x faster.
    Transient,     // multiplier: 0.1x
    /// T1: Survived initial validation. Demurrage 2x faster.
    Working,       // multiplier: 0.5x
    /// T2: Repeatedly validated. Demurrage at base rate.
    Consolidated,  // multiplier: 1.0x
    /// T3: Consensus-backed. Demurrage 5x slower.
    Persistent,    // multiplier: 5.0x
}
```

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
| Working | Transient | 3+ consecutive gate failures OR balance below 0.15 |
| Transient | Cold | Balance below `COLD_THRESHOLD` (0.05) |

### Validation flow

When Agent B retrieves a knowledge Signal published by Agent A, uses it during a task, and passes a gate:

1. The gate-pass event generates a confirmation.
2. The Signal's balance is reinforced (`+0.15 * novelty` for gate-pass).
3. `validated_count` increments on A's Signal.
4. `last_touched_at` resets, extending effective lifetime.
5. A's reputation increases proportionally.

### Challenge flow

When an Agent believes a knowledge Signal is wrong:

1. The challenger submits a challenge with counter-evidence.
2. `challenged_count` increments.
3. If `challenged_count >= 3`, the Signal enters consortium review.
4. During review, balance is halved and the Signal is flagged in query results.
5. Resolution paths: **upheld** (challenges dismissed, balance restored), **refuted** (Signal converted to AntiKnowledge), or **revised** (author publishes amended version).

---

## 9. AntiKnowledge

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
| Above 0.7 | Halve the new Signal's initial balance (discount factor 0.5) |
| Above 0.9 | Reject the Signal outright — it is not stored |

### AntiKnowledge lifecycle

1. **Creation**: A knowledge Signal is refuted through the challenge flow (3+ challenges, consortium review upholds).
2. **Conversion**: The refuted Signal's Kind is changed to `AntiKnowledge`. Its content is preserved but its role inverts.
3. **Demurrage**: AntiKnowledge decays via demurrage like other Signals (30-day base at Consolidated tier). Old mistakes eventually stop blocking new discoveries.
4. **Override**: If overwhelming evidence contradicts an AntiKnowledge entry, the challenge flow can convert it back or archive it.

---

## 10. Dream Consolidation

Dream consolidation is the offline process where Agents compress raw episodes into durable knowledge. It runs when an Agent accumulates enough unprocessed experience — what the system calls "sleep pressure." Dream consolidation is a **Loop** specialization: a Graph that feeds output back to input on the delta timescale. The full four-phase cycle is defined in [doc-10 §4](10-LEARNING-LOOPS.md).

### Four phases

```rust
pub enum DreamPhase {
    /// NREM replay: priority replay of high-surprise episodes.
    NremReplay,
    /// Hindsight: relabel failed trajectories for achieved sub-goals.
    HindsightRelabeling,
    /// REM imagination: counterfactual generation.
    RemImagination,
    /// Integration: promote validated insights to higher tiers.
    Integration,
}
```

### Phase 1: NREM Replay

Select episodes with highest prediction error, cluster by HDC similarity, extract patterns into Insight Signals at Transient tier. Clusters with 3+ supporting episodes and 0.7+ confidence become candidates.

### Phase 2: Hindsight Relabeling

Failed trajectories are decomposed into sub-goals. Sub-goals that were achieved are relabeled as positive episodes and fed back into NREM replay. This recovers useful learning signal from at least 45% of otherwise-discarded episodes (see [doc-10 §4](10-LEARNING-LOOPS.md)).

### Phase 3: REM Imagination

Generate counterfactual scenarios from high-value Insights. Useful counterfactuals that would have passed gates become StrategyFragment Signals.

**Threat rehearsal** runs as a sub-phase: enumerate plausible threat scenarios, generate Warning Signals (ephemeral, published on Bus with short TTL).

### Phase 4: Integration

Promote validated Insights and StrategyFragments through tiers. Write new knowledge. The three-stage distillation pipeline:

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

## 11. Pheromone Mechanism (Pulse-based)

In the unified vocabulary, pheromones are **Pulses** (ephemeral) with a typed `PheromoneKind`, location hash, and intensity. They are not a separate primitive — they are Pulses published to the Bus that happen to carry pheromone semantics.

### Pheromone types

```rust
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

### Stigmergy

The pheromone mechanism implements digital stigmergy:

1. **Agents modify the shared environment** — deposit pheromone Pulses with typed intensity at a location hash.
2. **Future Agents observe modifications** — query by location hash, ranked by decayed intensity.
3. **Coordination emerges** — without direct communication.

### Demurrage on pheromones

Pheromone intensity decays via the same demurrage mechanism, but as Pulses they live on the Bus ring buffer rather than in Store. Default half-life is 1 hour. When intensity drops below 0.01, the pheromone Pulse expires from the ring buffer.

Reinforcement resets the decay clock: when multiple Agents independently deposit pheromone Pulses at the same location hash, the cumulative signal is strong and persists longer.

### Pipeline integration

During the **Observe step** (step 1) of the 9-step pipeline ([doc-07 §7](07-AGENT-RUNTIME.md)), an Agent reads the pheromone field for its current context. Pheromone gradients influence prediction error:

- **THREAT** signal: increases prior for danger, biases toward caution
- **OPPORTUNITY** signal: decreases threshold for exploration
- **WISDOM** signal: boosts confidence in related knowledge Signals
- **CURIOSITY** signal: increases prediction error, biases toward investigation

---

## 12. Knowledge in the 9-Step Pipeline

Knowledge participates at two points in the Agent's 9-step pipeline.

### RETRIEVE (Step 2)

During context assembly, the Agent queries the Memory store and assembles context via the CognitiveWorkspace VCG auction ([doc-07 §10](07-AGENT-RUNTIME.md)).

**Query flow**:

1. Compute an HDC fingerprint for the current task prompt.
2. Query local neuro store (similarity search, ~170us at 10K entries).
3. Optionally query InsightStore on-chain (same similarity function, chain latency).
4. Score results using ContextAssemblyWeights (HDC 40%, keyword 30%, utility 20%, freshness 10%, cross-domain +15%).
5. Results enter the VCG auction as knowledge bidders alongside `NeuroContextBidder`, `TaskContextBidder`, `ResearchContextBidder`, `HeuristicBidder`, and others.
6. Winning entries are injected into the system prompt.
7. Heuristic bidders are weighted by calibration score — poorly calibrated heuristics bid lower.

### REFLECT (Step 9)

After execution and gating:

1. Log the episode to `.roko/episodes.jsonl` with an HDC fingerprint.
2. If a gate passed, reinforce balance on any knowledge Signals that were in the context pack (gate-pass reinforcement).
3. If a gate failed, do NOT reinforce — demurrage continues uninterrupted, naturally demoting the Signal.
4. Check heuristic falsifiers: if any heuristic in the context pack had its falsifier condition met, record a calibration receipt.
5. Emit knowledge events on the Bus.

---

## 13. On-Chain Integration

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
    function getEntry(uint256 entryId) external view returns (...);
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

    event PheromoneDeposited(uint256 indexed id, address indexed depositor, uint8 ptype, uint16 intensity);
    event PheromoneReinforced(uint256 indexed id, uint16 newIntensity);
    event PheromoneExpired(uint256 indexed id);
}
```

Detailed chain integration in [doc-18 (On-Chain Registries)](18-ON-CHAIN-REGISTRIES.md).

---

## 14. Event Types

Knowledge and pheromone events are published as Pulses on the Bus.

### Knowledge events

```json
{"type": "knowledge.published", "signal_id": "a1b2c3", "kind": "Insight", "balance": 1.0}
{"type": "knowledge.reinforced", "signal_id": "a1b2c3", "kind": "gate_passed", "new_balance": 0.85, "novelty": 0.72}
{"type": "knowledge.challenged", "signal_id": "a1b2c3", "challenger": "agent-y", "reason_hash": "..."}
{"type": "knowledge.demurrage", "count": 142, "total_balance_lost": 3.21}
{"type": "knowledge.frozen", "signal_id": "a1b2c3", "validators": ["agent-x", "agent-z", "agent-w"]}
{"type": "knowledge.promoted", "signal_id": "a1b2c3", "old_tier": "Transient", "new_tier": "Working"}
{"type": "knowledge.demoted", "signal_id": "a1b2c3", "old_tier": "Working", "new_tier": "Transient"}
{"type": "knowledge.cold_archived", "signal_id": "a1b2c3", "final_balance": 0.04}
{"type": "heuristic.calibrated", "signal_id": "h1b2c3", "new_score": 0.82, "receipts": 47}
{"type": "heuristic.falsifier_triggered", "signal_id": "h1b2c3", "falsifier_count": 3}
{"type": "heuristic.retired", "signal_id": "h1b2c3", "children_spawned": 2}
{"type": "worldview.formed", "name": "careful-api-engineering", "heuristic_count": 5}
{"type": "worldview.swapped", "domain": "api", "new_main": "wv-1", "new_challenger": "wv-2"}
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
{"type": "dream.phase_changed", "agent_id": "agent-alpha", "phase": "HindsightRelabeling"}
{"type": "dream.hindsight_relabeled", "original_episode": "ep-42", "subgoals_recovered": 3}
{"type": "dream.insight_promoted", "signal_id": "d4e5f6", "old_tier": "Transient", "new_tier": "Working"}
{"type": "dream.heuristic_born", "signal_id": "h7e8f9", "when": "...", "then": "...", "falsifier": "..."}
{"type": "dream.completed", "agent_id": "agent-alpha", "insights_produced": 4, "duration_secs": 12}
```

---

## 15. API Endpoints

### Knowledge endpoints

```
GET    /api/knowledge/entries              List Signals (paginated, filtered)
GET    /api/knowledge/entries/:id          Get a single knowledge Signal
POST   /api/knowledge/publish              Publish a new knowledge Signal
POST   /api/knowledge/validate/:id         Validate an existing Signal
POST   /api/knowledge/challenge/:id        Challenge an existing Signal
GET    /api/knowledge/search               HDC similarity search
  ?vector=<base64>&top_k=10&domain=<domain>&kind=<kind>&min_balance=0.1
GET    /api/knowledge/stats                Store statistics (tier distribution, avg balance)
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

## 16. TOML Configuration

```toml
[knowledge]
store_path = ".roko/neuro/knowledge.jsonl"
max_entries = 100000

[knowledge.demurrage]
enabled = true
apply_interval = "1h"                  # how often to apply demurrage across store
cold_threshold = 0.05                  # archive below this balance
novelty_reset_on_new_context = true    # new agent/domain resets retrieval counter

[knowledge.demurrage.base_rates]
Insight = 0.033                        # balance/day
Heuristic = 0.011
Warning = 24.0
CausalLink = 0.017
StrategyFragment = 0.071
AntiKnowledge = 0.033

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

## 17. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Memory Block implements Store protocol (put/get/query/prune) | Unit test: CRUD operations on knowledge Signals |
| Knowledge Signals decay via demurrage with per-Kind rates | Unit test: compute balance at t=0, t=30d for Insight |
| Tier multipliers applied correctly (Transient decays 10x faster) | Unit test: compare Transient vs Persistent balance over time |
| Retrieval reinforces balance with novelty weighting | Unit test: retrieve 10 times, verify diminishing reinforcement |
| Gate-pass reinforcement restores balance | Integration test: pass gate, verify balance increase |
| Cold threshold triggers archival | Unit test: decay below 0.05, verify cold archival |
| Frozen Signals skip demurrage entirely | Unit test: freeze Signal, advance time, verify balance unchanged |
| Heuristic creation requires mandatory falsifier | Unit test: attempt create without falsifier, verify rejection |
| Heuristic calibration score updates on gate outcome | Integration test: pass/fail gates, verify calibration receipts |
| Heuristic retirement when falsifier fires above threshold | Integration test: trigger falsifier 30%+, verify retirement |
| Retired heuristic spawns refined children | Integration test: verify children with narrower when-clauses |
| Worldview forms from co-citation cluster | Integration test: co-cite 3 heuristics 20+ times, verify worldview |
| Worldview swap when challenger exceeds main calibration | Integration test: 20 consecutive evaluations |
| AntiKnowledge at 0.7 similarity halves initial balance | Unit test |
| AntiKnowledge at 0.9 similarity rejects entry | Unit test |
| HDC encode + similarity produces correct results | Unit test: encode similar structures, verify similarity > 0.6 |
| Resonator Network factorizes bundle into constituent role-fillers | Unit test: encode 3 role-fillers, bundle, factorize, verify recovery |
| Cross-domain bonus of 15% applied when domains differ | Unit test |
| Dream NREM phase clusters episodes by HDC similarity | Integration test |
| Dream hindsight relabeling recovers sub-goals from failures | Integration test |
| Dream REM phase generates counterfactuals | Integration test |
| Dream Integration writes new Signals at Transient tier | Integration test |
| D2 distillation produces Heuristic with when/then + falsifier | Integration test |
| Pheromone Pulses decay with 1-hour default half-life | Unit test |
| Pheromone reinforcement resets decay clock | Unit test |
| Novelty attenuation: retrieval_count=10 yields ~0.30 novelty | Unit test |
| RETRIEVE step queries store and enters VCG auction | Integration test |
| REFLECT step reinforces balance on gate pass | Integration test |
| Cold Signals archived with provenance | Integration test |

---

## 18. Crate Mapping

| Crate | Responsibility |
|---|---|
| `roko-neuro` | Local knowledge store, tier progression, demurrage, retrieval scoring, AntiKnowledge, heuristic lifecycle |
| `roko-primitives` | HdcVector (bind/bundle/permute/similarity), Resonator Networks, item memory, accumulators |
| `roko-dreams` | Dream cycle orchestration, NREM replay, hindsight relabeling, REM imagination, threat rehearsal |
| `roko-learn` | Episode logger, HDC fingerprinting, playbook store, efficiency tracking |
| `roko-serve` | HTTP endpoints for knowledge, heuristic, and pheromone APIs |
| `roko-chain` (phase 2) | On-chain InsightStore and PheromoneRegistry interactions |

---

## 19. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal/Pulse duality, demurrage field | [doc-01](01-SIGNAL.md) | §2, §5 |
| Store protocol | [doc-02](02-BLOCK.md) | §3.1 |
| Verify redesign (continuous reward, evidence) | [doc-02](02-BLOCK.md) | §3.3 |
| CognitiveWorkspace (VCG auction, section effects) | [doc-07](07-AGENT-RUNTIME.md) | §10 |
| Somatic markers (contrarian retrieval) | [doc-07](07-AGENT-RUNTIME.md) | §9 |
| Novelty attenuation | [doc-07](07-AGENT-RUNTIME.md) | §10 |
| L3 dream consolidation (4-phase) | [doc-10](10-LEARNING-LOOPS.md) | §4 |
| Hindsight relabeling | [doc-10](10-LEARNING-LOOPS.md) | §4 |
| Seven compounding feedback loops | [doc-10](10-LEARNING-LOOPS.md) | §9 |
| DriftLens (knowledge health) | [doc-09](09-TELEMETRY.md) | §3.6 |
| StateHub knowledge projection | [doc-09](09-TELEMETRY.md) | §6 |
| On-chain registries | [doc-18](18-ON-CHAIN-REGISTRIES.md) | — |
