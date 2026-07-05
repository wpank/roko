# 08 -- Daimon integration: affect-modulated trading behavior

> **Scope**: Make the daimon affect engine respond to trading outcomes. Map P&L to PAD vectors with loss aversion, connect affect to position sizing and tilt detection, and bind TA patterns to somatic markers via HDC.
>
> **Primary crate**: `roko-daimon` (`crates/roko-daimon/src/`)

---

## Batch 8.1: PAD mapping from P&L and loss aversion

> **Effort**: M | **Depends on**: 7.1 (TradingReflect P&L data) | **Crate**: roko-daimon
> **Branch**: `defi/batch-8.1-pad-from-pnl`

### Context

The daimon affect engine appraises events via the `AffectEvent` enum at `lib.rs:1691`. Current variants -- `GateResult`, `TaskOutcome`, `Blocked`, `TimePressure`, `QueueWait`, `DreamFailure`, `DreamOutcome` -- all fire on code-task outcomes. The `appraise` implementation at `lib.rs:2204` maps these to PAD deltas through hardcoded constants (e.g., gate pass = pleasure +0.05, dominance +0.03; gate fail = pleasure -0.10, arousal +0.04).

Trading produces continuous outcomes: +$47, -$12, 0.3% slippage. The PAD vector needs continuous input signals mapped through three channels per PRD `03-daimon/01-appraisal.md`:

- **Pleasure**: positive if outcome better than predicted, negative if worse. Kahneman-Tversky prospect theory gives losses 1.6x the weight of gains.
- **Arousal**: proportional to the absolute prediction error regardless of direction.
- **Dominance**: derived from accuracy trend. Improving accuracy raises dominance.

The `AffectState::apply_delta` method at `lib.rs:370` already handles the PAD delta application through the ALMA three-layer model (emotion tau=0.1, mood tau=0.5, temperament tau=0.9). This batch adds a `TradingOutcome` variant to `AffectEvent`, implements prospect-theory asymmetric weighting, and wires it into the appraise dispatch.

### Read first

| File | Why |
|------|-----|
| `crates/roko-daimon/src/lib.rs:1691-1755` | `AffectEvent` enum -- all current variants |
| `crates/roko-daimon/src/lib.rs:2204-2319` | `appraise` implementation -- PAD delta mapping per event |
| `crates/roko-daimon/src/lib.rs:370-395` | `AffectState::apply_delta` -- how PAD deltas propagate through ALMA layers |
| `crates/roko-daimon/src/lib.rs:310-340` | `AffectState` struct -- pad, confidence, behavioral_state, alma, tick_count |
| `crates/roko-daimon/src/mortality.rs:86-100` | `MortalityEmotion::intensity` -- pattern for continuous intensity computation |
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

**8.1.1** Add `TradingOutcome` variant to `AffectEvent` at `lib.rs:1691`:

```rust
/// Trading position closed with continuous P&L outcome.
TradingOutcome {
    /// Realized P&L in USD (positive = profit, negative = loss).
    pnl_usd: f64,
    /// Prediction error: actual return minus predicted return.
    prediction_error: f64,
    /// Rolling accuracy trend in [-1.0, 1.0]. Positive = improving.
    accuracy_trend: f64,
    /// Asset traded.
    asset: String,
    /// Strategy or playbook that produced this trade.
    strategy_id: Option<String>,
},
```

**8.1.2** Create `crates/roko-daimon/src/prospect.rs` for Kahneman-Tversky value function:

```rust
/// Prospect-theory value function parameters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProspectParams {
    /// Loss aversion coefficient (default 1.6, Kahneman-Tversky).
    pub loss_aversion: f64,
    /// Diminishing sensitivity exponent for gains (default 0.88).
    pub gain_exponent: f64,
    /// Diminishing sensitivity exponent for losses (default 0.88).
    pub loss_exponent: f64,
    /// Reference point adaptation rate (EMA tau, default 0.05).
    pub reference_adaptation_rate: f64,
}

impl Default for ProspectParams {
    fn default() -> Self {
        Self {
            loss_aversion: 1.6,
            gain_exponent: 0.88,
            loss_exponent: 0.88,
            reference_adaptation_rate: 0.05,
        }
    }
}

/// Prospect-theory value function with adaptive reference point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProspectValueFunction {
    pub params: ProspectParams,
    /// Adaptive reference point (shifts with recent P&L via EMA).
    pub reference_point: f64,
}

impl ProspectValueFunction {
    pub fn new(params: ProspectParams) -> Self { ... }

    /// Compute the subjective value of a P&L outcome.
    /// Gains: v(x) = x^alpha
    /// Losses: v(x) = -lambda * |x|^beta
    pub fn value(&self, pnl: f64) -> f64 { ... }

    /// Update the reference point via EMA: ref = (1 - tau) * ref + tau * pnl.
    pub fn adapt_reference(&mut self, pnl: f64) { ... }
}
```

**8.1.3** Implement the `TradingOutcome` branch in `appraise` at `lib.rs:2204`:

```rust
AffectEvent::TradingOutcome {
    pnl_usd,
    prediction_error,
    accuracy_trend,
    asset: _,
    strategy_id: _,
} => {
    // Pleasure: prospect-theory weighted P&L.
    // Losses weighted 1.6x more than gains.
    let subjective = self.prospect.value(pnl_usd);
    let pleasure = (subjective / self.pnl_scale).clamp(-0.40, 0.25);

    // Arousal: proportional to absolute prediction error.
    let arousal = (prediction_error.abs() * 0.3).clamp(0.0, 0.30);

    // Dominance: EMA of accuracy trend.
    let dominance = (accuracy_trend * 0.15).clamp(-0.20, 0.15);

    // Confidence: correlated with accuracy trend.
    let confidence = (accuracy_trend * 0.10).clamp(-0.15, 0.10);

    self.prospect.adapt_reference(pnl_usd);
    self.state.apply_delta(pleasure, arousal, dominance, confidence, now);
}
```

**8.1.4** Add `prospect: ProspectValueFunction` and `pnl_scale: f64` fields to `DaimonState`. Initialize prospect with `ProspectParams::default()` and pnl_scale = 1000.0 (normalizes typical trade P&L to ~0.1 pleasure delta). Make both serializable for persistence.

**8.1.5** Wire `TradingOutcome` events from orchestrate.rs -- add a helper function `appraise_trading_outcome` that constructs the `AffectEvent::TradingOutcome` from a `TradingReflectEvent` and calls `daimon.appraise()`.

### Wiring

In `crates/roko-daimon/src/lib.rs`, add:
```rust
/// Prospect-theory value function for asymmetric loss aversion.
pub mod prospect;
```

Re-export `ProspectValueFunction` and `ProspectParams`.

### Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn loss_aversion_weights_losses_more_than_gains() {
        // +100 P&L -> subjective value V. -100 P&L -> subjective value < -V * 1.5.
        let pv = ProspectValueFunction::new(ProspectParams::default());
        let gain = pv.value(100.0);
        let loss = pv.value(-100.0);
        assert!(loss.abs() > gain.abs() * 1.5);
    }

    #[test]
    fn trading_outcome_moves_pad_vector() {
        // Appraise a +50 P&L -> pleasure should increase.
        // Appraise a -50 P&L -> pleasure should decrease more than the increase.
    }

    #[test]
    fn reference_point_adapts_to_recent_performance() {
        // After 10 gains of ~100, reference point shifts upward.
        let mut pv = ProspectValueFunction::new(ProspectParams::default());
        for _ in 0..10 {
            pv.adapt_reference(100.0);
        }
        assert!(pv.reference_point > 50.0);
    }

    #[test]
    fn arousal_proportional_to_prediction_error_magnitude() {
        // Large prediction error -> high arousal delta.
        // Small prediction error -> low arousal delta.
    }
}
```

### Verification

```bash
cargo test -p roko-daimon -- prospect
cargo test -p roko-daimon -- appraise
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-daimon
```

### Acceptance criteria

- [ ] `AffectEvent::TradingOutcome` variant added with pnl_usd, prediction_error, accuracy_trend
- [ ] `ProspectValueFunction` implements Kahneman-Tversky asymmetric weighting (1.6x loss aversion)
- [ ] `ProspectParams` configurable with defaults matching the literature
- [ ] Reference point adapts via EMA so the "neutral" outcome shifts with recent performance
- [ ] `appraise` maps `TradingOutcome` through prospect theory to PAD deltas
- [ ] Pleasure asymmetry: losses produce larger negative pleasure than equivalent gains produce positive
- [ ] Arousal proportional to absolute prediction error
- [ ] Dominance derived from accuracy trend
- [ ] `DaimonState` persists prospect function state across sessions
- [ ] All tests pass, clippy clean, fmt clean

### Commit message

```
feat(roko-daimon): add P&L-to-PAD mapping with prospect-theory loss aversion
```

---

## Batch 8.2: Affect-to-position-sizing and tilt detection

> **Effort**: M | **Depends on**: 8.1, 4.1 (risk limits) | **Crate**: roko-daimon, roko-agent
> **Branch**: `defi/batch-8.2-affect-position-sizing`

### Context

The daimon produces five behavioral modulation channels through `AffectBehaviorModulation` at `phase2_stubs.rs:92`. The critical channel for DeFi is `risk_tolerance` (float in [0.0, 1.0]). The anxious profile at `phase2_stubs.rs:139` sets risk_tolerance = 0.15; the confident profile at line 120 (inferred from the balanced default) sets 0.70. These values modulate dispatch strategy but do not yet affect position sizing.

The safety layer at `roko-agent/src/safety/risk.rs` provides `kelly_fraction` at line 167 (`f* = (p * b - q) / b`) and `SafetyBudget` at line 223 with `cost_limit_usd` and `irreversibility_limit`. The gap is connecting the daimon's `risk_tolerance` to these safety primitives so that affect state directly constrains position size.

Tilt detection -- the state where emotional arousal overrides rational decision-making -- is not implemented. The PAD pattern for tilt (arousal > 0.8, dominance < 0.2, pleasure < -0.3) maps to the `Anxious` octant in `AffectOctant::from_pad` at `phase2_stubs.rs:42`, but there is no forced cooldown or intervention. This batch adds explicit tilt detection, intervention rules, and the affect-to-sizing pipeline.

### Read first

| File | Why |
|------|-----|
| `crates/roko-daimon/src/phase2_stubs.rs:82-113` | `AffectBehaviorModulation` -- 5 channels, `risk_tolerance` |
| `crates/roko-daimon/src/phase2_stubs.rs:139-150` | `anxious()` profile -- most conservative risk_tolerance = 0.15 |
| `crates/roko-daimon/src/phase2_stubs.rs:42-62` | `AffectOctant::from_pad` -- PAD-to-octant classification |
| `crates/roko-agent/src/safety/risk.rs:148-175` | `kelly_fraction`, `confidence_multiplier` |
| `crates/roko-agent/src/safety/risk.rs:223-234` | `SafetyBudget` fields: irreversibility_limit, cost_limit_usd |
| `crates/roko-daimon/src/lib.rs:310-340` | `AffectState` -- pad, confidence, behavioral_state |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**8.2.1** Create `crates/roko-daimon/src/position_modulation.rs`

```rust
use serde::{Deserialize, Serialize};
use roko_core::PadVector;

/// Configuration for affect-based position sizing modulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionModulationConfig {
    /// How strongly arousal reduces position size. Default 0.5.
    pub arousal_weight: f64,
    /// How strongly dominance permits larger positions. Default 0.3.
    pub dominance_weight: f64,
    /// Minimum position size multiplier (floor). Default 0.1.
    pub min_multiplier: f64,
    /// Maximum position size multiplier (ceiling). Default 1.0.
    pub max_multiplier: f64,
}

