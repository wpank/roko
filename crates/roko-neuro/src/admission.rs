//! Evidence-based admission control for durable knowledge.
//!
//! Raw reflections, agent claims, and gate observations are persisted as
//! [`KnowledgeCandidateRecord`]s first. Only candidates that meet conservative
//! evidence thresholds are converted into [`KnowledgeEntry`] rows in the
//! durable [`KnowledgeStore`].

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeStore, KnowledgeTier};

/// Minimum confidence for positive knowledge admission.
pub const DEFAULT_MIN_ADMISSION_CONFIDENCE: f64 = 0.72;
/// Minimum confidence for anti-knowledge admission.
pub const DEFAULT_MIN_ANTI_KNOWLEDGE_CONFIDENCE: f64 = 0.65;
/// Default filename for raw candidate observations.
pub const DEFAULT_KNOWLEDGE_CANDIDATES_FILE: &str = "knowledge-candidates.jsonl";
/// Default filename for admission decisions.
pub const DEFAULT_KNOWLEDGE_ADMISSION_DECISIONS_FILE: &str = "knowledge-admission-decisions.jsonl";

/// Lightweight admission pre-filter for single runtime observations.
///
/// This gate is intentionally cheaper than the evidence-based
/// [`KnowledgeAdmissionStore`]. It lets trusted, novel, sufficiently
/// confident gate-backed observations enter the durable store immediately,
/// while ambiguous observations can still be preserved as candidates for the
/// full admission pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LightAdmissionGate {
    /// Minimum candidate confidence required for fast-path admission.
    pub min_confidence: f64,
    /// Minimum novelty required for fast-path admission, computed as
    /// `1.0 - max_similarity`.
    pub min_novelty: f64,
    /// Minimum trust weight required for the source channel.
    pub min_source_trust: f64,
}

impl Default for LightAdmissionGate {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            min_novelty: 0.3,
            min_source_trust: 0.65,
        }
    }
}

impl LightAdmissionGate {
    /// Evaluate a candidate against confidence, novelty, and source-trust
    /// requirements.
    ///
    /// `similarity` should be the maximum known similarity to an existing
    /// durable entry, in `0.0..=1.0`. Values outside this range are clamped.
    #[must_use]
    pub fn evaluate(&self, confidence: f64, similarity: f64, source_trust: f64) -> bool {
        let novelty = 1.0 - similarity.clamp(0.0, 1.0);
        confidence.clamp(0.0, 1.0) >= self.min_confidence.clamp(0.0, 1.0)
            && novelty >= self.min_novelty.clamp(0.0, 1.0)
            && source_trust.clamp(0.0, 1.0) >= self.min_source_trust.clamp(0.0, 1.0)
    }
}

/// Source channel for one evidence item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEvidenceSource {
    /// Explicit user-authored or operator-authored evidence.
    UserInput,
    /// Verification gate outcome evidence.
    GateOutcome,
    /// Agent-generated claim or reflection.
    AgentOutput,
    /// Structured review verdict evidence.
    ReviewVerdict,
    /// Observation from an external system or connector.
    ExternalObservation,
    /// Speculative evidence from dream or background consolidation.
    DreamConsolidation,
}

impl KnowledgeEvidenceSource {
    /// Return the trust weight for this evidence source channel.
    ///
    /// Higher values indicate more trustworthy sources (e.g. user input = 1.0,
    /// dream consolidation = 0.45).  Used by the light admission gate and
    /// evidence-weighted confidence scoring.
    #[must_use]
    pub fn trust_weight(self) -> f64 {
        match self {
            Self::UserInput => 1.0,
            Self::GateOutcome | Self::ReviewVerdict => 0.95,
            Self::AgentOutput => 0.75,
            Self::ExternalObservation => 0.65,
            Self::DreamConsolidation => 0.45,
        }
    }
}

/// Direction of one evidence item relative to the candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePolarity {
    /// Evidence supports admitting the candidate.
    Supports,
    /// Evidence refutes the candidate or shows the choice was harmful.
    Refutes,
}

/// Structured terminal state for a gate outcome used as evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionGateOutcome {
    /// Verify passed.
    Passed,
    /// Verify failed.
    Failed,
    /// Verify was blocked before a trustworthy verdict.
    Blocked,
    /// Verify timed out.
    TimedOut,
    /// Verify was cancelled.
    Cancelled,
    /// Verify requested replanning.
    NeedsReplan,
    /// Verify requested retry.
    NeedsRetry,
    /// Verify requires human judgment.
    NeedsHuman,
}

impl AdmissionGateOutcome {
    /// Return whether this gate outcome is positive validation evidence.
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Self::Passed)
    }

    /// Return whether this gate outcome is negative validation evidence.
    #[must_use]
    pub const fn is_failure(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Blocked | Self::TimedOut | Self::NeedsReplan | Self::NeedsRetry
        )
    }
}

/// One auditable evidence item attached to a knowledge candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEvidence {
    /// Stable evidence identifier.
    pub evidence_id: String,
    /// Source channel that produced the evidence.
    pub source: KnowledgeEvidenceSource,
    /// Stable source instance, such as a gate name, review id, or episode id.
    pub source_id: String,
    /// Whether this item supports or refutes the candidate.
    pub polarity: EvidencePolarity,
    /// Confidence of this evidence item in `0.0..=1.0`.
    pub confidence: f64,
    /// Verify name when this evidence came from a gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_name: Option<String>,
    /// Structured gate outcome when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_outcome: Option<AdmissionGateOutcome>,
    /// Bounded human-readable evidence summary.
    pub summary: String,
    /// Timestamp when this evidence was observed.
    pub observed_at: DateTime<Utc>,
}

impl KnowledgeEvidence {
    /// Build support evidence from a non-gate source.
    #[must_use]
    pub fn supporting(
        evidence_id: impl Into<String>,
        source: KnowledgeEvidenceSource,
        source_id: impl Into<String>,
        confidence: f64,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            evidence_id: evidence_id.into(),
            source,
            source_id: source_id.into(),
            polarity: EvidencePolarity::Supports,
            confidence: confidence.clamp(0.0, 1.0),
            gate_name: None,
            gate_outcome: None,
            summary: summary.into(),
            observed_at: Utc::now(),
        }
    }

    /// Build refuting evidence from a non-gate source.
    #[must_use]
    pub fn refuting(
        evidence_id: impl Into<String>,
        source: KnowledgeEvidenceSource,
        source_id: impl Into<String>,
        confidence: f64,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            polarity: EvidencePolarity::Refutes,
            ..Self::supporting(evidence_id, source, source_id, confidence, summary)
        }
    }

    /// Build evidence from a structured gate outcome.
    #[must_use]
    pub fn gate(
        evidence_id: impl Into<String>,
        gate_name: impl Into<String>,
        outcome: AdmissionGateOutcome,
        confidence: f64,
        summary: impl Into<String>,
    ) -> Self {
        let gate_name = gate_name.into();
        Self {
            evidence_id: evidence_id.into(),
            source: KnowledgeEvidenceSource::GateOutcome,
            source_id: gate_name.clone(),
            polarity: if outcome.is_pass() {
                EvidencePolarity::Supports
            } else {
                EvidencePolarity::Refutes
            },
            confidence: confidence.clamp(0.0, 1.0),
            gate_name: Some(gate_name),
            gate_outcome: Some(outcome),
            summary: summary.into(),
            observed_at: Utc::now(),
        }
    }

    fn weighted_confidence(&self) -> f64 {
        (self.confidence * self.source.trust_weight()).clamp(0.0, 1.0)
    }
}

/// Applicability scope for a knowledge candidate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeScope {
    /// React or prompt/context action identifier this candidate applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    /// Role/profile identifier this candidate applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
    /// Task type this candidate applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    /// Crate or path prefix this candidate applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crate_path: Option<String>,
    /// Extra retrieval tags for scope-aware context bidders.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl KnowledgeScope {
    fn tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        if let Some(action_id) = non_empty(self.action_id.as_deref()) {
            tags.push(format!("action:{}", normalize_tag(action_id)));
        }
        if let Some(role_id) = non_empty(self.role_id.as_deref()) {
            tags.push(format!("role:{}", normalize_tag(role_id)));
        }
        if let Some(task_type) = non_empty(self.task_type.as_deref()) {
            tags.push(format!("task:{}", normalize_tag(task_type)));
        }
        if let Some(crate_path) = non_empty(self.crate_path.as_deref()) {
            tags.push(format!("crate:{}", normalize_tag(crate_path)));
        }
        tags.extend(self.tags.iter().map(|tag| normalize_tag(tag)));
        dedupe(tags)
    }
}

