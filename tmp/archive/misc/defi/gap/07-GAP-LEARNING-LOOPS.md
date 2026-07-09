# 07 -- Learning loops: self-improvement from trading outcomes

> **Scope**: Make roko learn from trading results. Attribute P&L to decisions, track indicator accuracy, detect regime shifts, build trading playbooks, and replace binary gate-pass reward with risk-adjusted continuous reward.
>
> **Primary crate**: `roko-learn` (`crates/roko-learn/src/`)

---

## Batch 7.1: TradingReflect -- FIFO P&L attribution

> **Effort**: L | **Depends on**: 2.1 (VenueAdapter for trade events) | **Crate**: roko-learn
> **Branch**: `defi/batch-7.1-trading-reflect`

### Context

Every downstream learning subsystem in roko operates on a binary reward signal: gate passed = 1.0, gate failed = 0.0. For DeFi, the ground-truth signal is realized P&L. When a trade closes, roko must trace the outcome back to the decision that opened it -- which agent, which indicators, which model, what regime -- and feed a continuous reward into the cascade router, playbook outcomes, and episode logger.

The learning infrastructure already handles this fan-out. `LearningRuntime::record_completed_run` in `runtime_feedback.rs:326` ingests a `CompletedRunInput` and distributes to all subsystems. `Episode` records at `episode_logger.rs:168` carry `gate_verdicts`, `success`, `usage`, `hdc_fingerprint`, and an `extra` map. The gap is that none of these carry trade-level data: entry price, exit price, realized P&L, gas cost, slippage.

This batch builds a FIFO matching engine that pairs position entries with exits, computes realized P&L, and emits a `TradingReflectEvent` that plugs into `LearningRuntime`. Every batch in docs 07 and 08 depends on this one.

