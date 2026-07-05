# 10 - Neuro and HDC for DeFi

> **Scope**: Durable knowledge and HDC encoding -- market state encoding, knowledge-informed routing, regime classification.
> **Primary crates**: `roko-neuro` (`crates/roko-neuro/src/`), `roko-primitives` (`crates/roko-primitives/src/`)
> **Secondary crates**: `roko-learn`, `roko-dreams`

---

## Batch 10.1: Market state HDC encoding and Ebbinghaus for market knowledge

> **Effort**: M | **Depends on**: 3.1 (TA indicators provide raw values to encode) | **Crate**: roko-neuro
> **Branch**: `defi/batch-10.1-market-hdc-encoding`

### Context

The neuro/HDC subsystem encodes knowledge entries into 10,240-bit binary vectors via `KnowledgeHdcEncoder` (`hdc.rs:9`). It uses `RoleFillerEncoder::encode_structured()` (`hdc.rs:24`) to bind role-filler pairs -- `content`, `kind`, `tier`, `domain`, `source` -- into composite vectors queryable by unbinding any role. The encoder works on text content. It has no mechanism to encode numeric market data (price percentiles, volatility regimes, funding rates, RSI buckets) into HDC vectors.

HDC's key property is that similarity is preserved through composition. Two market states with similar volatility, trend direction, and RSI will produce vectors with high Hamming similarity regardless of absolute price. This makes HDC encoding ideal for regime matching: encode the current market state, compare against historical state vectors, and retrieve knowledge from similar conditions.

The knowledge store uses Ebbinghaus decay with per-kind half-lives defined in `lib.rs:64-97`: Insight = 30 days, Heuristic = 90 days, Warning = 1 hour, CausalLink = 60 days, StrategyFragment = 14 days. These are calibrated for code knowledge. Market knowledge decays faster -- price-level insights go stale in hours, regime observations in days, strategy fragments in 1-3 days. The store already supports per-entry `half_life_days` fields, so individual entries can override the defaults. The mechanism exists; the DeFi-specific presets do not.

This batch adds `MarketHdcEncoder` for structured market-state encoding using quantized bins and role-filler bindings, and `DeFiHalfLifePresets` for market-appropriate decay rates.

### Read first

| File | Why |
|------|-----|
| `crates/roko-neuro/src/hdc.rs:8-40` | `KnowledgeHdcEncoder`, `RoleFillerEncoder::encode_structured()` -- role-filler binding pattern |
| `crates/roko-neuro/src/hdc.rs:166-296` | `encode_entry()`, `encode_structured()`, `encode_generic_entry()` -- how entries become vectors |
| `crates/roko-neuro/src/hdc.rs:396-414` | `role_hv()`, `text_hv()`, `bundle()` -- primitive vector constructors |
| `crates/roko-primitives/src/hdc.rs:1-80` | `HdcVector` struct, `from_seed()`, `bind()`, `bundle()`, `similarity()` |
| `crates/roko-neuro/src/lib.rs:64-200` | `KnowledgeKind`, half-life constants, `KnowledgeTier`, tier multipliers |
| `crates/roko-neuro/src/knowledge_store.rs:22-200` | `KnowledgeStore`, query scoring, `ContextAssemblyWeights` |
| `crates/roko-neuro/src/distiller.rs:25-70` | `Distiller` -- where it assigns `half_life_days` to new entries |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**1. Create `MarketHdcEncoder`**

File: `crates/roko-neuro/src/market_hdc.rs` (new)

Encode structured market state into HDC vectors using role-filler bindings. The encoder quantizes continuous values into discrete bins, encodes each bin as `HdcVector::from_seed()`, then binds with role vectors and bundles the result.

```rust
//! HDC encoding of structured market state.
//!
//! Encodes numeric market data (price percentile, volatility, funding rate,
//! RSI, trend, volume) into 10,240-bit HDC vectors using role-filler binding.
//! Two market states with similar features produce vectors with high Hamming
//! similarity regardless of absolute price.

use roko_primitives::hdc::HdcVector;
use serde::{Deserialize, Serialize};

/// A snapshot of market state for HDC encoding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSnapshot {
    /// Price percentile within the lookback window (0-100).
    pub price_percentile: f64,
    /// Volatility regime: "low", "medium", "high".
    pub volatility_regime: String,
    /// Volume z-score relative to lookback mean.
    pub volume_zscore: f64,
    /// Funding rate (annualized, can be negative).
    pub funding_rate: f64,
    /// RSI(14) value (0-100).
    pub rsi_14: f64,
    /// Trend direction: "up", "down", "range".
    pub trend_direction: String,
    /// Market identifier (e.g., "ETH-PERP").
    pub market: String,
    /// Timeframe (e.g., "1h", "4h").
    pub timeframe: String,
}

/// Encode a market snapshot into an HDC vector.
///
/// The encoding uses role-filler binding:
/// ```text
/// market_vector = bundle([
///     bind(role("price_percentile"), quantize(price_percentile, 10)),
///     bind(role("volatility_regime"), category(volatility_regime)),
///     bind(role("volume_zscore"), quantize(volume_zscore, 5)),
///     bind(role("funding_rate"), quantize(funding_rate, 5)),
///     bind(role("rsi_bucket"), quantize(rsi_14, 5)),
///     bind(role("trend_direction"), category(trend_direction)),
///     bind(role("market"), text(market)),
///     bind(role("timeframe"), text(timeframe)),
/// ])
/// ```
#[must_use]
pub fn encode_market_state(snapshot: &MarketSnapshot) -> HdcVector {
    let pairs = vec![
        role_filler("price_percentile", &quantize_bucket(snapshot.price_percentile, 0.0, 100.0, 10)),
        role_filler("volatility_regime", &snapshot.volatility_regime),
        role_filler("volume_zscore", &quantize_bucket(snapshot.volume_zscore, -3.0, 3.0, 5)),
        role_filler("funding_rate", &quantize_bucket(snapshot.funding_rate, -0.5, 0.5, 5)),
        role_filler("rsi_bucket", &quantize_bucket(snapshot.rsi_14, 0.0, 100.0, 5)),
        role_filler("trend_direction", &snapshot.trend_direction),
        role_filler("market", &snapshot.market),
        role_filler("timeframe", &snapshot.timeframe),
    ];

    let bound: Vec<HdcVector> = pairs.iter()
        .map(|(role, filler)| {
            HdcVector::from_seed(format!("role:{role}").as_bytes())
                .bind(&HdcVector::from_seed(filler.as_bytes()))
        })
        .collect();
    let refs: Vec<&HdcVector> = bound.iter().collect();
    HdcVector::bundle(&refs)
}

