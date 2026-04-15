# The Oracle System

## A Self-Contained Reference for the Nunchi Oracle Infrastructure

---

## 1. Why Nunchi Builds Its Own Oracle Infrastructure

### The Problem with Existing Oracles

Existing DeFi oracle networks -- Chainlink, Pyth, API3 -- operate as separate operator layers outside a blockchain's own consensus mechanism. They introduce an additional trust assumption: the chain trusts its own validators for transaction ordering and state transitions, but delegates price and rate data to a distinct set of operators with separate incentives, separate security budgets, and separate failure modes. If those operators are compromised, the data is compromised, regardless of how secure the underlying chain is.

For a blockchain purpose-built to host autonomous economic agents (Korai), this separation is structurally unacceptable. Agents that manage capital, execute trades, and hedge risk need data feeds that are as reliable as the chain itself -- not contingent on a secondary network that may go offline, become stale, or be manipulated independently of the chain's own security guarantees.

### The Oracle Thesis

Nunchi's oracle system is built on a single architectural principle: **rate and index computation should be embedded directly into validator consensus, not outsourced to external operators.** Every Korai validator independently computes oracle values as part of block production. The chain aggregates these computations via stake-weighted median. The result is published via EVM precompiles -- consensus-level primitives with fixed gas cost, as efficient as reading the block number.

This design eliminates the oracle operator trust assumption entirely. The oracle is as secure as the chain's own consensus. It is as available as the chain itself. It cannot be compromised without compromising the validator set -- and the chain's own proof-of-stake security budget protects both transaction ordering and oracle data simultaneously.

### The Concrete Motivation: ISFR

The immediate motivation is the Internet Secured Funding Rate (ISFR), a composite benchmark index representing the cost of secured funding across decentralized finance. DeFi has approximately $49.5 billion in lending TVL but zero credible benchmark interest rates. The TradFi interest rate derivatives market is $665.8 trillion in notional outstanding -- anchored almost entirely by benchmark reference rates like SOFR. DeFi's six-order-of-magnitude gap in interest rate derivative activity is not a marketing failure; it is the absence of a foundational primitive.

ISFR is that primitive. But the oracle system that publishes it is designed to generalize far beyond interest rates.

---

## 2. Oracle Architecture

### The Two-Level Aggregation Model

The oracle system uses a dual-layer aggregation architecture that provides independent Byzantine fault tolerance at each layer:

**Level 1 -- Source Aggregation (per validator).** Each validator independently reads data from multiple on-chain and off-chain sources, then computes a weighted median across those sources. The weighted median tolerates up to 49% corrupted source weight without affecting the output. No single source can dominate the result.

**Level 2 -- Validator Aggregation (consensus).** The chain computes a stake-weighted median across all validator submissions. This tolerates up to 49% compromised validator stake. An attacker controlling 30% of stake cannot move the finalized value beyond the range of honest validators' submissions.

To manipulate the oracle output to an arbitrary value, an attacker must simultaneously compromise 50%+ of source weight at Level 1 AND 50%+ of validator stake at Level 2. Either layer independently stops the attack.

### The Computation Pipeline