**Data flow from isolated agents**: Trading agents run on isolated Fly Machines. P&L data flows back to the control plane via the agent lifecycle API (`POST /api/agents/{id}/events` on the control plane's roko-serve). The control plane ingests `TradingReflectEvent` payloads and fans them through `LearningRuntime::record_trading_outcome`. This keeps the learning pipeline centralized while execution remains isolated.

**P&L reporting endpoint**: Isolated trading agents POST to `POST /api/agents/{id}/events` on the control plane. Request body:

```json
{
    "event_type": "trading_reflect",
    "payload": {
        "closed_positions": [],
        "pnl_usd": 47.23,
        "gas_cost_usd": 1.02,
        "agent_id": "trade-executor-1",
        "timestamp": "2026-04-24T00:00:00Z"
    }
}
```

The control plane deserializes this into a `TradingReflectEvent` and passes it to `LearningRuntime::record_trading_outcome()`. Authentication uses the agent's `ROKO_TOKEN` (set when the Fly Machine is provisioned).

**Mirage-rs backtesting**: Agents can validate P&L calculations by replaying historical trades against forked chain state. The pattern: spawn an ephemeral mirage-rs instance forked to the block where the original trade executed, re-execute the trade, and compare the simulated receipt against the recorded outcome. Discrepancies surface accounting bugs or slippage miscalculations before they compound. Wire this as an optional verification step in `record_trading_outcome` -- when `backtest_on_reflect` is enabled in `LearningConfig`, the runtime spawns a mirage instance and replays the trade before recording the event.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/runtime_feedback.rs` | `LearningRuntime`, `CompletedRunInput`, `LearningPaths`, `record_completed_run` fan-out |
| `crates/roko-learn/src/episode_logger.rs:168-250` | `Episode` struct fields, `Usage`, `GateVerdict`, `extra` map |
| `crates/roko-learn/src/model_router.rs:284-370` | `compute_routing_reward_v2`, `RewardWeights` usage |
| `crates/roko-learn/src/local_reward.rs:14-59` | `LocalRewardFunction` -- binary observe, neutral prior |
| `crates/roko-learn/src/lib.rs` | Module declarations, crate structure |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**7.1.1** Create `crates/roko-learn/src/trading_reflect.rs`

Core types for FIFO P&L attribution:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata captured when a position is opened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionEntry {
    pub trade_id: String,
    pub episode_id: String,
    pub agent_id: String,
    pub model: String,
    pub asset: String,
    pub side: TradeSide,
    pub size: f64,
    pub entry_price: f64,
    pub gas_cost_usd: f64,
    pub regime_label: Option<String>,
    pub indicator_snapshot: Vec<IndicatorReading>,
    pub opened_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeSide { Long, Short }

/// One indicator reading at the time of entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorReading {
    pub name: String,
    pub value: f64,
    pub signal: IndicatorSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndicatorSignal { Buy, Sell, Neutral }
```

**7.1.2** Add FIFO matching engine in the same file:

```rust
/// FIFO matching engine that pairs position entries with exits.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FifoMatcher {
    open_entries: Vec<PositionEntry>,
}

impl FifoMatcher {
    pub fn record_entry(&mut self, entry: PositionEntry) { ... }

    /// Match an exit against the oldest open entry for the same asset+side.
    /// Returns the closed position with computed P&L.
    pub fn record_exit(
        &mut self,
        asset: &str,
        side: TradeSide,
        exit_price: f64,
        exit_size: f64,
        exit_gas_usd: f64,
        slippage_bps: f64,
    ) -> Option<ClosedPosition> { ... }

    pub fn open_positions(&self) -> &[PositionEntry] { ... }
}
```

**7.1.3** Add `ClosedPosition` and `TradingReflectEvent`:

```rust
/// A position that has been closed with realized P&L.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedPosition {
    pub entry: PositionEntry,
    pub exit_price: f64,
    pub realized_pnl: f64,
    pub gas_cost_total_usd: f64,
    pub slippage_bps: f64,
    pub hold_duration_secs: f64,
    pub closed_at: DateTime<Utc>,
}

/// Event emitted when a position closes, for LearningRuntime consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingReflectEvent {
    pub closed_position: ClosedPosition,
    /// Continuous reward in [-1.0, 1.0], derived from realized P&L.
    pub reward: f64,
}
```

**7.1.4** Add `TradingReflectStore` -- JSONL-backed persistence:

```rust
/// Append-only store for closed positions (JSONL).
pub struct TradingReflectStore {
    path: PathBuf,
}

impl TradingReflectStore {
    pub fn new(path: impl Into<PathBuf>) -> Self { ... }
    pub async fn append(&self, event: &TradingReflectEvent) -> io::Result<()> { ... }
    pub async fn read_all(&self) -> io::Result<Vec<TradingReflectEvent>> { ... }
    pub fn recent_pnl(&self, events: &[TradingReflectEvent], window: usize) -> f64 { ... }
}
```

**7.1.5** Wire into `LearningRuntime` -- add a `trading_reflect` field and a `record_trading_outcome` method in `runtime_feedback.rs` that converts `TradingReflectEvent` into the existing `record_completed_run` path by mapping continuous P&L reward into the episode's `extra` map and feeding the reward to the cascade router.

```rust
impl LearningRuntime {
    /// Record a trading outcome, distributing reward signals to all learning subsystems.
    pub fn record_trading_outcome(&mut self, event: &TradingReflectEvent) -> Result<()> {
        // 1. Compute continuous reward from P&L
        let reward = self.compute_pnl_reward(event);

        // 2. Build CompletedRunInput with trading data in extra map
        let input = CompletedRunInput {
            task_id: event.agent_id.clone(),
            model: event.model_used.clone().unwrap_or_default(),
            success: event.pnl_usd > 0.0,
            usage: Usage::default(),
            gate_verdicts: vec![],
            extra: {
                let mut m = HashMap::new();
                m.insert("trading_pnl_usd".into(), serde_json::json!(event.pnl_usd));
                m.insert("trading_gas_usd".into(), serde_json::json!(event.gas_cost_usd));
                m.insert("trading_reward".into(), serde_json::json!(reward));
                m.insert("closed_positions".into(), serde_json::json!(event.closed_positions.len()));
                m
            },
            ..Default::default()
        };

        // 3. Fan out to all subsystems via existing record_completed_run
        self.record_completed_run(input)?;

        Ok(())
    }

    fn compute_pnl_reward(&self, event: &TradingReflectEvent) -> f64 {
        // Continuous reward: scale P&L to [-1, 1] range
        // Positive P&L -> reward in (0, 1], negative -> penalty in [-1, 0)
        // Use tanh to bound: reward = tanh(pnl_usd / scale_factor)
        let scale = 100.0; // $100 maps to ~0.76 reward
        (event.pnl_usd / scale).tanh()
    }
}
```

**7.1.6** Extend `LearningPaths` (at `runtime_feedback.rs:84`) with `pub trading_reflect_jsonl: PathBuf` set to `root.join("trading-reflect.jsonl")`.

### Wiring

In `crates/roko-learn/src/lib.rs`, add:
```rust
/// FIFO P&L attribution for trading outcomes.
pub mod trading_reflect;
```

Re-export the main types from the crate root.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_matches_oldest_entry_first() {
        // Open two long ETH positions, close one. The first entry matches.
    }

    #[test]
    fn pnl_computation_includes_gas_and_slippage() {
        // Open at 2000, close at 2100, gas 5 + 5, slippage 10 bps.
        // Realized P&L = (2100 - 2000) * size - gas - slippage cost.
    }

    #[test]
    fn no_match_returns_none() {
        // Try to close a position with no matching open entry.
    }

    #[test]
    fn reward_maps_pnl_to_bounded_range() {
        // Positive P&L -> positive reward in (0, 1].
        // Negative P&L -> negative reward in [-1, 0).
    }
}
```

### Verification

```bash
cargo test -p roko-learn -- trading_reflect
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `FifoMatcher` correctly pairs entries with exits on asset+side FIFO order
- [ ] `ClosedPosition` computes realized P&L including gas and slippage
- [ ] `TradingReflectEvent` carries a continuous reward in [-1.0, 1.0]
- [ ] `TradingReflectStore` persists and reads events from JSONL
- [ ] `LearningRuntime::record_trading_outcome` fans reward to cascade router and episode logger
- [ ] `LearningPaths` includes `trading_reflect_jsonl`
- [ ] All tests pass, clippy clean, fmt clean

