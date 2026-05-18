# 01: Relay Upgrade — Topic Pub/Sub + Chain Watcher

Upgrade the existing `apps/agent-relay/` with topic-based pub/sub, ring buffer for reconnection, and a chain event watcher. All changes are additive — existing request/response functionality stays untouched.

## Current State

**File:** `apps/agent-relay/src/protocol.rs` (~182 lines)

```
AgentInboundFrame: Hello | Card | Response | Error | Ping
RelayOutboundFrame: Ack | Message | Error | Pong
```

**File:** `apps/agent-relay/src/lib.rs` (~346 lines)
- axum routes: `/relay/agents/ws`, `/relay/messages`, `/relay/agents`, `/relay/cards/{id}`
- WebSocket handler: Hello → Ack → Card → Ack → message loop
- State: agents HashMap, cards HashMap, pending responses HashMap, workspaces HashMap

**File:** `crates/roko-agent-server/src/features/relay_client.rs` (~290 lines)
- Outbound WebSocket, Hello+Card handshake, Message dispatch loop
- No subscribe/publish capability

## What to Add

### 1. New Frame Types in `protocol.rs`

Add to `AgentInboundFrame`:

```rust
/// Subscribe to one or more topics for pub/sub delivery.
Subscribe {
    topics: Vec<String>,
},
/// Unsubscribe from topics.
Unsubscribe {
    topics: Vec<String>,
},
/// Publish a message to a topic.
Publish {
    topic: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    msg_type: Option<String>,
    payload: Value,
},
```

Add to `RelayOutboundFrame`:

```rust
/// Welcome frame sent after hello, includes current sequence number.
Welcome {
    seq: u64,
},
/// Envelope containing a published message routed by topic.
Envelope {
    seq: u64,
    ts: u64,
    topic: String,
    from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    msg_type: Option<String>,
    payload: Value,
},
/// Indicates a gap in sequence numbers (messages lost during disconnect).
Gap {
    from_seq: u64,
    to_seq: u64,
},
```

**Design notes:**
- `msg_type` and `payload` are opaque to the relay — applications define semantics
- `seq` is per-connection, monotonic, relay-assigned
- `ts` is millisecond Unix timestamp
- Uses the same `#[serde(tag = "type", rename_all = "snake_case")]` pattern as existing frames
- `Welcome` replaces the initial `Ack { event: "hello" }` for pub/sub-aware clients (old clients still get `Ack`)

**~40 lines added to protocol.rs**

### 2. Bus Module (`bus.rs`)

New file: `apps/agent-relay/src/bus.rs`