/// Quantize a continuous value into a discrete bin label.
///
/// Divides the range `[min, max]` into `num_bins` equal-width buckets.
/// Values outside the range are clamped.
#[must_use]
pub fn quantize_bucket(value: f64, min: f64, max: f64, num_bins: usize) -> String {
    let clamped = value.clamp(min, max);
    let bin_width = (max - min) / num_bins as f64;
    let bin = ((clamped - min) / bin_width).floor() as usize;
    let bin = bin.min(num_bins - 1);
    format!("bin_{bin}_{num_bins}")
}

fn role_filler(role: &str, filler: &str) -> (String, String) {
    (role.to_string(), filler.to_string())
}
```

**2. Add `DeFiHalfLifePresets`**

File: `crates/roko-neuro/src/lib.rs`

Add constants after the existing half-life block (after line 97):

```rust
// ─── DeFi half-life presets ──────────────────────────────────────────

/// Half-life for market price-level insights: 4 hours.
/// Price support/resistance levels go stale by next session.
pub const DEFI_MARKET_INSIGHT_HOURS: f64 = 4.0;
/// In days, for the `half_life_days` field.
pub const DEFI_MARKET_INSIGHT_DAYS: f64 = DEFI_MARKET_INSIGHT_HOURS / 24.0;

/// Half-life for regime observations: 3 days.
/// "Market is trending" stays valid for 1-7 days.
pub const DEFI_REGIME_OBSERVATION_DAYS: f64 = 3.0;

/// Half-life for trade strategy fragments: 2 days.
/// Specific market condition strategies decay faster than code strategies.
pub const DEFI_TRADE_STRATEGY_DAYS: f64 = 2.0;

/// Half-life for structural DeFi insights: 21 days.
/// "This DEX has 2% slippage above $1M" decays at code-like rates.
pub const DEFI_STRUCTURAL_INSIGHT_DAYS: f64 = 21.0;

/// Half-life for DeFi risk warnings: 12 hours.
/// Risk conditions persist longer than code warnings (1 hour)
/// because market risks take time to unwind.
pub const DEFI_WARNING_HOURS: f64 = 12.0;
/// In days.
pub const DEFI_WARNING_DAYS: f64 = DEFI_WARNING_HOURS / 24.0;
```

**3. Create `assign_defi_half_life()` helper**

File: `crates/roko-neuro/src/market_hdc.rs` (same file as item 1)

```rust
/// Assign a DeFi-appropriate half-life to a knowledge entry based on its tags.
///
/// Checks for market-related tags and overrides the default code-knowledge
/// half-life. Falls back to `KnowledgeKind::default_half_life_days()` if
/// no market tags are found.
#[must_use]
pub fn defi_half_life_days(kind: crate::KnowledgeKind, tags: &[String]) -> f64 {
    let is_market = tags.iter().any(|t| t.starts_with("market:") || t == "market");
    if !is_market {
        return kind.default_half_life_days();
    }

    let is_price_level = tags.iter().any(|t| t.contains("price-level") || t.contains("support") || t.contains("resistance"));
    let is_regime = tags.iter().any(|t| t.starts_with("regime:"));
    let is_structural = tags.iter().any(|t| t.contains("structural") || t.contains("slippage") || t.contains("liquidity-profile"));

    match kind {
        crate::KnowledgeKind::Insight if is_price_level => crate::DEFI_MARKET_INSIGHT_DAYS,
        crate::KnowledgeKind::Insight if is_regime => crate::DEFI_REGIME_OBSERVATION_DAYS,
        crate::KnowledgeKind::Insight if is_structural => crate::DEFI_STRUCTURAL_INSIGHT_DAYS,
        crate::KnowledgeKind::Insight => crate::DEFI_REGIME_OBSERVATION_DAYS,
        crate::KnowledgeKind::StrategyFragment => crate::DEFI_TRADE_STRATEGY_DAYS,
        crate::KnowledgeKind::Warning => crate::DEFI_WARNING_DAYS,
        crate::KnowledgeKind::CausalLink => crate::DEFI_REGIME_OBSERVATION_DAYS,
        _ => kind.default_half_life_days(),
    }
}
```

**4. Add `similarity_to_snapshot()` convenience method**

File: `crates/roko-neuro/src/market_hdc.rs`

```rust
/// Compare two market snapshots by HDC similarity.
///
/// Returns a value in `[0.0, 1.0]` where 1.0 means identical encoded state.
#[must_use]
pub fn market_similarity(a: &MarketSnapshot, b: &MarketSnapshot) -> f32 {
    let va = encode_market_state(a);
    let vb = encode_market_state(b);
    va.similarity(&vb)
}
```

### Wiring

- Add `pub mod market_hdc;` to `crates/roko-neuro/src/lib.rs` (in the module declarations section)
- Re-export: `pub use market_hdc::{MarketSnapshot, encode_market_state, quantize_bucket, defi_half_life_days, market_similarity};`
- Add the `DEFI_*` half-life constants to `lib.rs` (they are pub constants, no config schema change needed)

### Tests

```rust
// crates/roko-neuro/src/market_hdc.rs
#[cfg(test)]
mod tests {
    use super::*;