### Commit message

```
feat(roko-learn): add FIFO P&L attribution pipeline for trading outcomes
```

---

## Batch 7.2: Indicator accuracy tracking

> **Effort**: M | **Depends on**: 3.1 (classical indicators), 7.1 | **Crate**: roko-learn
> **Branch**: `defi/batch-7.2-indicator-accuracy`

### Context

TA indicators produce directional predictions: "RSI below 30 suggests oversold bounce." Roko does not track whether those predictions were correct. Without accuracy feedback, indicator weights remain static and the system cannot learn which indicators are reliable in which conditions.

The Kalman filter at `crates/roko-learn/src/kalman.rs:43` already ships with a `for_oracle_smoothing` preset (line 85: `KalmanFilter::new(initial_estimate, 1.0, 0.001, 0.1)`) designed for exactly this use case -- smoothing noisy accuracy observations into stable confidence estimates.

The cascade router's context vector (`model_router.rs:61`, `CONTEXT_DIM = 18`) encodes `TaskCategory` via one-hot but has no way to pass indicator confidence into routing decisions. This batch adds per-indicator accuracy tracking, smooths it with Kalman filters, and feeds the composite confidence into the routing context.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/kalman.rs:43-95` | `KalmanFilter`, `for_oracle_smoothing` preset, `update` method |
| `crates/roko-learn/src/model_router.rs:130-168` | `RoutingContext` fields, `daimon_policy`, `tier_thresholds` |
| `crates/roko-learn/src/trading_reflect.rs` | `IndicatorReading`, `IndicatorSignal` from batch 7.1 |
| `crates/roko-learn/src/drift.rs:89-105` | `DriftDetector` -- JSD comparison pattern to reuse |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**7.2.1** Create `crates/roko-learn/src/indicator_accuracy.rs`

```rust
use crate::kalman::KalmanFilter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Key: (indicator_name, timeframe, asset).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndicatorKey {
    pub indicator: String,
    pub timeframe: String,
    pub asset: String,
}

/// Tracks prediction accuracy for one indicator configuration.
#[derive(Debug, Clone)]
pub struct IndicatorAccuracyTracker {
    /// Raw success/total counts.
    pub successes: u64,
    pub total: u64,
    /// Kalman-smoothed accuracy estimate.
    pub filter: KalmanFilter,
}

impl IndicatorAccuracyTracker {
    pub fn new() -> Self {
        Self {
            successes: 0,
            total: 0,
            filter: KalmanFilter::for_oracle_smoothing(0.5),
        }
    }

    /// Record a prediction outcome and update the Kalman estimate.
    pub fn record(&mut self, correct: bool) { ... }

    /// Smoothed accuracy estimate in [0.0, 1.0].
    pub fn accuracy(&self) -> f64 { ... }
}
```

