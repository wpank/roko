# TEE Clearing Engine

## Reference Specification for the Cooperative Batch Clearing System

---

## 1. What TEE Clearing Is

TEE clearing is a cryptographic order-matching system that runs inside a Trusted Execution Environment (specifically AWS Nitro Enclaves). It takes sealed orders from multiple agents, decrypts them inside a hardware-isolated enclave where no external party can observe the plaintext, runs a mathematical optimization to find the best possible matching, and emits a verifiable proof that the result is optimal.

The system is the settlement layer for a yield perpetual contract -- a financial instrument that lets DeFi participants hedge against changes in on-chain interest rates. The perpetual's reference rate is the ISFR (Internet Secured Funding Rate), a composite benchmark computed from lending rates across Aave, Compound, Ethena, and ETH staking. Orders to buy or sell this perpetual flow through the TEE clearing engine rather than a traditional order book.

The core innovation is that clearing happens in **cooperative batches** rather than continuous order-by-order matching. Every 10 seconds, a batch of orders is sealed, a solver inside the enclave finds the single uniform clearing price that maximizes total economic surplus across all orders, and a mathematical certificate proves the result is optimal. This certificate can be verified on-chain in O(n) time without re-running the optimization.

The engine is designed for a network of autonomous agents -- software programs that trade on behalf of users or protocols. These agents submit orders, predict rates, and execute hedging strategies. The TEE ensures that no agent can see another agent's order before the batch is sealed, eliminating front-running and information leakage.

---

## 2. Why TEE -- The Collusion-Proof Property

### The problem with visible orders

In a standard order book (centralized or on-chain), an order is visible before it executes. This creates three attack vectors:

1. **Front-running.** An agent sees a large buy order in the mempool or order queue and places its own buy order ahead of it, profiting from the price impact.
2. **Sandwich attacks.** An agent places orders on both sides of a victim's order, extracting value from the price movement the victim's order causes.
3. **Information leakage.** Even if orders are not directly exploitable, seeing order flow reveals strategy -- which agents are hedging, which are speculating, and in what size. This information advantage compounds over time.

These problems are especially acute in agent-to-agent markets. Agents operate at machine speed, monitor each other's behavior algorithmically, and can detect and exploit patterns that human traders cannot. A plain order book in an agent economy is a surveillance tool for sophisticated participants.

### How the TEE solves this

The TEE clearing engine eliminates all three attack vectors through a commit-reveal-clear protocol:

**Phase 1 -- Commit.** Each agent submits a sealed commitment: a keccak256 hash of their order parameters (risk aversion, cost coefficients, inventory bounds) concatenated with a random nonce. The hash reveals nothing about the order. An observer sees only a 32-byte digest.

**Phase 2 -- Reveal.** After the commit deadline, agents reveal their actual parameters. The contract verifies that each reveal matches the previously submitted hash. Early reveals (before the commit phase ends) are penalized -- 1% of stake -- because they expose the agent's strategy to others.

**Phase 3 -- Clear.** The revealed parameters are forwarded to the Nitro Enclave via VSOCK (a virtual socket connection between the parent instance and the enclave). Inside the enclave, orders are decrypted, the optimization runs, and a clearing result with a KKT optimality certificate is produced. No data leaves the enclave except the final result and the proof.

The key property: **at no point is any agent's order visible to any other agent, to the relay operator, or to the enclave operator before the batch is sealed.** The commit phase ensures orders cannot be read. The enclave ensures that even the machine operator running the enclave cannot inspect orders during the solve phase.

### Why not ZK proofs instead?

Zero-knowledge proofs could theoretically verify clearing correctness without a TEE. The system uses TEE instead for a practical reason: the clearing optimization is O(N log N) for sorting plus O(80N) for the bisection solver. Generating a ZK proof for this computation at the required 10-second batch cadence is not feasible with current proving systems. AWS Nitro NSM attestation is available today and provides equivalent integrity guarantees (with a different trust model -- hardware trust vs. mathematical trust). The design explicitly notes this as a tradeoff: TEE is the Phase 1 trust root, with a potential migration path to ZK as proving systems mature.

---

## 3. The Clearing Cycle

