# ISFR: The Internet Secured Funding Rate

## A Complete Technical Reference

---

## 1. What ISFR Is

The Internet Secured Funding Rate (ISFR) is a composite benchmark index that measures the cost of secured funding across decentralized finance. It aggregates yield signals from the largest DeFi lending, staking, structured yield, and perpetual funding rate protocols into a single rate, published on-chain every 10 seconds.

ISFR is to DeFi what SOFR (Secured Overnight Financing Rate) is to traditional finance: the reference rate that financial instruments -- interest rate swaps, perpetual futures, floating-rate notes -- settle against. Without a benchmark rate, none of these instruments can be priced, hedged, or settled. Traditional finance anchors approximately $668 trillion in interest rate derivative notional on benchmark rates. DeFi, which holds approximately $49.5 billion in lending TVL (as of April 2026), has no equivalent. On-chain interest rate derivative TVL sits under $100 million. That six-order-of-magnitude gap is not a marketing failure. It is the absence of a foundational primitive.

ISFR is designed to be published by validators on a purpose-built blockchain called Korai. It is computed at the consensus layer -- not by an external oracle operator, not by a multisig, not by a DAO vote. Each validator independently reads source rates, computes the aggregation, and submits its result. The chain then computes a stake-weighted median across all validator submissions. The rate is accessible to any smart contract via a precompile at address `0xA01` with fixed gas cost, as efficient as reading the block number.

### What ISFR Is Not

ISFR is not a lending rate. It does not tell you what rate you will receive on any specific protocol. It is a benchmark -- a composite measure of the broad market that serves as a reference for derivative pricing, hedging, and settlement.

ISFR is not an oracle feed. Oracle feeds push external data onto a chain. ISFR is computed by the chain's own validators from data they independently observe.

ISFR is not a governance-managed parameter. No DAO vote sets the rate. The methodology is fixed in the protocol specification. Governance can adjust source weights (within bounds) and add or remove sources in V2, but cannot override the computed value.

---

## 2. The Problem ISFR Solves

### The Benchmark Dependency Chain

In traditional finance, almost nothing works without a benchmark interest rate. The causal chain is explicit: **benchmark rate -> derivative pricing -> hedging -> risk management -> capital efficiency -> market depth -> lower borrowing costs.** Remove the benchmark and the entire chain collapses. The LIBOR transition proved it -- when confidence in the benchmark wavered, trillions in notional faced repricing uncertainty.

SOFR is published daily by the Federal Reserve Bank of New York. It measures the cost of borrowing cash overnight, collateralized by U.S. Treasury securities, computed from roughly $2 trillion in daily overnight repo volume. SOFR replaced LIBOR after the 2012 rate-rigging scandal, in which traders at multiple global banks manipulated LIBOR submissions. The lesson: a benchmark rate that depends on voluntary submissions from interested parties will be gamed. SOFR was designed to be manipulation-resistant by construction.

The instruments that depend on SOFR are staggering:

| Instrument class | Notional outstanding |
|-----------------|---------------------|
| Interest rate swaps | ~$400T |
| Futures and options (CME) | ~$120T |
| Floating-rate notes | ~$48T |
| Adjustable-rate mortgages | ~$2.5T |
| Corporate floating-rate debt | ~$1.8T |
| **Total** | **~$570T+** |

### DeFi's Structural Gap

DeFi has no equivalent to SOFR. The protocols that exist today demonstrate demand while failing to fill the gap:

| Protocol | TVL | What it offers | Why it falls short |
|----------|-----|---------------|-------------------|
| Pendle | ~$5.7B avg (2025) | Yield tokenization with fixed-maturity PT/YT tokens | Expiring instruments fragment liquidity across hundreds of pools. No benchmark rate. No leverage. Rollover costs 50+ bps per maturity cycle. |
| IPOR | ~$10-20M | Interest rate swaps with a proprietary index | Single-methodology flat-average index (3 sources). No two-level aggregation, no manipulation tolerance guarantees. Thin liquidity. |
| Spectra | ~$44M | Yield tokenization with Curve integration | Same expiration and fragmentation constraints as Pendle. |
| Voltz | Shut down Dec 2023 | IR swaps on concentrated liquidity AMM | Demonstrated that the AMM model does not work for rates. |

The gap is structural, not competitive. A benchmark rate requires properties that no individual protocol rate possesses:

1. **Multi-source aggregation** -- a single source is a quote, not a benchmark.
2. **Manipulation resistance** -- individual protocol rates can be moved by flash loans, governance attacks, or liquidity manipulation.
3. **Independent computation** -- if the entity publishing the rate is also a market participant, conflicts of interest are structural.
4. **Continuous availability** -- a rate that goes offline during market stress is not a benchmark.
5. **Standardized methodology** -- different protocols compute rates differently, making them incomparable without a standardized aggregation layer.

Every dollar of the ~$50 billion in DeFi lending TVL carries unhedged variable rate exposure because there is nothing to hedge against.

---

## 3. The Four Source Protocols