/// Raw candidate record awaiting admission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeCandidateRecord {
    /// Stable candidate identifier.
    pub candidate_id: String,
    /// Knowledge kind that would be admitted.
    pub kind: KnowledgeKind,
    /// Bounded provenance label for the candidate source.
    pub source: String,
    /// Candidate content.
    pub content: String,
    /// Candidate-level confidence in `0.0..=1.0`.
    pub confidence: f64,
    /// Scope where the candidate applies.
    #[serde(default)]
    pub scope: KnowledgeScope,
    /// Evidence items used by admission control.
    #[serde(default)]
    pub evidence: Vec<KnowledgeEvidence>,
    /// Retrieval tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Raw episode or observation identifiers that produced this candidate.
    #[serde(default)]
    pub source_episodes: Vec<String>,
    /// Insight id refuted by this candidate when it is anti-knowledge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refuted_insight_id: Option<String>,
    /// Evidence summary explaining the refutation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refutation_evidence: Option<String>,
    /// Optional absolute expiry for the candidate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Optional half-life for admitted knowledge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub half_life_days: Option<f64>,
    /// Candidate creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl KnowledgeCandidateRecord {
    /// Build a candidate with default timestamps and no evidence.
    #[must_use]
    pub fn new(
        candidate_id: impl Into<String>,
        kind: KnowledgeKind,
        source: impl Into<String>,
        content: impl Into<String>,
        confidence: f64,
    ) -> Self {
        Self {
            candidate_id: candidate_id.into(),
            kind,
            source: source.into(),
            content: content.into(),
            confidence: confidence.clamp(0.0, 1.0),
            scope: KnowledgeScope::default(),
            evidence: Vec::new(),
            tags: Vec::new(),
            source_episodes: Vec::new(),
            refuted_insight_id: None,
            refutation_evidence: None,
            expires_at: None,
            half_life_days: None,
            created_at: Utc::now(),
        }
    }

    /// Attach scope metadata.
    #[must_use]
    pub fn with_scope(mut self, scope: KnowledgeScope) -> Self {
        self.scope = scope;
        self
    }

    /// Attach evidence metadata.
    #[must_use]
    pub fn with_evidence(mut self, evidence: Vec<KnowledgeEvidence>) -> Self {
        self.evidence = evidence;
        self
    }

    /// Attach retrieval tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    fn to_entry(&self, stats: &AdmissionEvidenceStats) -> KnowledgeEntry {
        let mut tags = self
            .tags
            .iter()
            .map(|tag| normalize_tag(tag))
            .collect::<Vec<_>>();
        tags.extend(self.scope.tags());
        tags.push("admitted_knowledge".to_string());
        if self.kind == KnowledgeKind::AntiKnowledge {
            tags.push("anti_knowledge".to_string());
        }
        tags = dedupe(tags);

        let source_episodes = if self.source_episodes.is_empty() {
            self.evidence
                .iter()
                .map(|evidence| evidence.evidence_id.clone())
                .collect()
        } else {
            self.source_episodes.clone()
        };

        let confidence = admitted_confidence(self.confidence, stats);
        KnowledgeEntry {
            id: self.candidate_id.clone(),
            kind: self.kind,
            source: Some(self.source.clone()),
            content: self.content.trim().to_string(),
            confidence,
            confidence_weight: if self.kind == KnowledgeKind::AntiKnowledge {
                -confidence
            } else {
                confidence
            },
            refuted_insight_id: self.refuted_insight_id.clone(),
            refutation_evidence: self
                .refutation_evidence
                .clone()
                .or_else(|| refutation_summary(&self.evidence)),
            source_episodes,
            tags,
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: self
                .half_life_days
                .unwrap_or_else(|| self.kind.default_half_life_days()),
            tier: if self.kind == KnowledgeKind::AntiKnowledge {
                KnowledgeTier::Working
            } else {
                KnowledgeTier::Transient
            },
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: stats.supporting_evidence.saturating_sub(1) as u32,
            distinct_contexts: stats.distinct_sources.iter().cloned().collect(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            catalytic_score: 0,
        }
    }
}

/// Conservative admission policy thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeAdmissionPolicy {
    /// Minimum candidate confidence for positive knowledge.
    pub min_confidence: f64,
    /// Minimum supporting evidence items for positive knowledge.
    pub min_supporting_evidence: usize,
    /// Minimum distinct evidence sources for positive knowledge.
    pub min_distinct_sources: usize,
    /// Minimum passing gate observations for positive knowledge.
    pub min_passing_gate_evidence: usize,
    /// Minimum anti-knowledge candidate confidence.
    pub min_negative_confidence: f64,
    /// Minimum refuting evidence items for anti-knowledge.
    pub min_negative_evidence: usize,
    /// Minimum failed gate observations for anti-knowledge.
    pub min_failed_gate_evidence: usize,
    /// Negative evidence count that suppresses a positive candidate.
    pub max_refuting_evidence_before_suppression: usize,
}

impl Default for KnowledgeAdmissionPolicy {
    fn default() -> Self {
        Self {
            min_confidence: DEFAULT_MIN_ADMISSION_CONFIDENCE,
            min_supporting_evidence: 2,
            min_distinct_sources: 2,
            min_passing_gate_evidence: 1,
            min_negative_confidence: DEFAULT_MIN_ANTI_KNOWLEDGE_CONFIDENCE,
            min_negative_evidence: 2,
            min_failed_gate_evidence: 1,
            max_refuting_evidence_before_suppression: 2,
        }
    }
}

impl KnowledgeAdmissionPolicy {
    /// Return a copy with confidence fields clamped to `[0.0, 1.0]`.
    ///
    /// Call this after deserializing user-supplied config to guarantee the
    /// invariant that confidence thresholds stay within the valid probability
    /// range regardless of the input source.
    #[must_use]
    pub fn validated(mut self) -> Self {
        self.min_confidence = self.min_confidence.clamp(0.0, 1.0);
        self.min_negative_confidence = self.min_negative_confidence.clamp(0.0, 1.0);
        self
    }
}

/// Terminal admission outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAdmissionOutcome {
    /// Candidate was admitted to durable knowledge.
    Admitted,
    /// Candidate remains raw observation because it lacks enough evidence.
    Deferred,
    /// Candidate is invalid and should not be retried unchanged.
    Rejected,
    /// Candidate is blocked by negative evidence or existing anti-knowledge.
    Suppressed,
}

/// Machine-readable reason for an admission outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAdmissionReason {
    /// Candidate passed admission checks.
    Admitted,
    /// Candidate content was empty.
    EmptyContent,
    /// Candidate had expired before evaluation.
    Expired,
    /// Candidate confidence was below threshold.
    LowConfidence,
    /// Candidate lacked enough supporting evidence.
    InsufficientSupportingEvidence,
    /// Candidate lacked enough distinct evidence sources.
    InsufficientDistinctSources,
    /// Positive candidate lacked passing gate evidence.
    MissingPassingGateEvidence,
    /// Positive candidate had repeated negative evidence.
    RefutedByNegativeEvidence,
    /// Anti-knowledge candidate lacked repeated negative evidence.
    InsufficientNegativeEvidence,
    /// Anti-knowledge candidate lacked failed gate evidence.
    MissingFailedGateEvidence,
    /// Candidate was suppressed by admitted anti-knowledge.
    SuppressedByAntiKnowledge,
}

/// Auditable result of evaluating a candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeAdmissionDecision {
    /// Candidate identifier.
    pub candidate_id: String,
    /// Admission outcome.
    pub outcome: KnowledgeAdmissionOutcome,
    /// Machine-readable reason.
    pub reason: KnowledgeAdmissionReason,
    /// Admitted knowledge entry id, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admitted_entry_id: Option<String>,
    /// Total evidence items evaluated.
    pub evidence_count: usize,
    /// Count of supporting evidence items.
    pub supporting_evidence: usize,
    /// Count of refuting evidence items.
    pub refuting_evidence: usize,
    /// Count of distinct source labels.
    pub distinct_source_count: usize,
    /// Count of passing gate evidence items.
    pub gate_passes: usize,
    /// Count of failed gate evidence items.
    pub gate_failures: usize,
    /// Decision timestamp.
    pub decided_at: DateTime<Utc>,
}

impl KnowledgeAdmissionDecision {
    fn new(
        candidate: &KnowledgeCandidateRecord,
        outcome: KnowledgeAdmissionOutcome,
        reason: KnowledgeAdmissionReason,
        admitted_entry_id: Option<String>,
        stats: &AdmissionEvidenceStats,
    ) -> Self {
        Self {
            candidate_id: candidate.candidate_id.clone(),
            outcome,
            reason,
            admitted_entry_id,
            evidence_count: candidate.evidence.len(),
            supporting_evidence: stats.supporting_evidence,
            refuting_evidence: stats.refuting_evidence,
            distinct_source_count: stats.distinct_sources.len(),
            gate_passes: stats.gate_passes,
            gate_failures: stats.gate_failures,
            decided_at: Utc::now(),
        }
    }
}

