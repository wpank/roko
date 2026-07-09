# Yield Perpetuals: A Comprehensive Reference

## What This Document Covers

This document explains yield perpetual instruments from first principles. It is written for someone who understands how perpetual futures work on exchanges like Binance, Bybit, dYdX, or Hyperliquid, but has never encountered a perpetual contract whose underlying is a DeFi yield rate instead of an asset price.

No prior knowledge of any specific project, protocol, or proprietary system is assumed. All formulas, parameters, and mechanics are fully defined here.

---

## 1. What Is a Yield Perpetual?

A yield perpetual is a perpetual futures contract whose underlying is a DeFi yield reference rate -- measured in basis points -- rather than the spot price of an asset like BTC or ETH.

The instrument has no expiration date. It uses continuous funding rate payments to anchor its trading price to the underlying reference rate, exactly as traditional crypto perpetual futures anchor to spot prices. But instead of tracking "what does 1 BTC cost in USD," a yield perpetual tracks "what is the annualized cost of secured funding across DeFi lending protocols."

This is a category-level distinction from standard crypto perpetuals:

| Property | Standard Crypto Perp (e.g., BTC-PERP) | Yield Perpetual |
|----------|----------------------------------------|-----------------|
| Underlying | Asset spot price (e.g., BTC/USD) | DeFi yield reference rate (e.g., composite lending APY) |
| Quote unit | USD or stablecoin | Basis points (1 bp = 0.01%) |
| What a long position means | Bet that the asset price rises | Bet that DeFi lending yields rise |
| What a short position means | Bet that the asset price falls | Bet that DeFi lending yields fall (the hedging use case) |
| Funding rate purpose | Anchor perp price to spot price | Anchor perp price to the reference yield rate |

The concept is not entirely novel. In traditional finance, CME's SOFR futures and the legacy Eurodollar futures have provided rate-based derivatives for decades. What is new is combining the perpetual structure -- no expiry, continuous funding, single liquidity pool per benchmark -- with on-chain rate trading and settlement against a consensus-computed benchmark rate.

---

## 2. Why Yield Perpetuals Exist: The Problem They Solve

### 2.1 The Unhedged $49.5 Billion

DeFi lending protocols collectively hold approximately $49.5 billion in total value locked (TVL) as of April 2026. The largest venues:

| Protocol | TVL | Supply Rate (USDC) | Rate Hedging Available |
|----------|-----|---------------------|------------------------|
| Aave V3 | ~$23.5B | 3-8% variable | None |
| Morpho | ~$10B+ | 3-8% variable | None |
| Spark (MakerDAO) | ~$7.9B | Variable (DSR) | None |
| Compound V3 | ~$2.1B | 3-8% variable | None |

Every dollar deposited in these protocols carries unhedged variable rate exposure. A treasury holding $10M on Aave at 8% APY has no way to lock in that rate. If rates drop to 3%, the treasury faces a $500K annualized shortfall -- and today, there is no instrument to hedge against it.

### 2.2 Why Pendle's Expiring Futures Do Not Solve This

The most prominent existing approach to on-chain yield trading is Pendle, which peaked at $13.4B in TVL in September 2025 and processed $47.8B in 2025 trading volume. Pendle splits yield-bearing assets into Principal Tokens (PT) and Yield Tokens (YT) with fixed maturities.

The limitations of this model are structural, not competitive:

**Maturity fragmentation.** Every asset/maturity pair creates a separate market. "Aave USDC at 30 days," "Aave USDC at 90 days," and "Aave USDC at 180 days" are three separate pools with three separate liquidity books. This fragments capital across hundreds of pools, reducing depth in each.

**Manual rollover.** When a PT/YT pair expires, the holder must manually close the position and open a new one at the next maturity. This rollover process costs approximately 50+ basis points per cycle in execution costs and slippage, and creates operational burden.

**No benchmark rate.** Pendle trades yield on individual assets at individual venues. There is no aggregated, multi-source reference rate. Each pool reflects the idiosyncratic dynamics of one protocol's one token.

**No leverage.** Yield tokens provide 1:1 exposure. For treasuries managing tens of millions in yield exposure, the capital efficiency is poor.

Yield perpetuals solve all four problems simultaneously:

1. **Single pool per benchmark rate.** One instrument, one liquidity pool, no maturity fragmentation.
2. **No rollover.** The position persists indefinitely. Funding rate payments handle the anchoring continuously.
3. **Multi-source reference rate.** Settlement against a composite benchmark (described in Section 8) that aggregates rates across Aave, Compound, Ethena, and ETH staking.
4. **Leverage.** 10x maximum leverage with 10% initial margin, appropriate for rate markets where volatility is lower than asset prices.