ISFR V1 aggregates rates from four DeFi yield sources, each capturing a distinct yield generation mechanism.

### Source Taxonomy (V4 Specification)

The V4 specification organizes sources into four mutually exclusive, collectively exhaustive classes:

| # | Source | Class | What It Measures | Update Frequency | Why Included |
|---|--------|-------|-----------------|-----------------|--------------|
| 1 | **Aave V3** (Ethereum mainnet) | LENDING | USDC supply APY | Per-block (~12s) | Largest lending protocol by TVL (~$23.5B). Broad market signal for collateralized lending yield. |
| 2 | **Compound V3** (Ethereum mainnet) | LENDING | USDC supply APY | Per-block (~12s) | Second-largest lending protocol (~$2.1B). Different utilization curve from Aave provides diversification. |
| 3 | **Ethena** (sUSDe) | STRUCTURED | 7-day rolling yield | As funding rates settle (~8h intervals) | Represents delta-neutral funding rate exposure. Captures yield from perpetual funding, structurally distinct from lending. |
| 4 | **ETH Beacon Chain** | STAKING | Consensus rewards + MEV tips, annualized | Per-epoch (~6.4 min, ~225 epochs/day) | The most decentralized yield source in crypto. ~$115B+ staked. Resistant to single-entity manipulation. The "risk-free rate" of crypto. |

Note: there is an evolution between earlier design documents and the V4 specification. The earlier research document listed Hyperliquid ETH perpetual funding rate as the fourth source (in place of ETH Beacon Chain staking yield). The V4 specification moved Hyperliquid into a separate FUNDING class (see Section 7 on sub-indices). The table above reflects the V4 canonical source set.

### Why These Four

Each source captures a distinct yield generation mechanism:

- **Aave and Compound** measure the cost of borrowing USDC against crypto collateral -- the closest DeFi analog to overnight secured lending rates in TradFi.
- **Ethena (sUSDe)** measures the funding rate on perpetual futures positions via a delta-neutral strategy. This captures speculative sentiment and leverage demand that pure lending rates miss.
- **ETH staking yield** measures the base cost of securing the Ethereum network. It is the "risk-free rate" of crypto, analogous to the Treasury yield in TradFi.

Together, they span the primary yield generation mechanisms in DeFi: lending, structured yield (funding rates), and staking. No single-venue shock can dominate the composite.

### V2 Candidate Sources

| Candidate | What it measures | Rationale |
|-----------|-----------------|-----------|
| Morpho (Ethereum) | Optimized lending yield (USDC) | Growing TVL (~$10B+), different liquidation mechanism |
| MakerDAO DSR | Dai Savings Rate | Governance-set rate, represents MKR holders' view of cost of capital |
| Spark (Aave V3 fork) | USDC supply APY | Additional lending data point (~$7.9B TVL) |
| Pendle PT yields | Fixed-maturity implied yield | Captures yield curve expectations |
| Hyperliquid ETH perp funding | Speculative funding rate | Real signal on speculative positioning |
| dYdX / GMX funding rates | Perpetual funding rates | Additional funding rate data points |
| Lido stETH / rETH APR | Liquid staking yield | More granular staking yield signals |

V2 targets a minimum of 7 sources across classes, increasing manipulation resistance. With 7 equally-weighted sources, an attacker must corrupt 4 simultaneously to move the median arbitrarily.

---

## 4. Computation Methodology

ISFR uses a two-level aggregation architecture with weighted median at both layers. This dual-median design provides independent Byzantine fault tolerance at the source level and at the validator level.

### 4.1 Level 1 -- Intra-Class Aggregation (TVL-Weighted Median)

Within each source class (LENDING, STRUCTURED, FUNDING, STAKING), sources are aggregated into a single class rate using a TVL-weighted median with confidence modulation:

```
effective_weight(source) = tvl(source) * (confidence(source) / 100)
```

Confidence modulates a source's contribution: a new source entering with confidence = 30 contributes only 30% of its TVL-proportional weight, enabling smooth phase-in without disrupting the existing rate. Confidence scores are governance-assigned during a 30-day probation period in V1, then transition to automatic calibration via leave-one-out MSPE in V2.

The computation:
1. Sort source rates ascending within the class.
2. Accumulate effective weights.
3. Return the value where cumulative weight reaches 50%.

The TVL-weighted median tolerates up to 49% corrupted weight within each class.

### 4.2 Level 2 -- Inter-Class Aggregation (Weighted Sum)

The final ISFR is a deterministic weighted sum of the four class rates:

```
ISFR = 0.60 * LENDING + 0.25 * STRUCTURED + 0.10 * FUNDING + 0.05 * STAKING
```

These weights reflect the relative importance of each yield mechanism for hedging:

