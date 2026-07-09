# 09 - Dreams integration for DeFi

> **Scope**: Offline strategy discovery -- replay trades, rehearse risks, discover strategies through dreaming.
> **Primary crate**: `roko-dreams` (`crates/roko-dreams/src/`)
> **Secondary crates**: `roko-neuro`, `roko-learn`, `roko-daimon`

---

## Batch 9.1: Chain triggers, counterfactual trade replay, and threat rehearsal

> **Effort**: L | **Depends on**: 7.1 (TradingReflect P&L for counterfactual scoring), 1.2 (WS events for chain triggers) | **Crate**: roko-dreams
> **Branch**: `defi/batch-9.1-dream-triggers-replay`

### Context

The dreams subsystem runs a multi-phase consolidation cycle: hypnagogia (creative association), NREM replay (Mattar-Daw prioritization), REM imagination (counterfactual synthesis), threat rehearsal, and integration. All of it targets code-task episodes. None of it knows what a trade is.

This batch adds three capabilities. First, chain events (price moves, liquidations, drawdowns) become dream triggers alongside the existing `Idle`, `Scheduled`, `Manual`, `EpisodeCount`, `BusPulse`, and `CoordinationPattern` variants in `DreamTrigger` (runner.rs:222). Second, the counterfactual imagination engine gains DeFi-specific perturbation axes -- exit timing, position sizing, entry price, stop-loss placement, instrument substitution -- extending the five existing axes (`Plan`, `TaskType`, `Model`, `Outcome`, `FailureReason`) in `CounterfactualAxis` (cycle.rs:186). Third, threat rehearsal gains a `DeFiThreatGenerator` that produces `ThreatScenario` values from market risk parameters (flash crash, oracle failure, liquidity drain, cascading liquidation, MEV extraction) instead of from episode failure patterns.

**Mirage-rs counterfactual validation**: The counterfactual question "what if I'd exited 2 blocks earlier?" can be answered precisely by spawning a mirage-rs instance forked to the target block, executing the alternative trade, and comparing the resulting receipt against the actual outcome. This gives exact P&L deltas instead of heuristic estimates. The `projected_delta_defi` function (work item 3 below) should support an optional `MirageClient` parameter -- when present, it forks to the counterfactual block, executes the alternative transaction, and returns the real P&L difference. When absent, it falls back to the heuristic delta calculation. This keeps the fast path (no mirage) available for bulk counterfactual sweeps while enabling precise validation for the top-ranked hypotheses. The existing `rehearse_single()` heuristic (rehearsal.rs:90) is too simple for market risks -- it checks `detection_difficulty < 0.7 && !mitigation.is_empty() && severity < 0.8`. The new generator produces scenarios parameterized by current position data and liquidity profiles so rehearsal outcomes are actionable.

### Read first

| File | Why |
|------|-----|
| `crates/roko-dreams/src/runner.rs:220-248` | `DreamTrigger` enum -- all six existing variants |
| `crates/roko-dreams/src/cycle.rs:185-302` | `CounterfactualAxis` enum and `DreamCounterfactualRecord` |
| `crates/roko-dreams/src/imagination.rs:27-175` | `ImaginationMode`, `CausalModel`, `imagine()`, `synthesize_hypotheses()` |
| `crates/roko-dreams/src/rehearsal.rs:1-167` | `rehearse_threats()`, `rehearse_single()`, `RehearsalOutcome`, `RehearsalReport` |
| `crates/roko-dreams/src/threat.rs:14-81` | `ThreatScenario`, `enumerate_threats()` -- severity = likelihood * impact * (1 - detection_difficulty) |
| `crates/roko-dreams/src/replay.rs:35-120` | `ReplayUtility` Mattar-Daw scoring -- gain/need/spacing |
| `crates/roko-dreams/src/lib.rs:46-78` | Module declarations and public re-exports |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**1. Add `ChainEvent` variant to `DreamTrigger`**

File: `crates/roko-dreams/src/runner.rs`

Add a new variant to the `DreamTrigger` enum (line 222):

```rust
/// Chain event trigger: market conditions warrant targeted consolidation.
ChainEvent {
    /// Type of chain event that triggered the dream.
    #[serde(default)]
    event_type: ChainEventKind,
    /// Chain identifier (e.g., "ethereum", "arbitrum").
    #[serde(default)]
    chain: String,
    /// Magnitude of the event (e.g., price move percentage, drawdown fraction).
    #[serde(default)]
    magnitude: f64,
    /// Affected position identifiers, if any.
    #[serde(default)]
    affected_positions: Vec<String>,
},
```

Add a sibling enum for event classification:

