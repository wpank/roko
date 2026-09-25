//! Notification batching orchestrator (#417).
//!
//! [`NotificationBatcher`] accumulates notification strings that arrive within
//! a configurable time window (default: 5 seconds) and flushes them as a
//! single batch via a provided callback.  This prevents per-task chat spam
//! when many tasks complete in quick succession.
//!
//! # Usage
//!
//! ```rust,ignore
//! let batcher = NotificationBatcher::new(Duration::from_secs(5));
//! let handle = batcher.spawn(|batch| async move {
//!     // send all items as one message
//!     platform.send_message("#ops", &batch.join("\n")).await
//! });
//! handle.push("task A completed").await;
//! handle.push("task B completed").await;
//! // → after 5 s both arrive in one callback invocation
//! ```
//!
//! The batcher runs as a background Tokio task. Dropping the
//! [`NotificationBatcherHandle`] does not flush pending items; call
//! [`NotificationBatcherHandle::flush`] to drain before shutdown.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time;

// ---------------------------------------------------------------------------
// BatcherConfig
// ---------------------------------------------------------------------------

/// Configuration for a [`NotificationBatcher`].
#[derive(Debug, Clone)]
pub struct BatcherConfig {
    /// How long to wait before flushing a non-empty batch.
    ///
    /// Defaults to 5 seconds.
    pub window: Duration,

    /// Maximum number of items to accumulate before flushing early.
    ///
    /// When the pending count reaches this limit the batch is flushed
    /// immediately without waiting for the window to expire.
    /// `None` means no cap (flush only on window expiry or explicit flush).
    pub max_batch_size: Option<usize>,
}

impl Default for BatcherConfig {
    fn default() -> Self {
        Self {
            window: Duration::from_secs(5),
            max_batch_size: None,
        }
    }
}

