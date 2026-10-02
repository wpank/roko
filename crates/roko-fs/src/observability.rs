//! Convenience wiring for filesystem-backed observability sinks.
//!
//! Runtime callers attach a persistent trace sink. [`FsObservabilitySinks`]
//! constructs it from either a workspace root or an existing `.roko/`
//! directory and exposes typed and trait-object handles. Tool calls keep no
//! separate metrics file: each call's trace and its tool-audit record say
//! what it did (backlog 2123).
//!
//! # Additional contracts
//!
//! - **[`RunScrubber`]** (T036): builds a [`LogScrubber`] seeded with configured
//!   literal secrets, shared across all sinks in a run.
//! - **[`SinkTelemetry`]** (T020): lightweight counters for queue depth and
//!   write throughput, queryable at runtime.
//! - **[`RetentionPolicy`]** (T021): configurable retention/quota/disk-failure
//!   contract shared by audit and traces.
//! - **[`CorrelationQuery`]** (T022): read-only lookup from a tool-call ID to
//!   its audit record and trace file.
//! - **Metric cardinality** (T023): testable rules ([`validate_metric_key`],
//!   [`validate_cardinality`]) ensuring metric keys never contain secrets
//!   and cardinality stays bounded.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use roko_core::obs::LogScrubber;
use roko_core::obs::scrub::MIN_SECRET_LEN;
use roko_core::tool::TraceSink;
use serde::{Deserialize, Serialize};

use crate::JsonlTraceSink;
use crate::trace_sink::TraceSinkHealth;

// ─── FsObservabilitySinks ────────────────────────────────────────────────────

/// The filesystem-backed trace sink runtime dispatch code attaches.
#[derive(Debug, Clone)]
pub struct FsObservabilitySinks {
    /// Persistent JSONL trace sink.
    pub trace_sink: Arc<JsonlTraceSink>,
}

impl FsObservabilitySinks {
    /// Build sinks rooted at a workspace directory: traces go to
    /// `<workdir>/.roko/traces/`.
    #[must_use]
    pub fn for_workdir(workdir: impl AsRef<Path>) -> Self {
        Self {
            trace_sink: Arc::new(JsonlTraceSink::for_workdir(workdir.as_ref())),
        }
    }

    /// Build sinks rooted at an existing `.roko/` directory: traces go to
    /// `<roko_dir>/traces/`.
    #[must_use]
    pub fn for_roko_dir(roko_dir: impl AsRef<Path>) -> Self {
        Self {
            trace_sink: Arc::new(JsonlTraceSink::for_roko_dir(roko_dir.as_ref())),
        }
    }

    /// Build sinks rooted at a workspace directory and create their
    /// backing directory immediately.
    ///
    /// This is idempotent: calling it repeatedly only re-validates that the
    /// directory structure exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the trace directory cannot be created.
    pub fn initialized_for_workdir(workdir: impl AsRef<Path>) -> io::Result<Self> {
        let sinks = Self::for_workdir(workdir);
        sinks.initialize()?;
        Ok(sinks)
    }

    /// Build sinks rooted at an existing `.roko/` directory and create their
    /// backing directory immediately.
    ///
    /// This is idempotent: calling it repeatedly only re-validates that the
    /// directory structure exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the trace directory cannot be created.
    pub fn initialized_for_roko_dir(roko_dir: impl AsRef<Path>) -> io::Result<Self> {
        let sinks = Self::for_roko_dir(roko_dir);
        sinks.initialize()?;
        Ok(sinks)
    }

    /// Create the trace directory.
    ///
    /// This makes startup initialization explicit for callers that want to
    /// prepare observability before the first trace write.
    ///
    /// The operation is idempotent: an existing directory is left untouched.
    ///
    /// # Errors
    ///
    /// Returns an error if the trace directory cannot be created.
    pub fn initialize(&self) -> io::Result<()> {
        std::fs::create_dir_all(self.trace_sink.root())
    }

    /// Clone as a dynamic trace sink trait object.
    #[must_use]
    pub fn trace_sink_dyn(&self) -> Arc<dyn TraceSink> {
        self.trace_sink.clone()
    }

