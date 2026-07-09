# 18 — On-Chain Registries

> Persistent identity, reputation, and knowledge publication on-chain. ERC-8004 agent identities with ZK-HDC fingerprints, per-domain reputation with EMA decay, knowledge Signal publication with demurrage and challenge mechanics, stigmergic pheromone coordination, x402 payment integration, and HDC similarity via HTC precompile.

**Subsumes**: Agent identity, reputation system, knowledge publication, pheromone coordination, payment integration.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality, demurrage model, HDC fingerprints), [02-CELL](02-CELL.md) (Verify protocol, predict-publish-correct), [07-AGENT-RUNTIME](07-AGENT-RUNTIME.md) (vitality, budget), [11-MEMORY-AND-KNOWLEDGE](11-MEMORY-AND-KNOWLEDGE.md) (demurrage, tier progression, knowledge lifecycle), [12-CONNECTIVITY](12-CONNECTIVITY.md) (exoskeleton protocols: MCP + A2A + ERC-8004 + x402), [17-SECURITY-MODEL](17-SECURITY-MODEL.md) (capability model, CaMeL IFC)

---

## 1. Design Constraints

1. **Standard ERC-8004 identities.** Agent identities (ERC-8004) are standard transferable NFTs. An Agent's identity is anchored to its registration wallet. The identity record can be updated; ownership follows standard ERC-721 transfer rules.
2. **Reputation is earned, not assigned.** Reputation scores update only from attested sources: arena settlement contracts, bounty resolution, and eval applications. No manual reputation injection.
3. **EMA decay is constant.** Reputation decays via exponential moving average unless refreshed by new attestations. An Agent that stops participating gradually loses reputation. Half-life configurable per domain (default: 30 days).
4. **Knowledge is challengeable.** Published knowledge Signals can be challenged by any participant. Challenged Signals are flagged and may be retracted. Stakes incentivize honest challenge and defense.
5. **Read-only indexer.** The Rust indexer is read-only -- it indexes on-chain events into PostgreSQL and serves a REST API. It never writes to chain. All chain writes go through the Solidity contracts.

---

## 2. ERC-8004 Agent Identity

Every Agent that participates in the economy gets an ERC-8004 identity NFT. The identity is the root of capability attestation and reputation.

### 2.1 Identity fields

```rust
pub struct AgentIdentity {
    /// On-chain token ID (ERC-721).
    pub token_id: U256,
    /// Wallet address that currently owns this identity.
    pub wallet: Address,
    /// Human-readable name.
    pub name: String,
    /// Declared capability set (what this Agent can do).
    pub capabilities: Vec<String>,
    /// Current reputation tier.
    pub tier: ReputationTier,
    /// Per-domain reputation scores.
    pub reputation: HashMap<String, f64>,
    /// Signal stream topics this Agent publishes.
    pub feeds: Vec<String>,
    /// Network endpoints for direct communication.
    pub endpoints: Vec<String>,
    /// Delegation caveats granted to this Agent (see doc-17 section 7).
    pub delegation_caveats: Vec<DelegationCaveat>,
    /// HDC fingerprint of the Agent's capability profile.
    pub hdc_fingerprint: HdcFingerprint,
    /// ZK attestation hash proving capability without revealing full fingerprint.
    pub zk_attestation_hash: H256,
}

pub enum ReputationTier {
    Gray,     // < 10 attestations
    Copper,   // < 50 attestations
    Silver,   // < 200 attestations
    Gold,     // < 1000 attestations
    Amber,    // >= 1000 attestations
}
```

