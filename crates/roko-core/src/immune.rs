//! Cognitive immune system -- detecting and quarantining compromised knowledge.
//!
//! This module provides the core quarantine mechanism for the Roko immune system.
//! Signals that fail validation (anomalous outputs, hallucinated data, tainted
//! sources) are quarantined instead of being immediately rejected or accepted.
//!
//! # Architecture
//!
//! ```text
//! Signal ──check()──► AnomalyDetector ──score()──► QuarantineDecision
//!                                                       │
//!                          ┌──── Accept (score < threshold)
//!                          ├──── Quarantine (score >= threshold)
//!                          └──── Reject (auto_reject && score >= threshold)
//!
//! QuarantineVault ───── stores quarantined signals for review
//! IncidentLink ──────── connects related taint incidents
//! ImmuneResponse ────── recovery action after quarantine review
//! ```

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
#[cfg(test)]
use std::{
    fs::{self, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ContentHash, Taint};

/// Default maximum number of review entries retained by a quarantine vault.
pub const DEFAULT_QUARANTINE_VAULT_CAPACITY: usize = 50;
/// Strict maximum serialized size accepted for a quarantine vault.
pub const MAX_QUARANTINE_VAULT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_QUARANTINE_LABEL_BYTES: usize = 256;
const MAX_REVIEWER_NOTES_BYTES: usize = 4 * 1024;
const MAX_ANOMALY_DIMENSIONS: usize = 32;
const MAX_SERIALIZED_TAINT_BYTES: usize = 4 * 1024;

/// An anomaly score computed for a signal during immune screening.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnomalyScore {
    /// Overall anomaly score in [0, 1]. Higher = more anomalous.
    pub score: f64,
    /// Per-dimension scores that contributed to the overall score.
    pub dimensions: HashMap<String, f64>,
    /// Which taint classification was detected, if any.
    pub detected_taint: Option<Taint>,
}

impl AnomalyScore {
    /// Create a clean (non-anomalous) score.
    #[must_use]
    pub fn clean() -> Self {
        Self {
            score: 0.0,
            dimensions: HashMap::new(),
            detected_taint: None,
        }
    }

    /// Create an anomaly score with a single dimension.
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            dimensions: HashMap::new(),
            detected_taint: None,
        }
    }

    /// Add a dimension score.
    pub fn with_dimension(mut self, name: impl Into<String>, score: f64) -> Self {
        self.dimensions.insert(name.into(), score.clamp(0.0, 1.0));
        self
    }

    /// Attach detected taint.
    pub fn with_taint(mut self, taint: Taint) -> Self {
        self.detected_taint = Some(taint);
        self
    }

    /// Whether the score exceeds the given threshold.
    #[must_use]
    pub fn exceeds_threshold(&self, threshold: f64) -> bool {
        self.score >= threshold
    }
}

/// Decision made by the immune system about an engram.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineDecision {
    /// Engram is clean, allow it through.
    Accept,
    /// Engram is suspicious, quarantine for review.
    Quarantine,
    /// Engram is flagged and auto-reject is enabled.
    Reject,
}

/// A quarantined engram entry in the vault.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineEntry {
    /// Content hash of the quarantined engram.
    pub hash: ContentHash,
    /// Anomaly score that triggered quarantine.
    pub anomaly_score: AnomalyScore,
    /// When the engram was quarantined.
    pub quarantined_at: DateTime<Utc>,
    /// Current review status.
    pub status: QuarantineStatus,
    /// Optional reviewer notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer_notes: Option<String>,
    /// Incident links to related quarantine events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incident_links: Vec<IncidentLink>,
    /// Stable boundary scope used for atomic same-source/session linking.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incident_scopes: Vec<String>,
}

/// Status of a quarantined entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineStatus {
    /// Awaiting review.
    Pending,
    /// Reviewed and approved (will be released).
    Approved,
    /// Reviewed and rejected (will be purged).
    Rejected,
    /// Escalated for higher-level review.
    Escalated,
}

/// A link between related taint incidents.
///
/// When multiple engrams are quarantined due to related causes (e.g., the same
/// tainted source), incident links connect them for batch review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncidentLink {
    /// The related quarantine entry hash.
    pub related_hash: ContentHash,
    /// Relationship description.
    pub relation: IncidentRelation,
    /// When the link was established.
    pub linked_at: DateTime<Utc>,
}

/// Kind of relationship between incident entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentRelation {
    /// Both engrams came from the same tainted source.
    SameSource,
    /// One engram propagated taint to the other.
    Propagated,
    /// Both engrams contradict each other.
    Contradiction,
    /// Both engrams were produced in the same agent session.
    SameSession,
}

/// Recovery action taken after quarantine review.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmuneResponse {
    /// Which quarantine entry this response applies to.
    pub entry_hash: ContentHash,
    /// The action taken.
    pub action: ResponseAction,
    /// When the response was issued.
    pub responded_at: DateTime<Utc>,
    /// Optional description of the recovery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Action taken in response to a quarantine review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseAction {
    /// Release the engram back into the hot substrate.
    Release,
    /// Purge the engram permanently.
    Purge,
    /// Retag the engram with corrected taint classification.
    Retag,
    /// Move to cold storage for archival.
    Archive,
    /// Isolate the producing agent while the incident is investigated.
    IsolateAgent,
    /// Temporarily reduce the producing agent's action rate.
    RateLimitAgent,
}