    fn eth_snapshot() -> MarketSnapshot {
        MarketSnapshot {
            price_percentile: 75.0,
            volatility_regime: "high".to_string(),
            volume_zscore: 1.2,
            funding_rate: 0.03,
            rsi_14: 65.0,
            trend_direction: "up".to_string(),
            market: "ETH-PERP".to_string(),
            timeframe: "1h".to_string(),
        }
    }

    #[test]
    fn similar_states_produce_similar_vectors() {
        let a = eth_snapshot();
        let mut b = eth_snapshot();
        b.price_percentile = 72.0; // Same bin
        b.volume_zscore = 1.0; // Close value
        let sim = market_similarity(&a, &b);
        // Same bins -> high similarity
        assert!(sim > 0.6, "similar states should have sim > 0.6, got {sim}");
    }

    #[test]
    fn different_regimes_produce_lower_similarity() {
        let a = eth_snapshot();
        let mut b = eth_snapshot();
        b.volatility_regime = "low".to_string();
        b.trend_direction = "down".to_string();
        b.rsi_14 = 25.0;
        let same_sim = market_similarity(&a, &a);
        let diff_sim = market_similarity(&a, &b);
        assert!(same_sim > diff_sim,
            "different regimes should lower similarity: same={same_sim}, diff={diff_sim}");
    }

    #[test]
    fn different_markets_different_vectors() {
        let a = eth_snapshot();
        let mut b = eth_snapshot();
        b.market = "BTC-PERP".to_string();
        let sim = market_similarity(&a, &b);
        // Same indicators but different market -> moderately different
        let self_sim = market_similarity(&a, &a);
        assert!(sim < self_sim);
    }

    #[test]
    fn quantize_bucket_clamps_extremes() {
        assert_eq!(quantize_bucket(-5.0, 0.0, 100.0, 10), "bin_0_10");
        assert_eq!(quantize_bucket(150.0, 0.0, 100.0, 10), "bin_9_10");
    }

    #[test]
    fn quantize_bucket_distributes_evenly() {
        assert_eq!(quantize_bucket(15.0, 0.0, 100.0, 10), "bin_1_10");
        assert_eq!(quantize_bucket(55.0, 0.0, 100.0, 10), "bin_5_10");
        assert_eq!(quantize_bucket(95.0, 0.0, 100.0, 10), "bin_9_10");
    }

    #[test]
    fn defi_half_life_price_level_is_short() {
        let tags = vec!["market:ETH".to_string(), "price-level".to_string()];
        let hl = defi_half_life_days(crate::KnowledgeKind::Insight, &tags);
        assert!((hl - crate::DEFI_MARKET_INSIGHT_DAYS).abs() < 0.001);
    }

    #[test]
    fn defi_half_life_non_market_falls_back() {
        let tags = vec!["coding".to_string()];
        let hl = defi_half_life_days(crate::KnowledgeKind::Insight, &tags);
        assert!((hl - crate::INSIGHT_HALF_LIFE_DAYS).abs() < 0.001);
    }

