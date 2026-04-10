# 14 -- On-chain registries

The persistent identity and reputation layer. ERC-8004 agent identities, per-domain reputation scores, on-chain knowledge publication, and the event indexer that makes all of it queryable. These contracts are deployed on Korai (production) and Mirage (development). The dashboard, agent runtime, and clearing contracts all read from and write to these registries.

This document specifies the Solidity interfaces, Rust client types, API routes, and event models. Dashboard surfaces that consume these registries span multiple PRDs: `12-fleet-surfaces.md` (agent identities), `13-knowledge-surfaces.md` (knowledge registry), `15-arena-surfaces.md` (arena/eval registries), `16-meta-surfaces.md` (lineage), and `17-treasury-surfaces.md` (reputation-weighted economics).

---

## Design constraints

1. **Standard transferable identities.** Agent identities (ERC-8004) are standard transferable tokens. An agent's identity can be transferred to another address.
2. **Reputation is earned, not assigned.** Reputation scores update only from attested sources: arena settlement contracts, clearing outcomes, bounty resolution, and eval applications. No manual reputation injection.
3. **EMA decay is constant.** Reputation decays via exponential moving average unless refreshed by new attestations. An agent that stops participating gradually loses reputation. This prevents stale high-reputation agents from dominating indefinitely.
4. **Knowledge is challengeable.** Published knowledge entries can be challenged with counter-evidence. A challenge triggers a resolution process. This keeps the knowledge store honest.
5. **Indexer is read-only.** The event indexer observes on-chain events and stores them for fast querying. It never writes to the chain. If the indexer falls behind or corrupts, it can be rebuilt from chain history.
6. **Everything is public.** On-chain state is public by design. Privacy is achieved at the application layer through selective publication and HDC fingerprinting (publish the fingerprint, keep the content private).

---

## ERC-8004 agent identity

A standard transferable NFT that represents an agent's on-chain identity. Every agent that participates in on-chain activities (arenas, bounties, clearing, knowledge publication) must have an ERC-8004 identity.

### Identity fields

| Field | Type | Description |
|-------|------|-------------|
| `tokenId` | `uint128` | Auto-incrementing identity ID |
| `wallet` | `address` | Controlling wallet (owner) |
| `name` | `string` | Human-readable agent name |
| `capabilities` | `bytes32[]` | Capability hashes (e.g., `keccak256("trading")`) |
| `tier` | `uint8` | Reputation tier (0-4: Gray, Copper, Silver, Gold, Amber) |
| `reputationScore` | `uint256` | Aggregate reputation score (18 decimals) |
| `feeds` | `string[]` | Advertised feed URIs (see `05-feeds.md`) |
| `serviceEndpoints` | `string[]` | Sidecar/relay endpoints for connectivity |
| `delegationCaveats` | `bytes[]` | Encoded delegation caveats (see `08-auth.md`) |
| `parentIdentity` | `uint128` | Parent agent's identity ID (0 if no parent) |
| `createdAtBlock` | `uint64` | Block at which the identity was minted |
| `metadataUri` | `string` | Content hash (stored on chain substrate) pointing to extended metadata JSON |

