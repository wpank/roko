//! Bounded asynchronous batch queue.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use dashmap::DashMap;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::{GatewayError, GatewayResult, InferenceRequest, InferenceResponse};

/// Provider batch completion polling interval.
pub const BATCH_POLL_INTERVAL: Duration = Duration::from_mins(1);
/// Default item-count flush threshold.
pub const DEFAULT_FLUSH_SIZE: usize = 50;
/// Default time flush threshold.
pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_secs(30);
const DEFAULT_MAX_QUEUE: usize = 1_000;
const LOCAL_FLUSH_CONCURRENCY: usize = 8;

/// One queued batch request.
#[derive(Debug, Clone)]
pub struct BatchEntry {
    /// Request, marked for batch pricing before processing.
    pub request: InferenceRequest,
    /// Stable correlation id (`roko-{uuid}`).
    pub custom_id: String,
    /// Queue insertion time.
    pub submitted_at: Instant,
}

/// Batch item lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Accepted but not complete.
    Pending,
    /// Completed successfully.
    Complete,
    /// Provider or pipeline failure.
    Failed,
}

/// Pollable batch result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchResult {
    /// Completed response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<InferenceResponse>,
    /// Lifecycle state.
    pub status: BatchStatus,
    /// Sanitized failure text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BatchResult {
    fn pending() -> Self {
        Self {
            response: None,
            status: BatchStatus::Pending,
            error: None,
        }
    }
}

/// Pluggable batch submission boundary.
#[async_trait]
pub trait BatchProcessor: Send + Sync {
    /// Process a drained batch. Implementations may use a native provider batch
    /// endpoint or run bounded local gateway calls.
    async fn process_batch(
        &self,
        entries: Vec<BatchEntry>,
    ) -> Vec<(String, GatewayResult<InferenceResponse>)>;
}

struct UnconfiguredBatchProcessor;

#[async_trait]
impl BatchProcessor for UnconfiguredBatchProcessor {
    async fn process_batch(
        &self,
        entries: Vec<BatchEntry>,
    ) -> Vec<(String, GatewayResult<InferenceResponse>)> {
        entries
            .into_iter()
            .map(|entry| {
                (
                    entry.custom_id,
                    Err(GatewayError::ProvidersExhausted(
                        "batch processor is not configured".into(),
                    )),
                )
            })
            .collect()
    }
}

/// Bounded queue with item-count and age-based auto-flush.
pub struct BatchQueue {
    entries: Mutex<VecDeque<BatchEntry>>,
    results: DashMap<String, BatchResult>,
    flush_interval: Duration,
    flush_size: usize,
    max_queue: usize,
    processor: Arc<dyn BatchProcessor>,
}

impl BatchQueue {
    /// Construct with an unconfigured processor. Use [`Self::with_processor`]
    /// for live submission.
    #[must_use]
    pub fn new(flush_interval: Duration, flush_size: usize) -> Self {
        Self::with_processor(
            flush_interval,
            flush_size,
            DEFAULT_MAX_QUEUE,
            Arc::new(UnconfiguredBatchProcessor),
        )
    }

    /// Construct with an explicit live/native batch processor.
    #[must_use]
    pub fn with_processor(
        flush_interval: Duration,
        flush_size: usize,
        max_queue: usize,
        processor: Arc<dyn BatchProcessor>,
    ) -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
            results: DashMap::new(),
            flush_interval,
            flush_size: flush_size.max(1),
            max_queue: max_queue.max(1),
            processor,
        }
    }

    /// Queue a request and return its generated correlation id.
    pub async fn submit(&self, mut request: InferenceRequest) -> GatewayResult<String> {
        request.metadata.is_batch = true;
        let custom_id = format!("roko-{}", uuid::Uuid::new_v4());
        let should_flush = {
            let mut entries = self.entries.lock().map_err(|_| {
                GatewayError::ProvidersExhausted("batch queue lock poisoned".into())
            })?;
            if entries.len() >= self.max_queue {
                return Err(GatewayError::BatchQueueFull {
                    capacity: self.max_queue,
                });
            }
            entries.push_back(BatchEntry {
                request,
                custom_id: custom_id.clone(),
                submitted_at: Instant::now(),
            });
            entries.len() >= self.flush_size
        };
        self.results
            .insert(custom_id.clone(), BatchResult::pending());
        if should_flush {
            self.flush().await?;
        }
        Ok(custom_id)
    }

    /// Drain and submit the current queue.
    pub async fn flush(&self) -> GatewayResult<usize> {
        let entries = {
            let mut queue = self.entries.lock().map_err(|_| {
                GatewayError::ProvidersExhausted("batch queue lock poisoned".into())
            })?;
            queue.drain(..).collect::<Vec<_>>()
        };
        if entries.is_empty() {
            return Ok(0);
        }
        let count = entries.len();
        for (custom_id, result) in self.processor.process_batch(entries).await {
            let batch_result = match result {
                Ok(response) => BatchResult {
                    response: Some(response),
                    status: BatchStatus::Complete,
                    error: None,
                },
                Err(error) => BatchResult {
                    response: None,
                    status: BatchStatus::Failed,
                    error: Some(error.to_string()),
                },
            };
            self.results.insert(custom_id, batch_result);
        }
        Ok(count)
    }

    /// Flush only when the oldest item reached the time threshold.
    pub async fn flush_if_due(&self) -> GatewayResult<usize> {
        let due = self
            .entries
            .lock()
            .ok()
            .and_then(|entries| entries.front().map(|entry| entry.submitted_at.elapsed()))
            .is_some_and(|elapsed| elapsed >= self.flush_interval);
        if due { self.flush().await } else { Ok(0) }
    }

    /// Spawn the 30-second (or configured) age-based auto-flush loop.
    pub fn spawn_auto_flush(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(self.flush_interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if let Err(error) = self.flush_if_due().await {
                    tracing::warn!(%error, "gateway batch auto-flush failed");
                }
            }
        })
    }

    /// Retrieve one current/completed result.
    #[must_use]
    pub fn get_result(&self, custom_id: &str) -> Option<BatchResult> {
        self.results.get(custom_id).map(|result| result.clone())
    }

    /// Current queued item count.
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.entries.lock().map_or(0, |entries| entries.len())
    }

    /// Provider polling interval contract.
    #[must_use]
    pub const fn poll_interval(&self) -> Duration {
        BATCH_POLL_INTERVAL
    }
}

