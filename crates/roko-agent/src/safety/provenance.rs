//! Provenance-facing safety records and custody logging.
//!
//! The dispatcher already emits audit events, but several safety documents refer
//! to richer custody and taint records. These structs provide the documented
//! shapes inside the live safety crate without forcing a heavier persistence
//! backend into the runtime path.
//!
//! The [`CustodyLogger`] provides append-only JSONL persistence for custody
//! records, following the same pattern as `EpisodeLogger`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::safety::authz::AuthorizationEvidence;

/// Trust label carried by an input or action lineage in the custody layer.
///
/// This is the **action-centric** taint classification for custody logging —
/// it classifies where an input came from (external fetch, plugin, user, etc.).
///
/// For the **signal-level** taint (hallucination tracking, stale data, propagation),
/// see `roko_core::Taint` which is the canonical provenance-layer type.
///
/// These two serve different architectural layers:
/// - `CustodyTaint`: local safety decision (should this action be restricted?)
/// - `roko_core::Taint`: global signal lineage (should downstream consumers trust this?)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyTaint {
    /// No active taint.
    None,
    /// Data came directly from a human operator or user.
    UserInput,
    /// Data was fetched from an external source.
    ExternalFetch(String),
    /// Data was produced by a third-party plugin or extension.
    ThirdPartyPlugin(String),
    /// Data was imported from a legacy or foreign system.
    LegacyImport,
}

/// Backwards-compatible type alias.
pub type Taint = CustodyTaint;

impl CustodyTaint {
    /// Returns `true` when the label denotes untrusted or review-worthy input.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Convert to the canonical `roko_core::Taint` for signal-level tracking.
    ///
    /// This bridges the custody layer to the provenance layer when a custody
    /// event needs to propagate taint information into the signal graph.
    #[must_use]
    pub fn to_signal_taint(&self) -> roko_core::Taint {
        match self {
            Self::None => roko_core::Taint::Clean,
            Self::UserInput => roko_core::Taint::UserInput {
                detail: "custody: user input".into(),
            },
            Self::ExternalFetch(url) => roko_core::Taint::UnverifiedSource {
                detail: format!("custody: external fetch from {url}"),
            },
            Self::ThirdPartyPlugin(name) => roko_core::Taint::UnverifiedSource {
                detail: format!("custody: third-party plugin {name}"),
            },
            Self::LegacyImport => roko_core::Taint::Custom("custody: legacy import".into()),
        }
    }
}

/// Assurance tier for an audited record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttestationLevel {
    /// Local session-level attestation.
    LocalAgent,
    /// Human or organization-backed attestation.
    OrgRole,
    /// External witness or chain-backed attestation.
    ChainWitness,
}