    /// Flush all open trace writers to disk.
    ///
    /// Ensures buffered data from in-progress traces lands on disk even when
    /// the run terminates before every trace calls `finish()`.
    pub fn flush_traces(&self) {
        self.trace_sink.flush_all();
    }
}

// ─── RunScrubber (T036) ─────────────────────────────────────────────────────

/// A [`LogScrubber`] seeded with built-in regex patterns and additional
/// configured literal secret values (from config or env). Shared across
/// all sinks in a single run.
///
/// Construction: call [`RunScrubber::build`] with secret key-value pairs
/// sourced from `roko.toml [secrets]` or environment variables. The `roko`
/// binary calls [`RunScrubber::install`] once at startup: its log layers
/// scrub with the result, and the persistence writers scrub what they write
/// with its literal values (see [`roko_core::obs::scrub`]).
pub struct RunScrubber;

impl RunScrubber {
    /// Build a [`LogScrubber`] pre-loaded with:
    /// 1. All built-in regex patterns (API keys, tokens, etc.)
    /// 2. Each `(name, value)` pair as a literal-match pattern.
    ///
    /// Values shorter than [`MIN_SECRET_LEN`] are skipped: short values also
    /// occur in ordinary text. Invalid patterns are logged and skipped — they
    /// never prevent construction.
    #[must_use]
    pub fn build(configured_secrets: &[(&str, &str)]) -> Arc<LogScrubber> {
        let scrubber = LogScrubber::new();
        for &(name, value) in configured_secrets {
            if value.len() < MIN_SECRET_LEN {
                continue;
            }
            if let Err(e) = scrubber.add_literal_value(value, name) {
                tracing::warn!(
                    name,
                    error = %e,
                    "RunScrubber: failed to add literal secret pattern"
                );
            }
        }
        Arc::new(scrubber)
    }

    /// Build from environment variable names. For each name, looks up the
    /// value via `std::env::var`. Missing or empty values are skipped.
    #[must_use]
    pub fn from_env_vars(var_names: &[&str]) -> Arc<LogScrubber> {
        let pairs: Vec<(String, String)> = var_names
            .iter()
            .filter_map(|name| {
                std::env::var(name)
                    .ok()
                    .filter(|v| !v.is_empty())
                    .map(|v| (name.to_string(), v))
            })
            .collect();
        let refs: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_str()))
            .collect();
        Self::build(&refs)
    }

    /// Build the process's scrubber and install it as its secret scrubber
    /// ([`roko_core::obs::install_secret_scrubber`]). Its secrets are the
    /// secrets among `env_file_entries`, the `(name, value)` pairs loaded
    /// from `.env` files ([`RunScrubber::build_from_env_file`]), then the
    /// values of the provider key variables roko knows
    /// ([`roko_core::child_env::PROVIDER_KEY_VARS`] and the provider
    /// catalog's), so a key exported in the shell is scrubbed too; each
    /// config roko loads adds its own. Returns the scrubber for the log
    /// layers.
    pub fn install(env_file_entries: &[(String, String)]) -> Arc<LogScrubber> {
        let scrubber = Self::build_from_env_file(env_file_entries);
        roko_core::obs::install_secret_scrubber(Some(Arc::clone(&scrubber)));
        roko_core::obs::add_secret_env_values(
            roko_core::child_env::PROVIDER_KEY_VARS
                .iter()
                .copied()
                .chain(
                    roko_core::provider_catalog::catalog()
                        .iter()
                        .map(|entry| entry.api_key_env)
                        .filter(|name| !name.is_empty()),
                ),
        );
        scrubber
    }

    /// Build a scrubber from a `.env` file's `(name, value)` entries,
    /// registering the secrets among them ([`is_env_file_secret`]). A
    /// setting kept there, such as `ROKO_LOG=debug,roko=trace` or a model
    /// name, stays readable in records.
    #[must_use]
    pub fn build_from_env_file(env_file_entries: &[(String, String)]) -> Arc<LogScrubber> {
        let secrets: Vec<(&str, &str)> = env_file_entries
            .iter()
            .filter(|(name, value)| is_env_file_secret(name, value))
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        Self::build(&secrets)
    }
}

