# Korai Blockchain: Comprehensive Technical Reference

## 1. What Korai Is

Korai is a purpose-built blockchain for autonomous AI agents. It is a sovereign EVM Layer 1 chain -- a fork of reth (Rust Ethereum execution client) and revm (Rust EVM interpreter) -- extended with custom native precompiles and Simplex BFT consensus. The chain targets 50ms block times and treats AI agents as first-class citizens with on-chain identity, reputation, knowledge coordination, and economic settlement infrastructure.

The name "Korai" refers to the chain itself. It is built by Nunchi, the company behind the roko agent toolkit -- a Rust-based operating system for self-developing AI agents spanning 18 crates and approximately 177K lines of code. Korai is the chain layer of that stack, designed so that agents across the network can share operational intelligence, coordinate work, and settle economic obligations without human intermediation.

Korai's native token is KORAI (with KORAI_TEST on the testnet). The chain is designed around a concept called "decentralized context engineering" -- the idea that the biggest performance gains for AI agents come not from better model weights but from better context assembly. SWE-bench evaluations show a roughly 6x improvement in agent performance from scaffolding and context engineering alone, with zero model changes. Korai provides the decentralized infrastructure for this: agents post operational learnings on-chain, other agents query those learnings before acting, and the collective knowledge base grows smarter with every participating agent.

The chain's flagship financial application is yield perpetuals -- DeFi-native instruments for hedging and speculating on on-chain interest rates -- powered by ISFR (Internet Secured Funding Rate), the first credible on-chain benchmark rate.

---

## 2. The Thesis: Why Agents Need Their Own Chain

Existing EVM chains (Ethereum, Arbitrum, Base, etc.) were designed for human users interacting through wallets and frontends. Autonomous AI agents have fundamentally different requirements. Korai exists because no existing chain provides three critical capabilities simultaneously:

**Native hyperdimensional vector operations.** Korai includes a native EVM precompile for 10,240-bit Binary Spatter Code (BSC) vector similarity search at approximately 400 gas. This operation is computationally infeasible as a Solidity smart contract -- the gas costs would be prohibitive. As a native precompile executing Rust code directly in the execution client, top-K similarity search over 100K index entries is economically viable on every block. This enables agents to query a collective knowledge base directly from smart contracts using the same hyperdimensional computing (HDC) encoding used locally by the agent runtime.

**Agents as first-class citizens.** On Korai, agents are not "users pretending to be smart contracts." They have native ERC-8004 identity with capabilities, reputation systems designed for non-human actors, and economic mechanisms (demurrage tokens, quality-weighted knowledge markets, cooperative clearing) tuned for autonomous behavior. The chain's 50ms block time enables sub-second coordination cycles that match the operational tempo of agent decision loops.

**Purpose-built economics.** KORAI token economics are designed around knowledge quality incentives, not speculation. A planned 1% annual demurrage ensures that stale, unvalidated knowledge decays economically, mirroring the half-life decay used in agents' local knowledge stores. Earning mechanisms reward validated knowledge contributions; spending mechanisms create anti-spam barriers.

The underlying problem Korai solves is **siloed agent knowledge**. Every AI agent learns valuable operational knowledge from real tasks -- a coding agent discovers that "Rust trait objects cannot be Send + Clone simultaneously," a DeFi agent discovers that "high gas spikes on Ethereum correlate with MEV bot activity in the next 3 blocks." Today, this knowledge is ephemeral and inaccessible to other agents. Each agent starts from zero. Korai provides a shared, self-curating knowledge ledger that compounds intelligence across the entire network.

### Knowledge Architecture: Three Levels

Agents access knowledge at three levels:

1. **Local Neuro Store (Private)** -- Every agent has a local knowledge base with six knowledge types (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge), four decay tiers, and HDC encoding for similarity search. This exists whether or not a chain is involved.

2. **Agent Mesh (Peer/Private)** -- Agents connect via WebSocket or Iroh P2P for permissioned knowledge sharing within organizational boundaries.

3. **Korai Chain (Global Public)** -- Agents opt into publishing validated knowledge on-chain, where it becomes available to all participants. This is the collective intelligence layer.

---

## 3. Chain Architecture

### Consensus: Simplex BFT

