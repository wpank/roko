//! [`JsonlTraceSink`] — persistent [`TraceSink`] backed by a JSONL file
//! per trace, organized into daily directories (§36.99).
//!
//! # Path layout
//!
//! ```text
//! .roko/traces/
//!   2026-04-05/
//!     0102030405060708090a0b0c0d0e0f10.jsonl   ← one trace per file
//!     ...
//!   2026-04-06/
//!     ...
//! ```
//!
//! Each event is appended as a single JSON line to the trace's file; the
//! terminal [`ToolTrace`] snapshot is written as the closing line at
//! [`TraceSink::finish`], then the file handle is flushed and dropped.
//!
//! # Design notes
//!
//! - **Synchronous I/O** — traces are best-effort and never block agent
//!   execution for long; the [`TraceSink`] trait itself is `fn` (not async).
//! - **Best-effort writes** — I/O errors are counted via atomic failure
//!   counters (T034) and swallowed rather than panicking; losing a trace
//!   line must not kill an agent run.
//! - **One file per trace** — the file path is decided at first append
//!   using the current clock's date; events after midnight still land in
//!   that initial file (the trace is a single logical unit).
//! - **Clock injection** — [`JsonlTraceSink::with_clock`] lets tests pin
//!   the date for rotation assertions.
//!
//! # Observability (T034)
//!
//! [`TraceSinkHealth`] exposes failure counts and the last error message
//! via [`JsonlTraceSink::degraded_state`]. Callers (e.g. `roko doctor`)
//! can inspect this to detect silent data loss.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;
use roko_core::tool::trace::{ToolTrace, ToolTraceEvent, TraceId, TraceSink};
use serde::{Deserialize, Serialize};

/// Relative path of the default trace directory from a workspace root.
pub const DEFAULT_TRACE_DIR_REL_PATH: &str = ".roko/traces";

/// Pluggable wall-clock source for the trace sink.
///
/// The default clock returns [`chrono::Utc::now`]. Tests construct a
/// [`JsonlTraceSink`] with a custom clock via
/// [`JsonlTraceSink::with_clock`] to assert midnight-rotation behavior.
pub type Clock = Arc<dyn Fn() -> chrono::DateTime<chrono::Utc> + Send + Sync>;

// ─── TraceSinkHealth (T034) ──────────────────────────────────────────────────

/// Health snapshot for a [`JsonlTraceSink`], reporting whether errors
/// have accumulated during the current session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceSinkHealth {
    /// Number of I/O errors encountered since construction.
    pub failure_count: u64,
    /// The most recent error message, if any.
    pub last_error: Option<String>,
    /// Whether the sink is currently degraded (failure_count > 0).
    pub degraded: bool,
}

/// Shared failure counters for [`JsonlTraceSink`].
struct FailureState {
    count: AtomicU64,
    last_error: Mutex<Option<String>>,
}

impl FailureState {
    fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            last_error: Mutex::new(None),
        }
    }

    fn record_error(&self, msg: String) {
        self.count.fetch_add(1, Ordering::Relaxed);
        *self.last_error.lock() = Some(msg);
    }

    fn snapshot(&self) -> TraceSinkHealth {
        let failure_count = self.count.load(Ordering::Relaxed);
        let last_error = self.last_error.lock().clone();
        TraceSinkHealth {
            failure_count,
            last_error,
            degraded: failure_count > 0,
        }
    }
}

/// Persistent, file-per-trace JSONL sink.
///
/// Cheap to clone — all shared state lives behind an [`Arc`]/[`Mutex`],
/// so multiple handles refer to the same bag of open writers.
#[derive(Clone)]
pub struct JsonlTraceSink {
    root: PathBuf,
    inner: Arc<Mutex<Inner>>,
    clock: Clock,
    failures: Arc<FailureState>,
}

struct Inner {
    /// One open writer per live trace, keyed by its [`TraceId`].
    writers: HashMap<TraceId, TraceWriter>,
}

struct TraceWriter {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl JsonlTraceSink {
    /// Construct a sink from a workspace root directory.
    ///
    /// Traces are persisted under `<workdir>/.roko/traces/`.
    #[must_use]
    pub fn for_workdir(workdir: impl AsRef<Path>) -> Self {
        Self::new(workdir.as_ref().join(DEFAULT_TRACE_DIR_REL_PATH))
    }

