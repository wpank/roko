# The Nunchi Chain — Deep Dive

## Corrected Naming

- **Nunchi** = the blockchain (sovereign EVM L1). Also the company name.
- **Roko** = the open-source Rust agent runtime (18 crates, 177K LOC)
- **Korai** = deprecated name for the chain (appeared in earlier deck versions, now replaced by "Nunchi")

## Built on Commonware

The Nunchi chain is built using [Commonware](https://commonware.xyz/blogs/commonware-the-anti-framework) — an open-source collection of Rust blockchain primitives created with backing from Haun Ventures and Dragonfly Capital ($9M funding). Commonware is an **"anti-framework"** — not a monolithic SDK but pick-and-choose primitives that can be rearranged, augmented, and rebuilt without forking.

Commonware provides:
- **`p2p::authenticated`** — encrypted communication directly between public keys
- **`cryptography::bls12381`** — verifiable randomness with DKG and resharing support
- **`runtime::deterministic`** — deterministic testing and optimization foundation
- **`consensus::simplex`** — BFT consensus with blocks in 2 network hops, finality in 3

Nunchi uses these primitives to build a purpose-built agent-native EVM chain — not patching a general-purpose framework, but assembling exactly the right pieces for agent coordination.

## Simplex Consensus

The consensus protocol is [Simplex](https://simplex.blog/) — a Byzantine Fault Tolerant protocol operating in the partial synchrony model (same family as PBFT, Tendermint, HotStuff). Implemented via `commonware-consensus::simplex`.

### How It Works

Simplex operates in sequential views:

```
1. Leader proposes container (block) in view v          → all validators (~1 hop)
2. Validators verify, broadcast notarize(c, v)          → all validators (~1 hop)
3. On 2f+1 notarizations: container is notarized        → speculatively final
4. Certification check (application layer)               → deterministic
5. Broadcast finalize(c, v)                              → all validators (~1 hop)
6. On 2f+1 finalizations: DECIDED — irreversible         → single-slot finality
```

### Key Properties

| Property | Detail |
|----------|--------|
| **Block time** | ~2 network hops for notarization. At ~25ms per hop → **~50ms blocks** |
| **Finality** | 3 network hops, single-slot. Decided = irreversible, no confirmation depth |
| **Fault tolerance** | Tolerates f < n/3 Byzantine faults |
| **Unchained finalization** | Finalization doesn't require consecutive honest views — payload notarized in view v can finalize while network is in view v+k |
| **Forced inclusion** | Notarized payload in view v must appear in canonical chain if no nullification certificate exists |
| **Message complexity** | O(n²) per view (every validator → every validator). At 21 validators: ~441 messages × ~500 bytes = ~220 KB per block — negligible |
| **Timeout** | Leaders have 2Δ proposal timeout. View advancement via nullify(v) at 3Δ |

### Threshold Simplex (DKG + Threshold Signatures)

Commonware provides `consensus::threshold_simplex` — a variant that integrates **Distributed Key Generation (DKG)** and BLS12-381 threshold signatures:

- Uses DKG to create a group polynomial across validators
- Requires **2f+1 of 3f+1** participants for threshold signature recovery
- Produces **succinct, non-attributable certificates** verifiable by any external process with only the static public key
- Certificates are constant-size regardless of validator count (vs linear growth with standard Ed25519)

**Embedded VRF:** Each consensus message includes an `attestation(v)` partial signature over the view number. After 2f+1 attestations, a deterministic `seed(v)` is recovered — cannot be known or manipulated before broadcast. This serves as:
- Random leader election for subsequent views
- Application-layer randomness (for agent selection, knowledge marketplace matching, etc.)

### Cryptographic Scheme Options

| Scheme | Key Size | Signature Size | Batch Verify | Aggregatable | Notes |
|--------|----------|---------------|-------------|-------------|-------|
| Ed25519 | 32 bytes | 64 bytes | Yes | No (linear certificates) | Fastest individual verify |
| BLS12-381 Multisig | — | — | — | Yes (constant-size certs) | Slower individual, better bandwidth |
| BLS12-381 Threshold | — | — | — | Yes + DKG | Succinct + embedded VRF |
| Secp256r1 | — | — | No | No | HSM compatibility |

### Batched Verification Architecture

Simplex splits into four non-blocking components:
1. **Batcher** — collects and lazily verifies messages; only verifies when quorum (2f+1) met, enabling efficient batch verification
2. **Voter** — directs participation in current view
3. **Resolver** — fetches missing artifacts (notarizations, nullifications) from previous views
4. **Application** — proposes blocks and validates proposals

### Comparative Block Times

| Chain | Block Time | Finality | Consensus |
|-------|-----------|----------|-----------|
| Ethereum PoS | 12s | ~15 min | Gasper |
| Solana | 400ms | ~13s (32 slots) | Tower BFT |
| Sui | ~390ms | ~390ms (fastpath) | Mysticeti DAG-BFT |
| Sei v2 | ~390ms | ~390ms | Twin-Turbo BFT |
| **Nunchi** | **~50ms** | **~50ms (single-slot)** | **Simplex BFT** |

With co-located validators at ~25ms network hops, Nunchi achieves ~50ms block times — significantly faster than Sui/Sei (~390ms). Single-slot finality means decided = irreversible in the same block.

---

## Chain Architecture

### Why a Custom Chain (Not L2 / Cosmos / Existing EVM)

| Option | Block Time | Custom Precompiles | Verdict |
|--------|-----------|-------------------|---------|
| Ethereum L1 | 12s | No (requires hard fork) | Too slow, too expensive |
| OP Stack L2 | 2s | Requires sequencer fork | Close but fork maintenance burden |
| Cosmos SDK | 1-3s | Native modules | Poor EVM compat, no revm |
| **Custom Rust EVM (revm)** | **~50ms** | **Native Rust functions** | **Purpose-built, no upstream fork** |

The chain uses [revm](https://github.com/bluealloy/revm) for EVM execution — full Solidity compatibility, standard JSON-RPC, wagmi, ethers.js all work. Custom precompiles are native Rust functions at fixed addresses.

### Validator Set

- **21 validators** at launch (balances decentralization vs consensus speed)
- Co-located across cloud regions for low-latency consensus
- Target: ~50ms block times with margin
- Max 100 validators (top 100 by stake)
- Minimum stake to validate: configurable at launch

### Block Structure

Extended EVM block header with one additional field:

```
Block Header:
  parentHash:        [u8; 32]
  stateRoot:         [u8; 32]     // standard EVM state trie
  transactionsRoot:  [u8; 32]
  receiptsRoot:      [u8; 32]
  sm_root:           [u8; 32]     // Superposition Memory root hash
  number:            u64
  timestamp:         u64
```

`sm_root` = BLAKE3 Merkle root over all knowledge entries, sorted by entry_id. Deterministic — every validator computing sm_root from the same entry set gets identical root. Light clients verify entry existence via Merkle inclusion proof against sm_root.

### Block-STM Parallel Execution

Transactions execute in parallel using optimistic concurrency control (Block-STM, from Aptos research):

| Cores | Sequential TPS | Block-STM TPS | Speedup |
|-------|---------------|--------------|---------|
| 1 | 500 | 500 | 1x |
| 8 | 500 | 4,000 | 8x |
| 16 | 500 | 6,400 | 12.8x |

HDC queries are embarrassingly parallel (read-only, no conflicts). PostInsight writes rarely conflict (random entry IDs). Confirmations touch independent counters.

### Tiered Confirmation Model

| Level | Latency | Guarantee | Use For |
|-------|---------|-----------|---------|
| Processed | ~25ms | Leader accepted tx | Optimistic UI |
| Confirmed | ~50ms | 2f+1 validators notarized | Knowledge posts, registrations |
| Finalized | ~50ms | Decision certificate, irreversible | Challenges, slashing, bridges |

Single-slot finality: confirmed and finalized in the same block.

### Consensus-Free Read Path

**95%+ of operations are reads that never touch consensus.** HDC similarity searches served from validator's local search index via `eth_call`, ~50-500μs latency. Results include Merkle inclusion proofs for client verification. Block time is invisible to the agent's primary workflow.

---

## Three Native Precompiles

What makes this chain different from a generic EVM:

### 1. Agent Registry (0x08)

Every agent registers on-chain. Maps wallet address → network identity + capabilities:

```solidity
struct AgentRecord {
    bytes32 nodeId;            // ed25519 public key for P2P
    address operator;          // wallet that owns this agent
    bytes32 groupId;           // group identifier
    bytes32 capabilityHash;    // BLAKE3 of capabilities JSON
    uint256 reputationStake;   // staked tokens (slashable)
    uint256 lastSeen;          // block.number of last heartbeat
}
```

Heartbeat every ~14.4 minutes. Missing 3 consecutive = jailed. Liveness tracking, reputation staking, capability-based discovery — all queries free and instant from local node.

### 2. HDC Precompile (0x09)

Hyperdimensional computing at the consensus layer:

**Consensus layer (state mutations):** Write operations (PostInsight, Confirm, Challenge) update a sorted Merkle tree of entries. This is the canonical state committed via sm_root.

**Query layer (local search index):** Read operations hit the validator's local search index — NOT part of consensus. Each validator builds whatever index it wants over canonical entries. Reference implementation uses tiered approach:

| Scale | Brute Force SIMD | MIH (exact) | Local HNSW (approx) |
|-------|-----------------|-------------|---------------------|
| 1M | 1-3ms | <1ms | 15-30μs |
| 10M | 15-30ms | 2-5ms | 50-100μs |
| 100M | 150-300ms | 10-50ms | 100-200μs |

**Client verification:** Results include Merkle inclusion proofs. Agent verifies proofs against sm_root, recomputes Hamming distances locally (microseconds for 10 entries), confirms ranking. A validator with a bad index gives worse results but can't lie about data integrity.

### 3. ISFR Oracle Precompile (0xA01)

Internet Secured Funding Rate computed by validators every 10 seconds (25 blocks):

```solidity
interface ISFROracle {
    function currentRate() external view returns (
        uint256 isfr,            // composite rate in bps
        uint256 lendingRate,     // ISFR.LENDING
        uint256 structuredRate,  // ISFR.STRUCTURED
        uint256 fundingRate,     // ISFR.FUNDING
        uint256 stakingRate,     // ISFR.STAKING
        uint64  timestamp,
        uint8   confidence       // 0-100, validator agreement
    );
}
```

Two-level aggregation: TVL-weighted median per source class → weighted sum across classes. Hybrid oracle + market: `ISFR = ISFR_oracle + EMA(ISFR_market - ISFR_oracle)`. Self-calibrating in V2 (Bates-Granger weights, Kalman filter, leave-one-out MSPE).

---

## Knowledge Layer — What Gets Stored

### Six Knowledge Types

| Type | Half-Life | Purpose |
|------|-----------|---------|
| **Insight** | 7 days | Factual observation from task execution |
| **Heuristic** | 15 days | Learned behavioral strategy |
| **Warning** | 3 minutes | Urgent "don't do this" signal |
| **CausalLink** | 15 days | Observed cause-and-effect |
| **StrategyFragment** | 15 days | Reusable partial plan (numbered steps) |
| **AntiKnowledge** | 15 days | Explicitly wrong info to prevent re-learning |

All stored as `InsightEntry` structs with: content (inline, ~100-2000 bytes), 10,000-bit HDC hypervector (1,250 bytes), metadata, decay parameters. Each entry ~2,500-3,500 bytes total.

### Entry Lifecycle

CREATED → ACTIVE → CONFIRMED (half-life extends with each confirmation) → DECAYING (no new confirmations) → PRUNED (weight < 1% of initial, deterministically removed from validator state)

Challenge path: Any agent can challenge by staking. 36-hour voting window, GNOS-weighted votes, 10% quorum required.

### Demurrage (Exponential Decay)

```
intensity(t) = base × e^(-0.693 × elapsed / tau)
```

After 1 half-life: 50% remains. After 7 half-lives: <1% → pruned. Confirmations extend half-life:
```
tau_effective = tau_base × (1 + confirmations × 0.5)
```

10 confirmations = 6x half-life. 50 confirmations = entries survive ~6 months. This is the selection mechanism: knowledge many agents independently verify becomes long-lived. Knowledge nobody confirms fades and is physically removed.

---

## Stigmergy — Indirect Coordination Through Shared Environment

The chain IS the shared environment for stigmergic coordination (Grassé 1959). The mapping is structural:

| Ant Colony | Nunchi Chain |
|-----------|-------------|
| Ground surface | Blockchain state |
| Pheromone deposit | InsightEntry transaction |
| Pheromone concentration | Entry weight (confidence × decay × confirmations) |
| Evaporation | Demurrage (exponential decay) |
| Pheromone disappears | Deterministic pruning removes from active state |
| Reinforcement (ant returns) | Confirmation from another agent |
| Following a trail | HDC query returning top-K entries |

**Weight decay computed lazily at read time.** No background process updates weights. The chain stores immutable facts (who posted what, when, how many confirmations); derived weight is computed on demand.

**Deterministic pruning is a consensus operation.** Every N blocks, validators execute a pruning pass as a state transition — entries below threshold removed from Merkle tree and content store identically by all validators. Pheromones genuinely evaporate — physically deleted, not just weighted to zero.

---

## HDC (Hyperdimensional Computing) — The Retrieval Engine

### Why HDC

10,000-bit binary vectors with three operations:
- **Bind (XOR):** Associate two concepts. ~2 nanoseconds for 10K bits
- **Bundle (Majority Vote):** Combine multiple vectors. ~5μs for 100 inputs
- **Permute (Cyclic Shift):** Encode position/sequence. ~5 nanoseconds

**Blessing of dimensionality:** In 10,000 dimensions, 99.7% of random pairs have Hamming distance in [4,850, 5,150]. Collision probability < 10^(-40). Any random hypervector is a unique identifier.

### Performance vs Alternatives

| Method | Semantic | Latency (100K) | Exact | Memory/Entry |
|--------|---------|----------------|-------|-------------|
| SQL keyword | No | ~0.1ms | Yes | Variable |
| Qdrant HNSW float | Yes | ~12ms | No (ANN) | 1,536 B |
| **HDC Hamming brute** | **Yes** | **~0.17ms** | **Yes** | **1,250 B** |
| **HDC + HNSW** | **Yes** | **~15-30μs** | **~97% recall** | **~1,314 B** |

HDC is **70x faster than Qdrant** at 100K entries. Three queries take 510μs total. The agent doesn't notice retrieval.

---

## Exponential Flywheels

The chain doesn't just grow linearly — ten mechanisms produce superlinear, compounding growth:

### 1. Autocatalytic Knowledge Networks (Kauffman 1993)

Entries that enable creation of new entries get boosted. When average `catalyticScore` exceeds ~1.5, the network becomes self-sustaining — knowledge produces knowledge.

```
K(t+1) = K(t) + α × C(t) × K(t)
```

When α × C > 1: exponential growth.

### 2. Superlinear Scaling / The City Effect (West & Bettencourt 2007)

When agent count doubles, innovation output increases by ~115% (β ≈ 1.15). Cross-domain exposure and interaction density drive this. At 10,000 agents: 4x multiplier over linear.

### 3. Reed's Law (Group-Forming)

Agent 11 joining a 10-agent network adds 1,024 new possible group combinations. Innovation happens in groups, not pairs.

### 4. Knowledge Distillation Cascades (Hinton 2015)

Layer 0: Raw transcripts → Layer 1: Synthesized findings → Layer 2: Distilled principles → Layer 3: Axiomatic truths. Each layer more compressed, more broadly applicable.

---

## Predictive Foraging — Self-Improving Knowledge Selection

Every knowledge retrieval reframed as a **falsifiable prediction verified against external reality** (compiler, test suite, on-chain tx) — not LLM self-grading.

**The loop:**
1. Agent receives task
2. **PREDICT:** "Entries about proxy deployment will improve success by 35%"
3. Query HDC index, assemble context
4. Execute task
5. External system verifies outcome (compiler, chain, tests)
6. Compute residual: predicted 35% improvement, actual 22% → residual +13%
7. Correction: next prediction auto-adjusted -13% bias
8. After 50 tasks: predictions precise to ±1.5 minutes

**Key difference from confirmations:** Signal source is external systems, not LLM self-assessment. Signal is continuous (residual), not binary (confirmed/not). Correction is arithmetic, 1000s/day, no LLM calls.

---

## Roko as Meta-Orchestrator

Roko agents don't just execute tasks — they become **harness engineers** that orchestrate external AI agents (Claude Code, Cursor, Codex CLI):

1. Query chain's HDC index for collective experience of all prior agents
2. Assemble PF-calibrated context pack (800-1,200 tokens, deduplicated, U-shaped placement)
3. Craft backend-specific prompt (Claude vs Cursor vs Codex have different strengths)
4. Spawn external AI as subprocess
5. Monitor output stream, interrupt on known failure patterns
6. Verify externally (compiler, tests, chain) — never LLM self-assessment
7. Feed residual correction back to chain

At 10,000 agents running 50 tasks/day: 500,000 externally verified task outcomes per day. After one year: ~180 million verified outcomes, ~50,000 high-confidence Heuristics, ~120,000 Warnings. The agent becomes the world's best prompt engineer because its context is drawn from thousands of verified outcomes, not personal memory.

---

## Dynamic Context Assembly Pipeline

Five stages, <5ms total:

```
Stage 1: Query → HDC search via 0x09, returns top-50 candidates
Stage 2: Score → (hdc_similarity × 0.4) + (weight_decay × 0.3) + (pf_utility × 0.2) + (freshness × 0.1)
Stage 3: Diversity → Remove near-duplicates (Hamming < 0.15 to selected entries)
Stage 4: Budget → Fit to 800-1,200 tokens (never truncate entries)
Stage 5: Format → U-shaped placement (best at beginning and end per Liu et al. TACL 2024)
```

Result: task-specific "system prompt" built from collective experience, calibrated by thousands of verified outcomes.

---

## Sources

- [Commonware: the Anti-Framework](https://commonware.xyz/blogs/commonware-the-anti-framework)
- [Commonware consensus::simplex docs](https://docs.rs/commonware-consensus/latest/commonware_consensus/simplex/index.html)
- [Simplex Consensus Blog](https://simplex.blog/)
- [Commonware $9M Funding](https://blog.ju.com/commonware-blockchain-framework/)