A clearing round proceeds through six stages. The full cycle maps to one epoch (default: 8 hours, divided by phase allocations), though the batch-level matching within cooperative clearing operates on 10-second cycles for the yield perp instrument.

### Stage 1: Accumulation

Orders enter a pending batch from three sources:

- **Active limit orders** submitted directly by traders or agents.
- **Clearing profile activations** -- persistent on-chain intents whose trigger conditions (e.g., "go short if ISFR drops below 700 bps") have been met.
- **Liquidation orders** from positions that have breached maintenance margin.

The batch accumulates until one of four trigger conditions fires:

| Trigger | Threshold | Rationale |
|---------|-----------|-----------|
| Order count | 5+ orders | Minimum batch size for meaningful surplus optimization |
| Time elapsed | 10 seconds | Maximum wait time for responsiveness |
| Imbalance ratio | 3:1 (buy:sell or sell:buy) | Severe imbalance signals market stress |
| ISFR movement | 10+ bps since last clearing | Rate movement creates urgency |

The first trigger to fire closes the batch.

### Stage 2: Batch Close

The order set is sealed. No new orders enter. The sealed batch is published to solvers. The batch data structure:

```rust
struct ClearingBatch {
    batch_id: u64,              // Monotonically increasing identifier
    isfr_at_close_bps: u32,     // ISFR value when batch sealed
    orders: Vec<ClearingOrder>, // All orders (buy and sell sides)
    total_buy_notional: U256,   // Aggregate buy-side size
    total_sell_notional: U256,  // Aggregate sell-side size
    block_height: u64,          // Chain block at batch close
}

struct ClearingOrder {
    order_id: B256,             // Unique identifier
    side: Side,                 // Buy (long) or Sell (short)
    limit_bps: u32,             // Limit price in basis points
    notional: U256,             // Notional size in USD (1e18 scaled)
    partial_fill: bool,         // Whether partial fills are allowed
    source: OrderSource,        // Active, Profile, or Liquidation
}
```

### Stage 3: Solver Competition

Multiple independent solver agents have 800 milliseconds (2 Korai blocks) to compute the optimal clearing solution. The solver's objective: find the uniform clearing price that maximizes total surplus.

Total surplus is the sum of all individual surpluses across filled orders:

```
BuyerSurplus_i  = (BuyLimit_i - ClearingPrice) * FillSize_i
SellerSurplus_j = (ClearingPrice - SellLimit_j) * FillSize_j
TotalSurplus    = sum(BuyerSurplus) + sum(SellerSurplus)
```

The solver submits a `ClearingSolution` containing the uniform clearing price, fill amounts per order, a KKT certificate, the solver's identity (ERC-8004 passport address), and a staked bond for accountability.

### Stage 4: KKT Verification

The chain verifies that the solution satisfies Karush-Kuhn-Tucker optimality conditions. Because the clearing problem is a convex linear program (linear payoffs, continuous position sizes, partially fillable orders), KKT conditions are both necessary and sufficient for global optimality. If the certificate passes, the solution is provably the best possible matching.

Three conditions are checked in a single O(n) pass:

1. **Primal feasibility.** Every filled buy order fills at or below its limit price. Every filled sell order fills at or above its limit price. Total filled buy notional equals total filled sell notional. Partial-fill constraints are respected.

2. **Dual feasibility.** Shadow prices (Lagrange multipliers) on each binding constraint are non-negative. Relaxing any active constraint would not improve the objective.

3. **Complementary slackness.** For partially filled orders, the limit price must equal the clearing price. Fully filled or unfilled orders may have non-zero dual variables.

The verification logic in Rust:

```rust
fn verify_kkt(batch: &ClearingBatch, solution: &ClearingSolution) -> bool {
    let p = solution.clearing_price_bps;
    let mut total_buy_fill = U256::ZERO;
    let mut total_sell_fill = U256::ZERO;

    for (order, fill) in batch.orders.iter().zip(solution.fills.iter()) {
        if fill.amount > order.notional { return false; }
        if fill.amount > U256::ZERO && fill.amount < order.notional
           && !order.partial_fill { return false; }

        match order.side {
            Side::Buy => {
                if fill.amount > U256::ZERO && p > order.limit_bps { return false; }
                total_buy_fill += fill.amount;
            }
            Side::Sell => {
                if fill.amount > U256::ZERO && p < order.limit_bps { return false; }
                total_sell_fill += fill.amount;
            }
        }
    }
    if total_buy_fill != total_sell_fill { return false; }

    // Complementary slackness: partial fills must be at clearing price
    for (order, fill) in batch.orders.iter().zip(solution.fills.iter()) {
        if fill.amount > U256::ZERO && fill.amount < order.notional {
            if order.limit_bps != p { return false; }
        }
    }
    true
}
```

