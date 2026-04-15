# DeFi Integration Layer

## A Self-Contained Reference for the Nunchi DeFi Stack

---

## 1. The DeFi Thesis: Why an Agent Toolkit Needs Financial Primitives

Nunchi is building an infrastructure stack where autonomous AI agents operate on behalf of users. The DeFi integration layer answers a specific question: what is the first product where agent-mediated execution creates value that no existing protocol can deliver?

The answer is yield perpetuals -- perpetual futures contracts that settle against an on-chain interest rate benchmark. This is the beachhead product.

### The structural gap

Traditional finance manages interest rate risk through a deep ecosystem of derivatives -- swaps, swaptions, caps, floors, futures -- cleared by CME and LCH, priced against SOFR (the Secured Overnight Financing Rate). CME reported total revenue of $6.5 billion in 2025, with interest rate products driving 14.2 million contracts per day. SOFR futures and options alone averaged 5.4 million contracts per day. The global OTC interest rate derivatives market reached $665.8 trillion in notional outstanding by mid-2025.

DeFi has approximately $49.5 billion in lending TVL across Aave ($23.5B), Morpho ($10B+), Spark ($7.9B), and Compound ($2.1B). Every dollar lent carries unhedged variable rate exposure. A treasury with $10M earning variable yield on Aave has no way to lock in today's rate. If rates drop from 8% to 3%, that treasury faces a $500K annualized shortfall with no instrument to hedge against it.

On-chain interest rate derivative TVL is under $100 million. The TradFi equivalent is $665.8 trillion. That is a six-order-of-magnitude gap. It is not a marketing failure. It is the absence of a foundational primitive: DeFi has no credible, manipulation-resistant, continuously-published benchmark rate that derivatives can settle against.

### Why agents are essential

The beachhead product is not just the instrument. It is the combination of the instrument with autonomous agents that execute on behalf of users. Without agents, yield perpetuals are a complex derivatives product requiring monitoring, position management, margin maintenance, and clearing round participation. With agents, the user creates a single "clearing profile" -- one signature declaring risk preferences -- and the agent handles everything else: monitoring rates, detecting trigger conditions, constructing positions, routing through clearing, managing margin, and closing positions when deactivation criteria are met.

This transforms yield perpetuals from "DeFi experts can hedge rates" to "any treasury with a multisig can hedge rates." The agent layer is what makes the DeFi product accessible at scale.

---

## 2. Source Protocols: Where the Data Comes From

ISFR (the Internet Secured Funding Rate, described in section 3) aggregates yield data from four source protocols, each representing a structurally distinct yield generation mechanism. The sources are organized into four classes with differentiated weights.

### Aave V3

**What it provides:** USDC supply APY on Ethereum mainnet.

**TVL:** ~$23.5B (April 2026, per DeFiLlama). The largest lending protocol by TVL.

**Update frequency:** Per Ethereum block (~12 seconds). Every deposit, withdrawal, or liquidation recalculates the supply rate through Aave's utilization curve.

**Liveness timeout:** 120 seconds (10 missed Ethereum blocks).

**Why it matters:** Aave's supply rate is the closest DeFi analog to the overnight secured lending rate that SOFR measures in TradFi. It reflects the cost of borrowing USDC against crypto collateral, which is the fundamental secured funding cost on-chain.

**Data path:** Korai validators read Aave's supply rate by querying the Aave V3 Pool contract on Ethereum mainnet via their own full nodes. No shared data endpoint. No single point of failure.

### Compound V3

**What it provides:** USDC supply APY on Ethereum mainnet.

**TVL:** ~$2.1B (April 2026).

**Update frequency:** Per Ethereum block.

**Liveness timeout:** 120 seconds.

**Why it matters:** Compound uses a different utilization curve from Aave. Including both provides diversification within the lending class -- protocol-specific dynamics (governance parameters, liquidity incentives, pool-specific risk premia) are smoothed out by the median.

### Ethena sUSDe

**What it provides:** 7-day rolling yield on sUSDe, Ethena's staked USDe token.

**TVL/Supply:** ~$5.88B in USDe supply.

**Update frequency:** Updated as perpetual funding rates settle (~8 hours).

**Liveness timeout:** 24 hours.

**Why it matters:** Ethena represents delta-neutral funding rate exposure. sUSDe yield derives from the funding rate on perpetual futures positions (primarily short ETH perps hedging a spot ETH position). This captures a structurally different yield source than lending protocols -- it measures speculative sentiment and leverage demand, which pure lending rates miss.

### ETH Beacon Chain Staking

**What it provides:** Consensus rewards plus MEV tips, expressed as annualized yield.

**Staked value:** ~$115B+ (approximately 38 million ETH staked).

**Update frequency:** Per epoch (~6.4 minutes, approximately 225 epochs/day).

**Liveness timeout:** 30 minutes (5 missed epochs).

**Why it matters:** ETH staking yield is the most decentralized yield source in crypto. It is the base cost of securing the Ethereum network, analogous to the Treasury yield in TradFi. It is highly resistant to single-entity manipulation and provides the floor rate for the Ethereum economy.

