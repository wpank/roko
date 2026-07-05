# 04 -- Safety and Risk Management: Agent-Executable Work Batches

> **Batch count**: 4 | **Total items**: ~28 | **Phase**: 0/1
> **Primary crate**: `roko-agent` (batches 4.1, 4.3), `roko-chain` (batch 4.2), `roko-conductor` (batch 4.4)

---

## Batch 4.1: DeFi risk limits and position tracking

> **Effort**: L | **Depends on**: 2.1 (VenueAdapter for position tracking) | **Crate**: roko-agent
> **Branch**: `defi/batch-4.1-risk-limits`

### Context

Roko's adaptive risk module in `roko-agent/src/safety/risk.rs` provides Kelly criterion position sizing, multi-dimension confidence tracking, and a 5-dimension safety budget (irreversibility, blast radius, footprint, uncertainty, cost). These are calibrated for code-generation tasks -- the irreversibility scale scores `rm -rf` at 0.8 and `cargo publish` at 0.9.

DeFi operations need different risk dimensions. A swap that loses 5% to slippage is not analogous to editing the wrong file. Position sizing needs to account for portfolio concentration, drawdown limits, and asset whitelist enforcement. The Kelly fraction math already exists (line 167 of `risk.rs`) but has no position data to operate on.

This batch adds a `DeFiRiskEngine` that wraps the existing `OperationalConfidenceTracker` and `SafetyBudgetTracker` with DeFi-specific dimensions: max position size per asset, aggregate portfolio exposure limits, drawdown enforcement, asset whitelisting, and trade rate limiting. The `PolicyCageConfig` in `roko-chain/src/heartbeat_ext.rs` (line 44) already defines `max_open_positions`, `max_position_size`, `max_daily_volume_usd`, and `approved_assets` -- the risk engine references these but adds the portfolio-level aggregation that the PolicyCage lacks.

**Mirage pre-trade simulation**: before executing a real trade, the risk engine should support a `simulate_trade` method that spawns an ephemeral mirage-rs instance, forks the current chain state, executes the proposed trade against the fork, and verifies risk limits hold post-execution. This catches issues that static `check_trade` misses -- slippage impact on portfolio concentration, gas cost exceeding budget, or revert conditions. The simulation step is optional (controlled by `DeFiRiskConfig::simulate_before_trade: bool`, default `true`).

```rust
/// Simulate a trade against a mirage-rs fork before live execution.
pub async fn simulate_trade(
    &self,
    rpc_url: &str,
    tx: &TxRequest,
) -> Result<SimulationResult, RiskViolation> {
    let sim = MirageSimulator::new(rpc_url).await
        .map_err(|e| RiskViolation::SimulationFailed(e.to_string()))?;

    let result = sim.simulate(tx).await
        .map_err(|e| RiskViolation::SimulationFailed(e.to_string()))?;

    if !result.success {
        return Err(RiskViolation::SimulationReverted);
    }

    // Check portfolio state post-trade against risk limits
    let mut projected = self.portfolio.clone();
    projected.apply_simulation_result(&result);
    self.check_limits(&projected)?;

    sim.shutdown().await.ok();
    Ok(SimulationResult {
        gas_used: result.gas_used,
        output_amount: result.output_amount(),
        new_portfolio_state: projected,
        success: true,
    })
}
```

### Read first

| File | Why |
|------|-----|
| `crates/roko-agent/src/safety/risk.rs` | Lines 16-22: `BetaDistribution` for confidence tracking. Lines 72-146: `OperationalConfidenceTracker` with multi-dimension posteriors. Lines 167-175: `kelly_fraction()`. Lines 204-219: `effective_limit()`. Lines 222-481: `SafetyBudget` and `SafetyBudgetTracker` with 5-dimension check/consume. |
| `crates/roko-chain/src/heartbeat_ext.rs` | Lines 44-58: `PolicyCageConfig` with `max_open_positions`, `max_position_size`, `max_daily_volume_usd`, `approved_assets`, `max_gas_gwei`. Lines 74-82: `PolicyCageState` tracking `open_positions`, `daily_volume_usd`, `current_gas_gwei`. Lines 227-311: `validate()` method checking each constraint. |
| `crates/roko-agent/src/safety/mod.rs` | Lines 1-27: module architecture doc. Lines 29-47: submodule declarations. New module goes here. |
| `crates/roko-agent/src/safety/contract.rs` | Lines 24-39: `AgentContract` struct. Lines 42-52: `permissive()` fallback. Lines 94-106: `check_pre_execution()` validates tool calls against invariants and governance rules. DeFi role contracts (trader, hedger) will extend this. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-agent/src/safety/defi_risk.rs`**. Module-level doc: `//! DeFi risk engine: position limits, exposure caps, drawdown enforcement.`

2. **Define `DeFiRiskConfig`**. Config struct controlling all risk parameters:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeFiRiskConfig {
    /// Maximum fraction of portfolio in a single position.
    pub max_position_pct: f64,            // default 0.25 (25%)
    /// Maximum number of simultaneous positions.
    pub max_open_positions: usize,         // default 5
    /// Maximum aggregate notional exposure in USD.
    pub max_aggregate_exposure_usd: f64,   // default 100_000.0
    /// Drawdown warning threshold (fraction of portfolio peak).
    pub drawdown_warn_pct: f64,            // default 0.10 (10%)
    /// Drawdown halt threshold (fraction of portfolio peak).
    pub drawdown_halt_pct: f64,            // default 0.20 (20%)
    /// Approved asset addresses (empty = all allowed).
    pub approved_assets: HashSet<String>,
    /// Maximum trades per hour per agent.
    pub max_trades_per_hour: u32,          // default 20
    /// Maximum acceptable slippage in basis points.
    pub max_slippage_bps: u32,             // default 100 (1%)
    /// Whether to run mirage-rs fork simulation before each trade.
    pub simulate_before_trade: bool,       // default true
}
```

**`SimulationResult`** -- returned by `simulate_trade()`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub gas_used: u64,
    pub output_amount: u128,
    pub new_portfolio_state: PortfolioState,
    pub success: bool,
}
```

