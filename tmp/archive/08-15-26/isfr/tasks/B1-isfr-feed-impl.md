# B1: Implement ISFRFeed (Relay-Backed Pulse Producer)

## Context

The ISFR system needs a component that subscribes to relay topics and bridges received messages into the local PulseBus as Pulses. This enables agents running locally to react to ISFR rate updates published by the keeper via the relay.

**Key insight**: There is NO `Feed` trait in roko-core. The `feed.rs` module contains a `FeedRegistry` (metadata registry for feed discovery). The actual data transport is the **Bus** (PulseBus/BroadcastBus). So ISFRFeed is really a **relay-to-bus bridge** — it subscribes to relay topics and publishes Pulses to the local bus.

## Relevant Types

**Bus trait** (`crates/roko-core/src/traits.rs`):
```rust
pub trait Bus: Send + Sync {
    type Receiver: Send;
    fn publish(&self, pulse: Pulse) -> Result<u64>;
    fn subscribe(&self, filter: TopicFilter) -> Result<Self::Receiver>;
}
```

**BusErased** (`crates/roko-core/src/bus_backends.rs`):
```rust
pub trait BusErased: Send + Sync {
    fn publish_erased(&self, pulse: Pulse) -> Result<u64>;
}
```

**PulseBuilder** (`crates/roko-core/src/pulse.rs`):
```rust
impl PulseBuilder {
    pub fn new(seq: u64, topic: Topic, kind: Kind) -> Self;
    pub fn body(mut self, body: Body) -> Self;
    pub const fn created_at_ms(mut self, t: i64) -> Self;
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn build(self) -> Pulse;
}
// Also: Pulse::builder(seq, topic, kind) -> PulseBuilder
```

**TopicHandler** (from A6, in `crates/roko-agent-server/src/features/relay_client.rs`):
```rust
#[async_trait]
pub trait TopicHandler: Send + Sync + 'static {
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

## Design

ISFRFeed implements `TopicHandler` (from A6). When wired to a RelayHandle, it receives topic messages from the relay and publishes them as Pulses to the local bus. This is a simple adapter pattern.

## File to Create

- `crates/roko-core/src/isfr_feed.rs` (NEW)

## File to Modify

- `crates/roko-core/src/lib.rs` — add `pub mod isfr_feed;`

## Implementation

### Step 1: Create `crates/roko-core/src/isfr_feed.rs`

```rust
//! ISFRFeed — relay-to-bus bridge for ISFR rate data.
//!
//! Implements the TopicHandler interface (from A6) to receive messages from the
//! relay and republishes them as Pulses on the local PulseBus.
//!
//! Usage:
//!   let bus: Arc<dyn BusErased> = Arc::new(PulseBus::new(1024));
//!   let feed = ISFRFeed::new(bus);
//!   // Pass Arc::new(feed) as the TopicHandler when connecting relay client.
//!   // Then subscribe to relay topics: "isfr:rates", "isfr:epochs", "chain:31337"

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::bus_backends::BusErased;
use crate::pulse::{Pulse, PulseBuilder, Topic};
use crate::{Body, Kind};

/// Relay-to-bus bridge for ISFR data.
///
/// Receives topic messages from the relay (via TopicHandler interface)
/// and publishes them as Pulses on the local bus.
pub struct ISFRFeed {
    bus: Arc<dyn BusErased>,
    seq: AtomicU64,
    messages_received: AtomicU64,
    pulses_published: AtomicU64,
}

impl ISFRFeed {
    /// Create a new ISFRFeed that publishes to the given bus.
    pub fn new(bus: Arc<dyn BusErased>) -> Self {
        Self {
            bus,
            seq: AtomicU64::new(1),
            messages_received: AtomicU64::new(0),
            pulses_published: AtomicU64::new(0),
        }
    }

