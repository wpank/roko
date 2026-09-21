//! Public types, structs, enums, and constants for the knowledge store.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{KnowledgeEntry, KnowledgeKind};

// ── Constants ────────────────────────────────────────────────────────

/// Default garbage-collection threshold for knowledge entries.
pub const DEFAULT_GC_MIN_CONFIDENCE: f64 = 0.05;
/// Minimum relevance score an entry must exceed to be returned.
pub const QUERY_SCORE_FLOOR: f64 = 0.0;
/// Minimum retained confidence for AntiKnowledge entries.
pub(crate) const ANTI_KNOWLEDGE_CONFIDENCE_FLOOR: f64 = 0.3;
/// Multiplier applied when a knowledge entry has multiple independent sources.
pub(crate) const CONFIRMATION_BOOST: f64 = 1.5;
/// Additive weight for the balance/freshness contribution to the query score.
///
/// Kept small (0.15) so it acts as a tie-breaker and lift for reinforced entries
/// without overriding keyword relevance, which can range up to ~3.0 for a strong match.
pub(crate) const BALANCE_FRESHNESS_WEIGHT: f64 = 0.15;

/// Death threshold: when recency factor falls below 1% of initial weight,
/// the entry is considered "dead" and eligible for pruning.
///
/// Per spec (agent-chain-new/04-knowledge-layer.md): entries below 1% of
/// initial weight enter the Death stage and should be pruned.
pub const DEATH_THRESHOLD: f64 = 0.01;

/// Confirmation decay adjustment factor.
///
/// Per spec: `weight(b) = initialWeight * 0.5^(age/halfLife) * (1 + confirmations * 0.1)`
/// Each confirmation extends effective lifetime by 10%.
pub(crate) const CONFIRMATION_DECAY_FACTOR: f64 = 0.1;

/// Base confidence for resurrected entries (re-confirmed dead/frozen entries).
///
/// When an entry that was previously pruned or frozen is re-confirmed by a
/// new episode, it is "resurrected" with this starter confidence and reset
/// to Transient tier for re-validation.
pub const RESURRECTION_CONFIDENCE: f64 = 0.6;
/// Minimum number of shared tags for two entries to be considered similar.
pub(crate) const MIN_TAG_OVERLAP: usize = 1;
/// Minimum number of shared content keywords for two entries to be
/// considered similar (applied when tag overlap meets the threshold).
pub(crate) const MIN_KEYWORD_OVERLAP: usize = 2;
#[cfg(feature = "hdc")]
pub(crate) const HDC_SIMILARITY_BASELINE: f64 = 0.5;
/// Minimum raw HDC similarity treated as a meaningful query signal.
///
/// Independent 10,240-bit vectors center tightly around `0.5`; requiring a
/// modest margin prevents random Hamming noise from making unrelated entries
/// eligible for freshness and balance boosts.
#[cfg(feature = "hdc")]
pub(crate) const HDC_QUERY_RELEVANCE_THRESHOLD: f64 = 0.525;

/// HDC similarity threshold at which an AntiKnowledge match logs a warning.
#[cfg(feature = "hdc")]
pub(crate) const ANTI_KNOWLEDGE_WARN_THRESHOLD: f64 = 0.5;
/// HDC similarity threshold at which a new entry's confidence is discounted.
#[cfg(feature = "hdc")]
pub(crate) const ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD: f64 = 0.7;
/// HDC similarity threshold at which a new entry is rejected entirely.
#[cfg(feature = "hdc")]
pub(crate) const ANTI_KNOWLEDGE_REJECT_THRESHOLD: f64 = 0.9;
/// Confidence multiplier applied when a new entry conflicts with AntiKnowledge
/// at the discount threshold.
#[cfg(feature = "hdc")]
pub(crate) const ANTI_KNOWLEDGE_DISCOUNT_FACTOR: f64 = 0.5;

pub(crate) const HDC_VECTOR_BYTES: usize = 1280;

// ── Public types ─────────────────────────────────────────────────────