impl QuarantineStatus {
    /// Terminal action implied by a completed review status.
    #[must_use]
    pub const fn response_action(self) -> Option<ResponseAction> {
        match self {
            Self::Approved => Some(ResponseAction::Release),
            Self::Rejected => Some(ResponseAction::Purge),
            Self::Pending | Self::Escalated => None,
        }
    }
}

/// Assessed severity for one immune-system finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreatSeverity {
    /// Background variation; no containment required.
    Low,
    /// Suspicious behavior that warrants bounded automatic mitigation.
    Medium,
    /// Strong threat evidence requiring isolation and human review.
    High,
    /// Immediate danger requiring rejection and escalation.
    Critical,
}

/// Layer 1 output: perceived anomaly attached to its exact target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmunePerception {
    /// Signal under review.
    pub target: ContentHash,
    /// Immutable anomaly evidence.
    pub anomaly: AnomalyScore,
}

/// Layer 2 output: threat classification derived from perception evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmuneAssessment {
    /// Perception that was assessed.
    pub perception: ImmunePerception,
    /// Deterministic severity classification.
    pub severity: ThreatSeverity,
}

/// Layer 3 output: containment decision and its exact effect scope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmuneContainment {
    /// Assessment that selected this response.
    pub assessment: ImmuneAssessment,
    /// Quarantine decision applied to the target.
    pub decision: QuarantineDecision,
    /// Concrete response, if mitigation is required.
    pub action: Option<ResponseAction>,
    /// Signals the response would affect.
    pub affected_signals: Vec<ContentHash>,
}

/// Layer 4 output: collateral-damage validation of the proposed response.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmuneValidation {
    /// Proposed containment response.
    pub containment: ImmuneContainment,
    /// Whether the response is confined to the finding's target.
    pub collateral_safe: bool,
}

/// Layer 5 output and complete result of the immune pipeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmunePipelineResult {
    /// Validated containment response.
    pub validation: ImmuneValidation,
    /// High/Critical threats and unsafe response scopes require human review.
    pub escalation_required: bool,
}

/// Fixed five-layer cognitive immune pipeline.
///
/// [`Self::run`] fixes the canonical order: perception -> assessment ->
/// response -> validation -> escalation. Individual pure stages are exposed so
/// typed runtime Cells can host the exact same policy logic; those Cells own
/// the fail-closed predecessor validation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImmunePipeline {
    quarantine_threshold: f64,
    critical_threshold: f64,
}

impl ImmunePipeline {
    /// Construct a pipeline with explicit bounded thresholds.
    #[must_use]
    pub fn new(quarantine_threshold: f64, critical_threshold: f64) -> Self {
        let quarantine_threshold = quarantine_threshold.clamp(0.0, 1.0);
        Self {
            quarantine_threshold,
            critical_threshold: critical_threshold.clamp(quarantine_threshold, 1.0),
        }
    }

    /// Execute all five layers without IO or mutable global state.
    #[must_use]
    pub fn run(
        &self,
        target: ContentHash,
        anomaly: AnomalyScore,
        affected_signals: Vec<ContentHash>,
    ) -> ImmunePipelineResult {
        let perception = self.perceive(target, anomaly);
        let assessment = self.assess(perception);
        let containment = self.respond(assessment, affected_signals);
        let validation = self.validate(containment);
        self.escalate(validation)
    }

    /// Stage 1: bind immutable anomaly evidence to its exact target.
    #[must_use]
    pub fn perceive(&self, target: ContentHash, anomaly: AnomalyScore) -> ImmunePerception {
        ImmunePerception { target, anomaly }
    }

    /// Stage 2: classify a perception using the pipeline's bounded thresholds.
    #[must_use]
    pub fn assess(&self, perception: ImmunePerception) -> ImmuneAssessment {
        let score = perception.anomaly.score;
        let severity = if score >= self.critical_threshold {
            ThreatSeverity::Critical
        } else if score >= self.quarantine_threshold {
            ThreatSeverity::High
        } else if score >= self.quarantine_threshold * 0.5 {
            ThreatSeverity::Medium
        } else {
            ThreatSeverity::Low
        };
        ImmuneAssessment {
            perception,
            severity,
        }
    }

    /// Stage 3: select containment and bind it to an explicit effect scope.
    #[must_use]
    pub fn respond(
        &self,
        assessment: ImmuneAssessment,
        mut affected_signals: Vec<ContentHash>,
    ) -> ImmuneContainment {
        let (decision, action) = match assessment.severity {
            ThreatSeverity::Low => (QuarantineDecision::Accept, None),
            ThreatSeverity::Medium => (
                QuarantineDecision::Quarantine,
                Some(ResponseAction::RateLimitAgent),
            ),
            ThreatSeverity::High => (
                QuarantineDecision::Quarantine,
                Some(ResponseAction::IsolateAgent),
            ),
            ThreatSeverity::Critical => (QuarantineDecision::Reject, Some(ResponseAction::Purge)),
        };
        if action.is_some() && affected_signals.is_empty() {
            affected_signals.push(assessment.perception.target);
        }
        ImmuneContainment {
            assessment,
            decision,
            action,
            affected_signals,
        }
    }

    /// Stage 4: reject containment scopes that exceed the exact finding target.
    #[must_use]
    pub fn validate(&self, containment: ImmuneContainment) -> ImmuneValidation {
        let target = containment.assessment.perception.target;
        let collateral_safe = containment.action.is_none()
            || containment
                .affected_signals
                .iter()
                .all(|affected| *affected == target);
        ImmuneValidation {
            containment,
            collateral_safe,
        }
    }