3. **Define `PortfolioState`**. Live portfolio tracking:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortfolioState {
    /// Current positions keyed by asset address.
    pub positions: HashMap<String, Position>,
    /// Portfolio high-water mark in USD (for drawdown calculation).
    pub peak_value_usd: f64,
    /// Current portfolio value in USD.
    pub current_value_usd: f64,
    /// Trades executed in the current hour.
    pub trades_this_hour: u32,
    /// Timestamp of the hour boundary.
    pub hour_start_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Asset address.
    pub asset: String,
    /// Current size in base units.
    pub size: f64,
    /// Average entry price in USD.
    pub entry_price_usd: f64,
    /// Current mark price in USD.
    pub mark_price_usd: f64,
    /// Notional value in USD (size * mark_price).
    pub notional_usd: f64,
}
```

4. **Implement `DeFiRiskEngine`**. Core struct holding config + state:

```rust
pub struct DeFiRiskEngine {
    config: DeFiRiskConfig,
    state: parking_lot::RwLock<PortfolioState>,
    confidence: parking_lot::RwLock<OperationalConfidenceTracker>,
}
```

Methods:
- `check_trade(&self, asset: &str, size: f64, price: f64) -> Result<(), RiskViolation>` -- validates asset whitelist, position size limit, aggregate exposure, rate limit, drawdown.
- `record_trade(&self, asset: &str, size: f64, price: f64)` -- updates portfolio state.
- `record_fill(&self, asset: &str, fill_price: f64, expected_price: f64)` -- tracks slippage for confidence updates.
- `update_marks(&self, prices: &HashMap<String, f64>)` -- mark-to-market all positions.
- `drawdown_pct(&self) -> f64` -- current drawdown from peak.
- `is_halted(&self) -> bool` -- true if drawdown exceeds halt threshold.
- `kelly_position_size(&self, asset: &str, win_rate: f64, payoff: f64) -> f64` -- Kelly-optimal position size using `kelly_fraction()` from `risk.rs` (line 167), clamped by `max_position_pct`.

5. **Define `RiskViolation` enum**. Return type for `check_trade()`:

```rust
#[derive(Debug, Clone, Error)]
pub enum RiskViolation {
    #[error("asset {asset} not in approved list")]
    UnapprovedAsset { asset: String },
    #[error("position would be {pct:.1}% of portfolio, max {max:.1}%")]
    PositionTooLarge { pct: f64, max: f64 },
    #[error("would exceed max aggregate exposure: {total:.0} > {max:.0} USD")]
    ExposureLimitExceeded { total: f64, max: f64 },
    #[error("max open positions reached: {count} >= {max}")]
    TooManyPositions { count: usize, max: usize },
    #[error("drawdown halt: {pct:.1}% exceeds {max:.1}%")]
    DrawdownHalt { pct: f64, max: f64 },
    #[error("rate limit: {count} trades this hour, max {max}")]
    RateLimitExceeded { count: u32, max: u32 },
    #[error("simulation reverted: trade would fail on-chain")]
    SimulationReverted,
    #[error("simulation failed: {0}")]
    SimulationFailed(String),
}
```

6. **Wire DeFi confidence dimensions**. Register dimensions in `OperationalConfidenceTracker`:
- `"trade_execution"` -- success = fill within slippage tolerance
- `"position_pnl"` -- success = position closed in profit
- `"slippage_accuracy"` -- success = actual slippage < predicted

Call `record_success` / `record_failure` from `record_fill()`.

7. **Add DeFi role contracts**. Create `crates/roko-agent/src/safety/contracts/trader.yaml` and `hedger.yaml`:

```json
{
  "role": "trader",
  "invariants": [
    { "MaxTokensPerTurn": 8000 }
  ],
  "governance": [
    { "MaxToolCallsPerTurn": 10 },
    { "ForbiddenTools": ["cargo_publish", "git_push"] },
    { "MaxCostPerTurn": 5.0 }
  ],
  "recovery": [
    { "trigger": "contract_violation", "action": "Abort" }
  ]
}
```

### Wiring

Add to `crates/roko-agent/src/safety/mod.rs`:
```rust
pub mod defi_risk;
```

### Tests

```rust
#[test]
fn check_trade_approved_asset() {
    let config = DeFiRiskConfig {
        approved_assets: ["0xweth".into()].into_iter().collect(),
        ..Default::default()
    };
    let engine = DeFiRiskEngine::new(config);
    assert!(engine.check_trade("0xweth", 1.0, 2000.0).is_ok());
    assert!(engine.check_trade("0xshitcoin", 1.0, 1.0).is_err());
}

#[test]
fn check_trade_position_size_limit() {
    let config = DeFiRiskConfig {
        max_position_pct: 0.25,
        ..Default::default()
    };
    let engine = DeFiRiskEngine::new(config);
    // Set portfolio value to 100k
    engine.state.write().current_value_usd = 100_000.0;
    engine.state.write().peak_value_usd = 100_000.0;
    // 30% position should fail
    let result = engine.check_trade("0xeth", 15.0, 2000.0);
    assert!(matches!(result, Err(RiskViolation::PositionTooLarge { .. })));
}

#[test]
fn drawdown_triggers_halt() {
    let config = DeFiRiskConfig {
        drawdown_halt_pct: 0.20,
        ..Default::default()
    };
    let engine = DeFiRiskEngine::new(config);
    {
        let mut state = engine.state.write();
        state.peak_value_usd = 100_000.0;
        state.current_value_usd = 75_000.0; // 25% drawdown
    }
    assert!(engine.is_halted());
    assert!(engine.check_trade("0xeth", 1.0, 2000.0).is_err());
}

#[test]
fn kelly_position_size_respects_cap() {
    let config = DeFiRiskConfig {
        max_position_pct: 0.25,
        ..Default::default()
    };
    let engine = DeFiRiskEngine::new(config);
    engine.state.write().current_value_usd = 100_000.0;
    // Perfect win rate, high payoff -> Kelly wants 100%, but cap at 25%
    let size = engine.kelly_position_size("0xeth", 1.0, 10.0);
    assert!(size <= 25_000.0, "Kelly size should be capped at 25% of portfolio");
}

