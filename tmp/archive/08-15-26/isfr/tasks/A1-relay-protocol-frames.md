# A1: Add Pub/Sub Frame Types to Relay Protocol

## Context

The agent relay (`apps/agent-relay/`) currently handles agent presence and request/response message forwarding. It has NO topic-based pub/sub. This task adds the frame types needed for topic subscriptions and publishing.

## File to Modify

- `apps/agent-relay/src/protocol.rs`

## Pre-Check

```bash
# Verify current frame types
grep -n "enum AgentInboundFrame\|enum RelayOutboundFrame" apps/agent-relay/src/protocol.rs
# Confirm no existing Subscribe/Publish
grep -n "Subscribe\|Publish\|TopicEnvelope" apps/agent-relay/src/protocol.rs
```

## Implementation

### Step 1: Add variants to `AgentInboundFrame`

Add these variants to the existing enum (after `Ping`):

```rust
    /// Subscribe to a topic. Relay will forward matching TopicEnvelopes.
    Subscribe { topic: String },
    /// Unsubscribe from a previously subscribed topic.
    Unsubscribe { topic: String },
    /// Publish a message to a topic. Relay fans out to all subscribers.
    Publish { topic: String, msg_type: String, payload: serde_json::Value },
```

### Step 2: Add variant to `RelayOutboundFrame`

Add this variant to the existing enum (after `Pong`):

```rust
    /// A message published to a topic this agent is subscribed to.
    TopicMessage {
        topic: String,
        msg_type: String,
        payload: serde_json::Value,
        publisher_id: Option<String>,
        seq: u64,
    },
```

### Step 3: Add TopicEnvelope struct (after the enums)

```rust
/// Internal representation of a published topic message.
/// Used within the relay bus; serialized as TopicMessage when sent to agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicEnvelope {
    pub topic: String,
    pub msg_type: String,
    pub payload: serde_json::Value,
    pub publisher_id: Option<String>,
    pub seq: u64,
    pub timestamp_ms: i64,
}

impl TopicEnvelope {
    pub fn new(topic: impl Into<String>, msg_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            topic: topic.into(),
            msg_type: msg_type.into(),
            payload,
            publisher_id: None,
            seq: 0,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn with_publisher(mut self, id: impl Into<String>) -> Self {
        self.publisher_id = Some(id.into());
        self
    }

    pub fn with_seq(mut self, seq: u64) -> Self {
        self.seq = seq;
        self
    }
}
```

### Step 4: Add `chrono` dependency (if not already present)

Check `apps/agent-relay/Cargo.toml`. If `chrono` is not listed:

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
```

## Critical Notes

### Serde format

The relay uses **internally-tagged JSON** via `#[serde(tag = "type", rename_all = "snake_case")]`.
This means frames serialize as `{"type":"subscribe","topic":"..."}`, NOT as `{"Subscribe":{"topic":"..."}}`.
All downstream tasks (A3 integration tests, A6 client) MUST use this format.

### Eq derive on RelayOutboundFrame

`RelayOutboundFrame` currently derives `PartialEq, Eq`. Adding `payload: serde_json::Value`
to the `TopicMessage` variant is incompatible with `Eq` because `serde_json::Value` does not
implement `Eq` (floats). **You must remove the `Eq` derive** from `RelayOutboundFrame` when
adding the `TopicMessage` variant. `PartialEq` is fine to keep.

Check: `grep "derive.*Eq" apps/agent-relay/src/protocol.rs` — if `Eq` is present, remove it.
Any code comparing `RelayOutboundFrame` with `==` will still work (via `PartialEq`).

### Chrono dependency

The workspace already has `chrono` in `[workspace.dependencies]`. Use workspace syntax:
```toml
chrono = { workspace = true }
```
If chrono is NOT in workspace deps, add it with `version = "0.4", features = ["serde"]`.

### Insertion points

- `AgentInboundFrame`: Insert new variants **after** `Ping` (which is the last current variant).
- `RelayOutboundFrame`: Insert `TopicMessage` **after** `Pong`.
- `TopicEnvelope` struct: Place **after both enum definitions**, before any `impl` blocks.

## Verification

```bash
cargo build -p agent-relay
cargo test -p agent-relay
# Confirm new variants parse:
grep -n "Subscribe\|Publish\|TopicMessage\|TopicEnvelope" apps/agent-relay/src/protocol.rs
```

## Dependencies

- None (pure type additions, no logic changes)

## What This Enables

- A2 (Bus module) uses TopicEnvelope as its internal message type
- A3 (WebSocket wiring) handles Subscribe/Unsubscribe/Publish frames
- A4 (Chain watcher) publishes TopicEnvelopes for chain events
- A6 (Relay client) sends Subscribe/Publish frames