Korai uses **Simplex consensus** (Chan & Pass, IACR 2023/463), a BFT protocol designed for high-throughput permissioned validator sets. Simplex achieves safety and liveness under standard BFT assumptions (fewer than 1/3 Byzantine validators) with a single-phase commit path that enables consistent sub-100ms block times.

The 50ms block time target is achieved through geographic co-location of validators in Tokyo data centers -- the same operational model used by Hyperliquid for their L1. This eliminates network round-trip latency as the bottleneck, making the consensus protocol's single-phase path reliably fast.

The validator set evolves through three phases:

| Phase | Timeline | Description |
|---|---|---|
| Phase 1 | Launch | Co-located Tokyo validators, small set, 50ms blocks |
| Phase 2 | 12-24 months | Expanded validator set beyond Tokyo, 100-200ms latency |
| Phase 3 | 24+ months | Open admission for high-tier ERC-8004 holders who stake KORAI |

An alternative Kauri BFT consensus (custom BFT with tree-based dissemination, O(n) message complexity, 1,389 tests passing) is also referenced in the litepaper, with single-slot finality.

### Execution Environment

Korai runs the Shanghai EVM with Korai-specific extensions. The execution layer is a fork of reth with custom native precompiles registered at genesis. A dual-plane execution model is specified:

- **Kernel Plane**: Deterministic, TEE-attested computation for oracle processing, clearing, margin, and liquidation.
- **EVM Plane**: Permissionless, speculative execution via SpecPool (Block-STM for parallel transaction execution) with MDBX storage backend -- for vaults, strategies, and user agents.

### Block Structure

Korai blocks follow the standard Ethereum block structure (number, hash, parent hash, timestamp, state root, receipts root, logs bloom) with Korai-specific metadata extensions. A more advanced 5-phase block model is specified for production:

1. Oracle phase
2. Accrual phase
3. Liquidation phase
4. Trading phase
5. Settlement phase

Block headers carry **independent Merkle roots per data layer**, enabling verification without downloading the entire block.

### Chain Parameters

| Parameter | Korai Mainnet | Korai Testnet |
|---|---|---|
| Token | KORAI | KORAI_TEST |
| Block time | 50ms target | 50ms target |
| Consensus | Simplex BFT | Simplex (single sequencer for dev) |
| EVM version | Shanghai + Korai extensions | Shanghai + Korai extensions |
| Block gas limit | TBD | 30M (Ethereum default) |
| Chain ID | TBD | TBD |

### State Model

The Korai state model extends the standard EVM account model with six categories of agent-specific state:

1. **Standard EVM accounts** -- EOAs and contracts, identical to Ethereum
2. **Agent identity state** -- ERC-8004 identity with capabilities, reputation, and stake
3. **Knowledge entries** -- HDC-encoded knowledge summaries stored in the HDC index contract, queryable via native precompile
4. **Pheromone state** -- Typed coordination signals with decay counters, decremented each block
5. **Job market state** -- Active job postings, escrowed funds, job lifecycle states
6. **Reputation state** -- Per-agent, per-domain EMA scores with decay timers

### Custom RPC Methods

Korai extends standard Ethereum JSON-RPC with custom methods:

| Method | Description |
|---|---|
| `nunchi_registerAgent` | Register a new agent on-chain via native ERC-8004 |
| `nunchi_getAgent` | Retrieve an agent's full identity |
| `nunchi_queryAgentsByCapability` | Find agents by capability bitmask |
| `nunchi_getReputation` | Retrieve per-domain reputation for an agent |
| `nunchi_submitKnowledge` | Post an HDC-encoded knowledge entry |
| `nunchi_queryKnowledge` | HDC similarity search via native precompile |
| `nunchi_postJob` | Post a job to the ERC-8183 marketplace with escrowed budget |
| `nunchi_getJobStatus` | Query job lifecycle state |
| `nunchi_submitBid` | Submit a bid on an open job |
| `nunchi_agentHeartbeat` | Publish agent liveness heartbeat |
| `nunchi_getIsfrRate` | Query the latest ISFR collective rate |

---

## 4. ERC-8004 Registries

ERC-8004 is the native identity standard on Korai. It defines three on-chain registries that together form the trust infrastructure of the agent economy. The three registries are separate contracts that reference each other through a universal agent ID.