/// Utility processor for bounded per-item completion through one client.
pub struct ClientBatchProcessor {
    client: Arc<dyn crate::InferenceClient>,
}

impl ClientBatchProcessor {
    /// Bind a gateway client/handle.
    #[must_use]
    pub fn new(client: Arc<dyn crate::InferenceClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl BatchProcessor for ClientBatchProcessor {
    async fn process_batch(
        &self,
        entries: Vec<BatchEntry>,
    ) -> Vec<(String, GatewayResult<InferenceResponse>)> {
        stream::iter(entries)
            .map(|entry| {
                let client = Arc::clone(&self.client);
                async move {
                    let result = client.complete(entry.request).await;
                    (entry.custom_id, result)
                }
            })
            .buffer_unordered(LOCAL_FLUSH_CONCURRENCY)
            .collect()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoProcessor;

    #[async_trait]
    impl BatchProcessor for EchoProcessor {
        async fn process_batch(
            &self,
            entries: Vec<BatchEntry>,
        ) -> Vec<(String, GatewayResult<InferenceResponse>)> {
            entries
                .into_iter()
                .map(|entry| {
                    assert!(entry.request.metadata.is_batch);
                    (
                        entry.custom_id,
                        Ok(InferenceResponse {
                            text: "done".into(),
                            model: "batch-model".into(),
                            ..InferenceResponse::default()
                        }),
                    )
                })
                .collect()
        }
    }

    fn queue(size: usize, interval: Duration) -> Arc<BatchQueue> {
        Arc::new(BatchQueue::with_processor(
            interval,
            size,
            1_000,
            Arc::new(EchoProcessor),
        ))
    }

    #[tokio::test]
    async fn batch_auto_flushes_at_item_threshold_and_uses_roko_ids() {
        let queue = queue(2, Duration::from_secs(30));
        let first = queue.submit(InferenceRequest::default()).await.unwrap();
        assert!(first.starts_with("roko-"));
        assert_eq!(
            queue.get_result(&first).unwrap().status,
            BatchStatus::Pending
        );
        let second = queue.submit(InferenceRequest::default()).await.unwrap();
        assert_eq!(queue.queue_len(), 0);
        assert_eq!(
            queue.get_result(&first).unwrap().status,
            BatchStatus::Complete
        );
        assert_eq!(
            queue.get_result(&second).unwrap().status,
            BatchStatus::Complete
        );
    }

    #[tokio::test(start_paused = true)]
    async fn batch_auto_flushes_at_time_threshold_and_polls_no_faster_than_sixty_seconds() {
        let queue = queue(50, Duration::from_secs(30));
        let custom_id = queue.submit(InferenceRequest::default()).await.unwrap();
        let task = Arc::clone(&queue).spawn_auto_flush();
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(31)).await;
        tokio::task::yield_now().await;
        assert_eq!(
            queue.get_result(&custom_id).unwrap().status,
            BatchStatus::Complete
        );
        assert_eq!(queue.poll_interval(), Duration::from_secs(60));
        task.abort();
    }

    #[tokio::test]
    async fn batch_queue_is_hard_bounded() {
        let queue = Arc::new(BatchQueue::with_processor(
            Duration::from_secs(30),
            50,
            1,
            Arc::new(EchoProcessor),
        ));
        let _ = queue.submit(InferenceRequest::default()).await.unwrap();
        assert!(matches!(
            queue.submit(InferenceRequest::default()).await,
            Err(GatewayError::BatchQueueFull { capacity: 1 })
        ));
    }
}