impl Default for PositionModulationConfig {
    fn default() -> Self {
        Self {
            arousal_weight: 0.5,
            dominance_weight: 0.3,
            min_multiplier: 0.1,
            max_multiplier: 1.0,
        }
    }
}

/// Compute a position size multiplier from current affect state.
///
/// High arousal reduces size. High dominance permits larger size.
/// Output in [config.min_multiplier, config.max_multiplier].
///
/// Formula:
///   base = risk_tolerance (from AffectBehaviorModulation, in [0, 1])
///   arousal_penalty = arousal * arousal_weight
///   dominance_bonus = dominance.max(0) * dominance_weight
///   multiplier = (base - arousal_penalty + dominance_bonus).clamp(min, max)
pub fn position_size_multiplier(
    pad: &PadVector,
    risk_tolerance: f64,
    config: &PositionModulationConfig,
) -> f64 { ... }

/// Modulate a Kelly fraction by affect state.
///
/// kelly_adjusted = kelly_base * position_size_multiplier(pad, risk_tolerance)
pub fn affect_adjusted_kelly(
    kelly_base: f64,
    pad: &PadVector,
    risk_tolerance: f64,
    config: &PositionModulationConfig,
) -> f64 { ... }
```

**8.2.2** Add tilt detection in `crates/roko-daimon/src/tilt.rs`:

```rust
use chrono::{DateTime, Utc};
use roko_core::PadVector;
use serde::{Deserialize, Serialize};

/// Tilt detection thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiltThresholds {
    pub min_arousal: f64,       // default 0.8
    pub max_dominance: f64,     // default 0.2
    pub max_pleasure: f64,      // default -0.3
}

impl Default for TiltThresholds {
    fn default() -> Self {
        Self {
            min_arousal: 0.8,
            max_dominance: 0.2,
            max_pleasure: -0.3,
        }
    }
}

/// Tilt state tracker with cooldown and recovery monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiltDetector {
    pub thresholds: TiltThresholds,
    pub is_tilted: bool,
    pub tilt_start: Option<DateTime<Utc>>,
    pub cooldown_secs: u64,
    pub tilt_count: u64,
    pub last_recovery: Option<DateTime<Utc>>,
}

impl TiltDetector {
    pub fn new(thresholds: TiltThresholds, cooldown_secs: u64) -> Self { ... }

    /// Check PAD vector against tilt thresholds.
    pub fn check(&mut self, pad: &PadVector, now: DateTime<Utc>) -> TiltStatus { ... }

    /// Returns true if the agent is tilted or in cooldown.
    pub fn is_restricted(&self, now: DateTime<Utc>) -> bool { ... }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TiltStatus {
    /// Normal operation.
    Clear,
    /// Tilt detected -- restrict trading.
    Tilted,
    /// Recovering from tilt -- still in cooldown.
    Cooldown,
}
```

**8.2.3** Integrate tilt detection into `DaimonState`. Add a `tilt: TiltDetector` field. After every `appraise` call, run `tilt.check(pad)`. When tilted, force `risk_tolerance` to 0.0 in the modulation profile.

**8.2.4** Add `TiltEvent` for episode logging:

```rust
/// Event recorded when tilt is detected or recovery occurs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiltEvent {
    pub status: String, // "tilted", "cooldown", "recovered"
    pub pad_at_trigger: PadVector,
    pub timestamp: DateTime<Utc>,
    pub tilt_duration_secs: Option<f64>,
}
```

**8.2.5** Wire `position_size_multiplier` into the safety budget. In the integration point where `SafetyBudget` is constructed for a trading task, apply the affect-modulated multiplier to `cost_limit_usd`:

```rust
budget.cost_limit_usd *= position_size_multiplier(&pad, risk_tolerance, &config);
```

### Wiring

In `crates/roko-daimon/src/lib.rs`, add:
```rust
/// Affect-to-position-sizing modulation.
pub mod position_modulation;
/// Tilt detection and cooldown.
pub mod tilt;
```

Re-export `PositionModulationConfig`, `position_size_multiplier`, `TiltDetector`, `TiltStatus`.

### Tests

```rust
// position_modulation tests
#[cfg(test)]
mod tests {
    #[test]
    fn high_arousal_reduces_position_size() {
        let pad = PadVector::new(-0.2, 0.9, -0.1);
        let mult = position_size_multiplier(&pad, 0.5, &PositionModulationConfig::default());
        assert!(mult < 0.3);
    }

