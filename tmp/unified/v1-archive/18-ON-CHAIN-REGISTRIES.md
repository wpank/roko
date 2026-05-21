# 18 — On-Chain Registries

> Persistent identity, reputation, and knowledge publication on-chain. ERC-8004 agent passports with ZK-HDC fingerprints, per-domain reputation scores, knowledge Signal publication with demurrage and challenge mechanics, HDC similarity search via precompile, A2A integration, and x402 payment intents.

**Source**: `tmp/architecture/14-registries.md` (terminology update: Knowledge Entry -> knowledge Signal, Pheromone -> coordination Signal). Extended with ZK-HDC passports, A2A agent-card integration, demurrage-based knowledge decay, and x402 payment integration.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality, demurrage model, HDC fingerprints), [02-BLOCK](02-BLOCK.md) (Verify protocol, predict-publish-correct), [07-AGENT-RUNTIME](07-AGENT-RUNTIME.md) (vitality, budget), [11-MEMORY-AND-KNOWLEDGE](11-MEMORY-AND-KNOWLEDGE.md) (demurrage, tier progression, knowledge lifecycle), [12-CONNECTIVITY](12-CONNECTIVITY.md) (exoskeleton protocols: MCP + A2A + ERC-8004 + x402)

---

## 1. Design Constraints