### Class weights in ISFR

| Class | Weight | Sources | Rationale |
|-------|--------|---------|-----------|
| LENDING | 0.60 | Aave V3, Compound V3 | Most analogous to SOFR; deepest, most stable DeFi yield market; primary hedging target for treasuries |
| STRUCTURED | 0.25 | Ethena sUSDe | Delta-neutral yield captures funding with dampening; structurally distinct from lending |
| FUNDING | 0.10 | Hyperliquid ETH perp funding rate | Signal on speculative positioning; downscaled due to volatility |
| STAKING | 0.05 | ETH staking rate | Floor rate for Ethereum; very stable |

In V1 of the original ISFR service (a Python microservice), the four sources are equally weighted at 0.25 each. The class-weighted approach described above is the V2 methodology intended for the Korai validator-computed version. The equal-weight V1 approach maximizes diversification and avoids governance complexity during bootstrapping. V2 introduces governance-adjustable weights with constraints: no single source can exceed 35% of total weight, no source can drop below 5%.

---

## 3. ISFR: The Internet Secured Funding Rate

ISFR is a composite benchmark index representing the cost of secured funding across DeFi. It is to DeFi what SOFR is to TradFi: the reference rate that financial instruments settle against.

### Core properties

**Consensus-level computation.** Every Korai validator independently computes ISFR as part of block production. The computation happens every 25 blocks, which at Korai's 400ms block time yields an update cadence of approximately 10 seconds -- 8,640 updates per day versus SOFR's single daily publication. There is no separate oracle operator, no off-chain bridge, no Chainlink dependency.

**Dual median aggregation.** Two layers of median-based aggregation provide Byzantine fault tolerance:

- **Layer 1 (source aggregation):** Each validator reads rates from the source protocols and computes a weighted median across sources within each class, then a weighted sum across classes.
- **Layer 2 (validator aggregation):** The chain computes a stake-weighted median across all validator submissions.

To manipulate ISFR to an arbitrary value, an attacker must compromise 50%+ of source weight AND 50%+ of validator stake simultaneously. Either layer alone stops the attack.

**Native precompile publication.** ISFR is published via a dedicated precompile at address `0xA01` on Korai. Any smart contract can read the current ISFR value with a single precompile call at fixed gas cost:

```solidity
// Read current ISFR
(uint32 valueBps, uint8 state) = ISFROracle(0xA01).current();

// Read historical ISFR at a specific block
ISFRSnapshot memory snap = ISFROracle(0xA01).at(blockHeight);

// Read TWAP over a time range
uint32 twapBps = ISFROracle(0xA01).twap(startBlock, endBlock);
```

### On-chain storage format

```solidity
struct ISFRSnapshot {
    uint32 valueBps;          // ISFR in basis points (e.g., 600 = 6.00%)
    uint64 blockHeight;       // Block at which this value was computed
    uint64 timestamp;         // Unix timestamp
    uint8  state;             // 0=Live, 1=Degraded, 2=Stale, 3=Halted
    uint16 confidenceBps;     // Percentage of validators within 1-sigma, in bps
    uint32 numSources;        // Number of active sources contributing
    uint32 numValidatorVotes; // Number of validator votes in this round
}
```

Historical values are retained for 90 days (approximately 19.4 million snapshots at 10-second cadence).

### Validator submission format

```rust
struct OracleVote {
    /// The validator's computed ISFR value in basis points.
    value_bps: u32,
    /// Block height this vote applies to.
    block_height: u64,
    /// Validator's BLS signature over (value_bps, block_height).
    signature: BlsSignature,
    /// Validator index in the current committee.
    validator_index: u32,
}
```

### Computation example

Suppose at a given update epoch, the four sources report:

| Source | Rate |
|--------|------|
| Aave V3 USDC supply APY | 4.50% |
| Compound V3 USDC supply APY | 5.80% |
| Ethena sUSDe 7-day rolling yield | 6.20% |
| ETH staking yield | 7.10% |

With equal V1 weights (0.25 each), sort ascending and walk cumulative weights:

| Rate | Weight | Cumulative |
|------|--------|------------|
| 4.50% | 0.25 | 0.25 |
| 5.80% | 0.25 | 0.50 |
| 6.20% | 0.25 | 0.75 |
| 7.10% | 0.25 | 1.00 |

The cumulative weight reaches 0.50 at the boundary between 5.80% and 6.20%. The ISFR is the mean of the bracketing values: **(5.80% + 6.20%) / 2 = 6.00% (600 basis points)**.

### Why median resists manipulation

Consider a flash loan attack spiking Aave's rate to 50%:

- **Median result:** Sorted rates become 5.80%, 6.20%, 7.10%, 50.00%. Median = (6.20% + 7.10%) / 2 = 6.65%. A 65 bps transient distortion that corrects in 10 seconds.
- **Mean result:** (5.80% + 6.20% + 7.10% + 50.00%) / 4 = 17.28%. An 1,128 bps spike, nearly tripling the index.

