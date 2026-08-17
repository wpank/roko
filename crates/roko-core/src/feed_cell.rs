//! Runtime feed Cell primitives.
//!
//! A feed is the kernel composition `Cell + Connect + Trigger + optional Store`.
//! This module keeps the external source, event trigger, and persistence backend
//! behind object-safe async contracts so the lifecycle manager can supervise
//! heterogeneous feeds without knowing their implementation.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::cell::{Cell, CellContext, CellVersion, ProtocolId};
use crate::connector::ConnectorStatus;
use crate::error::{Result, RokoError};
use crate::feed::{FeedAccess, FeedKind};
use crate::{Body, Engram, Kind};

/// Runtime lifecycle state for a feed Cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedStatus {
    /// Constructed but not connected.
    Idle,
    /// A connection attempt is in progress.
    Connecting,
    /// Connected and listening for source events.
    Connected,
    /// Still running, but the source is unhealthy or produced an error.
    Degraded,
    /// Explicitly stopped or disconnected after an error.
    Disconnected,
}

/// Runtime configuration shared by every feed Cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedCellConfig {
    /// Feed lineage classification.
    pub kind: FeedKind,
    /// Access policy applied by consumers.
    pub access: FeedAccess,
    /// Minimum source polling cadence.
    #[serde(with = "duration_millis")]
    pub polling_interval: Duration,
    /// Canonical output topic advertised by the source.
    pub topic: String,
}

/// A structured event emitted by a running feed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedPulse {
    /// Feed instance that emitted the event.
    pub feed_id: String,
    /// Source-level topic. The Bus bridge maps this to `feed:{id}:data`.
    pub topic: String,
    /// Monotonic sequence local to the feed instance.
    pub sequence: u64,
    /// Structured event payload.
    pub payload: Value,
    /// Unix timestamp in milliseconds.
    pub emitted_at_ms: i64,
    /// Optional source metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl FeedPulse {
    /// Construct a pulse for `feed_id` and `topic`.
    #[must_use]
    pub fn new(
        feed_id: impl Into<String>,
        topic: impl Into<String>,
        sequence: u64,
        payload: Value,
    ) -> Self {
        Self {
            feed_id: feed_id.into(),
            topic: topic.into(),
            sequence,
            payload,
            emitted_at_ms: now_ms(),
            metadata: BTreeMap::new(),
        }
    }

    /// Approximate serialized payload size used by bridge accounting.
    #[must_use]
    pub fn payload_bytes(&self) -> u64 {
        serde_json::to_vec(&self.payload).map_or(0, |bytes| bytes.len() as u64)
    }
}

/// Object-safe asynchronous connector owned by a feed Cell.
#[async_trait]
pub trait ConnectorOps: Send + Sync {
    /// Establish the external connection.
    async fn connect(&self) -> Result<()>;
    /// Execute an on-demand source query.
    async fn query(&self, query: &str) -> Result<Value>;
    /// Tear down the connection.
    async fn disconnect(&self) -> Result<()>;
    /// Return current connector health.
    async fn health(&self) -> ConnectorStatus;
}

/// Object-safe asynchronous trigger that produces feed events.
#[async_trait]
pub trait FeedTriggerOps: Send + Sync {
    /// Listen until cancelled, publishing every accepted event to `output`.
    async fn listen(
        &self,
        feed_id: &str,
        output: broadcast::Sender<FeedPulse>,
        cancel: CancellationToken,
    ) -> Result<()>;
    /// Test whether a raw source event should be emitted.
    fn filter(&self, event: &Value) -> bool;
    /// Minimum delay between accepted events.
    fn debounce_ms(&self) -> u64;
    /// Wrap a raw event as a typed feed pulse.
    fn fire(&self, feed_id: &str, sequence: u64, event: Value) -> FeedPulse;
}

/// Optional asynchronous persistence backend used by durable feeds.
#[async_trait]
pub trait StoreOps: Send + Sync {
    /// Persist a structured value.
    async fn put(&self, key: &str, value: Value) -> Result<()>;
    /// Retrieve a structured value.
    async fn get(&self, key: &str) -> Result<Option<Value>>;
}