    #[test]
    fn high_dominance_permits_larger_positions() {
        let pad = PadVector::new(0.3, 0.1, 0.8);
        let mult = position_size_multiplier(&pad, 0.5, &PositionModulationConfig::default());
        assert!(mult > 0.5);
    }

    #[test]
    fn multiplier_stays_in_configured_bounds() {
        // Extreme PAD values still produce output within [min, max].
    }
}

// tilt tests
#[cfg(test)]
mod tests {
    #[test]
    fn detects_tilt_from_pad_pattern() {
        let pad = PadVector::new(-0.5, 0.9, 0.1);
        let mut detector = TiltDetector::new(TiltThresholds::default(), 300);
        assert_eq!(detector.check(&pad, Utc::now()), TiltStatus::Tilted);
    }

    #[test]
    fn cooldown_prevents_immediate_recovery() {
        // Tilt detected, then PAD returns to normal. Still in cooldown.
    }

    #[test]
    fn tilt_forces_zero_risk_tolerance() {
        // When tilted, modulation profile risk_tolerance = 0.0.
    }
}
```

### Verification

```bash
cargo test -p roko-daimon -- position_modulation
cargo test -p roko-daimon -- tilt
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-daimon
```

### Acceptance criteria

- [ ] `position_size_multiplier` reduces sizing with high arousal, increases with high dominance
- [ ] `affect_adjusted_kelly` produces kelly_base * multiplier, clamped to [0, 1]
- [ ] `TiltDetector` detects tilt when arousal > 0.8, dominance < 0.2, pleasure < -0.3
- [ ] Cooldown period prevents immediate recovery from tilt
- [ ] Tilt forces risk_tolerance to 0.0 in modulation profile
- [ ] `TiltEvent` recorded for episode logging
- [ ] Position size modulation wired into `SafetyBudget.cost_limit_usd`
- [ ] `DaimonState` persists tilt detector state
- [ ] All tests pass, clippy clean, fmt clean

### Commit message

```
feat(roko-daimon): add affect-to-position-sizing modulation and tilt detection
```

---

## Batch 8.3: Somatic-TA HDC binding and strategy space

> **Effort**: L | **Depends on**: 8.1, 3.6 (HDC composite indicators) | **Crate**: roko-daimon, roko-primitives
> **Branch**: `defi/batch-8.3-somatic-hdc`

### Context

The PRD's somatic-TA specification describes HDC bindings between TA pattern hypervectors and PAD affect states:

```
somatic_marker = bind(pattern_hv, affect_hv)
```

This creates a somatic map -- a single 10,240-bit hypervector encoding the agent's pattern-outcome-emotion history. Given a detected pattern, unbind to recover the associated affect before deliberation begins.

The existing implementation at `somatic_ta.rs:52` uses float-space `StrategyCoordinates` (8D `[f64; 8]`) with KdTree nearest-neighbor lookup via `kiddo::KdTree<f64, 8>`. This works for coding tasks but misses the HDC composition that makes somatic maps transferable across agent generations and compressible into fixed-size summaries.

HDC primitives already exist in `roko-primitives/src/hdc.rs`: `HdcVector` (10,240-bit), `bind` (XOR, line 113), `bundle` (majority vote, line 129), and `text_fingerprint`. The gap is encoding PAD vectors as hypervectors and composing the somatic map from (pattern, affect) bindings.

The current 8D strategy space dimensions (complexity, risk, novelty, confidence, time_pressure, scope, reversibility, dependency_depth) are defined at `lib.rs:427-522`. `StrategySpaceDefinition` at line 526 is configurable via `roko.toml [daimon.strategy_space]` with domain defaulting to "coding". The DeFi strategy space needs dimensions like volatility, position_concentration, liquidity_depth, signal_strength, time_to_funding, correlation_regime, drawdown_proximity, gas_congestion. The configuration mechanism exists; the missing piece is a `from_market_state` constructor.

### Read first

| File | Why |
|------|-----|
| `crates/roko-primitives/src/hdc.rs:113-140` | `HdcVector::bind` (XOR), `HdcVector::bundle` (majority vote) |
| `crates/roko-daimon/src/somatic_ta.rs:52-106` | `SomaticOracleContext`, `somatic_confidence_bias`, `from_landscape` |
| `crates/roko-daimon/src/lib.rs:427-522` | `StrategyCoordinates`, 8D fields, `as_array` |
| `crates/roko-daimon/src/lib.rs:524-613` | `StrategySpaceDefinition`, `validate`, `computer` |
| `crates/roko-daimon/src/lib.rs:615-650` | `TaskStrategyObservation` -- task-to-coordinate projection |
| `crates/roko-daimon/src/somatic_ta.rs:150-200` | `SomaticRetrieval`, `SomaticRetrievalConfig` |
| `crates/roko-daimon/src/lib.rs:165-175` | `STRATEGY_DIMENSIONS = 8`, `SomaticTree`, somatic constants |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**8.3.1** Create `crates/roko-daimon/src/somatic_hdc.rs` -- HDC encoding for PAD vectors:

```rust
use roko_core::PadVector;
use roko_primitives::hdc::HdcVector;
use serde::{Deserialize, Serialize};

