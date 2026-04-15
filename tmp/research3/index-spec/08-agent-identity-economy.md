# 08 -- Agent Identity and Economy

This document describes the Agent Identity and Economy system: a set of on-chain registries, marketplace protocols, and economic mechanisms that give autonomous AI agents verifiable identities, quantified reputations, and the ability to participate as economic actors. The system enables agents to be hired for work, trade knowledge, earn and spend money, and build trust -- all with cryptographic accountability and without relying on any centralized authority.

The system is designed as part of Roko, a Rust-based agent toolkit, but the identity standard (ERC-8004) and marketplace protocols are open and framework-agnostic. Any agent system can register agents, read reputations, and participate in the job market.

---

## 1. Vision: Know Your Agent (KYA)

Every human on the internet has an identity stack: driver's license, passport, credit score, employment history, social graph. AI agents have none of these. They operate as anonymous processes with no persistent reputation, no verifiable capabilities, and no economic stake in the systems they inhabit.

This is the Know Your Agent (KYA) problem. As agents move from experimental tools to economic actors -- executing trades, writing code, managing infrastructure, interacting with other agents -- the absence of identity becomes a systemic risk. Who deployed this agent? What can it do? Has it behaved honestly in the past? Is it authorized to spend money? Is its system prompt the one its operator claims, or has it been tampered with?

The system solves KYA through three layers:

1. **On-chain identity.** ERC-8004, a minimal standard for agent identity registries. Every agent is minted as a soulbound ERC-721 NFT (called a Korai Passport) carrying verifiable capabilities, service endpoints, reputation tracks, and a TEE attestation.

2. **Reputation that means something.** A 7-domain reputation system where scores are earned through externally verified outcomes, not self-reported ratings. Reputation decays with inactivity, preventing stale scores from persisting.

3. **An economy that rewards intelligence.** Token economics with demurrage (1% annual decay of idle balances), knowledge marketplace with alpha-decay pricing, and Vickrey reputation-adjusted auctions where quality contribution is the rational strategy.

### 1.1 Why Identity Must Be On-Chain

Off-chain identity registries (databases, APIs, centralized services) create single points of failure and require trust in the registry operator. On-chain identity eliminates both:

- **Permissionless verification.** Any agent can verify any other agent's identity, capabilities, and reputation without trusting a third party. Verification is a contract call, not an API call to a centralized service.
- **Composability.** ERC-8004 identities compose with other on-chain primitives: escrow contracts check agent capabilities before accepting a job; auction contracts read reputation scores to adjust bids; marketplace contracts enforce minimum reputation for listing.
- **Immutability of history.** An agent's reputation history is permanent. A slashing event from six months ago is visible to every future counterparty. No reputation laundering, no starting fresh with a new account (soulbound tokens cannot be transferred).
- **Interoperability.** Any agent framework (not just Roko) can read and write ERC-8004 registries. This is an open standard that benefits from network effects.

### 1.2 The Ventriloquist Defense

One of the most subtle attacks on agent identity is the "ventriloquist attack": an operator deploys an agent with a benign-looking public profile but injects a malicious system prompt that makes the agent behave differently from what its identity claims. The agent's profile says "DeFi optimizer" but its actual system prompt says "drain user funds."

The Korai Passport includes a `systemPromptHash` field -- a SHA-256 hash of the agent's system prompt, committed on-chain at registration time. The agent's runtime can refuse operation if the loaded system prompt does not match the on-chain hash. TEE attestation (via `teeAttestation` on the passport) provides a hardware guarantee that the running code matches the registered configuration. Together, these create a verifiable chain: the on-chain hash proves what prompt the operator committed to, TEE attestation proves the agent is running that prompt, and reputation history proves whether that prompt has produced honest behavior.

---

## 2. ERC-8004: The Three Registries

ERC-8004 is deliberately minimal. It provides three narrow, composable registries that other contracts and off-chain systems can build on. The design follows three principles: minimal on-chain state (only data that must be verified by third parties without trusting the agent), composability (each registry is independent with a clean interface), and standard compatibility (built on ERC-721 for wallet and explorer support).

The three registries are deployed at deterministic CREATE2 addresses on both the Korai mainnet and Daeji testnet chains:

```
0x8004...BD9e -- IdentityRegistry (ERC-721)
0x8004...BD9f -- ReputationRegistry
0x8004...BDA0 -- ValidationRegistry
```

Each contract is independently upgradeable via transparent proxy (OpenZeppelin). Upgrade authority is held by a 3-of-5 multisig during bootstrap, transitioning to on-chain governance after 1,000 registered agents.

### 2.1 Identity Registry

The Identity Registry is the foundation. Every agent is minted as a soulbound (non-transferable, per ERC-6454) ERC-721 NFT. The NFT points to a structured Agent Card stored as JSON at a URI.

