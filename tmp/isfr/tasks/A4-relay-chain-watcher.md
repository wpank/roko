# A4: Add Chain Event Watcher to Relay

## Context

The relay should watch a local EVM chain (via WebSocket RPC) and publish block/event notifications as topic messages. This enables agents to react to on-chain events without each agent maintaining its own chain connection.

The relay already has TopicBus (A2) and WebSocket pub/sub wiring (A3). This task adds a background task that subscribes to new blocks and emits `TopicEnvelope`s.

## File to Create

- `apps/agent-relay/src/chain_watcher.rs` (NEW)

## File to Modify

- `apps/agent-relay/src/lib.rs` — add `mod chain_watcher;`, spawn watcher on startup
- `apps/agent-relay/src/main.rs` — add `--rpc-ws-url` CLI argument
- `apps/agent-relay/Cargo.toml` — add alloy dependencies

## Implementation

### Step 1: Add CLI argument

In `apps/agent-relay/src/main.rs`, add to the CLI struct:

```rust
/// WebSocket RPC URL for chain watching (e.g., ws://localhost:8545).
/// If not provided, chain watcher is disabled.
#[arg(long)]
rpc_ws_url: Option<String>,
```

### Step 2: Create `apps/agent-relay/src/chain_watcher.rs`

```rust
//! Chain event watcher — subscribes to new blocks via WebSocket RPC
//! and publishes them to the relay's TopicBus.

use crate::bus::TopicBus;
use crate::protocol::TopicEnvelope;
use crate::state::RelayState;
use crate::protocol::RelayOutboundFrame;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, error};

/// Configuration for the chain watcher.
pub struct ChainWatcherConfig {
    /// WebSocket RPC URL (e.g., ws://localhost:8545).
    pub rpc_ws_url: String,
    /// Chain ID for topic naming (e.g., "31337" → topic "chain:31337").
    pub chain_id: String,
}

/// Start the chain watcher background task.
///
/// Subscribes to `eth_subscribe("newHeads")` and publishes each new block
/// header as a TopicEnvelope on `chain:{chain_id}`.
///
/// Also watches for specific contract events if configured.
pub async fn start_chain_watcher(
    config: ChainWatcherConfig,
    state: Arc<RelayState>,
    cancel: CancellationToken,
) {
    let topic = format!("chain:{}", config.chain_id);
    info!(rpc_url = %config.rpc_ws_url, topic = %topic, "starting chain watcher");

    // NOTE: Full implementation uses alloy's pubsub provider:
    //
    // use alloy::providers::{ProviderBuilder, WsConnect};
    // use alloy::rpc::types::eth::Filter;
    //
    // let ws = WsConnect::new(&config.rpc_ws_url);
    // let provider = ProviderBuilder::new().on_ws(ws).await?;
    // let sub = provider.subscribe_blocks().await?;
    //
    // For now, implement a polling fallback that works without alloy WS:

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
    let mut last_block: u64 = 0;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("chain watcher shutting down");
                break;
            }
            _ = interval.tick() => {
                // Poll for new block number via JSON-RPC.
                match poll_block_number(&config.rpc_ws_url).await {
                    Ok(block_num) if block_num > last_block => {
                        let envelope = TopicEnvelope::new(
                            &topic,
                            "new_block",
                            serde_json::json!({
                                "block_number": block_num,
                                "chain_id": &config.chain_id,
                            }),
                        ).with_publisher("chain-watcher");

                        let (seq, subscribers) = state.bus.publish(envelope.clone());

                        // Fan out to subscribers.
                        for sub_id in &subscribers {
                            let frame = RelayOutboundFrame::TopicMessage {
                                topic: envelope.topic.clone(),
                                msg_type: envelope.msg_type.clone(),
                                payload: envelope.payload.clone(),
                                publisher_id: envelope.publisher_id.clone(),
                                seq,
                            };
                            state.send_to_agent(sub_id, frame);
                        }

                        last_block = block_num;
                    }
                    Ok(_) => {} // No new block
                    Err(e) => {
                        warn!(error = %e, "failed to poll block number");
                    }
                }
            }
        }
    }
}

/// Poll eth_blockNumber via HTTP JSON-RPC.
async fn poll_block_number(rpc_url: &str) -> anyhow::Result<u64> {
    // Convert ws:// to http:// for polling
    let http_url = rpc_url
        .replace("ws://", "http://")
        .replace("wss://", "https://");

    let client = reqwest::Client::new();
    let resp = client
        .post(&http_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let hex = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no result in response"))?;
    let num = u64::from_str_radix(hex.trim_start_matches("0x"), 16)?;
    Ok(num)
}
```

### Step 3: Spawn watcher on startup

In `apps/agent-relay/src/main.rs` (or wherever the server starts), after creating RelayState:

```rust
// Spawn chain watcher if RPC URL provided.
if let Some(rpc_ws_url) = args.rpc_ws_url {
    let watcher_config = chain_watcher::ChainWatcherConfig {
        rpc_ws_url,
        chain_id: "31337".to_string(), // TODO: detect from chain or CLI arg
    };
    let state_clone = state.clone();
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        chain_watcher::start_chain_watcher(watcher_config, state_clone, cancel_clone).await;
    });
}
```

### Step 4: Add dependencies

In `apps/agent-relay/Cargo.toml`:

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
anyhow = "1"
tokio-util = { version = "0.7", features = ["rt"] }
```

## Verification

```bash
cargo build -p agent-relay
# Start with chain watching:
# cargo run -p agent-relay -- --rpc-ws-url ws://localhost:8545
# Then connect an agent and subscribe to chain:31337
```

## Dependencies

- A2 (TopicBus with `publish()`)
- A3 (`send_to_agent()` helper on RelayState)

## Critical Notes

### Dependencies note

This task uses `state.send_to_agent()` which is added by A3. If implementing A4 before A3,
you can still publish to the bus (storing messages in the ring buffer) — fan-out just won't
work until A3's helper method exists.

### reqwest already in agent-relay?

Check before adding:
```bash
grep "reqwest" apps/agent-relay/Cargo.toml
```
If already present, don't add a duplicate entry.

### CancellationToken

Uses `tokio_util::sync::CancellationToken`. The existing relay code likely already has a
shutdown mechanism — find it and pass the same cancel token to the chain watcher:
```bash
grep -n "CancellationToken\|shutdown\|cancel" apps/agent-relay/src/main.rs
```

## Future Enhancement

Replace polling with alloy WebSocket subscription (`eth_subscribe newHeads`) for instant block notifications. The polling approach works for dev but adds 2s latency.