Other protocols have attempted to address parts of this problem. IPOR (~$10-20M TVL) offers interest rate swaps with a proprietary index but has thin liquidity and a single-methodology aggregation that lacks manipulation resistance. Voltz attempted IR swaps on a concentrated liquidity AMM but shut down in December 2023. Spectra (~$44M TVL) uses the same PT/YT model as Pendle with the same structural constraints.

---

## 3. Instrument Definition

### 3.1 Contract Specification

| Parameter | Value | Notes |
|-----------|-------|-------|
| **Instrument type** | Perpetual futures (no expiry) | Continuous-funding, like Binance/Hyperliquid spot perps |
| **Underlying** | DeFi yield reference rate (e.g., composite lending APY across major protocols) | Not a single venue's rate; a multi-source benchmark |
| **Quote unit** | Basis points (1 bp = 0.01%) | Industry convention for rate products |
| **Contract multiplier** | $1 notional per 1 bp of rate movement per unit | Linear payoff; essential for clearing optimality proofs |
| **Minimum tick size** | 0.25 bp (0.0025%) | Matches CME SOFR futures tick size |
| **Lot size** | 1 unit ($1/bp) | Low barrier to entry |
| **Max leverage** | 10x (10% initial margin, 5% maintenance margin) | Conservative for rate markets |
| **Funding interval** | Every 8 hours | Standard perpetual funding cadence; 3 events per day |
| **Trading hours** | 24/7/365 | DeFi rates move continuously |
| **Initial margin** | 10% of notional | |
| **Maintenance margin** | 5% of notional | |

### 3.2 Payoff Structure

The payoff is linear in basis points:

```
PnL per unit = (Exit_bps - Entry_bps) x Direction x $1
```

Where:
- `Exit_bps` = reference rate value in basis points at exit
- `Entry_bps` = reference rate value in basis points at entry
- `Direction` = +1 for long, -1 for short

**Example -- Long position, rates rise:**

Enter long at 600 bps (6.00%). Reference rate rises to 650 bps (6.50%). Position size: 1,000 units.

```
PnL = (650 - 600) x (+1) x $1 x 1,000 = $50,000
```

**Example -- Short position, rates fall:**

Enter short at 600 bps (6.00%). Reference rate drops to 450 bps (4.50%). Position size: 500 units.

```
PnL = (600 - 450) x (+1) x $1 x 500 = $75,000
```

The linear payoff is a deliberate design choice, not a simplification. Convex or concave payoffs break the mathematical conditions (Karush-Kuhn-Tucker conditions) that allow cooperative clearing solutions to be proved globally optimal. Linearity preserves the convexity of the clearing optimization problem.

---

## 4. Long vs. Short Positions

### 4.1 Position Semantics

| Position | Economic Meaning | Who Uses It | Funding Behavior |
|----------|------------------|-------------|------------------|
| **Long** | Bet that the reference rate will rise | Speculators who think yields will increase; natural longs with floating-rate liabilities who benefit from higher rates | Pays funding when mark price > oracle rate; receives funding when mark price < oracle rate |
| **Short** | Bet that the reference rate will fall | Treasuries and depositors hedging floating-rate exposure; anyone wanting to lock in today's rate | Receives funding when mark price > oracle rate; pays funding when mark price < oracle rate |

### 4.2 The Short Side: The Core Hedging Use Case

The short side is the primary economic reason yield perpetuals exist. Consider a treasury with $10M deposited on Aave at 8% APY. The treasury is long the rate by default -- if rates stay high, the treasury earns well; if rates fall, income drops.

By shorting a yield perpetual, the treasury creates an offsetting position:

- **Rates fall:** The Aave position earns less, but the short yield perp gains value (the rate moved in the short's favor). The perp PnL compensates for the reduced lending income.
- **Rates rise:** The Aave position earns more, but the short yield perp loses value. The extra lending income compensates for the perp loss.

The net effect is that the treasury locks in an effective rate close to the entry level, regardless of where rates move. This is the same hedging dynamic that makes interest rate swaps the largest derivatives market in traditional finance.

### 4.3 The Long Side: Who Takes the Other Side?

For every short, there must be a long. Natural long counterparties include:

1. **Speculators** who believe rates will rise (e.g., expecting increased borrowing demand, upcoming protocol governance changes that raise utilization, or macro shifts toward higher on-chain yields).
2. **Borrowers** on lending protocols who want to hedge their floating-rate borrowing cost. If a borrower pays 8% variable and rates spike to 15%, a long yield perp offsets the increased cost.
3. **Arbitrageurs** who earn the funding rate when shorts are paying to maintain their hedge. If short demand is structurally higher than long demand, the funding rate will be positive, and longs earn a carry premium.

---

## 5. The Canonical Use Case: Aave Liquidation Backstop

This is the simplest end-to-end demonstration of why yield perpetuals paired with automated agents produce value that manual monitoring cannot.

### 5.1 Setup

- A user has 10 ETH deposited on Aave at an 8% USDC supply rate.
- The user has borrowed against this position, giving a health factor of 1.4.
- If the supply rate drops significantly, cascading liquidations could push the health factor below 1.0.

### 5.2 Without Yield Perpetuals

The user has two options:
1. Actively monitor the rate 24/7 and manually adjust the position.
2. Do nothing and risk liquidation.

In practice, 99% of DeFi users choose option 2 by default.

### 5.3 With Yield Perpetuals and an Automated Agent

1. The user sets a **clearing profile** -- a single on-chain transaction: "hedge me if the rate drops below 6%."
2. A reactive agent subscribes to the reference rate feed and the yield perp order book.
3. The agent's predictive model flags an elevated probability of the rate crossing 6% -- not after the drop, but as it begins forming.
4. The agent submits an intent to the clearing engine to open a short yield perp position.
5. The clearing engine matches the intent with counterparties who are long the rate (speculators, natural longs) in a cooperative batch.
6. The hedge executes at 6.15%.
7. The rate eventually drops to 5.2%.
8. Without the hedge, the user faces an estimated $2,340 loss or liquidation. With the hedge, the loss is limited to the hedging cost -- a fraction of the unhedged alternative.

**Total user actions: 1** (set the clearing profile once).

---

## 6. Mark Price Formula

The mark price governs unrealized PnL, margin calculations, and liquidation triggers. It blends the oracle reference rate with the order book mid-price to prevent manipulation of either component individually.

### 6.1 Standard Formula (Normal Conditions)

```
MarkPrice = OracleSpot + EMA_150s(OrderBookMid - OracleSpot)
```

Where:
- `OracleSpot` = the stake-weighted median reference rate from the oracle (the latest benchmark value)
- `OrderBookMid` = mid-price from the on-chain order book for the yield perp
- `EMA_150s` = exponential moving average with a 150-second time constant, using smoothing factor `alpha = 1 - exp(-dt / 150)`

The oracle component dominates (preventing order book manipulation), while the order book component allows the mark price to reflect genuine supply/demand dynamics not yet captured in the next reference rate update.

An alternative formulation used in some configurations weights the blend explicitly:

```
MarkPrice = 0.7 x OracleRate + 0.3 x EMA(OrderBookMid, 300s)
```

### 6.2 Degraded Conditions

When the reference rate oracle enters a degraded or stale state, the mark price formula adjusts to reduce reliance on potentially unreliable book prices:

| Oracle State | Mark Price Formula | Rationale |
|---|---|---|
| **Live** | `OracleSpot + EMA_150s(BookMid - OracleSpot)` | Normal blending |
| **Degraded** | `0.9 x OracleRate + 0.1 x EMA(BookMid, 600s)` | Longer EMA, more oracle weight |
| **Stale** | `1.0 x LastLiveRate` | Pure oracle, frozen at last known good value |
| **Halted** | Mark price frozen. No new liquidations. | Emergency mode |

### 6.3 Thin Liquidity Fallback

When the order book spread exceeds a threshold indicating unreliable book prices:

```
If BookSpread > SPREAD_THRESHOLD:
    MarkPrice = OracleSpot + EMA_30s(BookMid - MarkPrice)
```

The shorter 30-second EMA converges faster toward the oracle price when the book is unreliable.

---

## 7. Funding Rate Formula

The funding rate is the mechanism that anchors the perpetual's trading price to the underlying reference rate. Without funding, the perpetual could diverge arbitrarily from the benchmark.

### 7.1 Two-Component Structure

The yield perp funding rate has two components:

```
FundingRate = PremiumComponent + CarryComponent
```

#### Premium Component

This is the standard perpetual funding mechanism, measuring how far the trading price has diverged from the oracle:

```
PremiumComponent = clamp(
    EMA(BookMid - OracleRate, 300s) / OracleRate,
    -0.05%,    // floor: -5 bps per 8-hour period
    +0.05%     // cap: +5 bps per 8-hour period
)
```

When the perp trades above the oracle rate (positive premium), longs pay shorts. When below, shorts pay longs. This creates arbitrage pressure that pulls the perpetual price toward the benchmark.

The clamp to +/-0.05% per funding interval prevents unbounded liquidation risk during fast rate moves.

#### Carry Component (Unique to Yield Perpetuals)

Standard crypto perps do not need a carry component because their underlying (a spot price) has no inherent yield. Yield perps do -- the underlying itself is a rate, and holding rate exposure has an inherent cost.

```
CarryComponent = (ReferenceRate - RiskFreeRate) x (FundingInterval / Year)
```

Where:
- `ReferenceRate` = the rate the instrument tracks (e.g., composite DeFi lending APY)
- `RiskFreeRate` = the base cost of capital (e.g., ETH staking yield, used as crypto's "risk-free" rate analog)
- `FundingInterval / Year` = the fraction of a year elapsed in one funding interval (8 hours / 8,760 hours = ~0.000913)

The carry component prevents a free arbitrage from forming when the yield curve is not flat. Without it, one side of the trade would systematically receive a risk-free transfer at the other's expense during stable rate environments. The dual-rate term ensures that in steady state, holding a yield perp position costs exactly the hedging cost -- no free carry for either side.

### 7.2 Payment Calculation

Every 8 hours, funding payments are computed and applied:

```
FundingPayment = PositionSize x FundingRate
```

- If `FundingRate > 0`: longs pay shorts
- If `FundingRate < 0`: shorts pay longs

### 7.3 Comparison to Standard Crypto Perp Funding

| Aspect | Standard Crypto Perp (Binance/Hyperliquid) | Yield Perpetual |
|--------|---------------------------------------------|-----------------|
| Premium term | `(Mark - OracleSpot) / OracleSpot` | Same formula applied to rate values |
| Carry term | None (or fixed interest rate component) | `(ReferenceRate - RiskFreeRate) x dt` |
| Clamp | +/-0.05% per interval | Same clamp on premium; carry unclamped |
| Interval | 8 hours (standard) | 8 hours (configurable) |
| Settlement | USD-margined or coin-margined | USD-margined |

---

## 8. The Three-Layer Stack

Yield perpetuals do not operate in isolation. They are the top layer of a three-layer architecture. Each layer solves a specific problem that the layers above it depend on.

### 8.1 Layer 1: The Yield Perpetual Instrument (Product Layer)

This is the instrument itself -- the tradable contract with the specifications described in Sections 3 through 7. It defines what users trade, how positions work, and how PnL is computed.

**Problem it solves:** DeFi users have no continuous hedge for yield risk.

**Why a thinner solution fails:** Expiring futures (Pendle model) fragment liquidity across maturities and require manual rollover. There is no "set it and forget it" yield hedge in DeFi today.

### 8.2 Layer 2: The Reference Rate Index (Data Layer)

A yield perpetual needs something to settle against. That something is a composite reference rate -- a benchmark that aggregates yield signals from multiple DeFi protocols into a single, manipulation-resistant number.

The reference rate is called ISFR (Internet Secured Funding Rate). It is to DeFi what SOFR (Secured Overnight Financing Rate) is to traditional finance: the benchmark that financial instruments settle against.

**Key properties of the reference rate:**

**Multi-source composition.** The index aggregates rates from structurally distinct yield categories using a two-level aggregation:

| Source Class | What It Measures | V1 Sources | Weight |
|---|---|---|---|
| LENDING | Collateralized lending yield | Aave V3, Compound V3 | 0.60 |
| STRUCTURED | Multi-instrument strategy yield | Ethena sUSDe | 0.25 |
| FUNDING | Perpetual futures funding rate | Hyperliquid ETH perp | 0.10 |
| STAKING | Proof-of-stake validator yield | ETH staking rate | 0.05 |

**Two-level aggregation:**

- **Level 1 (intra-class):** Within each class, sources are aggregated via TVL-weighted median. This tolerates up to 49% of weight being corrupted within any single class.
- **Level 2 (inter-class):** The final rate is a weighted sum of the four class rates:

```
ISFR = 0.60 x LENDING + 0.25 x STRUCTURED + 0.10 x FUNDING + 0.05 x STAKING
```

**Worked example:**

| Class | Source Rates | TVL-Weighted Median | Class Weight |
|---|---|---|---|
| LENDING | Aave V3: 6.20%, Compound V3: 5.80% | 6.20% (Aave dominates by TVL) | 0.60 |
| STRUCTURED | Ethena sUSDe: 7.10% | 7.10% | 0.25 |
| FUNDING | Hyperliquid ETH perp: 12.40% | 12.40% | 0.10 |
| STAKING | ETH staking: 3.20% | 3.20% | 0.05 |

```
ISFR = 0.60 x 6.20% + 0.25 x 7.10% + 0.10 x 12.40% + 0.05 x 3.20%
     = 3.720% + 1.775% + 1.240% + 0.160%
     = 6.895%
     ~ 690 basis points
```

The design ensures that a spike in the FUNDING class (e.g., speculative mania driving perpetual funding to 200%) contributes at most `0.10 x 200% = 20 percentage points` to the composite, while the remaining 90% of weight anchors the rate near lending fundamentals.

**Sub-indices:** Every computation round also publishes the four class-level rates (LENDING, STRUCTURED, FUNDING, STAKING) as separate sub-indices. A protocol hedging Aave supply rate risk specifically can reference the LENDING sub-index directly, while the composite serves as the canonical settlement rate for yield perpetuals.

**Update cadence:** Every 10 seconds (8,640 updates per day). For comparison, SOFR publishes once per day and IPOR updates approximately every 15 minutes.

**Manipulation resistance:** The dual-median design means an attacker must compromise both the source-level aggregation AND the validator-level consensus simultaneously -- two independent security layers. A flash loan that spikes one protocol's rate is absorbed by the median: the index shifts by at most a few basis points rather than tracking the outlier.

**Problem Layer 2 solves:** Yield perps need a single agreed-upon reference rate that cannot be gamed by manipulating one lending venue.

**Why a thinner solution fails:** Pulling from one source (e.g., just Aave's rate) lets a single protocol's utilization curve dynamics, governance parameter changes, or flash loan attacks move the entire settlement benchmark.

### 8.3 Layer 3: Cooperative Clearing (Execution Layer)

Orders on yield perpetuals are not matched via a traditional continuous limit order book (CLOB). Instead, they are matched in cooperative batch clearing cycles.

**How it works:**

1. **Accumulation.** Orders enter a pending batch from three sources: active limit orders, triggered clearing profiles, and liquidation orders. The batch seals when any trigger fires (5+ orders, 10 seconds elapsed, 3:1 imbalance, or 10+ bp rate movement).

2. **Solver competition.** Multiple independent solver agents have 800 milliseconds to compute the optimal clearing solution -- the uniform clearing price that maximizes total surplus across all orders.

```
TotalSurplus = sum(BuyLimit_i - ClearingPrice) x FillSize_i     (for filled buys)
             + sum(ClearingPrice - SellLimit_j) x FillSize_j     (for filled sells)
```

3. **KKT verification.** The submitted solution is verified against Karush-Kuhn-Tucker optimality conditions. For the yield perp clearing problem (which is a convex linear program due to linear payoffs and continuous positions), KKT conditions are necessary AND sufficient for global optimality. Verification complexity is O(n) -- a single pass through the order set.

4. **Execution.** Verified fills are executed atomically.

The three KKT conditions verified:
- **Primal feasibility:** All order constraints satisfied (limit prices respected, partial fills honored, buy volume = sell volume).
- **Dual feasibility:** Shadow prices on binding constraints are non-negative.
- **Complementary slackness:** Either a constraint is binding or its dual variable is zero.

**Privacy preservation.** Orders are submitted as encrypted ciphertexts (ECIES-sealed) to a Trusted Execution Environment (TEE) enclave. The enclave decrypts, matches, generates the KKT optimality certificate, and produces an attestation. This prevents strategy leakage before fills -- a critical property for institutional participants who cannot have their hedging intent visible to adversarial traders.

**Problem Layer 3 solves:** Agents need to submit orders without revealing strategies, and batch clearing needs provable optimality.

**Why a thinner solution fails:** A plain CLOB leaks intent before the fill, enabling front-running. A pure on-chain match has gas costs and latency that prevent the 10-second batch cycle. Traditional off-chain matching lacks verifiability.

### 8.4 How the Three Layers Interact

The data flow through the stack works as follows:

1. **Layer 2 (reference rate)** reads raw yield data from DeFi protocols (Aave, Compound, Ethena, ETH staking) and produces the benchmark rate every 10 seconds.
2. **Layer 1 (instrument)** uses the benchmark rate to compute mark prices, funding rates, and liquidation triggers.
3. **Layer 3 (clearing)** receives orders from traders and agents, matches them in batches using the mark price from Layer 1, and executes fills.
4. Market-discovered prices from Layer 3 feed back into Layer 2 as an endogenous signal, creating a hybrid oracle that blends external observation with market price discovery. At launch, the oracle dominates; as the market matures, the market-clearing signal becomes progressively more informative.

```
DeFi Protocols (Aave, Compound, Ethena, ETH staking)
    |
    v
[Layer 2: Reference Rate Index] -- computes benchmark every 10s
    |
    v
[Layer 1: Yield Perpetual Instrument] -- mark price, funding, margins
    |                                          ^
    v                                          |
[Layer 3: Cooperative Clearing] -- matches orders, executes fills
    |                                          |
    +------ market-clearing price feeds back --+
```

---

## 9. Settlement Mechanics, Margins, and Liquidation

### 9.1 Position Lifecycle

1. **Open.** User or agent submits an order with direction (long/short), size (in units), and collateral. Collateral must meet the 10% initial margin requirement.
2. **Mark.** Between clearing rounds, positions accrue unrealized PnL based on the current mark price vs. their entry rate.
3. **Settle.** During each 8-hour funding interval, funding payments flow between longs and shorts. The funding rate is computed as described in Section 7.
4. **Close.** User or agent closes the position, realizing PnL and reclaiming remaining collateral.

### 9.2 Margin Requirements

| Parameter | Value |
|-----------|-------|
| Initial margin | 10% of notional (10x max leverage) |
| Maintenance margin | 5% of notional |
| Liquidation trigger | `equity / notional < maintenance_margin` equivalently, mark-to-market loss exceeds `(margin - maintenance)` |
| Insurance fund contribution | 0.5 bps per cleared trade |

### 9.3 Liquidation Example

A trader opens a long position with $10,000 notional at 10x leverage ($1,000 margin).

```
Available margin before liquidation = $1,000 (initial) - $500 (maintenance) = $500
```

At $1/bp per unit with 10,000 units, a $500 loss corresponds to a 50 bp adverse rate move.

If the trader is long at 600 bps and the mark price drops below 550 bps, the position becomes liquidatable.

Liquidation is permissionless -- any address can trigger the liquidation of an undercollateralized position and receive a 2% bonus from the liquidated margin. Liquidation orders enter the next cooperative clearing round.

### 9.4 Oracle Liveness Guards

Liquidations are paused when the reference rate oracle enters Stale or Halted state. This prevents liquidations based on stale or unreliable price data -- a failure mode that has caused significant losses in other DeFi protocols.

The phase ordering within each block guarantees liquidations always use the freshest data:

```
Phase 1: ORACLE      -- apply latest rate update
Phase 2: ACCRUAL     -- compute funding using fresh oracle prices
Phase 3: LIQUIDATION -- check margins using fresh mark prices
Phase 4: PERPS       -- match orders
```

### 9.5 Publication States and Circuit Breakers

The reference rate operates in one of four states based on data availability and validator agreement:

| State | Condition | Yield Perp Behavior |
|-------|-----------|---------------------|
| **Live** | 3+ sources reporting, confidence >= 70% | Normal operations |
| **Degraded** | 2 sources or confidence 50-70% | Normal with warning; tight clearing profile triggers may pause |
| **Stale** | 1 source reporting | No new positions; existing positions use frozen rate |
| **Halted** | 0 sources or confidence < 50% | Liquidations paused; no new positions; emergency CLOB mode |

Recovery from Degraded/Stale to Live requires confidence exceeding 80% for 3 consecutive update periods (30 seconds). This hysteresis (70% down, 80% up) prevents oscillation.

---

## 10. Comparison to Traditional Crypto Perpetuals

Yield perpetuals share the core mechanism of standard crypto perps -- continuous funding to anchor the contract to a reference value -- but differ in several important ways:

| Dimension | Standard Crypto Perp | Yield Perpetual |
|-----------|----------------------|-----------------|
| **Underlying** | Spot asset price | Composite yield rate |
| **Volatility regime** | High (BTC can move 10%+ daily) | Lower (DeFi rates rarely move 100+ bps daily) |
| **Appropriate leverage** | 20-125x common | 10x max (lower volatility = less liquidation risk but also less need for extreme leverage) |
| **Funding rate** | Single premium term | Premium + carry term |
| **Reference source** | Spot exchanges (Binance, Coinbase, etc.) | Multi-source benchmark index |
| **Primary use case** | Speculation on price direction | Hedging floating-rate exposure |
| **Counterparty profile** | Primarily retail speculators | Treasuries (short), speculators and borrowers (long) |
| **Oracle requirements** | Spot price feeds (well-established) | Multi-venue yield rate aggregation (novel) |
| **Market sizing** | ~$100B+ daily volume across exchanges | Nascent; benchmarks against $665.8T TradFi IR derivatives |
| **Clearing mechanism** | Continuous matching engine | Cooperative batch clearing (KKT-verified) |

The most important structural difference is in the funding rate. Standard perps have a single premium-based term. Yield perps add a carry component because the underlying itself is a rate -- there is an inherent cost to holding rate exposure that does not exist when the underlying is a price.

---

## 11. All Formulas, Constants, and Parameters

### 11.1 Core Formulas

**PnL (per unit):**
```
PnL = (Exit_bps - Entry_bps) x Direction x $1
Direction: +1 (long), -1 (short)
```

**Mark Price (Live state):**
```
Mark = OracleSpot + EMA_150s(BookMid - OracleSpot)
EMA alpha = 1 - exp(-dt / 150)
```

**Alternative mark price formulation:**
```
Mark = 0.7 x OracleRate + 0.3 x EMA(BookMid, 300s)
```

**Funding Rate:**
```
FundingRate = PremiumComponent + CarryComponent

PremiumComponent = clamp(
    EMA(BookMid - OracleRate, 300s) / OracleRate,
    -0.05%, +0.05%
)

CarryComponent = (ReferenceRate - RiskFreeRate) x (FundingInterval / Year)
```

**Funding Payment:**
```
FundingPayment = PositionSize x FundingRate
```

**Reference Rate (ISFR) Two-Level Computation:**
```
Level 1 (per class): TVL-weighted median of source rates within each class
Level 2 (composite): ISFR = 0.60 x LENDING + 0.25 x STRUCTURED + 0.10 x FUNDING + 0.05 x STAKING
```

**Confidence Score:**
```
confidence = sum(s_i for all i where |v_i - median| <= sigma) / sum(all s_i)
```

**Total Clearing Surplus:**
```
TotalSurplus = sum((BuyLimit_i - ClearingPrice) x FillSize_i)   [filled buys]
             + sum((ClearingPrice - SellLimit_j) x FillSize_j)   [filled sells]
```

### 11.2 Constants and Parameters

| Parameter | Value |
|-----------|-------|
| Contract multiplier | $1 / bp / unit |
| Minimum tick | 0.25 bp |
| Lot size | 1 unit |
| Initial margin | 10% |
| Maintenance margin | 5% |
| Max leverage | 10x |
| Funding interval | 8 hours |
| Funding premium clamp | +/- 0.05% per interval |
| Insurance fund contribution | 0.5 bps per trade |
| Liquidation bonus | 2% of liquidated margin |
| LENDING class weight | 0.60 |
| STRUCTURED class weight | 0.25 |
| FUNDING class weight | 0.10 |
| STAKING class weight | 0.05 |
| Oracle update cadence | 10 seconds (every 25 blocks at 400ms block time) |
| Mark price EMA time constant | 150 seconds (standard) / 30 seconds (thin liquidity fallback) |
| Clearing batch max wait | 10 seconds |
| Solver computation window | 800 milliseconds (2 blocks) |
| Confidence threshold: Live -> Degraded | 70% |
| Confidence threshold: Degraded -> Live | 80% (hysteresis) |
| Historical rate retention | 90 days (~19.4 million snapshots) |

### 11.3 V2 Parameters (Governance-Adjustable)

| Parameter | V1 Value | V2 Range |
|-----------|----------|----------|
| Max weight per source class | Fixed | Governance rails: e.g., LENDING 30-80%, FUNDING 0-20% |
| Source confidence | Governance-assigned (0-100) | MSPE leave-one-out (automatic) |
| Intra-class aggregation | TVL-weighted median | Cost-stratified trimmed mean |
| Class weight adaptation | Fixed | Bates-Granger optimal combination with bounds |
| Smoothing | EMA | Kalman filter (separates measurement noise from process noise) |

---

## 12. Market Sizing and the Thesis

### 12.1 The TradFi Interest Rate Derivatives Market

Interest rate derivatives are the largest financial market in the world by notional:

| Metric | Value | Source |
|--------|-------|--------|
| OTC IRD notional outstanding | $665.8T (mid-2025) | BIS Statistical Bulletin, Dec 2025 |
| Share of total OTC derivatives | 78.7% of $845.7T | BIS |
| OTC IRD daily turnover | $7.9T (April 2025) | BIS Triennial Survey 2025 |
| Growth in daily turnover (2022-2025) | +59% (from $5.0T) | BIS Triennial Survey |
| SOFR-linked OIS traded notional (2024) | $72.1T | ISDA, June 2025 |
| SOFR OIS growth (2021-2024) | 11.8x (from $6.1T) | ISDA |
| OIS share of total IRD (2024) | 66.6% | ISDA |

The structural shift is significant: overnight index swaps (OIS) -- the instrument class most directly analogous to yield perpetuals -- now account for two-thirds of all interest rate derivative volume. This restructuring followed the LIBOR-to-SOFR transition and concentrated the market around overnight secured rates.

DeFi lending protocols produce exactly this type of rate: variable, collateralized, with transparent on-chain settlement. The parallel is direct.

### 12.2 The Six-Order-of-Magnitude Gap

| Metric | Value |
|--------|-------|
| TradFi interest rate derivative notional | ~$665.8T |
| DeFi lending TVL (unhedged rate exposure) | ~$49.5B |
| On-chain interest rate derivative TVL | < $100M |

The gap between DeFi lending TVL ($49.5B in assets carrying unhedged rate exposure) and on-chain interest rate derivative activity (under $100M) is six orders of magnitude. This is not a marketing or timing problem. It is the absence of a foundational primitive: a credible, multi-source benchmark rate against which derivatives can settle.

### 12.3 The Benchmark Flywheel

Every successful benchmark rate in history has exhibited the same network effect:

1. A credible rate is published.
2. Derivatives reference it.
3. Derivatives create hedging instruments.
4. Institutional capital enters (institutions require hedging capability).
5. Institutional capital deepens liquidity.
6. Deeper liquidity makes the benchmark more credible.
7. Greater credibility attracts more derivatives.

Once this flywheel reaches critical mass, displacement becomes prohibitively expensive. LIBOR survived 30+ years of known structural deficiencies because the switching costs outweighed the design flaws. SOFR displaced LIBOR only through coordinated regulatory force.

The thesis is that the first credible DeFi benchmark rate will capture the same position in on-chain markets that SOFR holds in traditional finance. The causal chain is explicit: **benchmark rate -> derivative pricing -> hedging instruments -> institutional capital -> market depth -> lower borrowing costs -> more derivatives -> stronger benchmark.** Remove the benchmark and the entire chain does not start.

### 12.4 Why Now

Several structural conditions align:

1. **DeFi lending has reached institutional scale.** $49.5B in TVL is large enough to sustain a benchmark-quality rate and large enough that the hedging gap is economically meaningful.
2. **The LIBOR-to-SOFR transition proved the template.** The $250T migration demonstrated that benchmark transitions are possible, that transaction-based rates win over survey-based rates, and that overnight secured rates become the standard.
3. **Perpetual futures are a proven DeFi primitive.** Combined daily volume across Binance, Bybit, Hyperliquid, dYdX, and other venues routinely exceeds $100B. The mechanism works. Applying it to rates is an extension, not an invention.
4. **Existing attempts have failed to capture the benchmark position.** Pendle trades yield but produces no benchmark. IPOR publishes an index but lacks manipulation resistance and institutional credibility. Voltz shut down entirely. The position is open.

---

## Appendix: Clearing Profile Specification

A clearing profile is a persistent, on-chain intent that automates the hedge setup described in Section 5. It sits dormant until market conditions activate it, creating the "set it and forget it" hedge that makes yield perpetuals accessible to non-expert users.

```
ClearingProfile {
    account:          Address     // Owner
    market:           bytes32     // Market identifier (e.g., "ISFR-PERP-V1")
    direction:        Direction   // LONG or SHORT
    trigger:          uint256     // ISFR threshold in bps that activates the profile
                                 // SHORT: activates when ISFR < trigger
                                 // LONG: activates when ISFR > trigger
    maxNotional:      uint256     // Maximum notional exposure in USD
    maxFeeBps:        uint16      // Maximum acceptable clearing fee (bps)
    expiry:           uint64      // Expiry timestamp (0 = no expiry)
    minFillNotional:  uint256     // Minimum fill per clearing round (prevents dust)
    maxRounds:        uint32      // Max clearing rounds to participate in (0 = unlimited)
}
```

Lifecycle:
1. **Created** via one transaction (~50K gas).
2. **Dormant** with zero carrying cost. No keeper, no monitoring.
3. **Activated** when the reference rate crosses the trigger threshold.
4. **Filled** across cooperative clearing rounds until max notional, max rounds, or expiry.
5. **Complete/Cancelled.** The resulting position is a standard yield perpetual position.

If rates never cross the trigger, the profile never activates. The hedge costs $0. It is free insurance.

---

## Appendix: Worked Hedging Example -- $10M DAO Treasury

**Situation.** A DAO holds $10M USDC on Aave V3 at 8.00% APY. The treasury is concerned about rate compression over 6 months. If rates drop to 3.00%, the treasury loses ~$250,000 in expected yield.

**Action.** Create a clearing profile:
- Direction: SHORT
- Trigger: ISFR < 700 bps (7.00%)
- Max notional: $10,000,000
- Max fee: 10 bps
- Expiry: 180 days

**Scenario A: Rates drop.** Over 3 months, ISFR declines from 8.00% to 3.00%. The profile activates when ISFR crosses 7.00%, entering a short at 693 bps (clearing price).

```
Effective position (partial fill): 5,000 units
PnL on perp = (693 - 300) x $1 x 5,000 = $1,965,000

Aave yield shortfall over 6 months:
  Expected at 8.00%: $10M x 8.00% x (180/365) = $394,521
  Actual at avg 5.00%: $10M x 5.00% x (180/365) = $246,575
  Shortfall: $147,946

Funding cost over 90 days at avg 0.01% per 8h interval:
  5,000 x 693 x 0.01% x (90 x 3) = ~$935

Net result: +$1,965,000 - $935 = $1,964,065 gain on hedge
             vs. $147,946 yield shortfall
```

The hedge cost $935 and generated $1.96M against a $148K shortfall. The treasury is protected.

**Scenario B: Rates stay high.** ISFR never drops below 7.00%. The profile never activates. The treasury earned 8.00% APY on $10M. Cost of the hedge: $0.

---

## References

1. Bank for International Settlements. "OTC derivatives statistics at end-June 2025." BIS Statistical Bulletin, December 2025.
2. Bank for International Settlements. "OTC interest rate derivatives turnover in April 2025." BIS Triennial Central Bank Survey, 2025.
3. International Swaps and Derivatives Association. "Key Trends in the Size and Composition of OTC Derivatives Markets in the First Half of 2025." ISDA, 2025.
4. International Swaps and Derivatives Association. "Interest Rate Derivatives Trading in the US, EU and UK." ISDA Research, June 2025.
5. Federal Reserve Bank of New York, Alternative Reference Rates Committee. SOFR Transition documentation.
6. IOSCO. "Principles for Financial Benchmarks." International Organization of Securities Commissions, FR07/13, July 2013.
7. Kim, H. & Park, A. "Designing Funding Rates for Perpetual Futures." arXiv:2506.08573, 2025.
8. DeFiLlama. Lending protocols dashboard. Accessed April 2026.