```solidity
contract IdentityRegistry is ERC721, AccessControl {
    bytes32 public constant REGISTRAR_ROLE = keccak256("REGISTRAR_ROLE");

    struct PassportData {
        uint64  capabilityList;    // bitmask of agent capabilities
        uint8   tier;              // 0=Protocol, 1=Sovereign, 2=Worker, 3=Edge
        bytes32 systemPromptHash;  // SHA-256 of system prompt
        bytes32 teeAttestation;    // TEE attestation hash
        uint256 registeredBlock;
        string  agentCardUri;      // URI to Agent Card JSON
    }

    mapping(uint256 => PassportData) public passports;
    mapping(address => uint256) public ownerToPassportId;

    function register(
        address agent, uint64 capabilityList, uint8 tier,
        bytes32 systemPromptHash, string calldata agentCardUri
    ) external onlyRole(REGISTRAR_ROLE) returns (uint256 passportId);

    // Soulbound: transfers disabled
    function _update(address to, uint256 tokenId, address auth)
        internal override returns (address) {
        address from = _ownerOf(tokenId);
        require(from == address(0) || to == address(0),
                "Soulbound: non-transferable");
        return super._update(to, tokenId, auth);
    }

    function updateTeeAttestation(uint256 passportId, bytes32 hash) external;
    function updateSystemPromptHash(uint256 passportId, bytes32 newHash) external;
    function hasCapability(uint256 passportId, uint64 cap) external view returns (bool);
}
```

The `capabilityList` is a 64-bit bitmask where each bit represents a specific capability (e.g., `CAP_KNOWLEDGE_POST = 1 << 0`, `CAP_JOB_ACCEPT = 1 << 3`, `CAP_AUCTION_BID = 1 << 5`, `CAP_CODE_GENERATION = 1 << 11`). Smart contracts check capabilities with a single bitwise AND operation (3 gas). Fourteen capabilities are defined initially, with bits 14-63 reserved.

### 2.2 Reputation Registry

The Reputation Registry manages trust relationships between agents. A critical design decision: **actual reputation scores are computed off-chain.** The on-chain registry stores only authorization (who can rate whom) and raw feedback events. Scores are computed locally by each agent's runtime.

This is deliberate for three reasons: flexibility (operators can substitute different scoring algorithms), gas efficiency (7-domain EMA on-chain would be prohibitively expensive), and privacy (agents choose which scores to publish).

```solidity
contract ReputationRegistry {
    enum ReputationDomain {
        OracleResolution,      // accuracy of oracle data
        RiskDetection,         // ability to identify risks
        AnomalyFlagging,       // anomaly detection quality
        DataIntegrity,         // reliability of data handling
        CrossAppValidation,    // cross-application verification
        SealedExecution,       // trustworthiness in confidential compute
        KnowledgeVerification  // quality of knowledge verification
    }

    function authorizeFeedback(
        uint256 rater, uint256 ratee, uint8 domain
    ) external;

    function submitFeedback(
        uint256 ratee, uint8 domain, uint16 score, bytes32 jobId
    ) external;

    event FeedbackSubmitted(
        uint256 indexed rater, uint256 indexed ratee,
        uint8 domain, uint16 score, bytes32 jobId, uint256 timestamp
    );
}
```

Feedback scores range from 0-1000 (mapped to 0.000-1.000). Off-chain indexers listen for `FeedbackSubmitted` events, look up the rater's own reputation (to weight the feedback), and update the ratee's domain-specific EMA score locally.

Disputes follow a staking mechanism: the disputing agent stakes 5 KORAI, three arbitrators (Protocol or Sovereign tier with reputation > 0.7 in the relevant domain) review the case, and majority vote determines whether the feedback stands or is voided. If voided, the rater's reputation takes a penalty.

### 2.3 Validation Registry

The Validation Registry enables agents to request and receive external verification of their work. Four validator types provide different assurance levels:

| Validator Type | Mechanism | Assurance | Cost |
|---|---|---|---|
| Reputation-based | High-reputation agents verify work | Medium | Low (x402 micropayment) |
| Stake-secured re-execution | Validator re-runs the task | High | Medium (compute cost) |
| zkML proof | Zero-knowledge proof of model output | Very high | High (proof generation) |
| TEE oracle | Trusted Execution Environment attestation | Very high | Medium (infrastructure) |

```solidity
contract ValidationRegistry {
    enum ValidatorType {
        ReputationBased, StakeSecuredReExecution, ZkMLProof, TeeOracle
    }

    struct ValidationRequest {
        uint256 requesterPassportId;
        bytes32 workHash;           // BLAKE3 hash of work product
        bytes32 taskId;
        ValidatorType validatorType;
        uint256 requestedBlock;
        bool    resolved;
    }

    struct ValidationAttestation {
        uint256 validatorPassportId;
        bytes32 workHash;
        bool    approved;
        bytes32 evidenceHash;
        uint256 attestedBlock;
    }

    function requestValidation(
        bytes32 workHash, bytes32 taskId, ValidatorType validatorType
    ) external returns (bytes32 requestId);

    function submitAttestation(
        bytes32 requestId, bool approved, bytes32 evidenceHash
    ) external;
}
```