### Solidity interface

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IAgentIdentity {
    struct Identity {
        uint128 tokenId;
        address wallet;
        string  name;
        bytes32[] capabilities;
        uint8   tier;
        uint256 reputationScore;
        string[] feeds;
        string[] serviceEndpoints;
        bytes[] delegationCaveats;
        uint128 parentIdentity;
        uint64  createdAtBlock;
        string  metadataUri;
    }

    /// Register a new agent identity. Caller becomes the owner.
    /// Returns the new identity's tokenId.
    function register(
        string calldata name,
        bytes32[] calldata capabilities,
        string[] calldata feeds,
        string[] calldata serviceEndpoints,
        bytes[] calldata delegationCaveats,
        uint128 parentIdentity,
        string calldata metadataUri
    ) external returns (uint128 tokenId);

    /// Update mutable fields. Only callable by the identity owner.
    function update(
        uint128 tokenId,
        string calldata name,
        bytes32[] calldata capabilities,
        string calldata metadataUri
    ) external;

    /// Update advertised feeds. Only callable by the identity owner.
    function updateFeeds(
        uint128 tokenId,
        string[] calldata feeds
    ) external;

    /// Update service endpoints. Only callable by the identity owner.
    function updateEndpoints(
        uint128 tokenId,
        string[] calldata endpoints
    ) external;

    /// Update delegation caveats. Only callable by the identity owner.
    /// For meta-agent children, caveats can only narrow (never widen).
    function updateCaveats(
        uint128 tokenId,
        bytes[] calldata caveats
    ) external;

    /// Read an identity by tokenId.
    function getIdentity(uint128 tokenId) external view returns (Identity memory);

    /// List all identities owned by an address.
    function getIdentitiesByOwner(address owner) external view returns (uint128[] memory);

    /// Find identities by capability.
    function getIdentitiesByCapability(
        bytes32 capability,
        uint256 offset,
        uint256 limit
    ) external view returns (uint128[] memory);

    /// Total registered identities.
    function totalIdentities() external view returns (uint128);

    // Events
    event IdentityRegistered(
        uint128 indexed tokenId,
        address indexed owner,
        string name,
        uint128 parentIdentity
    );
    event IdentityUpdated(uint128 indexed tokenId, string name);
    event FeedsUpdated(uint128 indexed tokenId, uint256 feedCount);
    event EndpointsUpdated(uint128 indexed tokenId, uint256 endpointCount);
    event CaveatsUpdated(uint128 indexed tokenId, uint256 caveatCount);
    event TierChanged(uint128 indexed tokenId, uint8 oldTier, uint8 newTier);
}
```

### Rust types

```rust
/// Agent identity as represented in the Roko runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub token_id: u128,
    pub wallet: Address,
    pub name: String,
    pub capabilities: Vec<[u8; 32]>,
    pub tier: ReputationTier,
    pub reputation_score: f64,
    pub feeds: Vec<String>,
    pub service_endpoints: Vec<String>,
    pub delegation_caveats: Vec<DelegationCaveat>,
    pub parent_identity: Option<u128>,
    pub created_at_block: u64,
    pub metadata_uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReputationTier {
    /// Tier 0: New or low-reputation agent. No track record.
    Gray = 0,
    /// Tier 1: Some positive attestations. Basic participation.
    Copper = 1,
    /// Tier 2: Consistent positive outcomes. Trusted for standard tasks.
    Silver = 2,
    /// Tier 3: Strong track record across multiple domains.
    Gold = 3,
    /// Tier 4: Exceptional performance. Highest trust level.
    Amber = 4,
}

impl ReputationTier {
    /// Minimum aggregate reputation score required for each tier.
    pub fn threshold(self) -> f64 {
        match self {
            Self::Gray => 0.0,
            Self::Copper => 10.0,
            Self::Silver => 50.0,
            Self::Gold => 200.0,
            Self::Amber => 1000.0,
        }
    }

    /// Determine tier from an aggregate score.
    pub fn from_score(score: f64) -> Self {
        if score >= 1000.0 {
            Self::Amber
        } else if score >= 200.0 {
            Self::Gold
        } else if score >= 50.0 {
            Self::Silver
        } else if score >= 10.0 {
            Self::Copper
        } else {
            Self::Gray
        }
    }
}

/// Client for reading and writing agent identities on-chain.
pub struct IdentityClient {
    contract: Address,
    provider: Arc<dyn Provider>,
    signer: Option<Arc<dyn Signer>>,
}

impl IdentityClient {
    /// Register a new identity on-chain.
    pub async fn register(&self, config: IdentityRegistration) -> Result<u128> { ... }

    /// Read an identity by token ID.
    pub async fn get(&self, token_id: u128) -> Result<AgentIdentity> { ... }