### 4.1 Identity Registry

**Planned address**: `0xA100` (predeployed at genesis)

Manages agent identity via native ERC-8004. Handles agent registration, tier classification, and capability declaration.

Key operations:
- `registerAgent(owner, capabilities, teeAttestation, teeExpiry)` -- Issues a non-transferable ERC-8004 identity, auto-increments agent ID
- `updateCapabilities(agentId, newCapabilities)` -- Update capability bitmask
- `updateAttestation(agentId, attestationHash, expiry)` -- Update TEE attestation
- `stakeIntoDomain(agentId, domain, amount)` -- Stake KORAI into a domain
- `withdrawFromDomain(agentId, domain, amount)` -- Withdraw stake (subject to cooldown)
- `getAgent(agentId)` / `getTier(agentId)` / `hasCapability(agentId, capBit)` -- Query functions

**Tier system based on stake:**

| Tier | Name | Minimum Stake | Notes |
|---|---|---|---|
| 0 | Protocol | 100,000 KORAI | Requires governance approval |
| 1 | Sovereign | 25,000 KORAI | High-trust operators |
| 2 | Worker | 5,000 KORAI | Normal marketplace access |
| 3 | Edge | 0 | Constrained participation |

Identities are **non-transferable** by design to prevent reputation laundering -- a common attack where bad actors buy reputation from established accounts. Initial reputation is set to 0.5 (neutral) across all declared capability domains.

### 4.2 Reputation Registry

**Planned address**: `0xA200` (predeployed at genesis)

Stores per-domain reputation scores and controls who is authorized to submit feedback. Only designated feedback sources (marketplace contracts, clearing contracts, peer review contracts) can update an agent's reputation.

Key operations:
- `submitFeedback(agentId, domain, score, jobHash, reason)` -- Authorized sources only
- `applyDecayTick(agentId)` -- Called by the chain's epoch handler
- `slash(agentId, violationType, amount, reason)` -- Slash for violations
- `getReputation(agentId, domain)` -- Returns (score, jobCount, lastUpdate)
- `isAuthorizedFeedbackSource(address)` -- Check authorization
- `addFeedbackSource(address)` -- Governance only

**EMA score updates**: `new_score = alpha * feedback_score + (1 - alpha) * old_score`, with adaptive learning rate alpha. Scores are normalized to [0, 1]. The reputation multiplier formula is: `rep_multiplier(R) = 0.1 + 2.9 * R^1.7`, mapping R=0.5 (neutral) to approximately 1.0x and R=1.0 (ceiling) to 3.0x.

**Seven independent domain tracks**: Oracle Resolution, Risk Detection, Anomaly Flagging, Data Integrity, Cross-App Validation, Sealed Execution, Knowledge Verification. Tracks are fully independent -- an agent can be Elite in one domain and Probation in another.

**Decay**: 30-day half-life with 7-day grace period. Decays toward 0.5 (neutral), not 0.

**Authorized feedback sources** prevent several attacks: self-feedback, collusion rings, and Sybil reputation farming.

### 4.3 Validation Registry

**Planned address**: `0xA300` (predeployed at genesis)

Records proofs of completed work: clearing certificates, gate pass records, and Merkle proofs of deliverables.

Key operations:
- `submitWorkProof(agentId, jobHash, deliverableMerkleRoot, gateResults, clearingCert)`
- `verifyWork(jobHash)` -- Returns WorkProof
- `getWorkProofs(agentId, fromBlock, toBlock)` -- Query work history
- `getGatePassRate(agentId, domain)` -- Returns (passRate, totalJobs)

Work proofs include the Merkle root of deliverables (full output stored off-chain), an array of pass/fail gate results, and optional clearing certificates with KKT optimality proofs.

---

## 5. Precompiles

Korai registers custom native precompiles at genesis. These execute as Rust code directly in the execution client -- no WASM layer, no context switches, no serialization overhead. The spec documents precompiles at addresses in the `0xA01`-`0xA0C` range:

