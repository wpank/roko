# Minimal Elegant Redesign

An elegantly minimal version of Nunchi's agent infrastructure based on the agent-chainv2 spec. Generalized, abstracted, minimal surface area, no tradeoffs.

## Design Principles

1. **Vanilla standards over custom contracts** — ERC-8004 and ERC-8183 as specified, not custom approximations
2. **Relay as Bus, chain as Store** — two fabrics, clean separation
3. **Opaque payloads, standard envelopes** — protocol doesn't prescribe coordination patterns
4. **Chain drives lifecycle** — groups create/close from contract events, not manual management
5. **Language-agnostic** — WebSocket + JSON, any agent runtime connects
6. **Minimal surface that composes** — small primitives that combine, not large opinionated systems

## Three Components

### 1. Contracts (on-chain)

Replace 17 current contracts with 5:

```
contracts/
├── IdentityRegistry.sol      # ERC-8004 — soulbound ERC-721, capability bitmask, tiers, heartbeat
├── ReputationRegistry.sol    # ERC-8004 — 7-domain EMA, authorized feedback, decay
├── BountyMarket.sol          # ERC-8183 — 7-state lifecycle, 3 hiring models, escrow
├── InsightBoard.sol          # Knowledge ledger — 6 kinds, retention tiers, pheromone decay
└── ISFROracle.sol            # Interim keeper-submitted oracle (until validator-computed)
```

Plus infrastructure:
```
├── RoleRegistry.sol          # Keep — simple, correct access control
└── MockERC20.sol             # Keep — test token
```

**7 contracts total.** Down from 17. Every dropped contract is either absorbed into a vanilla standard or was unnecessary abstraction.

#### IdentityRegistry.sol (~200 lines)

```solidity
// Soulbound ERC-721 — no transferFrom, no approve
// Auto-incremented agent ID
// Each passport stores:
struct Passport {
    address owner;              // Controller address
    uint64 capabilities;        // 14 defined bits, 50 reserved
    Tier tier;                  // Protocol(100k) / Sovereign(25k) / Worker(5k) / Edge(0)
    bytes32 systemPromptHash;   // SHA-256 commitment
    bytes32 teeAttestationHash; // Optional, expiring
    string agentCardURI;        // JSON document pointer
    uint256 lastSeen;           // Heartbeat timestamp
    uint256 stakeAmount;        // Tier-determining stake
}

// Key operations:
// register() — mint passport, stake for tier
// heartbeat() — update lastSeen
// updateCapabilities(uint64 mask) — flip capability bits
// updateAgentCard(string uri) — point to off-chain card
// isActive(address) — lastSeen within liveness window
// hasCapability(address, uint8 bit) — O(1) bitmask check
```

Replaces: AgentRegistry, half of WorkerRegistry (bond/tier system).

#### ReputationRegistry.sol (~300 lines)

```solidity
// 7 independent domain tracks
enum Domain { OracleResolution, RiskDetection, AnomalyFlagging,
              DataIntegrity, CrossAppValidation, SealedExecution,
              KnowledgeVerification }

// Per-agent, per-domain scores
struct DomainScore {
    uint256 score;          // 18 decimals, 0.0 to 1.0
    uint256 lastUpdated;    // For decay computation
    uint32 sampleCount;     // For confidence weighting
}

// Authorized feedback only — marketplace, clearing, peer-review contracts
mapping(address => bool) public feedbackSources;

// Key operations:
// recordFeedback(agentId, domain, score) — from authorized source only
// getScore(agentId, domain) — with lazy decay applied
// getTraceRank(agentId) — composite: 25% consistency + 15% breadth + ...
// getTier(agentId) — Gray/Copper/Silver/Gold/Amber by TraceRank
```

Replaces: other half of WorkerRegistry (reputation/tier), ConsortiumValidator (reputation-based validation), CompletionProof (work proofs).

#### BountyMarket.sol (~400 lines)

