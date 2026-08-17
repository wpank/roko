//! Structured auth audit trail.
//!
//! Every authentication and authorisation event is appended as a JSONL record
//! to `.roko/auth-audit.jsonl`.  The writer is append-only and best-effort:
//! individual write failures are logged but never propagate to callers.
//!
//! ## Record format
//!
//! ```json
//! {
//!   "timestamp": "2026-08-12T10:00:00Z",
//!   "actor":      "api-key:my-key",
//!   "action":     "PermissionDenied",
//!   "target":     "team:manage",
//!   "outcome":    "denied",
//!   "ip":         "127.0.0.1",
//!   "user_agent": "curl/8.4.0",
//!   "metadata":   {}
//! }
//! ```
//!
//! ## Retention
//!
//! [`AuthAuditLog::sweep_old_entries`] discards records whose `timestamp` is
//! older than the configured `retention_days` (default 30).  Call it
//! periodically (e.g. from a scheduled task or a maintenance endpoint) to
//! prevent unbounded growth.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

// ── AuthAuditEvent ──────────────────────────────────────────────────────────

/// Every distinct auth / authorisation operation that can be audited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AuthAuditAction {
    /// A user/key successfully authenticated and received a session.
    Login,
    /// A new API key or agent token was issued.
    TokenIssued,
    /// A token/key was explicitly revoked.
    TokenRevoked,
    /// A key or token was rotated (old hash retired, new one issued).
    #[serde(alias = "KeyRotated")]
    TokenRotated,
    /// A token/key was accepted for a request (used successfully).
    TokenUsed,
    /// A permission check passed.
    PermissionGranted,
    /// A permission check failed — the actor lacked the required permission.
    PermissionDenied,
    /// An invitation was created.
    InviteCreated,
    /// An invitation was accepted.
    InviteAccepted,
    /// An invitation expired before being accepted.
    InviteExpired,
    /// A key's `expires_at` timestamp was reached and the key rejected.
    KeyExpired,
    /// A team member's role was changed.
    RoleChanged,
    /// A relay token was delegated from one agent to another.
    TokenDelegated,
}

impl AuthAuditAction {
    /// Machine-readable variant name (same as serde rename).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Login => "Login",
            Self::TokenIssued => "TokenIssued",
            Self::TokenRevoked => "TokenRevoked",
            Self::TokenRotated => "TokenRotated",
            Self::TokenUsed => "TokenUsed",
            Self::PermissionGranted => "PermissionGranted",
            Self::PermissionDenied => "PermissionDenied",
            Self::InviteCreated => "InviteCreated",
            Self::InviteAccepted => "InviteAccepted",
            Self::InviteExpired => "InviteExpired",
            Self::KeyExpired => "KeyExpired",
            Self::RoleChanged => "RoleChanged",
            Self::TokenDelegated => "TokenDelegated",
        }
    }
}

/// Outcome of an auth / authorisation event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthOutcome {
    /// The operation succeeded.
    Success,
    /// The operation was denied.
    Denied,
    /// The operation failed due to an internal error.
    Failure,
}

/// A single structured auth audit record.
///
/// Appended as a JSONL line to `.roko/auth-audit.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthAuditEvent {
    /// ISO 8601 UTC timestamp when the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Identity of the actor (e.g. `"api-key:my-key"`, `"agent:agent-id"`, `"jwt:sub"`).
    pub actor: String,
    /// The action that was performed or attempted.
    pub action: AuthAuditAction,
    /// What was acted on (e.g. a permission name, token_id, route path, role).
    pub target: String,
    /// Whether the action succeeded or was denied.
    pub outcome: AuthOutcome,
    /// Remote IP address of the caller, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// User-Agent header value, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// Arbitrary extra context (e.g. `{"from_role": "member", "to_role": "admin"}`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl AuthAuditEvent {
    /// Create a minimal event with the required fields.
    pub fn new(
        actor: impl Into<String>,
        action: AuthAuditAction,
        target: impl Into<String>,
        outcome: AuthOutcome,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            actor: actor.into(),
            action,
            target: target.into(),
            outcome,
            ip: None,
            user_agent: None,
            metadata: HashMap::new(),
        }
    }

    /// Attach an optional IP address.
    pub fn with_ip(mut self, ip: Option<impl Into<String>>) -> Self {
        self.ip = ip.map(|s| s.into());
        self
    }

    /// Attach an optional User-Agent string.
    pub fn with_user_agent(mut self, ua: Option<impl Into<String>>) -> Self {
        self.user_agent = ua.map(|s| s.into());
        self
    }

    /// Attach a single metadata key/value pair.
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// ── AuthAuditLog ─────────────────────────────────────────────────────────────