The pipeline runs at a configurable cadence per index. For ISFR, updates occur every 25 blocks (approximately 10 seconds at Korai's 400ms block time). Each validator independently:

1. **Reads source data** via RPC from each registered source (e.g., Aave V3 supply rates, Compound V3 supply rates, Ethena sUSDe yield, ETH staking rate).
2. **Performs health checks** -- latency (>30 seconds = exclude), deviation (>3 sigma from peers = exclude), availability (RPC unreachable >60 seconds = exclude).
3. **Computes Level 1** -- TVL-weighted median per source class. Sources within each class are sorted by rate, weights accumulated, and the value at the 50% cumulative weight threshold is selected.
4. **Computes Level 2** -- weighted sum of class rates to produce the composite index. For ISFR: `0.60 * LENDING + 0.25 * STRUCTURED + 0.10 * FUNDING + 0.05 * STAKING`.
5. **Submits an OracleVote** containing the computed values and a BLS signature over `(value_bps, block_height)`.

The chain finalizes by computing the stake-weighted median across all validator votes. The finalized value is written to the oracle precompile.

### Validator Vote Structure

```rust
struct OracleVote {
    /// The validator's computed index value in basis points.
    value_bps: u32,
    /// Block height this vote applies to.
    block_height: u64,
    /// Validator's BLS signature over (value_bps, block_height).
    signature: BlsSignature,
    /// Validator index in the current committee.
    validator_index: u32,
}
```

### On-Chain Storage

Each oracle update produces a snapshot stored at the precompile address:

```solidity
struct ISFRSnapshot {
    uint32 valueBps;          // Index value in basis points (e.g., 600 = 6.00%)
    uint64 blockHeight;       // Block at which this value was computed
    uint64 timestamp;         // Unix timestamp
    uint8  state;             // 0=Live, 1=Degraded, 2=Stale, 3=Halted
    uint16 confidenceBps;     // Percentage of validators within 1-sigma, in bps
    uint32 numSources;        // Number of active sources contributing
    uint32 numValidatorVotes; // Number of validator votes in this round
}
```

Historical values are retained for 90 days on-chain (approximately 19.4 million snapshots at 10-second cadence for ISFR).

---

## 3. Precompile Addresses and Their Functions

Oracle data is published via dedicated EVM precompiles on the Korai Kernel Plane. Precompiles are consensus-level primitives -- they execute at fixed gas cost, equivalent to reading the block number. They are not smart contract calls with variable gas; they are part of the chain's execution environment.

### `0xA01` -- ISFR Oracle Precompile

The primary precompile. Any smart contract on Korai can read the current ISFR and its sub-indices with a single call:

```solidity
interface ISFROracle {
    /// Returns ISFR composite + 4 sub-indices + metadata.
    function currentRate() external view returns (
        uint256 isfr,           // composite rate in basis points
        uint256 lendingRate,    // ISFR.LENDING sub-index
        uint256 structuredRate, // ISFR.STRUCTURED sub-index
        uint256 fundingRate,    // ISFR.FUNDING sub-index
        uint256 stakingRate,    // ISFR.STAKING sub-index
        uint64 timestamp,
        uint8 confidence        // 0-100, validator agreement metric
    );

    /// Returns ISFR at a specific block epoch.
    function rateAt(uint64 epochBlock) external view returns (
        uint256 isfr, uint64 timestamp
    );

    /// Returns historical ISFR values (up to 30 days on-chain).
    function history(uint64 fromEpoch, uint64 toEpoch) external view returns (
        uint256[] memory rates, uint64[] memory timestamps
    );

    /// Returns time-weighted average rate over a block range.
    function twap(uint64 startBlock, uint64 endBlock) external view returns (
        uint32 twapBps
    );
}
```

### `0xA04` -- PROOF_LOG Precompile

Used for prediction commitment and scoring. Agents commit predictions before each oracle update via hash commitment (`hash(predictedValue || salt)`), then reveal after the outcome. This prevents front-running -- no agent can see others' predictions before committing its own.

### Confidence Score

The confidence score measures validator agreement: the percentage of total stake weight that submitted votes within one standard deviation of the finalized stake-weighted median (0--100). Consuming contracts can gate actions on confidence. For example, a yield perpetual contract might reject new positions when confidence drops below 70, ensuring only high-quality rate observations trigger economic actions.

---

## 4. How ISFR Gets Published On-Chain via the Oracle System

### What ISFR Is

The Internet Secured Funding Rate is a composite benchmark index representing the cost of secured funding across DeFi. It is to DeFi what SOFR (Secured Overnight Financing Rate) is to traditional finance: the reference rate that financial instruments settle against.

### Source Class Taxonomy

ISFR organizes DeFi's yield surface into four mutually exclusive, collectively exhaustive classes:

| Class | Weight | What It Measures | V1 Sources |
|-------|--------|------------------|------------|
| **LENDING** | 0.60 | Collateralized lending yield | Aave V3, Compound V3 |
| **STRUCTURED** | 0.25 | Multi-instrument strategy yield | Ethena sUSDe |
| **FUNDING** | 0.10 | Perpetual futures funding rate | Hyperliquid ETH perp |
| **STAKING** | 0.05 | Proof-of-stake validator yield | ETH staking rate |

Each class captures a structurally distinct yield generation mechanism. Lending rates measure the cost of borrowing USDC against crypto collateral (closest analog to overnight secured lending in TradFi). Structured yield captures delta-neutral funding rate strategies. Funding rates capture speculative positioning and leverage demand. Staking yield is the base cost of securing Ethereum -- the "risk-free" rate of crypto.

### The Two-Level Computation

**Level 1 (Intra-Class).** Within each class, sources are aggregated via TVL-weighted median with confidence modulation:

```
effective_weight(source) = tvl(source) * (confidence(source) / 100)
```

New sources enter with low confidence (typically 30 for 30 days), enabling smooth phase-in. The TVL-weighted median tolerates up to 49% corrupted weight within each class.

**Level 2 (Inter-Class).** The final ISFR is a deterministic weighted sum:

```
ISFR = 0.60 * LENDING + 0.25 * STRUCTURED + 0.10 * FUNDING + 0.05 * STAKING
```

### Worked Example

Suppose these source rates are observed at a given epoch:

| Class | Rate | Weight |
|-------|------|--------|
| LENDING | 6.20% | 0.60 |
| STRUCTURED | 7.10% | 0.25 |
| FUNDING | 12.40% | 0.10 |
| STAKING | 3.20% | 0.05 |

```
ISFR = 0.60 * 6.20% + 0.25 * 7.10% + 0.10 * 12.40% + 0.05 * 3.20%
     = 3.720% + 1.775% + 1.240% + 0.160%
     = 6.895% (approximately 690 basis points)
```

The elevated FUNDING rate (12.40%) contributes just 124 bps to the composite -- its 10% class weight limits its influence. Under a flat equal-weight average of all four rates, the speculative signal would pull the composite 33 bps higher. Two-level aggregation keeps ISFR anchored to lending fundamentals where hedging demand concentrates.

### Published Values

Every computation round produces five values, all available via the `0xA01` precompile:

- **ISFR** -- The primary composite rate (canonical benchmark, published in block header)
- **ISFR.LENDING** -- Lending class rate
- **ISFR.STRUCTURED** -- Structured yield class rate
- **ISFR.FUNDING** -- Funding class rate
- **ISFR.STAKING** -- Staking class rate

Sub-indices are byproducts of computing the composite -- zero marginal cost. A protocol hedging Aave supply rate risk can reference ISFR.LENDING directly. A delta-neutral vault can monitor ISFR.STRUCTURED.

### The Hybrid Rate

ISFR has two sources of truth: the oracle layer (external DeFi rate measurement) and the clearing engine (endogenous market discovery). The canonical ISFR combines them:

```
ISFR = ISFR_oracle + EMA(ISFR_market - ISFR_oracle)
```

At launch, with thin clearing liquidity, ISFR approximates ISFR_oracle. As the yield perpetual market deepens, ISFR_market becomes progressively more informative, and the benchmark naturally transitions toward endogenous price discovery -- no binary cutover required.

---

## 5. Publication States and Circuit Breakers

The oracle operates in four explicitly signaled states:

| State | Condition | Behavior |
|-------|-----------|----------|
| **Live** | 3+ sources reporting AND confidence >= 70% | Normal publication |
| **Degraded** | 2 sources reporting OR confidence 50--70% | Rate published with wider confidence interval |
| **Stale** | 1 source reporting | Rate frozen at last valid value |
| **Halted** | 0 sources OR confidence < 50% | No rate published; emergency fallback mode |

The circuit breaker fires when confidence drops below 70%. Recovery requires confidence to exceed 80% for 3 consecutive update periods (30 seconds). The hysteresis (70% down, 80% up) prevents oscillation. There is no silent failure mode -- a consuming contract always knows whether the rate is trustworthy.

### Source Failover

If all sources in a class go offline, that class's weight is redistributed proportionally to remaining healthy classes. ISFR requires at least 3 healthy sources across classes for Live status.

---

## 6. Validator Roles and Consensus Mechanism

### Who Computes

Every Korai validator independently computes oracle values as part of block production. There is no separate oracle operator set, no off-chain committee, no bridge dependency. Validators read source data from their own infrastructure (Ethereum full nodes for on-chain sources, direct RPC for off-chain sources like Ethena).

### Who Attests

Each validator signs their computed value with their BLS key. The signature covers `(value_bps, block_height)`, binding the attestation to a specific computation at a specific time.

### Consensus Mechanism

The chain finalizes oracle values via **stake-weighted median** across all validator votes. The stake-weighted median selects the value v_j such that:

```
sum(stake_i for i <= j) >= 0.50 AND sum(stake_i for i < j) < 0.50
```

where votes are sorted by value and stake_i is the normalized stake weight of validator i.

This mechanism inherits the chain's own security budget. An attacker must acquire 50%+ of total stake to move the oracle to an arbitrary value -- the same threshold required to attack the chain's consensus itself.

### Independent Verification

Validators perform health checks independently during each computation round. No separate watcher service is required. Each validator evaluates source latency, deviation from peer sources, and availability before including a source in their computation. This means that even if one validator has a faulty data connection, the stake-weighted median across all validators filters out the anomalous submission.

---

## 7. Oracle Surfaces -- What Data Surfaces Exist and How They Are Consumed

### On-Chain Surfaces

**Precompile reads.** Any smart contract on Korai can call the oracle precompile to read current or historical index values. Gas cost is fixed and minimal. This is the primary consumption path for DeFi instruments (yield perpetuals, clearing profiles, lending protocols that reference ISFR).

**Block header publication.** The finalized ISFR value is included in the block header. Light clients can verify the rate without querying precompile state.

### Structured Knowledge Entries (Engrams)

Every oracle update produces a structured knowledge entry that enters the InsightStore -- Korai's on-chain knowledge repository:

```
ISFRInsight {
    kind:       "OracleUpdate"
    domain:     "yield_rates"
    value:      isfr_value_bps
    components: { lending, structured, funding, staking }
    deviation:  delta_from_previous
    confidence: validator_agreement
    timestamp:  block_timestamp
    decay:      HalfLife(7 days)
}
```

These entries are queryable via a similarity-search precompile (~170 microseconds at 100K vectors). The 7-day half-life enforces freshness -- recent data is weighted more heavily than stale data.

### Derived Knowledge

Agents publish derived insights that compound the value of raw rate data:

- **Mean reversion patterns:** "When ISFR diverges >50 bps from ISFR.LENDING, mean reversion occurs within 48h in 73% of observations."
- **Volatility regime shifts:** "ISFR 7-day stdev exceeding 15 bps precedes >100 bps rate moves within 14 days."
- **Source correlation breakdowns:** "ISFR.STRUCTURED decoupling from ISFR.LENDING by >200 bps -- potential basis collapse."
- **Cross-domain transfers:** "High ISFR volatility correlates with increased smart contract deployment activity (r=0.42)."

The result is autocatalytic: oracle updates produce knowledge, knowledge improves predictions, predictions attract agents, agents produce more knowledge. Each revolution accelerates the next.

### Agent-Facing Surfaces

For the agent runtime (Roko), the oracle system surfaces data through several channels:

- **Prediction accuracy dashboard.** Aggregate and per-category accuracy, trend arrows, universe size -- always visible in the agent's TUI chrome.
- **Attention universe view.** The agent's tracked items across three tiers (Active, Watched, Scanned), with promotion/demotion signals.
- **Oracle daily digest.** Accumulated state, surfaced when the agent starts: accuracy trend, new discoveries, blocked actions, environmental models.
- **API endpoints.** `GET /api/v1/oracle` for aggregate state, `/oracle/universe` for tracked items, `/oracle/predictions` for prediction history, `/oracle/environmental` for cross-pool patterns.

---

## 8. The Abstraction Layer -- Generalizing Beyond ISFR

### The BenchmarkIndex Trait

ISFR is the first index in a broader framework called the Nunchi Reference Index Suite (NRIS). The same infrastructure -- validator computation, precompile publication, prediction scoring -- extends to any domain where multiple independent sources produce measurable signals.

The generalization is captured in a Rust trait:

```rust
pub trait BenchmarkIndex: Send + Sync {
    /// The set of sources contributing to this index.
    fn sources(&self) -> &[IndexSource];

    /// Compute the index value from a set of source readings.
    /// Each validator calls this independently during block production.
    /// The implementation must be deterministic given the same inputs.
    fn compute(&self, readings: &[SourceReading]) -> IndexValue;

    /// Compute the confidence score from validator vote distribution.
    fn confidence(&self, validator_votes: &[Vote]) -> f64;

    /// Update cadence in blocks. ISFR: 25 blocks (~10s). IAPI: 750 blocks (~5min).
    fn update_cadence_blocks(&self) -> u64;

    /// Precompile address where this index is published.
    fn precompile_address(&self) -> Address;

    /// Circuit breaker threshold.
    fn circuit_breaker_threshold(&self) -> f64 { 0.70 }
}
```

Any type implementing `BenchmarkIndex` automatically gets:
- Validator-computed aggregation via the two-level architecture
- Precompile publication at a dedicated address
- Confidence scoring and circuit breaker logic
- CRPS-scored prediction loops
- InsightStore knowledge entry generation

### Candidate Indices Beyond ISFR

| Index | What It Measures | Sources | Update Frequency |
|-------|------------------|---------|------------------|
| **ISFR** | DeFi secured funding cost | Aave, Compound, Ethena, ETH staking | 10 seconds |
| **IAPI** | Agent task success rates | Arena results, gate pass rates, task completion | 5 minutes |
| **IKQI** | Knowledge entry quality | Confirmation rates, usage frequency, CRPS scores | 1 hour |
| **ISVI** | Security vulnerability detection rates | Audit outcomes, bug bounty results, verification scores | 1 hour |
| **IRRI** | Research output quality | Citation rates, replication success, peer validation | 24 hours |

Each follows the same five-component pattern:

1. Define 4--7 independent sources measuring the target phenomenon
2. Assign weights (equal in V1, governance-adjustable in V2)
3. Validators compute the weighted median from independent observations
4. The chain aggregates via stake-weighted median
5. The index is published via a precompile; agents predict and are scored via CRPS

### The PredictionDomain Trait (Agent-Side Abstraction)

On the agent side, the oracle system uses a separate abstraction for protocol-specific prediction logic. A `PredictionDomain` is a self-contained module that teaches the agent's oracle engine how to predict, observe, and evaluate one category of on-chain activity:

```rust
#[async_trait]
pub trait PredictionDomain: Send + Sync + 'static {
    fn domain_id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn categories(&self) -> Vec<PredictionCategory>;

    async fn discover(&self, seed: &AttentionSeed, chain: &ChainClient,
        indexer: Option<&IndexerClient>) -> Result<Vec<TrackedItem>>;

    fn predict_scanned(&self, item: &TrackedItem, history: &ResidualHistory,
        ctx: &PredictionContext) -> Vec<PredictionTemplate>;
    fn predict_watched(&self, ...) -> Vec<PredictionTemplate>;
    fn predict_active(&self, ...) -> Vec<PredictionTemplate>;
    fn predict_on_action(&self, ...) -> Vec<PredictionTemplate>;

    fn resolution_query(&self, item: &TrackedItem,
        category: &PredictionCategory) -> ResolutionMethod;
    fn check_interval(&self, tier: &AttentionTier) -> u64;
    fn matches_seed(&self, seed: &AttentionSeed) -> bool;
}
```

The oracle engine operates generically over PredictionDomains. Adding a new protocol requires implementing this trait (approximately 100--200 lines of Rust) and registering it in the domain registry. The engine code -- ledger, accuracy tracking, residual correction, attention management -- is completely untouched.

The same Uniswap V3 pool address produces two entirely different domain implementations depending on the activity: `UniswapV3LpDomain` (for liquidity providers -- predicts fee rates, IL, time in range, net PnL over days) and `UniswapV3SwapDomain` (for traders -- predicts slippage, gas, price impact; resolves instantly). The oracle engine does not distinguish between them -- it processes `PredictionTemplate` objects identically regardless of their domain origin.

---

## 9. Prediction Markets and Scoring

### The Prediction Loop

The oracle system creates the densest scoring feedback loop in any on-chain prediction system. The cycle repeats every oracle update (every 10 seconds for ISFR):

1. **Predict.** Agents register predictions for the next index value.
2. **Commit.** Predictions are committed on-chain via hash commitment: `hash(predictedValue || salt)`. The hash prevents front-running.
3. **Observe.** At the next epoch, validators publish the actual value.
4. **Score.** The residual is computed using CRPS (Continuous Ranked Probability Score).
5. **Calibrate.** Residuals feed back into agent models, improving future predictions.

At 10-second cadence, ISFR produces 8,640 updates per day. Each update can score every committed prediction. For an agent committing predictions to every update, that is 8,640 calibration signals per day -- compared to 1 per day for SOFR.

### CRPS -- Why Honest Prediction Is the Only Rational Strategy

ISFR uses the Continuous Ranked Probability Score (CRPS), a strictly proper scoring rule first rigorously characterized by Gneiting and Raftery (2007). **Strict propriety** is a mathematical property (not an assumption): the unique optimal strategy is truthful reporting of one's best estimate. Hedging, sandbagging, and strategic misreporting all produce worse expected scores.

**V1 -- Point predictions.** CRPS reduces to Mean Absolute Error:

```
CRPS(agent_i) = |predicted_i - actual|
```

Lower is better. An agent predicting ISFR = 610 bps when actual is 595 bps scores 15.

**V2 -- Distributional predictions.** Agents submit a full cumulative distribution function:

```
CRPS(F, y) = integral [F(x) - 1(x >= y)]^2 dx
```

This rewards agents who accurately quantify their uncertainty -- "688 bps plus or minus 5 bps at 90% confidence" earns a better score than "688 bps plus or minus 50 bps" even if both nail the point estimate.

### Calibration and Residual Correction

The oracle system includes an automatic residual correction mechanism that runs on every prediction resolution at zero LLM cost. It operates on two axes:

**Bias correction.** Over many resolved predictions in the same category, if residuals have a non-zero mean, there is a systematic bias correctable with arithmetic. The corrector shifts future predictions by the mean residual, clamped to prevent wild swings (maximum 30% of predicted value).

**Interval width calibration.** If prediction intervals are consistently too narrow (actuals land outside the range more than the target coverage rate), the system widens them. If too wide, it tightens them. This is conformal prediction with online residual updates -- a technique with convergence guarantees.

The corrector runs on every prediction resolution. With the attention system producing approximately 15,000 resolutions per day across a typical agent's foraging universe, the corrector gets 15,000 opportunities to adjust per day. This compounding arithmetic correction handles approximately 80% of improvable prediction error. The remaining 20% is addressed by the agent's slower dream-cycle mechanisms (structural pattern discovery, heuristic proposals).

### Epistemic Reputation Tiers

Each agent accumulates a rolling CRPS score that determines its epistemic reputation tier, with concrete economic consequences:

| CRPS Percentile | Tier | Economic Benefit |
|-----------------|------|------------------|
| Top 10% | **Oracle** | 2x InsightStore query quota; priority clearing; 0.5x risk friction |
| 10--30% | **Calibrated** | 1.5x query quota; 0.75x risk friction |
| 30--70% | **Standard** | Base access, base friction |
| 70--100% | **Uncalibrated** | 0.5x query quota; 1.25x risk friction |

The risk friction discount (gamma) is where reputation becomes economically material. In the clearing engine, gamma determines effective spread and margin requirements. Oracle-tier agents pay half the friction cost of Uncalibrated agents. This creates a direct flywheel: accurate predictions lead to higher reputation, which leads to lower trading costs, which leads to more profitable strategies, which leads to more predictions, which leads to better accuracy.

Reputation decays with a 30-day half-life. Tier is earned, not purchased.

---

## 10. How Agents Consume Oracle Data for Decision-Making

### The Three-Tier Attention System

Agents do not manually configure which markets, pools, or protocols to monitor. They discover opportunities through prediction error -- the same signal used for learning. The agent maintains a universe of tracked items organized into three tiers:

| Tier | Items | Predictions Per Check | Check Frequency |
|------|-------|----------------------|-----------------|
| **Active** | 5--15 | 10--15 (full cascade) | Every tick (~60s) |
| **Watched** | 20--50 | 2--3 (key metrics) | Every ~10 ticks (~10 min) |
| **Scanned** | 50--200 | 1 (summary metric) | Every ~60 ticks (~1 hour) |

Items flow between tiers based on prediction error. A pool whose actual fee rate exceeds the prediction for several consecutive checks earns a higher anomaly score and gets promoted. A pool that has been predictable (boring) for longer than its tier's patience window gets demoted. Current positions are never demoted while open.

### Action Gating

The oracle system gates agent actions based on prediction accuracy. Before executing a write action (trade, LP entry, lending deposit), the system checks the agent's accuracy in the relevant prediction category against a threshold that scales with the action's cost-to-value ratio:

```
required_accuracy = 0.50 + min((cost / expected_value), 0.45)
```

High cost relative to expected value requires high prediction accuracy to justify. If the agent's accuracy is below the threshold, the action is blocked. This mechanism makes patience an emergent property of the math -- an agent does not over-trade because the accuracy gate prevents it from acting until it has demonstrated sufficient understanding.

### Cross-Pool Environmental Models

As agents track many items simultaneously, they discover correlations that span pools:

> "When gas prices drop below 5 gwei on Base, Uniswap V3 ETH/USDC pool fee rates spike by ~40% within 2 hours -- because arbitrageurs start submitting more transactions."

These environmental models are learned automatically from correlated residual patterns across the tracked universe. A single model can improve prediction accuracy across dozens of pools simultaneously -- this is the "exponential moment" in an agent's learning curve.

### The Oracle as Decision Context

At every cognitive step, the agent's oracle state is injected into its decision context. The agent sees its aggregate prediction accuracy, its weakest categories, and its strongest categories. This metacognitive signal -- "which types of predictions am I worst at?" -- is computed directly from the prediction ledger's accuracy data, requiring zero LLM introspection.

The oracle also surfaces inaction predictions. Inaction is treated as a prediction: "no action will be profitable in the next N ticks." When inaction predictions are more accurate than action predictions, the accuracy gate naturally suppresses trading. The agent holds not because it was told to, but because holding has a better track record than acting.

### Self-Improving Infrastructure

The oracle system self-improves through mechanisms that calibrate automatically based on performance data:

- **V1:** Governance-assigned source confidence scores, fixed class weights, simple EMA smoothing.
- **V2:** Leave-one-out MSPE for automatic source confidence, Bates-Granger optimal combination for class weights with governance rails, Kalman filter smoothing that distinguishes measurement noise from process noise, cost-stratified trim fractions reflecting actual manipulation economics.
- **V2 yield curve:** Nelson-Siegel parameterization publishes 4 parameters characterizing the entire yield curve, enabling any consumer to reconstruct ISFR at any tenor. Extended to quantile curves (p05/p50/p95), this publishes an on-chain term structure of rate volatility without external volatility oracles.

The system transitions from human-managed parameters at launch to self-calibrating mechanisms as empirical data accumulates -- with governance retaining bounded override authority at every stage.

---

## Summary of Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Validator-computed, not operator-dependent | Eliminates separate oracle trust assumption; inherits chain security budget |
| Dual-layer median aggregation | Independent Byzantine tolerance at source and validator levels |
| Precompile publication | Fixed gas cost; as efficient as reading block number; no contract call overhead |
| CRPS scoring with strict propriety | Truthful reporting is the unique optimal strategy; system cannot be gamed |
| Source class taxonomy with bounded weights | Structural firewall against volatility contamination from any single yield source |
| Hybrid oracle + market rate | Useful from day one (oracle-driven); more useful as market grows (market-driven) |
| PredictionDomain trait abstraction | Protocol-specific knowledge isolated from engine; adding new protocols requires only trait implementation |
| BenchmarkIndex trait | Same infrastructure generalizes from interest rates to any measurable multi-source phenomenon |
| Automatic residual correction | 15,000 arithmetic adjustments per day at zero LLM cost; handles 80% of improvable error |
| Attention foraging from prediction error | Agent discovers what to monitor through the same mechanism it uses to learn; no manual configuration |

---

## References

1. Gneiting, T. & Raftery, A.E. "Strictly proper scoring rules, prediction, and estimation." *Journal of the American Statistical Association*, 102(477), 2007.
2. Bates, J.M. & Granger, C.W.J. "The Combination of Forecasts." *Operational Research Quarterly*, 20(4), 1969.
3. IOSCO. "Principles for Financial Benchmarks." International Organization of Securities Commissions, FR07/13, July 2013.
4. Bank for International Settlements. "OTC derivatives statistics at end-June 2025." BIS Statistical Bulletin, December 2025.
5. Crisostomo, R. "Estimating Real-World Probabilities: A Forward-Looking Behavioral Framework." *Journal of Futures Markets*, 41(11), 2021.
6. Kim, H. & Park, A. "Designing Funding Rates for Perpetual Futures." arXiv:2506.08573, 2025.
7. Clark, A. "Whatever next? Predictive brains, situated agents." *Behavioral and Brain Sciences*, 36(3), 2013.
8. Stephens, D.W. & Krebs, J.R. *Foraging Theory*. Princeton University Press, 1986.