    /// Stage 5: require human review for unsafe scopes and high-severity threats.
    #[must_use]
    pub fn escalate(&self, validation: ImmuneValidation) -> ImmunePipelineResult {
        let severity = validation.containment.assessment.severity;
        ImmunePipelineResult {
            escalation_required: !validation.collateral_safe
                || matches!(severity, ThreatSeverity::High | ThreatSeverity::Critical),
            validation,
        }
    }
}

impl Default for ImmunePipeline {
    fn default() -> Self {
        Self::new(0.8, 0.95)
    }
}

/// Counts of entries in each quarantine review state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineStats {
    /// All entries currently retained in the vault.
    pub total: usize,
    /// Entries awaiting review.
    pub pending: usize,
    /// Entries approved for release.
    pub approved: usize,
    /// Entries rejected for purge.
    pub rejected: usize,
    /// Entries awaiting higher-level review.
    pub escalated: usize,
}

/// Persistent quarantine vault for holding suspect engrams.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineVault {
    /// Entries indexed by content hash.
    entries: HashMap<ContentHash, QuarantineEntry>,
    /// Anomaly threshold for quarantine decisions.
    threshold: f64,
    /// Maximum number of entries before escalation.
    max_entries: usize,
    /// Whether to auto-reject above-threshold engrams.
    auto_reject: bool,
}

impl QuarantineVault {
    /// Create a new vault with the given configuration.
    #[must_use]
    pub fn new(threshold: f64, max_entries: usize, auto_reject: bool) -> Self {
        Self {
            entries: HashMap::new(),
            threshold: threshold.clamp(0.0, 1.0),
            max_entries,
            auto_reject,
        }
    }