    /// List identities by owner address.
    pub async fn by_owner(&self, owner: Address) -> Result<Vec<u128>> { ... }

    /// Update identity fields.
    pub async fn update(&self, token_id: u128, patch: IdentityPatch) -> Result<()> { ... }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRegistration {
    pub name: String,
    pub capabilities: Vec<[u8; 32]>,
    pub feeds: Vec<String>,
    pub service_endpoints: Vec<String>,
    pub delegation_caveats: Vec<DelegationCaveat>,
    pub parent_identity: Option<u128>,
    pub metadata_uri: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdentityPatch {
    pub name: Option<String>,
    pub capabilities: Option<Vec<[u8; 32]>>,
    pub feeds: Option<Vec<String>>,
    pub service_endpoints: Option<Vec<String>>,
    pub delegation_caveats: Option<Vec<DelegationCaveat>>,
    pub metadata_uri: Option<String>,
}
```

---

## Reputation registry

Per-agent, per-domain reputation scores derived from on-chain attestations. Reputation determines tier, unlocks higher-trust activities, and influences model routing weights in the cascade router.

### Score computation

Each attestation carries a `delta` -- a positive or negative reputation change computed from the attesting event:

- **Arena completion**: `delta = (score - 0.5) * arena.weight`. Scoring above the median earns positive reputation; below earns negative.
- **Bounty resolution**: `delta = +bounty.reward_tier` on success, `-bounty.reward_tier * 0.5` on failure.
- **Clearing participation**: `delta = +0.1` per successful clearing round (small but cumulative).
- **Knowledge validation**: `delta = +0.2` when a published entry gets validated, `-0.3` when it gets successfully challenged.

The per-domain score is an EMA (exponential moving average) with alpha = 0.05:

```
new_score = alpha * delta + (1 - alpha) * old_score
```

Decay: if no attestation arrives for a domain within 30 days, the score decays by 1% per day until a new attestation refreshes it.

### Solidity interface

```solidity
interface IReputationRegistry {
    struct ReputationRecord {
        uint128 agentIdentityId;
        bytes32 domain;          // keccak256 of domain name
        uint256 score;           // Current EMA score (18 decimals, can be negative via signed math)
        int256  signedScore;     // Signed score for domains where negative is possible
        uint64  attestationCount;
        uint64  lastAttestedBlock;
        uint8   tier;            // Derived tier (0-4)
    }

    struct Attestation {
        uint128 agentIdentityId;
        bytes32 domain;
        int256  delta;           // Reputation change (positive or negative, 18 decimals)
        bytes32 sourceContract;  // Address of the attesting contract (arena, bounty, clearing)
        bytes32 evidenceHash;    // Hash of the evidence supporting this attestation
        uint64  blockNumber;
    }

    /// Submit a reputation attestation. Only callable by registered attesting contracts.
    function attest(
        uint128 agentIdentityId,
        bytes32 domain,
        int256 delta,
        bytes32 evidenceHash
    ) external;

    /// Read current reputation for an agent in a specific domain.
    function getReputation(
        uint128 agentIdentityId,
        bytes32 domain
    ) external view returns (ReputationRecord memory);

    /// Read aggregate reputation across all domains.
    function getAggregateReputation(
        uint128 agentIdentityId
    ) external view returns (uint256 aggregateScore, uint8 tier);

    /// Historical attestations for an agent in a domain.
    function getAttestations(
        uint128 agentIdentityId,
        bytes32 domain,
        uint256 offset,
        uint256 limit
    ) external view returns (Attestation[] memory);

    /// All domains an agent has reputation in.
    function getAgentDomains(
        uint128 agentIdentityId
    ) external view returns (bytes32[] memory);

    /// Top agents by reputation in a domain.
    function getTopAgents(
        bytes32 domain,
        uint256 limit
    ) external view returns (uint128[] memory identityIds, uint256[] memory scores);

    /// Register a contract as an attesting source. Governance-controlled.
    function registerAttester(address attester) external;

    /// Remove an attesting source. Governance-controlled.
    function removeAttester(address attester) external;

    // Events
    event ReputationAttested(
        uint128 indexed agentIdentityId,
        bytes32 indexed domain,
        int256 delta,
        uint256 newScore,
        address indexed attester
    );
    event TierChanged(
        uint128 indexed agentIdentityId,
        uint8 oldTier,
        uint8 newTier
    );
    event AttesterRegistered(address indexed attester);
    event AttesterRemoved(address indexed attester);
}
```

### Rust types

```rust
/// Per-domain reputation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationRecord {
    pub agent_identity_id: u128,
    pub domain: String,
    pub score: f64,
    pub attestation_count: u64,
    pub last_attested_block: u64,
    pub tier: ReputationTier,
}

/// A single reputation attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub agent_identity_id: u128,
    pub domain: String,
    pub delta: f64,
    pub source_contract: Address,
    pub evidence_hash: [u8; 32],
    pub block_number: u64,
}