impl BatcherConfig {
    /// Construct a config with `window` and no batch-size cap.
    #[must_use]
    pub fn with_window(window: Duration) -> Self {
        Self {
            window,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Internal channel messages
// ---------------------------------------------------------------------------

enum BatcherMsg {
    Push(String),
    Flush,
    Shutdown,
}

// ---------------------------------------------------------------------------
// NotificationBatcher / Handle
// ---------------------------------------------------------------------------

/// A handle for pushing notifications into a running batcher.
///
/// Cheaply cloneable (backed by `Arc` + `mpsc` channel).
#[derive(Clone, Debug)]
pub struct NotificationBatcherHandle {
    tx: mpsc::Sender<BatcherMsg>,
}

impl NotificationBatcherHandle {
    /// Push a notification string into the batch.
    ///
    /// Returns an error only if the batcher task has already exited.
    pub async fn push(&self, item: impl Into<String>) -> Result<(), String> {
        self.tx
            .send(BatcherMsg::Push(item.into()))
            .await
            .map_err(|_| "batcher task has exited".to_string())
    }

    /// Force an immediate flush of pending notifications.
    ///
    /// Useful before shutdown to ensure no items are lost.
    pub async fn flush(&self) -> Result<(), String> {
        self.tx
            .send(BatcherMsg::Flush)
            .await
            .map_err(|_| "batcher task has exited".to_string())
    }

    /// Signal the batcher to exit after flushing remaining items.
    pub async fn shutdown(&self) -> Result<(), String> {
        self.tx
            .send(BatcherMsg::Shutdown)
            .await
            .map_err(|_| "batcher task has exited".to_string())
    }
}

/// Notification batching orchestrator.
///
/// Construct via [`NotificationBatcher::new`] or [`NotificationBatcher::with_config`],
/// then call [`NotificationBatcher::spawn`] with a flush callback to start the loop.
pub struct NotificationBatcher {
    config: BatcherConfig,
}

impl NotificationBatcher {
    /// Construct a batcher with the default 5-second window.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: BatcherConfig::default(),
        }
    }

    /// Construct a batcher with a custom window duration.
    #[must_use]
    pub fn with_window(window: Duration) -> Self {
        Self {
            config: BatcherConfig::with_window(window),
        }
    }

    /// Construct a batcher with an explicit config.
    #[must_use]
    pub fn with_config(config: BatcherConfig) -> Self {
        Self { config }
    }

    /// Spawn the batcher loop with the provided flush callback.
    ///
    /// The callback receives a non-empty `Vec<String>` of notifications
    /// accumulated during the window.  It must return a `Future<Output = ()>`.
    ///
    /// The spawned task runs until the handle is shut down or dropped.
    pub fn spawn<F, Fut>(self, on_flush: F) -> NotificationBatcherHandle
    where
        F: Fn(Vec<String>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<BatcherMsg>(256);
        let config = self.config;
        let on_flush = Arc::new(on_flush);

        tokio::spawn(async move {
            let mut pending: Vec<String> = Vec::new();
            let deadline = time::sleep(config.window);
            tokio::pin!(deadline);

            loop {
                tokio::select! {
                    // Window expired — flush if there is anything pending.
                    () = &mut deadline => {
                        if !pending.is_empty() {
                            let batch = std::mem::take(&mut pending);
                            on_flush(batch).await;
                        }
                        // Reset the window timer.
                        deadline
                            .as_mut()
                            .reset(time::Instant::now() + config.window);
                    }

                    // Incoming message.
                    msg = rx.recv() => {
                        match msg {
                            None | Some(BatcherMsg::Shutdown) => {
                                // Flush whatever remains, then exit.
                                if !pending.is_empty() {
                                    let batch = std::mem::take(&mut pending);
                                    on_flush(batch).await;
                                }
                                break;
                            }
                            Some(BatcherMsg::Flush) => {
                                if !pending.is_empty() {
                                    let batch = std::mem::take(&mut pending);
                                    on_flush(batch).await;
                                }
                                // Reset the window after explicit flush.
                                deadline
                                    .as_mut()
                                    .reset(time::Instant::now() + config.window);
                            }
                            Some(BatcherMsg::Push(item)) => {
                                pending.push(item);
                                // Check the batch-size cap.
                                if let Some(max) = config.max_batch_size {
                                    if pending.len() >= max {
                                        let batch = std::mem::take(&mut pending);
                                        on_flush(batch).await;
                                        deadline
                                            .as_mut()
                                            .reset(time::Instant::now() + config.window);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        NotificationBatcherHandle { tx }
    }
}

impl Default for NotificationBatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn items_batched_within_window() {
        let collected: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let collected2 = collected.clone();

        let batcher = NotificationBatcher::with_window(Duration::from_millis(100));
        let handle = batcher.spawn(move |batch| {
            let c = collected2.clone();
            async move {
                c.lock().await.push(batch);
            }
        });

        handle.push("a").await.unwrap();
        handle.push("b").await.unwrap();
        handle.push("c").await.unwrap();

        // Wait for the window to expire.
        tokio::time::sleep(Duration::from_millis(200)).await;

        let batches = collected.lock().await;
        // All three should have been flushed in a single batch.
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0], vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn explicit_flush_drains_pending() {
        let flushed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let flushed2 = flushed.clone();

        let batcher = NotificationBatcher::with_window(Duration::from_secs(60));
        let handle = batcher.spawn(move |batch| {
            let f = flushed2.clone();
            async move {
                f.lock().await.extend(batch);
            }
        });

        handle.push("x").await.unwrap();
        handle.push("y").await.unwrap();
        handle.flush().await.unwrap();

        // Give the background task time to process the flush.
        tokio::time::sleep(Duration::from_millis(50)).await;

        let items = flushed.lock().await;
        assert_eq!(*items, vec!["x", "y"]);
    }

    #[tokio::test]
    async fn max_batch_size_triggers_early_flush() {
        let flush_count = Arc::new(AtomicUsize::new(0));
        let flush_count2 = flush_count.clone();

        let config = BatcherConfig {
            window: Duration::from_secs(60),
            max_batch_size: Some(3),
        };
        let batcher = NotificationBatcher::with_config(config);
        let handle = batcher.spawn(move |_batch| {
            let fc = flush_count2.clone();
            async move {
                fc.fetch_add(1, Ordering::SeqCst);
            }
        });

        for i in 0..3 {
            handle.push(format!("item-{i}")).await.unwrap();
        }

        // Give the background task time to process the cap.
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(flush_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn shutdown_flushes_remaining() {
        let flushed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let flushed2 = flushed.clone();

        let batcher = NotificationBatcher::with_window(Duration::from_secs(60));
        let handle = batcher.spawn(move |batch| {
            let f = flushed2.clone();
            async move {
                f.lock().await.extend(batch);
            }
        });

        handle.push("final").await.unwrap();
        handle.shutdown().await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let items = flushed.lock().await;
        assert_eq!(*items, vec!["final"]);
    }
}