**7.2.2** Add `IndicatorAccuracyRegistry`:

```rust
/// Registry of per-indicator accuracy trackers, keyed by (indicator, timeframe, asset).
#[derive(Debug, Clone, Default)]
pub struct IndicatorAccuracyRegistry {
    trackers: HashMap<IndicatorKey, IndicatorAccuracyTracker>,
}

impl IndicatorAccuracyRegistry {
    pub fn record_prediction(
        &mut self,
        key: IndicatorKey,
        predicted_direction: IndicatorSignal,
        actual_direction: IndicatorSignal,
    ) { ... }

    /// Composite accuracy across all tracked indicators, weighted by observation count.
    pub fn composite_accuracy(&self) -> f64 { ... }

    /// Per-indicator accuracy snapshot for routing context.
    pub fn accuracy_snapshot(&self) -> Vec<(IndicatorKey, f64)> { ... }
}
```

**7.2.3** Add persistence: `save`/`load` methods on `IndicatorAccuracyRegistry` using JSON under the learning paths root.

**7.2.4** Extend `RoutingContext` in `model_router.rs:130` with `pub indicator_confidence: f64` -- a composite indicator accuracy score in [0.0, 1.0] that downstream routing can use (does not change `CONTEXT_DIM` yet; stored as metadata).

### Wiring

In `crates/roko-learn/src/lib.rs`, add:
```rust
/// Per-indicator prediction accuracy tracking with Kalman smoothing.
pub mod indicator_accuracy;
```

### Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn accuracy_tracks_rolling_prediction_quality() {
        // 7 correct out of 10 -> accuracy near 0.7 after Kalman smoothing.
    }

    #[test]
    fn composite_accuracy_weights_by_observation_count() {
        // Indicator with 100 obs at 0.8 should dominate one with 5 obs at 0.5.
    }

    #[test]
    fn kalman_smoothing_rejects_noise() {
        // Feed alternating correct/incorrect; smoothed accuracy converges to ~0.5.
    }
}
```

### Verification

```bash
cargo test -p roko-learn -- indicator_accuracy
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `IndicatorAccuracyTracker` uses `KalmanFilter::for_oracle_smoothing` preset
- [ ] `IndicatorAccuracyRegistry` tracks per-(indicator, timeframe, asset) accuracy
- [ ] Composite accuracy is observation-count weighted
- [ ] Registry serializes/deserializes to JSON
- [ ] `RoutingContext` carries `indicator_confidence`
- [ ] All tests pass, clippy clean, fmt clean

### Commit message

```
feat(roko-learn): add Kalman-smoothed indicator accuracy tracking
```

---

## Batch 7.3: Regime detection and strategy learning

> **Effort**: L | **Depends on**: 3.1 (classical indicators), 7.1 | **Crate**: roko-learn
> **Branch**: `defi/batch-7.3-regime-detection`

### Context

Strategies that work in trending markets fail in ranging markets. Roko's `DriftDetector` at `drift.rs:89` computes Jensen-Shannon divergence over agent behavioral distributions. It does not detect drift in market return distributions. The system has no regime labels, so it cannot correlate strategy performance with market state.

This batch adds a lightweight regime classifier that labels market state from return distributions, attaches regime labels to episodes and routing context, and tracks per-regime strategy performance. The existing `DriftDetector` JSD computation pattern is reused for detecting regime transitions.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/drift.rs:89-120` | `DriftDetector`, `ActionDistribution`, JSD computation |
| `crates/roko-learn/src/model_router.rs:130-168` | `RoutingContext` -- where regime label will be added |
| `crates/roko-learn/src/episode_logger.rs:168-250` | `Episode` struct, `extra` map for regime metadata |
| `crates/roko-learn/src/bandits.rs:76-103` | `UcbBandit`, `BanditArm` -- for strategy selection |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**7.3.1** Create `crates/roko-learn/src/regime.rs`

```rust
use serde::{Deserialize, Serialize};

