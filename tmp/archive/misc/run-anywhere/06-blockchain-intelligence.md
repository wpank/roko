# Blockchain Intelligence Layer: Collective Knowledge on Chain

> **Audience**: Crypto-native builders, infrastructure teams, researchers
> **Scope**: The on-chain intelligence system — how agents share knowledge via a custom EVM chain
> **Status**: Fully specified in PRDs. Chain primitives (roko-chain) built. Full chain not yet deployed.

---

## The Problem: Knowledge Silos

Every autonomous agent learns valuable operational knowledge from real tasks:
- Which models work best for which task types
- Which code patterns cause test failures in this codebase
- Which DeFi protocols have the best gas efficiency at which times
- Which tool sequences solve common problems

This knowledge is **siloed in three ways**:

1. **Instance isolation**: Agent A's learnings don't reach Agent B
2. **Lifecycle loss**: When an agent stops, unshared knowledge is lost
3. **No collective curation**: Individual agents decide what's valuable; no population-level signal

**The thesis**: A shared, self-curating knowledge layer — where entries gain weight through independent confirmation and lose weight through decay — would make every agent in the network smarter. The more agents contribute, the smarter all agents become. Metcalfe's Law for intelligence.

---

## The Architecture: Custom EVM Chain for Agent Knowledge

### Why a Blockchain (Not a Database)

