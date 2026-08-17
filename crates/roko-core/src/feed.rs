//! Feed trait for agent-produced data streams.
//!
//! Feeds are typed data channels produced by agents. Each feed has a kind
//! (Raw, Derived, Composite, Meta), an access level, and an optional JSON
//! schema describing the payload shape. The [`FeedRegistry`] tracks all
//! registered feeds and supports queries by kind, agent, and free-text search.
//!
//! [`FeedRegistry`] remains the static descriptor catalog. Runnable instances
//! are [`FeedCell`](crate::FeedCell)s supervised by
//! [`RuntimeRegistry`](crate::RuntimeRegistry), and their output is routed to
//! canonical Bus Pulse topics by [`FeedBusBridge`](crate::FeedBusBridge).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Enums ─────────────────────────────────────────────────────────

/// Classification of a feed's data lineage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FeedKind {
    /// Unprocessed source data (e.g. price ticks, log lines).
    Raw,
    /// Computed from one or more raw feeds (e.g. moving average).
    Derived,
    /// Assembled from multiple derived feeds (e.g. portfolio risk).
    Composite,
    /// Metadata about other feeds (e.g. schema registry, lineage graph).
    Meta,
}

/// Visibility / payment gate for a feed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FeedAccess {
    /// Readable by any agent.
    Public,
    /// Restricted to the producing agent and explicit subscribers.
    Private,
    /// Requires payment or staking to access.
    Paid,
}

/// Reputation-derived commercial tier for paid feed access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingTier {
    /// Selling is disabled and no payment is required.
    Free,
    /// Entry-level access at half the base price.
    Starter,
    /// Default access at the base price.
    Standard,
    /// Priority access at one-and-a-half times the base price.
    Professional,
    /// SLA-backed access at twice the base price.
    Enterprise,
}

impl PricingTier {
    /// Multiplier applied to a feed's configured base price.
    #[must_use]
    pub const fn price_multiplier(self) -> f64 {
        match self {
            Self::Free => 0.0,
            Self::Starter => 0.5,
            Self::Standard => 1.0,
            Self::Professional => 1.5,
            Self::Enterprise => 2.0,
        }
    }
}

/// Payment protocol used to authorize paid feed access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentProtocol {
    /// One authorization per HTTP request.
    X402,
    /// One metered authorization per session.
    Mpp,
}

/// Metered pricing terms for a longer-lived feed session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionPricing {
    /// Base KORAI charge per minute.
    pub base_rate_per_minute: f64,
    /// Additional KORAI charge for burst usage.
    pub burst_rate: f64,
    /// Hard maximum KORAI charge for one session.
    pub max_session_cost: f64,
    /// Frequency at which accrued usage should be settled.
    pub settlement_interval_secs: u64,
}

/// Commercial terms advertised by a feed descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedPricingConfig {
    /// Reputation-derived pricing tier.
    pub tier: PricingTier,
    /// Base KORAI charge for one request.
    pub per_request_cost: f64,
    /// Optional metered-session terms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_pricing: Option<SessionPricing>,
    /// Authorization protocol expected from consumers.
    pub protocol: PaymentProtocol,
}

// ── Structs ───────────────────────────────────────────────────────

/// Full descriptor for a registered feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedInfo {
    /// Unique feed identifier (assigned by the registry on registration).
    pub id: String,
    /// Runtime Cell identifier. Empty for descriptor-only legacy feeds.
    #[serde(default)]
    pub cell_id: String,
    /// Human-readable feed name.
    pub name: String,
    /// Data lineage classification.
    pub kind: FeedKind,
    /// Visibility / access level.
    pub access: FeedAccess,
    /// Agent that produces this feed.
    pub agent_id: String,
    /// Short description of what the feed contains.
    #[serde(default)]
    pub description: String,
    /// Optional JSON Schema describing individual feed payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
    /// Optional commercial terms, normally present for [`FeedAccess::Paid`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing: Option<FeedPricingConfig>,
    /// When the feed was first registered.
    pub created_at: DateTime<Utc>,
}

// ── Registry ──────────────────────────────────────────────────────

