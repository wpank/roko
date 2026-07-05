# 03 -- TA Indicators and Market Analysis: Agent-Executable Work Batches

> **Batch count**: 6 | **Total items**: ~35 | **Phase**: 0/1
> **Primary crate**: `roko-learn`

---

## Batch 3.1: Classical indicator expansion

> **Effort**: S | **Depends on**: none | **Crate**: roko-learn
> **Branch**: `defi/batch-3.1-classical-indicators`

### Context

The `ChainOracle` in `roko-learn/src/oracles/chain.rs` already computes four classical TA indicators: SMA(20), EMA(12), RSI(14), and Bollinger Bands(20,2). Each indicator is a standalone function that takes a `&[f64]` price slice and returns an `Option` value. The `compute_indicators()` method at line 197 calls all four and produces a `Vec<IndicatorOutput>` with name, value, signal direction (-1 to +1), and confidence (0 to 1).

This batch adds six missing classical indicators: MACD, Stochastic oscillator, ATR, ADX, OBV, and Williams %R. The existing `PricePoint` struct (line 16) already carries `high`, `low`, and `close` fields needed for ATR and ADX, but the current indicator functions operate on `&[f64]` price slices extracted from a `VecDeque<f64>` in `price_history`. To use OHLC data, the oracle needs a parallel `VecDeque<PricePoint>` or the existing functions need access to high/low/close arrays. The simplest approach: add an `observe_ohlc()` method alongside the existing `observe_price()` and store `PricePoint` values in a new field, then feed those to the OHLC-aware indicators.