#[test]
fn rate_limit_blocks_after_threshold() {
    let config = DeFiRiskConfig {
        max_trades_per_hour: 5,
        ..Default::default()
    };
    let engine = DeFiRiskEngine::new(config);
    engine.state.write().current_value_usd = 100_000.0;
    engine.state.write().peak_value_usd = 100_000.0;
    for _ in 0..5 {
        engine.record_trade("0xeth", 0.1, 2000.0);
    }
    let result = engine.check_trade("0xeth", 0.1, 2000.0);
    assert!(matches!(result, Err(RiskViolation::RateLimitExceeded { .. })));
}
```

### Verification

```bash
cargo test -p roko-agent -- safety::defi_risk
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-agent
```

### Acceptance criteria

- [ ] `defi_risk.rs` module created and declared in `safety/mod.rs`
- [ ] `DeFiRiskConfig` with all 8 parameters
- [ ] `PortfolioState` with positions, peak, current value, rate tracking
- [ ] `DeFiRiskEngine` validates trades against all constraints
- [ ] `RiskViolation` enum covers 6 failure modes
- [ ] Kelly position sizing capped by `max_position_pct`
- [ ] Drawdown halt stops all trading at threshold
- [ ] Rate limiter enforces trades-per-hour cap
- [ ] DeFi confidence dimensions wired into existing `OperationalConfidenceTracker`
- [ ] `trader.yaml` and `hedger.yaml` contract files created
- [ ] `simulate_trade` method spawns ephemeral mirage-rs fork, executes proposed trade, verifies risk limits post-execution
- [ ] `DeFiRiskConfig::simulate_before_trade` flag controls whether simulation runs (default `true`)
- [ ] 10+ tests pass

### Commit message

```
feat(roko-agent): add DeFi risk engine with position limits, drawdown, and Kelly sizing
```

---

## Batch 4.2: MEV protection pipeline

> **Effort**: L | **Depends on**: 1.2 (WS subscription for mempool) | **Crate**: roko-chain
> **Branch**: `defi/batch-4.2-mev-protection`

### Context

Roko's `MevGate` in `roko-chain/src/gate/mev_gate.rs` detects five MEV attack patterns (sandwich, front-run, back-run, JIT liquidity, cyclic arb). It operates as a standalone gate -- it takes a `MevAnalysisInput` containing the victim tx and mempool context, runs detection, and returns a verdict. The gate passes or fails based on severity.

The gap is that detection alone does not protect. Protection requires submitting transactions through private channels that exclude the public mempool. Flashbots Protect and MEV Blocker are the two primary services: they accept transactions via a special RPC endpoint and include them in blocks without public mempool exposure.

This batch adds a `MevProtector` module that wraps transaction submission with: (1) mempool analysis using the existing `MevDetector`, (2) private submission via Flashbots Protect RPC when risk is detected, (3) automatic slippage calculation from pool state, and (4) historical MEV rate tracking for the on-chain indicators in batch 3.4.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/gate/mev_gate.rs` | Lines 49-78: `MempoolTx` struct. Lines 81-88: `MevAnalysisInput`. Lines 219-260: `MevDetector::detect()` -- reuse this directly. Lines 517-528: `classify_severity()`. Lines 531-540: `has_critical()` / `has_warnings()` helpers. |
| `crates/roko-chain/src/gate/wallet_gate.rs` | Lines 65-69: `WalletGate` struct holding `Arc<dyn ChainWallet>` + `Arc<dyn ChainClient>`. Lines 185-192: `needed_wei()` for gas cost computation. The MEV protector needs the same wallet/client access pattern. |
| `crates/roko-chain/src/heartbeat_ext.rs` | Lines 151-163: `ChainHeartbeatExtension` holds `Arc<dyn TxSimulator>`. The MEV protector fits between SIMULATE and SIGN in this pipeline. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-chain/src/mev_protection.rs`**. Module-level doc: `//! MEV protection pipeline: detection, private submission, and slippage control.`

2. **Define `MevProtectorConfig`**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevProtectorConfig {
    /// Flashbots Protect RPC URL (e.g. "https://rpc.flashbots.net").
    pub private_rpc_url: Option<String>,
    /// MEV Blocker RPC URL as fallback.
    pub mev_blocker_url: Option<String>,
    /// Whether to always use private submission (even without detected MEV).
    pub always_private: bool,
    /// Maximum acceptable slippage in basis points (auto-calculated if 0).
    pub max_slippage_bps: u32,
    /// Detector configuration (passed through to MevDetector).
    pub detector_config: MevDetectorConfig,
}
```

3. **Define `SubmissionStrategy` enum**:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmissionStrategy {
    /// Submit via public mempool (no MEV risk detected).
    PublicMempool,
    /// Submit via Flashbots Protect (MEV risk or always_private).
    FlashbotsProtect,
    /// Submit via MEV Blocker (fallback if Flashbots unavailable).
    MevBlocker,
    /// Do not submit (critical MEV risk, no private channel available).
    Abort { reason: String },
}
```

4. **Implement `MevProtector`**:

```rust
pub struct MevProtector {
    config: MevProtectorConfig,
    detector: MevDetector,
    /// Rolling history of MEV alerts for rate tracking.
    alert_history: parking_lot::RwLock<VecDeque<MevAlertRecord>>,
    /// Maximum alert history entries.
    max_history: usize,
}
```

Methods:
- `analyze_and_decide(&self, tx: &TxRequest, mempool_txs: &[MempoolTx]) -> (SubmissionStrategy, Vec<MevAlert>)` -- runs the detector, decides submission strategy based on severity and available channels.
- `calculate_min_output(&self, pool_state: &PoolState, input_amount: u128, slippage_bps: u32) -> u128` -- computes minimum acceptable output from pool math. `PoolState` is a new input type: `PoolState { reserve0: u128, reserve1: u128, fee_bps: u32, tick: Option<i32> }`.
- `record_alert(&self, alerts: &[MevAlert])` -- appends to `alert_history` for rate tracking.
- `mev_rate(&self, window_ms: i64) -> f64` -- returns alerts-per-tx rate over the window.