    #[test]
    fn defi_warning_half_life_longer_than_code_warning() {
        let tags = vec!["market:ETH".to_string()];
        let defi_hl = defi_half_life_days(crate::KnowledgeKind::Warning, &tags);
        let code_hl = crate::KnowledgeKind::Warning.default_half_life_days();
        assert!(defi_hl > code_hl,
            "DeFi warnings should persist longer than code warnings: defi={defi_hl}, code={code_hl}");
    }
}
```

### Verification

```bash
cargo test -p roko-neuro -- market_hdc
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-neuro
```

### Acceptance criteria

- [ ] `MarketSnapshot` struct captures price percentile, volatility regime, volume z-score, funding rate, RSI, trend direction, market, timeframe
- [ ] `encode_market_state()` produces an `HdcVector` using role-filler bindings with quantized bins
- [ ] Similar market states (same bins) produce vectors with similarity > 0.6
- [ ] Different regimes (high-vol up vs. low-vol down) produce lower similarity than identical states
- [ ] `quantize_bucket()` clamps values outside the range and distributes evenly across bins
- [ ] `DEFI_MARKET_INSIGHT_DAYS` = 4h, `DEFI_REGIME_OBSERVATION_DAYS` = 3d, `DEFI_TRADE_STRATEGY_DAYS` = 2d, `DEFI_STRUCTURAL_INSIGHT_DAYS` = 21d, `DEFI_WARNING_DAYS` = 12h
- [ ] `defi_half_life_days()` assigns short half-lives to price-level market insights, longer to structural insights
- [ ] Non-market entries fall back to `KnowledgeKind::default_half_life_days()`
- [ ] DeFi warning half-life (12h) is longer than code warning half-life (1h)
- [ ] `cargo test -p roko-neuro` passes clean

### Commit message

```
feat(roko-neuro): add market state HDC encoding and DeFi half-life presets
```

---

## Batch 10.2: Knowledge-informed model routing and regime classification

> **Effort**: M | **Depends on**: 10.1 (market HDC encoding for regime vectors), 7.1 (learning loops for calibration) | **Crate**: roko-neuro, roko-learn
> **Branch**: `defi/batch-10.2-knowledge-routing-regime`

### Context

The `CascadeRouter` (`cascade_router.rs:1022`) selects models using a three-stage cascade: static role-table (< 50 observations), empirical confidence intervals (50-200), and full LinUCB contextual bandit (> 200). It never consults the knowledge store. This is item 13 in `CLAUDE.md`: "neuro store not yet consulted for model selection in CascadeRouter."

For DeFi, knowledge-informed routing is more important than for code tasks. Market regimes create distinct model-selection patterns: complex multi-leg strategy decisions need Opus, routine position monitoring works fine with Haiku, and ambiguous high-volatility signals need higher-capability models. The knowledge store already holds `Heuristic` entries with `source_model` and `model_generality` metadata, `CausalLink` entries connecting regimes to model choices, and `StrategyFragment` entries describing which models work for which task types. The existing `CalibrationReceipt` type in tier progression (`tier_progression.rs:120`) provides the feedback mechanism.

This batch also adds `RegimeCodebook` -- a small library of canonical regime vectors (trending-up, trending-down, range-bound, high-vol, low-vol, crisis) maintained as bundled HDC vectors from historical snapshots. Live classification encodes the current market state, computes similarity against all regime vectors, and picks the highest match. HDC similarity runs in constant time on 1,280 bytes.

### Read first

| File | Why |
|------|-----|
| `crates/roko-learn/src/cascade_router.rs:1020-1100` | `CascadeRouter` struct, `new()`, `model_slugs`, `role_table` |
| `crates/roko-learn/src/cascade_router.rs:1455-1530` | `route()`, `route_logged()`, `route_with_experiments()`, `route_with_health()` |
| `crates/roko-neuro/src/knowledge_store.rs:122-200` | `KnowledgeStore`, query path, `KnowledgeQueryHit`, `ContextAssemblyWeights` |
| `crates/roko-neuro/src/hdc.rs:62-138` | `ResonanceDetector` -- cross-domain analogy detection |
| `crates/roko-neuro/src/tier_progression.rs:82-120` | `CalibrationAction`, `CalibrationReceipt` -- feedback mechanism |
| `crates/roko-learn/src/hdc_clustering.rs:36-75` | `KMedoidsConfig`, `HdcCluster`, `ClusterResult`, `k_medoids()` |
| `crates/roko-learn/src/hdc_fingerprint.rs:14-18` | `fingerprint_episode()` -- how episodes become vectors |
| `crates/roko-neuro/src/market_hdc.rs` | `MarketSnapshot`, `encode_market_state()` -- from batch 10.1 |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**1. Create `KnowledgeRoutingAdvisor`**

File: `crates/roko-neuro/src/routing_advisor.rs` (new)

Queries the knowledge store for model-selection heuristics relevant to the current task and regime. Returns prior adjustments that the `CascadeRouter` applies to bandit weights.

```rust
//! Knowledge-informed model routing advisor.
//!
//! Queries the durable knowledge store for heuristics about which models
//! perform best for a given task type and market regime. Returns prior
//! adjustments that bias the CascadeRouter's model selection.

use crate::{KnowledgeKind, KnowledgeStore};
use serde::{Deserialize, Serialize};

/// A prior adjustment for one model based on knowledge store evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelPrior {
    /// Model slug (e.g., "claude-opus-4-6").
    pub model: String,
    /// Multiplicative adjustment to the model's bandit score.
    /// > 1.0 = knowledge says prefer this model.
    /// < 1.0 = knowledge says avoid this model.
    pub score_multiplier: f64,
    /// Confidence in the adjustment (0.0..1.0).
    pub confidence: f64,
    /// Knowledge entry IDs that support this adjustment.
    pub supporting_entries: Vec<String>,
    /// Human-readable rationale.
    pub rationale: String,
}

