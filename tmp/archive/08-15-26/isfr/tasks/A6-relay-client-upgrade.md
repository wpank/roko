# A6: Upgrade Relay Client with Pub/Sub Support

## Context

The relay client is in `crates/roko-agent-server/src/features/relay_client.rs`. It currently:
- Connects via WebSocket to `/relay/agents/ws`
- Sends Hello + Card frames
- Handles incoming Message frames (request/response pattern)
- Uses `mpsc` + `tokio::spawn` for the reader loop

This task EXTENDS the existing client to also handle:
- Subscribing to topics (Send Subscribe frame)
- Receiving TopicMessage frames (new variant in outbound)
- Publishing to topics (Send Publish frame)
- Providing a callback mechanism for topic message delivery

## File to Modify

- `crates/roko-agent-server/src/features/relay_client.rs`

## File to Create

- `crates/roko-agent-server/src/features/relay_subscriber.rs` (NEW — thin wrapper for pub/sub)

## Pre-Check

```bash
grep -n "enum.*Frame\|handle_relay_frame\|fn connect" crates/roko-agent-server/src/features/relay_client.rs
```

## Implementation

### Step 1: Update frame handling in relay_client.rs

The existing `run()` loop processes incoming frames. Currently it handles:
- `RelayOutboundFrame::Message` → dispatch to agent
- `RelayOutboundFrame::Ack` → ignored or logged
- `RelayOutboundFrame::Pong` → ignored
- `RelayOutboundFrame::Error` → logged

Add handling for the new `TopicMessage` variant:

```rust
RelayOutboundFrame::TopicMessage { topic, msg_type, payload, publisher_id, seq } => {
    tracing::debug!(topic = %topic, msg_type = %msg_type, seq, "received topic message");
    if let Some(ref handler) = topic_handler {
        handler.on_topic_message(&topic, &msg_type, payload, publisher_id.as_deref(), seq).await;
    }
}
```

### Step 2: Add TopicHandler trait

In `relay_client.rs`, add:

```rust
/// Callback interface for receiving topic messages from the relay.
#[async_trait::async_trait]
pub trait TopicHandler: Send + Sync + 'static {
    /// Called when a message arrives on a subscribed topic.
    async fn on_topic_message(
        &self,
        topic: &str,
        msg_type: &str,
        payload: serde_json::Value,
        publisher_id: Option<&str>,
        seq: u64,
    );
}
```

### Step 3: Add subscribe/publish methods

The existing `connect()` function returns after spawning the reader task. We need a handle to send frames back. The current architecture spawns a task with a WebSocket sink. To send Subscribe/Publish, we need a sender channel.

Modify the spawned task to also accept outbound frames. Add an `mpsc::UnboundedSender` that the caller retains:

```rust
/// Handle returned from connect() that allows sending pub/sub frames.
#[derive(Clone)]
pub struct RelayHandle {
    outbound_tx: mpsc::UnboundedSender<AgentInboundFrame>,
}

impl RelayHandle {
    /// Subscribe to a topic on the relay.
    pub fn subscribe(&self, topic: impl Into<String>) -> Result<()> {
        self.outbound_tx
            .send(AgentInboundFrame::Subscribe { topic: topic.into() })
            .map_err(|_| anyhow::anyhow!("relay connection closed"))
    }

    /// Unsubscribe from a topic.
    pub fn unsubscribe(&self, topic: impl Into<String>) -> Result<()> {
        self.outbound_tx
            .send(AgentInboundFrame::Unsubscribe { topic: topic.into() })
            .map_err(|_| anyhow::anyhow!("relay connection closed"))
    }

    /// Publish a message to a topic.
    pub fn publish(
        &self,
        topic: impl Into<String>,
        msg_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<()> {
        self.outbound_tx
            .send(AgentInboundFrame::Publish {
                topic: topic.into(),
                msg_type: msg_type.into(),
                payload,
            })
            .map_err(|_| anyhow::anyhow!("relay connection closed"))
    }
}
```