/// Channel-backed trigger useful for adapters, tests, and webhook-like feeds.
pub struct FeedTrigger {
    topic: String,
    debounce_ms: u64,
    receiver: tokio::sync::Mutex<mpsc::Receiver<Value>>,
    filter_fn: Arc<dyn Fn(&Value) -> bool + Send + Sync>,
}

impl std::fmt::Debug for FeedTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeedTrigger")
            .field("topic", &self.topic)
            .field("debounce_ms", &self.debounce_ms)
            .finish_non_exhaustive()
    }
}

impl FeedTrigger {
    /// Create a bounded event trigger and its producer handle.
    #[must_use]
    pub fn channel(
        topic: impl Into<String>,
        debounce_ms: u64,
        capacity: usize,
        filter: impl Fn(&Value) -> bool + Send + Sync + 'static,
    ) -> (Self, mpsc::Sender<Value>) {
        let (sender, receiver) = mpsc::channel(capacity.max(1));
        (
            Self {
                topic: topic.into(),
                debounce_ms,
                receiver: tokio::sync::Mutex::new(receiver),
                filter_fn: Arc::new(filter),
            },
            sender,
        )
    }
}

#[async_trait]
impl FeedTriggerOps for FeedTrigger {
    async fn listen(
        &self,
        feed_id: &str,
        output: broadcast::Sender<FeedPulse>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let mut receiver = self.receiver.lock().await;
        let mut last_emit = None;
        let mut sequence = 0_u64;
        loop {
            tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                event = receiver.recv() => {
                    let Some(event) = event else { return Ok(()) };
                    if !self.filter(&event) {
                        continue;
                    }
                    let elapsed_ok = last_emit.is_none_or(|last: tokio::time::Instant| {
                        last.elapsed() >= Duration::from_millis(self.debounce_ms)
                    });
                    if !elapsed_ok {
                        continue;
                    }
                    let pulse = self.fire(feed_id, sequence, event);
                    sequence = sequence.saturating_add(1);
                    last_emit = Some(tokio::time::Instant::now());
                    let _ = output.send(pulse);
                }
            }
        }
    }

    fn filter(&self, event: &Value) -> bool {
        (self.filter_fn)(event)
    }

    fn debounce_ms(&self) -> u64 {
        self.debounce_ms
    }

    fn fire(&self, feed_id: &str, sequence: u64, event: Value) -> FeedPulse {
        FeedPulse::new(feed_id, &self.topic, sequence, event)
    }
}

/// Runtime feed instance supervised by [`RuntimeRegistry`](crate::feed_runtime::RuntimeRegistry).
pub struct FeedCell {
    cell_id: String,
    cell_name: String,
    version: CellVersion,
    connector: Arc<dyn ConnectorOps>,
    trigger: Arc<dyn FeedTriggerOps>,
    store: Option<Arc<dyn StoreOps>>,
    config: FeedCellConfig,
    status: Arc<RwLock<FeedStatus>>,
    output: broadcast::Sender<FeedPulse>,
    emitted: AtomicU64,
    last_emitted_at_ms: AtomicU64,
}

impl std::fmt::Debug for FeedCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeedCell")
            .field("cell_id", &self.cell_id)
            .field("cell_name", &self.cell_name)
            .field("version", &self.version)
            .field("config", &self.config)
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}

impl FeedCell {
    /// Construct a runtime feed Cell with bounded fan-out.
    #[must_use]
    pub fn new(
        cell_id: impl Into<String>,
        cell_name: impl Into<String>,
        version: CellVersion,
        connector: Arc<dyn ConnectorOps>,
        trigger: Arc<dyn FeedTriggerOps>,
        store: Option<Arc<dyn StoreOps>>,
        config: FeedCellConfig,
    ) -> Self {
        let (output, _) = broadcast::channel(256);
        Self {
            cell_id: cell_id.into(),
            cell_name: cell_name.into(),
            version,
            connector,
            trigger,
            store,
            config,
            status: Arc::new(RwLock::new(FeedStatus::Idle)),
            output,
            emitted: AtomicU64::new(0),
            last_emitted_at_ms: AtomicU64::new(0),
        }
    }