```rust
/// Classification of chain events that trigger dreams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainEventKind {
    /// Large price move (>5% in 1h) -- triggers hypnagogia for creative recombination.
    PriceMove,
    /// Liquidation event -- triggers threat rehearsal with real parameters.
    Liquidation,
    /// Portfolio drawdown exceeding threshold -- triggers counterfactual replay.
    Drawdown,
    /// Funding rate inversion -- triggers strategy recombination via hypnagogia.
    FundingInversion,
    /// Regime transition detected by TA indicators.
    RegimeTransition,
}
```

Update the `DreamTrigger::label()` method (line 250) to handle `ChainEvent`.

**2. Add DeFi counterfactual axes**

File: `crates/roko-dreams/src/cycle.rs`

Extend `CounterfactualAxis` (line 185) with trading-specific variants:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CounterfactualAxis {
    // Existing
    Plan,
    TaskType,
    Model,
    Outcome,
    FailureReason,
    // DeFi additions
    ExitTiming,
    PositionSize,
    EntryPrice,
    StopLoss,
    Instrument,
    HedgeStrategy,
}
```

Update `CounterfactualAxis::ALL` (line 195) to include the new variants. Update `axis_label()` and `axis_value_from_episode()` helpers to extract trade metadata from `episode.extra` fields (keyed as `"trade:exit_block"`, `"trade:size"`, `"trade:entry_price"`, `"trade:stop_pct"`, `"trade:instrument"`, `"trade:hedge"`).

**3. Implement `projected_delta()` overloads for trade axes**

File: `crates/roko-dreams/src/imagination.rs`

The existing `projected_delta()` function (line 147 area) returns a heuristic delta based on variable name and mode. Add trade-aware logic:

```rust
fn projected_delta_defi(
    axis: &str,
    original: &str,
    replacement: &str,
    mode: ImaginationMode,
    was_successful: bool,
) -> f64 {
    match axis {
        "exit_timing" => {
            // Earlier exit on a losing trade = positive delta
            // Later exit on a winning trade = positive delta
            let orig_blocks: f64 = original.parse().unwrap_or(0.0);
            let repl_blocks: f64 = replacement.parse().unwrap_or(0.0);
            let direction = if was_successful { 1.0 } else { -1.0 };
            ((repl_blocks - orig_blocks) * direction * 0.01).clamp(-0.3, 0.3)
        }
        "position_size" => {
            // Scaling up a winner or down a loser = positive delta
            let scale_factor: f64 = replacement.parse().unwrap_or(1.0)
                / original.parse::<f64>().unwrap_or(1.0).max(0.01);
            let direction = if was_successful { 1.0 } else { -1.0 };
            ((scale_factor - 1.0) * direction * 0.15).clamp(-0.3, 0.3)
        }
        "entry_price" => {
            // What if entry price had been shift_bps better/worse?
            // Adjust entry price and recompute P&L: delta = (shift / 10000) * position_size
            let shift_bps: f64 = replacement.parse().unwrap_or(0.0);
            let position_size: f64 = original.parse().unwrap_or(0.0);
            (position_size * (shift_bps / 10_000.0)).clamp(-0.3, 0.3)
        }
        "stop_loss" => {
            // What if stop-loss had been at price_pct% below entry?
            let price_pct: f64 = replacement.parse().unwrap_or(0.0);
            let entry: f64 = original.parse().unwrap_or(0.0);
            // Approximate: tighter stop on losers saves money, on winners costs money
            let direction = if was_successful { -1.0 } else { 1.0 };
            (price_pct / 100.0 * direction * 0.2).clamp(-0.3, 0.3)
        }
        "instrument" => {
            // What if we'd traded a different instrument?
            // Heuristic: random perturbation scaled by mode creativity
            let creativity = match mode {
                ImaginationMode::Conservative => 0.05,
                ImaginationMode::Moderate => 0.10,
                ImaginationMode::Creative => 0.20,
                _ => 0.10,
            };
            if was_successful { -creativity } else { creativity }
        }
        "hedge" => {
            // What if we'd hedged? Hedging reduces both upside and downside.
            let hedge_ratio: f64 = replacement.parse().unwrap_or(0.5);
            let direction = if was_successful { -1.0 } else { 1.0 };
            (hedge_ratio * direction * 0.1).clamp(-0.3, 0.3)
        }
        _ => 0.0,
    }
}
```

Wire this into `imagine()` by checking whether the intervention variable starts with a DeFi prefix.

**4. Create `DeFiThreatGenerator`**

File: `crates/roko-dreams/src/defi_threats.rs` (new)

```rust
//! DeFi-specific threat scenario generation.
//!
//! Produces `ThreatScenario` values from market risk parameters
//! instead of from episode failure patterns.

use crate::threat::ThreatScenario;
use serde::{Deserialize, Serialize};