/// Number of quantization buckets per PAD dimension.
const PAD_BUCKETS: usize = 7;

/// Encode a PAD vector as a hypervector using thermometer encoding.
///
/// Each PAD dimension (pleasure, arousal, dominance) is quantized into
/// 7 buckets over [-1, 1], then thermometer-encoded: bucket k activates
/// all bits for buckets 0..=k. The three dimension encodings are bound
/// together to produce a single 10,240-bit vector.
pub fn encode_pad(pad: &PadVector) -> HdcVector { ... }

/// Decode a PAD hypervector back to approximate PAD values.
///
/// This is lossy due to quantization, but preserves octant and magnitude.
pub fn decode_pad(hv: &HdcVector, basis: &PadBasis) -> PadVector { ... }

/// Pre-computed random basis vectors for PAD dimension encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadBasis {
    /// One basis vector per PAD dimension (3 total).
    pub dimension_bases: [HdcVector; 3],
    /// One basis vector per quantization bucket (7 total).
    pub bucket_bases: [HdcVector; PAD_BUCKETS],
}

impl PadBasis {
    /// Generate a random basis. Use a fixed seed for reproducibility.
    pub fn seeded(seed: u64) -> Self { ... }
}
```

**8.3.2** Add somatic map operations in the same file:

```rust
/// A somatic marker binding a TA pattern to its associated affect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SomaticBinding {
    pub pattern_label: String,
    pub marker: HdcVector,
    pub timestamp_ms: i64,
}

/// Somatic map: bundle of (pattern, affect) bindings.
///
/// The map is a single 10,240-bit hypervector that encodes the agent's
/// entire pattern-outcome-emotion history. Sub-100ns retrieval via unbind.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SomaticMap {
    pub bindings: Vec<SomaticBinding>,
    /// Bundled composite of all bindings. Updated on each insert.
    pub composite: HdcVector,
}