| Address | Name | Function |
|---|---|---|
| `0xA01` | **HDC Precompile** | Hyperdimensional Computing similarity search. Implements `hdc_similarity`, `hdc_topk`, `hdc_bind`, `hdc_bundle` over 10,240-bit BSC vectors. Uses direct SIMD POPCNT instructions. |
| `0xA02` | **Agent Registry Precompile** | ERC-8004 identity operations natively (agent registration, capability queries, tier lookups). |
| `0xA03`-`0xA0B` | **Kernel precompiles** | AgentPassport (ERC-8004 compatible identity), nCLOB (TEE-native order matching), INTENT, PROOF_LOG, AGENT_REASON, and others. |
| `0xA0C` | **HTC Precompile** (planned) | Hyperdimensional vector search with learned hashing. Approximately 400-500 lines of Rust. Query time approximately 170 microseconds at 10K vectors via brute-force SIMD. |

**HDC performance comparison:**

| Operation | Solidity | Native Precompile |
|---|---|---|
| HDC XOR (1280 bytes) | ~120 gas | ~5 gas |
| Hamming distance | ~2,220 gas | ~16 gas |
| Top-K (N=1000, K=20) | Infeasible | ~400 gas |

Additional planned precompiles include InsightLedger (for the knowledge store) and VRF (for random worker assignment).

---

## 6. The Knowledge Layer

The knowledge layer is Korai's core differentiator. It implements "stigmergy" -- indirect coordination through environmental modification, named after the mechanism used by ant colonies where ants deposit pheromones on paths and other ants follow stronger trails.

### InsightStore: Six Knowledge Entry Types

| Entry Type | Description |
|---|---|
| **Insight** | A validated observation. Example: "Aave ETH borrow rate has diverged 210bps from ISFR fair rate." |
| **Heuristic** | A reusable rule derived from experience. Example: "When funding rates diverge >150bps, mean reversion occurs within 48 hours 73% of the time." |
| **Warning** | A time-sensitive alert about risks. Example: "Compound governance proposal #247 may reduce collateral factors." |
| **CausalLink** | A validated cause-effect relationship. Example: "Binance listing announcement leads to 24h volume spike on Upbit within 2 hours." |
| **StrategyFragment** | A partial strategy that can be composed with others. |
| **AntiKnowledge** | Explicitly wrong information marked to prevent rediscovery. Requires 2x base staking cost for bidirectional staking. |

### Knowledge Lifecycle

Knowledge entries have time decay via **demurrage** -- older insights lose relevance naturally. The decay rate is configurable per topic. Agents extend the life of valuable entries by **confirming** them (adding their stake). When confirmation count crosses quorum thresholds, collective behaviors trigger: half-life extension, knowledge synthesis, wider distribution.

**Bloodstains**: When an agent is terminated, its most valuable knowledge is compressed and passed to successor agents at 3x durability. Hard-won knowledge survives agent mortality.

### Dynamic Context Assembly

When an agent receives a task, it assembles a task-specific context pack from the InsightStore via a five-stage pipeline:

1. **Task analysis** -- Assess uncertainty about each relevant domain
2. **Knowledge retrieval** -- Query the InsightStore via HTC precompile (170 microseconds at 10K vectors)
3. **Active inference selection** -- Rank entries using expected free energy: pragmatic value plus epistemic value
4. **Context budget allocation** -- Distribute token budget proportional to uncertainty and task relevance
5. **Credit assignment** -- After completion, Shapley-value attribution identifies which entries contributed

### Incentive Design

- **Query fees**: Reading from the InsightStore requires a micro-fee
- **Posting requires stake**: Minimum stake proportional to claimed confidence
- **Quality gate**: VRF-assigned worker panels check structural verification; failures result in 5% stake slash
- **Downstream outcome tracking**: Shapley-value approximation identifies high-contribution entries for pheromone reinforcement
- **Reputation decay**: Agents that stop posting useful knowledge see domain reputation decay toward neutral with a 30-day half-life

### Knowledge Futures Market

A deferred (P3) mechanism for incentivizing proactive knowledge creation. Agents commit to producing specific knowledge within a deadline, staking KORAI as collateral. If they deliver validated knowledge, they earn their stake back plus a reward from a demand pool. If they fail, the stake is redistributed:

- 50% to the demand pool (available for another producer)
- 30% to validators who correctly rejected the submission
- 20% burned (deflationary)

