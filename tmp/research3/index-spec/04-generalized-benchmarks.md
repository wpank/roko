# Generalized Benchmark Framework

## From a Single Rate to a Suite of Indices

---

## 1. The Core Insight

Financial benchmarks are among the most powerful primitives in any economy. The Secured Overnight Financing Rate (SOFR), published by the Federal Reserve Bank of New York, underpins over $570 trillion in interest rate derivatives. The VIX underpins the volatility derivatives market. The S&P 500 underpins the index derivatives market. In every case, the pattern is the same: a credible benchmark is published, derivatives reference it, and once adoption reaches critical mass, the benchmark becomes a natural monopoly -- displacement requires regulatory force (as the LIBOR-to-SOFR transition demonstrated over a painful five-year period).

The Internet Secured Funding Rate (ISFR) was designed to fill the equivalent gap in decentralized finance (DeFi). DeFi holds approximately $49.5 billion in lending total value locked (TVL) as of April 2026, yet has zero credible benchmark rates. On-chain interest rate derivative TVL sits below $100 million -- a six-order-of-magnitude gap compared to traditional finance's $668 trillion in interest rate derivative notional.

While building the ISFR, a broader insight emerged: **the computational pattern behind ISFR is domain-agnostic.** The pipeline -- multi-source aggregation, dual-median computation, validator consensus, oracle publication, derivative instruments -- has nothing inherently specific to interest rates. Any domain where multiple independent sources produce measurable signals can support a benchmark index with identical properties.

This document specifies the generalized benchmark framework that extracts this pattern into a reusable system, and defines the five benchmark indices that compose the Nunchi Reference Index Suite (NRIS).

---

## 2. The BenchmarkIndex Trait

The generalization is captured in a Rust trait defined in the `roko-core` crate. Any benchmark index -- whether it measures DeFi funding costs, agent performance, knowledge quality, security posture, or research rigor -- must implement this trait.

### Full trait definition

```rust
/// A benchmark index computed by Korai validators at the consensus layer.
///
/// All benchmark indices share the same computational structure: multi-source
/// aggregation via weighted median, validator-level consensus via stake-weighted
/// median, and publication via precompile. The trait captures this invariant
/// structure while allowing domain-specific source definitions and update cadences.
pub trait BenchmarkIndex: Send + Sync {
    /// The set of sources contributing to this index.
    ///
    /// Each source independently measures the same underlying phenomenon.
    /// Sources are defined at index creation and can be modified via
    /// governance (add/remove sources, adjust weights within bounds).
    fn sources(&self) -> &[IndexSource];

    /// Compute the index value from a set of source readings.
    ///
    /// Each validator calls this independently during block production.
    /// The implementation MUST be deterministic given the same inputs --
    /// two validators with identical readings must produce identical outputs.
    /// The standard implementation uses weighted median aggregation.
    fn compute(&self, readings: &[SourceReading]) -> IndexValue;

    /// Compute the confidence score from validator vote distribution.
    ///
    /// Returns the fraction of stake that submitted votes within one
    /// standard deviation of the stake-weighted median. A confidence
    /// of 0.95 means 95% of validating stake agrees on the index value
    /// (within one sigma). Used to trigger circuit breaker state transitions.
    fn confidence(&self, validator_votes: &[Vote]) -> f64;

    /// The update cadence in blocks.
    ///
    /// At Korai's 400ms block time:
    /// - ISFR: 25 blocks (~10 seconds)
    /// - IAPI: 750 blocks (~5 minutes)
    /// - IKQI: 9,000 blocks (~1 hour)
    /// - ISVI: 9,000 blocks (~1 hour)
    /// - IRRI: 216,000 blocks (~24 hours)
    fn update_cadence_blocks(&self) -> u64;

    /// Precompile address where this index is published on the Korai Kernel Plane.
    ///
    /// Any smart contract on Korai can read the current index value with a
    /// single precompile call at this address. Fixed gas cost, no oracle
    /// dependency, no contract call indirection.
    fn precompile_address(&self) -> Address;

    /// Circuit breaker threshold. Below this confidence, the index enters
    /// Degraded publication state.
    ///
    /// Default is 0.70 (70% of stake must agree within one sigma).
    /// Individual indices can override this based on their domain's
    /// tolerance for uncertainty.
    fn circuit_breaker_threshold(&self) -> f64 {
        0.70
    }
}
```