| Class | Weight | Rationale |
|-------|--------|-----------|
| LENDING | 0.60 | Most analogous to SOFR (secured overnight funding). Deepest, most stable DeFi yield market. Primary hedging target for treasuries. |
| STRUCTURED | 0.25 | Delta-neutral yield captures funding with dampening. Structurally distinct from lending. |
| FUNDING | 0.10 | Real signal on speculative positioning. Explicitly downscaled due to volatility. |
| STAKING | 0.05 | Floor rate for the Ethereum economy. Very stable, analogous to overnight repo. |

The two-level design creates a natural firewall against volatility contamination. If the funding rate spikes to 200% during a speculative mania, it contributes at most `0.10 * 200% = 20` percentage points to the composite. The other three classes, holding 90% of the weight, anchor the rate near their stable levels.

### 4.3 Validator-Level Aggregation (Stake-Weighted Median)

After each validator independently computes the source-level and class-level aggregations, the chain computes a **stake-weighted median** across all validator submissions.

Each validator submits an `OracleVote`:

```rust
struct OracleVote {
    value_bps: u32,          // The validator's computed ISFR value in basis points
    block_height: u64,       // Block height this vote applies to
    signature: BlsSignature, // Validator's BLS signature over (value_bps, block_height)
    validator_index: u32,    // Validator index in the current committee
}
```

Formally: let V = {(v_i, s_i)} be the set of validator votes, where v_i is the submitted ISFR value and s_i is the validator's normalized stake weight (sum of all s_i = 1). Sort votes by value. The aggregate ISFR is the value v_j such that:

```
sum(s_i for i <= j) >= 0.50 AND sum(s_i for i < j) < 0.50
```

If the 0.50 boundary falls exactly between two votes, interpolate linearly.

### 4.4 Two-Layer Byzantine Tolerance

**Layer 1 (sources):** The source-level weighted median tolerates manipulation of up to floor(k/2) sources, where k is the number of sources. With 4 sources, an attacker must corrupt 2 to move the median arbitrarily. With 7 V2 sources, the threshold rises to 4.

**Layer 2 (validators):** The validator-level stake-weighted median tolerates up to 49% compromised stake.

**Combined:** To manipulate ISFR to an arbitrary value, an attacker must compromise 50%+ of source weight AND 50%+ of validator stake simultaneously. Either layer alone stops the attack.

### 4.5 Worked Example -- Full Two-Level Computation

Suppose the following source rates are observed at a given 10-second epoch:

**Level 1 -- Intra-class medians:**

| Class | Sources (TVL, Confidence) | Rates | TVL-Weighted Median |
|-------|---------------------------|-------|---------------------|
| LENDING | Aave V3 ($23.5B, conf=95), Compound V3 ($2.1B, conf=90) | 6.20%, 5.80% | **6.20%** (Aave holds 92% of effective weight) |
| STRUCTURED | Ethena sUSDe ($5.2B, conf=85) | 7.10% | **7.10%** (single source) |
| FUNDING | Hyperliquid ETH perp ($1.8B OI, conf=70) | 12.40% | **12.40%** (single source) |
| STAKING | ETH staking ($35B, conf=98) | 3.20% | **3.20%** (single source) |

**Level 2 -- Inter-class weighted sum:**

```
ISFR = 0.60 * 6.20% + 0.25 * 7.10% + 0.10 * 12.40% + 0.05 * 3.20%
     = 3.720%        + 1.775%        + 1.240%         + 0.160%
     = 6.895%
     ~ 690 basis points
```

### 4.6 Manipulation Resistance -- Why Median, Not Mean

Consider an attack: a flash loan spikes the Aave USDC supply rate to 50% for a single block. Using the simpler earlier 4-source equal-weight model:

**Rates after attack:**

```
5.80%, 6.20%, 7.10%, 50.00%
```

**Median computation:** Cumulative weights reach 0.50 between 6.20% and 7.10%.

```
ISFR_attacked = (6.20% + 7.10%) / 2 = 6.65%
```

The ISFR shifted 65 basis points -- a transient distortion that corrects on the next update 10 seconds later.

**If ISFR used the mean instead:**

```
mean = (5.80% + 6.20% + 7.10% + 50.00%) / 4 = 17.28%
```

The mean-based rate would have jumped from 5.90% to 17.28% -- an 1,138 bps spike, nearly tripling the index. The median absorbed the outlier. The mean amplified it.

This is why every credible benchmark uses a median (or trimmed mean). SOFR uses a volume-weighted median for the same reason.

### 4.7 Epoch Configuration and Update Cadence

**On-chain cadence:** Every 25 blocks. At Korai's 400ms block time, this yields an update approximately every 10 seconds, producing 8,640 rate observations per day. This is 8,640x the frequency of SOFR (1 daily publication) and approximately 90x the frequency of IPOR (~96 updates per day at 15-minute intervals).

**Off-chain service cadence:** The `isfr-service` (a Python service that pre-computes the rate for development and testing purposes) defaults to hourly updates, configurable via `ISFR_SCHEDULE_HOURS`.