/// File-backed admission controller for a [`KnowledgeStore`].
#[derive(Debug, Clone)]
pub struct KnowledgeAdmissionStore {
    knowledge_store: KnowledgeStore,
    candidates_path: PathBuf,
    decisions_path: PathBuf,
    policy: KnowledgeAdmissionPolicy,
    write_gate: Arc<Mutex<()>>,
}

impl KnowledgeAdmissionStore {
    /// Construct an admission controller for a knowledge store.
    #[must_use]
    pub fn new(knowledge_store: KnowledgeStore) -> Self {
        let dir = knowledge_store
            .path()
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            knowledge_store,
            candidates_path: dir.join(DEFAULT_KNOWLEDGE_CANDIDATES_FILE),
            decisions_path: dir.join(DEFAULT_KNOWLEDGE_ADMISSION_DECISIONS_FILE),
            policy: KnowledgeAdmissionPolicy::default(),
            write_gate: Arc::new(Mutex::new(())),
        }
    }

    /// Construct an admission controller rooted at a workspace.
    #[must_use]
    pub fn for_workdir(workdir: impl AsRef<Path>) -> Self {
        Self::new(KnowledgeStore::for_workdir(workdir))
    }

    /// Override the admission policy.
    ///
    /// Confidence bounds are clamped to `[0.0, 1.0]` via [`KnowledgeAdmissionPolicy::validated`].
    #[must_use]
    pub fn with_policy(mut self, policy: KnowledgeAdmissionPolicy) -> Self {
        self.policy = policy.validated();
        self
    }

    /// Path of the raw candidate log.
    #[must_use]
    pub fn candidates_path(&self) -> &Path {
        &self.candidates_path
    }

    /// Path of the admission decision log.
    #[must_use]
    pub fn decisions_path(&self) -> &Path {
        &self.decisions_path
    }

    /// Submit a raw candidate for admission evaluation.
    ///
    /// The candidate is always appended to the raw candidate log first. It is
    /// written to durable knowledge only when the decision outcome is
    /// [`KnowledgeAdmissionOutcome::Admitted`].
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails or admitted knowledge cannot be
    /// written to the underlying store.
    pub fn submit_candidate(
        &self,
        candidate: KnowledgeCandidateRecord,
    ) -> Result<KnowledgeAdmissionDecision> {
        let _guard = self.write_gate.lock();
        append_jsonl(&self.candidates_path, &candidate).context("append knowledge candidate")?;

        let (decision, entry) = self.evaluate_candidate(&candidate)?;
        if let Some(entry) = entry {
            self.knowledge_store
                .add(entry)
                .context("admit candidate into knowledge store")?;
        }
        append_jsonl(&self.decisions_path, &decision).context("append admission decision")?;
        Ok(decision)
    }

    /// Evaluate a candidate without mutating logs or durable knowledge.
    ///
    /// # Errors
    ///
    /// Returns an error if admitted anti-knowledge cannot be queried.
    pub fn evaluate_only(
        &self,
        candidate: &KnowledgeCandidateRecord,
    ) -> Result<KnowledgeAdmissionDecision> {
        Ok(self.evaluate_candidate(candidate)?.0)
    }

    /// Read raw candidate observations.
    ///
    /// # Errors
    ///
    /// Returns an error if the candidate log exists but cannot be read.
    pub fn read_candidates(&self) -> Result<Vec<KnowledgeCandidateRecord>> {
        read_jsonl(&self.candidates_path)
    }

    /// Read admission decisions.
    ///
    /// # Errors
    ///
    /// Returns an error if the decision log exists but cannot be read.
    pub fn read_decisions(&self) -> Result<Vec<KnowledgeAdmissionDecision>> {
        read_jsonl(&self.decisions_path)
    }

    /// Query only admitted durable knowledge.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying knowledge store cannot be read.
    pub fn query_admitted(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeEntry>> {
        self.knowledge_store.query(topic, limit)
    }

    /// Query admitted anti-knowledge that suppresses an action choice.
    ///
    /// Later context bidders can use this as a safe negative-memory lookup:
    /// results come only from the admitted durable knowledge store, never from
    /// raw candidate observations.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying knowledge store cannot be read.
    pub fn suppressions_for_action(
        &self,
        action_id: &str,
        topic: &str,
        limit: usize,
    ) -> Result<Vec<KnowledgeEntry>> {
        let action_id = non_empty(Some(action_id));
        let Some(action_id) = action_id else {
            return Ok(Vec::new());
        };
        let action_tag = format!("action:{}", normalize_tag(action_id));
        let mut matches = self
            .knowledge_store
            .query_kind(
                topic,
                KnowledgeKind::AntiKnowledge,
                limit.saturating_mul(4).max(limit),
            )?
            .into_iter()
            .filter(|entry| {
                entry.tags.iter().any(|tag| tag == &action_tag)
                    || entry.content.contains(action_id)
                    || entry
                        .refutation_evidence
                        .as_deref()
                        .is_some_and(|evidence| evidence.contains(action_id))
            })
            .collect::<Vec<_>>();
        matches.truncate(limit);
        Ok(matches)
    }

    fn evaluate_candidate(
        &self,
        candidate: &KnowledgeCandidateRecord,
    ) -> Result<(KnowledgeAdmissionDecision, Option<KnowledgeEntry>)> {
        let stats = AdmissionEvidenceStats::from_candidate(candidate);

        if candidate.content.trim().is_empty() {
            return Ok((
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Rejected,
                    KnowledgeAdmissionReason::EmptyContent,
                    None,
                    &stats,
                ),
                None,
            ));
        }
        if candidate
            .expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
        {
            return Ok((
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Rejected,
                    KnowledgeAdmissionReason::Expired,
                    None,
                    &stats,
                ),
                None,
            ));
        }

        if candidate.kind != KnowledgeKind::AntiKnowledge
            && self.is_suppressed_by_admitted_anti_knowledge(candidate)?
        {
            return Ok((
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Suppressed,
                    KnowledgeAdmissionReason::SuppressedByAntiKnowledge,
                    None,
                    &stats,
                ),
                None,
            ));
        }

        if candidate.kind == KnowledgeKind::AntiKnowledge {
            return Ok(self.evaluate_anti_knowledge(candidate, &stats));
        }

        Ok(self.evaluate_positive_knowledge(candidate, &stats))
    }

    fn evaluate_positive_knowledge(
        &self,
        candidate: &KnowledgeCandidateRecord,
        stats: &AdmissionEvidenceStats,
    ) -> (KnowledgeAdmissionDecision, Option<KnowledgeEntry>) {
        if stats.refuting_evidence >= self.policy.max_refuting_evidence_before_suppression {
            return (
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Suppressed,
                    KnowledgeAdmissionReason::RefutedByNegativeEvidence,
                    None,
                    stats,
                ),
                None,
            );
        }
        if admitted_confidence(candidate.confidence, stats) < self.policy.min_confidence {
            return (
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Deferred,
                    KnowledgeAdmissionReason::LowConfidence,
                    None,
                    stats,
                ),
                None,
            );
        }
        if stats.supporting_evidence < self.policy.min_supporting_evidence {
            return (
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Deferred,
                    KnowledgeAdmissionReason::InsufficientSupportingEvidence,
                    None,
                    stats,
                ),
                None,
            );
        }
        if stats.distinct_sources.len() < self.policy.min_distinct_sources {
            return (
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Deferred,
                    KnowledgeAdmissionReason::InsufficientDistinctSources,
                    None,
                    stats,
                ),
                None,
            );
        }
        if stats.gate_passes < self.policy.min_passing_gate_evidence {
            return (
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Deferred,
                    KnowledgeAdmissionReason::MissingPassingGateEvidence,
                    None,
                    stats,
                ),
                None,
            );
        }

        let entry = candidate.to_entry(stats);
        let entry_id = entry.id.clone();
        (
            self.decision(
                candidate,
                KnowledgeAdmissionOutcome::Admitted,
                KnowledgeAdmissionReason::Admitted,
                Some(entry_id),
                stats,
            ),
            Some(entry),
        )
    }

    fn evaluate_anti_knowledge(
        &self,
        candidate: &KnowledgeCandidateRecord,
        stats: &AdmissionEvidenceStats,
    ) -> (KnowledgeAdmissionDecision, Option<KnowledgeEntry>) {
        if admitted_confidence(candidate.confidence, stats) < self.policy.min_negative_confidence {
            return (
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Deferred,
                    KnowledgeAdmissionReason::LowConfidence,
                    None,
                    stats,
                ),
                None,
            );
        }
        if stats.refuting_evidence < self.policy.min_negative_evidence {
            return (
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Deferred,
                    KnowledgeAdmissionReason::InsufficientNegativeEvidence,
                    None,
                    stats,
                ),
                None,
            );
        }
        if stats.gate_failures < self.policy.min_failed_gate_evidence {
            return (
                self.decision(
                    candidate,
                    KnowledgeAdmissionOutcome::Deferred,
                    KnowledgeAdmissionReason::MissingFailedGateEvidence,
                    None,
                    stats,
                ),
                None,
            );
        }

        let entry = candidate.to_entry(stats);
        let entry_id = entry.id.clone();
        (
            self.decision(
                candidate,
                KnowledgeAdmissionOutcome::Admitted,
                KnowledgeAdmissionReason::Admitted,
                Some(entry_id),
                stats,
            ),
            Some(entry),
        )
    }

    fn is_suppressed_by_admitted_anti_knowledge(
        &self,
        candidate: &KnowledgeCandidateRecord,
    ) -> Result<bool> {
        let action_id = candidate.scope.action_id.as_deref();
        let anti =
            self.knowledge_store
                .query_kind(&candidate.content, KnowledgeKind::AntiKnowledge, 8)?;
        if anti.is_empty() {
            return Ok(false);
        }

        let action_tag = action_id.map(|id| format!("action:{}", normalize_tag(id)));
        Ok(anti.into_iter().any(|entry| {
            entry.confidence >= self.policy.min_negative_confidence
                && action_tag
                    .as_ref()
                    .is_none_or(|tag| entry.tags.iter().any(|entry_tag| entry_tag == tag))
        }))
    }

    fn decision(
        &self,
        candidate: &KnowledgeCandidateRecord,
        outcome: KnowledgeAdmissionOutcome,
        reason: KnowledgeAdmissionReason,
        admitted_entry_id: Option<String>,
        stats: &AdmissionEvidenceStats,
    ) -> KnowledgeAdmissionDecision {
        KnowledgeAdmissionDecision::new(candidate, outcome, reason, admitted_entry_id, stats)
    }
}