impl SomaticMap {
    /// Bind a pattern hypervector with a PAD affect vector and add to the map.
    pub fn record(&mut self, pattern_label: &str, pattern_hv: &HdcVector, pad: &PadVector, basis: &PadBasis) {
        let affect_hv = encode_pad(pad);
        let marker = pattern_hv.bind(&affect_hv);
        // ... store and re-bundle composite
    }

    /// Retrieve the associated affect for a detected pattern via unbind.
    ///
    /// unbind(composite, pattern_hv) -> approximate affect_hv -> decode to PAD.
    pub fn retrieve_affect(&self, pattern_hv: &HdcVector, basis: &PadBasis) -> Option<PadVector> { ... }

    /// Number of bindings in the map.
    pub fn len(&self) -> usize { self.bindings.len() }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool { self.bindings.is_empty() }
}
```

**8.3.3** Add DeFi strategy space constructor in `lib.rs`:

```rust
/// Market state observation for DeFi strategy coordinate projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketStrategyObservation {
    /// Rolling volatility of the target asset in [0.0, 1.0].
    pub volatility: f64,
    /// Current exposure as fraction of portfolio in [0.0, 1.0].
    pub position_concentration: f64,
    /// Normalized liquidity depth at target price levels in [0.0, 1.0].
    pub liquidity_depth: f64,
    /// Composite TA indicator confidence in [0.0, 1.0].
    pub signal_strength: f64,
    /// Hours until next funding payment, normalized in [0.0, 1.0].
    pub time_to_funding: f64,
    /// Cross-asset correlation state in [0.0, 1.0].
    pub correlation_regime: f64,
    /// Distance to max drawdown limit in [0.0, 1.0].
    pub drawdown_proximity: f64,
    /// Current L1/L2 gas utilization in [0.0, 1.0].
    pub gas_congestion: f64,
}

impl MarketStrategyObservation {
    /// Convert to strategy coordinates for the somatic landscape.
    pub fn to_coordinates(&self) -> StrategyCoordinates {
        StrategyCoordinates::new(
            self.volatility,
            self.position_concentration,
            self.liquidity_depth,
            self.signal_strength,
            self.time_to_funding,
            self.correlation_regime,
            self.drawdown_proximity,
            self.gas_congestion,
        )
    }
}
```

**8.3.4** Add `StrategySpaceDefinition::defi()` factory:

```rust
impl StrategySpaceDefinition {
    pub fn defi() -> Self {
        Self {
            domain: "defi".to_string(),
            dimensions: [
                "volatility".to_string(),
                "position_concentration".to_string(),
                "liquidity_depth".to_string(),
                "signal_strength".to_string(),
                "time_to_funding".to_string(),
                "correlation_regime".to_string(),
                "drawdown_proximity".to_string(),
                "gas_congestion".to_string(),
            ],
        }
    }
}
```

**8.3.5** Add `somatic_hdc` persistence: `SomaticMap::save` and `SomaticMap::load` methods writing to `.roko/daimon/somatic-map.json`.

### Wiring

In `crates/roko-daimon/src/lib.rs`, add:
```rust
/// HDC-encoded somatic markers for pattern-affect binding.
pub mod somatic_hdc;
```

Re-export `SomaticMap`, `PadBasis`, `encode_pad`, `MarketStrategyObservation`.

### Tests

```rust
// somatic_hdc tests
#[cfg(test)]
mod tests {
    #[test]
    fn encode_decode_preserves_pad_octant() {
        let pad = PadVector::new(0.5, -0.3, 0.7);
        let basis = PadBasis::seeded(42);
        let hv = encode_pad(&pad);
        let decoded = decode_pad(&hv, &basis);
        // Octant should match (positive P, negative A, positive D).
        assert!(decoded.pleasure > 0.0);
        assert!(decoded.arousal < 0.0);
        assert!(decoded.dominance > 0.0);
    }

