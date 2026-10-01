//! Append-only JSONL audit log for tool dispatches (§36.52).
//!
//! Every admitted tool call emits one `{"kind":"admit",...}` line;
//! every terminal result emits one `{"kind":"result",...}` line. The
//! file is safe to `tail -f` for live observability and safe to replay
//! for post-hoc audit.
//!
//! # Design
//!
//! - Single file: `<root>/.roko/tool_audit.jsonl`.
//! - Append-only, create-if-missing, line-buffered via `BufWriter`.
//! - `tokio::sync::Mutex<BufWriter<File>>` — short critical section.
//! - Pure sink: records facts, never rejects or mutates the call.
//!
//! # Scrubbed adapter (T028)
//!
//! [`ScrubAuditAdapter`] wraps a `ToolAuditLog` and a `LogScrubber`,
//! sanitizing tool arguments and result content before persistence.
//! Raw `ToolCall.arguments` are never written — only bounded, scrubbed
//! representations land on disk.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use roko_core::obs::LogScrubber;
use roko_core::tool::{CorrelationEnvelope, ToolCall, ToolResult};

/// The default audit-log path relative to the worktree.
pub const DEFAULT_AUDIT_PATH: &str = ".roko/tool_audit.jsonl";

/// Maximum byte length for serialized tool arguments in audit records.
/// Arguments exceeding this are truncated with a `[truncated]` marker.
const MAX_ARGUMENTS_BYTES: usize = 4096;

/// Maximum byte length of each correlation id in an audit record.
const MAX_CORRELATION_ID_BYTES: usize = 256;

// ─── Wire types ───────────────────────────────────────────────────────────────

/// A single audit-log line. The `kind` discriminator is written as a JSON
/// field so consumers can distinguish admit lines from result lines while
/// `tail -f`-ing a single file.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditLine {
    /// A tool call was admitted for execution.
    Admit {
        /// Timestamp in milliseconds.
        ts_ms: i64,
        /// Provider-assigned call identifier.
        call_id: String,
        /// Canonical tool name.
        call_name: String,
        /// Scrubbed argument summary.
        arguments_scrubbed: String,
        /// The run, task, attempt and agent the call belongs to; omitted
        /// when the dispatcher knew none.
        #[serde(default, skip_serializing_if = "is_uncorrelated")]
        correlation: CorrelationEnvelope,
    },
    /// A tool call completed with a result.
    Result {
        /// Timestamp in milliseconds.
        ts_ms: i64,
        /// Provider-assigned call identifier.
        call_id: String,
        /// Canonical tool name.
        call_name: String,
        /// Whether the call succeeded.
        ok: bool,
        /// Scrubbed result content.
        content_scrubbed: String,
        /// The run, task, attempt and agent the call belongs to; omitted
        /// when the dispatcher knew none.
        #[serde(default, skip_serializing_if = "is_uncorrelated")]
        correlation: CorrelationEnvelope,
    },
}

/// Whether `correlation` names nothing, so a line can leave it out.
fn is_uncorrelated(correlation: &CorrelationEnvelope) -> bool {
    *correlation == CorrelationEnvelope::empty()
}

/// A single raw (unscrubbed) audit-log line — internal only.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RawAuditLine<'a> {
    Admit {
        ts_ms: i64,
        call: &'a ToolCall,
    },
    Result {
        ts_ms: i64,
        call_id: &'a str,
        call_name: &'a str,
        result: &'a ToolResult,
    },
}

// ─── ToolAuditLog ─────────────────────────────────────────────────────────────

/// Append-only JSONL audit log for tool dispatches.
///
/// Each write acquires a per-file advisory lock (`tool_audit.jsonl.lock`) via
/// [`fs2::FileExt::lock_exclusive`] before opening, writing, syncing, and
/// closing the target file.  This ensures that concurrent processes writing to
/// the same workspace `.roko/tool_audit.jsonl` never interleave bytes.
///
/// In-process serialization is provided by the same lock: a `spawn_blocking`
/// call serializes the blocking I/O onto the thread pool.
///
/// Cheap to clone via [`std::sync::Arc`]; wrap in `Arc<ToolAuditLog>` to
/// share across tasks.
pub struct ToolAuditLog {
    path: PathBuf,
}