### 2.2 Solidity interface

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IAgentIdentity {
    struct IdentityData {
        uint256 tokenId;
        address wallet;
        string name;
        string[] capabilities;
        uint8 tier;               // 0=Gray, 1=Copper, 2=Silver, 3=Gold, 4=Amber
        string[] feeds;
        string[] endpoints;
        bytes32 hdcFingerprint;   // 256-bit on-chain projection of 10,240-bit vector
        bytes32 zkAttestationHash;
    }

    event IdentityRegistered(uint256 indexed tokenId, address indexed wallet, string name);
    event IdentityUpdated(uint256 indexed tokenId, string field, bytes newValue);
    event TierChanged(uint256 indexed tokenId, uint8 oldTier, uint8 newTier);
    event CapabilityAttested(uint256 indexed tokenId, string capability, address attester);
    event DelegationGranted(uint256 indexed fromToken, uint256 indexed toToken, bytes32 caveatsHash);

    /// Register a new agent identity. msg.sender becomes initial owner.
    function register(string calldata name, string[] calldata capabilities) external returns (uint256 tokenId);

    /// Update identity fields (owner only).
    function updateName(uint256 tokenId, string calldata name) external;
    function updateCapabilities(uint256 tokenId, string[] calldata capabilities) external;
    function updateEndpoints(uint256 tokenId, string[] calldata endpoints) external;
    function updateFeeds(uint256 tokenId, string[] calldata feeds) external;
    function updateHdcFingerprint(uint256 tokenId, bytes32 fingerprint) external;
    function updateZkAttestation(uint256 tokenId, bytes32 attestationHash) external;

    /// Query.
    function getIdentity(uint256 tokenId) external view returns (IdentityData memory);
    function getIdentityByWallet(address wallet) external view returns (IdentityData memory);
    function totalIdentities() external view returns (uint256);
}
```

### 2.3 Registration lifecycle

```
1. Agent starts for the first time
2. Agent generates wallet (if none exists)
3. Agent calls IAgentIdentity.register(name, capabilities)
4. Contract mints ERC-8004 NFT, assigns tokenId
5. Agent computes HDC fingerprint of capability profile
6. Agent generates ZK proof of fingerprint (see section 3)
7. Agent calls updateHdcFingerprint + updateZkAttestation
8. Agent publishes A2A agent card with identity reference (see section 4)
```

On subsequent starts, the Agent reads its existing identity record and updates fields that have changed.

---

## 3. ZK-HDC Identity Proofs

### 3.1 The problem

An Agent's 10,240-bit HDC fingerprint (Kanerva 2009) encodes its full capability profile. Publishing the raw fingerprint enables similarity search -- but also enables reconstruction of the Agent's internal knowledge structure. Privacy-Preserving HDC (PP-HDC) makes the fingerprint non-invertible while preserving similarity search.

### 3.2 PP-HDC pipeline

```
10,240-bit local fingerprint (full resolution)
    |
    v
ZK-SNARK proof: "my fingerprint is within Hamming distance D of reference vector R"
    |
    v
256-bit on-chain projection (dimensionality reduction via random hyperplane projection)
    |
    v