/// Market regime labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketRegime {
    /// Strong directional movement, positive returns.
    TrendingUp,
    /// Strong directional movement, negative returns.
    TrendingDown,
    /// Low volatility, mean-reverting.
    Ranging,
    /// High volatility, no clear direction.
    Volatile,
    /// Unknown or insufficient data.
    Unknown,
}
```

**7.3.2** Add `RegimeClassifier`:

```rust
/// Classifies market regime from a window of returns.
///
/// Uses volatility (std dev of returns) and trend (mean return / std dev)
/// to assign a regime label. This is a simple threshold-based classifier
/// that can be replaced with HMM or k-means later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeClassifier {
    /// Rolling window of recent returns.
    returns: Vec<f64>,
    /// Window size for regime classification.
    window_size: usize,
    /// Volatility threshold separating low from high.
    volatility_threshold: f64,
    /// Trend z-score threshold for directional classification.
    trend_threshold: f64,
}

impl RegimeClassifier {
    pub fn new(window_size: usize) -> Self { ... }
    pub fn push_return(&mut self, r: f64) { ... }
    pub fn classify(&self) -> MarketRegime { ... }
}
```

**7.3.3** Add `RegimeTransitionDetector` using JSD from `drift.rs`:

```rust
/// Detects regime transitions by comparing return distributions across windows.
/// Reuses the JSD pattern from DriftDetector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeTransitionDetector {
    previous_regime: MarketRegime,
    transition_count: u64,
    jsd_threshold: f64,
}
```

**7.3.4** Add `PerRegimePerformance` tracker:

```rust
/// Tracks strategy performance aggregated by regime.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerRegimePerformance {
    /// Map from (strategy_id, regime) -> (total_pnl, trade_count, win_count).
    records: HashMap<(String, MarketRegime), PerformanceRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceRecord {
    pub total_pnl: f64,
    pub trade_count: u64,
    pub win_count: u64,
}
```

**7.3.5** Extend `RoutingContext` in `model_router.rs` with `pub regime: Option<MarketRegime>`.

### Wiring

In `crates/roko-learn/src/lib.rs`, add:
```rust
/// Market regime classification and per-regime performance tracking.
pub mod regime;
```

### Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn classifies_trending_from_positive_returns() {
        // Feed 20 returns with mean 0.02, low std dev -> TrendingUp.
    }

    #[test]
    fn classifies_volatile_from_high_variance() {
        // Feed 20 returns with mean ~0, high std dev -> Volatile.
    }

    #[test]
    fn detects_regime_transition() {
        // Feed trending returns, then switch to ranging. Detector fires.
    }

    #[test]
    fn per_regime_performance_accumulates() {
        // Record 3 wins in TrendingUp, 2 losses in Ranging. Verify totals.
    }
}
```

### Verification

```bash
cargo test -p roko-learn -- regime
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `RegimeClassifier` assigns regime labels from a return window
- [ ] `RegimeTransitionDetector` fires when JSD between windows exceeds threshold
- [ ] `PerRegimePerformance` tracks (strategy, regime) -> performance records
- [ ] `RoutingContext` carries `regime: Option<MarketRegime>`
- [ ] Episode `extra` map can store `regime_label`
- [ ] All tests pass, clippy clean, fmt clean

### Commit message

```
feat(roko-learn): add regime classification and per-regime performance tracking
```

---

## Batch 7.4: Trading playbooks

> **Effort**: M | **Depends on**: 7.1 | **Crate**: roko-learn
> **Branch**: `defi/batch-7.4-trading-playbooks`

### Context

The playbook system at `playbook.rs:77` stores named action sequences proven to achieve a goal. `PlaybookStep` carries a description, action_kind discriminator, and expected_signals. `PlaybookStore` tracks success/failure counters via `record_outcome`. All current playbooks describe code-task patterns ("fix concurrency bug" = "replace HashMap with DashMap").

Trading playbooks capture strategy patterns: "mean-reversion entry when RSI < 30, funding negative, and volatility contracting." They need market-condition triggers instead of file-glob triggers, continuous P&L outcomes instead of binary success/failure, and per-(playbook, regime, asset) performance tracking.

The playbook rules system at `playbook_rules.rs:66` already supports if-then rules with `Triggers` (file_globs, tags, categories, error_signatures, roles). The gap is extending `Triggers` with market-condition trigger types.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/playbook.rs:37-97` | `PlaybookStep`, `Playbook`, `PlaybookStore` |
| `crates/roko-learn/src/playbook_rules.rs:34-100` | `Triggers`, `Rule`, confidence dynamics |
| `crates/roko-learn/src/trading_reflect.rs` | `ClosedPosition`, `TradingReflectEvent` from batch 7.1 |
| `crates/roko-learn/src/regime.rs` | `MarketRegime` from batch 7.3 (if available, otherwise use `Option<String>`) |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**7.4.1** Create `crates/roko-learn/src/trading_playbook.rs`