```rust
use dashmap::DashMap;
use serde_json::Value;
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

/// Connection identifier (unique per WebSocket session).
pub type ConnId = u64;

/// A routed message on the bus.
#[derive(Clone, Debug)]
pub struct Envelope {
    pub seq: u64,
    pub ts: u64,
    pub topic: String,
    pub from: String,
    pub msg_type: Option<String>,
    pub payload: Value,
}

/// Per-connection state: outbound sender + replay ring.
pub struct ConnectionState {
    pub sender: mpsc::UnboundedSender<Envelope>,
    ring: VecDeque<Envelope>,
    ring_capacity: usize,
    seq: AtomicU64,
}

impl ConnectionState {
    pub fn new(sender: mpsc::UnboundedSender<Envelope>, ring_capacity: usize) -> Self {
        Self {
            sender,
            ring: VecDeque::with_capacity(ring_capacity),
            ring_capacity,
            seq: AtomicU64::new(0),
        }
    }

    pub fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed)
    }

    pub fn push_ring(&mut self, envelope: Envelope) {
        if self.ring.len() >= self.ring_capacity {
            self.ring.pop_front();
        }
        self.ring.push_back(envelope);
    }

    pub fn replay_after(&self, after_seq: u64) -> Vec<Envelope> {
        self.ring
            .iter()
            .filter(|e| e.seq > after_seq)
            .cloned()
            .collect()
    }
}

/// Topic-based pub/sub bus with per-connection ring buffers.
pub struct Bus {
    /// topic → set of subscriber connection IDs
    subscriptions: DashMap<String, HashSet<ConnId>>,
    /// connection ID → connection state
    connections: DashMap<ConnId, ConnectionState>,
    /// Default ring buffer capacity per connection
    ring_capacity: usize,
}

impl Bus {
    pub fn new(ring_capacity: usize) -> Self {
        Self {
            subscriptions: DashMap::new(),
            connections: DashMap::new(),
            ring_capacity,
        }
    }

    /// Register a new connection. Returns a receiver for outbound envelopes.
    pub fn connect(&self, conn_id: ConnId) -> mpsc::UnboundedReceiver<Envelope> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.connections.insert(conn_id, ConnectionState::new(tx, self.ring_capacity));
        rx
    }

    /// Remove a connection and all its subscriptions.
    pub fn disconnect(&self, conn_id: ConnId) {
        self.connections.remove(&conn_id);
        // Remove from all subscription sets
        self.subscriptions.iter_mut().for_each(|mut entry| {
            entry.value_mut().remove(&conn_id);
        });
    }

    /// Subscribe a connection to topics. Creates topics if they don't exist.
    pub fn subscribe(&self, conn_id: ConnId, topics: &[String]) {
        for topic in topics {
            self.subscriptions
                .entry(topic.clone())
                .or_insert_with(HashSet::new)
                .insert(conn_id);
        }
    }

    /// Unsubscribe a connection from topics.
    pub fn unsubscribe(&self, conn_id: ConnId, topics: &[String]) {
        for topic in topics {
            if let Some(mut subs) = self.subscriptions.get_mut(topic) {
                subs.remove(&conn_id);
            }
        }
    }

    /// Publish to a topic. Returns the number of subscribers that received it.
    pub fn publish(
        &self,
        topic: &str,
        from: &str,
        msg_type: Option<String>,
        payload: Value,
    ) -> usize {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let subscribers = match self.subscriptions.get(topic) {
            Some(subs) => subs.clone(),
            None => return 0,
        };

        let mut delivered = 0;
        for conn_id in &subscribers {
            if let Some(mut conn) = self.connections.get_mut(conn_id) {
                let seq = conn.next_seq();
                let envelope = Envelope {
                    seq,
                    ts,
                    topic: topic.to_string(),
                    from: from.to_string(),
                    msg_type: msg_type.clone(),
                    payload: payload.clone(),
                };
                conn.push_ring(envelope.clone());
                let _ = conn.sender.send(envelope);
                delivered += 1;
            }
        }
        delivered
    }

    /// Replay missed messages for a reconnecting client.
    pub fn resume(&self, conn_id: ConnId, after_seq: u64) -> Vec<Envelope> {
        self.connections
            .get(&conn_id)
            .map(|conn| conn.replay_after(after_seq))
            .unwrap_or_default()
    }

    /// List all active topics.
    pub fn topics(&self) -> Vec<String> {
        self.subscriptions
            .iter()
            .filter(|entry| !entry.value().is_empty())
            .map(|entry| entry.key().clone())
            .collect()
    }
}
```

**Key design decisions:**
- `DashMap` for lock-free concurrent access (already a dependency pattern in roko)
- Per-connection ring buffer (not global) — each client has its own replay window
- Topics are dynamic — subscribing creates them, no pre-registration
- `publish()` fans out to all subscribers of a topic
- Ring buffer capacity configurable (default 65536 entries)
- Pattern identical to roko-core's `EventBus<E>` but keyed by topic and over WebSocket

**~150 lines**

### 3. Wire Into WebSocket Handler (`lib.rs`)

Add bus handling to the existing WebSocket message loop:

```rust
// In the WebSocket handler, after existing Hello/Card/Response/Error/Ping handling:
AgentInboundFrame::Subscribe { topics } => {
    bus.subscribe(conn_id, &topics);
    send_relay_frame(&mut sink, RelayOutboundFrame::Ack {
        event: "subscribed".to_string(),
    }).await?;
}
AgentInboundFrame::Unsubscribe { topics } => {
    bus.unsubscribe(conn_id, &topics);
}
AgentInboundFrame::Publish { topic, msg_type, payload } => {
    bus.publish(&topic, &agent_id, msg_type, payload);
}
```