    /// Create a vault with default settings (threshold=0.8, max=50, no auto-reject).
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(0.8, DEFAULT_QUARANTINE_VAULT_CAPACITY, false)
    }

    /// Screen an engram and decide whether to quarantine it.
    #[must_use]
    pub fn screen(&self, anomaly: &AnomalyScore) -> QuarantineDecision {
        if anomaly.exceeds_threshold(self.threshold) {
            if self.auto_reject {
                QuarantineDecision::Reject
            } else {
                QuarantineDecision::Quarantine
            }
        } else {
            QuarantineDecision::Accept
        }
    }

    /// Add an engram to quarantine.
    ///
    /// Returns `true` when the hash is present after the call (including a
    /// retry), or `false` when a new entry cannot be added because the vault is
    /// full.
    pub fn quarantine(&mut self, hash: ContentHash, anomaly: AnomalyScore) -> bool {
        // Retried delivery must not erase an existing review decision or links.
        if self.entries.contains_key(&hash) {
            return true;
        }
        if self.entries.len() >= self.max_entries {
            return false;
        }

        let entry = QuarantineEntry {
            hash,
            anomaly_score: anomaly,
            quarantined_at: Utc::now(),
            status: QuarantineStatus::Pending,
            reviewer_notes: None,
            incident_links: Vec::new(),
            incident_scopes: Vec::new(),
        };
        self.entries.insert(hash, entry);
        true
    }

    /// Add one entry and link it to every entry already carrying the exact
    /// same bounded scope. Callers must hold their durable vault transaction
    /// lock across this operation.
    pub fn quarantine_scoped(
        &mut self,
        hash: ContentHash,
        anomaly: AnomalyScore,
        scope: &str,
        relation: IncidentRelation,
    ) -> Result<bool, &'static str> {
        if scope.trim().is_empty() || scope.len() > MAX_QUARANTINE_LABEL_BYTES {
            return Err("quarantine incident scope is invalid");
        }
        if let Some(existing) = self.entries.get_mut(&hash) {
            if !existing
                .incident_scopes
                .iter()
                .any(|existing_scope| existing_scope == scope)
            {
                if existing.incident_scopes.len() >= DEFAULT_QUARANTINE_VAULT_CAPACITY {
                    return Err("quarantine entry scope capacity is exhausted");
                }
                existing.incident_scopes.push(scope.to_string());
            }
        } else {
            if !self.quarantine(hash, anomaly) {
                return Ok(false);
            }
            let Some(entry) = self.entries.get_mut(&hash) else {
                return Err("newly quarantined entry disappeared");
            };
            entry.incident_scopes.push(scope.to_string());
        }
        let related = self
            .entries
            .iter()
            .filter_map(|(related_hash, entry)| {
                (*related_hash != hash
                    && entry
                        .incident_scopes
                        .iter()
                        .any(|existing_scope| existing_scope == scope))
                .then_some(*related_hash)
            })
            .collect::<Vec<_>>();
        for related_hash in related {
            let _ = self.link_incidents(hash, related_hash, relation);
        }
        Ok(true)
    }

    /// Review and update the status of a quarantined entry.
    pub fn review(
        &mut self,
        hash: &ContentHash,
        status: QuarantineStatus,
        notes: Option<String>,
    ) -> bool {
        if let Some(entry) = self.entries.get_mut(hash) {
            entry.status = status;
            entry.reviewer_notes = notes;
            true
        } else {
            false
        }
    }

    /// Apply one review decision to every existing unique hash in `hashes`.
    ///
    /// Missing hashes are ignored. The return value is the number of distinct
    /// entries reviewed, so retrying a batch with duplicate hashes is stable.
    pub fn batch_review<I, H>(
        &mut self,
        hashes: I,
        status: QuarantineStatus,
        notes: Option<String>,
    ) -> usize
    where
        I: IntoIterator<Item = H>,
        H: Borrow<ContentHash>,
    {
        let mut seen = HashSet::new();
        let mut reviewed = 0;
        for hash in hashes {
            let hash = *hash.borrow();
            if seen.insert(hash) && self.review(&hash, status, notes.clone()) {
                reviewed += 1;
            }
        }
        reviewed
    }

    /// Link two quarantine entries as related incidents.
    pub fn link_incidents(
        &mut self,
        a: ContentHash,
        b: ContentHash,
        relation: IncidentRelation,
    ) -> bool {
        if a == b || !self.entries.contains_key(&a) || !self.entries.contains_key(&b) {
            return false;
        }

        let now = Utc::now();

        if let Some(entry_a) = self.entries.get_mut(&a)
            && !entry_a
                .incident_links
                .iter()
                .any(|link| link.related_hash == b && link.relation == relation)
        {
            entry_a.incident_links.push(IncidentLink {
                related_hash: b,
                relation,
                linked_at: now,
            });
        }
        if let Some(entry_b) = self.entries.get_mut(&b)
            && !entry_b
                .incident_links
                .iter()
                .any(|link| link.related_hash == a && link.relation == relation)
        {
            entry_b.incident_links.push(IncidentLink {
                related_hash: a,
                relation,
                linked_at: now,
            });
        }

        true
    }

    /// Return every transitively linked incident, excluding `hash` itself.
    ///
    /// Traversal is breadth-first and cycle-safe. Dangling links are ignored;
    /// vaults loaded through [`Self::load`] reject them as corrupt.
    #[must_use]
    pub fn incidents_for(&self, hash: &ContentHash) -> Vec<&QuarantineEntry> {
        if !self.entries.contains_key(hash) {
            return Vec::new();
        }
        let mut visited = HashSet::from([*hash]);
        let mut queue = VecDeque::from([*hash]);
        let mut incidents = Vec::new();
        while let Some(current) = queue.pop_front() {
            let Some(entry) = self.entries.get(&current) else {
                continue;
            };
            for link in &entry.incident_links {
                if visited.insert(link.related_hash)
                    && let Some(related) = self.entries.get(&link.related_hash)
                {
                    incidents.push(related);
                    queue.push_back(link.related_hash);
                }
            }
        }
        incidents
    }

    /// Get a quarantine entry by hash.
    #[must_use]
    pub fn get(&self, hash: &ContentHash) -> Option<&QuarantineEntry> {
        self.entries.get(hash)
    }

    /// All pending entries.
    #[must_use]
    pub fn pending(&self) -> Vec<&QuarantineEntry> {
        self.entries
            .values()
            .filter(|e| e.status == QuarantineStatus::Pending)
            .collect()
    }

    /// Remove approved and rejected entries, returning the released hashes.
    pub fn drain_resolved(&mut self) -> (Vec<ContentHash>, Vec<ContentHash>) {
        let mut released = Vec::new();
        let mut purged = Vec::new();

        self.entries.retain(|hash, entry| match entry.status {
            QuarantineStatus::Approved => {
                released.push(*hash);
                false
            }
            QuarantineStatus::Rejected => {
                purged.push(*hash);
                false
            }
            _ => true,
        });
        for entry in self.entries.values_mut() {
            entry.incident_links.retain(|link| {
                !released.contains(&link.related_hash) && !purged.contains(&link.related_hash)
            });
        }

        (released, purged)
    }

    /// Return counts for all review states currently retained in the vault.
    #[must_use]
    pub fn stats(&self) -> QuarantineStats {
        let mut stats = QuarantineStats {
            total: self.entries.len(),
            ..QuarantineStats::default()
        };
        for entry in self.entries.values() {
            match entry.status {
                QuarantineStatus::Pending => stats.pending += 1,
                QuarantineStatus::Approved => stats.approved += 1,
                QuarantineStatus::Rejected => stats.rejected += 1,
                QuarantineStatus::Escalated => stats.escalated += 1,
            }
        }
        stats
    }

    /// Atomically persist the complete vault, including configuration and links.
    #[cfg(test)]
    fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.validate_integrity()?;
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        fs::create_dir_all(&parent)?;
        let file_name = path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "quarantine path has no file name",
            )
        })?;
        static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            sequence
        ));
        let result = (|| {
            let file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, self)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            writer.write_all(b"\n")?;
            writer.flush()?;
            writer.get_ref().sync_all()?;
            drop(writer);
            fs::rename(&temporary, path)?;
            #[cfg(unix)]
            File::open(&parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// Load and strictly validate a previously persisted vault.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = File::open(path)?;
        if file.metadata()?.len() > MAX_QUARANTINE_VAULT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "quarantine vault exceeds its byte limit",
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_QUARANTINE_VAULT_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_QUARANTINE_VAULT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "quarantine vault exceeds its byte limit",
            ));
        }
        let vault: Self = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        vault.validate_integrity()?;
        Ok(vault)
    }

    /// Strictly validate persisted vault invariants without mutating state.
    ///
    /// Transactional adapters call this before and after mutation so a
    /// structurally valid but internally inconsistent vault is never silently
    /// overwritten.
    pub fn validate_integrity(&self) -> io::Result<()> {
        let invalid = |message| io::Error::new(io::ErrorKind::InvalidData, message);
        if !self.threshold.is_finite() || !(0.0..=1.0).contains(&self.threshold) {
            return Err(invalid("quarantine threshold is outside [0, 1]"));
        }
        if self.max_entries == 0 || self.max_entries > DEFAULT_QUARANTINE_VAULT_CAPACITY {
            return Err(invalid("quarantine vault capacity is invalid"));
        }
        if self.entries.len() > self.max_entries {
            return Err(invalid("quarantine entry count exceeds vault capacity"));
        }
        for (hash, entry) in &self.entries {
            if *hash != entry.hash {
                return Err(invalid(
                    "quarantine entry hash does not match its index key",
                ));
            }
            if entry.anomaly_score.dimensions.len() > MAX_ANOMALY_DIMENSIONS
                || entry.anomaly_score.dimensions.keys().any(|dimension| {
                    dimension.trim().is_empty() || dimension.len() > MAX_QUARANTINE_LABEL_BYTES
                })
                || entry
                    .anomaly_score
                    .detected_taint
                    .as_ref()
                    .is_some_and(|taint| {
                        serde_json::to_vec(taint)
                            .map(|bytes| bytes.len() > MAX_SERIALIZED_TAINT_BYTES)
                            .unwrap_or(true)
                    })
            {
                return Err(invalid("quarantine entry anomaly evidence exceeds bounds"));
            }
            if entry
                .reviewer_notes
                .as_ref()
                .is_some_and(|notes| notes.len() > MAX_REVIEWER_NOTES_BYTES)
                || entry.incident_scopes.len() > DEFAULT_QUARANTINE_VAULT_CAPACITY
                || entry.incident_scopes.iter().any(|scope| {
                    scope.trim().is_empty() || scope.len() > MAX_QUARANTINE_LABEL_BYTES
                })
                || entry.incident_links.len() >= self.max_entries
            {
                return Err(invalid("quarantine entry metadata exceeds bounds"));
            }
            let mut scopes = HashSet::new();
            if entry
                .incident_scopes
                .iter()
                .any(|scope| !scopes.insert(scope))
            {
                return Err(invalid(
                    "quarantine entry contains duplicate incident scopes",
                ));
            }
            if !entry.anomaly_score.score.is_finite()
                || !(0.0..=1.0).contains(&entry.anomaly_score.score)
                || entry
                    .anomaly_score
                    .dimensions
                    .values()
                    .any(|score| !score.is_finite() || !(0.0..=1.0).contains(score))
            {
                return Err(invalid(
                    "quarantine entry contains an invalid anomaly score",
                ));
            }
            let mut links = HashSet::new();
            for link in &entry.incident_links {
                if link.related_hash == *hash
                    || !links.insert((link.related_hash, link.relation))
                    || !self.entries.contains_key(&link.related_hash)
                {
                    return Err(invalid(
                        "quarantine entry contains an invalid incident link",
                    ));
                }
                let reciprocal = self.entries[&link.related_hash]
                    .incident_links
                    .iter()
                    .any(|other| other.related_hash == *hash && other.relation == link.relation);
                if !reciprocal {
                    return Err(invalid("quarantine incident link is not reciprocal"));
                }
            }
        }
        Ok(())
    }

    /// Current number of quarantined entries.
    #[must_use]
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Whether the vault has reached maximum capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.max_entries
    }

    /// Whether escalation is needed (vault is >= 80% full).
    #[must_use]
    pub fn needs_escalation(&self) -> bool {
        self.entries.len() as f64 >= self.max_entries as f64 * 0.8
    }
}

