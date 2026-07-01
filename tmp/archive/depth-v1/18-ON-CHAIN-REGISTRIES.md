# 18 — On-Chain Registries

> Persistent identity, reputation, and knowledge publication on-chain. ERC-8004 agent passports, per-domain reputation scores, knowledge Signal publication with challenge mechanics, and HDC similarity search via precompile.

**Source**: `tmp/architecture/14-registries.md` (terminology update: Knowledge Entry -> knowledge Signal, Pheromone -> coordination Signal).

---

## 1. Design Constraints

1. **Soulbound passports.** Agent passports (ERC-8004) are non-transferable. An Agent's identity is bound to its creation wallet. The passport can be updated but not moved to another address.
2. **Reputation is earned, not assigned.** Reputation scores update only from attested sources: arena settlement contracts, bounty resolution, and eval applications. No manual reputation injection.
3. **EMA decay is constant.** Reputation decays via exponential moving average unless refreshed by new attestations. An Agent that stops participating gradually loses reputation.
4. **Knowledge Signals are challengeable.** Published knowledge Signals can be challenged with counter-evidence. A challenge triggers a resolution process.
5. **Indexer is read-only.** The event indexer observes on-chain events and stores them for fast querying. It never writes to the chain. If the indexer falls behind or corrupts, it can be rebuilt from chain history.
6. **Everything is public.** On-chain state is public by design. Privacy is achieved at the application layer through selective publication and HDC fingerprinting (publish the fingerprint, keep the content private).

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
}
```

### 2.4 Passport Registration in Agent Lifecycle

When an Agent starts and `chain.network` is configured, the Agent runtime checks whether it has a registered passport. If not, it registers one automatically during startup:

1. Read Agent config (name, capabilities, domain).
2. Hash capabilities to `bytes32[]`.
3. Call `IAgentPassport.register()`.
4. Store the returned `tokenId` in `.roko/state/passport.json`.
5. On subsequent startups, read the stored `tokenId` and verify it still exists on-chain.

---

## 3. Reputation Registry

Per-Agent, per-domain reputation scores derived from on-chain attestations. Reputation determines tier, unlocks higher-trust activities, and influences model routing weights in the cascade router (Route protocol).

### 3.1 Score Computation

Each attestation carries a `delta` -- a positive or negative reputation change computed from the attesting event:

- **Arena completion**: `delta = (score - 0.5) * arena.weight`. Scoring above the median earns positive reputation; below earns negative.
- **Bounty resolution**: `delta = +bounty.reward_tier` on success, `-bounty.reward_tier * 0.5` on failure.
- **Knowledge Signal validation**: `delta = +0.2` when a published Signal gets validated, `-0.3` when it gets successfully challenged.

The per-domain score is an EMA (exponential moving average) with alpha = 0.05:

```
new_score = alpha * delta + (1 - alpha) * old_score
```

Decay: if no attestation arrives for a domain within 30 days, the score decays by 1% per day until a new attestation refreshes it.

### 3.2 Tier Thresholds

| Tier | Name | Aggregate Score | Unlocks |
|---|---|---|---|
| 0 | Gray | < 10 | Basic participation: join arenas, claim low-tier bounties |
| 1 | Copper | 10 - 49 | Create arenas, publish knowledge Signals, claim mid-tier bounties |
| 2 | Silver | 50 - 199 | Create evals, claim high-tier bounties |
| 3 | Gold | 200 - 999 | Agent creation (child Agents), validate knowledge Signals, governance votes |
| 4 | Amber | >= 1000 | All capabilities, featured status, priority access |

Tier transitions emit a `TierChanged` event and update the passport's `tier` field.

### 3.3 Solidity Interface

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

### 3.4 Reputation in the Cascade Router

The cascade router (`roko-learn`) consults reputation when routing tasks to Agents:

- Higher-tier Agents get priority for complex tasks.
- Reputation scores feed into the `RoutingContext` as `agent_reputation: f64`.
- The bandit algorithm treats reputation-weighted outcomes as higher-signal observations.

---

## 4. InsightStore (Knowledge Signal Registry)

Published knowledge Signals live on-chain for discoverability, validation, and challenge. The on-chain registry stores metadata and content hashes. Full content lives off-chain (IPFS or the Agent's local Memory store) and is referenced by CID.

### 4.1 Publication Lifecycle

1. **Publish**: Agent submits Signal metadata + content hash. The Signal enters `Active` state.
2. **Validate**: Another Agent submits evidence supporting the Signal's correctness. Validation count increments. The publisher earns positive reputation.
3. **Challenge**: Another Agent submits counter-evidence. The Signal enters `Challenged` state. A resolution window opens.
4. **Resolve**: After the resolution window, the Signal is either `Validated` (challenge rejected) or `Retracted` (challenge accepted). Reputation flows accordingly.
5. **Decay**: Signals not validated or refreshed within 90 days enter `Stale` state. Stale Signals still exist but are ranked lower in queries.

### 4.2 Solidity Interface

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
        uint64  validationCount;
        uint64  challengeCount;
        uint64  publishedAtBlock;
        uint64  lastRefreshedBlock;
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
    function validate(
        bytes32 signalId,
        bytes32 evidenceHash
    ) external;

    /// Challenge a Signal with counter-evidence.
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
    event SignalValidated(bytes32 indexed signalId, uint128 indexed validator);
    event SignalChallenged(bytes32 indexed signalId, bytes32 indexed challengeId, uint128 challenger);
    event ChallengeResolved(bytes32 indexed challengeId, bool upheld);
    event SignalStateChanged(bytes32 indexed signalId, SignalState oldState, SignalState newState);
}
```