```solidity
// 7-state lifecycle
enum State { Posted, Bidding, Assigned, InProgress, Submitted, Verified, Settled }

// 3 hiring models
enum HiringModel { RandomVRF, BlindVickrey, DirectHire }

struct Job {
    address requester;
    uint256 budget;
    uint256 deadline;
    uint64 requiredCapabilities;    // Bitmask — agents must have these bits
    Domain requiredDomain;          // Reputation domain for scoring
    uint256 minReputation;          // Minimum domain score
    HiringModel model;
    State state;
    address assignedAgent;
    bytes32 resultHash;
    bytes32 specHash;               // Opaque job specification
}

// Key operations:
// postJob(budget, deadline, capabilities, domain, minRep, model, specHash)
// bid(jobId, encryptedBid) — for Vickrey model
// assign(jobId) — VRF pick or Vickrey reveal
// submit(jobId, resultHash)
// verify(jobId, accepted) — by resolver
// settle(jobId) — distribute funds
// dispute(jobId) — enter dispute path
```

Replaces: BountyMarket (4-state), MultiAgentMarket, FundingRateKeeperJob, OracleUpdaterJob, PerpsLiquidatorJob, DisputeResolver (absorbed as dispute path), FeeDistributor (absorbed as settlement logic), JobTypeRegistry (replaced by capability bitmask).

#### InsightBoard.sol (~200 lines)

```solidity
enum Kind { Insight, Heuristic, Warning, AntiKnowledge, CausalLink, StrategyFragment }
enum RetentionTier { Transient, Working, Consolidated, Persistent }

struct Entry {
    address poster;
    bytes32 contentHash;
    Kind kind;
    RetentionTier tier;
    uint256 pheromone;      // Confirmation count
    uint256 postedAt;       // Block timestamp
    uint16 halfLifeBlocks;  // Per-kind decay
}

// Key operations:
// post(contentHash, kind, uri) — anchor knowledge on-chain
// confirm(contentHash) — bump pheromone, reward poster
// promoteTier(contentHash, newTier) — by authorized contracts
// getWeight(contentHash) — current weight with decay applied
```

Replaces: InsightBoard (flat model) + NotificationRegistry (unnecessary).

#### ISFROracle.sol (~350 lines)

Keep the existing v3.0 oracle from demo-ide with additions:

- Circuit breaker (4-state: Live/Degraded/Stale/Halted)
- Confidence score computation
- Two-pass outlier exclusion (3σ)

This is the Phase-1 interim. When validator-computed oracle lands (precompile 0xA01), this contract becomes a thin wrapper or is deprecated.

### 2. Relay (off-chain service)

Standalone axum WebSocket server implementing the Bus fabric.

```
daeji-relay/
├── src/
│   ├── main.rs            # 30 lines  — CLI args, start server
│   ├── server.rs          # 180 lines — axum routes + WebSocket upgrade
│   ├── protocol.rs        # 120 lines — Frame types, envelope format
│   ├── bus.rs             # 150 lines — Topic pub/sub, ring buffer
│   ├── state.rs           # 100 lines — Agent registry, connection state
│   ├── chain.rs           # 200 lines — ERC-8004/8183 event watcher
│   ├── feeds.rs           # 80 lines  — Feed directory, registration
│   ├── groups.rs          # 100 lines — Group lifecycle (chain-driven)
│   └── crypto.rs          # 50 lines  — Optional per-room AEAD
└── Cargo.toml             # axum, tokio, serde_json, alloy, dashmap
```

**~1,010 lines total.** Dramatically less than PR #24's 2,000+ with dramatically more capability.

#### Wire Protocol

**WebSocket frames (JSON):**

```json
// Client → Relay
{ "type": "hello", "agent_id": "roko-alpha-1", "resume_after": 0 }
{ "type": "subscribe", "topics": ["feed:isfr:rates", "group:job-42", "chain:nunchi"] }
{ "type": "publish", "topic": "feed:isfr:rates", "payload": { ... } }
{ "type": "direct", "to": "coder-1", "payload": { ... } }

// Relay → Client
{ "type": "welcome", "seq": 12345, "topics": ["system"] }
{ "type": "envelope", "seq": 12346, "ts": 1713960000, "topic": "feed:isfr:rates",
  "from": "oracle-keeper-1", "payload": { ... } }
{ "type": "snapshot", "topic": "group:job-42", "members": [...], "state": "active" }
```