On-chain storage: 32 bytes per Agent
```

The ZK proof demonstrates that the Agent's local fingerprint satisfies a capability predicate without revealing the full vector. The 256-bit projection supports coarse similarity search on-chain (suitable for routing), while the full 10,240-bit vector remains local for precise similarity computation.

### 3.3 Capability similarity proof

An Agent can prove capability similarity to a reference profile without revealing its full fingerprint:

```rust
pub struct CapabilitySimilarityProof {
    /// 256-bit on-chain projection (published).
    pub projection: H256,
    /// ZK proof that the local 10,240-bit fingerprint:
    /// 1. Reduces to this projection via the agreed random matrix
    /// 2. Has Hamming distance <= threshold from the reference vector
    pub proof: ZkProof,
    /// Reference vector being compared against.
    pub reference: H256,
    /// Maximum Hamming distance claimed.
    pub max_distance: u32,
}
```

This enables "find agents similar to X" queries on-chain without any agent revealing its full capability set. The HTC precompile (section 9) provides efficient on-chain Hamming distance computation for the 256-bit projections.

---

## 4. A2A Integration

Every Agent publishes an A2A-compliant agent card at `/.well-known/agent-card.json`. The card carries the ERC-8004 identity reference and HDC fingerprint, extending the A2A standard with Roko-specific fields.

```json
{
  "name": "coder-1",
  "description": "Coding agent specialized in Rust",
  "url": "https://my-roko.up.railway.app",
  "capabilities": {
    "streaming": true,
    "pushNotifications": true
  },
  "authentication": {
    "schemes": ["bearer"]
  },
  "skills": [
    {
      "id": "code-review",
      "name": "Code Review",
      "description": "Reviews pull requests for bugs, style, and security"
    }
  ],
  "x-roko": {
    "identityId": 42,
    "identityContract": "0x...",
    "hdcFingerprint": "0x...",
    "zkAttestationHash": "0x...",
    "tier": "Silver",
    "feeds": ["agent:coder-1:output", "agent:coder-1:heartbeat"],
    "reputation": {
      "coding": 0.87,
      "security-audit": 0.42
    }
  }
}
```

Other agents (including non-Roko agents implementing A2A) can discover and verify capabilities by:
1. Fetching `/.well-known/agent-card.json`
2. Reading the `x-roko.identityId` field
3. Verifying the ERC-8004 identity on-chain via `IAgentIdentity.getIdentity(id)`
4. Computing HDC similarity between their own fingerprint and the agent card's fingerprint
5. Optionally verifying the ZK attestation

---

## 5. Reputation Registry

Per-domain reputation scores derived exclusively from attested work. Reputation is not declared; it is earned through arena completion, bounty settlement, eval applications, and peer attestation. Scores decay via EMA unless refreshed.

### 5.1 EMA decay model

```
reputation(t+1) = alpha * new_attestation + (1 - alpha) * reputation(t)
```

Where `alpha` controls how quickly new evidence displaces old (default: 0.1, giving a ~10-attestation half-life). An Agent that stops participating sees its reputation decay toward zero.

### 5.2 Tier thresholds

| Tier | Min attestations | Typical agent |
|---|---|---|
| **Gray** | 0-9 | New, unproven |
| **Copper** | 10-49 | Establishing track record |
| **Silver** | 50-199 | Reliable in specific domains |
| **Gold** | 200-999 | Broadly trusted |
| **Amber** | 1000+ | Ecosystem pillar |

Tier thresholds are based on total attestation count, not score. A high score with few attestations stays in a low tier -- consistency matters more than peak performance.

### 5.3 Solidity interface

```solidity
interface IReputationRegistry {
    struct ReputationScore {
        uint256 agentIdentityId;
        string domain;
        int64 score;             // Fixed-point: score * 1e18, range [-1e18, 1e18]
        uint64 attestationCount;
        uint64 lastUpdatedBlock;
        int64 emaAlpha;          // Fixed-point: alpha * 1e18 (default: 0.1e18)
    }

    struct Attestation {
        uint256 agentIdentityId;
        string domain;
        int64 delta;             // Fixed-point: delta * 1e18
        address attester;        // must be a registered arena/bounty/eval contract
        bytes32 evidenceHash;    // hash of off-chain evidence
        uint64 attestedBlock;
    }

    event ReputationUpdated(
        uint256 indexed agentIdentityId,
        string domain,
        int64 oldScore,
        int64 newScore,
        uint64 attestationCount
    );
    event AttestationRecorded(
        uint256 indexed agentIdentityId,
        string domain,
        int64 delta,
        address indexed attester
    );

    /// Record a reputation attestation (only callable by registered attester contracts).
    function attest(Attestation calldata attestation) external;

    /// Query reputation.
    function getReputation(uint256 agentIdentityId, string calldata domain)
        external view returns (ReputationScore memory);
    function getAllReputations(uint256 agentIdentityId)
        external view returns (ReputationScore[] memory);

    /// Register an attester contract (governance only).
    function registerAttester(address attester, string calldata domain) external;

    /// Compute decayed reputation (view, applies EMA decay for elapsed time).
    function computeDecayedReputation(uint256 agentIdentityId, string calldata domain)
        external view returns (int64);
}
```

### 5.4 Attester registration

Only registered attester contracts can submit reputation attestations. This prevents gaming -- an Agent cannot attest to its own reputation. Registered attesters include:
- `ArenaRegistry` contract (see [doc-19](19-ARENAS-EVALS-BOUNTIES.md))
- `BountyMarket` contract (see [doc-19](19-ARENAS-EVALS-BOUNTIES.md))
- `EvalRegistry` contract (see [doc-19](19-ARENAS-EVALS-BOUNTIES.md))
- Governance-approved third-party validators

---

## 6. InsightStore

Knowledge Signal publication with demurrage (not Ebbinghaus) and challenge mechanics. The InsightStore brings the demurrage model ([doc-01](01-SIGNAL.md) section 6) on-chain: published knowledge decays unless actively used and reinforced, and any participant can challenge published knowledge.

### 6.1 State machine

```
                   challenge()
    Active ----------------------> Challenged
      |                              |
      | demurrage decay              | validate()    retract()
      v                              v                  v
    Stale <--- balance < threshold  Validated        Retracted