**Clearing cycle epoch (Rust implementation):** The `IsfrConfig` in the Rust implementation defines an epoch of 28,800 seconds (8 hours), producing 3 clearing cycles per day. This clearing epoch is distinct from the 10-second publication cadence -- the rate publishes every 10 seconds, while the clearing cycle (for derivative settlement) operates on 8-hour epochs.

The clearing cycle has six phases, each allocated a fraction of the 8-hour epoch:

| Phase | Allocation | Duration (8h epoch) | Purpose |
|-------|-----------|---------------------|---------|
| Commit | 40% | 3h 12m | Agents submit sealed rate commitments |
| Reveal | 15% | 1h 12m | Agents reveal their rates by providing preimage |
| Solve | 15% | 1h 12m | Clearing engine runs weighted median + outlier exclusion |
| Certificate | 10% | 48m | KKT clearing certificate generated |
| Verify | 10% | 48m | On-chain verification of the certificate |
| Settle | 10% | 48m | Settlement: rates published, rewards distributed |

### 4.8 Staleness Handling and Source Liveness

Each source has a liveness timeout. When a source exceeds its timeout, validators exclude it from their computation and report the reduced source count:

| Source | Expected update frequency | Liveness timeout |
|--------|--------------------------|------------------|
| Aave V3 | Every Ethereum block (~12s) | 120 seconds (10 missed blocks) |
| Compound V3 | Every Ethereum block (~12s) | 120 seconds |
| Ethena sUSDe | As funding settles (~8 hours) | 24 hours |
| ETH Beacon Chain | Per epoch (~6.4 minutes) | 30 minutes (5 missed epochs) |

Additionally, validators perform health checks independently each computation round:

| Health Metric | Threshold | Action |
|---------------|-----------|--------|
| Latency | >30 seconds since last update | Mark source degraded; exclude |
| Deviation | >3 sigma from peer sources in same class | Exclude from intra-class computation |
| Availability | Source RPC unreachable for >60 seconds | Exclude; reweight remaining class sources |

**Source failover:** If all sources in a class go offline, that class's weight is redistributed proportionally to remaining healthy classes. For example, if FUNDING's sole source goes offline, its 10% is redistributed: LENDING gets +6.67%, STRUCTURED +2.78%, STAKING +0.56%.

### 4.9 Outlier Exclusion (Rust Implementation)

The Rust implementation in `isfr.rs` uses a two-pass outlier exclusion algorithm:

1. Compute an initial weighted median across all submissions.
2. Compute the weighted standard deviation around that median.
3. Exclude any submission more than `outlier_sigma` (default: 3.0) standard deviations from the initial median.
4. Recompute the weighted median on the filtered set.

```rust
// Step 1: initial weighted median
let initial_median = weighted_median(&weighted);

// Step 2: compute std dev and exclude 3-sigma outliers
let std_dev = weighted_std_deviation(&weighted, initial_median);
let sigma_bound = self.config.outlier_sigma * std_dev;

let filtered: Vec<(f64, f64)> = weighted.iter()
    .filter(|(rate, _)| (rate - initial_median).abs() <= sigma_bound)
    .copied()
    .collect();

// Step 3: recompute on filtered set
let median_rate = weighted_median(&filtered);
```

Weights for each submission are computed as `confidence * reputation`, where reputation is a per-agent score in [0.0, 1.0] (minimum 0.5 for eligibility).

---

## 5. V1 vs V2 Methodology Differences

The V1 methodology is deliberately simple -- designed for auditability and ease of verification. V2 introduces self-calibrating mechanisms that replace fixed parameters with data-driven optimization.

| Mechanism | V1 (Launch) | V2 (Self-Calibrating) |
|-----------|-------------|----------------------|
| **Source confidence** | Governance-assigned (0-100) | Leave-one-out MSPE (automatic) |
| **Class weights** | Fixed (60/25/10/5) | Bates-Granger optimal combination with governance rails |
| **Intra-class aggregation** | TVL-weighted median | Cost-stratified trimmed mean |
| **Smoothing** | Simple EMA | Kalman filter (separates measurement noise from process noise) |
| **Yield curve** | Discrete rate points | Nelson-Siegel 4-parameter continuous curve |
| **Volatility premium** | 0 (no premium) | Computed from source disagreement (formula TBD) |

### V2 Self-Calibrating Source Confidence

V1 uses governance-assigned confidence scores. V2 replaces these with leave-one-out Mean Squared Prediction Error (MSPE):

```
ISFR_loo[s] = aggregate(all sources except s)
residual[s][t] = source_rate[s][t] - ISFR_loo[s][t]
MSPE[s][t] = lambda * MSPE[s][t-1] + (1-lambda) * residual[s][t]^2
confidence[s][t] = 1 / (1 + MSPE[s][t] / MSPE_floor)
```

Leave-one-out breaks circularity: a source cannot inflate its own confidence by dominating the index. Sources that consistently agree with the consensus earn higher weight organically.

### V2 Adaptive Class Weights (Bates-Granger)

V1's fixed weights (60/25/10/5) are replaced with weights inversely proportional to each class's prediction error, bounded by governance rails:

```
MSPE_class[k][t] = lambda * MSPE_class[k][t-1] + (1-lambda) * (class_rate[k][t-1] - ISFR_realized[t])^2
w_k_raw[t] = 1 / MSPE_class[k][t]
w_k[t] = clamp(w_k_raw[t] / sum(w_j_raw[t]), w_min[k], w_max[k])
```

Governance sets bounds per class (e.g., LENDING: 30-80%, FUNDING: 0-20%). The system finds optimal weights within those bounds from empirical data, following the forecast combination framework of Bates and Granger (1969).

### V2 Cost-Stratified Trim Fractions

V2 replaces the flat TVL-weighted median with a trimmed mean per class, where the trim fraction reflects the economic cost of manipulating that source type:

| Class | Trim alpha | Rationale |
|-------|-----------|-----------|
| LENDING | 0.15 | Manipulation requires actual capital deployment |
| STRUCTURED | 0.20 | Delta-neutral strategies can be unwound rapidly |
| FUNDING | 0.30 | Perpetual positioning can be cheaply manipulated via leverage |
| STAKING | 0.05 | Requires attacking Ethereum consensus |

### V2 Kalman Filter Smoothing

V2 replaces the simple EMA with a Kalman filter that adapts to signal quality:

```rust
struct ISFRKalmanState {
    x_hat: PU18,    // filtered ISFR estimate
    p: PU18,        // error covariance (uncertainty)
    r1_ema: PU18,   // oracle measurement noise
    r2_ema: PU18,   // clearing measurement noise
    q: PU18,        // process noise (governance parameter)
}
```

An EMA treats all deviations as signal noise. A Kalman filter distinguishes measurement noise (source disagreement, stale data) from process noise (genuine rate movement).

### V2 Nelson-Siegel Yield Curve

V2 publishes not discrete rate points but 4 parameters characterizing the entire yield curve:

```
y(tau) = beta_1 + beta_2 * [(1 - e^(-lambda*tau)) / (lambda*tau)]
       + beta_3 * [(1 - e^(-lambda*tau)) / (lambda*tau) - e^(-lambda*tau)]
```

Where beta_1 = long-run level, beta_2 = slope, beta_3 = curvature, lambda = decay speed. Any consumer can reconstruct ISFR at any tenor tau from these 4 numbers.

### V2 Governance Constraints on Source Weights

- Maximum weight per source: 0.35 (no single source can exceed 35% influence)
- Minimum weight per source: 0.05 (no source can be effectively zeroed without removal)
- Weight changes require a 7-day timelock and super-majority governance vote

---

## 6. The ISFR Acronym Collision

Two different concepts share the acronym "ISFR" across the Nunchi codebase and documentation. This collision is important to understand when reading any Nunchi source code or documentation.

| Term | Full Name | Scope | Where You Will Find It |
|------|-----------|-------|------------------------|
| **ISFR** (the index) | **Internet Secured Funding Rate** (originally: Implied Secured Funding Rate) | External DeFi reference rate. Product-facing. What yield perpetuals settle against. | `isfr-service` repo, oracle specifications, yield perp contracts, `roko-chain/src/isfr.rs` (as `IsfrRegistry`, `IsfrAggregate`) |
| **ISFR_score** (the metric) | **Internal Solvency & Funding Ratio** | Internal per-agent risk/health metric computed by the TEE clearing engine's scoreboard. Measures an agent's solvency health. | TEE architecture docs, `scoreboard.py`, risk manager code |

**Rule of thumb:** If the context is an oracle, a rate, a benchmark, or a yield perpetual, it is the index. If the context is a scoreboard, a risk manager, or a per-agent health metric in the clearing engine, it is the solvency ratio.

The Rust implementation in `roko-chain/src/isfr.rs` is about the index, not the solvency metric, despite the module-level doc comment referring to it as the "Intersubjective Fact Registry" -- a third expansion of the acronym used within the agent-economy context, where ISFR serves as a collective rate discovery mechanism for agents submitting rate observations for hierarchical market IDs.

---

## 7. Sub-Indices

Every ISFR computation round produces five values, all available via the oracle precompile:

| Index | What It Contains | Use Case |
|-------|-----------------|----------|
| **ISFR** | The primary composite rate (canonical benchmark) | Settlement rate for yield perpetuals. Published in block header. |
| **ISFR.LENDING** | Lending class rate (Aave V3, Compound V3) | A protocol hedging Aave supply rate risk references this directly. |
| **ISFR.STRUCTURED** | Structured yield class rate (Ethena sUSDe) | A delta-neutral vault monitors this for strategy performance. |
| **ISFR.FUNDING** | Funding class rate (Hyperliquid ETH perp) | Captures speculative sentiment. High FUNDING divergence from LENDING signals leverage demand. |
| **ISFR.STAKING** | Staking class rate (ETH Beacon Chain) | The base-layer floor rate. Very stable. |