The median absorbs outliers. The mean amplifies them. This is why SOFR, ISFR, and every credible benchmark uses a median.

### Publication states and circuit breakers

ISFR operates in four states based on data availability and validator agreement:

| State | Condition | Behavior |
|-------|-----------|----------|
| **Live** | 3+ sources reporting AND confidence >= 70% | Normal publication. All instruments settle against the live rate. |
| **Degraded** | 2 sources OR confidence 50-70% | Rate published with wider confidence interval. Instruments settle normally. Clearing profiles with tight triggers may pause. |
| **Stale** | 1 source reporting | Rate frozen at last known Live or Degraded value. Clearing continues on stale rate. New profile activations suspended. |
| **Halted** | 0 sources OR confidence < 50% OR consensus failure | Rate frozen. Clearing falls back to emergency CLOB. No new positions. No liquidations. |

Confidence is the percentage of validator stake submitting votes within one standard deviation of the stake-weighted median. Recovery from Degraded/Stale requires confidence exceeding 80% for 3 consecutive update periods (30 seconds) before transitioning back to Live. The hysteresis (70% down, 80% up) prevents oscillation.

### Solidity oracle interface

```solidity
interface IISFROracle {
    struct Component {
        bytes32 sourceId;       // Protocol identifier (e.g., keccak256("aave-usdc-eth"))
        uint256 rate;           // Rate in basis points (1 bps = 0.01%)
        uint256 tvl;            // TVL in USD (18 decimals)
        uint64  chain;          // Source chain ID
        uint64  timestamp;      // When the rate was observed
    }

    struct RateSnapshot {
        uint256 rate;           // ISFR in basis points
        uint64  timestamp;      // When this snapshot was computed
        uint64  blockNumber;    // Block at which it was recorded
        uint8   componentCount; // Number of components used
        bytes32 merkleRoot;     // Merkle root of component data
    }

    function submitComponents(Component[] calldata components) external;
    function aggregate() external;
    function getCurrentRate() external view returns (RateSnapshot memory);
    function getRateAt(uint64 blockNumber) external view returns (RateSnapshot memory);
    function getComponents() external view returns (Component[] memory);
    function isUpdateDue() external view returns (bool);

    event ComponentSubmitted(address indexed validator, bytes32 sourceId, uint256 rate);
    event RateAggregated(uint256 rate, uint64 timestamp, uint8 componentCount);
    event DeviationTriggered(uint256 oldRate, uint256 newRate, uint256 deviationBps);
}
```

---

## 4. The Yield Perpetuals Product

A yield perpetual is a perpetual futures contract whose underlying is a DeFi yield reference rate -- not an asset price. Users take long or short positions on the rate, pay or receive funding each interval, and hold indefinitely without expiration. It settles against ISFR.

### Why perpetuals instead of fixed-term instruments

The dominant on-chain yield trading protocol is Pendle ($5.7B average TVL in 2025, $47.8B trading volume). Pendle tokenizes yield into Principal Tokens (PT) and Yield Tokens (YT) with fixed maturities. This design has fundamental structural limitations:

| Problem | Pendle (fixed-term) | Yield Perpetuals |
|---------|---------------------|------------------|
| **Liquidity fragmentation** | 200+ pools (one per asset + maturity). Most pools under $200M. | 1 pool per benchmark rate. All trading concentrates in one venue. |
| **Rollover cost** | 20-80+ bps slippage per roll, 4x/year = 200 bps annually. For $10M, that is $200K/year in pure friction. | Zero. Position persists indefinitely. |
| **Leverage** | 1x (no native leverage). | Up to 10x (10% initial margin). |
| **Underlying** | Individual asset yield (stETH, GLP). | Composite benchmark rate (ISFR). One instrument hedges aggregate exposure. |
| **Agent composability** | Manual rollover, pool selection, maturity management. | One clearing profile. Set trigger, walk away. Agent handles everything. |

### Contract specification

| Parameter | Value |
|-----------|-------|
| Underlying | DeFi yield rate via ISFR |
| Quote unit | Basis points (1 bp = 0.01%) |
| Contract multiplier | $1 notional per 1 bp per unit |
| Minimum tick | 0.25 bp (matching CME SOFR futures tick size) |
| Lot size | 1 unit ($1/bp) |
| Max leverage | 10x (10% initial margin, 5% maintenance margin) |
| Funding interval | 8 hours (3 funding events per day) |
| Settlement | Cooperative clearing (batch, KKT-verified) |
| Trading hours | 24/7/365 |

### Payoff structure

The payoff is linear in basis points:

```
PnL per unit = (Exit_bps - Entry_bps) * Direction * $1
```

Where Direction = +1 for long, -1 for short. Linear payoff is a design requirement -- convex or concave payoffs break the KKT conditions that make cooperative clearing provably optimal.

**Example:** Enter long at 600 bps (6.00%). ISFR rises to 650 bps. Position size: 1,000 units.

