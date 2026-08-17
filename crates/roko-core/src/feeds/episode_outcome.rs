//! Append-only episode outcome feed.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::connector::ConnectorStatus;
use crate::error::Result;
use crate::feed::{FeedAccess, FeedKind};
use crate::feed_cell::{ConnectorOps, FeedCell, FeedCellConfig, FeedPulse, FeedTriggerOps};

/// Configuration for the append-only episode outcome feed.
#[derive(Debug, Clone)]
pub struct EpisodeOutcomeFeedConfig {
    /// JSONL source file.
    pub path: PathBuf,
    /// Tail polling interval.
    pub polling_interval: Duration,
}

/// Factory for the episode-outcome feed replacing the retired ISFR vertical.
pub struct EpisodeOutcomeFeed;

impl EpisodeOutcomeFeed {
    /// Build a feed that emits each newly appended, valid JSON line once.
    #[must_use]
    pub fn build(config: EpisodeOutcomeFeedConfig) -> Arc<FeedCell> {
        let offset = Arc::new(AtomicU64::new(0));
        Arc::new(FeedCell::new(
            "episode-outcome-feed",
            "Episode outcomes",
            (1, 0, 0),
            Arc::new(EpisodeConnector {
                path: config.path.clone(),
                connected: RwLock::new(false),
            }),
            Arc::new(EpisodeTrigger {
                path: config.path,
                interval: config.polling_interval,
                offset,
            }),
            None,
            FeedCellConfig {
                kind: FeedKind::Raw,
                access: FeedAccess::Private,
                polling_interval: config.polling_interval,
                topic: "episode.outcome".to_string(),
            },
        ))
    }
}

#[derive(Debug)]
struct EpisodeConnector {
    path: PathBuf,
    connected: RwLock<bool>,
}

#[async_trait]
impl ConnectorOps for EpisodeConnector {
    async fn connect(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !self.path.exists() {
            std::fs::File::create(&self.path)?;
        }
        *self.connected.write() = true;
        Ok(())
    }
    async fn query(&self, _query: &str) -> Result<Value> {
        Ok(json!({"path": self.path, "bytes": std::fs::metadata(&self.path)?.len()}))
    }
    async fn disconnect(&self) -> Result<()> {
        *self.connected.write() = false;
        Ok(())
    }
    async fn health(&self) -> ConnectorStatus {
        if *self.connected.read() && self.path.exists() {
            ConnectorStatus::Connected
        } else {
            ConnectorStatus::Disconnected
        }
    }
}

#[derive(Debug)]
struct EpisodeTrigger {
    path: PathBuf,
    interval: Duration,
    offset: Arc<AtomicU64>,
}

#[async_trait]
impl FeedTriggerOps for EpisodeTrigger {
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
                    let contents = std::fs::read_to_string(&self.path)?;
                    let start = self.offset.load(Ordering::Relaxed).min(contents.len() as u64) as usize;
                    let tail = &contents[start..];
                    let complete_len = tail.rfind('\n').map_or(0, |index| index + 1);
                    for line in tail[..complete_len].lines().filter(|line| !line.trim().is_empty()) {
                        if let Ok(value) = serde_json::from_str::<Value>(line) {
                            let _ = output.send(self.fire(feed_id, sequence, value));
                            sequence = sequence.saturating_add(1);
                        }
                    }
                    self.offset.store((start + complete_len) as u64, Ordering::Relaxed);
                }
            }
        }
    }
    fn filter(&self, event: &Value) -> bool {
        event.is_object()
    }
    fn debounce_ms(&self) -> u64 {
        self.interval.as_millis() as u64
    }
    fn fire(&self, feed_id: &str, sequence: u64, event: Value) -> FeedPulse {
        FeedPulse::new(feed_id, "episode.outcome", sequence, event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn emits_only_complete_json_lines() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("episodes.jsonl");
        let cell = EpisodeOutcomeFeed::build(EpisodeOutcomeFeedConfig {
            path: path.clone(),
            polling_interval: Duration::from_millis(5),
        });
        cell.connect().await.unwrap();
        let mut events = cell.subscribe();
        let cancel = CancellationToken::new();
        let task = {
            let cell = Arc::clone(&cell);
            let cancel = cancel.clone();
            tokio::spawn(async move { cell.listen(cancel).await })
        };
        writeln!(
            std::fs::OpenOptions::new().append(true).open(path).unwrap(),
            "{{\"ok\":true}}"
        )
        .unwrap();
        let pulse = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(pulse.payload["ok"], true);
        cancel.cancel();
        task.await.unwrap().unwrap();
    }
}