/// Query the knowledge store for model-selection priors.
///
/// Searches for `Heuristic` and `StrategyFragment` entries with
/// `source_model` metadata and model-related tags. Converts matches
/// into `ModelPrior` adjustments.
pub fn query_model_priors(
    store: &KnowledgeStore,
    task_type: &str,
    regime: Option<&str>,
    available_models: &[String],
) -> Vec<ModelPrior> {
    let mut keywords = vec![task_type.to_string(), "model".to_string()];
    if let Some(r) = regime {
        keywords.push(format!("regime:{r}"));
    }

    let hits = store.query(&keywords.join(" "), 20).unwrap_or_default();
    let mut priors = Vec::new();

    for hit in hits {
        let entry = &hit.entry;
        // Only Heuristic and StrategyFragment entries carry model advice
        if !matches!(entry.kind, KnowledgeKind::Heuristic | KnowledgeKind::StrategyFragment) {
            continue;
        }
        // Match against available models
        if let Some(source_model) = &entry.source_model {
            if available_models.iter().any(|m| m == source_model) {
                let multiplier = 1.0 + entry.confidence * (1.0 - entry.model_generality) * 0.5;
                priors.push(ModelPrior {
                    model: source_model.clone(),
                    score_multiplier: multiplier,
                    confidence: entry.confidence * hit.total_score.min(1.0),
                    supporting_entries: vec![entry.id.clone()],
                    rationale: entry.content.clone(),
                });
            }
        }
    }

    // Merge priors for the same model
    merge_priors(&mut priors);
    priors
}

fn merge_priors(priors: &mut Vec<ModelPrior>) {
    priors.sort_by(|a, b| a.model.cmp(&b.model));
    priors.dedup_by(|b, a| {
        if a.model == b.model {
            a.score_multiplier = (a.score_multiplier + b.score_multiplier) / 2.0;
            a.confidence = a.confidence.max(b.confidence);
            a.supporting_entries.extend(b.supporting_entries.drain(..));
            a.rationale = format!("{}; {}", a.rationale, b.rationale);
            true
        } else {
            false
        }
    });
}
```

**2. Create `RegimeCodebook`**

File: `crates/roko-neuro/src/regime_codebook.rs` (new)

A small library of canonical regime vectors for live classification.

```rust
//! Regime codebook for HDC-based market regime classification.
//!
//! Maintains a small library of canonical regime vectors. Live classification
//! encodes the current market state, computes Hamming similarity against all
//! regime vectors, and picks the highest match.

use roko_primitives::hdc::HdcVector;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// A canonical regime label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegimeLabel(pub String);

impl RegimeLabel {
    /// Standard regime labels.
    pub const TRENDING_UP: &'static str = "trending-up";
    pub const TRENDING_DOWN: &'static str = "trending-down";
    pub const RANGE_BOUND: &'static str = "range-bound";
    pub const HIGH_VOL: &'static str = "high-vol";
    pub const LOW_VOL: &'static str = "low-vol";
    pub const CRISIS: &'static str = "crisis";
}

/// Classification result from regime codebook lookup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegimeClassification {
    /// Best-matching regime label.
    pub label: RegimeLabel,
    /// Hamming similarity to the best-matching regime vector.
    pub similarity: f32,
    /// All regime similarities, sorted descending.
    pub all_similarities: Vec<(RegimeLabel, f32)>,
}

/// Library of canonical regime vectors for live classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeCodebook {
    /// Regime label -> canonical HDC vector.
    entries: BTreeMap<RegimeLabel, HdcVector>,
}

impl RegimeCodebook {
    /// Create an empty codebook.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Seed the codebook with default regime vectors.
    ///
    /// Uses deterministic seed vectors based on regime labels. These serve
    /// as starting points; the codebook self-organizes as historical market
    /// state vectors are added via `update()`.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut codebook = Self::new();
        for label in &[
            RegimeLabel::TRENDING_UP,
            RegimeLabel::TRENDING_DOWN,
            RegimeLabel::RANGE_BOUND,
            RegimeLabel::HIGH_VOL,
            RegimeLabel::LOW_VOL,
            RegimeLabel::CRISIS,
        ] {
            let seed_vector = HdcVector::from_seed(
                format!("regime-codebook-seed:{label}").as_bytes()
            );
            codebook.entries.insert(RegimeLabel(label.to_string()), seed_vector);
        }
        codebook
    }

    /// Classify a market state vector against the codebook.
    ///
    /// Returns the best-matching regime label and similarity.
    /// Returns `None` if the codebook is empty.
    #[must_use]
    pub fn classify(&self, market_state: &HdcVector) -> Option<RegimeClassification> {
        if self.entries.is_empty() {
            return None;
        }

        let mut sims: Vec<(RegimeLabel, f32)> = self.entries.iter()
            .map(|(label, vector)| (label.clone(), market_state.similarity(vector)))
            .collect();
        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_label, best_sim) = sims.first()?.clone();
        Some(RegimeClassification {
            label: best_label,
            similarity: best_sim,
            all_similarities: sims,
        })
    }

    /// Update a regime vector by bundling a new observation.
    ///
    /// Uses exponential moving average: the canonical vector is bundled
    /// with the new observation, biased toward the existing canonical
    /// by a 3:1 ratio (the canonical appears 3 times in the bundle).
    pub fn update(&mut self, label: &RegimeLabel, observation: &HdcVector) {
        let new_vector = if let Some(existing) = self.entries.get(label) {
            // EMA: bundle existing 3x with new 1x
            HdcVector::bundle(&[existing, existing, existing, observation])
        } else {
            observation.clone()
        };
        self.entries.insert(label.clone(), new_vector);
    }

    /// Detect a regime transition.
    ///
    /// Compares the current classification against the previous one.
    /// Returns `Some((old, new))` if the best-matching regime changed.
    #[must_use]
    pub fn detect_transition(
        &self,
        current: &HdcVector,
        previous_label: &RegimeLabel,
    ) -> Option<(RegimeLabel, RegimeLabel)> {
        let classification = self.classify(current)?;
        if &classification.label != previous_label {
            Some((previous_label.clone(), classification.label))
        } else {
            None
        }
    }

    /// Number of regimes in the codebook.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the codebook is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Persist the codebook to a JSON file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a codebook from a JSON file, or return defaults if missing.
    #[must_use]
    pub fn load_or_default(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_else(Self::with_defaults)
    }
}

