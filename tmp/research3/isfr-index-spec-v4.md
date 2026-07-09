# ISFR: The Internet Secured Funding Rate

## Index Specification v4.0 — Nunchi Labs

---

## Executive Summary

The Internet Secured Funding Rate (ISFR) is a composite benchmark index representing the cost of secured funding across decentralized finance. It aggregates yield signals from the largest DeFi lending, staking, structured yield, and funding rate protocols into a single, validator-computed rate — published on-chain every 10 seconds via a consensus-level oracle precompile on Korai, a blockchain purpose-built for autonomous economic agents.

This document specifies ISFR's methodology, architecture, competitive positioning, and credibility roadmap. It is written for institutional audiences evaluating Korai as infrastructure and ISFR as a settlement benchmark.

**Key claims, with evidence:**

- The global OTC interest rate derivatives market reached $665.8 trillion in notional outstanding as of mid-2025 (78.7% of the $845.7T total OTC derivatives market). [BIS Statistical Bulletin, December 2025; ISDA H1 2025 report]
- OTC interest rate derivatives daily turnover hit $7.9 trillion in April 2025, up 59% from $5.0T in 2022. [BIS Triennial Survey 2025]
- SOFR-linked OIS traded notional rose from $6.1T in 2021 to $72.1T in 2024 — an 11.8x increase in three years. OIS now accounts for 66.6% of all interest rate derivative volume. [ISDA, June 2025]
- DeFi lending protocols hold approximately $49.5B in TVL (April 2026), with Aave ($23.5B), Morpho ($10B+), Spark ($7.9B), and Compound ($2.1B) as leading venues. Total DeFi TVL is approximately $94B. [DeFiLlama, April 2026]
- On-chain interest rate derivative products collectively hold under $100M in TVL. The largest attempt (Voltz) shut down in December 2023. No validator-computed, multi-source benchmark rate exists on any chain.
- ISFR is designed to fill this gap — providing the foundational rate layer that enables DeFi interest rate derivatives at institutional scale.

---

## 1. Why Benchmark Rates Are Natural Monopolies

Benchmark rates exhibit the strongest network effects in finance. Every derivative contract, every hedging strategy, every institutional framework that references a benchmark rate raises the cost of switching to any alternative. This is not a theoretical observation — it is the empirical record of every successful benchmark in history.

### 1.1 The LIBOR-to-SOFR Precedent

The transition from LIBOR to SOFR is the most instructive case study in benchmark dynamics:

- **1986–2014**: LIBOR governed over $350 trillion in financial contracts globally, despite known structural weaknesses (survey-based, susceptible to manipulation, thin underlying transaction volume). Switching costs — legal, operational, systemic — kept it in place for decades.
- **2014**: The Alternative Reference Rates Committee (ARRC) was convened by the Federal Reserve Bank of New York to identify a replacement for USD LIBOR.
- **2017**: ARRC selected SOFR (Secured Overnight Financing Rate), based on the ~$1 trillion/day U.S. Treasury repo market. SOFR's advantage was not sophistication — it was credibility: transaction-based, deep underlying volume, administered by the NY Fed.
- **2018–2023**: A five-year transition migrated $250+ trillion in contracts from LIBOR to SOFR. The transition required regulatory mandates, legislative backstops (LIBOR Act of 2022), and coordinated industry effort.
- **June 30, 2023**: All remaining USD LIBOR tenors ceased publication permanently.

The lesson: SOFR won not because it was the theoretically optimal benchmark design. SOFR won because it was first to achieve credibility — transaction-based methodology, institutional backing (NY Fed), and enough early adoption to trigger the network effects that make switching costs prohibitive.

The same dynamics apply on-chain. DeFi has $49.5B in lending TVL and zero credible benchmark rates. The first rate that achieves institutional credibility will capture the position — and the network effects that defend it.

### 1.2 The Benchmark Flywheel

Benchmark rates create a self-reinforcing cycle:

1. A credible rate is published → Derivatives reference it
2. Derivatives create hedging instruments → Institutional capital enters (institutions require hedging)
3. Institutional capital deepens liquidity → The benchmark becomes more credible
4. Greater credibility attracts more derivatives → Cycle accelerates

Once this flywheel reaches critical mass, displacement becomes prohibitively expensive. LIBOR survived 30+ years of known deficiencies because the switching costs outweighed the design flaws. SOFR displaced LIBOR only through regulatory force.

ISFR is designed to initiate this flywheel for DeFi. The rate must be credible enough to attract the first derivatives, which attract the first institutional capital, which deepens the liquidity that makes the rate more credible. Every element of ISFR's design — multi-source methodology, validator consensus, IOSCO alignment — serves this single objective.

---

## 2. The Opportunity: $665.8 Trillion in TradFi, $49.5 Billion in DeFi, Zero On-Chain Benchmark

### 2.1 The TradFi Interest Rate Derivatives Market

The interest rate derivatives market is the largest financial market in the world by notional volume:

| Metric | Value | Source |
|--------|-------|--------|
| OTC IRD notional outstanding | $665.8T (mid-2025) | BIS Statistical Bulletin, Dec 2025 |
| Share of total OTC derivatives | 78.7% of $845.7T | BIS |
| OTC IRD daily turnover | $7.9T (April 2025) | BIS Triennial Survey 2025 |
| Growth in daily turnover (2022→2025) | +59% (from $5.0T) | BIS Triennial Survey 2025 |
| SOFR-linked OIS traded notional (2024) | $72.1T | ISDA, June 2025 |
| SOFR OIS growth (2021→2024) | 11.8x (from $6.1T) | ISDA, June 2025 |
| OIS share of total IRD (2024) | 66.6% ($244.0T of $366.3T) | ISDA, June 2025 |