/// Append-only JSONL audit log for auth events.
///
/// Writes are serialised by an internal [`Mutex`] so the log can be shared
/// across threads without external locking.  Each write is flushed immediately
/// for crash resilience (best-effort; flush errors are logged but swallowed).
pub struct AuthAuditLog {
    writer: Mutex<BufWriter<File>>,
    path: PathBuf,
    /// How many days of records to retain.  Default: 30.
    pub retention_days: u64,
}

impl AuthAuditLog {
    /// Open (or create) the audit log at `path`.
    ///
    /// # Errors
    ///
    /// Returns an `std::io::Error` if the parent directory cannot be created or
    /// the file cannot be opened in append mode.
    pub fn open(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        Self::open_with_retention(path, 30)
    }

    /// Open (or create) the audit log with a custom retention period.
    pub fn open_with_retention(
        path: impl Into<PathBuf>,
        retention_days: u64,
    ) -> std::io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
            path,
            retention_days,
        })
    }

    /// Append an [`AuthAuditEvent`] to the log (best-effort, never panics).
    ///
    /// Serialisation or write errors are logged via `tracing::warn!` but do
    /// not propagate — audit logging must never block or fail a request.
    pub fn append(&self, event: &AuthAuditEvent) {
        let line = match serde_json::to_string(event) {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!(error = %e, "auth_audit: failed to serialize event");
                return;
            }
        };
        let mut guard = match self.writer.lock() {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!(error = %e, "auth_audit: writer mutex poisoned");
                return;
            }
        };
        if let Err(e) = writeln!(*guard, "{line}") {
            tracing::warn!(error = %e, "auth_audit: failed to write event");
            return;
        }
        // Flush immediately so crash-recovery captures the event.
        if let Err(e) = guard.flush() {
            tracing::warn!(error = %e, "auth_audit: failed to flush");
        }
    }

    /// Remove records older than `retention_days` from the log file.
    ///
    /// Reads the file line-by-line, discards old lines, and rewrites atomically.
    /// This is intended to be called from a scheduled maintenance task.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or rewritten.
    pub fn sweep_old_entries(&self) -> std::io::Result<usize> {
        let cutoff = Utc::now() - Duration::days(self.retention_days as i64);

        // Read current content while holding the writer lock so no concurrent
        // appends produce partial lines during the read.
        let mut guard = self
            .writer
            .lock()
            .map_err(|_| std::io::Error::other("writer mutex poisoned"))?;

        // Flush any buffered writes before reading.
        guard.flush()?;

        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        let mut kept = Vec::new();
        let mut removed = 0usize;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // Parse just the timestamp field to avoid full deserialisation cost.
            let keep = serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|v| v["timestamp"].as_str().map(str::to_string))
                .and_then(|ts| ts.parse::<DateTime<Utc>>().ok())
                .map(|ts| ts >= cutoff)
                .unwrap_or(true); // Unparseable lines are kept (conservative).

            if keep {
                kept.push(line.to_string());
            } else {
                removed += 1;
            }
        }

        // Rewrite the file with kept lines.
        let new_content = kept.join("\n") + if kept.is_empty() { "" } else { "\n" };
        std::fs::write(&self.path, new_content.as_bytes())?;

        // Reopen the file in append mode so the writer stays valid.
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        *guard = BufWriter::new(file);

        Ok(removed)
    }

    /// Query the audit log with optional filters.
    ///
    /// Reads the file line-by-line and returns matching [`AuthAuditEvent`]s.
    /// All filters are optional; omitting one skips that check.
    pub fn query(
        &self,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        actor: Option<&str>,
        action: Option<&AuthAuditAction>,
    ) -> std::io::Result<Vec<AuthAuditEvent>> {
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(e),
        };
        let reader = BufReader::new(file);
        let mut results = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: AuthAuditEvent = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            if let Some(from) = from {
                if event.timestamp < from {
                    continue;
                }
            }
            if let Some(to) = to {
                if event.timestamp > to {
                    continue;
                }
            }
            if let Some(actor_filter) = actor {
                if event.actor != actor_filter {
                    continue;
                }
            }
            if let Some(action_filter) = action {
                if &event.action != action_filter {
                    continue;
                }
            }

            results.push(event);
        }

        Ok(results)
    }

    /// Return the path to the underlying JSONL file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