Sub-indices are byproducts of computing ISFR -- zero marginal cost. This granularity has no equivalent in IPOR (single index), Pendle (per-asset prices), or traditional benchmarks (SOFR publishes one rate plus percentiles).

The sub-indices enable precise hedging. A treasury with USDC on Aave cares about lending rates specifically, not a composite that blends in perpetual funding volatility. They reference ISFR.LENDING. A delta-neutral vault using Ethena's strategy monitors ISFR.STRUCTURED. The composite ISFR serves as the canonical settlement rate for yield perpetuals that track aggregate DeFi yield.

---

## 8. Publication Mechanism

### 8.1 The Oracle Precompile

ISFR is published via a dedicated EVM precompile at address `0xA01` on the Korai Kernel Plane -- a consensus-level primitive with fixed gas cost.

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

Alternative access patterns:

```solidity
// Read current ISFR (simplified)
(uint32 valueBps, uint8 state) = ISFROracle(0xA01).current();

// Read historical ISFR at a specific block
ISFRSnapshot memory snap = ISFROracle(0xA01).at(blockHeight);

// Read TWAP over a time range
uint32 twapBps = ISFROracle(0xA01).twap(startBlock, endBlock);
```

### 8.2 On-Chain Storage Structure

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

### 8.3 Publication States and Circuit Breakers

ISFR operates in one of four states at any given time, determined automatically by the consensus layer based on data availability and validator agreement:

| State | Condition | ISFR Behavior | Derivative Behavior |
|-------|-----------|---------------|---------------------|
| **Live** | 3+ sources reporting AND confidence >= 70% | Normal publication | Normal operations |
| **Degraded** | Exactly 2 sources OR confidence 50-70% | Rate published with wider confidence interval | Normal with warning flag; tight clearing profiles may pause |
| **Stale** | Exactly 1 source OR 50-67% validator participation | Rate frozen at last Live/Degraded value | No new clearing profile activations |
| **Halted** | 0 sources OR confidence < 50% OR consensus failure | Rate frozen | Emergency CLOB only. No new positions. Existing positions preserved. |

### 8.4 Confidence Score

Confidence measures the degree of validator agreement:

```
confidence = sum(s_i for all i where |v_i - median| <= sigma) / sum(all s_i)
```

Where sigma is the standard deviation of all submitted votes weighted by stake. High confidence (>90%) means validators substantially agree. Low confidence (<70%) means significant disagreement.

In the V4 specification, confidence is defined as the percentage of total stake weight that submitted votes within 10 bps of the finalized median (0-100 scale).

### 8.5 Circuit Breaker Trigger

When confidence drops below 70%, the circuit breaker fires:

1. ISFR state transitions from Live to Degraded (if >= 50%) or Halted (if < 50%).
2. The previous Live rate is cached as the fallback.
3. An `ISFRCircuitBreaker` event is emitted on-chain.
4. The clearing engine switches to wider spread limits (Degraded) or emergency CLOB mode (Halted).
5. **Recovery hysteresis:** confidence must exceed 80% for 3 consecutive update periods (30 seconds) before transitioning back to Live. The 70% down / 80% up hysteresis prevents oscillation.

### 8.6 Validator Consensus Pipeline

Each validator independently:

1. Pulls latest source data via RPC from each registered source.
2. Performs health checks (latency, deviation, availability).
3. Computes Level 1: TVL-weighted median per source class.
4. Computes Level 2: weighted sum of class rates to produce ISFR.
5. Submits an `OracleVote` with all five values (ISFR + 4 sub-indices) to the chain.

The chain finalizes via stake-weighted median across all validator votes.

### 8.7 The Hybrid Rate Formula

The block header publishes both oracle-derived and market-derived rates. The canonical ISFR combines them:

```
ISFR = ISFR_oracle + EMA(ISFR_market - ISFR_oracle)
```

At launch with thin clearing liquidity, the EMA contribution is negligible: `ISFR ~ ISFR_oracle`. As the yield perpetual market deepens, `ISFR_market` becomes progressively more informative, and the benchmark naturally transitions toward endogenous price discovery without a binary cutover.

### 8.8 Publication Path Options

Three options exist for bridging the off-chain `isfr-service` rate to the on-chain oracle during the transition period:

| Option | How It Works | Status |
|--------|-------------|--------|
| **Validator sidecar integration** | Each validator's `oracle-sidecar` polls `/v1/isfr/current` and includes the value in its `OracleVote` | Recommended for production |
| **DeskFeed connector** | A `FeedSource` trait implementation for ISFR; uses authenticated HTTPS to a Nunchi-operated endpoint | Recommended for V1 testnet |
| **On-chain publisher contract** | Single signer posts values to a contract | Simplest but has single-signer risk |

---

## 9. The Service Architecture

### 9.1 The `isfr-service` Repository

The off-chain service that pre-computes ISFR for development, testing, and the initial publication path:

| Field | Value |
|-------|-------|
| Repository | `https://github.com/Nunchi-trade/isfr.git` |
| Language | Python |
| Entry point | `python -m isfr.main` (scheduler + API) or `uvicorn isfr.api:app --host 0.0.0.0 --port 8000` (API only) |
| Installation | `pip install -e ".[dev]"` |

