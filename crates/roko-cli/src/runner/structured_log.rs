//! Structured JSONL logger for plan execution events.
//!
//! When `--log-file <path>` is passed to `roko plan run`, every
//! [`RunnerEvent`](super::types::RunnerEvent) emitted by the event loop
//! is serialized as a single JSON line and flushed to the file.
//! The logger is a no-op when no path is configured.
//!
//! ## Graph Engine support (#115)
//!
//! [`GraphEventLogger`] implements [`GraphEventSink`] so the same
//! `--log-file` flag works with `--engine graph`. Each JSONL line contains
//! a `"source"` field (`"graph"` or `"runtime"`) and the serialized event.
//! A companion [`FanOutGraphEventSink`] forwards events to multiple sinks
//! when both structured logging and other sinks (e.g. StateHub) are active.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use roko_graph::events::{
    GraphEventDisposition, GraphEventError, GraphEventSink, GraphExecutionEvent,
};

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

// ---------------------------------------------------------------------------
// GraphEventLogger: canonical --log-file recorder for Graph Engine (#115)
// ---------------------------------------------------------------------------

/// JSONL envelope written by [`GraphEventLogger`].
///
/// Each line in the log file is one of these envelopes. The `source` field
/// disambiguates graph-local events from their runtime-mapped equivalents
/// when both are recorded.
#[derive(serde::Serialize)]
struct GraphLogEntry<'a> {
    /// ISO-8601 timestamp.
    ts: String,
    /// Discriminator: `"graph"` for the raw [`GraphExecutionEvent`].
    source: &'static str,
    /// The serialized event payload.
    event: &'a GraphExecutionEvent,
}

/// [`GraphEventSink`] implementation that writes every
/// [`GraphExecutionEvent`] as a JSONL line to a file.
///
/// Thread-safe and cheaply cloneable. Serialization or I/O failures are
/// silently swallowed -- structured logging must never block or crash the
/// graph engine.
#[derive(Clone)]
pub struct GraphEventLogger {
    inner: Arc<Mutex<BufWriter<File>>>,
}

impl GraphEventLogger {
    /// Open (create/truncate) the log file at `path`.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }
}

#[async_trait]
impl GraphEventSink for GraphEventLogger {
    async fn publish(
        &self,
        event: &GraphExecutionEvent,
    ) -> Result<GraphEventDisposition, GraphEventError> {
        let entry = GraphLogEntry {
            ts: chrono::Utc::now().to_rfc3339(),
            source: "graph",
            event,
        };
        let Ok(mut guard) = self.inner.lock() else {
            return Ok(GraphEventDisposition::Dropped);
        };
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = guard.write_all(json.as_bytes());
            let _ = guard.write_all(b"\n");
            // Flush on terminal/reliable events; buffer best-effort.
            if event.is_terminal()
                || matches!(
                    event.delivery(),
                    roko_graph::events::GraphEventDelivery::Reliable
                )
            {
                let _ = guard.flush();
            }
        }
        Ok(GraphEventDisposition::Acknowledged)
    }
}

// ---------------------------------------------------------------------------
// FanOutGraphEventSink: multiplex to N sinks
// ---------------------------------------------------------------------------

/// Forwards each [`GraphExecutionEvent`] to multiple [`GraphEventSink`]s.
///
/// All sinks are called in registration order. A failure in one sink does
/// not prevent delivery to subsequent sinks.
pub struct FanOutGraphEventSink {
    sinks: Vec<Arc<dyn GraphEventSink>>,
}

impl FanOutGraphEventSink {
    /// Create a fan-out from two sinks.
    pub fn pair(a: Arc<dyn GraphEventSink>, b: Arc<dyn GraphEventSink>) -> Self {
        Self { sinks: vec![a, b] }
    }
}

#[async_trait]
impl GraphEventSink for FanOutGraphEventSink {
    async fn publish(
        &self,
        event: &GraphExecutionEvent,
    ) -> Result<GraphEventDisposition, GraphEventError> {
        let mut last = GraphEventDisposition::Acknowledged;
        for sink in &self.sinks {
            match sink.publish(event).await {
                Ok(disposition) => last = disposition,
                Err(error) => {
                    tracing::warn!(%error, "fan-out graph event sink delivery failed");
                }
            }
        }
        Ok(last)
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

    // ── GraphEventLogger tests ──────────────────────────────────────────

    fn make_graph_started() -> GraphExecutionEvent {
        use roko_graph::events::{CommonFields, GRAPH_EVENT_SCHEMA_VERSION};
        GraphExecutionEvent::GraphStarted {
            common: CommonFields {
                schema_version: GRAPH_EVENT_SCHEMA_VERSION,
                run_id: "test-run".to_string(),
                graph_id: "test-graph".to_string(),
                seq: 1,
            },
        }
    }

    #[tokio::test]
    async fn graph_event_logger_writes_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("graph-events.jsonl");
        let logger = GraphEventLogger::open(&path).unwrap();

        let event = make_graph_started();
        let result = logger.publish(&event).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), GraphEventDisposition::Acknowledged);
        drop(logger);

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);
        let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(v["source"], "graph");
        assert!(v.get("ts").is_some());
        assert!(v.get("event").is_some());
        assert_eq!(v["event"]["kind"], "graph_started");
    }

    #[tokio::test]
    async fn fan_out_delivers_to_both_sinks() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.jsonl");
        let path_b = dir.path().join("b.jsonl");
        let a = Arc::new(GraphEventLogger::open(&path_a).unwrap());
        let b = Arc::new(GraphEventLogger::open(&path_b).unwrap());
        let fanout = FanOutGraphEventSink::pair(a, b);

        let event = make_graph_started();
        let result = fanout.publish(&event).await;
        assert!(result.is_ok());

        // Both files should have exactly one line.
        let content_a = std::fs::read_to_string(&path_a).unwrap();
        let content_b = std::fs::read_to_string(&path_b).unwrap();
        assert_eq!(content_a.lines().count(), 1);
        assert_eq!(content_b.lines().count(), 1);
    }
}