On-chain verification cost: approximately 50,000 gas for 100 participants.

### Stage 5: Settlement

After KKT verification passes:

1. **Position updates.** New positions are created, existing positions adjusted, liquidation fills closed.
2. **Solver fee.** The winning solver earns 5% of total surplus, capped at 50 KORAI per batch: `SolverFee = min(TotalSurplus * 0.05, 50 KORAI)`.
3. **Insurance fund contribution.** Each filled order contributes 0.5 basis points of notional.
4. **ClearingInsight emission.** A structured knowledge entry is written to the chain's InsightStore (see section 5).

### Stage 6: Prediction Scoring

All agent predictions committed before the batch close are scored against the clearing price using CRPS (Continuous Ranked Probability Score): `CRPS_i = |predicted_bps_i - clearing_price_bps|`. Lower is better. Scores update each agent's 30-day rolling epistemic reputation tier.

---

## 4. The ISFR_score (Internal Solvency & Funding Ratio)

The term "ISFR" appears in two distinct contexts in this system. Understanding the distinction is critical.

### ISFR the Index

The Internet Secured Funding Rate is a composite benchmark index -- the DeFi equivalent of SOFR (Secured Overnight Financing Rate) in traditional finance. It aggregates yield signals from four sources with equal 0.25 weight:

| Source | What It Measures |
|--------|-----------------|
| Aave V3 (Ethereum mainnet) | USDC supply APY |
| Compound V3 (Ethereum mainnet) | USDC supply APY |
| Ethena (sUSDe) | 7-day rolling yield |
| ETH Beacon Chain | Consensus rewards + MEV tips as annualized yield |

The index is computed as a weighted median (not mean) to resist manipulation. It updates every 10 seconds via a consensus-level oracle precompile at address `0xA01` on the Korai chain. No oracle operator exists -- validators compute ISFR as part of block production.

Dual-median aggregation provides Byzantine fault tolerance at two levels:

- **Source level:** The weighted median tolerates manipulation of up to 49% of source weight (1 of 4 sources with equal weights).
- **Validator level:** A stake-weighted median across all validator submissions tolerates up to 49% compromised stake.

An attacker must compromise both layers simultaneously to move ISFR to an arbitrary value.

### ISFR_score in Clearing Context

Within the cooperative clearing protocol, each agent's submissions are weighted by a combination of factors that together form an agent-specific solvency and reputation score:

```
weight_i = confidence_i * reputation_multiplier(R_i)
```

Where:
- `confidence_i` is the agent's self-reported confidence in their rate observation (0.0 to 1.0)
- `R_i` is the agent's domain reputation score (minimum 0.5 to be eligible)
- The product determines how much influence the agent has on the aggregate rate

This per-agent score is distinct from the ISFR index value itself. An agent with high reputation and high confidence has more weight in the clearing aggregate. An agent with reputation below 0.5 is rejected entirely. Quarantined or revoked agents cannot submit.

The implementation in `roko-chain/src/isfr.rs` enforces these checks:

```rust
pub fn check_eligibility(&self, submission: &IsfrSubmission) -> SubmitterStatus {
    if self.quarantined.contains(&submission.submitter_passport_id) {
        return SubmitterStatus::Quarantined;
    }
    let rep = self.reputation_scores
        .get(&submission.submitter_passport_id)
        .copied()
        .unwrap_or(0.0);
    if rep < self.config.min_reputation {
        return SubmitterStatus::InsufficientReputation;
    }
    if let Some(bound) = self.config.max_rate_bound {
        if submission.rate.abs() > bound {
            return SubmitterStatus::RateOutOfBounds;
        }
    }
    if !submission.components_valid() {
        return SubmitterStatus::ComponentMismatch;
    }
    SubmitterStatus::Eligible
}
```

