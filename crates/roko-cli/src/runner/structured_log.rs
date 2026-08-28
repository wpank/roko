//! Structured JSONL logger for plan execution events.
//!
//! When `--log-file <path>` is passed to `roko plan run`, every
//! [`RunnerEvent`](super::types::RunnerEvent) emitted by the event loop
//! is serialized as a single JSON line and flushed to the file.
//! The logger is a no-op when no path is configured.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::types::RunnerEvent;

/// Thread-safe structured logger backed by an optional JSONL file.
///
/// Cloneable via `Arc<Mutex<...>>` so it can be shared across the event
/// loop without requiring `&mut` at every emit site.
#[derive(Clone)]
pub struct StructuredLogger {
    inner: Option<Arc<Mutex<BufWriter<File>>>>,
}

impl StructuredLogger {
    /// Create a logger that writes to the given path.
    ///
    /// The file is created (or truncated) immediately. Returns an error
    /// if the file cannot be opened.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        Ok(Self {
            inner: Some(Arc::new(Mutex::new(BufWriter::new(file)))),
        })
    }

    /// Create a no-op logger that discards all events.
    #[must_use]
    pub fn noop() -> Self {
        Self { inner: None }
    }

    /// Write a runner event as a single JSONL line.
    ///
    /// Silently drops events when serialization or I/O fails (structured
    /// logging must never block or crash the executor).
    pub fn log(&self, event: &RunnerEvent) {
        let Some(ref writer) = self.inner else {
            return;
        };
        let Ok(mut guard) = writer.lock() else {
            return;
        };
        if serde_json::to_writer(&mut *guard, event).is_ok() {
            let _ = guard.write_all(b"\n");
            let _ = guard.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_run_started() -> RunnerEvent {
        RunnerEvent::RunStarted {
            timestamp: chrono::Utc::now().to_rfc3339(),
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
            run_id: "test-run-001".into(),
            plan_ids: vec!["plan-a".into()],
            total_tasks: 1,
            resumed: false,
            resume_session: None,
        }
    }

    #[test]
    fn log_writes_valid_jsonl_with_type_and_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let logger = StructuredLogger::open(&path).unwrap();
        logger.log(&make_run_started());
        drop(logger);

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1, "expected exactly one line");
        let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert!(v.get("type").is_some(), "missing 'type' field");
        assert_eq!(v["type"], "run.started");
    }

    #[test]
    fn noop_logger_does_not_create_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("should-not-exist.jsonl");
        let logger = StructuredLogger::noop();
        logger.log(&make_run_started());
        assert!(!path.exists());
    }

    #[test]
    fn log_multiple_events_produces_multiple_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("multi.jsonl");
        let logger = StructuredLogger::open(&path).unwrap();
        logger.log(&make_run_started());
        logger.log(&make_run_started());
        drop(logger);

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }
    }

    #[test]
    fn open_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deeply/nested/dir/events.jsonl");
        let logger = StructuredLogger::open(&path).unwrap();
        logger.log(&make_run_started());
        drop(logger);
        assert!(path.exists());
    }

    #[test]
    fn clone_shares_underlying_writer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shared.jsonl");
        let logger1 = StructuredLogger::open(&path).unwrap();
        let logger2 = logger1.clone();
        logger1.log(&make_run_started());
        logger2.log(&make_run_started());
        drop(logger1);
        drop(logger2);

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2, "both clones should write to same file");
    }
}