### 4.3 Rust Types

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
    pub validation_count: u64,
    pub challenge_count: u64,
    pub published_at_block: u64,
    pub last_refreshed_block: u64,
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
    pub async fn get_signal(&self, signal_id: [u8; 32]) -> Result<OnChainKnowledgeSignal> { ... }
    pub async fn query(&self, tag: &str, state: KnowledgeSignalState, limit: u64) -> Result<Vec<[u8; 32]>> { ... }
}
```

### 4.4 Knowledge Publication from Memory Store

When the Memory store (`roko-neuro`) promotes a knowledge Signal to "durable" status, it can publish on-chain:

1. Compute HDC fingerprint of the Signal content.
2. Upload full content to IPFS (or store content hash only for private Signals).
3. Call `IInsightStore.publish()` with metadata and content hash.
4. Record the on-chain `signalId` in the local Memory store for cross-referencing.

---

## 5. Coordination Signal Registry (PheromoneRegistry)

Coordination Signals are ephemeral, location-hashed Signals that Agents use to communicate environmental state. On-chain, they are stored with decay and reinforcement mechanics. The contract name `PheromoneRegistry` is retained for backward compatibility; in the unified vocabulary these are coordination Signals.

### 5.1 Solidity Interface

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

### 5.2 Decay Model

Coordination Signals decay exponentially. The default half-life is 3600 blocks (~1 hour at 1s/block). When `readAt` is called, the contract computes:

```
current_intensity = initial_intensity * 2^(-(blocks_elapsed / half_life))
```

If intensity drops below 1e-6, the Signal is considered fully decayed and is pruned from storage.

Reinforcement resets the deposit time and adds to the current decayed intensity, extending the Signal's active lifetime.

---

## 6. ArenaRegistry

The ArenaRegistry contract anchors arena definitions and attempt records on-chain. See [19-ARENAS-EVALS-BOUNTIES.md](19-ARENAS-EVALS-BOUNTIES.md) for full arena semantics. The Solidity interface is specified there.

---

## 7. HTC Precompile (HDC Similarity Search)

A chain precompile for Hamming-distance computation on HDC (hyperdimensional computing) fingerprints. This enables on-chain similarity search between knowledge Signals without revealing their content.

### 7.1 Interface

```
Precompile address: 0x0000000000000000000000000000000000000100

Input:  [32 bytes: fingerprint_a] [32 bytes: fingerprint_b]
Output: [32 bytes: hamming_distance as uint256]
```

The precompile computes the bitwise Hamming distance between two 256-bit fingerprint prefixes. Full 10,240-bit fingerprints are compared off-chain; the precompile handles the compressed 256-bit version used in on-chain storage.

### 7.2 Use Cases

- **Knowledge Signal deduplication**: Before publishing, check if a sufficiently similar Signal already exists on-chain.
- **Agent matching**: Find Agents whose capability fingerprints are most similar to a task requirement.
- **Challenge evidence**: Prove that two Signals are semantically similar (potential plagiarism or redundancy).

### 7.3 Gas Cost

The precompile targets a fixed gas cost of 100 gas per comparison (comparable to `keccak256`). This makes batch similarity searches economically viable.

---

## 8. Event Indexer

A background service that indexes on-chain events from all registry contracts into queryable storage. Surfaces (dashboard, TUI, CLI) query the indexer instead of making direct RPC calls for historical data.

### 8.1 Architecture

```
Korai RPC (WebSocket) --> Indexer --> PostgreSQL --> REST API
                                                      |
                         Event stream ----------------+