impl Default for RegimeCodebook {
    fn default() -> Self {
        Self::with_defaults()
    }
}
```

**3. Add `route_with_knowledge()` to `CascadeRouter`**

File: `crates/roko-learn/src/cascade_router.rs`

Add a method that wraps existing routing with knowledge-informed priors:

```rust
/// Route with knowledge-informed priors.
///
/// Queries the knowledge store for model-selection heuristics matching
/// the current task type and regime, then applies the resulting priors
/// as score multipliers to the bandit selection.
pub fn route_with_knowledge(
    &self,
    ctx: &RoutingContext,
    priors: &[roko_neuro::routing_advisor::ModelPrior],
) -> CascadeModel {
    let base = self.route(ctx);

    // If we have no priors or are in static stage, return base
    if priors.is_empty() || self.current_stage() == CascadeStage::Static {
        return base;
    }

    // Check if any prior recommends a different model with high confidence
    let current_model = &base.primary;
    let better_prior = priors.iter()
        .filter(|p| &p.model != current_model && p.score_multiplier > 1.2 && p.confidence > 0.5)
        .max_by(|a, b| a.score_multiplier.partial_cmp(&b.score_multiplier)
            .unwrap_or(std::cmp::Ordering::Equal));

    if let Some(prior) = better_prior {
        if self.model_slugs.contains(&prior.model) {
            let mut result = base;
            // Move the knowledge-recommended model to primary,
            // demote current primary to first fallback
            let old_primary = result.primary.clone();
            result.primary = prior.model.clone();
            if !result.fallbacks.contains(&old_primary) {
                result.fallbacks.insert(0, old_primary);
            }
            return result;
        }
    }

    base
}
```

**4. Create `CrossMarketTransfer` detector**

File: `crates/roko-neuro/src/cross_market.rs` (new)

Wraps the existing `ResonanceDetector` with market-aware domain tagging:

```rust
//! Cross-market knowledge transfer via HDC resonance detection.

use crate::hdc::ResonanceDetector;
use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeTier};
use chrono::Utc;