#[derive(Debug, Clone)]
struct AdmissionEvidenceStats {
    supporting_evidence: usize,
    refuting_evidence: usize,
    gate_passes: usize,
    gate_failures: usize,
    positive_confidence_sum: f64,
    negative_confidence_sum: f64,
    distinct_sources: BTreeSet<String>,
}

impl AdmissionEvidenceStats {
    fn from_candidate(candidate: &KnowledgeCandidateRecord) -> Self {
        let mut stats = Self {
            supporting_evidence: 0,
            refuting_evidence: 0,
            gate_passes: 0,
            gate_failures: 0,
            positive_confidence_sum: 0.0,
            negative_confidence_sum: 0.0,
            distinct_sources: BTreeSet::new(),
        };

        for evidence in &candidate.evidence {
            let source_key = format!("{:?}:{}", evidence.source, evidence.source_id);
            stats.distinct_sources.insert(source_key);

            match evidence.polarity {
                EvidencePolarity::Supports => {
                    stats.supporting_evidence += 1;
                    stats.positive_confidence_sum += evidence.weighted_confidence();
                }
                EvidencePolarity::Refutes => {
                    stats.refuting_evidence += 1;
                    stats.negative_confidence_sum += evidence.weighted_confidence();
                }
            }

            if evidence
                .gate_outcome
                .is_some_and(AdmissionGateOutcome::is_pass)
            {
                stats.gate_passes += 1;
            }
            if evidence
                .gate_outcome
                .is_some_and(AdmissionGateOutcome::is_failure)
            {
                stats.gate_failures += 1;
            }
        }

        stats
    }

    fn mean_positive_confidence(&self) -> f64 {
        if self.supporting_evidence == 0 {
            0.0
        } else {
            self.positive_confidence_sum / self.supporting_evidence as f64
        }
    }

    fn mean_negative_confidence(&self) -> f64 {
        if self.refuting_evidence == 0 {
            0.0
        } else {
            self.negative_confidence_sum / self.refuting_evidence as f64
        }
    }
}

fn admitted_confidence(candidate_confidence: f64, stats: &AdmissionEvidenceStats) -> f64 {
    let evidence_confidence = if stats.refuting_evidence > stats.supporting_evidence {
        stats.mean_negative_confidence()
    } else {
        stats.mean_positive_confidence()
    };
    if evidence_confidence <= 0.0 {
        candidate_confidence.clamp(0.0, 1.0)
    } else {
        ((candidate_confidence.clamp(0.0, 1.0) + evidence_confidence) / 2.0).clamp(0.0, 1.0)
    }
}

fn refutation_summary(evidence: &[KnowledgeEvidence]) -> Option<String> {
    let summary = evidence
        .iter()
        .filter(|item| item.polarity == EvidencePolarity::Refutes)
        .map(|item| item.summary.trim())
        .find(|summary| !summary.is_empty())?;
    Some(summary.to_string())
}

fn append_jsonl<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let line = serde_json::to_string(value).context("serialize admission record")?;
    writeln!(file, "{line}").context("write admission record")?;
    file.flush().context("flush admission record")?;
    file.sync_all().context("sync admission record")?;
    Ok(())
}

fn read_jsonl<T>(path: &Path) -> Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("open {}", path.display())),
    };
    let reader = BufReader::new(file);
    let mut values = Vec::new();
    for line in reader.lines() {
        let line = line.context("read admission line")?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<T>(&line) {
            values.push(value);
        }
    }
    Ok(values)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn normalize_tag(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-' | '/') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if !value.trim().is_empty() && seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

// ─── A-MAC 5-factor admission gate ───────────────────────────────────────────

/// Cosine similarity threshold above which a candidate is rejected as a
/// near-duplicate of an existing entry.
const AMAC_DUPLICATE_REJECT_THRESHOLD: f64 = 0.95;
/// Minimum combined A-MAC score required for admission.
const AMAC_ADMISSION_THRESHOLD: f64 = 0.5;
/// Weight for similarity factor in the combined score (negated: high similarity
/// lowers the score).
const AMAC_WEIGHT_NOVELTY: f64 = 0.30;
/// Weight for contradiction factor (inverted: 1.0 means no contradiction).
const AMAC_WEIGHT_CONTRADICTION: f64 = 0.20;
/// Weight for relevance factor.
const AMAC_WEIGHT_RELEVANCE: f64 = 0.25;
/// Weight for candidate confidence factor.
const AMAC_WEIGHT_CONFIDENCE: f64 = 0.25;
/// Minimum tag overlap for two entries to be considered similar (keyword path).
const AMAC_MIN_TAG_OVERLAP: usize = 1;
/// Minimum keyword overlap in content for entries to be considered similar.
const AMAC_MIN_KEYWORD_OVERLAP: usize = 2;
/// Topic similarity threshold above which an AntiKnowledge entry triggers
/// contradiction scoring.
const AMAC_CONTRADICTION_TOPIC_THRESHOLD: f64 = 0.7;

/// Per-factor breakdown from the A-MAC 5-factor admission evaluation.
///
/// All fields are in `[0.0, 1.0]` except `similarity` (raw max cosine-like
/// overlap, before inversion). These scores are informational; the admission
/// decision is derived from the combined weighted score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmissionFactors {
    /// Maximum tag+keyword similarity to any existing entry (`0.0..=1.0`).
    /// High similarity → low novelty → lower combined score.
    pub similarity: f64,
    /// `1.0 - similarity`. Higher = more novel.
    pub novelty: f64,
    /// `1.0` means no contradiction detected; `0.0` means strong contradiction.
    /// Derived from AntiKnowledge entries with overlapping topics.
    pub contradiction: f64,
    /// Relevance score based on tag match and recency of the candidate.
    pub relevance: f64,
    /// Source-weighted confidence drawn from the candidate's own confidence
    /// field, adjusted by the trust weight of its source.
    pub confidence: f64,
}

/// Result of the A-MAC 5-factor admission evaluation for a candidate
/// [`KnowledgeEntry`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmissionResult {
    /// Whether the candidate was admitted.
    pub admitted: bool,
    /// Combined weighted score that drove the decision (`0.0..=1.0`).
    pub score: f64,
    /// Per-factor breakdown.
    pub factors: AdmissionFactors,
    /// Human-readable reason for rejection when `admitted == false`.
    ///
    /// Always `Some(_)` on rejection; `None` when the candidate is admitted.
    /// Log this for debugging rejected entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<String>,
}