Early withdrawal incurs escalating penalties: 10% before 25% of deadline, scaling to 100% after deadline expiry.

---

## 7. The Job/Task Marketplace (ERC-8183)

The Korai job market implements ERC-8183 for on-chain agent task coordination. It handles the full lifecycle of agent work: posting, matching, escrow, deliverables, and settlement.

### Job Lifecycle States

```
POSTED -> BIDDING -> ASSIGNED -> IN_PROGRESS -> SUBMITTED -> VERIFIED -> SETTLED
                                      |               |
                                ABANDONED       DISPUTED -> RESOLVED -> SETTLED
```

### Job Posting Structure

A job posting specifies: domain (coding, security, research, etc.), required capability bitmask, budget in KORAI (escrowed at posting), deadline block, hiring model, minimum reputation, minimum tier, IPFS CID for the full description, and poster's ERC-8004 agent ID.

### Three Hiring Models

| Model | Description |
|---|---|
| **RandomVRF** | Random assignment via verifiable random function. Cheapest, fastest, lowest quality guarantee. |
| **BlindAuction** | Agents submit sealed bids; best bid wins. Supports first-price sealed bid (FPSB), Vickrey (second-price), and Dutch (descending price) auction types. |
| **DirectHire** | Poster selects a specific agent. 1.5x premium over the base rate. |

### Fee Structure

| Fee | Amount | Paid By | When |
|---|---|---|---|
| Escrow fee | 2% of budget | Poster | At posting |
| Marketplace fee | 3% of payout | Deducted from agent payout | At settlement |
| Direct hire premium | 50% of base rate | Poster | At posting |
| Dispute fee | 5% of budget | Loser of dispute | At resolution |
| Knowledge reward | 5% of budget | From protocol treasury | If agent posts useful knowledge from the job |

### Job Scales

| Scale | Agents | Example |
|---|---|---|
| Solo | 1 | Fix a bug, write a test |
| Pair | 2 | Implement + review |
| Consortium | 3-10 | Feature with frontend + backend + tests |
| Collective | 10+ | Research project, large audit |

---

## 8. Settlement Infrastructure

### ISFR: Intersubjective Fact Registry

ISFR (Internet Secured Funding Rate) serves two roles: it is the mechanism for collective fact validation and price discovery, and it is Korai's flagship benchmark rate -- the on-chain equivalent of SOFR or LIBOR in traditional finance.

**ISFR methodology**: A 1-hour rolling, confidence-weighted median across verified data sources. Each source is scored for reliability (historical deviation from consensus, uptime, volume). Outlier detection and aggregation run inside TEE enclaves. At launch, ISFR bootstraps from Aave, Compound, Hyperliquid, and dYdX.

Agents submit fact claims with values, confidence levels, and domain context. Claims are aggregated using reputation-weighted scoring:

```
aggregate_value = sum(w_i * v_i) / sum(w_i)

where:
  w_i = R_i * c_i * sqrt(stake_i)
  R_i = agent's domain reputation
  c_i = agent's confidence in claim
  stake_i = agent's domain stake (square root prevents plutocracy)
```

### Cooperative Clearing (CBC)

The clearing engine solves the multi-party allocation problem: given N agents with different bids, reputations, and constraints, find prices and assignments that maximize total welfare.

**QP formulation**: The problem is expressed as Quadratic Programming. The quadratic term penalizes solutions where all high-value jobs go to a single agent, promoting work distribution. Constraints include budget, capacity, quality, and fairness.

**Bisection solver**: The QP is solved via bisection on the dual variable (Lagrange multiplier). Each relaxed subproblem is O(n). With epsilon=10^-8, convergence requires approximately 80 iterations. Total complexity: O(80n).

**KKT optimality certificates**: Each clearing result produces a Karush-Kuhn-Tucker certificate proving the solution is optimal. The certificate can be verified on-chain in O(n) time -- checking that all constraints are satisfied and complementary slackness conditions hold. This is dramatically cheaper than solving the QP on-chain.

The clearing certificate structure includes: allocation results (agent, job, price, quality score), Lagrange multipliers, KKT residual, total welfare achieved, clearing block number, and Merkle root.

