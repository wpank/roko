# 03: ISFR Keeper Agent — ISFRSource Trait + Relay Integration

The ISFR keeper agent that fetches rates from DeFi protocols, publishes observations to the relay, coordinates block-range voting, and submits on-chain. Designed for extensibility — adding a new rate source is one struct.

## Architecture

```
ISFRSource implementations (pluggable)
  │  AaveV3Source, CompoundV3Source, EthenaSUSDe, BeaconStaking
  │  Each: async fetch_rate() → Option<SourceReading>
  │
  ▼
ISFRKeeper (orchestrator)
  │  Polls sources on interval
  │  Computes composite via weighted median
  │  Publishes to relay feed:isfr:rates
  │  Coordinates block-range voting via feed:isfr:ranges
  │
  ▼
Relay (WebSocket)
  │  Routes to all subscribers
  │  Chain watcher publishes on-chain events back
  │
  ▼
On-chain (ISFROracle on target chain)
  │  submitRate() for fast path
  │  submitRateForRange() for block-range path
  │
  ▼
Consumers
  │  Other agents, dashboards, yield perpetuals
```

## ISFRSource Trait

**File:** `crates/roko-chain/src/isfr_sources.rs` (new module)

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A single rate reading from a DeFi protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReading {
    /// Source name (e.g., "aave-v3-usdc").
    pub source: String,
    /// Annualized rate in basis points.
    pub rate_bps: u32,
    /// Unix timestamp (seconds) when this reading was taken.
    pub timestamp: u64,
    /// Whether the source was responsive.
    pub is_live: bool,
    /// Source-specific metadata (e.g., utilization rate, reserve size).
    pub metadata: Option<serde_json::Value>,
}

/// Health status of a rate source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceStatus {
    /// Source is responding normally.
    Live,
    /// Source responded but data is older than liveness timeout.
    Stale,
    /// Source is not responding.
    Offline,
}

/// Metadata about a configured rate source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMeta {
    pub name: String,
    pub weight: f64,
    pub liveness_timeout: Duration,
    pub last_reading: Option<SourceReading>,
    pub status: SourceStatus,
    pub consecutive_failures: u32,
}

/// A rate source contributing to the ISFR composite.
///
/// Implement this trait to add a new DeFi protocol to the ISFR.
/// Each source fetches its protocol's current rate and reports
/// its weight in the weighted median aggregation.
///
/// # Extension
///
/// Adding a new source is one struct + one impl:
/// ```rust
/// struct MyProtocolSource { /* config */ }
///
/// #[async_trait]
/// impl ISFRSource for MyProtocolSource {
///     fn name(&self) -> &str { "my-protocol" }
///     async fn fetch_rate(&self) -> Option<SourceReading> { /* ... */ }
///     fn weight(&self) -> f64 { 0.15 }
///     fn rate_class(&self) -> RateClass { RateClass::Lending }
///     fn liveness_timeout(&self) -> Duration { Duration::from_secs(300) }
/// }
/// ```
#[async_trait]
pub trait ISFRSource: Send + Sync {
    /// Human-readable name (e.g., "aave-v3-usdc").
    fn name(&self) -> &str;

    /// Fetch the current rate. Returns None if unreachable or stale.
    async fn fetch_rate(&self) -> Option<SourceReading>;

    /// Weight in the weighted median (0.0 to 1.0).
    fn weight(&self) -> f64;

    /// Which rate class this source belongs to.
    fn rate_class(&self) -> RateClass;

    /// How long before a reading is considered stale.
    fn liveness_timeout(&self) -> Duration;
}

/// ISFR rate classes — the four components of the composite.
///
/// From the spec: ISFR = weighted median of lending, structured,
/// funding, and staking rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RateClass {
    /// Protocol lending rates (Aave, Compound).
    Lending,
    /// Structured yield (Ethena sUSDe, etc.).
    Structured,
    /// Perpetual funding rates.
    Funding,
    /// Beacon chain staking yield.
    Staking,
}

impl RateClass {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Lending => "lending",
            Self::Structured => "structured",
            Self::Funding => "funding",
            Self::Staking => "staking",
        }
    }
}
```

## Concrete Source Implementations

**File:** `crates/roko-chain/src/isfr_sources/` (module with one file per source)

### Mock Source (for local dev / any chain without live DeFi)

```rust
/// Mock source that returns configurable rates.
/// Used for local development (mirage-rs, devnets) without real DeFi protocol access.
pub struct MockSource {
    name: String,
    rate_bps: u32,
    weight: f64,
    class: RateClass,
    jitter_bps: u32, // random +/- jitter for realistic simulation
}