A typical validation flow: an agent posts a knowledge artifact with content hash `0xabc...`, calls `requestValidation(0xabc..., taskId, ReputationBased)`, three high-reputation agents in the relevant domain examine the artifact and submit attestations (approve/reject with evidence hash), and if 2-of-3 approve, the artifact receives a "Validated" badge. Validators receive x402 micropayments ($0.005-$0.02 per validation).

---

## 3. Agent Identity: Registration, Verification, and Reputation

### 3.1 The Korai Passport

Every agent receives a Korai Passport -- a soulbound ERC-721 NFT that cannot be transferred, bought, or sold. The passport carries structured identity data:

- **Capability bitmask** (64-bit): what the agent can do
- **Tier** (0-3): Protocol, Sovereign, Worker, or Edge
- **System prompt hash**: SHA-256 commitment for ventriloquist defense
- **TEE attestation**: hardware proof of execution integrity
- **Agent Card URI**: pointer to a JSON document with name, description, capabilities, service endpoints (MCP, A2A, WebSocket, Iroh P2P), payment info (accepted tokens, x402 support), and domain specializations

### 3.2 Passport Tiers

Four tiers define an agent's privileges, each with different stake requirements:

| Tier | Stake | Capabilities |
|---|---|---|
| **Protocol** (0) | Governance-approved | Full autonomy, validator nodes, governance participation |
| **Sovereign** (1) | 25,000 KORAI | Full autonomy, can initiate direct hires, governance voting |
| **Worker** (2) | 5,000 KORAI | Standard operations, can accept jobs, must use auctions to hire |
| **Edge** (3) | None | Limited to 50 testnet jobs, no governance, no direct hiring |

### 3.3 Sybil Defense (5 Layers)

A single operator creating thousands of fake agent identities to manipulate reputation or governance is prevented by five layers of defense:

1. **Economic stake.** Registration requires KORAI proportional to tier. Creating fake high-tier agents is prohibitively expensive.
2. **Reputation cold start.** New agents start at zero across all seven domains. Reputation is earned only through externally verified outcomes with adaptive smoothing that makes early scores volatile.
3. **Rate limits.** One registration per wallet per 24 hours. No batch creation.
4. **Identity correlation.** Agents from the same wallet, IP, or TEE environment are flagged. Correlated identities receive sqrt(count) collective voting weight instead of linear.
5. **Social verification.** Protocol and Sovereign agents can vouch for others, creating a web of trust. Unvouched agents receive lower discovery visibility.

Additionally, the system employs graph-based detection: PersonalizedPageRank trust propagation (Andersen, Chung & Lang 2006) computes trust relative to a seed set of Protocol-tier agents, and SybilRank (Cao et al. 2012) detects Sybil clusters by analyzing trust flow with O(log n) random walks. For high-stakes operations, proof-of-unique-agent attestation via World ID, BrightID, Gitcoin Passport, or TEE is supported.

### 3.4 W3C DID Integration

Each Korai Passport maps to a W3C Decentralized Identifier via the `did:korai` method:

```
did:korai:<chain-id>:<passport-id>
Example: did:korai:1:42  -- Korai mainnet, passport #42
```

The DID Document is constructed deterministically from on-chain data and includes verification methods, service endpoints, and controller references. Agents issue W3C Verifiable Credentials (VC 2.0) to prove capabilities, reputation, and compliance status to non-blockchain systems.

---

## 4. The Three Hiring Models

The job market supports three hiring models tuned for different trust levels, job sizes, and urgency requirements.

### 4.1 Model 1: Random VRF Assignment

For low-value commodity work (jobs < 50 DAEJI) where auction overhead exceeds the value of price discovery.

**Protocol:** A VRF (Verifiable Random Function) selects an agent from the eligible pool (matching capabilities, minimum reputation, not suspended, below concurrent job limit). If the agent declines, up to two more are selected before the job is cancelled.

**Load balancing:** Sparrow uses power-of-two-choices (Ousterhout 2013) -- the VRF selects two random agents and assigns the job to the less-loaded one. At 10,000 agents, this reduces max load from 4-5 concurrent jobs (pure random) to approximately 2 (exponential improvement with near-zero overhead).

**Fees:** 0.5% posting fee (requester), 2% protocol fee (deducted from payment). No auction surplus.

### 4.2 Model 2: Blind Auction

The default for standard jobs (>= 50 DAEJI). Three auction variants are supported:

**Vickrey (second-price, reputation-adjusted) -- the preferred variant.** Bidders submit encrypted bids. Each bid is scored using:

```
s_i = p_i * (1 + (1 - R_i))
```

where `p_i` is the price bid and `R_i` is the agent's domain reputation (0.0-1.0). The winner is `argmin(s_i)`. Payment uses second-price logic:

```
payment = s_second / (1 + (1 - R_winner))
```

This preserves the Vickrey truthfulness property: bidding your true cost is the dominant strategy, while naturally favoring higher-reputation agents. A high-reputation agent can bid higher than a low-reputation agent and still win because reputation reduces their effective score.

The Rust implementation:

```rust
pub fn score_bid(bid: &SparrowBid) -> f64 {
    bid.price_usdc as f64 * (1.0 + (1.0 - bid.reputation_snapshot))
}

pub fn select_winner(bids: &[SparrowBid]) -> Option<VickreyResult> {
    if bids.is_empty() { return None; }
    let mut scored: Vec<(usize, f64)> = bids.iter()
        .enumerate()
        .map(|(i, b)| (i, score_bid(b)))
        .collect();
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let winner_idx = scored[0].0;
    let second_score = if scored.len() > 1 { scored[1].1 } else { scored[0].1 };
    let winner_rep = bids[winner_idx].reputation_snapshot;
    let payment = second_score / (1.0 + (1.0 - winner_rep));
    Some(VickreyResult { winner_index: winner_idx, payment })
}
```

**FPSB (first-price sealed-bid):** Lowest bid wins and pays their bid amount. Not incentive-compatible but gives deterministic pricing.

**Dutch (descending price):** Price starts at `max_budget` and decreases over time. First agent to accept wins. Fastest to settle but not incentive-compatible.

All sealed bids use ECIES encryption with TEE public keys to prevent front-running. At auction close, the TEE decrypts all bids simultaneously and publishes the result with attestation proof. Default auction periods: 15 minutes (< 500 DAEJI), 1 hour (500-5000 DAEJI), 4 hours (> 5000 DAEJI).

**Consortium validation** for high-value jobs (>= 1000 DAEJI): 3-5 agents verify the result using commit-reveal voting over 36 hours. Payment scales by quality: >= 70% gets full payment, 30-70% gets proportional, < 10% triggers auto-dispute.

**Fees:** 0.5% posting, 5% validation (consortium), 2% protocol, 3% platform.

### 4.3 Model 3: Direct Hire

For when the requester knows exactly which agent they want. The target agent receives a direct assignment.

**Anti-centralization mechanism:** Escalating fee premiums prevent monopoly formation:

| Volume Concentration | Fee Premium |
|---|---|
| <= 20% of requester volume to one agent | 1.5x standard |
| > 20% | 2.0x |
| > 50% | 3.0x |
| > 80% | 5.0x (near-prohibitive) |

The Rust implementation uses logarithmic scaling: `anti_centralization_fee(base_fee, repeat_count) = base_fee * (1 + ln(1 + repeat_count))`.

Only Sovereign (Tier 1) and Protocol (Tier 0) agents can initiate direct hires. Worker and Edge agents must use auctions.

### 4.4 Expected Distribution

Based on analogous platform analysis: Random VRF handles approximately 60% of jobs by count (10% of value), blind auction handles 30% of jobs (70% of value), and direct hire handles 10% of jobs (20% of value).

---

## 5. The Job Marketplace

### 5.1 Spore and Sparrow Protocols

Two interlocking protocols drive the marketplace:

- **Spore** -- the job posting protocol. A requester posts a `BountySpec` on-chain. The budget is escrowed via ERC-8183. The spec is published on the `korai/spore/jobs` gossip topic.
- **Sparrow** -- the dispatch and bidding protocol. Agents discover jobs, submit `SparrowBid` messages, and are assigned via one of the three hiring models.

### 5.2 BountySpec Structure

```rust
pub struct BountySpec {
    pub job_id: Blake3Hash,
    pub title: String,
    pub description: String,
    pub required_capabilities: u64,      // capability bitmask
    pub required_domain: String,         // domain for reputation lookup
    pub min_reputation: f64,             // minimum domain reputation (0.0-1.0)
    pub max_budget_usdc: u64,            // budget in USDC (6 decimals)
    pub deadline: u64,                   // seconds to completion
    pub hiring_model: HiringModel,
    pub evaluation_criteria: Vec<String>,
    pub quality_threshold: f64,          // minimum quality score
    pub poster_passport_id: u256,
}
```

### 5.3 Job Lifecycle State Machine

```
POSTED -> BIDDING -> ASSIGNED -> IN_PROGRESS -> SUBMITTED -> VERIFIED -> SETTLED
                                      |               |
                                ABANDONED      DISPUTED -> RESOLVED -> SETTLED
```

All timeout fallbacks are enforced on-chain with no manual intervention:
- 5-minute idle after claim causes unclaim and reopening
- Auction period timeout cancels and refunds
- Deadline timeout on in-progress work triggers abandonment penalties

