# A3: Wire Bus into Relay WebSocket Handler

## Context

After A1 (frame types) and A2 (TopicBus), the relay can store subscriptions and publish messages. This task connects the WebSocket handler to the bus so that agents can subscribe/unsubscribe/publish via their existing WebSocket connection.

## File to Modify

- `apps/agent-relay/src/lib.rs` — add arms to `handle_agent_frame()` match

## Pre-Check

```bash
# Verify handle_agent_frame exists and handles current variants
grep -n "handle_agent_frame\|AgentInboundFrame" apps/agent-relay/src/lib.rs
# Verify TopicBus is on RelayState
grep -n "bus:" apps/agent-relay/src/state.rs
```

## Implementation

### Step 1: Add match arms to `handle_agent_frame()`

Find the `handle_agent_frame()` function in `apps/agent-relay/src/lib.rs`. It currently matches on Card, Response, Error, Ping, Hello. The function signature includes a parameter named `outbound_tx` (NOT `tx`). Verify with: `grep -n "outbound_tx\|fn handle_agent_frame" apps/agent-relay/src/lib.rs`

Add three new arms:

```rust
AgentInboundFrame::Subscribe { topic } => {
    tracing::debug!(agent_id = %agent_id, topic = %topic, "subscribing to topic");
    let replay = state.bus.subscribe(agent_id, &topic);

    // Send replay messages to the agent.
    for envelope in replay {
        let frame = RelayOutboundFrame::TopicMessage {
            topic: envelope.topic,
            msg_type: envelope.msg_type,
            payload: envelope.payload,
            publisher_id: envelope.publisher_id,
            seq: envelope.seq,
        };
        if outbound_tx.send(frame).is_err() {
            tracing::warn!(agent_id, "failed to send replay — agent disconnected");
            break;
        }
    }

    // ACK the subscription.
    let _ = outbound_tx.send(RelayOutboundFrame::Ack {
        event: format!("subscribed:{topic}"),
    });
}

AgentInboundFrame::Unsubscribe { topic } => {
    tracing::debug!(agent_id = %agent_id, topic = %topic, "unsubscribing from topic");
    state.bus.unsubscribe(agent_id, &topic);
    let _ = outbound_tx.send(RelayOutboundFrame::Ack {
        event: format!("unsubscribed:{topic}"),
    });
}

AgentInboundFrame::Publish { topic, msg_type, payload } => {
    tracing::debug!(agent_id = %agent_id, topic = %topic, msg_type = %msg_type, "publishing to topic");
    let envelope = crate::protocol::TopicEnvelope::new(&topic, &msg_type, payload)
        .with_publisher(agent_id);
    let (seq, subscribers) = state.bus.publish(envelope.clone());

    // Fan out to subscribers (skip publisher).
    for sub_id in &subscribers {
        if sub_id == agent_id {
            continue;
        }
        let frame = RelayOutboundFrame::TopicMessage {
            topic: envelope.topic.clone(),
            msg_type: envelope.msg_type.clone(),
            payload: envelope.payload.clone(),
            publisher_id: envelope.publisher_id.clone(),
            seq,
        };
        state.send_to_agent(sub_id, frame);
    }

    let _ = outbound_tx.send(RelayOutboundFrame::Ack {
        event: format!("published:{topic}:{seq}"),
    });
}
```

**Note on accessing agents**: The Publish handler needs to fan out to subscribers. The existing code pattern uses `state.inner.read()` to access registered agents. However, `RelayStateInner` should stay private. Instead, add a helper method to `RelayState` (see Step 3 below) to encapsulate this logic.

### Step 2: Clean up subscriptions on disconnect

In `handle_agent_socket()`, find the cleanup section after the read loop exits (where `state.unregister_agent()` is called). Add:

```rust
// Before or after unregister_agent:
state.bus.unsubscribe_all(&agent_id);
```

### Step 3: Add `send_to_agent` helper to RelayState

In `apps/agent-relay/src/state.rs`, add this public method to the `RelayState` impl block:

```rust
/// Send a frame to a connected agent by ID. Returns false if agent not found/disconnected.
pub fn send_to_agent(&self, agent_id: &str, frame: RelayOutboundFrame) -> bool {
    let inner = self.inner.read();
    if let Some(handle) = inner.agents.get(agent_id) {
        handle.tx.send(frame).is_ok()
    } else {
        false
    }
}
```

This keeps agent registry encapsulation while providing the fan-out capability needed by the Publish handler.

## Verification

```bash
cargo build -p agent-relay
cargo test -p agent-relay
```

Integration test (add to `apps/agent-relay/tests/integration.rs`):

**CRITICAL**: The relay protocol uses **internally-tagged serde** (`#[serde(tag = "type",
rename_all = "snake_case")]`). All JSON frames use `{"type":"variant_name",...}` format,
NOT externally-tagged `{"VariantName":{...}}`.

```rust
#[tokio::test]
async fn topic_subscribe_and_receive() {
    let app = test_app().await;

    // Connect two agents
    let (mut agent1, _) = connect_agent(&app, "publisher-1").await;
    let (mut agent2, _) = connect_agent(&app, "subscriber-1").await;

    // Agent2 subscribes to "isfr:rates" — internally-tagged format
    agent2.send(json!({"type": "subscribe", "topic": "isfr:rates"})).await;
    let ack = agent2.recv().await;
    assert_eq!(ack["type"], "ack");
    assert_eq!(ack["event"], "subscribed:isfr:rates");

    // Agent1 publishes — internally-tagged format
    agent1.send(json!({
        "type": "publish",
        "topic": "isfr:rates",
        "msg_type": "rate_update",
        "payload": {"bps": 620}
    })).await;
    let pub_ack = agent1.recv().await;
    assert_eq!(pub_ack["type"], "ack");
    assert!(pub_ack["event"].as_str().unwrap().starts_with("published:isfr:rates:"));

    // Agent2 receives the message — internally-tagged format
    let msg = agent2.recv().await;
    assert_eq!(msg["type"], "topic_message");
    assert_eq!(msg["topic"], "isfr:rates");
    assert_eq!(msg["msg_type"], "rate_update");
    assert_eq!(msg["payload"]["bps"], 620);
    assert_eq!(msg["publisher_id"], "publisher-1");
}
```

## Dependencies

- A1 (frame types: Subscribe, Unsubscribe, Publish, TopicMessage)
- A2 (TopicBus on RelayState)

## What This Enables

- Agents can now subscribe/publish via WebSocket
- A4 (chain watcher) publishes events that agents receive
- A6 (relay client) can subscribe and process topic messages