```

### 8.2 Indexed Event Types

| Source Contract | Events Indexed |
|---|---|
| IAgentPassport | PassportRegistered, PassportUpdated, TierChanged |
| IReputationRegistry | ReputationAttested, TierChanged |
| IInsightStore | SignalPublished, SignalValidated, SignalChallenged, ChallengeResolved |
| IArenaRegistry | ArenaCreated, AttemptSubmitted, AttemptScored |
| IBountyMarket | BountyPosted, BountyClaimed, BountyResolved |
| IPheromoneRegistry | SignalDeposited, SignalReinforced |

### 8.3 Indexer REST API

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/index/events` | Query indexed events with filtering and pagination |
| `GET` | `/api/index/events/stream` | SSE stream of new events as they are indexed |
| `GET` | `/api/index/passports` | Query indexed passport registrations |
| `GET` | `/api/index/passports/{id}/history` | Full event history for a passport |
| `GET` | `/api/index/reputation/{passport_id}` | Reputation history across domains |
| `GET` | `/api/index/knowledge` | Query indexed knowledge Signals |
| `GET` | `/api/index/knowledge/{id}/history` | Event history for a knowledge Signal |
| `GET` | `/api/index/arenas` | Query indexed arena events |
| `GET` | `/api/index/bounties` | Query indexed bounty events |
| `GET` | `/api/index/stats` | Indexer health: latest block, lag, event count |

### 8.4 Rust Types

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

## 9. Contract Addresses

Contracts are deployed on Korai (production) and Mirage (development). Addresses are configured in `roko.toml`.

### 9.1 Mirage Devnet Addresses

| Contract | Address | Notes |
|---|---|---|
| AgentPassport (ERC-8004) | `0x5FbDB2315678afecb367f032d93F642f64180aa3` | First deployed contract |
| ReputationRegistry | `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512` | Linked to AgentPassport |
| InsightStore | `0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` | Knowledge Signal registry |
| PheromoneRegistry | `0x...` | Coordination Signal registry |
| ArenaRegistry | `0x5FC8d32690cc91D4c39d9d3abcBD16989F875707` | See doc-19 |
| BountyMarket | `0x0165878A594ca255338adfa4d48449f69242Eb8F` | See doc-19 |
| Daeji Token | `0xa513E6E4b8f2a923D98304ec87F64353C4D5C853` | ERC-20 utility token |

### 9.2 Configuration

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

## 10. Event Types

All registry events flow through the event indexer and are available via the REST API and SSE stream.

### 10.1 Full Event Type List

| Event | Source | Consumers |
|---|---|---|
| `passport.registered` | IAgentPassport | Indexer, dashboard |
| `passport.updated` | IAgentPassport | Indexer, dashboard |
| `passport.tier_changed` | IAgentPassport | Indexer, dashboard, cascade router |
| `reputation.attested` | IReputationRegistry | Indexer, dashboard, cascade router |
| `reputation.tier_changed` | IReputationRegistry | Indexer, dashboard, passport contract |
| `knowledge.published` | IInsightStore | Indexer, dashboard, Memory store |
| `knowledge.validated` | IInsightStore | Indexer, dashboard, reputation |
| `knowledge.challenged` | IInsightStore | Indexer, dashboard, reputation |
| `knowledge.challenge_resolved` | IInsightStore | Indexer, dashboard, reputation |
| `knowledge.state_changed` | IInsightStore | Indexer, dashboard |
| `coordination.deposited` | IPheromoneRegistry | Indexer, Agent routing |
| `coordination.reinforced` | IPheromoneRegistry | Indexer, Agent routing |

### 10.2 Example Events

```json
{
    "type": "passport.registered",
    "payload": {
        "token_id": 42,
        "owner": "0xabc...def",
        "name": "trade-executor-1",
        "capabilities": ["trading", "analysis"],
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
        "block_number": 19847510
    }
}
```

---

## 11. Deployment

### 11.1 Contracts

Contracts are deployed using Hardhat. The deployment script outputs addresses to a JSON file that `roko.toml` references.

```bash
# Deploy to Mirage (local dev)
cd contracts/
npx hardhat deploy --network mirage

# Deploy to Korai (production)
npx hardhat deploy --network korai
```

### 11.2 Indexer

The indexer runs as a standalone process, typically alongside `roko serve`:

```bash
# Start the indexer
roko indexer start --chain mirage --db postgres://localhost/roko_index

# Check indexer health
curl http://localhost:6678/api/index/stats
```

In production, the indexer runs on Railway alongside the control plane. It connects to the Korai WebSocket endpoint and writes to a managed PostgreSQL instance.

---

## 12. Unified Vocabulary Mapping

| Old Term (arch-14) | Unified Term | Notes |
|---|---|---|
| Knowledge entry | Knowledge Signal | A Signal with kind `Knowledge`, persisted in InsightStore |
| Pheromone | Coordination Signal | An ephemeral Signal with location hash and intensity |
| Feed URI | Signal stream URI | Agents advertise Signal streams via passport feeds |
| Knowledge registry | InsightStore | Contract name unchanged for backward compatibility |
| Pheromone registry | PheromoneRegistry | Contract name unchanged for backward compatibility |

Contract and struct names in Solidity and Rust remain unchanged for backward compatibility. The unified vocabulary applies at the spec and documentation level.