/// In-memory registry of [`FeedInfo`] entries.
///
/// Runtime lifecycle deliberately lives beside this catalog in
/// [`RuntimeRegistry`](crate::RuntimeRegistry), keeping serializable metadata
/// separate from task handles and cancellation tokens.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedRegistry {
    feeds: Vec<FeedInfo>,
    /// Monotonic counter used to generate unique feed IDs.
    #[serde(default)]
    next_id: u64,
}

impl FeedRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            feeds: Vec::new(),
            next_id: 1,
        }
    }

    /// Register a new feed and return its assigned ID.
    ///
    /// The `id` field on the incoming [`FeedInfo`] is **overwritten** with a
    /// registry-assigned value to guarantee uniqueness.
    pub fn register(&mut self, mut feed: FeedInfo) -> String {
        let id = format!("feed-{}", self.next_id);
        self.next_id += 1;
        feed.id = id.clone();
        self.feeds.push(feed);
        id
    }

    /// Remove a feed by its ID. Returns `true` if it was present.
    pub fn unregister(&mut self, id: &str) -> bool {
        let before = self.feeds.len();
        self.feeds.retain(|f| f.id != id);
        self.feeds.len() != before
    }

    /// Look up a feed by its ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&FeedInfo> {
        self.feeds.iter().find(|f| f.id == id)
    }

    /// List all registered feeds.
    #[must_use]
    pub fn list(&self) -> &[FeedInfo] {
        &self.feeds
    }

    /// List feeds filtered by kind.
    #[must_use]
    pub fn list_by_kind(&self, kind: FeedKind) -> Vec<&FeedInfo> {
        self.feeds.iter().filter(|f| f.kind == kind).collect()
    }

    /// List feeds produced by a specific agent.
    #[must_use]
    pub fn list_by_agent(&self, agent_id: &str) -> Vec<&FeedInfo> {
        self.feeds
            .iter()
            .filter(|f| f.agent_id == agent_id)
            .collect()
    }

    /// Simple substring search across feed name and description.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&FeedInfo> {
        let q = query.to_lowercase();
        self.feeds
            .iter()
            .filter(|f| {
                f.name.to_lowercase().contains(&q) || f.description.to_lowercase().contains(&q)
            })
            .collect()
    }
}

// ── Runtime status ───────────────────────────────────────────────