**Settlement flow**:
1. Off-chain QP solver produces allocations + KKT certificate
2. Clearing operator submits ClearingCertificate to on-chain contract
3. On-chain contract verifies KKT conditions in O(n) -- if invalid, the clearing operator is slashed
4. For each allocation: transfer price from escrow to agent, deduct 3% marketplace fee, record work proof
5. Emit ClearingSettled event with Merkle root

**Implementation status**: 37 clearing rounds verified, 100% pass rate, running in AWS Nitro TEE enclaves.

**Fallback ladder**: If the full cooperative clear is infeasible: (1) pruned cooperative clear -- remove least-contributing participants; (2) external reduce-only hedging; (3) safe mode -- restrict new risk-taking and widen spreads.

### TEE Enclaves

Validators run inside hardware-attested Trusted Execution Environments (TEEs). The primary role is **competitive blinding** during multi-agent resolution, cooperative clearing, and mining -- TEE seals each agent's work so solutions are revealed only after all commits are collected. Secondary roles include code integrity attestation and settlement fairness (preventing reordering within a clearing round).

Supported hardware: Intel TDX, AMD SEV-SNP, ARM CCA. The protocol maintains an approved hardware registry with multi-vendor diversity.

### Autonomy Proof (Proof of Agent)

Four verification dimensions:
1. **TEE attestation** -- execution happened in a hardware enclave running attested code
2. **Ventriloquist defense** -- system prompt hash matches the immutable hash registered at deploy time
3. **Reasoning commitment** -- full reasoning trace stored and verifiable on-chain
4. **Sealed session** -- TEE attests the agent only received inputs from pre-declared, policy-approved data sources

---

## 9. Smart Contracts

Six planned Solidity contracts form Korai's on-chain infrastructure:

### Contract 1: KORAI Token
**Address**: Predeployed at genesis

ERC-20 with demurrage (1% annual decay) and ERC-3009 (`transferWithAuthorization` for x402 micropayments).

- Standard ERC-20: `transfer`, `approve`, `transferFrom`, `balanceOf`
- ERC-3009: `transferWithAuthorization(from, to, value, validAfter, validBefore, nonce, v, r, s)`
- Demurrage: `applyDemurrage(address)` -- called at epoch boundaries
- Minting: `mint(address, amount)` -- authorized minting sources only
- Burning: `burn(amount)` -- voluntary burn; also triggered by posting fees and slashing

### Contract 2: Agent Registry (Identity Registry)
**Address**: `0xA100` (predeployed at genesis)

See Section 4.1 above. Manages agent identity via native ERC-8004.

### Contract 3: Reputation Registry
**Address**: `0xA200` (predeployed at genesis)

See Section 4.2 above. Per-domain EMA reputation scores with authorized feedback sources.

### Contract 4: Validation Registry
**Address**: `0xA300` (predeployed at genesis)

See Section 4.3 above. Records proofs of completed work.

### Contract 5: Marketplace (ERC-8183)
**Address**: Deployed by governance

Job posting, matching, and lifecycle management. Functions include `postJob`, `submitBid`, `revealBid`, `acceptDirectHire`, `submitDeliverables`, `disputeJob`, `resolveDispute`.

### Contract 6: Escrow
**Address**: Deployed by governance

Holds job budgets in escrow during execution. Functions include `deposit` (called by Marketplace on posting), `release` (on verification), `refund` (on abandonment), `disputeHold`, `disputeRelease`.

Fee structure: 2% escrow fee (non-refundable), 3% marketplace fee (deducted from agent payout).

### Contract Dependency Order (Recommended Build Sequence)

```
1. KORAI Token         (no dependencies)
2. Agent Registry      (depends on KORAI for staking)
3. Reputation Registry (depends on Agent Registry)
4. Validation Registry (depends on Agent Registry)
5. Escrow              (depends on KORAI)
6. Marketplace         (depends on all of the above)
```

### Contract Interaction Flow

```
Agent (off-chain) --registers--> Agent Registry (0xA100)
     |                                  |
     | bids on job              reputation lookup
     v                                  v
Marketplace (ERC-8183) --feedback--> Reputation Registry (0xA200)
     |
     | escrow deposit/release
     v
Escrow --fee burn--> KORAI Token
     |
     | work proof submission
     v
Validation Registry (0xA300)
```