/// Market risk parameters used to generate DeFi threat scenarios.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketRiskParams {
    /// Current portfolio value in USD.
    pub portfolio_value_usd: f64,
    /// Maximum single-position concentration (0.0..1.0).
    pub max_concentration: f64,
    /// Current leverage ratio across all positions.
    pub aggregate_leverage: f64,
    /// Available liquidity in target pools (USD).
    pub pool_liquidity_usd: f64,
    /// Whether positions have on-chain stop-losses.
    pub has_onchain_stops: bool,
    /// Oracle staleness tolerance in seconds.
    pub oracle_staleness_tolerance_secs: u64,
}

/// Generate DeFi-specific threat scenarios from market risk parameters.
#[must_use]
pub fn enumerate_defi_threats(params: &MarketRiskParams) -> Vec<ThreatScenario> {
    let mut threats = Vec::new();
    threats.push(flash_crash_threat(params));
    threats.push(oracle_failure_threat(params));
    threats.push(liquidity_drain_threat(params));
    threats.push(cascading_liquidation_threat(params));
    threats.push(mev_extraction_threat(params));
    threats.sort_by(|a, b| b.severity().partial_cmp(&a.severity())
        .unwrap_or(std::cmp::Ordering::Equal));
    threats
}
```

Implement each threat constructor using `params` to parameterize severity:

```rust
fn flash_crash_threat(params: &MarketRiskParams) -> ThreatScenario {
    ThreatScenario {
        name: "flash_crash".into(),
        description: "Sudden 20%+ price drop across all held assets".into(),
        severity: 0.9,
        likelihood: (0.05 * params.aggregate_leverage / 2.0).clamp(0.01, 0.5),
        detection_difficulty: 0.3,
        impact: params.portfolio_value_usd * 0.2 * params.max_concentration,
        mitigation: vec![
            "Set stop-loss orders at -10%".into(),
            "Reduce position sizes during high-volatility regime".into(),
            "Monitor funding rates for leverage-driven crash signals".into(),
        ],
    }
}

fn oracle_failure_threat(params: &MarketRiskParams) -> ThreatScenario {
    ThreatScenario {
        name: "oracle_failure".into(),
        description: "Price oracle returns stale/manipulated data, causing incorrect trade execution".into(),
        severity: 0.8,
        likelihood: 0.02,
        detection_difficulty: if params.oracle_staleness_tolerance_secs > 60 { 0.7 } else { 0.5 },
        impact: params.portfolio_value_usd * 0.15,
        mitigation: vec![
            "Cross-reference multiple price sources".into(),
            "Reject trades when oracle freshness > 60 seconds".into(),
            "Implement median-of-N oracle aggregation".into(),
        ],
    }
}

fn liquidity_drain_threat(params: &MarketRiskParams) -> ThreatScenario {
    ThreatScenario {
        name: "liquidity_drain".into(),
        description: "LP positions become illiquid -- cannot exit at expected price".into(),
        severity: 0.7,
        likelihood: if params.pool_liquidity_usd < 500_000.0 { 0.2 } else { 0.1 },
        detection_difficulty: 0.4,
        impact: params.portfolio_value_usd * 0.1,
        mitigation: vec![
            "Monitor pool TVL and set minimum liquidity thresholds".into(),
            "Avoid concentrated positions in low-TVL pools".into(),
            "Implement gradual position unwinding over multiple blocks".into(),
        ],
    }
}

fn cascading_liquidation_threat(params: &MarketRiskParams) -> ThreatScenario {
    ThreatScenario {
        name: "cascading_liquidation".into(),
        description: "Leveraged positions across DeFi protocols trigger cascading liquidations".into(),
        severity: 0.95,
        likelihood: (0.03 * params.aggregate_leverage / 2.0).clamp(0.01, 0.3),
        detection_difficulty: 0.5,
        impact: params.portfolio_value_usd * 0.5,
        mitigation: vec![
            "Maintain health factor > 2.0 on all lending positions".into(),
            "Monitor aggregate protocol liquidation thresholds".into(),
            "Keep emergency USDC reserve for health factor top-up".into(),
        ],
    }
}