impl Default for QuarantineVault {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_hash(n: u8) -> ContentHash {
        let mut bytes = [0u8; 32];
        bytes[0] = n;
        ContentHash(bytes)
    }

    #[test]
    fn anomaly_score_clean() {
        let score = AnomalyScore::clean();
        assert_eq!(score.score, 0.0);
        assert!(!score.exceeds_threshold(0.5));
    }

    #[test]
    fn anomaly_score_from_value() {
        let score = AnomalyScore::from_score(0.9);
        assert!(score.exceeds_threshold(0.8));
        assert!(!score.exceeds_threshold(0.95));
    }

    #[test]
    fn anomaly_score_with_dimensions() {
        let score = AnomalyScore::from_score(0.7)
            .with_dimension("coherence", 0.3)
            .with_dimension("novelty", 0.9);
        assert_eq!(score.dimensions.len(), 2);
        assert_eq!(score.dimensions["coherence"], 0.3);
    }

    #[test]
    fn anomaly_score_clamping() {
        let score = AnomalyScore::from_score(2.0);
        assert_eq!(score.score, 1.0);

        let score = AnomalyScore::from_score(-1.0);
        assert_eq!(score.score, 0.0);
    }

    #[test]
    fn immune_pipeline_accepts_low_severity_without_containment() {
        let result = ImmunePipeline::new(0.8, 0.95).run(
            dummy_hash(1),
            AnomalyScore::from_score(0.2),
            Vec::new(),
        );

        assert_eq!(
            result.validation.containment.assessment.severity,
            ThreatSeverity::Low
        );
        assert_eq!(
            result.validation.containment.decision,
            QuarantineDecision::Accept
        );
        assert_eq!(result.validation.containment.action, None);
        assert!(result.validation.collateral_safe);
        assert!(!result.escalation_required);
    }

    #[test]
    fn immune_pipeline_auto_contains_medium_severity_at_exact_target() {
        let target = dummy_hash(2);
        let result =
            ImmunePipeline::new(0.8, 0.95).run(target, AnomalyScore::from_score(0.4), Vec::new());

        assert_eq!(
            result.validation.containment.assessment.severity,
            ThreatSeverity::Medium
        );
        assert_eq!(
            result.validation.containment.decision,
            QuarantineDecision::Quarantine
        );
        assert_eq!(
            result.validation.containment.action,
            Some(ResponseAction::RateLimitAgent)
        );
        assert_eq!(result.validation.containment.affected_signals, vec![target]);
        assert!(result.validation.collateral_safe);
        assert!(!result.escalation_required);
    }