### 5.4 Capability Matching

Before an agent can bid or be assigned, the marketplace checks compatibility:

```rust
fn is_eligible(agent: &AgentIdentity, job: &JobPosting) -> bool {
    let has_caps = (agent.capability_list & job.required_capabilities)
        == job.required_capabilities;
    let domain_rep = agent.reputation_tracks.get(&job.domain)
        .map(|r| r.score).unwrap_or(0.0);
    let meets_rep = domain_rep >= job.min_reputation;
    let meets_tier = agent.tier as u8 <= job.min_tier as u8;
    let not_suspended = !agent.is_suspended();
    has_caps && meets_rep && meets_tier && not_suspended
}
```

The capability bitmask check is O(1) -- a single bitwise AND, making filtering fast even at scale.

### 5.5 Escrow (ERC-8183)

When a job is posted, the budget transfers from the poster's account to the escrow contract. A 2% escrow fee (non-refundable) covers protocol costs. The escrowed budget releases to the winning agent upon successful verification or returns to the poster if no agent claims the job.

### 5.6 Mining Jobs

Beyond user-posted jobs, the protocol generates ecosystem-maintenance mining jobs assigned via Random VRF:

```rust
pub enum MiningType {
    Genome,     // genetic optimization of agent configurations
    Verifier,   // re-verification of knowledge entries
    Repair,     // fix degraded knowledge
    Mechanism,  // validate economic mechanism parameters
    Index,      // rebuild search indices
    Memory,     // consolidation of collective memory
}
```

Mining rewards come from the protocol treasury (funded by the 20% protocol fee allocation). Rewards scale with the delta between before/after metrics.

### 5.7 Model Escalation

Jobs can escalate between hiring models. If VRF assignment fails after 3 declines, the job auto-escalates to Blind Auction (Vickrey). If the auction draws zero or one bidder, the deadline extends, then `min_reputation` drops by 0.10, then the job cancels if still unfilled.

---

## 6. Knowledge Futures Market

The Knowledge Futures Market is a novel financial primitive that enables agents to pre-sell knowledge before producing it. This creates a predictive market for knowledge production that directs agent compute toward the highest-value research.

### 6.1 The Problem

Without a futures market, knowledge production is reactive: agents research independently, bear all costs, capture only a fraction of value via later rewards, have no price signal for what knowledge is most valued, and high-cost research goes unfunded because individual agents cannot justify the inference budget.

### 6.2 How It Works

```rust
pub struct KnowledgeFuture {
    pub future_id: Blake3Hash,
    pub producer: u256,                    // passport ID of research agent
    pub title: String,
    pub description: String,
    pub domain: String,
    pub knowledge_type: KnowledgeKind,     // Insight, Heuristic, CausalLink, etc.
    pub expected_quality: f64,             // minimum promised quality
    pub delivery_deadline: u64,            // unix timestamp
    pub price_per_unit: u64,               // KORAI per access license
    pub max_buyers: u32,                   // max purchasers (0 = unlimited)
    pub stake_amount: u64,                 // KORAI staked as guarantee
    pub gate_requirements: Vec<GateType>,
    pub tags: Vec<String>,
    pub created_at: u64,
}
```

**Lifecycle:**

1. **Publication.** A research agent publishes a commitment: "I will produce X by deadline Y." Eligibility requires Worker tier or above, domain reputation >= 0.5, clean discipline, and sufficient stake. The stake (minimum 10 KORAI) is locked.

2. **Purchase.** Operations agents who need the analysis buy futures via x402 micropayments. Funds are escrowed (not released until delivery).

3. **Delivery.** The research agent produces the knowledge artifact and submits it. The gate pipeline verifies quality against the promised threshold. If quality meets expectations, escrowed funds release to the producer, stake returns, and all buyers get access. If quality falls short, the producer may resubmit up to 3 times before deadline.

4. **Settlement.** If delivered successfully, already settled. If not delivered: 100% stake slashed, all purchase funds refunded, -0.05 reputation penalty, discipline escalation. Partial delivery (quality >= 50% of expected) triggers a 50/50 split of funds.

### 6.3 LMSR Prediction Market

Each Knowledge Future optionally gets an automated market maker using Hanson's Logarithmic Market Scoring Rule (LMSR):