/// A record emitted when a newly ingested knowledge entry overlaps with
/// an existing entry, indicating that an insight has been independently
/// confirmed by a separate episode.
///
/// These records are consumed by the C-Factor metrics
/// (`knowledge_integration_rate` and `convergence_velocity`) in
/// `roko-learn`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeConfirmationRecord {
    /// Timestamp of the confirmation event.
    pub created_at: DateTime<Utc>,
    /// Combined source episodes from the existing entry and the new entry.
    pub source_episodes: Vec<String>,
    /// ID of the existing entry that was confirmed.
    pub confirmed_entry_id: String,
    /// ID of the new entry that confirmed the existing one.
    pub confirming_entry_id: String,
}

/// A record of a conflict detected between a newly ingested entry and an
/// existing AntiKnowledge entry. Emitted during `ingest()` for observability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AntiKnowledgeConflict {
    /// ID of the new entry that conflicts with AntiKnowledge.
    pub entry_id: String,
    /// ID of the existing AntiKnowledge entry.
    pub anti_knowledge_id: String,
    /// HDC similarity score between the two entries.
    pub similarity: f64,
    /// Action taken: "warned", "discounted", or "rejected".
    pub action: String,
}

/// Result of checking a learned rule's falsifiable predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FalsifierOutcome {
    /// The predicate survived another observation but is not yet immunized.
    Survived,
    /// The predicate survived enough observations to earn durable standing.
    Immunized,
    /// An observation violated the predicate and reduced its credibility.
    Discredited,
}

/// Aggregate statistics for a durable knowledge store snapshot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnowledgeStats {
    /// Total number of retained knowledge entries.
    pub total_entries: usize,
    /// Number of entries per semantic kind.
    pub kind_counts: BTreeMap<String, usize>,
    /// Number of entries per validation tier.
    pub tier_counts: BTreeMap<String, usize>,
    /// Number of entries per source label.
    pub source_counts: BTreeMap<String, usize>,
    /// Number of AntiKnowledge entries.
    pub anti_knowledge_count: usize,
    /// Mean confidence across all entries.
    pub average_confidence: Option<f64>,
    /// Oldest entry in the store, if any.
    pub oldest_entry: Option<KnowledgeEntry>,
    /// Newest entry in the store, if any.
    pub newest_entry: Option<KnowledgeEntry>,
}

/// Score breakdown for one knowledge query result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnowledgeQueryBreakdown {
    /// Keyword overlap between the query and the entry tags/content.
    pub keyword_score: f64,
    /// Confidence after anti-knowledge floors, confirmation boosts, and
    /// emotional consolidation adjustments.
    pub effective_confidence: f64,
    /// Exponential freshness multiplier derived from effective half-life.
    pub recency_factor: f64,
    /// Retrieval multiplier derived from emotional congruence and intensity.
    pub emotional_boost: f64,
    /// Additive boost from the entry's reinforcement balance and freshness decay.
    ///
    /// Derived as `BALANCE_FRESHNESS_WEIGHT * freshness(now).clamp(0, 1)`.
    /// Zero for zero-balance entries; up to `BALANCE_FRESHNESS_WEIGHT` for fully
    /// reinforced fresh entries. Acts as a tie-breaker rather than a dominant factor.
    pub balance_freshness_boost: f64,
    /// Optional HDC similarity contribution when the `hdc` feature is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hdc_similarity: Option<f64>,
}

/// One scored hit returned from the durable knowledge query path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnowledgeQueryHit {
    /// The matched entry.
    pub entry: KnowledgeEntry,
    /// Total score used for ranking.
    pub total_score: f64,
    /// Individual scoring components that contributed to `total_score`.
    pub breakdown: KnowledgeQueryBreakdown,
}

/// Context assembly weights for scoring knowledge entries during retrieval (P1-59).
///
/// Per spec (agent-chain-new/07-context-assembly.md):
/// - HDC similarity: 40%
/// - Pheromone/keyword weight: 30%
/// - Predictive Foraging utility: 20%
/// - Freshness/recency: 10%
///
/// Also supports:
/// - Cross-domain diversity bonus (P1-60): 10-20% bonus for entries from different domains
/// - Three-tier injection (P1-61): Warning/Insight get priority, CausalLink/AntiKnowledge on-demand
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextAssemblyWeights {
    /// Weight for HDC similarity score [0..1]. Default 0.40.
    pub hdc_similarity: f64,
    /// Weight for keyword/pheromone relevance [0..1]. Default 0.30.
    pub keyword_relevance: f64,
    /// Weight for predictive foraging utility [0..1]. Default 0.20.
    pub pf_utility: f64,
    /// Weight for freshness/recency [0..1]. Default 0.10.
    pub freshness: f64,
    /// Cross-domain diversity bonus [0..1]. Entries whose tags don't overlap
    /// with the majority get this fractional boost. Default 0.15.
    pub cross_domain_bonus: f64,
    /// Whether to apply three-tier injection ordering.
    pub tier_injection: bool,
}

