//! Core types for the shared diagnostic and preflight service.
//!
//! All check IDs, severity levels, findings, requests, and reports live here.
//! Callers select a subset of [`DiagnosticCheckId`] via [`DiagnosticRequest`]
//! and receive a sorted [`DiagnosticReport`].

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

// ── Check IDs ────────────────────────────────────────────────────────────

/// The fixed set of shared diagnostic check identifiers.
///
/// Each variant maps 1:1 to a check function in [`super::checks`].
/// Do not add a second naming scheme; this enum is the canonical source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCheckId {
    /// Configuration file presence and parse validity.
    Config,
    /// LLM provider credentials (API keys, CLI binaries).
    Credentials,
    /// Available disk space on the workspace partition.
    Disk,
    /// Git working tree state (repo presence, uncommitted changes).
    Git,
    /// Plan directory presence and tasks.toml validity.
    Plans,
    /// Rust toolchain availability and version adequacy.
    Toolchain,
    /// Workspace lock file (stale PID, another runner).
    Lock,
    /// Workspace layout (.roko/ directory structure).
    Workspace,
    /// Config and storage schema version compatibility.
    SchemaVersion,
    /// Configured LLM provider availability and health.
    Providers,
    /// Default model configuration and resolution.
    Models,
}

impl DiagnosticCheckId {
    /// All 11 check IDs in canonical sort order.
    pub const ALL: &'static [Self] = &[
        Self::Config,
        Self::Credentials,
        Self::Disk,
        Self::Git,
        Self::Lock,
        Self::Models,
        Self::Plans,
        Self::Providers,
        Self::SchemaVersion,
        Self::Toolchain,
        Self::Workspace,
    ];

    /// Stable snake_case string for this check ID.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Credentials => "credentials",
            Self::Disk => "disk",
            Self::Git => "git",
            Self::Plans => "plans",
            Self::Toolchain => "toolchain",
            Self::Lock => "lock",
            Self::Workspace => "workspace",
            Self::SchemaVersion => "schema_version",
            Self::Providers => "providers",
            Self::Models => "models",
        }
    }
}

impl std::fmt::Display for DiagnosticCheckId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Severity ─────────────────────────────────────────────────────────────

/// Severity level for a diagnostic finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    /// Informational: no action required.
    Info,
    /// Warning: operation can proceed but the user should investigate.
    Warning,
    /// Error: operation should not proceed until resolved.
    Error,
}

impl DiagnosticSeverity {
    /// Fixed-width label for human-readable output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warn",
            Self::Error => "FAIL",
        }
    }
}

// ── Remediation ──────────────────────────────────────────────────────────

/// Suggested fix for a diagnostic finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticRemediation {
    /// Human-readable summary of the fix.
    pub summary: String,
    /// Optional CLI command that would resolve the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Whether applying this fix requires a mutation (file write, process
    /// kill, etc.) as opposed to being purely informational.
    pub mutation_required: bool,
}

// ── Finding ──────────────────────────────────────────────────────────────

/// A single diagnostic finding produced by a check.
///
/// Findings are sorted by `(check_id, code, message)` inside a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFinding {
    /// Which shared check produced this finding.
    pub check_id: DiagnosticCheckId,
    /// Stable, unique code for this specific finding (e.g. `"config_missing"`).
    pub code: String,
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Human-readable description of the finding.
    pub message: String,
    /// Optional remediation suggestion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<DiagnosticRemediation>,
    /// Structured evidence: arbitrary key-value pairs relevant to the finding.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub evidence: BTreeMap<String, String>,
}

impl DiagnosticFinding {
    /// Sort key: `(check_id, code, message)`.
    fn sort_key(&self) -> (&DiagnosticCheckId, &str, &str) {
        (&self.check_id, &self.code, &self.message)
    }
}

// ── Request ──────────────────────────────────────────────────────────────

/// Request to run a set of diagnostic checks.
#[derive(Debug, Clone)]
pub struct DiagnosticRequest {
    /// Workspace root to inspect.
    pub workdir: PathBuf,
    /// Which checks to run. If empty, no checks are executed.
    pub selected: BTreeSet<DiagnosticCheckId>,
    /// Optional profile name for check-specific tuning.
    pub profile: Option<String>,
    /// Whether repair operations are allowed. Graph preflight always
    /// passes `false`.
    pub allow_repairs: bool,
}

// ── Report ───────────────────────────────────────────────────────────────

/// Aggregated diagnostic report containing sorted findings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// All findings, sorted by `(check_id, code, message)`.
    pub findings: Vec<DiagnosticFinding>,
    /// Unix timestamp (ms) when the diagnostic run started.
    pub started_at_ms: u64,
    /// Unix timestamp (ms) when the diagnostic run completed.
    pub completed_at_ms: u64,
}

impl DiagnosticReport {
    /// Create a new report from unsorted findings, applying canonical sort.
    pub(crate) fn new(mut findings: Vec<DiagnosticFinding>, started_at_ms: u64) -> Self {
        findings.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        let completed_at_ms = now_ms();
        Self {
            findings,
            started_at_ms,
            completed_at_ms,
        }
    }

    /// Returns `true` if any finding has [`DiagnosticSeverity::Error`].
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity == DiagnosticSeverity::Error)
    }

    /// Returns `true` if any finding has [`DiagnosticSeverity::Warning`] or higher.
    #[must_use]
    pub fn has_warnings_or_errors(&self) -> bool {
        self.findings.iter().any(|f| {
            matches!(
                f.severity,
                DiagnosticSeverity::Warning | DiagnosticSeverity::Error
            )
        })
    }

    /// Count of findings at each severity level.
    #[must_use]
    pub fn severity_counts(&self) -> (usize, usize, usize) {
        let mut info = 0;
        let mut warn = 0;
        let mut error = 0;
        for f in &self.findings {
            match f.severity {
                DiagnosticSeverity::Info => info += 1,
                DiagnosticSeverity::Warning => warn += 1,
                DiagnosticSeverity::Error => error += 1,
            }
        }
        (info, warn, error)
    }
}

/// Current time in milliseconds since the Unix epoch.
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