    #[test]
    fn immune_pipeline_isolates_and_escalates_high_severity() {
        let result = ImmunePipeline::new(0.8, 0.95).run(
            dummy_hash(3),
            AnomalyScore::from_score(0.8),
            Vec::new(),
        );

        assert_eq!(
            result.validation.containment.assessment.severity,
            ThreatSeverity::High
        );
        assert_eq!(
            result.validation.containment.action,
            Some(ResponseAction::IsolateAgent)
        );
        assert!(result.validation.collateral_safe);
        assert!(result.escalation_required);
    }

    #[test]
    fn immune_pipeline_rejects_and_escalates_critical_severity() {
        let result = ImmunePipeline::new(0.8, 0.95).run(
            dummy_hash(4),
            AnomalyScore::from_score(0.95),
            Vec::new(),
        );

        assert_eq!(
            result.validation.containment.assessment.severity,
            ThreatSeverity::Critical
        );
        assert_eq!(
            result.validation.containment.decision,
            QuarantineDecision::Reject
        );
        assert_eq!(
            result.validation.containment.action,
            Some(ResponseAction::Purge)
        );
        assert!(result.escalation_required);
    }

    #[test]
    fn immune_pipeline_escalates_collateral_scope() {
        let result = ImmunePipeline::new(0.8, 0.95).run(
            dummy_hash(5),
            AnomalyScore::from_score(0.5),
            vec![dummy_hash(6)],
        );

        assert_eq!(
            result.validation.containment.assessment.severity,
            ThreatSeverity::Medium
        );
        assert!(!result.validation.collateral_safe);
        assert!(result.escalation_required);
    }

    #[test]
    fn immune_pipeline_normalizes_thresholds() {
        let result = ImmunePipeline::new(2.0, -1.0).run(
            dummy_hash(7),
            AnomalyScore::from_score(0.99),
            Vec::new(),
        );

        assert_eq!(
            result.validation.containment.assessment.severity,
            ThreatSeverity::Medium
        );
        assert!(!result.escalation_required);
    }

    #[test]
    fn screen_accept_below_threshold() {
        let vault = QuarantineVault::new(0.8, 50, false);
        let score = AnomalyScore::from_score(0.5);
        assert_eq!(vault.screen(&score), QuarantineDecision::Accept);
    }

    #[test]
    fn screen_quarantine_above_threshold() {
        let vault = QuarantineVault::new(0.8, 50, false);
        let score = AnomalyScore::from_score(0.9);
        assert_eq!(vault.screen(&score), QuarantineDecision::Quarantine);
    }

    #[test]
    fn screen_reject_with_auto_reject() {
        let vault = QuarantineVault::new(0.8, 50, true);
        let score = AnomalyScore::from_score(0.9);
        assert_eq!(vault.screen(&score), QuarantineDecision::Reject);
    }

    #[test]
    fn quarantine_and_retrieve() {
        let mut vault = QuarantineVault::with_defaults();
        let hash = dummy_hash(1);
        let score = AnomalyScore::from_score(0.85);

        assert!(vault.quarantine(hash, score.clone()));
        assert_eq!(vault.count(), 1);

        let entry = vault.get(&hash).unwrap();
        assert_eq!(entry.status, QuarantineStatus::Pending);
        assert_eq!(entry.anomaly_score.score, 0.85);
    }

    #[test]
    fn quarantine_respects_capacity() {
        let mut vault = QuarantineVault::new(0.5, 2, false);

        assert!(vault.quarantine(dummy_hash(1), AnomalyScore::from_score(0.6)));
        assert!(vault.quarantine(dummy_hash(2), AnomalyScore::from_score(0.7)));
        assert!(!vault.quarantine(dummy_hash(3), AnomalyScore::from_score(0.8)));
        assert!(vault.is_full());
    }

    #[test]
    fn review_updates_status() {
        let mut vault = QuarantineVault::with_defaults();
        let hash = dummy_hash(1);
        vault.quarantine(hash, AnomalyScore::from_score(0.9));

        assert!(vault.review(&hash, QuarantineStatus::Approved, Some("looks fine".into())));
        let entry = vault.get(&hash).unwrap();
        assert_eq!(entry.status, QuarantineStatus::Approved);
        assert_eq!(entry.reviewer_notes.as_deref(), Some("looks fine"));
    }

    #[test]
    fn review_nonexistent_returns_false() {
        let mut vault = QuarantineVault::with_defaults();
        assert!(!vault.review(&dummy_hash(99), QuarantineStatus::Rejected, None));
    }

    #[test]
    fn drain_resolved_separates_approved_and_rejected() {
        let mut vault = QuarantineVault::with_defaults();
        let h1 = dummy_hash(1);
        let h2 = dummy_hash(2);
        let h3 = dummy_hash(3);

        vault.quarantine(h1, AnomalyScore::from_score(0.9));
        vault.quarantine(h2, AnomalyScore::from_score(0.85));
        vault.quarantine(h3, AnomalyScore::from_score(0.95));

        vault.review(&h1, QuarantineStatus::Approved, None);
        vault.review(&h2, QuarantineStatus::Rejected, None);
        // h3 stays pending

        let (released, purged) = vault.drain_resolved();
        assert_eq!(released.len(), 1);
        assert!(released.contains(&h1));
        assert_eq!(purged.len(), 1);
        assert!(purged.contains(&h2));
        assert_eq!(vault.count(), 1); // only h3 remains
    }