5. **Define `MevAlertRecord`** for history tracking:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevAlertRecord {
    pub timestamp_ms: i64,
    pub pattern: MevPattern,
    pub severity: MevSeverity,
    pub estimated_profit_wei: u128,
}
```

6. **Define `PoolState`** for slippage calculation input:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub reserve0: u128,
    pub reserve1: u128,
    pub fee_bps: u32,
    pub tick: Option<i32>,
    pub sqrt_price_x96: Option<u128>,
}
```

**Slippage calculation methods on `PoolState`**:

```rust
impl PoolState {
    /// Constant-product AMM output calculation.
    /// output = (input * (10000 - fee_bps) * reserve_out) / (reserve_in * 10000 + input * (10000 - fee_bps))
    pub fn calculate_output(&self, amount_in: u128, is_token0_in: bool) -> u128 {
        let (reserve_in, reserve_out) = if is_token0_in {
            (self.reserve0, self.reserve1)
        } else {
            (self.reserve1, self.reserve0)
        };
        let fee_factor = 10_000u128 - self.fee_bps as u128;
        let numerator = amount_in * fee_factor * reserve_out;
        let denominator = reserve_in * 10_000 + amount_in * fee_factor;
        numerator / denominator
    }

    /// Calculate minimum output given max slippage in basis points.
    pub fn min_output_with_slippage(&self, amount_in: u128, is_token0_in: bool, max_slippage_bps: u32) -> u128 {
        let expected = self.calculate_output(amount_in, is_token0_in);
        expected * (10_000 - max_slippage_bps as u128) / 10_000
    }
}
```

`MevProtector::calculate_min_output` delegates to `PoolState::min_output_with_slippage`. For V3 pools with a `sqrt_price_x96` value, the spot price from reserves is used as an approximation -- full tick-aware math is deferred to the V3-specific venue adapter.

7. **Implement slippage auto-calculation**. For constant-product AMMs (Uniswap V2 style): `output = (input * fee_factor * reserve_out) / (reserve_in + input * fee_factor)`. Minimum output = output * (1 - slippage_bps / 10000). For Uniswap V3 (tick-based), the calculation requires the sqrt price and tick spacing -- provide a simplified version that uses the spot price from reserves and applies the slippage tolerance.

8. **Wire into `ChainHeartbeatExtension`**. Add an optional `MevProtector` field to `ChainHeartbeatExtension` (line 151 of `heartbeat_ext.rs`). In `pre_act_check()`, after SIMULATE passes, run the MEV analysis. If the strategy is `Abort`, fail the pre-act check. If `FlashbotsProtect` or `MevBlocker`, annotate the `ChainPreActResult` with the strategy so the caller knows which RPC to use. Do NOT modify the `pre_act_check` signature -- add strategy to `ChainPreActResult`.

### Wiring

Add to `crates/roko-chain/src/lib.rs` (or the appropriate module root):
```rust
pub mod mev_protection;
```

Add `submission_strategy: Option<SubmissionStrategy>` field to `ChainPreActResult` in `heartbeat_ext.rs` (line 133).

### Tests

```rust
#[test]
fn no_mev_uses_public_mempool() {
    let protector = MevProtector::new(MevProtectorConfig::default());
    let tx = TxRequest::default();
    let (strategy, alerts) = protector.analyze_and_decide(&tx, &[]);
    assert_eq!(strategy, SubmissionStrategy::PublicMempool);
    assert!(alerts.is_empty());
}

#[test]
fn critical_mev_with_flashbots_uses_protect() {
    let protector = MevProtector::new(MevProtectorConfig {
        private_rpc_url: Some("https://rpc.flashbots.net".into()),
        ..Default::default()
    });
    let tx = TxRequest::default();
    // Construct mempool with sandwich attack
    let mempool = vec![/* sandwich txs targeting the victim */];
    let (strategy, _alerts) = protector.analyze_and_decide(&tx, &mempool);
    // With a private RPC available, should use it
    assert!(matches!(strategy, SubmissionStrategy::FlashbotsProtect | SubmissionStrategy::PublicMempool));
}

#[test]
fn always_private_forces_flashbots() {
    let protector = MevProtector::new(MevProtectorConfig {
        private_rpc_url: Some("https://rpc.flashbots.net".into()),
        always_private: true,
        ..Default::default()
    });
    let tx = TxRequest::default();
    let (strategy, _) = protector.analyze_and_decide(&tx, &[]);
    assert_eq!(strategy, SubmissionStrategy::FlashbotsProtect);
}

#[test]
fn slippage_calculation_constant_product() {
    let protector = MevProtector::new(MevProtectorConfig::default());
    let pool = PoolState {
        reserve0: 1_000_000, reserve1: 2_000_000_000,
        fee_bps: 30, tick: None,
    };
    let min_out = protector.calculate_min_output(&pool, 1000, 100);
    assert!(min_out > 0, "should produce positive minimum output");
    // With 1% slippage, min_out should be ~99% of expected
    let no_slip = protector.calculate_min_output(&pool, 1000, 0);
    assert!(min_out < no_slip, "slippage should reduce minimum output");
}

#[test]
fn mev_rate_tracks_history() {
    let protector = MevProtector::new(MevProtectorConfig::default());
    let alerts = vec![MevAlert {
        pattern: MevPattern::Sandwich,
        severity: MevSeverity::Critical,
        description: "test".into(),
        involved_txs: vec![],
        estimated_profit_wei: 1000,
        sandwich: None,
    }];
    protector.record_alert(&alerts);
    assert!(protector.mev_rate(60_000) > 0.0);
}
```

### Verification