### Supporting types

```rust
/// A source contributing to a benchmark index.
pub struct IndexSource {
    /// Human-readable source name (e.g., "Aave V3 USDC", "Arena Gate Pass Rate").
    pub name: String,

    /// Weight in the weighted median computation.
    /// All weights across sources must sum to 1.0.
    pub weight: f64,

    /// Maximum weight this source can have (governance constraint).
    /// Prevents any single source from dominating the index.
    pub max_weight: f64,

    /// Liveness timeout in seconds.
    /// A source is excluded from computation after this duration without updates.
    pub liveness_timeout_secs: u64,

    /// Method for reading this source (chain query, API, internal metric, etc.).
    pub reader: SourceReader,
}

/// A single reading from a source at a point in time.
pub struct SourceReading {
    /// Index into the sources array.
    pub source_idx: usize,

    /// Value in basis points (1 bps = 0.01%).
    /// All index values are denominated in basis points for precision
    /// without floating-point ambiguity.
    pub value_bps: u32,

    /// Unix timestamp of the reading.
    pub timestamp: u64,

    /// Whether the reading is within the source's liveness window.
    /// Stale readings are excluded from computation.
    pub is_live: bool,
}

/// The computed index value with metadata.
pub struct IndexValue {
    /// The index value in basis points.
    pub value_bps: u32,

    /// Number of live sources used in computation.
    /// An index computed from 4 of 4 live sources is more reliable
    /// than one computed from 2 of 4.
    pub num_sources: u32,

    /// Publication state, determined by confidence and source liveness.
    pub state: PublicationState,
}

/// Publication state model for all benchmark indices.
///
/// Four states, ordered by severity:
/// - Live: normal operation, full confidence
/// - Degraded: confidence dropped below threshold but index is still computing
/// - Stale: no new readings within the liveness window
/// - Halted: critical failure, index is not publishing
pub enum PublicationState {
    Live,
    Degraded,
    Stale,
    Halted,
}

/// A validator's vote for an index value.
pub struct Vote {
    /// The value this validator computed, in basis points.
    pub value_bps: u32,

    /// Index of this validator in the active committee.
    pub validator_index: u32,

    /// Normalized stake weight of this validator (sum of all weights = 1.0).
    pub stake_weight: f64,
}
```

### Method semantics

| Method | Input | Output | Determinism | Purpose |
|--------|-------|--------|-------------|---------|
| `sources()` | None | `&[IndexSource]` | Deterministic | Enumerate all sources contributing to the index. Governance can add/remove sources. |
| `compute()` | `&[SourceReading]` | `IndexValue` | Strictly deterministic | Each validator independently computes the index from its own source observations. Must produce identical output for identical inputs. |
| `confidence()` | `&[Vote]` | `f64` in [0.0, 1.0] | Deterministic | Measures validator agreement. Used to trigger circuit breaker (Degraded/Stale/Halted). |
| `update_cadence_blocks()` | None | `u64` | Constant | How often the index updates. Determines the publication frequency. |
| `precompile_address()` | None | `Address` | Constant | Fixed address on the Korai Kernel Plane. Contracts read the index with a single call. |
| `circuit_breaker_threshold()` | None | `f64` | Constant | Confidence below this value triggers Degraded state. Default 0.70. |

---

## 3. The Five Benchmark Indices

### 3.1 ISFR -- Internet Secured Funding Rate

**What it measures:** The cost of secured funding across decentralized finance. ISFR is to DeFi what SOFR (Secured Overnight Financing Rate) is to traditional finance: the reference rate that financial instruments settle against.

**Why it matters:** DeFi holds $49.5 billion in lending TVL with zero hedging instruments. Every lender and borrower carries unhedged variable rate exposure. A treasury earning 8% variable yield on Aave has no way to lock in that rate. If rates drop to 3%, the treasury faces a $500K annualized shortfall on a $10M position -- and there is nothing to hedge against. ISFR creates the reference point that enables hedging.

**Sources (V1):**

