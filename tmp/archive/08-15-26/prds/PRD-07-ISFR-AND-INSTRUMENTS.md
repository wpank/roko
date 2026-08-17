# PRD-07: ISFR and instruments

**Status:** Draft
**Author:** Will
**Date:** 2026-04-21
**Crates affected:** `roko-chain` (ISFR oracle, clearing engine), `roko-core` (benchmark trait), `roko-agent` (prediction commitment), `roko-learn` (CRPS scoring integration)

---

## Table of contents

1. [The problem: DeFi has no standard reference rate](#1-the-problem-defi-has-no-standard-reference-rate)
2. [What is ISFR?](#2-what-is-isfr)
3. [V1 source composition and weighting](#3-v1-source-composition-and-weighting)
4. [Computation pipeline](#4-computation-pipeline)
5. [Publication states and circuit breakers](#5-publication-states-and-circuit-breakers)
6. [ISFR as knowledge production](#6-isfr-as-knowledge-production)
7. [Yield perpetuals](#7-yield-perpetuals)
8. [Worked hedging examples](#8-worked-hedging-examples)
9. [Clearing profiles](#9-clearing-profiles)
10. [Cooperative clearing](#10-cooperative-clearing)
11. [Why yield perpetuals vs Pendle](#11-why-yield-perpetuals-vs-pendle)
12. [Generalized benchmark framework](#12-generalized-benchmark-framework)
13. [ISFR path to credibility](#13-isfr-path-to-credibility)
14. [ISFR vs SOFR comparison](#14-isfr-vs-sofr-comparison)
15. [Solver economics and anti-gaming](#15-solver-economics-and-anti-gaming)
16. [Academic citations](#16-academic-citations)
17. [Korai integration gaps (ISFR-specific)](#17-korai-integration-gaps-isfr-specific)
18. [ISFR as EventFabric source](#18-isfr-as-eventfabric-source)
19. [Cross-domain ISFR usage](#19-cross-domain-isfr-usage)
20. [Agent roles in the ISFR ecosystem](#20-agent-roles-in-the-isfr-ecosystem)
21. [Multi-chain ISFR sources](#21-multi-chain-isfr-sources)

---

## 1. The problem: DeFi has no standard reference rate

### SOFR and the $668 trillion benchmark

In traditional finance, almost nothing works without a benchmark interest rate. The Secured Overnight Financing Rate (SOFR) is published daily by the Federal Reserve Bank of New York. It measures the cost of borrowing cash overnight, collateralized by U.S. Treasury securities. The rate is computed from roughly $2 trillion in daily transaction volume across three segments of the overnight repo market: tri-party repo, GCF repo, and bilateral Treasury repo.

SOFR replaced LIBOR (London Interbank Offered Rate) after the 2012 rate-rigging scandal, in which traders at multiple global banks manipulated LIBOR submissions for profit. The transition took years and cost the industry billions. The lesson was permanent: a benchmark rate that depends on voluntary submissions from interested parties will be gamed. SOFR's methodology -- transaction-weighted median of actual overnight repo trades, computed by the Federal Reserve from clearing data -- was designed to be manipulation-resistant by construction.

The instruments that depend on SOFR are staggering in scale:

| Instrument class | Notional outstanding |
|-----------------|---------------------|
| Interest rate swaps | ~$400T |
| Futures and options (CME) | ~$120T |
| Floating-rate notes | ~$48T |
| Adjustable-rate mortgages | ~$2.5T |
| Corporate floating-rate debt | ~$1.8T |
| **Total** | **~$570T+** |

When the Bank for International Settlements reported $668 trillion in total interest rate derivative notional in 2024, that number was anchored almost entirely by benchmark reference rates. Without SOFR, none of these instruments can be priced, hedged, or settled.

The causal chain is worth stating explicitly: **benchmark rate -> derivative pricing -> hedging -> risk management -> capital efficiency -> market depth -> lower borrowing costs**. Remove the benchmark and the entire chain collapses. This is not theoretical. The LIBOR transition proved it -- when confidence in the benchmark wavered, trillions in notional faced repricing uncertainty.

### DeFi's structural gap

DeFi has no equivalent to SOFR. The consequences are visible in the data.

On-chain interest rate derivative TVL: under $100 million. TradFi interest rate derivative notional: $668 trillion. That is a six-order-of-magnitude gap. It is not a marketing failure or a timing issue. It is the absence of a foundational primitive.

The protocols that exist today demonstrate demand while failing to fill the gap:

| Protocol | TVL | What it offers | Why it falls short |
|----------|-----|---------------|-------------------|
| Pendle | ~$1.9B | Yield tokenization with fixed-maturity PT/YT tokens | Expiring instruments fragment liquidity across hundreds of pools. No benchmark rate. No leverage. Rollover costs 50+ bps per maturity cycle. |
| IPOR | ~$10-20M | Interest rate swaps with a proprietary index | Single-methodology index without Byzantine tolerance. Thin liquidity. Limited adoption as a reference. |
| Spectra | ~$44M | Yield tokenization with Curve integration | Same expiration and fragmentation constraints as Pendle. |
| Voltz | -- | IR swaps on concentrated liquidity AMM | Shut down December 2023. Demonstrated that the AMM model does not work for rates. |

The gap is structural, not competitive. These protocols cannot solve the problem because the problem is not "build a better swap." The problem is that DeFi lacks a credible, manipulation-resistant, continuously-published, consensus-computed benchmark rate that derivatives can settle against.

### Why individual protocol rates are not benchmarks

Aave publishes Aave's supply rates. Compound publishes Compound's supply rates. These rates reflect protocol-specific dynamics: utilization curves, governance parameters, liquidity incentives, and pool-specific risk premia. They are not benchmarks. They are individual venue quotes.

A benchmark rate requires specific properties that no individual protocol rate possesses:

1. **Multi-source aggregation.** A single source is a quote. A benchmark aggregates across venues to represent the broader market.
2. **Manipulation resistance.** Individual protocol rates can be moved by flash loans, governance attacks, or liquidity manipulation. A benchmark must absorb these distortions.
3. **Independent computation.** If the entity publishing the rate is also a market participant, conflicts of interest are structural. LIBOR proved this.
4. **Continuous availability.** A rate that goes offline during market stress -- exactly when it is most needed -- is not a benchmark.
5. **Standardized methodology.** Different protocols compute rates differently. Aave's utilization curve differs from Compound's. Without a standardized aggregation methodology, rates are not comparable.

DeFi has individual rates. It does not have a benchmark. Every dollar of the ~$50 billion in DeFi lending TVL carries unhedged variable rate exposure because there is nothing to hedge against.

---

## 2. What is ISFR?

The Internet Secured Funding Rate is a composite benchmark index representing the cost of secured funding across DeFi. It is to DeFi what SOFR is to TradFi: the reference rate that financial instruments settle against.

### Defining properties

**Consensus-level computation.** Every Korai validator independently computes ISFR as part of block production. The computation happens every 25 blocks, which at Korai's 400ms block time yields an update cadence of approximately 10 seconds. There is no separate oracle operator, no off-chain infrastructure, no bridge dependency. ISFR is a property of the chain itself.

**Dual median aggregation.** Two layers of median-based aggregation provide Byzantine fault tolerance at both the source level and the validator level.

- Layer 1 (source aggregation): each validator reads rates from multiple DeFi protocols and computes a weighted median across sources.
- Layer 2 (validator aggregation): the chain computes a stake-weighted median across all validator submissions.

This dual-layer design tolerates up to 49% compromised stake at the validator level AND up to 49% compromised weight at the source level. Both layers must be simultaneously compromised to move ISFR to an arbitrary value.

**No oracle operator.** ISFR is published via a dedicated precompile at address `0xA01` on the Kernel Plane. Any smart contract on Korai can read the current ISFR value with a single precompile call. There is no Chainlink dependency, no multisig, no oracle committee. The rate is as available as the chain itself.

**8,640x higher frequency than SOFR.** SOFR publishes once per day at 8:00 AM Eastern. ISFR publishes every 10 seconds. That is 8,640 updates per day versus 1. In markets where rates can move hundreds of basis points within hours (DeFi lending rate spikes during high-utilization events are not rare), a daily rate is a relic. A 10-second rate captures the dynamics that matter.

### What ISFR is not

ISFR is not a lending rate. It does not tell you what rate you will receive on Aave or Compound. It is a benchmark -- a composite measure of the broad market that serves as a reference point for derivative pricing, hedging, and settlement. The distinction matters. Individual lending rates are venue-specific quotes. ISFR is the market-level signal that those quotes collectively produce.

ISFR is not an oracle feed. Oracle feeds push external data onto a chain. ISFR is computed by the chain's own validators from data they independently observe. The chain does not need to trust an external data provider. Each validator can verify the inputs.

ISFR is not a governance-managed parameter. No DAO vote sets the rate. The methodology is fixed in the protocol specification. Governance can adjust source weights (within bounds) and add or remove sources in V2, but cannot override the computed value.

---

## 3. V1 source composition and weighting

### The four sources

V1 uses four sources, each weighted equally at 0.25:

| # | Source | What it measures | Update frequency | Why included |
|---|--------|-----------------|-----------------|--------------|
| 1 | Aave V3 (Ethereum mainnet) | USDC supply APY | Per-block (every new deposit/withdrawal/liquidation recalculates) | Largest lending protocol by TVL. ~$10B+ in TVL. Broad market signal. |
| 2 | Compound V3 (Ethereum mainnet) | USDC supply APY | Per-block | Second-largest lending protocol. Different utilization curve from Aave. Provides diversification. |
| 3 | Ethena (sUSDe) | 7-day rolling yield | Updated as funding rates settle | Represents delta-neutral funding rate exposure. Captures a different yield source (perpetual funding) than lending protocols. |
| 4 | ETH Beacon Chain | Consensus rewards + MEV tips as annualized yield | Per-epoch (every 6.4 minutes, ~225 epochs/day) | The most decentralized yield source in crypto. ~$115B+ staked. Resistant to single-entity manipulation. |

### Why these four

Each source captures a distinct yield generation mechanism:

- **Aave and Compound** measure the cost of borrowing USDC against crypto collateral. This is the closest DeFi analog to overnight secured lending rates in TradFi.
- **Ethena** measures the funding rate on perpetual futures positions. This captures speculative sentiment and leverage demand -- a signal that pure lending rates miss.
- **ETH staking yield** measures the base cost of securing the Ethereum network. It is the "risk-free" rate of crypto, analogous to the Treasury yield in TradFi.

Together, they span the primary yield generation mechanisms in DeFi: lending, funding rates, and staking. No single-venue shock can dominate the composite.

### Why equal weighting

Equal weights (0.25 per source) are a deliberate design choice, not a default.

**Manipulation resistance.** With equal weights, an attacker must corrupt at least 50% of source weight (two of four sources) to move the median to an arbitrary value. If weights were TVL-proportional, Aave's ~60% market share would give it ~60% influence. Corrupting one source would be sufficient to move the benchmark.

**No governance surface.** TVL-proportional weights change every block as deposits and withdrawals shift TVL across protocols. This creates a governance surface: which TVL measurement methodology? What snapshot cadence? How to handle flash-loan-inflated TVL? Equal weights eliminate these questions entirely.

**Intellectual honesty.** The correct weights for a benchmark are an empirical question that requires years of data to answer. Pretending to have the answer on day one would be false precision. Equal weights are the maximum-entropy prior -- the weighting that assumes the least about relative importance.

### V2 source evolution

V2 introduces governance-adjustable weights with the following constraints:

- Maximum weight per source: 0.35 (no single source can exceed 35% influence)
- Minimum weight per source: 0.05 (no source can be effectively zeroed without removal)
- Weight changes require a 7-day timelock and super-majority governance vote
- Additional V2 sources under consideration:

| Candidate source | What it measures | Rationale |
|-----------------|-----------------|-----------|
| Morpho (Ethereum) | Optimized lending yield (USDC) | Growing TVL, different liquidation mechanism |
| MakerDAO DSR | Dai Savings Rate | Governance-set rate, represents MKR holder's view of cost of capital |
| Spark (Aave V3 fork) | USDC supply APY | Additional lending data point |

V2 expands the source set to a minimum of 7, further increasing manipulation resistance. With 7 equally-weighted sources, an attacker must corrupt 4 sources simultaneously to move the median arbitrarily.

---

## 4. Computation pipeline

### Step-by-step with a worked example

Suppose at block height 1,000,025 (the 25th block, triggering an ISFR update), the four sources report:

| Source | Rate |
|--------|------|
| Aave V3 USDC supply APY | 4.50% |
| Compound V3 USDC supply APY | 5.80% |
| Ethena sUSDe 7-day rolling yield | 6.20% |
| ETH Beacon Chain staking yield | 7.10% |

**Step 1: Source reading.** Each validator independently reads the four rates. Validators read from their own Ethereum full nodes (for Aave, Compound, and Beacon Chain data) and from Ethena's published sUSDe rate. No shared data source. No single point of failure.

**Step 2: Sort ascending.**

```
4.50%, 5.80%, 6.20%, 7.10%
```

**Step 3: Accumulate weights.**

| Rate | Weight | Cumulative weight |
|------|--------|-------------------|
| 4.50% | 0.25 | 0.25 |
| 5.80% | 0.25 | 0.50 |
| 6.20% | 0.25 | 0.75 |
| 7.10% | 0.25 | 1.00 |

**Step 4: Find the 0.50 percentile (median).** The cumulative weight reaches 0.50 at the boundary between 5.80% and 6.20%. For an even split at the median boundary, take the arithmetic mean of the two bracketing values.

**Step 5: Compute ISFR.**

```
ISFR = (5.80% + 6.20%) / 2 = 6.00%
```

The ISFR for this update is **6.00% (600 basis points)**.

### Why the median resists manipulation

Consider an attack: a flash loan spikes the Aave USDC supply rate to 50% for a single block.

**Rates after attack:**

```
5.80%, 6.20%, 7.10%, 50.00%
```

(Aave moved from 4.50% to 50.00%; other sources unaffected.)

**Median computation:** Cumulative weights reach 0.50 between 6.20% and 7.10%.

```
ISFR_attacked = (6.20% + 7.10%) / 2 = 6.65%
```

**Impact:** ISFR shifted from 6.00% to 6.65%. A 65 basis point move. The attacker spent the cost of a flash loan (gas + protocol fees) to produce a transient 65 bps distortion that will correct on the next update 10 seconds later.

**If ISFR used the mean instead:**

```
mean = (5.80% + 6.20% + 7.10% + 50.00%) / 4 = 17.28%
```

The mean-based rate would have jumped from 5.90% to 17.28% -- a 1,138 bps spike, nearly tripling the index. The median absorbed the outlier. The mean amplified it.

This is why every credible benchmark uses a median (or trimmed mean), not a simple average. SOFR uses a volume-weighted median for the same reason.

### Validator-level aggregation

Each of the N validators on the Korai network independently computes the source-level median and submits it as an `OracleVote`:

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

The chain aggregates votes using a **stake-weighted median**. If validator A controls 10% of total stake and validator B controls 5%, validator A's vote receives twice the weight of B's.

Formally: let V = {(v_i, s_i)} be the set of validator votes, where v_i is the submitted ISFR value and s_i is the validator's stake weight (normalized so that the sum of all s_i = 1). Sort votes by value. The aggregate ISFR is the value v_j such that:

```
sum(s_i for i <= j) >= 0.50 and sum(s_i for i < j) < 0.50
```

If the 0.50 boundary falls exactly between two votes, interpolate linearly.

### Two-layer Byzantine tolerance

The dual-median design provides independent Byzantine fault tolerance at each layer:

**Layer 1 (sources):** The source-level weighted median tolerates manipulation of up to floor(k/2) sources, where k is the number of sources. With 4 equally-weighted sources, an attacker must corrupt 2 sources to move the median arbitrarily. With 7 V2 sources, the threshold rises to 4.

**Layer 2 (validators):** The validator-level stake-weighted median tolerates up to 49% compromised stake. An attacker controlling 30% of stake cannot move the validator median beyond the range of honest validators' submissions.

**Combined:** To manipulate ISFR to an arbitrary value, an attacker must compromise 50%+ of source weight AND 50%+ of validator stake simultaneously. Either layer alone stops the attack.

### On-chain storage

The ISFR precompile at `0xA01` stores:

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

Historical values are retained for 90 days (approximately 19.4 million snapshots at 10-second cadence). Any smart contract can read the current or historical ISFR by calling the precompile:

```solidity
// Read current ISFR
(uint32 valueBps, uint8 state) = ISFROracle(0xA01).current();

// Read historical ISFR at a specific block
ISFRSnapshot memory snap = ISFROracle(0xA01).at(blockHeight);

// Read TWAP over a time range
uint32 twapBps = ISFROracle(0xA01).twap(startBlock, endBlock);
```

---

## 5. Publication states and circuit breakers

### Four states

ISFR operates in one of four states at any given time. The state is determined automatically by the consensus layer based on data availability and validator agreement.

| State | Condition | Behavior |
|-------|-----------|----------|
| **Live** | 3+ sources reporting AND confidence >= 70% | Normal publication. All instruments settle against the live rate. |
| **Degraded** | Exactly 2 sources reporting OR confidence between 50-70% | Rate still published but with a wider confidence interval. Instruments settle normally, but clearing profiles with tight triggers may pause. |
| **Stale** | Exactly 1 source reporting | Rate frozen at last known Live or Degraded value. Clearing continues using the stale rate. New clearing profile activations are suspended. |
| **Halted** | 0 sources reporting OR confidence < 50% OR consensus failure | Rate frozen. Clearing falls back to the emergency CLOB (continuous limit order book). No new positions can be opened. Existing positions are not liquidated. |

### Confidence score

Confidence measures the degree of validator agreement. It is the percentage of validator stake that submitted votes within one standard deviation of the stake-weighted median.

```
confidence = sum(s_i for all i where |v_i - median| <= sigma) / sum(all s_i)
```

Where sigma is the standard deviation of all submitted votes weighted by stake.

High confidence (>90%) means validators substantially agree on the rate. Low confidence (<70%) means significant disagreement -- possibly due to stale data, network partitions, or attempted manipulation.

### Circuit breaker trigger

When confidence drops below 70%, the ISFR circuit breaker fires:

1. ISFR state transitions from Live to Degraded (if >=50%) or Halted (if <50%).
2. The previous Live rate is cached as the fallback.
3. An `ISFRCircuitBreaker` event is emitted on-chain, triggering monitoring alerts.
4. The clearing engine switches to wider spread limits (Degraded) or emergency CLOB mode (Halted).
5. Recovery requires confidence to exceed 80% for 3 consecutive update periods (30 seconds) before transitioning back to Live. The hysteresis (70% down, 80% up) prevents oscillation.

### Source liveness detection

Each source has a liveness timeout:

| Source | Expected update frequency | Liveness timeout |
|--------|--------------------------|------------------|
| Aave V3 | Every Ethereum block (~12s) | 120 seconds (10 missed blocks) |
| Compound V3 | Every Ethereum block (~12s) | 120 seconds |
| Ethena sUSDe | As funding settles (~8 hours) | 24 hours |
| ETH Beacon Chain | Per epoch (~6.4 minutes) | 30 minutes (5 missed epochs) |

When a source exceeds its liveness timeout, validators exclude it from their computation and report the reduced source count. The state machine transitions accordingly (3+ sources = Live, 2 = Degraded, 1 = Stale, 0 = Halted).

---

## 6. ISFR as knowledge production

### Every update is an epistemic event

Most oracle systems produce a single output: a number. ISFR produces both a number and a structured knowledge artifact. Every ISFR update generates an `ISFRInsight` entry:

```rust
struct ISFRInsight {
    /// The computed ISFR value in basis points.
    value_bps: u32,

    /// Per-source rates used in computation.
    source_rates: Vec<SourceRate>,

    /// Validator agreement statistics.
    confidence: f64,
    validator_spread_bps: u32,

    /// Rate of change from previous update.
    delta_bps: i32,

    /// 1-hour, 24-hour, and 7-day moving averages.
    ma_1h_bps: u32,
    ma_24h_bps: u32,
    ma_7d_bps: u32,

    /// Regime classification.
    regime: RateRegime,    // Stable, Rising, Falling, Volatile, Crisis

    /// Timestamp and block reference.
    timestamp: u64,
    block_height: u64,
}
```

These insights feed the Korai InsightStore. Agents can query historical ISFR patterns, rate regime transitions, source divergence events, and moving average crossovers. The oracle is not a data pipe. It is an epistemic engine.

### Prediction commitment and scoring

Before each ISFR update, agents can commit predictions to the `PROOF_LOG` precompile at `0xA04`:

```rust
struct ISFRPrediction {
    /// Agent's ERC-8004 passport address.
    agent: Address,
    /// Predicted ISFR value (basis points).
    predicted_bps: u32,
    /// Predicted confidence interval width (basis points).
    confidence_interval_bps: u32,
    /// Block height this prediction targets.
    target_block: u64,
    /// Hash commitment (revealed after ISFR publication).
    commitment: B256,
}
```

Predictions are scored using CRPS (Continuous Ranked Probability Score), a strictly proper scoring rule. "Strictly proper" means the unique optimal strategy is to report your true belief. No meta-strategy -- hedging, sandbagging, anchoring to others -- beats honest reporting. The mathematical guarantee ensures that the scoring system cannot be gamed.

CRPS for a point prediction p against outcome y:

```
CRPS(p, y) = |p - y|
```

For a distributional prediction with CDF F against outcome y:

```
CRPS(F, y) = integral from -inf to +inf of [F(x) - 1(x >= y)]^2 dx
```

Lower CRPS is better. A perfect prediction scores 0.

### 8,640 scoring events per day

At 10-second update cadence, ISFR produces 8,640 updates per day. Each update can score every committed prediction. For an agent committing predictions to every update, that is 8,640 calibration signals per day -- compared to 1 per day for SOFR.

This density matters. Calibration is a statistical property. More scored predictions produce tighter confidence intervals around the agent's true calibration. An agent that predicts 8,640 times per day reaches statistical significance on calibration changes within hours, not months.

### Epistemic reputation tiers

Prediction accuracy over a rolling 30-day window determines each agent's epistemic reputation tier:

| CRPS percentile | Tier | Knowledge query quota | Clearing priority | Other benefits |
|-----------------|------|----------------------|-------------------|----------------|
| Top 10% | Oracle | 2x base | First | Priority solver submission; governance weight |
| 10-30% | Calibrated | 1.5x base | Standard | -- |
| 30-70% | Standard | 1x base (100 queries/day) | Standard | -- |
| 70-100% | Uncalibrated | 0.5x base | Last | -- |

Reputation decays with a 30-day half-life. An Oracle-tier agent that stops predicting will drop to Standard within 60 days. Tier is earned, not purchased.

The mechanism is self-selecting: agents that are good at predicting rates get more access to rate-related knowledge, which makes them better at predicting rates, which raises their tier further. Agents that are bad at it lose access, which reduces noise in the knowledge substrate. Quality begets quality.

---

## 7. Yield perpetuals

### What they are

A yield perpetual is a perpetual futures contract where the underlying is a **yield rate measured in basis points**, not an asset price. It settles against ISFR. It has no expiration date and requires no rollover.

This is a category-level distinction. Standard perpetual futures (on Binance, dYdX, Hyperliquid) track asset prices: the price of BTC, the price of ETH. Yield perpetuals track interest rates: the cost of secured funding in DeFi. The underlying is a percentage, not a dollar amount.

The instrument format is not novel. CME's SOFR futures and Eurodollar futures have traded rate-based contracts for decades. What is novel is applying the perpetual structure -- no expiry, continuous funding, single pool per benchmark -- to on-chain rate trading, with settlement against a consensus-computed benchmark.

### Contract specification

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| **Underlying** | DeFi yield rate via ISFR | Observable, manipulation-resistant, consensus-computed |
| **Quote unit** | Basis points (1 bp = 0.01%) | Industry convention for rate products |
| **Contract multiplier** | $1 notional per 1 bp per unit | Linear payoff. Essential for KKT verification. |
| **Minimum tick** | 0.25 bp (0.0025%) | Matches CME SOFR futures tick size |
| **Lot size** | 1 unit ($1/bp) | Low barrier to entry |
| **Max leverage** | 10x (10% initial margin, 5% maintenance margin) | Conservative for rate markets. Rate volatility is lower than asset price volatility. |
| **Funding interval** | 8 hours | Standard perpetual funding cadence. 3 funding events per day. |
| **Settlement** | Cooperative clearing (batch, KKT-verified) | Mathematical optimality. No trust required. |
| **Trading hours** | 24/7/365 | DeFi rates move continuously. No close-of-business. |

### Payoff structure

The payoff is linear in basis points:

```
PnL per unit = (Exit_bps - Entry_bps) * Direction * $1
```

Where:
- `Exit_bps` is the ISFR value (in basis points) at exit
- `Entry_bps` is the ISFR value at entry
- `Direction` is +1 for long, -1 for short

**Example:** Enter long at 600 bps (6.00%). ISFR rises to 650 bps (6.50%). Position size: 1,000 units.

```
PnL = (650 - 600) * (+1) * $1 * 1,000 = $50,000
```

**Example:** Enter short at 600 bps. ISFR drops to 450 bps. Position size: 500 units.

```
PnL = (600 - 450) * (+1) * $1 * 500 = $75,000
```

(Direction is factored into the formula: for shorts, `(Entry - Exit)` is equivalent to `(Exit - Entry) * -1`.)

The linear payoff is a design requirement, not a simplification. Convex or concave payoffs break the KKT conditions that make cooperative clearing provably optimal. Linearity preserves convexity of the clearing optimization problem, which guarantees that the KKT solution is globally optimal.

### Mark price

The mark price determines unrealized PnL and liquidation triggers. It blends the oracle rate with the order book to prevent manipulation of either component:

```
MarkPrice = 0.7 * ISFR_Oracle + 0.3 * EMA(OrderBook_MidPrice, 300s)
```

Where:
- `ISFR_Oracle` is the latest ISFR value from the `0xA01` precompile
- `EMA(OrderBook_MidPrice, 300s)` is the 300-second exponential moving average of the order book mid price

The 70/30 split ensures that the oracle dominates (preventing order book manipulation) while the order book component allows the mark price to reflect genuine supply/demand dynamics that have not yet been captured in the next ISFR update.

If ISFR enters Degraded or Stale state, the mark price formula adjusts:
- Degraded: `0.9 * ISFR_Oracle + 0.1 * EMA(MidPrice, 600s)` (longer EMA, more oracle weight)
- Stale: `1.0 * Last_Live_ISFR` (pure oracle, frozen)
- Halted: Mark price frozen. No new liquidations. Emergency CLOB only.

### Funding rate

The funding rate anchors the perpetual price to the underlying ISFR. Without funding, the perpetual could diverge arbitrarily from the benchmark. The funding rate has two components:

```
FundingRate = PremiumComponent + CarryComponent
```

**Premium component.** This is the standard perpetual funding mechanism:

```
PremiumComponent = clamp(
    EMA(MidPrice - ISFR, 300s) / ISFR,
    -0.05%,    // floor: -5 bps per 8-hour period
    +0.05%     // cap: +5 bps per 8-hour period
)
```

When the perpetual trades above ISFR (positive premium), longs pay shorts. When below (negative premium), shorts pay longs. This creates arbitrage pressure that pulls the perpetual price toward the benchmark.

**Carry component.** Unique to rate-based perpetuals. The carry component prevents a free arbitrage from forming when the yield curve is not flat:

```
CarryComponent = (ISFR - RiskFreeRate) * (FundingInterval / Year)
```

Where `RiskFreeRate` is the ETH staking yield (source #4 in the ISFR computation). The carry component captures the cost of holding a rate-based position and prevents the funding rate from systematically transferring value between longs and shorts when the rate environment is stable.

**Payment.** Every 8 hours, funding payments are computed and applied:

```
FundingPayment = PositionSize * FundingRate
```

If `FundingRate > 0`: longs pay shorts.
If `FundingRate < 0`: shorts pay longs.

### Position semantics

**Long (ISFR will rise).** A long position profits when DeFi yields increase. Use case: speculating that lending demand will increase, that leverage will expand, or that staking yields will rise.

**Short (ISFR will fall).** A short position profits when DeFi yields decrease. This is the core hedging use case. A treasury holding yield-bearing positions on Aave shorts the yield perpetual to lock in a minimum effective rate. If rates drop, the short position's gains offset the treasury's reduced yield income.

### Margin and liquidation

| Parameter | Value |
|-----------|-------|
| Initial margin | 10% of notional |
| Maintenance margin | 5% of notional |
| Liquidation trigger | Mark-to-market loss exceeds (margin - maintenance) |
| Liquidation method | Cooperative clearing (batch) |
| Insurance fund contribution | 0.5 bps per cleared trade |

**Liquidation example:** A trader opens a long position with $10,000 notional at 10x leverage ($1,000 margin). Initial margin: $1,000. Maintenance margin: $500.

```
Available margin before liquidation = $1,000 - $500 = $500
Liquidation triggers when mark-to-market loss exceeds $500.
At $1/bp per unit, with 10,000 units: $500 loss = 50 bp adverse move.
```

If the trader is long at 600 bps and the mark price drops below 550 bps, the position is liquidated through the next cooperative clearing round.

---

## 8. Worked hedging examples

### Example 1: $10M DAO treasury

**Situation.** A DAO holds $10 million USDC deposited on Aave V3, currently earning 8.00% APY. The treasury manager is concerned about rate compression over the next 6 months. If rates drop to 3.00%, the treasury loses $250,000 in expected yield over six months.

**Action.** The treasury creates a clearing profile (see section 9):
- Direction: SHORT
- Trigger: ISFR < 700 bps (7.00%)
- Max notional: $10,000,000
- Max fee: 10 bps
- Expiry: 180 days

This is one transaction. The profile sits on-chain with zero carrying cost until the trigger fires.

**Scenario: rates drop.** Over the next 3 months, ISFR gradually declines from 8.00% to 3.00%. The clearing profile activates when ISFR crosses 7.00%.

Perp position: SHORT, entered at 693 bps (the clearing price when the profile activated). Final exit at 300 bps.

```
Position size (effective): $10,000,000 / ($1/bp) = 10,000 units
Approximate size matched through clearing: conservative fill = 5,000 units

PnL on perp = (693 - 300) * $1 * 5,000 = $1,965,000
```

Meanwhile, the Aave position's yield dropped from 8.00% to an average of ~5.00% over 6 months:

```
Expected yield at 8.00%: $10,000,000 * 8.00% * (180/365) = $394,521
Actual yield at average 5.00%: $10,000,000 * 5.00% * (180/365) = $246,575
Yield shortfall: $147,946
```

The perp's $1,965,000 gain far exceeds the $147,946 yield shortfall. The hedge was effective.

**Funding cost.** Assume average funding rate of 0.01% per 8-hour interval:

```
Funding cost over 90 days = 5,000 units * 693 bps * 0.01% * (90 * 3) = ~$935
```

Total hedge cost: $935 in funding. Against $1,965,000 in gains. Cost basis: 0.05%.

**Scenario: rates stay high.** ISFR never drops below 7.00%. The clearing profile never activates. The treasury earned 8.00% APY on $10M. The cost of the hedge: $0. The profile was free insurance.

### Example 2: $50M institutional portfolio

**Situation.** An institutional fund manages $50 million across three yield protocols:
- $20M on Aave V3 at 7.50% APY
- $15M on Compound V3 at 7.80% APY
- $15M on Morpho at 8.20% APY

The blended yield is 7.77% APY. The fund's mandate requires a minimum 5.00% annual return.

**Action.** The fund creates a clearing profile:
- Direction: SHORT
- Trigger: ISFR < 550 bps (5.50% -- a 50 bps buffer above the minimum)
- Max notional: $50,000,000
- Max fee: 8 bps
- Expiry: 365 days

**Scenario: prolonged rate compression.** Over 12 months, ISFR declines from 7.77% to 2.50%. The profile activates at 543 bps (clearing price when trigger fired).

```
Effective position: 20,000 units (partial fill -- 40% of max)
PnL on perp = (543 - 250) * $1 * 20,000 = $5,860,000
```

The portfolio's actual yield income over 12 months at a blended average of 4.50%:

```
Yield income = $50,000,000 * 4.50% = $2,250,000
```

Without the hedge, the fund returns 4.50% -- below its 5.00% mandate. With the hedge:

```
Total return = ($2,250,000 + $5,860,000) / $50,000,000 = 16.22%
```

Funding cost over the active period (~9 months):

```
Funding = 20,000 * 543 * 0.01% * (270 * 3) = ~$8,786
```

The hedge cost $8,786 and generated $5,860,000. The fund comfortably exceeded its 5.00% mandate.

### What these examples demonstrate

1. **One-action setup.** Both hedges required a single transaction to create the clearing profile. No ongoing management.
2. **Zero cost if not needed.** If rates stay above the trigger, the profile never activates. The hedge is free.
3. **Automatic execution.** No monitoring required. The profile activates at the consensus level when ISFR crosses the threshold.
4. **Leverage-appropriate.** The hedge does not need to be 1:1 with the underlying position. The clearing profile's max notional and the cooperative clearing process determine the actual fill.

---

## 9. Clearing profiles

### The one-action hedge

A clearing profile is a persistent, on-chain intent that sits dormant until market conditions activate it. It is the mechanism that makes "set it and forget it" hedging possible.

The intent is specific: the user declares their desired position, the conditions under which it should activate, and the constraints on execution. The chain handles everything else.

### Data structure

```solidity
struct ClearingProfile {
    /// Owner's address (EOA or smart contract).
    address account;

    /// Market identifier (e.g., keccak256("ISFR-PERP-V1")).
    bytes32 market;

    /// Direction: 0 = LONG, 1 = SHORT.
    Direction direction;

    /// ISFR threshold in basis points that activates the profile.
    /// For SHORT: activates when ISFR < trigger.
    /// For LONG: activates when ISFR > trigger.
    uint256 trigger;

    /// Maximum notional exposure in USD (scaled by 1e18).
    uint256 maxNotional;

    /// Maximum acceptable clearing fee in basis points.
    /// Profile skips a clearing round if the solver's fee exceeds this.
    uint16 maxFeeBps;

    /// Expiry timestamp. 0 = no expiry (profile persists until cancelled).
    uint64 expiry;

    /// Minimum fill size per clearing round (prevents dust fills).
    uint256 minFillNotional;

    /// Maximum number of clearing rounds to participate in.
    /// 0 = unlimited. Useful for one-shot hedges.
    uint32 maxRounds;
}
```

### Lifecycle

1. **Creation.** User submits a transaction to the Kernel Plane's clearing contract. The profile is stored on-chain. Gas cost: one storage write (~50K gas).

2. **Dormancy.** The profile sits on-chain. It does nothing. It costs nothing. No keeper, no monitoring, no maintenance. The consensus layer checks trigger conditions during each ISFR update.

3. **Activation.** When ISFR crosses the trigger threshold:
   - The clearing engine includes the profile's order in the next accumulation batch.
   - The order is sized as `min(maxNotional - filledSoFar, availableCounterparty)`.
   - If `maxFeeBps` is exceeded, the profile skips that round and retries on the next.

4. **Filling.** The profile participates in cooperative clearing rounds until one of:
   - `maxNotional` is fully filled
   - `maxRounds` is reached
   - `expiry` timestamp passes
   - The user cancels

5. **Completion/Cancellation.** The resulting position is a standard yield perpetual position. The clearing profile is marked as completed (or cancelled if the user intervenes).

### Why this matters

Without clearing profiles, hedging DeFi rates requires:
- Monitoring rates 24/7
- Submitting orders at the right time
- Managing partial fills across multiple clearing rounds
- Adjusting positions as rates move

With clearing profiles:
- One transaction, one time
- The chain does the rest

This is the UX difference between "DeFi experts can hedge rates" and "any treasury with a multisig can hedge rates." The profile abstraction removes the operational complexity that makes rate hedging prohibitive for all but the most sophisticated participants.

---

## 10. Cooperative clearing

### Innovation: clearing-as-inference

Traditional clearing systems produce one output: matched trades. Korai's cooperative clearing produces two: matched trades and a structured knowledge artifact (`ClearingInsight`). Every settlement round is an inference event that enriches the network's understanding of rate dynamics.

This dual output is not decorative. It creates the epistemic flywheel: clearing produces insights, insights improve predictions, better predictions attract volume, more volume produces more clearing rounds, more rounds produce more insights. The clearinghouse is an engine of intelligence, not a matching service.

### Clearing lifecycle

A cooperative clearing round proceeds through six stages:

#### Stage 1: Accumulation

Orders enter a pending batch. Orders come from three sources:
- Active limit orders submitted by traders
- Activated clearing profiles whose trigger conditions are met
- Liquidation orders from positions that have breached maintenance margin

The batch accumulates until one of four trigger conditions fires:

| Trigger | Threshold | Rationale |
|---------|-----------|-----------|
| Order count | 5+ orders | Minimum batch size for meaningful surplus optimization |
| Time elapsed | 10 seconds | Maximum wait time to ensure responsiveness |
| Imbalance ratio | 3:1 (buy:sell or sell:buy) | Severe imbalance signals market stress requiring immediate clearing |
| ISFR movement | 10+ bps since last clearing | Rate movement creates urgency for pending orders |

The first trigger to fire closes the batch.

#### Stage 2: Batch close

The order set is sealed. No new orders can enter this batch. The sealed set is published to solvers. The batch includes:

```rust
struct ClearingBatch {
    /// Monotonically increasing batch identifier.
    batch_id: u64,
    /// ISFR at batch close.
    isfr_at_close_bps: u32,
    /// All orders in the batch (buy and sell sides).
    orders: Vec<ClearingOrder>,
    /// Total buy-side notional.
    total_buy_notional: U256,
    /// Total sell-side notional.
    total_sell_notional: U256,
    /// Block height at batch close.
    block_height: u64,
}

struct ClearingOrder {
    /// Unique order identifier.
    order_id: B256,
    /// Whether this is a buy (long) or sell (short).
    side: Side,
    /// Limit price in basis points. Orders fill at this price or better.
    limit_bps: u32,
    /// Notional size in USD (scaled 1e18).
    notional: U256,
    /// Whether the order allows partial filling.
    partial_fill: bool,
    /// Source: Active (trader), Profile (clearing profile), Liquidation.
    source: OrderSource,
}
```

#### Stage 3: Solver competition

Multiple independent solver agents have 800 milliseconds (2 Korai blocks) to compute the optimal clearing solution. The solver's goal: find the uniform clearing price that maximizes total surplus across all orders.

**What "total surplus" means:** Each buy order has a limit price (the maximum the buyer will pay). Each sell order has a limit price (the minimum the seller will accept). When a buy order fills below its limit and a sell order fills above its limit, the difference is surplus. Total surplus is the sum of all individual surpluses.

```
BuyerSurplus_i = (BuyLimit_i - ClearingPrice) * FillSize_i   (for filled buys)
SellerSurplus_j = (ClearingPrice - SellLimit_j) * FillSize_j  (for filled sells)
TotalSurplus = sum(BuyerSurplus_i) + sum(SellerSurplus_j)
```

The solver submits a `ClearingSolution`:

```rust
struct ClearingSolution {
    /// The uniform clearing price in basis points.
    clearing_price_bps: u32,
    /// Fill amounts for each order (0 = unfilled).
    fills: Vec<FillAmount>,
    /// KKT certificate proving optimality.
    kkt_certificate: KKTCertificate,
    /// Solver's ERC-8004 passport address.
    solver: Address,
    /// Solver's bond amount (staked for accountability).
    bond: U256,
}
```

#### Stage 4: KKT verification

The chain verifies that the submitted solution satisfies the Karush-Kuhn-Tucker conditions. This is the mathematical proof that no better solution exists.

**Why KKT works here.** The clearing problem is a constrained optimization:

- **Objective:** maximize total surplus
- **Constraints:** orders fill at limit price or better, partial fills respected, buy volume = sell volume at the clearing price

Yield perpetuals have three properties that make this problem a convex linear program:

1. **Linear payoff**: $1 per bp per unit. No convexity in the payoff function.
2. **Continuous positions**: order sizes are not restricted to discrete lots.
3. **Partially-fillable orders**: orders can partially fill, making the feasible set convex.

For convex programs, the KKT conditions are necessary AND sufficient for optimality. If a solution satisfies KKT, it is globally optimal. There is no ambiguity.

**The three KKT conditions:**

1. **Primal feasibility.** All constraints are satisfied:
   - Every filled buy order fills at or below its limit price
   - Every filled sell order fills at or above its limit price
   - Total filled buy notional equals total filled sell notional
   - Partial fill constraints respected

2. **Dual feasibility.** Shadow prices (dual variables) are non-negative:
   - The shadow price on each binding constraint is >= 0
   - Intuitively: relaxing any active constraint would not improve the objective

3. **Complementary slackness.** For each constraint, either the constraint is binding (holds with equality) or its dual variable is zero:
   - If an order is partially filled, its limit price must equal the clearing price
   - If an order is fully filled or unfilled, its dual variable may be non-zero

**Verification complexity: O(n).** The verifier loops through each of the n orders exactly once, checking that the three conditions hold. No matrix inversions. No iterative solvers. A single pass through the order set.

```rust
fn verify_kkt(batch: &ClearingBatch, solution: &ClearingSolution) -> bool {
    let p = solution.clearing_price_bps;

    // Check 1: Primal feasibility
    let mut total_buy_fill = U256::ZERO;
    let mut total_sell_fill = U256::ZERO;

    for (order, fill) in batch.orders.iter().zip(solution.fills.iter()) {
        // Fill must not exceed order size
        if fill.amount > order.notional {
            return false;
        }
        // Partial fill allowed only if order permits it
        if fill.amount > U256::ZERO && fill.amount < order.notional && !order.partial_fill {
            return false;
        }
        match order.side {
            Side::Buy => {
                // Buy fills at clearing price or below limit
                if fill.amount > U256::ZERO && p > order.limit_bps {
                    return false;
                }
                total_buy_fill += fill.amount;
            }
            Side::Sell => {
                // Sell fills at clearing price or above limit
                if fill.amount > U256::ZERO && p < order.limit_bps {
                    return false;
                }
                total_sell_fill += fill.amount;
            }
        }
    }

    // Buy and sell volumes must balance
    if total_buy_fill != total_sell_fill {
        return false;
    }

    // Check 2 & 3: Dual feasibility + complementary slackness
    for (order, fill) in batch.orders.iter().zip(solution.fills.iter()) {
        if fill.amount > U256::ZERO && fill.amount < order.notional {
            // Partially filled: limit price must equal clearing price
            // (complementary slackness)
            match order.side {
                Side::Buy => {
                    if order.limit_bps != p {
                        return false;
                    }
                }
                Side::Sell => {
                    if order.limit_bps != p {
                        return false;
                    }
                }
            }
        }
    }

    true
}
```

#### Stage 5: Settlement

After KKT verification passes:

1. **Positions are updated.** New positions are created. Existing positions are adjusted. Liquidation fills are closed.
2. **Solver fee is computed.** The winning solver earns 5% of total surplus, capped at 50 KORAI per batch.

```
SolverFee = min(TotalSurplus * 0.05, 50 KORAI)
```

3. **Insurance fund contribution.** Each filled order contributes 0.5 bps of notional to the insurance fund.
4. **ClearingInsight is emitted.** A structured knowledge entry is written to the InsightStore:

```rust
struct ClearingInsight {
    batch_id: u64,
    clearing_price_bps: u32,
    total_surplus: U256,
    num_orders_filled: u32,
    num_orders_unfilled: u32,
    buy_sell_imbalance: f64,
    time_to_solve_ms: u32,
    solver: Address,
    isfr_at_clear: u32,
    spread_to_isfr_bps: i32,    // clearing_price - ISFR
    timestamp: u64,
}
```

#### Stage 6: Prediction scoring

All agent predictions committed before the batch close are scored against the clearing price:

```
CRPS_i = |predicted_bps_i - clearing_price_bps|
```

Scores are recorded on each agent's ERC-8004 passport and factor into the 30-day rolling CRPS percentile that determines epistemic reputation tier.

### Worked clearing example: 47 agents

Consider a clearing round with 47 orders from various agents and clearing profiles:

**Batch composition:**

| Order type | Count | Total notional |
|-----------|-------|---------------|
| Active buy (long) orders | 18 | $4,200,000 |
| Active sell (short) orders | 12 | $3,800,000 |
| Profile-activated short orders | 15 | $2,100,000 |
| Liquidation sell orders | 2 | $350,000 |
| **Total** | **47** | **$10,450,000** |

**Trigger:** Batch closed on the 10-second timeout with ISFR at 587 bps.

**Buy side (sorted by limit price, descending):**

| Limit (bps) | Notional | Cumulative buy |
|-------------|----------|----------------|
| 610 | $800K | $800K |
| 605 | $600K | $1,400K |
| 600 | $1,100K | $2,500K |
| 595 | $700K | $3,200K |
| 590 | $500K | $3,700K |
| 588 | $500K | $4,200K |
| (remaining 12 buys with limits below 585) | ... | ... |

**Sell side (sorted by limit price, ascending):**

| Limit (bps) | Notional | Cumulative sell |
|-------------|----------|----------------|
| 575 | $350K | $350K (liquidation) |
| 580 | $500K | $850K |
| 583 | $400K | $1,250K |
| 585 | $600K | $1,850K |
| 588 | $800K | $2,650K |
| 590 | $1,200K | $3,850K |
| 592 | $900K | $4,750K |
| (remaining 12 sells with limits above 595) | ... | ... |

**Optimal clearing price:** The solver finds that buy cumulative and sell cumulative curves cross at approximately 589 bps with $3,850,000 matched.

```
Clearing price: 589 bps
Matched volume: $3,850,000 per side
Unfilled buy orders: $350,000 (limits below 589)
Unfilled sell orders: $2,400,000 (limits above 589)
```

**Total surplus calculation:**

```
Buy surplus: sum over filled buys of (limit_i - 589) * fill_i
  = (610-589)*800K + (605-589)*600K + (600-589)*1100K + (595-589)*700K + (590-589)*500K + (589-589)*250K
  = $16,800 + $9,600 + $12,100 + $4,200 + $500 + $0
  = $43,200

Sell surplus: sum over filled sells of (589 - limit_j) * fill_j
  = (589-575)*350K + (589-580)*500K + (589-583)*400K + (589-585)*600K + (589-588)*800K + (589-589)*1200K*partial
  = $4,900 + $4,500 + $2,400 + $2,400 + $800 + $0
  = $15,000

Total surplus = $43,200 + $15,000 = $58,200
```

**Solver fee:** min($58,200 * 0.05, 50 KORAI) = $2,910 (assuming 50 KORAI > $2,910).

**Insurance fund:** $3,850,000 * 2 sides * 0.5 bps = $385.

**Solve time:** 340ms (well within the 800ms window).

### Fallback ladder

If cooperative clearing fails, the system degrades gracefully through four levels:

| Level | Condition | Action |
|-------|-----------|--------|
| **Normal** | Solver submits valid KKT solution within 800ms | Standard cooperative clearing |
| **Retry** | No valid solution within 800ms | Batch rolls to the next block. Solvers get another 400ms. |
| **Emergency CLOB** | No valid solution after 2 retries | Continuous limit order book activates. Orders match directly at limit prices. No surplus optimization. |
| **Circuit breaker** | ISFR enters Halted state | Trading paused. Existing positions preserved. No new orders accepted. |

The emergency CLOB is a degraded mode, not a failure mode. Orders still fill. Positions still settle. The difference is that the CLOB does not optimize surplus -- it matches orders sequentially by price-time priority. Users get execution but not the economic optimality that cooperative clearing provides.

---

## 11. Why yield perpetuals vs Pendle

Pendle is the largest on-chain yield trading protocol. It has achieved real traction ($1.9B TVL, $47.8B 2025 trading volume) by tokenizing the yield component of yield-bearing assets into separate Principal Tokens (PT) and Yield Tokens (YT) with fixed maturities. Understanding where Pendle falls short clarifies what yield perpetuals are designed to solve.

| Property | Pendle | Yield perpetuals |
|----------|--------|-----------------|
| **Maturity** | Fixed (3, 6, 12 months typical) | None. Perpetual. No expiry. |
| **Rollover** | Required at maturity. Costs 50+ bps in slippage per roll. | Not applicable. Zero rollover cost. |
| **Liquidity pools** | 200+ pools (one per asset + maturity combination). | 1 pool per benchmark rate. All ISFR trading concentrates in one venue. |
| **Per-pool TVL** | Fragmented. Most pools under $200M. Many under $10M. | Concentrated. One pool captures all demand for the benchmark. |
| **Leverage** | 1x (no native leverage). | Up to 10x. |
| **Underlying** | Individual asset yield (e.g., stETH yield, GLP yield). | Composite benchmark rate (ISFR). Not tied to any single asset. |
| **Pricing reference** | Internal AMM curve per pool. | ISFR oracle (consensus-computed, manipulation-resistant). |
| **Knowledge production** | None. Trades produce no epistemic artifacts. | Every clearing round emits ClearingInsight. Every ISFR update enables prediction scoring. |
| **Hedging UX** | Manual: choose maturity, choose pool, manage rollover, accept fragmented liquidity. | One clearing profile. Set trigger, walk away. |
| **Cross-protocol exposure** | Each pool tracks one protocol's yield. No cross-protocol composite. | ISFR is a composite across Aave, Compound, Ethena, and ETH staking. One instrument hedges aggregate rate exposure. |

### The rollover problem

Rollover cost is the defining structural weakness of fixed-maturity instruments. When a 3-month PT/YT position expires, the holder must close the position and open a new one in the next maturity. Each rollover incurs:

- Slippage: 20-80+ bps depending on pool depth and urgency
- Gas costs: two transactions (close + open)
- Timing risk: the new pool may have different implied rates
- Opportunity cost: execution delay between close and reopen

For a treasury running a continuous hedge, rolling four times per year at 50 bps per roll costs 200 bps annually. That is 2% of notional per year in friction alone. For a $10M position, that is $200,000/year in pure execution cost.

Yield perpetuals eliminate this cost entirely. The position persists indefinitely. The funding rate mechanism handles convergence to the benchmark. No rollover, no slippage, no timing risk.

### The fragmentation problem

Pendle's pool-per-maturity-per-asset architecture fragments liquidity by design. stETH 3-month, stETH 6-month, GLP 3-month, GLP 6-month -- each is a separate pool with separate liquidity. A trader wanting to hedge $50M of aggregate yield exposure must split across multiple pools, accept thin liquidity in each, and manage the portfolio of positions.

Yield perpetuals concentrate all rate-hedging demand into a single instrument per benchmark. Every participant -- DAOs, funds, protocols, individual traders, agent systems -- trades the same contract. Concentration produces depth. Depth produces tight spreads. Tight spreads attract more participants. The liquidity flywheel that Pendle's architecture structurally prevents is the natural equilibrium for perpetuals.

---

## 12. Generalized benchmark framework

### The pattern

ISFR demonstrates a pattern that generalizes beyond interest rates. Any domain where multiple independent sources produce measurable signals can support a benchmark index with the same properties: multi-source aggregation, dual-median computation, validator-level consensus, oracle publication, and derivative instruments.

The pattern has five components:

1. **Multiple sources** that independently measure the same underlying phenomenon
2. **Weighted median aggregation** that resists manipulation by any minority of sources
3. **Validator computation** that eliminates oracle operator dependency
4. **Precompile publication** that makes the index available to any on-chain contract
5. **Prediction scoring** that turns each update into an epistemic calibration event

### Candidate benchmark indices

| Index | What it measures | Sources | Update frequency |
|-------|-----------------|---------|-----------------|
| **ISFR** (Internet Secured Funding Rate) | DeFi secured funding cost | Aave, Compound, Ethena, ETH staking (V1) | 10 seconds |
| **IAPI** (Internet Agent Performance Index) | Agent task success rates across arenas | Arena results, gate pass rates, task completion metrics | 5 minutes |
| **IKQI** (Internet Knowledge Quality Index) | InsightStore entry accuracy and utility | Confirmation rates, usage frequency, CRPS scores | 1 hour |
| **ISVI** (Internet Security Vulnerability Index) | Security detection rates across codebases | Audit outcomes, bug bounty results, formal verification scores | 1 hour |
| **IRRI** (Internet Research Rigor Index) | Research output quality across agents | Citation rates, replication success, peer validation | 24 hours |

Each follows the same construction:

1. Define 4-7 sources that independently measure the target phenomenon
2. Assign weights (equal in V1, governance-adjustable in V2)
3. Validators compute the weighted median from their independent observations
4. The chain aggregates validator submissions via stake-weighted median
5. The index is published via a precompile
6. Agents commit predictions before each update; predictions are scored via CRPS

### The BenchmarkIndex trait

The generalization is captured in a Rust trait that any benchmark must implement:

```rust
/// A benchmark index computed by Korai validators at the consensus layer.
///
/// All benchmark indices share the same computational structure: multi-source
/// aggregation via weighted median, validator-level consensus via stake-weighted
/// median, and publication via precompile. The trait captures this invariant
/// structure while allowing domain-specific source definitions and update cadences.
pub trait BenchmarkIndex: Send + Sync {
    /// The set of sources contributing to this index.
    fn sources(&self) -> &[IndexSource];

    /// Compute the index value from a set of source readings.
    ///
    /// Each validator calls this independently during block production.
    /// The implementation must be deterministic given the same inputs.
    fn compute(&self, readings: &[SourceReading]) -> IndexValue;

    /// Compute the confidence score from validator vote distribution.
    ///
    /// Returns the fraction of stake that submitted votes within one
    /// standard deviation of the stake-weighted median.
    fn confidence(&self, validator_votes: &[Vote]) -> f64;

    /// The update cadence in blocks. ISFR: 25 blocks (~10s). IAPI: 750 blocks (~5min).
    fn update_cadence_blocks(&self) -> u64;

    /// Precompile address where this index is published.
    fn precompile_address(&self) -> Address;

    /// Circuit breaker threshold. Below this confidence, the index enters Degraded state.
    fn circuit_breaker_threshold(&self) -> f64 {
        0.70 // default; overridable per index
    }
}

/// A source contributing to a benchmark index.
pub struct IndexSource {
    /// Human-readable source name (e.g., "Aave V3 USDC").
    pub name: String,
    /// Weight in the weighted median computation. Weights must sum to 1.0.
    pub weight: f64,
    /// Maximum weight this source can have (governance constraint).
    pub max_weight: f64,
    /// Liveness timeout in seconds. Source is excluded after this duration without updates.
    pub liveness_timeout_secs: u64,
    /// Method for reading this source (chain query, API, etc.).
    pub reader: SourceReader,
}

/// A single reading from a source at a point in time.
pub struct SourceReading {
    /// Source index.
    pub source_idx: usize,
    /// Value in basis points.
    pub value_bps: u32,
    /// Timestamp of the reading.
    pub timestamp: u64,
    /// Whether the reading is within the liveness window.
    pub is_live: bool,
}

/// The computed index value with metadata.
pub struct IndexValue {
    /// The index value in basis points.
    pub value_bps: u32,
    /// Number of live sources used in computation.
    pub num_sources: u32,
    /// Publication state.
    pub state: PublicationState,
}
```

### Why generalize

The generalization is not speculative architecture. It is a strategic bet on a specific thesis: **the first credible benchmark in any domain captures the derivative market for that domain.** SOFR captured interest rate derivatives. The VIX captured volatility derivatives. The S&P 500 captured index derivatives.

If Korai establishes ISFR as the DeFi rate benchmark, the same infrastructure (validators, precompiles, dual-median aggregation, cooperative clearing) can be reused to launch benchmark indices in adjacent domains. Each new index is a new derivative market. Each new derivative market generates clearing volume. Each clearing round produces knowledge. The marginal cost of adding a benchmark index is low; the marginal value is an entire market.

---

## 13. ISFR path to credibility

Benchmark rates are natural monopolies. SOFR displaced LIBOR not because it was marginally better but because benchmark credibility requires universal adoption and universal adoption requires a single standard. There is no market for the third-most-credible benchmark.

This means ISFR cannot launch as a derivatives settlement layer on day one. Credibility is earned through a sequence of increasingly committed uses.

### Phase 1: Publication and transparency (months 1-6)

**Goal:** Establish ISFR as a reliable, transparent, verifiable rate publication.

- Korai validators compute and publish ISFR every 10 seconds
- Full methodology documentation is public (this PRD and technical supplements)
- Historical ISFR data is freely accessible via precompile and API
- Real-time comparison dashboard shows ISFR against individual protocol rates
- Source rate discrepancy alerts are public
- No derivatives. No financial instruments. Publication only.

**Success metric:** ISFR is referenced in at least 5 external protocol governance discussions as a rate benchmark within 6 months.

### Phase 2: Yield perpetual launch (months 6-12)

**Goal:** Launch the first derivative instrument settled against ISFR.

- Yield perpetuals go live on Korai
- Clearing profiles enable one-action hedging
- Cooperative clearing with KKT verification operates in production
- Agent prediction markets provide initial liquidity and volume
- ISFR prediction scoring drives epistemic reputation tiers

**Why agents provide anchor demand:** Korai agents commit CRPS predictions to every ISFR update. Predictions that are scored create an incentive to trade on those predictions. An agent that predicts "ISFR will rise 50 bps" has a natural reason to go long. The prediction market and the perpetual market reinforce each other. This bootstraps initial volume without external market makers.

**Success metric:** $50M+ monthly trading volume within 6 months of launch.

### Phase 3: External integration (months 12-24)

**Goal:** ISFR becomes the standard reference rate for DeFi yield products.

- External protocols reference ISFR in their interest rate models
- "ISFR + spread" pricing appears in lending protocol documentation
- ISFR-indexed floating-rate vaults launch on other chains (bridged via standard oracle interfaces)
- Academic and industry publications cite ISFR methodology

**The self-referencing moment:** At some point, ISFR gains the property that made SOFR authoritative: it becomes the rate that markets reference, which makes it the rate that markets must reference. This transition is nonlinear. Below a threshold of adoption, ISFR is a publication. Above the threshold, ISFR is a standard.

**Success metric:** At least one external DeFi protocol integrates ISFR as a pricing reference.

---

## 14. ISFR vs SOFR comparison

| Property | SOFR | ISFR |
|----------|------|------|
| **Full name** | Secured Overnight Financing Rate | Internet Secured Funding Rate |
| **Publisher** | Federal Reserve Bank of New York | Korai validator set (decentralized) |
| **Update frequency** | Once daily (8:00 AM ET) | Every 10 seconds (8,640/day) |
| **Computation method** | Volume-weighted median of overnight Treasury repo transactions | Dual weighted median (source-level + validator-level) |
| **Data sources** | ~$2T daily tri-party, GCF, and bilateral Treasury repo | 4 DeFi protocol rates (V1), 7+ (V2) |
| **Trust model** | Trust the Federal Reserve to compute accurately from bank-reported data | Trust that >50% of validator stake is honest (Byzantine fault tolerance) |
| **Manipulation resistance** | Transaction-based (hard to fake $2T in repo volume) | Dual-median + 49% Byzantine tolerance at each layer |
| **Availability** | Weekdays only. No holidays. No weekends. | 24/7/365. No downtime except consensus failure. |
| **Latency** | T+1. Published the morning after the trading day. | Real-time. Available on-chain within 10 seconds of computation. |
| **Programmability** | Not natively programmable. Requires off-chain integration. | Native precompile. Any smart contract reads ISFR with one call. |
| **Knowledge production** | None. SOFR is a number. | Every update generates ISFRInsight. Agents score predictions against each update. |
| **Derivative notional** | ~$570T+ | $0 (pre-launch). Target: $50M+ monthly volume within 12 months. |
| **Governance** | Federal Reserve sets methodology. No market participant input. | Methodology fixed in protocol. Source weights adjustable via governance (V2). |
| **Historical data** | Available via FRED (Federal Reserve Economic Data). | On-chain for 90 days. Archival via IPFS. |
| **Circuit breakers** | None (SOFR is published or not published). | Four-state model (Live, Degraded, Stale, Halted) with confidence-based triggers. |

### Key asymmetries

**Frequency.** SOFR's daily cadence was designed for overnight lending markets where rates change slowly. DeFi rates can spike hundreds of basis points within hours during high-utilization events. A 10-second cadence captures these dynamics; a daily rate misses them entirely.

**Programmability.** SOFR requires off-chain data integration to use in automated systems. ISFR is a precompile call. The difference between "query an API and push the result on-chain" and "read a native on-chain value" is the difference between an oracle dependency and a protocol primitive.

**Trust model.** SOFR depends on the Federal Reserve accurately processing bank-reported data. The LIBOR scandal showed that centralized rate computation invites manipulation. ISFR depends on a majority of validator stake being honest -- the same assumption that secures the chain itself. If the validator set is compromised, ISFR is the least of the problems.

---

## 15. Solver economics and anti-gaming

### Solver fee structure

Solvers earn revenue from two sources:

1. **Surplus share:** 5% of total clearing surplus per batch, capped at 50 KORAI
2. **MEV (maximum extractable value):** Solvers can capture the difference between the optimal clearing price and any better price they discover, up to the cap

**Example economics for a solver processing 100 batches per day:**

```
Average surplus per batch: $58,200 (from worked example)
Solver fee per batch: min($58,200 * 0.05, 50 KORAI)
  = $2,910 (if 50 KORAI > $2,910)

Daily solver revenue (100 batches): ~$291,000
Monthly solver revenue: ~$8.7M
```

These numbers scale with trading volume. Early-stage volume will produce proportionally smaller surplus.

### Solver accountability: the challenge mechanism

Every clearing solution is subject to a permissionless challenge window of 10 blocks (4 seconds at 400ms block time). Any observer can submit a strictly better solution -- one that produces higher total surplus while satisfying all KKT conditions.

```rust
struct ClearingChallenge {
    /// Batch being challenged.
    batch_id: u64,
    /// The challenger's improved solution.
    improved_solution: ClearingSolution,
    /// Challenger's ERC-8004 passport address.
    challenger: Address,
}
```

**Challenge validation:** The chain verifies that:
1. The challenger's solution satisfies KKT conditions (same O(n) check)
2. The challenger's total surplus strictly exceeds the original solution's total surplus
3. The challenge is submitted within the 10-block window

**If the challenge succeeds:**
- The original solver is slashed 10% of their bond
- The challenger receives 5% of the original solver's bond as a bounty
- The remaining 5% goes to the insurance fund
- The improved solution replaces the original (positions are re-settled at the better price)

**If the challenge fails (the proposed solution is not strictly better):**
- The challenger loses their challenge deposit (0.5% of the original solver's bond)
- The original solution stands

### Anti-collusion mechanisms

Solver collusion -- where multiple solvers agree to submit suboptimal solutions to reduce competitive pressure -- is prevented by three mechanisms:

1. **Permissionless challenges.** Anyone can challenge, not only registered solvers. A bot running the optimal clearing algorithm can challenge any suboptimal solution profitably. The challenge bounty (5% of the solver's bond) provides strong incentive for monitoring.

2. **Solver rotation.** Solvers are selected from a rotating set. No solver can guarantee they will process consecutive batches. Collusion requires corrupting the entire rotation.

3. **KKT certificate transparency.** Every solution's KKT certificate is published on-chain. Suboptimal solutions are detectable by anyone who can run the clearing algorithm -- which is publicly specified.

### Slashing schedule

| Offense | Slash amount | Recovery |
|---------|-------------|----------|
| Successful challenge (suboptimal solution) | 10% of solver bond | Must post new bond. 3-strike lockout: 3 successful challenges in 30 days = 30-day suspension. |
| Timeout (no solution submitted within 800ms) | No slash | Batch rolls to next block. Solver loses fee opportunity. |
| Invalid KKT certificate (malformed proof) | 20% of solver bond | Immediate suspension pending review. |
| Repeated timeouts (5+ in 24 hours) | 5% of solver bond | Temporary deprioritization in rotation. |

### Solver bond requirements

| Volume tier | Minimum bond | Maximum batches per day |
|------------|-------------|------------------------|
| Tier 1 (<$1M daily volume) | 1,000 KORAI | Unlimited |
| Tier 2 ($1M-$100M) | 10,000 KORAI | Unlimited |
| Tier 3 (>$100M) | 100,000 KORAI | Unlimited |

Bond tiers scale with the damage a malicious solver could cause. Higher-volume markets require larger bonds because suboptimal clearing at higher volume produces larger losses.

---

## 16. Academic citations

1. **Federal Reserve Bank of New York** (2018). "Secured Overnight Financing Rate (SOFR)." The benchmark that replaced LIBOR as the primary USD reference rate. SOFR's design -- transaction-weighted median from overnight Treasury repo data, computed by the central bank -- is the direct methodological ancestor of ISFR. SOFR's success (from $0 to $72.1T in SOFR-linked OIS notional in 3 years) demonstrates that a credible benchmark rate unlocks derivative markets.

2. **Karush, W.** (1939). "Minima of Functions of Several Variables with Inequalities as Side Constraints." M.Sc. thesis, University of Chicago. / **Kuhn, H.W. and Tucker, A.W.** (1951). "Nonlinear Programming." In *Proceedings of the Second Berkeley Symposium on Mathematical Statistics and Probability*. Together, these established the KKT conditions: necessary conditions for a solution to be optimal in a constrained optimization problem. For convex programs (which yield perpetual clearing provably is), KKT conditions are both necessary and sufficient. This is the mathematical foundation of cooperative clearing's optimality guarantee.

3. **Gneiting, T. and Raftery, A.E.** (2007). "Strictly Proper Scoring Rules, Prediction, and Estimation." *Journal of the American Statistical Association*, 102(477), 359-378. Defines the CRPS scoring rule and proves strict propriety: the unique optimal strategy is truthful prediction. No agent can improve its score by reporting anything other than its genuine belief. This result is the foundation of ISFR prediction scoring and epistemic reputation tiers.

4. **Vickrey, W.** (1961). "Counterspeculation, Auctions, and Competitive Sealed Tenders." *Journal of Finance*, 16(1), 8-37. / **Clarke, E.H.** (1971). "Multipart Pricing of Public Goods." *Public Choice*, 11, 17-33. / **Groves, T.** (1973). "Incentives in Teams." *Econometrica*, 41(4), 617-631. The VCG mechanism: a truthful auction where each participant pays the externality they impose on others. Used in Roko's context allocation (PRD-04) and referenced in cooperative clearing's surplus distribution.

5. **Condorcet, M.J.A.N.** (1785). *Essai sur l'Application de l'Analyse a la Probabilite des Decisions Rendues a la Pluralite des Voix*. Condorcet's jury theorem: if each voter is more likely right than wrong, majority vote accuracy approaches 1.0 as voters increase, provided voters are independent. This is the theoretical basis for ISFR's dual-median aggregation: if each validator is independently more likely to compute correctly than not, the stake-weighted median converges to the true rate as the validator set grows.

6. **Pendle Finance** (2021-present). Yield tokenization protocol that separates yield-bearing assets into Principal Tokens (PT) and Yield Tokens (YT). Pendle's peak TVL of $13.4B and $47.8B in 2025 trading volume demonstrate that demand for yield trading exists. Pendle's structural limitations -- fixed maturity, liquidity fragmentation, no benchmark rate, no leverage -- define the design space that yield perpetuals occupy.

7. **Bank for International Settlements** (2024). "OTC Derivatives Statistics." Reports $668 trillion in interest rate derivative notional outstanding globally. This figure anchors the market size comparison: $668T in TradFi vs. <$100M on-chain. The six-order-of-magnitude gap is the opportunity.

8. **Alternative Reference Rates Committee (ARRC)** (2017-2023). "Paced Transition Plan for SOFR." Documented the multi-year transition from LIBOR to SOFR. Key lessons for ISFR: (a) benchmark transitions take years of parallel publication before derivatives launch, (b) the benchmark must be free, transparent, and methodologically sound before adoption, (c) regulatory endorsement accelerates but does not determine adoption.

9. **Boyd, S. and Vandenberghe, L.** (2004). *Convex Optimization*. Cambridge University Press. Chapter 5: Duality and KKT conditions for convex programs. The textbook proof that KKT conditions are necessary and sufficient for convex optimization. Yield perpetual clearing is a linear program (a strict subset of convex programs), making the KKT guarantee apply with full force.

10. **Budish, E., Cramton, P., and Shim, J.** (2015). "The High-Frequency Trading Arms Race: Frequent Batch Auctions as a Market Design Response." *Quarterly Journal of Economics*, 130(4), 1547-1621. Makes the case for batch auctions over continuous limit order books. Batch auctions eliminate speed advantages, reduce adverse selection, and improve price discovery. Korai's cooperative clearing is a batch auction with the added property of provable optimality via KKT certificates.

11. **Friston, K.** (2010). "The Free-Energy Principle: A Unified Brain Theory?" *Nature Reviews Neuroscience*, 11(2), 127-138. The free energy principle: biological systems act to minimize surprise (prediction error). Referenced in the context of ISFR prediction commitment: agents minimize their CRPS (a form of prediction error) by producing increasingly accurate forecasts, which requires increasingly sophisticated models of DeFi rate dynamics. The prediction scoring mechanism harnesses the same optimization pressure.

---

## 17. Korai integration gaps (ISFR-specific)

Seven gaps stand between the ISFR specification above and a working system. Each gap is scoped, has a known blocker, and has a concrete fix path.

### Gap 1: No live ISFR source adapters

**Current state.** The `BenchmarkIndex` trait and `ISFRInsight` struct exist as specifications. No adapter reads live data from Aave V3, Compound V3, Ethena sUSDe, or the ETH Beacon Chain. Validator nodes have no code path that fetches these rates.

**Blocker.** Each source needs a dedicated adapter that handles the source's specific data format, update cadence, and failure modes. Aave and Compound rates come from on-chain view functions. Ethena's sUSDe yield derives from staking contract state. ETH staking yield requires Beacon Chain API calls (separate from the execution layer).

**Fix path.** Implement four adapters in `roko-chain`:

| Adapter | Data source | Read method | Complexity |
|---------|------------|-------------|------------|
| `AaveV3Adapter` | Aave LendingPool on Ethereum | `eth_call` to `getReserveData(USDC)`, extract `currentLiquidityRate` | Low -- single view function, well-documented ABI |
| `CompoundV3Adapter` | Compound Comet on Ethereum | `eth_call` to `getSupplyRate(0)`, convert from per-second to APY | Low -- similar to Aave |
| `EthenaSUSDeAdapter` | sUSDe staking contract | `eth_call` to read totalAssets/totalSupply, compute 7-day rolling yield from historical snapshots | Medium -- requires maintaining a 7-day window of snapshots |
| `EthBeaconAdapter` | Beacon Chain API (`/eth/v1/beacon/states/head/validators`) | HTTP GET to consensus client, compute annualized yield from attestation + proposal rewards | Medium -- requires consensus client connection, epoch-based aggregation |

Each adapter implements a `SourceAdapter` trait with `fn read(&self) -> Result<SourceReading>`. Validators call all four adapters during the ISFR computation window.

### Gap 2: ISFR precompile not deployed on Mirage testnet

**Current state.** The `IISFROracle` Solidity interface is specified (Appendix B). The precompile address `0xA01` is reserved. No bytecode is deployed on Mirage (Korai's testnet).

**Blocker.** Precompile deployment requires modifications to the Korai execution client (the EVM implementation must route calls to `0xA01` to native Rust code rather than interpreting bytecode). This is a consensus-layer change that requires a coordinated validator upgrade.

**Fix path.**
1. Implement the precompile handler in the Korai execution client's precompile registry.
2. The handler reads from the in-state ISFR storage trie (written by the consensus layer during block production).
3. Deploy to Mirage testnet as part of the next coordinated upgrade.
4. Validate with integration tests that call `ISFROracle(0xA01).current()` from a test contract.

### Gap 3: CRPS scoring engine has no implementation

**Current state.** The CRPS formula is defined (section 6). The `ISFRPrediction` struct and `PROOF_LOG` precompile at `0xA04` are specified. No scoring engine processes predictions against outcomes.

**Blocker.** The scoring engine must run at consensus time (inside block production) to score predictions committed before the ISFR update against the just-computed ISFR value. This is a hot path -- it must complete within the block production window.

**Fix path.**
1. Implement `CRPSScorer` in `roko-chain` with O(n) scoring per ISFR update (one pass over committed predictions).
2. The scorer reads predictions from the `PROOF_LOG` precompile, computes `|predicted_bps - actual_bps|` for each, and writes scores back to the proof log.
3. Rolling 30-day percentile computation runs off-chain (in the agent sidecar or a dedicated indexer) and writes tier assignments to a state trie that the `PROOF_LOG` precompile exposes.
4. Unit test: commit 1,000 predictions with known distributions, verify CRPS scores match analytical expectations.

### Gap 4: Yield perp contract not deployed

**Current state.** The yield perpetual specification is complete (section 7): contract parameters, payoff structure, mark price, funding rate, margin, and liquidation. No Solidity contract implements this specification.

**Blocker.** The contract depends on a working ISFR precompile (Gap 2) for mark price computation and settlement. It also depends on the cooperative clearing engine (Gap 5) for trade execution. Both must be live before the perp contract can be meaningfully tested.

**Fix path.**
1. Write the yield perp contract in Solidity, targeting the Korai Kernel Plane.
2. Mock the ISFR precompile for unit testing (hardcode rates, verify PnL computation).
3. Integrate with the clearing engine once Gap 5 is resolved.
4. Deploy to Mirage testnet for end-to-end testing with synthetic ISFR data.

### Gap 5: Cooperative clearing engine not integrated with agent dispatch

**Current state.** The clearing lifecycle is specified in detail (section 10): accumulation, batch close, solver competition, KKT verification, settlement, and prediction scoring. Roko's agent dispatch system (`roko-agent`) has no code path that submits solver solutions or processes clearing batches.

**Blocker.** Solver agents need a dispatch path that receives `ClearingBatch` events, runs the optimization, and submits `ClearingSolution` transactions. The current agent dispatch architecture dispatches to LLM backends; solver dispatch is a non-LLM code path that calls a mathematical optimizer.

**Fix path.**
1. Add a `SolverDispatcher` variant to `roko-agent`'s dispatcher that routes clearing batches to an LP solver (e.g., `minilp` or `good_lp` crate).
2. The solver receives a `ClearingBatch`, constructs the linear program, solves it, generates the KKT certificate, and submits via RPC.
3. Wire this dispatcher to the orchestration loop so that solver agents are automatically started when clearing batches accumulate.
4. The existing `ProcessSupervisor` manages solver agent lifecycle.

### Gap 6: No clearing profile creation UX flow

**Current state.** The `ClearingProfile` Solidity struct is specified (section 9). The lifecycle is documented. No CLI command, TUI panel, or API endpoint lets a user create a clearing profile.

**Blocker.** Profile creation requires constructing and submitting a Korai transaction that writes the `ClearingProfile` to the clearing contract's storage. Roko needs a user-facing flow that collects the parameters (direction, trigger, max notional, max fee, expiry) and constructs the transaction.

**Fix path.**
1. Add `roko clearing create` CLI subcommand that prompts for profile parameters and submits the transaction.
2. Add a clearing panel to the TUI dashboard (new F-key tab) showing active profiles, their status, and fill progress.
3. Add clearing REST endpoints to `roko-serve` (`POST /clearing/profiles`, `GET /clearing/profiles/:id`, `DELETE /clearing/profiles/:id`).
4. Wire the TUI panel to the REST endpoints for real-time profile monitoring.

### Gap 7: ISFR source adapters span multiple chains

**Current state.** The four V1 sources all live on Ethereum mainnet or the Beacon Chain. V2 expands to sources on Base and Arbitrum (see section 17 below on multi-chain ISFR sources). No ChainActor infrastructure exists to subscribe to these sources across chains.

**Blocker.** The multi-chain ChainActor architecture is specified in PRD-09 but not implemented. ISFR source adapters need one ChainActor per source chain, each feeding rate readings to the ISFR aggregation layer.

**Fix path.**
1. Implement PRD-09's `ChainActor` for Ethereum mainnet first (covers all V1 sources).
2. For V2: add ChainActors for Base and Arbitrum to read Aave/Compound deployments on those chains.
3. The ISFR aggregation layer collects `SourceReading` events from all ChainActors and runs the weighted median.
4. Each ChainActor independently handles its chain's block time and reorg depth.

---

## 18. ISFR as EventFabric source

Every ISFR update is not a passive data point. It is an active event that flows through the agent runtime and triggers downstream behavior.

### Event emission

When Korai validators publish a new ISFR value, the Korai `ChainActor` detects the state change at the `0xA01` precompile and emits a `CanonicalEvent` with payload type `EventPayload::ISFRUpdate`:

```rust
CanonicalEvent {
    chain_id: ChainId::Korai,
    block_number: 1_000_025,
    event_type: EventType::ISFRUpdate,
    payload: EventPayload::ISFRUpdate {
        value_bps: 600,
        previous_bps: 595,
        delta_bps: 5,
        state: PublicationState::Live,
        confidence_bps: 9200,
        num_sources: 4,
    },
    timestamp: 1745280000,
    finality: Finality::Deterministic,
}
```

This event enters the `CanonicalEventBus`, where any agent with an `EventFilter::ISFRUpdate` subscription receives it.

### Agent subscription

Blockchain agents subscribe to ISFR updates through their domain profile's wakeup event configuration:

```toml
[[wakeup_events]]
event_type = "ISFRUpdate"
severity_threshold = 0.0    # receive every update
```

The event arrives at the agent's heartbeat pipeline during the next tick. Extensions in the chain receive the `ISFRUpdate` event via `on_observe`.

### Large ISFR moves trigger attention amplification

When the delta between consecutive ISFR updates exceeds 50 basis points, the `ForagingModel` (PRD-09 section 13) treats this as a high-value signal. The Gittins index for all ISFR-related sources receives a 5x multiplier:

```rust
if isfr_event.delta_bps.unsigned_abs() > 50 {
    foraging_model.boost_patch(
        PatchId::ISFRSources,
        GittinsMultiplier(5.0),
        Duration::from_secs(300),  // boost lasts 5 minutes
    );
}
```

This amplification reallocates the agent's attention budget toward rate-sensitive sources -- lending protocol events, funding rate changes, staking yield shifts -- for the next 5 minutes. The agent spends more monitoring cycles on the sources that explain the rate move.

### ISFR Halted triggers circuit breaker

When ISFR transitions to the `Halted` state (0 sources reporting, confidence below 50%, or consensus failure), agents with active clearing profiles must react. The `ISFRUpdate` event carries the state transition:

```rust
if isfr_event.state == PublicationState::Halted {
    // Pause all clearing profile monitoring
    clearing_monitor.pause_all_profiles();

    // Emit high-severity alert for operator notification
    event_bus.emit(AlertEvent {
        severity: Severity::Critical,
        source: "isfr_circuit_breaker",
        message: "ISFR halted -- clearing profiles paused, emergency CLOB active",
    });
}
```

Clearing profiles do not auto-cancel on halt. They pause and resume when ISFR returns to Live state (after the hysteresis recovery: confidence > 80% for 3 consecutive updates).

### ClearingInsights flow into WorldGraph

Every cooperative clearing round emits a `ClearingInsight` (section 10, stage 5). These insights are not isolated data points -- they update the agent's WorldGraph as market entity observations:

```rust
// After receiving a ClearingInsight event:
worldgraph.update_entity(
    EntityId::Market("ISFR-PERP-V1"),
    EntityUpdate {
        clearing_price_bps: insight.clearing_price_bps,
        surplus: insight.total_surplus,
        buy_sell_imbalance: insight.buy_sell_imbalance,
        spread_to_isfr: insight.spread_to_isfr_bps,
        last_updated: insight.timestamp,
    },
);
```

The WorldGraph entity for the ISFR perpetual market accumulates a time series of clearing prices, surplus values, and imbalance ratios. This history feeds the foraging model's reward estimation and the context injection layer's bid calculations.

---

## 19. Cross-domain ISFR usage

ISFR is designed for DeFi rate hedging. Its value extends beyond blockchain agents.

### Research agents

Research agents use ISFR as a macro signal. A rising ISFR (DeFi funding costs increasing) correlates with leverage expansion, speculative demand, and potential market stress. A falling ISFR correlates with risk-off sentiment, capital outflows, or protocol derisking.

Research agents subscribe to `ISFRUpdate` events at a filtered cadence (hourly or daily summary) and use rate regime transitions as triggers for research topic generation:

- ISFR regime transition to `Volatile` triggers research on "DeFi leverage cycles" or "funding rate dynamics."
- ISFR crossing historical percentile thresholds (e.g., above 95th percentile of 30-day range) triggers deep-dive analysis.
- Sustained divergence between ISFR sources (Aave vs. Compound spread widening) triggers protocol-specific research.

The research output feeds the InsightStore, where blockchain agents can query it for context during clearing and hedging decisions.

### Coding agents

Agents writing DeFi protocol code benefit from live ISFR data as development context. When a coding agent works on a lending protocol, yield optimizer, or liquidation bot, the current and historical ISFR provides:

- **Realistic test parameters.** Instead of hardcoded 5% yield in tests, the agent references actual current rates.
- **Rate range calibration.** The agent knows the observed min/max/mean ISFR over the past 30 days, producing code that handles realistic rate ranges.
- **Protocol comparison context.** The per-source breakdown (Aave rate vs. Compound rate vs. Ethena rate) helps the agent understand rate differentials when writing cross-protocol logic.

The coding agent does not subscribe to real-time ISFR updates. It queries the ISFR snapshot via the InsightStore when it needs rate context during code generation or review.

### Security agents

ISFR manipulation detection is a security monitoring signal. Sudden, large ISFR moves that do not correlate with market conditions suggest attempted manipulation of source rates. Security agents monitor for:

- **Flash loan signatures.** A source rate spike that reverts within one block is consistent with flash loan manipulation. The security agent flags the source and the transaction.
- **Source divergence anomalies.** If one source moves 200+ bps while all others remain stable, the divergent source may be under governance attack or experiencing a bug.
- **Confidence score drops.** A sudden drop in validator confidence (from 90%+ to below 70%) indicates validator disagreement, which may signal a consensus-layer attack.

Security agents publish their findings as `SecurityInsight` entries in the InsightStore, which other agents (including ISFR oracle agents) can query to adjust their behavior.

### The generalized BenchmarkIndex pattern

The cross-domain utility of ISFR demonstrates why the `BenchmarkIndex` trait (section 12) is domain-agnostic. Any benchmark index -- IAPI (agent performance), IKQI (knowledge quality), ISVI (security vulnerability) -- follows the same pattern. Non-domain agents consume the index as a contextual signal. Domain agents consume it as an operational input. The same infrastructure serves both use cases because the event flow (`BenchmarkUpdate` -> `EventFabric` -> agent subscription) is identical regardless of what the index measures.

---

## 20. Agent roles in the ISFR ecosystem

The ISFR ecosystem requires specialized agent roles. Each role has distinct capabilities, incentives, and feedback loops.

### Oracle agents

Oracle agents are the backbone of ISFR computation. Each Korai validator runs an oracle agent that:

1. **Monitors sources.** The agent maintains persistent connections to all ISFR source adapters (Aave, Compound, Ethena, ETH staking). It reads rates at the cadence each source requires.
2. **Detects deviations.** When a source rate deviates from its expected range (based on the agent's internal model), the oracle agent flags the deviation before submitting its `OracleVote`.
3. **Submits votes.** Every 25 blocks, the agent computes the weighted median across live sources and submits a signed `OracleVote` to the consensus layer.
4. **Earns oracle mining rewards.** Validators whose votes are within one standard deviation of the final aggregate ISFR receive oracle mining rewards proportional to their stake. Outlier votes (more than 2 sigma from the median) receive reduced rewards.

Oracle agents improve over time. Their internal models of source behavior sharpen with each ISFR update, reducing false deviation alerts and improving vote accuracy. This learning happens through the same EpisodeLogger and efficiency tracking that all Roko agents use.

### Hedging agents

Hedging agents manage clearing profiles on behalf of users (DAOs, funds, individual accounts). Their responsibilities:

1. **Create clearing profiles.** Based on the user's risk parameters (direction, trigger level, max notional, max fee), the agent constructs and submits the profile transaction.
2. **Monitor margin.** For active perpetual positions, the agent tracks mark-to-market PnL against available margin and alerts when the position approaches maintenance margin.
3. **Auto-rebalance.** When a profile's fill reaches a configurable threshold (e.g., 80% of max notional), the agent can create a new profile with adjusted parameters to continue hedging.
4. **Report performance.** The agent generates periodic reports comparing hedged vs. unhedged returns, funding costs, and effective rate locked.

Hedging agents use the `blockchain` domain profile with clearing-specific extensions. Their heartbeat ticks accelerate during volatile ISFR regimes (crisis: 2s ticks) and slow during calm periods (calm: 120s ticks).

### Advisory agents

Advisory agents analyze ISFR history and recommend hedging strategies. They do not trade. Their outputs are recommendations, reports, and educational content.

1. **Rate analysis.** The agent tracks ISFR moving averages (1h, 24h, 7d), regime transitions, source divergences, and historical percentile rankings.
2. **Profile recommendations.** Given a user's yield exposure (protocol, amount, duration), the agent recommends clearing profile parameters: trigger level, max notional, max fee, expiry.
3. **User education.** The agent produces explanatory content about yield perpetual mechanics, hedging strategies, and risk parameters. This content is stored in the InsightStore and surfaced through the TUI or chat interface.
4. **Backtesting.** The agent runs historical simulations: "If you had created this clearing profile 90 days ago, your hedged return would have been X% vs. unhedged Y%."

Advisory agents use a `research + blockchain` composed profile. They need blockchain data (ISFR history, clearing data) and research capabilities (synthesis, citation, report generation).

### Research agents

Research agents in the ISFR ecosystem produce Knowledge Futures -- forward-looking analyses of rate dynamics:

1. **Rate regime forecasting.** The agent analyzes macro and on-chain signals to predict regime transitions (e.g., "ISFR likely to enter Volatile regime within 48 hours due to leveraged position buildup on Aave").
2. **Source correlation analysis.** The agent studies how source rates co-move and diverge, identifying structural breaks that may signal market microstructure changes.
3. **Cross-market analysis.** The agent compares ISFR dynamics with TradFi rates (SOFR, Fed Funds), identifying arbitrage or divergence patterns.
4. **Publication.** Research outputs are committed to the InsightStore with CRPS-scored predictions embedded. Other agents query these entries for decision support.

### Verifier agents

Verifier agents maintain the quality of ISFR-related InsightStore entries:

1. **Challenge low-quality predictions.** When an agent's CRPS scores consistently rank in the bottom percentiles, verifier agents flag the predictions as unreliable and downweight them in query results.
2. **Cross-validate insights.** Verifier agents check ClearingInsight entries against on-chain data, flagging discrepancies between reported surplus values and independently computed values.
3. **Reputation monitoring.** Verifier agents track the 30-day rolling CRPS percentiles for all predicting agents and publish tier transition reports.
4. **Methodology auditing.** Verifier agents compare the ISFR computation against the specification, flagging any validator whose votes systematically deviate from the expected methodology.

### Trading agents

Trading agents take directional bets on ISFR:

1. **Rate speculation.** The agent goes long (expecting ISFR to rise) or short (expecting ISFR to fall) based on its internal model of rate dynamics.
2. **CRPS prediction competition.** The agent commits predictions to the `PROOF_LOG` before each ISFR update, competing for epistemic reputation tier advancement. Higher tiers grant more knowledge query quota and clearing priority.
3. **Clearing surplus capture.** The agent submits limit orders timed to maximize its share of cooperative clearing surplus. Orders placed at prices far from the expected clearing price capture more surplus when filled.
4. **Cross-instrument arbitrage.** When the yield perpetual price diverges from ISFR (positive or negative premium), the agent arbitrages the divergence by trading the perp against the underlying rate exposure.

Trading agents use the `blockchain` domain profile at high tick frequency (crisis: 2s, volatile: 5s). Their gate pipeline includes a `risk_limit` gate that enforces position size and concentration limits.

---

## 21. Multi-chain ISFR sources

ISFR V1 reads four sources. V2 expands to seven or more. These sources span multiple blockchains, each with its own block time, finality model, and RPC interface.

### V1 source chain mapping

| Source | Chain | Contract/endpoint | Block time | Finality |
|--------|-------|-------------------|-----------|----------|
| Aave V3 USDC supply APY | Ethereum mainnet | LendingPool `getReserveData()` | 12s | Probabilistic (~13 min / 2 epochs) |
| Compound V3 USDC supply APY | Ethereum mainnet | Comet `getSupplyRate()` | 12s | Probabilistic (~13 min / 2 epochs) |
| Ethena sUSDe 7-day yield | Ethereum mainnet | sUSDe staking contract `totalAssets()` / `totalSupply()` | 12s | Probabilistic (~13 min / 2 epochs) |
| ETH Beacon Chain staking yield | Consensus layer | Beacon API `/eth/v1/beacon/states/head/validators` | 6.4 min (epoch) | 2 epochs (~13 min) |

V1 is Ethereum-only. A single `ChainActor` for Ethereum mainnet plus a Beacon Chain API client covers all four sources. This simplifies the initial deployment: one chain connection, one set of RPC endpoints, one reorg handling strategy.

### V2 source chain expansion

| Source | Chain | Contract/endpoint | Block time | Finality |
|--------|-------|-------------------|-----------|----------|
| Morpho USDC yield | Ethereum mainnet | Morpho vault `totalAssets()` / `totalSupply()` | 12s | Probabilistic |
| Aave V3 on Base | Base (L2) | LendingPool `getReserveData()` | 2s | Probabilistic (L1 confirmations) |
| Compound V3 on Arbitrum | Arbitrum (L2) | Comet `getSupplyRate()` | ~250ms | Probabilistic (L1 confirmations) |

V2 requires ChainActors for three chains: Ethereum (Morpho + existing V1 sources), Base (Aave deployment), and Arbitrum (Compound deployment). Each actor operates independently at its chain's native speed.

### ChainActor subscription architecture

Each V2 source requires its own `ChainActor` subscription. The actor-per-chain model from PRD-09 (section 9) provides the foundation:

```
ChainActor(Ethereum) ──> SourceReading(Aave V3)
                    ──> SourceReading(Compound V3)
                    ──> SourceReading(Ethena sUSDe)
                    ──> SourceReading(Morpho)

ChainActor(Beacon)  ──> SourceReading(ETH staking)

ChainActor(Base)    ──> SourceReading(Aave V3 Base)

ChainActor(Arb)     ──> SourceReading(Compound V3 Arbitrum)
```

Each `SourceReading` flows to the ISFR aggregation layer, which collects readings from all actors and runs the weighted median computation. The aggregation layer handles clock skew between chains: readings are timestamped by source chain block time, and the aggregator uses the latest reading from each source within a configurable staleness window (120 seconds for lending rates, 24 hours for Ethena, 30 minutes for Beacon Chain).

### Cross-chain aggregation into unified rate

The ISFR oracle aggregates across chain actors into a single value:

1. Each ChainActor independently produces `SourceReading` events as it observes rate changes on its chain.
2. The `ISFRAggregator` maintains the latest reading from each source. When an ISFR computation window opens (every 25 Korai blocks), the aggregator snapshots all current readings.
3. Sources whose latest reading is older than their liveness timeout are excluded (section 5, source liveness detection).
4. The aggregator computes the weighted median across live sources.
5. The result is the ISFR value that validators submit in their `OracleVote`.

This design means ISFR reflects multi-chain rate conditions with the latency of the slowest chain that has a live source. For V1 (all Ethereum), latency is bounded by Ethereum's 12-second block time. For V2 (Ethereum + Base + Arbitrum), the bound remains Ethereum's 12 seconds because the L2 sources update faster than the L1 sources.

---

## Appendix A: Notation reference

| Symbol | Meaning |
|--------|---------|
| bps | Basis points. 1 bps = 0.01%. 100 bps = 1%. |
| ISFR | Internet Secured Funding Rate (this document) |
| SOFR | Secured Overnight Financing Rate (Federal Reserve) |
| CRPS | Continuous Ranked Probability Score (Gneiting & Raftery, 2007) |
| KKT | Karush-Kuhn-Tucker optimality conditions |
| VCG | Vickrey-Clarke-Groves mechanism |
| PT | Principal Token (Pendle terminology) |
| YT | Yield Token (Pendle terminology) |
| TVL | Total Value Locked |
| APY | Annual Percentage Yield |
| EMA | Exponential Moving Average |
| CLOB | Continuous Limit Order Book |
| sigma | Standard deviation of validator vote distribution |
| s_i | Stake weight of validator i (normalized, sum = 1) |
| v_i | ISFR value submitted by validator i |
| F(x) | Cumulative distribution function of a prediction |
| 1(x >= y) | Indicator function: 1 if x >= y, 0 otherwise |
| N | Number of validators in the active committee |
| k | Number of sources in the ISFR computation |
| p | Clearing price in basis points |

## Appendix B: ISFR precompile interface

```solidity
/// ISFR Oracle Precompile at address 0xA01 on Korai Kernel Plane.
interface IISFROracle {
    /// Returns the current ISFR value in basis points and its publication state.
    function current() external view returns (uint32 valueBps, uint8 state);

    /// Returns the full ISFR snapshot at a specific block height.
    /// Reverts if blockHeight is older than 90 days.
    function at(uint64 blockHeight) external view returns (ISFRSnapshot memory);

    /// Returns the time-weighted average ISFR between two block heights.
    /// Both blocks must be within the 90-day retention window.
    function twap(uint64 startBlock, uint64 endBlock) external view returns (uint32 twapBps);

    /// Returns the number of sources currently reporting (0-4 in V1, 0-7+ in V2).
    function activeSources() external view returns (uint32);

    /// Returns the current confidence score in basis points (0-10000 = 0-100%).
    function confidence() external view returns (uint16 confidenceBps);

    /// Returns the rate of change since the previous update, in signed basis points.
    function delta() external view returns (int32 deltaBps);
}

struct ISFRSnapshot {
    uint32 valueBps;
    uint64 blockHeight;
    uint64 timestamp;
    uint8  state;            // 0=Live, 1=Degraded, 2=Stale, 3=Halted
    uint16 confidenceBps;
    uint32 numSources;
    uint32 numValidatorVotes;
}
```

## Appendix C: Related PRDs

| Document | Relationship |
|----------|-------------|
| PRD-01 (Overview) | Defines ISFR, yield perpetuals, and cooperative clearing in the glossary. This PRD expands those definitions into full specifications. |
| PRD-03 (Cognitive engine) | CRPS prediction scoring feeds the same epistemic reputation system used for cognitive gating model selection. |
| PRD-04 (Context engineering) | VCG auction mechanism referenced in cooperative clearing surplus distribution. |
| PRD-05 (Knowledge and stigmergy) | ISFRInsight and ClearingInsight are InsightStore entry types. ISFR as knowledge production depends on the InsightStore architecture. |
| PRD-06 (Domains and arenas) | Generalized benchmark indices (section 12) connect to the arena framework. Agent Performance Index depends on arena scoring. |
| IMPL-06 (ISFR) | Implementation plan for everything specified in this PRD. |