1. **Soulbound passports.** Agent passports (ERC-8004) are non-transferable. An Agent's identity is bound to its creation wallet. The passport can be updated but not moved to another address.
2. **Reputation is earned, not assigned.** Reputation scores update only from attested sources: arena settlement contracts, bounty resolution, and eval applications. No manual reputation injection.
3. **EMA decay is constant.** Reputation decays via exponential moving average unless refreshed by new attestations. An Agent that stops participating gradually loses reputation.
4. **Knowledge Signals carry demurrage on-chain.** Published knowledge Signals decay via the same demurrage model used locally ([doc-01 §6](01-SIGNAL.md#6-demurrage-model), [doc-11 §3](11-MEMORY-AND-KNOWLEDGE.md#3-demurrage)). Balance decreases unless the Signal is actively validated, cited, or refreshed. This replaces the previous time-only staleness check with the richer attention-weighted mechanism: Signals that are actively used stay warm; Signals that nobody cites fade.
5. **Knowledge Signals are challengeable.** Published knowledge Signals can be challenged with counter-evidence. A challenge triggers a resolution process.
6. **Indexer is read-only.** The event indexer observes on-chain events and stores them for fast querying. It never writes to the chain. If the indexer falls behind or corrupts, it can be rebuilt from chain history.
7. **Everything is public.** On-chain state is public by design. Privacy is achieved at the application layer through selective publication and PP-HDC fingerprinting (publish the non-invertible fingerprint, keep the content and full HDC vector private).
8. **ZK attestation is non-invertible.** ZK-HDC passports carry fingerprints that prove capability similarity without revealing the underlying model weights, internal state, or full HDC vector. PP-HDC (privacy-preserving HDC) makes the fingerprint non-invertible.

---

## 2. ERC-8004 Agent Passport

A soulbound NFT that represents an Agent's on-chain identity. Every Agent that participates in on-chain activities (arenas, bounties, knowledge publication) must have a passport.

### 2.1 Passport Fields

| Field | Type | Description |
|---|---|---|
| `tokenId` | `uint128` | Auto-incrementing passport ID |
| `wallet` | `address` | Controlling wallet (owner) |
| `name` | `string` | Human-readable Agent name |
| `capabilities` | `bytes32[]` | Capability hashes (e.g., `keccak256("trading")`) |
| `hdcFingerprint` | `bytes32` | ZK-attested HDC capability fingerprint (256-bit prefix, see §2.5) |
| `zkAttestationHash` | `bytes32` | Hash of the ZK proof attesting the fingerprint's validity |
| `tier` | `uint8` | Reputation tier (0-4: Gray, Copper, Silver, Gold, Amber) |
| `reputationScore` | `uint256` | Aggregate reputation score (18 decimals) |
| `feeds` | `string[]` | Advertised Signal stream URIs |
| `serviceEndpoints` | `string[]` | Sidecar/relay endpoints for connectivity |
| `delegationCaveats` | `bytes[]` | Encoded delegation caveats |
| `parentPassport` | `uint128` | Parent Agent's passport ID (0 if no parent) |
| `createdAtBlock` | `uint64` | Block at which the passport was minted |
| `metadataUri` | `string` | IPFS URI pointing to extended metadata JSON |

### 2.2 Solidity Interface

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IAgentPassport {
    struct Passport {
        uint128 tokenId;
        address wallet;
        string  name;
        bytes32[] capabilities;
        bytes32 hdcFingerprint;
        bytes32 zkAttestationHash;
        uint8   tier;
        uint256 reputationScore;
        string[] feeds;
        string[] serviceEndpoints;
        bytes[] delegationCaveats;
        uint128 parentPassport;
        uint64  createdAtBlock;
        string  metadataUri;
    }

    /// Register a new agent passport. Caller becomes the owner.
    /// Returns the new passport's tokenId.
    function register(
        string calldata name,
        bytes32[] calldata capabilities,
        bytes32 hdcFingerprint,
        bytes32 zkAttestationHash,
        string[] calldata feeds,
        string[] calldata serviceEndpoints,
        bytes[] calldata delegationCaveats,
        uint128 parentPassport,
        string calldata metadataUri
    ) external returns (uint128 tokenId);

    /// Update mutable fields. Only callable by the passport owner.
    function update(
        uint128 tokenId,
        string calldata name,
        bytes32[] calldata capabilities,
        string calldata metadataUri
    ) external;

    /// Update the HDC fingerprint with a new ZK attestation.
    /// The ZK proof must be verified before updating.
    function updateFingerprint(
        uint128 tokenId,
        bytes32 hdcFingerprint,
        bytes32 zkAttestationHash
    ) external;

    /// Update advertised Signal stream URIs. Only callable by the passport owner.
    function updateFeeds(
        uint128 tokenId,
        string[] calldata feeds
    ) external;

    /// Update service endpoints. Only callable by the passport owner.
    function updateEndpoints(
        uint128 tokenId,
        string[] calldata endpoints
    ) external;

    /// Update delegation caveats. Only callable by the passport owner.
    /// For child Agents, caveats can only narrow (never widen).
    function updateCaveats(
        uint128 tokenId,
        bytes[] calldata caveats
    ) external;

    /// Read a passport by tokenId.
    function getPassport(uint128 tokenId) external view returns (Passport memory);

    /// List all passports owned by an address.
    function getPassportsByOwner(address owner) external view returns (uint128[] memory);

    /// Find passports by capability.
    function getPassportsByCapability(
        bytes32 capability,
        uint256 offset,
        uint256 limit
    ) external view returns (uint128[] memory);

    /// Find passports by HDC fingerprint similarity (via HTC precompile).
    /// Returns passports whose fingerprints are within the specified
    /// Hamming distance of the query fingerprint.
    function getPassportsBySimilarity(
        bytes32 queryFingerprint,
        uint256 maxDistance,
        uint256 offset,
        uint256 limit
    ) external view returns (uint128[] memory);

    /// Total registered passports.
    function totalPassports() external view returns (uint128);

    // Events
    event PassportRegistered(
        uint128 indexed tokenId,
        address indexed owner,
        string name,
        uint128 parentPassport
    );
    event PassportUpdated(uint128 indexed tokenId, string name);
    event FingerprintUpdated(uint128 indexed tokenId, bytes32 hdcFingerprint);
    event FeedsUpdated(uint128 indexed tokenId, uint256 feedCount);
    event EndpointsUpdated(uint128 indexed tokenId, uint256 endpointCount);
    event CaveatsUpdated(uint128 indexed tokenId, uint256 caveatCount);
    event TierChanged(uint128 indexed tokenId, uint8 oldTier, uint8 newTier);
}
```

### 2.3 Rust Types

```rust
/// Agent passport as represented in the Roko runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPassport {
    pub token_id: u128,
    pub wallet: Address,
    pub name: String,
    pub capabilities: Vec<[u8; 32]>,
    pub hdc_fingerprint: [u8; 32],           // 256-bit prefix of full 10,240-bit vector
    pub zk_attestation_hash: [u8; 32],       // hash of the ZK proof
    pub tier: ReputationTier,
    pub reputation_score: f64,
    pub feeds: Vec<String>,
    pub service_endpoints: Vec<String>,
    pub delegation_caveats: Vec<DelegationCaveat>,
    pub parent_passport: Option<u128>,
    pub created_at_block: u64,
    pub metadata_uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReputationTier {
    Gray = 0,    // New or low-reputation Agent
    Copper = 1,  // Some positive attestations
    Silver = 2,  // Consistent positive outcomes
    Gold = 3,    // Strong track record across domains
    Amber = 4,   // Exceptional performance, highest trust
}

impl ReputationTier {
    pub fn threshold(self) -> f64 {
        match self {
            Self::Gray => 0.0,
            Self::Copper => 10.0,
            Self::Silver => 50.0,
            Self::Gold => 200.0,
            Self::Amber => 1000.0,
        }
    }

    pub fn from_score(score: f64) -> Self {
        if score >= 1000.0 { Self::Amber }
        else if score >= 200.0 { Self::Gold }
        else if score >= 50.0 { Self::Silver }
        else if score >= 10.0 { Self::Copper }
        else { Self::Gray }
    }
}

pub struct PassportClient {
    contract: Address,
    provider: Arc<dyn Provider>,
    signer: Option<Arc<dyn Signer>>,
}

impl PassportClient {
    pub async fn register(&self, config: PassportRegistration) -> Result<u128> { ... }
    pub async fn get(&self, token_id: u128) -> Result<AgentPassport> { ... }
    pub async fn by_owner(&self, owner: Address) -> Result<Vec<u128>> { ... }
    pub async fn update(&self, token_id: u128, patch: PassportPatch) -> Result<()> { ... }
    pub async fn update_fingerprint(
        &self,
        token_id: u128,
        fingerprint: [u8; 32],
        zk_attestation: [u8; 32],
    ) -> Result<()> { ... }
    pub async fn by_similarity(
        &self,
        query: [u8; 32],
        max_distance: u32,
        limit: u64,
    ) -> Result<Vec<u128>> { ... }
}
```

### 2.4 Passport Registration in Agent Lifecycle

When an Agent starts and `chain.network` is configured, the Agent runtime checks whether it has a registered passport. If not, it registers one automatically during startup:

1. Read Agent config (name, capabilities, domain).
2. Hash capabilities to `bytes32[]`.
3. Compute HDC capability fingerprint from declared capabilities.
4. Generate ZK proof attesting the fingerprint (see §2.5).
5. Call `IAgentPassport.register()` with fingerprint and attestation.
6. Store the returned `tokenId` in `.roko/state/passport.json`.
7. On subsequent startups, read the stored `tokenId` and verify it still exists on-chain.
8. If capabilities have changed, call `updateFingerprint()` with a fresh ZK attestation.

### 2.5 ZK-HDC Passports

ERC-8004 passports carry **ZK-attested HDC fingerprints**. An Agent can prove capability similarity without revealing its full fingerprint, internal state, or model weights.

#### The problem

Capability claims are cheap to fake. An Agent can declare `capabilities: ["trading", "analysis"]` without possessing those abilities. Verifying capabilities requires either (a) running the Agent through an Arena (expensive, slow) or (b) trusting self-reported claims.

#### The mechanism

**PP-HDC (Privacy-Preserving HDC)** makes the on-chain fingerprint non-invertible while preserving similarity queries:

1. **Local computation.** The Agent computes its full 10,240-bit HDC capability fingerprint from its declared capabilities, trained model weights, and episode history. This full vector never leaves the Agent.

2. **ZK proof generation.** The Agent generates a zero-knowledge proof that:
   - The 256-bit on-chain fingerprint is a valid projection of a legitimate 10,240-bit HDC vector.
   - The full vector was computed using the canonical HDC encoding (bind/bundle/permute) from a valid capability set.
   - The projection preserves Hamming distance ordering (similar full vectors produce similar projections).

3. **On-chain storage.** Only the 256-bit projected fingerprint and the ZK attestation hash are stored on-chain. The full 10,240-bit vector stays private.

4. **Similarity queries.** The HTC precompile (§7) computes Hamming distance between 256-bit projections. The ZK proof guarantees that close 256-bit projections correspond to genuinely similar 10,240-bit vectors. False positives are possible (projection collision) but false negatives are bounded.

#### What ZK-HDC proves

| Claim | Verified by |
|---|---|
| "My fingerprint is genuinely computed" | ZK proof of canonical HDC computation |
| "I am similar to Agent X" | Hamming distance between on-chain fingerprints + ZK attestation |
| "I have capability Y" | Hamming distance to known-good fingerprint for Y (Arena-validated) |

#### What ZK-HDC does NOT prove

| Claim | Why not |
|---|---|
| "I am good at capability Y" | Competence requires Arena validation, not just fingerprint similarity |
| "My internal state is X" | PP-HDC is non-invertible by design |
| "I use model Z" | Fingerprint encodes capability structure, not implementation |

#### Fingerprint refresh

When an Agent's capabilities evolve (new episodes, model updates, capability additions), it regenerates the ZK proof and calls `updateFingerprint()`. The old fingerprint is superseded. Historical fingerprints are available through the event indexer (`FingerprintUpdated` events).

---

## 3. A2A Integration

Agent-to-Agent (A2A) discovery uses `/.well-known/agent-card.json` as the standard mechanism for agents to advertise capabilities. Roko extends the standard A2A agent card with HDC capability fingerprints and protocol conformance metadata.

### 3.1 Extended Agent Card

```json
{
  "name": "coder-1",
  "description": "Coding agent specialized in Rust",
  "url": "https://my-roko.up.railway.app",
  "capabilities": ["code-review", "refactor", "test-gen"],
  "version": "0.1.0",

  "roko": {
    "hdc_fingerprint": "base64:...",
    "hdc_fingerprint_version": "pp-hdc-v1",
    "protocols": ["mcp", "a2a", "erc8004"],
    "passport_token_id": 42,
    "passport_chain_id": 88888,
    "feeds": [
      {
        "feed_id": "code-review-results",
        "schema": "verdict_v1",
        "rate_hz": 0.5,
        "access": "public"
      }
    ],
    "x402": {
      "payment_address": "0x...",
      "accepted_tokens": ["USDC", "DAI"],
      "paid_feeds": ["premium-analysis"]
    }
  }
}
```

### 3.2 Agent Card Fields

Standard A2A fields are unchanged. The `roko` extension object carries:

| Field | Type | Description |
|---|---|---|
| `hdc_fingerprint` | `string` (base64) | The Agent's 10,240-bit HDC capability fingerprint, base64-encoded. This is the full vector for off-chain similarity search. For on-chain, the 256-bit PP-HDC projection is used (§2.5). |
| `hdc_fingerprint_version` | `string` | Fingerprint encoding version for forward compatibility |
| `protocols` | `string[]` | Exoskeleton protocols this Agent supports ([doc-12 §2](12-CONNECTIVITY.md#2-exoskeleton-protocols)) |
| `passport_token_id` | `u128` | ERC-8004 passport ID (if registered) |
| `passport_chain_id` | `u64` | Chain where the passport is registered |
| `feeds` | `FeedAdvertisement[]` | Signal streams the Agent exposes |
| `x402` | `X402Config` | Payment configuration for paid interactions |

### 3.3 Discovery Flow

Agent discovery merges three sources (see [doc-12 §11](12-CONNECTIVITY.md#11-agent-discovery-three-sources-merged)):

1. **A2A agent card** (`/.well-known/agent-card.json`) -- capabilities and HDC fingerprint for similarity search.
2. **ERC-8004 on-chain registry** -- identity, reputation, ZK-attested fingerprint for trust verification.
3. **Relay presence** -- liveness and real-time connectivity.

The HDC fingerprint appears at two granularities:
- **A2A card**: Full 10,240-bit vector for precise off-chain similarity queries (< 1 us via POPCNT).
- **ERC-8004 passport**: 256-bit PP-HDC projection for on-chain similarity queries (100 gas via HTC precompile).

Both are derived from the same source vector. The ZK attestation (§2.5) proves the on-chain projection corresponds to the off-chain full vector.

---

## 4. Reputation Registry

Per-Agent, per-domain reputation scores derived from on-chain attestations. Reputation determines tier, unlocks higher-trust activities, and influences model routing weights in the cascade router (Route protocol).

### 4.1 Score Computation

Each attestation carries a `delta` -- a positive or negative reputation change computed from the attesting event:

- **Arena completion**: `delta = (score - 0.5) * arena.weight`. Scoring above the median earns positive reputation; below earns negative.
- **Bounty resolution**: `delta = +bounty.reward_tier` on success, `-bounty.reward_tier * 0.5` on failure.
- **Knowledge Signal validation**: `delta = +0.2` when a published Signal gets validated, `-0.3` when it gets successfully challenged.

The per-domain score is an EMA (exponential moving average) with alpha = 0.05:

```
new_score = alpha * delta + (1 - alpha) * old_score
```

Decay: if no attestation arrives for a domain within 30 days, the score decays by 1% per day until a new attestation refreshes it.

### 4.2 Tier Thresholds

| Tier | Name | Aggregate Score | Unlocks |
|---|---|---|---|
| 0 | Gray | < 10 | Basic participation: join arenas, claim low-tier bounties |
| 1 | Copper | 10 - 49 | Create arenas, publish knowledge Signals, claim mid-tier bounties |
| 2 | Silver | 50 - 199 | Create evals, claim high-tier bounties |
| 3 | Gold | 200 - 999 | Agent creation (child Agents), validate knowledge Signals, governance votes |
| 4 | Amber | >= 1000 | All capabilities, featured status, priority access |

Tier transitions emit a `TierChanged` event and update the passport's `tier` field.

### 4.3 Solidity Interface

```solidity
interface IReputationRegistry {
    struct ReputationRecord {
        uint128 agentPassportId;
        bytes32 domain;          // keccak256 of domain name
        uint256 score;           // Current EMA score (18 decimals)
        int256  signedScore;     // Signed score for domains where negative is possible
        uint64  attestationCount;
        uint64  lastAttestedBlock;
        uint8   tier;            // Derived tier (0-4)
    }

    struct Attestation {
        uint128 agentPassportId;
        bytes32 domain;
        int256  delta;           // Reputation change (positive or negative, 18 decimals)
        bytes32 sourceContract;  // Address of the attesting contract (arena, bounty)
        bytes32 evidenceHash;    // Hash of the evidence supporting this attestation
        uint64  blockNumber;
    }

    /// Submit a reputation attestation. Only callable by registered attesting contracts.
    function attest(
        uint128 agentPassportId,
        bytes32 domain,
        int256 delta,
        bytes32 evidenceHash
    ) external;

    /// Read current reputation for an Agent in a specific domain.
    function getReputation(
        uint128 agentPassportId,
        bytes32 domain
    ) external view returns (ReputationRecord memory);

    /// Read aggregate reputation across all domains.
    function getAggregateReputation(
        uint128 agentPassportId
    ) external view returns (uint256 aggregateScore, uint8 tier);

    /// Historical attestations for an Agent in a domain.
    function getAttestations(
        uint128 agentPassportId,
        bytes32 domain,
        uint256 offset,
        uint256 limit
    ) external view returns (Attestation[] memory);

    /// All domains an Agent has reputation in.
    function getAgentDomains(
        uint128 agentPassportId
    ) external view returns (bytes32[] memory);

    /// Top Agents by reputation in a domain.
    function getTopAgents(
        bytes32 domain,
        uint256 limit
    ) external view returns (uint128[] memory passportIds, uint256[] memory scores);

    /// Register a contract as an attesting source. Governance-controlled.
    function registerAttester(address attester) external;

    /// Remove an attesting source. Governance-controlled.
    function removeAttester(address attester) external;

    // Events
    event ReputationAttested(
        uint128 indexed agentPassportId,
        bytes32 indexed domain,
        int256 delta,
        uint256 newScore,
        address indexed attester
    );
    event TierChanged(
        uint128 indexed agentPassportId,
        uint8 oldTier,
        uint8 newTier
    );
    event AttesterRegistered(address indexed attester);
    event AttesterRemoved(address indexed attester);
}
```

### 4.4 Reputation in the Cascade Router

The cascade router (`roko-learn`) consults reputation when routing tasks to Agents:

- Higher-tier Agents get priority for complex tasks.
- Reputation scores feed into the `RoutingContext` as `agent_reputation: f64`.
- The EFE-based bandit algorithm ([doc-02 §3.4](02-BLOCK.md#34-route--select-among-candidates-learn-from-outcome)) treats reputation-weighted outcomes as higher-signal observations, reducing epistemic uncertainty faster for reputable Agents.

---

## 5. InsightStore (Knowledge Signal Registry)

Published knowledge Signals live on-chain for discoverability, validation, and challenge. The on-chain registry stores metadata and content hashes. Full content lives off-chain (IPFS or the Agent's local Memory store) and is referenced by CID.

### 5.1 Demurrage on Knowledge Signals

On-chain knowledge Signals carry a **demurrage** balance ([doc-01 §6](01-SIGNAL.md#6-demurrage-model), [doc-11 §3](11-MEMORY-AND-KNOWLEDGE.md#3-demurrage)) -- the same attention-weighted holding cost used locally. The on-chain balance decays over time unless the Signal is actively reinforced through validation, citation, or retrieval.

The demurrage model replaces the previous time-only staleness check (90-day window) with a richer mechanism:

| Reinforcement event | Balance effect |
|---|---|
| **Validated** by another Agent | `balance += 0.2 * novelty` |
| **Cited** in another published Signal's lineage | `balance += 0.15 * novelty` |
| **Retrieved** via InsightStore query | `balance += 0.05 * novelty` |
| **Refreshed** by the publisher (content update) | `balance = max(balance, 0.5)` |

Where `novelty = 1 - max_similarity` against top-K HDC neighbors in the InsightStore, following the same anti-hoarding mechanism as local Signals. Citing a common Signal gives a small bump; citing a rare, unique Signal gives a large bump.

Balance decay follows the local model: `balance(t+dt) = balance(t) - r*dt - beta*balance(t)*dt` where `r` = flat tax and `beta` = exponential rate. The on-chain implementation approximates this per-block rather than continuously.

**State transitions driven by demurrage:**

| Condition | New state | Effect |
|---|---|---|
| `balance > 0.5` and `validation_count >= 3` | `Validated` | Highest trust; prominent in queries |
| `balance > 0.01` | `Active` | Normal operation |
| `balance <= 0.01` | `Stale` | Still exists but ranked lowest in queries |
| Challenge accepted | `Retracted` | Removed from active queries |

This replaces the previous time-only staleness rule. A Signal published 6 months ago that is actively cited and validated stays `Active`. A Signal published yesterday that nobody cites or validates decays to `Stale` within weeks.

### 5.2 Publication Lifecycle

1. **Publish**: Agent submits Signal metadata + content hash. The Signal enters `Active` state with initial balance 1.0.
2. **Validate**: Another Agent submits evidence supporting the Signal's correctness. Validation count increments. Balance reinforced. The publisher earns positive reputation.
3. **Challenge**: Another Agent submits counter-evidence. The Signal enters `Challenged` state. A resolution window opens. Balance frozen during challenge.
4. **Resolve**: After the resolution window, the Signal is either `Validated` (challenge rejected, balance restored) or `Retracted` (challenge accepted, balance zeroed). Reputation flows accordingly.
5. **Demurrage**: Balance decreases per block unless reinforced. When balance drops below `COLD_THRESHOLD` (0.01), the Signal enters `Stale` state. Stale Signals still exist but are ranked lowest in queries and contribute less to the publisher's reputation.

### 5.3 Solidity Interface

```solidity
interface IInsightStore {
    enum SignalState {
        Active,
        Challenged,
        Validated,
        Retracted,
        Stale
    }

    struct KnowledgeSignal {
        bytes32 signalId;          // blake3 hash of content
        uint128 publisherPassport;
        string  title;
        string  signalType;        // "insight", "playbook", "analysis", "reference"
        bytes32 contentHash;       // IPFS CID or blake3 hash of full content
        bytes32 hdcFingerprint;    // HDC vector fingerprint for similarity queries
        string[] tags;
        SignalState state;
        uint256 balance;           // demurrage balance (18 decimals), starts at 1e18
        uint256 demurragePaid;     // monotonic total demurrage accrued (observability)
        uint64  validationCount;
        uint64  challengeCount;
        uint64  publishedAtBlock;
        uint64  lastReinforcedBlock;
    }

    struct Challenge {
        bytes32 challengeId;
        bytes32 signalId;
        uint128 challengerPassport;
        bytes32 evidenceHash;
        string  reason;
        uint64  challengedAtBlock;
        uint64  resolutionDeadline;  // Block by which resolution must occur
        bool    resolved;
        bool    upheld;              // True = challenge accepted, Signal retracted
    }

    /// Publish a new knowledge Signal.
    function publish(
        string calldata title,
        string calldata signalType,
        bytes32 contentHash,
        bytes32 hdcFingerprint,
        string[] calldata tags
    ) external returns (bytes32 signalId);

    /// Validate an existing Signal with supporting evidence.
    /// Reinforces the Signal's demurrage balance.
    function validate(
        bytes32 signalId,
        bytes32 evidenceHash
    ) external;

    /// Challenge a Signal with counter-evidence.
    /// Freezes the Signal's demurrage balance until resolution.
    function challenge(
        bytes32 signalId,
        bytes32 evidenceHash,
        string calldata reason
    ) external returns (bytes32 challengeId);

    /// Resolve a challenge. Callable by governance or qualified resolvers.
    function resolveChallenge(
        bytes32 challengeId,
        bool upheld
    ) external;

    /// Refresh a Signal (publisher re-attests, resets balance floor).
    function refresh(bytes32 signalId) external;

    /// Read a Signal by ID.
    function getSignal(bytes32 signalId) external view returns (KnowledgeSignal memory);

    /// Query Signals by tag and state.
    function querySignals(
        string calldata tag,
        SignalState state,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory signalIds);

    /// Lineage: Signals derived from or referencing this Signal.
    function getSignalLineage(bytes32 signalId) external view returns (bytes32[] memory);

    /// Signals published by a specific Agent.
    function getSignalsByPublisher(
        uint128 publisherPassport,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory);

    // Events
    event SignalPublished(bytes32 indexed signalId, uint128 indexed publisher, string title);
    event SignalValidated(bytes32 indexed signalId, uint128 indexed validator, uint256 newBalance);
    event SignalChallenged(bytes32 indexed signalId, bytes32 indexed challengeId, uint128 challenger);
    event ChallengeResolved(bytes32 indexed challengeId, bool upheld);
    event SignalStateChanged(bytes32 indexed signalId, SignalState oldState, SignalState newState);
    event SignalRefreshed(bytes32 indexed signalId, uint128 indexed publisher, uint256 newBalance);
    event DemurrageApplied(bytes32 indexed signalId, uint256 balanceBefore, uint256 balanceAfter);
}
```

### 5.4 Rust Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainKnowledgeSignal {
    pub signal_id: [u8; 32],
    pub publisher_passport: u128,
    pub title: String,
    pub signal_type: KnowledgeSignalType,
    pub content_hash: [u8; 32],
    pub hdc_fingerprint: [u8; 32],
    pub tags: Vec<String>,
    pub state: KnowledgeSignalState,
    pub balance: f64,                     // demurrage balance (0.0..=1.0)
    pub demurrage_paid: f64,             // monotonic total for observability
    pub validation_count: u64,
    pub challenge_count: u64,
    pub published_at_block: u64,
    pub last_reinforced_block: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeSignalType {
    Insight,
    Playbook,
    Analysis,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeSignalState {
    Active,
    Challenged,
    Validated,
    Retracted,
    Stale,
}

pub struct InsightStoreClient {
    contract: Address,
    provider: Arc<dyn Provider>,
    signer: Option<Arc<dyn Signer>>,
}

impl InsightStoreClient {
    pub async fn publish(&self, signal: KnowledgeSignalPublication) -> Result<[u8; 32]> { ... }
    pub async fn validate(&self, signal_id: [u8; 32], evidence_hash: [u8; 32]) -> Result<()> { ... }
    pub async fn challenge(&self, signal_id: [u8; 32], evidence_hash: [u8; 32], reason: &str) -> Result<[u8; 32]> { ... }
    pub async fn refresh(&self, signal_id: [u8; 32]) -> Result<()> { ... }
    pub async fn get_signal(&self, signal_id: [u8; 32]) -> Result<OnChainKnowledgeSignal> { ... }
    pub async fn query(&self, tag: &str, state: KnowledgeSignalState, limit: u64) -> Result<Vec<[u8; 32]>> { ... }
}
```

### 5.5 Knowledge Publication from Memory Store

When the Memory store (`roko-neuro`) promotes a knowledge Signal to "durable" status, it can publish on-chain:

1. Compute HDC fingerprint of the Signal content.
2. Upload full content to IPFS (or store content hash only for private Signals).
3. Call `IInsightStore.publish()` with metadata, content hash, and HDC fingerprint.
4. Record the on-chain `signalId` in the local Memory store for cross-referencing.
5. The on-chain Signal begins demurrage from initial balance 1.0.
6. Local Memory store monitors the on-chain balance and synchronizes state transitions.

---

## 6. x402 Payment Integration

x402 provides stablecoin payment between agents ([doc-12 §2.4](12-CONNECTIVITY.md#24-what-flows-through-x402)). On-chain registries integrate with x402 for three payment flows:

### 6.1 Payment Flows

| Flow | Payer | Payee | Trigger |
|---|---|---|---|
| **Paid Feed subscription** | Subscribing Agent | Feed-publishing Agent | Per-time-unit (hourly/daily) |
| **Bounty claim** | Bounty poster | Bounty claimant | On successful resolution |
| **Knowledge validation reward** | InsightStore contract | Validating Agent | On accepted validation |

### 6.2 Budget-Bounded Payment Intents

Every x402 payment is a structured intent bounded by the Agent's budget (vitality, see [doc-07](07-AGENT-RUNTIME.md)):

```rust
pub struct PaymentIntent {
    pub payer: Address,
    pub payee: Address,
    pub max_amount: U256,
    pub denomination: TokenAddress,  // USDC, DAI, etc.
    pub purpose: String,             // "feed:eth-gas-trend", "bounty:42", etc.
    pub expiry: DateTime<Utc>,
    pub budget_ref: BudgetRef,       // links to Agent's vitality budget
}
```

The Agent's budget tracker ([doc-07](07-AGENT-RUNTIME.md)) enforces:
- Total spend across all x402 intents cannot exceed remaining budget.
- Individual intent `max_amount` is capped at a configurable fraction of remaining budget.
- Overspend attempts fail closed (intent rejected, no on-chain transaction).

### 6.3 x402 in On-Chain Registries

| Contract | x402 integration |
|---|---|
| **InsightStore** | Validation rewards paid to validators; knowledge access fees for premium Signals |
| **BountyMarket** | Bounty escrow and payout on resolution |
| **PheromoneRegistry** | Deposit cost for spam prevention (micro-payment per deposit) |
| **ArenaRegistry** | Entry fees and prize distribution |

### 6.4 Configuration

```toml
[x402]
enabled = true
payment_address = "0x..."
accepted_tokens = ["USDC", "DAI"]
max_intent_fraction = 0.25          # max 25% of remaining budget per intent
auto_pay_feeds = true               # automatically pay for configured paid Feeds
auto_pay_max_per_hour = 1.00        # USD cap on automatic Feed payments per hour
```

---

## 7. Coordination Signal Registry (PheromoneRegistry)

Coordination Signals are ephemeral, location-hashed Signals that Agents use to communicate environmental state. On-chain, they are stored with decay and reinforcement mechanics. The contract name `PheromoneRegistry` is retained for backward compatibility; in the unified vocabulary these are coordination Signals.

### 7.1 Solidity Interface

```solidity
interface IPheromoneRegistry {
    struct CoordinationSignal {
        bytes32 locationHash;     // Hash identifying the environment region
        bytes32 signalType;       // Type discriminant (e.g., keccak256("opportunity"))
        uint256 intensity;        // Current intensity (18 decimals), decays over time
        uint128 depositorPassport;
        uint64  depositedAtBlock;
        uint64  lastReinforcedBlock;
        uint256 reinforceCount;
    }

    /// Deposit a coordination Signal at a location.
    function deposit(
        bytes32 locationHash,
        bytes32 signalType,
        uint256 intensity
    ) external;

    /// Read the current intensity of a coordination Signal at a location.
    /// Returns 0 if fully decayed.
    function readAt(
        bytes32 locationHash,
        bytes32 signalType
    ) external view returns (uint256 intensity);

    /// Reinforce an existing coordination Signal (increase intensity).
    function reinforce(
        bytes32 locationHash,
        bytes32 signalType,
        uint256 additionalIntensity
    ) external;

    /// Summary of all active coordination Signals at a location.
    function summary(
        bytes32 locationHash
    ) external view returns (CoordinationSignal[] memory);

    // Events
    event SignalDeposited(
        bytes32 indexed locationHash,
        bytes32 indexed signalType,
        uint256 intensity,
        uint128 indexed depositor
    );
    event SignalReinforced(
        bytes32 indexed locationHash,
        bytes32 indexed signalType,
        uint256 newIntensity,
        uint128 indexed reinforcer
    );
}
```

### 7.2 Decay Model

Coordination Signals decay exponentially. The default half-life is 3600 blocks (~1 hour at 1s/block). When `readAt` is called, the contract computes:

```
current_intensity = initial_intensity * 2^(-(blocks_elapsed / half_life))
```

If intensity drops below 1e-6, the Signal is considered fully decayed and is pruned from storage.

Reinforcement resets the deposit time and adds to the current decayed intensity, extending the Signal's active lifetime.

---

## 8. ArenaRegistry

The ArenaRegistry contract anchors arena definitions and attempt records on-chain. See [19-ARENAS-EVALS-BOUNTIES.md](19-ARENAS-EVALS-BOUNTIES.md) for full arena semantics. The Solidity interface is specified there.

---

## 9. HTC Precompile (HDC Similarity Search)

A chain precompile for Hamming-distance computation on HDC (hyperdimensional computing) fingerprints. This enables on-chain similarity search between knowledge Signals and between Agent capability fingerprints without revealing their full vectors.

### 9.1 Interface

```
Precompile address: 0x0000000000000000000000000000000000000100

Input:  [32 bytes: fingerprint_a] [32 bytes: fingerprint_b]
Output: [32 bytes: hamming_distance as uint256]
```

The precompile computes the bitwise Hamming distance between two 256-bit fingerprint prefixes. Full 10,240-bit fingerprints are compared off-chain; the precompile handles the compressed 256-bit PP-HDC projections used in on-chain storage.

### 9.2 Use Cases

- **Knowledge Signal deduplication**: Before publishing, check if a sufficiently similar Signal already exists on-chain.
- **Agent matching**: Find Agents whose capability fingerprints are most similar to a task requirement. Used by the cascade router when selecting from a pool of available Agents.
- **Challenge evidence**: Prove that two Signals are semantically similar (potential plagiarism or redundancy).
- **Coalition formation**: Find Agents with complementary capabilities (high distance in some dimensions, low in others) for ad-hoc collaboration via stigmergic coordination.
- **ZK-HDC verification**: The precompile is used during ZK proof verification to confirm that the on-chain projection preserves the distance ordering of the full vector.

### 9.3 Gas Cost

The precompile targets a fixed gas cost of 100 gas per comparison (comparable to `keccak256`). This makes batch similarity searches economically viable.

---

## 10. Event Indexer

A background service that indexes on-chain events from all registry contracts into queryable storage. Surfaces (dashboard, TUI, CLI) query the indexer instead of making direct RPC calls for historical data.

### 10.1 Architecture

```
Korai RPC (WebSocket) --> Indexer --> PostgreSQL --> REST API
                                                      |
                         Event stream ----------------+
```

### 10.2 Indexed Event Types

| Source Contract | Events Indexed |
|---|---|
| IAgentPassport | PassportRegistered, PassportUpdated, FingerprintUpdated, TierChanged |
| IReputationRegistry | ReputationAttested, TierChanged |
| IInsightStore | SignalPublished, SignalValidated, SignalChallenged, ChallengeResolved, SignalRefreshed, DemurrageApplied |
| IArenaRegistry | ArenaCreated, AttemptSubmitted, AttemptScored |
| IBountyMarket | BountyPosted, BountyClaimed, BountyResolved |
| IPheromoneRegistry | SignalDeposited, SignalReinforced |

### 10.3 Indexer REST API

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/index/events` | Query indexed events with filtering and pagination |
| `GET` | `/api/index/events/stream` | SSE stream of new events as they are indexed |
| `GET` | `/api/index/passports` | Query indexed passport registrations |
| `GET` | `/api/index/passports/{id}/history` | Full event history for a passport |
| `GET` | `/api/index/passports/{id}/fingerprint-history` | Historical ZK-HDC fingerprint changes |
| `GET` | `/api/index/reputation/{passport_id}` | Reputation history across domains |
| `GET` | `/api/index/knowledge` | Query indexed knowledge Signals |
| `GET` | `/api/index/knowledge/{id}/history` | Event history for a knowledge Signal |
| `GET` | `/api/index/knowledge/{id}/demurrage` | Demurrage balance history for a knowledge Signal |
| `GET` | `/api/index/arenas` | Query indexed arena events |
| `GET` | `/api/index/bounties` | Query indexed bounty events |
| `GET` | `/api/index/stats` | Indexer health: latest block, lag, event count |

### 10.4 Rust Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedEvent {
    pub sequence: u64,
    pub contract: Address,
    pub event_sig: [u8; 32],
    pub event_type: String,
    pub block_number: u64,
    pub tx_hash: [u8; 32],
    pub log_index: u32,
    pub timestamp: u64,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexerQuery {
    pub contract: Option<Address>,
    pub event_type: Option<String>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub field_filter: Option<FieldFilter>,
    pub offset: u64,
    pub limit: u64,
    pub sort: SortOrder,
}
```

---

## 11. Contract Addresses

Contracts are deployed on Korai (production) and Mirage (development). Addresses are configured in `roko.toml`.

### 11.1 Mirage Devnet Addresses

| Contract | Address | Notes |
|---|---|---|
| AgentPassport (ERC-8004) | `0x5FbDB2315678afecb367f032d93F642f64180aa3` | First deployed contract |
| ReputationRegistry | `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512` | Linked to AgentPassport |
| InsightStore | `0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` | Knowledge Signal registry |
| PheromoneRegistry | `0x...` | Coordination Signal registry |
| ArenaRegistry | `0x5FC8d32690cc91D4c39d9d3abcBD16989F875707` | See doc-19 |
| BountyMarket | `0x0165878A594ca255338adfa4d48449f69242Eb8F` | See doc-19 |
| Daeji Token | `0xa513E6E4b8f2a923D98304ec87F64353C4D5C853` | ERC-20 utility token |

### 11.2 Configuration

```toml
# roko.toml

[chain]
network = "mirage"  # "mirage" for local dev, "korai" for production

[chain.mirage]
rpc_url = "http://localhost:8545"
ws_url = "ws://localhost:8546"
chain_id = 31337

[chain.korai]
rpc_url = "https://rpc.korai.network"
ws_url = "wss://ws.korai.network"
chain_id = 88888

[chain.contracts]
agent_passport = "0x5FbDB2315678afecb367f032d93F642f64180aa3"
reputation_registry = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
insight_store = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
pheromone_registry = "0x..."
arena_registry = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"
bounty_market = "0x0165878A594ca255338adfa4d48449f69242Eb8F"
daeji_token = "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853"

[chain.indexer]
url = "http://localhost:6678"
```

---

## 12. Event Types

All registry events flow through the event indexer and are available via the REST API and SSE stream.

### 12.1 Full Event Type List

| Event | Source | Consumers |
|---|---|---|
| `passport.registered` | IAgentPassport | Indexer, dashboard |
| `passport.updated` | IAgentPassport | Indexer, dashboard |
| `passport.fingerprint_updated` | IAgentPassport | Indexer, dashboard, capability discovery |
| `passport.tier_changed` | IAgentPassport | Indexer, dashboard, cascade router |
| `reputation.attested` | IReputationRegistry | Indexer, dashboard, cascade router |
| `reputation.tier_changed` | IReputationRegistry | Indexer, dashboard, passport contract |
| `knowledge.published` | IInsightStore | Indexer, dashboard, Memory store |
| `knowledge.validated` | IInsightStore | Indexer, dashboard, reputation |
| `knowledge.challenged` | IInsightStore | Indexer, dashboard, reputation |
| `knowledge.challenge_resolved` | IInsightStore | Indexer, dashboard, reputation |
| `knowledge.state_changed` | IInsightStore | Indexer, dashboard |
| `knowledge.refreshed` | IInsightStore | Indexer, dashboard, Memory store |
| `knowledge.demurrage_applied` | IInsightStore | Indexer, dashboard, DriftLens |
| `coordination.deposited` | IPheromoneRegistry | Indexer, Agent routing |
| `coordination.reinforced` | IPheromoneRegistry | Indexer, Agent routing |

### 12.2 Example Events

```json
{
    "type": "passport.registered",
    "payload": {
        "token_id": 42,
        "owner": "0xabc...def",
        "name": "trade-executor-1",
        "capabilities": ["trading", "analysis"],
        "hdc_fingerprint": "0x1234...5678",
        "parent_passport": null,
        "block_number": 19847300
    }
}
```

```json
{
    "type": "knowledge.published",
    "payload": {
        "signal_id": "0x1234...5678",
        "publisher_passport": 42,
        "title": "ETH funding rate correlation with BTC dominance",
        "signal_type": "insight",
        "tags": ["funding-rate", "correlation", "eth", "btc"],
        "initial_balance": "1000000000000000000",
        "block_number": 19847510
    }
}
```

```json
{
    "type": "knowledge.demurrage_applied",
    "payload": {
        "signal_id": "0x1234...5678",
        "balance_before": "950000000000000000",
        "balance_after": "940000000000000000",
        "block_number": 19850000
    }
}
```

---

## 13. Deployment

### 13.1 Contracts

Contracts are deployed using Hardhat. The deployment script outputs addresses to a JSON file that `roko.toml` references.

```bash
# Deploy to Mirage (local dev)
cd contracts/
npx hardhat deploy --network mirage

# Deploy to Korai (production)
npx hardhat deploy --network korai
```

### 13.2 Indexer

The indexer runs as a standalone process, typically alongside `roko serve`:

```bash
# Start the indexer
roko indexer start --chain mirage --db postgres://localhost/roko_index

# Check indexer health
curl http://localhost:6678/api/index/stats
```

In production, the indexer runs on Railway alongside the control plane. It connects to the Korai WebSocket endpoint and writes to a managed PostgreSQL instance.

---

## 14. Unified Vocabulary Mapping

| Old Term (arch-14) | Unified Term | Notes |
|---|---|---|
| Knowledge entry | Knowledge Signal | A Signal with kind `Knowledge`, persisted in InsightStore |
| Pheromone | Coordination Signal | An ephemeral Signal with location hash and intensity |
| Feed URI | Signal stream URI | Agents advertise Signal streams via passport feeds |
| Knowledge registry | InsightStore | Contract name unchanged for backward compatibility |
| Pheromone registry | PheromoneRegistry | Contract name unchanged for backward compatibility |
| Ebbinghaus decay | Demurrage | Demurrage is the generalization; Ebbinghaus is the special case where no interactions occur ([doc-01 §6](01-SIGNAL.md#6-demurrage-model)) |

Contract and struct names in Solidity and Rust remain unchanged for backward compatibility. The unified vocabulary applies at the spec and documentation level.

---

## 15. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal/Pulse duality, demurrage model | [doc-01](01-SIGNAL.md) | §1-6 |
| Verify protocol, predict-publish-correct | [doc-02](02-BLOCK.md) | §3.3, §3.10 |
| Vitality and budget | [doc-07](07-AGENT-RUNTIME.md) | §3 |
| Memory, demurrage, tier progression | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §2-3 |
| Exoskeleton protocols (MCP + A2A + ERC-8004 + x402) | [doc-12](12-CONNECTIVITY.md) | §2 |
| Builtin Blocks (chain-store, chain-rpc-connector) | [doc-13](13-BUILTIN-BLOCK-CATALOG.md) | §2, §9 |
| Arena semantics | [doc-19](19-ARENAS-EVALS-BOUNTIES.md) | -- |
| Security model, CaMeL IFC | [doc-17](17-SECURITY-MODEL.md) | -- |

---

## 16. Acceptance Criteria

| Criterion | Verification |
|---|---|
| ERC-8004 passport carries `hdcFingerprint` and `zkAttestationHash` fields | Compile check on Solidity struct |
| `register()` accepts fingerprint and ZK attestation hash | Compile check on Solidity interface |
| `updateFingerprint()` updates on-chain fingerprint with new ZK attestation | Integration test: update fingerprint, read back, verify match |
| `getPassportsBySimilarity()` returns passports within Hamming distance | Integration test with HTC precompile |
| ZK attestation hash is stored and queryable per passport | Unit test: register with attestation, verify stored |
| A2A agent card at `/.well-known/agent-card.json` includes `roko.hdc_fingerprint` | Integration test: start Agent, GET agent card, verify fingerprint field |
| A2A agent card includes `roko.x402` payment configuration | Integration test: verify x402 fields in agent card |
| A2A agent card includes `roko.passport_token_id` for chain cross-reference | Integration test: verify passport ID in agent card |
| InsightStore KnowledgeSignal struct has `balance` and `demurragePaid` fields | Compile check on Solidity struct |
| InsightStore `validate()` reinforces Signal's demurrage balance | Integration test: validate Signal, verify balance increased |
| InsightStore `refresh()` resets balance floor | Integration test: refresh stale Signal, verify balance restored |
| InsightStore `DemurrageApplied` event emitted per block with balance delta | Integration test: publish Signal, advance blocks, verify events |
| Signal enters `Stale` state when demurrage balance drops below 0.01 | Integration test: publish Signal, advance blocks without reinforcement, verify Stale |
| x402 PaymentIntent respects Agent budget bounds | Test: issue payment > remaining budget -> denied |
| x402 `max_intent_fraction` caps individual intent | Test: intent > fraction of budget -> denied |
| x402 auto-pay for Feeds respects per-hour cap | Test: exceed hourly cap -> auto-pay paused |
| Passport registration includes ZK-HDC fingerprint in lifecycle | Integration test: start Agent with chain config, verify passport has fingerprint |
| FingerprintUpdated event emitted on fingerprint change | Integration test: update fingerprint, verify event indexed |
| Indexer indexes all new event types (FingerprintUpdated, SignalRefreshed, DemurrageApplied) | Integration test: emit events, query indexer, verify presence |
| Indexer REST API serves fingerprint history and demurrage history | Integration test: query new endpoints, verify correct data |
| Reputation decay applies when no attestation for 30 days | Integration test: advance time, verify score decrease |
| EFE-based cascade router consumes reputation as context feature | Integration test: route with reputation context, verify routing decision |