The structural shift is significant: overnight index swaps (OIS) — the instrument class most directly relevant to ISFR — now account for two-thirds of all interest rate derivative trading volume, up from a minority share before SOFR adoption. The LIBOR-to-SOFR transition did not merely replace one benchmark with another; it restructured the entire market around overnight secured rates.

This restructuring creates the template for DeFi. Overnight secured lending rates are exactly what DeFi lending protocols produce — variable-rate, collateralized, with transparent on-chain settlement.

### 2.2 DeFi's $49.5 Billion Gap

DeFi lending is a $49.5 billion market (DeFiLlama, April 2026). But every dollar lent carries unhedged rate exposure:

| Protocol | TVL (April 2026) | Supply Rate (USDC) | Rate Hedging Available |
|----------|-------------------|---------------------|------------------------|
| Aave V3 | ~$23.5B | 3–8% variable | None |
| Morpho | ~$10B+ | 3–8% variable | None |
| Spark (MakerDAO) | ~$7.9B | Variable (DSR) | None |
| Compound V3 | ~$2.1B | 3–8% variable | None |

A treasury with $10M earning variable yield on Aave has no way to lock in today's rate. If rates drop from 8% to 3%, that treasury faces a $500K annualized shortfall — and there is no instrument to hedge against it.

The gap is not from lack of demand. It is from lack of infrastructure: there is no credible, multi-source benchmark rate against which interest rate derivatives can settle.

### 2.3 The Competitive Landscape — And Why The Gap Remains

Demand for rate products is proven. But existing protocols address symptoms, not the root cause:

**Pendle** ($5.7B average TVL in 2025; peaked at $13.4B in September 2025; $47.8B in 2025 trading volume; ~$44.6M in fees; 36 employees):
- Offers yield tokenization — split yield-bearing assets into Principal Tokens (PT) and Yield Tokens (YT) with fixed maturities.
- Boros extension launched for funding rate trading ($80M open interest, $5.5B notional traded).
- Limitation: Expiring instruments with fragmented per-asset liquidity. No benchmark rate. Manual rollover required. Each asset/maturity pair is a separate market.

**Spectra** (~$44M current TVL; peaked ~$190M in 2025):
- Similar PT/YT yield tokenization model with Curve integration.
- Same expiration and liquidity fragmentation constraints as Pendle.

**IPOR** (~$10–20M TVL; Zug, Switzerland; founded 2021; 12 employees; $5.55M total funding via early-stage VC in February 2022):
- Interest rate swaps with a benchmark index.
- Single-methodology flat-average index (3 sources). No two-level aggregation, no sub-indices, no manipulation tolerance guarantees. Thin liquidity limits institutional utility.

**Voltz** (Sunset December 2023):
- Interest rate swaps on a concentrated liquidity AMM.
- The most prominent prior attempt. Shut down after failing to achieve sufficient liquidity and sustainable economics.

**Allora Network**:
- Decentralized AI inference network. Not a benchmark rate, but relevant as infrastructure for rate prediction models.

**Chainlink / Pyth / API3**:
- Price oracle networks. Provide data feeds but do not compute benchmark rates. Operate as separate operator layers outside chain consensus.

What is missing across all of these is the foundational layer: a multi-source, validator-computed, manipulation-resistant benchmark rate with sub-indices, published at the consensus layer, available in a single opcode. ISFR is that layer.

---

## 3. Two-Level Aggregation — How ISFR Is Computed

### 3.1 Design Philosophy

ISFR's methodology mirrors SOFR's approach — pool sources by type, then aggregate across types — adapted for DeFi's heterogeneous yield surface. The design follows five principles:

1. **Multi-source composition** — No single venue can move the rate. Unlike IPOR's flat 3-source average, ISFR aggregates across structurally distinct yield categories.
2. **Manipulation tolerance** — Weighted median aggregation tolerates up to 49% corrupted weight without affecting the output. Two independent layers of median defense compound this resistance.
3. **Validator-computed, not operator-dependent** — Korai validators compute ISFR as part of consensus, eliminating the separate operator trust assumption that Chainlink, Pyth, and API3 require.
4. **On-chain native** — Published via a precompile at the consensus layer (address `0xA01`), accessible to any contract in a single opcode with fixed gas cost. Not a contract call with variable gas.
5. **Extensible by design** — Adding new sources within a class improves precision without changing the formula. Adding a new class requires only a governance vote on one weight parameter.

These principles align with the IOSCO Principles for Financial Benchmarks (2013), which govern credible benchmark design globally — including SOFR, SONIA, and €STR.

### 3.2 Source Class Taxonomy

ISFR organizes DeFi's yield surface into four mutually exclusive, collectively exhaustive classes:

| Class | What It Measures | V1 Sources | V2 Candidates | Weight | Rationale |
|-------|------------------|------------|---------------|--------|-----------|
| **LENDING** | Collateralized lending yield | Aave V3, Compound V3 | Morpho, Spark, Maker DSR | 0.60 | Most analogous to SOFR (secured overnight funding); deepest, most stable DeFi yield market |
| **STRUCTURED** | Multi-instrument strategy yield | Ethena sUSDe | Pendle PT yields, SOFR on-chain | 0.25 | Delta-neutral yield captures funding with dampening; structurally distinct from lending |
| **FUNDING** | Perpetual futures funding rate | Hyperliquid ETH perp | dYdX, GMX | 0.10 | Real signal on speculative positioning; explicitly downscaled due to volatility |
| **STAKING** | Proof-of-stake validator yield | ETH staking rate | Lido stETH APR, rETH APR | 0.05 | Floor rate for the Ethereum economy; very stable, analogous to overnight repo |