fn mev_extraction_threat(params: &MarketRiskParams) -> ThreatScenario {
    ThreatScenario {
        name: "mev_extraction".into(),
        description: "Sandwich attacks or frontrunning extract value from pending transactions".into(),
        severity: 0.4,
        likelihood: 0.3,
        detection_difficulty: 0.7,
        impact: params.portfolio_value_usd * 0.02,
        mitigation: vec![
            "Use private mempool (Flashbots/MEV Blocker)".into(),
            "Set tight slippage bounds".into(),
            "Prefer limit orders over market swaps".into(),
        ],
    }
}
```

**5. Wire DeFi threats into rehearsal**

File: `crates/roko-dreams/src/rehearsal.rs`

Add a public function:

```rust
/// Run threat rehearsal against DeFi-specific market risk scenarios.
#[must_use]
pub fn rehearse_defi_threats(
    params: &crate::defi_threats::MarketRiskParams,
    max_scenarios: Option<usize>,
    now: DateTime<Utc>,
) -> RehearsalReport {
    let threats = crate::defi_threats::enumerate_defi_threats(params);
    // Reuse existing rehearse_single + outcome_to_episode pipeline
    // ...
}
```

**6. Map `ChainEventKind` to dream phases**

File: `crates/roko-dreams/src/runner.rs`

Add a method to `ChainEventKind`:

```rust
impl ChainEventKind {
    /// Which dream phase should be prioritized for this event type.
    #[must_use]
    pub const fn preferred_phase(self) -> &'static str {
        match self {
            Self::PriceMove | Self::FundingInversion => "hypnagogia",
            Self::Liquidation => "threat_rehearsal",
            Self::Drawdown => "causal_replay",
            Self::RegimeTransition => "full_cycle",
        }
    }
}
```

### Wiring

- Add `pub mod defi_threats;` to `crates/roko-dreams/src/lib.rs` (after line 54)
- Re-export: `pub use defi_threats::{MarketRiskParams, enumerate_defi_threats};`
- Re-export: `pub use runner::ChainEventKind;`
- No config schema changes -- `ChainEvent` trigger is constructed programmatically by the chain event subscriber (batch 1.2), not from `roko.toml`

### Tests

```rust
// crates/roko-dreams/src/defi_threats.rs
#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> MarketRiskParams {
        MarketRiskParams {
            portfolio_value_usd: 100_000.0,
            max_concentration: 0.3,
            aggregate_leverage: 2.0,
            pool_liquidity_usd: 1_000_000.0,
            has_onchain_stops: true,
            oracle_staleness_tolerance_secs: 30,
        }
    }

    #[test]
    fn enumerate_produces_five_scenarios() {
        let threats = enumerate_defi_threats(&default_params());
        assert_eq!(threats.len(), 5);
        for t in &threats {
            assert!(t.severity() >= 0.0);
            assert!(t.severity() <= 1.0);
        }
    }

    #[test]
    fn high_leverage_increases_flash_crash_severity() {
        let low = enumerate_defi_threats(&MarketRiskParams {
            aggregate_leverage: 1.0,
            ..default_params()
        });
        let high = enumerate_defi_threats(&MarketRiskParams {
            aggregate_leverage: 5.0,
            ..default_params()
        });
        let flash_low = low.iter().find(|t| t.description.contains("flash crash"));
        let flash_high = high.iter().find(|t| t.description.contains("flash crash"));
        assert!(flash_high.unwrap().severity() > flash_low.unwrap().severity());
    }

    #[test]
    fn threats_sorted_by_descending_severity() {
        let threats = enumerate_defi_threats(&default_params());
        for window in threats.windows(2) {
            assert!(window[0].severity() >= window[1].severity());
        }
    }
}

// crates/roko-dreams/src/rehearsal.rs -- add to existing tests
#[test]
fn rehearse_defi_threats_produces_episodes() {
    let params = crate::defi_threats::MarketRiskParams {
        portfolio_value_usd: 50_000.0,
        max_concentration: 0.5,
        aggregate_leverage: 3.0,
        pool_liquidity_usd: 500_000.0,
        has_onchain_stops: false,
        oracle_staleness_tolerance_secs: 60,
    };
    let report = rehearse_defi_threats(&params, Some(3), Utc::now());
    assert!(report.rehearsals_performed <= 3);
    assert_eq!(report.outcomes.len(), report.generated_episodes.len());
}