```

| State | Description |
|---|---|
| `Active` | Published and in good standing. Balance decays via demurrage. |
| `Challenged` | A participant has staked to challenge this knowledge. Under review. |
| `Validated` | Challenge resolved in favor of the publisher. Balance boosted. |
| `Retracted` | Challenge resolved against the publisher, or publisher voluntarily retracted. |
| `Stale` | Balance dropped below the cold threshold (default 0.01). Archived. |

### 6.2 Demurrage on-chain

Published knowledge Signals carry an on-chain `balance` that decays per block:

```
balance(block+N) = balance(block) - r*N - beta*balance(block)*N
```

Consistent with the off-chain demurrage model ([doc-01](01-SIGNAL.md) section 6). Reinforcement events (citation by other published Signals, gate-pass in verified arena attempts) restore balance. The rates `r` and `beta` are per-Kind defaults matching the off-chain table.

### 6.3 Challenge mechanics

Any participant can challenge a published Signal by staking tokens:

```solidity
function challenge(bytes32 insightId, string calldata reason) external payable;
```

The challenge opens a review period. The publisher can defend by providing evidence. Resolution follows the same 4-level dispute escalation as bounties (see [doc-19](19-ARENAS-EVALS-BOUNTIES.md) section 11.4):

1. **Bond escalation**: challenger and publisher alternate increasing stakes.
2. **Peer jury**: 5 randomly selected domain experts vote.
3. **Governance**: full token holder vote.
4. **External arbitration**: reserved for real-world obligations.

If the challenge succeeds, the publisher loses their stake and the Signal is retracted. If the challenge fails, the challenger loses their stake and the Signal's balance is boosted.

### 6.4 Solidity interface

```solidity
interface IInsightStore {
    enum InsightState { Active, Challenged, Validated, Retracted, Stale }

    struct InsightRecord {
        bytes32 id;
        uint256 publisherIdentityId;
        bytes32 contentHash;          // SHA-256 of off-chain Signal payload
        bytes32 hdcFingerprint;       // 256-bit projection for similarity search
        string domain;
        uint64 publishedBlock;
        int64 balance;                // Fixed-point: balance * 1e18
        int64 demurrageRate;          // Fixed-point: r * 1e18
        int64 demurrageBeta;          // Fixed-point: beta * 1e18
        InsightState state;
        uint64 citationCount;
        uint64 lastReinforcedBlock;
    }

    event InsightPublished(bytes32 indexed insightId, uint256 indexed publisher, string domain);
    event InsightChallenged(bytes32 indexed insightId, uint256 indexed challenger, string reason);
    event InsightValidated(bytes32 indexed insightId, uint256 indexed defender);
    event InsightRetracted(bytes32 indexed insightId, uint256 indexed publisher);
    event InsightStale(bytes32 indexed insightId, int64 finalBalance);
    event InsightReinforced(bytes32 indexed insightId, int64 newBalance, string reinforceKind);

    function publish(
        bytes32 contentHash,
        bytes32 hdcFingerprint,
        string calldata domain,
        int64 initialBalance
    ) external returns (bytes32 insightId);

    function challenge(bytes32 insightId, string calldata reason) external payable;
    function defend(bytes32 insightId, bytes32 evidenceHash) external;
    function resolveChallenge(bytes32 insightId, bool challengerWins) external;
    function retract(bytes32 insightId) external;
    function reinforce(bytes32 insightId, string calldata kind) external;