/// Runtime status snapshot for an active feed.
///
/// Returned by the `/api/feeds/runtime/{id}` endpoint and consumed by
/// `roko feed status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedRuntimeStatus {
    /// Stable feed identifier (e.g. `"file-watch-roko-dir"`).
    pub id: String,
    /// Topic string (e.g. `"fs.changed"`, `"provider.health"`).
    pub topic: String,
    /// Feed kind label (`"Raw"`, `"Derived"`, `"Composite"`, `"Meta"`).
    pub kind: String,
    /// Whether the feed is currently connected and producing pulses.
    pub connected: bool,
    /// Approximate output rate in Hz.
    #[serde(default)]
    pub rate_hz: f64,
    /// Total number of pulses emitted since startup.
    #[serde(default)]
    pub pulses_produced: u64,
    /// Epoch-ms timestamp of the last pulse, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_update_ms: Option<u64>,
    /// Error string if the feed is in a degraded state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_feed(name: &str, kind: FeedKind, agent: &str) -> FeedInfo {
        FeedInfo {
            id: String::new(), // will be overwritten by register()
            cell_id: String::new(),
            name: name.to_string(),
            kind,
            access: FeedAccess::Public,
            agent_id: agent.to_string(),
            description: format!("Test feed: {name}"),
            schema: None,
            pricing: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn register_assigns_id() {
        let mut reg = FeedRegistry::new();
        let id = reg.register(sample_feed("prices", FeedKind::Raw, "agent-1"));
        assert_eq!(id, "feed-1");
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].id, "feed-1");
    }

    #[test]
    fn ids_are_monotonic() {
        let mut reg = FeedRegistry::new();
        let id1 = reg.register(sample_feed("a", FeedKind::Raw, "x"));
        let id2 = reg.register(sample_feed("b", FeedKind::Derived, "y"));
        assert_eq!(id1, "feed-1");
        assert_eq!(id2, "feed-2");
    }

    #[test]
    fn unregister_returns_true_when_present() {
        let mut reg = FeedRegistry::new();
        let id = reg.register(sample_feed("prices", FeedKind::Raw, "agent-1"));
        assert!(reg.unregister(&id));
        assert!(reg.list().is_empty());
    }

    #[test]
    fn unregister_returns_false_when_absent() {
        let mut reg = FeedRegistry::new();
        assert!(!reg.unregister("feed-999"));
    }

    #[test]
    fn get_returns_entry() {
        let mut reg = FeedRegistry::new();
        let id = reg.register(sample_feed("prices", FeedKind::Raw, "agent-1"));
        let entry = reg.get(&id).expect("should find feed");
        assert_eq!(entry.name, "prices");
        assert!(reg.get("feed-999").is_none());
    }

    #[test]
    fn list_by_kind_filters() {
        let mut reg = FeedRegistry::new();
        reg.register(sample_feed("raw1", FeedKind::Raw, "a"));
        reg.register(sample_feed("derived1", FeedKind::Derived, "b"));
        reg.register(sample_feed("raw2", FeedKind::Raw, "c"));

        let raws = reg.list_by_kind(FeedKind::Raw);
        assert_eq!(raws.len(), 2);
        assert!(raws.iter().all(|f| f.kind == FeedKind::Raw));

        let derived = reg.list_by_kind(FeedKind::Derived);
        assert_eq!(derived.len(), 1);
    }

    #[test]
    fn list_by_agent_filters() {
        let mut reg = FeedRegistry::new();
        reg.register(sample_feed("a", FeedKind::Raw, "agent-1"));
        reg.register(sample_feed("b", FeedKind::Raw, "agent-2"));
        reg.register(sample_feed("c", FeedKind::Derived, "agent-1"));

        let agent1_feeds = reg.list_by_agent("agent-1");
        assert_eq!(agent1_feeds.len(), 2);
    }

    #[test]
    fn search_matches_name_and_description() {
        let mut reg = FeedRegistry::new();
        reg.register(sample_feed("ETH prices", FeedKind::Raw, "a"));
        reg.register(sample_feed("BTC volume", FeedKind::Raw, "b"));

        // Matches name
        let results = reg.search("eth");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "ETH prices");

        // Matches description (all descriptions contain "Test feed:")
        let results = reg.search("test feed");
        assert_eq!(results.len(), 2);

        // No match
        let results = reg.search("zzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let mut reg = FeedRegistry::new();
        reg.register(sample_feed("x", FeedKind::Composite, "a"));
        let json = serde_json::to_string(&reg).expect("serialize");
        let restored: FeedRegistry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.list().len(), 1);
        assert_eq!(restored.list()[0].name, "x");
        assert_eq!(restored.next_id, 2);
    }

    #[test]
    fn paid_feed_pricing_survives_serde_roundtrip() {
        let mut feed = sample_feed("premium-prices", FeedKind::Derived, "agent-1");
        feed.access = FeedAccess::Paid;
        feed.pricing = Some(FeedPricingConfig {
            tier: PricingTier::Professional,
            per_request_cost: 1.25,
            session_pricing: Some(SessionPricing {
                base_rate_per_minute: 0.5,
                burst_rate: 0.1,
                max_session_cost: 20.0,
                settlement_interval_secs: 600,
            }),
            protocol: PaymentProtocol::Mpp,
        });

        let json = serde_json::to_string(&feed).expect("serialize priced feed");
        let restored: FeedInfo = serde_json::from_str(&json).expect("deserialize priced feed");
        assert_eq!(restored.pricing, feed.pricing);
    }

    #[test]
    fn legacy_feed_json_without_pricing_still_deserializes() {
        let json = serde_json::json!({
            "id": "feed-legacy",
            "name": "legacy",
            "kind": "raw",
            "access": "public",
            "agent_id": "agent-legacy",
            "description": "old descriptor",
            "created_at": "2026-01-01T00:00:00Z"
        });

        let restored: FeedInfo = serde_json::from_value(json).expect("deserialize legacy feed");
        assert!(restored.pricing.is_none());
    }

    #[test]
    fn pricing_tiers_expose_canonical_multipliers() {
        assert_eq!(PricingTier::Free.price_multiplier(), 0.0);
        assert_eq!(PricingTier::Starter.price_multiplier(), 0.5);
        assert_eq!(PricingTier::Standard.price_multiplier(), 1.0);
        assert_eq!(PricingTier::Professional.price_multiplier(), 1.5);
        assert_eq!(PricingTier::Enterprise.price_multiplier(), 2.0);
    }
}