| Source | What it measures | Weight | Liveness timeout | Chain |
|--------|-----------------|--------|-----------------|-------|
| Aave V3 USDC supply APY | Collateralized lending yield on the largest DeFi lending protocol (~$23.5B TVL) | 0.25 | 120 seconds | Ethereum mainnet |
| Compound V3 USDC supply APY | Collateralized lending yield on the second-largest lending protocol (~$2.1B TVL) | 0.25 | 120 seconds | Ethereum mainnet |
| Ethena sUSDe 7-day rolling yield | Delta-neutral structured yield from perpetual futures funding rates (~$5.2B TVL) | 0.25 | 86,400 seconds (24h) | Ethereum mainnet |
| ETH Beacon Chain staking yield | Proof-of-stake consensus rewards + MEV tips (~$115B staked) | 0.25 | 1,800 seconds (30 min) | Consensus layer |

V1 uses equal weights (0.25 per source). This is a deliberate maximum-entropy prior: the correct weights require years of empirical data, and equal weighting assumes the least about relative importance while ensuring no single source can move the median (an attacker must corrupt 50%+ of source weight).

**V2 source expansion** adds Morpho ($10B+ TVL), Hyperliquid perpetual funding rate, and cross-chain sources (Aave on Base, Compound on Arbitrum). V2 introduces governance-adjustable weights with constraints: maximum 0.35 per source, minimum 0.05 per source, 7-day timelock on weight changes.

**Computation:** Two-level aggregation. Level 1 (intra-class): within each yield class (Lending, Structured, Funding, Staking), sources are aggregated via TVL-weighted median with confidence modulation. Level 2 (inter-class): the four class rates are combined via weighted sum (Lending 0.60, Structured 0.25, Funding 0.10, Staking 0.05). The dual-median design creates a firewall -- if the funding rate spikes to 200% during a speculative mania, it contributes at most 0.10 x 200% = 20 percentage points, while the other three classes anchor the rate.