    /// Return the feed configuration.
    #[must_use]
    pub fn config(&self) -> &FeedCellConfig {
        &self.config
    }

    /// Return the latest lifecycle state.
    #[must_use]
    pub fn status(&self) -> FeedStatus {
        *self.status.read()
    }

    /// Subscribe to emitted feed pulses.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<FeedPulse> {
        self.output.subscribe()
    }

    /// Number of pulses observed by this Cell's output monitor.
    #[must_use]
    pub fn emitted_count(&self) -> u64 {
        self.emitted.load(Ordering::Relaxed)
    }

    /// Unix timestamp in milliseconds of the most recently emitted pulse.
    #[must_use]
    pub fn last_emitted_at_ms(&self) -> Option<u64> {
        match self.last_emitted_at_ms.load(Ordering::Relaxed) {
            0 => None,
            timestamp => Some(timestamp),
        }
    }

    /// Connect to the external source.
    pub async fn connect(&self) -> Result<()> {
        *self.status.write() = FeedStatus::Connecting;
        match self.connector.connect().await {
            Ok(()) => {
                *self.status.write() = FeedStatus::Connected;
                Ok(())
            }
            Err(error) => {
                *self.status.write() = FeedStatus::Degraded;
                Err(error)
            }
        }
    }

    /// Execute an on-demand connector query.
    pub async fn query(&self, query: &str) -> Result<Value> {
        self.connector.query(query).await
    }

    /// Listen for source events until cancellation or source failure.
    pub async fn listen(&self, cancel: CancellationToken) -> Result<()> {
        // Give the trigger a private channel so accounting and optional
        // persistence happen before events reach public subscribers.
        let (trigger_output, _) = broadcast::channel(256);
        let mut trigger_events = trigger_output.subscribe();
        let listen = self
            .trigger
            .listen(&self.cell_id, trigger_output, cancel.clone());
        tokio::pin!(listen);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                result = &mut listen => return result,
                event = trigger_events.recv() => match event {
                    Ok(pulse) => {
                        self.emitted.fetch_add(1, Ordering::Relaxed);
                        self.last_emitted_at_ms
                            .store(pulse.emitted_at_ms.max(0) as u64, Ordering::Relaxed);
                        if let Some(store) = &self.store {
                            store
                                .put(
                                    &format!("{}:{}", pulse.feed_id, pulse.sequence),
                                    pulse.payload.clone(),
                                )
                                .await?;
                        }
                        let _ = self.output.send(pulse);
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                },
            }
        }
    }

    /// Return connector health and synchronize the feed lifecycle state.
    pub async fn health(&self) -> ConnectorStatus {
        let status = self.connector.health().await;
        *self.status.write() = match status {
            ConnectorStatus::Connected => FeedStatus::Connected,
            ConnectorStatus::Degraded => FeedStatus::Degraded,
            ConnectorStatus::Disconnected => FeedStatus::Disconnected,
        };
        status
    }

    /// Disconnect from the source.
    pub async fn disconnect(&self) -> Result<()> {
        let result = self.connector.disconnect().await;
        *self.status.write() = FeedStatus::Disconnected;
        result
    }
}

#[async_trait]
impl Cell for FeedCell {
    fn cell_id(&self) -> &str {
        &self.cell_id
    }

    fn cell_name(&self) -> &str {
        &self.cell_name
    }

    fn cell_version(&self) -> CellVersion {
        self.version
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        let mut protocols = vec![ProtocolId::Connect, ProtocolId::Trigger];
        if self.store.is_some() {
            protocols.push(ProtocolId::Store);
        }
        protocols
    }

    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
        let query = input
            .first()
            .and_then(|signal| signal.body.as_text().ok())
            .unwrap_or_default();
        let value = self.query(query).await?;
        let signal = Engram::builder(Kind::Custom("feed.query.result".to_string()))
            .body(Body::Json(value))
            .tag("feed_id", self.cell_id.clone())
            .build();
        Ok(vec![signal])
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

mod duration_millis {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        u64::deserialize(deserializer).map(Duration::from_millis)
    }
}