---

## 5. The Clearing Scoreboard

Every clearing round emits a `ClearingInsight` -- a structured knowledge artifact that tracks per-batch metrics:

```rust
struct ClearingInsight {
    batch_id: u64,              // Which batch
    clearing_price_bps: u32,    // Uniform clearing price
    total_surplus: U256,        // Total economic surplus achieved
    num_orders_filled: u32,     // Orders that received fills
    num_orders_unfilled: u32,   // Orders that did not clear
    buy_sell_imbalance: f64,    // Ratio of buy to sell volume
    time_to_solve_ms: u32,     // Solver computation time
    solver: Address,           // Winning solver's passport address
    isfr_at_clear: u32,        // ISFR value at time of clearing
    spread_to_isfr_bps: i32,   // clearing_price - ISFR (basis spread)
    timestamp: u64,
}
```

Per-agent metrics tracked across clearing rounds include:

- **CRPS score (rolling 30-day).** Prediction accuracy against clearing outcomes. Determines epistemic reputation tier (Oracle / Calibrated / Standard / Uncalibrated).
- **Fill rate.** Percentage of submitted orders that receive fills.
- **Surplus contribution.** How much economic surplus the agent's orders contributed.
- **Clearing participation.** Number of rounds the agent participated in.

Additionally, the ISFR aggregate itself records per-epoch statistics:

```rust
pub struct IsfrAggregate {
    pub median_rate: f64,         // Computed weighted median
    pub submission_count: usize,  // Number of included submissions
    pub std_deviation: f64,       // Rate dispersion
    pub excluded_count: usize,    // Outliers excluded (> 3-sigma)
    pub market_id: MarketId,      // Which market
    pub epoch: u64,               // Epoch number
    pub clearing_block: u64,      // Block at computation
}
```

Epistemic reputation tiers based on CRPS percentile:

| CRPS Percentile | Tier | Clearing Priority | Benefits |
|-----------------|------|-------------------|----------|
| Top 10% | Oracle | First | 2x knowledge query quota, priority solver submission |
| 10-30% | Calibrated | Standard | 1.5x query quota |
| 30-70% | Standard | Standard | Base quota (100 queries/day) |
| 70-100% | Uncalibrated | Last | 0.5x query quota |

---

## 6. Settlement Flow

Settlement connects the off-chain clearing result to on-chain state changes. The flow is:

### From TEE Match to On-Chain Settlement

```
1. Clearing Phase (off-chain, in TEE)
   Enclave runs QP solver, produces allocations + KKT certificate.

2. Certificate Submission
   Clearing operator submits ClearingCertificate to on-chain contract.

3. On-Chain Verification (O(n))
   Contract verifies KKT conditions:
   - Zero-sum check: sum of all net transfers = 0
   - Primal feasibility: all constraints satisfied
   - Dual feasibility: all Lagrange multipliers >= 0
   - Complementary slackness: lambda * g(x) ~ 0
   - Stationarity: gradient of Lagrangian ~ 0
   - TEE attestation: NSM signature valid

4. Settlement (atomic)
   For each agent: credit or debit net transfer.
   Sum of all transfers = 0 (enforced by contract).
   3% marketplace fee deducted.
   0.5% clearing operator fee credited.

5. Finalization
   ClearingSettled event emitted with Merkle root.
   ClearingInsight written to InsightStore.
```

### The ClearingCertificate

The on-chain data structure:

```rust
pub struct ClearingCertificate {
    pub allocations: Vec<Allocation>,       // Who gets what at what price
    pub dual_variables: Vec<f64>,           // Lagrange multipliers
    pub kkt_residual: f64,                  // Must be < epsilon (1e-6)
    pub total_welfare: f64,                 // Objective value achieved
    pub clearing_block: u64,                // Block reference
    pub merkle_root: [u8; 32],              // Commitment to full data
}

pub struct Allocation {
    pub agent_passport_id: u256,            // ERC-8004 passport
    pub job_id: [u8; 32],                   // Job/order identifier
    pub price: u256,                        // Clearing price
    pub quality_score: f64,                 // Assignment quality
}
```