impl std::fmt::Debug for ToolAuditLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolAuditLog")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl ToolAuditLog {
    /// Open (or create) the audit log under `<root>/.roko/tool_audit.jsonl`.
    ///
    /// Creates `.roko/` if it does not exist.  The file is written lazily
    /// on the first record — `open` only ensures the parent directory exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    pub async fn open(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = root.as_ref().join(DEFAULT_AUDIT_PATH);
        Self::open_at(path).await
    }

    /// Open the audit log at an explicit path (no `.roko/` suffix is added).
    ///
    /// Parent directories are created if missing.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    pub async fn open_at(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(Self { path })
    }

    /// Path to the underlying JSONL file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Record that a call was admitted for dispatch.
    ///
    /// Writes one `{"kind":"admit","ts_ms":…,"call":…}` line and syncs to
    /// disk under an exclusive per-file advisory lock.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or the write fails.
    pub async fn record_admit(&self, call: &ToolCall) -> std::io::Result<()> {
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let line = RawAuditLine::Admit { ts_ms, call };
        let bytes = Self::serialize_line(&line)?;
        self.write_line(bytes).await
    }

    /// Record the terminal result of a dispatched call.
    ///
    /// Writes one `{"kind":"result","ts_ms":…,"call_id":…,"call_name":…,"result":…}` line
    /// and syncs under an exclusive per-file advisory lock.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or the write fails.
    pub async fn record_result(&self, call: &ToolCall, result: &ToolResult) -> std::io::Result<()> {
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let line = RawAuditLine::Result {
            ts_ms,
            call_id: &call.id,
            call_name: &call.name,
            result,
        };
        let bytes = Self::serialize_line(&line)?;
        self.write_line(bytes).await
    }

    /// Record a pre-scrubbed audit line directly.
    ///
    /// Used by [`ScrubAuditAdapter`] to write lines that have already been
    /// sanitized through a [`LogScrubber`].
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or the write fails.
    pub async fn record_scrubbed(&self, line: &AuditLine) -> std::io::Result<()> {
        let bytes = Self::serialize_line(line)?;
        self.write_line(bytes).await
    }

    /// No-op flush kept for API compatibility.
    ///
    /// `record_admit`, `record_result`, and `record_scrubbed` all sync to
    /// disk before returning, so there is no in-memory buffer to flush.
    pub async fn flush(&self) -> std::io::Result<()> {
        Ok(())
    }

    // ─── private ─────────────────────────────────────────────────────────────

    /// Serialize `value` to JSONL bytes with a trailing newline.
    fn serialize_line(value: &impl Serialize) -> std::io::Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// Write `bytes` to the audit JSONL file while holding the per-file
    /// advisory lock (`<path>.lock`) so that concurrent processes cannot
    /// interleave partial records.
    ///
    /// The sequence is:
    ///   1. acquire `<path>.lock` (blocking, via `fs2::FileExt::lock_exclusive`)
    ///   2. open (or create) the target file in append mode
    ///   3. write all bytes
    ///   4. sync_data
    ///   5. drop file handle (close)
    ///   6. drop lock file handle (unlock)
    async fn write_line(&self, bytes: Vec<u8>) -> std::io::Result<()> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            // Ensure parent directory exists (in case it was removed after open).
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Acquire exclusive advisory lock for this file.
            let _lock = crate::log_rotation::lock_jsonl(&path)?;
            // Append under the lock.
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            file.write_all(&bytes)?;
            file.sync_data()
        })
        .await
        .map_err(|e| std::io::Error::other(format!("tool audit write task failed: {e}")))?
    }
}

// ─── ScrubAuditAdapter (T028) ────────────────────────────────────────────────