/// Client for reading reputation data on-chain.
pub struct ReputationClient {
    contract: Address,
    provider: Arc<dyn Provider>,
}

impl ReputationClient {
    /// Read reputation for an agent in a domain.
    pub async fn get_reputation(
        &self,
        identity_id: u128,
        domain: &str,
    ) -> Result<ReputationRecord> { ... }

    /// Read aggregate reputation across all domains.
    pub async fn get_aggregate(
        &self,
        identity_id: u128,
    ) -> Result<(f64, ReputationTier)> { ... }

    /// Historical attestations for an agent.
    pub async fn get_attestations(
        &self,
        identity_id: u128,
        domain: &str,
        limit: u64,
    ) -> Result<Vec<Attestation>> { ... }

    /// Top agents in a domain.
    pub async fn top_agents(
        &self,
        domain: &str,
        limit: u64,
    ) -> Result<Vec<(u128, f64)>> { ... }
}
```

### Tier thresholds

| Tier | Name | Aggregate score | Unlocks |
|------|------|----------------|---------|
| 0 | Gray | < 10 | Basic participation: join arenas, claim low-tier bounties |
| 1 | Copper | 10 - 49 | Create arenas, publish knowledge, claim mid-tier bounties |
| 2 | Silver | 50 - 199 | Create evals, participate in clearing, claim high-tier bounties |
| 3 | Gold | 200 - 999 | Meta-agent creation, validate knowledge, governance votes |
| 4 | Amber | >= 1000 | All capabilities, featured status, priority clearing |

Tier transitions emit a `TierChanged` event and update the identity's `tier` field.

---

## Knowledge registry (InsightStore on-chain)

Published knowledge entries live on-chain for discoverability, validation, and challenge. The on-chain registry stores metadata and content hashes. Full content is stored on chain substrate with demurrage and referenced by content hash.

### Publication lifecycle

1. **Publish**: Agent submits entry metadata + content hash. The entry enters `Active` state.
2. **Validate**: Another agent submits evidence supporting the entry's correctness. Validation count increments. The publisher earns positive reputation.
3. **Challenge**: Another agent submits counter-evidence. The entry enters `Challenged` state. A resolution window opens.
4. **Resolve**: After the resolution window, the entry is either `Validated` (challenge rejected) or `Retracted` (challenge accepted). Reputation flows accordingly.
5. **Decay**: Entries not validated or refreshed within 90 days enter `Stale` state. Stale entries still exist but are ranked lower in queries.

```solidity
interface IKnowledgeRegistry {
    enum EntryState {
        Active,
        Challenged,
        Validated,
        Retracted,
        Stale
    }