// ── Convenience constructor ────────────────────────────────────────────────────

/// Open the auth audit log at the canonical path under `workdir`.
///
/// Creates `.roko/auth-audit.jsonl` (and any missing parent directories).
///
/// # Errors
///
/// Returns `std::io::Error` if the file or its parent directory cannot be
/// created.
pub fn open_audit_log(workdir: &Path) -> std::io::Result<AuthAuditLog> {
    let path = workdir.join(".roko").join("auth-audit.jsonl");
    AuthAuditLog::open(path)
}

/// Open the audit log with a custom retention period.
pub fn open_audit_log_with_retention(
    workdir: &Path,
    retention_days: u64,
) -> std::io::Result<AuthAuditLog> {
    let path = workdir.join(".roko").join("auth-audit.jsonl");
    AuthAuditLog::open_with_retention(path, retention_days)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_event(action: AuthAuditAction, outcome: AuthOutcome) -> AuthAuditEvent {
        AuthAuditEvent::new("api-key:test", action, "test-target", outcome)
    }

    // ── AuthAuditEvent construction ───────────────────────────────────────────

    #[test]
    fn audit_event_new_sets_required_fields() {
        let event = sample_event(AuthAuditAction::Login, AuthOutcome::Success);
        assert_eq!(event.actor, "api-key:test");
        assert_eq!(event.action, AuthAuditAction::Login);
        assert_eq!(event.target, "test-target");
        assert_eq!(event.outcome, AuthOutcome::Success);
        assert!(event.ip.is_none());
        assert!(event.user_agent.is_none());
        assert!(event.metadata.is_empty());
    }

    #[test]
    fn audit_event_with_ip_and_user_agent() {
        let event = sample_event(AuthAuditAction::TokenUsed, AuthOutcome::Success)
            .with_ip(Some("192.168.1.1"))
            .with_user_agent(Some("curl/8.0"));
        assert_eq!(event.ip.as_deref(), Some("192.168.1.1"));
        assert_eq!(event.user_agent.as_deref(), Some("curl/8.0"));
    }

    #[test]
    fn audit_event_with_meta_stores_kv() {
        let event = sample_event(AuthAuditAction::RoleChanged, AuthOutcome::Success)
            .with_meta("from_role", "member")
            .with_meta("to_role", "admin");
        assert_eq!(
            event.metadata.get("from_role").map(|s| s.as_str()),
            Some("member")
        );
        assert_eq!(
            event.metadata.get("to_role").map(|s| s.as_str()),
            Some("admin")
        );
    }

    // ── AuthAuditLog round-trip ───────────────────────────────────────────────

    #[test]
    fn audit_log_append_and_query_roundtrip() {
        let dir = tempdir().unwrap();
        let log = open_audit_log(dir.path()).unwrap();

        let event = sample_event(AuthAuditAction::PermissionDenied, AuthOutcome::Denied)
            .with_ip(Some("10.0.0.1"))
            .with_meta("permission", "team:manage");
        log.append(&event);

        let results = log.query(None, None, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, AuthAuditAction::PermissionDenied);
        assert_eq!(results[0].outcome, AuthOutcome::Denied);
        assert_eq!(results[0].ip.as_deref(), Some("10.0.0.1"));
        assert_eq!(
            results[0].metadata.get("permission").map(|s| s.as_str()),
            Some("team:manage")
        );
    }

    #[test]
    fn audit_log_multiple_events_all_recovered() {
        let dir = tempdir().unwrap();
        let log = open_audit_log(dir.path()).unwrap();

        log.append(&sample_event(AuthAuditAction::Login, AuthOutcome::Success));
        log.append(&sample_event(
            AuthAuditAction::TokenRotated,
            AuthOutcome::Success,
        ));
        log.append(&sample_event(
            AuthAuditAction::PermissionDenied,
            AuthOutcome::Denied,
        ));

        let results = log.query(None, None, None, None).unwrap();
        assert_eq!(results.len(), 3);
    }

    // ── Query filters ─────────────────────────────────────────────────────────

    #[test]
    fn query_filters_by_actor() {
        let dir = tempdir().unwrap();
        let log = open_audit_log(dir.path()).unwrap();

        log.append(&AuthAuditEvent::new(
            "actor-A",
            AuthAuditAction::Login,
            "target",
            AuthOutcome::Success,
        ));
        log.append(&AuthAuditEvent::new(
            "actor-B",
            AuthAuditAction::Login,
            "target",
            AuthOutcome::Success,
        ));

        let results = log.query(None, None, Some("actor-A"), None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actor, "actor-A");
    }

    #[test]
    fn query_filters_by_action() {
        let dir = tempdir().unwrap();
        let log = open_audit_log(dir.path()).unwrap();

        log.append(&sample_event(AuthAuditAction::Login, AuthOutcome::Success));
        log.append(&sample_event(
            AuthAuditAction::TokenIssued,
            AuthOutcome::Success,
        ));
        log.append(&sample_event(
            AuthAuditAction::PermissionDenied,
            AuthOutcome::Denied,
        ));

        let results = log
            .query(None, None, None, Some(&AuthAuditAction::PermissionDenied))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, AuthAuditAction::PermissionDenied);
    }

    #[test]
    fn query_returns_empty_when_no_log_file() {
        let dir = tempdir().unwrap();
        // Do NOT call open_audit_log — no file exists.
        let log = AuthAuditLog {
            writer: Mutex::new(BufWriter::new(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.path().join("nonexistent_query_target.jsonl"))
                    .unwrap(),
            )),
            path: dir.path().join("does_not_exist.jsonl"),
            retention_days: 30,
        };
        let results = log.query(None, None, None, None).unwrap();
        assert!(results.is_empty());
    }

    // ── Retention sweep ───────────────────────────────────────────────────────

    #[test]
    fn sweep_removes_old_entries() {
        let dir = tempdir().unwrap();
        let log = open_audit_log(dir.path()).unwrap();

        // Write one recent and one artificially old event.
        let recent = sample_event(AuthAuditAction::Login, AuthOutcome::Success);
        let mut old = sample_event(AuthAuditAction::TokenUsed, AuthOutcome::Success);
        old.timestamp = Utc::now() - Duration::days(60); // 60 days ago

        log.append(&recent);
        log.append(&old);

        // Sweep with 30-day retention.
        let removed = log.sweep_old_entries().unwrap();
        assert_eq!(removed, 1, "exactly the old event should be removed");

        let results = log.query(None, None, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, AuthAuditAction::Login);
    }

    #[test]
    fn sweep_keeps_all_recent_entries() {
        let dir = tempdir().unwrap();
        let log = open_audit_log(dir.path()).unwrap();

        log.append(&sample_event(AuthAuditAction::Login, AuthOutcome::Success));
        log.append(&sample_event(
            AuthAuditAction::TokenIssued,
            AuthOutcome::Success,
        ));

        let removed = log.sweep_old_entries().unwrap();
        assert_eq!(
            removed, 0,
            "no entries should be removed when all are recent"
        );

        let results = log.query(None, None, None, None).unwrap();
        assert_eq!(results.len(), 2);
    }

    // ── AuthAuditAction display ───────────────────────────────────────────────

    #[test]
    fn action_as_str_matches_serde_rename() {
        assert_eq!(AuthAuditAction::Login.as_str(), "Login");
        assert_eq!(
            AuthAuditAction::PermissionDenied.as_str(),
            "PermissionDenied"
        );
        assert_eq!(AuthAuditAction::KeyExpired.as_str(), "KeyExpired");
        assert_eq!(AuthAuditAction::RoleChanged.as_str(), "RoleChanged");
    }

    // ── open_audit_log path ───────────────────────────────────────────────────

    #[test]
    fn open_audit_log_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let log = open_audit_log(dir.path()).unwrap();
        assert!(log.path().parent().unwrap().exists());
        assert_eq!(log.path(), dir.path().join(".roko/auth-audit.jsonl"));
    }

    #[test]
    fn audit_event_serde_roundtrip() {
        let event = sample_event(AuthAuditAction::PermissionDenied, AuthOutcome::Denied)
            .with_ip(Some("127.0.0.1"))
            .with_user_agent(Some("agent/1.0"))
            .with_meta("route", "/api/team");
        let json = serde_json::to_string(&event).unwrap();
        let decoded: AuthAuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.actor, event.actor);
        assert_eq!(decoded.action, event.action);
        assert_eq!(decoded.outcome, event.outcome);
        assert_eq!(decoded.ip, event.ip);
        assert_eq!(decoded.user_agent, event.user_agent);
        assert_eq!(decoded.metadata, event.metadata);
    }
}
