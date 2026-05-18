# A2: Implement Topic Pub/Sub Bus Module

## Context

The relay needs a topic-based pub/sub system. Agents subscribe to topics and receive messages published to those topics. This is a NEW module that works alongside the existing `RelayState`.

## File to Create

- `apps/agent-relay/src/bus.rs` (NEW)

## File to Modify

- `apps/agent-relay/src/lib.rs` — add `mod bus; pub use bus::TopicBus;`

## Design

The TopicBus:
- Tracks which agent_ids are subscribed to which topics
- Stores a bounded ring buffer of recent messages per topic (for replay)
- When a message is published, iterates subscribers and sends via their outbound channel
- Does NOT own WebSocket connections — receives a reference to agent senders

## Implementation

### Step 1: Create `apps/agent-relay/src/bus.rs`

```rust
//! Topic-based pub/sub bus for the agent relay.
//!
//! Agents subscribe to topics via WebSocket frames. Published messages
//! are fanned out to all subscribers of the matching topic.

use crate::protocol::TopicEnvelope;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// Configuration for the topic bus.
pub struct TopicBusConfig {
    /// Max messages retained per topic for replay.
    pub ring_capacity: usize,
}

impl Default for TopicBusConfig {
    fn default() -> Self {
        Self { ring_capacity: 128 }
    }
}

/// Topic-based pub/sub bus.
///
/// Thread-safe. Designed to be wrapped in Arc and shared with handler tasks.
pub struct TopicBus {
    /// topic → set of subscribed agent_ids.
    subscriptions: RwLock<HashMap<String, Vec<String>>>,
    /// topic → ring buffer of recent envelopes.
    rings: RwLock<HashMap<String, VecDeque<TopicEnvelope>>>,
    /// Monotonically increasing sequence counter.
    seq: AtomicU64,
    /// Max messages per topic ring.
    ring_capacity: usize,
}

impl TopicBus {
    pub fn new(config: TopicBusConfig) -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
            rings: RwLock::new(HashMap::new()),
            seq: AtomicU64::new(1),
            ring_capacity: config.ring_capacity,
        }
    }

    /// Subscribe an agent to a topic. Returns recent messages for replay.
    pub fn subscribe(&self, agent_id: &str, topic: &str) -> Vec<TopicEnvelope> {
        // Add to subscription set.
        {
            let mut subs = self.subscriptions.write();
            let agents = subs.entry(topic.to_string()).or_default();
            if !agents.contains(&agent_id.to_string()) {
                agents.push(agent_id.to_string());
            }
        }

        // Return ring contents for replay.
        let rings = self.rings.read();
        rings
            .get(topic)
            .map(|ring| ring.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Unsubscribe an agent from a topic.
    pub fn unsubscribe(&self, agent_id: &str, topic: &str) {
        let mut subs = self.subscriptions.write();
        if let Some(agents) = subs.get_mut(topic) {
            agents.retain(|id| id != agent_id);
            if agents.is_empty() {
                subs.remove(topic);
            }
        }
    }

    /// Remove all subscriptions for an agent (called on disconnect).
    pub fn unsubscribe_all(&self, agent_id: &str) {
        let mut subs = self.subscriptions.write();
        subs.retain(|_topic, agents| {
            agents.retain(|id| id != agent_id);
            !agents.is_empty()
        });
    }

    /// Publish a message to a topic.
    ///
    /// Returns (seq, list of subscriber agent_ids that should receive it).
    /// The caller is responsible for actually sending the frame to each agent.
    pub fn publish(&self, mut envelope: TopicEnvelope) -> (u64, Vec<String>) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        envelope.seq = seq;

        // Store in ring buffer.
        {
            let mut rings = self.rings.write();
            let ring = rings
                .entry(envelope.topic.clone())
                .or_insert_with(|| VecDeque::with_capacity(self.ring_capacity));
            if ring.len() >= self.ring_capacity {
                ring.pop_front();
            }
            ring.push_back(envelope.clone());
        }

        // Get subscribers.
        let subscribers = {
            let subs = self.subscriptions.read();
            subs.get(&envelope.topic)
                .cloned()
                .unwrap_or_default()
        };

        (seq, subscribers)
    }

    /// Get all topics with their subscriber counts.
    pub fn topic_stats(&self) -> Vec<(String, usize)> {
        let subs = self.subscriptions.read();
        subs.iter()
            .map(|(topic, agents)| (topic.clone(), agents.len()))
            .collect()
    }

    /// Get subscriber count for a specific topic.
    pub fn subscriber_count(&self, topic: &str) -> usize {
        let subs = self.subscriptions.read();
        subs.get(topic).map(|a| a.len()).unwrap_or(0)
    }

    /// Current sequence number (for diagnostics).
    pub fn current_seq(&self) -> u64 {
        self.seq.load(Ordering::Relaxed)
    }
}
```

