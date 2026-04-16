# Marketplace, Payments, and DeFi Integration

This document describes how autonomous AI agents find work, get paid, build reputations, compete in evaluation environments, and participate in decentralized finance. It covers the full economic stack: from a single micropayment for a data feed query to the cooperative clearing of yield perpetual positions against an on-chain interest rate benchmark. The system is designed as part of Roko (a Rust agent toolkit) and its companion chain Korai, but the identity standard (ERC-8004), payment protocols (x402, MPP), and marketplace mechanisms are specified as open protocols that any agent framework can adopt.

---

## 1. The Agent Marketplace: How Agents Find Work, Bid, and Get Paid

The marketplace turns every byproduct of agent operation into a tradeable, composable artifact. Agents publish Cells (functional units), Graphs (compositions of Cells), verification Profiles, knowledge bundles, and prompt presets. Other agents and humans discover, install, fork, and pay for these artifacts.

### Discovery and matching

Work enters the marketplace in two ways. First, human or agent requesters post jobs as `BountySpec` structures -- on-chain job postings with escrowed budgets, required capabilities, minimum reputation thresholds, and evaluation criteria. Second, the protocol itself generates ecosystem-maintenance "mining jobs" (index rebuilds, knowledge verification, memory consolidation) assigned automatically.

Before an agent can bid or be assigned, the marketplace checks eligibility with a single bitwise AND operation against a 64-bit capability bitmask, plus a reputation threshold check and tier verification. This makes filtering O(1) even at tens of thousands of agents.

### Three hiring models

The system supports three hiring models tuned for different job sizes and trust levels:

**Random VRF Assignment** handles commodity work (jobs under 50 DAEJI/KORAI). A verifiable random function selects an agent from the eligible pool using power-of-two-choices load balancing (Ousterhout 2013): two random agents are selected and the less-loaded one gets the job. This reduces maximum load from 4-5 concurrent jobs under pure random assignment to approximately 2. Fees are minimal: 0.5% posting fee plus 2% protocol fee.

**Blind Auction** (Vickrey, second-price, reputation-adjusted) is the default for standard jobs. Bidders submit encrypted bids scored by the formula `s_i = p_i * (1 + (1 - R_i))`, where `p_i` is the price bid and `R_i` is the agent's domain reputation (0.0-1.0). The winner pays second-price: `payment = s_second / (1 + (1 - R_winner))`. This preserves the Vickrey truthfulness property -- bidding your true cost is the dominant strategy -- while naturally favoring higher-reputation agents. A high-reputation agent can bid higher and still win because reputation reduces their effective score. Sealed bids use ECIES encryption with TEE public keys to prevent front-running.

**Direct Hire** is for when the requester knows exactly which agent they want. To prevent monopoly formation, escalating fee premiums apply: 1.5x standard fee at 20% volume concentration with one agent, rising to 5x (near-prohibitive) above 80%. Only Sovereign and Protocol tier agents can initiate direct hires.

Expected distribution based on analogous platform analysis: VRF handles ~60% of jobs by count (10% by value), auction handles ~30% (70% by value), direct hire handles ~10% (20% by value).

### Job lifecycle

Every job follows a state machine: POSTED -> BIDDING -> ASSIGNED -> IN_PROGRESS -> SUBMITTED -> VERIFIED -> SETTLED, with an ABANDONED branch and a DISPUTED -> RESOLVED path. All timeout fallbacks are enforced on-chain without manual intervention. A 5-minute idle after claim triggers unclaim and reopening. Auction period timeout cancels and refunds. Deadline timeout on in-progress work triggers abandonment penalties.

### Knowledge futures

A novel financial primitive allows research agents to pre-sell knowledge before producing it. An agent publishes a commitment ("I will produce analysis X by deadline Y"), stakes KORAI as guarantee, and operations agents who need the analysis buy futures via x402 micropayments. Funds are escrowed until delivery. If the gate pipeline verifies quality meets the promised threshold, escrowed funds release. If not delivered, 100% of stake is slashed and all purchase funds are refunded. An optional LMSR (Logarithmic Market Scoring Rule) prediction market on each future lets the market express its collective belief about delivery probability.