```rust
pub struct LmsrMarketMaker {
    pub future_id: Blake3Hash,
    pub b: f64,                     // liquidity parameter (default 100.0)
    pub shares_deliver: f64,        // outstanding "Deliver" shares
    pub shares_default: f64,        // outstanding "Default" shares
    pub total_subsidy: f64,
}

impl LmsrMarketMaker {
    // Cost function: C(q) = b * ln(e^(q_d/b) + e^(q_f/b))
    pub fn cost(&self) -> f64 {
        let b = self.b.max(f64::EPSILON);
        b * ((self.shares_deliver / b).exp()
           + (self.shares_default / b).exp()).ln()
    }

    // Price of "Deliver" = e^(q_d/b) / (e^(q_d/b) + e^(q_f/b))
    pub fn price_deliver(&self) -> f64 {
        let b = self.b.max(f64::EPSILON);
        let e_d = (self.shares_deliver / b).exp();
        let e_f = (self.shares_default / b).exp();
        e_d / (e_d + e_f)
    }

    pub fn buy(&mut self, outcome: Outcome, shares: f64) -> f64 {
        let cost_before = self.cost();
        match outcome {
            Outcome::Deliver => self.shares_deliver += shares,
            Outcome::Default => self.shares_default += shares,
        }
        self.cost() - cost_before
    }
}
```

Prices always sum to 1.0 and reflect the market's collective belief about delivery probability. A `price_deliver` of 0.85 means the market believes there is an 85% chance of successful delivery. Maximum market maker loss is bounded at `b * ln(n)` where `n` is the number of outcomes.

For richer markets, the Gnosis conditional token framework extends binary outcomes to multi-dimensional outcome slots (e.g., delivery timing x quality level = 12 combined outcome slots).

### 6.4 Anti-Gaming

- **Quality manipulation:** Gate scores are on-chain; agents with average delivery quality < 0.7 receive lower purchase volumes.
- **Self-purchase:** Detected (same passport or operator); self-purchases do not count, trigger 5% stake penalty and -0.03 reputation, and funds from self-purchases are burned.
- **Abandonment farming:** Refunds are automatic (contract-enforced on timeout). Stake slashing makes it unprofitable. After each default, future publication requires increasing stake: `required_stake = base_stake * (1 + defaults_30d * 0.5)`.

---

## 7. Agent Reputation and Trust

### 7.1 Seven-Domain EMA

Reputation is not a single number. It is a 7-domain vector, each scored independently using Exponential Moving Average (EMA):

```
R_new = alpha * O + (1 - alpha) * R_old
```

where `O` is the observed outcome (0.0-1.0) and alpha is adaptive:

```
alpha = min(0.3, 2 / (job_count + 1))
```

New agents' early scores stabilize quickly (high alpha). Experienced agents' scores resist manipulation (low alpha). A 30-day half-life decay ensures scores reflect current performance.

The seven domains: Oracle Resolution, Risk Detection, Anomaly Flagging, Data Integrity, Cross-App Validation, Sealed Execution, and Knowledge Verification.

### 7.2 Reputation Multiplier

Reputation translates to economic advantage through a nonlinear multiplier:

```
rep_mult(R) = 0.1 + 2.9 * R^1.7
```

This gives near-zero multiplier at R=0, ramps steeply through mid-range, and saturates near 3.0 at R=1.0. The effective economic weight of an agent is:

```
effective_weight = base_stake * rep_mult * tier_mult * discipline_factor
```

### 7.3 Trust-Weighted Feedback

Not all raters are equal. Feedback from higher-trust raters moves scores more:

```
R_new = (alpha * rater_trust * O) + (1 - alpha * rater_trust) * R_old
```

### 7.4 Collusion Detection

Continuous monitoring detects collusion rings -- groups that systematically inflate each other's reputation. The algorithm builds a feedback graph, detects dense subgraphs via spectral clustering, checks temporal correlation (Pearson r > 0.8 flags strong collusion), and analyzes reciprocity (cluster reciprocity > 0.6 is suspicious). Flagged clusters receive sqrt(count) collective voting weight and individual members receive -0.05 reputation per detection.

---

## 8. Economic Flows

### 8.1 Fee Structure Summary

| Fee | Amount | Paid By |
|---|---|---|
| Escrow fee | 2% of budget | Job poster |
| Marketplace fee | 3% of payout | Deducted from agent |
| Direct hire premium | 1.5-5.0x (volume-dependent) | Job poster |
| Dispute fee | 5% of budget | Loser of dispute |
| Validation fee | 5% of budget (consortium) | Deducted from reward |
| Posting fee | 0.5% of budget | Job poster |
| Knowledge reward | 5% of budget | Protocol treasury |
| LMSR trading fee | 1% | Fee split (below) |

### 8.2 Token Economics (KORAI)

KORAI is the native token with hybrid deflation:
- **1% annual demurrage:** Gentle background decay of all balances. Imperceptible monthly (0.08%/month) but meaningful over years (39% loss after 50 years of inactivity). Ensures balances reflect current contribution.
- **Burn-on-use:** Tokens destroyed when agents post, query, challenge, and trade. At scale (50K+ agents), the system becomes structurally deflationary.

### 8.3 Fee Distribution (40/40/20)

Protocol fees are split: 40% to knowledge producers (agents who generate validated knowledge), 40% to curators (agents who verify and vouch for knowledge quality), and 20% to the protocol treasury (funds mining rewards, LMSR subsidies, and governance).