### 9.2 Architecture

```
Ethereum RPC --> Scheduler (ISFR_SCHEDULE_HOURS)
                    |
                    +--> Aave V3 scraper
                    +--> Compound V3 scraper
                    +--> Hyperliquid funding scraper
                    +--> Ethena yield scraper
                    |
                    v
                Calculator (weighted_median())
                    |
                    +--> JSONL storage (ISFR_DATA_DIR)
                    +--> FastAPI / uvicorn (:8000)
                              |
                              +--> /v1/isfr/current --> Consumers
                              +--> /v1/isfr/history --> Consumers
```

### 9.3 API Endpoints

| Endpoint | Method | Returns | Notes |
|----------|--------|---------|-------|
| `/v1/isfr/current` | GET | Latest ISFR rate | The value yield perps reference for mark price |
| `/v1/isfr/history` | GET | Historical rates (query `?days=30`) | For backtesting, charting, audit |
| `/health` | GET | Service health | Liveness probe |

### 9.4 Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `ETH_RPC_URL` | `https://eth.llamarpc.com` | Ethereum RPC endpoint for on-chain scraping |
| `ISFR_DATA_DIR` | `data` | Directory for JSONL storage of historical rates |
| `ISFR_SCHEDULE_HOURS` | `1` | Calculation interval in hours |
| `ISFR_PORT` | `8000` | API server port |

### 9.5 The Rust Implementation (`roko-chain/src/isfr.rs`)

The Rust implementation in the `roko-chain` crate provides the on-chain data structures and aggregation logic:

**Key types:**

- `IsfrConfig` -- epoch duration (default 28,800s = 8h), max KKT residual (1e-6), min submissions (2), min reputation (0.5), max rate bound (0.1), outlier sigma (3.0).
- `ClearingPhase` -- six-phase state machine: Commit -> Reveal -> Solve -> Certificate -> Verify -> Settle.
- `PhaseAllocations` -- fraction of epoch per phase (40/15/15/10/10/10 percent).
- `ClearingCycleState` -- tracks current epoch, phase, and timing.
- `MarketId` -- hierarchical market identifier (e.g., "knowledge/defi", "compute/inference").
- `IsfrSubmission` -- a rate observation with market_id, rate, components, confidence, and submitter passport ID.
- `IsfrAggregate` -- output: median_rate, submission_count, std_deviation, excluded_count.
- `IsfrRegistry` -- the main registry that collects submissions, checks eligibility, and computes aggregates.

**Key constants in the default configuration:**

| Constant | Value | Purpose |
|----------|-------|---------|
| `epoch_duration_secs` | 28,800 (8 hours) | One clearing cycle |
| `max_kkt_residual` | 1e-6 | Certificate acceptance threshold |
| `min_submissions_for_clearing` | 2 | Minimum submissions before aggregation |
| `min_reputation` | 0.5 | Eligibility floor |
| `max_rate_bound` | 0.1 (10%) | Maximum absolute rate value |
| `outlier_sigma` | 3.0 | Sigma multiplier for outlier exclusion |

---

## 10. NRIS -- The Nunchi Reference Index Suite

ISFR is the first index in a broader framework called the **Nunchi Reference Index Suite (NRIS)**. NRIS is a separate, already-live index suite covering equities, FX, and fixed income for Hyperliquid perpetual markets.

| Property | ISFR | NRIS |
|----------|------|------|
| What it measures | DeFi yield rates | Equity prices, FX baskets, bond yields |
| Sources | 4 DeFi venues (V1); 7+ (V2) | Pyth, Stork, carry-forward fallback |
| Architecture | Python service (`isfr-service`) + Rust on-chain (`roko-chain`) | Rust workspace with `nris-core`; 7 SEDA oracle programs |
| Outputs | 1 composite index + 4 sub-indices | 12 index outputs across 4 asset classes |
| Status (April 2026) | V1 in development | Live on testnet, 12 indices running |
| Target venue | Korai oracle precompile for yield perps | Hyperliquid HIP-3 markets |

NRIS and ISFR are complementary, not alternatives. NRIS covers traditional asset classes; ISFR covers DeFi yield. Both feed their respective perpetual market suites.

The generalized benchmark pattern that ISFR demonstrates -- multi-source aggregation, dual-median computation, validator consensus, precompile publication, and prediction scoring -- is designed to extend to additional domains:

| Proposed Index | What It Measures | Sources |
|----------------|-----------------|---------|
| **ISFR** | DeFi secured funding cost | Aave, Compound, Ethena, ETH staking |
| **IAPI** | Agent task success rates | Arena results, gate pass rates, task completion |
| **IKQI** | Knowledge quality | Confirmation rates, usage frequency, CRPS scores |
| **ISVI** | Security detection rates | Audit outcomes, bug bounty results |
| **IRRI** | Research output quality | Citation rates, replication success |