impl MockSource {
    pub fn aave_mock() -> Self {
        Self {
            name: "mock-aave-v3".into(),
            rate_bps: 620,  // 6.20% APY
            weight: 0.30,
            class: RateClass::Lending,
            jitter_bps: 15,
        }
    }

    pub fn compound_mock() -> Self {
        Self {
            name: "mock-compound-v3".into(),
            rate_bps: 580,
            weight: 0.25,
            class: RateClass::Lending,
            jitter_bps: 20,
        }
    }

    pub fn ethena_mock() -> Self {
        Self {
            name: "mock-ethena-susde".into(),
            rate_bps: 710,
            weight: 0.25,
            class: RateClass::Structured,
            jitter_bps: 30,
        }
    }

    pub fn staking_mock() -> Self {
        Self {
            name: "mock-beacon-staking".into(),
            rate_bps: 320,
            weight: 0.20,
            class: RateClass::Staking,
            jitter_bps: 5,
        }
    }
}

#[async_trait]
impl ISFRSource for MockSource {
    fn name(&self) -> &str { &self.name }

    async fn fetch_rate(&self) -> Option<SourceReading> {
        let jitter = (rand::random::<u32>() % (self.jitter_bps * 2))
            .saturating_sub(self.jitter_bps);
        Some(SourceReading {
            source: self.name.clone(),
            rate_bps: self.rate_bps + jitter,
            timestamp: now_secs(),
            is_live: true,
            metadata: None,
        })
    }

    fn weight(&self) -> f64 { self.weight }
    fn rate_class(&self) -> RateClass { self.class }
    fn liveness_timeout(&self) -> Duration { Duration::from_secs(300) }
}
```

### Real Source Template (Aave V3)

```rust
/// Aave V3 rate source — reads from the Pool contract.
pub struct AaveV3Source {
    pool_address: Address,
    asset_address: Address,  // USDC
    provider: Arc<dyn Provider>,
    weight: f64,
}

#[async_trait]
impl ISFRSource for AaveV3Source {
    fn name(&self) -> &str { "aave-v3-usdc" }

    async fn fetch_rate(&self) -> Option<SourceReading> {
        // Call pool.getReserveData(asset)
        // Extract currentLiquidityRate (ray-encoded, 27 decimals)
        // Convert to annualized basis points
        let reserve_data = self.provider
            .call(/* getReserveData ABI */)
            .await.ok()?;
        let liquidity_rate_ray = extract_liquidity_rate(&reserve_data)?;
        let rate_bps = ray_to_bps(liquidity_rate_ray);
        Some(SourceReading {
            source: "aave-v3-usdc".into(),
            rate_bps,
            timestamp: now_secs(),
            is_live: true,
            metadata: Some(serde_json::json!({
                "utilization_rate": extract_utilization(&reserve_data),
            })),
        })
    }

