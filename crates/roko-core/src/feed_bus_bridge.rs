//! Generic bridge from runtime feed events to the canonical Bus.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;

use crate::error::Result;
use crate::feed_cell::FeedPulse;
use crate::{Body, Bus, Kind, Pulse, Topic};

/// Per-feed routing metrics.
#[derive(Debug, Default)]
pub struct FeedRouteStats {
    /// Pulses successfully published.
    pub pulses_routed: AtomicU64,
    /// Approximate serialized payload bytes published.
    pub bytes_routed: AtomicU64,
    /// Unix timestamp of the latest routed source pulse.
    pub last_routed_at_ms: AtomicU64,
}

/// Routes any FeedCell output into an existing Bus instance.
pub struct FeedBusBridge<B: Bus + 'static> {
    bus: Arc<B>,
    counters: Arc<RwLock<HashMap<String, Arc<FeedRouteStats>>>>,
}

impl<B: Bus + 'static> Clone for FeedBusBridge<B> {
    fn clone(&self) -> Self {
        Self {
            bus: Arc::clone(&self.bus),
            counters: Arc::clone(&self.counters),
        }
    }
}

impl<B: Bus + 'static> FeedBusBridge<B> {
    /// Attach to the caller-provided canonical Bus.
    #[must_use]
    pub fn new(bus: Arc<B>) -> Self {
        Self {
            bus,
            counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Route one feed event to `feed:{feed_id}:data`.
    pub fn route_pulse(&self, pulse: FeedPulse) -> Result<u64> {
        let stats = self
            .counters
            .write()
            .entry(pulse.feed_id.clone())
            .or_default()
            .clone();
        let bytes = pulse.payload_bytes();
        let routed = Pulse::builder(
            pulse.sequence,
            Topic::new(format!("feed:{}:data", pulse.feed_id)),
            Kind::Custom("feed.data".to_string()),
        )
        .body(Body::Json(pulse.payload))
        .created_at_ms(pulse.emitted_at_ms)
        .tag("feed_id", pulse.feed_id)
        .tag("source_topic", pulse.topic)
        .tag("source_sequence", pulse.sequence.to_string())
        .build();
        let sequence = self.bus.publish(routed)?;
        stats.pulses_routed.fetch_add(1, Ordering::Relaxed);
        stats.bytes_routed.fetch_add(bytes, Ordering::Relaxed);
        stats
            .last_routed_at_ms
            .store(pulse.emitted_at_ms.max(0) as u64, Ordering::Relaxed);
        Ok(sequence)
    }

    /// Spawn a bounded forwarding loop for a feed subscription.
    #[must_use]
    pub fn spawn(
        &self,
        mut receiver: tokio::sync::broadcast::Receiver<FeedPulse>,
    ) -> tokio::task::JoinHandle<()> {
        let bridge = self.clone();
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(pulse) => {
                        let _ = bridge.route_pulse(pulse);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        })
    }

    /// Return counters for a feed if it has routed at least one pulse.
    #[must_use]
    pub fn stats(&self, feed_id: &str) -> Option<Arc<FeedRouteStats>> {
        self.counters.read().get(feed_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryBus;
    use serde_json::json;

    #[test]
    fn routes_to_existing_bus_and_tracks_bytes() {
        let bridge = FeedBusBridge::new(Arc::new(MemoryBus::new(16)));
        bridge
            .route_pulse(FeedPulse::new("source", "raw", 4, json!({"n": 7})))
            .unwrap();
        let stats = bridge.stats("source").unwrap();
        assert_eq!(stats.pulses_routed.load(Ordering::Relaxed), 1);
        assert!(stats.bytes_routed.load(Ordering::Relaxed) > 0);
    }
}
