//! Connector contracts and legacy registry for external system I/O.
//!
//! [`Connect`] is the transport-independent five-method lifecycle protocol.
//! [`ConnectorRegistry`] remains available for the descriptor-based HTTP API
//! while callers migrate to concrete `Connect` implementations.

use std::cmp::Ordering;
use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

// ── Enums ─────────────────────────────────────────────────────────

/// The kind of external system a connector integrates with.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ConnectorKind {
    /// Model Context Protocol server.
    Mcp,
    /// Generic REST / gRPC API.
    Api,
    /// Relational or document database.
    Database,
    /// On-chain RPC endpoint.
    Blockchain,
    /// Streaming data feed.
    Feed,
    /// User-defined connector.
    Custom,
    /// Blockchain RPC endpoint (the v2 spelling of [`Blockchain`](Self::Blockchain)).
    #[serde(alias = "chain-rpc", alias = "blockchain_rpc")]
    ChainRpc,
    /// Centralized exchange API.
    Exchange,
    /// MCP tool server (the v2 spelling of [`Mcp`](Self::Mcp)).
    #[serde(alias = "mcp-server")]
    McpServer,
    /// Agent-to-Agent agent-card peer.
    #[serde(alias = "a2a", alias = "a2a-client")]
    A2aClient,
    /// Outbound HTTP webhook endpoint.
    Webhook,
}

/// Liveness status reported by a connector health check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorStatus {
    /// The connector is reachable and responding normally.
    Connected,
    /// The connector is not reachable.
    Disconnected,
    /// The connector is reachable but responding slowly or with errors.
    Degraded,
}

/// Reconnection policy declared by a connector manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "strategy")]
pub enum ReconnectStrategy {
    /// Retry with bounded exponential delay and optional jitter.
    ExponentialBackoff {
        base_ms: u64,
        max_ms: u64,
        jitter: bool,
    },
    /// Retry at a fixed interval.
    FixedInterval { interval_ms: u64 },
    /// Reconnect only after an explicit caller action.
    Manual,
}

impl ReconnectStrategy {
    /// Validate delay bounds before a runtime consumes this policy.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::ExponentialBackoff {
                base_ms, max_ms, ..
            } if *base_ms == 0 || *max_ms < *base_ms => Err(crate::RokoError::invalid(
                "exponential reconnect requires base_ms > 0 and max_ms >= base_ms",
            )),
            Self::FixedInterval { interval_ms } if *interval_ms == 0 => Err(
                crate::RokoError::invalid("fixed reconnect interval_ms must be greater than zero"),
            ),
            _ => Ok(()),
        }
    }
}

/// Confidence assigned to an event observed on a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalityLevel {
    /// Canonical with sufficient confirmation depth for irreversible actions.
    Final,
    /// Moderately confirmed and suitable for most reversible operations.
    QuasiFinalized,
    /// Recent, pending, or otherwise susceptible to reorganization.
    Reversible,
}

impl FinalityLevel {
    const fn confidence_rank(self) -> u8 {
        match self {
            Self::Reversible => 0,
            Self::QuasiFinalized => 1,
            Self::Final => 2,
        }
    }
}

impl Ord for FinalityLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        self.confidence_rank().cmp(&other.confidence_rank())
    }
}

impl PartialOrd for FinalityLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Origin contributing facts to a merged agent discovery view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentDiscoverySource {
    Relay,
    A2A,
    Chain,
    Deployment,
}

/// Hosting environment for a directly reachable agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployPlatform {
    Railway,
    Fly,
    Docker,
    Custom,
}

/// Reputation/stake tier supplied by the identity registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTier {
    Gray,
    Copper,
    Silver,
    Gold,
    Amber,
}

// ── Structs ───────────────────────────────────────────────────────

/// Static configuration for a connector.
#[derive(Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// Human-readable connector name (unique within a registry).
    pub name: String,
    /// What kind of system this connects to.
    pub kind: ConnectorKind,
    /// Target endpoint URL or address.
    pub endpoint: String,
    /// Optional authentication token / credential.
    #[serde(default, skip_serializing)]
    pub auth: Option<String>,
    /// Optional HTTP headers to attach to every request.
    #[serde(default, skip_serializing)]
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Request timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