On-chain verification in Solidity:

```solidity
function verifyClearingCertificate(
    ClearingCertificate calldata cert
) external view returns (bool) {
    // Zero-sum check
    int256 sum = 0;
    for (uint i = 0; i < cert.netTransfers.length; i++) {
        sum += cert.netTransfers[i];
    }
    require(sum == 0, "NOT_ZERO_SUM");

    // Primal, dual, complementary slackness, stationarity
    require(cert.kktPrimalFeasibility, "PRIMAL_INFEASIBLE");
    for (uint i = 0; i < cert.kktDualFeasibility.length; i++) {
        require(cert.kktDualFeasibility[i] >= 0, "DUAL_INFEASIBLE");
    }
    require(cert.kktComplementarySlackness < 1e-12, "SLACKNESS_VIOLATED");
    for (uint i = 0; i < cert.kktStationarity.length; i++) {
        require(abs(cert.kktStationarity[i]) < 1e-12, "STATIONARITY_VIOLATED");
    }

    // TEE attestation
    require(verifyTeeAttestation(cert.teeAttestation), "INVALID_ATTESTATION");
    return true;
}
```

### Fallback Ladder

If the QP solver fails, the system degrades gracefully:

| Level | Condition | Action |
|-------|-----------|--------|
| Normal | Valid KKT solution within 800ms | Standard cooperative clearing |
| Retry | No solution within 800ms | Batch rolls to next block; solvers get 400ms more |
| Emergency CLOB | No solution after 2 retries | Continuous limit order book activates; orders match at limit prices without surplus optimization |
| Circuit Breaker | ISFR enters Halted state | Trading paused; positions preserved; no new orders |

In simulation: 95% of epochs succeed at Step 1, 4% at Step 2 (pruned solve), 0.9% at Step 3 (external hedge), 0.1% at Step 4 (safe mode / circuit breaker).

### Epoch-Based Settlement Batching

Rather than settling each clearing round individually, the system batches settlements by epoch (every 8 hours, aligned with ISFR updates and the funding interval):

1. Collect all completed fills in the epoch.
2. Compute net transfers per agent.
3. Apply cooperative clearing to net obligations (reduces total transfer volume by 40-60%).
4. Execute netted transfers atomically.
5. Publish settlement report on-chain.

Jobs marked `urgent: true` bypass batching and settle immediately (0.1% surcharge).

---

## 7. The Clearing Profile -- Consumer-Facing Intent Layer

A clearing profile is the mechanism that makes "set it and forget it" rate hedging possible. It is a persistent, on-chain intent that sits dormant until market conditions activate it.

### Data Structure

```solidity
struct ClearingProfile {
    address account;           // Owner (EOA or smart contract)
    bytes32 market;            // Market ID (e.g., keccak256("ISFR-PERP-V1"))
    Direction direction;       // 0 = LONG, 1 = SHORT
    uint256 trigger;           // ISFR threshold in bps that activates the profile
    uint256 maxNotional;       // Maximum USD exposure (1e18 scaled)
    uint16 maxFeeBps;          // Maximum acceptable clearing fee
    uint64 expiry;             // 0 = no expiry
    uint256 minFillNotional;   // Minimum fill per round (anti-dust)
    uint32 maxRounds;          // 0 = unlimited rounds
}
```

### Lifecycle

1. **Creation.** User submits one transaction. Profile stored on-chain. Gas cost: ~50K (one storage write).
2. **Dormancy.** Profile sits on-chain. Costs nothing. No keeper, no monitoring. The consensus layer checks trigger conditions during each ISFR update.
3. **Activation.** When ISFR crosses the trigger: clearing engine includes the profile's order in the next batch. Order sized as `min(maxNotional - filledSoFar, availableCounterparty)`. If `maxFeeBps` is exceeded, profile skips that round and retries.
4. **Filling.** Profile participates in clearing rounds until `maxNotional` is filled, `maxRounds` reached, `expiry` passed, or user cancels.
5. **Completion.** The resulting position is a standard yield perpetual position.

### Why This Matters

