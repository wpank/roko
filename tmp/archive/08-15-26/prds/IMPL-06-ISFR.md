# IMPL-06: ISFR and instruments

**Implements:** PRD-07 (ISFR and instruments)
**Status:** Draft
**Date:** 2026-04-21
**Estimated effort:** 12-16 weeks across 7 phases

---

## Context

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/` with 18 crates. Korai is the companion blockchain. This document specifies every task required to implement the Internet Secured Funding Rate (ISFR) oracle, yield perpetual contracts, clearing profiles, cooperative clearing, and the generalized benchmark framework.

The ISFR is a composite benchmark index -- DeFi's equivalent of SOFR. It aggregates rates from Aave V3, Compound V3, Ethena sUSDe, and ETH Beacon Chain staking into a single manipulation-resistant number via dual-median aggregation. Yield perpetuals settle against this rate. Cooperative clearing uses KKT-verified batch auctions for provably optimal settlement.

### Workspace layout

| Crate | Path | Role in ISFR |
|-------|------|-------------|
| `roko-chain` | `crates/roko-chain/` | ISFR oracle, clearing engine, yield perps, precompile stubs |
| `roko-core` | `crates/roko-core/` | `BenchmarkIndex` trait, shared types |
| `roko-agent` | `crates/roko-agent/` | Prediction commitment integration |
| `roko-learn` | `crates/roko-learn/` | CRPS scoring, epistemic reputation |
| `roko-gate` | `crates/roko-gate/` | Gate integration for clearing verification |
| `roko-cli` | `crates/roko-cli/` | CLI commands for ISFR queries |

### What already exists

The `roko-chain` crate contains substantial ISFR infrastructure. Before writing new code, read these files first:

- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs` -- `IsfrConfig`, `IsfrRegistry`, `ClearingPhase` state machine (Commit/Reveal/Solve/Certificate/Verify/Settle), `ClearingCycleState`, `PhaseAllocations`, weighted median, outlier exclusion, QP clearing solver, `ClearingCertificate`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/futures_market.rs` -- `FuturesMarket`, `KnowledgeFuture` lifecycle (Open/Committed/Submitted/Fulfilled/Expired), demand pools
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/korai_token.rs` -- `KoraiToken` with lazy demurrage, `BalanceRecord`, 5 earning pathways, 5 spending mechanisms
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/agent_registry.rs` -- ERC-8004 soulbound passports, 10 capability bits, prompt hash timelock
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/reputation_registry.rs` -- 7-domain EMA scoring, 4 discipline states, 30-day half-life decay
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` -- Phase 2 chain stubs, type aliases (`u256`, `B256`, `Address`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs` -- All public exports, module structure

**Critical:** Read the existing code before implementing. The crate already has a weighted median, outlier exclusion, and a QP solver. Extend what exists rather than duplicating.

---

## Phase 1: ISFR oracle implementation

Goal: compute a manipulation-resistant composite rate from 4 DeFi sources using dual-median aggregation.

### Task 1.1: Define `ISFRSource` trait

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs` (full file -- understand `IsfrConfig`, existing aggregation)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` (type aliases)

**File to create/modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs`

**What to implement:**

```rust
/// A rate source contributing to the ISFR composite.
pub trait ISFRSource: Send + Sync {
    /// Human-readable name (e.g., "Aave V3 USDC").
    fn name(&self) -> &str;

    /// Fetch the current annualized rate in basis points.
    /// Returns None if the source is unreachable or stale.
    async fn fetch_rate(&self) -> Option<u32>;

    /// Weight in the weighted median (0.0 to 1.0).
    fn weight(&self) -> f64;

    /// Liveness timeout in seconds.
    fn liveness_timeout_secs(&self) -> u64;
}
```

**Checklist:**
- [ ] Define `ISFRSource` trait in `isfr.rs`
- [ ] Add `SourceReading` struct (source_name, value_bps, timestamp, is_live)
- [ ] Add `SourceStatus` enum (Live, Stale, Offline)
- [ ] Add `SourceMetadata` struct (name, weight, liveness_timeout, last_reading, status)
- [ ] Ensure the trait is object-safe (`dyn ISFRSource`)
- [ ] Export from `lib.rs`
- [ ] Unit test: mock source returns correct rate and weight

**Test:** `cargo test -p roko-chain -- isfr_source`

---

### Task 1.2: Implement Aave V3 source

**Read first:**
- Task 1.1 output (`ISFRSource` trait)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs` (ChainClient trait for on-chain reads)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/alloy_impl.rs` (Alloy backend)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr_sources/aave_v3.rs`

**What to implement:**

Read the Aave V3 Pool contract's `getReserveData(address asset)` for USDC (0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 on mainnet). Extract the `currentLiquidityRate` field (ray-encoded, 27 decimals). Convert to annualized basis points.

**Checklist:**
- [ ] Create `crates/roko-chain/src/isfr_sources/` module directory
- [ ] Create `crates/roko-chain/src/isfr_sources/mod.rs` with submodule declarations
- [ ] Implement `AaveV3Source` struct with Aave V3 Pool contract address
- [ ] Implement `ISFRSource` for `AaveV3Source`
- [ ] Convert ray (1e27) to basis points: `rate_bps = (currentLiquidityRate / 1e23) as u32`
- [ ] Handle connection failure (return None)
- [ ] Set weight to 0.25 (equal weighting per PRD-07 section 3)
- [ ] Set liveness timeout to 120 seconds (10 missed Ethereum blocks)
- [ ] Unit test with mock chain client returning known rate
- [ ] Integration test (feature-gated `#[cfg(feature = "integration")]`) against a fork

**Test:** `cargo test -p roko-chain -- aave_v3_source`

---

### Task 1.3: Implement Compound V3 source

**Read first:** Task 1.2 (same pattern)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr_sources/compound_v3.rs`

**What to implement:**

Read Compound V3 Comet contract's `getSupplyRate(uint utilization)` for the USDC market. Convert per-second rate to annualized basis points: `annual_bps = (per_second_rate * 31557600) / 1e14`.

**Checklist:**
- [ ] Implement `CompoundV3Source` struct with Comet contract address
- [ ] Implement `ISFRSource` for `CompoundV3Source`
- [ ] Read `getUtilization()` then `getSupplyRate(utilization)`
- [ ] Convert per-second rate to annualized bps
- [ ] Weight: 0.25, liveness timeout: 120s
- [ ] Unit test with mock chain client
- [ ] Handle Compound-specific error codes

**Test:** `cargo test -p roko-chain -- compound_v3_source`

---

### Task 1.4: Implement Ethena sUSDe source

**Read first:** Task 1.2 (same pattern)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr_sources/ethena.rs`

**What to implement:**

Read sUSDe contract's share price history to compute a 7-day rolling yield. The sUSDe token is a vault token -- its share price increases as yield accrues. Compute: `yield_7d = (price_now / price_7d_ago - 1) * (365.25 / 7) * 10000` to get annualized basis points.

**Checklist:**
- [ ] Implement `EthenaSUSDESource` struct with sUSDe vault contract address
- [ ] Implement `ISFRSource` for `EthenaSUSDESource`
- [ ] Store a ring buffer of daily share prices (7 entries)
- [ ] Compute 7-day rolling annualized yield
- [ ] Weight: 0.25, liveness timeout: 86400s (24 hours -- Ethena updates less frequently)
- [ ] Handle edge case: insufficient history (fewer than 7 days of data)
- [ ] Unit test with mock prices showing known yield