    fn weight(&self) -> f64 { self.weight }
    fn rate_class(&self) -> RateClass { RateClass::Lending }
    fn liveness_timeout(&self) -> Duration { Duration::from_secs(300) }
}
```

**Design note:** Real sources are used when the target chain has live DeFi protocol access (mainnet fork, mainnet). For local dev chains (mirage-rs, daeji devnet), MockSource provides realistic rate data. The trait boundary means switching from mock to real is a config change, not a code change — same keeper binary, different `[[isfr.sources]]` entries in `roko.toml`.

## ISFRKeeper Orchestrator

**File:** `crates/roko-chain/src/isfr_keeper.rs` (new)

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

/// The ISFR keeper agent — polls sources, computes composite, publishes.
pub struct ISFRKeeper {
    /// Registered rate sources.
    sources: Vec<Box<dyn ISFRSource>>,
    /// Relay client for publishing.
    relay: Arc<RelayClient>,
    /// Current aggregated rates.
    current_rates: RwLock<Option<CompositeRate>>,
    /// Keeper identity for relay and on-chain.
    keeper_id: String,
    /// Poll interval.
    poll_interval: Duration,
    /// ISFR config (epoch duration, outlier sigma, etc.).
    config: IsfrConfig,
    /// Chain identifier — determines relay chain topic (e.g., "mirage", "daeji", "mainnet").
    chain_id: String,
}

/// Aggregated composite rate from all sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeRate {
    /// Composite rate in basis points (weighted median of class rates).
    pub composite_bps: u32,
    /// Per-class rates.
    pub lending_bps: u32,
    pub structured_bps: u32,
    pub funding_bps: u32,
    pub staking_bps: u32,
    /// Confidence score (0-10000 bps, based on source liveness).
    pub confidence_bps: u32,
    /// Unix timestamp.
    pub timestamp: u64,
    /// Individual source readings.
    pub sources: Vec<SourceReading>,
}

impl ISFRKeeper {
    pub fn new(
        sources: Vec<Box<dyn ISFRSource>>,
        relay: Arc<RelayClient>,
        keeper_id: impl Into<String>,
        config: IsfrConfig,
        chain_id: impl Into<String>,
    ) -> Self {
        Self {
            sources,
            relay,
            current_rates: RwLock::new(None),
            keeper_id: keeper_id.into(),
            poll_interval: Duration::from_secs(10),
            config,
            chain_id: chain_id.into(),
        }
    }

    /// Default keeper with mock sources for local dev (mirage, devnets).
    pub fn mock_keeper(relay: Arc<RelayClient>, chain_id: &str) -> Self {
        Self::new(
            vec![
                Box::new(MockSource::aave_mock()),
                Box::new(MockSource::compound_mock()),
                Box::new(MockSource::ethena_mock()),
                Box::new(MockSource::staking_mock()),
            ],
            relay,
            "isfr-keeper-mock",
            IsfrConfig::default(),
            chain_id,
        )
    }

    /// Start the keeper loop.
    pub async fn run(&self) -> anyhow::Result<()> {
        // Subscribe to coordination and chain event topics
        let chain_topic = format!("chain:{}", self.chain_id);
        self.relay.subscribe(&[
            "feed:isfr:rates",
            "feed:isfr:ranges",
            &chain_topic,
        ]).await?;

        loop {
            // 1. Poll all sources
            let readings = self.poll_sources().await;

            // 2. Compute composite rate
            let composite = self.compute_composite(&readings);

            // 3. Publish to relay
            self.relay.publish(
                "feed:isfr:rates",
                Some("rate_observation"),
                serde_json::to_value(&composite)?,
            ).await?;

            // 4. Store current rates
            *self.current_rates.write().await = Some(composite);

            // 5. Handle incoming messages (range proposals, chain events)
            self.process_incoming().await;

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Poll all sources concurrently.
    async fn poll_sources(&self) -> Vec<SourceReading> {
        let futures: Vec<_> = self.sources
            .iter()
            .map(|s| s.fetch_rate())
            .collect();
        let results = futures::future::join_all(futures).await;
        results.into_iter().flatten().collect()
    }

    /// Compute composite rate from source readings.
    ///
    /// Uses the same weighted median + outlier exclusion from roko-chain/src/isfr.rs.
    fn compute_composite(&self, readings: &[SourceReading]) -> CompositeRate {
        // Group by class
        let mut by_class: HashMap<RateClass, Vec<(f64, u32)>> = HashMap::new();
        for r in readings {
            let source = self.sources.iter().find(|s| s.name() == r.source);
            if let Some(s) = source {
                by_class.entry(s.rate_class())
                    .or_default()
                    .push((s.weight(), r.rate_bps));
            }
        }

        // Compute per-class weighted medians
        let lending = weighted_median(by_class.get(&RateClass::Lending));
        let structured = weighted_median(by_class.get(&RateClass::Structured));
        let funding = weighted_median(by_class.get(&RateClass::Funding));
        let staking = weighted_median(by_class.get(&RateClass::Staking));

        // Composite = weighted median of class rates
        let class_rates = vec![
            (0.30, lending),
            (0.25, structured),
            (0.25, funding),
            (0.20, staking),
        ];
        let composite = weighted_median(Some(&class_rates));

        // Confidence based on source coverage
        let live_count = readings.iter().filter(|r| r.is_live).count();
        let confidence = (live_count as f64 / self.sources.len() as f64 * 10_000.0) as u32;

        CompositeRate {
            composite_bps: composite,
            lending_bps: lending,
            structured_bps: structured,
            funding_bps: funding,
            staking_bps: staking,
            confidence_bps: confidence,
            timestamp: now_secs(),
            sources: readings.to_vec(),
        }
    }

    /// Handle incoming relay messages — range proposals and chain events.
    async fn process_incoming(&self) {
        // Non-blocking check for incoming envelopes
        // Range proposals: if we see a range_propose, vote with our computed rate
        // Chain events: if we see a RangeClosed event, log completion
    }
}

/// Weighted median computation.
///
/// Reuses the logic from roko-chain/src/isfr.rs `weighted_median()`.
fn weighted_median(data: Option<&Vec<(f64, u32)>>) -> u32 {
    let data = match data {
        Some(d) if !d.is_empty() => d,
        _ => return 0,
    };

    let mut sorted: Vec<_> = data.clone();
    sorted.sort_by_key(|(_, rate)| *rate);

    let total_weight: f64 = sorted.iter().map(|(w, _)| w).sum();
    let mut cumulative = 0.0;
    for (weight, rate) in &sorted {
        cumulative += weight / total_weight;
        if cumulative >= 0.5 {
            return *rate;
        }
    }
    sorted.last().map(|(_, r)| *r).unwrap_or(0)
}
```

## Block-Range Coordination

