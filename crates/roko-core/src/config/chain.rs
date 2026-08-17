//! Chain and relay configuration sections.

use serde::{Deserialize, Serialize};

/// Chain connection settings used by the `chain.*` tool domain.
///
/// When the `[chain]` section is present in TOML but `enabled` is omitted,
/// serde uses `default_true()` → chain is enabled. When no `[chain]` section
/// exists at all, `ChainConfig::default()` sets `enabled: false`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Whether the chain subsystem is active. Default: `false` (no `[chain]`
    /// section = chain off). When `[chain]` is present, defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Chain profile name: "mirage" (local dev), "daeji" (testnet), or custom.
    /// Resolves into a ChainProfile at runtime via ChainProfile::from_roko_config().
    #[serde(default = "default_chain_profile")]
    pub profile: String,
    /// HTTP JSON-RPC endpoint (e.g. `https://mirage-devnet.up.railway.app`).
    #[serde(default)]
    pub rpc_url: Option<String>,
    /// Chain ID. Must match the endpoint. Mirage uses 1.
    #[serde(default)]
    pub chain_id: Option<u64>,
    /// Hex-encoded private key (0x-prefixed or bare). Used to sign txs.
    #[serde(default)]
    pub wallet_key: Option<String>,
    /// ERC-8004 IdentityRegistry contract address.
    #[serde(default)]
    pub identity_registry: Option<String>,
    /// ERC-8004 ReputationRegistry contract address.
    #[serde(default)]
    pub reputation_registry: Option<String>,
    /// ERC-8004 ValidationRegistry contract address.
    #[serde(default)]
    pub validation_registry: Option<String>,
    /// KnowledgeRegistry / InsightBoard contract address used by the read-only indexer.
    #[serde(default)]
    pub knowledge_registry: Option<String>,
    /// AgentRegistry contract address. Required for on-chain agent features.
    #[serde(default)]
    pub agent_registry: Option<String>,
    /// BountyMarket contract address. Required for on-chain bounty features.
    #[serde(default)]
    pub bounty_market: Option<String>,
    /// Deployer / funder address.
    #[serde(default)]
    pub deployer: Option<String>,
    /// Confirmation depth required before an event is considered final.
    #[serde(default)]
    pub finality_confirmations: Option<u64>,
}

/// Relay registration and workspace discovery settings.
///
/// When enabled, `roko serve` registers itself with the relay on startup so
/// that dashboards can auto-discover the workspace without manual URL entry.
///
/// ```toml
/// [relay]
/// url = "wss://relay.nunchi.dev"
/// workspace_name = "will-dev"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RelayConfig {
    /// Relay WebSocket URL (e.g. `wss://relay.nunchi.dev`).
    /// If unset, workspace registration is disabled.
    #[serde(default)]
    pub url: Option<String>,
    /// Human-readable workspace name shown in dashboard discovery.
    /// Defaults to hostname.
    #[serde(default)]
    pub workspace_name: Option<String>,
    /// Public URL of this roko instance (e.g. `https://my-roko.up.railway.app`).
    /// Auto-detected from RAILWAY_PUBLIC_DOMAIN or FLY_APP_NAME if not set.
    #[serde(default)]
    pub public_url: Option<String>,
    /// Heartbeat interval in seconds for workspace presence. Default: 30.
    #[serde(default = "default_relay_heartbeat")]
    pub heartbeat_interval_secs: u64,
    /// Number of relay events retained for resume replay. Default: 65,536.
    #[serde(default = "default_ring_buffer_size")]
    pub ring_buffer_size: usize,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            url: None,
            workspace_name: None,
            public_url: None,
            heartbeat_interval_secs: 30,
            ring_buffer_size: default_ring_buffer_size(),
        }
    }
}

const fn default_relay_heartbeat() -> u64 {
    30
}

const fn default_ring_buffer_size() -> usize {
    65_536
}

/// `[feed_agents]` section in roko.toml — controls whether the 10 built-in
/// feed agents are spawned at serve startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeedAgentsConfig {
    /// Whether feed agents are enabled (default: false).
    pub enabled: bool,
}

impl Default for FeedAgentsConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: default_chain_profile(),
            rpc_url: None,
            chain_id: None,
            wallet_key: None,
            identity_registry: None,
            reputation_registry: None,
            validation_registry: None,
            knowledge_registry: None,
            agent_registry: None,
            bounty_market: None,
            deployer: None,
            finality_confirmations: None,
        }
    }
}

const fn default_true() -> bool {
    true
}

fn default_chain_profile() -> String {
    "mirage".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_config_profile_default() {
        let config: ChainConfig = toml::from_str("").unwrap();
        assert_eq!(config.profile, "mirage");
        // When [chain] section IS present, enabled defaults to true.
        assert!(config.enabled);
    }

    #[test]
    fn chain_config_default_impl_disabled() {
        // When no [chain] section exists, Default::default() has enabled=false.
        let config = ChainConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn chain_config_explicit_enabled_false() {
        let config: ChainConfig = toml::from_str("enabled = false").unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn chain_finality_confirmations_are_optional_and_round_trip() {
        let legacy: ChainConfig = toml::from_str("enabled = true").unwrap();
        assert_eq!(legacy.finality_confirmations, None);

        let configured: ChainConfig = toml::from_str("finality_confirmations = 64").unwrap();
        assert_eq!(configured.finality_confirmations, Some(64));
        let encoded = toml::to_string(&configured).expect("serialize chain config");
        assert_eq!(
            toml::from_str::<ChainConfig>(&encoded)
                .expect("restore chain config")
                .finality_confirmations,
            Some(64)
        );
    }

    #[test]
    fn relay_ring_buffer_defaults_for_legacy_and_default_configs() {
        let parsed: RelayConfig = toml::from_str("").expect("parse legacy relay config");
        assert_eq!(parsed.ring_buffer_size, 65_536);
        assert_eq!(RelayConfig::default().ring_buffer_size, 65_536);

        let custom: RelayConfig =
            toml::from_str("ring_buffer_size = 128").expect("parse configured relay");
        assert_eq!(custom.ring_buffer_size, 128);
    }
}