**Test:** `cargo test -p roko-chain -- ethena_source`

---

### Task 1.5: Implement ETH Beacon Chain source

**Read first:** Task 1.2 (same pattern)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr_sources/eth_beacon.rs`

**What to implement:**

Compute ETH staking yield from consensus rewards + execution layer tips. Read from the Beacon Chain API (or use an on-chain oracle that publishes staking APR). Convert to annualized basis points.

Two approaches:
1. If a Beacon Chain API endpoint is available, read `/eth/v1/beacon/states/head/validators` and compute rewards per epoch.
2. If using an on-chain oracle (e.g., Lido's stETH/ETH rate), compute the effective staking yield from the rebase rate.

**Checklist:**
- [ ] Implement `EthBeaconSource` struct
- [ ] Implement `ISFRSource` for `EthBeaconSource`
- [ ] Support both Beacon API mode and on-chain oracle mode (configurable)
- [ ] Compute annualized consensus + MEV yield in basis points
- [ ] Weight: 0.25, liveness timeout: 1800s (30 minutes, 5 missed epochs)
- [ ] Unit test with mock beacon data
- [ ] Handle beacon API connection failure gracefully

**Test:** `cargo test -p roko-chain -- eth_beacon_source`

---

### Task 1.6: Implement dual-median aggregation

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs` -- the existing `IsfrRegistry` already has a weighted median. Determine if it matches the PRD-07 spec (dual-level: source-weighted median, then stake-weighted validator median).

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs`

**What to implement:**

Two-layer aggregation:
1. **Layer 1 (source aggregation):** Each validator sorts source readings by value, accumulates weights, finds the 0.50 percentile. If the boundary splits two values, take the arithmetic mean.
2. **Layer 2 (validator aggregation):** Sort validator-submitted values by rate, weight by stake, find the stake-weighted 0.50 percentile.

**Checklist:**
- [ ] Review existing weighted median in `IsfrRegistry` -- determine if it needs replacement or extension
- [ ] Implement `compute_source_median(readings: &[SourceReading]) -> Option<u32>` -- weighted median of source rates
- [ ] Implement `compute_validator_median(votes: &[OracleVote]) -> Option<u32>` -- stake-weighted median of validator submissions
- [ ] Implement `OracleVote` struct: `{ value_bps: u32, block_height: u64, validator_index: u32, stake_weight: f64 }`
- [ ] Handle edge cases: odd vs even number of sources at the median boundary
- [ ] Handle missing sources (exclude from computation, reduce source count)
- [ ] Wire: `ISFROracle::compute(sources, votes) -> ISFRSnapshot`
- [ ] Unit test: 4 sources, known weights, verify median matches hand-calculated value from PRD-07 section 4 (4.50%, 5.80%, 6.20%, 7.10% -> ISFR = 6.00%)
- [ ] Unit test: 3 validators with different stakes, verify stake-weighted median

**Test:** `cargo test -p roko-chain -- dual_median`

---

### Task 1.7: Implement confidence score

**Read first:** PRD-07 section 5

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs`

**What to implement:**

```
confidence = sum(s_i for all i where |v_i - median| <= sigma) / sum(all s_i)
```

Where sigma is the stake-weighted standard deviation of all validator votes.

**Checklist:**
- [ ] Implement `compute_confidence(votes: &[OracleVote], median_bps: u32) -> f64`
- [ ] Compute stake-weighted standard deviation
- [ ] Sum stake of all validators within 1 sigma of the median
- [ ] Return as fraction (0.0 to 1.0)
- [ ] Handle degenerate case: all validators submit the same value (confidence = 1.0, sigma = 0)
- [ ] Unit test: 5 validators, 4 agree, 1 outlier -- verify confidence reflects the 4 agreeing validators' stake

**Test:** `cargo test -p roko-chain -- isfr_confidence`

---

### Task 1.8: Implement circuit breaker state machine

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs` -- existing `ClearingPhase` state machine for reference

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs`

**What to implement:**

Four states: Live, Degraded, Stale, Halted. Transitions based on source count and confidence:
- Live: 3+ sources AND confidence >= 0.70
- Degraded: exactly 2 sources OR confidence 0.50-0.70
- Stale: exactly 1 source
- Halted: 0 sources OR confidence < 0.50

Recovery hysteresis: confidence must exceed 0.80 for 3 consecutive updates before transitioning from Degraded/Halted back to Live.

**Checklist:**
- [ ] Define `ISFRState` enum: `Live`, `Degraded`, `Stale`, `Halted`
- [ ] Implement `ISFRCircuitBreaker` struct with state, last_live_value, recovery_count
- [ ] Implement `transition(num_sources: u32, confidence: f64) -> ISFRState`
- [ ] Implement hysteresis: track consecutive updates above 0.80 for recovery
- [ ] Cache the last Live ISFR value as fallback for Stale/Halted states
- [ ] Emit `ISFRCircuitBreakerEvent` on state transitions
- [ ] Unit test: walk through Live -> Degraded -> Halted -> recovery back to Live
- [ ] Unit test: verify hysteresis prevents oscillation (confidence bouncing around 0.70)

**Test:** `cargo test -p roko-chain -- isfr_circuit_breaker`

---

### Task 1.9: Flash loan resistance test

**Read first:** Tasks 1.6, 1.7

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/isfr_manipulation.rs`

**What to implement:**

Reproduce the flash loan attack scenario from PRD-07 section 4:
- Normal: Aave 4.50%, Compound 5.80%, Ethena 6.20%, Beacon 7.10% -> ISFR = 6.00%
- Attack: Aave spiked to 50.00% by flash loan -> ISFR should be 6.65%
- Verify: shift is <100 bps despite a 4,550 bps spike on a single source

**Checklist:**
- [ ] Create integration test that constructs 4 mock sources with known rates
- [ ] Compute ISFR under normal conditions, assert 600 bps
- [ ] Spike one source to 5000 bps (50%), recompute, assert result is between 620-670 bps
- [ ] Verify the shift is <100 bps (the median absorbed the outlier)
- [ ] Test with 2 sources spiked (should shift ISFR more -- 2 of 4 = 50% compromised)
- [ ] Document expected behavior in test comments

**Test:** `cargo test -p roko-chain --test isfr_manipulation`

---

## Phase 2: ISFR precompile

Goal: expose ISFR as a precompile at address 0xA01 on the Korai Kernel Plane.

### Task 2.1: Define precompile interface

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` (existing precompile stubs)
- PRD-07 section 4 (ISFRSnapshot struct, Solidity interface)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/isfr_oracle.rs`

**What to implement:**

The ISFR precompile at address 0xA01. Three callable functions:
1. `current()` -- returns `(uint32 valueBps, uint8 state)`
2. `at(uint64 blockHeight)` -- returns full `ISFRSnapshot`
3. `twap(uint64 startBlock, uint64 endBlock)` -- returns time-weighted average

**Checklist:**
- [ ] Create `crates/roko-chain/src/precompiles/` module directory
- [ ] Create `crates/roko-chain/src/precompiles/mod.rs`
- [ ] Define `ISFRPrecompile` struct at constant address `0xA01`
- [ ] Define `ISFRSnapshot` struct matching PRD-07: `{ value_bps: u32, block_height: u64, timestamp: u64, state: u8, confidence_bps: u16, num_sources: u32, num_validator_votes: u32 }`
- [ ] Implement `current()` method
- [ ] Implement `at(block_height)` with lookup in snapshot ring buffer
- [ ] Implement `twap(start_block, end_block)` as arithmetic mean of snapshots in range
- [ ] Store snapshots in a ring buffer (90 days at 10-second cadence = ~777,600 entries)
- [ ] ABI-encode return values for EVM compatibility
- [ ] Export from `lib.rs`
- [ ] Unit test: store 3 snapshots, call `current()`, call `at(block_2)`, call `twap(block_1, block_3)`

**Test:** `cargo test -p roko-chain -- isfr_precompile`

---

### Task 2.2: ISFR publication schedule

**Read first:** Task 2.1, PRD-07 section 2 (every 25 blocks at 400ms = ~10 seconds)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/isfr_oracle.rs`