```bash
cargo test -p roko-chain -- mev_protection
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `mev_protection.rs` module created
- [ ] `MevProtectorConfig` with private RPC URLs and slippage settings
- [ ] `SubmissionStrategy` enum with 4 variants
- [ ] `MevProtector::analyze_and_decide()` runs detection and picks strategy
- [ ] Slippage auto-calculation for constant-product AMMs
- [ ] MEV rate history tracking via `VecDeque<MevAlertRecord>`
- [ ] `ChainPreActResult` extended with optional `SubmissionStrategy`
- [ ] `ChainHeartbeatExtension` runs MEV analysis when protector is configured
- [ ] 8+ tests pass

### Commit message

```
feat(roko-chain): add MEV protection pipeline with Flashbots and auto-slippage
```

---

## Batch 4.3: Custody controls and transaction lifecycle

> **Effort**: L | **Depends on**: 4.1 (risk engine for position tracking) | **Crate**: roko-chain
> **Branch**: `defi/batch-4.3-custody-lifecycle`

### Context

Roko has a `ChainWallet` trait with `address()`, `sign_tx()`, `nonce()`, `balance()` and three implementations: `LocalWallet` (dev), `MockWallet` (test), `EnvWallet` (from env var). The `WalletGate` at `roko-chain/src/gate/wallet_gate.rs` verifies balance and nonce before signing.

The gap is a transaction lifecycle state machine that sequences all the existing gates into a pipeline. The PRD specifies: IDLE -> PLANNING -> SIMULATING -> SIGNING -> BROADCASTING -> CONFIRMING -> IDLE, with error states at each transition. Roko has the individual gates (WalletGate, TxSimGate, MevGate) but no orchestration between them.

This batch builds a `TxLifecycle` state machine that: (1) validates against the DeFi risk engine (batch 4.1) before planning, (2) simulates via `ChainHeartbeatExtension`, (3) runs MEV analysis via `MevProtector` (batch 4.2), (4) signs via the wallet, (5) broadcasts via the appropriate RPC, and (6) confirms inclusion and updates portfolio state. Each transition has explicit error handling and timeout logic.

**SIMULATE phase with mirage-rs**: the SIMULATING state should support two simulation modes. The fast path uses `ChainHeartbeatExtension::pre_act_check()` which calls `eth_call` for gas estimation and basic validation. The full path spawns an ephemeral mirage-rs fork, executes the transaction against real forked state, and inspects the full execution trace -- storage diffs, event emissions, gas consumption, and revert reasons. The full path catches issues that `eth_call` misses (state-dependent reverts, multi-step transaction effects). Configure via `TxLifecycleConfig::simulation_mode: SimulationMode` with variants `FastOnly`, `FullFork`, and `FastThenFork` (default: run fast first, escalate to full fork if the transaction value exceeds a threshold).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationMode {
    /// eth_call only (fast, no fork).
    FastOnly,
    /// Full mirage-rs fork execution.
    FullFork,
    /// eth_call first; escalate to full fork if value > threshold.
    FastThenFork { value_threshold_usd: f64 },
}
```

**`TxLifecycleConfig`** -- controls simulation behavior for the lifecycle:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLifecycleConfig {
    /// Simulation mode: FastOnly (eth_call only), FullFork (spawn mirage), or FastThenFork (try fast, escalate if high value).
    pub simulation_mode: SimulationMode,
    /// RPC URL for mirage fork (used in FullFork and FastThenFork modes).
    pub fork_rpc_url: Option<String>,
    /// Value threshold in USD above which FastThenFork escalates to FullFork.
    pub full_fork_threshold_usd: f64,  // default: 1000.0
    /// Maximum time to wait for simulation before proceeding (seconds).
    pub simulation_timeout_secs: u64,  // default: 10
    /// Whether to block on simulation result (true) or fire-and-forget (false).
    pub blocking_simulation: bool,     // default: true
}
```

The `simulate()` method on `TxLifecycle` uses `SimulationMode` to select the execution path:

```rust
async fn simulate(&self, config: &TxLifecycleConfig, tx: &TxRequest) -> Result<SimulateResult> {
    match config.simulation_mode {
        SimulationMode::FastOnly => {
            // eth_call against live RPC -- fast but no state isolation
            self.chain_client.eth_call(tx).await
        }
        SimulationMode::FullFork => {
            // Spawn ephemeral mirage-rs fork -- slow but accurate
            let rpc = config.fork_rpc_url.as_deref().unwrap_or(&self.default_rpc);
            let sim = MirageSimulator::new(rpc).await?;
            let result = sim.simulate(tx).await?;
            sim.shutdown().await.ok();
            Ok(result)
        }
        SimulationMode::FastThenFork => {
            // Try fast first; if tx value > threshold, also do full fork
            let fast_result = self.chain_client.eth_call(tx).await?;
            if tx.estimated_value_usd() > config.full_fork_threshold_usd {
                let rpc = config.fork_rpc_url.as_deref().unwrap_or(&self.default_rpc);
                let sim = MirageSimulator::new(rpc).await?;
                let fork_result = sim.simulate(tx).await?;
                sim.shutdown().await.ok();
                // Use fork result (more accurate) but log if it disagrees with fast
                if fast_result.success != fork_result.success {
                    tracing::warn!("fast vs fork disagreement on tx simulation");
                }
                Ok(fork_result)
            } else {
                Ok(fast_result)
            }
        }
    }
}
```

This batch does NOT implement on-chain smart contract enforcement (PolicyCage) or MPC key management -- those are XL items deferred to a later phase.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/gate/wallet_gate.rs` | Lines 65-69: `WalletGate` struct. Lines 146-178: `check()` method for balance. Lines 254-312: `Gate::verify()` implementation with nonce checking. |
| `crates/roko-chain/src/heartbeat_ext.rs` | Lines 151-163: `ChainHeartbeatExtension` with `pre_act_check()`. Lines 168-194: `pre_act_check()` runs SIMULATE then VALIDATE. |
| `crates/roko-chain/src/gate/mev_gate.rs` | Lines 560-566: `MevGate` struct. Used as part of the lifecycle. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-chain/src/tx_lifecycle.rs`**. Module-level doc: `//! Transaction lifecycle state machine: PLAN -> SIMULATE -> SIGN -> BROADCAST -> CONFIRM.`