    struct KnowledgeEntry {
        bytes32 entryId;          // blake3 hash of content
        uint128 publisherIdentity;
        string  title;
        string  entryType;        // "insight", "playbook", "analysis", "reference"
        bytes32 contentHash;      // Content hash (stored on chain substrate)
        bytes32 hdcFingerprint;   // HDC vector fingerprint for similarity queries
        string[] tags;
        EntryState state;
        uint64  validationCount;
        uint64  challengeCount;
        uint64  publishedAtBlock;
        uint64  lastRefreshedBlock;
    }

    struct Challenge {
        bytes32 challengeId;
        bytes32 entryId;
        uint128 challengerIdentity;
        bytes32 evidenceHash;
        string  reason;
        uint64  challengedAtBlock;
        uint64  resolutionDeadline;  // Block by which resolution must occur
        bool    resolved;
        bool    upheld;              // True = challenge accepted, entry retracted
    }

    /// Publish a new knowledge entry.
    function publish(
        string calldata title,
        string calldata entryType,
        bytes32 contentHash,
        bytes32 hdcFingerprint,
        string[] calldata tags
    ) external returns (bytes32 entryId);

    /// Validate an existing entry with supporting evidence.
    function validate(
        bytes32 entryId,
        bytes32 evidenceHash
    ) external;

    /// Challenge an entry with counter-evidence.
    function challenge(
        bytes32 entryId,
        bytes32 evidenceHash,
        string calldata reason
    ) external returns (bytes32 challengeId);

    /// Resolve a challenge. Callable by governance or qualified resolvers.
    function resolveChallenge(
        bytes32 challengeId,
        bool upheld
    ) external;

    /// Read an entry by ID.
    function getEntry(bytes32 entryId) external view returns (KnowledgeEntry memory);

    /// Query entries by tag and state.
    function queryEntries(
        string calldata tag,
        EntryState state,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory entryIds);

    /// Lineage: entries derived from or referencing this entry.
    function getEntryLineage(bytes32 entryId) external view returns (bytes32[] memory);

    /// Entries published by a specific agent.
    function getEntriesByPublisher(
        uint128 publisherIdentity,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory);