// crates/roko-dreams/src/runner.rs -- add to existing tests
#[test]
fn chain_event_trigger_serde_roundtrip() {
    let trigger = DreamTrigger::ChainEvent {
        event_type: ChainEventKind::PriceMove,
        chain: "ethereum".to_string(),
        magnitude: 0.08,
        affected_positions: vec!["pos-1".to_string()],
    };
    let json = serde_json::to_string(&trigger).unwrap();
    let decoded: DreamTrigger = serde_json::from_str(&json).unwrap();
    assert_eq!(trigger, decoded);
}
```

### Verification

```bash
cargo test -p roko-dreams -- defi_threats
cargo test -p roko-dreams -- rehearsal
cargo test -p roko-dreams -- runner::tests::chain_event
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-dreams
```

### Acceptance criteria

- [ ] `DreamTrigger::ChainEvent` variant exists with `ChainEventKind`, `chain`, `magnitude`, `affected_positions`
- [ ] `ChainEventKind` maps each event type to a preferred dream phase
- [ ] `CounterfactualAxis` has six DeFi variants: `ExitTiming`, `PositionSize`, `EntryPrice`, `StopLoss`, `Instrument`, `HedgeStrategy`
- [ ] `projected_delta()` returns non-zero values for DeFi axes using trade metadata
- [ ] `enumerate_defi_threats()` produces five parameterized threat scenarios
- [ ] High leverage increases flash crash threat severity (tested)
- [ ] `rehearse_defi_threats()` feeds DeFi threats through the existing rehearsal pipeline
- [ ] All DeFi threat scenarios carry actionable mitigation text
- [ ] Serde roundtrip works for `ChainEvent` trigger variant
- [ ] `cargo test -p roko-dreams` passes clean

### Commit message

```
feat(roko-dreams): add chain event triggers, DeFi counterfactual axes, and market threat rehearsal
```

---

## Batch 9.2: Strategy discovery via hypnagogia, dream journal, and regime transition dreams

> **Effort**: L | **Depends on**: 9.1 (chain triggers for regime transitions), 3.1 (TA indicators for regime detection and market knowledge entries) | **Crate**: roko-dreams
> **Branch**: `defi/batch-9.2-strategy-discovery`

### Context

The hypnagogia engine (`hypnagogia.rs:89`) is a four-layer creativity pipeline: thalamic gate (relevance filter with stochastic resonance), executive loosener (associative pairing by shared tags), Dali interrupt (random episode injection), homuncular observer (rank by confidence + novelty + diversity, keep top 6). It operates on `KnowledgeEntry` signals and `Episode` records, and it is domain-agnostic in principle. But all scoring functions assume text-based knowledge entries. Market-aware creative recombination needs TA indicator patterns fed as `KnowledgeEntry` values, cross-market/cross-timeframe pairing in the loosener, trade episodes as Dali interrupt sources, and a scorer that values edge potential over text novelty.

The `DreamJournal` (`phase2/advanced.rs:304`) and `DreamJournalEntry` (`phase2/advanced.rs:392`) already exist and persist to `.roko/dreams/journal.jsonl`. The journal records cycle metadata -- durations, hypothesis counts, nightmares, diversity. But it carries no market context (regime, volatility, open positions) and no outcome attribution (did the dream-discovered strategy actually work?).

The dream cycle has no concept of market regimes. `DreamClusterKey` groups by `plan_id`, `task_type`, `outcome`, `model` (cycle.rs:307). Regime-transition dreams need a trigger (from batch 9.1's `ChainEventKind::RegimeTransition`), a targeted replay of episodes from the ending regime, and regime tags on all dream-generated knowledge entries so they surface when similar regimes recur.

### Read first

| File | Why |
|------|-----|
| `crates/roko-dreams/src/hypnagogia.rs:87-200` | `HypnagogiaEngine` struct, `run()`, four layers |
| `crates/roko-dreams/src/phase2/advanced.rs:299-440` | `DreamJournal`, `DreamJournalEntry`, `DreamTrendAnalysis` |
| `crates/roko-dreams/src/cycle.rs:305-395` | `DreamClusterKey`, `DreamClusterReport`, `DreamCycle` struct |
| `crates/roko-dreams/src/cycle.rs:67-110` | `DreamCycleReport` -- all fields including `hypnagogia_entries_count` |
| `crates/roko-dreams/src/staging.rs:16-95` | `StagingBuffer`, `StagingEntry`, `ConfidenceStage` |
| `crates/roko-neuro/src/lib.rs:117-200` | `KnowledgeKind`, `KnowledgeTier` and their methods |
| `crates/roko-neuro/src/hdc.rs:1-40` | `RoleFillerEncoder::encode_structured()` for structured HDC encoding |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**1. Create `MarketKnowledgeBuilder`**

File: `crates/roko-dreams/src/market_knowledge.rs` (new)

Converts TA indicator readings into `KnowledgeEntry` values that the hypnagogia engine can process. The thalamic gate filters by `confidence >= 0.45`, so entries need meaningful confidence scores derived from indicator signal strength.

```rust
//! Convert TA indicator readings into KnowledgeEntry values for dream processing.

use chrono::{DateTime, Utc};
use roko_neuro::{KnowledgeEntry, KnowledgeKind, KnowledgeTier};
use serde::{Deserialize, Serialize};

/// A snapshot of technical indicator readings for one market/timeframe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorSnapshot {
    /// Market identifier (e.g., "ETH-PERP").
    pub market: String,
    /// Timeframe (e.g., "1h", "4h").
    pub timeframe: String,
    /// RSI(14) value (0-100).
    pub rsi_14: f64,
    /// Volatility percentile (0-100) over the lookback window.
    pub volatility_percentile: f64,
    /// Funding rate (annualized, can be negative).
    pub funding_rate: f64,
    /// Trend direction: "up", "down", or "range".
    pub trend_direction: String,
    /// Volume z-score relative to the lookback mean.
    pub volume_zscore: f64,
    /// Detected regime label (e.g., "trending", "mean-reverting", "high-vol").
    pub regime: String,
}