```
PnL = (650 - 600) * (+1) * $1 * 1,000 = $50,000
```

### Mark price formula

Mark price governs margining, liquidation, and unrealized PnL:

```
MarkPrice = 0.7 * ISFR_Oracle + 0.3 * EMA(OrderBook_MidPrice, 300s)
```

The 70/30 split ensures the oracle dominates (preventing order book manipulation) while the order book component reflects genuine supply/demand dynamics. During degraded states:

- **Degraded:** `0.9 * ISFR_Oracle + 0.1 * EMA(MidPrice, 600s)`
- **Stale:** `1.0 * Last_Live_ISFR`
- **Halted:** Mark price frozen. No new liquidations.

### Funding rate

```
FundingRate = PremiumComponent + CarryComponent
```

**Premium component** (standard perpetual convergence mechanism):

```
PremiumComponent = clamp(
    EMA(MidPrice - ISFR, 300s) / ISFR,
    -0.05%,    // floor: -5 bps per 8h period
    +0.05%     // cap: +5 bps per 8h period
)
```

When the perp trades above ISFR, longs pay shorts. Below ISFR, shorts pay longs.

**Carry component** (unique to rate perpetuals):

```
CarryComponent = (ISFR - ETH_Staking_Yield) * (FundingInterval / Year)
```

This prevents a free arbitrage from forming when the yield curve is not flat. Without it, simultaneously holding the underlying yield asset and shorting the perpetual would create risk-free profit.

### Position semantics

| Position | Economic meaning | Who uses it |
|----------|------------------|-------------|
| **Long** | Profits when rates rise. Receives funding when market rate > locked rate. | Speculators betting on rate increases. |
| **Short** | Profits when rates fall. The core hedging use case. | Treasuries locking in current rates. Aave suppliers hedging against rate drops. |

---

## 5. Hyperliquid Integration

Hyperliquid plays two roles in the architecture: as a data source and as a settlement venue.

### As data source

Hyperliquid's ETH perpetual funding rate is one of the four ISFR inputs (in the FUNDING class, weighted at 0.10). This is a read-only data relationship. Korai validators query the Hyperliquid API for the current ETH perp funding rate and include it in their ISFR computation. Korai has no settlement, operational, or systemic dependency on Hyperliquid as an exchange.

### As settlement venue

Yield perpetuals are designed to settle on Hyperliquid via HIP-3 (Hyperliquid Improvement Proposal 3), which allows builder-operated perpetual markets on the HyperEVM. This means Nunchi can deploy its own perp market on Hyperliquid's infrastructure, leveraging Hyperliquid's existing liquidity, order matching, and settlement guarantees while controlling the instrument specification, oracle source (ISFR), and clearing logic.

The HyperEVM deployment path means the YieldPerpMarket, ISFROracle, and ClearingHouse smart contracts are deployed on HyperEVM -- Hyperliquid's EVM-compatible execution environment. The ISFR oracle value is bridged from Korai to HyperEVM via a publisher that feeds the Daeji oracle precompile at `0xA01`.

### Daeji oracle integration

The Daeji oracle is a precompile on the settlement layer that provides mark prices to the clearing and liquidation engines. ISFR values reach the Daeji oracle through a publisher that polls the ISFR service and submits values as `OracleVote` entries. Mark price computation follows Hyperliquid's formula adapted for rate instruments:

```
Mark[i] = OracleSpot[i] + EMA_150s(DaejiMid[i] - OracleSpot[i])
```

Where `OracleSpot[i]` is the ISFR value and `DaejiMid[i]` is the mid-price from the on-chain order book.

### Phase ordering per block

Each Daeji block processes operations in a fixed sequence to guarantee liquidations always use fresh prices:

```
Phase 1: ORACLE       -> apply_oracle_tick()
Phase 2: ACCRUAL      -> compute funding using fresh oracle prices
Phase 3: LIQUIDATION  -> check margins using fresh mark prices
Phase 4: PERPS        -> match orders
```

---

## 6. Smart Contracts: Interfaces and Events

### YieldPerpMarket (implied by the ClearingHouse)

The primary on-chain contract is the ClearingHouse, which manages the full lifecycle of yield perpetual positions.

### IClearingHouse