    /// Construct a sink from an existing `.roko/` directory.
    ///
    /// Traces are persisted under `<roko_dir>/traces/`.
    #[must_use]
    pub fn for_roko_dir(roko_dir: impl AsRef<Path>) -> Self {
        Self::new(roko_dir.as_ref().join("traces"))
    }

    /// Construct a sink rooted at `root` (typically `.roko/traces/`).
    ///
    /// Directories are created on first append; nothing is touched until
    /// a trace event arrives.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            inner: Arc::new(Mutex::new(Inner {
                writers: HashMap::new(),
            })),
            clock: Arc::new(chrono::Utc::now),
            failures: Arc::new(FailureState::new()),
        }
    }

    /// Construct a sink with a custom clock (for tests).
    #[must_use]
    pub fn with_clock(root: impl Into<PathBuf>, clock: Clock) -> Self {
        Self {
            root: root.into(),
            inner: Arc::new(Mutex::new(Inner {
                writers: HashMap::new(),
            })),
            clock,
            failures: Arc::new(FailureState::new()),
        }
    }

    /// Filesystem root directory for traces.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Is the in-memory writer bag empty? (primarily for tests)
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().writers.is_empty()
    }

    /// Return the current health/degradation state of this sink (T034).
    ///
    /// A non-zero `failure_count` indicates silent data loss has occurred.
    /// Callers should surface this in `roko doctor` or dashboard health checks.
    #[must_use]
    pub fn degraded_state(&self) -> TraceSinkHealth {
        self.failures.snapshot()
    }

    /// Flush all open trace writers to disk without closing them.
    ///
    /// This ensures buffered data from in-progress traces is persisted even
    /// when the run terminates before every trace calls [`TraceSink::finish`].
    /// Best-effort: individual flush failures are silently ignored.
    pub fn flush_all(&self) {
        let mut inner = self.inner.lock();
        for writer in inner.writers.values_mut() {
            let _ = writer.writer.flush();
        }
    }

    /// Ensure a writer exists for `trace_id`; open one if missing.
    ///
    /// Returns `None` if the file could not be opened (logged + swallowed).
    fn ensure_writer<'a>(
        &self,
        inner: &'a mut Inner,
        trace_id: TraceId,
    ) -> Option<&'a mut TraceWriter> {
        use std::collections::hash_map::Entry;
        match inner.writers.entry(trace_id) {
            Entry::Occupied(e) => Some(e.into_mut()),
            Entry::Vacant(v) => {
                let now = (self.clock)();
                let date_dir = self.root.join(now.format("%Y-%m-%d").to_string());
                if let Err(e) = std::fs::create_dir_all(&date_dir) {
                    self.failures
                        .record_error(format!("failed to create {}: {e}", date_dir.display()));
                    return None;
                }
                let path = date_dir.join(format!("{}.jsonl", trace_id.to_hex()));
                let file = match OpenOptions::new().create(true).append(true).open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        self.failures
                            .record_error(format!("failed to open {}: {e}", path.display()));
                        return None;
                    }
                };
                Some(v.insert(TraceWriter {
                    path,
                    writer: BufWriter::new(file),
                }))
            }
        }
    }
}

impl std::fmt::Debug for JsonlTraceSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let health = self.failures.snapshot();
        f.debug_struct("JsonlTraceSink")
            .field("root", &self.root)
            .field("open_writers", &self.inner.lock().writers.len())
            .field("failure_count", &health.failure_count)
            .field("clock", &"<dyn Fn>")
            .finish()
    }
}

impl TraceSink for JsonlTraceSink {
    fn append(&self, trace_id: TraceId, event: ToolTraceEvent) {
        let line = match serde_json::to_string(&event) {
            Ok(s) => s,
            Err(e) => {
                self.failures
                    .record_error(format!("failed to serialize event: {e}"));
                return;
            }
        };
        let mut inner = self.inner.lock();
        let Some(writer) = self.ensure_writer(&mut inner, trace_id) else {
            return;
        };
        if let Err(e) = writer.writer.write_all(line.as_bytes()) {
            self.failures
                .record_error(format!("write failed ({}): {e}", writer.path.display()));
            return;
        }
        if let Err(e) = writer.writer.write_all(b"\n") {
            self.failures
                .record_error(format!("write failed ({}): {e}", writer.path.display()));
        }
    }