**What to implement:**

The ISFR updates every 25 Korai blocks. At each update:
1. Collect source readings from all 4 sources
2. Compute source-level weighted median
3. Collect validator votes (each validator's computed median)
4. Compute stake-weighted validator median
5. Compute confidence score
6. Update circuit breaker state
7. Store snapshot
8. Emit `ISFRInsight` to InsightStore

**Checklist:**
- [ ] Implement `ISFRPublisher` struct that holds sources, circuit breaker, snapshot buffer
- [ ] Implement `tick(block_height: u64, timestamp: u64)` -- only computes on blocks where `block_height % 25 == 0`
- [ ] Wire source reading, aggregation, confidence, circuit breaker, and snapshot storage into `tick()`
- [ ] Implement `ISFRInsight` struct per PRD-07 section 6: value_bps, source_rates, confidence, delta_bps, moving averages, regime
- [ ] Implement rate regime classification: Stable (delta < 10bps/hr), Rising (delta > 10bps/hr up), Falling (delta > 10bps/hr down), Volatile (stddev > 50bps/day), Crisis (circuit breaker active)
- [ ] Unit test: call `tick()` at blocks 25, 50, 75 -- verify 3 snapshots stored
- [ ] Unit test: call `tick()` at block 26 -- verify no snapshot (not on the 25-block cadence)

**Test:** `cargo test -p roko-chain -- isfr_publisher`

---

## Phase 3: CRPS prediction system

Goal: agents commit predictions before each ISFR update. Predictions are scored using CRPS. Scores determine epistemic reputation tiers.

### Task 3.1: Define prediction types

**Read first:**
- PRD-07 section 6 (`ISFRPrediction` struct, CRPS formula, epistemic tiers)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/` (episode logger, efficiency events)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr_prediction.rs`

**What to implement:**

```rust
pub struct ISFRPrediction {
    pub agent: Address,           // ERC-8004 passport address
    pub predicted_bps: u32,       // predicted ISFR value
    pub confidence_interval_bps: u32, // width of confidence interval
    pub target_block: u64,        // block height this targets
    pub commitment: [u8; 32],     // hash commitment (commit-reveal)
}
```

**Checklist:**
- [ ] Define `ISFRPrediction` struct in new file
- [ ] Define `ISFRPredictionScore` struct: `{ agent, predicted_bps, actual_bps, crps_score: f64, block_height }`
- [ ] Implement CRPS scoring function: `crps_point(predicted: u32, actual: u32) -> f64` = `|predicted - actual| as f64`
- [ ] Implement commit-reveal: `commitment = keccak256(predicted_bps || nonce || agent)`
- [ ] Add to `crates/roko-chain/src/lib.rs` module declarations
- [ ] Unit test: prediction of 600 against actual 605 scores 5.0
- [ ] Unit test: prediction of 600 against actual 600 scores 0.0

**Test:** `cargo test -p roko-chain -- isfr_prediction`

---

### Task 3.2: Implement CRPS scoring engine

**Read first:** Task 3.1, PRD-07 section 6 (rolling 30-day window, epistemic tiers)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr_prediction.rs`

**What to implement:**

A scoring engine that:
1. Accepts predictions committed before each ISFR update
2. After the ISFR update, reveals and scores each prediction
3. Maintains a 30-day rolling CRPS percentile per agent

**Checklist:**
- [ ] Implement `PredictionEngine` struct with `predictions: HashMap<u64, Vec<ISFRPrediction>>` (keyed by target block)
- [ ] Implement `commit(prediction: ISFRPrediction)` -- store sealed prediction
- [ ] Implement `reveal_and_score(block_height: u64, actual_bps: u32) -> Vec<ISFRPredictionScore>`
- [ ] Implement `rolling_crps(agent: &Address, window_days: u32) -> f64` -- mean CRPS over the window
- [ ] Implement `crps_percentile(agent: &Address) -> f64` -- agent's percentile rank (lower CRPS = better rank)
- [ ] Store scored predictions in a ring buffer (30 days retention)
- [ ] Unit test: 5 agents, 10 predictions each, verify percentile rankings match expected order
- [ ] Unit test: verify rolling window drops old predictions after 30 days

**Test:** `cargo test -p roko-chain -- prediction_engine`

---

### Task 3.3: Implement epistemic reputation tiers

**Read first:**
- Task 3.2 output
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/reputation_registry.rs` (existing reputation system)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr_prediction.rs`

**What to implement:**

Map CRPS percentile to tier:

| CRPS percentile | Tier | Knowledge quota | Clearing priority |
|-----------------|------|-----------------|-------------------|
| Top 10% | Oracle | 2x base | First |
| 10-30% | Calibrated | 1.5x base | Standard |
| 30-70% | Standard | 1x base (100 queries/day) | Standard |
| 70-100% | Uncalibrated | 0.5x base | Last |

**Checklist:**
- [ ] Define `EpistemicTier` enum: `Oracle`, `Calibrated`, `Standard`, `Uncalibrated`
- [ ] Implement `tier_from_percentile(percentile: f64) -> EpistemicTier`
- [ ] Implement `knowledge_quota_multiplier(tier: EpistemicTier) -> f64`
- [ ] Implement `clearing_priority(tier: EpistemicTier) -> u8` (0 = first, 255 = last)
- [ ] Wire tier computation into the prediction engine's 30-day evaluation
- [ ] Implement 30-day half-life: Oracle-tier agent that stops predicting drops to Standard within 60 days
- [ ] Wire tier into reputation registry (extend `ReputationRegistry` with an `epistemic_tier` field per agent)
- [ ] Unit test: agent in top 10% CRPS percentile -> Oracle tier
- [ ] Unit test: agent stops predicting, verify tier decays over simulated 60 days

**Test:** `cargo test -p roko-chain -- epistemic_tier`

---

## Phase 4: Yield perpetual contract

Goal: implement the core yield perpetual instrument -- a perpetual futures contract where the underlying is a yield rate in basis points.

### Task 4.1: Define yield perpetual types

**Read first:**
- PRD-07 sections 7, 8 (contract spec, payoff, mark price, funding, margin)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/futures_market.rs` (existing futures infrastructure)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/yield_perp.rs`

**What to implement:**

Contract spec from PRD-07 section 7:
- Underlying: ISFR (basis points)
- Contract multiplier: $1 notional per 1 bp per unit
- Minimum tick: 0.25 bp
- Max leverage: 10x (10% initial margin, 5% maintenance)
- Funding interval: 8 hours

**Checklist:**
- [ ] Define `YieldPerpMarket` struct: `{ market_id, underlying_index, tick_size_bps: f64, max_leverage: u32, initial_margin_pct: f64, maintenance_margin_pct: f64, funding_interval_secs: u64 }`
- [ ] Define `YieldPerpPosition` struct: `{ position_id, account, market_id, direction: Direction, entry_bps: u32, size_units: u64, margin: u256, unrealized_pnl: i128, last_funding_timestamp: u64 }`
- [ ] Define `Direction` enum: `Long`, `Short`
- [ ] Implement `PnL` calculation: `pnl = (current_bps - entry_bps) * direction_sign * size_units`
- [ ] Implement `is_liquidatable(mark_bps: u32) -> bool`: true when mark-to-market loss exceeds (margin - maintenance_margin)
- [ ] Add to `lib.rs` exports
- [ ] Unit test: long position at 600, ISFR rises to 650, verify PnL = +50 * size_units
- [ ] Unit test: short position at 600, ISFR drops to 450, verify PnL = +150 * size_units

**Test:** `cargo test -p roko-chain -- yield_perp_types`

---

### Task 4.2: Implement mark price computation

**Read first:** PRD-07 section 7 (mark price formula)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/yield_perp.rs`

**What to implement:**

```
MarkPrice = 0.7 * ISFR_Oracle + 0.3 * EMA(OrderBook_MidPrice, 300s)
```

Adjustments by ISFR state:
- Live: 0.7/0.3 split
- Degraded: 0.9/0.1 with 600s EMA
- Stale: 1.0 * last Live ISFR
- Halted: frozen

**Checklist:**
- [ ] Implement `MarkPriceEngine` struct with ISFR state reference and order book EMA
- [ ] Implement `EMA` struct with configurable half-life (300s default)
- [ ] Implement `update_ema(new_mid_price: u32, timestamp: u64)`
- [ ] Implement `mark_price(isfr_state: ISFRState, isfr_bps: u32, ema_bps: u32) -> u32`
- [ ] Handle Degraded: widen EMA window to 600s, increase oracle weight to 0.9
- [ ] Handle Stale: return last known Live ISFR
- [ ] Handle Halted: return frozen mark price
- [ ] Unit test: Live state with ISFR=600, EMA=610 -> mark = 603
- [ ] Unit test: Degraded state -> verify wider EMA and higher oracle weight

**Test:** `cargo test -p roko-chain -- mark_price`

---

### Task 4.3: Implement funding rate

**Read first:** PRD-07 section 7 (premium + carry components)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/yield_perp.rs`

**What to implement:**

```
FundingRate = PremiumComponent + CarryComponent

PremiumComponent = clamp(EMA(MidPrice - ISFR, 300s) / ISFR, -0.0005, +0.0005)
CarryComponent = (ISFR - RiskFreeRate) * (FundingInterval / Year)
```

Every 8 hours: `FundingPayment = PositionSize * FundingRate`

**Checklist:**
- [ ] Implement `compute_premium(mid_price_bps: u32, isfr_bps: u32, ema_spread: f64) -> f64`
- [ ] Implement `compute_carry(isfr_bps: u32, risk_free_bps: u32, interval_secs: u64) -> f64`
- [ ] Implement `compute_funding_rate(premium: f64, carry: f64) -> f64`
- [ ] Implement `apply_funding(position: &mut YieldPerpPosition, funding_rate: f64)`
- [ ] Clamp premium to [-5 bps, +5 bps] per 8-hour period
- [ ] Use ETH staking yield (source 4 from ISFR) as risk-free rate
- [ ] Track last funding timestamp per position to prevent double-application
- [ ] Unit test: positive premium (perp above ISFR) -> longs pay shorts
- [ ] Unit test: negative premium (perp below ISFR) -> shorts pay longs
- [ ] Unit test: verify funding amounts match hand-calculated values

**Test:** `cargo test -p roko-chain -- funding_rate`

---

### Task 4.4: Implement margin requirements

**Read first:** PRD-07 section 7 (margin table, liquidation example)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/yield_perp.rs`

**What to implement:**

| Parameter | Value |
|-----------|-------|
| Initial margin | 10% of notional |
| Maintenance margin | 5% of notional |
| Insurance fund contribution | 0.5 bps per cleared trade |

**Checklist:**
- [ ] Implement `required_initial_margin(notional: u256) -> u256`: 10% of notional
- [ ] Implement `required_maintenance_margin(notional: u256) -> u256`: 5% of notional
- [ ] Implement `check_margin(position: &YieldPerpPosition, mark_bps: u32) -> MarginStatus`
- [ ] Define `MarginStatus` enum: `Healthy`, `Warning` (margin < 2x maintenance), `Liquidatable` (margin < maintenance)
- [ ] Implement insurance fund contribution: 0.5 bps per trade notional
- [ ] Unit test: $10,000 notional at 10x leverage, 50 bps adverse move -> liquidation triggers (per PRD-07 section 7 example)
- [ ] Unit test: position with sufficient margin -> Healthy status

**Test:** `cargo test -p roko-chain -- margin`

---

### Task 4.5: End-to-end yield perp P&L test

**Read first:** PRD-07 section 8 (worked hedging examples)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/yield_perp_pnl.rs`

**What to implement:**

Reproduce the $10M DAO treasury hedging example from PRD-07 section 8:
- Enter short at 693 bps, 5,000 units
- ISFR declines to 300 bps
- Expected PnL: (693 - 300) * $1 * 5,000 = $1,965,000
- Funding cost over 90 days at 0.01% per 8-hour interval

**Checklist:**
- [ ] Create integration test with full lifecycle: open position -> mark-to-market -> funding payments -> close position
- [ ] Verify PnL matches hand-calculated $1,965,000
- [ ] Verify funding cost is approximately $935 (per PRD-07 section 8)
- [ ] Test the scenario where rates stay high (profile never activates, cost = $0)
- [ ] Assert all values within 1% of expected (floating point tolerance)

**Test:** `cargo test -p roko-chain --test yield_perp_pnl`

---

## Phase 5: Clearing profiles

Goal: implement the "one-action hedge" -- a persistent on-chain intent that activates when ISFR crosses a trigger threshold.

### Task 5.1: Define `ClearingProfile` types

**Read first:** PRD-07 section 9 (ClearingProfile Solidity struct, lifecycle)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/clearing_profile.rs`

**What to implement:**

```rust
pub struct ClearingProfile {
    pub account: Address,
    pub market: [u8; 32],      // keccak256("ISFR-PERP-V1")
    pub direction: Direction,   // Long or Short
    pub trigger_bps: u32,       // ISFR threshold
    pub max_notional: u256,
    pub max_fee_bps: u16,
    pub expiry: u64,            // 0 = no expiry
    pub min_fill_notional: u256,
    pub max_rounds: u32,        // 0 = unlimited
    // runtime state
    pub filled_notional: u256,
    pub rounds_participated: u32,
    pub status: ProfileStatus,
    pub created_at: u64,
}
```

**Checklist:**
- [ ] Define `ClearingProfile` struct
- [ ] Define `ProfileStatus` enum: `Dormant`, `Active`, `Filled`, `Expired`, `Cancelled`
- [ ] Implement `is_triggered(current_isfr_bps: u32) -> bool`: Short triggers when ISFR < trigger, Long triggers when ISFR > trigger
- [ ] Implement `remaining_notional() -> u256`: max_notional - filled_notional
- [ ] Implement `is_expired(now: u64) -> bool`
- [ ] Implement `is_round_limit_reached() -> bool`
- [ ] Implement `to_clearing_order(isfr_bps: u32) -> Option<ClearingOrder>`: produces an order if triggered and not exhausted
- [ ] Unit test: SHORT profile with trigger 700bps, ISFR at 650 -> triggered
- [ ] Unit test: SHORT profile with trigger 700bps, ISFR at 750 -> not triggered
- [ ] Unit test: profile with max_rounds=3, rounds_participated=3 -> exhausted

**Test:** `cargo test -p roko-chain -- clearing_profile`

---

### Task 5.2: Implement profile activation

**Read first:** Task 5.1, PRD-07 section 9 (lifecycle stages 2-4)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/clearing_profile.rs`

**What to implement:**

A `ProfileRegistry` that:
1. Stores all active profiles
2. On each ISFR update, checks which profiles are triggered
3. Converts triggered profiles into clearing orders for the next batch

**Checklist:**
- [ ] Implement `ProfileRegistry` struct: `HashMap<[u8; 32], ClearingProfile>` keyed by profile ID
- [ ] Implement `create_profile(profile: ClearingProfile) -> [u8; 32]` (returns profile ID)
- [ ] Implement `cancel_profile(profile_id: [u8; 32], account: Address) -> Result<()>` (only the owner can cancel)
- [ ] Implement `check_triggers(current_isfr_bps: u32, now: u64) -> Vec<ClearingOrder>` -- returns orders from all triggered, non-exhausted, non-expired profiles
- [ ] Implement `record_fill(profile_id: [u8; 32], filled_notional: u256, round: u32)` -- updates filled state after clearing
- [ ] Handle max_fee_bps: skip a clearing round if solver fee exceeds the profile's max fee
- [ ] Unit test: create 3 profiles with different triggers, verify correct subset activates at ISFR=650
- [ ] Unit test: profile fills across 3 rounds, verify filled_notional accumulates correctly

**Test:** `cargo test -p roko-chain -- profile_registry`

---

### Task 5.3: One-action creation flow

**Read first:** Task 5.2, PRD-08 section 2.3 (OpenClaw flow)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/clearing_profile.rs`

**What to implement:**

A convenience constructor that creates a clearing profile from minimal user input:

```rust
pub fn quick_hedge(
    account: Address,
    direction: Direction,
    trigger_bps: u32,
    max_notional: u256,
) -> ClearingProfile
```

Fills in sensible defaults: max_fee_bps=10, expiry=0 (never), min_fill_notional=100, max_rounds=0 (unlimited).

**Checklist:**
- [ ] Implement `quick_hedge()` convenience constructor
- [ ] Validate inputs: trigger_bps > 0, max_notional > 0
- [ ] Derive market ID from the ISFR-PERP-V1 constant
- [ ] Unit test: quick_hedge produces a valid profile that activates at the expected threshold

**Test:** `cargo test -p roko-chain -- quick_hedge`

---

## Phase 6: Cooperative clearing engine

Goal: batch auction settlement with solver competition and KKT verification. This is the mechanism that matches buy and sell orders to produce provably optimal clearing prices.

### Task 6.1: Implement batch accumulation

**Read first:**
- PRD-07 section 10 (Stage 1: Accumulation, 4 trigger conditions)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs` (existing `ClearingCycleState`)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/clearing_engine.rs`

**What to implement:**

Orders accumulate in a batch until one of 4 triggers fires:

| Trigger | Threshold |
|---------|-----------|
| Order count | 5+ orders |
| Time elapsed | 10 seconds |
| Imbalance ratio | 3:1 buy:sell or sell:buy |
| ISFR movement | 10+ bps since last clearing |

**Checklist:**
- [ ] Define `ClearingBatch` struct: `{ batch_id: u64, orders: Vec<ClearingOrder>, isfr_at_close_bps: u32, total_buy_notional: u256, total_sell_notional: u256, block_height: u64, timestamp: u64 }`
- [ ] Define `ClearingOrder` struct: `{ order_id: [u8; 32], side: Side, limit_bps: u32, notional: u256, partial_fill: bool, source: OrderSource }`
- [ ] Define `OrderSource` enum: `Active`, `Profile`, `Liquidation`
- [ ] Define `Side` enum: `Buy`, `Sell`
- [ ] Implement `BatchAccumulator` struct with pending orders and trigger state
- [ ] Implement `add_order(order: ClearingOrder)`
- [ ] Implement `should_close(elapsed_secs: u64, isfr_delta_bps: u32) -> bool` -- checks all 4 triggers
- [ ] Implement `close() -> ClearingBatch` -- seals the batch, resets accumulator
- [ ] Unit test: add 5 orders -> trigger fires on count
- [ ] Unit test: add 2 orders, wait 10s -> trigger fires on time
- [ ] Unit test: add 3 buys and 1 sell -> trigger fires on 3:1 imbalance

**Test:** `cargo test -p roko-chain -- batch_accumulator`

---

### Task 6.2: Implement solver interface

**Read first:** PRD-07 section 10 (Stage 3: Solver competition)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/clearing_engine.rs`

**What to implement:**

Solvers receive a sealed batch and have 800ms to compute the optimal clearing price that maximizes total surplus.

```rust
pub struct ClearingSolution {
    pub clearing_price_bps: u32,
    pub fills: Vec<FillAmount>,
    pub kkt_certificate: KKTCertificate,
    pub solver: Address,
    pub solve_time_ms: u32,
}

pub struct FillAmount {
    pub order_id: [u8; 32],
    pub amount: u256,
}
```

**Checklist:**
- [ ] Define `ClearingSolution`, `FillAmount`, `KKTCertificate` structs
- [ ] Implement `SurplusOptimizer` -- a reference solver that finds the optimal uniform clearing price
- [ ] Algorithm: sort buys descending by limit, sells ascending by limit, find the crossing point
- [ ] Compute total surplus: `sum(buyer_surplus) + sum(seller_surplus)`
- [ ] Handle partial fills: if the crossing point falls within an order, partially fill it
- [ ] Implement `solve(batch: &ClearingBatch) -> ClearingSolution`
- [ ] Add 800ms timeout (solver must return within this window)
- [ ] Unit test: 3 buys and 3 sells with known limits, verify clearing price and fills match expected
- [ ] Unit test: batch with no crossing (all buys below all sells) -> no fills
- [ ] Compute solver fee: min(total_surplus * 0.05, 50 KORAI cap)

**Test:** `cargo test -p roko-chain -- surplus_optimizer`

---

### Task 6.3: Implement KKT verification

**Read first:**
- PRD-07 section 10 (Stage 4: KKT verification, 3 conditions, pseudocode)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/clearing_engine.rs`

**What to implement:**

O(n) verification of the 3 KKT conditions:

1. **Primal feasibility:** fills don't exceed order sizes, buy fills at or below limit, sell fills at or above limit, total buy = total sell
2. **Dual feasibility:** shadow prices are non-negative
3. **Complementary slackness:** partially filled orders have limit price = clearing price

**Checklist:**
- [ ] Implement `verify_kkt(batch: &ClearingBatch, solution: &ClearingSolution) -> KKTVerificationResult`
- [ ] Define `KKTVerificationResult`: `{ valid: bool, primal_feasible: bool, dual_feasible: bool, complementary_slack: bool, violation_detail: Option<String> }`
- [ ] Check primal feasibility: loop through orders, verify fill <= order size, buy fills at p or better, sell fills at p or better, total_buy == total_sell
- [ ] Check partial fill constraint: if `0 < fill < order_size` and `!order.partial_fill`, reject
- [ ] Check complementary slackness: if `0 < fill < order_size`, then `order.limit_bps == clearing_price_bps`
- [ ] Return detailed violation information on failure
- [ ] Unit test: valid solution passes all 3 checks
- [ ] Unit test: solution with overfilled order -> primal feasibility fails
- [ ] Unit test: solution with buy filling above limit -> primal feasibility fails
- [ ] Unit test: partial fill with limit != clearing price -> complementary slackness fails
- [ ] Unit test: buy/sell volume mismatch -> primal feasibility fails

**Test:** `cargo test -p roko-chain -- kkt_verification`

---

### Task 6.4: Implement `ClearingInsight` emission

**Read first:** PRD-07 section 10 (Stage 5: Settlement, ClearingInsight struct)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/clearing_engine.rs`

**What to implement:**

After every settlement round, emit a structured knowledge artifact:

```rust
pub struct ClearingInsight {
    pub batch_id: u64,
    pub clearing_price_bps: u32,
    pub total_surplus: u256,
    pub num_orders_filled: u32,
    pub num_orders_unfilled: u32,
    pub buy_sell_imbalance: f64,
    pub time_to_solve_ms: u32,
    pub solver: Address,
    pub isfr_at_clear: u32,
    pub spread_to_isfr_bps: i32,
    pub timestamp: u64,
}
```

**Checklist:**
- [ ] Define `ClearingInsight` struct
- [ ] Implement `emit_insight(batch: &ClearingBatch, solution: &ClearingSolution) -> ClearingInsight`
- [ ] Compute buy/sell imbalance ratio
- [ ] Compute spread to ISFR: `clearing_price - isfr_at_close`
- [ ] Implement `InsightEmitter` trait for pluggable storage (InsightStore, local JSONL, etc.)
- [ ] Wire insight emission into the settlement pipeline
- [ ] Unit test: after settlement, insight contains correct values
- [ ] Unit test: imbalance calculation with 3:1 buy:sell ratio

**Test:** `cargo test -p roko-chain -- clearing_insight`

---

### Task 6.5: Implement fallback ladder

**Read first:** PRD-07 section 10 (fallback table: Normal -> Retry -> Emergency CLOB -> Circuit Breaker)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/clearing_engine.rs`

**What to implement:**

| Level | Condition | Action |
|-------|-----------|--------|
| Normal | Valid KKT solution within 800ms | Standard cooperative clearing |
| Retry | No valid solution within 800ms | Batch rolls to next block, solvers get another 400ms |
| Emergency CLOB | No valid solution after 2 retries | Price-time priority matching, no surplus optimization |
| Circuit Breaker | ISFR enters Halted state | Trading paused, positions preserved |

**Checklist:**
- [ ] Implement `ClearingFallback` state machine: `Normal`, `Retry(attempt: u8)`, `EmergencyCLOB`, `CircuitBreaker`
- [ ] Implement `emergency_clob_match(batch: &ClearingBatch) -> Vec<(ClearingOrder, ClearingOrder, u256)>` -- sequential price-time matching
- [ ] Implement `execute_clearing(batch: ClearingBatch, solver_timeout_ms: u64) -> ClearingResult`
- [ ] Handle retry: if solver times out, roll batch with retry counter
- [ ] After 2 retries, switch to emergency CLOB
- [ ] If ISFR state is Halted, reject all new orders and freeze
- [ ] Define `ClearingResult` enum: `Cooperative(ClearingSolution)`, `CLOB(Vec<Match>)`, `Halted`
- [ ] Unit test: solver returns in time -> Normal path
- [ ] Unit test: solver times out twice -> Emergency CLOB activates
- [ ] Unit test: ISFR halted -> no clearing

**Test:** `cargo test -p roko-chain -- clearing_fallback`

---

### Task 6.6: End-to-end clearing test (47 agents)

**Read first:** PRD-07 section 10 (worked clearing example with 47 orders)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/cooperative_clearing.rs`

**What to implement:**

Reproduce the 47-order clearing example from PRD-07 section 10:
- 18 active buy orders, 12 active sell orders, 15 profile-activated shorts, 2 liquidation sells
- ISFR at 587 bps
- Expected clearing price: ~589 bps
- Expected matched volume: ~$3,850,000 per side
- Expected total surplus: ~$58,200

**Checklist:**
- [ ] Create full clearing pipeline: accumulate 47 orders -> close batch -> solve -> verify KKT -> settle -> emit insight
- [ ] Verify clearing price is approximately 589 bps
- [ ] Verify total surplus is approximately $58,200
- [ ] Verify solver fee: min(58200 * 0.05, 50 KORAI)
- [ ] Verify insurance fund contribution: $3,850,000 * 2 * 0.5 bps
- [ ] Verify all KKT conditions pass
- [ ] Verify ClearingInsight is emitted with correct values
- [ ] Assert solve time is <800ms

**Test:** `cargo test -p roko-chain --test cooperative_clearing`

---

## Phase 7: Generalized benchmark index

Goal: extract the ISFR computation pattern into a generic `BenchmarkIndex` trait so the same infrastructure supports non-financial indices.

### Task 7.1: Define `BenchmarkIndex` trait

**Read first:**
- PRD-07 section 12 (the pattern, BenchmarkIndex trait, IndexSource, SourceReading, IndexValue)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/` (where core traits live)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/benchmark.rs`

**What to implement:**

The trait from PRD-07 section 12, placed in `roko-core` so any crate can implement it:

```rust
pub trait BenchmarkIndex: Send + Sync {
    fn sources(&self) -> &[IndexSource];
    fn compute(&self, readings: &[SourceReading]) -> IndexValue;
    fn confidence(&self, validator_votes: &[Vote]) -> f64;
    fn update_cadence_blocks(&self) -> u64;
    fn precompile_address(&self) -> Address;
    fn circuit_breaker_threshold(&self) -> f64 { 0.70 }
}
```

**Checklist:**
- [ ] Define `BenchmarkIndex` trait in `roko-core`
- [ ] Define `IndexSource` struct: name, weight, max_weight, liveness_timeout_secs, reader
- [ ] Define `SourceReading` struct: source_idx, value_bps, timestamp, is_live
- [ ] Define `IndexValue` struct: value_bps, num_sources, state (PublicationState)
- [ ] Define `PublicationState` enum: `Live`, `Degraded`, `Stale`, `Halted`
- [ ] Define `Vote` struct: value_bps, validator_index, stake_weight
- [ ] Add to `roko-core/src/lib.rs` exports
- [ ] Unit test: trait is object-safe (`Box<dyn BenchmarkIndex>` compiles)

**Test:** `cargo test -p roko-core -- benchmark_index`

---

### Task 7.2: Implement ISFR as first `BenchmarkIndex`

**Read first:** Tasks 7.1, 1.6

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/isfr.rs`

**What to implement:**

Implement `BenchmarkIndex` for the existing ISFR oracle, using the dual-median aggregation from Phase 1.

**Checklist:**
- [ ] Implement `BenchmarkIndex` for `ISFROracle`
- [ ] `sources()` returns the 4 V1 sources (Aave, Compound, Ethena, Beacon)
- [ ] `compute()` delegates to `compute_source_median()` from Task 1.6
- [ ] `confidence()` delegates to `compute_confidence()` from Task 1.7
- [ ] `update_cadence_blocks()` returns 25 (every 25 blocks at 400ms)
- [ ] `precompile_address()` returns 0xA01
- [ ] Verify the ISFR oracle passes all Phase 1 tests when accessed through the BenchmarkIndex trait
- [ ] Unit test: construct ISFROracle, call through `dyn BenchmarkIndex`, verify correct computation

**Test:** `cargo test -p roko-chain -- isfr_benchmark_index`

---

### Task 7.3: Stub additional indices

**Read first:** PRD-07 section 12 (candidate indices table: IAPI, IKQI, ISVI, IRRI)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/benchmark_indices.rs`

**What to implement:**

Stub implementations for two non-financial indices to prove the trait generalizes:

1. **IAPI (Internet Agent Performance Index)** -- measures agent task success rates. 4 sources: arena results, gate pass rates, task completion metrics, CRPS scores. Update cadence: 750 blocks (~5 min).

2. **IKQI (Internet Knowledge Quality Index)** -- measures InsightStore entry accuracy. 4 sources: confirmation rates, usage frequency, CRPS scores, peer validation counts. Update cadence: 9000 blocks (~1 hour).

**Checklist:**
- [ ] Implement `IAPIIndex` struct implementing `BenchmarkIndex`
- [ ] Implement `IKQIIndex` struct implementing `BenchmarkIndex`
- [ ] Use mock sources (the real data sources come in future phases)
- [ ] Verify both compile and pass the same confidence/circuit-breaker logic as ISFR
- [ ] Precompile addresses: IAPI at 0xA06, IKQI at 0xA07
- [ ] Unit test: IAPI with mock sources produces a valid IndexValue
- [ ] Unit test: IKQI with mock sources produces a valid IndexValue
- [ ] Unit test: demonstrate that `Vec<Box<dyn BenchmarkIndex>>` holds both ISFR and IAPI

**Test:** `cargo test -p roko-chain -- benchmark_indices`

---

## Phase 8: Generalized BenchmarkIndex integration with runtime

**Goal**: Wire the generalized `BenchmarkIndex` trait into the runtime event system so that ISFR updates propagate to the agent's attention model and dynamic worldview.

### Task 8.1: Wire ISFR updates into EventFabric

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs` (or the EventFabric module from IMPL-01)

**Read first:**
- `crates/roko-runtime/src/event_bus.rs` -- `RokoEvent` enum, `EventBus`
- `crates/roko-core/src/benchmark.rs` -- `BenchmarkIndex` trait, `IndexValue`, `PublicationState` (from Phase 7 Task 7.1)
- `crates/roko-chain/src/isfr.rs` -- `ISFROracle`, `IsfrAggregate`

**What to do:**

1. Add an `ISFRUpdate` variant to the `RokoEvent` enum:

```rust
pub enum RokoEvent {
    // ... existing variants ...
    /// A BenchmarkIndex (e.g., ISFR) published a new value.
    BenchmarkUpdate {
        index_name: String,
        value: IndexValue,
        previous_value: Option<IndexValue>,
        delta_bps: i32,
    },
}
```

2. In the ISFR publication path (Phase 2 Task 2.2 or the consensus block pipeline), emit a `BenchmarkUpdate` event after each ISFR publication:

```rust
event_bus.send(RokoEvent::BenchmarkUpdate {
    index_name: "ISFR".to_string(),
    value: new_isfr_value,
    previous_value: Some(old_isfr_value),
    delta_bps: (new_isfr_value.value_bps as i32 - old_isfr_value.value_bps as i32),
});
```

3. Any extension subscribed to the event bus receives the update. The cognitive engine can use `delta_bps` to modulate prediction error (large delta = high PE = escalation).

**Files to modify:**
- `crates/roko-runtime/src/event_bus.rs`
- `crates/roko-chain/src/isfr.rs` (or the publication path)

**Test:**
- Publish an ISFR value. Assert `BenchmarkUpdate` event received by a test subscriber.
- Delta computation: old=500, new=520 -> delta_bps=20.
- State transition from Live to Degraded emits event with correct `PublicationState`.

- [ ] `BenchmarkUpdate` event variant added to `RokoEvent`
- [ ] ISFR publication emits event with value, previous value, and delta
- [ ] Event received by subscribed extensions

---

### Task 8.2: Wire ISFR predictions into ForagingModel

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (or the foraging integration point)

**Read first:**
- IMPL-09 Phase 8 (foraging model, `ForagingEntity`, `GittinsComputer`)
- `crates/roko-learn/src/crps.rs` -- CRPS scoring
- Task 8.1 output

**What to do:**

1. When a `BenchmarkUpdate` event arrives, update the foraging model's attention allocation for blockchain entities:

```rust
fn handle_isfr_update(
    foraging: &mut ForagingModel,
    update: &BenchmarkUpdate,
) {
    let abs_delta = update.delta_bps.unsigned_abs();

    // Large ISFR movements increase the reward for blockchain entities
    if abs_delta > 50 {
        // Significant movement: boost blockchain entity Gittins indices
        for entity in foraging.entities_by_domain("blockchain") {
            entity.total_reward += abs_delta as f64 / 100.0;
            entity.last_observation = now_secs();
        }
        tracing::info!(delta_bps = update.delta_bps, "ISFR movement boosts blockchain attention");
    }

    // If ISFR enters Degraded/Stale/Halted, dramatically increase attention
    match update.value.state {
        PublicationState::Degraded | PublicationState::Stale => {
            foraging.set_domain_urgency("blockchain", 2.0);  // 2x attention multiplier
        }
        PublicationState::Halted => {
            foraging.set_domain_urgency("blockchain", 5.0);  // 5x attention multiplier
        }
        PublicationState::Live => {
            foraging.set_domain_urgency("blockchain", 1.0);  // normal
        }
    }
}
```

2. The foraging model's `allocate()` method already uses Gittins indices, so boosted rewards flow through to increased monitoring frequency.

3. Wire the handler into the event bus subscription in the orchestrator.

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`

**Test:**
- ISFR delta of 100 bps: blockchain entities get reward boost. Assert their Gittins index increases.
- ISFR enters Halted: foraging model applies 5x urgency multiplier. Assert blockchain monitoring interval decreases by ~5x.
- ISFR returns to Live: urgency resets to 1.0.

- [ ] Large ISFR movements boost blockchain entity rewards
- [ ] ISFR state transitions modulate foraging urgency
- [ ] Wired into event bus subscription in orchestrator

---

### Task 8.3: Wire ClearingInsights into WorldGraph

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (or the WorldGraph integration point)

**Read first:**
- Phase 6 Task 6.4 (`ClearingInsight` struct)
- IMPL-09 Phase 8 (WorldGraph entity model)
- Task 8.1 output

**What to do:**

1. After each clearing round emits a `ClearingInsight`, update the WorldGraph with market state:

```rust
fn handle_clearing_insight(
    worldgraph: &mut WorldGraph,
    insight: &ClearingInsight,
) {
    // Update or create "ISFR Market" entity
    let market_entity = worldgraph.get_or_create_entity(
        "isfr-market",
        "ISFR Yield Perp Market",
        "Market",
    );

    // Update properties with clearing data
    market_entity.set_property("clearing_price_bps", &insight.clearing_price_bps.to_string());
    market_entity.set_property("total_surplus", &insight.total_surplus.to_string());
    market_entity.set_property("orders_filled", &insight.num_orders_filled.to_string());
    market_entity.set_property("buy_sell_imbalance", &format!("{:.2}", insight.buy_sell_imbalance));
    market_entity.set_property("last_clear_time", &insight.timestamp.to_string());

    // Create relationship: ISFR Market -[settles_against]-> ISFR Oracle
    worldgraph.add_relationship(
        "isfr-market",
        "isfr-oracle",
        "settles_against",
        1.0,
    );

    // If imbalance is extreme (>3:1), create a "MarketStress" entity
    if insight.buy_sell_imbalance > 3.0 || insight.buy_sell_imbalance < 0.33 {
        let stress = worldgraph.get_or_create_entity(
            &format!("market-stress-{}", insight.batch_id),
            &format!("Market stress event (imbalance {:.1}:1)", insight.buy_sell_imbalance),
            "Event",
        );
        worldgraph.add_relationship(
            &stress.id,
            "isfr-market",
            "affects",
            insight.buy_sell_imbalance.abs(),
        );
    }
}
```

2. The WorldGraph bidder (IMPL-03 Phase 8) picks up these entities and includes them in agent context for blockchain tasks.

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`

**Test:**
- Clearing insight with 589 bps price and 3.2:1 imbalance: WorldGraph contains "ISFR Market" entity with correct properties and a "MarketStress" event entity.
- Clearing insight with balanced 1.1:1 imbalance: no stress event created.
- Subsequent blockchain task context includes WorldGraph market data.

- [ ] ClearingInsight updates WorldGraph "ISFR Market" entity
- [ ] Extreme imbalance creates "MarketStress" event entity
- [ ] Relationships added between market, oracle, and stress entities

---

### Task 8.4: Integration test for BenchmarkIndex runtime integration

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/benchmark_runtime.rs` (new file)

**Read first:**
- Tasks 8.1 through 8.3

**Do:**

1. **Scenario A: ISFR event triggers blockchain agent theta tick**
   - Set up an event bus with a subscriber
   - Publish an ISFR value with delta_bps = 80
   - Assert: `BenchmarkUpdate` event received
   - Assert: foraging model boosts blockchain entities
   - Assert: blockchain agent tick triggered (if tick threshold met)

2. **Scenario B: ISFR halt triggers emergency attention**
   - Publish ISFR with state = Halted
   - Assert: foraging urgency = 5.0 for blockchain domain
   - Assert: blockchain monitoring interval decreases

3. **Scenario C: Clearing round updates WorldGraph**
   - Emit a `ClearingInsight` with known values
   - Assert: WorldGraph contains "ISFR Market" entity
   - Assert: entity properties match insight values
   - Run context assembly for a blockchain task
   - Assert: WorldGraph market data appears in assembled context

4. Run: `cargo test -p roko-chain --test benchmark_runtime`

- [ ] ISFR events propagate through EventFabric
- [ ] Foraging model responds to ISFR movements
- [ ] Halted state triggers emergency attention
- [ ] ClearingInsights update WorldGraph
- [ ] All integration tests pass

---

## Acceptance criteria

These are the exit conditions for this implementation plan. Every item maps to a testable assertion.

- [ ] ISFR computes correctly from 4 sources with dual-median aggregation (Task 1.6)
- [ ] Flash loan attack on single source shifts ISFR <100 bps (Task 1.9)
- [ ] Circuit breaker transitions correctly through Live/Degraded/Stale/Halted (Task 1.8)
- [ ] Confidence score reflects validator agreement (Task 1.7)
- [ ] ISFR precompile returns current value, historical value, and TWAP (Task 2.1)
- [ ] ISFR publishes every 25 blocks (Task 2.2)
- [ ] CRPS scoring correctly ranks agent prediction accuracy (Tasks 3.1, 3.2)
- [ ] Epistemic tiers map CRPS percentile to Oracle/Calibrated/Standard/Uncalibrated (Task 3.3)
- [ ] Yield perp PnL matches hand-calculated expected values (Task 4.5)
- [ ] Mark price adjusts by ISFR state (Live/Degraded/Stale/Halted) (Task 4.2)
- [ ] Funding rate has premium + carry components, applied every 8 hours (Task 4.3)
- [ ] Margin requirements enforce 10% initial, 5% maintenance (Task 4.4)
- [ ] Clearing profile activates when ISFR crosses trigger threshold (Task 5.2)
- [ ] Clearing profile respects max_notional, max_rounds, max_fee_bps, and expiry (Task 5.1)
- [ ] KKT verification passes for valid solutions, rejects invalid ones (Task 6.3)
- [ ] Cooperative clearing matches 47 orders with ~$58,200 surplus (Task 6.6)
- [ ] ClearingInsight emitted after every settlement round (Task 6.4)
- [ ] Fallback ladder degrades from cooperative clearing to CLOB to halt (Task 6.5)
- [ ] BenchmarkIndex trait generalizes beyond ISFR (Tasks 7.1, 7.3)
- [ ] All tests pass: `cargo test -p roko-chain`
- [ ] Clippy clean: `cargo clippy -p roko-chain --no-deps -- -D warnings`

---

## Dependencies

| This phase | Depends on | Reason |
|-----------|------------|--------|
| Phase 2 | Phase 1 | Precompile wraps the oracle |
| Phase 3 | Phase 2 | Predictions target published ISFR values |
| Phase 4 | Phase 1 | Yield perps settle against ISFR |
| Phase 5 | Phase 4 | Profiles create yield perp positions |
| Phase 6 | Phase 4, Phase 5 | Clearing settles yield perp orders from profiles and traders |
| Phase 7 | Phase 1 | Generalization extracts the ISFR pattern |

Phases 1 and 7 can be developed in parallel. Phases 4 and 5 can be developed in parallel once Phase 1 is complete.

---

## Build and test commands

```bash
# Build the chain crate
cargo build -p roko-chain

# Run all ISFR tests
cargo test -p roko-chain -- isfr

# Run clearing tests
cargo test -p roko-chain -- clearing

# Run yield perp tests
cargo test -p roko-chain -- yield_perp

# Run the full integration test suite
cargo test -p roko-chain --test isfr_manipulation
cargo test -p roko-chain --test yield_perp_pnl
cargo test -p roko-chain --test cooperative_clearing

# Lint
cargo clippy -p roko-chain --no-deps -- -D warnings
```