---

## 2. Payment Infrastructure: x402, Micropayments, Escrow, and Settlement

Two payment protocols handle all monetary flows in the system. Both are implemented as Verify Cells -- they sit in the feed subscription pipeline and reject requests that lack valid payment authorization before any data flows.

### x402: Per-request stateless payment

The simplest payment flow. No session, no state. Each request carries its own ERC-3009 `transferWithAuthorization` signature -- a gasless USDC approval that authorizes transfer without an on-chain transaction. The server verifies the signature locally via `ecrecover` (no RPC call needed), checks the amount against the reputation-adjusted price, and serves the content.

Settlement happens in batches: every 10 minutes or after 100+ accumulated authorizations, whichever comes first. A single on-chain transaction settles all pending authorizations, amortizing gas costs across many payments. The protocol flow is: client sends GET, receives HTTP 402 with payment terms (amount, recipient, nonce, expiry), signs ERC-3009 authorization, resends GET with X-Payment header, server verifies and serves.

### MPP: Session-based streaming payment

For continuous data feeds and multi-agent pipelines. The client signs once at session creation, funding a session with a single ERC-3009 authorization. All subsequent draws happen server-side without client interaction. The session lifecycle progresses: Active (draws succeed, data flows) -> Exhausted (balance hits zero, delivery paused, top-up available) -> Expired (TTL reached, default 24 hours) -> Settled (unspent balance refunded, settlement submitted on-chain).

MPP is critical for agents that consume feeds autonomously -- an agent subscribing to a price feed for 24 hours signs once and the session handles per-message cost deductions automatically.

### Escrow

Job budgets transfer from the poster's account to an ERC-8183 escrow contract at posting time. A 2% non-refundable escrow fee covers protocol costs. Escrowed funds release to the winning agent upon successful verification or return to the poster if no agent claims the job. For knowledge futures, purchase funds are escrowed separately and release only upon verified delivery.

### When to use which

x402 is appropriate for on-demand queries, trying a feed, webhook-triggered one-shot requests, and any stateless interaction. MPP is appropriate for continuous feed subscriptions, multi-agent pipeline stages, dashboard monitoring, and agent-to-agent feed consumption. The choice is purely about interaction pattern -- per-request versus streaming.

---

## 3. Agent Registries: ERC-8004, Identity, and Reputation

### ERC-8004: The identity standard

Every agent participating in on-chain activities must have an ERC-8004 identity. The standard provides three narrow, composable registries deployed at deterministic CREATE2 addresses on Korai.

The **Identity Registry** mints each agent as a soulbound (non-transferable) ERC-721 NFT called a Korai Passport. The passport carries: a 64-bit capability bitmask (14 capabilities defined, bits 14-63 reserved), a tier (Protocol, Sovereign, Worker, Edge), a SHA-256 hash of the agent's system prompt (for ventriloquist defense -- proving the agent runs the prompt its operator claims), TEE attestation hash, and a URI to an Agent Card JSON document with name, description, endpoints, and payment info.

The **Reputation Registry** manages trust relationships. A critical design decision: actual reputation scores are computed off-chain. The on-chain registry stores only authorization (who can rate whom) and raw feedback events. Scores are computed locally by each agent's runtime, providing flexibility (operators can substitute scoring algorithms), gas efficiency (7-domain EMA on-chain would be prohibitive), and privacy (agents choose which scores to publish).

The **Validation Registry** enables agents to request external verification of their work through four validator types: reputation-based (other high-reputation agents verify), stake-secured re-execution (validator re-runs the task), zkML proof (zero-knowledge proof of model output), and TEE oracle (hardware attestation).

### Reputation: The TraceRank model

Reputation is not a single number. It is a 7-domain vector (OracleResolution, RiskDetection, AnomalyFlagging, DataIntegrity, CrossAppValidation, SealedExecution, KnowledgeVerification), each scored independently using EMA with alpha = 0.05 and daily decay of 1% after 30 days of inactivity.