/// Build KnowledgeEntry values from indicator snapshots.
#[must_use]
pub fn indicators_to_knowledge(
    snapshot: &IndicatorSnapshot,
    created_at: DateTime<Utc>,
) -> Vec<KnowledgeEntry> {
    let mut entries = Vec::new();

    // RSI extremes produce strategy fragment entries
    if snapshot.rsi_14 < 30.0 || snapshot.rsi_14 > 70.0 {
        let direction = if snapshot.rsi_14 < 30.0 { "oversold" } else { "overbought" };
        entries.push(market_entry(
            KnowledgeKind::StrategyFragment,
            format!(
                "{} RSI({:.0}) on {} {} -- {} signal",
                direction, snapshot.rsi_14, snapshot.market, snapshot.timeframe,
                if snapshot.rsi_14 < 30.0 { "potential long" } else { "potential short" }
            ),
            &snapshot,
            0.6 + (snapshot.rsi_14 - 50.0).abs() / 100.0,
            created_at,
        ));
    }

    // Regime observation
    entries.push(market_entry(
        KnowledgeKind::Insight,
        format!(
            "{} {} regime: {} (vol pct {:.0}, trend {})",
            snapshot.market, snapshot.timeframe,
            snapshot.regime, snapshot.volatility_percentile, snapshot.trend_direction
        ),
        &snapshot,
        0.7,
        created_at,
    ));

    // Funding divergence produces a causal link
    if snapshot.funding_rate.abs() > 0.05 {
        entries.push(market_entry(
            KnowledgeKind::CausalLink,
            format!(
                "Extreme funding rate ({:.4}) on {} -> likely mean reversion",
                snapshot.funding_rate, snapshot.market
            ),
            &snapshot,
            0.5 + snapshot.funding_rate.abs().min(0.3),
            created_at,
        ));
    }

    entries
}
```

Include a helper `market_entry()` that builds a `KnowledgeEntry` with appropriate tags: `["market", "dream", "ta-indicator", format!("market:{}", snapshot.market), format!("timeframe:{}", snapshot.timeframe), format!("regime:{}", snapshot.regime)]`.

**2. Extend `DreamJournalEntry` with market context**

File: `crates/roko-dreams/src/phase2/advanced.rs`

Add optional fields to `DreamJournalEntry` (line 392):

```rust
/// Market regime active at dream time, if known.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub market_regime: Option<String>,
/// Volatility percentile at dream time.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub volatility_percentile: Option<f64>,
/// Number of open positions at dream time.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub open_position_count: Option<usize>,
/// Unrealized P&L at dream time in USD.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub unrealized_pnl_usd: Option<f64>,
/// Strategy hypotheses generated during this cycle with their tags.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub strategy_hypotheses: Vec<String>,
/// Backtested edge of dream-discovered strategies (filled post-hoc).
#[serde(default, skip_serializing_if = "Option::is_none")]
pub backtest_edge: Option<f64>,
```

All new fields use `serde(default)` for backward compatibility with existing journal entries.

**3. Create `DreamCalibrator`**

File: `crates/roko-dreams/src/dream_calibrator.rs` (new)

Tracks how often dream-generated strategies produce positive outcomes. Reads journal entries, correlates strategy hypotheses with later backtest/live results, and produces a calibration score.

```rust
//! Dream accuracy calibration over rolling windows.

use crate::phase2::advanced::DreamJournalEntry;
use serde::{Deserialize, Serialize};

/// Rolling accuracy of dream-generated strategy hypotheses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DreamCalibration {
    /// Number of dream cycles with outcome data.
    pub cycles_with_outcomes: usize,
    /// Number of strategies that produced positive edge.
    pub positive_edge_count: usize,
    /// Number of strategies that produced negative edge.
    pub negative_edge_count: usize,
    /// Mean edge across all measured strategies.
    pub mean_edge: f64,
    /// Hit rate: fraction of strategies with positive edge.
    pub hit_rate: f64,
}