/// Connector useful for in-memory/adaptor feeds with no external handshake.
#[derive(Debug, Default)]
pub struct NoopConnector {
    connected: RwLock<bool>,
}

#[async_trait]
impl ConnectorOps for NoopConnector {
    async fn connect(&self) -> Result<()> {
        *self.connected.write() = true;
        Ok(())
    }

    async fn query(&self, _query: &str) -> Result<Value> {
        Ok(Value::Null)
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

/// Bounded in-memory Store adapter used by tests and transient local feeds.
#[derive(Debug, Default)]
pub struct MemoryFeedStore {
    values: RwLock<std::collections::HashMap<String, Value>>,
}

#[async_trait]
impl StoreOps for MemoryFeedStore {
    async fn put(&self, key: &str, value: Value) -> Result<()> {
        self.values.write().insert(key.to_string(), value);
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Value>> {
        Ok(self.values.read().get(key).cloned())
    }
}

/// Trigger that always returns a clear implementation error.
///
/// Useful when a descriptor is registered without a runnable event source.
#[derive(Debug)]
pub struct UnavailableTrigger {
    /// Human-readable reason the feed cannot start.
    pub reason: String,
}

#[async_trait]
impl FeedTriggerOps for UnavailableTrigger {
    async fn listen(
        &self,
        _feed_id: &str,
        _output: broadcast::Sender<FeedPulse>,
        _cancel: CancellationToken,
    ) -> Result<()> {
        Err(RokoError::Invalid(self.reason.clone()))
    }

    fn filter(&self, _event: &Value) -> bool {
        false
    }

    fn debounce_ms(&self) -> u64 {
        0
    }

    fn fire(&self, feed_id: &str, sequence: u64, event: Value) -> FeedPulse {
        FeedPulse::new(feed_id, "feed.unavailable", sequence, event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn channel_trigger_filters_debounces_and_emits() {
        let (trigger, sender) = FeedTrigger::channel("test.events", 0, 4, |event| {
            event.get("accept").and_then(Value::as_bool) == Some(true)
        });
        let cell = Arc::new(FeedCell::new(
            "test-feed",
            "Test Feed",
            (1, 0, 0),
            Arc::new(NoopConnector::default()),
            Arc::new(trigger),
            Some(Arc::new(MemoryFeedStore::default())),
            FeedCellConfig {
                kind: FeedKind::Raw,
                access: FeedAccess::Public,
                polling_interval: Duration::from_millis(1),
                topic: "test.events".to_string(),
            },
        ));
        cell.connect().await.unwrap();
        let mut pulses = cell.subscribe();
        let cancel = CancellationToken::new();
        let task = {
            let cell = Arc::clone(&cell);
            let cancel = cancel.clone();
            tokio::spawn(async move { cell.listen(cancel).await })
        };
        sender
            .send(serde_json::json!({"accept": false}))
            .await
            .unwrap();
        sender
            .send(serde_json::json!({"accept": true, "value": 7}))
            .await
            .unwrap();
        let pulse = tokio::time::timeout(Duration::from_secs(1), pulses.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(pulse.feed_id, "test-feed");
        assert_eq!(pulse.payload["value"], 7);
        cancel.cancel();
        task.await.unwrap().unwrap();
    }

    #[test]
    fn feed_cell_declares_kernel_protocols() {
        let (trigger, _sender) = FeedTrigger::channel("test", 0, 1, |_| true);
        let cell = FeedCell::new(
            "id",
            "name",
            (1, 0, 0),
            Arc::new(NoopConnector::default()),
            Arc::new(trigger),
            Some(Arc::new(MemoryFeedStore::default())),
            FeedCellConfig {
                kind: FeedKind::Raw,
                access: FeedAccess::Public,
                polling_interval: Duration::from_secs(1),
                topic: "test".to_string(),
            },
        );
        assert_eq!(
            cell.protocols(),
            vec![ProtocolId::Connect, ProtocolId::Trigger, ProtocolId::Store]
        );
    }
}