Without clearing profiles, hedging requires 24/7 rate monitoring, manual order submission, managing partial fills across rounds, and adjusting as rates move. With profiles, one transaction handles everything. This is the UX difference between "DeFi experts can hedge" and "any treasury with a multisig can hedge."

**Worked example: $10M DAO treasury.** A DAO earning 8% on Aave creates a SHORT profile with trigger at ISFR < 700 bps, max notional $10M. If rates never drop below 7%, the hedge costs $0 -- the profile never activates. If rates drop to 3%, the profile activates and the perp position compensates for the lost yield income. One transaction. Zero carrying cost until needed.

---

## 8. Mirage-rs -- The Clearing Engine Implementation

Mirage-rs is the in-process EVM simulator that serves as the development and simulation runtime for the clearing engine. It is a full EVM environment built on revm (the Rust EVM used by Foundry and Reth), extended with custom chain functionality.

### Architecture

Mirage-rs runs in the same process as the agent. It provides:

- **Local mode:** Fresh EVM state for testing. No external dependencies.
- **Fork mode:** Clones mainnet state at a specific block for simulation against real-world data.
- **Chain extensions:** Custom precompiles and state modules that emulate Korai-specific functionality (HDC vectors, agent registry, reputation registry, knowledge store, pheromone field, ISFR oracle).

### ISFR API Endpoints

Mirage-rs exposes ISFR data through its HTTP REST API:

**`GET /api/isfr/current`** -- Returns the latest ISFR data. In production, proxies to an upstream ISFR service via `ISFR_SERVICE_URL`. If the upstream is unavailable (and `ISFR_STRICT_PROXY` is not set), falls back to a local minimal payload:

```json
{
    "status": "ok",
    "source": "mirage-local-isfr-minimal",
    "state": "active",
    "composite_rate_bps": 690,
    "value_bps": 690,
    "value": 0.069,
    "confidence": 0.85,
    "components": [
        {"venue": "hyperliquid", "rate_bps": 720, "weight": 0.35, "market": "ETH-PERP"},
        {"venue": "dydx",        "rate_bps": 650, "weight": 0.25, "market": "ETH-USD"},
        {"venue": "gmx",         "rate_bps": 710, "weight": 0.20, "market": "ETH-USD"},
        {"venue": "aevo",        "rate_bps": 680, "weight": 0.12, "market": "ETH-PERP"},
        {"venue": "vertex",      "rate_bps": 660, "weight": 0.08, "market": "ETH-PERP"}
    ],
    "window": {"duration_hours": 8, "start_block": ..., "end_block": ...}
}
```

**`GET /api/isfr/history`** -- Returns historical ISFR data points. Accepts `limit` query parameter. Falls back to synthetic oscillating data around 690 bps when the upstream is unavailable.

### Additional Dashboard Endpoints

The full mirage-rs HTTP API serves as the dashboard backend with endpoints for:

- Agent topology and registry (`/api/agents/*`)
- Knowledge graph and pheromone field (`/api/knowledge/*`, `/api/pheromones/*`)
- Task tracking (`/api/tasks/*`)
- Prediction sessions and calibration (`/api/predictions/*`)
- Contract deployment registry (`/api/deployment`)
- WebSocket live event streaming (`/api/ws`)
- Combined statistics (`/api/stats`)

### Clearing Engine in roko-chain

The clearing logic itself lives in `roko-chain/src/isfr.rs`. Key components:

**`IsfrRegistry`** -- Collects submissions, runs weighted median aggregation with 3-sigma outlier exclusion, produces `IsfrAggregate` outputs. Also retains a legacy QP clearing solver path via `clear_epoch()` that produces `ClearingCertificate` objects.

**`ClearingCycleState`** -- A state machine tracking the six clearing phases (COMMIT, REVEAL, SOLVE, CERTIFICATE, VERIFY, SETTLE) with configurable phase duration allocations:

```rust
pub struct ClearingCycleState {
    pub epoch: u64,
    pub phase: ClearingPhase,
    pub phase_started_at: u64,
    pub epoch_started_at: u64,
    pub allocations: PhaseAllocations,      // Default: 40/15/15/10/10/10%
    pub epoch_duration_secs: u64,           // Default: 28800 (8 hours)
}
```