    #[test]
    fn pending_returns_only_pending() {
        let mut vault = QuarantineVault::with_defaults();
        let h1 = dummy_hash(1);
        let h2 = dummy_hash(2);

        vault.quarantine(h1, AnomalyScore::from_score(0.9));
        vault.quarantine(h2, AnomalyScore::from_score(0.85));
        vault.review(&h1, QuarantineStatus::Approved, None);

        let pending = vault.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].hash, h2);
    }

    #[test]
    fn link_incidents_bidirectional() {
        let mut vault = QuarantineVault::with_defaults();
        let h1 = dummy_hash(1);
        let h2 = dummy_hash(2);

        vault.quarantine(h1, AnomalyScore::from_score(0.9));
        vault.quarantine(h2, AnomalyScore::from_score(0.85));

        assert!(vault.link_incidents(h1, h2, IncidentRelation::SameSource));

        let e1 = vault.get(&h1).unwrap();
        assert_eq!(e1.incident_links.len(), 1);
        assert_eq!(e1.incident_links[0].related_hash, h2);

        let e2 = vault.get(&h2).unwrap();
        assert_eq!(e2.incident_links.len(), 1);
        assert_eq!(e2.incident_links[0].related_hash, h1);
    }

    #[test]
    fn link_fails_for_missing_entries() {
        let mut vault = QuarantineVault::with_defaults();
        assert!(!vault.link_incidents(dummy_hash(1), dummy_hash(2), IncidentRelation::SameSource));
    }

    #[test]
    fn batch_review_is_idempotent_and_stats_preserve_status_semantics() {
        let mut vault = QuarantineVault::with_defaults();
        let hashes = [dummy_hash(1), dummy_hash(2), dummy_hash(3), dummy_hash(4)];
        for hash in hashes {
            assert!(vault.quarantine(hash, AnomalyScore::from_score(0.9)));
        }

        assert_eq!(
            vault.batch_review(
                &[hashes[0], hashes[0], hashes[1], dummy_hash(99)],
                QuarantineStatus::Approved,
                Some("same-source incident cleared".into()),
            ),
            2
        );
        assert!(vault.review(
            &hashes[2],
            QuarantineStatus::Rejected,
            Some("confirmed".into())
        ));
        assert!(vault.review(&hashes[3], QuarantineStatus::Escalated, None));

        assert_eq!(
            vault.stats(),
            QuarantineStats {
                total: 4,
                pending: 0,
                approved: 2,
                rejected: 1,
                escalated: 1,
            }
        );
        assert_eq!(
            QuarantineStatus::Approved.response_action(),
            Some(ResponseAction::Release)
        );
        assert_eq!(
            QuarantineStatus::Rejected.response_action(),
            Some(ResponseAction::Purge)
        );
        assert_eq!(QuarantineStatus::Pending.response_action(), None);
        assert_eq!(QuarantineStatus::Escalated.response_action(), None);
    }

    #[test]
    fn incidents_for_is_transitive_cycle_safe_and_links_are_idempotent() {
        let mut vault = QuarantineVault::with_defaults();
        let a = dummy_hash(1);
        let b = dummy_hash(2);
        let c = dummy_hash(3);
        for hash in [a, b, c] {
            assert!(vault.quarantine(hash, AnomalyScore::from_score(0.9)));
        }

        assert!(vault.link_incidents(a, b, IncidentRelation::SameSource));
        assert!(vault.link_incidents(b, c, IncidentRelation::Propagated));
        assert!(vault.link_incidents(c, a, IncidentRelation::SameSession));
        assert!(vault.link_incidents(a, b, IncidentRelation::SameSource));
        assert!(!vault.link_incidents(a, a, IncidentRelation::Contradiction));

        let incidents: HashSet<_> = vault
            .incidents_for(&a)
            .into_iter()
            .map(|entry| entry.hash)
            .collect();
        assert_eq!(incidents, HashSet::from([b, c]));
        assert_eq!(vault.get(&a).unwrap().incident_links.len(), 2);
        assert_eq!(vault.get(&b).unwrap().incident_links.len(), 2);
        assert!(vault.incidents_for(&dummy_hash(99)).is_empty());
    }

    #[test]
    fn repeated_quarantine_preserves_review_and_drain_removes_dangling_links() {
        let mut vault = QuarantineVault::with_defaults();
        let approved = dummy_hash(1);
        let pending = dummy_hash(2);
        assert!(vault.quarantine(approved, AnomalyScore::from_score(0.81)));
        assert!(vault.quarantine(pending, AnomalyScore::from_score(0.82)));
        assert!(vault.link_incidents(approved, pending, IncidentRelation::SameSource));
        assert!(vault.review(
            &approved,
            QuarantineStatus::Approved,
            Some("reviewed".into())
        ));

        assert!(vault.quarantine(approved, AnomalyScore::from_score(1.0)));
        assert_eq!(
            vault.get(&approved).unwrap().status,
            QuarantineStatus::Approved
        );
        assert_eq!(vault.get(&approved).unwrap().anomaly_score.score, 0.81);

        let (released, purged) = vault.drain_resolved();
        assert_eq!(released, vec![approved]);
        assert!(purged.is_empty());
        assert!(vault.get(&pending).unwrap().incident_links.is_empty());
    }

    #[test]
    fn vault_atomic_persistence_roundtrips_complete_graph_and_configuration() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("nested/.roko/quarantine.json");
        let mut vault = QuarantineVault::new(0.73, 9, true);
        let hashes = [dummy_hash(1), dummy_hash(2), dummy_hash(3), dummy_hash(4)];
        let anomaly_scores = [0.8125, 0.875, 0.9375, 1.0];
        let source_scores = [0.25, 0.5, 0.75, 1.0];
        for (index, hash) in hashes.into_iter().enumerate() {
            assert!(
                vault.quarantine(
                    hash,
                    AnomalyScore::from_score(anomaly_scores[index])
                        .with_dimension("source", source_scores[index]),
                )
            );
        }
        assert!(vault.review(
            &hashes[1],
            QuarantineStatus::Approved,
            Some("release".into())
        ));
        assert!(vault.review(&hashes[2], QuarantineStatus::Rejected, Some("purge".into())));
        assert!(vault.review(
            &hashes[3],
            QuarantineStatus::Escalated,
            Some("human".into())
        ));
        assert!(vault.link_incidents(hashes[0], hashes[1], IncidentRelation::SameSource));
        assert!(vault.link_incidents(hashes[1], hashes[2], IncidentRelation::Propagated));
        assert!(vault.link_incidents(hashes[2], hashes[3], IncidentRelation::Contradiction));

        vault.save(&path).expect("first atomic save");
        vault.save(&path).expect("idempotent replacement save");
        let restored = QuarantineVault::load(&path).expect("load persisted vault");

        assert_eq!(restored, vault);
        assert_eq!(
            restored.stats(),
            QuarantineStats {
                total: 4,
                pending: 1,
                approved: 1,
                rejected: 1,
                escalated: 1,
            }
        );
        assert_eq!(restored.incidents_for(&hashes[0]).len(), 3);
        let files: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(files, vec![std::ffi::OsString::from("quarantine.json")]);
    }

    #[test]
    fn persistence_rejects_corruption_without_overwriting_last_good_state() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("quarantine.json");
        let mut vault = QuarantineVault::with_defaults();
        let hash = dummy_hash(1);
        assert!(vault.quarantine(hash, AnomalyScore::from_score(0.9)));
        vault.save(&path).expect("save valid state");
        let valid_bytes = fs::read(&path).unwrap();

        vault.threshold = 2.0;
        let error = vault
            .save(&path)
            .expect_err("invalid state must not be persisted");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read(&path).unwrap(), valid_bytes);

        let mut json: serde_json::Value = serde_json::from_slice(&valid_bytes).unwrap();
        let entries = json["entries"].as_object_mut().unwrap();
        let entry = entries.values_mut().next().unwrap();
        entry["hash"] = serde_json::to_value(dummy_hash(99)).unwrap();
        fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let error = QuarantineVault::load(&path).expect_err("mismatched index must fail closed");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);

        fs::write(&path, b"{truncated").unwrap();
        let error = QuarantineVault::load(&path).expect_err("malformed JSON must fail closed");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn vault_load_rejects_oversized_input_before_deserialization() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oversized-vault.json");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_QUARANTINE_VAULT_BYTES + 1).unwrap();

        let error = QuarantineVault::load(&path).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            fs::metadata(path).unwrap().len(),
            MAX_QUARANTINE_VAULT_BYTES + 1
        );
    }

    #[test]
    fn vault_load_rejects_unknown_nested_persisted_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unknown-nested.json");
        let mut vault = QuarantineVault::with_defaults();
        assert!(vault.quarantine(dummy_hash(1), AnomalyScore::from_score(0.9)));
        vault.save(&path).unwrap();
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let entry = json["entries"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        entry["anomaly_score"]["unexpected"] = serde_json::json!(true);
        fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

        let error = QuarantineVault::load(&path).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn needs_escalation_at_80_percent() {
        let mut vault = QuarantineVault::new(0.5, 10, false);
        for i in 0..8 {
            vault.quarantine(dummy_hash(i), AnomalyScore::from_score(0.6));
        }
        assert!(vault.needs_escalation());
    }

    #[test]
    fn no_escalation_when_below_threshold() {
        let mut vault = QuarantineVault::new(0.5, 10, false);
        for i in 0..5 {
            vault.quarantine(dummy_hash(i), AnomalyScore::from_score(0.6));
        }
        assert!(!vault.needs_escalation());
    }

    #[test]
    fn serde_roundtrip_quarantine_entry() {
        let entry = QuarantineEntry {
            hash: dummy_hash(1),
            anomaly_score: AnomalyScore::from_score(0.85)
                .with_dimension("coherence", 0.3)
                .with_taint(Taint::LlmHallucination {
                    detail: "made up fact".into(),
                }),
            quarantined_at: Utc::now(),
            status: QuarantineStatus::Pending,
            reviewer_notes: Some("checking".into()),
            incident_links: vec![IncidentLink {
                related_hash: dummy_hash(2),
                relation: IncidentRelation::SameSource,
                linked_at: Utc::now(),
            }],
            incident_scopes: Vec::new(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: QuarantineEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.hash, entry.hash);
        assert_eq!(back.status, QuarantineStatus::Pending);
        assert_eq!(back.incident_links.len(), 1);
    }

    #[test]
    fn serde_roundtrip_immune_response() {
        let resp = ImmuneResponse {
            entry_hash: dummy_hash(1),
            action: ResponseAction::Release,
            responded_at: Utc::now(),
            description: Some("confirmed valid".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: ImmuneResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, ResponseAction::Release);
    }
}