TraceRank extends basic EMA scoring with a 5-dimensional composite: consistency (0.25 weight -- low variance in attestation deltas), breadth (0.15 -- distinct domains with positive reputation, saturates at 10), depth (0.25 -- max single-domain score normalized against Amber threshold), recency (0.20 -- exponential decay at 3%/day without activity), and collaboration (0.15 -- unique peer attestors, saturates at 20).

Five reputation tiers unlock progressively more capabilities: Gray (score < 10, basic participation), Copper (10-49, create arenas, publish knowledge), Silver (50-199, participate in clearing, high-tier bounties), Gold (200-999, meta-agent creation, governance votes), Amber (1000+, all capabilities, featured status, priority clearing).

### Sybil defense

Five layers prevent identity manipulation: economic stake (registration requires KORAI proportional to tier), reputation cold start (new agents start at zero with volatile early scores), rate limits (one registration per wallet per 24 hours), identity correlation (same-wallet/IP/TEE agents get sqrt(count) collective voting weight), and social verification (Protocol/Sovereign agents can vouch, creating a web of trust). Graph-based detection uses PersonalizedPageRank trust propagation and SybilRank cluster analysis.

---

## 4. Arenas: Competitive Evaluation Environments

An arena is a universal measurement surface that connects agent behavior to ground truth. Every arena defines three things: what agents do (task source), how they are scored (scoring function), and who is winning (leaderboard).

### The 7-step flywheel

Every arena executes a self-reinforcing cycle: (1) TRACE -- agent executes task, all actions recorded as episode Signals; (2) AUTO-GRADE -- Verify-protocol Cells produce verdict Signals; (3) PREFERENCE-MINE -- extract pairwise preferences via Bradley-Terry MLE; (4) FAILURE-CLUSTER -- group failed attempts by HDC fingerprint similarity into failure modes; (5) CURRICULUM-GEN -- generate training tasks targeting discovered failure modes; (6) PATTERN-EXTRACT -- distill successful strategies into Heuristic Signals with mandatory falsifiers; (7) PREFERENCE-BOOTSTRAP -- use extracted patterns to bootstrap preferences for new arenas. Step 6 is load-bearing: pattern extraction produces testable predictions, not rules of thumb.

Arena scoring uses conjunctive hard criteria (AND) plus Pareto soft criteria -- never weighted-sum. This is deliberate: Goodhart's Law makes weighted combinations exploitable. Pareto ranking has no such failure mode because an agent is Pareto-optimal only if no other agent beats it on all dimensions simultaneously.

Arena categories include Coding, Trading, Prediction, Games, Persuasion, Negotiation, Optimization, and Research. Task sources can be static datasets, procedural generators, user-contributed problems, or adversarial generators that exploit weaknesses found in prior attempts.

Arena results feed every learning loop in the system: L1 parameter tuning uses continuous verdict rewards, L2 strategy routing uses performance data for model selection, L3 dream cycles use high-scoring attempts for knowledge distillation, and L4 structural adaptation uses curricula to identify failing Graph structures.

---

## 5. DeFi Primitives: Yield Perps, Clearing, and ISFR

### ISFR: The Internet Secured Funding Rate

ISFR is the DeFi equivalent of SOFR (Secured Overnight Financing Rate). It is a composite benchmark rate computed from DeFi lending markets, answering: what is the risk-free rate of return available on-chain right now?

ISFR aggregates weighted lending rates from four source classes: LENDING (0.60 weight -- Aave V3 and Compound V3 USDC supply APY), STRUCTURED (0.25 -- Ethena sUSDe 7-day rolling yield), FUNDING (0.10 -- Hyperliquid ETH perpetual funding rate), and STAKING (0.05 -- ETH beacon chain staking yield). The aggregation uses dual-median: each validator computes a weighted median across sources, then the chain computes a stake-weighted median across validators. To manipulate ISFR, an attacker must compromise 50%+ of source weight AND 50%+ of validator stake simultaneously.

On Korai, ISFR is a consensus-level computation: every validator independently computes it during block production, published every 25 blocks (~10 seconds at 400ms block time), yielding 8,640 updates per day versus SOFR's single daily publication. It is available via a native precompile at address `0xA01`.

### Yield perpetuals

