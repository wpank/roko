//! Cross-platform message audit trail (#416).
//!
//! Every message sent or received through any [`ChatBridge`] adapter is
//! appended to `.roko/audit/messages.jsonl` as a [`MessageAuditRecord`].
//! The log is append-only and safe to write from multiple async tasks.
//!
//! # Record format
//!
//! ```json
//! {"message_id":"msg-42","platform_id":"discord-main","channel":"#ops",
//!  "direction":"outbound","timestamp_ms":1700000000000}
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// MessageDirection
// ---------------------------------------------------------------------------

/// Whether the message was sent outbound or received inbound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    /// A message sent by the roko runtime to a platform channel.
    Outbound,
    /// A message received from a platform channel.
    Inbound,
}

// ---------------------------------------------------------------------------
// MessageAuditRecord
// ---------------------------------------------------------------------------

/// A single audit entry for one message crossing a platform bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAuditRecord {
    /// Platform-assigned (or caller-assigned) message identifier.
    pub message_id: String,
    /// Platform connection ID from `[[platforms]]` config (e.g. `"discord-main"`).
    pub platform_id: String,
    /// Channel or room identifier (platform-specific).
    pub channel: String,
    /// Whether the message was sent outbound or received inbound.
    pub direction: MessageDirection,
    /// Unix epoch milliseconds when the record was written.
    pub timestamp_ms: u64,
}

impl MessageAuditRecord {
    /// Construct a new audit record with the current wall-clock timestamp.
    #[must_use]
    pub fn new(
        message_id: impl Into<String>,
        platform_id: impl Into<String>,
        channel: impl Into<String>,
        direction: MessageDirection,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        Self {
            message_id: message_id.into(),
            platform_id: platform_id.into(),
            channel: channel.into(),
            direction,
            timestamp_ms,
        }
    }

    /// Construct an outbound audit record.
    #[must_use]
    pub fn outbound(
        message_id: impl Into<String>,
        platform_id: impl Into<String>,
        channel: impl Into<String>,
    ) -> Self {
        Self::new(message_id, platform_id, channel, MessageDirection::Outbound)
    }

    /// Construct an inbound audit record.
    #[must_use]
    pub fn inbound(
        message_id: impl Into<String>,
        platform_id: impl Into<String>,
        channel: impl Into<String>,
    ) -> Self {
        Self::new(message_id, platform_id, channel, MessageDirection::Inbound)
    }
}

// ---------------------------------------------------------------------------
// MessageAuditLog
// ---------------------------------------------------------------------------

/// Appends [`MessageAuditRecord`]s to `.roko/audit/messages.jsonl`.
///
/// Cheap to clone (backed by `Arc`). The file and its parent directories are
/// created on the first write.
#[derive(Clone, Debug)]
pub struct MessageAuditLog {
    inner: Arc<AuditInner>,
}

#[derive(Debug)]
struct AuditInner {
    path: PathBuf,
    lock: Mutex<()>,
}

impl MessageAuditLog {
    /// Create a log that writes to `{workdir}/.roko/audit/messages.jsonl`.
    #[must_use]
    pub fn new(workdir: impl AsRef<Path>) -> Self {
        let path = workdir
            .as_ref()
            .join(".roko")
            .join("audit")
            .join("messages.jsonl");
        Self {
            inner: Arc::new(AuditInner {
                path,
                lock: Mutex::new(()),
            }),
        }
    }

    /// Create a log that writes to an explicit path (useful for tests).
    #[must_use]
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(AuditInner {
                path: path.into(),
                lock: Mutex::new(()),
            }),
        }
    }

    /// Return the file path this log writes to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// Append a [`MessageAuditRecord`] to the audit log.
    ///
    /// Creates the file and parent directories on demand.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the directory cannot be created, the file
    /// cannot be opened, or the write fails.
    pub async fn record(&self, rec: &MessageAuditRecord) -> std::io::Result<()> {
        let mut line = serde_json::to_string(rec).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("audit serialize: {e}"),
            )
        })?;
        line.push('\n');

        let _guard = self.inner.lock.lock().await;

        if let Some(parent) = self.inner.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.inner.path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    /// Read all audit records from the log file.
    ///
    /// Returns an empty `Vec` when the file does not yet exist.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file exists but cannot be read or contains
    /// malformed JSON lines.
    pub async fn read_all(&self) -> std::io::Result<Vec<MessageAuditRecord>> {
        let _guard = self.inner.lock.lock().await;
        let contents = match fs::read_to_string(&self.inner.path).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut records = Vec::new();
        for (i, line) in contents.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let rec: MessageAuditRecord = serde_json::from_str(line).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("audit line {}: {e}", i + 1),
                )
            })?;
            records.push(rec);
        }
        Ok(records)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn record_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let log = MessageAuditLog::with_path(dir.path().join("audit.jsonl"));

        let r1 = MessageAuditRecord::outbound("msg-1", "discord-main", "#ops");
        let r2 = MessageAuditRecord::inbound("msg-2", "slack-ops", "#alerts");

        log.record(&r1).await.unwrap();
        log.record(&r2).await.unwrap();

        let records = log.read_all().await.unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].message_id, "msg-1");
        assert_eq!(records[0].direction, MessageDirection::Outbound);
        assert_eq!(records[1].direction, MessageDirection::Inbound);
    }

    #[tokio::test]
    async fn read_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let log = MessageAuditLog::with_path(dir.path().join("absent.jsonl"));
        assert!(log.read_all().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("audit.jsonl");
        let log = MessageAuditLog::with_path(nested);
        let rec = MessageAuditRecord::outbound("msg-x", "matrix", "!room");
        log.record(&rec).await.unwrap();
        let records = log.read_all().await.unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn default_path_inside_workdir() {
        let log = MessageAuditLog::new("/some/workspace");
        assert_eq!(
            log.path().to_string_lossy().as_ref(),
            "/some/workspace/.roko/audit/messages.jsonl"
        );
    }
}