2. **Define `TxState` enum**:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxState {
    Idle,
    Planning,
    Simulating,
    Signing,
    Broadcasting,
    Confirming { tx_hash: String },
    Completed { tx_hash: String, block_number: u64 },
    Failed { state: Box<TxState>, reason: String },
}
```

3. **Define `TxLifecycleEvent`** for audit logging:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLifecycleEvent {
    pub tx_id: String,
    pub from_state: TxState,
    pub to_state: TxState,
    pub timestamp_ms: i64,
    pub details: Option<String>,
}
```

4. **Implement `TxLifecycle`**:

```rust
pub struct TxLifecycle {
    /// Current state of the transaction.
    state: parking_lot::RwLock<TxState>,
    /// Transaction ID for correlation.
    tx_id: String,
    /// Event log for audit trail.
    events: parking_lot::RwLock<Vec<TxLifecycleEvent>>,
    /// Timeout for each state (ms).
    state_timeout_ms: HashMap<String, u64>,
}
```

Methods:
- `fn new(tx_id: impl Into<String>) -> Self` -- starts in `Idle` state.
- `async fn plan(&self, tx: &TxRequest, risk_check: impl FnOnce(&TxRequest) -> Result<(), String>) -> Result<(), TxLifecycleError>` -- transition Idle -> Planning -> Simulating. Calls the risk check function. Returns error if risk check fails (-> Failed state).
- `async fn simulate(&self, ext: &ChainHeartbeatExtension, tx: &TxRequest, cage_state: &PolicyCageState) -> Result<ChainPreActResult, TxLifecycleError>` -- transition Simulating -> Signing. Calls `ext.pre_act_check()`. If simulation fails, -> Failed state.
- `async fn sign(&self, wallet: &dyn ChainWallet, tx: &TxRequest) -> Result<Vec<u8>, TxLifecycleError>` -- transition Signing -> Broadcasting. Calls `wallet.sign_tx()`. If signing fails, -> Failed state.
- `async fn broadcast(&self, client: &dyn ChainClient, signed_tx: &[u8]) -> Result<String, TxLifecycleError>` -- transition Broadcasting -> Confirming. Returns tx hash.
- `async fn confirm(&self, client: &dyn ChainClient, timeout_ms: u64) -> Result<TxReceipt, TxLifecycleError>` -- transition Confirming -> Completed. Polls for receipt up to timeout. If timeout, -> Failed state.
- `fn state(&self) -> TxState` -- read current state.
- `fn events(&self) -> Vec<TxLifecycleEvent>` -- read audit log.
- `fn is_terminal(&self) -> bool` -- true if Completed or Failed.

5. **Define `TxLifecycleError`**:

```rust
#[derive(Debug, Clone, Error)]
pub enum TxLifecycleError {
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidTransition { from: TxState, to: TxState },
    #[error("risk check failed: {reason}")]
    RiskCheckFailed { reason: String },
    #[error("simulation failed: {reason}")]
    SimulationFailed { reason: String },
    #[error("signing failed: {reason}")]
    SigningFailed { reason: String },
    #[error("broadcast failed: {reason}")]
    BroadcastFailed { reason: String },
    #[error("confirmation timeout after {timeout_ms}ms")]
    ConfirmationTimeout { timeout_ms: u64 },
    #[error("transaction reverted: {reason}")]
    Reverted { reason: String },
}
```

6. **Define `TxReceipt`** (simplified):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxReceipt {
    pub tx_hash: String,
    pub block_number: u64,
    pub gas_used: u64,
    pub success: bool,
    pub logs: Vec<String>,
}
```

7. **Add state transition validation**. Each transition method checks that the current state is the expected predecessor. Transitioning from the wrong state returns `TxLifecycleError::InvalidTransition`. Every transition logs a `TxLifecycleEvent` to the audit trail.

### Wiring

Add to `crates/roko-chain/src/lib.rs` (or the appropriate module root):
```rust
pub mod tx_lifecycle;
```

### Tests

```rust
#[test]
fn lifecycle_starts_idle() {
    let lc = TxLifecycle::new("tx-001");
    assert_eq!(lc.state(), TxState::Idle);
}

#[tokio::test]
async fn lifecycle_plan_transitions_to_simulating() {
    let lc = TxLifecycle::new("tx-002");
    let tx = TxRequest::default();
    lc.plan(&tx, |_| Ok(())).await.unwrap();
    assert_eq!(lc.state(), TxState::Simulating);
    assert_eq!(lc.events().len(), 2); // Idle->Planning, Planning->Simulating
}

#[tokio::test]
async fn lifecycle_plan_risk_failure_goes_to_failed() {
    let lc = TxLifecycle::new("tx-003");
    let tx = TxRequest::default();
    let result = lc.plan(&tx, |_| Err("position too large".into())).await;
    assert!(result.is_err());
    assert!(matches!(lc.state(), TxState::Failed { .. }));
}

#[test]
fn lifecycle_invalid_transition() {
    let lc = TxLifecycle::new("tx-004");
    // Cannot sign from Idle state
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let wallet = MockChainWallet::funded(1000);
        lc.sign(&wallet, &TxRequest::default()).await
    });
    assert!(matches!(result, Err(TxLifecycleError::InvalidTransition { .. })));
}

#[test]
fn lifecycle_events_logged() {
    let lc = TxLifecycle::new("tx-005");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        lc.plan(&TxRequest::default(), |_| Ok(())).await.unwrap();
    });
    let events = lc.events();
    assert!(events.len() >= 2);
    assert_eq!(events[0].from_state, TxState::Idle);
    assert_eq!(events[0].to_state, TxState::Planning);
}
```

### Verification

```bash
cargo test -p roko-chain -- tx_lifecycle
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `tx_lifecycle.rs` module created
- [ ] `TxState` enum with 7 states (Idle through Failed)
- [ ] `TxLifecycle` enforces valid state transitions
- [ ] Each transition logs a `TxLifecycleEvent`
- [ ] Risk check failure transitions to Failed
- [ ] Simulation failure transitions to Failed
- [ ] Invalid transitions return `TxLifecycleError::InvalidTransition`
- [ ] `TxReceipt` captures on-chain confirmation data
- [ ] `is_terminal()` correctly identifies Completed and Failed
- [ ] `SimulationMode` enum with `FastOnly`, `FullFork`, and `FastThenFork` variants
- [ ] SIMULATING state supports mirage-rs full fork execution when configured
- [ ] 8+ tests pass