### Step 4: Modify `connect()` to return RelayHandle

Change the function signature:

```rust
pub async fn connect(
    config: RelayClientConfig,
    state: Arc<AgentState>,
    card: AgentCard,
    topic_handler: Option<Arc<dyn TopicHandler>>,
) -> Result<RelayHandle>
```

Inside `connect()`, after establishing the WebSocket:

```rust
// Create outbound channel for pub/sub frames.
let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<AgentInboundFrame>();

// Spawn task that:
// 1. Reads from WebSocket → dispatches messages / topic messages
// 2. Reads from outbound_rx → sends frames to WebSocket
tokio::spawn(async move {
    loop {
        tokio::select! {
            // Incoming from relay
            Some(msg) = ws_stream.next() => {
                // ... existing frame handling + new TopicMessage arm ...
            }
            // Outgoing from our code (subscribe/publish)
            Some(frame) = outbound_rx.recv() => {
                let json = serde_json::to_string(&frame).unwrap();
                if ws_sink.send(WsMessage::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    }
});

Ok(RelayHandle { outbound_tx })
```

### Step 5: Update callers

The only caller is in `crates/roko-agent-server/src/registration.rs`. Update:

```rust
// Before:
relay_client::connect(relay.clone(), state, card.clone()).await?;

// After:
let _handle = relay_client::connect(relay.clone(), state, card.clone(), None).await?;
```

For now, pass `None` for topic_handler — the ISFR keeper (C2) will use the handle later.

### Step 6: Add frame types to relay_client.rs

The client needs the new frame variants. Either:
- Import from `agent-relay` crate (if it's a workspace dependency), OR
- Define matching types locally (if relay is a separate binary)

Since `agent-relay` is in `apps/` and roko-agent-server is in `crates/`, they likely share types via a common protocol definition. Check if `AgentInboundFrame` is already imported. If not, add the new variants to whatever local frame enum the client uses:

```rust
// Add to whatever local enum represents outbound frames:
Subscribe { topic: String },
Unsubscribe { topic: String },
Publish { topic: String, msg_type: String, payload: serde_json::Value },

// Add to whatever local enum represents inbound frames from relay:
TopicMessage { topic: String, msg_type: String, payload: serde_json::Value, publisher_id: Option<String>, seq: u64 },
```

## Critical Implementation Notes

### WebSocket split

The current relay_client.rs likely uses `tokio_tungstenite` and does a `ws_stream.split()`
into `(sink, stream)`. The reader loop processes the stream side. To send outbound frames
(subscribe/publish), you need access to the sink. The `select!` approach in Step 4 merges
both into one task.

Check the actual WS type alias:
```bash
grep -n "WsMessage\|Message\|tungstenite" crates/roko-agent-server/src/features/relay_client.rs
```

`WsMessage` may be `tokio_tungstenite::tungstenite::Message` — use the correct path.

### Frame format

The relay uses internally-tagged serde: `{"type":"subscribe","topic":"..."}`. The client
must serialize `AgentInboundFrame` the same way. Verify the serde attributes match.

### Existing `run()` function

The current code may have a `run()` function that owns the WS connection. The refactor
either replaces `run()` with `connect() -> RelayHandle` or wraps it. Check whether `run()`
is called from a task or inline — if it's `tokio::spawn(run(...))`, the handle approach
works cleanly.

### Handle lifetime

`RelayHandle` holds an `UnboundedSender`. If the background task dies (relay disconnects),
sends via the handle will return errors. Callers must handle this gracefully. The handle
itself stays alive (no panic); failed sends return `Err`.

## Verification

```bash
cargo build -p roko-agent-server
cargo test -p roko-agent-server
```

## Dependencies

- A1 (frame types must be defined ��� at minimum the same JSON shape)

## What This Enables

- B1 (ISFRFeed) subscribes to relay topics via RelayHandle
- C2 (ISFRKeeper) publishes rate updates via RelayHandle
- E2 (CLI) can create a relay connection for the keeper