Add a concurrent task that forwards bus envelopes to the WebSocket:

```rust
// Spawn a task to forward bus envelopes to the WebSocket sink
let mut bus_rx = bus.connect(conn_id);
tokio::spawn(async move {
    while let Some(envelope) = bus_rx.recv().await {
        let frame = RelayOutboundFrame::Envelope {
            seq: envelope.seq,
            ts: envelope.ts,
            topic: envelope.topic,
            from: envelope.from,
            msg_type: envelope.msg_type,
            payload: envelope.payload,
        };
        if send_relay_frame(&mut envelope_sink, frame).await.is_err() {
            break;
        }
    }
});
```

**Design note:** The WebSocket handler splits the sink — one half for direct responses (Ack, Pong), one half for envelope forwarding. This is the same split-sink pattern already used in the relay.

**~60 lines added to lib.rs**

### 4. Add Bus to Relay State

```rust
// In state.rs or wherever RelayState is defined:
pub struct RelayState {
    // ... existing fields ...
    pub bus: Arc<Bus>,
}
```

Bus is created at startup with configurable ring capacity:

```rust
let bus = Arc::new(Bus::new(65_536)); // 64K entries per connection
```

**~10 lines**

### 5. Chain Event Watcher (`chain.rs`)

New file: `apps/agent-relay/src/chain.rs`

This is what makes the relay chain-aware. It watches for contract events and publishes them as envelopes on the bus.

```rust
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::Filter;
use alloy::sol_types::SolEvent;
use std::sync::Arc;

use crate::bus::Bus;

/// Watch chain events and publish to bus topics.
pub struct ChainWatcher {
    bus: Arc<Bus>,
    rpc_url: String,
    chain_topic: String,
}

impl ChainWatcher {
    pub fn new(bus: Arc<Bus>, rpc_url: String, chain_id: &str) -> Self {
        Self {
            bus,
            rpc_url,
            chain_topic: format!("chain:{chain_id}"),
        }
    }

    /// Start watching. Subscribes to contract events via alloy WS provider.
    pub async fn start(self) -> anyhow::Result<()> {
        let ws = WsConnect::new(&self.rpc_url);
        let provider = ProviderBuilder::new().on_ws(ws).await?;

        // Watch for ISFROracle events
        let filter = Filter::new();
        let mut sub = provider.subscribe_logs(&filter).await?;

        while let Some(log) = sub.next().await {
            // Decode known events, publish as typed envelopes
            let payload = serde_json::to_value(&log)?;
            let event_type = decode_event_type(&log);

            self.bus.publish(
                &self.chain_topic,
                "chain-watcher",
                Some(event_type),
                payload,
            );
        }
        Ok(())
    }
}

fn decode_event_type(log: &alloy::rpc::types::Log) -> String {
    // Match on topic[0] (event signature hash) to identify event type
    // ISFROracle events: RateSubmitted, RangeClosed, EpochAdvanced
    // ISFRBountyPool events: BountyDeposited, RewardClaimed
    // WorkerRegistry events: WorkerRegistered, ReputationUpdated
    // Default to "unknown" for unrecognized events
    "chain.event".to_string()
}
```

**Design notes:**
- Uses alloy's WS provider for real-time event subscription
- Publishes to `chain:{chain_id}` topic — agents subscribe to get chain events without running their own watcher
- Event type decoding is extensible — add new event signature matches as contracts are added
- `rpc_url` and `chain_id` come from the active `ChainProfile` (see 05-contracts-deployment.md). For mirage-rs: `ws://localhost:8545`, for daeji: the kora RPC endpoint, for mainnet: any Ethereum RPC.
- Chain watcher is optional — relay works without it for pure pub/sub use cases

**~100 lines**

### 6. HTTP Endpoints for Feed Discovery

Add to existing routes:

```rust
// GET /relay/topics — list active topics with subscriber counts
async fn list_topics(State(state): State<Arc<RelayState>>) -> Json<Vec<TopicInfo>> {
    let topics = state.bus.topics();
    Json(topics.into_iter().map(|t| TopicInfo {
        topic: t.clone(),
        subscribers: state.bus.subscriber_count(&t),
    }).collect())
}

// GET /relay/feeds — list topics with feed: prefix + metadata
async fn list_feeds(State(state): State<Arc<RelayState>>) -> Json<Vec<FeedInfo>> {
    let feeds = state.bus.topics()
        .into_iter()
        .filter(|t| t.starts_with("feed:"))
        .map(|t| {
            let meta = state.feed_registry.get(&t);
            FeedInfo { topic: t, meta }
        })
        .collect();
    Json(feeds)
}

// POST /relay/feeds — register feed metadata
async fn register_feed(
    State(state): State<Arc<RelayState>>,
    Json(info): Json<FeedMeta>,
) -> StatusCode {
    state.feed_registry.insert(info.topic.clone(), info);
    StatusCode::CREATED
}
```

Feed metadata structure:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedMeta {
    pub topic: String,
    pub name: String,
    pub kind: String,       // "raw", "derived", "composite", "meta"
    pub producer: String,   // agent_id of the producer
    pub schema: Option<Value>,
    pub description: Option<String>,
}
```

**~90 lines across feeds.rs**

### 7. Relay Client Upgrade (`relay_client.rs`)

Add subscribe/publish to the agent-side relay client:

```rust
// New methods on the relay connection:

/// Subscribe to topics on the relay bus.
pub async fn subscribe(&mut self, topics: &[&str]) -> Result<()> {
    let frame = AgentInboundFrame::Subscribe {
        topics: topics.iter().map(|t| t.to_string()).collect(),
    };
    send_frame(&mut self.socket, frame).await
}

/// Publish a message to a relay topic.
pub async fn publish(
    &mut self,
    topic: &str,
    msg_type: Option<&str>,
    payload: Value,
) -> Result<()> {
    let frame = AgentInboundFrame::Publish {
        topic: topic.to_string(),
        msg_type: msg_type.map(String::from),
        payload,
    };
    send_frame(&mut self.socket, frame).await
}
```

Also update the `run()` loop to handle new outbound frames:

```rust
RelayOutboundFrame::Envelope { topic, payload, .. } => {
    // Dispatch to registered topic handlers
    if let Some(handler) = self.topic_handlers.get(&topic) {
        handler(payload).await;
    }
}
RelayOutboundFrame::Welcome { seq } => {
    self.current_seq = seq;
}
```

**Design note:** The relay client becomes a dual-mode connection:
1. **Request/response** (existing) — relay forwards messages, agent responds
2. **Pub/sub** (new) — agent subscribes to topics, receives envelopes, publishes to topics

Both modes coexist on the same WebSocket. No second connection needed.

**~50 lines added to relay_client.rs**

## File Summary

| File | Action | Lines |
|------|--------|-------|
| `apps/agent-relay/src/protocol.rs` | Add Subscribe/Unsubscribe/Publish/Welcome/Envelope/Gap frames | ~40 |
| `apps/agent-relay/src/bus.rs` | NEW — Topic pub/sub + ring buffer | ~150 |
| `apps/agent-relay/src/lib.rs` | Wire subscribe/publish/envelope into WebSocket handler | ~60 |
| `apps/agent-relay/src/chain.rs` | NEW — Chain event watcher → bus | ~100 |
| `apps/agent-relay/src/feeds.rs` | NEW — Feed metadata registry + HTTP endpoints | ~90 |
| `crates/roko-agent-server/src/features/relay_client.rs` | Add subscribe/publish methods | ~50 |
| **Total** | | **~490** |

## Testing

```bash
# Unit tests for bus
cargo test -p agent-relay -- bus

# Integration test: two agents pub/sub on same topic
cargo test -p agent-relay -- pubsub_integration

# Integration test: reconnect with resume_after
cargo test -p agent-relay -- resume_integration

# Verify existing tests still pass
cargo test -p agent-relay
```

## Compatibility

- Existing agents that don't use Subscribe/Publish continue working unchanged
- The `Ack { event: "hello" }` response is preserved for backwards compatibility
- New pub/sub-aware clients also receive `Welcome { seq }` after hello
- No changes to the HTTP `/relay/messages` endpoint
- No changes to the dashboard event WebSocket at `/relay/events/ws`
