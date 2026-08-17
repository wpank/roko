//! Debounced filesystem change feed.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use notify::{RecursiveMode, Watcher};
use parking_lot::RwLock;
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::connector::ConnectorStatus;
use crate::error::{Result, RokoError};
use crate::feed::{FeedAccess, FeedKind};
use crate::feed_cell::{ConnectorOps, FeedCell, FeedCellConfig, FeedPulse, FeedTriggerOps};

/// Factory for the canonical `.roko` file-change feed.
pub struct FileWatchFeed;

impl FileWatchFeed {
    /// Build a runnable feed watching `directory` recursively.
    #[must_use]
    pub fn build(directory: impl Into<PathBuf>) -> Arc<FeedCell> {
        let directory = directory.into();
        Arc::new(FeedCell::new(
            "file-watch-roko-dir",
            "Roko file changes",
            (1, 0, 0),
            Arc::new(FileWatchConnector::new(directory.clone())),
            Arc::new(FileWatchTrigger { directory }),
            None,
            FeedCellConfig {
                kind: FeedKind::Raw,
                access: FeedAccess::Private,
                polling_interval: Duration::from_millis(200),
                topic: "fs.changed".to_string(),
            },
        ))
    }
}

#[derive(Debug)]
struct FileWatchConnector {
    directory: PathBuf,
    connected: RwLock<bool>,
}

impl FileWatchConnector {
    fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            connected: RwLock::new(false),
        }
    }
}

#[async_trait]
impl ConnectorOps for FileWatchConnector {
    async fn connect(&self) -> Result<()> {
        if !self.directory.is_dir() {
            return Err(RokoError::invalid(format!(
                "feed watch directory does not exist: {}",
                self.directory.display()
            )));
        }
        *self.connected.write() = true;
        Ok(())
    }

    async fn query(&self, _query: &str) -> Result<Value> {
        let mut paths = std::fs::read_dir(&self.directory)?
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path().display().to_string())
            .collect::<Vec<_>>();
        paths.sort();
        Ok(json!({"directory": self.directory, "entries": paths}))
    }

    async fn disconnect(&self) -> Result<()> {
        *self.connected.write() = false;
        Ok(())
    }

    async fn health(&self) -> ConnectorStatus {
        if *self.connected.read() && self.directory.is_dir() {
            ConnectorStatus::Connected
        } else {
            ConnectorStatus::Disconnected
        }
    }
}

#[derive(Debug)]
struct FileWatchTrigger {
    directory: PathBuf,
}

#[async_trait]
impl FeedTriggerOps for FileWatchTrigger {
    async fn listen(
        &self,
        feed_id: &str,
        output: broadcast::Sender<FeedPulse>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        })
        .map_err(|error| RokoError::invalid(format!("cannot create file watcher: {error}")))?;
        watcher
            .watch(&self.directory, RecursiveMode::Recursive)
            .map_err(|error| RokoError::invalid(format!("cannot watch feed directory: {error}")))?;

        let mut sequence = 0;
        let mut last_emit = tokio::time::Instant::now() - Duration::from_millis(200);
        loop {
            tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                event = receiver.recv() => {
                    let Some(event) = event else { return Ok(()) };
                    let event = event.map_err(|error| RokoError::invalid(format!("file watch error: {error}")))?;
                    let paths = event.paths.into_iter().filter(|path| self.accept_path(path)).collect::<Vec<_>>();
                    if paths.is_empty() || last_emit.elapsed() < Duration::from_millis(self.debounce_ms()) {
                        continue;
                    }
                    last_emit = tokio::time::Instant::now();
                    let pulse = self.fire(feed_id, sequence, json!({
                        "kind": format!("{:?}", event.kind).to_lowercase(),
                        "paths": paths,
                    }));
                    sequence = sequence.saturating_add(1);
                    let _ = output.send(pulse);
                }
            }
        }
    }

    fn filter(&self, event: &Value) -> bool {
        event
            .get("paths")
            .and_then(Value::as_array)
            .is_some_and(|paths| !paths.is_empty())
    }

    fn debounce_ms(&self) -> u64 {
        200
    }

    fn fire(&self, feed_id: &str, sequence: u64, event: Value) -> FeedPulse {
        FeedPulse::new(feed_id, "fs.changed", sequence, event)
    }
}

impl FileWatchTrigger {
    fn accept_path(&self, path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        !(name.ends_with(".tmp") || name.ends_with('~') || name.starts_with(".#"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_write_paths_are_ignored() {
        let trigger = FileWatchTrigger {
            directory: PathBuf::new(),
        };
        assert!(!trigger.accept_path(Path::new("state.toml.tmp")));
        assert!(!trigger.accept_path(Path::new("state.toml~")));
        assert!(trigger.accept_path(Path::new("state.toml")));
    }
}