The keeper also participates in block-range coordination:

```rust
impl ISFRKeeper {
    /// Propose a new block range for coordinated rate submission.
    async fn propose_range(&self, start: u64, end: u64) -> anyhow::Result<()> {
        self.relay.publish(
            "feed:isfr:ranges",
            Some("range_propose"),
            serde_json::json!({
                "type": "range_propose",
                "start": start,
                "end": end,
                "proposed_by": self.keeper_id,
                "proposed_at": now_secs(),
            }),
        ).await
    }

    /// Vote on an existing range proposal.
    async fn vote_range(&self, start: u64, end: u64) -> anyhow::Result<()> {
        let rates = self.current_rates.read().await;
        let rates = rates.as_ref().ok_or_else(|| anyhow::anyhow!("no current rates"))?;

        self.relay.publish(
            "feed:isfr:ranges",
            Some("range_vote"),
            serde_json::json!({
                "type": "range_vote",
                "start": start,
                "end": end,
                "composite_bps": rates.composite_bps,
                "components": [
                    rates.lending_bps,
                    rates.structured_bps,
                    rates.funding_bps,
                    rates.staking_bps,
                ],
                "confidence_bps": rates.confidence_bps,
                "voter": self.keeper_id,
            }),
        ).await
    }

    /// Submit aggregated rate to ISFROracle contract.
    async fn submit_onchain(&self, rates: &CompositeRate) -> anyhow::Result<()> {
        // Use chain tools (isfr.submit_rate) to submit
        // Or directly via alloy contract call
        todo!("wire to chain tools from 04-chain-tools.md")
    }
}
```

## Module Structure

```
crates/roko-chain/src/
├── isfr.rs                    # Existing: IsfrConfig, ClearingPhase, weighted median
├── isfr_sources.rs            # NEW: ISFRSource trait, RateClass, SourceReading
├── isfr_sources/
│   ├── mod.rs                 # Module registration
│   ├── mock.rs                # MockSource for local dev (mirage, devnets)
│   ├── aave_v3.rs             # AaveV3Source (placeholder until mainnet)
│   ├── compound_v3.rs         # CompoundV3Source (placeholder)
│   ├── ethena.rs              # EthenaSUSDe (placeholder)
│   └── staking.rs             # BeaconStaking (placeholder)
├── isfr_keeper.rs             # NEW: ISFRKeeper orchestrator
└── lib.rs                     # Add module exports
```

## Registration as Roko Agent

The ISFR keeper runs as a roko agent with a specific card:

```rust
// In roko-agent-server or a standalone binary:
let card = AgentCard {
    name: "isfr-keeper".into(),
    capabilities: vec![
        "isfr.rate_observation".into(),
        "isfr.range_coordination".into(),
        "isfr.chain_submission".into(),
    ],
    domain_tags: vec!["defi".into(), "oracle".into(), "isfr".into()],
    version: "1.0.0".into(),
    endpoints: AgentCardEndpoints {
        rest: Some("http://localhost:6677".into()),
        ..Default::default()
    },
};
```

The keeper registers with the relay via the standard Hello handshake, then enters its rate polling loop. Other agents discover it via the relay agent directory.

## Configuration

```toml
# roko.toml — ISFR keeper configuration
[isfr]
epoch_duration_secs = 28800
poll_interval_secs = 10
min_submissions = 2
outlier_sigma = 3.0

[isfr.relay]
url = "ws://localhost:9011/relay/agents/ws"
topics = ["feed:isfr:rates", "feed:isfr:ranges"]
# chain topic is auto-derived from [chain].chain_id → "chain:{chain_id}"

# Source weights (must sum to 1.0)
[[isfr.sources]]
name = "mock-aave-v3"
kind = "mock"
weight = 0.30
class = "lending"
rate_bps = 620
jitter_bps = 15

[[isfr.sources]]
name = "mock-compound-v3"
kind = "mock"
weight = 0.25
class = "lending"
rate_bps = 580
jitter_bps = 20

[[isfr.sources]]
name = "mock-ethena-susde"
kind = "mock"
weight = 0.25
class = "structured"
rate_bps = 710
jitter_bps = 30

[[isfr.sources]]
name = "mock-beacon-staking"
kind = "mock"
weight = 0.20
class = "staking"
rate_bps = 320
jitter_bps = 5
```

## Testing

```bash
# Unit tests for ISFRSource trait and mock sources
cargo test -p roko-chain -- isfr_sources

# Unit tests for weighted median computation
cargo test -p roko-chain -- weighted_median

# Unit tests for ISFRKeeper with mock sources (no relay needed)
cargo test -p roko-chain -- isfr_keeper

# Integration test: keeper publishes to relay, subscriber receives
cargo test -p roko-chain -- isfr_keeper_relay --ignored
```