impl Default for ContextAssemblyWeights {
    fn default() -> Self {
        Self {
            hdc_similarity: 0.40,
            keyword_relevance: 0.30,
            pf_utility: 0.20,
            freshness: 0.10,
            cross_domain_bonus: 0.15,
            tier_injection: true,
        }
    }
}

impl ContextAssemblyWeights {
    /// Compute the weighted composite score for a knowledge entry.
    ///
    /// `keyword`: keyword/pheromone relevance score
    /// `hdc`: HDC similarity score (0.0 if not available)
    /// `recency`: freshness/recency factor
    /// `utility`: predictive foraging utility (confidence_weight as proxy)
    /// `is_cross_domain`: whether this entry is from a different domain than the query
    pub fn composite(
        &self,
        keyword: f64,
        hdc: f64,
        recency: f64,
        utility: f64,
        is_cross_domain: bool,
    ) -> f64 {
        let base = self.hdc_similarity * hdc
            + self.keyword_relevance * keyword
            + self.pf_utility * utility
            + self.freshness * recency;

        if is_cross_domain {
            base * (1.0 + self.cross_domain_bonus)
        } else {
            base
        }
    }

    /// Sort knowledge entries by three-tier injection priority (P1-61).
    ///
    /// Tier 1 (compact inject): Warning, Insight -- always included first
    /// Tier 2 (relevant include): Heuristic, StrategyFragment -- included if relevant
    /// Tier 3 (on-demand): CausalLink, AntiKnowledge -- included only when specifically needed
    pub fn injection_tier(kind: KnowledgeKind) -> u8 {
        match kind {
            KnowledgeKind::Warning | KnowledgeKind::Insight => 1, // Always inject
            KnowledgeKind::Heuristic | KnowledgeKind::StrategyFragment => 2, // If relevant
            KnowledgeKind::CausalLink | KnowledgeKind::AntiKnowledge => 3, // On demand
        }
    }
}

/// Current canonical knowledge backup format version.
pub const KNOWLEDGE_BACKUP_VERSION: u32 = 2;

/// Versioned header written as the first line of a backup JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupHeader {
    /// Backup format version. Currently `2`.
    pub version: u32,
    /// When the backup was created.
    pub created_at: DateTime<Utc>,
    /// Number of entries in the backup.
    pub entry_count: usize,
    /// Path of the source knowledge store that was exported.
    pub source_path: String,
    /// SHA-256 Merkle root computed over canonical entry JSON, sorted by ID.
    /// Hex-encoded. Version 1 used an ID-only root and is accepted only through
    /// the explicit legacy import path.
    #[serde(default)]
    pub merkle_root: String,
}

/// Bundle returned by [`super::KnowledgeStore::export_with_verification`].
///
/// Contains the exported entries (confidence-sorted, secrets-filtered) and
/// the Merkle root computed over complete canonical entry JSON.
#[derive(Debug, Clone)]
pub struct ExportBundle {
    /// Exported entries, sorted by confidence descending.
    pub entries: Vec<KnowledgeEntry>,
    /// SHA-256 Merkle root over complete canonical entry JSON, hex-encoded.
    pub merkle_root: String,
}

/// Secret patterns used to exclude sensitive entries from exports.
///
/// Entries whose content or tags match any of these patterns are silently
/// skipped when `ExportFilter::filter_secrets` is `true`.
pub(crate) const SECRET_PATTERNS: &[&str] = &[
    "api_key",
    "api-key",
    "apikey",
    "secret",
    "password",
    "passwd",
    "token",
    "bearer",
    "private_key",
    "private-key",
    "privatekey",
    "access_key",
    "access-key",
    "auth_token",
    "auth-token",
    "credential",
];