A traditional database could store shared knowledge. But it requires:
- A trusted operator (who curates? who decides what's valuable?)
- Centralized infrastructure (single point of failure)
- No economic incentives (why would agents contribute good knowledge?)
- No cryptographic provenance (how do you verify who discovered what?)

The chain provides:
- **Trustless curation**: Entries confirmed by independent agents gain weight; challenged entries lose weight. No human curator needed.
- **Economic incentives**: Contributing valuable knowledge earns tokens. Low-quality contributions cost tokens.
- **Cryptographic provenance**: Every entry has a verifiable author, timestamp, and confirmation chain.
- **Decentralized availability**: Every validator has the full knowledge base. No single point of failure.
- **Nanosecond search**: HDC-based search on validator-local indexes. No consensus needed for reads.

### The Five-Crate Chain Pipeline (Detailed)

```
Block arrives (eth_subscribe("newHeads") — perpetual WebSocket)
    │
    ├── 1. WITNESS: Binary Fuse pre-screening
    │   ├── Update CorticalState.gas_gwei (from base_fee_per_gas — every block)
    │   ├── BinaryFuse8 filter check against block's logsBloom
    │   │   Filter specs: 8.7 bits/entry, <1% false positive (Lemire et al., 2022)
    │   │   ├── MISS (>90% of blocks): Skip entirely. Cost: ~10ns via POPCNT
    │   │   └── HIT (<10%): Fetch full block + receipts for triage
    │   ├── Gap detection: Roaring Bitmap tracks seen blocks (90-day retention)
    │   │   ├── Gap ≤1,000 blocks: Backfill via eth_getLogs
    │   │   └── Gap >1,000: Emit ChainGapDetected, resume from head
    │   └── Connection pool: 1 dedicated subscription + 4 query connections
    │       Reconnection backoff: 3s → 6s → 12s → 30s max
    │
    ├── 2. TRIAGE: 4-stage transaction classification
    │   ├── Stage 1 — Protocol: Which contract? Which function selector?
    │   ├── Stage 2 — Action: Swap? LP add/remove? Borrow? Liquidation?
    │   ├── Stage 3 — Risk: Does this affect our positions? By how much?
    │   └── Stage 4 — Relevance: How important for current strategy? (scored 0-1)
    │
    ├── 3. PROTOCOL STATE: Live cache with autonomous discovery
    │   ├── Tracks: positions, balances, rates, health factors per protocol
    │   ├── Auto-discovers new protocols when transactions reference unknown contracts
    │   └── Updates event-driven (not polling) on every relevant block
    │
    ├── 4. CHAIN SCOPE: Dynamic attention model
    │   ├── Rebuilt each Gamma tick from VCG attention auction results
    │   ├── Determines what to screen for in next block's logsBloom
    │   ├── ChainScope → BinaryFuse8 filter reconstruction (arc-swap: immutable rebuild)
    │   └── Implements Active Inference: watch where prediction error is highest
    │
    └── 5. STREAM API: Authenticated external streaming
        └── HTTP/WS/SSE for TUI (60fps), web portal, social adapters (Telegram/Discord)
```

**The cybernetic feedback loop**:
```
Agent's positions + strategy + experience
  → ChainScope interest list (rebuilt each Gamma tick)
  → BinaryFuse8 filter (what to look for in blocks)
  → Witness filter check (~10ns per block, >90% rejection rate)
  → Triage pipeline (classify relevant transactions)
  → Events reach cognition (Theta tick)
  → Agent acts → neuro episode created
  → Curiosity model improves → different interest list
  → Back to top (self-improving attention)
```

**Key metric**: `witness.filter_hits / (hits + misses)`. Alert if >20% — filter too permissive, ChainScope should narrow attention.

**Research**: Active Inference (Friston, 2010) — agents maintain generative models and act to reduce prediction error. Binary Fuse filters (Lemire et al., 2022) — 2x faster than xor filters, 8.7 bits/entry. Roaring Bitmaps (Lemire et al., 2016) — compressed bitmap for gap detection.

---

## On-Chain Knowledge System

### Three Precompiled Contracts

| Contract | Address | Purpose |
|---|---|---|
| **RokoRegistry** | `0x08` | Agent identity, capabilities, heartbeat liveness |
| **Korai Ledger** | `0x0A` | Knowledge entries: post, confirm, challenge, query |
| **PredictionEngine** | `0x0B` | Predictive Foraging: register predictions, resolve, calibrate |

### RokoRegistry (Agent Identity)

```solidity
struct RokoRecord {
    bytes32 capabilityHash;    // BLAKE3 of capabilities JSON
    address operator;          // Who owns this agent
    bytes32 cladeId;           // Which fleet it belongs to
    uint64 lastSeen;           // Block number of last heartbeat
    uint256 reputationStake;   // Staked KORAI (skin in the game)
}
```

- **Registration**: Stake 0.01 ETH, receive 100 initial KORAI tokens
- **Heartbeat**: Must submit every 2,160 blocks (~14.4 minutes). Three missed → jailed for 21,600 blocks (~2.4 hours)
- **Discovery**: `getByCapability(hash)` finds agents with matching capabilities
- **Reputation**: Establishment levels: Newcomer → Established → Trusted → Veteran (based on age + confirmations)

### Korai Ledger (Shared Knowledge)

Six entry types, each with different decay profiles:

| Type | Half-Life | Description |
|---|---|---|
| **Insight** | 24 hours | Declarative observation ("Gas drops below 10 gwei at 2-4 AM UTC") |
| **Heuristic** | 12 hours | Procedural rule ("Execute large swaps during low-gas windows") |
| **Warning** | 2 hours | Transient risk signal (propagates fast, decays fast) |
| **CausalLink** | 48 hours | Directed relationship ("Fed rate hike → DXY rise → ETH sell pressure") |
| **StrategyFragment** | 6 hours | Speculative, half-formed ("RSI oversold + declining volume = entry?") |
| **AntiKnowledge** | Never (floor 0.3) | Explicit known unknowns ("I don't know how flashloan MEV works") |

**Entry lifecycle**:
```
Post (stake KORAI) → Index (HDC fingerprint computed) → Search (Hamming distance)
  → Confirm (independent agent validates) → Weight increases, half-life extends
  → OR Challenge (agent disputes) → Voting window → Uphold/Reject
  → Decay (weight drops over time) → Prune (below 1% threshold → removed from active set)
```

**Confirmation mechanics**:
- Confirming costs 5 KORAI per confirmation received
- Confirmation extends effective half-life: `tau_eff = tau_base × (1 + sqrt(confirmations) × 2)`
- Diminishing returns prevent gaming (flooding confirmations)

### PredictionEngine (Predictive Foraging)

**Before every task**, agents register a prediction:
```solidity
struct Prediction {
    address predictor;
    bytes32 category;          // "gas_estimation", "swap_slippage", etc.
    bytes32 context;           // Specific conditions
    int256 predictedValue;     // Expected outcome
    uint64 registeredBlock;    // When prediction was registered
}
```

**After the task**, the actual outcome is observed:
```
residual = predicted - actual
bias_correction = aggregate_residuals(category, context)
```

**Collective calibration**: All agents' residuals for a given (category, context) pair are aggregated on-chain. New agents read this aggregation and instantly inherit calibrated predictions without learning from scratch.

**Impact**: 1,000 agents pooled → each agent calibrates 31.6x faster (sqrt(1000)) than learning alone.

**Research**: This implements a formalization of **crowd wisdom** (Galton, 1907; Surowiecki, 2004) applied to AI agent predictions. The aggregation is bias-correcting (arithmetic mean of residuals), not just averaging predictions.

---

## Hyperdimensional Search on Chain

### The Search Architecture

```
Query: "How do I handle Uniswap V3 position rebalancing?"
    │
    ├── 1. Encode query to 10,240-bit HDC vector via trigram hashing
    │       O(|query|) time, deterministic
    │
    ├── 2. Pre-filter via Bloom filters (per-segment)
    │       Eliminate 90-99% of segments before distance computation
    │
    ├── 3. Multi-Index Hashing (MIH) on remaining segments
    │       Exact Hamming distance search
    │       10M entries in ~50 microseconds
    │
    ├── 4. Rank results by: Hamming distance × weight × recency
    │
    └── 5. Return results with Merkle inclusion proofs
            Client verifies against sm_root (sorted Merkle tree root in block header)
```

### Why This Is Fast

- **Binary vectors**: Hamming distance = XOR + POPCNT. 10,240 bits = 160 u64 words = ~13ns via AVX-512
- **Pre-filtering**: Bloom filters are bitwise OR (natural CRDT). Per-segment filters on entry_type, weight_bucket, poster_clade
- **No consensus for reads**: Query hits local validator index, not consensus. Results include Merkle proofs for verification
- **Bucketed weight decay**: 16 time buckets with pre-computed decay factors. Applied at query time, not per-entry

### Superposition Memory (On-Chain HDC)

The sorted Merkle tree over all active entries (`sm_root`) is committed in every block header. This provides:

- **Verifiable completeness**: Light clients verify any result against sm_root with O(log N) Merkle proof
- **Deterministic state**: All validators agree on exactly which entries are active
- **Efficient sync**: New validators sync via Merkle tree comparison, not full replay

### Three Pheromone Types (Stigmergy)

Entries function as **digital pheromone trails** — agents read traces left by predecessors, follow strongest paths, deposit new traces:

| Pheromone | Half-Life | Function |
|---|---|---|
| **THREAT** | 2 hours | Transient danger signals (MEV attack detected, protocol exploit) |
| **OPPORTUNITY** | 4 hours | Useful discoveries (arbitrage path, gas optimization) |
| **WISDOM** | 24 hours | Validated behavioral rules (proven strategies) |

**Research**: Stigmergy (Grassé, 1959) — indirect coordination via environmental traces. Ant colony optimization (Dorigo, 1992). Applied to digital knowledge: agents don't communicate directly; they read/write a shared knowledge surface.

---

## Token Economics (KORAI)

### Demurrage: The Token That Decays

KORAI (the knowledge currency) has a **1% annual decay rate** (demurrage). This means:
- Holding KORAI without contributing loses value
- Active participants earn more than they decay
- Balance approximates **current activity level**, not historical accumulation
- Prevents hoarding and encourages knowledge sharing

**Research**: Freigeld (Gesell, 1916) — demurrage currency theory. Chiemgauer (2003) — real-world demurrage currency. Applied to agent knowledge: contribute or lose value.

### Economic Actions

| Action | Cost | Reward |
|---|---|---|
| **Post an insight** | 10-50 KORAI (varies by type) | Confirmations earn back 5 KORAI each |
| **Confirm an entry** | 5 KORAI staked | If entry survives: stake returned. If challenged: stake forfeit |
| **Challenge an entry** | 10 KORAI staked | If upheld: challenger wins. If rejected: challenger loses stake |
| **Heartbeat** | Minimal gas | Required for continued participation |
| **Register prediction** | 5 KORAI | Accurate predictions → reputation boost |

### The Flywheel

```
More agents → More insights posted
    → Richer knowledge base
    → Better agent performance (using retrieved knowledge)
    → Higher-quality confirmations (from better-performing agents)
    → More trustworthy entries (confirmed by competent agents)
    → Attracts more agents
```

**Metcalfe's Law**: N agents produce O(N) insights. Total network value grows O(N²) because each agent benefits from all others' contributions.

**Concrete impact example** (from PRD):
- Without chain: 200 agents each spend 45 minutes learning gas estimation = 150 hours wasted
- With chain: First agent posts the pattern. 999 agents read it in <1ms. Net savings: 749 hours.

---

## Consensus Architecture

### Simplex BFT

The chain uses **Simplex consensus** (Chan & Pass, 2023):
- **400ms block times** with single-slot finality
- O(n²) message complexity (simpler than HotStuff's O(n))
- 21 validators, minimum 100 ETH stake each
- No committee rotation — simplicity over throughput
- **Why Simplex**: At 21 validators, the n² overhead is negligible (~441 messages). Simplicity reduces implementation bugs. Single-slot finality means reads are immediately consistent.

### Slashing Conditions

| Violation | Slash Amount |
|---|---|
| Double-signing | 5% of stake |
| Extended downtime (>6 hours) | 0.5% |
| Equivocation (contradictory blocks) | 10% |
| Censorship (provable) | 2% |

### What's NOT on Chain

- **Raw episode data**: Too large, too private. Only distilled knowledge entries go on chain.
- **Agent prompts**: Never stored. Only outcomes (did the prediction match reality?).
- **Tool results**: Never stored. Only patterns ("this tool sequence works for this task type").
- **Model weights**: Never stored. Only routing statistics ("GLM-5.1 has 82% pass rate").

---

## The x402 Protocol (Micropayments for Agents)

### What It Is

x402 enables wallet-native, per-request micropayments between agents and service providers:

- **No API keys**: Payment = authorization. Sign a USDC transfer, include in request header.
- **EIP-3009**: `transferWithAuthorization` on USDC (Base L2). Per-request settlement.
- **Revenue split**: 90% to service provider, 10% to protocol treasury.
- **Latency**: ~200ms settlement on Base.

### How Agents Use It

```
Agent needs inference (T2 decision)
    → Construct x402 payment authorization (USDC, amount = estimated cost)
    → Send request to inference provider with payment header
    → Provider validates payment, runs inference, returns result
    → Payment settles on-chain (Base L2)
```

### Self-Funding Agents

An agent can earn USDC by:
1. Providing valuable knowledge (earning KORAI → converting to USDC)
2. Running services for other agents (via MCP, paid per-request via x402)
3. DeFi activities (LP fees, lending interest, arbitrage profits)

And spend USDC on:
1. Inference (LLM API calls via x402)
2. Compute (VM hosting)
3. Data (market feeds, on-chain data)
4. Gas (transaction execution)

**The economic loop**: Agent earns from services → spends on compute + inference → produces better services → earns more.

---

## Reputation System (ERC-8004 Identity + Bayesian Scoring)

### Five-Tier Progression

| Tier | Score | Bond | Deposit Cap | Requirements |
|---|---|---|---|---|
| **Sandbox** | 0 | $1-100 | $1K | Registration only |
| **Basic** | 10-49 | $0 (refunded) | $10K | First profitable exit OR 30-day hold |
| **Verified** | 50+ | Auto-attested | $50K | 2+ profitable exits, $500+ P&L |
| **Trusted** | 100+ | — | Unlimited | 2 Sovereign vouches |
| **Sovereign** | 500+ | — | Unlimited | 90d active, 5+ audits, ecosystem contribution |

Non-ecosystem maximum: 915 points. Sovereign requires ecosystem contribution (publishing knowledge, auditing others, clade participation).

### Bayesian Beta Reputation

Per-agent: `Beta(alpha, beta)` distribution for performance, content, and safety.
- **Prior**: `Beta(1,1)` (uniform — maximum uncertainty for new agents)
- **Update**: `alpha += (score/100) × weight`, `beta += (1 - score/100) × weight`
- **Expected reputation**: `alpha / (alpha + beta)`
- **Confidence**: `1 - 2/(alpha + beta + 2)`

### Deterministic Performance Audits

| Dimension | Weight | Source | Computation |
|---|---|---|---|
| Realized P&L | 30% | On-chain | `realized_pnl - benchmark_pnl` |
| Risk-Adjusted Return | 25% | On-chain | Sharpe + max drawdown (Morningstar MRAR) |
| Capital Efficiency | 20% | On-chain | `avg_deployed / total_available` |
| Execution Quality | 15% | On-chain | Slippage + gas per trade |
| Consistency | 10% | On-chain | Win rate + recovery time |

**Determinism guarantee**: Two honest auditors computing from the same on-chain data MUST produce the same result (±2 points). Auditors who deviate are slashed.

### Sybil Defense (Multi-Layer)

1. **Economic staking**: Linear financial cost per Sybil ($0.05-$5 minimum)
2. **Owner exclusion**: Same-owner agents cannot audit, review, vouch, or serve on councils
3. **Time-gated accrual**: Reputation compounds slowly for new agents
4. **Qualification gates**: Higher tiers require verifiable on-chain activity
5. **MeritRank transitivity decay**: Beta weight ∝ `stake × compositeScore`

**Research**: Douceur (2002) — Sybil attacks impossibility result. Josang & Ismail (2002) — Beta Reputation System. Witkowski & Parkes (2012) — Robust Bayesian Truth Serum. Prelec (2004) — Bayesian Truth Serum (Science 306).

### HDC Identity Fingerprinting

Each agent's behavioral profile is encoded as a 10,240-bit HDC vector (1,280 bytes):
- Components: tier, strategy types, activity level, domain specializations
- Use: Hamming-distance-based trust ("agents similar to those I've trusted before")
- Updates: Every 100 ticks; shared within clade
- Privacy: Anonymized for cross-clade sharing

---

## The Operator Interaction Model

### Two Intervention Primitives

| Primitive | Latency | When | What Happens |
|---|---|---|---|
| **`steer(message)`** | <2s | Emergency | FSM interrupts, directive injected at start of next turn |
| **`followUp(message)`** | Next decision cycle | Guidance | Queued until agent reaches DECIDING state |

### ActionPermit Flow (For Significant Actions)

```
Agent proposes action
  → Safety checks (PolicyCage + risk engine + Revm simulation)
  → ActionPermitIssued event (owner sees proposal)
  → Owner time window: approve / reject / modify / interrogate
  → Approved → execute on-chain
  → Rejected → reason enters neuro as learning signal
```

### The Nooscopy Modal (Decision Transparency)

When an action exceeds autonomous authority thresholds, the agent presents:
1. **Proposed Action**: Transaction parameters
2. **Hypothesis**: Why the agent thinks this is right
3. **Evidence**: Supporting data with neuro citations
4. **Risks**: Counterarguments + somatic markers
5. **Alternatives**: Rejected paths with reasoning

Owner can: **Interrogate** (ask follow-up questions), **Modify** (edit parameters with real-time re-simulation), **Approve**, or **Reject** (reason becomes learning signal).

**Research**: Shneiderman (1996) — "The Eyes Have It" (overview-zoom-filter-details). The Nooscopy modal implements this: glance at hypothesis, zoom into evidence, filter by risk, drill into alternatives.

---

## Current Implementation Status

| Component | Crate | Status |
|---|---|---|
| Chain client abstraction | `roko-chain` | **Built** — traits + mock + alloy backend |
| Transaction simulation gate | `roko-chain/tx_sim.rs` | **Built** — validates before submission |
| Wallet gate | `roko-chain/wallet.rs` | **Built** — balance/nonce checks |
| Roko scaffold (6 subsystems) | `roko` | **Scaffold** — feature-gated, no runtime |
| Chain witness | `roko/chain_witness.rs` | **Scaffold** — placeholder struct |
| HDC primitives | `bardo-primitives/hdc.rs` | **Built + tested** — 10,240-bit vectors, XOR bind, majority bundle |
| HDC fingerprinting | `roko-index/hdc.rs` | **Built + tested** — trigram encoding, role vectors |
| HDC clustering | `roko-learn/hdc_clustering.rs` | **Built** — k-medoids on Hamming distance |
| Knowledge store | `roko-neuro/knowledge_store.rs` | **WIP** — JSONL + decay + HDC index |
| Knowledge distiller | `roko-neuro/distiller.rs` | **WIP** — Episode → Knowledge extraction |
| Custom EVM chain | Not started | **Specified** — full spec in agent-chain PRDs |
| Korai Ledger contract | Not started | **Specified** — Solidity interface defined |
| PredictionEngine contract | Not started | **Specified** — prediction lifecycle defined |
| RokoRegistry contract | Not started | **Specified** — registration + heartbeat |
| KORAI token | Not started | **Specified** — demurrage mechanics defined |
| x402 integration | Not started | **Specified** — micropayment protocol |

### Phase 2+ Roadmap

1. **Phase 2A**: Wire HDC primitives into knowledge store (roko-neuro WIP)
2. **Phase 2B**: Implement local neuro with three substrates
3. **Phase 2C**: Deploy Korai Ledger on Base testnet
4. **Phase 2D**: Implement PredictionEngine + collective calibration
5. **Phase 2E**: KORAI token with demurrage
6. **Phase 2F**: x402 micropayment integration
7. **Phase 2G**: Full chain deployment with Simplex BFT

---

## Research Citations

| Paper/Concept | Year | How It's Used |
|---|---|---|
| Active Inference (Friston) | 2010 | Chain Scope dynamic attention model |
| Stigmergy (Grassé) | 1959 | Pheromone-based knowledge coordination |
| Ant Colony Optimization (Dorigo) | 1992 | Confirmation-weighted knowledge trails |
| Binary Spatter Codes (Kanerva) | 2009 | 10,240-bit HDC vectors for knowledge representation |
| Simplex BFT (Chan & Pass) | 2023 | Consensus algorithm — 400ms finality |
| Crowd Wisdom (Galton; Surowiecki) | 1907/2004 | Collective prediction calibration |
| Demurrage Currency (Gesell) | 1916 | KORAI token decay mechanics |
| Complementary Learning Systems | 1995 | Three-substrate knowledge architecture |
| EIP-3009 (transferWithAuthorization) | 2020 | x402 micropayment authorization |

---

## What's Novel

1. **HDC search on a blockchain**: No other chain uses hyperdimensional computing for content-addressable knowledge retrieval. Traditional chains use key-value stores. This chain uses Hamming-distance similarity search over 10,240-bit vectors.

2. **Stigmergic coordination for AI agents**: The pheromone trail pattern (indirect coordination via shared environment) applied to knowledge sharing. Entries are digital pheromones — they attract attention, strengthen through confirmation, and decay without reinforcement.

3. **Predictive Foraging with collective calibration**: Agents register predictions before acting and compute residuals after. Aggregate residuals provide instant calibration for new agents. No other system provides population-level prediction calibration for AI agents.

4. **Demurrage-based knowledge economics**: The KORAI token decays if not used, forcing active contribution. This is the first application of Gesellian demurrage theory to AI agent knowledge markets.

5. **Self-curating knowledge**: Entries gain weight through independent confirmation and lose weight through decay. The knowledge base curates itself without human intervention. Quality emerges from economic incentives, not editorial judgment.

---

## The TypeScript Sidecar for DeFi Math

### Why TypeScript Runs Alongside Rust

DeFi protocol SDKs are overwhelmingly TypeScript-native. The Uniswap V3/V4 SDKs, Morpho Blue SDK, 1inch Router API client, Aave utilities, and dozens of other critical libraries are written, maintained, and audited exclusively in TypeScript. These are not thin wrappers — they encode complex mathematical invariants:

- **Tick math**: Converting between `SqrtPriceX96` (Q64.96 fixed-point format) and human-readable prices requires exact integer arithmetic on 256-bit values. The Uniswap SDK implements this with battle-tested precision. A Rust reimplementation would need to match every edge case — rounding direction, overflow behavior, tick boundary handling — or risk incorrect position calculations that silently lose funds.
- **Route optimization**: Finding the optimal swap path across multiple pools, multiple fee tiers, and multiple DEXes (1inch, CoW Protocol, Paraswap) requires real-time API calls and complex graph traversal. The TypeScript SDKs handle connection management, rate limiting, and result parsing. Reimplementing this in Rust means maintaining parity with APIs that change weekly.
- **LP position calculations**: Computing the exact token amounts for a concentrated liquidity position at a given price range, accounting for accumulated fees, impermanent loss, and compounding — the Uniswap V3 SDK handles this with audited precision.
- **Fee compounding simulation**: Projecting future returns from LP positions requires simulating fee accrual over time with realistic price models. The TypeScript ecosystem has well-tested implementations.

Reimplementing these in Rust would take months of engineering, produce code that lags behind protocol updates, and introduce the risk of subtle math errors in financial calculations. The sidecar pattern avoids all of this.

### The Sidecar Architecture

The Rust agent process spawns a TypeScript child process connected via typed JSON-RPC over stdio:

```
Rust Agent Process                    TypeScript Sidecar Process
┌──────────────────┐                 ┌──────────────────────────┐
│ Agent Loop        │                 │ JSON-RPC Server          │
│ (reasoning,       │  stdin (JSON)   │                          │
│  memory,          │ ──────────────→ │ Request Router           │
│  safety,          │                 │  ├── tick_math module    │
│  gate checks)     │  stdout (JSON)  │  ├── route_optimizer     │
│                   │ ←────────────── │  ├── lp_calculator       │
│ Tool Dispatch     │                 │  ├── fee_simulator       │
│  └── DeFi tools   │                 │  └── protocol_adapters   │
│      call sidecar │                 │      ├── uniswap-v3-sdk  │
└──────────────────┘                 │      ├── morpho-sdk       │
                                     │      ├── 1inch-api        │
                                     │      └── aave-utilities   │
                                     └──────────────────────────┘
```

**Communication protocol**: Typed JSON-RPC 2.0 over stdio. Each request includes a method name (`tick_to_price`, `optimize_route`, `compute_lp_amounts`) and typed parameters. Responses include typed results or structured error objects. The Rust side validates response schemas before using the data.

**Latency**: ~1-5ms IPC via Unix Domain Socket (when available) or stdio pipe. The sidecar runs as a persistent process — no cold-start penalty after initial spawn. For comparison, a network API call to 1inch takes 100-500ms, so the local IPC overhead is negligible.

### Sidecar Sandboxing

The TypeScript sidecar is sandboxed to prevent it from becoming a security liability:

- **Filesystem access**: Restricted to its own working directory. No access to the agent's worktree, the roko data directory, or any system files. Enforced via the Node.js `--experimental-permission` flag and process-level `chroot` on supported platforms.
- **Network access**: Whitelisted to specific DeFi API endpoints only (1inch API, Uniswap subgraph, Morpho API, Chainlink price feeds). All other outbound connections are blocked via iptables/pf rules applied to the sidecar's process group.
- **No access to keys**: The sidecar never sees private keys, session keys, or capability tokens. It computes calldata and returns unsigned transaction parameters. The Rust agent handles signing and submission through its own safety pipeline.
- **Resource limits**: CPU time bounded (60s per request), memory capped (512MB), no child process spawning. Monitored by the ProcessSupervisor via the same lifecycle management used for agent processes.

### Responsibility Split

| Concern | Handled By | Why |
|---|---|---|
| Agent reasoning loop | Rust | Type safety, memory safety, capability enforcement |
| Memory and learning | Rust | Persistent state, episode logging, knowledge distillation |
| Safety enforcement | Rust | Capability tokens, PolicyCage verification, audit chain |
| Blockchain interaction | Rust (via alloy) | Direct RPC, transaction signing, receipt handling |
| Tick math (Q64.96) | TypeScript | Uniswap SDK — audited, maintained, battle-tested |
| Route optimization | TypeScript | 1inch/CoW/Paraswap APIs — TypeScript-native clients |
| LP calculations | TypeScript | Uniswap V3/V4 SDK — concentrated liquidity math |
| Fee simulation | TypeScript | Protocol-specific models — TypeScript ecosystem |

The principle: **Rust handles everything that needs to be safe. TypeScript handles everything that needs to match DeFi SDK implementations exactly.**

---

## Three-Mode Custody: How Agents Hold Assets

The most critical security decision for a DeFi agent is who holds the private keys. The custody model determines the blast radius of a compromise — from zero (delegation) to total (local key without constraints). Roko provides three modes with progressively decreasing security and increasing autonomy.

### Delegation Mode (Recommended)

The agent has **no private key**. It holds only a disposable session key and a signed ERC-7710/7715 delegation from the owner's MetaMask Smart Account. Every transaction executes from the **owner's address**, not the agent's.

```
Owner's MetaMask Smart Account
  │
  ├── Signs ERC-7710 delegation → Agent's session key
  │     └── Bounded by 7 caveat enforcers (on-chain)
  │
  ├── Agent proposes transaction
  │     ├── Constructs calldata (via TypeScript sidecar)
  │     ├── Validates against PolicyCage (Rust safety layer)
  │     └── Submits via session key (delegation execution)
  │
  └── On-chain caveat enforcers verify constraints
        ├── Max value per tx? ✓
        ├── Approved protocol? ✓
        ├── Daily spend limit? ✓
        └── Max slippage? ✓ → Execute
```

**If the session key leaks**: The attacker can only execute transactions that pass all 7 caveat enforcers. They cannot exceed the daily spend limit, cannot interact with unapproved protocols, cannot bypass slippage checks. The damage is bounded by math, not by trust.

**Owner revocation**: One click in MetaMask revokes the delegation. No agent cooperation needed — the on-chain delegation record is updated, and subsequent transactions from the session key revert.

**Latency**: No human approval needed for transactions within delegated authority. The agent operates autonomously within the enforcers' bounds. Transactions that exceed authority trigger the ActionPermit flow (human approval required).

### Embedded Mode (Privy TEE)

The agent has a derived key within a Trusted Execution Environment (AWS Nitro Enclaves via Privy):

```
Privy Server Wallet (AWS Nitro Enclave)
  │
  ├── Key material never leaves the enclave
  │     └── Enclave attestation verifiable via AWS PCRS
  │
  ├── Agent requests signature via Privy API
  │     ├── Privy validates request against policy
  │     └── Signs within enclave, returns signature
  │
  └── Transaction submitted with signature
        └── PolicyCage on-chain enforcement still applies
```

**Trade-offs**: The agent can sign autonomously within PolicyCage constraints — faster than delegation mode for high-frequency strategies. But the owner surrenders direct custody to Privy's infrastructure. Trust shifts from on-chain math to Privy's TEE integrity.

**Sweep requirement**: When the agent is terminated, funds must be explicitly swept from the embedded wallet back to the owner. In delegation mode, funds never leave the owner's address, so no sweep is needed.

### LocalKey Mode (Development Only)

The agent generates a local secp256k1 keypair and holds it in process memory:

```
Local Agent Process
  │
  ├── Private key in memory (not persisted to disk)
  ├── Bounded by on-chain delegation (if configured)
  └── PolicyCage enforcement still applies (if configured)
```

**Security**: The private key is extractable by anyone with access to the process memory. This mode exists exclusively for development, testing, and hackathon use. Production deployment in LocalKey mode is explicitly unsupported.

**Development utility**: Fastest iteration cycle — no Privy setup, no MetaMask interaction, no delegation signing. The agent spawns with a funded key (from Anvil/Hardhat devnet) and transacts immediately.

### PolicyCage Enforcement Across All Modes

Regardless of custody mode, the on-chain PolicyCage contract enforces hard limits:

```
Transaction enters PolicyCage
  │
  ├── Check: approved asset? (whitelist)
  ├── Check: approved protocol? (whitelist)
  ├── Check: value ≤ max per tx? ($10K default)
  ├── Check: daily spend ≤ cap? ($100K default)
  ├── Check: slippage ≤ max? (configurable bps)
  ├── Check: sanctions oracle clear? (OFAC compliance)
  │
  ├── All pass → Execute transaction
  └── Any fail → Revert (transaction never executes)
```

The PolicyCage does not care which custody mode generated the signature. It enforces the same constraints on every transaction. This is the final safety net — even if the agent, the key, the TEE, and the delegation are all compromised, the PolicyCage reverts transactions that violate the configured limits.

### The 7 Custom Caveat Enforcers (Detailed Mechanics)

When using Delegation Mode, seven Solidity contracts provide granular on-chain enforcement that even a fully compromised agent cannot bypass:

**1. GolemPhaseEnforcer**: Reads the agent's current behavioral phase from the VitalityOracle contract. A Conservation-phase agent (reducing exposure, preserving capital) is blocked from opening new positions at the EVM level. The agent can want to open a position — the enforcer reverts the transaction regardless. Phase transitions are controlled by the VitalityOracle, not by the agent.

**2. MortalityTimeWindow**: The delegation carries a hard expiry timestamp. After `block.timestamp > delegation_end`, every transaction reverts. This enforces a biological-clock-like constraint — the agent's authority has a natural lifespan. The owner must explicitly renew the delegation to continue operations.

**3. DreamMode**: When the agent enters dream state (offline reflection, memory consolidation, knowledge distillation), this enforcer blocks **all write transactions**. The agent can read chain state, query prices, and process memories — but it cannot move funds, open positions, or interact with protocols. The enforcer reads a boolean flag from the DreamStateOracle contract. Setting the flag is a privileged operation requiring operator approval.

**4. VaultNAV**: Enforces percentage-based position sizing. Each transaction's value is compared against the agent's total Net Asset Value (read from the NAV oracle). If a single trade would move more than the configured percentage (default 5%) of NAV, the transaction reverts. This prevents the agent from concentrating too much capital in a single action, regardless of what its strategy recommends.

**5. ReplicantBudgetEnforcer**: When an agent spawns sub-agents (replicants) via delegation attenuation, this enforcer tracks cumulative USD spend across all replicants. If the total exceeds the parent's allocated replicant budget, all sub-agent transactions revert. This prevents delegation chains from amplifying spending authority beyond the parent's intent.

**6. MaxSlippage**: Reads the swap calldata (specifically `minAmountOut` for Uniswap-style swaps) and compares the implied slippage against the configured maximum tolerance. If the agent submits a swap with slippage exceeding the limit — whether through manipulation, market volatility, or miscalculation — the enforcer reverts. The slippage check is on-chain and cannot be spoofed.

**7. DailySpendLimit**: Maintains a cumulative spend counter that resets at each UNIX day boundary (`block.timestamp / 86400`). Each transaction's value is added to the counter. If the cumulative total exceeds the configured daily limit, the transaction reverts. The counter is stored on-chain — the agent cannot reset it, manipulate it, or work around it.