### Step 2: Add module to `apps/agent-relay/src/lib.rs`

After the existing module declarations, add:

```rust
mod bus;
pub use bus::{TopicBus, TopicBusConfig};
```

### Step 3: Add TopicBus to shared state

In `apps/agent-relay/src/lib.rs`, the router setup creates `Arc<RelayState>`. Add TopicBus alongside it.

Find where `RelayState::new()` is called and the shared state is built. Add:

```rust
// In the router setup function (build_router or similar):
let bus = Arc::new(TopicBus::new(TopicBusConfig::default()));
```

Then pass both `relay_state` and `bus` to handlers that need them. The cleanest approach: add `TopicBus` as a field on `RelayState`:

In `apps/agent-relay/src/state.rs`, add:
```rust
use crate::bus::TopicBus;

pub struct RelayState {
    inner: RwLock<RelayStateInner>,
    events_tx: broadcast::Sender<RelayEvent>,
    pub bus: TopicBus,  // NEW
}
```

And update `RelayState::new()`:
```rust
pub fn new() -> Self {
    let (events_tx, _) = broadcast::channel(256);
    Self {
        inner: RwLock::new(RelayStateInner::default()),
        events_tx,
        bus: TopicBus::new(TopicBusConfig::default()),  // NEW
    }
}
```

## Verification

```bash
cargo build -p agent-relay
cargo test -p agent-relay
```

Add a unit test in `bus.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_and_publish() {
        let bus = TopicBus::new(TopicBusConfig::default());

        // Subscribe agent
        let replay = bus.subscribe("agent-1", "isfr:rates");
        assert!(replay.is_empty());

        // Publish
        let envelope = TopicEnvelope::new("isfr:rates", "rate_update", serde_json::json!({"bps": 620}));
        let (seq, subscribers) = bus.publish(envelope);
        assert_eq!(seq, 1);
        assert_eq!(subscribers, vec!["agent-1"]);

        // New subscriber gets replay
        let replay = bus.subscribe("agent-2", "isfr:rates");
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].seq, 1);
    }

    #[test]
    fn unsubscribe_removes_from_fanout() {
        let bus = TopicBus::new(TopicBusConfig::default());
        bus.subscribe("agent-1", "chain:31337");
        bus.unsubscribe("agent-1", "chain:31337");

        let envelope = TopicEnvelope::new("chain:31337", "block", serde_json::json!({}));
        let (_seq, subscribers) = bus.publish(envelope);
        assert!(subscribers.is_empty());
    }

    #[test]
    fn ring_bounded() {
        let bus = TopicBus::new(TopicBusConfig { ring_capacity: 2 });
        for i in 0..5 {
            let env = TopicEnvelope::new("t", "x", serde_json::json!(i));
            bus.publish(env);
        }
        let replay = bus.subscribe("a", "t");
        assert_eq!(replay.len(), 2);
        assert_eq!(replay[0].seq, 4); // Oldest retained
        assert_eq!(replay[1].seq, 5);
    }

    #[test]
    fn unsubscribe_all_on_disconnect() {
        let bus = TopicBus::new(TopicBusConfig::default());
        bus.subscribe("agent-1", "topic-a");
        bus.subscribe("agent-1", "topic-b");
        bus.unsubscribe_all("agent-1");
        assert_eq!(bus.subscriber_count("topic-a"), 0);
        assert_eq!(bus.subscriber_count("topic-b"), 0);
    }
}
```

## Cargo.toml Verification

The bus module uses `parking_lot::RwLock`. Verify `parking_lot` is in agent-relay's deps:
```bash
grep "parking_lot" apps/agent-relay/Cargo.toml
```
If missing, add: `parking_lot = "0.12"`

Also uses `tokio::sync::mpsc` (already present since agent-relay uses tokio).

## Dependencies

- A1 (TopicEnvelope type must exist in protocol.rs)

## What This Enables

- A3 wires Subscribe/Unsubscribe/Publish frames to this bus
- A4 uses `bus.publish()` to emit chain events
- A5 exposes `bus.topic_stats()` via HTTP
