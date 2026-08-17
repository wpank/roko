#![allow(missing_docs)]

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_subscriber::EnvFilter;

use agent_relay::{
    app,
    bus::TopicBusConfig,
    chain_watcher::{ChainWatcherConfig, start_chain_watcher},
    registry::{RegistryPublishersFile, RegistryStore},
    state::RelayState,
};

#[derive(Debug, Parser)]
#[command(name = "agent-relay")]
#[command(about = "Standalone websocket relay for agent presence and forwarding.")]
struct Cli {
    /// Address to bind, for example 127.0.0.1:9011.
    #[arg(long, env = "ROKO_AGENT_RELAY_BIND", default_value = "127.0.0.1:9011")]
    bind: String,

    /// WebSocket RPC URL for chain event watching (e.g. ws://localhost:8545).
    /// When provided, the relay polls for new blocks and publishes them to the
    /// `chain:{chain_id}` topic. Chain watching is disabled when omitted.
    #[arg(long, env = "ROKO_AGENT_RELAY_RPC_WS_URL")]
    rpc_ws_url: Option<String>,

    /// Chain ID reported in chain-watcher topic messages (default: 31337 = Anvil/Hardhat).
    /// Ignored when `--rpc-ws-url` is not set.
    #[arg(long, env = "ROKO_AGENT_RELAY_CHAIN_ID", default_value = "31337")]
    chain_id: String,

    /// Persistent extension registry directory. Registry routes are absent
    /// when this is not configured.
    #[arg(long, env = "ROKO_EXTENSION_REGISTRY_DIR")]
    registry_dir: Option<std::path::PathBuf>,

    /// JSON file containing publisher ids, token SHA-256 digests, and Ed25519
    /// public keys. Without this trust configuration registry routes stay absent.
    #[arg(long, env = "ROKO_EXTENSION_REGISTRY_PUBLISHERS_FILE")]
    registry_publishers_file: Option<std::path::PathBuf>,

    /// Maximum number of events retained by the global replay ring.
    #[arg(long, env = "ROKO_AGENT_RELAY_RING_ENTRIES", default_value_t = 65_536)]
    ring_entries: usize,

    /// Maximum serialized bytes retained by the global replay ring.
    #[arg(long, env = "ROKO_AGENT_RELAY_RING_BYTES", default_value_t = 64 * 1024 * 1024)]
    ring_bytes: usize,

    /// Maximum queued frames per connected relay consumer.
    #[arg(long, env = "ROKO_AGENT_RELAY_DELIVERY_ENTRIES", default_value_t = 256)]
    delivery_entries: usize,

    /// Maximum logical serialized bytes queued per relay consumer.
    #[arg(long, env = "ROKO_AGENT_RELAY_DELIVERY_BYTES", default_value_t = 8 * 1024 * 1024)]
    delivery_bytes: usize,
}

impl Cli {
    fn bus_config(&self) -> Result<TopicBusConfig> {
        let config = TopicBusConfig {
            ring_capacity: self.ring_entries,
            ring_byte_capacity: self.ring_bytes,
            delivery_capacity: self.delivery_entries,
            delivery_byte_capacity: self.delivery_bytes,
            ..TopicBusConfig::default()
        };
        // Validate through the public constructor before any listener state is
        // made visible.
        agent_relay::TopicBus::try_new(config.clone())
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("ROKO_LOG")
                .unwrap_or_else(|_| EnvFilter::new("agent_relay=info,tower_http=info")),
        )
        .init();

    let cli = Cli::parse();
    let bus_config = cli.bus_config()?;
    let listener = TcpListener::bind(&cli.bind)
        .await
        .with_context(|| format!("bind agent relay to {}", cli.bind))?;
    let addr = listener.local_addr().context("read bound relay address")?;
    info!(%addr, "agent relay listening");

    if cli.registry_publishers_file.is_some() && cli.registry_dir.is_none() {
        anyhow::bail!("registry publisher config requires --registry-dir");
    }
    let state = if let Some(registry_dir) = cli.registry_dir {
        let publishers = if let Some(path) = cli.registry_publishers_file {
            let encoded = std::fs::read(&path)
                .with_context(|| format!("read registry publisher config {}", path.display()))?;
            serde_json::from_slice::<RegistryPublishersFile>(&encoded)
                .with_context(|| format!("parse registry publisher config {}", path.display()))?
                .publishers
        } else {
            Vec::new()
        };
        let registry = RegistryStore::open(registry_dir, publishers)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        Arc::new(
            RelayState::try_with_registry_config(registry, bus_config)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        )
    } else {
        Arc::new(
            RelayState::try_with_config(bus_config)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        )
    };
    let cancel = CancellationToken::new();

    // Expire stale workspaces every 30 seconds (stale = no heartbeat in 60s).
    let expiry_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let expired = expiry_state.expire_stale_workspaces(60_000);
            for id in &expired {
                tracing::debug!(workspace_id = %id, "expired stale workspace");
            }
        }
    });

    // Spawn chain watcher if an RPC URL was provided.
    if let Some(rpc_ws_url) = cli.rpc_ws_url {
        let watcher_config = ChainWatcherConfig {
            rpc_ws_url,
            chain_id: cli.chain_id,
        };
        let watcher_state = Arc::clone(&state);
        let watcher_cancel = cancel.clone();
        tokio::spawn(async move {
            start_chain_watcher(watcher_config, watcher_state, watcher_cancel).await;
        });
    }

    axum::serve(listener, app(state))
        .await
        .context("serve agent relay router")?;

    // Signal background tasks to stop.
    cancel.cancel();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_relay_bounds_are_validated_and_applied() {
        let cli = Cli::try_parse_from([
            "agent-relay",
            "--ring-entries",
            "17",
            "--ring-bytes",
            "4096",
            "--delivery-entries",
            "9",
            "--delivery-bytes",
            "2048",
        ])
        .expect("parse relay bounds");
        let config = cli.bus_config().expect("valid relay bounds");
        assert_eq!(config.ring_capacity, 17);
        assert_eq!(config.ring_byte_capacity, 4096);
        assert_eq!(config.delivery_capacity, 9);
        assert_eq!(config.delivery_byte_capacity, 2048);

        let invalid = Cli::try_parse_from(["agent-relay", "--ring-entries", "0"])
            .expect("parse invalid bound");
        assert!(invalid.bus_config().is_err());
    }
}