```solidity
interface IClearingHouse {
    struct Position {
        uint128 id;
        address owner;
        bool    isLong;
        uint256 size;           // Notional in USDC (18 decimals)
        uint256 entryRate;      // ISFR at entry (basis points)
        uint256 collateral;     // Posted collateral in USDC (18 decimals)
        uint64  openedAtBlock;
        uint64  lastSettledBlock;
    }

    struct ClearingRound {
        uint128 roundId;
        uint256 clearingRate;   // ISFR snapshot used for this round
        uint256 totalLongSize;
        uint256 totalShortSize;
        uint256 fundingPaid;    // Total funding transferred this round
        uint64  settledAtBlock;
    }

    // --- Position management ---
    function openPosition(
        bool isLong,
        uint256 size,
        uint256 collateral
    ) external returns (uint128 positionId);

    function closePosition(uint128 positionId) external;

    function addCollateral(uint128 positionId, uint256 amount) external;

    function removeCollateral(uint128 positionId, uint256 amount) external;

    // --- Clearing ---
    function settle() external returns (uint128 roundId);

    // --- Liquidation ---
    function liquidate(uint128 positionId) external;

    // --- Query ---
    function getPosition(uint128 positionId) external view returns (Position memory);
    function getPositionsByOwner(address owner) external view returns (Position[] memory);
    function getLatestRound() external view returns (ClearingRound memory);
    function isLiquidatable(uint128 positionId) external view returns (bool);
    function nextSettlementBlock() external view returns (uint64);

    // --- Events ---
    event PositionOpened(uint128 indexed positionId, address indexed owner,
                         bool isLong, uint256 size);
    event PositionClosed(uint128 indexed positionId, uint256 realizedPnl,
                         bool profitable);
    event RoundSettled(uint128 indexed roundId, uint256 clearingRate,
                       uint256 fundingPaid);
    event PositionLiquidated(uint128 indexed positionId, address indexed liquidator,
                             uint256 bonus);
    event CollateralAdded(uint128 indexed positionId, uint256 amount);
    event CollateralRemoved(uint128 indexed positionId, uint256 amount);
}
```

### ClearingProfile (on-chain intent)

```solidity
struct ClearingProfile {
    address account;
    bytes32 market;           // e.g., keccak256("ISFR-PERP-V1")
    Direction direction;      // 0 = LONG, 1 = SHORT
    uint256 trigger;          // ISFR threshold in bps that activates the profile
    uint256 maxNotional;      // Maximum notional exposure in USD (1e18 scaled)
    uint16 maxFeeBps;         // Maximum acceptable clearing fee
    uint64 expiry;            // 0 = no expiry
    uint256 minFillNotional;  // Minimum fill size per round (prevents dust fills)
    uint32 maxRounds;         // Max clearing rounds to participate in. 0 = unlimited.
}
```

### Cooperative clearing data structures

```rust
struct ClearingBatch {
    batch_id: u64,
    isfr_at_close_bps: u32,
    orders: Vec<ClearingOrder>,
    total_buy_notional: U256,
    total_sell_notional: U256,
    block_height: u64,
}

struct ClearingOrder {
    order_id: B256,
    side: Side,               // Buy (long) or Sell (short)
    limit_bps: u32,           // Limit price in basis points
    notional: U256,           // Notional size in USD (1e18)
    partial_fill: bool,
    source: OrderSource,      // Active, Profile, Liquidation
}

struct ClearingSolution {
    clearing_price_bps: u32,
    fills: Vec<FillAmount>,
    kkt_certificate: KKTCertificate,
    solver: Address,
    bond: U256,
}
```

---

## 7. Agent-Mediated DeFi

### How agents execute trades

Agents interact with DeFi protocols through a layered stack. No agent ever calls protocol-specific ABIs directly. The stack, from bottom to top:

1. **VenueAdapter** (Act-protocol Cell): Normalized interface to a DeFi protocol. One implementation per protocol. Provides `swap()`, `add_liquidity()`, `remove_liquidity()`, `get_pool_state()`, `get_quote()`.

2. **DeFiRiskEngine** (Verify-protocol Cell): Every trade passes through this engine before execution. Enforces position limits, drawdown caps, daily loss limits, slippage tolerance, and MEV protection. If a trade violates any constraint, it is rejected before submission.

3. **Affect-modulated sizing** (Route-protocol Cell): Position sizing passes through the daimon affect engine. The core mechanism is prospect theory (Tversky & Kahneman 1992): losses are weighted 2.25x relative to gains. An agent that just suffered a loss will naturally reduce position size.

4. **TradingReflect** (Score-protocol Cell): FIFO matching of position entries to exits. When a position closes, TradingReflect traces the P&L back to the decision that opened it -- which agent, which model, which indicators, what regime. This continuous reward feeds the cascade router, playbook outcomes, and episode logger.

### The clearing profile flow

The canonical user experience is:

1. User creates a clearing profile (one transaction): "hedge me if the rate drops below 6%."
2. An agent subscribes to the ISFR feed and the yield perp order book.
3. The agent's predictive model detects elevated probability of a rate crossing.
4. The agent submits an intent to the clearing engine.
5. The clearing engine matches the intent with counterparties in a cooperative batch.
6. The hedge executes.
7. The agent manages the position (margin, funding, exit) autonomously.

**User action count: 1.**

### Agent roles in the ISFR ecosystem