/// Whether a `.env` entry holds a secret.
///
/// It does when its name looks like a credential's (`OPENAI_API_KEY`,
/// `GITHUB_TOKEN`; see [`roko_core::child_env::is_secret_env_name`]), or when
/// its value is a URL that carries credentials
/// (`postgres://user:pass@host/db`, `https://token@host/repo`).
#[must_use]
pub fn is_env_file_secret(name: &str, value: &str) -> bool {
    roko_core::child_env::is_secret_env_name(name)
        || value.split_once("://").is_some_and(|(_, rest)| {
            rest.split(['/', '?', '#'])
                .next()
                .is_some_and(|authority| authority.contains('@'))
        })
}

// ─── SinkTelemetry (T020) ───────────────────────────────────────────────────

/// Lightweight atomic counters for observing sink throughput and failures.
///
/// Each sink type (audit, trace) can share one of these. Counters are
/// monotonic and lock-free.
#[derive(Debug)]
pub struct SinkTelemetry {
    /// Total events successfully written.
    pub writes_ok: AtomicU64,
    /// Total events that failed to write.
    pub writes_err: AtomicU64,
    /// Total bytes written (best-effort; excludes failed writes).
    pub bytes_written: AtomicU64,
}

impl SinkTelemetry {
    /// Create a zeroed counter set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            writes_ok: AtomicU64::new(0),
            writes_err: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
        }
    }

    /// Record a successful write of `n` bytes.
    pub fn record_ok(&self, bytes: u64) {
        self.writes_ok.fetch_add(1, Ordering::Relaxed);
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a failed write attempt.
    pub fn record_err(&self) {
        self.writes_err.fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot the current counters.
    #[must_use]
    pub fn snapshot(&self) -> SinkTelemetrySnapshot {
        SinkTelemetrySnapshot {
            writes_ok: self.writes_ok.load(Ordering::Relaxed),
            writes_err: self.writes_err.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
        }
    }
}

impl Default for SinkTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot of [`SinkTelemetry`] counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SinkTelemetrySnapshot {
    /// Successful write operations.
    pub writes_ok: u64,
    /// Failed write operations.
    pub writes_err: u64,
    /// Total bytes written.
    pub bytes_written: u64,
}

// ─── RetentionPolicy (T021) ─────────────────────────────────────────────────

/// Configurable retention/quota/disk-failure contract that applies uniformly
/// to audit and trace JSONL files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum age of files before they are eligible for cleanup (days).
    pub max_age_days: u32,
    /// Maximum total bytes across all files in the managed directory.
    /// When exceeded, oldest files are removed first.
    pub max_total_bytes: u64,
    /// Minimum free disk space (MB) required before writes are allowed.
    /// Below this threshold, new writes are dropped and a degradation
    /// event is emitted rather than crashing.
    pub min_free_disk_mb: u64,
    /// When true, the sink degrades gracefully on disk-full instead of
    /// returning errors to the caller. Degraded writes are silently
    /// dropped and counted via [`SinkTelemetry`].
    pub graceful_degradation: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_age_days: 30,
            max_total_bytes: 512 * 1024 * 1024, // 512 MB
            min_free_disk_mb: 100,
            graceful_degradation: true,
        }
    }
}

impl RetentionPolicy {
    /// Check whether available disk space is above the minimum threshold.
    ///
    /// Returns `true` if writes should proceed, `false` if disk pressure
    /// warrants degradation.
    #[must_use]
    pub fn disk_ok(&self, free_mb: u64) -> bool {
        free_mb >= self.min_free_disk_mb
    }
}

// ─── CorrelationQuery (T022) ────────────────────────────────────────────────

/// Read-only query surface for correlating a tool call ID to its audit
/// record and trace file.
///
/// This does not hold open file handles; each query scans the relevant
/// JSONL file(s).
#[derive(Debug)]
pub struct CorrelationQuery {
    /// Root workspace directory (typically the worktree root).
    workdir: PathBuf,
}

/// Result of a correlation lookup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrelationResult {
    /// Lines from the audit log matching the call ID.
    pub audit_lines: Vec<serde_json::Value>,
    /// Paths to trace files that reference the call ID.
    pub trace_files: Vec<PathBuf>,
}

impl CorrelationQuery {
    /// Create a query surface rooted at a workspace directory.
    #[must_use]
    pub fn new(workdir: impl Into<PathBuf>) -> Self {
        Self {
            workdir: workdir.into(),
        }
    }

