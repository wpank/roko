//! Supervised lifecycle for runtime feed Cells.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::cell::Cell;
use crate::error::{Result, RokoError};
use crate::feed::{FeedInfo, FeedRuntimeStatus};
use crate::feed_cell::{FeedCell, FeedStatus};

/// Restart policy for a supervised feed.
#[derive(Debug, Clone, Copy)]
pub struct ReconnectPolicy {
    /// First delay after a failed connection/listen cycle.
    pub initial: Duration,
    /// Longest delay between retries.
    pub maximum: Duration,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial: Duration::from_secs(1),
            maximum: Duration::from_secs(60),
        }
    }
}

/// A registered feed factory and its public descriptor.
#[derive(Clone)]
struct FeedFactory {
    info: FeedInfo,
    build: Arc<dyn Fn() -> Arc<FeedCell> + Send + Sync>,
}

/// Cloneable handle for a supervised feed task.
#[derive(Clone)]
pub struct FeedHandle {
    cell: Arc<FeedCell>,
    cancel: CancellationToken,
    task: Arc<tokio::sync::Mutex<Option<JoinHandle<()>>>>,
    started_at_ms: u64,
    error_count: Arc<AtomicU64>,
    last_error: Arc<RwLock<Option<String>>>,
}

impl std::fmt::Debug for FeedHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeedHandle")
            .field("cell", &self.cell)
            .field("started_at_ms", &self.started_at_ms)
            .field("error_count", &self.error_count.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl FeedHandle {
    /// Runtime Cell managed by this handle.
    #[must_use]
    pub fn cell(&self) -> &Arc<FeedCell> {
        &self.cell
    }

    /// Unix timestamp in milliseconds when supervision began.
    #[must_use]
    pub const fn started_at_ms(&self) -> u64 {
        self.started_at_ms
    }

    /// Number of failed connection/listen cycles.
    #[must_use]
    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    /// Last supervisor error, if any.
    #[must_use]
    pub fn last_error(&self) -> Option<String> {
        self.last_error.read().clone()
    }

    /// Cancel and await the background supervisor.
    pub async fn stop(&self) -> Result<()> {
        self.cancel.cancel();
        if let Some(task) = self.task.lock().await.take() {
            task.await
                .map_err(|error| RokoError::invalid(format!("feed task failed: {error}")))?;
        }
        self.cell.disconnect().await
    }

    /// Convert live counters into the stable API status shape.
    #[must_use]
    pub fn status(&self) -> FeedRuntimeStatus {
        let elapsed = now_ms().saturating_sub(self.started_at_ms).max(1);
        let pulses = self.cell.emitted_count();
        FeedRuntimeStatus {
            id: self.cell.cell_id().to_string(),
            topic: self.cell.config().topic.clone(),
            kind: format!("{:?}", self.cell.config().kind),
            connected: self.cell.status() == FeedStatus::Connected,
            rate_hz: pulses as f64 * 1_000.0 / elapsed as f64,
            pulses_produced: pulses,
            last_update_ms: self.cell.last_emitted_at_ms(),
            error: self.last_error(),
        }
    }
}

/// Static discovery plus bounded runtime supervision for feed Cells.
pub struct RuntimeRegistry {
    factories: RwLock<HashMap<String, FeedFactory>>,
    running: RwLock<HashMap<String, FeedHandle>>,
    reconnect: ReconnectPolicy,
}

impl std::fmt::Debug for RuntimeRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeRegistry")
            .field("registered", &self.factories.read().len())
            .field("running", &self.running.read().len())
            .field("reconnect", &self.reconnect)
            .finish()
    }
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self::new(ReconnectPolicy::default())
    }
}