| Role | Function |
|------|----------|
| **Rate Observer** | Monitors DeFi protocol rates, submits observations to ISFR computation. Earns reputation based on accuracy. |
| **Prediction Agent** | Commits predictions on future ISFR values before each update. Scored via CRPS (Continuous Ranked Probability Score). Top-performing agents earn "Oracle" epistemic tier. |
| **Trade Executor** | Opens, manages, and closes yield perp positions on behalf of users based on clearing profiles. |
| **Solver Agent** | Competes to compute optimal clearing solutions within the 800ms window. Earns 5% of clearing surplus, capped at 50 KORAI per batch. Must post a 50,000 KORAI bond. |
| **Liquidation Agent** | Monitors positions approaching maintenance margin. Permissionless -- any agent can liquidate undercollateralized positions and earn a 2% bonus. |
| **Arbitrageur** | Maintains price convergence between the yield perp and ISFR oracle by trading the basis. |

---

## 8. Risk Management

### DeFiRiskEngine parameters

```rust
pub struct DeFiRiskEngine {
    pub max_position_fraction: f64,    // Max position as fraction of portfolio (default 0.10)
    pub max_open_positions: usize,     // Max concurrent positions (default 5)
    pub max_drawdown: f64,             // Portfolio drawdown halt threshold (default 0.10)
    pub max_slippage_bps: f64,         // MEV protection: max slippage (default 50 bps)
    pub use_private_mempool: bool,     // Use Flashbots/private mempool when available
    pub daily_loss_limit_usd: f64,     // Daily realized loss halt threshold (default $500)
}
```

### Risk verdicts

Every trade proposal produces one of three verdicts:

- **Approved** -- trade is within all limits.
- **Rejected** -- trade violates a hard constraint (reason attached).
- **SizeReduced** -- trade is partially acceptable; size reduced from original to adjusted amount with reason.

### Liquidation mechanics

| Parameter | Value |
|-----------|-------|
| Initial margin | 10% of notional |
| Maintenance margin | 5% of notional |
| Liquidation trigger | Equity / notional falls below maintenance margin |
| Liquidation method | Batch (cooperative clearing), not cascade |
| Liquidation bonus | 2% of liquidated margin to the liquidator |
| Insurance fund contribution | 0.5 bps of notional per cleared trade |
| Stale-price guard | Liquidations paused if oracle liveness = Stale or Halted |

Batch liquidation is a deliberate design choice. Cascade liquidations -- where one liquidation triggers another -- amplify market stress. Batch liquidations net positions against each other in the clearing round, reducing market impact.

### Circuit breakers

Three levels of circuit breaker protect the system:

1. **ISFR circuit breaker.** When confidence drops below 70%, ISFR transitions from Live to Degraded or Halted. Clearing spreads widen or trading pauses. Recovery requires confidence above 80% for 30 seconds.

2. **DeFiRiskEngine halt.** When portfolio drawdown exceeds `max_drawdown` or daily realized losses exceed `daily_loss_limit_usd`, the engine halts all trading for the affected agent. Reset occurs at the start of each UTC day (for daily limits) or requires manual intervention (for drawdown).

3. **Clearing fallback ladder.** If no solver submits a valid KKT solution within 800ms, the batch rolls to the next block (retry). After 2 retries, the system falls back to a continuous limit order book (emergency CLOB) where orders match at limit prices without surplus optimization. If ISFR enters Halted state, trading pauses entirely.

### Affect-modulated sizing

Position sizing incorporates prospect theory to prevent agents from doubling down after losses:

```rust
pub fn prospect_value(pnl: f64) -> f64 {
    const LOSS_AVERSION: f64 = 2.25;
    const CURVATURE: f64 = 0.88;
    if pnl >= 0.0 {
        pnl.powf(CURVATURE)
    } else {
        -LOSS_AVERSION * pnl.abs().powf(CURVATURE)
    }
}
```

The resulting affect state produces a size multiplier in [0.25, 1.5]:
- Low arousal, positive valence: full size (1.0-1.5x)
- High arousal, negative valence: reduced size (0.25-0.5x)

---

## 9. The Full Flow: User Intent to On-Chain Settlement

### Step-by-step

**Step 1: User creates clearing profile.** One transaction. Declares: direction (SHORT), trigger (ISFR < 700 bps), max notional ($10M), max fee (10 bps), expiry (180 days). Profile stored on-chain.

**Step 2: Profile sits dormant.** Zero cost. No keeper. The consensus layer checks trigger conditions during each ISFR update (every 10 seconds).

**Step 3: ISFR crosses trigger.** ISFR drops below 700 bps. The clearing engine activates the profile.

**Step 4: Batch accumulation.** The profile's order enters the pending batch along with active limit orders, other triggered profiles, and liquidation orders. The batch accumulates until one of four triggers fires: 5+ orders, 10 seconds elapsed, 3:1 buy/sell imbalance, or 10+ bps ISFR movement since last clearing.

**Step 5: Batch closes.** The order set is sealed and published to solvers.

**Step 6: Solver competition.** Multiple solver agents have 800 milliseconds (2 Korai blocks) to compute the optimal clearing price that maximizes total surplus. Each solver submits a `ClearingSolution` with a KKT certificate proving optimality.