impl std::fmt::Debug for ConnectorConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConnectorConfig")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("endpoint", &"[CONFIGURED]")
            .field("auth", &self.auth.as_ref().map(|_| "[REDACTED]"))
            .field(
                "headers",
                &self.headers.as_ref().map(|headers| headers.len()),
            )
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

fn default_timeout_ms() -> u64 {
    5000
}

/// Configuration passed to [`Connect::connect`].
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectConfig {
    /// Target endpoint URL or transport address.
    pub endpoint: String,
    /// Optional authentication token or credential reference.
    #[serde(default, skip_serializing)]
    pub auth: Option<String>,
    /// Optional transport headers.
    #[serde(default, skip_serializing)]
    pub headers: Option<HashMap<String, String>>,
    /// Operation timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

impl std::fmt::Debug for ConnectConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConnectConfig")
            .field("endpoint", &"[CONFIGURED]")
            .field("auth", &self.auth.as_ref().map(|_| "[REDACTED]"))
            .field(
                "headers",
                &self.headers.as_ref().map(|headers| headers.len()),
            )
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

impl ConnectConfig {
    /// Validate invariants shared by every connector transport.
    pub fn validate(&self) -> Result<()> {
        if self.endpoint.trim().is_empty() {
            return Err(crate::RokoError::invalid(
                "connector endpoint must not be empty",
            ));
        }
        if self.timeout_ms == 0 {
            return Err(crate::RokoError::invalid(
                "connector timeout_ms must be greater than zero",
            ));
        }
        Ok(())
    }
}

/// Idempotent read request sent through a connector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequest {
    pub operation: String,
    #[serde(default)]
    pub params: Value,
}

impl QueryRequest {
    pub fn validate(&self) -> Result<()> {
        if self.operation.trim().is_empty() {
            return Err(crate::RokoError::invalid(
                "connector query operation must not be empty",
            ));
        }
        Ok(())
    }
}

/// Successful connector query response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResponse {
    pub data: Value,
    pub latency_ms: u64,
}

/// Potentially mutating request sent through a connector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub operation: String,
    #[serde(default)]
    pub params: Value,
}

impl ExecuteRequest {
    pub fn validate(&self) -> Result<()> {
        if self.operation.trim().is_empty() {
            return Err(crate::RokoError::invalid(
                "connector execute operation must not be empty",
            ));
        }
        Ok(())
    }
}

/// Successful connector execution response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteResponse {
    pub result: Value,
    pub latency_ms: u64,
}

/// Live health result returned by the `Connect` protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectHealthStatus {
    pub status: ConnectorStatus,
    pub latency_ms: u64,
    pub last_check: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The external-system lifecycle protocol.
///
/// It is intentionally independent of [`crate::cell::Cell`]: this contract
/// can be implemented by transports first and composed into Cells separately.
#[async_trait]
pub trait Connect: Send + Sync {
    /// Establish the external connection. Failure leaves it unavailable.
    async fn connect(&mut self, config: &ConnectConfig) -> Result<()>;

    /// Perform an idempotent read operation.
    async fn query(&self, req: QueryRequest) -> Result<QueryResponse>;

    /// Perform a potentially mutating operation.
    async fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse>;

    /// Return the current transport health.
    async fn health(&self) -> Result<ConnectHealthStatus>;

    /// Gracefully release connection resources.
    async fn disconnect(&mut self) -> Result<()>;
}

/// Static identity, configuration schema, and lifecycle policy of a connector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorManifest {
    pub name: String,
    pub kind: ConnectorKind,
    pub version: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<Value>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub health_interval_secs: u64,
    pub reconnect_strategy: ReconnectStrategy,
}

impl ConnectorManifest {
    /// Validate identity and lifecycle intervals before registration.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() || self.version.trim().is_empty() {
            return Err(crate::RokoError::invalid(
                "connector manifest name and version must not be empty",
            ));
        }
        if self.health_interval_secs == 0 {
            return Err(crate::RokoError::invalid(
                "connector health_interval_secs must be greater than zero",
            ));
        }
        self.reconnect_strategy.validate()
    }
}

/// Finality metadata attached to a chain event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalityTag {
    pub level: FinalityLevel,
    pub chain_id: u64,
    pub block_number: u64,
    pub confirmations: u64,
    pub timestamp: DateTime<Utc>,
}