    fn finish(&self, trace: ToolTrace) {
        let trace_id = trace.trace_id;
        let line = match serde_json::to_string(&trace) {
            Ok(s) => s,
            Err(e) => {
                self.failures
                    .record_error(format!("failed to serialize trace: {e}"));
                return;
            }
        };
        let mut inner = self.inner.lock();
        if let Some(writer) = self.ensure_writer(&mut inner, trace_id) {
            if let Err(e) = writer.writer.write_all(line.as_bytes()) {
                self.failures.record_error(format!(
                    "finish write failed ({}): {e}",
                    writer.path.display()
                ));
            }
            if let Err(e) = writer.writer.write_all(b"\n") {
                self.failures.record_error(format!(
                    "finish write failed ({}): {e}",
                    writer.path.display()
                ));
            }
            if let Err(e) = writer.writer.flush() {
                self.failures
                    .record_error(format!("flush failed ({}): {e}", writer.path.display()));
            }
        }
        // Drop the writer to release the file handle.
        inner.writers.remove(&trace_id);
    }
}

/// Convenience factory: create a [`JsonlTraceSink`] rooted at
/// `roko_dir/traces/` — the conventional location for tool-trace
/// persistence (§36.99).
///
/// The returned trait object is ready for injection into a
/// [`TraceBuilder`](roko_core::tool::trace::TraceBuilder) or any
/// consumer that accepts `Box<dyn TraceSink>`.
#[must_use]
pub fn default_trace_sink(roko_dir: &Path) -> Box<dyn TraceSink> {
    let root = roko_dir.join("traces");
    Box::new(JsonlTraceSink::new(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;
    use roko_core::tool::ToolFormat;
    use roko_core::tool::trace::{CancelSource, ToolOutcome};
    use std::fs;
    use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

    fn trace_id(byte: u8) -> TraceId {
        TraceId::from_bytes([byte; 16])
    }

    fn fixed_clock(secs: i64) -> Clock {
        Arc::new(move || {
            chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0).unwrap_or_default()
        })
    }

    fn make_trace(id: TraceId, started_at_ms: i64) -> ToolTrace {
        ToolTrace {
            trace_id: id,
            call_id: "call-1".to_string(),
            role: AgentRole::Implementer,
            model: "mock".to_string(),
            format_used: ToolFormat::OpenAiJson,
            started_at_ms,
            ended_at_ms: started_at_ms + 10,
            events: Vec::new(),
            outcome: ToolOutcome::success(10, 0.0),
            enforcement_owner: None,
            policy_owner: None,
        }
    }

    #[test]
    fn for_workdir_uses_default_trace_path() {
        let sink = JsonlTraceSink::for_workdir("/repo");
        assert_eq!(sink.root(), Path::new("/repo/.roko/traces"));
    }

    #[test]
    fn for_roko_dir_uses_traces_subdir() {
        let sink = JsonlTraceSink::for_roko_dir("/repo/.roko");
        assert_eq!(sink.root(), Path::new("/repo/.roko/traces"));
    }

    #[test]
    fn append_and_finish_writes_jsonl_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sink = JsonlTraceSink::with_clock(tmp.path(), fixed_clock(1_700_000_000));
        let id = trace_id(0x11);

        for i in 0..5 {
            sink.append(
                id,
                ToolTraceEvent::StreamCoerced {
                    at_ms: 1_700_000_000_000 + i,
                },
            );
        }
        sink.finish(make_trace(id, 1_700_000_000_000));

        // File should exist under the clocked date directory.
        let date_dir = tmp.path().join("2023-11-14"); // 1700000000 = 2023-11-14 UTC
        assert!(date_dir.is_dir(), "date dir should exist: {date_dir:?}");
        let file = date_dir.join(format!("{}.jsonl", id.to_hex()));
        assert!(file.is_file(), "trace file should exist: {file:?}");

        // Should contain 5 event lines + 1 trace summary line = 6 lines.
        let contents = fs::read_to_string(&file).expect("read file");
        let line_count = contents.lines().count();
        assert_eq!(line_count, 6, "expected 6 lines, got: {contents}");

        // Writer should be closed after finish.
        assert!(sink.is_empty());
    }

    #[test]
    fn midnight_rotation_uses_new_directory_for_new_trace() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let clock_secs = Arc::new(AtomicI64::new(1_700_000_000)); // 2023-11-14 UTC
        let clock_secs_inner = Arc::clone(&clock_secs);
        let clock: Clock = Arc::new(move || {
            let secs = clock_secs_inner.load(AtomicOrdering::Relaxed);
            chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0).unwrap_or_default()
        });
        let sink = JsonlTraceSink::with_clock(tmp.path(), clock);

        // First trace on day 1.
        let id_a = trace_id(0x01);
        sink.append(id_a, ToolTraceEvent::StreamCoerced { at_ms: 1 });
        sink.finish(make_trace(id_a, 1));

        // Advance the clock 1 day.
        clock_secs.fetch_add(86_400, AtomicOrdering::Relaxed);

        // Second trace on day 2.
        let id_b = trace_id(0x02);
        sink.append(id_b, ToolTraceEvent::StreamCoerced { at_ms: 2 });
        sink.finish(make_trace(id_b, 2));

        assert!(tmp.path().join("2023-11-14").is_dir());
        assert!(tmp.path().join("2023-11-15").is_dir());
        assert!(
            tmp.path()
                .join("2023-11-14")
                .join(format!("{}.jsonl", id_a.to_hex()))
                .is_file()
        );
        assert!(
            tmp.path()
                .join("2023-11-15")
                .join(format!("{}.jsonl", id_b.to_hex()))
                .is_file()
        );
    }

    #[test]
    fn two_concurrent_traces_produce_two_non_interleaved_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sink = JsonlTraceSink::with_clock(tmp.path(), fixed_clock(1_700_000_000));
        let id_a = trace_id(0xAA);
        let id_b = trace_id(0xBB);

        // Interleave appends between the two traces.
        sink.append(id_a, ToolTraceEvent::StreamCoerced { at_ms: 1 });
        sink.append(id_b, ToolTraceEvent::StreamCoerced { at_ms: 2 });
        sink.append(
            id_a,
            ToolTraceEvent::Cancellation {
                source: CancelSource::UserAbort,
                at_ms: 3,
            },
        );
        sink.append(id_b, ToolTraceEvent::StreamCoerced { at_ms: 4 });
        sink.finish(make_trace(id_a, 1));
        sink.finish(make_trace(id_b, 2));

        let date_dir = tmp.path().join("2023-11-14");
        let file_a = date_dir.join(format!("{}.jsonl", id_a.to_hex()));
        let file_b = date_dir.join(format!("{}.jsonl", id_b.to_hex()));

        // Each file contains only its own events + its own trace summary.
        let contents_a = fs::read_to_string(&file_a).expect("read a");
        let contents_b = fs::read_to_string(&file_b).expect("read b");
        assert_eq!(contents_a.lines().count(), 3); // 2 events + 1 trace summary
        assert_eq!(contents_b.lines().count(), 3); // 2 events + 1 trace summary
        // File A should contain the cancellation event, file B should not.
        assert!(contents_a.contains("cancellation"));
        assert!(!contents_b.contains("cancellation"));
    }

    #[test]
    fn round_trip_events_parse_back() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sink = JsonlTraceSink::with_clock(tmp.path(), fixed_clock(1_700_000_000));
        let id = trace_id(0xCC);

        let events = vec![
            ToolTraceEvent::StreamCoerced { at_ms: 100 },
            ToolTraceEvent::Truncation {
                kept: 50,
                total: 100,
                at_ms: 101,
            },
            ToolTraceEvent::StreamCoerced { at_ms: 102 },
        ];
        for e in &events {
            sink.append(id, e.clone());
        }
        sink.finish(make_trace(id, 100));

        let file = tmp
            .path()
            .join("2023-11-14")
            .join(format!("{}.jsonl", id.to_hex()));
        let contents = fs::read_to_string(&file).expect("read");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 4);
        for (i, line) in lines.iter().take(3).enumerate() {
            let parsed: ToolTraceEvent = serde_json::from_str(line).expect("parse event line");
            assert_eq!(parsed, events[i]);
        }
        // Final line is the ToolTrace summary.
        let _parsed: ToolTrace = serde_json::from_str(lines[3]).expect("parse trace line");
    }

    #[test]
    fn finish_without_prior_append_still_writes_trace_line() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sink = JsonlTraceSink::with_clock(tmp.path(), fixed_clock(1_700_000_000));
        let id = trace_id(0xDD);
        sink.finish(make_trace(id, 1));

        let file = tmp
            .path()
            .join("2023-11-14")
            .join(format!("{}.jsonl", id.to_hex()));
        assert!(file.is_file());
        let contents = fs::read_to_string(&file).expect("read");
        assert_eq!(contents.lines().count(), 1);
    }

    #[test]
    fn default_trace_sink_creates_jsonl_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sink = super::default_trace_sink(tmp.path());
        let id = trace_id(0xEE);

        // Emit one event and finish.
        sink.append(id, ToolTraceEvent::StreamCoerced { at_ms: 1 });
        sink.finish(make_trace(id, 1));

        // The traces/ subdirectory should have been created and contain a file.
        let traces_dir = tmp.path().join("traces");
        assert!(
            traces_dir.is_dir(),
            "traces/ dir must exist: {traces_dir:?}"
        );

        // Walk into the date directory and find the JSONL file.
        let entries: Vec<_> = fs::read_dir(&traces_dir)
            .expect("read traces dir")
            .filter_map(Result::ok)
            .collect();
        assert!(
            !entries.is_empty(),
            "should have at least one date directory"
        );
        let date_dir = &entries[0].path();
        let jsonl_file = date_dir.join(format!("{}.jsonl", id.to_hex()));
        assert!(
            jsonl_file.is_file(),
            "JSONL file must exist: {jsonl_file:?}"
        );

        let contents = fs::read_to_string(&jsonl_file).expect("read");
        assert_eq!(contents.lines().count(), 2, "1 event + 1 trace summary");
    }

    #[test]
    fn flush_all_persists_buffered_data_without_closing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sink = JsonlTraceSink::with_clock(tmp.path(), fixed_clock(1_700_000_000));
        let id = trace_id(0xFF);

        // Append events but do NOT call finish.
        sink.append(id, ToolTraceEvent::StreamCoerced { at_ms: 1 });
        sink.append(id, ToolTraceEvent::StreamCoerced { at_ms: 2 });

        // flush_all should persist buffered bytes without closing the writer.
        sink.flush_all();

        // The file should exist and contain the two event lines.
        let date_dir = tmp.path().join("2023-11-14");
        let file = date_dir.join(format!("{}.jsonl", id.to_hex()));
        assert!(file.is_file(), "trace file should exist after flush_all");
        let contents = fs::read_to_string(&file).expect("read");
        assert_eq!(
            contents.lines().count(),
            2,
            "expected 2 event lines after flush_all"
        );

        // Writer should still be open (not empty).
        assert!(
            !sink.is_empty(),
            "writer bag must not be empty after flush_all"
        );
    }

    // ── T034 tests ──────────────────────────────────────────────────────────

    #[test]
    fn degraded_state_starts_healthy() {
        let sink = JsonlTraceSink::new("/nonexistent/traces");
        let health = sink.degraded_state();
        assert_eq!(health.failure_count, 0);
        assert!(!health.degraded);
        assert!(health.last_error.is_none());
    }

    #[test]
    fn degraded_state_records_write_failures() {
        // Use a path that will fail to create directories.
        let sink =
            JsonlTraceSink::with_clock("/dev/null/cannot/create/dirs", fixed_clock(1_700_000_000));
        let id = trace_id(0x01);

        // This should fail (can't create directories under /dev/null).
        sink.append(id, ToolTraceEvent::StreamCoerced { at_ms: 1 });

        let health = sink.degraded_state();
        assert!(health.degraded, "should be degraded after failed append");
        assert!(health.failure_count > 0);
        assert!(health.last_error.is_some());
    }

    #[test]
    fn degraded_state_serializes() {
        let health = TraceSinkHealth {
            failure_count: 3,
            last_error: Some("disk full".to_string()),
            degraded: true,
        };
        let json = serde_json::to_string(&health).expect("serialize");
        let decoded: TraceSinkHealth = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, health);
    }
}
