//! Provider-health feed sourced from canonical runtime observations.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::connector::ConnectorStatus;
use crate::error::Result;
use crate::feed::{FeedAccess, FeedKind};
use crate::feed_cell::{ConnectorOps, FeedCell, FeedCellConfig, FeedPulse, FeedTriggerOps};

/// One provider's latest canonical health observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderHealthSample {
    /// Stable provider name.
    pub provider: String,
    /// Whether the provider can currently accept work.
    pub healthy: bool,
    /// Optional observed latency.
    pub latency_ms: Option<u64>,
    /// Optional failure or degradation detail.
    pub error: Option<String>,
}

/// Callback used to read the provider registry without duplicating probes.
pub type ProviderHealthSnapshot = Arc<dyn Fn() -> Vec<ProviderHealthSample> + Send + Sync>;

/// Factory for the provider-health meta feed.
pub struct ProviderHealthFeed;

impl ProviderHealthFeed {
    /// Build a feed polling `snapshot` at `interval`.
    #[must_use]
    pub fn build(snapshot: ProviderHealthSnapshot, interval: Duration) -> Arc<FeedCell> {
        Arc::new(FeedCell::new(
            "provider-health-feed",
            "Provider health",
            (1, 0, 0),
            Arc::new(ProviderConnector {
                snapshot: Arc::clone(&snapshot),
                connected: RwLock::new(false),
            }),
            Arc::new(ProviderTrigger { snapshot, interval }),
            None,
            FeedCellConfig {
                kind: FeedKind::Meta,
                access: FeedAccess::Private,
                polling_interval: interval,
                topic: "provider.health".to_string(),
            },
        ))
    }
}

struct ProviderConnector {
    snapshot: ProviderHealthSnapshot,
    connected: RwLock<bool>,
}

impl std::fmt::Debug for ProviderConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderConnector").finish_non_exhaustive()
    }
}

#[async_trait]
impl ConnectorOps for ProviderConnector {
    async fn connect(&self) -> Result<()> {
        let _ = (self.snapshot)();
        *self.connected.write() = true;
        Ok(())
    }
    async fn query(&self, _query: &str) -> Result<Value> {
        Ok(json!((self.snapshot)()))
    }
    async fn disconnect(&self) -> Result<()> {
        *self.connected.write() = false;
        Ok(())
    }
    async fn health(&self) -> ConnectorStatus {
        if *self.connected.read() {
            ConnectorStatus::Connected
        } else {
            ConnectorStatus::Disconnected
        }
    }
}

struct ProviderTrigger {
    snapshot: ProviderHealthSnapshot,
    interval: Duration,
}

impl std::fmt::Debug for ProviderTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderTrigger").finish_non_exhaustive()
    }
}

#[async_trait]
impl FeedTriggerOps for ProviderTrigger {
    async fn listen(
        &self,
        feed_id: &str,
        output: broadcast::Sender<FeedPulse>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(self.interval.max(Duration::from_millis(1)));
        let mut sequence = 0_u64;
        loop {
            tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                _ = interval.tick() => {
                    let samples = (self.snapshot)();
                    let pulse = self.fire(feed_id, sequence, json!({"providers": samples}));
                    sequence = sequence.saturating_add(1);
                    let _ = output.send(pulse);
                }
            }
        }
    }
    fn filter(&self, _event: &Value) -> bool {
        true
    }
    fn debounce_ms(&self) -> u64 {
        self.interval.as_millis() as u64
    }
    fn fire(&self, feed_id: &str, sequence: u64, event: Value) -> FeedPulse {
        FeedPulse::new(feed_id, "provider.health", sequence, event)
    }
}