    /// Handle a topic message from the relay.
    /// Call this from a TopicHandler implementation.
    pub fn handle_message(
        &self,
        topic: &str,
        msg_type: &str,
        payload: serde_json::Value,
        publisher_id: Option<&str>,
        _relay_seq: u64,
    ) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);

        // Map relay topic to local bus topic.
        let bus_topic = self.map_topic(topic);

        // Determine the Kind from msg_type.
        let kind = Kind::Custom(msg_type.to_string());

        // Build the pulse.
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let mut builder = Pulse::builder(seq, Topic::new(bus_topic), kind)
            .body(Body::Json(payload));

        // Add metadata tags.
        builder = builder
            .tag("source", "relay")
            .tag("relay_topic", topic)
            .tag("msg_type", msg_type);

        if let Some(pub_id) = publisher_id {
            builder = builder.tag("publisher", pub_id);
        }

        let pulse = builder.build();

        // Publish to local bus.
        match self.bus.publish_erased(pulse) {
            Ok(_) => {
                self.pulses_published.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                tracing::warn!(error = %e, topic, "ISFRFeed: failed to publish pulse to bus");
            }
        }
    }

    /// Map relay topic names to local bus topic names.
    /// Returns &'static str for all known patterns — no allocations.
    fn map_topic(&self, relay_topic: &str) -> &'static str {
        match relay_topic {
            "isfr:rates" => "isfr.rates",
            "isfr:epochs" => "isfr.epochs",
            _ if relay_topic.starts_with("chain:") => "isfr.chain_event",
            _ => "isfr.unknown",
        }
    }

    /// Number of messages received from relay.
    pub fn messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    /// Number of pulses successfully published to bus.
    pub fn pulses_published(&self) -> u64 {
        self.pulses_published.load(Ordering::Relaxed)
    }

    /// Topics this feed should subscribe to on the relay.
    pub fn relay_topics(chain_id: &str) -> Vec<String> {
        vec![
            "isfr:rates".to_string(),
            "isfr:epochs".to_string(),
            format!("chain:{chain_id}"),
        ]
    }
}
```

### Step 2: Add module to lib.rs

In `crates/roko-core/src/lib.rs`:
```rust
pub mod isfr_feed;
```

### Step 3: Integrate with RelayHandle (from A6)

The caller (e.g., `roko isfr start` from E2) wires ISFRFeed as the TopicHandler:

```rust
// In the CLI or serve startup:
use roko_core::isfr_feed::ISFRFeed;

let bus: Arc<dyn BusErased> = Arc::new(PulseBus::new(1024));
let feed = Arc::new(ISFRFeed::new(bus.clone()));

// Create a TopicHandler adapter (implements the trait from A6):
struct FeedAdapter(Arc<ISFRFeed>);

#[async_trait]
impl TopicHandler for FeedAdapter {
    async fn on_topic_message(
        &self,
        topic: &str,
        msg_type: &str,
        payload: serde_json::Value,
        publisher_id: Option<&str>,
        seq: u64,
    ) {
        self.0.handle_message(topic, msg_type, payload, publisher_id, seq);
    }
}

// Pass to relay client connect:
let handler = Arc::new(FeedAdapter(feed.clone()));
let handle = relay_client::connect(config, state, card, Some(handler)).await?;

// Subscribe to ISFR topics:
for topic in ISFRFeed::relay_topics("31337") {
    handle.subscribe(topic)?;
}
```

**Note**: The `TopicHandler` trait lives in `roko-agent-server`, not `roko-core`. The adapter pattern above avoids coupling roko-core to roko-agent-server. The adapter is defined at the integration point (CLI or serve).

## Critical Notes

### BusErased implementation

The tests use `BroadcastBus::new()` — verify this type implements `BusErased`:
```bash
grep -n "impl BusErased\|struct BroadcastBus" crates/roko-core/src/bus_backends.rs
```

If `BroadcastBus` doesn't implement `BusErased`, use whatever type does, or use `PulseBus`:
```bash
grep -n "impl BusErased" crates/roko-core/src/ -r
```

### serde_json dependency

`isfr_feed.rs` uses `serde_json::Value`. Verify `serde_json` is in roko-core's Cargo.toml.
It almost certainly is, but confirm.

## Verification

```bash
cargo build -p roko-core
cargo test -p roko-core
```

Test:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus_backends::BroadcastBus;

    #[test]
    fn feed_publishes_to_bus() {
        let bus = Arc::new(BroadcastBus::new());
        let feed = ISFRFeed::new(bus.clone());

        feed.handle_message(
            "isfr:rates",
            "composite_rate",
            serde_json::json!({"bps": 620}),
            Some("keeper-1"),
            42,
        );

        assert_eq!(feed.messages_received(), 1);
        assert_eq!(feed.pulses_published(), 1);
    }

    #[test]
    fn feed_maps_topics() {
        let bus = Arc::new(BroadcastBus::new());
        let feed = ISFRFeed::new(bus);

        // NOTE: map_topic is private. These tests work because they're in the same module.
        // If you move tests to a separate file, test via handle_message() output instead.
        assert_eq!(feed.map_topic("isfr:rates"), "isfr.rates");
        assert_eq!(feed.map_topic("isfr:epochs"), "isfr.epochs");
        assert_eq!(feed.map_topic("chain:31337"), "isfr.chain_event");
        assert_eq!(feed.map_topic("random"), "isfr.unknown");
    }

    #[test]
    fn relay_topics_includes_chain() {
        let topics = ISFRFeed::relay_topics("31337");
        assert_eq!(topics.len(), 3);
        assert!(topics.contains(&"chain:31337".to_string()));
    }
}
```

## Dependencies

- A6 (RelayHandle + TopicHandler trait for actual relay connection)
- roko-core Bus types (already exist)

## What This Enables

- Local agents can subscribe to `isfr.rates` on the PulseBus to react to rate updates
- The keeper publishes to the relay, ISFRFeed bridges to bus, agents consume from bus
- Clean separation: relay transport (A6) → feed bridge (B1) → local bus → agent subscribers