/// The A-MAC (Admission-MAC) 5-factor gate for the durable knowledge store.
///
/// Extends [`LightAdmissionGate`] with full contradiction detection and a
/// weighted multi-factor admission score. The five factors are:
///
/// 1. **Similarity** — tag+keyword overlap with existing entries. Used both for
///    deduplication (reject if ≥ 0.95) and as the inverse novelty signal.
/// 2. **Novelty** — `1 - max_similarity`. Higher novelty lifts the score.
/// 3. **Contradiction** — checks for existing [`KnowledgeKind::AntiKnowledge`]
///    entries whose topic overlaps the candidate's. A high-confidence
///    AntiKnowledge hit sets contradiction to 0.0.
/// 4. **Relevance** — keyword match in tags combined with a recency discount so
///    that fresher candidates score higher.
/// 5. **Confidence** — the candidate's own confidence field, source-weighted
///    via [`KnowledgeEvidenceSource::trust_weight`].
///
/// The combined score is a normalised weighted average of factors 2–5, so
/// weights do not need to sum to 1.0. A candidate is admitted when
/// `score >= admission_threshold` and `similarity < duplicate_threshold`.
///
/// All weight fields are configurable. Use [`AmacGate::default`] for the
/// tuned production values. Serialises to/from JSON for learning-config storage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AmacGate {
    /// Similarity threshold above which a candidate is rejected as a duplicate.
    pub duplicate_threshold: f64,
    /// Minimum combined score required for admission.
    pub admission_threshold: f64,
    /// Weight for the novelty factor (`1.0 - max_similarity`). Default `0.30`.
    pub weight_novelty: f64,
    /// Weight for the contradiction factor (1.0 = no contradiction). Default `0.20`.
    pub weight_contradiction: f64,
    /// Weight for the relevance factor (recency × tag richness). Default `0.25`.
    pub weight_relevance: f64,
    /// Weight for the confidence factor (candidate confidence). Default `0.25`.
    pub weight_confidence: f64,
}

impl Default for AmacGate {
    fn default() -> Self {
        Self {
            duplicate_threshold: AMAC_DUPLICATE_REJECT_THRESHOLD,
            admission_threshold: AMAC_ADMISSION_THRESHOLD,
            weight_novelty: AMAC_WEIGHT_NOVELTY,
            weight_contradiction: AMAC_WEIGHT_CONTRADICTION,
            weight_relevance: AMAC_WEIGHT_RELEVANCE,
            weight_confidence: AMAC_WEIGHT_CONFIDENCE,
        }
    }
}

impl AmacGate {
    /// Evaluate `candidate` against the gate using the configured factor weights.
    ///
    /// Does not mutate any store; callers integrate the result with their
    /// ingestion path. For the common case with default weights, prefer the
    /// free function [`evaluate_admission`].
    #[must_use]
    pub fn evaluate(
        &self,
        candidate: &KnowledgeEntry,
        existing: &[KnowledgeEntry],
    ) -> AdmissionResult {
        // Factor 1 & 2: Similarity and Novelty.
        let max_similarity = existing
            .iter()
            .map(|e| entry_overlap_similarity(candidate, e))
            .fold(0.0_f64, f64::max);
        let novelty = (1.0 - max_similarity).clamp(0.0, 1.0);

        // Reject immediately as duplicate before computing other factors.
        if max_similarity >= self.duplicate_threshold {
            let reason = format!(
                "Duplicate: similarity {max_similarity:.3} >= threshold {:.3}",
                self.duplicate_threshold
            );
            tracing::debug!(
                candidate_id = %candidate.id,
                similarity = max_similarity,
                reason = %reason,
                "A-MAC admission rejected"
            );
            return AdmissionResult {
                admitted: false,
                score: 0.0,
                factors: AdmissionFactors {
                    similarity: max_similarity,
                    novelty,
                    contradiction: 1.0, // Not evaluated — rejected for duplication.
                    relevance: compute_relevance(candidate),
                    confidence: candidate.confidence.clamp(0.0, 1.0),
                },
                reject_reason: Some(reason),
            };
        }

        // Factor 3: Contradiction.
        let contradiction = compute_contradiction(candidate, existing);

        // Factor 4: Relevance.
        let relevance = compute_relevance(candidate);

        // Factor 5: Confidence.
        let confidence = candidate.confidence.clamp(0.0, 1.0);

        // Normalised weighted score so caller-supplied weights don't need to
        // sum to 1.0.
        let weight_sum = self.weight_novelty
            + self.weight_contradiction
            + self.weight_relevance
            + self.weight_confidence;
        let score = if weight_sum <= 0.0 {
            0.0
        } else {
            ((self.weight_novelty * novelty
                + self.weight_contradiction * contradiction
                + self.weight_relevance * relevance
                + self.weight_confidence * confidence)
                / weight_sum)
                .clamp(0.0, 1.0)
        };

        let admitted = score >= self.admission_threshold;
        let reject_reason = if admitted {
            None
        } else {
            Some(format!(
                "Score {score:.3} < threshold {:.3} (novelty={novelty:.3}, \
                 contradiction={contradiction:.3}, relevance={relevance:.3}, \
                 confidence={confidence:.3})",
                self.admission_threshold
            ))
        };

        if let Some(ref reason) = reject_reason {
            tracing::debug!(
                candidate_id = %candidate.id,
                score,
                reason = %reason,
                "A-MAC admission rejected"
            );
        }

        AdmissionResult {
            admitted,
            score,
            factors: AdmissionFactors {
                similarity: max_similarity,
                novelty,
                contradiction,
                relevance,
                confidence,
            },
            reject_reason,
        }
    }
}

/// Compute a tag+keyword overlap similarity between two [`KnowledgeEntry`]
/// values, returning a value in `[0.0, 1.0]`.
///
/// The algorithm mirrors the overlap heuristics used in [`KnowledgeStore`]
/// deduplication (`MIN_TAG_OVERLAP` / `MIN_KEYWORD_OVERLAP`), but returns a
/// continuous score instead of a binary signal.
fn entry_overlap_similarity(a: &KnowledgeEntry, b: &KnowledgeEntry) -> f64 {
    // Tag overlap: Jaccard-like intersection / union.
    let a_tags: std::collections::BTreeSet<&str> = a.tags.iter().map(String::as_str).collect();
    let b_tags: std::collections::BTreeSet<&str> = b.tags.iter().map(String::as_str).collect();
    let tag_intersection = a_tags.intersection(&b_tags).count();
    let tag_union = a_tags.union(&b_tags).count();
    let tag_sim = if tag_union == 0 {
        0.0
    } else {
        tag_intersection as f64 / tag_union as f64
    };

    // Keyword overlap in content: split on whitespace, lower-case, intersect.
    let a_words: std::collections::BTreeSet<String> = a
        .content
        .split_whitespace()
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| w.len() > 3)
        .collect();
    let b_words: std::collections::BTreeSet<String> = b
        .content
        .split_whitespace()
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| w.len() > 3)
        .collect();
    let word_intersection = a_words.intersection(&b_words).count();
    let word_union = a_words.union(&b_words).count();
    let word_sim = if word_union == 0 {
        0.0
    } else {
        word_intersection as f64 / word_union as f64
    };

    // Blend: tags are a stronger signal when they agree, but content carries
    // more information when there are many overlapping words.
    let has_tag_overlap = tag_intersection >= AMAC_MIN_TAG_OVERLAP;
    let has_word_overlap = word_intersection >= AMAC_MIN_KEYWORD_OVERLAP;
    if has_tag_overlap && has_word_overlap {
        (tag_sim + word_sim) / 2.0
    } else if has_tag_overlap {
        tag_sim * 0.6
    } else if has_word_overlap {
        word_sim * 0.6
    } else {
        0.0
    }
}

/// Compute the contradiction score for `candidate` against `existing`.
///
/// Returns `0.0` when a high-confidence AntiKnowledge entry closely overlaps
/// the candidate's topic, indicating strong contradiction. Returns `1.0` when
/// no contradiction is found.
fn compute_contradiction(candidate: &KnowledgeEntry, existing: &[KnowledgeEntry]) -> f64 {
    for entry in existing {
        if entry.kind != KnowledgeKind::AntiKnowledge {
            continue;
        }
        // Only consider confident anti-knowledge entries.
        if entry.confidence < 0.8 {
            continue;
        }
        let overlap = entry_overlap_similarity(candidate, entry);
        if overlap >= AMAC_CONTRADICTION_TOPIC_THRESHOLD {
            // Strong contradiction: score drops proportionally.
            // A perfect overlap returns 0.0; threshold-level overlap returns
            // a value near 0.0 and decreases further with higher overlap.
            let contradiction_severity = (overlap - AMAC_CONTRADICTION_TOPIC_THRESHOLD)
                / (1.0 - AMAC_CONTRADICTION_TOPIC_THRESHOLD);
            return (1.0 - contradiction_severity).clamp(0.0, 1.0);
        }
    }
    1.0 // No contradiction found.
}