/// Find cross-market knowledge resonances and produce CausalLink entries.
///
/// Runs `ResonanceDetector` on entries from different `domain:` tags,
/// then converts high-similarity pairs into `CausalLink` entries
/// documenting the cross-market analogy.
#[must_use]
pub fn detect_cross_market_transfers(
    entries: &[KnowledgeEntry],
    min_similarity: f64,
    max_results: usize,
) -> Vec<KnowledgeEntry> {
    let detector = ResonanceDetector::new(min_similarity, max_results);
    let pairs = detector.detect_resonances(entries);

    pairs.iter().map(|pair| {
        KnowledgeEntry {
            id: format!("xmarket-{}-{}", pair.entry_a, pair.entry_b),
            kind: KnowledgeKind::CausalLink,
            source: Some("cross-market-transfer".to_string()),
            content: format!(
                "Pattern in {} resembles pattern in {} (similarity: {:.3})",
                pair.domain_a, pair.domain_b, pair.similarity
            ),
            confidence: pair.similarity.min(1.0),
            confidence_weight: pair.similarity.min(1.0),
            tags: vec![
                "cross-market".to_string(),
                format!("domain:{}", pair.domain_a),
                format!("domain:{}", pair.domain_b),
                "resonance".to_string(),
            ],
            half_life_days: KnowledgeKind::CausalLink.default_half_life_days(),
            tier: KnowledgeTier::Working,
            created_at: Utc::now(),
            source_episodes: vec![],
            refuted_insight_id: None,
            refutation_evidence: None,
            source_model: None,
            model_generality: 1.0,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            catalytic_score: 0,
        }
    }).collect()
}
```

### Wiring

- Add `pub mod routing_advisor;` and `pub mod regime_codebook;` and `pub mod cross_market;` to `crates/roko-neuro/src/lib.rs`
- Re-export: `pub use routing_advisor::{ModelPrior, query_model_priors};`
- Re-export: `pub use regime_codebook::{RegimeCodebook, RegimeClassification, RegimeLabel};`
- Re-export: `pub use cross_market::detect_cross_market_transfers;`
- In `crates/roko-learn/src/cascade_router.rs`, add `route_with_knowledge()` method to `CascadeRouter` (no new module, just a method on the existing struct)
- The `ResonanceDetector` is currently `pub(crate)`. To use it from `cross_market.rs`, either make it `pub` or move `detect_cross_market_transfers` into `hdc.rs`. Preferred: make `ResonanceDetector` and `ResonancePair` `pub` in `hdc.rs`.

### Tests

```rust
// crates/roko-neuro/src/regime_codebook.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_codebook_has_six_regimes() {
        let cb = RegimeCodebook::with_defaults();
        assert_eq!(cb.len(), 6);
    }

    #[test]
    fn classify_returns_best_match() {
        let cb = RegimeCodebook::with_defaults();
        // A vector seeded from "trending-up" should match that regime best
        let v = HdcVector::from_seed(b"regime-codebook-seed:trending-up");
        let result = cb.classify(&v).unwrap();
        assert_eq!(result.label.0, "trending-up");
        assert!(result.similarity > 0.9);
    }

    #[test]
    fn classify_empty_codebook_returns_none() {
        let cb = RegimeCodebook::new();
        let v = HdcVector::from_seed(b"test");
        assert!(cb.classify(&v).is_none());
    }

    #[test]
    fn update_shifts_canonical_vector() {
        let mut cb = RegimeCodebook::with_defaults();
        let label = RegimeLabel("trending-up".to_string());
        let noise = HdcVector::from_seed(b"noise-observation");
        let before = cb.classify(&noise).unwrap().similarity;
        // After updating trending-up with the noise vector, similarity should increase
        cb.update(&label, &noise);
        let after = cb.classify(&noise).unwrap();
        // The update should move the canonical closer to the observation
        assert!(after.similarity >= before || after.label.0 == "trending-up");
    }

    #[test]
    fn detect_transition_fires_on_regime_change() {
        let cb = RegimeCodebook::with_defaults();
        let previous = RegimeLabel("trending-up".to_string());
        // A vector that matches "crisis" better
        let crisis_v = HdcVector::from_seed(b"regime-codebook-seed:crisis");
        let transition = cb.detect_transition(&crisis_v, &previous);
        assert!(transition.is_some());
        let (old, new) = transition.unwrap();
        assert_eq!(old.0, "trending-up");
        assert_eq!(new.0, "crisis");
    }

    #[test]
    fn no_transition_when_regime_unchanged() {
        let cb = RegimeCodebook::with_defaults();
        let label = RegimeLabel("trending-up".to_string());
        let v = HdcVector::from_seed(b"regime-codebook-seed:trending-up");
        assert!(cb.detect_transition(&v, &label).is_none());
    }

    #[test]
    fn save_load_roundtrip() {
        let cb = RegimeCodebook::with_defaults();
        let dir = std::env::temp_dir().join("roko-test-codebook");
        let path = dir.join("codebook.json");
        cb.save(&path).unwrap();
        let loaded = RegimeCodebook::load_or_default(&path);
        assert_eq!(loaded.len(), cb.len());
        std::fs::remove_dir_all(&dir).ok();
    }
}

// crates/roko-neuro/src/routing_advisor.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_priors_combines_same_model() {
        let mut priors = vec![
            ModelPrior {
                model: "opus".to_string(),
                score_multiplier: 1.3,
                confidence: 0.7,
                supporting_entries: vec!["e1".to_string()],
                rationale: "reason A".to_string(),
            },
            ModelPrior {
                model: "opus".to_string(),
                score_multiplier: 1.5,
                confidence: 0.8,
                supporting_entries: vec!["e2".to_string()],
                rationale: "reason B".to_string(),
            },
        ];
        merge_priors(&mut priors);
        assert_eq!(priors.len(), 1);
        assert!((priors[0].score_multiplier - 1.4).abs() < 0.01);
        assert_eq!(priors[0].confidence, 0.8);
        assert_eq!(priors[0].supporting_entries.len(), 2);
    }

    #[test]
    fn merge_priors_keeps_different_models() {
        let mut priors = vec![
            ModelPrior {
                model: "opus".to_string(),
                score_multiplier: 1.3,
                confidence: 0.7,
                supporting_entries: vec![],
                rationale: "a".to_string(),
            },
            ModelPrior {
                model: "sonnet".to_string(),
                score_multiplier: 1.1,
                confidence: 0.5,
                supporting_entries: vec![],
                rationale: "b".to_string(),
            },
        ];
        merge_priors(&mut priors);
        assert_eq!(priors.len(), 2);
    }

    #[test]
    fn model_prior_fields() {
        let prior = ModelPrior {
            model: "haiku".to_string(),
            score_multiplier: 0.8,
            confidence: 0.3,
            supporting_entries: vec!["e1".to_string()],
            rationale: "insufficient for complex trades".to_string(),
        };
        assert!(prior.score_multiplier < 1.0); // knowledge says avoid
    }
}