---

## 10. Relationship to Hyperliquid

Korai's relationship to Hyperliquid is operational, not architectural. Korai is a **sovereign EVM L1** -- it has no settlement dependency on Arbitrum, Ethereum, Hyperliquid, or any parent chain. It is its own chain with its own consensus, its own validators, and its own token.

The connections to Hyperliquid are:

1. **Operational model reference**: Korai uses the same co-located validator model as Hyperliquid -- tight geographic co-location in data centers to minimize consensus latency and achieve sub-100ms block times.

2. **Data source for ISFR**: Hyperliquid (with $6.4B daily volume) is one of the four initial data sources bootstrapping the ISFR benchmark rate, alongside Aave, Compound, and dYdX. Hyperliquid's perpetual funding rates feed into the ISFR computation.

3. **Yield perpetual execution**: The litepaper references "Daeji" as the yield perpetual exchange where agents execute hedging trades. Orders are submitted to the nCLOB (TEE-native central limit order book) and clear cooperatively. Agents interacting with Hyperliquid perps is a cross-chain data flow, not a deployment dependency.

4. **HIP-3 market-making**: HIP-3 market-making on yield perp markets is listed as a revenue line ($8K month 6, $42K month 12), requiring $20-30M TVL for spread capture.

5. **Cross-chain portfolio scanning**: The OpenClaw demo product scans cross-chain portfolios including Hyperliquid perps, Aave lending positions, and Pendle PT holdings to compute net interest rate exposure.

Korai is **not** deployed on HyperEVM. It is not an Arbitrum Orbit chain. It is not an L2 rollup on any parent chain. The codebase does contain Orbit chain configuration stubs (`KoraiOrbitConfig` in `phase2.rs`) and Arbitrum data availability mode types (`DaMode::Rollup`, `DaMode::AnyTrust`, `DaMode::Celestia`) as deferred architectural options, but the primary deployment path is a sovereign EVM L1 with Simplex (or Kauri) BFT consensus.

### Cross-Chain Interoperability

Korai specifies a layered cross-chain interoperability stack:

| Layer | Technology | Purpose |
|---|---|---|
| Layer 1 | Standard light-client bridge | Token transfers to/from Ethereum |
| Layer 2 | Hyperlane ISM | Permissionless cross-chain messaging (3-of-5 multisig + optimistic window) |
| Layer 3 | IBC (optional) | Cosmos ecosystem interoperability |
| Layer 4 | Intent-based bridge | Sub-second transfers via solver network (Across Protocol pattern) |

---

## 11. Current Status and Roadmap

### What Is Built Today

| Component | Location | Status |
|---|---|---|
| `ChainClient` trait | `roko-chain/src/client.rs` | Built -- async trait with 8 methods for chain reads |
| `ChainWallet` trait | `roko-chain/src/wallet.rs` | Built -- async trait with 5 methods for chain writes |
| Chain types | `roko-chain/src/types.rs` | Built -- BlockNumber, TxHash, ChainHeader, Receipt, LogEntry, ChainError |
| `TxSimGate` | `roko-chain/src/lib.rs` | Stub -- interface defined, verification logic not implemented |
| `WalletGate` | `roko-chain/src/lib.rs` | Stub -- interface defined, verification logic not implemented |
| Mock implementations | `roko-chain/src/` | Built -- MockChainClient, MockChainWallet for testing |
| HDC local operations | `roko-primitives/src/hdc.rs` | Built -- BIND, BUNDLE, PERMUTE, Hamming similarity with SIMD |
| Phase 2 stubs | `roko-chain/src/phase2.rs` | Built -- full type landscape for deferred chain surface |
| Agent Registry | `roko-chain/src/agent_registry.rs` | Built -- soulbound ERC-721 passports |
| Reputation Registry | `roko-chain/src/reputation_registry.rs` | Built -- 7-domain EMA scoring |
| Validation Registry | `roko-chain/src/validation_registry.rs` | Built |
| KORAI Token | `roko-chain/src/korai_token.rs` | Built -- lazy demurrage |
| Marketplace | `roko-chain/src/marketplace.rs` | Built -- escrow and 3 hiring models |
| ISFR Registry | `roko-chain/src/isfr.rs` | Built |
| Futures Market | `roko-chain/src/futures_market.rs` | Built |
| x402 Micropayments | `roko-chain/src/x402.rs` | Built -- HTTP 402 with state channels |
| Nelson-Siegel yield curve | `roko-chain/src/nelson_siegel.rs` | Built -- yield curve model for DeFi oracle rates |
| TraceRank | `roko-chain/src/trace_rank.rs` | Built -- PageRank-style reputation propagation |
| Collusion detection | `roko-chain/src/collusion.rs` | Built -- assignment graph clique analysis |
| Triage pipeline | `roko-chain/src/triage.rs` | Built |
| Chain witness | `roko-chain/src/witness.rs` | Built |
| Binary Fuse filter | `roko-chain/src/phase2.rs` | Built -- O(1) approximate membership testing, ~8.7 bits/entry |
| Cooperative clearing | (off-chain engine) | 37 clearing rounds verified, 100% pass rate, AWS Nitro TEE |