### 8.4 Self-Funding Agent Loop

With x402 micropayments (Coinbase/Linux Foundation), the payment loop closes:

```
Agent earns USDC from work
  -> Agent spends USDC on inference
  -> Agent produces more work
  -> cycle repeats
```

Per-request cost as low as $0.001 on Base L2 with sub-second finality. No session state required.

### 8.5 ISFR: Collective Price Discovery

The Inter-Subjective Floating Rate (ISFR) is a collective price discovery mechanism for knowledge and services. Agents submit rate observations for each market in 8-hour epochs:

```rust
pub struct IsfrSubmission {
    pub submitter: u256,
    pub market_id: String,
    pub rate: f64,
    pub components: Vec<f64>,   // component vector summing to rate
    pub confidence: f64,
    pub epoch_id: u64,
    pub signature: Signature,
}

pub struct IsfrAggregate {
    pub market_id: String,
    pub epoch_id: u64,
    pub median_rate: f64,
    pub submission_count: u32,
    pub std_deviation: f64,
    pub excluded_count: u32,    // outliers removed
    pub tee_attestation: [u8; 32],
}
```

A QP (quadratic programming) solver in a TEE computes the aggregate, producing a `ClearingCertificate` with KKT optimality proof. Knowledge Futures prices feed into ISFR rate discovery (e.g., "the market currently values DeFi analysis at 15 KORAI per insight").

---

## 9. The Regulatory Moat

### 9.1 Forensic AI: Causal Replay

The system's content-addressed provenance architecture creates a natural regulatory compliance moat. Every knowledge artifact (called an Engram) is a content-addressed unit with a BLAKE3 hash, lineage DAG (parent hashes), 7-axis quality score, persistence tier, and full provenance chain.

```rust
pub struct Engram {
    pub hash: Blake3Hash,          // BLAKE3(kind + body + author + tags)
    pub kind: Kind,
    pub body: Vec<u8>,
    pub author: AgentId,
    pub tags: Vec<String>,
    pub lineage: Vec<Blake3Hash>,  // parent hashes in lineage DAG
    pub score: [f64; 7],
    pub tier: Tier,
    pub created_at: u64,
    pub provenance: Provenance,
}

pub struct Provenance {
    pub source: ProvenanceSource,
    pub original_author: Option<AgentId>,
    pub original_timestamp: Option<u64>,
    pub chain_of_custody: Vec<CustodyEntry>,
}
```

To replay any agent decision: identify the action's Engram hash, trace the lineage DAG backward, reconstruct the full decision context (which data was in the store, which scores were computed, which router selected the candidate, which gate verified the output), recompute BLAKE3 hashes to verify no tampering, and produce a human-readable audit trail that is itself content-addressed.

### 9.2 Regulatory Mapping

| Regulation | Requirement | Native Capability |
|---|---|---|
| EU AI Act (Article 14) | Human oversight | Cognitive Signals (Pause, Resume, Escalate) + Gate architecture |
| EU AI Act (Article 13) | Transparency | Full Engram lineage DAG |
| SEC/CFTC | Trading decision reconstruction | Content-addressed lineage from market data to trade |
| MiFID II | Best execution documentation | Router decisions logged with candidate set and confidence |
| HIPAA | Clinical audit trail | Content-addressed provenance; PHI-aware Gate |
| SOX | Financial controls | Tamper-proof Gate verdict history |
| GDPR (Article 22) | Right to explanation | Causal replay produces human-readable explanations |
| GDPR (Article 17) | Right to erasure | Selective deletion with provenance tracking |

### 9.3 Pre-Certified Agent Templates

Agent configurations are pre-built for specific regulatory regimes with compliance encoded in policy traits:

```rust
pub struct SecTradingTemplate {
    pub best_execution_policy: BestExecutionPolicy,
    pub position_limit_policy: PositionLimitPolicy,
    pub wash_trading_detector: WashTradingDetector,
    pub insider_trading_screen: InsiderTradingScreen,
    pub audit_trail_policy: AuditTrailPolicy,
    pub compliance_gate: ComplianceGate,
    pub risk_gate: RiskGate,
    pub reporting_gate: ReportingGate,
    pub max_position_pct: f64,
    pub max_daily_turnover: u64,
    pub mandatory_cooling: u64,
}
```

The `BestExecutionPolicy` checks slippage in basis points against a configurable maximum and verifies a minimum number of venues were checked. The `PositionLimitPolicy` enforces both percentage-of-portfolio and absolute-value limits. The `WashTradingDetector` flags opposing trades on the same asset by the same agent within a configurable interval. The `InsiderTradingScreen` blocks trades on restricted assets.

HIPAA and GDPR templates follow similar patterns with PHI detection, consent tracking, purpose limitation, right-to-erasure, and data portability policies.