Each follows the same construction: define sources, assign weights, validators compute weighted median, chain aggregates via stake-weighted median, publish via precompile, agents commit predictions scored via CRPS.

### The BenchmarkIndex Trait

The generalization is captured in a Rust trait:

```rust
pub trait BenchmarkIndex: Send + Sync {
    fn sources(&self) -> &[IndexSource];
    fn compute(&self, readings: &[SourceReading]) -> IndexValue;
    fn confidence(&self, validator_votes: &[Vote]) -> f64;
    fn update_cadence_blocks(&self) -> u64;  // ISFR: 25 blocks (~10s)
    fn precompile_address(&self) -> Address;
    fn circuit_breaker_threshold(&self) -> f64 { 0.70 }
}
```

---

## Appendix A: Key Formulas Reference

**Weighted median:**
Sort (value, weight) pairs by value. Accumulate weights. Return the value where cumulative weight >= 50% of total weight.

**ISFR composite (V1 equal-weight model):**
```
ISFR = Weighted_Median(Sources) + Volatility_Premium
V1: Volatility_Premium = 0
```

**ISFR composite (V4 two-level model):**
```
ISFR = 0.60 * LENDING + 0.25 * STRUCTURED + 0.10 * FUNDING + 0.05 * STAKING
```

**Effective source weight:**
```
effective_weight(source) = tvl(source) * (confidence(source) / 100)
```

**Hybrid mark price:**
```
ISFR = ISFR_oracle + EMA(ISFR_market - ISFR_oracle)
```

**Yield perp mark price:**
```
MarkPrice = 0.7 * ISFR_Oracle + 0.3 * EMA(OrderBook_MidPrice, 300s)
```

**CRPS scoring (V1, point prediction):**
```
CRPS(agent_i) = |predicted_i - actual|
```

**CRPS scoring (V2, distributional prediction):**
```
CRPS(F, y) = integral[-inf, +inf] [F(x) - 1(x >= y)]^2 dx
```

**Epistemic score:**
```
epistemicScore(agent, "isfr") = EMA_30d(CRPS scores)
```

**V2 leave-one-out confidence:**
```
ISFR_loo[s] = aggregate(all sources except s)
residual[s][t] = source_rate[s][t] - ISFR_loo[s][t]
MSPE[s][t] = lambda * MSPE[s][t-1] + (1-lambda) * residual[s][t]^2
confidence[s][t] = 1 / (1 + MSPE[s][t] / MSPE_floor)
```

**V2 Bates-Granger adaptive weights:**
```
MSPE_class[k][t] = lambda * MSPE_class[k][t-1] + (1-lambda) * (class_rate[k][t-1] - ISFR_realized[t])^2
w_k_raw[t] = 1 / MSPE_class[k][t]
w_k[t] = clamp(w_k_raw[t] / sum(w_j_raw[t]), w_min[k], w_max[k])
```

**Nelson-Siegel yield curve (V2):**
```
y(tau) = beta_1 + beta_2 * [(1 - e^(-lambda*tau)) / (lambda*tau)]
       + beta_3 * [(1 - e^(-lambda*tau)) / (lambda*tau) - e^(-lambda*tau)]
```

## Appendix B: Credibility Roadmap

| Phase | Timeline | Key Activities |
|-------|----------|----------------|
| Phase 1: Curated Aggregation | Q3 2026 | Launch with 4 V1 sources; governance-assigned weights; anchor agent consumers |
| Phase 2: Track Record | Q4 2026 | 90+ days uninterrupted publication; source expansion (30-day probation); V2 self-calibration activates |
| Phase 3: Reflexive Loop | Q1-Q2 2027 | ISFR-settled derivatives grow; external institutional evaluation; IOSCO alignment review |
| V3: Cross-chain | Q3 2027+ | Solana, L2 sources (Aave on Arbitrum/Base/Optimism) via same probation framework |
| V4: TradFi Bridges | 2028+ | SOFR on-chain via attested data feeds; UST-3M; DeFi-to-TradFi basis instruments |

## Appendix C: ISFR vs SOFR

| Property | SOFR | ISFR |
|----------|------|------|
| Publisher | Federal Reserve Bank of New York | Korai validator set (decentralized) |
| Update frequency | Once daily (8:00 AM ET) | Every 10 seconds (8,640/day) |
| Computation | Volume-weighted median of overnight Treasury repo | Dual weighted median (source-level + validator-level) |
| Data sources | ~$2T daily tri-party, GCF, bilateral Treasury repo | 4 DeFi protocol rates (V1), 7+ (V2) |
| Trust model | Trust the Federal Reserve | Trust that >50% of validator stake is honest |
| Availability | Weekdays only | 24/7/365 |
| Latency | T+1 (published morning after) | Real-time (10-second updates) |
| Programmability | Not natively programmable | Native precompile; one opcode call |
| Circuit breakers | None (published or not) | Four-state model with confidence-based triggers |
| Derivative notional | ~$570T+ | $0 (pre-launch) |