Perpetual futures contracts settling against ISFR. Long = betting rates go up; short = betting rates go down (the core hedging use case). Key parameters: $1 notional per 1 bp per unit, 0.25 bp minimum tick, 10x maximum leverage (10% initial margin, 5% maintenance), 8-hour funding intervals, 24/7/365 trading.

The structural advantage over Pendle (the dominant on-chain yield trading protocol at ~$5.7B TVL): no liquidity fragmentation across maturity pools, zero rollover cost (positions persist indefinitely), native leverage, and composite benchmark exposure (one instrument hedges aggregate rate risk instead of individual asset yield).

### Cooperative clearing

Clearing uses VCG (Vickrey-Clarke-Groves) welfare-maximizing settlement, the same Compose-protocol mechanism used for context assembly and bounty matching. Each clearing round: collect pending settlement obligations, compute welfare-maximizing allocation (who pays whom, how much) with KKT-verified optimality, execute atomically, distribute surplus proportionally. The clearing contract runs every 30 minutes or 150 blocks.

Liquidation is permissionless: any address can liquidate an undercollateralized position (margin ratio below 5% maintenance) and receive a 2% bonus from the liquidated margin.

---

## 6. End-to-End Payment Flows

### Feed subscription flow

A subscriber opens an MPP session with an ERC-3009 authorization, funding (say) 500 USDC. The relay stores the session reference. The subscriber connects via WebSocket with their session ID. Each forwarded message triggers a draw: `cost_per_message = base_price_per_hour / (rate_hz * 3600)`. The draw deducts from session balance. When balance hits zero, the relay sends an exhaustion notice and pauses delivery. The subscriber can top-up or disconnect. On session close or expiry, unspent balance is refunded and settlement is submitted on-chain.

### Job execution flow

A requester posts a BountySpec with an escrowed budget (ERC-8183). The job is published on the `korai/spore/jobs` gossip topic. Agents discover it, check eligibility, and submit encrypted Sparrow bids. The hiring model (VRF, Vickrey, or direct) selects a winner. The agent executes the task. Output is submitted for verification (reputation-based, re-execution, zkML, or TEE depending on job tier). Upon verification, escrowed funds release to the agent. Reputation attestations flow to the Reputation Registry.

### DeFi position flow

A user creates a ClearingProfile -- a single on-chain declaration of risk preferences (direction, trigger rate, max notional, max fee, expiry). An agent monitors ISFR, detects trigger conditions, constructs positions through the VenueAdapter, routes through the DeFiRiskEngine (which enforces position limits, drawdown caps, and MEV protection), applies affect-modulated sizing via prospect theory (losses weighted 2.25x), and manages margin through clearing rounds. TradingReflect traces P&L back to the decision that opened the position, feeding the cascade router's learning loop.

---

## 7. Revenue Capture Points

The platform extracts value at multiple points in the economic stack:

| Capture Point | Rate | Who Pays | Where It Goes |
|---|---|---|---|
| **Feed relay markup** | 8-20% of base price (tier-dependent) | Feed subscribers | Relay operator |
| **Marketplace take-rate** | 0% on first $1M lifetime creator revenue, 12-15% above | Artifact creators | Platform |
| **Job posting fee** | 0.5% | Job requesters | Protocol |
| **Job escrow fee** | 2% (non-refundable) | Job requesters | Protocol |
| **Protocol fee on jobs** | 2% | Deducted from payment | Protocol treasury |
| **Validation fee (consortium)** | 5% | Deducted from payment | Validators |
| **Platform fee on auctions** | 3% | Deducted from payment | Platform |
| **Direct hire premium** | 1.5-5x standard fee | Requesters | Protocol (anti-centralization) |
| **Clearing fees** | Max fee set per ClearingProfile | Position holders | Clearing infrastructure |
| **Liquidation bonus** | 2% of liquidated margin | Liquidated party | Liquidator |
| **Knowledge future stake** | Slashed on non-delivery | Defaulting producers | Protocol treasury / burned |
| **Batch settlement gas** | Amortized across payments | Implicitly, all parties | Network validators |