    /// Look up all observability records for a given tool call ID.
    ///
    /// Scans the audit log and the trace directories. This is a read-only,
    /// best-effort operation — missing files are not errors.
    ///
    /// # Errors
    ///
    /// Returns an error only on unrecoverable I/O failures.
    pub fn lookup(&self, call_id: &str) -> io::Result<CorrelationResult> {
        let audit_lines = self.scan_audit(call_id)?;
        let trace_files = self.scan_traces(call_id)?;
        Ok(CorrelationResult {
            audit_lines,
            trace_files,
        })
    }

    fn scan_audit(&self, call_id: &str) -> io::Result<Vec<serde_json::Value>> {
        let audit_path = self.workdir.join(crate::tool_audit::DEFAULT_AUDIT_PATH);
        let contents = match std::fs::read_to_string(&audit_path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut matches = Vec::new();
        for line in contents.lines() {
            if line.contains(call_id)
                && let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
            {
                matches.push(v);
            }
        }
        Ok(matches)
    }

    fn scan_traces(&self, call_id: &str) -> io::Result<Vec<PathBuf>> {
        let traces_dir = self
            .workdir
            .join(crate::trace_sink::DEFAULT_TRACE_DIR_REL_PATH);
        let mut matching_files = Vec::new();
        let date_dirs = match std::fs::read_dir(&traces_dir) {
            Ok(d) => d,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        for entry in date_dirs.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let files = match std::fs::read_dir(entry.path()) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for file_entry in files.flatten() {
                let path = file_entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                // Quick check: does the file mention this call_id?
                if let Ok(contents) = std::fs::read_to_string(&path)
                    && contents.contains(call_id)
                {
                    matching_files.push(path);
                }
            }
        }
        Ok(matching_files)
    }
}

// ─── MetricCardinality (T023) ───────────────────────────────────────────────

/// Maximum number of distinct metric keys before we consider cardinality
/// unbounded (indicative of a label-explosion bug).
pub const MAX_METRIC_CARDINALITY: usize = 10_000;

/// Forbidden substrings in metric label values. If any label contains one
/// of these patterns, the metric is rejected.
const FORBIDDEN_LABEL_PATTERNS: &[&str] = &[
    "sk-",
    "ghp_",
    "gho_",
    "ghs_",
    "ghu_",
    "ghr_",
    "xoxb-",
    "Bearer ",
    "bearer ",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
];

/// Validate that a metric key does not contain secret material.
///
/// Returns `Ok(())` if the key is clean, `Err(reason)` if it contains
/// a forbidden pattern.
pub fn validate_metric_key(key: &str) -> Result<(), &'static str> {
    for &pattern in FORBIDDEN_LABEL_PATTERNS {
        if key.contains(pattern) {
            return Err("metric key contains a secret pattern");
        }
    }
    Ok(())
}

/// Validate that a set of metric keys stays within cardinality bounds.
///
/// Returns `Ok(())` if the count is within [`MAX_METRIC_CARDINALITY`],
/// `Err(count)` otherwise.
pub fn validate_cardinality(key_count: usize) -> Result<(), usize> {
    if key_count > MAX_METRIC_CARDINALITY {
        Err(key_count)
    } else {
        Ok(())
    }
}

// ─── ObservabilityHealth ────────────────────────────────────────────────────