/// Canonical description of a detected chain reorganization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainReorgPulse {
    pub chain_id: u64,
    pub old_head: String,
    pub new_head: String,
    pub orphaned_range_start: u64,
    pub orphaned_range_end: u64,
    pub new_range_start: u64,
    pub new_range_end: u64,
    pub depth: u64,
}

impl ChainReorgPulse {
    /// Validate inclusive range ordering and a non-zero detected depth.
    pub fn validate(&self) -> Result<()> {
        if self.orphaned_range_start > self.orphaned_range_end
            || self.new_range_start > self.new_range_end
        {
            return Err(crate::RokoError::invalid(
                "chain reorg ranges must have start <= end",
            ));
        }
        if self.depth == 0 {
            return Err(crate::RokoError::invalid(
                "chain reorg depth must be greater than zero",
            ));
        }
        Ok(())
    }
}

/// Facts merged from relay, A2A, identity-chain, and deployment discovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergedAgent {
    pub id: String,
    pub name: String,
    pub online: bool,
    pub last_seen: u64,
    pub mode: Option<String>,
    pub profile: Option<String>,
    pub a2a_capabilities: Option<Vec<String>>,
    pub hdc_fingerprint: Option<String>,
    pub supported_protocols: Option<Vec<String>>,
    pub wallet: Option<String>,
    pub reputation: Option<f64>,
    pub stake: Option<u128>,
    pub tier: Option<AgentTier>,
    pub direct_url: Option<String>,
    pub deploy_platform: Option<DeployPlatform>,
}

/// Live health snapshot for a connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorHealth {
    /// Current liveness status.
    pub status: ConnectorStatus,
    /// Round-trip latency of the most recent health check (milliseconds).
    pub latency_ms: u64,
    /// When the last health check was performed.
    pub last_check: DateTime<Utc>,
}

/// Full descriptor for a registered connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    /// Unique connector name.
    pub name: String,
    /// Connector kind.
    pub kind: ConnectorKind,
    /// Latest health snapshot.
    pub health: ConnectorHealth,
    /// When the connector was first registered.
    pub created_at: DateTime<Utc>,
    /// Arbitrary metadata attached by the registrant.
    #[serde(default)]
    pub metadata: Value,
}

// ── Registry ──────────────────────────────────────────────────────

/// In-memory registry of [`ConnectorInfo`] entries.
///
/// Descriptor registry retained for the existing HTTP discovery API.
/// New transport implementations should implement [`Connect`] independently.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectorRegistry {
    connectors: Vec<ConnectorInfo>,
}