**Why these weights:** Lending rates are the primary hedging target for DeFi treasuries. The 60% LENDING weight ensures ISFR tracks the rates hedgers care about most, while the remaining 40% captures funding pressure (speculative sentiment), structural yield (delta-neutral strategies), and the base-layer staking rate (Ethereum's risk-free equivalent). Weights are governance-adjustable; V1 values are starting points calibrated to current DeFi market structure.

**Hyperliquid as a data source, not a dependency.** Hyperliquid's ETH perpetual funding rate is consumed as a read-only data input — functionally identical to reading Aave's supply rate via RPC. Korai has no settlement, operational, or systemic dependency on any external exchange.

### 3.3 Level 1 — Intra-Class Aggregation (TVL-Weighted Median)

Within each class, sources are aggregated into a single class rate using a TVL-weighted median with confidence modulation:

```
effective_weight(source) = tvl(source) × (confidence(source) / 100)
```

Confidence modulates contribution: a new source entering with confidence = 30 contributes 30% of its TVL-proportional weight, enabling smooth phase-in without disrupting the existing rate. Confidence scores are governance-assigned during a 30-day probation period, then transition to automatic calibration via leave-one-out MSPE (Section 8.1).

The computation sorts source rates ascending, accumulates effective weights, and returns the value where cumulative weight reaches 50%. The TVL-weighted median tolerates up to 49% corrupted weight within each class: an attacker must corrupt sources representing more than 50% of effective weight to move the class rate by even 1 basis point.

### 3.4 Level 2 — Inter-Class Aggregation (Weighted Sum)

The final ISFR is a deterministic weighted sum of the four class rates:

```
ISFR = Σ (w_class × class_rate)
     = 0.60 × LENDING + 0.25 × STRUCTURED + 0.10 × FUNDING + 0.05 × STAKING
```

The two-level design creates a natural firewall against volatility contamination. If the funding rate spikes to 200% during a speculative mania, it contributes at most 0.10 × 200% = 20 percentage points to the composite. Meanwhile, the other three classes — holding 90% of the weight — anchor the rate near their stable levels.

Under a flat equal-weight average of all sources (the approach IPOR uses), the same spike would dominate the composite. ISFR's architecture prevents this structurally.

### 3.5 Worked Example — Full Two-Level Computation

Suppose the following source rates are observed at a given 10-second epoch:

**Level 1 — Intra-class medians:**

| Class | Sources (TVL, Confidence) | Rates | TVL-Weighted Median |
|-------|---------------------------|-------|---------------------|
| LENDING | Aave V3 ($23.5B, conf=95), Compound V3 ($2.1B, conf=90) | 6.20%, 5.80% | **6.20%** (Aave holds 92% of effective weight) |
| STRUCTURED | Ethena sUSDe ($5.2B, conf=85) | 7.10% | **7.10%** (single source) |
| FUNDING | Hyperliquid ETH perp ($1.8B OI, conf=70) | 12.40% | **12.40%** (single source) |
| STAKING | ETH staking ($35B, conf=98) | 3.20% | **3.20%** (single source) |

**Level 2 — Inter-class weighted sum:**

```
ISFR = 0.60 × 6.20% + 0.25 × 7.10% + 0.10 × 12.40% + 0.05 × 3.20%
     = 3.720%        + 1.775%        + 1.240%         + 0.160%
     = 6.895%
     ≈ 690 basis points
```

**Published values this epoch:**

| Index | Value |
|-------|-------|
| ISFR | 6.90% (690 bps) |
| ISFR.LENDING | 6.20% (620 bps) |
| ISFR.STRUCTURED | 7.10% (710 bps) |
| ISFR.FUNDING | 12.40% (1240 bps) |
| ISFR.STAKING | 3.20% (320 bps) |

Note: The elevated FUNDING rate (12.40%) contributes just 124 bps to the composite — its 10% class weight limits its influence. Under a flat equal-weight average of all four class rates, ISFR would be 7.23% instead of 6.90% — the speculative signal pulls the rate 33 bps higher. Two-level aggregation keeps the composite anchored to lending fundamentals where hedging demand concentrates.

### 3.6 Source Registry — On-Chain Governance

The Source Registry stores each source as an on-chain struct:

```solidity
struct ISFRSource {
    bytes32 id;              // unique identifier
    SourceClass class;       // LENDING | STRUCTURED | FUNDING | STAKING
    address adapter;         // oracle adapter contract that fetches the rate
    uint256 tvl;             // total value locked / open interest (updated per epoch)
    uint8 confidence;        // 0-100, governs effective weight
    uint32 maxStaleness;     // maximum age in seconds before exclusion
    bool active;             // emergency suspension via guardian multisig
}

enum SourceClass { LENDING, STRUCTURED, FUNDING, STAKING }
```

New sources enter with low confidence (typically 30 for 30 days). Governance adjusts confidence upward after the observation period demonstrates stability. Emergency suspension via guardian multisig handles protocol exploits or oracle failures.

For integrators: adding a new source to ISFR requires registering one struct and surviving a 30-day probation. No code changes, no formula updates, no redeployment.

### 3.7 Published Values and Sub-Indices

Every computation round produces five values, all available via the oracle precompile:

- **ISFR** — The primary composite rate (canonical benchmark, published in block header)
- **ISFR.LENDING** — Lending class rate
- **ISFR.STRUCTURED** — Structured yield class rate
- **ISFR.FUNDING** — Funding class rate
- **ISFR.STAKING** — Staking class rate

Sub-indices are byproducts of computing ISFR — zero marginal cost. A protocol hedging Aave supply rate risk references ISFR.LENDING directly. A delta-neutral vault monitors ISFR.STRUCTURED. The composite ISFR serves as the canonical settlement rate for yield perpetuals. This granularity has no equivalent in IPOR (single index), Pendle (per-asset), or traditional benchmarks (SOFR publishes one rate plus percentiles).

### 3.8 Architectural Properties

**Extensible.** Adding 10 new LENDING sources improves precision without changing the formula. Adding a new class requires only one new weight parameter.

**Robust.** Two independent layers of median-based defense: intra-class TVL-weighted median + inter-class weight bounds. A compromised source moves at most one class rate, bounded by its class weight in the composite.

**Manipulation-resistant.** An attacker who spikes Compound V3's rate to 50% barely moves the LENDING median — Aave's $23.5B in TVL dominates. Even if the attacker shifted the LENDING class rate by 100 bps, the composite moves only 60 bps (0.60 × 100). Two layers of defense make manipulation exponentially more expensive than attacking a flat average.

**SOFR-parallel.** SOFR aggregates three repo source types (tri-party, FICC-cleared bilateral, GCF) before computing a volume-weighted median. ISFR mirrors this — pool by structural type, then aggregate — making it legible to institutional counterparties already familiar with SOFR methodology.

**IOSCO-alignable.** Time-locked governance, 30-day probation periods, guardian emergency procedures, and transparent on-chain methodology follow the IOSCO Principles for Financial Benchmarks (2013), which establish the global standard for benchmark credibility.

---

## 4. The Hybrid Rate — Oracle Meets Market Discovery

Most benchmark rates derive from a single source of truth: SOFR from repo transactions, LIBOR from bank submissions. ISFR has two — and the interaction between them is a core innovation.

### 4.1 ISFR_oracle — The External Anchor

The oracle layer (Section 3) measures external DeFi rates: validators scrape external protocol sources, compute the two-level aggregation, and produce ISFR_oracle via stake-weighted median of their votes. This is the "anchor" — an objective measurement of what DeFi rates actually are, observed from outside.

### 4.2 ISFR_market — Endogenous Price Discovery

Korai's clearing engine solves a Quadratic Programming (QP) problem each clearing round — a mathematical optimization that matches all buy and sell orders for rate exposure at the price that clears the market, minimizing total cost subject to constraints (margin, risk limits, position bounds). The optimization naturally produces a market-clearing rate: the price at which participants are collectively willing to transfer rate exposure.

The clearing engine does not set this rate. It emerges from the mathematics of optimal allocation — the rate at which aggregate supply of rate exposure meets aggregate demand.

### 4.3 The Mark Price Formula

The block header publishes both values. The canonical ISFR combines them:

```
ISFR = ISFR_oracle + EMA(ISFR_market - ISFR_oracle)
```

At launch, with thin clearing liquidity, the EMA contribution is negligible and ISFR ≈ ISFR_oracle. As the yield perp market deepens, ISFR_market becomes progressively more informative, and the benchmark naturally transitions toward endogenous price discovery.

### 4.4 Convergence — No Binary Cutover

The transition from oracle-driven to market-driven is continuous, not a discrete switch:

| Stage | ISFR Behavior | Analogy |
|-------|---------------|---------|
| **Launch** | ISFR ≈ ISFR_oracle (pure external observation) | Weather forecast based entirely on satellite data |
| **Early growth** | ISFR_market contributes small adjustments | Satellite data + ground sensors |
| **Maturity** | ISFR_market provides the leading signal; ISFR_oracle anchors against manipulation | Ground sensors primary, satellite as validation |

The rate and the market co-evolve — each making the other more credible. This is unique among benchmark designs: no traditional benchmark (SOFR, €STR, SONIA) incorporates endogenous market discovery, and no DeFi protocol (IPOR, Pendle) embeds external rate observation into validator consensus.

---

## 5. How Validators Compute Rates Without Trusting Anyone

Existing DeFi oracles — Chainlink, Pyth, API3 — rely on a separate operator layer outside the chain's own consensus. If those operators are compromised, the data is compromised. ISFR takes a fundamentally different approach: rate computation is embedded directly into validator consensus.

### 5.1 The Computation Pipeline

**Update cadence: every 25 blocks (10 seconds).** DeFi rates change over hours, not milliseconds. 10-second granularity provides sufficient resolution for yield perp mark-to-market and clearing while reducing validator workload 25× versus per-block computation. This cadence creates 8,640 rate observations per day — orders of magnitude denser than SOFR (1 daily publication) or IPOR (~96 per day at 15-minute intervals).

Each validator independently:
1. Pulls latest source data via RPC from each registered source
2. Performs health checks (latency, deviation, availability)
3. Computes Level 1: TVL-weighted median per source class
4. Computes Level 2: weighted sum of class rates → ISFR
5. Submits an OracleVote with all five values to the chain

The chain finalizes via stake-weighted median across all validator votes. An attacker must compromise both the source-level aggregation and the validator-level consensus simultaneously — two independent security layers.

### 5.2 One Opcode, One Rate: The Oracle Precompile

ISFR is published via a dedicated EVM precompile at address `0xA01` — a consensus-level primitive with fixed gas cost, as efficient as reading the block number.

```solidity
interface ISFROracle {
    /// Returns ISFR + 4 sub-indices + confidence
    function currentRate() external view returns (
        uint256 isfr,           // composite rate in basis points
        uint256 lendingRate,    // ISFR.LENDING
        uint256 structuredRate, // ISFR.STRUCTURED
        uint256 fundingRate,    // ISFR.FUNDING
        uint256 stakingRate,    // ISFR.STAKING
        uint64 timestamp,
        uint8 confidence        // 0-100, validator agreement metric
    );

    /// Returns ISFR at a specific block epoch
    function rateAt(uint64 epochBlock) external view returns (
        uint256 isfr, uint64 timestamp
    );

    /// Returns historical ISFR values (up to 30 days on-chain)
    function history(uint64 fromEpoch, uint64 toEpoch) external view returns (
        uint256[] memory rates, uint64[] memory timestamps
    );
}
```

The **confidence score** measures validator agreement: the percentage of total stake weight that submitted votes within 10 bps of the finalized median (0–100). Consuming contracts can gate actions on confidence — a yield perp contract might reject new positions when confidence drops below 70, ensuring that only high-quality rate observations trigger economic actions.

### 5.3 Liveness and Degradation States

The oracle operates in four states, with explicit degradation signaling:

| State | Condition | ISFR Behavior | Yield Perp Behavior |
|-------|-----------|---------------|---------------------|
| **Live** | ≥90% validator participation | Computed normally | Normal operations |
| **Degraded** | 67–90% participation | Computed from available votes (median still robust) | Normal with warning flag |
| **Stale** | 50–67% participation | Last valid ISFR frozen | No new positions; existing positions at frozen rate |
| **Halted** | <50% participation OR no update for 5 min | No ISFR published | Liquidations paused; new positions blocked |

For traders: ISFR either publishes a trustworthy rate or explicitly signals degradation. There is no silent failure mode — a critical difference from oracle networks where stale data may be consumed without warning.

### 5.4 Data Source Resilience

Validators perform health checks independently during each computation round — no separate watcher service required:

| Health Metric | Threshold | Action |
|---------------|-----------|--------|
| Latency | >30 seconds since last update | Mark source degraded; exclude |
| Deviation | >3σ from peer sources in same class | Exclude from intra-class computation |
| Availability | Source RPC unreachable for >60 seconds | Exclude; reweight remaining class sources |

**Source failover:** If all sources in a class go offline, that class's weight is redistributed proportionally to remaining healthy classes. If FUNDING's sole source goes offline, its 10% is redistributed: LENDING gets +6.67%, STRUCTURED +2.78%, STAKING +0.56%. ISFR requires at least 3 healthy sources across classes for Live status; below 2, the oracle enters Stale state.

A rate as difficult to corrupt as consensus itself — and as easy to consume as reading a storage slot.

---

## 6. ISFR as Prediction Target — The Epistemic Engine

Prediction markets produce knowledge only when participants have skin in the game and outcomes resolve frequently. Most on-chain prediction targets — governance votes, token prices at future dates, protocol metrics — resolve daily, weekly, or not at all. ISFR creates a falsifiable prediction target every 10 seconds: 8,640 scoring opportunities per agent per day.

### 6.1 The Prediction Loop

The cycle repeats every 10 seconds:

1. **Predict.** Agents register predictions for the next ISFR value: "ISFR will be X basis points at epoch N+1."
2. **Commit.** Predictions are committed on-chain before the outcome via hash commitment: `hash(predictedValue || salt)`. The hash prevents front-running — no agent can see others' predictions before committing its own. The salt is revealed after the epoch boundary.
3. **Observe.** At epoch N+1, validators publish the actual ISFR.
4. **Score.** The residual is computed using the Continuous Ranked Probability Score (CRPS).
5. **Calibrate.** Residuals feed back into agent models, improving future predictions.

### 6.2 CRPS — Why Honest Prediction Is the Only Rational Strategy

ISFR uses the Continuous Ranked Probability Score (CRPS), a strictly proper scoring rule for continuous outcomes first rigorously characterized by Gneiting and Raftery (2007) in their foundational work on scoring rules and estimation. The CRPS framework has been validated for financial applications by Crisóstomo (2021) in the Journal of Futures Markets and applied to density forecasting by Loaiza-Maya et al. (2021) in the Journal of Applied Econometrics.

**V1 — Point predictions.** Agents submit a single predicted value. CRPS reduces to Mean Absolute Error:

```
CRPS(agent_i) = |predicted_i - actual|
```

Lower is better. **Strict propriety** is a mathematical property (not an assumption): the unique optimal strategy is truthful reporting of one's best estimate. Hedging, sandbagging, and strategic misreporting all produce worse expected scores under strict propriety. This makes ISFR predictions incentive-compatible by construction.

**V2 — Distributional predictions.** Agents submit a full cumulative distribution function (CDF) over possible ISFR values:

```
CRPS(F, y) = ∫ [F(x) - 1(x ≥ y)]² dx
```

This rewards agents who accurately quantify their uncertainty — an agent who says "688 bps ± 5 bps at 90% confidence" earns a better score than "688 bps ± 50 bps" even if both nail the point estimate. Distributional predictions are also strictly proper.

**Worked Example:**

An agent predicts ISFR = 610 bps at epoch N+1. The actual ISFR published by validators is 595 bps.

CRPS = |610 − 595| = 15 basis points

This residual enters the agent's rolling 30-day exponential moving average. If the agent's previous EMA was 12 bps, the new EMA shifts slightly upward. Over time, an agent that consistently predicts within 10 bps outranks one averaging 20 bps — even if the higher-error agent occasionally nails an exact prediction. The scoring rule rewards consistent calibration, not lucky guesses.

### 6.3 Epistemic Reputation — Better Predictions Earn Better Economics

Each agent accumulates a rolling CRPS score per prediction domain:

```
epistemicScore(agent, "isfr") = EMA_30d(CRPS scores)
```

Agents are ranked into reputation tiers with concrete economic consequences:

| CRPS Percentile | Tier | Economic Benefit |
|-----------------|------|------------------|
| Top 10% | **Oracle** | 2× InsightStore query quota; priority clearing; γ discount of 0.5× |
| 10–30% | **Calibrated** | 1.5× InsightStore query quota; γ discount of 0.75× |
| 30–70% | **Standard** | Base access, base γ |
| 70–100% | **Uncalibrated** | 0.5× InsightStore query quota; γ premium of 1.25× |

The γ discount is where reputation becomes economically material. In the clearing engine, γ (risk aversion) determines effective spread and margin requirements. Oracle-tier agents receive γ_effective = γ_declared × 0.5 — half the friction cost of an Uncalibrated agent.

This creates a direct flywheel: accurate predictions → higher reputation → lower trading costs → more profitable strategies → more predictions → better accuracy. Epistemic quality is rewarded with financial advantage.

### 6.4 Collective Calibration — Intelligence as Network Effect

As more agents make ISFR predictions, collective prediction accuracy improves. The expected scaling relationship follows from Central Limit Theorem dynamics:

```
collective_accuracy(t) ∝ 1 - 1/√(N × t)
```

At 1,000 agents, a new agent reaches baseline accuracy approximately 31.6× faster than one calibrating alone — it can learn from the network's collective prediction history. Key assumptions: approximate independence across agent models and equally informative observations. This is a heuristic model; empirical validation on actual agent populations is needed.

The counter-intuitive property: in most markets, information asymmetry is profitable. In ISFR's prediction layer, it is self-defeating — the CRPS scoring rule makes truthful reporting the dominant strategy regardless of what other agents do. The system cannot be gamed.

---

## 7. Knowledge Production — From Rate Data to Compounding Intelligence

Most market data is consumed once and discarded. ISFR is designed to produce compounding knowledge — every 10-second update enriches the network's collective intelligence through structured knowledge entries.

### 7.1 Structured Knowledge Entries (Engrams)

Every ISFR update produces a structured knowledge entry (an Engram with `kind: OracleUpdate`) that enters the InsightStore — Korai's on-chain knowledge repository:

```
ISFRInsight {
    kind:          "OracleUpdate"
    domain:        "yield_rates"
    value:         isfr_value_bps           // e.g., 690
    components:    {
        lending:    620,
        structured: 710,
        funding:    1240,
        staking:    320
    }
    deviation:     delta_from_previous      // rate of change signal
    confidence:    validator_agreement       // consensus quality metric
    timestamp:     block_timestamp
    decay:         HalfLife(7 days)          // recent data weighted more heavily
}
```

These entries are queryable via a dedicated similarity-search precompile (~170μs at 100K vectors). The 7-day half-life implements a deliberate design philosophy: data without renewal loses value. This is the same demurrage principle applied to KORAI tokens and agent knowledge entries across the platform — freshness is enforced at every layer.

### 7.2 The Autocatalytic Loop

ISFR data is not consumed in isolation. Agents publish derived insights that compound the value of raw rate data:

| Insight Type | Example | Knowledge Entry Type |
|-------------|---------|---------------------|
| Mean reversion pattern | "When ISFR diverges >50bps from ISFR.LENDING, mean reversion occurs within 48h in 73% of observations" | Heuristic |
| Volatility regime shift | "ISFR 7-day stdev exceeding 15bps precedes >100bps rate moves within 14 days" | CausalLink |
| Source correlation breakdown | "ISFR.STRUCTURED decoupling from ISFR.LENDING by >200bps — potential basis collapse" | Warning |
| Cross-domain transfer | "High ISFR volatility correlates with increased smart contract deployment activity (r=0.42)" | Insight |

The result is autocatalytic: ISFR updates produce knowledge → Knowledge improves predictions → Predictions attract agents → Agents produce knowledge → Each revolution of the loop accelerates the next.

---

## 8. Self-Improving Rate Infrastructure

ISFR self-improves through mechanisms that calibrate automatically based on performance data. V1 scaffolds the architecture; V2/V3 activate self-calibration.

### 8.1 Self-Calibrating Source Confidence (V2)

V1 uses governance-assigned confidence scores (0–100). V2 replaces these with leave-one-out Mean Squared Prediction Error (MSPE):

```
ISFR_loo[s] = aggregate(all sources except s)
residual[s][t] = source_rate[s][t] - ISFR_loo[s][t]
MSPE[s][t] = λ × MSPE[s][t-1] + (1-λ) × residual[s][t]²
confidence[s][t] = 1 / (1 + MSPE[s][t] / MSPE_floor)
```

Leave-one-out breaks circularity: a source cannot inflate its own confidence by dominating the index. Sources that consistently agree with the consensus earn higher weight organically. New sources enter with governance-assigned probation confidence, then transition to MSPE-computed after warm-up.

### 8.2 Adaptive Class Weights — Bates-Granger Optimal Combination (V2)

V1's fixed weights (60/25/10/5) are replaced with weights inversely proportional to each class's prediction error, bounded by governance rails:

```
MSPE_class[k][t] = λ × MSPE_class[k][t-1] + (1-λ) × (class_rate[k][t-1] - ISFR_realized[t])²
w_k_raw[t] = 1 / MSPE_class[k][t]
w_k[t] = clamp(w_k_raw[t] / Σ w_j_raw[t], w_min[k], w_max[k])
```

Governance sets bounds per class (e.g., LENDING: 30–80%, FUNDING: 0–20%). The system finds optimal weights within those bounds from empirical data, following the forecast combination framework of Bates and Granger (1969). V1 weights become initial conditions, not permanent parameters.

### 8.3 Cost-Stratified Trim Fractions (V2)

V2 replaces the flat TVL-weighted median with a trimmed mean per class, where trim fraction reflects the economic cost of manipulating that source type:

| Class | Trim α | Rationale |
|-------|--------|-----------|
| LENDING | 0.15 | Manipulation requires actual capital deployment (borrow all available liquidity) |
| STRUCTURED | 0.20 | Delta-neutral strategies can be unwound rapidly |
| FUNDING | 0.30 | Perpetual positioning can be cheaply manipulated via leverage |
| STAKING | 0.05 | Requires attacking Ethereum consensus |

A flat trim fraction ignores economic reality — STAKING rates (requiring Ethereum consensus attack to manipulate) get the same defense as FUNDING rates (requiring only a leveraged perp position). Per-class trim allocates defense proportional to actual manipulation cost.

### 8.4 Kalman Filter Smoothing (V2)

V2 replaces the simple EMA in the hybrid mark price formula with a Kalman filter that adapts to signal quality:

```rust
struct ISFRKalmanState {
    x_hat: PU18,    // filtered ISFR estimate
    p: PU18,        // error covariance (uncertainty)
    r1_ema: PU18,   // oracle measurement noise
    r2_ema: PU18,   // clearing measurement noise
    q: PU18,        // process noise (governance parameter)
}
```

An EMA treats all deviations as signal noise. A Kalman filter distinguishes measurement noise (source disagreement, stale data) from process noise (genuine rate movement). When oracle noise is high (sources disagree), the filter automatically upweights the clearing signal, and vice versa.

### 8.5 Nelson-Siegel Yield Curve (V2)

V2 publishes not discrete rate points but 4 parameters characterizing the entire yield curve:

```
y(τ) = β₁ + β₂[(1-e^(-λτ))/(λτ)] + β₃[(1-e^(-λτ))/(λτ) - e^(-λτ)]
```

Where β₁ = long-run level, β₂ = slope, β₃ = curvature, λ = decay speed. DeFi currently has no term structure — rates are quoted at a single tenor. Nelson-Siegel transforms discrete rate observations into a continuous curve with just 4 parameters. Any consumer can reconstruct ISFR at any tenor τ from these 4 numbers.

Extended to quantile curves (p05/p50/p95) using the Kalman filter's error covariance, the block header publishes an on-chain term structure of rate volatility — the spread between p05 and p95 is immediately usable for options pricing without external volatility oracles.

### 8.6 V1→V2 Evolution Summary

| Mechanism | V1 (Launch) | V2 (Self-Calibrating) |
|-----------|-------------|----------------------|
| Source confidence | Governance-assigned (0–100) | MSPE leave-one-out (automatic) |
| Class weights | Fixed (60/25/10/5) | Bates-Granger optimal with governance rails |
| Smoothing | EMA | Kalman filter (separates noise types) |
| Intra-class aggregation | TVL-weighted median | Cost-stratified trimmed mean |
| Yield curve | Discrete rate points | Nelson-Siegel 4-parameter continuous curve |

---

## 9. Competitive Comparison

| Dimension | ISFR (Korai) | IPOR Index | Pendle | SOFR (NY Fed) |
|-----------|--------------|------------|--------|---------------|
| Architecture | Two-level (4 classes, 5 sub-indices) | Flat mean (3 sources, 1 index) | Per-asset AMM (no benchmark) | Volume-weighted median (3 repo types) |
| Manipulation tolerance | 49% corrupted weight (two independent layers) | None (flat average) | N/A (per-asset AMM) | N/A (centralized, trusted) |
| Update frequency | Every 10 seconds (8,640/day) | ~15 minutes (~96/day) | Per trade | Daily (1/day) |
| On-chain native | Precompile, fixed gas | Contract, variable gas | AMM | No (NY Fed publication) |
| Hybrid oracle + market | Yes | No | No | No |
| Self-calibrating | V2: MSPE, Bates-Granger, Kalman | Fixed methodology | N/A | Fixed methodology |
| Knowledge production | InsightStore entries, prediction scoring | Data only | N/A | Data only |
| Term structure | V2: Nelson-Siegel yield curve | No | No | Percentiles only |
| TVL / Scale | Launch (2026) | ~$10–20M TVL | ~$5.7B avg TVL (2025) | ~$1T/day underlying |
| Funding | Pre-Series A | $5.55M (Early VC, 2022) | $15.1M+ | N/A (public institution) |

The structural differentiators are architectural, not incremental:

1. **Two-level aggregation with sub-indices.** ISFR preserves the structure of DeFi's yield surface. Hedgers reference the specific sub-index matching their exposure — ISFR.LENDING for lending risk, ISFR.FUNDING for speculative sentiment. No other protocol offers this.

2. **Hybrid oracle + clearing.** The only benchmark combining external observation with endogenous market discovery. Useful from day one (oracle-driven), more useful as the market grows (market-driven).

3. **Knowledge production as a first-class feature.** The first benchmark designed to make its consumers smarter: 8,640 scored predictions per agent per day, feeding a continuous calibration loop that improves the benchmark itself.

4. **Validator-embedded computation.** ISFR does not depend on a separate operator set. The rate is as secure as the chain's own consensus.

---

## 10. The Path to Credibility

Every new benchmark faces the same question: how does it go from zero to trusted?

### Phase 1 — Curated Aggregation (Launch)

ISFR launches with curated, highly-liquid V1 sources and governance-assigned weights. The focus is methodological transparency: every validator vote is on-chain, every computation is reproducible, every source is publicly verifiable.

Korai's own agents — running on the Roko cognitive runtime — are the first consumers, providing anchor demand for yield perps settled against ISFR. This is not circular: the agents produce genuine economic activity (trading, hedging, arbitrage), and ISFR measures real external rates. The agents demonstrate that the rate behaves as expected under live conditions.

### Phase 2 — Track Record (Months 3–6)

Uninterrupted publication builds the historical record institutions require. Source expansion begins (30-day probation per new source). V2 innovations — self-calibrating confidence, adaptive weights, Kalman smoothing — demonstrably improve stability and manipulation resistance.

Key milestones: 90+ days of uninterrupted publication, 3+ sources per class, external data provider validation of methodology.

### Phase 3 — Reflexive Loop (Months 6–12+)

As ISFR-settled instruments grow, the rate bootstraps its own credibility through the activity it enables. By the time external institutional participants evaluate ISFR, they find: a live rate with months of history, IOSCO-aligned methodology, an active derivatives market already settled against it, and self-calibrating mechanisms that demonstrably improve over time.

### Beyond Phase 3 — Source Expansion

- **Cross-chain (V3):** Solana lending rates (Kamino, MarginFi), L2 rates (Aave on Arbitrum/Base/Optimism). Cross-chain sources enter through the same 30-day probation and MSPE confidence calibration.
- **Traditional rate bridges (V4):** SOFR via attested data feeds, UST-3M (US Treasury 3-month). These create direct on-chain reference points for basis trades between DeFi yields and TradFi rates — enabling the first natively on-chain DeFi-to-TradFi yield spread instruments.

### ISFR Credibility Timeline

| Phase | Timeline | Key Activities |
|-------|----------|----------------|
| Phase 1: Curated | Q3 2026 | Launch with 4 V1 sources; governance-assigned weights; Korai agents as anchor consumers |
| Phase 2: Track Record | Q4 2026 | Uninterrupted publication; source expansion (30-day probation); V2 self-calibration activates |
| Phase 3: Reflexive Loop | Q1–Q2 2027 | ISFR-settled derivatives grow; external institutional evaluation; IOSCO alignment review |
| V3: Cross-chain | Q3 2027+ | Solana, L2 sources via same probation framework |
| V4: TradFi bridges | 2028+ | SOFR on-chain, UST-3M, DeFi-to-TradFi basis instruments |

---

## 11. ISFR Within the Korai Ecosystem

ISFR is the first index in Korai's broader oracle framework. The same infrastructure — validator computation, precompile publication, prediction scoring — extends to additional asset classes and domains as the Nunchi Reference Index Suite (NRIS).

But ISFR is not merely an index. It is the economic primitive that activates Korai's entire value stack:

- **Yield perpetuals** settle against ISFR, creating the first perpetual interest rate hedge in DeFi.
- **The prediction loop** uses ISFR as its canonical target, creating the densest scoring feedback loop in any on-chain prediction system.
- **The InsightStore** ingests ISFR observations as structured knowledge, enabling agent-to-agent intelligence transfer.
- **Epistemic reputation** is computed from ISFR prediction accuracy, creating an on-chain meritocracy of forecasting ability.
- **The clearing engine** uses ISFR as its mark price, with reputation-modulated γ creating direct economic incentives for prediction quality.

Every component reinforces every other component. ISFR is the keystone.

---

## 12. References

1. Bank for International Settlements. "OTC derivatives statistics at end-June 2025." BIS Statistical Bulletin, December 2025. https://www.bis.org/statistics/derstats.htm
2. Bank for International Settlements. "OTC interest rate derivatives turnover in April 2025." BIS Triennial Central Bank Survey, 2025. https://www.bis.org/statistics/rpfx25_ir.pdf
3. International Swaps and Derivatives Association. "Key Trends in the Size and Composition of OTC Derivatives Markets in the First Half of 2025." ISDA, 2025. https://www.isda.org/a/oSdgE/Key-trends-in-the-size-and-composition-of-OTC-derivatives-markets-in-the-first-half-of-2025.pdf
4. International Swaps and Derivatives Association. "Interest Rate Derivatives Trading in the US, EU and UK: Growth, Structural Shifts, and the Rise of OIS." ISDA Research, June 2025. https://www.isda.org/a/4ojgE/Interest-Rate-Derivatives-Trading-in-the-US-EU-and-UK-Growth-Structural-Shifts-and-the-Rise-of-OIS.pdf
5. Federal Reserve Bank of New York, Alternative Reference Rates Committee. "SOFR Transition." https://www.newyorkfed.org/arrc/sofr-transition
6. Gneiting, T. & Raftery, A.E. "Strictly proper scoring rules, prediction, and estimation." Journal of the American Statistical Association, 102(477), 2007.
7. Crisóstomo, R. "Estimating Real-World Probabilities: A Forward-Looking Behavioral Framework." Journal of Futures Markets, 41(11), 2021.
8. Loaiza-Maya, R., Martin, G.M. & Frazier, D.T. "Focused Bayesian prediction." Journal of Applied Econometrics, 36(5), 2021.
9. IOSCO. "Principles for Financial Benchmarks." International Organization of Securities Commissions, FR07/13, July 2013.
10. Bates, J.M. & Granger, C.W.J. "The Combination of Forecasts." Operational Research Quarterly, 20(4), 1969.
11. Kim, H. & Park, A. "Designing Funding Rates for Perpetual Futures." arXiv:2506.08573, 2025.
12. "Optimal Benchmark Design under Costly Manipulation." arXiv:2506.22142, 2025.
13. DeFiLlama. Lending protocols dashboard. Accessed April 2026. https://defillama.com/protocols/Lending
14. FalconX. "Pendle: One Venue, All of Fixed Income." FalconX Research, 2025. https://www.falconx.io/newsroom/pendle-one-venue-all-of-fixed-income
15. Kanerva, P. "Hyperdimensional computing: An introduction to computing in distributed representation with high-dimensional random vectors." Cognitive Computation 1(2), 2009.

---

*Architecture alone does not create a benchmark. Credibility does. ISFR is designed to earn it.*