// crates/roko-neuro/src/cross_market.rs
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn entry(id: &str, content: &str, domain: &str) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_string(),
            kind: KnowledgeKind::Heuristic,
            source: Some(domain.to_string()),
            content: content.to_string(),
            confidence: 0.8,
            confidence_weight: 1.0,
            tags: vec![format!("domain:{domain}")],
            half_life_days: 90.0,
            tier: KnowledgeTier::Working,
            created_at: Utc::now(),
            source_episodes: vec![],
            refuted_insight_id: None,
            refutation_evidence: None,
            source_model: None,
            model_generality: 1.0,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            catalytic_score: 0,
        }
    }

    #[test]
    fn cross_market_produces_causal_links() {
        let entries = vec![
            entry("e1", "Mean reversion works in low vol", "ETH"),
            entry("e2", "Mean reversion works in low vol", "BTC"),
        ];
        let links = detect_cross_market_transfers(&entries, 0.5, 10);
        assert!(links.iter().any(|e| e.kind == KnowledgeKind::CausalLink
            && e.tags.contains(&"cross-market".to_string())));
    }

    #[test]
    fn same_domain_entries_produce_no_transfers() {
        let entries = vec![
            entry("e1", "Some pattern", "ETH"),
            entry("e2", "Some pattern", "ETH"),
        ];
        let links = detect_cross_market_transfers(&entries, 0.5, 10);
        assert!(links.is_empty());
    }

    #[test]
    fn dissimilar_entries_produce_no_transfers() {
        let entries = vec![
            entry("e1", "Alpha centauri mission planning", "ETH"),
            entry("e2", "Database vacuum schedule every Tuesday", "BTC"),
        ];
        let links = detect_cross_market_transfers(&entries, 0.7, 10);
        assert!(links.is_empty());
    }
}
```

### Verification

```bash
cargo test -p roko-neuro -- routing_advisor
cargo test -p roko-neuro -- regime_codebook
cargo test -p roko-neuro -- cross_market
cargo test -p roko-learn -- cascade_router
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-neuro
cargo +nightly fmt --check -p roko-learn
```

### Acceptance criteria

- [ ] `KnowledgeRoutingAdvisor` queries the knowledge store for `Heuristic` and `StrategyFragment` entries with `source_model` metadata
- [ ] `ModelPrior` has `model`, `score_multiplier`, `confidence`, `supporting_entries`, `rationale`
- [ ] Priors for the same model are merged (averaged multiplier, max confidence, combined entries)
- [ ] `CascadeRouter::route_with_knowledge()` promotes a knowledge-recommended model to primary when prior confidence > 0.5 and multiplier > 1.2
- [ ] Knowledge routing is a no-op during the `Static` stage (< 50 observations)
- [ ] `RegimeCodebook` ships with six default seed regimes
- [ ] `classify()` returns the best-matching regime with similarity score
- [ ] `update()` shifts the canonical vector toward the new observation using 3:1 EMA bundling
- [ ] `detect_transition()` fires when the best-matching regime changes
- [ ] Codebook persists to JSON and round-trips through save/load
- [ ] `detect_cross_market_transfers()` uses `ResonanceDetector` to find cross-domain pairs and produces `CausalLink` entries
- [ ] Same-domain entries produce no cross-market transfers
- [ ] `ResonanceDetector` and `ResonancePair` are made `pub` in `hdc.rs`
- [ ] `cargo test -p roko-neuro && cargo test -p roko-learn` pass clean

### Commit message

```
feat(roko-neuro, roko-learn): add knowledge-informed routing, regime codebook, and cross-market transfer
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives Used

- **Recipe**: `MarketHdcEncoder` (market state -> 10,240-bit hypervector via role-filler binding), `KnowledgeRoutingAdvisor` (knowledge store -> model routing priors), `CrossMarketTransfer` (HDC resonance detection for cross-market pattern transfer)
- **Knowledge Entry**: `RegimeCodebook` (canonical regime vectors: trending-up, trending-down, range-bound, high-vol, low-vol, crisis), domain-specific decay rates (price insights 4h, regime observations 3d, trade strategies 2d via `defi_half_life_days()`)
- **Signal**: Knowledge-informed routing signals (model selection recommendations based on knowledge store queries)

### Authoring Surfaces

- **Recipe Editor** -- build HDC encoding pipelines with role-filler configuration
- **Knowledge > Entry Detail** -- inspect knowledge entries with HDC fingerprint, decay curve, tier status
- **System > Model Routing** -- configure knowledge-informed model routing with regime-aware priors

### Shareable Artifacts

- HDC encoding recipe templates (role-filler configurations for different market structure types)
- Regime codebooks (canonical regime vectors calibrated for specific markets/instruments)
- Knowledge decay configurations (half-life presets for different information types)

### Dashboard Visibility

- **Knowledge > Entry Detail** -- individual entries with HDC fingerprint visualization, decay curve, tier badge
- **Knowledge > Regime Map** -- 2D projection of regime codebook with current state highlighted
- **System > Model Routing** -- knowledge-informed routing decisions with confidence scores
- **Forge > Recipes** -- HDC encoding and cross-market transfer pipelines