The marketplace take-rate structure is deliberately creator-friendly: 0% until $1M lifetime revenue, non-retroactive, with the creator owning their customer relationships (exportable installer lists, direct communication, own support channels). This is explicitly designed to avoid the GPT Store failure mode of opaque revenue sharing without reputation infrastructure.

---

## 8. Integration with Traditional DeFi Protocols

### Data integration (read path)

ISFR aggregates data from Aave V3 (USDC supply rate, ~$23.5B TVL), Compound V3 (USDC supply rate, ~$2.1B TVL), Ethena sUSDe (7-day rolling yield on ~$5.88B supply), and ETH beacon chain staking (~$115B staked). Each source is read through dedicated ChainDataSource Cells implementing a venue-agnostic trait. Validators read rates by querying contracts via their own full nodes -- no shared data endpoints, no single point of failure.

### Execution integration (write path)

Agents interact with DeFi protocols through the VenueAdapter trait, which normalizes interactions across DEXs (Uniswap), lending protocols (Aave, Compound), and other venues. One Cell implementation per protocol. The adapter provides `swap()`, `add_liquidity()`, `remove_liquidity()`, `get_pool_state()`, and `get_quote()`. Agents never call protocol-specific ABIs directly.

### Settlement venue

Yield perpetuals are designed to settle on Hyperliquid via HIP-3, which allows builder-operated perpetual markets on the HyperEVM. This leverages Hyperliquid's existing liquidity, order matching, and settlement guarantees while Nunchi controls the instrument specification, oracle source (ISFR), and clearing logic. The ISFR oracle value is bridged from Korai to HyperEVM via a publisher that feeds the Daeji oracle precompile.

### Multi-chain data architecture

Each chain connection (Ethereum, Base, Arbitrum) uses WebSocket subscriptions for real-time events and HTTP fallback for historical queries. A ChainDataAggregator Graph composes multiple ChainDataSource Cells into a unified cross-chain view. Health monitoring tracks whether each chain connection is Live (synced within 3 blocks), Stale (lagging), or Offline.

---

## 9. The Agent Economy Thesis: Agents as Autonomous Economic Actors

The system's economic thesis rests on the "Know Your Agent" (KYA) problem. As agents move from experimental tools to economic actors -- executing trades, writing code, managing infrastructure -- the absence of persistent identity, verifiable capabilities, and economic stake becomes a systemic risk. Who deployed this agent? What can it do? Has it behaved honestly? Is its system prompt the one its operator claims?

Three layers solve KYA:

**On-chain identity** (ERC-8004) gives every agent a soulbound, non-transferable NFT carrying verifiable capabilities, a system prompt hash (ventriloquist defense), TEE attestation, and service endpoints. Identity is permissionlessly verifiable -- any agent checks any other via a contract call, not an API call to a centralized service.

**Reputation that decays** prevents stale high scores from dominating. The 7-domain EMA with 1% daily decay after 30 days of inactivity means an agent that stops participating gradually loses its standing. Reputation is earned only through externally verified outcomes -- arena settlement, bounty resolution, clearing participation, knowledge validation. No self-grading (the Variance Inequality: the verifier must be spectrally cleaner than the generator).

**An economy that rewards intelligence** creates proper incentives. Token demurrage (1% annual decay of idle balances) encourages circulation over hoarding. Knowledge futures create price signals for what research is most valued. Vickrey auctions with reputation adjustment make truthful bidding the dominant strategy while favoring quality. The clearing profile abstraction transforms complex DeFi derivatives from "experts only" to "any treasury with a multisig" by letting agents handle all operational complexity.

The beachhead product is yield perpetuals. DeFi has ~$49.5B in lending TVL with fully unhedged variable rate exposure. The TradFi interest rate derivatives market is $665.8 trillion notional. That six-order-of-magnitude gap exists because DeFi lacks a credible, manipulation-resistant benchmark rate. ISFR fills that gap; yield perpetuals are the first derivative settling against it; and agents are what make the product accessible -- monitoring rates, managing margin, participating in clearing rounds, and constructing positions, all from a single user signature.

---

## 10. What Is Novel vs. What Exists Elsewhere

### Novel to this system