**Step 7: KKT verification.** The chain verifies the solution in O(n) time -- one pass through the order set checking primal feasibility (all constraints satisfied), dual feasibility (shadow prices non-negative), and complementary slackness (partially-filled orders must have limit price equal to clearing price).

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

**Step 8: Settlement.** Positions are created or adjusted. Solver earns `min(TotalSurplus * 0.05, 50 KORAI)`. Insurance fund collects 0.5 bps of notional from each filled order.

**Step 9: ClearingInsight emitted.** A structured knowledge entry is written to the InsightStore, recording clearing price, surplus, fill rates, solve time, solver identity, ISFR at clearing, and spread-to-ISFR. This is the "clearing-as-inference" mechanism: every settlement round produces knowledge.

**Step 10: Prediction scoring.** All agent predictions committed before the batch close are scored via CRPS against the clearing price. Scores adjust 30-day rolling epistemic reputation tiers.

**Step 11: Position management.** The trade executor agent manages the resulting position: monitors margin ratio, adds collateral if needed, participates in 8-hour funding settlements, and closes the position when the clearing profile's deactivation criteria are met.

---

## 10. Market Microstructure

### Order types

Orders enter the clearing engine from three sources:

| Source | Description |
|--------|-------------|
| **Active limit orders** | Submitted by traders with a limit price in basis points. Can be fill-or-kill or partial-fill. |
| **Clearing profile activations** | Triggered automatically when ISFR crosses a profile's threshold. Size is `min(maxNotional - filledSoFar, availableCounterparty)`. |
| **Liquidation orders** | Generated when positions breach maintenance margin. Always partial-fill. Always filled first (priority in clearing). |

### Matching: cooperative clearing, not continuous orderbook

The default matching mechanism is batch cooperative clearing, not a continuous limit order book. Orders accumulate into batches and settle at a single uniform clearing price. This is the critical distinction from exchanges like Hyperliquid, dYdX, or GMX that use continuous matching.

Cooperative clearing produces surplus that is distributed to participants. In a continuous CLOB, market makers capture the spread. In cooperative clearing, the spread is minimized and the surplus is shared: 95% to participants pro-rata, 5% to the winning solver.

### Funding intervals

Funding payments settle every 8 hours, consistent with the standard established by centralized perpetual exchanges:

```
FundingPayment = PositionSize * FundingRate
```

If `FundingRate > 0`: longs pay shorts. If `FundingRate < 0`: shorts pay longs.

### Batch timing

| Trigger | Threshold | Rationale |
|---------|-----------|-----------|
| Order count | 5+ orders | Minimum for meaningful surplus optimization |
| Time elapsed | 10 seconds | Maximum wait for responsiveness |
| Imbalance ratio | 3:1 (buy:sell or inverse) | Market stress requiring immediate clearing |
| ISFR movement | 10+ bps since last clearing | Rate urgency for pending orders |

Settlement pipeline is fixed at 3 blocks (1.2 seconds) after batch completion.

---

## 11. Competitive Landscape

### Pendle

**What it does:** Yield tokenization. Splits yield-bearing assets into Principal Tokens (PT, fixed-rate exposure) and Yield Tokens (YT, variable-rate exposure) with fixed maturities.

**Scale:** $5.7B average TVL in 2025 (peaked $13.4B in September 2025). $47.8B trading volume in 2025. ~$44.6M in fees. 36 employees. Boros extension launched for funding rate trading ($80M open interest, $5.5B notional).

**Structural limitations vs yield perpetuals:**
- Expiring instruments requiring manual rollover (50+ bps per roll, $200K/year on $10M).
- 200+ separate pools fragmenting liquidity.
- No composite benchmark rate -- each pool tracks one asset's yield.
- No leverage (1x only).
- No agent composability -- requires active position management.

### GMX

**What it does:** Decentralized perpetual exchange using a multi-asset liquidity pool (GLP/GM) for settlement.

**Relevance:** GMX trades price-based perpetuals (BTC, ETH, etc.), not rate-based. It does not address interest rate risk. Its oracle design (Chainlink + keepers) is operator-dependent, unlike ISFR's consensus-computed approach.

### Hyperliquid native perps

**What it does:** Centralized-performance decentralized perpetual exchange. Fastest perps throughput in DeFi.

**Relevance:** Hyperliquid's native perps track asset prices, not rates. Yield perpetuals would deploy on Hyperliquid via HIP-3 as a builder-operated market, using Hyperliquid's matching infrastructure but with ISFR as the settlement oracle instead of a spot price feed. The relationship is complementary, not competitive -- Hyperliquid provides the venue, ISFR provides the instrument class it cannot natively serve.

### Drift

**What it does:** Perpetual trading on Solana. Supports price-based perps with a virtual AMM and decentralized orderbook.

**Relevance:** Like GMX and Hyperliquid native markets, Drift trades price perps, not rate perps. No benchmark rate. No interest rate derivative capability. Different chain ecosystem (Solana vs EVM).

### IPOR

**What it does:** Interest rate swaps with a proprietary benchmark index on Ethereum.

