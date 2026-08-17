# A5: Add Feed Metadata HTTP Endpoints

## Context

The relay should expose HTTP endpoints for discovering what topics exist, their subscriber counts, and recent messages. This enables monitoring dashboards and the `roko isfr status` command.

## File to Modify

- `apps/agent-relay/src/lib.rs` — add 3 new HTTP routes

## Implementation

### Step 1: Add routes to the router

In `apps/agent-relay/src/lib.rs`, find where routes are defined (the `Router::new()` chain). Add:

```rust
.route("/relay/topics", get(list_topics))
.route("/relay/topics/:topic/messages", get(topic_messages))
.route("/relay/topics/:topic/subscribers", get(topic_subscribers))
```

### Step 2: Implement handlers

Add these handler functions in `apps/agent-relay/src/lib.rs` (or extract to a `handlers.rs` if the file is getting large):

```rust
/// GET /relay/topics — list all topics with subscriber counts.
async fn list_topics(
    State(state): State<Arc<RelayState>>,
) -> axum::Json<serde_json::Value> {
    let stats = state.bus.topic_stats();
    let topics: Vec<serde_json::Value> = stats
        .iter()
        .map(|(topic, count)| {
            serde_json::json!({
                "topic": topic,
                "subscribers": count,
            })
        })
        .collect();
    axum::Json(serde_json::json!({ "topics": topics }))
}

/// GET /relay/topics/:topic/messages — get recent messages from ring buffer.
async fn topic_messages(
    State(state): State<Arc<RelayState>>,
    Path(topic): Path<String>,
    Query(params): Query<TopicMessagesQuery>,
) -> axum::Json<serde_json::Value> {
    // Subscribe temporarily to get replay, then immediately unsubscribe.
    let dummy_id = format!("__query_{}", uuid::Uuid::new_v4());
    let messages = state.bus.subscribe(&dummy_id, &topic);
    state.bus.unsubscribe(&dummy_id, &topic);

    let limit = params.limit.unwrap_or(50).min(200);
    let messages: Vec<serde_json::Value> = messages
        .into_iter()
        .rev()
        .take(limit)
        .map(|env| {
            serde_json::json!({
                "seq": env.seq,
                "topic": env.topic,
                "msg_type": env.msg_type,
                "payload": env.payload,
                "publisher_id": env.publisher_id,
                "timestamp_ms": env.timestamp_ms,
            })
        })
        .collect();
    axum::Json(serde_json::json!({ "messages": messages }))
}

/// GET /relay/topics/:topic/subscribers — list agents subscribed to a topic.
async fn topic_subscribers(
    State(state): State<Arc<RelayState>>,
    Path(topic): Path<String>,
) -> axum::Json<serde_json::Value> {
    let count = state.bus.subscriber_count(&topic);
    axum::Json(serde_json::json!({
        "topic": topic,
        "subscriber_count": count,
    }))
}

#[derive(Debug, serde::Deserialize)]
struct TopicMessagesQuery {
    limit: Option<usize>,
}
```

### Step 3: Add imports

At the top of `lib.rs`, ensure these are imported:

```rust
use axum::extract::{Path, Query};
```

## Verification

```bash
cargo build -p agent-relay
cargo test -p agent-relay
# Manual test after starting relay:
# curl http://localhost:9011/relay/topics
# curl http://localhost:9011/relay/topics/isfr:rates/messages
```

## Dependencies

- A2 (TopicBus with `topic_stats()`, `subscribe()`, `subscriber_count()`)
- A3 (WebSocket wiring must exist for agents to actually subscribe)

## Implementation Notes

### Ring buffer access pattern

The `topic_messages` endpoint uses a subscribe-then-unsubscribe trick to get the ring buffer
contents. A better approach: add a `peek_ring(topic: &str) -> Vec<TopicEnvelope>` method to
TopicBus (in A2) that reads the ring without modifying subscriptions:

```rust
/// Read ring buffer contents without subscribing.
pub fn peek_ring(&self, topic: &str) -> Vec<TopicEnvelope> {
    let rings = self.rings.read();
    rings.get(topic).map(|ring| ring.iter().cloned().collect()).unwrap_or_default()
}
```

If you add this to TopicBus in A2, use it here instead of the subscribe/unsubscribe trick.

### uuid dependency

The `topic_messages` handler uses `uuid::Uuid::new_v4()`. Check if `uuid` is already in
`apps/agent-relay/Cargo.toml`. If not, add it or use a simpler dummy ID like a timestamp.