```rust
use serde::{Deserialize, Serialize};

/// Entry conditions for a trading playbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingEntryCondition {
    pub indicator: String,
    pub comparison: Comparison,
    pub threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparison { Lt, Lte, Gt, Gte, Eq }

/// Exit conditions: stop-loss, take-profit, trailing stop, time-based.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingExitCondition {
    pub kind: ExitKind,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitKind { StopLoss, TakeProfit, TrailingStop, TimeoutSecs }

/// A trading playbook: named strategy pattern with entry/exit conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPlaybook {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entry_conditions: Vec<TradingEntryCondition>,
    pub exit_conditions: Vec<TradingExitCondition>,
    pub preferred_regime: Option<String>,
    pub position_size_fraction: f64,
    pub total_pnl: f64,
    pub trade_count: u64,
    pub win_count: u64,
    pub created_at_ms: i64,
    pub last_used_ms: Option<i64>,
}
```

**7.4.2** Add `TradingPlaybookStore`:

```rust
/// JSON-file-backed store for trading playbooks.
#[derive(Debug)]
pub struct TradingPlaybookStore {
    dir: PathBuf,
}

impl TradingPlaybookStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self { ... }
    pub async fn save(&self, playbook: &TradingPlaybook) -> io::Result<()> { ... }
    pub async fn load(&self, id: &str) -> io::Result<Option<TradingPlaybook>> { ... }
    pub async fn list(&self) -> io::Result<Vec<TradingPlaybook>> { ... }

    /// Record a trade outcome against a playbook.
    pub async fn record_outcome(
        &self,
        playbook_id: &str,
        pnl: f64,
        won: bool,
    ) -> io::Result<()> { ... }

    /// Query playbooks suitable for current market conditions.
    pub fn query_for_conditions(
        &self,
        playbooks: &[TradingPlaybook],
        indicators: &[(String, f64)],
        regime: Option<&str>,
    ) -> Vec<&TradingPlaybook> { ... }
}
```

**7.4.3** Extend `Triggers` in `playbook_rules.rs:34` with a new field for market-condition triggers:

```rust
/// Market condition patterns matched against indicator readings.
#[serde(default)]
pub market_conditions: Vec<String>,
```

### Wiring

In `crates/roko-learn/src/lib.rs`, add:
```rust
/// Trading-specific playbook patterns with market-condition triggers.
pub mod trading_playbook;
```

### Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn playbook_matches_entry_conditions() {
        // RSI < 30 condition matches when RSI = 25.
    }

    #[test]
    fn record_outcome_updates_pnl_and_counters() {
        // Record a win with +50 P&L, verify counters.
    }

    #[test]
    fn query_filters_by_regime() {
        // Playbook with preferred_regime "TrendingUp" matches only that regime.
    }
}
```

### Verification

```bash
cargo test -p roko-learn -- trading_playbook
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `TradingPlaybook` stores entry/exit conditions, sizing, and outcome counters
- [ ] `TradingPlaybookStore` persists playbooks as JSON, supports `record_outcome`
- [ ] `query_for_conditions` matches playbooks against current indicators and regime
- [ ] `Triggers` in `playbook_rules.rs` extended with `market_conditions`
- [ ] All tests pass, clippy clean, fmt clean

### Commit message

```
feat(roko-learn): add trading playbooks with market-condition triggers
```

---

## Batch 7.5: Risk-adjusted reward signal

> **Effort**: M | **Depends on**: 7.1 | **Crate**: roko-learn
> **Branch**: `defi/batch-7.5-risk-adjusted-reward`

### Context