/// Compute dream calibration from journal entries.
#[must_use]
pub fn calibrate(entries: &[DreamJournalEntry], window: usize) -> DreamCalibration {
    let entries_with_backtest: Vec<&DreamJournalEntry> = entries.iter()
        .rev()
        .take(window)
        .filter(|e| e.backtest_edge.is_some())
        .collect();

    if entries_with_backtest.is_empty() {
        return DreamCalibration {
            cycles_with_outcomes: 0,
            positive_edge_count: 0,
            negative_edge_count: 0,
            mean_edge: 0.0,
            hit_rate: 0.0,
        };
    }

    let edges: Vec<f64> = entries_with_backtest.iter()
        .map(|e| e.backtest_edge.unwrap())
        .collect();

    let positive_edge_count = edges.iter().filter(|&&e| e > 0.0).count();
    let negative_edge_count = edges.iter().filter(|&&e| e <= 0.0).count();
    let hit_rate = positive_edge_count as f64 / edges.len() as f64;
    let mean_edge = edges.iter().sum::<f64>() / edges.len() as f64;

    DreamCalibration {
        cycles_with_outcomes: entries_with_backtest.len(),
        positive_edge_count,
        negative_edge_count,
        mean_edge,
        hit_rate,
    }
}
```

**4. Add regime tag to `DreamClusterKey` and `DreamCycleReport`**

File: `crates/roko-dreams/src/cycle.rs`

Extend `DreamClusterKey` (line 307):

```rust
pub struct DreamClusterKey {
    pub plan_id: String,
    pub task_type: String,
    pub outcome: DreamOutcome,
    pub model: String,
    /// Market regime active when episodes were generated, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime: Option<String>,
}
```

Extend `DreamCycleReport` (line 68):

```rust
/// Market regime active during this dream cycle, if known.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub regime: Option<String>,
```

When clustering episodes, extract `regime` from `episode.extra.get("regime")` and include it in the cluster key.

**5. Tag dream-generated knowledge entries with regime**

File: `crates/roko-dreams/src/cycle.rs`

In the integration phase where `KnowledgeEntry` values are built (the distillation and warning-entry paths), add a `format!("regime:{}", regime)` tag when a regime is active. This ensures the knowledge store can filter by regime during context assembly.

**6. Create `RegimeTransitionHandler`**

File: `crates/roko-dreams/src/regime_dreams.rs` (new)

Coordinates regime-exit and regime-entry dream behavior:

```rust
//! Regime-transition dream coordination.

use roko_neuro::{KnowledgeEntry, KnowledgeStore};

/// Handles the dream response to a regime transition.
#[derive(Debug, Clone)]
pub struct RegimeTransitionHandler {
    /// The regime that is ending.
    pub exiting_regime: String,
    /// The regime that is beginning.
    pub entering_regime: String,
}

impl RegimeTransitionHandler {
    /// Filter episodes that belong to the exiting regime for targeted replay.
    pub fn filter_exiting_episodes(
        &self,
        episodes: &[roko_learn::episode_logger::Episode],
    ) -> Vec<roko_learn::episode_logger::Episode> {
        episodes.iter()
            .filter(|ep| ep.extra.get("regime").and_then(|v| v.as_str())
                == Some(self.exiting_regime.as_str()))
            .cloned()
            .collect()
    }

    /// Query knowledge store for entries from historical periods with the entering regime.
    pub fn query_entering_regime_knowledge(
        &self,
        store: &KnowledgeStore,
    ) -> Vec<KnowledgeEntry> {
        let tag = format!("regime:{}", self.entering_regime);
        store.query_by_tags(&[&tag], 20)
            .unwrap_or_default()
            .into_iter()
            .map(|hit| hit.entry)
            .collect()
    }
}
```

### Wiring

- Add `pub mod market_knowledge;` and `pub mod dream_calibrator;` and `pub mod regime_dreams;` to `crates/roko-dreams/src/lib.rs`
- Re-export: `pub use market_knowledge::{IndicatorSnapshot, indicators_to_knowledge};`
- Re-export: `pub use dream_calibrator::{DreamCalibration, calibrate};`
- Re-export: `pub use regime_dreams::RegimeTransitionHandler;`
- No config schema changes -- regime information flows from TA indicators (batch 3.1) through episode metadata

### Tests

```rust
// crates/roko-dreams/src/market_knowledge.rs
#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> IndicatorSnapshot {
        IndicatorSnapshot {
            market: "ETH-PERP".to_string(),
            timeframe: "1h".to_string(),
            rsi_14: 25.0,
            volatility_percentile: 80.0,
            funding_rate: 0.08,
            trend_direction: "down".to_string(),
            volume_zscore: 1.5,
            regime: "high-vol".to_string(),
        }
    }

    #[test]
    fn oversold_rsi_produces_strategy_fragment() {
        let entries = indicators_to_knowledge(&snapshot(), Utc::now());
        assert!(entries.iter().any(|e| e.kind == KnowledgeKind::StrategyFragment
            && e.content.contains("oversold")));
    }

    #[test]
    fn extreme_funding_produces_causal_link() {
        let entries = indicators_to_knowledge(&snapshot(), Utc::now());
        assert!(entries.iter().any(|e| e.kind == KnowledgeKind::CausalLink
            && e.content.contains("funding")));
    }

    #[test]
    fn normal_rsi_skips_strategy_fragment() {
        let mut s = snapshot();
        s.rsi_14 = 50.0;
        s.funding_rate = 0.01;
        let entries = indicators_to_knowledge(&s, Utc::now());
        assert!(!entries.iter().any(|e| e.kind == KnowledgeKind::StrategyFragment));
    }

    #[test]
    fn entries_carry_regime_and_market_tags() {
        let entries = indicators_to_knowledge(&snapshot(), Utc::now());
        for e in &entries {
            assert!(e.tags.iter().any(|t| t.starts_with("market:")));
            assert!(e.tags.iter().any(|t| t.starts_with("regime:")));
        }
    }
}