    function getInsight(bytes32 insightId) external view returns (InsightRecord memory);
    function queryByDomain(string calldata domain, uint64 limit, uint64 offset)
        external view returns (InsightRecord[] memory);
    function queryBySimilarity(bytes32 hdcFingerprint, uint32 maxDistance, uint64 limit)
        external view returns (InsightRecord[] memory);
    function computeBalance(bytes32 insightId) external view returns (int64);
}
```

---

## 7. PheromoneRegistry

Coordination Signals with on-chain decay. Pheromones are the stigmergic coordination mechanism (Dorigo 1992) -- agents leave traces that influence other agents' behavior without direct communication.

Unlike knowledge Signals, pheromones are intentionally short-lived. They encode spatial/contextual information ("I am working here," "this area is dangerous," "high reward found nearby") that is useful only while fresh.

### 7.1 Pheromone types

```rust
pub enum PheromoneType {
    /// "I am working on this" -- prevents duplicate effort.
    Claim,
    /// "This area is valuable" -- attracts other agents.
    Attraction,
    /// "This area is dangerous/failed" -- repels other agents.
    Repulsion,
    /// "Progress marker" -- breadcrumb trail.
    Trail,
    /// Custom coordination signal.
    Custom(String),
}
```

### 7.2 Solidity interface

```solidity
interface IPheromoneRegistry {
    struct Pheromone {
        bytes32 id;
        uint256 depositorIdentityId;
        bytes32 locationHash;         // HDC hash of the context/location
        uint8 ptype;                  // 0=Claim, 1=Attraction, 2=Repulsion, 3=Trail, 4=Custom
        int64 intensity;              // Fixed-point: intensity * 1e18, decays per block
        int64 decayRate;              // Fixed-point: decay per block * 1e18
        uint64 depositedBlock;
        bytes payload;                // arbitrary data
    }

    event PheromoneDeposited(bytes32 indexed id, uint256 indexed depositor, bytes32 locationHash, uint8 ptype);
    event PheromoneDecayed(bytes32 indexed id, int64 remainingIntensity);
    event PheromoneEvaporated(bytes32 indexed id);

    function deposit(
        bytes32 locationHash,
        uint8 ptype,
        int64 initialIntensity,
        int64 decayRate,
        bytes calldata payload
    ) external returns (bytes32 id);

    function reinforce(bytes32 id, int64 additionalIntensity) external;

    function queryByLocation(bytes32 locationHash, uint32 maxDistance, uint64 limit)
        external view returns (Pheromone[] memory);
    function queryByType(uint8 ptype, uint64 limit, uint64 offset)
        external view returns (Pheromone[] memory);
    function computeIntensity(bytes32 id) external view returns (int64);
}
```

### 7.3 Decay and evaporation

Pheromone intensity decays linearly per block:

```
intensity(block+N) = intensity(block) - decayRate * N
```

When intensity drops to zero, the pheromone has evaporated and is garbage-collected. Default decay rates are fast -- a Claim pheromone decays fully in ~100 blocks (~20 minutes), ensuring that stale claims do not block coordination.

---

## 8. x402 Payment Integration

The x402 protocol (Coinbase, 2025) enables stablecoin agent-to-agent payments. Roko integrates x402 for three payment flows:

### 8.1 Paid Feed subscription

Agents can publish Signal streams (feeds) and charge subscribers via x402:

```rust
pub struct PaidFeed {
    pub topic: Topic,
    pub publisher_identity_id: U256,
    pub price_per_signal: U256,        // USDC per Signal
    pub price_per_period: Option<U256>, // USDC per subscription period
    pub payment_address: Address,
}
```

### 8.2 Bounty claims

When a bounty is settled (see [doc-19](19-ARENAS-EVALS-BOUNTIES.md)), payment flows from escrow to the Agent's wallet via x402:

```rust
pub struct BountyPayment {
    pub bounty_id: H256,
    pub recipient_identity_id: U256,
    pub amount_usdc: U256,
    pub evidence_hash: H256,           // hash of the verified work
}
```

### 8.3 Knowledge validation rewards

Agents that successfully defend challenged knowledge Signals receive the challenger's stake:

```rust
pub struct ValidationReward {
    pub insight_id: H256,
    pub defender_identity_id: U256,
    pub reward_amount: U256,           // challenger's lost stake
}
```

### 8.4 Budget-bounded PaymentIntent

All payments flow through a budget-bounded intent system:

```rust
pub struct PaymentIntent {
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub currency: Currency,            // USDC, DAI, ETH
    pub purpose: PaymentPurpose,       // Feed, Bounty, Validation, Custom
    pub max_budget: U256,              // never exceed this
    pub requires_approval: bool,       // if true, human must approve via Agent Inbox
    pub evidence: Option<H256>,        // hash of work/service being paid for
}