**`IsfrConfig`** -- Configuration knobs:

```rust
pub struct IsfrConfig {
    pub epoch_duration_secs: u64,           // 28800 = 8 hours
    pub max_kkt_residual: f64,              // 1e-6 acceptance threshold
    pub min_submissions_for_clearing: usize, // Minimum 2 submissions
    pub min_reputation: f64,                // 0.5 minimum to submit
    pub max_rate_bound: Option<f64>,        // +-10% default bounds
    pub outlier_sigma: f64,                 // 3.0 sigma for outlier exclusion
}
```

---

## 9. How TEE Clearing Connects to the Yield Perp and ISFR Index

The three components form a vertical stack:

```
Layer 1: Yield Perpetual Contract (the product)
   |
   |  settles against
   v
Layer 2: ISFR Index (the reference rate)
   |
   |  orders matched via
   v
Layer 3: TEE Cooperative Batch Clearing (the settlement engine)
```

### The yield perpetual contract

A perpetual futures contract whose underlying is a yield rate measured in basis points, not an asset price. It has no expiration, requires no rollover, and uses continuous funding to track the benchmark. Key parameters:

- Contract multiplier: $1 notional per 1 bp per unit (linear payoff -- critical for KKT verification)
- Funding interval: 8 hours (3 funding events per day)
- Max leverage: 10x (10% initial margin, 5% maintenance)
- Mark price: `0.7 * ISFR_Oracle + 0.3 * EMA(OrderBook_MidPrice, 300s)`

The linear payoff is not a simplification -- it is a design requirement. Convex or concave payoffs break the KKT conditions that make cooperative clearing provably optimal.

### The ISFR index as anchor

ISFR provides the reference rate that the perpetual settles against. The mark price blends ISFR (70% weight) with the order book mid-price EMA (30% weight). Funding rate has two components:

- **Premium component:** `clamp(EMA(MidPrice - ISFR, 300s) / ISFR, -0.05%, +0.05%)` per 8-hour period. When the perp trades above ISFR, longs pay shorts.
- **Carry component:** `(ISFR - RiskFreeRate) * (FundingInterval / Year)` where the risk-free rate is ETH staking yield.

### The epistemic flywheel

Cooperative clearing does not just match trades -- it produces structured knowledge. Every clearing round emits a `ClearingInsight`. Every ISFR update enables prediction scoring. This creates a feedback loop:

1. Clearing produces insights about rate dynamics and market microstructure.
2. Insights improve agent prediction models.
3. Better predictions attract more volume (agents trade on their predictions).
4. More volume produces more clearing rounds.
5. More rounds produce more insights.

The clearing engine is an engine of intelligence, not merely a matching service. The dual output (matched trades + knowledge artifacts) is what distinguishes cooperative clearing from a traditional CLOB.

---

## 10. Security Model

### Attestation

The TEE clearing engine runs inside an AWS Nitro Enclave. Nitro Enclaves provide hardware-level isolation: the enclave has its own kernel, its own memory, and no persistent storage, no network access, and no interactive access. The parent instance communicates with the enclave solely through a VSOCK connection (ports 5000-5003).

Every clearing result includes a TEE attestation:

```rust
pub struct TeeAttestation {
    pub enclave_id: [u8; 32],    // Unique enclave identifier
    pub pcr0: [u8; 48],         // Platform Configuration Register 0
    pub timestamp: u64,          // Attestation time
    pub signature: Signature,    // NSM attestation signature
}
```

**PCR0/1/2 (Platform Configuration Registers)** are measurements of the enclave's boot chain:
- PCR0: Hash of the enclave image (the exact binary that is running)
- PCR1: Hash of the kernel and boot parameters
- PCR2: Hash of the application code

These values are signed by the Nitro Security Module (NSM), a hardware component in every Nitro-capable AWS instance. Anyone can verify that a specific, audited version of the clearing code produced the result -- not a modified version, not a different binary.

### Enclave Properties