/// Aggregate health across all observability subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityHealth {
    /// Health of the trace sink subsystem.
    pub trace_health: TraceSinkHealth,
    /// Aggregate sink write counters.
    pub sink_telemetry: Option<SinkTelemetrySnapshot>,
    /// Active retention configuration.
    pub retention_policy: RetentionPolicy,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolAuditLog;

    #[test]
    fn for_workdir_builds_expected_paths() {
        let sinks = FsObservabilitySinks::for_workdir("/repo");
        assert_eq!(sinks.trace_sink.root(), Path::new("/repo/.roko/traces"),);
    }

    #[test]
    fn dyn_accessors_return_trait_objects() {
        let sinks = FsObservabilitySinks::for_roko_dir("/repo/.roko");
        let _trace: Arc<dyn TraceSink> = sinks.trace_sink_dyn();
    }

    /// backlog 2123: the sinks keep traces only; no tool metrics directory
    /// is made.
    #[test]
    fn initialize_creates_the_trace_directory_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sinks = FsObservabilitySinks::for_workdir(tmp.path());

        sinks.initialize().expect("initialize observability");

        assert!(tmp.path().join(".roko").join("traces").is_dir());
        assert!(!tmp.path().join(".roko").join("metrics").exists());
    }

    #[test]
    fn initialized_for_roko_dir_is_idempotent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let roko_dir = tmp.path().join(".roko");

        let sinks = FsObservabilitySinks::initialized_for_roko_dir(&roko_dir)
            .expect("initialize from roko dir");
        sinks.initialize().expect("reinitialize");

        assert!(roko_dir.join("traces").is_dir());
    }

    #[test]
    fn flush_traces_delegates_to_trace_sink() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sinks =
            FsObservabilitySinks::initialized_for_workdir(tmp.path()).expect("initialize sinks");

        // Append a trace event but do not finish it.
        let trace_id = roko_core::tool::trace::TraceId::from_bytes([0xAA; 16]);
        sinks.trace_sink.append(
            trace_id,
            roko_core::tool::trace::ToolTraceEvent::StreamCoerced { at_ms: 1 },
        );

        // flush_traces should persist buffered data.
        sinks.flush_traces();

        // Verify the trace file exists with content.
        let traces_dir = tmp.path().join(".roko").join("traces");
        assert!(traces_dir.is_dir(), "traces dir must exist");
        // Walk into date directory.
        let entries: Vec<_> = std::fs::read_dir(&traces_dir)
            .expect("read traces dir")
            .filter_map(Result::ok)
            .collect();
        assert!(!entries.is_empty(), "should have a date directory");
        let date_dir = &entries[0].path();
        let trace_file = date_dir.join(format!("{}.jsonl", trace_id.to_hex()));
        assert!(
            trace_file.is_file(),
            "trace file must exist after flush: {trace_file:?}"
        );
        let contents = std::fs::read_to_string(&trace_file).expect("read trace");
        assert_eq!(contents.lines().count(), 1, "one event line expected");
    }

    // ── T036: RunScrubber tests ─────────────────────────────────────────────

    #[test]
    fn run_scrubber_includes_builtins() {
        let scrubber = RunScrubber::build(&[]);
        // Built-in patterns should scrub known API key formats.
        let input = "key=sk-ant-api03-ABCDEFGHIJKLMNOPqrstuvwxyz1234567890";
        let output = scrubber.scrub(input);
        assert!(
            !output.contains("sk-ant-api03"),
            "built-in patterns must work"
        );
    }

    #[test]
    fn run_scrubber_adds_literal_secrets() {
        let scrubber = RunScrubber::build(&[("MY_SECRET", "super-secret-value-42")]);
        let input = r#"{"token":"super-secret-value-42","name":"ok"}"#;
        let output = scrubber.scrub(input);
        assert!(
            !output.contains("super-secret-value-42"),
            "literal secret must be redacted"
        );
        assert!(
            output.contains("[REDACTED:MY_SECRET]"),
            "replacement must name the secret"
        );
    }

    #[test]
    fn run_scrubber_handles_nested_json_secrets() {
        let scrubber = RunScrubber::build(&[("DB_PASS", "p4ssw0rd!")]);
        let input = r#"{"config":{"db":{"password":"p4ssw0rd!"},"host":"localhost"}}"#;
        let output = scrubber.scrub(input);
        assert!(
            !output.contains("p4ssw0rd!"),
            "nested JSON secret must be redacted"
        );
    }

    #[test]
    fn run_scrubber_handles_secrets_in_error_strings() {
        let scrubber = RunScrubber::build(&[("API_KEY", "secret-key-xyz")]);
        let input = "Error: connection refused using key secret-key-xyz at host:443";
        let output = scrubber.scrub(input);
        assert!(
            !output.contains("secret-key-xyz"),
            "secret in error string must be redacted"
        );
    }

    #[test]
    fn run_scrubber_skips_empty_values() {
        let scrubber = RunScrubber::build(&[("EMPTY", ""), ("OK", "real-value")]);
        // Should not crash and should still scrub the non-empty one.
        let output = scrubber.scrub("found real-value here");
        assert!(!output.contains("real-value"));
    }

    #[test]
    fn run_scrubber_handles_binary_like_bodies() {
        let scrubber = RunScrubber::build(&[("TOKEN", "abc123def456")]);
        // Base64-ish content mixed with the secret.
        let input = "body: aGVsbG8= abc123def456 d29ybGQ=";
        let output = scrubber.scrub(input);
        assert!(!output.contains("abc123def456"));
    }

    #[test]
    fn run_scrubber_skips_short_values() {
        let scrubber = RunScrubber::build(&[("PORT", "8080"), ("MODE", "debug")]);
        assert_eq!(
            scrubber.scrub("port 8080 mode debug"),
            "port 8080 mode debug"
        );
    }

    /// bug-cef888: every `.env` value of 8 or more characters counted as a
    /// secret, so a setting kept there was redacted from every record.
    #[test]
    fn non_secret_env_settings_are_not_redacted() {
        let entries = [
            ("OPENAI_API_KEY", "sk-test-cef888-secret"),
            ("ROKO__SERVE__AUTH__API_KEY", "serve-key-cef888"),
            ("DATABASE_URL", "postgres://roko:pw-cef888@db.internal/roko"),
            ("ROKO_LOG", "debug,roko=trace"),
            ("ROKO_MODEL", "claude-sonnet-4-6"),
            ("REPO_URL", "https://github.com/nunchi/roko"),
        ]
        .map(|(name, value)| (name.to_string(), value.to_string()));
        let scrubber = RunScrubber::build_from_env_file(&entries);
        let record = "sk-test-cef888-secret serve-key-cef888 \
                      postgres://roko:pw-cef888@db.internal/roko debug,roko=trace \
                      claude-sonnet-4-6 https://github.com/nunchi/roko";
        let scrubbed = scrubber.scrub_literals(record);
        for secret in ["sk-test-cef888-secret", "serve-key-cef888", "pw-cef888"] {
            assert!(!scrubbed.contains(secret), "{scrubbed}");
        }
        for setting in [
            "debug,roko=trace",
            "claude-sonnet-4-6",
            "https://github.com/nunchi/roko",
        ] {
            assert!(scrubbed.contains(setting), "{scrubbed}");
        }
    }

    #[test]
    fn installed_run_scrubber_reaches_jsonl_appends() {
        let previous = roko_core::obs::secret_scrubber();
        let scrubber = RunScrubber::install(&[(
            "CANARY_TOKEN".to_string(),
            "fs-canary-secret-01".to_string(),
        )]);
        assert!(
            scrubber
                .scrub("x fs-canary-secret-01")
                .contains("[REDACTED:CANARY_TOKEN]")
        );

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("records.jsonl");
        let record = serde_json::json!({
            "output": "the agent printed fs-canary-secret-01",
            "created_at_ms": 1_727_700_000_000_u64,
        });
        crate::log_rotation::append_jsonl_line_sync(&path, record.to_string().as_bytes(), 10)
            .expect("append record");
        crate::log_rotation::append_jsonl_line_relaxed_sync(&path, br#"{"clean":true}"#, 10)
            .expect("append clean record");
        roko_core::obs::install_secret_scrubber(previous);

        let written = std::fs::read_to_string(&path).expect("read records");
        assert!(!written.contains("fs-canary-secret-01"), "{written}");
        let lines: Vec<serde_json::Value> = written
            .lines()
            .map(|line| serde_json::from_str(line).expect("record still parses"))
            .collect();
        assert_eq!(
            lines[0]["output"],
            "the agent printed [REDACTED:CANARY_TOKEN]"
        );
        assert_eq!(lines[0]["created_at_ms"], 1_727_700_000_000_u64);
        assert_eq!(lines[1], serde_json::json!({ "clean": true }));
    }

    // ── T020: SinkTelemetry tests ───────────────────────────────────────────

    #[test]
    fn sink_telemetry_starts_at_zero() {
        let t = SinkTelemetry::new();
        let s = t.snapshot();
        assert_eq!(s.writes_ok, 0);
        assert_eq!(s.writes_err, 0);
        assert_eq!(s.bytes_written, 0);
    }

    #[test]
    fn sink_telemetry_records_ok_and_err() {
        let t = SinkTelemetry::new();
        t.record_ok(100);
        t.record_ok(200);
        t.record_err();

        let s = t.snapshot();
        assert_eq!(s.writes_ok, 2);
        assert_eq!(s.writes_err, 1);
        assert_eq!(s.bytes_written, 300);
    }

    #[test]
    fn sink_telemetry_snapshot_serializes() {
        let snap = SinkTelemetrySnapshot {
            writes_ok: 10,
            writes_err: 2,
            bytes_written: 4096,
        };
        let json = serde_json::to_string(&snap).expect("serialize");
        let decoded: SinkTelemetrySnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, snap);
    }

    // ── T021: RetentionPolicy tests ─────────────────────────────────────────

    #[test]
    fn retention_policy_default_values() {
        let p = RetentionPolicy::default();
        assert_eq!(p.max_age_days, 30);
        assert_eq!(p.max_total_bytes, 512 * 1024 * 1024);
        assert_eq!(p.min_free_disk_mb, 100);
        assert!(p.graceful_degradation);
    }

    #[test]
    fn retention_policy_disk_ok_above_threshold() {
        let p = RetentionPolicy {
            min_free_disk_mb: 200,
            ..Default::default()
        };
        assert!(p.disk_ok(300));
        assert!(p.disk_ok(200));
        assert!(!p.disk_ok(199));
        assert!(!p.disk_ok(0));
    }

    #[test]
    fn retention_policy_serializes() {
        let p = RetentionPolicy::default();
        let json = serde_json::to_string(&p).expect("serialize");
        let decoded: RetentionPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, p);
    }

    // ── T022: CorrelationQuery tests ────────────────────────────────────────

    #[tokio::test]
    async fn correlation_query_finds_audit_lines() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let log = ToolAuditLog::open(tmp.path()).await.expect("open");
        let call = roko_core::tool::ToolCall::at("call-42", "bash", serde_json::json!({}), 1000);
        let result = roko_core::tool::ToolResult::text("ok");
        log.record_admit(&call).await.expect("admit");
        log.record_result(&call, &result).await.expect("result");
        log.flush().await.expect("flush");

        let query = CorrelationQuery::new(tmp.path());
        let result = query.lookup("call-42").expect("lookup");
        assert_eq!(result.audit_lines.len(), 2, "should find admit + result");
    }

    #[tokio::test]
    async fn correlation_query_returns_empty_for_missing_call() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // Don't write anything.
        let query = CorrelationQuery::new(tmp.path());
        let result = query.lookup("nonexistent").expect("lookup");
        assert!(result.audit_lines.is_empty());
        assert!(result.trace_files.is_empty());
    }

    // ── T023: MetricCardinality tests ───────────────────────────────────────

    #[test]
    fn metric_key_rejects_api_keys() {
        assert!(validate_metric_key("tool_sk-ant-secret123").is_err());
        assert!(validate_metric_key("ghp_ABCtoken").is_err());
        assert!(validate_metric_key("model_xoxb-12345").is_err());
    }

    #[test]
    fn metric_key_accepts_clean_keys() {
        assert!(validate_metric_key("read_file").is_ok());
        assert!(validate_metric_key("claude-opus-4-6").is_ok());
        assert!(validate_metric_key("roko_tool_latency_ms").is_ok());
    }

    #[test]
    fn cardinality_within_bounds() {
        assert!(validate_cardinality(100).is_ok());
        assert!(validate_cardinality(MAX_METRIC_CARDINALITY).is_ok());
    }

    #[test]
    fn cardinality_rejects_explosion() {
        assert_eq!(
            validate_cardinality(MAX_METRIC_CARDINALITY + 1),
            Err(MAX_METRIC_CARDINALITY + 1)
        );
    }

    #[test]
    fn forbidden_patterns_cover_known_secret_formats() {
        // Ensure all the key formats from LogScrubber are represented.
        let patterns = FORBIDDEN_LABEL_PATTERNS;
        assert!(patterns.contains(&"sk-"));
        assert!(patterns.contains(&"ghp_"));
        assert!(patterns.contains(&"xoxb-"));
        assert!(patterns.contains(&"Bearer "));
    }
}