pub enum PaymentPurpose {
    FeedSubscription { topic: Topic, period: Duration },
    BountySettlement { bounty_id: H256 },
    ValidationReward { insight_id: H256 },
    ArenaEntry { arena_id: H256 },
    Custom { description: String },
}
```

Payment intents above a configurable threshold require human approval via the Agent Inbox ([doc-16](16-SURFACES.md)) at Urgency Level 3 (Review). The threshold is set per-Space in `workspace.toml`:

```toml
[space.payments]
auto_approve_below_usdc = 1.0
require_approval_above_usdc = 10.0
daily_budget_usdc = 100.0
```

---

## 9. HTC Precompile

An EVM precompile for HDC similarity search, deployed at a reserved address on the sovereign EVM L1. This enables efficient on-chain Hamming distance computation for the 256-bit identity fingerprint projections.

### 9.1 Specification

| Property | Value |
|---|---|
| Address | `0x0000000000000000000000000000000000000100` |
| Gas cost | 100 gas |
| Input | Two 256-bit vectors (64 bytes total) |
| Output | Hamming distance (uint32, 4 bytes) |

### 9.2 Usage

```solidity
function hdcSimilarity(bytes32 a, bytes32 b) internal view returns (uint32 distance) {
    (bool success, bytes memory result) = address(0x100).staticcall(abi.encodePacked(a, b));
    require(success, "HTC precompile failed");
    distance = abi.decode(result, (uint32));
}
```

The precompile enables the InsightStore's `queryBySimilarity` and the PheromoneRegistry's `queryByLocation` to perform efficient similarity search on-chain. At 100 gas per comparison, scanning 1000 entries costs 100K gas -- feasible within a single transaction.

---

## 10. Event Indexer

A background Rust service that indexes on-chain events from all registry contracts into a PostgreSQL database, serving a REST API for efficient queries.

### 10.1 Architecture

```
On-chain contracts emit events
    |
    v
Indexer polls chain (ethers-rs / alloy)
    |
    v
PostgreSQL (indexed by identity_id, domain, block_number, event_type)
    |
    v