### Commit message

```
feat(roko-chain): add transaction lifecycle state machine with audit trail
```

---

## Batch 4.4: DeFi circuit breakers

> **Effort**: M | **Depends on**: 3.1 (needs indicator readings for triggers) | **Crate**: roko-conductor
> **Branch**: `defi/batch-4.4-defi-circuit-breakers`

### Context

Roko's conductor crate provides a circuit breaker at `roko-conductor/src/circuit_breaker.rs`. The `CircuitBreaker` struct tracks per-plan failure counts with DashMap for lock-free concurrent access. It trips when failures reach `MAX_PLAN_FAILURES` (default 2). The Holt exponential smoothing forecaster (lines 44-104) projects error rates and proactively trips the breaker before the count threshold.

This circuit breaker is domain-agnostic -- it counts plan failures regardless of cause. For DeFi, circuit breakers need market-condition triggers: price dislocations, liquidity crises, gas spikes, oracle failures, cascading liquidations, portfolio drawdown, and stablecoin depeg events. These are structurally different from plan-failure counting because they fire on external market conditions, not on agent execution errors.

The conductor already has 10 specialized watchers (stuck detection, cost anomalies, error patterns, throughput) that produce `ConductorDecision` values (Continue, Intervene, Abort). This batch adds DeFi-specific watchers that plug into the same framework.

### Read first

| File | Why |
|------|-----|
| `crates/roko-conductor/src/circuit_breaker.rs` | Lines 13-19: `MAX_PLAN_FAILURES` constant. Lines 21-30: `FailureRecord` struct. Lines 44-104: `HoltForecaster`. Lines 146-161: `CircuitBreaker` struct fields. Lines 169-181: `new()` constructor. Lines 214-243: `record_failure()` with count-based and predictive trip. Lines 276-304: `check_proactive()` returns `ProactiveTripSignal`. |
| `crates/roko-conductor/src/conductor.rs` | Lines 59-77: `Conductor` struct. Lines 1-19: imports including watchers. The conductor runs all watchers and merges results. |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

1. **Create `crates/roko-conductor/src/defi_breakers.rs`**. Module-level doc: `//! DeFi-specific circuit breakers: market condition triggers for trading halt.`

2. **Define `DeFiBreaker` trait**. Each breaker checks a market condition and returns whether to halt:

```rust
pub trait DeFiBreaker: Send + Sync {
    /// Name of this breaker (for logging).
    fn name(&self) -> &str;
    /// Check the current market condition. Returns `Some(reason)` if halted.
    fn check(&self, snapshot: &MarketSnapshot) -> Option<String>;
}
```

3. **Define `MarketSnapshot`** as the common input:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketSnapshot {
    /// Current price per asset (keyed by address or symbol).
    pub prices: HashMap<String, f64>,
    /// Previous prices (one period ago).
    pub prev_prices: HashMap<String, f64>,
    /// Pool TVL per pool address.
    pub pool_tvl: HashMap<String, f64>,
    /// Previous pool TVL.
    pub prev_pool_tvl: HashMap<String, f64>,
    /// Current base fee in gwei.
    pub base_fee_gwei: f64,
    /// Historical base fees (recent N blocks).
    pub base_fee_history: Vec<f64>,
    /// Stablecoin prices (keyed by symbol: "USDC", "DAI", etc.).
    pub stablecoin_prices: HashMap<String, f64>,
    /// Portfolio drawdown fraction (0.0 = no drawdown, 1.0 = total loss).
    pub portfolio_drawdown_pct: f64,
    /// Block number for the snapshot.
    pub block_number: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: i64,
}
```

4. **Implement `PriceDislocationBreaker`**. Triggers when any asset moves more than N% in one block. Config: `max_single_block_move_pct: f64` (default 10.0). Check: for each asset in `prices`, compare to `prev_prices`. If `|price - prev| / prev > threshold`, halt.

5. **Implement `LiquidityCrisisBreaker`**. Triggers when pool TVL drops more than M% over recent blocks. Config: `max_tvl_drop_pct: f64` (default 50.0). Check: for each pool, compare `pool_tvl` to `prev_pool_tvl`. If `(prev - current) / prev > threshold`, halt.

6. **Implement `GasSpikeBreaker`**. Triggers when base fee exceeds the Nth percentile of recent history. Config: `max_percentile: f64` (default 0.95). Check: compute the 95th percentile of `base_fee_history`, compare to current `base_fee_gwei`. If current exceeds, halt (delay trades until gas normalizes).

7. **Implement `DepegBreaker`**. Triggers when any stablecoin deviates more than N% from $1.00. Config: `max_deviation_pct: f64` (default 1.0). Check: for each stablecoin in `stablecoin_prices`, if `|price - 1.0| / 1.0 > threshold`, halt.

8. **Implement `DrawdownBreaker`**. Triggers when portfolio drawdown exceeds threshold. Config: `max_drawdown_pct: f64` (default 20.0). Check: if `portfolio_drawdown_pct > threshold`, halt. This mirrors the drawdown halt in the risk engine (batch 4.1) but operates at the conductor level for defense in depth.

9. **Implement `DeFiBreakerSet`** that aggregates all breakers:

```rust
pub struct DeFiBreakerSet {
    breakers: Vec<Box<dyn DeFiBreaker>>,
}