**Scale:** ~$10-20M TVL. 12 employees. $5.55M total funding.

**Relevance:** Closest analog to what ISFR + yield perpetuals are building. IPOR publishes an interest rate index and offers swaps against it. Key differences: IPOR uses a flat 3-source average (not a multi-level weighted median), has no Byzantine fault tolerance guarantees, no consensus-level computation, thin liquidity, and no agent-mediated execution layer. IPOR validates the demand; it does not fill the gap at institutional scale.

### Summary comparison

| Property | ISFR + Yield Perps | Pendle | GMX | Hyperliquid | Drift | IPOR |
|----------|-------------------|--------|-----|-------------|-------|------|
| Instrument type | Rate perpetual | Yield tokens (expiring) | Price perpetual | Price perpetual | Price perpetual | Rate swap |
| Benchmark rate | ISFR (consensus-computed) | None | None | None | None | IPOR index (3-source avg) |
| Expiration | Never | Fixed maturity | Never | Never | Never | Fixed maturity |
| Leverage | 10x | 1x | 50x | 50x | 10x | Variable |
| Rollover cost | $0 | 50+ bps/roll | $0 | $0 | $0 | Per maturity |
| Liquidity structure | 1 pool per benchmark | 200+ pools | Multi-asset pool | Orderbook | vAMM + orderbook | Per-tenor |
| Agent integration | Native (clearing profiles) | None | None | None | None | None |
| Settlement verification | KKT certificate (O(n)) | AMM curve | Oracle | Matching engine | vAMM | Protocol-specific |
| Knowledge production | ClearingInsight per batch | None | None | None | None | None |

---

## 12. Rust Implementation: The ISFR Registry

The Rust implementation in `roko-chain` provides an `IsfrRegistry` that manages the full ISFR clearing cycle. Key types from the implementation:

### Configuration

```rust
pub struct IsfrConfig {
    pub epoch_duration_secs: u64,          // Default: 28,800 (8 hours)
    pub max_kkt_residual: f64,             // Default: 1e-6
    pub min_submissions_for_clearing: usize, // Default: 2
    pub min_reputation: f64,               // Default: 0.5
    pub max_rate_bound: Option<f64>,       // Default: Some(0.1)
    pub outlier_sigma: f64,                // Default: 3.0
}
```

### Clearing cycle state machine

The ISFR clearing cycle proceeds through six phases:

```
COMMIT -> REVEAL -> SOLVE -> CERTIFICATE -> VERIFY -> SETTLE
```

Each phase consumes a fraction of the epoch (default: Commit 40%, Reveal 15%, Solve 15%, Certificate 10%, Verify 10%, Settle 10%).

### Submission and aggregation

```rust
pub struct IsfrSubmission {
    pub market_id: MarketId,              // Hierarchical market ID (e.g., "knowledge/defi")
    pub rate: f64,                        // Observed rate
    pub components: Vec<f64>,             // Rate components (must sum to rate)
    pub confidence: f64,                  // Submitter confidence [0, 1]
    pub submitter_passport_id: u256,      // ERC-8004 passport
    pub submitted_at_block: u64,
}

pub struct IsfrAggregate {
    pub median_rate: f64,
    pub submission_count: usize,
    pub std_deviation: f64,
    pub excluded_count: usize,            // Outliers excluded (> 3-sigma)
    pub market_id: MarketId,
    pub epoch: u64,
    pub clearing_block: u64,
}
```

The aggregation algorithm is a two-pass weighted median:
1. Compute initial weighted median (weight = submitter confidence * reputation).
2. Compute standard deviation; exclude submissions beyond 3-sigma.
3. Recompute weighted median on filtered set.

This produces a rate that is robust to both low-confidence submissions and outlier manipulation attempts.

---

## Appendix: Event Type Reference

| Event | Emitted By | Consumed By |
|-------|------------|-------------|
| `isfr.updated` | ISFROracle / ChainDataAggregator | Dashboard, yield perp mark-to-market, agent context |
| `isfr.deviation_triggered` | ISFROracle | Dashboard alert, clearing house |
| `position.opened` | ClearingHouse | Dashboard, risk engine, TradingReflect |
| `position.closed` | ClearingHouse | Dashboard, TradingReflect, learning pipeline |
| `position.liquidated` | ClearingHouse | Dashboard alert, risk engine |
| `clearing.started` | ClearingHouse | Dashboard, agent trading logic |
| `clearing.settled` | ClearingHouse | Dashboard, TradingReflect |
| `risk.drawdown_warning` | DeFiRiskEngine | Dashboard alert, agent trading logic |
| `risk.mev_detected` | MEV detection module | Dashboard alert, risk engine |
| `risk.daily_limit_hit` | DeFiRiskEngine | Dashboard alert, trading halt |
| `chain.health_changed` | ChainDataAggregator | Dashboard, ISFR oracle |
| `collateral.added` | ClearingHouse | Dashboard, risk engine |
| `collateral.removed` | ClearingHouse | Dashboard, risk engine |