/// Adapter that sanitizes tool call arguments and result content before
/// writing to a [`ToolAuditLog`].
///
/// Raw `ToolCall.arguments` JSON is never persisted. Instead, the adapter:
/// 1. Serializes the arguments to a string
/// 2. Truncates to [`MAX_ARGUMENTS_BYTES`]
/// 3. Runs the string through a [`LogScrubber`] to redact secrets
///
/// Similarly, result content is scrubbed before persistence.
#[derive(Debug)]
pub struct ScrubAuditAdapter {
    log: Arc<ToolAuditLog>,
    scrubber: Arc<LogScrubber>,
}

impl ScrubAuditAdapter {
    /// Create a new adapter wrapping the given log and scrubber.
    #[must_use]
    pub fn new(log: Arc<ToolAuditLog>, scrubber: Arc<LogScrubber>) -> Self {
        Self { log, scrubber }
    }

    /// Record an admitted call with scrubbed arguments, joined to the run
    /// by `correlation`.
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails.
    pub async fn record_admit(
        &self,
        call: &ToolCall,
        correlation: &CorrelationEnvelope,
    ) -> std::io::Result<()> {
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let arguments_raw = serde_json::to_string(&call.arguments).unwrap_or_default();
        let arguments_bounded = truncate_str(&arguments_raw, MAX_ARGUMENTS_BYTES);
        let arguments_scrubbed = self.scrubber.scrub(arguments_bounded);

        let line = AuditLine::Admit {
            ts_ms,
            call_id: call.id.clone(),
            call_name: call.name.clone(),
            arguments_scrubbed,
            correlation: bounded_correlation(correlation),
        };
        self.log.record_scrubbed(&line).await
    }

    /// Record a terminal result with scrubbed content, joined to the run by
    /// `correlation`.
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails.
    pub async fn record_result(
        &self,
        call: &ToolCall,
        result: &ToolResult,
        correlation: &CorrelationEnvelope,
    ) -> std::io::Result<()> {
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let (ok, raw_content) = match result {
            ToolResult::Ok { .. } => (true, result.text_content()),
            ToolResult::Err(e) => (false, format!("{e}")),
        };
        let content_bounded = truncate_str(&raw_content, MAX_ARGUMENTS_BYTES);
        let content_scrubbed = self.scrubber.scrub(content_bounded);

        let line = AuditLine::Result {
            ts_ms,
            call_id: call.id.clone(),
            call_name: call.name.clone(),
            ok,
            content_scrubbed,
            correlation: bounded_correlation(correlation),
        };
        self.log.record_scrubbed(&line).await
    }

    /// Reference to the underlying audit log.
    #[must_use]
    pub fn inner(&self) -> &Arc<ToolAuditLog> {
        &self.log
    }
}

/// `correlation` with each id cut to [`MAX_CORRELATION_ID_BYTES`], so an
/// oversized id cannot bloat the audit record.
fn bounded_correlation(correlation: &CorrelationEnvelope) -> CorrelationEnvelope {
    fn bound(id: &str) -> String {
        truncate_str(id, MAX_CORRELATION_ID_BYTES).to_string()
    }
    CorrelationEnvelope {
        run_id: bound(&correlation.run_id),
        task_id: bound(&correlation.task_id),
        attempt_id: bound(&correlation.attempt_id),
        turn_id: bound(&correlation.turn_id),
        agent_id: bound(&correlation.agent_id),
    }
}