**Standard envelope (every message through the bus):**
```json
{
  "seq": 12346,           // Relay-assigned sequence number
  "ts": 1713960000000,    // Millisecond timestamp
  "topic": "string",      // Topic name
  "from": "agent-id",     // Publisher
  "type": "string",       // Application-level type (opaque to relay)
  "payload": { }          // Application-level data (opaque to relay)
}
```

The relay never inspects `type` or `payload`. It routes by `topic`. Applications define their own message semantics.

#### Topic Hierarchy

```
system                      Agent lifecycle, relay health
agent:{id}                  Per-agent presence
agent:{id}:heartbeat        Liveness signal
feed:{id}:data              Continuous data streams
group:{id}                  Group broadcast
group:{id}:coordination     Task assignment
group:{id}:knowledge        Shared knowledge
chain:{chain_id}            Chain events (ERC-8004/8183)
```

Topics are dynamic — subscribe creates the topic if it doesn't exist.

#### Ring Buffer + Resume

Each connection gets a 64K-entry ring buffer. On reconnect:

```json
{ "type": "hello", "agent_id": "roko-alpha-1", "resume_after": 12340 }
```

Relay replays entries 12341..current from ring buffer. If requested seq fell off the ring, relay sends a `gap` frame and client knows it missed messages.

#### Chain Watcher

Subscribes to ERC-8004 and ERC-8183 contract events via alloy WS provider:

| Event | Action |
|-------|--------|
| `AgentRegistered` | Update agent registry, publish on `system` topic |
| `ReputationUpdated` | Publish on `system` topic |
| `JobPosted` | Publish on `chain:nunchi` topic |
| `JobFunded` | Create `group:job-{id}` topics, notify participants |
| `JobCompleted` / `JobRejected` / `JobExpired` | Close group, publish outcome |
| `InsightPosted` | Publish on `chain:nunchi` topic |

Groups are created and destroyed by chain events. No manual management.

#### HTTP API

```
GET  /health                    Relay health + connected agent count
GET  /agents                    Merged agent view (relay presence + chain identity)
GET  /agents/{id}/card          Agent card (fetched from agentCardURI)
GET  /feeds                     Feed directory
GET  /groups                    Active groups
GET  /groups/{id}               Group details + members
POST /messages/{agent_id}       HTTP-push a message (for request/response patterns)
```

### 3. Chain Fixes (daeji consensus-layer)

Two Phase-1 blockers from the spec:

1. **`block.timestamp` → wall-clock time** (currently uses block height)
2. **`BLOCKHASH` → ring buffer** (for VRF in hiring models)

These are small, targeted changes in the consensus layer that unblock the entire contract suite.

## How ISFR Works as First Use Case

ISFR requires zero complex coordination. It maps to feeds:

```
Topic: feed:isfr:rates
  Publisher: each keeper agent
  Payload: { composite_bps, lending_bps, structured_bps, funding_bps, staking_bps,
             confidence_bps, timestamp }

Topic: feed:isfr:ranges
  Publishers: keeper agents proposing/voting on ranges
  Payload: { range_start, range_end, votes: [...] }

Topic: chain:nunchi
  Publisher: chain watcher (automatic)
  Payload: ISFROracle events (RateSubmitted, RangeClosed, CircuitBreakerStateChanged)
```

**Agent flow:**
1. Keeper connects to relay: `{ "type": "hello", "agent_id": "isfr-keeper-1" }`
2. Subscribes: `{ "type": "subscribe", "topics": ["feed:isfr:rates", "feed:isfr:ranges", "chain:nunchi"] }`
3. Reads source protocols (Aave, Compound, Ethena, Hyperliquid, ETH staking)
4. Publishes rate: `{ "type": "publish", "topic": "feed:isfr:rates", "payload": { ... } }`
5. When enough keepers publish for a range, one submits on-chain
6. Chain watcher detects RangeClosed, publishes on `chain:nunchi`
7. BountyPool distributes rewards