    #[test]
    fn bind_unbind_recovers_approximate_affect() {
        let basis = PadBasis::seeded(42);
        let mut map = SomaticMap::default();
        let pattern_hv = HdcVector::random();
        let pad = PadVector::new(0.6, 0.2, 0.4);
        map.record("rsi_oversold", &pattern_hv, &pad, &basis);
        let recovered = map.retrieve_affect(&pattern_hv, &basis);
        assert!(recovered.is_some());
        // Recovered PAD should be in the same octant.
    }

    #[test]
    fn somatic_map_bundles_multiple_bindings() {
        // Record 5 different patterns. Retrieve each. All should approximately recover.
    }
}

// strategy space tests
#[cfg(test)]
mod tests {
    #[test]
    fn defi_strategy_space_has_correct_dimensions() {
        let space = StrategySpaceDefinition::defi();
        assert_eq!(space.domain, "defi");
        assert_eq!(space.dimensions[0], "volatility");
        assert_eq!(space.dimensions[7], "gas_congestion");
    }

    #[test]
    fn market_observation_to_coordinates_clamps_to_unit() {
        let obs = MarketStrategyObservation {
            volatility: 1.5, // should clamp to 1.0
            position_concentration: -0.1, // should clamp to 0.0
            ..Default::default()
        };
        let coords = obs.to_coordinates();
        assert!((0.0..=1.0).contains(&coords.complexity)); // mapped to first dim
    }
}
```

### Verification

```bash
cargo test -p roko-daimon -- somatic_hdc
cargo test -p roko-daimon -- market_strategy
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-daimon
```

### Acceptance criteria

- [ ] `encode_pad` produces a 10,240-bit `HdcVector` from a `PadVector`
- [ ] `decode_pad` recovers approximate PAD values (same octant, approximate magnitude)
- [ ] `SomaticMap::record` creates a `bind(pattern_hv, affect_hv)` marker
- [ ] `SomaticMap::retrieve_affect` recovers approximate PAD via unbind
- [ ] `SomaticMap` serializes and deserializes for cross-session persistence
- [ ] `MarketStrategyObservation` converts to `StrategyCoordinates` for KdTree queries
- [ ] `StrategySpaceDefinition::defi()` factory provides correct 8D DeFi dimensions
- [ ] `PadBasis::seeded` produces deterministic basis vectors for reproducibility
- [ ] All tests pass, clippy clean, fmt clean

### Commit message

```
feat(roko-daimon): add HDC somatic map and DeFi strategy space dimensions
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives Used

- **Extension**: `TiltDetector` (affect-based execution modifier — detects emotional tilt and throttles/pauses agent), `PositionSizer` (affect-modulated position sizing — reduces size during high-arousal states). Both are Tier 3 (Roko-native) extensions operating at the Affect layer (layer 6) of the `Extension` trait, using the `validate` hook to intercept actions when tilt is detected.
- **Recipe**: `ProspectValueFunction` (P&L -> PAD vector transform with 1.6x loss aversion weighting), affect-to-position-sizing pipeline
- **Knowledge Entry**: `SomaticMap` (TA pattern -> affect vector bindings -- durable memory of how market patterns feel)
- **Signal**: Tilt detection signals (alerts when agent enters tilt state), affect state change signals

### Authoring Surfaces

- **Extension Workshop** — configure affect modulation extensions: loss aversion weight, tilt thresholds, cooldown periods
- **Agent Detail > Affect Panel** -- live PAD vector visualization, tilt status, position sizing adjustments
- **Recipe Editor** -- build prospect value and somatic mapping pipelines

### Shareable Artifacts

- Affect modulation extension templates (conservative/moderate/aggressive tilt detection)
- Prospect value function configurations (loss aversion weights for different risk profiles)
- Somatic map templates (pre-trained TA pattern -> affect bindings)

### Dashboard Visibility

- **Agent Detail > Affect Panel** -- real-time PAD vector, tilt status indicator, position sizing factor
- **Pulse > Affect** -- fleet-wide affect heatmap
- **Knowledge > Somatic Maps** -- somatic marker library with pattern matches