/// Action-centric custody record for a safety-relevant operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Custody {
    /// Stable identifier for the action being recorded.
    pub action: String,
    /// Principal that initiated the action.
    pub principal: String,
    /// Unix-millis timestamp for the action.
    pub when: i64,
    /// Authorization evidence captured at decision time.
    pub authorized: Vec<AuthorizationEvidence>,
    /// Heuristics that materially influenced the action.
    pub why_heuristics: Vec<String>,
    /// Claims or assertions that materially influenced the action.
    pub why_claims: Vec<String>,
    /// Optional simulation or dry-run identifier.
    pub simulation: Option<String>,
    /// Verify or review stages that passed before execution.
    pub gates_passed: Vec<String>,
    /// Taint state active for the action.
    pub taint: Option<Taint>,
    /// Optional result identifier or digest.
    pub result: Option<String>,
    /// Optional external witness identifier.
    pub witness: Option<String>,
    /// Optional attestation tier for the record.
    pub attestation: Option<AttestationLevel>,
    /// SHA-256 hash of the previous record in the chain (`None` for the first record).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<String>,
    /// SHA-256 chain hash of this record: see [`Custody::compute_hash`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl Custody {
    /// Create a custody record with the required fields.
    #[must_use]
    pub fn new(
        action: impl Into<String>,
        principal: impl Into<String>,
        when: i64,
        authorized: Vec<AuthorizationEvidence>,
    ) -> Self {
        Self {
            action: action.into(),
            principal: principal.into(),
            when,
            authorized,
            why_heuristics: Vec::new(),
            why_claims: Vec::new(),
            simulation: None,
            gates_passed: Vec::new(),
            taint: None,
            result: None,
            witness: None,
            attestation: None,
            prev_hash: None,
            hash: None,
        }
    }

    /// Attach active taint to the record.
    #[must_use]
    pub fn with_taint(mut self, taint: Taint) -> Self {
        if taint.is_active() {
            self.taint = Some(taint);
        }
        self
    }

    /// Attach a result identifier or digest.
    #[must_use]
    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }

    /// Attach an attestation level.
    #[must_use]
    pub fn with_attestation(mut self, attestation: AttestationLevel) -> Self {
        self.attestation = Some(attestation);
        self
    }

    /// Attach gate names that passed before execution.
    #[must_use]
    pub fn with_gates_passed(mut self, gates: Vec<String>) -> Self {
        self.gates_passed = gates;
        self
    }

    /// Attach heuristic explanations.
    #[must_use]
    pub fn with_heuristics(mut self, heuristics: Vec<String>) -> Self {
        self.why_heuristics = heuristics;
        self
    }

    // ── P4-14: Custody chain hash computation ──────────────────────────

    /// Compute this record's chain hash, the lowercase hex SHA-256 of
    /// `prev_hash` (empty for the first record) followed by the canonical
    /// payload: the record's JSON with `prev_hash` and `hash` left out.
    ///
    /// This is the digest `roko knowledge custody` writes and verifies, so
    /// a chain verifies the same way here and there.
    #[must_use]
    pub fn compute_hash(&self) -> String {
        let mut payload = self.clone();
        let prev_hash = payload.prev_hash.take().unwrap_or_default();
        payload.hash = None;
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(serde_json::to_vec(&payload).unwrap_or_default());
        format!("{:x}", hasher.finalize())
    }

    /// Seal this record with computed hash and link to previous.
    ///
    /// Sets `prev_hash` to the given previous hash (or `None` for genesis)
    /// and computes `hash` from `prev_hash` and the canonical payload.
    pub fn seal(&mut self, prev_hash: Option<String>) {
        self.prev_hash = prev_hash;
        self.hash = Some(self.compute_hash());
    }

    /// Verify that this record's hash matches its payload.
    #[must_use]
    pub fn verify_hash(&self) -> bool {
        match &self.hash {
            Some(h) => h == &self.compute_hash(),
            None => false, // Unsealed records fail verification.
        }
    }
}

// ─── CustodyLogger ──────────────────────────────────────────────────

/// Append-only JSONL logger for custody records.
///
/// Each call to [`CustodyLogger::log`] serializes a [`Custody`] record
/// as a single JSON line and appends it to the custody log file. The
/// logger creates the parent directory on first write if it does not
/// exist.
///
/// # Thread safety
///
/// The logger is `Send + Sync` and can be shared across threads. Each
/// `log` call opens the file in append mode, so concurrent writes are
/// safe on POSIX (atomic under `O_APPEND` for small writes).
#[derive(Debug, Clone)]
pub struct CustodyLogger {
    /// Path to the custody JSONL file.
    path: PathBuf,
}

impl CustodyLogger {
    /// Create a logger that writes to the given path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Append a custody record to the log file.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or the file
    /// cannot be opened/written.
    pub fn log(&self, custody: &Custody) -> std::io::Result<()> {
        roko_core::io::append_jsonl(&self.path, custody)
    }

    /// Read all custody records from the log file.
    ///
    /// Returns an empty vec if the file does not exist. Lines that fail
    /// to parse are silently skipped.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read.
    pub fn read_all(&self) -> std::io::Result<Vec<Custody>> {
        let content = match fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let records = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(records)
    }

    /// Return the path to the custody log file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the number of records in the log.
    ///
    /// Returns 0 if the file does not exist or cannot be read.
    #[must_use]
    pub fn count(&self) -> usize {
        self.read_all().map(|records| records.len()).unwrap_or(0)
    }

    // ── P4-14: Chained custody logging ─────────────────────────────────

    /// Append a custody record with chain hash linking.
    ///
    /// Reads the last record's hash (if any) and seals the new record
    /// with `prev_hash` set to the previous record's hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read or written.
    pub fn log_chained(&self, custody: &mut Custody) -> std::io::Result<()> {
        let prev_hash = self.read_all()?.last().and_then(|last| last.hash.clone());
        custody.seal(prev_hash);
        self.log(custody)
    }

    /// Verify the integrity of the entire custody chain.
    ///
    /// Returns `Ok(true)` if every record's hash is valid and each
    /// `prev_hash` links to the preceding record. Returns `Ok(false)`
    /// if any record fails verification.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    pub fn verify_chain(&self) -> std::io::Result<CustodyChainVerification> {
        let records = self.read_all()?;
        let mut result = CustodyChainVerification {
            total_records: records.len(),
            valid: true,
            broken_at: None,
            sealed_count: 0,
            unsealed_count: 0,
        };