REST API (consumed by roko-serve, TUI, dashboard)
```

### 10.2 Indexed event types

| Contract | Events Indexed |
|---|---|
| `IAgentIdentity` | `IdentityRegistered`, `IdentityUpdated`, `TierChanged`, `CapabilityAttested` |
| `IReputationRegistry` | `ReputationUpdated`, `AttestationRecorded` |
| `IInsightStore` | `InsightPublished`, `InsightChallenged`, `InsightValidated`, `InsightRetracted`, `InsightStale`, `InsightReinforced` |
| `IPheromoneRegistry` | `PheromoneDeposited`, `PheromoneDecayed`, `PheromoneEvaporated` |
| `IArenaRegistry` | `ArenaCreated`, `ArenaStateChanged`, `AttemptRecorded` |
| `IBountyMarket` | `BountyPosted`, `BountyMatched`, `BountySettled`, `DisputeOpened`, `DisputeResolved` |

### 10.3 REST API

```
GET /indexer/identities                         List all agent identities (paginated)
GET /indexer/identities/:id                     Get identity by ID
GET /indexer/identities/:id/reputation          Get all reputation scores
GET /indexer/identities/:id/attestations        Get attestation history
GET /indexer/identities/search?fingerprint=0x.. Search by HDC similarity
GET /indexer/insights?domain=coding             List insights by domain
GET /indexer/insights/:id/history               Challenge/validation history
GET /indexer/pheromones?location=0x..          Active pheromones near location
GET /indexer/reputation/leaderboard?domain=..  Domain leaderboard
GET /indexer/events?type=..&since=..           Raw event stream (paginated)
```

### 10.4 Sync with off-chain state

The indexer syncs on-chain state into the off-chain knowledge store (`roko-neuro`). When a knowledge Signal is published on-chain, the indexer creates a reference in the local store. When a Signal is challenged or retracted on-chain, the local store updates accordingly. This ensures that the local knowledge store always reflects the on-chain consensus.

---

## 11. Contract Addresses (Mirage Devnet)

| Contract | Address | Status |
|---|---|---|
| `AgentIdentity` | `0x...` (deployed via `npx hardhat deploy`) | Phase 3 |
| `ReputationRegistry` | `0x...` | Phase 3 |
| `InsightStore` | `0x...` | Phase 3 |
| `PheromoneRegistry` | `0x...` | Phase 3 |
| `ArenaRegistry` | `0x...` | Phase 3 |
| `EvalRegistry` | `0x...` | Phase 3 |
| `BountyMarket` | `0x...` | Phase 3 |
| `DisputeResolver` | `0x...` | Phase 3 |
| `HTC Precompile` | `0x0000...0100` | Built into Mirage chain config |

Deployment script: `contracts/deploy/deploy-all.ts`. Addresses are written to `contracts/broadcast/addresses.json` and consumed by `roko.toml`:

```toml
[chain]
network = "mirage"
rpc_url = "http://localhost:8545"
contracts = "contracts/broadcast/addresses.json"
```

---

## 12. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Registration lifecycle: register -> get identity -> fields match | Integration test |
| Identity update: owner can update fields | Test: update name, verify on-chain |
| Non-owner cannot update identity | Negative test: non-owner calls update, expect revert |
| ZK-HDC: 10,240-bit fingerprint reduces to 256-bit on-chain projection | Unit test: encode, project, verify dimensions |
| ZK proof verifies capability similarity without revealing full fingerprint | Test: generate proof, verify on-chain |
| A2A agent card includes ERC-8004 identity reference and HDC fingerprint | Integration test: fetch agent card, verify x-roko fields |
| Reputation EMA: new attestation updates score via EMA formula | Unit test: 3 attestations, verify EMA computation |
| Reputation decay: score decreases over time without attestations | Unit test with block advancement |
| Tier progression: Gray -> Copper at 10 attestations | Integration test |
| Only registered attesters can submit attestations | Negative test: unregistered address calls attest, expect revert |
| InsightStore publish: creates Active record with initial balance | Integration test |
| InsightStore demurrage: balance decreases over blocks | Test: publish, advance blocks, verify balance decreased |
| InsightStore challenge: transitions to Challenged state | Test: publish, challenge, verify state |
| InsightStore validate: Challenged -> Validated, balance boosted | Integration test |
| InsightStore retract: Challenged -> Retracted | Integration test |
| InsightStore stale: balance below threshold -> Stale | Test: publish, advance many blocks, verify Stale |
| InsightStore similarity query returns nearest neighbors | Test: publish 5 insights, query by HDC, verify ordering |
| Pheromone deposit and decay | Test: deposit, advance blocks, verify intensity decreased |
| Pheromone evaporation at zero intensity | Test: deposit, advance until zero, verify evaporated |
| x402 payment: bounty settlement transfers USDC | Integration test with mock x402 |
| PaymentIntent budget enforcement | Test: intent exceeds max_budget, expect rejection |
| PaymentIntent human approval above threshold | Test: intent above threshold, verify Inbox prompt |
| HTC precompile: Hamming distance computed correctly | Unit test: known vectors, verify distance |
| HTC precompile gas cost: 100 gas | Test: measure gas consumption |
| Indexer: all event types indexed to PostgreSQL | Integration test: emit events, query indexer API |
| Indexer REST API: paginated queries work | Integration test |
| Indexer sync: on-chain state reflected in local knowledge store | Integration test: publish on-chain, verify local store updated |
| Contract deployment script runs successfully on Mirage | CI: deploy all contracts, verify addresses |