impl ConnectorRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            connectors: Vec::new(),
        }
    }

    /// Register a new connector (or replace an existing one with the same name).
    pub fn register(&mut self, info: ConnectorInfo) {
        // Replace if a connector with this name already exists.
        if let Some(existing) = self.connectors.iter_mut().find(|c| c.name == info.name) {
            *existing = info;
        } else {
            self.connectors.push(info);
        }
    }

    /// Remove a connector by name. Returns `true` if it was present.
    pub fn unregister(&mut self, name: &str) -> bool {
        let before = self.connectors.len();
        self.connectors.retain(|c| c.name != name);
        self.connectors.len() != before
    }

    /// Look up a connector by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ConnectorInfo> {
        self.connectors.iter().find(|c| c.name == name)
    }

    /// List all registered connectors.
    #[must_use]
    pub fn list(&self) -> &[ConnectorInfo] {
        &self.connectors
    }

    /// Count connectors whose health status is [`ConnectorStatus::Connected`].
    #[must_use]
    pub fn healthy_count(&self) -> usize {
        self.connectors
            .iter()
            .filter(|c| c.health.status == ConnectorStatus::Connected)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockConnect {
        connected: bool,
    }

    #[async_trait]
    impl Connect for MockConnect {
        async fn connect(&mut self, config: &ConnectConfig) -> Result<()> {
            config.validate()?;
            self.connected = true;
            Ok(())
        }

        async fn query(&self, req: QueryRequest) -> Result<QueryResponse> {
            req.validate()?;
            Ok(QueryResponse {
                data: req.params,
                latency_ms: 1,
            })
        }

        async fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse> {
            req.validate()?;
            Ok(ExecuteResponse {
                result: req.params,
                latency_ms: 2,
            })
        }

        async fn health(&self) -> Result<ConnectHealthStatus> {
            Ok(ConnectHealthStatus {
                status: if self.connected {
                    ConnectorStatus::Connected
                } else {
                    ConnectorStatus::Disconnected
                },
                latency_ms: 0,
                last_check: Utc::now(),
                error: None,
            })
        }

        async fn disconnect(&mut self) -> Result<()> {
            self.connected = false;
            Ok(())
        }
    }

    fn sample_info(name: &str, status: ConnectorStatus) -> ConnectorInfo {
        ConnectorInfo {
            name: name.to_string(),
            kind: ConnectorKind::Api,
            health: ConnectorHealth {
                status,
                latency_ms: 42,
                last_check: Utc::now(),
            },
            created_at: Utc::now(),
            metadata: Value::Null,
        }
    }

    #[test]
    fn register_and_list() {
        let mut reg = ConnectorRegistry::new();
        assert!(reg.list().is_empty());

        reg.register(sample_info("alpha", ConnectorStatus::Connected));
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].name, "alpha");
    }

    #[test]
    fn register_replaces_existing() {
        let mut reg = ConnectorRegistry::new();
        reg.register(sample_info("alpha", ConnectorStatus::Disconnected));
        reg.register(sample_info("alpha", ConnectorStatus::Connected));
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].health.status, ConnectorStatus::Connected);
    }

    #[test]
    fn unregister_returns_true_when_present() {
        let mut reg = ConnectorRegistry::new();
        reg.register(sample_info("alpha", ConnectorStatus::Connected));
        assert!(reg.unregister("alpha"));
        assert!(reg.list().is_empty());
    }

    #[test]
    fn unregister_returns_false_when_absent() {
        let mut reg = ConnectorRegistry::new();
        assert!(!reg.unregister("ghost"));
    }

    #[test]
    fn get_returns_entry() {
        let mut reg = ConnectorRegistry::new();
        reg.register(sample_info("beta", ConnectorStatus::Degraded));
        let entry = reg.get("beta").expect("should find beta");
        assert_eq!(entry.health.status, ConnectorStatus::Degraded);
        assert!(reg.get("nope").is_none());
    }

    #[test]
    fn healthy_count_filters_connected() {
        let mut reg = ConnectorRegistry::new();
        reg.register(sample_info("a", ConnectorStatus::Connected));
        reg.register(sample_info("b", ConnectorStatus::Disconnected));
        reg.register(sample_info("c", ConnectorStatus::Connected));
        reg.register(sample_info("d", ConnectorStatus::Degraded));
        assert_eq!(reg.healthy_count(), 2);
    }

    #[test]
    fn serde_roundtrip() {
        let mut reg = ConnectorRegistry::new();
        reg.register(sample_info("x", ConnectorStatus::Connected));
        let json = serde_json::to_string(&reg).expect("serialize");
        let restored: ConnectorRegistry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.list().len(), 1);
        assert_eq!(restored.list()[0].name, "x");
    }

    #[tokio::test]
    async fn async_connect_contract_exercises_all_five_methods() {
        let mut connector = MockConnect { connected: false };
        assert_eq!(
            connector.health().await.expect("initial health").status,
            ConnectorStatus::Disconnected
        );
        connector
            .connect(&ConnectConfig {
                endpoint: "https://example.test".to_owned(),
                auth: None,
                headers: None,
                timeout_ms: 100,
            })
            .await
            .expect("connect");
        assert_eq!(
            connector.health().await.expect("connected health").status,
            ConnectorStatus::Connected
        );
        assert_eq!(
            connector
                .query(QueryRequest {
                    operation: "get".to_owned(),
                    params: json!({"id": 1}),
                })
                .await
                .expect("query")
                .data,
            json!({"id": 1})
        );
        assert_eq!(
            connector
                .execute(ExecuteRequest {
                    operation: "put".to_owned(),
                    params: json!({"id": 1}),
                })
                .await
                .expect("execute")
                .latency_ms,
            2
        );
        connector.disconnect().await.expect("disconnect");
        assert_eq!(
            connector.health().await.expect("final health").status,
            ConnectorStatus::Disconnected
        );
    }

    #[test]
    fn connect_config_has_backward_compatible_timeout_and_rejects_zero() {
        let config: ConnectConfig = serde_json::from_value(json!({
            "endpoint": "https://example.test"
        }))
        .expect("deserialize config");
        assert_eq!(config.timeout_ms, 5_000);
        assert!(config.validate().is_ok());

        let mut invalid = config;
        invalid.timeout_ms = 0;
        assert!(invalid.validate().is_err());
        invalid.timeout_ms = 1;
        invalid.endpoint = "  ".to_owned();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn connector_kind_accepts_documented_compatibility_spellings() {
        for (wire, expected) in [
            ("chain_rpc", ConnectorKind::ChainRpc),
            ("chain-rpc", ConnectorKind::ChainRpc),
            ("blockchain_rpc", ConnectorKind::ChainRpc),
            ("mcp_server", ConnectorKind::McpServer),
            ("mcp-server", ConnectorKind::McpServer),
            ("a2a", ConnectorKind::A2aClient),
            ("a2a_client", ConnectorKind::A2aClient),
        ] {
            assert_eq!(
                serde_json::from_value::<ConnectorKind>(json!(wire)).expect("decode kind"),
                expected
            );
        }
        assert_eq!(
            serde_json::from_value::<ConnectorKind>(json!("blockchain"))
                .expect("legacy blockchain"),
            ConnectorKind::Blockchain
        );
    }

    #[test]
    fn manifest_validation_rejects_non_progressing_reconnect_policies() {
        let mut manifest = ConnectorManifest {
            name: "ethereum".to_owned(),
            kind: ConnectorKind::ChainRpc,
            version: "1.0.0".to_owned(),
            description: "Ethereum RPC".to_owned(),
            config_schema: Some(json!({"type": "object"})),
            capabilities: vec!["network".to_owned()],
            health_interval_secs: 30,
            reconnect_strategy: ReconnectStrategy::ExponentialBackoff {
                base_ms: 100,
                max_ms: 10_000,
                jitter: true,
            },
        };
        assert!(manifest.validate().is_ok());
        let encoded = serde_json::to_string(&manifest).expect("serialize manifest");
        assert_eq!(
            serde_json::from_str::<ConnectorManifest>(&encoded).expect("restore manifest"),
            manifest
        );

        manifest.reconnect_strategy = ReconnectStrategy::ExponentialBackoff {
            base_ms: 100,
            max_ms: 99,
            jitter: false,
        };
        assert!(manifest.validate().is_err());
        manifest.reconnect_strategy = ReconnectStrategy::FixedInterval { interval_ms: 0 };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn finality_reorg_and_discovery_contracts_round_trip() {
        assert!(FinalityLevel::Reversible < FinalityLevel::QuasiFinalized);
        assert!(FinalityLevel::QuasiFinalized < FinalityLevel::Final);

        let tag = FinalityTag {
            level: FinalityLevel::QuasiFinalized,
            chain_id: 1,
            block_number: 20,
            confirmations: 12,
            timestamp: Utc::now(),
        };
        let tag_json = serde_json::to_string(&tag).expect("serialize tag");
        assert_eq!(
            serde_json::from_str::<FinalityTag>(&tag_json).expect("restore tag"),
            tag
        );

        let mut reorg = ChainReorgPulse {
            chain_id: 8453,
            old_head: "0xold".to_owned(),
            new_head: "0xnew".to_owned(),
            orphaned_range_start: 100,
            orphaned_range_end: 102,
            new_range_start: 100,
            new_range_end: 103,
            depth: 3,
        };
        assert!(reorg.validate().is_ok());
        reorg.orphaned_range_start = 103;
        assert!(reorg.validate().is_err());

        let merged = MergedAgent {
            id: "agent-1".to_owned(),
            name: "coder".to_owned(),
            online: true,
            last_seen: 42,
            mode: Some("persistent".to_owned()),
            profile: Some("coding".to_owned()),
            a2a_capabilities: Some(vec!["review".to_owned()]),
            hdc_fingerprint: Some("base64:abc".to_owned()),
            supported_protocols: Some(vec!["a2a".to_owned()]),
            wallet: Some("0xabc".to_owned()),
            reputation: Some(0.9),
            stake: Some(100),
            tier: Some(AgentTier::Silver),
            direct_url: Some("https://agent.example".to_owned()),
            deploy_platform: Some(DeployPlatform::Fly),
        };
        let merged_json = serde_json::to_string(&merged).expect("serialize merged agent");
        assert_eq!(
            serde_json::from_str::<MergedAgent>(&merged_json).expect("restore merged agent"),
            merged
        );
    }
}