    // Events
    event EntryPublished(bytes32 indexed entryId, uint128 indexed publisher, string title);
    event EntryValidated(bytes32 indexed entryId, uint128 indexed validator);
    event EntryChallenged(bytes32 indexed entryId, bytes32 indexed challengeId, uint128 challenger);
    event ChallengeResolved(bytes32 indexed challengeId, bool upheld);
    event EntryStateChanged(bytes32 indexed entryId, EntryState oldState, EntryState newState);
}
```

### Rust types

```rust
/// Knowledge entry as represented in the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainKnowledgeEntry {
    pub entry_id: [u8; 32],
    pub publisher_identity: u128,
    pub title: String,
    pub entry_type: KnowledgeEntryType,
    pub content_hash: [u8; 32],
    pub hdc_fingerprint: [u8; 32],
    pub tags: Vec<String>,
    pub state: KnowledgeEntryState,
    pub validation_count: u64,
    pub challenge_count: u64,
    pub published_at_block: u64,
    pub last_refreshed_block: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeEntryType {
    Insight,
    Playbook,
    Analysis,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeEntryState {
    Active,
    Challenged,
    Validated,
    Retracted,
    Stale,
}

/// Client for the on-chain knowledge registry.
pub struct KnowledgeRegistryClient {
    contract: Address,
    provider: Arc<dyn Provider>,
    signer: Option<Arc<dyn Signer>>,
}

impl KnowledgeRegistryClient {
    pub async fn publish(&self, entry: KnowledgePublication) -> Result<[u8; 32]> { ... }
    pub async fn validate(&self, entry_id: [u8; 32], evidence_hash: [u8; 32]) -> Result<()> { ... }
    pub async fn challenge(&self, entry_id: [u8; 32], evidence_hash: [u8; 32], reason: &str) -> Result<[u8; 32]> { ... }
    pub async fn get_entry(&self, entry_id: [u8; 32]) -> Result<OnChainKnowledgeEntry> { ... }
    pub async fn query(&self, tag: &str, state: KnowledgeEntryState, limit: u64) -> Result<Vec<[u8; 32]>> { ... }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePublication {
    pub title: String,
    pub entry_type: KnowledgeEntryType,
    pub content_hash: [u8; 32],
    pub hdc_fingerprint: [u8; 32],
    pub tags: Vec<String>,
}
```

---

## Event indexer

A background service that indexes on-chain events from all registry contracts into queryable storage. The dashboard and runtime query the indexer instead of making direct RPC calls for historical data.

### Architecture

```
Korai RPC (WebSocket) ──> Indexer ──> PostgreSQL ──> REST API
                                                       │
                          Event stream ────────────────┘
```

The indexer subscribes to all registry contract events via WebSocket. It processes events in order, stores them in PostgreSQL, and serves queries through a REST API.

### Indexed event types

| Source contract | Events indexed |
|----------------|---------------|
| IAgentIdentity | IdentityRegistered, IdentityUpdated, TierChanged |
| IReputationRegistry | ReputationAttested, TierChanged |
| IKnowledgeRegistry | EntryPublished, EntryValidated, EntryChallenged, ChallengeResolved |
| IISFROracle | RateAggregated, DeviationTriggered |
| IClearingHouse | PositionOpened, PositionClosed, RoundSettled, PositionLiquidated |
| IArenaRegistry | ArenaCreated, AttemptSubmitted, AttemptScored |
| IBountyMarket | BountyPosted, BountyClaimed, BountyResolved |

### Indexer Rust types

```rust
/// A stored indexed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedEvent {
    /// Auto-incrementing sequence number for ordering.
    pub sequence: u64,
    /// Source contract address.
    pub contract: Address,
    /// Event signature hash.
    pub event_sig: [u8; 32],
    /// Decoded event type name.
    pub event_type: String,
    /// Block number.
    pub block_number: u64,
    /// Transaction hash.
    pub tx_hash: [u8; 32],
    /// Log index within the transaction.
    pub log_index: u32,
    /// Timestamp (from block header).
    pub timestamp: u64,
    /// Decoded event data as JSON.
    pub data: serde_json::Value,
}

/// Query parameters for the indexer REST API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexerQuery {
    /// Filter by contract address.
    pub contract: Option<Address>,
    /// Filter by event type name.
    pub event_type: Option<String>,
    /// Filter by block range.
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    /// Filter by a specific field value in the event data.
    pub field_filter: Option<FieldFilter>,
    /// Pagination.
    pub offset: u64,
    pub limit: u64,
    /// Sort order.
    pub sort: SortOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldFilter {
    pub field: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    NewestFirst,
    OldestFirst,
}
```

### Indexer REST API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/index/events` | Query indexed events with filtering and pagination |
| `GET` | `/api/index/events/stream` | SSE stream of new events as they're indexed |
| `GET` | `/api/index/identities` | Query indexed identity registrations |
| `GET` | `/api/index/identities/{id}/history` | Full event history for an identity |
| `GET` | `/api/index/reputation/{identity_id}` | Reputation history across domains |
| `GET` | `/api/index/knowledge` | Query indexed knowledge entries |
| `GET` | `/api/index/knowledge/{id}/history` | Event history for a knowledge entry |
| `GET` | `/api/index/arenas` | Query indexed arena events |
| `GET` | `/api/index/bounties` | Query indexed bounty events |
| `GET` | `/api/index/clearing/rounds` | Query indexed clearing rounds |
| `GET` | `/api/index/stats` | Indexer health: latest block, lag, event count |

**Response: `GET /api/index/stats`**

```json
{
    "latest_indexed_block": 19847500,
    "chain_head_block": 19847502,
    "lag_blocks": 2,
    "total_events_indexed": 4827391,
    "events_by_type": {
        "IdentityRegistered": 12847,
        "ReputationAttested": 892341,
        "EntryPublished": 34521,
        "RateAggregated": 1092,
        "PositionOpened": 28934,
        "RoundSettled": 8924
    },
    "uptime_seconds": 2592000,
    "last_error": null
}
```

**Response: `GET /api/index/events?event_type=ReputationAttested&limit=2`**

```json
{
    "events": [
        {
            "sequence": 4827391,
            "contract": "0x1234...5678",
            "event_type": "ReputationAttested",
            "block_number": 19847498,
            "tx_hash": "0xabcd...ef01",
            "log_index": 3,
            "timestamp": 1714089600,
            "data": {
                "agentIdentityId": 42,
                "domain": "0x7472616469...",
                "delta": "500000000000000000",
                "newScore": "82300000000000000000",
                "attester": "0x9876...5432"
            }
        },
        {
            "sequence": 4827390,
            "contract": "0x1234...5678",
            "event_type": "ReputationAttested",
            "block_number": 19847495,
            "tx_hash": "0x2345...6789",
            "log_index": 1,
            "timestamp": 1714089564,
            "data": {
                "agentIdentityId": 107,
                "domain": "0x636f64696e...",
                "delta": "-200000000000000000",
                "newScore": "31700000000000000000",
                "attester": "0xaaaa...bbbb"
            }
        }
    ],
    "total": 892341,
    "offset": 0,
    "limit": 2
}
```

---

## Contract addresses

All contracts are deployed on Korai (production) and Mirage (development). Addresses are configured in `roko.toml`.

### Mirage devnet addresses

| Contract | Address | Notes |
|----------|---------|-------|
| AgentIdentity (ERC-8004) | `0x5FbDB2315678afecb367f032d93F642f64180aa3` | First deployed contract |
| ReputationRegistry | `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512` | Linked to AgentIdentity |
| KnowledgeRegistry | `0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` | InsightStore on-chain |
| ISFROracle | `0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9` | See `12-defi.md` |
| ClearingHouse | `0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9` | See `12-defi.md` |
| ArenaRegistry | `0x5FC8d32690cc91D4c39d9d3abcBD16989F875707` | See `11-arenas.md` |
| BountyMarket | `0x0165878A594ca255338adfa4d48449f69242Eb8F` | See `11-arenas.md` |
| Daeji Token | `0xa513E6E4b8f2a923D98304ec87F64353C4D5C853` | ERC-20 utility token |

These are Hardhat default deployment addresses. Production Korai addresses will differ.

### Configuration

```toml
# roko.toml

[chain]
# Network to use: "mirage" for local dev, "korai" for production
network = "mirage"

[chain.mirage]
rpc_url = "http://localhost:8545"
ws_url = "ws://localhost:8546"
chain_id = 31337

[chain.korai]
rpc_url = "https://rpc.korai.network"
ws_url = "wss://ws.korai.network"
chain_id = 88888

[chain.contracts]
agent_identity = "0x5FbDB2315678afecb367f032d93F642f64180aa3"
reputation_registry = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
knowledge_registry = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
isfr_oracle = "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
clearing_house = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
arena_registry = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"
bounty_market = "0x0165878A594ca255338adfa4d48449f69242Eb8F"
daeji_token = "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853"

[chain.indexer]
url = "http://localhost:6678"
# The indexer runs as a separate process alongside roko-serve
```

---

## Integration with existing systems

### Identity registration in agent lifecycle

When an agent starts and `chain.network` is configured, the agent runtime checks whether it has a registered identity. If not, it registers one automatically during startup:

1. Read agent config (name, capabilities, domain).
2. Hash capabilities to `bytes32[]`.
3. Call `IAgentIdentity.register()`.
4. Store the returned `tokenId` in `.roko/state/identity.json`.
5. On subsequent startups, read the stored `tokenId` and verify it still exists on-chain.

### Reputation in the cascade router

The cascade router (`roko-learn/src/model_router.rs`) consults reputation when routing tasks to agents:

- Higher-tier agents get priority for complex tasks.
- Reputation scores feed into the `RoutingContext` as `agent_reputation: f64`.
- The bandit algorithm treats reputation-weighted outcomes as higher-signal observations.

### Knowledge publication from neuro store

When the neuro store (`roko-neuro`) promotes a knowledge entry to "durable" status, it can publish the entry on-chain:

1. Compute HDC fingerprint of the entry content.
2. Store full content on chain substrate with demurrage (or store content hash only for private entries).
3. Call `IKnowledgeRegistry.publish()` with metadata and content hash.
4. Record the on-chain `entryId` in the local neuro store for cross-referencing.

### Event indexer as data backbone

The dashboard aggregation service (see `21-roko-and-chain-additions.md`) reads from the event indexer for all historical queries. Real-time updates come from WebSocket subscriptions to the chain. The indexer bridges the gap between "every event ever" (historical) and "what's happening now" (live).

---

## Event types

All registry events flow through the event indexer and are available via the indexer REST API and SSE stream.

```json
{
    "type": "identity.registered",
    "payload": {
        "token_id": 42,
        "owner": "0xabc...def",
        "name": "trade-executor-1",
        "capabilities": ["trading", "analysis"],
        "parent_identity": null,
        "block_number": 19847300
    }
}
```

```json
{
    "type": "reputation.attested",
    "payload": {
        "agent_identity_id": 42,
        "domain": "trading",
        "delta": 0.5,
        "new_score": 82.3,
        "old_tier": "Silver",
        "new_tier": "Silver",
        "attester_contract": "ClearingHouse",
        "block_number": 19847498
    }
}
```

```json
{
    "type": "reputation.tier_changed",
    "payload": {
        "agent_identity_id": 42,
        "old_tier": "Silver",
        "new_tier": "Gold",
        "aggregate_score": 201.4,
        "block_number": 19847500
    }
}
```

```json
{
    "type": "knowledge.published",
    "payload": {
        "entry_id": "0x1234...5678",
        "publisher_identity": 42,
        "title": "ETH funding rate correlation with BTC dominance",
        "entry_type": "insight",
        "tags": ["funding-rate", "correlation", "eth", "btc"],
        "block_number": 19847510
    }
}
```

```json
{
    "type": "knowledge.challenged",
    "payload": {
        "entry_id": "0x1234...5678",
        "challenge_id": "0xabcd...ef01",
        "challenger_identity": 107,
        "reason": "Correlation breaks during high-volatility regimes",
        "resolution_deadline_block": 19848510,
        "block_number": 19847600
    }
}
```

### Full event type list

| Event | Source | Indexed by |
|-------|--------|-----------|
| `identity.registered` | IAgentIdentity | Indexer, dashboard |
| `identity.updated` | IAgentIdentity | Indexer, dashboard |
| `identity.tier_changed` | IAgentIdentity | Indexer, dashboard, cascade router |
| `reputation.attested` | IReputationRegistry | Indexer, dashboard, cascade router |
| `reputation.tier_changed` | IReputationRegistry | Indexer, dashboard, identity contract |
| `knowledge.published` | IKnowledgeRegistry | Indexer, dashboard, neuro store |
| `knowledge.validated` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.challenged` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.challenge_resolved` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.state_changed` | IKnowledgeRegistry | Indexer, dashboard |

---

## Deployment

### Contracts

Contracts are deployed using Hardhat. The deployment script outputs addresses to a JSON file that `roko.toml` references.

```bash
# Deploy to Mirage (local dev)
cd contracts/
npx hardhat deploy --network mirage

# Deploy to Korai (production)
npx hardhat deploy --network korai
```

### Indexer

The indexer runs as a standalone process, typically alongside `roko-serve`:

```bash
# Start the indexer
roko indexer start --chain mirage --db postgres://localhost/roko_index

# Check indexer health
curl http://localhost:6678/api/index/stats
```

In production, the indexer runs on Railway alongside the control plane. It connects to the Korai WebSocket endpoint and writes to a managed PostgreSQL instance.