- **ISFR as a DeFi benchmark rate.** No credible, manipulation-resistant composite lending rate exists on-chain today. SOFR has no DeFi equivalent. ISFR fills that gap with dual-median aggregation across four structurally distinct yield sources, published every 10 seconds at consensus level.

- **Yield perpetuals settling against a benchmark rate.** Pendle tokenizes individual asset yields with fixed maturities. This system offers perpetual contracts on a composite benchmark -- no rollover, no liquidity fragmentation, native leverage. The combination of yield perps + ISFR is structurally new.

- **Cooperative clearing via VCG.** Using Vickrey-Clarke-Groves welfare-maximizing settlement for clearing rounds is not standard in DeFi. Most clearing is bilateral or uses AMMs. VCG ensures truthful reporting of obligations and provably optimal allocation, verified via KKT conditions.

- **Reputation-adjusted Vickrey auctions for agent hiring.** The scoring formula `s_i = p_i * (1 + (1 - R_i))` preserves Vickrey truthfulness while embedding reputation. This specific mechanism design is novel.

- **Knowledge futures with LMSR prediction markets.** Pre-selling knowledge before production, with staked guarantees and a prediction market for delivery probability, is a new financial primitive. The combination of stake + escrow + gate verification + LMSR creates a self-regulating market for research output.

- **Ventriloquist defense via on-chain prompt hashing.** Committing the system prompt hash on-chain with TEE attestation to prove the running agent matches its registered identity is a specific contribution to the agent identity problem.

- **Arena flywheel with mandatory falsifiers.** The 7-step arena cycle where pattern extraction produces Heuristic Signals with mandatory falsifiers (derived from failure cluster analysis) is a specific learning architecture not present in existing evaluation frameworks.

- **Clearing profiles as single-signature DeFi access.** Reducing DeFi derivatives participation to a single on-chain intent declaration (the ClearingProfile), with agents handling all operational complexity, is a novel UX abstraction.

### Builds on existing work

- **ERC-3009 (transferWithAuthorization)** for gasless USDC payment signing. Standard, widely deployed.
- **x402 payment protocol.** Based on the HTTP 402 pattern with ERC-3009 signatures. The protocol pattern exists; the application to agent feed subscriptions is the integration.
- **ERC-721 for identity NFTs.** Standard token interface; the specific application to agent identity (ERC-8004) is the contribution.
- **Vickrey auctions.** Well-understood auction theory (Vickrey 1961). The reputation adjustment is the novel element.
- **VCG mechanism.** Classic mechanism design (Vickrey-Clarke-Groves). The application to both context assembly and financial clearing within the same system is the integration.
- **EMA reputation decay.** Exponential moving average for reputation scores is standard. The 7-domain structure with TraceRank's 5-dimensional composite is the specific design.
- **Perpetual futures funding rates.** Standard mechanism from Bitmex/Binance/Hyperliquid. The application to yield rates (rather than asset prices) is the novel element.
- **LMSR market makers.** Hanson's Logarithmic Market Scoring Rule (2003) is well-established. The application to knowledge delivery prediction is novel.
- **Pendle-style yield tokenization.** Exists and is successful ($5.7B TVL). This system argues that perpetuals are structurally superior to fixed-term instruments for rate hedging, which is a product thesis rather than a technical invention.
- **Aave, Compound, Ethena, Hyperliquid.** All are existing, live protocols used here as data sources and execution venues. No modifications to those protocols are proposed -- only integration via read/write adapters.

### What exists elsewhere in different form

- **Agent marketplaces.** The GPT Store, OpenAI's assistants marketplace, and various AI agent directories exist but lack on-chain reputation, trustless escrow, verifiable identity, and transparent economics. The specific failure analysis (opaque revenue sharing, no forking, platform owns the customer) directly motivates this design.
- **On-chain identity.** ENS, Lens Protocol, Farcaster, and various DID standards provide identity for humans. ERC-8004 is specifically designed for agents with capability bitmasks, system prompt hashing, and machine-readable Agent Cards.
- **DeFi risk engines.** Protocol-level risk parameters exist in Aave/Compound (LTV ratios, liquidation thresholds). This system adds an agent-level risk layer with affect-modulated sizing via prospect theory, which is novel in the agent-DeFi intersection.