The reward signal flowing through the learning pipeline is binary: gate pass = 1.0, gate fail = 0.0. The `compute_routing_reward_v2` function at `model_router.rs:339` computes a weighted composite of gate success (quality 0.5), cost efficiency (0.3), and latency (0.2) via `RewardWeights` at `roko-core/src/config/schema.rs:2511`. None of these weights account for trading risk.

For DeFi, reward must incorporate P&L magnitude, risk-adjusted return (Sharpe/Sortino ratios), and maximum drawdown. This batch extends `RewardWeights` with trading-specific dimensions and provides risk-adjusted reward computation that the cascade router and bandits can use directly.

The `UcbBandit` at `bandits.rs:76` expects rewards in [0, 1] (see doc comment at line 53-57). The risk-adjusted reward must map into this range.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/model_router.rs:284-370` | `compute_routing_reward_v2`, `compute_routing_reward_with_weights` |
| `crates/roko-core/src/config/schema.rs:2511-2521` | `RewardWeights` struct: quality, cost, latency fields |
| `crates/roko-learn/src/bandits.rs:53-62` | UCB1 reward scaling note: rewards must be in [0, 1] |
| `crates/roko-learn/src/local_reward.rs:14-59` | `LocalRewardFunction` -- binary observe pattern |
| `crates/roko-learn/src/trading_reflect.rs` | `ClosedPosition`, `TradingReflectEvent` from batch 7.1 |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**7.5.1** Create `crates/roko-learn/src/risk_reward.rs`

```rust
use serde::{Deserialize, Serialize};

/// Rolling window of returns for risk-adjusted metric computation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReturnWindow {
    returns: Vec<f64>,
    max_size: usize,
}

impl ReturnWindow {
    pub fn new(max_size: usize) -> Self { ... }
    pub fn push(&mut self, r: f64) { ... }

    /// Arithmetic mean of returns.
    pub fn mean_return(&self) -> f64 {
        if self.returns.is_empty() { return 0.0; }
        self.returns.iter().sum::<f64>() / self.returns.len() as f64
    }

    /// Annualized Sharpe ratio. risk_free_rate is annual (e.g., 0.05 for 5%).
    /// periods_per_year depends on tick frequency (e.g., 8760 for hourly).
    pub fn sharpe_ratio(&self, risk_free_rate: f64, periods_per_year: f64) -> f64 {
        let mean = self.mean_return();
        let rf_per_period = risk_free_rate / periods_per_year;
        let excess = mean - rf_per_period;
        let std_dev = self.std_dev();
        if std_dev == 0.0 { return 0.0; }
        excess / std_dev * periods_per_year.sqrt()
    }

    /// Sortino ratio -- penalizes only downside deviation.
    pub fn sortino_ratio(&self, risk_free_rate: f64, periods_per_year: f64) -> f64 {
        let mean = self.mean_return();
        let rf_per_period = risk_free_rate / periods_per_year;
        let excess = mean - rf_per_period;
        let downside: f64 = self.returns.iter()
            .filter(|&&r| r < rf_per_period)
            .map(|r| (r - rf_per_period).powi(2))
            .sum::<f64>() / self.returns.len() as f64;
        let downside_dev = downside.sqrt();
        if downside_dev == 0.0 { return 0.0; }
        excess / downside_dev * periods_per_year.sqrt()
    }

    /// Maximum drawdown as a fraction (0.0 to 1.0).
    pub fn max_drawdown(&self) -> f64 {
        let mut peak = 1.0f64;
        let mut max_dd = 0.0f64;
        let mut equity = 1.0f64;
        for &r in &self.returns {
            equity *= 1.0 + r;
            peak = peak.max(equity);
            let dd = (peak - equity) / peak;
            max_dd = max_dd.max(dd);
        }
        max_dd
    }

    fn std_dev(&self) -> f64 {
        let mean = self.mean_return();
        let variance = self.returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / self.returns.len() as f64;
        variance.sqrt()
    }
}
```

**7.5.2** Add `TradingRewardWeights` and reward computation:

```rust
/// Extended reward weights for trading model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingRewardWeights {
    /// Weight for P&L magnitude.
    pub pnl: f64,
    /// Weight for Sharpe ratio.
    pub sharpe: f64,
    /// Weight for low max drawdown.
    pub drawdown: f64,
    /// Weight for execution cost (gas + slippage).
    pub execution_cost: f64,
}