impl DeFiBreakerSet {
    pub fn new() -> Self { /* register all default breakers */ }
    pub fn check_all(&self, snapshot: &MarketSnapshot) -> Vec<(String, String)> { /* (name, reason) */ }
    pub fn any_halted(&self, snapshot: &MarketSnapshot) -> bool { /* any breaker triggered */ }
}
```

### Wiring

Add to `crates/roko-conductor/src/lib.rs`:
```rust
/// DeFi-specific circuit breakers for market condition monitoring.
pub mod defi_breakers;
```

### Tests

```rust
#[test]
fn price_dislocation_triggers_on_10pct_move() {
    let breaker = PriceDislocationBreaker::new(10.0);
    let snapshot = MarketSnapshot {
        prices: [("ETH".into(), 1800.0)].into_iter().collect(),
        prev_prices: [("ETH".into(), 2000.0)].into_iter().collect(),
        ..Default::default()
    };
    assert!(breaker.check(&snapshot).is_some(), "10% drop should trigger");
}

#[test]
fn price_dislocation_passes_on_small_move() {
    let breaker = PriceDislocationBreaker::new(10.0);
    let snapshot = MarketSnapshot {
        prices: [("ETH".into(), 1990.0)].into_iter().collect(),
        prev_prices: [("ETH".into(), 2000.0)].into_iter().collect(),
        ..Default::default()
    };
    assert!(breaker.check(&snapshot).is_none(), "0.5% move should pass");
}

#[test]
fn liquidity_crisis_triggers_on_tvl_drop() {
    let breaker = LiquidityCrisisBreaker::new(50.0);
    let snapshot = MarketSnapshot {
        pool_tvl: [("0xpool".into(), 500_000.0)].into_iter().collect(),
        prev_pool_tvl: [("0xpool".into(), 1_200_000.0)].into_iter().collect(),
        ..Default::default()
    };
    assert!(breaker.check(&snapshot).is_some(), ">50% TVL drop should trigger");
}

#[test]
fn gas_spike_triggers_on_high_fee() {
    let breaker = GasSpikeBreaker::new(0.95);
    let mut history: Vec<f64> = (0..100).map(|i| 20.0 + i as f64 * 0.5).collect();
    let snapshot = MarketSnapshot {
        base_fee_gwei: 200.0, // well above 95th percentile
        base_fee_history: history,
        ..Default::default()
    };
    assert!(breaker.check(&snapshot).is_some(), "extreme gas should trigger");
}

#[test]
fn depeg_triggers_on_stablecoin_deviation() {
    let breaker = DepegBreaker::new(1.0);
    let snapshot = MarketSnapshot {
        stablecoin_prices: [("USDC".into(), 0.98)].into_iter().collect(),
        ..Default::default()
    };
    assert!(breaker.check(&snapshot).is_some(), "2% depeg should trigger");
}

#[test]
fn breaker_set_aggregates_results() {
    let set = DeFiBreakerSet::new();
    let snapshot = MarketSnapshot {
        portfolio_drawdown_pct: 0.25, // 25% drawdown
        ..Default::default()
    };
    assert!(set.any_halted(&snapshot), "drawdown breaker should trigger");
}
```

### Verification

```bash
cargo test -p roko-conductor -- defi_breakers
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-conductor
```

### Acceptance criteria

- [ ] `defi_breakers.rs` module created and declared in `lib.rs`
- [ ] `DeFiBreaker` trait defined with `name()` and `check()` methods
- [ ] `MarketSnapshot` struct carries all required market data
- [ ] `PriceDislocationBreaker` detects large single-block price moves
- [ ] `LiquidityCrisisBreaker` detects TVL drops
- [ ] `GasSpikeBreaker` uses percentile-based gas detection
- [ ] `DepegBreaker` monitors stablecoin peg deviations
- [ ] `DrawdownBreaker` enforces portfolio drawdown halt
- [ ] `DeFiBreakerSet` aggregates all breakers with `check_all()` and `any_halted()`
- [ ] 8+ tests pass

### Commit message

```
feat(roko-conductor): add DeFi circuit breakers for price, liquidity, gas, depeg, drawdown
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives used

- **Gate**: The primary primitive for this gap doc. `DeFiRiskGate` runs in pre-action mode and enforces position limits, maximum slippage, and exposure bounds before any trade executes. `MevProtectionGate` simulates the transaction for MEV extraction risk and blocks or re-routes if the risk score exceeds threshold. `CircuitBreakerGate` fires in emergency mode — it halts all open positions and blocks new execution when market conditions breach configured limits (price dislocation, TVL drop, gas spike, stablecoin depeg, portfolio drawdown). `CustodyGate` verifies custody proof before large transfers proceed.
- **Extension**: `DaimonRiskModulator` is a Tier 3 (Roko-native) extension at the Affect layer (layer 6) that adjusts risk tolerance in real time based on the daimon affect state — fear suppresses position sizing; greed flags are dampened before they reach execution. Uses the `Extension` trait's `validate` hook to intercept proposed actions and the `observe` hook to read current PAD vectors.
- **Signal**: Risk alert Signals are emitted when a gate fires — each signal carries the gate name, the triggering metric, the threshold that was breached, and the action taken (blocked, rerouted, halted).

### Authoring surfaces

- **Gate Designer** — configure risk gates with pre-action mode: set position limits, slippage bounds, MEV score thresholds, custody rules, and circuit breaker triggers per gate; preview gate behavior against historical trade logs
- **System → Gates** — pipeline editor showing the full pre-action → execution → post-action gate chain with pass/fail counts per stage
- **Extension Workshop** — configure daimon risk modulation extension parameters: fear sensitivity, greed dampening coefficient, and affect window size

### Shareable artifacts

- Gate configurations: risk limit presets representing common risk profiles (conservative, moderate, aggressive) — importable and layer-able
- Safety pipeline templates: a complete bundled pre-action gate chain ready to attach to any DeFi agent
- Circuit breaker configurations: threshold and action presets for different market regimes (normal, stressed, crisis)

### Dashboard visibility

- **System → Gates** — risk gate pipeline view with live pass/fail rates, latency per gate, and blocked-action log
- **Agent Detail → Safety** — per-agent risk exposure panel showing current position against limits, gate history, and daimon affect state
- **Pulse → Alerts** — real-time risk alert signal stream with severity filter and mute controls
