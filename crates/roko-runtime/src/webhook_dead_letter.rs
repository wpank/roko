//! Webhook dead-letter queue (#418).
//!
//! Messages that exhaust all delivery retries are written to
//! `.roko/webhooks/dead-letter.jsonl`, one JSON record per line.
//!
//! # Record format
//!
//! Each line is a [`DeadLetterRecord`] serialised as compact JSON:
//!
//! ```json
//! {"message_id":"msg-42","platform_id":"discord-main","channel":"#ops",
//!  "text":"hello","total_attempts":4,"last_error":"connection refused",
//!  "dead_lettered_at_ms":1700000000000}
//! ```
//!
//! # Design
//!
//! [`DeadLetterWriter`] is cheaply cloneable (`Arc`-backed) and safe to use
//! from multiple tasks. Writes are append-only and atomic at the OS level
//! (single `write_all` per record, flushed immediately).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// DeadLetterRecord
// ---------------------------------------------------------------------------

/// A message that failed all delivery attempts and has been moved to the
/// dead-letter queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterRecord {
    /// Opaque identifier for the message, assigned by the caller.
    pub message_id: String,
    /// Platform connection ID (from `[[platforms]]` config).
    pub platform_id: String,
    /// Target channel or room identifier.
    pub channel: String,
    /// Plain-text message body that failed to deliver.
    pub text: String,
    /// Number of delivery attempts made (including the final failing one).
    pub total_attempts: u32,
    /// Error message from the last failing attempt.
    pub last_error: String,
    /// Unix epoch milliseconds when the record was written.
    pub dead_lettered_at_ms: u64,
}

impl DeadLetterRecord {
    /// Construct a new record with the current wall-clock timestamp.
    #[must_use]
    pub fn new(
        message_id: impl Into<String>,
        platform_id: impl Into<String>,
        channel: impl Into<String>,
        text: impl Into<String>,
        total_attempts: u32,
        last_error: impl Into<String>,
    ) -> Self {
        let dead_lettered_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        Self {
            message_id: message_id.into(),
            platform_id: platform_id.into(),
            channel: channel.into(),
            text: text.into(),
            total_attempts,
            last_error: last_error.into(),
            dead_lettered_at_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// DeadLetterWriter
// ---------------------------------------------------------------------------

/// Appends [`DeadLetterRecord`]s to `.roko/webhooks/dead-letter.jsonl`.
///
/// Cheap to clone (backed by `Arc`). The file is created (including all parent
/// directories) on the first write.
#[derive(Clone, Debug)]
pub struct DeadLetterWriter {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    path: PathBuf,
    /// Mutex prevents interleaved writes from concurrent tasks.
    lock: Mutex<()>,
}

impl DeadLetterWriter {
    /// Create a writer that appends to `{workdir}/.roko/webhooks/dead-letter.jsonl`.
    #[must_use]
    pub fn new(workdir: impl AsRef<Path>) -> Self {
        let path = workdir
            .as_ref()
            .join(".roko")
            .join("webhooks")
            .join("dead-letter.jsonl");
        Self {
            inner: Arc::new(Inner {
                path,
                lock: Mutex::new(()),
            }),
        }
    }

    /// Create a writer that appends to the given path directly.
    ///
    /// Useful for tests that want to control the exact file location.
    #[must_use]
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(Inner {
                path: path.into(),
                lock: Mutex::new(()),
            }),
        }
    }

    /// Return the path this writer appends to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// Append a [`DeadLetterRecord`] to the dead-letter file.
    ///
    /// Creates the file and its parent directories if they do not exist.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the directory cannot be created, the file
    /// cannot be opened, or the write fails.
    pub async fn write(&self, record: &DeadLetterRecord) -> std::io::Result<()> {
        let mut line = serde_json::to_string(record).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("dead-letter serialize: {e}"),
            )
        })?;
        line.push('\n');

        let _guard = self.inner.lock.lock().await;

        // Create parent directories on demand.
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

    /// Read all records from the dead-letter file.
    ///
    /// Returns an empty `Vec` if the file does not exist yet.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file exists but cannot be read or contains
    /// malformed JSON.
    pub async fn read_all(&self) -> std::io::Result<Vec<DeadLetterRecord>> {
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
            let record: DeadLetterRecord =
                serde_json::from_str(line).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("dead-letter line {}: {e}", i + 1),
                    )
                })?;
            records.push(record);
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
    async fn write_and_read_back() {
        let dir = tempfile::tempdir().expect("tempdir");
        let writer = DeadLetterWriter::with_path(dir.path().join("dl.jsonl"));

        let rec1 = DeadLetterRecord::new("msg-1", "discord", "#ops", "hello", 4, "timeout");
        let rec2 = DeadLetterRecord::new("msg-2", "slack", "#alerts", "world", 2, "404");

        writer.write(&rec1).await.unwrap();
        writer.write(&rec2).await.unwrap();

        let records = writer.read_all().await.unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].message_id, "msg-1");
        assert_eq!(records[1].message_id, "msg-2");
        assert_eq!(records[0].total_attempts, 4);
        assert_eq!(records[1].last_error, "404");
    }

    #[tokio::test]
    async fn read_missing_file_returns_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let writer = DeadLetterWriter::with_path(dir.path().join("absent.jsonl"));
        let records = writer.read_all().await.unwrap();
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn creates_parent_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("a").join("b").join("c").join("dl.jsonl");
        let writer = DeadLetterWriter::with_path(nested);
        let rec = DeadLetterRecord::new("msg-x", "matrix", "#room", "hi", 1, "refused");
        writer.write(&rec).await.unwrap();
        let records = writer.read_all().await.unwrap();
        assert_eq!(records.len(), 1);
    }

    #[tokio::test]
    async fn concurrent_writes_produce_all_records() {
        use std::sync::Arc;
        let dir = tempfile::tempdir().expect("tempdir");
        let writer = Arc::new(DeadLetterWriter::with_path(dir.path().join("concurrent.jsonl")));

        let mut handles = Vec::new();
        for i in 0u32..10 {
            let w = writer.clone();
            handles.push(tokio::spawn(async move {
                let rec = DeadLetterRecord::new(
                    format!("msg-{i}"),
                    "discord",
                    "#ops",
                    "body",
                    1,
                    "err",
                );
                w.write(&rec).await.unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        let records = writer.read_all().await.unwrap();
        assert_eq!(records.len(), 10);
    }

    #[test]
    fn default_path_inside_workdir() {
        let writer = DeadLetterWriter::new("/some/workspace");
        assert_eq!(
            writer.path().to_string_lossy().as_ref(),
            "/some/workspace/.roko/webhooks/dead-letter.jsonl"
        );
    }
}