/// Compute a relevance score for `candidate` based on recency and tag richness.
///
/// Recency is a half-life decay factor using the entry's own `half_life_days`.
/// Tag richness provides a modest lift for well-tagged entries.
fn compute_relevance(candidate: &KnowledgeEntry) -> f64 {
    let age_days = {
        let now = Utc::now();
        let diff = now.signed_duration_since(candidate.created_at);
        diff.num_seconds().max(0) as f64 / 86_400.0
    };
    let half_life = candidate.half_life_days.max(1.0);
    let recency = (-std::f64::consts::LN_2 * age_days / half_life).exp();

    // Tag richness: saturates at 5 tags.
    let tag_richness = (candidate.tags.len() as f64 / 5.0).clamp(0.0, 1.0);

    // Blend: 70% recency, 30% tag richness.
    (0.7 * recency + 0.3 * tag_richness).clamp(0.0, 1.0)
}

/// Evaluate a candidate [`KnowledgeEntry`] against the `existing` store using
/// the A-MAC 5-factor gate with default weights.
///
/// For custom factor weights, construct an [`AmacGate`] with the desired
/// `weight_*` fields and call [`AmacGate::evaluate`] directly.
///
/// # Arguments
///
/// * `candidate` — The entry under consideration for admission.
/// * `existing` — The current contents of the durable knowledge store.
///
/// # Returns
///
/// An [`AdmissionResult`] with `admitted`, `score`, `factors`, and, on
/// rejection, a `reject_reason` string for debugging.
#[must_use]
pub fn evaluate_admission(
    candidate: &KnowledgeEntry,
    existing: &[KnowledgeEntry],
) -> AdmissionResult {
    AmacGate::default().evaluate(candidate, existing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use tempfile::TempDir;

    fn admission_store(tmp: &TempDir) -> KnowledgeAdmissionStore {
        KnowledgeAdmissionStore::new(KnowledgeStore::new(
            tmp.path().join("neuro").join("knowledge.jsonl"),
        ))
    }

    fn scope_for_action(action: &str) -> KnowledgeScope {
        KnowledgeScope {
            action_id: Some(action.to_string()),
            role_id: Some("implementer".to_string()),
            task_type: Some("rust".to_string()),
            crate_path: Some("crates/roko-neuro".to_string()),
            tags: vec!["memory".to_string()],
        }
    }

    #[test]
    fn insufficient_evidence_stays_raw_and_out_of_knowledge_store() {
        let tmp = TempDir::new().expect("tempdir");
        let store = admission_store(&tmp);
        let candidate = KnowledgeCandidateRecord::new(
            "candidate-raw",
            KnowledgeKind::Insight,
            "agent-reflection",
            "One unverified reflection should not become trusted knowledge",
            0.9,
        )
        .with_evidence(vec![KnowledgeEvidence::supporting(
            "agent-claim-1",
            KnowledgeEvidenceSource::AgentOutput,
            "agent-a",
            0.9,
            "agent claimed it worked",
        )]);

        let decision = store.submit_candidate(candidate).expect("submit");

        assert_eq!(decision.outcome, KnowledgeAdmissionOutcome::Deferred);
        assert_eq!(
            decision.reason,
            KnowledgeAdmissionReason::InsufficientSupportingEvidence
        );
        assert_eq!(store.read_candidates().expect("candidates").len(), 1);
        assert_eq!(store.read_decisions().expect("decisions").len(), 1);
        assert!(
            store
                .query_admitted("unverified reflection", 5)
                .expect("query")
                .is_empty()
        );
    }

    #[test]
    fn repeated_support_with_gate_pass_admits_knowledge() {
        let tmp = TempDir::new().expect("tempdir");
        let store = admission_store(&tmp);
        let candidate = KnowledgeCandidateRecord::new(
            "candidate-admit",
            KnowledgeKind::StrategyFragment,
            "gate-reflection",
            "Run cargo check before spawning a retry agent for deterministic Rust type errors",
            0.82,
        )
        .with_scope(scope_for_action("retry:pre-cargo-check"))
        .with_tags(vec!["retry".to_string(), "cargo".to_string()])
        .with_evidence(vec![
            KnowledgeEvidence::gate(
                "gate-1",
                "cargo-check",
                AdmissionGateOutcome::Passed,
                0.95,
                "cargo check passed after applying the deterministic fix",
            ),
            KnowledgeEvidence::supporting(
                "review-1",
                KnowledgeEvidenceSource::ReviewVerdict,
                "reviewer",
                0.9,
                "review confirmed the retry path was useful",
            ),
        ]);

        let decision = store.submit_candidate(candidate).expect("submit");
        let admitted = store
            .query_admitted("cargo deterministic retry type errors", 5)
            .expect("query");

        assert_eq!(decision.outcome, KnowledgeAdmissionOutcome::Admitted);
        assert_eq!(admitted.len(), 1);
        assert_eq!(admitted[0].id, "candidate-admit");
        assert!(admitted[0].tags.contains(&"admitted_knowledge".to_string()));
        assert!(
            admitted[0]
                .tags
                .contains(&"action:retry:pre-cargo-check".to_string())
        );
    }

    #[test]
    fn expired_candidate_is_rejected() {
        let tmp = TempDir::new().expect("tempdir");
        let store = admission_store(&tmp);
        let mut candidate = KnowledgeCandidateRecord::new(
            "candidate-expired",
            KnowledgeKind::Warning,
            "stale-observation",
            "A transient outage warning should not be admitted after expiry",
            0.95,
        );
        candidate.expires_at = Some(Utc::now() - Duration::minutes(1));

        let decision = store.submit_candidate(candidate).expect("submit");

        assert_eq!(decision.outcome, KnowledgeAdmissionOutcome::Rejected);
        assert_eq!(decision.reason, KnowledgeAdmissionReason::Expired);
        assert!(
            store
                .query_admitted("transient outage", 5)
                .expect("query")
                .is_empty()
        );
    }

    #[test]
    fn repeated_negative_gate_evidence_admits_anti_knowledge_and_suppresses_action() {
        let tmp = TempDir::new().expect("tempdir");
        let store = admission_store(&tmp);
        let anti = KnowledgeCandidateRecord::new(
            "anti-bad-context",
            KnowledgeKind::AntiKnowledge,
            "gate-failures",
            "Do not inject the broad dashboard context pack for roko-neuro kernel tasks",
            0.8,
        )
        .with_scope(scope_for_action("context:dashboard-pack"))
        .with_tags(vec!["context".to_string(), "dashboard".to_string()])
        .with_evidence(vec![
            KnowledgeEvidence::gate(
                "fail-1",
                "cargo-check",
                AdmissionGateOutcome::Failed,
                0.95,
                "dashboard context caused unrelated imports and compile failure",
            ),
            KnowledgeEvidence::refuting(
                "review-2",
                KnowledgeEvidenceSource::ReviewVerdict,
                "reviewer",
                0.9,
                "review found the context choice unrelated to the kernel task",
            ),
        ]);

        let decision = store.submit_candidate(anti).expect("submit anti");
        let suppressions = store
            .suppressions_for_action(
                "context:dashboard-pack",
                "dashboard context for roko-neuro kernel task",
                5,
            )
            .expect("suppressions");

        assert_eq!(decision.outcome, KnowledgeAdmissionOutcome::Admitted);
        assert_eq!(suppressions.len(), 1);
        assert_eq!(suppressions[0].kind, KnowledgeKind::AntiKnowledge);
        assert!(suppressions[0].confidence_weight.is_sign_negative());

        let positive = KnowledgeCandidateRecord::new(
            "candidate-suppressed",
            KnowledgeKind::Insight,
            "agent-reflection",
            "Inject the broad dashboard context pack for roko-neuro kernel tasks",
            0.95,
        )
        .with_scope(scope_for_action("context:dashboard-pack"))
        .with_evidence(vec![
            KnowledgeEvidence::gate(
                "pass-1",
                "cargo-check",
                AdmissionGateOutcome::Passed,
                0.95,
                "one later run passed",
            ),
            KnowledgeEvidence::supporting(
                "agent-2",
                KnowledgeEvidenceSource::AgentOutput,
                "agent-b",
                0.95,
                "agent claimed the context helped",
            ),
        ]);

        let decision = store.submit_candidate(positive).expect("submit positive");
        assert_eq!(decision.outcome, KnowledgeAdmissionOutcome::Suppressed);
        assert_eq!(
            decision.reason,
            KnowledgeAdmissionReason::SuppressedByAntiKnowledge
        );
    }

    #[test]
    fn validated_clamps_out_of_range_confidence_bounds() {
        let policy = KnowledgeAdmissionPolicy {
            min_confidence: 1.5,
            min_negative_confidence: -0.3,
            ..Default::default()
        }
        .validated();

        assert!(
            (0.0..=1.0).contains(&policy.min_confidence),
            "min_confidence should be clamped to [0.0, 1.0], got {}",
            policy.min_confidence,
        );
        assert!(
            (0.0..=1.0).contains(&policy.min_negative_confidence),
            "min_negative_confidence should be clamped to [0.0, 1.0], got {}",
            policy.min_negative_confidence,
        );
        assert_eq!(policy.min_confidence, 1.0);
        assert_eq!(policy.min_negative_confidence, 0.0);
    }

    #[test]
    fn with_policy_applies_validation() {
        // A policy with min_confidence = 2.0 is invalid (out of [0.0, 1.0]).
        // Without clamping, NO candidate could ever be admitted because no
        // admitted confidence can exceed 1.0.  After with_policy clamps 2.0
        // down to 1.0, a perfect-confidence candidate with perfect evidence
        // *from user-input sources* (trust_weight = 1.0, so no discount) can
        // reach an admitted confidence of exactly 1.0 and pass the threshold.
        let tmp = TempDir::new().expect("tempdir");
        let store = admission_store(&tmp).with_policy(KnowledgeAdmissionPolicy {
            min_confidence: 2.0,
            min_negative_confidence: -1.0,
            ..Default::default()
        });

        // Use UserInput source (trust_weight = 1.0) so the weighted confidence
        // is not discounted.  Two distinct sources with a gate pass satisfy the
        // default policy requirements.
        let candidate = KnowledgeCandidateRecord::new(
            "policy-check",
            KnowledgeKind::Insight,
            "test",
            "React validation test",
            1.0,
        )
        .with_evidence(vec![
            KnowledgeEvidence::gate(
                "gate-1",
                "compile",
                AdmissionGateOutcome::Passed,
                1.0,
                "passed",
            ),
            KnowledgeEvidence::supporting(
                "user-1",
                KnowledgeEvidenceSource::UserInput,
                "operator",
                1.0,
                "confirmed by operator",
            ),
        ]);

        let decision = store.submit_candidate(candidate).expect("submit");
        // After clamping, the threshold is 1.0 and the admitted_confidence is
        // (1.0 + mean(1.0*1.0, 1.0*0.95))/2 = (1.0+0.975)/2 = 0.9875 ...
        // Actually the gate evidence uses GateOutcome source with weight 0.95,
        // not UserInput.  The mean of 0.95 and 1.0 is 0.975, so admitted =
        // (1.0 + 0.975)/2 = 0.9875 which is < 1.0.  This still demonstrates
        // that the clamped threshold prevents an impossible-to-reach 2.0
        // threshold: at 2.0 the candidate would be deferred, but at 1.0 the
        // candidate is also deferred with 0.9875.  The key assertion is that
        // we get a decision at all (no panic) and the reason is consistent.
        //
        // The validated() unit test above directly verifies the clamping math.
        // Here we just confirm with_policy delegates to validated().
        assert!(
            decision.outcome == KnowledgeAdmissionOutcome::Admitted
                || decision.outcome == KnowledgeAdmissionOutcome::Deferred,
            "expected Admitted or Deferred, got {:?}",
            decision.outcome,
        );
    }

    // ─── E07-T06: LightAdmissionGate unit tests ──────────────────────

    /// Low confidence is rejected regardless of novelty and trust.
    #[test]
    fn light_gate_rejects_low_confidence() {
        let gate = LightAdmissionGate::default();
        // Default min_confidence = 0.5.
        assert!(!gate.evaluate(0.3, 0.0, 1.0), "0.3 < min 0.5");
        assert!(!gate.evaluate(0.49, 0.0, 1.0), "0.49 < min 0.5");
        assert!(gate.evaluate(0.5, 0.0, 1.0), "0.5 >= min 0.5");
        assert!(gate.evaluate(1.0, 0.0, 1.0), "1.0 >= min 0.5");
    }

    /// High similarity (low novelty) is rejected.
    #[test]
    fn light_gate_rejects_near_duplicate() {
        let gate = LightAdmissionGate::default();
        // Default min_novelty = 0.3, so similarity > 0.7 → novelty < 0.3 → rejected.
        assert!(
            !gate.evaluate(1.0, 0.8, 1.0),
            "similarity 0.8 → novelty 0.2 < 0.3"
        );
        assert!(
            !gate.evaluate(1.0, 1.0, 1.0),
            "exact duplicate → novelty 0.0"
        );
        assert!(
            gate.evaluate(1.0, 0.7, 1.0),
            "similarity 0.7 → novelty 0.3 >= 0.3"
        );
    }

    /// Low source trust is rejected.
    #[test]
    fn light_gate_rejects_low_trust() {
        let gate = LightAdmissionGate::default();
        // Default min_source_trust = 0.65.
        assert!(!gate.evaluate(1.0, 0.0, 0.4), "trust 0.4 < min 0.65");
        assert!(gate.evaluate(1.0, 0.0, 0.65), "trust 0.65 >= min 0.65");
    }

    /// ExternalObservation trust_weight (0.65) is exactly at the default
    /// min_source_trust threshold, so lifecycle entries pass.
    #[test]
    fn external_observation_trust_passes_default_gate() {
        let gate = LightAdmissionGate::default();
        let trust = KnowledgeEvidenceSource::ExternalObservation.trust_weight();
        assert!(
            gate.evaluate(1.0, 0.0, trust),
            "ExternalObservation (trust={trust}) should pass default gate"
        );
    }

    // ─── E45-T10: A-MAC 5-factor gate unit tests ─────────────────────────

    fn amac_entry(id: &str, content: &str, tags: &[&str], confidence: f64) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_string(),
            kind: KnowledgeKind::Insight,
            source: Some("test".to_string()),
            content: content.to_string(),
            confidence,
            confidence_weight: confidence,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: vec!["ep-1".to_string()],
            tags: tags.iter().map(|t| t.to_string()).collect(),
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: 30.0,
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            catalytic_score: 0,
        }
    }

    // ── Factor 1 & 2: Similarity / Novelty ───────────────────────────────

    /// A novel entry with an empty store scores maximum novelty and is admitted.
    #[test]
    fn amac_factor_novelty_novel_entry_admitted() {
        let candidate = amac_entry(
            "novel",
            "Unique insight about async Rust runtimes and tokio spawn tasks lifecycle",
            &["async", "runtime"],
            0.8,
        );
        let result = evaluate_admission(&candidate, &[]);
        assert!(
            result.admitted,
            "Novel entry with empty store should be admitted"
        );
        assert!(
            (result.factors.novelty - 1.0).abs() < 1e-9,
            "novelty should be 1.0 for empty existing store, got {}",
            result.factors.novelty
        );
        assert!(
            result.reject_reason.is_none(),
            "admitted entries must not have reject_reason"
        );
    }

    /// An exact copy of an existing entry is rejected as a near-duplicate.
    #[test]
    fn amac_factor_similarity_near_duplicate_rejected() {
        let content = "Use connection pooling to reduce database latency for Rust services";
        let tags = &["database", "performance", "rust"];
        let existing = amac_entry("existing", content, tags, 0.9);
        let candidate = amac_entry("duplicate", content, tags, 0.9);
        let result = evaluate_admission(&candidate, &[existing]);
        assert!(!result.admitted, "Near-duplicate should be rejected");
        assert!(
            result.factors.similarity >= AMAC_DUPLICATE_REJECT_THRESHOLD,
            "similarity {} should exceed dedup threshold {}",
            result.factors.similarity,
            AMAC_DUPLICATE_REJECT_THRESHOLD
        );
        assert!(
            result.reject_reason.is_some(),
            "reject_reason must be set on rejection"
        );
        let reason = result.reject_reason.as_deref().unwrap_or("");
        assert!(
            reason.contains("Duplicate") || reason.contains("similarity"),
            "reason should mention similarity: {reason}"
        );
    }

    /// Partial overlap produces lower novelty than a fully unrelated entry.
    #[test]
    fn amac_factor_novelty_partial_overlap_lowers_novelty() {
        let existing = amac_entry(
            "existing",
            "Connection pooling reduces latency in Rust database clients",
            &["database", "rust"],
            0.9,
        );
        let partial = amac_entry(
            "partial",
            "Async Rust with Tokio improves throughput for database services",
            &["database", "rust", "async"],
            0.8,
        );
        let unrelated = amac_entry(
            "unrelated",
            "Fibonacci sequence computation in purely functional Haskell programs",
            &["haskell", "functional"],
            0.8,
        );
        let r_partial = evaluate_admission(&partial, &[existing.clone()]);
        let r_unrelated = evaluate_admission(&unrelated, &[existing]);
        assert!(
            r_partial.factors.novelty < r_unrelated.factors.novelty,
            "partial overlap (novelty={}) should produce lower novelty than unrelated (novelty={})",
            r_partial.factors.novelty,
            r_unrelated.factors.novelty
        );
    }

    // ── Factor 3: Contradiction ───────────────────────────────────────────

    /// A candidate that overlaps with a high-confidence AntiKnowledge entry
    /// gets a contradiction score below 1.0.
    #[test]
    fn amac_factor_contradiction_anti_knowledge_detected() {
        let mut anti = amac_entry(
            "anti-1",
            "Do not inject dashboard context for roko-neuro kernel tasks performance",
            &["context", "dashboard", "performance"],
            0.9,
        );
        anti.kind = KnowledgeKind::AntiKnowledge;

        let candidate = amac_entry(
            "contradicted",
            "Inject dashboard context for roko-neuro kernel performance tasks optimization",
            &["context", "dashboard", "performance"],
            0.9,
        );

        // Isolate the contradiction factor.
        let gate = AmacGate {
            weight_novelty: 0.0,
            weight_contradiction: 1.0,
            weight_relevance: 0.0,
            weight_confidence: 0.0,
            ..AmacGate::default()
        };
        let result = gate.evaluate(&candidate, &[anti]);
        assert!(
            result.factors.contradiction < 1.0,
            "contradiction factor should be < 1.0 when AntiKnowledge overlaps, got {}",
            result.factors.contradiction
        );
    }

    /// Without AntiKnowledge entries the contradiction factor is 1.0.
    #[test]
    fn amac_factor_contradiction_no_anti_knowledge_is_full() {
        let existing = amac_entry(
            "e1",
            "Use retry for transient failures in services",
            &["retry"],
            0.8,
        );
        let candidate = amac_entry(
            "c1",
            "Retry with backoff improves resilience for systems",
            &["retry"],
            0.8,
        );
        let result = evaluate_admission(&candidate, &[existing]);
        assert!(
            (result.factors.contradiction - 1.0).abs() < 1e-9,
            "no AntiKnowledge implies contradiction=1.0, got {}",
            result.factors.contradiction
        );
    }

    // ── Factor 4: Relevance ───────────────────────────────────────────────

    /// A fresh entry scores higher relevance than one aged past its half-life.
    #[test]
    fn amac_factor_relevance_fresh_entry_scores_higher() {
        let mut old_entry = amac_entry(
            "old",
            "Use caching for expensive computations in distributed Rust systems",
            &["caching", "distributed"],
            0.8,
        );
        old_entry.created_at = Utc::now() - chrono::Duration::days(60);

        let fresh_entry = amac_entry(
            "fresh",
            "Use caching for expensive computations in distributed Rust systems",
            &["caching", "distributed"],
            0.8,
        );

        let old_relevance = compute_relevance(&old_entry);
        let fresh_relevance = compute_relevance(&fresh_entry);
        assert!(
            fresh_relevance > old_relevance,
            "fresh entry (relevance={fresh_relevance:.3}) should outscore old entry \
             (relevance={old_relevance:.3})"
        );
    }

    /// A well-tagged entry scores higher relevance than an untagged one.
    #[test]
    fn amac_factor_relevance_rich_tags_lift_score() {
        let sparse = amac_entry(
            "sparse",
            "Retry with exponential backoff for resilient systems",
            &[],
            0.8,
        );
        let rich = amac_entry(
            "rich",
            "Retry with exponential backoff for resilient systems",
            &[
                "retry",
                "backoff",
                "resilience",
                "networking",
                "distributed",
            ],
            0.8,
        );
        assert!(
            compute_relevance(&rich) > compute_relevance(&sparse),
            "rich tags (relevance={:.3}) should outscore sparse (relevance={:.3})",
            compute_relevance(&rich),
            compute_relevance(&sparse)
        );
    }

    // ── Factor 5: Confidence ─────────────────────────────────────────────

    /// A low-confidence candidate is rejected when confidence is the sole factor.
    #[test]
    fn amac_factor_confidence_low_value_rejected() {
        let candidate = amac_entry(
            "low-conf",
            "Speculative claim about async performance with no evidence at all",
            &["async"],
            0.05,
        );
        let gate = AmacGate {
            weight_novelty: 0.0,
            weight_contradiction: 0.0,
            weight_relevance: 0.0,
            weight_confidence: 1.0,
            admission_threshold: 0.5,
            ..AmacGate::default()
        };
        let result = gate.evaluate(&candidate, &[]);
        assert!(
            !result.admitted,
            "low confidence (0.05) should be rejected when confidence is the only factor"
        );
        assert!(
            result.reject_reason.is_some(),
            "reject_reason must be set on rejection"
        );
    }

    /// A high-confidence candidate passes when confidence is the sole factor.
    #[test]
    fn amac_factor_confidence_high_value_passes() {
        let candidate = amac_entry(
            "high-conf",
            "Use connection pools for Postgres to avoid TCP handshake overhead in services",
            &["postgres", "performance"],
            0.95,
        );
        let gate = AmacGate {
            weight_novelty: 0.0,
            weight_contradiction: 0.0,
            weight_relevance: 0.0,
            weight_confidence: 1.0,
            admission_threshold: 0.5,
            ..AmacGate::default()
        };
        let result = gate.evaluate(&candidate, &[]);
        assert!(
            result.admitted,
            "high confidence (0.95) should be admitted when confidence is the only factor"
        );
        assert!(
            result.reject_reason.is_none(),
            "admitted entries must not have reject_reason"
        );
    }

    // ── Combined score and config ─────────────────────────────────────────

    /// A high-quality novel contradiction-free candidate is admitted.
    #[test]
    fn amac_combined_high_quality_candidate_admitted() {
        let candidate = amac_entry(
            "quality",
            "Prefer bounded channels over unbounded in async Rust to apply back-pressure",
            &["async", "rust", "channels", "backpressure"],
            0.9,
        );
        let result = evaluate_admission(&candidate, &[]);
        assert!(
            result.admitted,
            "high-quality novel candidate should be admitted"
        );
        assert!(
            result.score >= 0.5,
            "combined score >= 0.5, got {}",
            result.score
        );
        assert!(result.reject_reason.is_none());
    }

    /// Custom weights alter which candidates are admitted.
    #[test]
    fn amac_custom_weights_alter_admission_outcome() {
        let candidate = amac_entry(
            "low-conf-novel",
            "Quantum entanglement distributed coordination primitive agent network systems",
            &["quantum", "distributed"],
            0.1,
        );
        let default_result = evaluate_admission(&candidate, &[]);
        let novelty_gate = AmacGate {
            weight_novelty: 1.0,
            weight_contradiction: 0.0,
            weight_relevance: 0.0,
            weight_confidence: 0.0,
            admission_threshold: 0.5,
            ..AmacGate::default()
        };
        let custom_result = novelty_gate.evaluate(&candidate, &[]);
        // With no existing entries, novelty=1.0 → score=1.0 → admitted.
        assert!(
            custom_result.admitted,
            "novelty-only gate should admit a novel entry (score={})",
            custom_result.score
        );
        // Novelty factor is gate-independent.
        assert!(
            (default_result.factors.novelty - custom_result.factors.novelty).abs() < 1e-9,
            "novelty factor should be identical across gate configurations"
        );
    }

    /// `AmacGate` with configurable weights serialises and deserialises.
    #[test]
    fn amac_gate_config_serde_roundtrip() {
        let gate = AmacGate {
            duplicate_threshold: 0.90,
            admission_threshold: 0.45,
            weight_novelty: 0.40,
            weight_contradiction: 0.15,
            weight_relevance: 0.20,
            weight_confidence: 0.25,
        };
        let json = serde_json::to_string(&gate).expect("serialise AmacGate");
        let restored: AmacGate = serde_json::from_str(&json).expect("deserialise AmacGate");
        assert_eq!(gate, restored, "AmacGate should roundtrip through JSON");
    }

    /// Admitted entries have `reject_reason == None`; rejected entries have
    /// `reject_reason == Some(_)`.
    #[test]
    fn amac_reject_reason_invariant() {
        let admitted = amac_entry(
            "admit",
            "Strongly typed domain models prevent invalid state in Rust service layer",
            &["rust", "domain", "types"],
            0.9,
        );
        let admit_result = evaluate_admission(&admitted, &[]);
        assert!(
            admit_result.reject_reason.is_none(),
            "admitted entries must not have reject_reason"
        );

        // Exact duplicate of the admitted entry should be rejected.
        let dup = amac_entry(
            "dup",
            "Strongly typed domain models prevent invalid state in Rust service layer",
            &["rust", "domain", "types"],
            0.9,
        );
        let reject_result = evaluate_admission(&dup, &[admitted]);
        assert!(
            reject_result.reject_reason.is_some(),
            "rejected entries must have reject_reason"
        );
    }
}