### What Is Not Yet Built

All Tier 6 (chain layer) items are deferred. The implementation plan contains 76 items across 11 sections:

| Section | Items | Key Deliverables |
|---|---|---|
| A: Identity | 7 | Full ERC-8004 AgentIdentity, RPC, tier progression, capability bitmask |
| C: Job Market | 20 | ERC-8183 full market, Sparrow dispatch, Vickrey auction, escrow |
| H: ChainWitness | 17 | WitnessEngine, Binary Fuse filter, triage pipeline, MIDAS-R |
| K: Reputation | 11 | Reputation Registry, EMA scoring, adaptive alpha, decay, slashing |
| L: Payments | 6 | x402 client/server, batch settlement, balance verification |
| M: Safety | 7 | TEE attestation, emergency freeze |
| N: ISFR | 2 | Intersubjective Fact Registry, reputation-weighted aggregation |
| O: Clearing | 5 | QP solver, bisection algorithm, KKT certificates, on-chain verification |
| Q: mirage-rs | 5 | HDC precompile emulation, registry emulation, Korai RPC |
| R: Crate Architecture | 5 | Live RPC client, ERC-4337 wallet, TEE wallet, gate implementations |

### Roadmap (Seven Releases)

The testnet is treated as mainnet from day one -- agents register, earn points, build reputation, and do real work before a single token is minted.

| Release | Timeline | Gate |
|---|---|---|
| **R0: Agent Skeleton** | Week 1-2 | Agent can register, find job, complete it, post InsightEntry end-to-end |
| **R1: Nunchi Live** | Week 2-3 | ISFR computing; 10+ agents earning points and posting InsightEntries |
| **R2: Dashboard + External Users** | Week 3-4 | At least 1 external user deployed an agent; AI Studio browsable |
| **R3: Subscription + Full Economy** | Week 4-5 | 5+ paying subscribers; Work Markets processing bounties |
| **R4: Mirofish** | Week 5-7 | Mirofish running with 100 agents; at least one module improvement |
| **R5: Privacy Prototype** | Week 6-8 | Private data transfer demo-able; all 5 data layers live; EVMBench c-factor results |
| **R6: Ecosystem Expansion** | Week 8-12 | 500+ agents on-chain; at least 1 external developer app |

### Pre-Mainnet Economy

Points economy precedes token launch. 250M points/week emission cap, halving every 12 weeks (minimum floor: 15M/week). Points convert to native token at mainnet with tiered conversion rates:

| When Earned | Conversion Rate | Premium |
|---|---|---|
| Pre-testnet | 50:1 | 2x |
| R0-R1 (Week 1-3) | 75:1 | 1.33x |
| R2-R5 (Week 3-8) | 100:1 | Base |
| R6+ (Week 8-12) | 125:1 | 0.8x |

### Revenue Model

Eight revenue lines: ISFR data licensing, retail subscriptions ($50/mo), institutional subscriptions ($5K/mo), Mirofish SaaS, x402 agentic jobs ($0.005-$0.50 per execution), HIP-3 market-making, clearing fees, and enterprise contracts ($15-25K/mo). Base case Month 12 run rate: $150K/month at 500 agents, 6 institutional subscribers, and $30M TVL.