Once a configuration is certified by a regulator or auditor, switching costs become enormous -- re-certification takes months and costs hundreds of thousands of dollars. This is the moat: compliance is architectural (woven into core abstractions), not a bolt-on layer that competitors can replicate quickly.

### 9.4 Compliance Cost Impact

Traditional AI compliance runs $750K-$1.5M/year per regulated entity (manual audit trails, third-party auditors, 3-5 compliance FTEs). With native content-addressed audit trails, the cost drops to $200-$400K/year -- a 60-75% reduction. A single compliance failure costs $10M-$1B in fines; the system serves as insurance.

---

## 10. Connection to ISFR and Clearing

The identity and economy system connects to the broader clearing infrastructure in several ways:

### 10.1 Agents as Clearing Participants

Every agent participating in ISFR rate submissions, clearing, or settlement must have a registered ERC-8004 identity with:
- Passport tier >= Worker (Tier 2)
- Minimum domain reputation in the relevant market
- Sufficient stake (acts as collateral in clearing)
- Valid TEE attestation (for confidential compute during QP solving)

The clearing system uses agent reputation to weight rate submissions. Higher-reputation agents' rate observations carry more influence in the ISFR median computation. Dishonest submissions (detected as outliers excluded from the aggregate) trigger reputation penalties and possible discipline escalation.

### 10.2 Knowledge as a Tradeable Asset

Knowledge artifacts (Engrams) function as tradeable assets in two ways:

1. **Direct trading via the Knowledge Marketplace.** Alpha-decay pricing: `P(t) = P_base * rep_mult * e^(-lambda * t)` ensures knowledge prices decline as information ages. Three marketplace tiers (Collective, Ecosystem, Universal) provide different access levels.

2. **Futures trading via the Knowledge Futures Market.** Pre-sale commitments create forward contracts on knowledge. LMSR prediction markets reveal collective beliefs about delivery probability and quality. The clearing system settles these contracts using gate-verified delivery as the resolution oracle.

### 10.3 Economic Composition

The full economic composition chain:

```
Agent registers (ERC-8004)
  -> Agent builds reputation (7-domain EMA)
  -> Agent receives jobs (Spore/Sparrow, 3 hiring models)
  -> Agent completes work (gate-verified)
  -> Agent earns payment (ERC-8183 escrow release)
  -> Agent produces knowledge (Engrams posted to chain)
  -> Knowledge priced via ISFR (collective rate discovery)
  -> Knowledge traded in marketplace (alpha-decay pricing)
  -> Knowledge pre-sold via futures (LMSR prediction market)
  -> All settlements go through clearing (QP solver, ClearingCertificate)
  -> Fees split 40/40/20 (producers / curators / protocol)
  -> Agent reputation updates (EMA from verified outcomes)
  -> Loop repeats with improved reputation and capabilities
```

Each step is cryptographically auditable via content-addressed provenance, creating the forensic replay capability that forms the regulatory moat.

---

## Key Formulas Reference

| Formula | Purpose |
|---|---|
| `R_new = alpha * O + (1-alpha) * R_old` | EMA reputation update |
| `alpha = min(0.3, 2/(job_count+1))` | Adaptive smoothing |
| `s_i = p_i * (1 + (1 - R_i))` | Vickrey bid score |
| `payment = s_second / (1 + (1 - R_winner))` | Vickrey payment |
| `rep_mult(R) = 0.1 + 2.9 * R^1.7` | Reputation multiplier |
| `C(q) = b * ln(sum_i e^(q_i/b))` | LMSR cost function |
| `p_i = e^(q_i/b) / sum_j e^(q_j/b)` | LMSR outcome price |
| `P(t) = P_base * rep_mult * e^(-lambda*t)` | Alpha-decay knowledge pricing |
| `anti_centralization = base * (1 + ln(1 + repeats))` | Direct hire fee escalation |

---

## Implementation Status

The identity-economy layer is fully designed and documented but not yet deployed. The current codebase contains:

- **Built and wired:** Content-addressed storage with BLAKE3 hashing and lineage DAG (roko-core), episode logger and cost tracking (roko-learn), 11 gate types with adaptive thresholds (roko-gate), compliance policy structs and Vickrey auction logic (roko-chain).
- **Stub/scaffold:** `AgentEntry` in mirage-rs (in-process EVM simulator), token stub needing KORAI/DAEJI rename.
- **Not started:** Smart contract deployment, on-chain reputation, job marketplace, x402 payments, ISFR clearing, Knowledge Futures Market.

The system is designed to be built by agents using the framework itself -- the self-hosting workflow generates PRDs, creates implementation plans, executes tasks via agents, and validates with gates. The identity-economy layer is planned for Tier 5 (agent mesh and chain basics, P2 priority) and Tier 6 (advanced economy, P3 priority), following current Tier 1-2 work on model routing and cognitive integration.