impl Default for TradingRewardWeights {
    fn default() -> Self {
        Self { pnl: 0.3, sharpe: 0.35, drawdown: 0.2, execution_cost: 0.15 }
    }
}

/// Compute a risk-adjusted reward in [0.0, 1.0] for bandit/router consumption.
pub fn compute_trading_reward(
    normalized_pnl: f64,
    sharpe: f64,
    max_drawdown: f64,
    normalized_exec_cost: f64,
    weights: &TradingRewardWeights,
) -> f64 { ... }
```

**7.5.3** Extend `RewardWeights` in `roko-core/src/config/schema.rs:2511` with optional trading fields:

```rust
/// Optional weight for trading P&L (used when DeFi mode is active).
#[serde(default)]
pub trading_pnl: f64,
/// Optional weight for Sharpe ratio.
#[serde(default)]
pub trading_sharpe: f64,
```

**7.5.4** Wire `compute_trading_reward` as an alternative reward path in `LearningRuntime::record_trading_outcome` (from batch 7.1) -- when a `TradingReflectEvent` is processed, use the risk-adjusted reward instead of binary gate pass/fail.

### Wiring

In `crates/roko-learn/src/lib.rs`, add:
```rust
/// Risk-adjusted reward computation for trading (Sharpe, Sortino, drawdown).
pub mod risk_reward;
```

### Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn sharpe_ratio_positive_for_consistent_gains() {
        // Feed 20 returns of ~0.01 with small variance -> positive Sharpe.
    }

    #[test]
    fn sortino_ignores_upside_deviation() {
        // Same mean, different upside variance -> same Sortino.
    }

    #[test]
    fn max_drawdown_captures_peak_to_trough() {
        // Returns: +10%, +5%, -20%, +3% -> max drawdown ~20%.
    }

    #[test]
    fn trading_reward_in_unit_interval() {
        // Verify output is always in [0.0, 1.0] for various inputs.
    }
}
```

### Verification

```bash
cargo test -p roko-learn -- risk_reward
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `ReturnWindow` computes Sharpe, Sortino, and max drawdown from return series
- [ ] `TradingRewardWeights` provides configurable weight distribution
- [ ] `compute_trading_reward` produces reward in [0.0, 1.0] suitable for UCB1
- [ ] `RewardWeights` in roko-core extended with `trading_pnl` and `trading_sharpe` fields
- [ ] Risk-adjusted reward wired into `LearningRuntime::record_trading_outcome`
- [ ] All tests pass, clippy clean, fmt clean

### Commit message

```
feat(roko-learn): add Sharpe/Sortino risk-adjusted reward for trading
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives Used

- **Recipe**: `TradingReflect` (P&L attribution pipeline: fill events -> FIFO matching -> realized P&L), `FifoMatcher` (position matching transform), `IndicatorTracker` (Kalman-smoothed indicator accuracy pipeline), `RegimeDetector` (market regime classification pipeline), risk-adjusted reward pipeline (Sharpe/Sortino computation for UCB1 bandits)
- **Knowledge Entry**: `TradingPlaybook` (durable strategy patterns extracted from successful trades -- market condition triggers, optimal parameters)
- **Eval**: Strategy performance evals against benchmarks (buy-and-hold, equal-weight, risk-parity), indicator accuracy evals
- **Signal**: Risk-adjusted reward signals (continuous reward for bandit-based model routing), regime change signals

### Authoring Surfaces

- **Recipe Editor** -- build P&L attribution pipelines, indicator tracking, regime detection
- **Measurements > Evals** -- configure strategy evaluations with benchmark selection
- **Knowledge > Playbooks** -- browse/fork trading playbooks with performance history

### Shareable Artifacts

- P&L attribution recipe templates (FIFO, LIFO, average cost methods)
- Trading playbooks (market-condition -> strategy mappings)
- Regime detection recipes (classifier configurations for different markets)
- Eval configurations (benchmark suites for different asset classes)

### Dashboard Visibility

- **Measurements > Evals** -- strategy performance dashboards with benchmark comparison
- **Forge > Recipes** -- P&L attribution and indicator pipelines
- **Knowledge > Playbooks** -- playbook library with live performance metrics
- **Pulse > Learning** -- real-time reward signals, regime transitions