**Update frequency:** Every 25 blocks (~10 seconds at Korai's 400ms block time). This is 8,640 updates per day versus SOFR's single daily publication.

**Precompile address:** `0xA01` on the Korai Kernel Plane.

**Precompile interface:**

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
    uint32 valueBps;        // Index value in basis points
    uint64 blockHeight;     // Block at which this snapshot was computed
    uint64 timestamp;       // Unix timestamp
    uint8  state;           // 0=Live, 1=Degraded, 2=Stale, 3=Halted
    uint16 confidenceBps;   // Validator confidence (0-10000)
    uint32 numSources;      // Number of live sources used
    uint32 numValidatorVotes; // Number of validators who submitted votes
}
```

**Derivative instruments:** Yield perpetuals -- non-expiring swap contracts that settle against ISFR every 10 seconds. Unlike Pendle's expiring PT/YT tokens, yield perpetuals never expire, eliminating liquidity fragmentation and rollover costs.

---

### 3.2 IAPI -- Internet Agent Performance Index

**What it measures:** Aggregate task success rates for autonomous agents operating across competitive arenas. IAPI is the benchmark for agent capability -- a single number that captures how well autonomous agents perform their assigned work.

**Sources:**

| Source | What it measures | Weight | Liveness timeout |
|--------|-----------------|--------|-----------------|
| Arena results | Win/loss/draw outcomes across competitive evaluation arenas | 0.30 | 600 seconds |
| Gate pass rates | Percentage of tasks that pass automated validation gates (compile, test, clippy, diff review) | 0.30 | 600 seconds |
| Task completion metrics | End-to-end task completion rates across all active plans | 0.25 | 600 seconds |
| CRPS prediction scores | Calibration quality of agent predictions scored via Continuous Ranked Probability Score | 0.15 | 600 seconds |

**Update frequency:** Every 750 blocks (~5 minutes).

**Precompile address:** `0xA06`

**Why it matters:** Agent performance today is measured in isolated, protocol-specific ways. There is no cross-system benchmark for "how good are agents at completing tasks." IAPI aggregates performance signals from multiple independent evaluation systems into a single number that can serve as the basis for: agent reputation scoring, model routing decisions (which LLM backend to use for which task), derivative markets on agent capability trends, and insurance pricing for agent-executed work.

---

### 3.3 IKQI -- Internet Knowledge Quality Index

**What it measures:** The accuracy, utility, and reliability of entries in persistent knowledge stores. IKQI is the benchmark for epistemic quality -- how trustworthy is the knowledge that agents produce and consume.

**Sources:**

| Source | What it measures | Weight | Liveness timeout |
|--------|-----------------|--------|-----------------|
| Confirmation rates | Percentage of knowledge entries that are later confirmed by independent observation | 0.30 | 3,600 seconds |
| Usage frequency | How often knowledge entries are queried and used in downstream agent decisions | 0.25 | 3,600 seconds |
| CRPS scores | Prediction accuracy of knowledge-derived forecasts scored against realized outcomes | 0.25 | 3,600 seconds |
| Peer validation counts | Number of independent agents that corroborate a knowledge entry | 0.20 | 3,600 seconds |

**Update frequency:** Every 9,000 blocks (~1 hour).

**Precompile address:** `0xA07`

**Why it matters:** Knowledge stores are only useful if their contents are accurate. Without a benchmark for knowledge quality, there is no systematic way to distinguish high-quality knowledge producers from low-quality ones. IKQI provides the scoring function that underpins epistemic reputation: agents whose contributed knowledge entries consistently score well earn higher reputation tiers, which grant them more query quota, clearing priority, and system trust.

---

### 3.4 ISVI -- Internet Security Vulnerability Index

**What it measures:** Security detection rates across codebases, contracts, and systems. ISVI is the benchmark for security posture -- how effectively automated security analysis identifies vulnerabilities before they are exploited.

**Sources:**

| Source | What it measures | Weight | Liveness timeout |
|--------|-----------------|--------|-----------------|
| Audit outcomes | Results from formal smart contract and code audits (findings vs. clean reports) | 0.30 | 3,600 seconds |
| Bug bounty results | Vulnerability discovery rates through bug bounty programs | 0.25 | 3,600 seconds |
| Formal verification scores | Coverage and completeness of formal verification applied to critical code paths | 0.25 | 3,600 seconds |
| Automated scanner detection rates | True positive rates of automated security scanning tools across known vulnerability corpora | 0.20 | 3,600 seconds |

**Update frequency:** Every 9,000 blocks (~1 hour).

**Precompile address:** `0xA08`

**Why it matters:** Security is currently assessed on a per-audit, per-protocol basis with no cross-system comparability. ISVI provides a continuous, aggregate measure of security effectiveness that can serve as: a signal for smart contract insurance pricing, a routing input for security-sensitive agent tasks, and a basis for derivative instruments on aggregate security quality (e.g., hedging against systemic vulnerability discovery).

---

### 3.5 IRRI -- Internet Research Rigor Index

**What it measures:** The quality and reliability of research outputs produced by autonomous research agents. IRRI is the benchmark for research integrity -- how well research conclusions hold up under scrutiny.

**Sources:**

| Source | What it measures | Weight | Liveness timeout |
|--------|-----------------|--------|-----------------|
| Citation rates | How frequently research outputs are cited by other agents and systems | 0.25 | 86,400 seconds |
| Replication success | Percentage of research conclusions that are independently replicated | 0.30 | 86,400 seconds |
| Peer validation | Endorsement rates from independent agent reviewers | 0.25 | 86,400 seconds |
| Prediction accuracy | How well research-derived predictions match realized outcomes | 0.20 | 86,400 seconds |

**Update frequency:** Every 216,000 blocks (~24 hours).

**Precompile address:** `0xA09`

**Why it matters:** Autonomous research agents produce analysis, forecasts, and recommendations at scale. Without a benchmark for research quality, consumers of research output have no way to systematically evaluate its reliability. IRRI provides the scoring function that enables: research agent reputation tiers, quality-weighted research aggregation, and derivative markets on research output quality trends.

---

## 4. The Universal Pattern

All five indices follow an identical five-stage pipeline. The framework is domain-agnostic; only the source definitions and update cadences change.

### Stage 1: Multi-source aggregation

Multiple independent sources are defined for each index. Each source measures the same underlying phenomenon from a different vantage point. Sources are weighted (equal in V1, governance-adjustable in V2). The weighted median across sources produces a single value per validator.

The weighted median is specifically chosen over the weighted mean because it resists manipulation. With a weighted median, an attacker must corrupt sources representing more than 50% of total weight to move the output to an arbitrary value. With a weighted mean, a single extreme outlier can skew the result.

### Stage 2: Dual-median computation

Two layers of median-based aggregation provide Byzantine fault tolerance at both the source level and the validator level:

- **Layer 1 (source aggregation):** Each validator independently reads from all sources and computes a weighted median across source readings. This tolerates up to 49% of source weight being compromised.

- **Layer 2 (validator aggregation):** The chain computes a stake-weighted median across all validator submissions. This tolerates up to 49% of validator stake being compromised.

Both layers must be simultaneously compromised to move the index to an arbitrary value. The defense compounds: corrupting 49% of source weight AND 49% of validator stake is a qualitatively harder attack than corrupting either one alone.

### Stage 3: Validator consensus

Every validator on the Korai blockchain independently computes the index value as part of block production. There is no separate oracle operator, no off-chain infrastructure, no bridge dependency, no multisig. The index is a property of the chain itself -- as available as the chain's consensus.

Validators whose votes fall within one standard deviation of the stake-weighted median receive oracle mining rewards proportional to their stake. Outlier votes (more than 2 sigma from the median) receive reduced rewards. This creates an incentive for accurate computation without requiring trust in any specific validator.

### Stage 4: Oracle publication

The computed index value is published via a dedicated precompile on the Korai Kernel Plane. Any smart contract on Korai can read the current index value with a single precompile call at a fixed gas cost. This is not a contract call with variable gas and external dependency -- it is a protocol-level primitive, equivalent to reading the block timestamp or chain ID.

Each index has its own precompile address:

| Index | Precompile Address |
|-------|--------------------|
| ISFR  | `0xA01` |
| IAPI  | `0xA06` |
| IKQI  | `0xA07` |
| ISVI  | `0xA08` |
| IRRI  | `0xA09` |

### Stage 5: Prediction scoring and derivative instruments

Each index update is simultaneously an epistemic calibration event. Before each update, agents commit probability distributions (predictions) for the next index value. After publication, predictions are scored using the Continuous Ranked Probability Score (CRPS), a strictly proper scoring rule that rewards calibrated uncertainty estimates and penalizes both overconfidence and underconfidence.

Prediction accuracy feeds the epistemic reputation system: agents with consistently accurate predictions earn higher reputation tiers, which grant increased query quotas, clearing priority, and system trust. This creates a direct incentive for agents to develop accurate models of the phenomena each index measures.

Each index can also serve as the settlement layer for derivative instruments. ISFR settles yield perpetuals. The remaining indices can settle analogous instruments: performance futures, knowledge quality swaps, security insurance, and research quality derivatives.

---

## 5. Why Generalize -- The Strategic Thesis

The generalization is not speculative architecture. It is a strategic bet on a specific thesis:

> **The first credible benchmark in any domain captures the derivative market for that domain.**

This thesis is supported by every major benchmark in financial history:

- **SOFR** captured interest rate derivatives (now $665.8 trillion in notional outstanding)
- **The VIX** captured volatility derivatives
- **The S&P 500** captured index derivatives
- **LIBOR** held its monopoly for 30 years despite known structural deficiencies, because switching costs exceeded design flaws

The pattern: once a benchmark achieves critical adoption, network effects make displacement prohibitively expensive. Every derivative contract, every hedging strategy, every institutional framework that references the benchmark raises the cost of switching to an alternative. This is why benchmarks are natural monopolies and why the first credible entrant in an unoccupied domain has an asymmetric advantage.

If ISFR establishes itself as the DeFi rate benchmark, the same infrastructure -- validators, precompiles, dual-median aggregation, prediction scoring, clearing -- can be reused to launch benchmark indices in adjacent domains. Each new index is a new derivative market. Each new derivative market generates clearing volume. Each clearing round produces knowledge. The marginal cost of adding a benchmark index is low; the marginal value is an entire market.

This is the economic engine behind the generalized benchmark framework. Building one index (ISFR) is building infrastructure. Building a framework that launches indices is building a platform.

---

## 6. NRIS -- The Nunchi Reference Index Suite

The Nunchi Reference Index Suite (NRIS) is the umbrella framework encompassing all benchmark indices computed by Korai validators. ISFR is the first and flagship index, but NRIS is designed from the start to accommodate domain expansion.

### Architecture

NRIS inherits its architecture from ISFR's design:

1. **Validator-computed:** All indices are computed by the same validator set that produces Korai blocks. No separate oracle networks, no external operator dependencies.

2. **Precompile-published:** All indices are accessible via fixed-address precompiles on the Kernel Plane, with fixed gas cost and no contract indirection.

3. **Prediction-scored:** Every index update is a prediction target. Agent predictions are committed before each update and scored after, feeding the unified epistemic reputation system.

4. **Circuit-breaker protected:** All indices share the four-state publication model (Live, Degraded, Stale, Halted) with confidence-based triggers.

5. **Governance-extensible:** New sources can be added to existing indices, and new indices can be added to the suite, via governance proposals with timelock constraints.

### NRIS and the Korai value stack

ISFR is not merely an index within NRIS. It is the economic primitive that activates Korai's entire value stack:

- **Yield perpetuals** settle against ISFR, creating the first perpetual interest rate hedge in DeFi.
- **The prediction loop** uses ISFR as its canonical target, creating the densest scoring feedback loop in any on-chain prediction system (8,640 scoring events per day).
- **The knowledge store** ingests ISFR observations as structured knowledge, enabling agent-to-agent intelligence transfer about rate dynamics.
- **Epistemic reputation** is computed from ISFR prediction accuracy, creating an on-chain meritocracy of forecasting ability.
- **The clearing engine** uses ISFR as its mark price, with reputation-modulated parameters creating direct economic incentives for prediction quality.

Every component reinforces every other component. ISFR is the keystone of NRIS, and NRIS is the oracle infrastructure of Korai.

---

## 7. Cross-Domain Applications

A key property of benchmark indices is that they are useful outside their own domain. Non-domain agents consume indices as contextual signals, enriching their own decision-making without being specialists in the index's subject matter.

### Research agents consuming ISFR

Research agents use ISFR as a macroeconomic signal. A rising ISFR (DeFi funding costs increasing) correlates with leverage expansion, speculative demand, and potential market stress. A falling ISFR correlates with risk-off sentiment, capital outflows, or protocol de-risking.

Research agents subscribe to ISFR updates at a filtered cadence (hourly or daily summary) and use rate regime transitions as triggers for research topic generation:

- ISFR regime transition to "Volatile" triggers research on leverage cycles or funding rate dynamics.
- ISFR crossing historical percentile thresholds (e.g., above the 95th percentile of its 30-day range) triggers deep-dive analysis.
- Sustained divergence between ISFR sources (e.g., Aave rate diverging from Compound rate) triggers protocol-specific research.

### Coding agents consuming ISFR

Agents writing DeFi protocol code benefit from live ISFR data as development context:

- **Realistic test parameters.** Instead of hardcoded 5% yield in tests, agents reference actual current rates from ISFR.
- **Rate range calibration.** Agents know the observed min/max/mean ISFR over the past 30 days, producing code that handles realistic rate ranges rather than arbitrary constants.
- **Protocol comparison context.** The per-source breakdown (Aave rate vs. Compound rate vs. Ethena rate) helps agents understand rate differentials when writing cross-protocol logic.

### Security agents consuming ISFR

ISFR manipulation detection is itself a security monitoring signal. Sudden, large ISFR moves that do not correlate with market conditions suggest attempted manipulation of source rates. Security agents monitor for:

- **Flash loan signatures.** A source rate spike that reverts within one block is consistent with flash loan manipulation.
- **Source divergence anomalies.** If one source moves 200+ basis points while all others remain stable, the divergent source may be under governance attack or experiencing a bug.
- **Confidence score drops.** A sudden drop in validator confidence (from 90%+ to below 70%) indicates validator disagreement, which may signal a consensus-layer attack.

### The generalized cross-domain pattern

The cross-domain utility of ISFR demonstrates why the `BenchmarkIndex` trait is domain-agnostic. Any benchmark index follows the same event flow:

```
BenchmarkUpdate event → EventFabric → agent subscription → contextual signal
```

- **Domain agents** consume the index as an operational input (e.g., blockchain agents use ISFR for hedging decisions).
- **Non-domain agents** consume the index as a contextual signal (e.g., research agents use ISFR as a macro indicator).

The same infrastructure serves both use cases because the event delivery mechanism (`BenchmarkUpdate` propagated through the event bus to subscribed agents) is identical regardless of what the index measures. An IAPI update flows to security agents as a "how are agents performing?" signal. An ISVI update flows to coding agents as a "what vulnerability classes are trending?" signal. The framework does not need to know the semantics; it provides the delivery.

---

## 8. Implementation in Rust

The existing implementation in `roko-chain` provides the foundation. The `IsfrRegistry` already implements weighted median aggregation with 3-sigma outlier exclusion:

```rust
/// The ISFR registry: collects submissions and produces aggregates via weighted median.
pub struct IsfrRegistry {
    pub config: IsfrConfig,
    submissions: HashMap<(u64, MarketId), Vec<IsfrSubmission>>,
    current_epoch: u64,
    reputation_scores: HashMap<u256, f64>,
    quarantined: Vec<u256>,
    custom_markets: Vec<MarketId>,
}
```

The weighted median algorithm -- the mathematical core of every benchmark index -- is implemented as:

```rust
/// Compute the weighted median of a set of (value, weight) pairs.
///
/// The weighted median is the value where the cumulative weight from below
/// equals the cumulative weight from above. This is more robust than
/// weighted mean because it resists outlier influence.
fn weighted_median(entries: &[(f64, f64)]) -> f64 {
    if entries.is_empty() {
        return 0.0;
    }
    if entries.len() == 1 {
        return entries[0].0;
    }

    let mut sorted: Vec<(f64, f64)> = entries.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let total_weight: f64 = sorted.iter().map(|(_, w)| w).sum();
    let half_weight = total_weight / 2.0;

    let mut cumulative = 0.0;
    for (i, &(value, weight)) in sorted.iter().enumerate() {
        cumulative += weight;
        if cumulative >= half_weight {
            if (cumulative - half_weight).abs() < 1e-12 && i + 1 < sorted.len() {
                return (value + sorted[i + 1].0) / 2.0;
            }
            return value;
        }
    }

    sorted.last().unwrap().0
}
```

The clearing cycle state machine manages the six-phase clearing process (Commit, Reveal, Solve, Certificate, Verify, Settle) that applies to all benchmark indices:

```rust
pub enum ClearingPhase {
    Commit,      // Agents submit sealed rate commitments
    Reveal,      // Agents reveal their rates
    Solve,       // Clearing engine runs weighted median + outlier exclusion
    Certificate, // Clearing certificate with KKT proof is generated
    Verify,      // On-chain verification of the certificate
    Settle,      // Final settlement: rates published, rewards distributed
}
```

---

## 9. Implementation Phases and Current Status

### Phase 7: Generalized benchmark index (framework extraction)

| Task | Description | Status |
|------|-------------|--------|
| 7.1 | Define `BenchmarkIndex` trait in `roko-core` with `IndexSource`, `SourceReading`, `IndexValue`, `PublicationState`, and `Vote` types | Specified |
| 7.2 | Implement `BenchmarkIndex` for `ISFROracle` (the existing ISFR oracle refactored to implement the generic trait) | Specified |
| 7.3 | Stub IAPI and IKQI indices to prove trait generalization (mock sources, real confidence/circuit-breaker logic) | Specified |

**Trait placement:** The `BenchmarkIndex` trait lives in `roko-core` (the kernel crate) so that any crate in the workspace can implement it. Concrete indices live in `roko-chain`.

**Validation criteria for Task 7.3:** A `Vec<Box<dyn BenchmarkIndex>>` must be able to hold ISFR, IAPI, and IKQI simultaneously, and all three must pass the same confidence and circuit-breaker logic. This proves the trait is genuinely generic, not just a refactoring of ISFR-specific code.

### Phase 8: Runtime integration

| Task | Description | Status |
|------|-------------|--------|
| 8.1 | Wire `BenchmarkUpdate` events into the runtime event bus (`RokoEvent` enum) | Specified |
| 8.2 | Wire ISFR predictions into the foraging model (large deltas boost blockchain entity attention; publication state transitions modulate urgency) | Specified |
| 8.3 | Wire clearing insights into the world graph (market entity with clearing price, surplus, imbalance; stress events for extreme imbalance) | Specified |
| 8.4 | Integration test: end-to-end ISFR event triggers blockchain agent attention boost and world graph update | Specified |

**Event structure:**

```rust
pub enum RokoEvent {
    // ... existing variants ...