impl RuntimeRegistry {
    /// Create a registry with an explicit reconnect policy.
    #[must_use]
    pub fn new(reconnect: ReconnectPolicy) -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            running: RwLock::new(HashMap::new()),
            reconnect,
        }
    }

    /// Add or replace a discoverable feed factory.
    pub fn register(
        &self,
        info: FeedInfo,
        build: impl Fn() -> Arc<FeedCell> + Send + Sync + 'static,
    ) {
        self.factories.write().insert(
            info.id.clone(),
            FeedFactory {
                info,
                build: Arc::new(build),
            },
        );
    }

    /// Return all discoverable descriptors in stable identifier order.
    #[must_use]
    pub fn discover(&self) -> Vec<FeedInfo> {
        let mut feeds = self
            .factories
            .read()
            .values()
            .map(|factory| factory.info.clone())
            .collect::<Vec<_>>();
        feeds.sort_by(|left, right| left.id.cmp(&right.id));
        feeds
    }

    /// Search discoverable feeds by id, name, description, kind, or topic.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<FeedInfo> {
        let query = query.to_lowercase();
        self.discover()
            .into_iter()
            .filter(|feed| {
                feed.id.to_lowercase().contains(&query)
                    || feed.name.to_lowercase().contains(&query)
                    || feed.description.to_lowercase().contains(&query)
                    || format!("{:?}", feed.kind).to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Construct and start a registered feed by identifier.
    pub fn start_registered(&self, feed_id: &str) -> Result<FeedHandle> {
        let factory = self
            .factories
            .read()
            .get(feed_id)
            .cloned()
            .ok_or_else(|| RokoError::invalid(format!("unknown feed: {feed_id}")))?;
        self.start((factory.build)())
    }

    /// Start supervision for an already constructed feed Cell.
    pub fn start(&self, cell: Arc<FeedCell>) -> Result<FeedHandle> {
        let feed_id = cell.cell_id().to_string();
        if self.running.read().contains_key(&feed_id) {
            return Err(RokoError::invalid(format!(
                "feed already running: {feed_id}"
            )));
        }

        let cancel = CancellationToken::new();
        let error_count = Arc::new(AtomicU64::new(0));
        let last_error = Arc::new(RwLock::new(None));
        let policy = self.reconnect;
        let runner_cell = Arc::clone(&cell);
        let runner_cancel = cancel.clone();
        let runner_errors = Arc::clone(&error_count);
        let runner_last_error = Arc::clone(&last_error);
        let task = tokio::spawn(async move {
            let mut delay = policy.initial;
            loop {
                if runner_cancel.is_cancelled() {
                    break;
                }
                match runner_cell.connect().await {
                    Ok(()) => {
                        delay = policy.initial;
                        *runner_last_error.write() = None;
                        let cycle_cancel = runner_cancel.child_token();
                        if let Err(error) = runner_cell.listen(cycle_cancel).await {
                            runner_errors.fetch_add(1, Ordering::Relaxed);
                            *runner_last_error.write() = Some(error.to_string());
                        }
                    }
                    Err(error) => {
                        runner_errors.fetch_add(1, Ordering::Relaxed);
                        *runner_last_error.write() = Some(error.to_string());
                    }
                }
                let _ = runner_cell.disconnect().await;
                tokio::select! {
                    _ = runner_cancel.cancelled() => break,
                    _ = tokio::time::sleep(delay) => {}
                }
                delay = delay.saturating_mul(2).min(policy.maximum);
            }
            let _ = runner_cell.disconnect().await;
        });

        let handle = FeedHandle {
            cell,
            cancel,
            task: Arc::new(tokio::sync::Mutex::new(Some(task))),
            started_at_ms: now_ms(),
            error_count,
            last_error,
        };
        self.running.write().insert(feed_id, handle.clone());
        Ok(handle)
    }

    /// Stop a running feed and retain its discoverable registration.
    pub async fn stop(&self, feed_id: &str) -> Result<()> {
        let handle = self
            .running
            .write()
            .remove(feed_id)
            .ok_or_else(|| RokoError::invalid(format!("feed is not running: {feed_id}")))?;
        handle.stop().await
    }

    /// Cooperatively stop every active feed, attempting all of them even if
    /// one disconnect reports an error.
    pub async fn stop_all(&self) -> Result<()> {
        let ids = self.running.read().keys().cloned().collect::<Vec<_>>();
        let mut first_error = None;
        for id in ids {
            if let Err(error) = self.stop(&id).await {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Get a cloneable runtime handle.
    #[must_use]
    pub fn get(&self, feed_id: &str) -> Option<FeedHandle> {
        self.running.read().get(feed_id).cloned()
    }

    /// Return runtime status for all discoverable feeds, including stopped ones.
    #[must_use]
    pub fn health(&self) -> Vec<FeedRuntimeStatus> {
        let running = self.running.read();
        self.discover()
            .into_iter()
            .map(|info| {
                running.get(&info.id).map_or_else(
                    || FeedRuntimeStatus {
                        id: info.id,
                        topic: info
                            .schema
                            .as_ref()
                            .and_then(|schema| schema.get("topic"))
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        kind: format!("{:?}", info.kind),
                        connected: false,
                        rate_hz: 0.0,
                        pulses_produced: 0,
                        last_update_ms: None,
                        error: None,
                    },
                    FeedHandle::status,
                )
            })
            .collect()
    }

    /// Return status for a registered feed whether running or stopped.
    #[must_use]
    pub fn status(&self, feed_id: &str) -> Option<FeedRuntimeStatus> {
        self.health()
            .into_iter()
            .find(|status| status.id == feed_id)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    use async_trait::async_trait;
    use serde_json::Value;

    use crate::connector::ConnectorStatus;
    use crate::feed::{FeedAccess, FeedKind};
    use crate::feed_cell::{ConnectorOps, FeedCellConfig, FeedTrigger};

    struct FlakyConnector(AtomicUsize);

    #[async_trait]
    impl ConnectorOps for FlakyConnector {
        async fn connect(&self) -> Result<()> {
            if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
                Err(RokoError::invalid("first connection fails"))
            } else {
                Ok(())
            }
        }
        async fn query(&self, _query: &str) -> Result<Value> {
            Ok(Value::Null)
        }
        async fn disconnect(&self) -> Result<()> {
            Ok(())
        }
        async fn health(&self) -> ConnectorStatus {
            ConnectorStatus::Connected
        }
    }

    #[tokio::test]
    async fn reconnects_with_a_bounded_policy_and_stops() {
        let (trigger, _sender) = FeedTrigger::channel("test", 0, 1, |_| true);
        let cell = Arc::new(FeedCell::new(
            "retry",
            "Retry",
            (1, 0, 0),
            Arc::new(FlakyConnector(AtomicUsize::new(0))),
            Arc::new(trigger),
            None,
            FeedCellConfig {
                kind: FeedKind::Raw,
                access: FeedAccess::Public,
                polling_interval: Duration::from_millis(1),
                topic: "test".into(),
            },
        ));
        let registry = RuntimeRegistry::new(ReconnectPolicy {
            initial: Duration::from_millis(5),
            maximum: Duration::from_millis(10),
        });
        registry.start(cell).unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(registry.get("retry").unwrap().error_count() >= 1);
        registry.stop("retry").await.unwrap();
        assert!(registry.get("retry").is_none());
    }
}