// crates/roko-dreams/src/dream_calibrator.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibrate_empty_journal_returns_zeros() {
        let cal = calibrate(&[], 10);
        assert_eq!(cal.cycles_with_outcomes, 0);
        assert_eq!(cal.hit_rate, 0.0);
    }

    #[test]
    fn calibrate_respects_window_size() {
        // Create entries with backtest_edge, verify only window-sized subset is used
    }

    #[test]
    fn positive_edge_increases_hit_rate() {
        // Create entries with positive/negative edges, verify hit_rate
    }
}

// crates/roko-dreams/src/regime_dreams.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_exiting_episodes_matches_regime() {
        let handler = RegimeTransitionHandler {
            exiting_regime: "trending".to_string(),
            entering_regime: "mean-reverting".to_string(),
        };
        let mut ep = roko_learn::episode_logger::Episode::new("agent", "task-1");
        ep.extra.insert("regime".to_string(),
            serde_json::Value::String("trending".to_string()));
        let filtered = handler.filter_exiting_episodes(&[ep.clone()]);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn filter_excludes_wrong_regime() {
        let handler = RegimeTransitionHandler {
            exiting_regime: "trending".to_string(),
            entering_regime: "mean-reverting".to_string(),
        };
        let mut ep = roko_learn::episode_logger::Episode::new("agent", "task-1");
        ep.extra.insert("regime".to_string(),
            serde_json::Value::String("high-vol".to_string()));
        let filtered = handler.filter_exiting_episodes(&[ep]);
        assert!(filtered.is_empty());
    }

    #[test]
    fn handler_fields_accessible() {
        let handler = RegimeTransitionHandler {
            exiting_regime: "a".to_string(),
            entering_regime: "b".to_string(),
        };
        assert_eq!(handler.exiting_regime, "a");
        assert_eq!(handler.entering_regime, "b");
    }
}
```

### Verification

```bash
cargo test -p roko-dreams -- market_knowledge
cargo test -p roko-dreams -- dream_calibrator
cargo test -p roko-dreams -- regime_dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-dreams
```

### Acceptance criteria

- [ ] `IndicatorSnapshot` converts RSI extremes to `StrategyFragment` entries and funding divergence to `CausalLink` entries
- [ ] All market knowledge entries carry `market:`, `regime:`, and `timeframe:` tags
- [ ] Normal indicator readings (RSI 30-70, funding < 0.05) produce fewer entries than extreme readings
- [ ] `DreamJournalEntry` has optional fields for `market_regime`, `volatility_percentile`, `open_position_count`, `unrealized_pnl_usd`, `backtest_edge`
- [ ] New journal fields deserialize with `serde(default)` for backward compatibility
- [ ] `DreamCalibration` computes hit rate and mean edge from journal entries with backtest outcomes
- [ ] `DreamClusterKey` and `DreamCycleReport` carry optional `regime` field
- [ ] `RegimeTransitionHandler` filters episodes by exiting regime
- [ ] Dream-generated knowledge entries are tagged with `regime:` when a regime is active
- [ ] `cargo test -p roko-dreams` passes clean

### Commit message

```
feat(roko-dreams): add market knowledge builder, dream calibrator, and regime-transition dreams
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives Used

- **Feed**: `ChainEventTrigger` (dream-triggering event stream -- price moves, liquidations, drawdowns, funding inversions, regime transitions)
- **Recipe**: `CounterfactualEngine` (alternative outcome simulation -- replays trades with different parameters: exit timing, position size, entry price, stop loss, instrument, hedge strategy), `DeFiThreatGenerator` (threat scenario generation: flash crash, oracle failure, liquidity drain, cascading liquidation, MEV extraction), `MarketKnowledgeBuilder` (TA indicators -> knowledge entries), `RegimeTransitionHandler` (regime-filtered episode transform)
- **Eval**: `DreamCalibrator` (dream accuracy measurement -- tracks how well counterfactual predictions match actual outcomes)
- **Knowledge Entry**: Dream journal entries (consolidated insights from counterfactual replay and threat rehearsal)

### Authoring Surfaces

- **Recipe Editor** -- build counterfactual pipelines with parameter axes (what-if scenarios)
- **Knowledge > Dream Cycles** -- schedule and monitor dream consolidation runs
- **Measurements > Evals** -- dream accuracy evaluation with calibration charts

### Shareable Artifacts

- Counterfactual recipe templates (standard what-if scenarios for different strategy types)
- Threat scenario templates (bundled threat generators for different market conditions)
- Dream journal archives (consolidated insights from past dream cycles)

### Dashboard Visibility

- **Knowledge > Dream Cycles** -- dream run history, consolidation progress, insight extraction
- **Forge > Recipes** -- counterfactual and threat generation pipelines
- **Measurements > Dream Calibration** -- dream prediction accuracy over time