    /// A BenchmarkIndex (e.g., ISFR) published a new value.
    BenchmarkUpdate {
        index_name: String,       // "ISFR", "IAPI", "IKQI", etc.
        value: IndexValue,        // Current value with metadata
        previous_value: Option<IndexValue>,  // For delta computation
        delta_bps: i32,           // Signed change in basis points
    },
}
```

**Foraging model integration:** When a `BenchmarkUpdate` arrives, the system modulates agent attention allocation:

- ISFR delta > 50 bps: boost blockchain entity Gittins indices (increase monitoring frequency)
- ISFR enters Degraded or Stale: apply 2x urgency multiplier to blockchain domain
- ISFR enters Halted: apply 5x urgency multiplier (critical monitoring)
- ISFR returns to Live: reset urgency to 1.0

### ISFR credibility timeline

| Phase | Timeline | Activities |
|-------|----------|------------|
| Phase 1: Curated Aggregation | Q3 2026 | Launch with 4 V1 sources; governance-assigned weights; agents as anchor consumers |
| Phase 2: Track Record | Q4 2026 | Uninterrupted publication; source expansion with 30-day probation per new source; V2 self-calibration activates |
| Phase 3: Reflexive Loop | Q1-Q2 2027 | ISFR-settled derivatives grow; external institutional evaluation; IOSCO alignment review |
| V3: Cross-chain | Q3 2027+ | Solana lending rates, L2 sources via same probation framework |
| V4: TradFi Bridges | 2028+ | SOFR on-chain, UST-3M, DeFi-to-TradFi basis instruments |

---

## 10. The Strategic Thesis Behind Benchmark Generalization

The benchmark framework is the expression of a three-part strategic thesis.

### Part 1: Infrastructure compounding

Building the first benchmark index (ISFR) requires substantial infrastructure: validator computation, precompile publication, dual-median aggregation, prediction scoring, clearing engines, circuit breakers, and governance mechanisms. Building the second benchmark index requires almost none of that -- just new source definitions and an update cadence. The marginal cost of each additional index approaches zero while the marginal value (a new derivative market) remains high.

This is infrastructure compounding. The framework amortizes the cost of building benchmark infrastructure across an unlimited number of indices. Korai's position as the platform that publishes credible benchmark indices becomes more defensible with each new index launched.

### Part 2: The benchmark flywheel

Benchmark rates create a self-reinforcing cycle:

1. A credible rate is published.
2. Derivatives reference it and enable hedging.
3. Hedging instruments attract institutional capital (institutions require hedging to participate).
4. Institutional capital deepens liquidity and trading volume.
5. Deeper liquidity makes the benchmark more credible.
6. Greater credibility attracts more derivatives.

Once this flywheel reaches critical mass, displacement becomes prohibitively expensive. LIBOR survived 30 years of known deficiencies because switching costs outweighed design flaws. SOFR displaced LIBOR only through regulatory force (the LIBOR Act of 2022 and coordinated multi-year industry effort).

Each index in NRIS has its own flywheel. And the flywheels reinforce each other: an agent economy with credible benchmarks for funding costs, agent performance, knowledge quality, security posture, and research rigor is a more complete and more defensible platform than one with a single benchmark.

### Part 3: Knowledge production as a byproduct

Every benchmark update is a knowledge production event. Each update generates:

- A new observation (the index value itself)
- Prediction scores (CRPS evaluations of all agent predictions)
- Reputation updates (epistemic tier adjustments based on prediction accuracy)
- Insights (structured knowledge entries that other agents can query)

This means the benchmark framework is not just a price oracle -- it is a continuous knowledge factory. The density of the knowledge production is proportional to the update frequency: ISFR produces 8,640 knowledge events per day, IAPI produces 288, and even the daily IRRI produces a calibration event every 24 hours.

Over time, the accumulated knowledge from benchmark operations becomes itself a competitive advantage -- a dataset of prediction accuracy, model calibration, and agent performance that no competitor can replicate without operating the same infrastructure for the same duration.

---

## Summary of Constants and Addresses

| Index | Full Name | Precompile | Update Cadence | Blocks | Domain |
|-------|-----------|------------|---------------|--------|--------|
| ISFR | Internet Secured Funding Rate | `0xA01` | ~10 seconds | 25 | DeFi funding cost |
| IAPI | Internet Agent Performance Index | `0xA06` | ~5 minutes | 750 | Agent task success |
| IKQI | Internet Knowledge Quality Index | `0xA07` | ~1 hour | 9,000 | Knowledge accuracy |
| ISVI | Internet Security Vulnerability Index | `0xA08` | ~1 hour | 9,000 | Security detection |
| IRRI | Internet Research Rigor Index | `0xA09` | ~24 hours | 216,000 | Research quality |

All block counts assume Korai's 400ms block time. All indices share the same `BenchmarkIndex` trait, the same dual-median aggregation pipeline, the same prediction scoring via CRPS, the same four-state circuit breaker model, and the same precompile publication mechanism. The framework is one pattern instantiated five times.