/// Truncate a string to at most `max_bytes` UTF-8 bytes, appending
/// `[truncated]` if it was shortened.
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Find the last char boundary at or before max_bytes.
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    // ── helpers ────────────────────────────────────────────────────────────

    fn make_call(id: &str, name: &str) -> ToolCall {
        ToolCall::at(id, name, serde_json::json!({"x": 1}), 1_700_000_000_000)
    }

    fn make_result_ok() -> ToolResult {
        ToolResult::text("output text")
    }

    async fn read_lines(path: &Path) -> Vec<String> {
        let contents = tokio::fs::read_to_string(path).await.expect("read file");
        contents
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(String::from)
            .collect()
    }

    // ── 1 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn open_creates_jsonl_file_under_roko_dir() {
        let dir = TempDir::new().expect("tempdir");
        let log = ToolAuditLog::open(dir.path()).await.expect("open");

        assert_eq!(log.path(), dir.path().join(".roko/tool_audit.jsonl"));
        // The parent directory is created eagerly; the file itself is created
        // lazily on the first write.
        assert!(
            log.path().parent().expect("parent dir").exists(),
            "parent .roko dir should exist after open"
        );
    }

    // ── 2 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn record_admit_writes_one_line() {
        let dir = TempDir::new().expect("tempdir");
        let log = ToolAuditLog::open(dir.path()).await.expect("open");
        let call = make_call("c1", "read_file");

        log.record_admit(&call).await.expect("record_admit");

        let lines = read_lines(log.path()).await;
        assert_eq!(lines.len(), 1, "expected exactly 1 line, got: {lines:?}");
    }

    // ── 3 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn record_result_writes_one_line() {
        let dir = TempDir::new().expect("tempdir");
        let log = ToolAuditLog::open(dir.path()).await.expect("open");
        let call = make_call("c2", "write_file");
        let result = make_result_ok();

        log.record_result(&call, &result)
            .await
            .expect("record_result");

        let lines = read_lines(log.path()).await;
        assert_eq!(lines.len(), 1, "expected exactly 1 line, got: {lines:?}");
    }

    // ── 4 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn admit_line_has_kind_admit_and_call_fields() {
        let dir = TempDir::new().expect("tempdir");
        let log = ToolAuditLog::open(dir.path()).await.expect("open");
        let call = make_call("c3", "bash");

        log.record_admit(&call).await.expect("record_admit");

        let lines = read_lines(log.path()).await;
        let json: serde_json::Value = serde_json::from_str(&lines[0]).expect("parse line as JSON");

        assert_eq!(json["kind"], "admit", "kind must be 'admit'");
        assert!(json["ts_ms"].is_i64(), "ts_ms must be an integer");
        assert_eq!(json["call"]["id"], "c3", "call.id must match");
        assert_eq!(json["call"]["name"], "bash", "call.name must match");
    }

    // ── 5 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn result_line_has_kind_result_and_call_id() {
        let dir = TempDir::new().expect("tempdir");
        let log = ToolAuditLog::open(dir.path()).await.expect("open");
        let call = make_call("c4", "grep");
        let result = make_result_ok();

        log.record_result(&call, &result)
            .await
            .expect("record_result");

        let lines = read_lines(log.path()).await;
        let json: serde_json::Value = serde_json::from_str(&lines[0]).expect("parse line as JSON");

        assert_eq!(json["kind"], "result", "kind must be 'result'");
        assert_eq!(json["call_id"], "c4", "call_id must match");
        assert_eq!(json["call_name"], "grep", "call_name must match");
        assert!(json["ts_ms"].is_i64(), "ts_ms must be an integer");
        assert!(json["result"].is_object(), "result must be an object");
    }

    // ── 6 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn interleaved_writes_preserve_order() {
        let dir = TempDir::new().expect("tempdir");
        let log = ToolAuditLog::open(dir.path()).await.expect("open");
        let call_a = make_call("a", "read_file");
        let call_b = make_call("b", "write_file");
        let result = make_result_ok();

        log.record_admit(&call_a).await.expect("admit a");
        log.record_result(&call_a, &result).await.expect("result a");
        log.record_admit(&call_b).await.expect("admit b");
        log.record_result(&call_b, &result).await.expect("result b");

        let lines = read_lines(log.path()).await;
        assert_eq!(lines.len(), 4, "expected 4 lines");

        let kinds: Vec<&str> = lines
            .iter()
            .map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).expect("valid json");
                match v["kind"].as_str().expect("kind string") {
                    "admit" => "admit",
                    "result" => "result",
                    other => panic!("unexpected kind: {other}"),
                }
            })
            .collect();
        assert_eq!(kinds, ["admit", "result", "admit", "result"]);
    }

    // ── 7 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn reopen_appends_not_truncates() {
        let dir = TempDir::new().expect("tempdir");

        // First session: 3 lines.
        {
            let log = ToolAuditLog::open(dir.path()).await.expect("open first");
            let call = make_call("x", "glob");
            let result = make_result_ok();
            log.record_admit(&call).await.expect("admit 1");
            log.record_admit(&call).await.expect("admit 2");
            log.record_result(&call, &result).await.expect("result 1");
        }

        // Second session: 2 more lines.
        {
            let log = ToolAuditLog::open(dir.path()).await.expect("open second");
            let call = make_call("y", "ls");
            let result = make_result_ok();
            log.record_admit(&call).await.expect("admit 3");
            log.record_result(&call, &result).await.expect("result 2");
        }

        // Must see all 5 lines.
        let path = dir.path().join(".roko/tool_audit.jsonl");
        let lines = read_lines(&path).await;
        assert_eq!(lines.len(), 5, "expected 5 total lines after reopen");
    }

    // ── 8 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn nested_subdir_already_exists_is_ok() {
        let dir = TempDir::new().expect("tempdir");
        // Pre-create the `.roko/` directory.
        tokio::fs::create_dir_all(dir.path().join(".roko"))
            .await
            .expect("pre-create .roko");

        let log = ToolAuditLog::open(dir.path())
            .await
            .expect("open with pre-existing dir");
        let call = make_call("pre", "bash");
        log.record_admit(&call).await.expect("record_admit");

        let lines = read_lines(log.path()).await;
        assert_eq!(lines.len(), 1);
    }

    // ── 9 ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn open_at_uses_exact_path() {
        let dir = TempDir::new().expect("tempdir");
        let explicit = dir.path().join("my_custom_audit.jsonl");

        let log = ToolAuditLog::open_at(&explicit).await.expect("open_at");
        assert_eq!(
            log.path(),
            explicit,
            "path must match exactly — no .roko/ suffix"
        );

        let call = make_call("e", "edit_file");
        log.record_admit(&call).await.expect("record_admit");

        // The file lives at the explicit path, NOT under .roko/.
        assert!(explicit.exists(), "file must exist at explicit path");
        assert!(
            !dir.path().join(".roko/my_custom_audit.jsonl").exists(),
            "must not create .roko/ suffix"
        );
        let lines = read_lines(&explicit).await;
        assert_eq!(lines.len(), 1);
    }

    // ── 10 ─────────────────────────────────────────────────────────────────

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_writes_do_not_interleave_bytes() {
        let dir = TempDir::new().expect("tempdir");
        let log = Arc::new(ToolAuditLog::open(dir.path()).await.expect("open"));

        let tasks = 10usize;
        let admits_per_task = 5usize;

        let mut handles = Vec::with_capacity(tasks);
        for task_idx in 0..tasks {
            let log = Arc::clone(&log);
            handles.push(tokio::spawn(async move {
                for call_idx in 0..admits_per_task {
                    let call = make_call(&format!("t{task_idx}-c{call_idx}"), "bash");
                    log.record_admit(&call).await.expect("concurrent admit");
                }
            }));
        }
        for handle in handles {
            handle.await.expect("task panicked");
        }

        let lines = read_lines(log.path()).await;
        let expected = tasks * admits_per_task;
        assert_eq!(
            lines.len(),
            expected,
            "expected {expected} lines, got {}",
            lines.len()
        );

        // Every line must parse as valid JSON — no torn writes.
        for (i, line) in lines.iter().enumerate() {
            let _: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line {i} is not valid JSON: {e}\nLine: {line}"));
        }
    }

    // ── 11 ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn ts_ms_is_reasonable() {
        let dir = TempDir::new().expect("tempdir");
        let log = ToolAuditLog::open(dir.path()).await.expect("open");
        let call = make_call("ts", "read_file");

        let before = chrono::Utc::now().timestamp_millis();
        log.record_admit(&call).await.expect("record_admit");
        let after = chrono::Utc::now().timestamp_millis();

        let lines = read_lines(log.path()).await;
        let json: serde_json::Value = serde_json::from_str(&lines[0]).expect("parse JSON");
        let ts_ms = json["ts_ms"].as_i64().expect("ts_ms is i64");

        assert!(
            ts_ms >= before && ts_ms <= after + 1_000,
            "ts_ms {ts_ms} should be within 1s of now (before={before}, after={after})"
        );
    }

    // ── 12 ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn each_line_parses_as_valid_json() {
        let dir = TempDir::new().expect("tempdir");
        let log = ToolAuditLog::open(dir.path()).await.expect("open");
        let call_a = make_call("j1", "bash");
        let call_b = make_call("j2", "glob");
        let result = ToolResult::err(roko_core::tool::ToolError::Cancelled);

        log.record_admit(&call_a).await.expect("admit a");
        log.record_result(&call_a, &make_result_ok())
            .await
            .expect("result a");
        log.record_admit(&call_b).await.expect("admit b");
        log.record_result(&call_b, &result)
            .await
            .expect("result b (err)");

        let lines = read_lines(log.path()).await;
        assert_eq!(lines.len(), 4, "expected 4 lines");
        for (i, line) in lines.iter().enumerate() {
            let _: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line {i} is invalid JSON: {e}\nLine: {line}"));
        }
    }

    // ── 13 (T028) ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn scrub_adapter_redacts_arguments() {
        let dir = TempDir::new().expect("tempdir");
        let log = Arc::new(ToolAuditLog::open(dir.path()).await.expect("open"));
        let scrubber = Arc::new(LogScrubber::new());
        let adapter = ScrubAuditAdapter::new(log.clone(), scrubber);

        let call = ToolCall::at(
            "s1",
            "bash",
            serde_json::json!({
                "command": "curl -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.sig' http://api.example.com"
            }),
            1_700_000_000_000,
        );

        let correlation = CorrelationEnvelope {
            run_id: "run-1".to_string(),
            task_id: "T01".to_string(),
            attempt_id: "run-1:p:T01:1".to_string(),
            turn_id: String::new(),
            agent_id: "a".repeat(1_000),
        };
        adapter
            .record_admit(&call, &correlation)
            .await
            .expect("scrub admit");

        let lines = read_lines(log.path()).await;
        assert_eq!(lines.len(), 1);
        let json: serde_json::Value = serde_json::from_str(&lines[0]).expect("parse JSON");
        assert_eq!(json["kind"], "admit");
        assert_eq!(json["call_id"], "s1");
        assert_eq!(json["call_name"], "bash");
        // The line names its run, task and attempt, with each id bounded.
        assert_eq!(json["correlation"]["run_id"], "run-1");
        assert_eq!(json["correlation"]["task_id"], "T01");
        assert_eq!(json["correlation"]["attempt_id"], "run-1:p:T01:1");
        assert_eq!(
            json["correlation"]["agent_id"].as_str().map(str::len),
            Some(MAX_CORRELATION_ID_BYTES)
        );
        // The Bearer token must be redacted.
        let args = json["arguments_scrubbed"]
            .as_str()
            .expect("arguments_scrubbed");
        assert!(
            !args.contains("eyJhbGciOi"),
            "Bearer token must be scrubbed from arguments"
        );
    }

    // ── 14 (T028) ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn scrub_adapter_redacts_result_content() {
        let dir = TempDir::new().expect("tempdir");
        let log = Arc::new(ToolAuditLog::open(dir.path()).await.expect("open"));
        let scrubber = Arc::new(LogScrubber::new());
        // Add a custom literal secret.
        scrubber
            .add_literal_value("my-api-key-12345678901234567890", "CUSTOM_KEY")
            .unwrap();
        let adapter = ScrubAuditAdapter::new(log.clone(), scrubber);

        let call = make_call("s2", "read_file");
        let result = ToolResult::text("config: my-api-key-12345678901234567890\nhost: localhost");

        adapter
            .record_result(&call, &result, &CorrelationEnvelope::empty())
            .await
            .expect("scrub result");

        let lines = read_lines(log.path()).await;
        assert_eq!(lines.len(), 1);
        let json: serde_json::Value = serde_json::from_str(&lines[0]).expect("parse JSON");
        assert_eq!(json["kind"], "result");
        assert!(json["ok"].as_bool().unwrap());
        // An uncorrelated line leaves the field out.
        assert!(json.get("correlation").is_none(), "{json}");
        let content = json["content_scrubbed"].as_str().expect("content_scrubbed");
        assert!(
            !content.contains("my-api-key-12345678901234567890"),
            "literal secret must be scrubbed from result content"
        );
        assert!(
            content.contains("[REDACTED:CUSTOM_KEY]"),
            "replacement must name the secret"
        );
    }

    // ── 15 (T028) ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn scrub_adapter_truncates_large_arguments() {
        let dir = TempDir::new().expect("tempdir");
        let log = Arc::new(ToolAuditLog::open(dir.path()).await.expect("open"));
        let scrubber = Arc::new(LogScrubber::empty());
        let adapter = ScrubAuditAdapter::new(log.clone(), scrubber);

        // Create arguments larger than MAX_ARGUMENTS_BYTES.
        let big_value = "x".repeat(10_000);
        let call = ToolCall::at(
            "big",
            "write_file",
            serde_json::json!({"content": big_value}),
            1_700_000_000_000,
        );

        adapter
            .record_admit(&call, &CorrelationEnvelope::empty())
            .await
            .expect("admit big");

        let lines = read_lines(log.path()).await;
        let json: serde_json::Value = serde_json::from_str(&lines[0]).expect("parse JSON");
        let args = json["arguments_scrubbed"]
            .as_str()
            .expect("arguments_scrubbed");
        // Must be truncated to MAX_ARGUMENTS_BYTES or less.
        assert!(
            args.len() <= MAX_ARGUMENTS_BYTES,
            "arguments must be truncated: got {} bytes",
            args.len()
        );
    }

    // ── 16 ────────────────────────────────────────────────────────────────

    #[test]
    fn truncate_str_handles_ascii() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello", 3), "hel");
        assert_eq!(truncate_str("hello", 0), "");
    }

    #[test]
    fn truncate_str_handles_multibyte_utf8() {
        let emoji = "Hello 🌍 World";
        // '🌍' is 4 bytes at offset 6. Truncating at 8 should land inside
        // the emoji and back up to offset 6.
        let result = truncate_str(emoji, 8);
        assert!(result.len() <= 8);
        assert!(result.is_char_boundary(result.len()));
    }

    // ── 17 (T028) ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn scrub_adapter_handles_error_results() {
        let dir = TempDir::new().expect("tempdir");
        let log = Arc::new(ToolAuditLog::open(dir.path()).await.expect("open"));
        let scrubber = Arc::new(LogScrubber::empty());
        let adapter = ScrubAuditAdapter::new(log.clone(), scrubber);

        let call = make_call("err1", "bash");
        let result = ToolResult::err(roko_core::tool::ToolError::Cancelled);

        adapter
            .record_result(&call, &result, &CorrelationEnvelope::empty())
            .await
            .expect("err result");

        let lines = read_lines(log.path()).await;
        let json: serde_json::Value = serde_json::from_str(&lines[0]).expect("parse JSON");
        assert_eq!(json["kind"], "result");
        assert!(
            !json["ok"].as_bool().unwrap(),
            "error result should have ok=false"
        );
    }

    // ── 18 (T028) ─────────────────────────────────────────────────────────

    #[test]
    fn audit_line_roundtrips_through_serde() {
        let line = AuditLine::Admit {
            ts_ms: 1_700_000_000_000,
            call_id: "c1".to_string(),
            call_name: "bash".to_string(),
            arguments_scrubbed: r#"{"x":1}"#.to_string(),
            correlation: CorrelationEnvelope::empty(),
        };
        let json = serde_json::to_string(&line).expect("serialize");
        let decoded: AuditLine = serde_json::from_str(&json).expect("deserialize");
        match decoded {
            AuditLine::Admit {
                call_id, call_name, ..
            } => {
                assert_eq!(call_id, "c1");
                assert_eq!(call_name, "bash");
            }
            _ => panic!("expected Admit variant"),
        }
    }
}