Each new indicator follows the same pattern as the existing four: a pure function returning `Option<T>`, called from `compute_indicators()`, producing an `IndicatorOutput`. No chain data feed needed -- these compute on the same synthetic/cached price history the oracle already uses.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/oracles/chain.rs` | Lines 16-27: `PricePoint` struct with `high`/`low`/`close`. Lines 43-142: existing indicator functions. Lines 149-270: `ChainOracle` struct and `compute_indicators()`. Lines 400-486: existing tests. |
| `crates/roko-learn/src/kalman.rs` | Lines 36-56: `KalmanFilter` struct. Factory methods at lines 85-95. The Kalman filter can smooth noisy indicator outputs -- consider using `KalmanFilter::for_oracle_smoothing()` for volatile indicators like ATR. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Add `observe_ohlc()` to `ChainOracle`** (`chain.rs`, near line 176). Add a new field `ohlc_history: parking_lot::RwLock<HashMap<String, VecDeque<PricePoint>>>` to `ChainOracle`. The `observe_ohlc()` method pushes a `PricePoint`; `observe_price()` continues to work for backward compat. Add a helper `fn ohlc(&self, target: &str) -> Vec<PricePoint>` mirroring the existing `fn prices()` at line 188.

2. **Implement MACD** (`chain.rs`). Pure function `fn macd(prices: &[f64]) -> Option<MacdOutput>` where `MacdOutput { macd_line: f64, signal_line: f64, histogram: f64 }`. MACD line = EMA(12) - EMA(26). Signal line = EMA(9) of MACD line. Histogram = MACD - signal. Reuse existing `exponential_ma()` at line 52. Signal: histogram > 0 = bullish, < 0 = bearish. Confidence: 0.6.

3. **Implement Stochastic oscillator** (`chain.rs`). Pure function `fn stochastic(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Option<StochasticOutput>` where `StochasticOutput { k: f64, d: f64 }`. %K = 100 * (close - lowest_low) / (highest_high - lowest_low). %D = SMA(3) of %K. Signal: %K > 80 = overbought (-1.0), %K < 20 = oversold (+1.0), else %K crossover %D. Confidence: 0.6.

4. **Implement ATR** (`chain.rs`). Pure function `fn atr(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Option<f64>`. True range = max(high-low, |high-prev_close|, |low-prev_close|). ATR = Wilder smoothing of TR over `period`. No directional signal (ATR is a volatility measure); use signal 0.0, confidence 0.5. Value is the ATR itself -- useful for stop-loss sizing and as input to other indicators.

5. **Implement ADX, OBV, Williams %R** (`chain.rs`). ADX: compute +DI/-DI from true range, smooth with Wilder, ADX = smoothed |+DI - -DI| / (+DI + -DI). Signal: ADX > 25 = trending (magnitude), +DI > -DI = bullish, else bearish. OBV: running total of volume, requires a `volume` field on `PricePoint` -- add `pub volume: f64` to `PricePoint` (line 16), default 0.0. Williams %R: same as Stochastic %K but inverted range [-100, 0].

6. **Wire all new indicators into `compute_indicators()`** (`chain.rs`, line 197). Add calls to each new function after the existing Bollinger Bands block (line 257). Each produces an `IndicatorOutput` with appropriate name, value, signal, and confidence. The weighted consensus at line 312 automatically incorporates them. Do NOT change the consensus formula.

### Wiring

No changes to `lib.rs` -- all new code is added to `crates/roko-learn/src/oracles/chain.rs`.

Add `volume` field to `PricePoint`:
```rust
pub struct PricePoint {
    pub ts_ms: i64,
    pub price: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// Trading volume for the period.
    pub volume: f64,
}
```

### Tests

```rust
#[test]
fn macd_computation() {
    // 30 points with an uptrend
    let prices: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
    let result = macd(&prices).unwrap();
    assert!(result.macd_line > 0.0, "uptrend MACD should be positive");
}

#[test]
fn stochastic_overbought() {
    // Create a series where current close is near the high
    let highs: Vec<f64> = (0..20).map(|i| 110.0 + i as f64).collect();
    let lows: Vec<f64> = (0..20).map(|i| 90.0 + i as f64).collect();
    let closes: Vec<f64> = (0..20).map(|i| 109.0 + i as f64).collect();
    let result = stochastic(&highs, &lows, &closes, 14).unwrap();
    assert!(result.k > 70.0, "close near high should give high %K");
}

#[test]
fn atr_positive() {
    let highs: Vec<f64> = (0..20).map(|i| 105.0 + (i as f64).sin() * 3.0).collect();
    let lows: Vec<f64> = (0..20).map(|i| 95.0 + (i as f64).sin() * 3.0).collect();
    let closes: Vec<f64> = (0..20).map(|i| 100.0 + (i as f64).sin() * 3.0).collect();
    let result = atr(&highs, &lows, &closes, 14).unwrap();
    assert!(result > 0.0, "ATR should be positive");
}

#[test]
fn adx_trending_market() {
    // Strong uptrend: each bar higher than the last
    let highs: Vec<f64> = (0..30).map(|i| 110.0 + i as f64 * 2.0).collect();
    let lows: Vec<f64> = (0..30).map(|i| 100.0 + i as f64 * 2.0).collect();
    let closes: Vec<f64> = (0..30).map(|i| 105.0 + i as f64 * 2.0).collect();
    let result = adx(&highs, &lows, &closes, 14).unwrap();
    assert!(result.adx > 20.0, "strong trend should have ADX > 20");
}

#[test]
fn observe_ohlc_feeds_indicators() {
    let oracle = ChainOracle::new();
    for i in 0..30 {
        oracle.observe_ohlc("ETH", PricePoint {
            ts_ms: i * 1000,
            price: 2000.0 + i as f64 * 10.0,
            high: 2010.0 + i as f64 * 10.0,
            low: 1990.0 + i as f64 * 10.0,
            close: 2000.0 + i as f64 * 10.0,
            volume: 1000.0,
        });
    }
    let indicators = oracle.compute_indicators("ETH");
    assert!(indicators.len() > 4, "should have more than 4 indicators after expansion");
}
```

### Verification

```bash
cargo test -p roko-learn -- oracles::chain
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `PricePoint` has `volume: f64` field
- [ ] `ChainOracle` has `observe_ohlc()` method storing `VecDeque<PricePoint>`
- [ ] MACD function implemented, returns `MacdOutput`
- [ ] Stochastic oscillator returns `StochasticOutput` with %K and %D
- [ ] ATR returns `f64` volatility value
- [ ] ADX returns `AdxOutput` with ADX value and +DI/-DI
- [ ] OBV accumulates volume
- [ ] Williams %R returns value in [-100, 0]
- [ ] All six new indicators wired into `compute_indicators()`
- [ ] 10+ tests pass

### Commit message

```
feat(roko-learn): add MACD, stochastic, ATR, ADX, OBV, Williams %R to ChainOracle
```

---

## Batch 3.2: DeFi-native indicators

> **Effort**: L | **Depends on**: 1.1 (alloy chain data) | **Crate**: roko-learn
> **Branch**: `defi/batch-3.2-defi-native-indicators`

### Context

DeFi-native indicators exploit on-chain transparency that has no analog in traditional finance. Concentrated liquidity pools (Uniswap V3) expose tick-level position data, lending protocols publish utilization rates, and perpetual DEXes broadcast funding rates -- all readable from contract state.

This batch implements six DeFi-native indicators as a new module in `roko-learn`. Each indicator reads protocol state (provided as typed inputs, not raw RPC calls) and produces an `IndicatorOutput` compatible with the `ChainOracle`'s consensus system. The indicators are: tick asymmetry index, liquidity migration velocity, density gap detection, position concentration (HHI), utilization rate momentum, and funding rate divergence.

These indicators do NOT perform their own chain reads. They receive pre-fetched protocol state as function arguments. The chain data layer from batch 1.1 provides this state; these functions are pure computations over it.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/oracles/chain.rs` | Lines 30-40: `IndicatorOutput` struct that all indicators must produce. Lines 197-260: `compute_indicators()` pattern to follow. |
| `crates/roko-learn/src/lib.rs` | Module declarations -- new module goes here. |
| `crates/roko-learn/src/kalman.rs` | Lines 85-95: `for_oracle_smoothing()` and `for_tracking()` factories. Use Kalman smoothing for noisy tick-level data. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-learn/src/defi_indicators.rs`**. Module-level doc: `//! DeFi-native indicators for concentrated liquidity, lending, and derivatives.` Import `IndicatorOutput` from `super::oracles::chain`.

2. **Define input types**. `TickSnapshot { tick: i32, liquidity: u128 }`, `PoolTickState { current_tick: i32, ticks: Vec<TickSnapshot> }`, `LendingState { utilization_rate: f64, borrow_rate: f64, supply_rate: f64 }`, `FundingState { funding_rate: f64, spot_price: f64, perp_price: f64 }`. All derive `Debug, Clone, Serialize, Deserialize`.

3. **Implement tick asymmetry index**. `fn tick_asymmetry(state: &PoolTickState) -> IndicatorOutput`. Sum liquidity above current tick vs below. Ratio = (above - below) / (above + below). Negative ratio means more liquidity below (market expects price to fall). Signal: ratio directly. Confidence: 0.55 (noisy signal).

4. **Implement liquidity migration velocity**. `fn liquidity_migration_velocity(previous: &PoolTickState, current: &PoolTickState) -> IndicatorOutput`. Compare tick distributions between two snapshots. Compute center of mass of liquidity in each, velocity = delta / time_delta. Positive velocity = liquidity moving up (bullish). Confidence: 0.5.

5. **Implement density gap detection**. `fn density_gaps(state: &PoolTickState, min_gap_ticks: i32) -> Vec<(i32, i32)>`. Find contiguous ranges of zero-liquidity ticks wider than `min_gap_ticks`. Return vec of (gap_start, gap_end). Gaps near current price = potential discontinuities in price action. Produce an `IndicatorOutput` with signal = -1.0 if gaps exist within 100 ticks of current price (price may jump), else 0.0.

6. **Implement HHI concentration**. `fn position_concentration(state: &PoolTickState) -> IndicatorOutput`. Herfindahl-Hirschman Index: HHI = sum of (share_i^2) where share_i = liquidity_i / total_liquidity per tick range. HHI near 1.0 = one entity dominates (risky). Signal: -(HHI - 0.5) (high concentration = bearish). Confidence: 0.6.

7. **Implement utilization rate momentum and funding rate divergence**. Utilization: `fn utilization_momentum(rates: &[f64]) -> IndicatorOutput`. Apply EMA(12) to rate series, signal = direction of trend. Funding: `fn funding_rate_divergence(state: &FundingState) -> IndicatorOutput`. Signal = -sign(funding_rate) when |funding_rate| exceeds threshold (extreme funding = mean-reversion expected). Confidence: 0.65.

### Wiring

Add to `crates/roko-learn/src/lib.rs`:
```rust
/// DeFi-native indicators for concentrated liquidity, lending, and derivatives.
pub mod defi_indicators;
```

### Tests

```rust
#[test]
fn tick_asymmetry_balanced() {
    let state = PoolTickState {
        current_tick: 100,
        ticks: vec![
            TickSnapshot { tick: 90, liquidity: 1000 },
            TickSnapshot { tick: 110, liquidity: 1000 },
        ],
    };
    let result = tick_asymmetry(&state);
    assert!(result.signal.abs() < 0.1, "balanced pool should be neutral");
}

#[test]
fn density_gap_near_current_price() {
    let mut ticks = Vec::new();
    for t in 0..50 { ticks.push(TickSnapshot { tick: t, liquidity: 1000 }); }
    // gap from 50 to 70
    for t in 70..120 { ticks.push(TickSnapshot { tick: t, liquidity: 1000 }); }
    let state = PoolTickState { current_tick: 48, ticks };
    let gaps = density_gaps(&state, 10);
    assert!(!gaps.is_empty(), "should detect gap");
}

#[test]
fn hhi_single_provider() {
    let state = PoolTickState {
        current_tick: 0,
        ticks: vec![TickSnapshot { tick: 0, liquidity: 10000 }],
    };
    let result = position_concentration(&state);
    assert!(result.value > 0.9, "single tick should have HHI near 1.0");
}
```

### Verification

```bash
cargo test -p roko-learn -- defi_indicators
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `defi_indicators.rs` module created and declared in `lib.rs`
- [ ] Input types defined: `TickSnapshot`, `PoolTickState`, `LendingState`, `FundingState`
- [ ] Tick asymmetry index computes ratio and produces `IndicatorOutput`
- [ ] Liquidity migration velocity compares two snapshots
- [ ] Density gap detection finds zero-liquidity ranges
- [ ] HHI concentration produces meaningful scores
- [ ] Utilization rate momentum follows EMA trend
- [ ] Funding rate divergence flags extreme rates
- [ ] 6+ tests pass

### Commit message

```
feat(roko-learn): add DeFi-native indicators for ticks, lending, and funding rates
```

---

## Batch 3.3: Microstructure indicators

> **Effort**: L | **Depends on**: 1.1 (alloy chain data) | **Crate**: roko-learn
> **Branch**: `defi/batch-3.3-microstructure`

### Context

Microstructure indicators measure trade flow quality: who is trading, how informed they are, and how much impact each trade has on price. These signals distinguish between noise trades and informed trades, which is the core signal for market-making and adverse selection avoidance.

This batch implements three microstructure indicators as a new module: trade flow imbalance, VPIN (volume-synchronized probability of informed trading), and Kyle's lambda (price impact per unit volume). Each takes a series of trade events and produces an `IndicatorOutput`.

The causal microstructure module already exists at `crates/roko-learn/src/causal.rs` -- this batch is a complementary set of simpler, faster indicators that operate on raw trade flow rather than causal graph construction.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/oracles/chain.rs` | Lines 30-40: `IndicatorOutput` struct. |
| `crates/roko-learn/src/causal.rs` | Existing causal module -- do NOT duplicate. This batch builds simpler indicators. |
| `crates/roko-learn/src/drift.rs` | Lines 88-92: `DriftDetector` struct. JSD computation at lines 185-208 is a reference for categorical divergence calculations that VPIN uses. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-learn/src/microstructure.rs`**. Module-level doc: `//! Microstructure indicators: trade flow imbalance, VPIN, and Kyle's lambda.`

2. **Define `TradeEvent` input type**. `TradeEvent { price: f64, volume: f64, is_buy: bool, block_number: u64, timestamp_ms: i64 }`. Derive `Debug, Clone, Serialize, Deserialize`.

3. **Implement trade flow imbalance**. `fn trade_flow_imbalance(trades: &[TradeEvent]) -> IndicatorOutput`. Buy volume - sell volume / total volume. Range [-1, 1]. Positive = net buying pressure (bullish). Use a rolling window (last N trades). Signal = imbalance value. Confidence: 0.6.

4. **Implement VPIN** (volume-synchronized PIN). `fn vpin(trades: &[TradeEvent], bucket_volume: f64) -> IndicatorOutput`. Bucket trades into equal-volume buckets. For each bucket, compute |buy_volume - sell_volume| / bucket_volume. VPIN = average across recent N buckets. High VPIN = high probability of informed trading (risky for LPs). Signal: -(VPIN - 0.5) (high VPIN = bearish for passive participants). Confidence: 0.65.

5. **Implement Kyle's lambda**. `fn kyles_lambda(trades: &[TradeEvent]) -> IndicatorOutput`. Regress price change on signed volume (OLS). Lambda = slope coefficient. High lambda = low liquidity (each trade moves price more). Signal: -lambda.clamp(-1, 1) (high impact = bearish). Confidence: 0.55.

### Wiring

Add to `crates/roko-learn/src/lib.rs`:
```rust
/// Microstructure indicators: trade flow, VPIN, and price impact.
pub mod microstructure;
```

### Tests

```rust
#[test]
fn imbalance_all_buys() {
    let trades: Vec<TradeEvent> = (0..10).map(|i| TradeEvent {
        price: 100.0, volume: 1.0, is_buy: true,
        block_number: i, timestamp_ms: i as i64 * 1000,
    }).collect();
    let result = trade_flow_imbalance(&trades);
    assert!((result.signal - 1.0).abs() < 0.01, "all buys = +1.0 imbalance");
}

#[test]
fn vpin_balanced_market() {
    let mut trades = Vec::new();
    for i in 0..100 {
        trades.push(TradeEvent {
            price: 100.0, volume: 1.0, is_buy: i % 2 == 0,
            block_number: i, timestamp_ms: i as i64 * 1000,
        });
    }
    let result = vpin(&trades, 10.0);
    assert!(result.value < 0.3, "balanced market should have low VPIN, got {}", result.value);
}

#[test]
fn kyles_lambda_positive_for_moving_market() {
    let trades: Vec<TradeEvent> = (0..20).map(|i| TradeEvent {
        price: 100.0 + i as f64 * 0.5, volume: 1.0, is_buy: true,
        block_number: i, timestamp_ms: i as i64 * 1000,
    }).collect();
    let result = kyles_lambda(&trades);
    assert!(result.value > 0.0, "rising price with buy volume = positive lambda");
}
```

### Verification

```bash
cargo test -p roko-learn -- microstructure
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `microstructure.rs` module created and declared in `lib.rs`
- [ ] `TradeEvent` type defined
- [ ] Trade flow imbalance produces buy/sell ratio in [-1, 1]
- [ ] VPIN buckets trades by volume and computes informed trading probability
- [ ] Kyle's lambda regresses price change on signed volume
- [ ] Each indicator produces well-formed `IndicatorOutput`
- [ ] 6+ tests pass

### Commit message

```
feat(roko-learn): add microstructure indicators — trade flow imbalance, VPIN, Kyle's lambda
```

---

## Batch 3.4: On-chain signals and sentiment

> **Effort**: M | **Depends on**: 1.1 (alloy chain data) | **Crate**: roko-learn
> **Branch**: `defi/batch-3.4-onchain-sentiment`

### Context

On-chain signals bridge two TA families from the gap analysis: Family 5 (on-chain signals) and Family 7 (sentiment and positioning). Both families share a common pattern -- they read aggregate blockchain state and convert it into directional signals.

Family 5 covers MEV extraction rates, gas dynamics, and block fullness. The `MevDetector` in `roko-chain/src/gate/mev_gate.rs` already detects five MEV patterns (sandwich, front-run, back-run, JIT liquidity, cyclic arb) but operates as a gate, not an indicator. This batch wraps the detector output as a trackable rate indicator.

Family 7 covers funding rate momentum, open interest changes, and long/short ratios from perpetual DEXes. These use the same `FundingState` type defined in batch 3.2.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/gate/mev_gate.rs` | Lines 219-260: `MevDetector::detect()` returns `Vec<MevAlert>`. Lines 108-120: `MevPattern` enum. Lines 136-143: `MevSeverity` enum. This batch converts alert counts into a rate-based `IndicatorOutput`. |
| `crates/roko-learn/src/oracles/chain.rs` | Lines 30-40: `IndicatorOutput` struct. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-learn/src/onchain_indicators.rs`**. Module-level doc: `//! On-chain signal indicators: MEV rates, gas dynamics, block fullness, and sentiment.`

2. **Define input types**. `MevRateSnapshot { total_alerts: u32, critical_alerts: u32, total_txs: u32, timestamp_ms: i64 }`, `GasSnapshot { base_fee_gwei: f64, priority_fee_gwei: f64, block_number: u64 }`, `BlockSnapshot { gas_used: u64, gas_limit: u64, block_number: u64 }`, `SentimentSnapshot { funding_rate: f64, open_interest: f64, long_short_ratio: f64, timestamp_ms: i64 }`.

3. **Implement MEV extraction rate**. `fn mev_rate(snapshots: &[MevRateSnapshot]) -> IndicatorOutput`. Rate = critical_alerts / total_txs averaged over recent snapshots. High MEV rate = dangerous environment for passive participants. Signal: -(rate * 5.0).clamp(-1, 1). Confidence: 0.6.

4. **Implement gas price percentile**. `fn gas_percentile(snapshots: &[GasSnapshot], percentile: f64) -> IndicatorOutput`. Sort base fees, compute the Nth percentile. Signal: if current base fee > 90th percentile, signal = -0.8 (expensive, delay trades). Confidence: 0.7.

5. **Implement block fullness trend**. `fn block_fullness(snapshots: &[BlockSnapshot]) -> IndicatorOutput`. Fullness = gas_used / gas_limit per block. Compute EMA of fullness over recent blocks. Rising fullness above 95% = congested network. Signal: -(fullness - 0.5). Confidence: 0.55.

6. **Implement funding rate momentum**. `fn funding_momentum(snapshots: &[SentimentSnapshot]) -> IndicatorOutput`. Compute EMA of funding rates. Persistent positive funding = market overleveraged long (mean reversion expected). Signal: -sign(ema_funding) * min(1.0, |ema_funding| * 100.0). Confidence: 0.6.

7. **Implement open interest delta**. `fn open_interest_delta(snapshots: &[SentimentSnapshot]) -> IndicatorOutput`. Compute rate of change in OI. Rising OI + rising price = trend confirmation. Rising OI + falling price = aggressive shorts. Signal: sign(oi_delta) * sign(price_trend). Confidence: 0.55.

### Wiring

Add to `crates/roko-learn/src/lib.rs`:
```rust
/// On-chain signal indicators: MEV rates, gas, block fullness, and derivatives sentiment.
pub mod onchain_indicators;
```

### Tests

```rust
#[test]
fn mev_rate_no_alerts() {
    let snapshots = vec![MevRateSnapshot {
        total_alerts: 0, critical_alerts: 0, total_txs: 100, timestamp_ms: 0,
    }];
    let result = mev_rate(&snapshots);
    assert!(result.signal.abs() < 0.01, "no MEV = neutral signal");
}

#[test]
fn gas_percentile_high_fee() {
    let snapshots: Vec<GasSnapshot> = (0..100).map(|i| GasSnapshot {
        base_fee_gwei: i as f64, priority_fee_gwei: 2.0, block_number: i,
    }).collect();
    let result = gas_percentile(&snapshots, 0.9);
    assert!(result.value > 80.0, "90th percentile of 0..99 should be ~90");
}

#[test]
fn block_fullness_congested() {
    let snapshots: Vec<BlockSnapshot> = (0..20).map(|i| BlockSnapshot {
        gas_used: 29_500_000, gas_limit: 30_000_000, block_number: i,
    }).collect();
    let result = block_fullness(&snapshots);
    assert!(result.signal < -0.3, "98% full blocks should be bearish signal");
}
```

### Verification

```bash
cargo test -p roko-learn -- onchain_indicators
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `onchain_indicators.rs` module created and declared in `lib.rs`
- [ ] MEV extraction rate converts alert counts to trackable indicator
- [ ] Gas percentile computes Nth percentile from base fee history
- [ ] Block fullness tracks gas_used/gas_limit EMA
- [ ] Funding rate momentum applies EMA and produces directional signal
- [ ] Open interest delta detects trend confirmation/divergence
- [ ] 6+ tests pass

### Commit message

```
feat(roko-learn): add on-chain and sentiment indicators — MEV rates, gas, funding, OI
```

---

## Batch 3.5: Volatility and regime detection

> **Effort**: M | **Depends on**: 3.1 (classical indicators, for ATR) | **Crate**: roko-learn
> **Branch**: `defi/batch-3.5-volatility-regime`

### Context

Volatility and regime indicators address two TA families: Family 8 (volatility surface) and Family 9 (regime detection). These indicators determine *how* to trade rather than *what* to trade -- they classify market conditions and adapt strategy parameters accordingly.

The signal metabolism framework in `roko-learn/src/signal_metabolism.rs` already provides replicator dynamics for signal competition. Regime detection feeds this system: when the market regime changes, indicator fitness scores should shift, causing the replicator dynamics to evolve the population toward indicators suited to the new regime.

The `roko-gate/src/pelt.rs` has PELT change-point detection but is wired for gate verdicts. This batch builds a separate, indicator-oriented regime detector that can feed change-point signals into the TA pipeline.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/oracles/chain.rs` | Lines 119-142: `bollinger_bands()` -- Bollinger squeeze uses bandwidth from this function. |
| `crates/roko-learn/src/signal_metabolism.rs` | Lines 40-44: `SignalRegistry` struct. Lines 100-120: `replicator_step()` function. Regime changes should trigger fitness updates in the registry. |
| `crates/roko-learn/src/drift.rs` | Lines 88-92: `DriftDetector`. JSD computation can detect behavioral regime changes. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-learn/src/volatility_indicators.rs`**. Module-level doc: `//! Volatility and regime detection indicators.`

2. **Implement realized volatility (close-to-close)**. `fn realized_volatility(closes: &[f64], period: usize) -> Option<f64>`. Standard deviation of log returns: returns_i = ln(close_i / close_{i-1}), vol = std(returns) * sqrt(period). This is the most basic volatility estimate.

3. **Implement Parkinson volatility**. `fn parkinson_volatility(highs: &[f64], lows: &[f64], period: usize) -> Option<f64>`. High-low range estimator: vol = sqrt(1/(4*n*ln2) * sum(ln(high/low)^2)). More efficient than close-to-close because it uses intrabar range.

4. **Implement vol-of-vol**. `fn vol_of_vol(closes: &[f64], vol_period: usize, vov_period: usize) -> Option<f64>`. Compute rolling realized vol, then compute the standard deviation of that vol series. High vol-of-vol = unstable regime (reduce position sizes).

5. **Implement regime classifier**. `fn classify_regime(closes: &[f64], period: usize) -> RegimeClassification` where `RegimeClassification { regime: MarketRegime, confidence: f64 }` and `MarketRegime { Trending, MeanReverting, VolatileBreakout, LowVolConsolidation }`. Use a combination of: ADX > 25 = trending, Bollinger bandwidth < 5% of price = squeeze (low vol consolidation), recent breakout from squeeze = volatile breakout, variance ratio near 1.0 = random walk / neither trending nor reverting. Do NOT use external ADX function yet -- compute a simplified version inline.

6. **Implement breakout probability**. `fn breakout_probability(closes: &[f64], period: usize) -> IndicatorOutput`. Combine Bollinger squeeze detection (bandwidth narrowing) with volume expansion signal. Squeeze + volume spike = high breakout probability. Signal: breakout_probability in [0, 1]. Confidence: 0.5 (breakout prediction is inherently uncertain).

### Wiring

Add to `crates/roko-learn/src/lib.rs`:
```rust
/// Volatility and regime detection indicators.
pub mod volatility_indicators;
```

### Tests

```rust
#[test]
fn realized_vol_constant_price() {
    let closes: Vec<f64> = vec![100.0; 30];
    let vol = realized_volatility(&closes, 20).unwrap();
    assert!(vol < 0.001, "constant price = near-zero vol, got {vol}");
}

#[test]
fn realized_vol_trending_market() {
    let closes: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
    let vol = realized_volatility(&closes, 20).unwrap();
    assert!(vol > 0.0, "trending market has positive vol");
}

#[test]
fn parkinson_more_efficient() {
    // Parkinson should give lower variance estimate for same data
    let highs: Vec<f64> = (0..30).map(|i| 102.0 + (i as f64).sin() * 2.0).collect();
    let lows: Vec<f64> = (0..30).map(|i| 98.0 + (i as f64).sin() * 2.0).collect();
    let closes: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64).sin() * 2.0).collect();
    let p_vol = parkinson_volatility(&highs, &lows, 20).unwrap();
    let r_vol = realized_volatility(&closes, 20).unwrap();
    // Both should be positive
    assert!(p_vol > 0.0 && r_vol > 0.0);
}

#[test]
fn regime_classifier_trending() {
    // Strong uptrend
    let closes: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 2.0).collect();
    let result = classify_regime(&closes, 20);
    assert_eq!(result.regime, MarketRegime::Trending);
}

#[test]
fn vol_of_vol_stable_market() {
    let closes: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64 * 0.1).sin() * 2.0).collect();
    let vov = vol_of_vol(&closes, 14, 14).unwrap();
    assert!(vov < 0.1, "stable market should have low vol-of-vol");
}
```

### Verification

```bash
cargo test -p roko-learn -- volatility_indicators
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `volatility_indicators.rs` module created and declared in `lib.rs`
- [ ] Close-to-close realized vol implemented
- [ ] Parkinson high-low range vol implemented
- [ ] Vol-of-vol computes second-order volatility
- [ ] `MarketRegime` enum with 4 regimes
- [ ] `classify_regime()` uses ADX + Bollinger bandwidth + variance ratio
- [ ] Breakout probability combines squeeze + volume
- [ ] 8+ tests pass

### Commit message

```
feat(roko-learn): add volatility and regime detection indicators
```

---

## Batch 3.6: HDC composite indicators and market state encoding

> **Effort**: L | **Depends on**: 3.1, 3.2, 3.3, 3.4, 3.5 (needs indicator outputs to encode) | **Crate**: roko-learn
> **Branch**: `defi/batch-3.6-hdc-composite`

### Context

HDC (hyperdimensional computing) pattern algebra encodes market states as 10,240-bit binary vectors using three operations: XOR bind (role-filler pairs), majority-vote bundle (co-occurrence), and cyclic permutation (temporal position). `roko-primitives/src/hdc.rs` provides `HdcVector` with all three operations, plus `ItemMemory` for codebook lookup.

This batch builds the market-specific encoding layer on top of existing HDC primitives. A market observation (set of indicator readings at a timestamp) encodes as a bundled vector of role-filler pairs. A sequence of observations permute-encodes temporal position. The result is a 10,240-bit fingerprint that supports O(1) pattern matching via XOR + POPCNT -- no matrix multiply, no GPU, no floating point.

The `ResonantPattern` in `roko-learn/src/resonant_patterns.rs` already wraps `HdcVector` as a fitness-scored organism with Lotka-Volterra competition. The `pattern_discovery.rs` module uses `HdcVector` for k-medoids clustering. This batch connects those systems to real indicator data by providing the encoding functions.

### Read first

| File | Why |
|------|-----|
| `crates/roko-primitives/src/hdc.rs` | Lines 83-176: `HdcVector` -- `bind()` at line 113, `bundle()` at line 129, `permute()` at line 154, `similarity()` at line 223. Lines 400-468: `ItemMemory` codebook with `top_k()` and `nearest()`. Lines 254-327: `BundleAccumulator` for incremental bundling. Lines 334-398: `DecayingBundleAccumulator` for temporal bias. |
| `crates/roko-learn/src/resonant_patterns.rs` | Lines 20-38: `ResonantPattern` struct with `genome: HdcVector`. Lines 96-145: `lotka_volterra_step()`. |
| `crates/roko-learn/src/oracles/chain.rs` | Lines 30-40: `IndicatorOutput` -- the input type that this batch encodes into HDC vectors. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-learn/src/hdc_market.rs`**. Module-level doc: `//! HDC market state encoding: role-filler composition for indicator readings.`

2. **Build a role codebook**. `fn build_role_codebook() -> ItemMemory`. Seed deterministic role vectors for each indicator name: `"sma_20"`, `"ema_12"`, `"rsi_14"`, `"bollinger"`, `"macd"`, `"stochastic"`, `"atr"`, `"adx"`, `"tick_asymmetry"`, `"funding_rate"`, `"mev_rate"`, `"realized_vol"`, `"regime"`, etc. Use `HdcVector::from_seed(name.as_bytes())` for determinism. Cache the codebook in a `LazyLock<ItemMemory>`.

3. **Build a value quantizer**. `fn quantize_value(value: f64, bins: usize) -> HdcVector`. Map a continuous value to one of N bins, return the deterministic seed vector for that bin. Bins are uniform in [-1, 1] for signals or [0, 1] for confidence values. Use `HdcVector::from_seed(&format!("bin_{bin_index}").as_bytes())`.

4. **Encode a single observation**. `fn encode_observation(indicators: &[IndicatorOutput]) -> HdcVector`. For each indicator: bind its role vector with its quantized signal value. Bundle all bound pairs. Formula: `H = Bundle(Role_i XOR Value_i for each i)`. At D=10,240 and K=10 fields, SNR = sqrt(10240/9) = 33.7 -- well above the noise floor.

5. **Encode a temporal sequence**. `fn encode_sequence(observations: &[HdcVector]) -> HdcVector`. For a sequence `[o_1, ..., o_T]`: `H_seq = Bundle(Permute^(t-1)(o_t) for each t)`. Most recent observation at position 0 (no permutation), older observations permuted further. Use `BundleAccumulator` from `roko-primitives` for incremental computation.

6. **Build pattern matching**. `fn match_pattern(query: &HdcVector, codebook: &ItemMemory) -> Vec<(&str, f32)>`. Thin wrapper around `ItemMemory::top_k()`. Also add `fn market_state_fingerprint(indicators: &[IndicatorOutput]) -> HdcVector` as a convenience alias for `encode_observation()`.

7. **Wire to `ResonantPattern`**. `fn to_resonant_pattern(indicators: &[IndicatorOutput], id: u64, fitness: f64) -> ResonantPattern`. Encode the observation as an HDC vector and wrap it in a `ResonantPattern` for Lotka-Volterra competition. This connects the indicator pipeline to the evolutionary dynamics system.

### Wiring

Add to `crates/roko-learn/src/lib.rs`:
```rust
/// HDC market state encoding and pattern matching for indicator readings.
pub mod hdc_market;
```

### Tests

```rust
#[test]
fn encode_observation_deterministic() {
    let indicators = vec![
        IndicatorOutput { name: "sma_20".into(), value: 100.0, signal: 0.5, confidence: 0.6 },
        IndicatorOutput { name: "rsi_14".into(), value: 55.0, signal: 0.0, confidence: 0.7 },
    ];
    let a = encode_observation(&indicators);
    let b = encode_observation(&indicators);
    assert_eq!(a, b, "same inputs must produce identical vectors");
}

#[test]
fn similar_observations_have_high_similarity() {
    let ind_a = vec![
        IndicatorOutput { name: "sma_20".into(), value: 100.0, signal: 0.5, confidence: 0.6 },
    ];
    let ind_b = vec![
        IndicatorOutput { name: "sma_20".into(), value: 101.0, signal: 0.5, confidence: 0.6 },
    ];
    let a = encode_observation(&ind_a);
    let b = encode_observation(&ind_b);
    assert!(a.similarity(&b) > 0.7, "similar observations should be similar vectors");
}

#[test]
fn different_observations_have_low_similarity() {
    let ind_a = vec![
        IndicatorOutput { name: "sma_20".into(), value: 100.0, signal: 1.0, confidence: 0.9 },
    ];
    let ind_b = vec![
        IndicatorOutput { name: "rsi_14".into(), value: 30.0, signal: -1.0, confidence: 0.8 },
    ];
    let a = encode_observation(&ind_a);
    let b = encode_observation(&ind_b);
    assert!(a.similarity(&b) < 0.6, "different indicators should produce different vectors");
}

#[test]
fn sequence_encoding_biases_recent() {
    let obs: Vec<HdcVector> = (0..5).map(|i| {
        HdcVector::from_seed(&format!("obs_{i}").as_bytes())
    }).collect();
    let seq = encode_sequence(&obs);
    // Most recent observation (last) should be closest
    assert!(seq.similarity(&obs[4]) > seq.similarity(&obs[0]),
        "sequence should be more similar to recent observations");
}

#[test]
fn to_resonant_pattern_creates_valid_pattern() {
    let indicators = vec![
        IndicatorOutput { name: "sma_20".into(), value: 100.0, signal: 0.5, confidence: 0.6 },
    ];
    let pattern = to_resonant_pattern(&indicators, 42, 0.8);
    assert_eq!(pattern.id, 42);
    assert!((pattern.fitness - 0.8).abs() < f64::EPSILON);
    assert!(pattern.is_alive());
}
```

### Verification

```bash
cargo test -p roko-learn -- hdc_market
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `hdc_market.rs` module created and declared in `lib.rs`
- [ ] Role codebook built from deterministic seeds
- [ ] Value quantizer maps continuous signals to discrete HDC bins
- [ ] `encode_observation()` binds role-filler pairs and bundles
- [ ] `encode_sequence()` permute-encodes temporal position
- [ ] Pattern matching wraps `ItemMemory::top_k()`
- [ ] `to_resonant_pattern()` connects to Lotka-Volterra system
- [ ] Determinism: same inputs always produce identical vectors
- [ ] 8+ tests pass

### Commit message

```
feat(roko-learn): add HDC market state encoding and pattern matching
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives used

- **Recipe**: Every indicator type defined in this gap doc is a Recipe — a composable, reusable transform that takes a Feed as input and produces scored output. Classical indicators (MACD, RSI, ATR, ADX, OBV, Williams %R, Stochastic), DeFi-native indicators (tick asymmetry, liquidity migration, density gaps, HHI concentration), microstructure metrics (trade flow imbalance, VPIN, Kyle's lambda), on-chain signals (MEV rates, gas dynamics, funding momentum), volatility and regime classifiers (realized vol, Parkinson vol, vol-of-vol, regime state), and HDC composites (role-filler binding, bundling, temporal sequences) are all authored and stored as Recipes.
- **Knowledge Entry**: Regime codebook entries are Knowledge Entries — canonical HDC vectors representing known market states (trending-up, trending-down, range-bound, high-volatility, low-volatility, crisis). Agents query these entries during classification to identify the nearest known regime.
- **Signal**: The regime classifier emits a Signal when it detects a state transition — the signal carries the prior regime, the new regime, the confidence score, and the indicator values that triggered the change.

### Authoring surfaces

- **Recipe Editor** — build indicator pipelines by chaining transforms: Feed → indicator → indicator → scorer → Signal; preview live output against historical data before saving
- **Knowledge → Indicators** — browse the full indicator library with live sparkline charts, backtest performance summaries, and fork/customize controls

### Shareable artifacts

- Recipe templates: indicator pipelines ready to attach to an agent — for example, "RSI-MACD regime scorer" or "microstructure imbalance detector"
- Indicator packs: bundled sets of related indicators (classical technical pack, DeFi-native pack, microstructure pack) installable as a unit
- Regime codebooks: named sets of canonical regime vectors for specific markets or asset classes, forkable and tunable

### Dashboard visibility

- **Forge → Recipes** — indicator pipeline library with backtest result summaries, parameter sliders, and live preview
- **Knowledge → Entry Detail** — regime codebook entries with similarity search and nearest-neighbor visualization
- **Pulse → Indicators** — live indicator values per active agent with sparklines and regime state badge