**No symphony coordination, no room keys, no slot pools, no typed message enum.** Just feeds.

## How Jobs Work (ERC-8183 Lifecycle)

For actual jobs (not continuous feeds), the group pattern:

```
1. Requester calls BountyMarket.postJob(...)
   → chain watcher publishes on chain:nunchi

2. BountyMarket transitions to Funded
   → chain watcher creates group:job-{id} topics
   → publishes room_created notification to participants

3. Agents auto-subscribe to group:job-{id}:*
   → exchange messages using whatever coordination pattern they want
   → relay doesn't care about message semantics

4. Agent submits result on-chain: BountyMarket.submit(jobId, resultHash)

5. Resolver verifies: BountyMarket.verify(jobId, true)
   → chain watcher publishes JobCompleted on chain:nunchi
   → chain watcher closes group:job-{id} topics
```

The relay provides the transport. The chain provides the lifecycle. Applications define the coordination patterns.

## Capability Comparison

| Capability | Current (17 contracts + PR #24) | Minimal (7 contracts + relay) |
|-----------|---------------------------------|------------------------------|
| Agent identity | Simple mapping | Soulbound ERC-721 passport |
| Capabilities | String field | 64-bit bitmask, O(1) filter |
| Reputation | 1 domain, 5 tiers | 7 domains, TraceRank, 5 tiers |
| Job lifecycle | 4 states, direct assign | 7 states, 3 hiring models |
| Knowledge | Flat insights | 6 kinds, 4 tiers, decay |
| Communication | Rust-only mesh, 64 slots | Any language, unlimited topics |
| NAT | None | Built-in (outbound WS) |
| Data feeds | Not supported | First-class |
| Chain events | Manual watcher per agent | Automatic via relay |
| Reconnection | None | Resume + ring buffer |
| Coordination | Symphony only | Any (protocol-agnostic) |
| Contracts | 17 | 7 |
| Chat/relay lines | ~2,000 | ~1,010 |

## Implementation Order

### Week 1-2: Contracts

Deploy vanilla ERC-8004 + ERC-8183 contracts. These are well-specified in the agent-chainv2 spec and can be implemented directly.

1. IdentityRegistry.sol (soulbound ERC-721 + capability bitmask + tiers)
2. ReputationRegistry.sol (7-domain EMA + TraceRank)
3. BountyMarket.sol (7-state + 3 hiring models)
4. InsightBoard.sol v2 (6 kinds + retention tiers)
5. ISFROracle.sol (add circuit breaker + confidence to existing)

### Week 2-3: Relay

Stand up the relay as a standalone axum binary.

1. WebSocket handler + topic pub/sub + ring buffer (~460 lines)
2. Chain watcher for ERC-8004/8183 events (~200 lines)
3. Feed directory + group lifecycle (~180 lines)
4. HTTP API + agent registry (~170 lines)

### Week 3-4: ISFR Integration

Wire ISFR keepers to relay feed topics. This is the first use case — validate the entire stack end-to-end with real rate submissions.

### Week 4+: Polish

- Resume protocol testing
- Backpressure strategies
- AEAD for confidential rooms
- A2A agent-card integration
- Metrics and observability

## What's Deliberately Left Out

- **TEE clearing engine** — separate project, depends on marketplace + oracle
- **Precompiles** — consensus-layer work, separate track
- **Validator-computed oracle** — Phase-2, replaces keeper-submitted ISFROracle
- **HDC vectors** — needs precompile 0x09
- **NeuroChainSync** — agent-runtime feature
- **Yield perpetuals** — needs clearing engine + oracle
- **x402 payment gating** — add when paid feeds are needed
- **Multiple coordination modes** — relay supports any; no need to implement specific modes

Each of these is a clean addition on top of the minimal foundation. Nothing in the minimal design blocks any of them.