/// Filter criteria for [`super::KnowledgeStore::export`].
#[derive(Debug, Clone)]
pub struct ExportFilter {
    /// Only export entries of these kinds. `None` means all kinds.
    pub kinds: Option<Vec<KnowledgeKind>>,
    /// Minimum confidence threshold.
    pub min_confidence: Option<f64>,
    /// Only export entries with at least one of these tags.
    pub tags: Option<Vec<String>>,
    /// Only export entries created after this timestamp.
    pub since: Option<DateTime<Utc>>,
    /// Maximum number of entries to export after confidence sorting.
    pub max_entries: Option<usize>,
    /// When `true`, skip entries whose tags or content match known secret patterns.
    /// Defaults to `true`. Callers must opt out explicitly for a local-only,
    /// trusted export.
    pub filter_secrets: bool,
}

impl Default for ExportFilter {
    fn default() -> Self {
        Self {
            kinds: None,
            min_confidence: None,
            tags: None,
            since: None,
            max_entries: None,
            filter_secrets: true,
        }
    }
}

impl ExportFilter {
    /// Check if an entry matches this filter.
    pub(crate) fn matches(&self, entry: &KnowledgeEntry) -> bool {
        if let Some(kinds) = &self.kinds
            && !kinds.contains(&entry.kind)
        {
            return false;
        }
        if let Some(min) = self.min_confidence
            && entry.confidence < min
        {
            return false;
        }
        if let Some(required_tags) = &self.tags
            && !required_tags.iter().any(|t| entry.tags.contains(t))
        {
            return false;
        }
        if let Some(since) = self.since
            && entry.created_at < since
        {
            return false;
        }
        if self.filter_secrets && entry_contains_secret(entry) {
            return false;
        }
        true
    }
}

/// Returns `true` if the entry's tags or content match a known secret pattern.
pub(crate) fn entry_contains_secret(entry: &KnowledgeEntry) -> bool {
    let content_lower = entry.content.to_lowercase();
    for pattern in SECRET_PATTERNS {
        // Check tags first (cheap, no allocation).
        if entry
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(pattern))
        {
            return true;
        }
        // Check content text.
        if content_lower.contains(pattern) {
            return true;
        }
    }
    false
}

/// Options for [`super::KnowledgeStore::import`].
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Confidence multiplier applied to each imported entry (default 0.80).
    pub confidence_discount: f64,
    /// Whether to reset all imported entries to `KnowledgeTier::Transient`.
    pub reset_tier: bool,
    /// Label recorded in the `source` field of each imported entry.
    pub source_label: String,
    /// Optional kind filter applied after integrity validation.
    pub kinds: Option<Vec<KnowledgeKind>>,
    /// Optional minimum source confidence applied after integrity validation.
    pub min_confidence: Option<f64>,
    /// Explicitly allow strict migration of a legacy raw or version-1 backup.
    /// Canonical imports reject legacy input by default.
    pub allow_legacy: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            confidence_discount: 0.80,
            reset_tier: true,
            source_label: "restore".to_owned(),
            kinds: None,
            min_confidence: None,
            allow_legacy: false,
        }
    }
}

/// Accurate outcome of a canonical knowledge import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportResult {
    /// Number of valid entries in the source after integrity validation.
    pub source_entries: usize,
    /// Number of entries atomically added to the destination store.
    pub imported: usize,
    /// Number skipped by exact-ID or semantic deduplication.
    pub skipped_dedup: usize,
    /// Number skipped because they contradict high-confidence AntiKnowledge.
    pub skipped_contradiction: usize,
    /// Number skipped by explicit kind or confidence filters.
    pub skipped_filter: usize,
    /// Always zero on success; malformed input fails before any write.
    pub malformed: usize,
    /// Whether the explicit legacy migration path was used.
    pub legacy_input: bool,
}

/// One similarity-ranked hit returned from the durable fingerprint query path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnowledgeSimilarityHit {
    /// The matched entry.
    pub entry: KnowledgeEntry,
    /// Raw Hamming similarity against the supplied fingerprint.
    pub similarity: f32,
}