        let mut expected_prev: Option<String> = None;

        for (i, record) in records.iter().enumerate() {
            if record.hash.is_none() {
                result.unsealed_count += 1;
                continue;
            }
            result.sealed_count += 1;

            if !record.verify_hash() {
                result.valid = false;
                result.broken_at = Some(i);
                return Ok(result);
            }

            if record.prev_hash != expected_prev {
                result.valid = false;
                result.broken_at = Some(i);
                return Ok(result);
            }

            expected_prev = record.hash.clone();
        }

        Ok(result)
    }
}

/// Result of verifying a custody chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyChainVerification {
    /// Total records in the chain.
    pub total_records: usize,
    /// Whether the chain is valid end-to-end.
    pub valid: bool,
    /// Index of the first broken record, if any.
    pub broken_at: Option<usize>,
    /// Number of records with computed hashes.
    pub sealed_count: usize,
    /// Number of records without hashes (legacy).
    pub unsealed_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custody_record_round_trips_through_serde() {
        let custody = Custody::new("write_file", "agent-001", 1713600000000_i64, vec![])
            .with_taint(Taint::ExternalFetch("https://example.com".into()))
            .with_result("sha256:abc123")
            .with_attestation(AttestationLevel::LocalAgent)
            .with_gates_passed(vec!["compile".into(), "test".into()])
            .with_heuristics(vec!["irreversibility=0.2".into()]);

        let json = serde_json::to_string(&custody).unwrap();
        let decoded: Custody = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.action, "write_file");
        assert_eq!(decoded.principal, "agent-001");
        assert_eq!(decoded.gates_passed.len(), 2);
        assert!(decoded.taint.is_some());
        assert_eq!(decoded.result.as_deref(), Some("sha256:abc123"));
    }

    #[test]
    fn custody_logger_writes_and_reads() {
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("custody.jsonl");
        let logger = CustodyLogger::new(&log_path);

        let c1 = Custody::new("bash", "agent-1", 100, vec![]);
        let c2 = Custody::new("write_file", "agent-2", 200, vec![]).with_taint(Taint::UserInput);

        logger.log(&c1).unwrap();
        logger.log(&c2).unwrap();

        let records = logger.read_all().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].action, "bash");
        assert_eq!(records[1].action, "write_file");
        assert!(records[1].taint.is_some());
    }

    #[test]
    fn custody_logger_count_returns_zero_for_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let logger = CustodyLogger::new(tmp.path().join("nonexistent.jsonl"));
        assert_eq!(logger.count(), 0);
    }

    #[test]
    fn taint_none_is_not_active() {
        assert!(!Taint::None.is_active());
    }

    #[test]
    fn taint_external_fetch_is_active() {
        assert!(Taint::ExternalFetch("url".into()).is_active());
    }

    #[test]
    fn taint_with_none_does_not_set_field() {
        let custody = Custody::new("test", "p", 0, vec![]).with_taint(Taint::None);
        assert!(custody.taint.is_none());
    }

    /// bug-3ba5d8: the chain hash is SHA-256 over `prev_hash` and the
    /// record's JSON, the digest roko-cli's custody chain uses; it used to be
    /// a `DefaultHasher` digest, which is not stable across Rust releases.
    #[test]
    fn custody_hash_is_sha256() {
        let mut first = Custody::new("write_file", "agent-1", 1000, vec![]);
        first.seal(None);
        let first_hash = "f5f0e5873812b7c7ceeca713963ee6360088c087b917b16d5b42c155bcd2a716";
        assert_eq!(first.hash.as_deref(), Some(first_hash));

        let mut second = Custody::new("read_file", "agent-1", 2000, vec![]);
        second.seal(first.hash.clone());
        let second_hash = "8c80f41843ab79f8a5e1c2dd885859c999a45a088c8c6d63a86a7963ca86a783";
        assert_eq!(second.hash.as_deref(), Some(second_hash));
        assert!(second.verify_hash());

        let tmp = tempfile::tempdir().unwrap();
        let logger = CustodyLogger::new(tmp.path().join("custody.jsonl"));
        logger.log_chained(&mut first.clone()).unwrap();
        logger.log_chained(&mut second.clone()).unwrap();
        let chain = logger.verify_chain().unwrap();
        assert!(chain.valid);
        assert_eq!(chain.sealed_count, 2);
    }
}