| Property | Guarantee |
|----------|-----------|
| **No persistent storage** | The enclave cannot store data between invocations. State is ephemeral. |
| **No network access** | The enclave cannot communicate with external services. All I/O goes through VSOCK to the parent. |
| **No interactive access** | No SSH, no console, no debugging interface. The operator cannot inspect enclave state. |
| **Memory isolation** | The parent instance cannot read enclave memory, even with root access. |
| **Reproducible builds** | The enclave image is built deterministically. The PCR0 hash uniquely identifies the code. |

### Trust Assumptions

The security model rests on three trust assumptions:

1. **AWS hardware integrity.** The Nitro Security Module is a hardware root of trust manufactured by AWS. The assumption is that AWS has not backdoored the NSM and that the hardware attestation chain is correct. This is the same trust assumption made by every Nitro Enclave deployment (including AWS's own KMS).

2. **Enclave code correctness.** The clearing algorithm inside the enclave must be correct. The PCR0 hash proves *which* code ran, but not that the code is free of bugs. Auditing the enclave image is necessary. The KKT certificate provides a secondary check: even if the code has bugs, an incorrect result will fail on-chain verification.

3. **Commit-reveal integrity.** The commit-reveal protocol assumes that agents cannot collude to share commitments before the reveal phase. If a majority of agents share their commitments out-of-band, the privacy guarantee degrades. The 1% stake penalty for early reveals is an economic deterrent, not a cryptographic guarantee.

### What the TEE does NOT guarantee

- **Liveness.** If the enclave crashes or the parent relay goes offline, clearing stalls. The fallback ladder (retry, emergency CLOB, circuit breaker) handles this.
- **Censorship resistance.** The relay operator could refuse to forward orders to the enclave. Solver competition and multiple relay operators in the design mitigate this.
- **Long-term secrecy.** After the clearing round settles, the fills are public. The TEE protects pre-trade privacy, not post-trade privacy.

### 9-Step Verification Protocol

The parent relay performs a 9-step verification of every clearing result before forwarding fills to execution:

1. Verify NSM attestation signature
2. Check PCR0/1/2 against known-good values
3. Verify KKT stationarity conditions
4. Verify primal feasibility (all constraints satisfied)
5. Verify dual feasibility (multipliers non-negative)
6. Verify complementary slackness
7. Verify zero-sum property (total transfers net to zero)
8. Verify timestamp freshness (result is from current epoch)
9. Verify batch ID matches the sealed batch

Only after all 9 checks pass does the relay submit fills to the execution venue.

### Testnet Validation

As of the specification date, the clearing engine has completed 37 verification rounds on testnet with 333 out of 333 individual checks passed -- a 100% success rate across all verification steps.

---

## Appendix: Data Flow Summary

```
User
  |-- sets ClearingProfile (one tx) --> On-chain contract
  |
Agent (subscribed to ISFR rate feed)
  |-- ISFR crosses trigger threshold
  |-- POST /v1/commit (ECIES-sealed intent) --> Parent Relay
  |                                                |
  |                                          VSOCK 5001
  |                                                |
  |                                          Nitro Enclave
  |                                          - Decrypt orders
  |                                          - Run QP solver
  |                                          - Generate KKT cert
  |                                          - NSM attestation
  |                                                |
  |                                          VSOCK return
  |                                                |
  |-- GET /v1/result (fills + score) <-- Parent Relay
  |                                      (after 9-step verify)
  |
  |-- Fills placed on Hyperliquid L1
  |-- KKT proof submitted to HyperEVM (TEEClearingVault + KKTVerifier)
  |
User notified: "Hedged at X bps"
```

---

## Appendix: Key Source Files

| Component | Location |
|-----------|----------|
| ISFR registry, clearing cycle state machine, weighted median | `crates/roko-chain/src/isfr.rs` |
| ClearingCertificate, Allocation structs | `crates/roko-chain/src/phase2.rs` |
| Collusion detection (assignment graph analysis) | `crates/roko-chain/src/collusion.rs` |
| ISFR HTTP proxy endpoints | `apps/mirage-rs/src/http_api/isfr.rs` |
| HTTP API router and state | `apps/mirage-rs/src/http_api/mod.rs` |
| Compliance policies (best-execution, position limits, wash trading) | `crates/roko-chain/src/identity_economy_markets.rs` |
| Mirage-rs EVM simulator | `apps/mirage-rs/src/` |
