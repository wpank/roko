//! Append-only JSONL knowledge store.
//!
//! Knowledge entries live at `.roko/neuro/knowledge.jsonl` by default.
//! Writes append one JSON record per line, while maintenance operations
//! (`decay` and `gc`) rewrite the file atomically through a temporary
//! sibling.

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::admission::evaluate_admission;
#[cfg(feature = "hdc")]
use crate::hdc::{KnowledgeHdcEncoder, ResonanceDetector, ResonancePair, RoleFillerEncoder};
use crate::temporal::{AllenRelation, KnowledgeEpoch, TemporalIndex, TemporalInterval};
use crate::{
    Falsifier, KnowledgeEntry, KnowledgeKind, KnowledgeTier, NeuroStore, SourceChannel,
    apply_source_security_labels,
};

/// Default garbage-collection threshold for knowledge entries.
pub const DEFAULT_GC_MIN_CONFIDENCE: f64 = 0.05;
/// Minimum relevance score an entry must exceed to be returned.
pub const QUERY_SCORE_FLOOR: f64 = 0.0;
/// Minimum retained confidence for AntiKnowledge entries.
const ANTI_KNOWLEDGE_CONFIDENCE_FLOOR: f64 = 0.3;
/// Multiplier applied when a knowledge entry has multiple independent sources.
const CONFIRMATION_BOOST: f64 = 1.5;
/// Additive weight for the balance/freshness contribution to the query score.
///
/// Kept small (0.15) so it acts as a tie-breaker and lift for reinforced entries
/// without overriding keyword relevance, which can range up to ~3.0 for a strong match.
const BALANCE_FRESHNESS_WEIGHT: f64 = 0.15;

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
const CONFIRMATION_DECAY_FACTOR: f64 = 0.1;

/// Base confidence for resurrected entries (re-confirmed dead/frozen entries).
///
/// When an entry that was previously pruned or frozen is re-confirmed by a
/// new episode, it is "resurrected" with this starter confidence and reset
/// to Transient tier for re-validation.
pub const RESURRECTION_CONFIDENCE: f64 = 0.6;
/// Minimum number of shared tags for two entries to be considered similar.
const MIN_TAG_OVERLAP: usize = 1;
/// Minimum number of shared content keywords for two entries to be
/// considered similar (applied when tag overlap meets the threshold).
const MIN_KEYWORD_OVERLAP: usize = 2;
#[cfg(feature = "hdc")]
const HDC_SIMILARITY_BASELINE: f64 = 0.5;
/// Minimum raw HDC similarity treated as a meaningful query signal.
///
/// Independent 10,240-bit vectors center tightly around `0.5`; requiring a
/// modest margin prevents random Hamming noise from making unrelated entries
/// eligible for freshness and balance boosts.
#[cfg(feature = "hdc")]
const HDC_QUERY_RELEVANCE_THRESHOLD: f64 = 0.525;

/// HDC similarity threshold at which an AntiKnowledge match logs a warning.
#[cfg(feature = "hdc")]
const ANTI_KNOWLEDGE_WARN_THRESHOLD: f64 = 0.5;
/// HDC similarity threshold at which a new entry's confidence is discounted.
#[cfg(feature = "hdc")]
const ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD: f64 = 0.7;
/// HDC similarity threshold at which a new entry is rejected entirely.
#[cfg(feature = "hdc")]
const ANTI_KNOWLEDGE_REJECT_THRESHOLD: f64 = 0.9;
/// Confidence multiplier applied when a new entry conflicts with AntiKnowledge
/// at the discount threshold.
#[cfg(feature = "hdc")]
const ANTI_KNOWLEDGE_DISCOUNT_FACTOR: f64 = 0.5;

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

const HDC_VECTOR_BYTES: usize = 1280;

#[cfg(feature = "hdc")]
use roko_primitives::hdc::{HdcVector, text_fingerprint};

/// Persistent knowledge store backed by an append-only JSONL file.
///
/// The store is cheap to clone: it holds the path and a process-local
/// write gate so that concurrent maintenance operations never interleave
/// file rewrites.
///
/// When new entries overlap with existing entries (by tag and keyword
/// similarity), the store emits [`KnowledgeConfirmationRecord`]s to a
/// sibling JSONL file. These records feed the C-Factor metrics
/// `knowledge_integration_rate` and `convergence_velocity`.
#[derive(Debug, Clone)]
pub struct KnowledgeStore {
    path: PathBuf,
    confirmations_path: PathBuf,
    write_gate: Arc<Mutex<()>>,
    temporal_index: Option<Arc<Mutex<TemporalIndex>>>,
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
    /// Tier 1 (compact inject): Warning, Insight — always included first
    /// Tier 2 (relevant include): Heuristic, StrategyFragment — included if relevant
    /// Tier 3 (on-demand): CausalLink, AntiKnowledge — included only when specifically needed
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

/// Bundle returned by [`KnowledgeStore::export_with_verification`].
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
const SECRET_PATTERNS: &[&str] = &[
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

/// Filter criteria for [`KnowledgeStore::export`].
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
    fn matches(&self, entry: &KnowledgeEntry) -> bool {
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
fn entry_contains_secret(entry: &KnowledgeEntry) -> bool {
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

/// Compute a Merkle root over a sorted list of entry IDs.
///
/// The IDs are first sorted lexicographically so that the root is
/// deterministic regardless of export order. Each leaf is the SHA-256 hash
/// of the UTF-8 entry ID. The tree is built bottom-up by hashing pairs of
/// adjacent nodes; an odd node is promoted unchanged (a "lone sibling" carry).
/// Returns the hex-encoded root hash, or an empty string for an empty set.
pub(crate) fn compute_merkle_root(ids: &[String]) -> String {
    if ids.is_empty() {
        return String::new();
    }
    // Sort IDs for deterministic ordering.
    let mut sorted = ids.to_vec();
    sorted.sort();

    // Build leaves: SHA-256 of each entry ID.
    let mut layer: Vec<[u8; 32]> = sorted
        .iter()
        .map(|id| {
            let mut h = Sha256::new();
            h.update(id.as_bytes());
            h.finalize().into()
        })
        .collect();

    // Reduce pairs until one node remains.
    while layer.len() > 1 {
        let mut next: Vec<[u8; 32]> = Vec::with_capacity(layer.len().div_ceil(2));
        let mut i = 0;
        while i < layer.len() {
            if i + 1 < layer.len() {
                let mut h = Sha256::new();
                h.update(layer[i]);
                h.update(layer[i + 1]);
                next.push(h.finalize().into());
                i += 2;
            } else {
                // Odd node: promote as-is.
                next.push(layer[i]);
                i += 1;
            }
        }
        layer = next;
    }

    // Encode the single root as lowercase hex.
    layer[0].iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Compute the version-2 Merkle root over complete canonical entry JSON.
///
/// Entries are sorted by ID and then by their serialized bytes. Both content
/// and identity changes therefore invalidate the root.
fn compute_entry_merkle_root(entries: &[KnowledgeEntry]) -> Result<String> {
    if entries.is_empty() {
        return Ok(String::new());
    }

    let mut canonical = entries
        .iter()
        .map(|entry| {
            serde_json::to_vec(entry)
                .map(|bytes| (entry.id.clone(), bytes))
                .context("serialize knowledge entry for Merkle root")
        })
        .collect::<Result<Vec<_>>>()?;
    canonical.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let mut layer = canonical
        .into_iter()
        .map(|(_, bytes)| {
            let mut hash = Sha256::new();
            hash.update(bytes);
            <[u8; 32]>::from(hash.finalize())
        })
        .collect::<Vec<_>>();

    while layer.len() > 1 {
        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        let mut index = 0;
        while index < layer.len() {
            if let Some(right) = layer.get(index + 1) {
                let mut hash = Sha256::new();
                hash.update(layer[index]);
                hash.update(right);
                next.push(hash.finalize().into());
            } else {
                next.push(layer[index]);
            }
            index += 2;
        }
        layer = next;
    }

    Ok(layer[0].iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
        output
    }))
}

/// Options for [`KnowledgeStore::import`].
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

fn read_import_entries(input: &Path, allow_legacy: bool) -> Result<(Vec<KnowledgeEntry>, bool)> {
    let file =
        File::open(input).with_context(|| format!("open import file at {}", input.display()))?;
    let lines = BufReader::new(file)
        .lines()
        .enumerate()
        .map(|(index, line)| {
            line.with_context(|| format!("read import line {} from {}", index + 1, input.display()))
                .map(|line| (index + 1, line))
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(!lines.is_empty(), "import file is empty");

    if let Ok(header) = serde_json::from_str::<BackupHeader>(&lines[0].1) {
        let entries = parse_strict_import_lines(&lines[1..])?;
        ensure!(
            header.entry_count == entries.len(),
            "backup entry_count mismatch: header={}, actual={}",
            header.entry_count,
            entries.len()
        );

        match header.version {
            KNOWLEDGE_BACKUP_VERSION => {
                let actual_root = compute_entry_merkle_root(&entries)?;
                ensure!(
                    !header.merkle_root.is_empty() || entries.is_empty(),
                    "canonical backup is missing its Merkle root"
                );
                ensure!(
                    header.merkle_root == actual_root,
                    "backup Merkle verification failed"
                );
                return Ok((entries, false));
            }
            1 if allow_legacy => {
                ensure!(
                    !header.merkle_root.is_empty() || entries.is_empty(),
                    "legacy version-1 backup has no integrity root"
                );
                let ids = entries
                    .iter()
                    .map(|entry| entry.id.clone())
                    .collect::<Vec<_>>();
                ensure!(
                    header.merkle_root == compute_merkle_root(&ids),
                    "legacy backup ID Merkle verification failed"
                );
                return Ok((entries, true));
            }
            1 => {
                anyhow::bail!("legacy version-1 backup requires explicit allow_legacy migration");
            }
            version => {
                anyhow::bail!(
                    "unsupported backup version {version} (this build supports version {KNOWLEDGE_BACKUP_VERSION})"
                );
            }
        }
    }

    ensure!(
        allow_legacy,
        "import is not a canonical versioned backup; use explicit allow_legacy migration for a trusted raw JSONL store"
    );
    Ok((parse_strict_import_lines(&lines)?, true))
}

fn parse_strict_import_lines(lines: &[(usize, String)]) -> Result<Vec<KnowledgeEntry>> {
    let mut entries = Vec::with_capacity(lines.len());
    for (line_number, line) in lines {
        ensure!(
            !line.trim().is_empty(),
            "malformed_entries=1: blank import record at line {line_number}"
        );
        let entry = serde_json::from_str::<KnowledgeEntry>(line).with_context(|| {
            format!("malformed_entries=1: invalid knowledge entry at line {line_number}")
        })?;
        ensure!(
            !entry.id.trim().is_empty(),
            "malformed_entries=1: empty knowledge entry ID at line {line_number}"
        );
        entries.push(entry);
    }
    Ok(entries)
}

fn resolved_transfer_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut cursor = absolute.as_path();
    let mut suffix = Vec::new();
    while !cursor.exists() {
        let name = cursor
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("resolve transfer path {}", path.display()))?;
        suffix.push(name.to_os_string());
        cursor = cursor
            .parent()
            .ok_or_else(|| anyhow::anyhow!("resolve parent of {}", path.display()))?;
    }
    let mut resolved = fs::canonicalize(cursor)
        .with_context(|| format!("resolve existing path ancestor {}", cursor.display()))?;
    for component in suffix.into_iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn import_entry_is_contradicted(
    candidate: &KnowledgeEntry,
    existing: &[KnowledgeEntry],
    admitted: &[KnowledgeEntry],
) -> bool {
    if candidate.kind == KnowledgeKind::AntiKnowledge {
        return false;
    }

    existing.iter().chain(admitted).any(|entry| {
        entry.kind == KnowledgeKind::AntiKnowledge
            && entry.confidence > 0.8
            && import_contradiction_similarity(entry, candidate) > 0.9
    })
}

fn import_entry_is_semantic_duplicate(
    candidate: &KnowledgeEntry,
    existing: &[KnowledgeEntry],
    admitted: &[KnowledgeEntry],
) -> bool {
    existing.iter().chain(admitted).any(|entry| {
        // AntiKnowledge is a refutation, not a duplicate of the ordinary
        // knowledge it contradicts. Never discard it merely because its
        // content is highly similar to the claim being refuted.
        (entry.kind == KnowledgeKind::AntiKnowledge)
            == (candidate.kind == KnowledgeKind::AntiKnowledge)
            && import_semantic_similarity(entry, candidate) > 0.95
    })
}

#[cfg(feature = "hdc")]
fn import_semantic_similarity(left: &KnowledgeEntry, right: &KnowledgeEntry) -> f64 {
    let encoder = KnowledgeHdcEncoder;
    let structured = encoder
        .encode_entry(left)
        .similarity(&encoder.encode_entry(right));
    let content =
        fingerprint_content(&left.content).similarity(&fingerprint_content(&right.content));
    f64::from(structured.max(content))
}

#[cfg(not(feature = "hdc"))]
fn import_semantic_similarity(left: &KnowledgeEntry, right: &KnowledgeEntry) -> f64 {
    entry_similarity(left, right)
}

fn import_contradiction_similarity(left: &KnowledgeEntry, right: &KnowledgeEntry) -> f64 {
    let lexical = entry_similarity(left, right);
    #[cfg(feature = "hdc")]
    {
        lexical.max(import_semantic_similarity(left, right))
    }
    #[cfg(not(feature = "hdc"))]
    {
        lexical
    }
}

/// One similarity-ranked hit returned from the durable fingerprint query path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnowledgeSimilarityHit {
    /// The matched entry.
    pub entry: KnowledgeEntry,
    /// Raw Hamming similarity against the supplied fingerprint.
    pub similarity: f32,
}

impl KnowledgeStore {
    /// Construct a store pointed at an explicit JSONL path.
    ///
    /// Confirmation records are written to a sibling file named
    /// `knowledge-confirmations.jsonl` in the same directory.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let confirmations_path = path
            .parent()
            .map(|parent| parent.join("knowledge-confirmations.jsonl"))
            .unwrap_or_else(|| PathBuf::from("knowledge-confirmations.jsonl"));
        Self {
            path,
            confirmations_path,
            write_gate: Arc::new(Mutex::new(())),
            temporal_index: None,
        }
    }

    /// Construct a store from a `.roko/` root.
    ///
    /// The resulting file is `.roko/neuro/knowledge.jsonl`.
    #[must_use]
    pub fn for_roko_dir(roko_dir: impl AsRef<Path>) -> Self {
        Self::new(roko_dir.as_ref().join("neuro").join("knowledge.jsonl"))
    }

    /// Construct a store from a workspace root.
    ///
    /// The resulting file is `<workdir>/.roko/neuro/knowledge.jsonl`.
    #[must_use]
    pub fn for_workdir(workdir: impl AsRef<Path>) -> Self {
        Self::new(
            workdir
                .as_ref()
                .join(".roko")
                .join("neuro")
                .join("knowledge.jsonl"),
        )
    }

    /// Construct a store from an existing Roko layout.
    #[must_use]
    pub fn for_layout(layout: &roko_fs::RokoLayout) -> Self {
        Self::for_roko_dir(layout.root())
    }

    /// Path of the backing JSONL file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Path of the confirmation records JSONL file.
    #[must_use]
    pub fn confirmations_path(&self) -> &Path {
        &self.confirmations_path
    }

    /// Enable the optional temporal topology and index all durable entries.
    ///
    /// Existing entries receive open-ended intervals starting at their
    /// creation timestamps. New entries and removals are kept synchronized by
    /// the store's write paths.
    pub fn enable_temporal_index(&mut self) -> Result<()> {
        let mut index = TemporalIndex::new();
        for entry in self.read_all()? {
            index.add_entry(
                entry.id,
                TemporalInterval::new(entry.created_at.timestamp_millis(), i64::MAX),
            );
        }
        self.temporal_index = Some(Arc::new(Mutex::new(index)));
        Ok(())
    }

    /// Register an epoch in the optional temporal topology.
    ///
    /// Returns `false` when temporal indexing has not been enabled.
    pub fn add_temporal_epoch(&self, epoch: KnowledgeEpoch) -> bool {
        let Some(index) = &self.temporal_index else {
            return false;
        };
        index.lock().add_epoch(epoch);
        true
    }

    /// Query entries created during a registered temporal epoch.
    pub fn query_temporal(&self, epoch_seq: u64) -> Result<Vec<KnowledgeEntry>> {
        let Some(index) = &self.temporal_index else {
            return Ok(Vec::new());
        };
        let ids = index.lock().entries_in_epoch(epoch_seq);
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = ids.into_iter().collect::<HashSet<_>>();
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|entry| ids.contains(&entry.id))
            .collect())
    }

    /// Compute the Allen relation between two indexed knowledge entries.
    pub fn query_temporal_relation(
        &self,
        source_id: &str,
        target_id: &str,
    ) -> Result<Option<AllenRelation>> {
        Ok(self
            .temporal_index
            .as_ref()
            .and_then(|index| index.lock().relation(source_id, target_id)))
    }

    /// Append a knowledge entry to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created, the entry
    /// cannot be serialized, or the write fails.
    pub fn add(&self, entry: KnowledgeEntry) -> Result<()> {
        self.ingest(vec![entry])
    }

    /// Persist a deterministic compression of at least three admitted entries.
    ///
    /// Dream consolidation is a derived rewrite, not a new untrusted claim, so
    /// it bypasses novelty rejection while retaining strict provenance and ID
    /// deduplication checks.
    pub fn add_consolidated(&self, mut entry: KnowledgeEntry) -> Result<bool> {
        ensure!(
            entry.source_episodes.len() >= 3,
            "consolidated knowledge requires at least three source episodes"
        );
        entry.source = Some("dream-consolidation".to_string());
        let entry = normalize_entry_for_ingest(entry);
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        if entries.iter().any(|existing| existing.id == entry.id) {
            return Ok(false);
        }
        entries.push(entry.clone());
        self.rewrite_all(&entries)?;
        self.register_temporal_entries(std::slice::from_ref(&entry));
        Ok(true)
    }

    /// Persist an opt-in, discounted cross-domain derivative.
    #[cfg(feature = "hdc")]
    pub fn add_cross_domain_transfer(&self, entry: KnowledgeEntry) -> Result<bool> {
        ensure!(
            entry.source_model.as_deref() == Some("cross_domain_transfer"),
            "cross-domain derivatives require the cross_domain_transfer source model"
        );
        ensure!(
            entry.tags.iter().any(|tag| tag.starts_with("domain:")),
            "cross-domain derivatives require a target domain tag"
        );
        let entry = normalize_entry_for_ingest(entry);
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        if entries.iter().any(|existing| existing.id == entry.id) {
            return Ok(false);
        }
        entries.push(entry.clone());
        self.rewrite_all(&entries)?;
        self.register_temporal_entries(std::slice::from_ref(&entry));
        Ok(true)
    }

    /// Record a failed gate turn as AntiKnowledge.
    ///
    /// Similar failures are searched for in the existing store first. If a
    /// matching anti-pattern is found, its confidence is reinforced instead of
    /// creating a duplicate record.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn record_anti_pattern_from_failure(
        &self,
        task_id: &str,
        task_prompt: &str,
        gate_name: &str,
        gate_error: &str,
        agent_output: Option<&str>,
    ) -> Result<()> {
        let candidate = extract_anti_pattern_from_failure(
            task_id,
            task_prompt,
            gate_name,
            gate_error,
            agent_output,
        );

        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;

        if let Some(index) = entries.iter().position(|entry| entry.id == candidate.id) {
            let updated = {
                let existing = &mut entries[index];
                reinforce_anti_pattern(existing, &candidate);
                existing.clone()
            };
            self.rewrite_all(&entries)?;
            tracing::debug!(
                knowledge_id = %updated.id,
                "reinforced existing AntiKnowledge from gate failure"
            );
            return Ok(());
        }

        if let Some(index) = find_similar_anti_pattern_index(&entries, &candidate) {
            let updated = {
                let existing = &mut entries[index];
                reinforce_anti_pattern(existing, &candidate);
                existing.clone()
            };
            self.rewrite_all(&entries)?;
            tracing::debug!(
                knowledge_id = %updated.id,
                "reinforced similar AntiKnowledge from gate failure"
            );
            return Ok(());
        }

        entries.push(candidate);
        self.rewrite_all(&entries)?;
        Ok(())
    }

    /// NEURO-07: Append entries with source-channel confidence discounting.
    ///
    /// Each entry's confidence is multiplied by the channel's trust discount
    /// before being ingested into the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created, an entry
    /// cannot be serialized, or the write fails.
    pub fn ingest_with_source(
        &self,
        mut entries: Vec<KnowledgeEntry>,
        channel: SourceChannel,
    ) -> Result<()> {
        crate::apply_source_discount(&mut entries, channel);
        for entry in &mut entries {
            if entry.source.is_none() {
                entry.source = Some(channel.as_str().to_string());
            }
        }
        apply_source_security_labels(&mut entries, channel);
        self.ingest(entries)
    }

    /// Append a batch of knowledge entries to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created, an entry
    /// cannot be serialized, or the write fails.
    pub fn ingest(&self, entries: Vec<KnowledgeEntry>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).context("create knowledge directory")?;
        }

        let _guard = self.write_gate.lock();
        let mut existing = self.read_all().unwrap_or_default();
        let entries = coalesce_incoming_security_labels(prepare_entries_for_ingest(entries));
        let security_upgraded = join_replayed_security_labels(&mut existing, &entries);
        if security_upgraded {
            self.rewrite_all(&existing)?;
        }
        let entries = dedupe_entries_for_ingest(entries, &existing);
        if entries.is_empty() {
            return Ok(());
        }

        // A-MAC 5-factor admission gate: filter entries that fail the novelty,
        // contradiction, relevance, and confidence gate before persisting.
        // AntiKnowledge entries always bypass the gate so the contradiction
        // check for future positive entries works correctly.
        let entries: Vec<KnowledgeEntry> = entries
            .into_iter()
            .filter(|entry| {
                if entry.kind == KnowledgeKind::AntiKnowledge {
                    return true;
                }
                let result = evaluate_admission(entry, &existing);
                if !result.admitted {
                    tracing::debug!(
                        entry_id = %entry.id,
                        score = result.score,
                        reason = ?result.reject_reason,
                        "A-MAC gate rejected entry during ingest"
                    );
                }
                result.admitted
            })
            .collect();
        if entries.is_empty() {
            return Ok(());
        }

        // NEURO-04: Check new non-AntiKnowledge entries against existing
        // AntiKnowledge entries using HDC similarity. Entries that are
        // near-duplicates of refuted knowledge are rejected; moderate
        // conflicts have their confidence discounted.
        #[cfg(feature = "hdc")]
        let entries = check_against_anti_knowledge(entries, &existing);
        if entries.is_empty() {
            return Ok(());
        }

        let mut has_antiknowledge = false;
        for entry in &entries {
            if entry.kind == KnowledgeKind::AntiKnowledge
                && entry
                    .refuted_insight_id
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|refuted_id| !refuted_id.is_empty())
            {
                has_antiknowledge = true;
                break;
            }
        }

        if has_antiknowledge {
            let mut current = existing;
            current.extend(entries.iter().cloned());

            for anti in &entries {
                if anti.kind != KnowledgeKind::AntiKnowledge {
                    continue;
                }

                let Some(refuted_id) = anti.refuted_insight_id.as_deref().map(str::trim) else {
                    continue;
                };
                if refuted_id.is_empty() {
                    continue;
                }

                if let Some(original) = current.iter_mut().find(|entry| entry.id == refuted_id) {
                    original.confidence *= 0.5;
                }
            }

            self.rewrite_all(&current)?;
            self.register_temporal_entries(&entries);
            return Ok(());
        }

        // Detect confirmations by comparing new entries against existing ones.
        let confirmations = detect_confirmations(&existing, &entries);

        // Apply tier promotions for confirmed entries.
        if !confirmations.is_empty() {
            let mut updated_existing = existing;
            for confirmation in &confirmations {
                if let Some(entry) = updated_existing
                    .iter_mut()
                    .find(|e| e.id == confirmation.confirmed_entry_id)
                {
                    entry.confirmation_count = entry.confirmation_count.saturating_add(1);

                    // Add distinct context from the confirming entry's source episodes.
                    if let Some(confirming) = entries
                        .iter()
                        .find(|e| e.id == confirmation.confirming_entry_id)
                    {
                        for ep in &confirming.source_episodes {
                            if !entry.distinct_contexts.contains(ep) {
                                entry.distinct_contexts.push(ep.clone());
                            }
                        }
                    }

                    // Auto-promote based on thresholds.
                    match entry.tier {
                        KnowledgeTier::Transient if entry.confirmation_count >= 2 => {
                            entry.tier = KnowledgeTier::Working;
                        }
                        KnowledgeTier::Working if entry.distinct_contexts.len() >= 3 => {
                            entry.tier = KnowledgeTier::Consolidated;
                        }
                        _ => {}
                    }
                }
            }
            self.rewrite_all(&updated_existing)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("open knowledge store at {}", self.path.display()))?;
        for entry in &entries {
            let mut line = serde_json::to_string(&entry).context("serialize knowledge entry")?;
            line.push('\n');
            file.write_all(line.as_bytes())
                .context("append knowledge entry")?;
        }
        file.flush().context("flush knowledge entry")?;
        file.sync_all().context("sync knowledge entry")?;

        // Append confirmation records to the sibling JSONL file.
        if !confirmations.is_empty() {
            self.append_confirmations(&confirmations)?;
        }

        self.register_temporal_entries(&entries);

        Ok(())
    }

    /// Query the store for entries relevant to `topic`.
    ///
    /// Relevance is scored by keyword overlap in tags/content, multiplied
    /// by confidence, recency, and a 1.5× confirmation boost for entries
    /// backed by multiple independent episodes. When the `hdc` feature is
    /// enabled, HDC similarity is added as an extra signal. Only entries with
    /// `total_score > QUERY_SCORE_FLOOR` are returned.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn query(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeEntry>> {
        Ok(self
            .query_hits(topic, limit)?
            .into_iter()
            .map(|hit| hit.entry)
            .collect())
    }

    /// Query the store by a serialized 10,240-bit fingerprint.
    ///
    /// Entries without a valid stored fingerprint are skipped. Results are
    /// ranked by raw Hamming similarity and then by effective confidence.
    ///
    /// # Errors
    ///
    /// Returns an error if `fingerprint` is not 1280 bytes long or the
    /// backing file cannot be read.
    pub fn query_similar(
        &self,
        fingerprint: &[u8],
        limit: usize,
    ) -> Result<Vec<KnowledgeSimilarityHit>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        ensure!(
            fingerprint.len() == HDC_VECTOR_BYTES,
            "knowledge fingerprints must be {HDC_VECTOR_BYTES} bytes, got {}",
            fingerprint.len()
        );

        let entries = self.read_all()?;
        let mut scored = entries
            .into_iter()
            .filter_map(|entry| {
                let similarity = similarity_against_entry(fingerprint, &entry)?;
                Some(KnowledgeSimilarityHit { entry, similarity })
            })
            .collect::<Vec<_>>();

        scored.sort_by(compare_similarity_hits);
        scored.truncate(limit);
        Ok(scored)
    }

    /// Query the store for scored hits relevant to `topic`.
    ///
    /// The current contract is:
    ///
    /// `relevance_score = keyword_score * effective_confidence * recency_factor * emotional_boost + hdc_similarity`
    ///
    /// Entries must clear [`QUERY_SCORE_FLOOR`] with relevance alone. Their
    /// final score then adds the balance/freshness boost as a tie-breaker.
    /// `hdc_similarity` is zero when the `hdc` feature is disabled, the entry
    /// has no valid stored HDC vector, or raw similarity is indistinguishable
    /// from the random-vector baseline.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn query_hits(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeQueryHit>> {
        self.query_hits_filtered(topic, limit, |_| true)
    }

    /// Query the persisted HDC vectors directly, streaming the JSONL store.
    #[cfg(feature = "hdc")]
    pub fn query_hdc(
        &self,
        query_vector: &HdcVector,
        top_k: usize,
    ) -> Result<Vec<KnowledgeQueryHit>> {
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err).context("open knowledge store for HDC query"),
        };

        let mut hits = Vec::new();
        for (line_idx, line) in BufReader::new(file).lines().enumerate() {
            let line = line.with_context(|| format!("read HDC query line {}", line_idx + 1))?;
            let Ok(entry) = serde_json::from_str::<KnowledgeEntry>(&line) else {
                continue;
            };
            if entry.frozen {
                continue;
            }
            let Some(bytes) = entry.hdc_vector.as_deref() else {
                continue;
            };
            let Ok(bytes) = <[u8; HDC_VECTOR_BYTES]>::try_from(bytes) else {
                continue;
            };
            let similarity =
                f64::from(query_vector.hamming_similarity(&HdcVector::from_bytes(&bytes)));
            if similarity <= QUERY_SCORE_FLOOR {
                continue;
            }
            hits.push(KnowledgeQueryHit {
                entry: normalize_entry_security(entry),
                total_score: similarity,
                breakdown: KnowledgeQueryBreakdown {
                    keyword_score: 0.0,
                    effective_confidence: 0.0,
                    recency_factor: 0.0,
                    emotional_boost: 0.0,
                    balance_freshness_boost: 0.0,
                    hdc_similarity: Some(similarity),
                },
            });
        }
        hits.sort_by(|left, right| {
            right
                .total_score
                .partial_cmp(&left.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.entry.id.cmp(&right.entry.id))
        });
        hits.truncate(top_k);
        Ok(hits)
    }

    /// Encode structured role/filler bindings and perform direct HDC lookup.
    #[cfg(feature = "hdc")]
    pub fn query_by_role_filler(
        &self,
        roles_and_fillers: &[(String, String)],
        top_k: usize,
    ) -> Result<Vec<KnowledgeQueryHit>> {
        self.query_hdc(
            &RoleFillerEncoder::encode_structured(roles_and_fillers),
            top_k,
        )
    }

    /// Detect structurally resonant entries that belong to different domains.
    #[cfg(feature = "hdc")]
    pub fn find_resonances(&self, min_similarity: f64) -> Result<Vec<ResonancePair>> {
        let entries = self
            .read_all()?
            .into_iter()
            .filter(|entry| {
                !entry.frozen
                    && entry
                        .hdc_vector
                        .as_ref()
                        .is_some_and(|vector| vector.len() == HDC_VECTOR_BYTES)
            })
            .collect::<Vec<_>>();
        Ok(ResonanceDetector::new(min_similarity.clamp(0.0, 1.0), 20).detect_resonances(&entries))
    }

    /// Query the store for entries of a specific knowledge kind relevant to
    /// `topic`.
    ///
    /// This is a thin extension over [`KnowledgeStore::query`] used by prompt
    /// assembly to recall only the highest-tier distilled guidance (for
    /// example, StrategyFragment entries) without pulling lower-tier noise into the
    /// prompt.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn query_kind(
        &self,
        topic: &str,
        kind: KnowledgeKind,
        limit: usize,
    ) -> Result<Vec<KnowledgeEntry>> {
        Ok(self
            .query_kind_hits(topic, kind, limit)?
            .into_iter()
            .map(|hit| hit.entry)
            .collect())
    }

    /// Query the store for scored hits of a specific kind relevant to `topic`.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn query_kind_hits(
        &self,
        topic: &str,
        kind: KnowledgeKind,
        limit: usize,
    ) -> Result<Vec<KnowledgeQueryHit>> {
        self.query_hits_filtered(topic, limit, |entry| entry.kind == kind)
    }

    /// Filter all entries by their validation tier.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn by_tier(&self, tier: KnowledgeTier) -> Result<Vec<KnowledgeEntry>> {
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|entry| entry.tier == tier)
            .collect())
    }

    /// Return the maximum lightweight similarity between `candidate` and
    /// existing durable entries.
    ///
    /// The score is deterministic and file-local: tag Jaccard overlap plus
    /// content keyword Jaccard overlap. It is intended for admission
    /// pre-filtering, not semantic ranking.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing store cannot be read.
    pub fn max_similarity(&self, candidate: &KnowledgeEntry) -> Result<f64> {
        let entries = self.read_all()?;
        Ok(entries
            .iter()
            .filter(|entry| entry.id != candidate.id)
            .map(|entry| entry_similarity(entry, candidate))
            .fold(0.0, f64::max)
            .clamp(0.0, 1.0))
    }

    /// Return the most similar existing entry at or above `minimum`.
    pub fn find_similar_entry(
        &self,
        candidate: &KnowledgeEntry,
        minimum: f64,
    ) -> Result<Option<KnowledgeEntry>> {
        let minimum = minimum.clamp(0.0, 1.0);
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|entry| {
                entry.id != candidate.id && entry.kind == candidate.kind && !entry.frozen
            })
            .map(|entry| {
                let similarity = entry_similarity(&entry, candidate);
                (entry, similarity)
            })
            .filter(|(_, similarity)| *similarity > minimum)
            .max_by(|(left_entry, left), (right_entry, right)| {
                left.partial_cmp(right)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right_entry.id.cmp(&left_entry.id))
            })
            .map(|(entry, _)| entry))
    }

    fn query_hits_filtered<F>(
        &self,
        topic: &str,
        limit: usize,
        mut include: F,
    ) -> Result<Vec<KnowledgeQueryHit>>
    where
        F: FnMut(&KnowledgeEntry) -> bool,
    {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let now = Utc::now();
        let entries = self.read_all()?;
        let topic_terms = tokenize(topic);
        let topic_norm = normalize(topic);

        let mut scored: Vec<KnowledgeQueryHit> = entries
            .into_iter()
            .filter_map(|entry| {
                // NEURO-11: Frozen entries are excluded from hot queries.
                if entry.frozen {
                    return None;
                }
                if !include(&entry) {
                    return None;
                }
                score_entry_for_query(entry, &topic_terms, &topic_norm, topic, now)
            })
            .collect();

        scored.sort_by(compare_hit_scores);
        scored.truncate(limit);
        Ok(scored)
    }

    /// Compute aggregate statistics over the current knowledge corpus.
    ///
    /// The snapshot is derived from the current on-disk entries and
    /// ignores malformed JSONL lines, matching the store's tolerant read
    /// behavior.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn stats(&self) -> Result<KnowledgeStats> {
        let entries = self.read_all()?;
        let total_entries = entries.len();
        let mut kind_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut tier_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut source_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut anti_knowledge_count = 0usize;
        let mut confidence_sum = 0.0;
        let mut oldest_entry: Option<&KnowledgeEntry> = None;
        let mut newest_entry: Option<&KnowledgeEntry> = None;

        for entry in &entries {
            *kind_counts
                .entry(knowledge_kind_label(entry.kind).to_owned())
                .or_insert(0) += 1;

            let tier_label = match entry.tier {
                KnowledgeTier::Transient => "transient",
                KnowledgeTier::Working => "working",
                KnowledgeTier::Consolidated => "consolidated",
                KnowledgeTier::Persistent => "persistent",
            };
            *tier_counts.entry(tier_label.to_owned()).or_insert(0) += 1;

            if let Some(source) = entry.source.as_deref() {
                let trimmed = source.trim();
                if !trimmed.is_empty() {
                    *source_counts.entry(trimmed.to_owned()).or_insert(0) += 1;
                }
            }

            if entry.kind == KnowledgeKind::AntiKnowledge {
                anti_knowledge_count += 1;
            }

            confidence_sum += entry.confidence;

            if oldest_entry
                .map(|current| entry.created_at < current.created_at)
                .unwrap_or(true)
            {
                oldest_entry = Some(entry);
            }
            if newest_entry
                .map(|current| entry.created_at > current.created_at)
                .unwrap_or(true)
            {
                newest_entry = Some(entry);
            }
        }

        let average_confidence = if total_entries > 0 {
            Some(confidence_sum / total_entries as f64)
        } else {
            None
        };

        Ok(KnowledgeStats {
            total_entries,
            kind_counts,
            tier_counts,
            source_counts,
            anti_knowledge_count,
            average_confidence,
            oldest_entry: oldest_entry.cloned(),
            newest_entry: newest_entry.cloned(),
        })
    }

    /// Export the knowledge store to a JSONL file with versioned backup header.
    ///
    /// Entries are filtered by the provided [`ExportFilter`] (including optional
    /// secret filtering), then sorted by confidence descending so bounded exports
    /// retain the most valuable knowledge. A SHA-256 Merkle root over complete
    /// canonical entry JSON is included in the header.
    ///
    /// Returns the number of entries exported.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or the output cannot be
    /// written.
    pub fn export(&self, output: &Path, filter: &ExportFilter) -> Result<usize> {
        let source = resolved_transfer_path(&self.path)?;
        let destination = resolved_transfer_path(output)?;
        ensure!(
            source != destination,
            "knowledge export destination resolves to the live store at {}",
            self.path.display()
        );

        let entries = self.read_all_strict()?;
        let mut filtered: Vec<_> = entries.into_iter().filter(|e| filter.matches(e)).collect();

        // Sort highest confidence first so bounded exports retain the best entries.
        filtered.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        if let Some(max_entries) = filter.max_entries {
            filtered.truncate(max_entries);
        }

        let count = filtered.len();

        ensure!(
            filtered.iter().all(|entry| !entry.id.trim().is_empty()),
            "cannot export a knowledge entry with an empty ID"
        );
        let unique_ids = filtered
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<HashSet<_>>();
        ensure!(
            unique_ids.len() == filtered.len(),
            "cannot export duplicate knowledge entry IDs"
        );

        // Version 2 commits the complete serialized entries, not only IDs.
        let merkle_root = compute_entry_merkle_root(&filtered)?;

        let header = BackupHeader {
            version: KNOWLEDGE_BACKUP_VERSION,
            created_at: Utc::now(),
            entry_count: count,
            source_path: self.path.display().to_string(),
            merkle_root,
        };

        // Serialize the complete artifact before opening any staging file. The
        // shared atomic writer then fsyncs a unique same-directory temporary
        // file, renames it over the target, and fsyncs the parent directory.
        let mut bytes = Vec::new();
        serde_json::to_writer(&mut bytes, &header).context("serialize backup header")?;
        bytes.push(b'\n');
        for entry in &filtered {
            serde_json::to_writer(&mut bytes, entry).context("serialize knowledge entry")?;
            bytes.push(b'\n');
        }
        roko_fs::atomic_write_bytes(output, &bytes)
            .with_context(|| format!("atomically write export file at {}", output.display()))?;

        Ok(count)
    }

    /// Export all entries with integrity verification and return an [`ExportBundle`].
    ///
    /// This is a higher-level alternative to [`export`] for callers that need the
    /// exported data in memory (e.g. replication, sync) rather than written to a file.
    ///
    /// Applies a default [`ExportFilter`] with `filter_secrets = true`, sorts entries
    /// by confidence descending, and computes a SHA-256 Merkle root over complete
    /// canonical entry JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read.
    pub fn export_with_verification(&self) -> Result<ExportBundle> {
        let filter = ExportFilter {
            filter_secrets: true,
            ..Default::default()
        };
        let entries = self.read_all_strict()?;
        let mut filtered: Vec<KnowledgeEntry> =
            entries.into_iter().filter(|e| filter.matches(e)).collect();

        // Sort highest confidence first.
        filtered.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        let merkle_root = compute_entry_merkle_root(&filtered)?;
        Ok(ExportBundle {
            entries: filtered,
            merkle_root,
        })
    }

    /// Import knowledge entries from a versioned JSONL backup file.
    ///
    /// The complete input is parsed and its count and Merkle root are verified
    /// before the destination is read or written. Restored entries are reset to
    /// [`KnowledgeTier::Transient`] and their confidence is multiplied by the
    /// configured discount factor. Deduplication and contradiction checks are
    /// completed before one atomic destination rewrite.
    ///
    /// Returns exact admitted and skipped counts.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, aliases the live store, has
    /// an unsupported version, or entries cannot be ingested.
    pub fn import(&self, input: &Path, options: &ImportOptions) -> Result<ImportResult> {
        let source = resolved_transfer_path(input)?;
        let destination = resolved_transfer_path(&self.path)?;
        ensure!(
            source != destination,
            "knowledge import source resolves to the live store at {}",
            self.path.display()
        );
        ensure!(
            options.confidence_discount.is_finite()
                && (0.0..=1.0).contains(&options.confidence_discount),
            "confidence discount must be between 0.0 and 1.0"
        );
        if let Some(minimum) = options.min_confidence {
            ensure!(
                minimum.is_finite() && (0.0..=1.0).contains(&minimum),
                "minimum confidence must be between 0.0 and 1.0"
            );
        }

        let (entries, legacy_input) = read_import_entries(input, options.allow_legacy)?;
        let source_entries = entries.len();
        let mut skipped_filter = 0;
        let mut transformed = Vec::with_capacity(entries.len());
        let mut source_contradictions = Vec::new();
        for mut entry in entries {
            let kind_matches = options
                .kinds
                .as_ref()
                .is_none_or(|kinds| kinds.contains(&entry.kind));
            let confidence_matches = options
                .min_confidence
                .is_none_or(|minimum| entry.confidence >= minimum);
            if !kind_matches || !confidence_matches {
                skipped_filter += 1;
                continue;
            }
            // Preserve the source confidence for contradiction enforcement.
            // Applying the import discount must not weaken a high-confidence
            // refutation or make the result depend on source record order.
            if entry.kind == KnowledgeKind::AntiKnowledge && entry.confidence > 0.8 {
                source_contradictions.push(normalize_entry_security(entry.clone()));
            }
            if options.reset_tier {
                entry.tier = KnowledgeTier::Transient;
            }
            entry.confidence = (entry.confidence * options.confidence_discount).clamp(0.0, 1.0);
            entry.confidence_weight =
                (entry.confidence_weight * options.confidence_discount).clamp(0.0, 1.0);
            entry.source = Some(options.source_label.clone());
            transformed.push(normalize_entry_security(entry));
        }

        let _guard = self.write_gate.lock();
        let mut merged = self.read_all_strict()?;
        let security_upgraded = join_replayed_security_labels(&mut merged, &transformed);
        let mut admitted = Vec::new();
        let mut skipped_dedup = 0;
        let mut skipped_contradiction = 0;
        let mut seen_ids = merged
            .iter()
            .filter(|entry| !entry.id.trim().is_empty())
            .map(|entry| entry.id.clone())
            .collect::<HashSet<_>>();

        for entry in transformed {
            if entry.id.trim().is_empty() {
                anyhow::bail!("malformed_entries=1: imported knowledge entry has an empty ID");
            }
            if !seen_ids.insert(entry.id.clone()) {
                skipped_dedup += 1;
                continue;
            }
            if import_entry_is_contradicted(&entry, &merged, &source_contradictions) {
                skipped_contradiction += 1;
                continue;
            }
            if import_entry_is_semantic_duplicate(&entry, &merged, &admitted) {
                skipped_dedup += 1;
                continue;
            }
            admitted.push(entry);
        }

        let imported = admitted.len();
        if imported > 0 || security_upgraded {
            merged.extend(admitted.iter().cloned());
            self.rewrite_all(&merged)?;
            self.register_temporal_entries(&admitted);
        }

        let result = ImportResult {
            source_entries,
            imported,
            skipped_dedup,
            skipped_contradiction,
            skipped_filter,
            malformed: 0,
            legacy_input,
        };
        tracing::info!(
            entries_imported = result.imported,
            entries_skipped_dedup = result.skipped_dedup,
            entries_skipped_contradiction = result.skipped_contradiction,
            entries_skipped_filter = result.skipped_filter,
            malformed_entries = result.malformed,
            legacy_input = result.legacy_input,
            "knowledge import completed"
        );
        Ok(result)
    }

    /// Decay confidence for old entries using their configured half-life.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn decay(&self) -> Result<usize> {
        let _guard = self.write_gate.lock();
        let now = Utc::now();
        let mut entries = self.read_all()?;
        let decayed = entries.len();

        for entry in &mut entries {
            let factor = recency_factor(entry, now);
            let decayed_confidence = (entry.confidence.max(0.0) * factor).clamp(0.0, 1.0);
            entry.confidence = if entry.kind == KnowledgeKind::AntiKnowledge {
                decayed_confidence.max(ANTI_KNOWLEDGE_CONFIDENCE_FLOOR)
            } else {
                decayed_confidence
            };
        }

        self.rewrite_all(&entries)?;
        Ok(decayed)
    }

    /// Garbage-collect entries whose confidence falls below `min_confidence`.
    ///
    /// NEURO-11: Entries below the balance floor are frozen instead of
    /// deleted, preserving them for potential thawing later. Entries that
    /// are *both* below the confidence threshold *and* already frozen are
    /// permanently removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn gc(&self, min_confidence: f64) -> Result<usize> {
        let _guard = self.write_gate.lock();
        let threshold = min_confidence.max(0.0);
        let before = self.read_all()?;
        let before_len = before.len();
        let entries = before
            .into_iter()
            .filter(|entry| {
                entry.kind == KnowledgeKind::AntiKnowledge
                    || effective_confidence(entry) >= threshold
            })
            .collect::<Vec<_>>();
        let removed = before_len.saturating_sub(entries.len());
        self.rewrite_all(&entries)?;
        self.synchronize_temporal_entries(&entries);
        Ok(removed)
    }

    /// NEURO-11: Garbage-collect entries with freeze-before-delete semantics.
    ///
    /// Entries below the confidence threshold are frozen into cold storage
    /// instead of permanently deleted, provided they haven't been frozen
    /// already. Entries that are *already frozen* and still below the
    /// threshold are permanently removed. AntiKnowledge entries are always
    /// preserved.
    ///
    /// Returns the number of entries permanently removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn gc_with_freeze(&self, min_confidence: f64) -> Result<usize> {
        let _guard = self.write_gate.lock();
        let threshold = min_confidence.max(0.0);
        let before = self.read_all()?;
        let before_len = before.len();
        let mut entries = Vec::with_capacity(before_len);
        for mut entry in before {
            if entry.kind == KnowledgeKind::AntiKnowledge {
                entries.push(entry);
                continue;
            }
            let eff = effective_confidence(&entry);
            if eff >= threshold {
                entries.push(entry);
                continue;
            }
            // Below threshold: freeze or remove.
            if entry.frozen {
                // Already frozen and still below threshold: permanently remove.
                continue;
            }
            // First time below threshold: freeze into cold storage.
            entry.freeze();
            entries.push(entry);
        }
        let removed = before_len.saturating_sub(entries.len());
        self.rewrite_all(&entries)?;
        self.synchronize_temporal_entries(&entries);
        Ok(removed)
    }

    /// Resurrect a frozen/dead entry by re-confirming it with fresh weight.
    ///
    /// Per spec (agent-chain-new/04-knowledge-layer.md): entries that drop below
    /// 1% threshold enter the Death stage and are pruned. If they are later
    /// re-confirmed by a new episode, they are "resurrected" with fresh weight
    /// and reset to Transient tier for re-validation.
    ///
    /// Returns `true` if the entry was found and resurrected.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn resurrect(&self, entry_id: &str, confirming_episode: &str) -> Result<bool> {
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let mut found = false;

        for entry in &mut entries {
            if entry.id == entry_id && entry.frozen {
                // Resurrect: fresh confidence, reset tier, unfreeze.
                entry.confidence = RESURRECTION_CONFIDENCE;
                entry.tier = KnowledgeTier::Transient;
                entry.frozen = false;
                entry.frozen_at = None;
                entry.balance_depleted_at = None;
                entry.balance = 1.0; // Fresh balance
                entry.confirmation_count += 1;
                if !entry
                    .source_episodes
                    .contains(&confirming_episode.to_string())
                {
                    entry.source_episodes.push(confirming_episode.to_string());
                }
                entry.created_at = Utc::now(); // Reset age for decay calculation
                found = true;
                break;
            }
        }

        if found {
            self.rewrite_all(&entries)?;
        }
        Ok(found)
    }

    /// Prune entries that have decayed below the death threshold (1% of initial weight).
    ///
    /// Unlike `gc()` which uses confidence directly, this checks the recency-adjusted
    /// effective weight per the knowledge lifecycle spec.
    ///
    /// Returns the number of entries pruned (or frozen if using freeze semantics).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn prune_dead(&self) -> Result<usize> {
        let _guard = self.write_gate.lock();
        let now = Utc::now();
        let before = self.read_all()?;
        let before_len = before.len();
        let mut entries = Vec::with_capacity(before_len);

        for mut entry in before {
            // AntiKnowledge is always preserved.
            if entry.kind == KnowledgeKind::AntiKnowledge {
                entries.push(entry);
                continue;
            }

            if is_dead(&entry, now) {
                if entry.frozen {
                    // Already frozen and dead: permanent removal.
                    continue;
                }
                // Freeze into cold storage (preserves for potential resurrection).
                entry.freeze();
            }
            entries.push(entry);
        }

        let removed = before_len.saturating_sub(entries.len());
        self.rewrite_all(&entries)?;
        Ok(removed)
    }

    /// NEURO-10: Apply demurrage tax to all entries based on elapsed time
    /// since their creation.
    ///
    /// Returns the number of entries that were taxed.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn apply_demurrage(&self) -> Result<usize> {
        let _guard = self.write_gate.lock();
        let now = Utc::now();
        let mut entries = self.read_all()?;
        let mut taxed = 0usize;
        for entry in &mut entries {
            let elapsed_hours =
                now.signed_duration_since(entry.created_at).num_seconds() as f64 / 3600.0;
            if elapsed_hours > 0.0 && entry.balance > 0.0 {
                entry.apply_demurrage(elapsed_hours);
                taxed += 1;
            }
        }
        if taxed > 0 {
            self.rewrite_all(&entries)?;
        }
        Ok(taxed)
    }

    /// Deduct a fixed balance tax from every active entry.
    ///
    /// This daily-style sweep complements confidence decay. Crossing the
    /// non-positive balance boundary halves the entry's base half-life once;
    /// remaining depleted for more than seven days moves the entry into cold
    /// storage.
    pub fn demurrage(&self, tax_rate: f64) -> Result<usize> {
        ensure!(tax_rate.is_finite(), "demurrage tax rate must be finite");
        ensure!(tax_rate >= 0.0, "demurrage tax rate must be non-negative");
        if tax_rate == 0.0 {
            return Ok(0);
        }

        let now = Utc::now();
        self.update_entries(|entry| {
            if entry.frozen {
                return false;
            }

            let was_positive = entry.balance > 0.0;
            entry.balance -= tax_rate;
            if entry.balance <= 0.0 {
                if was_positive {
                    entry.half_life_days =
                        (entry.half_life_days.max(f64::EPSILON) / 2.0).max(f64::EPSILON);
                }
                let depleted_at = entry.balance_depleted_at.get_or_insert(now);
                if now.signed_duration_since(*depleted_at).num_days() > 7 {
                    entry.freeze();
                }
            } else {
                entry.balance_depleted_at = None;
            }
            true
        })
    }

    /// Apply the fixed balance bump associated with a reinforcement signal.
    pub fn reinforce(&self, entry_id: &str, signal: crate::ReinforcementSignal) -> Result<()> {
        let mut found = false;
        self.update_entries(|entry| {
            if entry.id != entry_id {
                return false;
            }
            entry.balance = (entry.balance + signal.base_value()).min(5.0);
            if entry.balance > 0.0 {
                entry.balance_depleted_at = None;
            }
            found = true;
            true
        })?;
        ensure!(found, "knowledge entry `{entry_id}` was not found");
        Ok(())
    }

    /// Check the active falsifier carried by a Heuristic or AntiKnowledge entry.
    pub fn check_falsifier(&self, entry_id: &str, violated: bool) -> Result<FalsifierOutcome> {
        const IMMUNITY_OBSERVATIONS: u32 = 3;

        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let entry = entries
            .iter_mut()
            .find(|entry| entry.id == entry_id)
            .with_context(|| format!("knowledge entry `{entry_id}` was not found"))?;
        ensure!(
            matches!(
                entry.kind,
                KnowledgeKind::Heuristic | KnowledgeKind::AntiKnowledge
            ),
            "falsifiers only apply to heuristic and anti-knowledge entries"
        );
        let falsifier: &mut Falsifier = entry
            .falsifier
            .as_mut()
            .context("knowledge entry has no falsifier")?;
        ensure!(falsifier.active, "knowledge entry falsifier is inactive");

        falsifier.observations = falsifier.observations.saturating_add(1);
        falsifier.last_checked = Utc::now();
        let outcome = if violated {
            falsifier.violations = falsifier.violations.saturating_add(1);
            falsifier.active = false;
            entry.confidence = (entry.confidence * 0.5).clamp(0.0, 1.0);
            FalsifierOutcome::Discredited
        } else if falsifier.observations >= IMMUNITY_OBSERVATIONS {
            entry.confidence = entry.confidence.max(0.9);
            if entry.tier.multiplier() < KnowledgeTier::Consolidated.multiplier() {
                entry.tier = KnowledgeTier::Consolidated;
            }
            FalsifierOutcome::Immunized
        } else {
            FalsifierOutcome::Survived
        };
        self.rewrite_all(&entries)?;
        Ok(outcome)
    }

    /// NEURO-10: Reinforce a specific entry by ID with the given signal.
    ///
    /// Returns `true` if the entry was found and reinforced.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn reinforce_entry(
        &self,
        entry_id: &str,
        signal: crate::ReinforcementSignal,
        novelty: f64,
    ) -> Result<bool> {
        let mut found = false;
        self.update_entries(|entry| {
            if entry.id == entry_id {
                entry.reinforce(signal, novelty);
                found = true;
                true
            } else {
                false
            }
        })?;
        Ok(found)
    }

    /// NEURO-10: Reinforce a batch of entries in one store rewrite.
    ///
    /// Returns the number of entries found and reinforced.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn reinforce_batch(
        &self,
        entry_ids: &[&str],
        signal: crate::ReinforcementSignal,
        novelty: f64,
    ) -> Result<usize> {
        if entry_ids.is_empty() {
            return Ok(0);
        }

        let id_set = entry_ids
            .iter()
            .map(|id| id.trim())
            .filter(|id| !id.is_empty())
            .collect::<HashSet<_>>();
        if id_set.is_empty() {
            return Ok(0);
        }

        self.update_entries(|entry| {
            if id_set.contains(entry.id.as_str()) {
                entry.reinforce(signal, novelty);
                true
            } else {
                false
            }
        })
    }

    /// Apply configurable tier promotion and demotion to the whole store.
    pub fn apply_tier_progression(
        &self,
        config: &crate::tier_progression::TierProgressionConfig,
    ) -> Result<crate::tier_progression::EntryTierProgressionReport> {
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let report =
            crate::tier_progression::TierProgression::default().evaluate_all(&mut entries, config);
        if !report.promoted.is_empty() || !report.demoted.is_empty() {
            self.rewrite_all(&entries)?;
        }
        Ok(report)
    }

    /// Score knowledge entries by prediction utility (P0-34).
    ///
    /// When a prediction resolves, entries that were in the context pack should
    /// receive utility increments (if the prediction was accurate) or decrements
    /// (if inaccurate). This shifts curation from popularity-based (confirmations)
    /// to effectiveness-based (did these entries help agents succeed?).
    ///
    /// `context_entry_ids`: IDs of entries that were in the context pack when
    ///   the prediction was made.
    /// `prediction_accurate`: whether the prediction residual was within the
    ///   predicted interval.
    /// `accuracy_score`: scalar accuracy in [0.0, 1.0] (higher = better prediction).
    ///
    /// Returns the number of entries updated.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn score_prediction_utility(
        &self,
        context_entry_ids: &[String],
        prediction_accurate: bool,
        accuracy_score: f64,
    ) -> Result<usize> {
        if context_entry_ids.is_empty() {
            return Ok(0);
        }

        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let mut updated = 0;

        // Utility delta: positive for accurate predictions, negative for inaccurate.
        // Scaled by accuracy_score so barely-accurate predictions give small bumps.
        let delta = if prediction_accurate {
            0.05 * accuracy_score.clamp(0.0, 1.0)
        } else {
            -0.03 * (1.0 - accuracy_score.clamp(0.0, 1.0))
        };

        for entry in &mut entries {
            if context_entry_ids.contains(&entry.id) {
                // Apply utility delta to confidence weight (not raw confidence).
                // This preserves the original confidence while adjusting the
                // retrieval priority based on demonstrated usefulness.
                entry.confidence_weight = (entry.confidence_weight + delta).clamp(0.05, 2.0);

                // Also bump/decay balance (the demurrage system's currency).
                entry.balance = (entry.balance + delta * 2.0).clamp(0.0, 5.0);
                updated += 1;
            }
        }

        if updated > 0 {
            self.rewrite_all(&entries)?;
        }
        Ok(updated)
    }

    /// Increment catalytic scores for entries that helped create new knowledge (P1-58).
    ///
    /// Call this when new knowledge entries are created after a successful task.
    /// `catalyst_entry_ids` are the IDs of entries that were in the context pack
    /// when the task ran.
    ///
    /// Returns the number of entries whose catalytic score was incremented.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn increment_catalytic_scores(&self, catalyst_entry_ids: &[String]) -> Result<usize> {
        if catalyst_entry_ids.is_empty() {
            return Ok(0);
        }

        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let mut updated = 0;

        for entry in &mut entries {
            if catalyst_entry_ids.contains(&entry.id) {
                entry.catalytic_score += 1;
                updated += 1;
            }
        }

        if updated > 0 {
            self.rewrite_all(&entries)?;
        }
        Ok(updated)
    }

    /// Check if the knowledge network is autocatalytic (P1-58).
    ///
    /// An autocatalytic network is self-sustaining: entries on average enable
    /// more than one new entry each. The threshold is configurable (default 1.5).
    ///
    /// Returns `(is_autocatalytic, avg_catalytic_score, entry_count)`.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read.
    pub fn is_autocatalytic(&self, threshold: f64) -> Result<(bool, f64, usize)> {
        let entries = self.read_all()?;
        let active: Vec<_> = entries.iter().filter(|e| !e.frozen).collect();

        if active.is_empty() {
            return Ok((false, 0.0, 0));
        }

        let total_catalytic: f64 = active.iter().map(|e| e.catalytic_score as f64).sum();
        let avg = total_catalytic / active.len() as f64;

        Ok((avg >= threshold, avg, active.len()))
    }

    /// NEURO-11: Thaw a frozen entry, restoring a starter balance.
    ///
    /// Returns `true` if the entry was found, was frozen, and was thawed.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn thaw_entry(&self, entry_id: &str, starter_balance: f64) -> Result<bool> {
        let mut thawed = false;
        self.update_entries(|entry| {
            if entry.id == entry_id && entry.frozen {
                entry.thaw(starter_balance);
                thawed = true;
                true
            } else {
                false
            }
        })?;
        Ok(thawed)
    }

    /// NEURO-11: Query frozen (cold-tier) entries, optionally filtered.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn query_cold(&self, limit: usize) -> Result<Vec<KnowledgeEntry>> {
        let entries = self.read_all()?;
        let mut cold: Vec<_> = entries.into_iter().filter(|e| e.frozen).collect();
        cold.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        cold.truncate(limit);
        Ok(cold)
    }

    /// NEURO-08: Garbage-collect entries while preserving the last
    /// representative of each worldview cluster.
    ///
    /// Uses tag-overlap clustering to group related entries. If all entries
    /// in a cluster would be removed, the highest-confidence entry is kept.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn gc_preserving_worldviews(
        &self,
        min_confidence: f64,
        min_tag_overlap: usize,
    ) -> Result<usize> {
        let _guard = self.write_gate.lock();
        let before = self.read_all()?;
        let before_len = before.len();
        let entries =
            crate::gc_with_worldview_preservation(before, min_confidence, min_tag_overlap);
        let removed = before_len.saturating_sub(entries.len());
        self.rewrite_all(&entries)?;
        Ok(removed)
    }

    /// Mutate matching entries in place and rewrite the store atomically.
    ///
    /// Returns the number of entries that changed.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn update_entries<F>(&self, mut update: F) -> Result<usize>
    where
        F: FnMut(&mut KnowledgeEntry) -> bool,
    {
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let mut changed = 0usize;
        for entry in &mut entries {
            if update(entry) {
                changed += 1;
            }
        }
        if changed > 0 {
            self.rewrite_all(&entries)?;
        }
        Ok(changed)
    }

    /// Backfill HDC vectors for existing knowledge entries that lack them.
    ///
    /// Reads all entries, computes HDC vectors for any entry whose
    /// `hdc_vector` field is absent or has the wrong byte length, and
    /// atomically rewrites the store. Entries that already have a valid
    /// HDC vector are left unchanged, making this operation idempotent.
    ///
    /// This function is only available when the `hdc` feature is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    #[cfg(feature = "hdc")]
    pub fn backfill_hdc_vectors(&self) -> Result<usize> {
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let mut changed = 0usize;
        for entry in &mut entries {
            let has_valid = entry
                .hdc_vector
                .as_ref()
                .is_some_and(|v| v.len() == HDC_VECTOR_BYTES);
            if !has_valid {
                entry.hdc_vector = Some(fingerprint_entry(entry).to_bytes().to_vec());
                changed += 1;
            }
        }
        if changed > 0 {
            self.rewrite_all(&entries)?;
        }
        Ok(changed)
    }

    /// Adjust the confidence score of a knowledge entry by `delta`.
    ///
    /// The resulting confidence is clamped to `[0.0, 1.0]`. If the entry
    /// is not found, this is a no-op and returns `Ok(false)`.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn update_confidence(&mut self, knowledge_id: &str, delta: f64) -> Result<bool> {
        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let mut found = false;

        for entry in &mut entries {
            if entry.id == knowledge_id {
                entry.confidence = (entry.confidence + delta).clamp(0.0, 1.0);
                Self::maybe_adjust_tier(entry);
                found = true;
                break;
            }
        }

        if found {
            self.rewrite_all(&entries)?;
        }

        Ok(found)
    }

    /// Record a usage outcome for a knowledge entry.
    ///
    /// Successful usage applies a small positive reinforcement (`+0.02`),
    /// while failed usage applies a stronger negative signal (`-0.05`).
    /// Entries that drop below confidence `0.1` after repeated failures are
    /// candidates for the next garbage-collection pass.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn record_usage(&mut self, knowledge_id: &str, succeeded: bool) -> Result<()> {
        let delta = if succeeded { 0.02 } else { -0.05 };
        self.update_confidence(knowledge_id, delta)?;
        tracing::debug!(
            knowledge_id,
            succeeded,
            delta,
            "recorded knowledge usage outcome"
        );
        Ok(())
    }

    /// Record usage outcomes for multiple knowledge entries at once.
    ///
    /// More efficient than calling [`Self::record_usage`] in a loop because it
    /// performs a single load-modify-write cycle.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or rewritten.
    pub fn batch_record_usage(&mut self, outcomes: &[(String, bool)]) -> Result<usize> {
        if outcomes.is_empty() {
            return Ok(0);
        }

        let _guard = self.write_gate.lock();
        let mut entries = self.read_all()?;
        let mut updated_ids = HashSet::new();

        for (knowledge_id, succeeded) in outcomes {
            let delta = if *succeeded { 0.02 } else { -0.05 };
            if let Some(entry) = entries.iter_mut().find(|entry| entry.id == *knowledge_id) {
                entry.confidence = (entry.confidence + delta).clamp(0.0, 1.0);
                Self::maybe_adjust_tier(entry);
                updated_ids.insert(knowledge_id.clone());
            }
        }

        if !updated_ids.is_empty() {
            self.rewrite_all(&entries)?;
        }

        Ok(updated_ids.len())
    }

    /// Read all knowledge entries from the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the store file cannot be read. Malformed nonblank
    /// legacy records are skipped on this compatibility path; imports use a
    /// strict reader before any rewrite.
    pub fn read_all(&self) -> Result<Vec<KnowledgeEntry>> {
        self.read_all_impl(false)
    }

    /// Read every knowledge entry and reject malformed nonblank records.
    ///
    /// This stricter path is used before import rewrites so a damaged existing
    /// store can never be silently shortened by a successful restore.
    fn read_all_strict(&self) -> Result<Vec<KnowledgeEntry>> {
        self.read_all_impl(true)
    }

    fn read_all_impl(&self, strict: bool) -> Result<Vec<KnowledgeEntry>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("open knowledge store at {}", self.path.display()));
            }
        };

        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for (line_idx, line) in reader.lines().enumerate() {
            let line = line.with_context(|| {
                format!(
                    "read knowledge line {} from {}",
                    line_idx + 1,
                    self.path.display()
                )
            })?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<KnowledgeEntry>(&line) {
                Ok(entry) => entries.push(normalize_entry_security(entry)),
                Err(error) if strict => {
                    return Err(error).with_context(|| {
                        format!(
                            "decode knowledge line {} from {}",
                            line_idx + 1,
                            self.path.display()
                        )
                    });
                }
                Err(_) => {}
            }
        }
        Ok(entries)
    }

    /// Read all confirmation records from the confirmations JSONL file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read.
    pub fn read_confirmations(&self) -> Result<Vec<KnowledgeConfirmationRecord>> {
        let file = match File::open(&self.confirmations_path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(err).with_context(|| {
                    format!(
                        "open confirmations file at {}",
                        self.confirmations_path.display()
                    )
                });
            }
        };

        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line.context("read confirmation line")?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<KnowledgeConfirmationRecord>(&line) {
                records.push(record);
            }
        }
        Ok(records)
    }

    fn append_confirmations(&self, records: &[KnowledgeConfirmationRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        if let Some(parent) = self.confirmations_path.parent() {
            fs::create_dir_all(parent).context("create confirmations directory")?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.confirmations_path)
            .with_context(|| {
                format!(
                    "open confirmations file at {}",
                    self.confirmations_path.display()
                )
            })?;
        for record in records {
            let mut line =
                serde_json::to_string(record).context("serialize confirmation record")?;
            line.push('\n');
            file.write_all(line.as_bytes())
                .context("append confirmation record")?;
        }
        file.flush().context("flush confirmation records")?;
        file.sync_all().context("sync confirmation records")?;
        Ok(())
    }

    /// Check if an entry's confidence warrants tier promotion or demotion.
    fn maybe_adjust_tier(entry: &mut KnowledgeEntry) {
        if entry.confidence >= 0.9
            && entry.tier.multiplier() < KnowledgeTier::Consolidated.multiplier()
        {
            entry.tier = KnowledgeTier::Consolidated;
        }

        if entry.confidence <= 0.2
            && entry.tier.multiplier() > KnowledgeTier::Transient.multiplier()
        {
            entry.tier = KnowledgeTier::Transient;
        }

        if entry.confidence <= 0.05 {
            entry.half_life_days = 1.0;
        }
    }

    fn register_temporal_entries(&self, entries: &[KnowledgeEntry]) {
        let Some(index) = &self.temporal_index else {
            return;
        };
        let mut index = index.lock();
        for entry in entries {
            index.add_entry(
                entry.id.clone(),
                TemporalInterval::new(entry.created_at.timestamp_millis(), i64::MAX),
            );
        }
    }

    fn synchronize_temporal_entries(&self, entries: &[KnowledgeEntry]) {
        let Some(index) = &self.temporal_index else {
            return;
        };
        index.lock().replace_entries(entries.iter().map(|entry| {
            (
                entry.id.clone(),
                TemporalInterval::new(entry.created_at.timestamp_millis(), i64::MAX),
            )
        }));
    }

    fn rewrite_all(&self, entries: &[KnowledgeEntry]) -> Result<()> {
        let mut bytes = Vec::new();
        for entry in entries {
            let entry = normalize_entry_security(entry.clone());
            serde_json::to_writer(&mut bytes, &entry).context("serialize knowledge entry")?;
            bytes.push(b'\n');
        }
        roko_fs::atomic_write_bytes(&self.path, &bytes).with_context(|| {
            format!("atomically rewrite knowledge store {}", self.path.display())
        })?;
        Ok(())
    }

    #[cfg(feature = "hdc")]
    /// Build an in-memory HDC index over the current knowledge store.
    ///
    /// The index fingerprints each entry's content once and keeps the
    /// resulting vectors resident for fast similarity search.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read.
    pub fn memory_index(&self) -> Result<MemoryIndex> {
        Ok(MemoryIndex::from_entries(self.read_all()?))
    }
}

impl NeuroStore for KnowledgeStore {
    fn init(path: &Path) -> Result<Self> {
        Ok(Self::new(path))
    }

    fn query(&self, topic: &str, limit: usize) -> Result<Vec<KnowledgeEntry>> {
        KnowledgeStore::query(self, topic, limit)
    }

    fn query_similar(
        &self,
        fingerprint: &[u8],
        limit: usize,
    ) -> Result<Vec<KnowledgeSimilarityHit>> {
        KnowledgeStore::query_similar(self, fingerprint, limit)
    }

    fn ingest(&mut self, entries: Vec<KnowledgeEntry>) -> Result<()> {
        KnowledgeStore::ingest(self, entries)
    }

    fn decay(&mut self) -> Result<usize> {
        KnowledgeStore::decay(self)
    }

    fn gc(&mut self, min_confidence: f64) -> Result<usize> {
        KnowledgeStore::gc(self, min_confidence)
    }

    fn update_confidence(&mut self, knowledge_id: &str, delta: f64) -> Result<bool> {
        KnowledgeStore::update_confidence(self, knowledge_id, delta)
    }

    fn record_usage(&mut self, knowledge_id: &str, succeeded: bool) -> Result<()> {
        KnowledgeStore::record_usage(self, knowledge_id, succeeded)
    }

    fn batch_record_usage(&mut self, outcomes: &[(String, bool)]) -> Result<usize> {
        KnowledgeStore::batch_record_usage(self, outcomes)
    }
}

#[cfg(feature = "hdc")]
/// A precomputed HDC index over durable knowledge entries.
///
/// The index stores both a normalized content fingerprint and the entry's
/// structured HDC vector. Searches rank by the stronger content or structured
/// match, preserving exact content lookup alongside role-aware causal lookup.
#[derive(Debug, Clone)]
pub struct MemoryIndex {
    entries: Vec<IndexedKnowledgeEntry>,
}

#[cfg(feature = "hdc")]
#[derive(Debug, Clone)]
struct IndexedKnowledgeEntry {
    entry: KnowledgeEntry,
    fingerprint: HdcVector,
    content_fingerprint: HdcVector,
}

#[cfg(feature = "hdc")]
/// One HDC search result from a [`MemoryIndex`].
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryHit {
    /// The matched knowledge entry.
    pub entry: KnowledgeEntry,
    /// Similarity against the query fingerprint in the range `0.0..=1.0`.
    pub similarity: f64,
}

#[cfg(feature = "hdc")]
impl MemoryIndex {
    /// Build an index from a collection of knowledge entries.
    ///
    /// Each entry receives content and structured fingerprints. Empty content
    /// still receives a deterministic vector, so the index remains total.
    #[must_use]
    pub fn from_entries(entries: Vec<KnowledgeEntry>) -> Self {
        let entries = entries
            .into_iter()
            .map(|entry| {
                let fingerprint = fingerprint_entry(&entry);
                let content_fingerprint = fingerprint_content(&entry.content);
                IndexedKnowledgeEntry {
                    entry,
                    fingerprint,
                    content_fingerprint,
                }
            })
            .collect();
        Self { entries }
    }

    /// Number of indexed entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Search the index for the `limit` most similar entries to `query`.
    ///
    /// The query is encoded as both content and structured probes, then
    /// compared against each precomputed entry vector. Results are sorted from
    /// highest to lowest similarity.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<MemoryHit> {
        if limit == 0 || self.entries.is_empty() {
            return Vec::new();
        }

        let query_fingerprint = KnowledgeHdcEncoder.encode_query(query);
        let query_content_fingerprint = fingerprint_content(query);
        let mut scored: Vec<MemoryHit> = self
            .entries
            .iter()
            .map(|indexed| {
                let structured_similarity = query_fingerprint.similarity(&indexed.fingerprint);
                let content_similarity =
                    query_content_fingerprint.similarity(&indexed.content_fingerprint);
                MemoryHit {
                    entry: indexed.entry.clone(),
                    similarity: structured_similarity.max(content_similarity) as f64,
                }
            })
            .collect();

        scored.sort_by(compare_hits);
        scored.truncate(limit);
        scored
    }

    /// Return the indexed entries with their precomputed fingerprints.
    ///
    /// This is mainly useful for testing and for consumers that want to
    /// inspect or reuse the durable entries directly.
    #[must_use]
    pub fn entries(&self) -> Vec<KnowledgeEntry> {
        self.entries
            .iter()
            .map(|indexed| indexed.entry.clone())
            .collect()
    }
}

#[cfg(feature = "hdc")]
fn fingerprint_entry(entry: &KnowledgeEntry) -> HdcVector {
    if let Some(vector) = entry.hdc_vector.as_deref()
        && let Ok(bytes) = <[u8; HDC_VECTOR_BYTES]>::try_from(vector)
    {
        return HdcVector::from_bytes(&bytes);
    }
    KnowledgeHdcEncoder.encode_entry(entry)
}

#[cfg(feature = "hdc")]
fn fingerprint_content(content: &str) -> HdcVector {
    text_fingerprint(&normalize(content))
}

#[cfg(feature = "hdc")]
fn prepare_entries_for_ingest(entries: Vec<KnowledgeEntry>) -> Vec<KnowledgeEntry> {
    entries
        .into_iter()
        .map(normalize_entry_for_ingest)
        .collect()
}

#[cfg(not(feature = "hdc"))]
fn prepare_entries_for_ingest(entries: Vec<KnowledgeEntry>) -> Vec<KnowledgeEntry> {
    entries
        .into_iter()
        .map(normalize_entry_for_ingest)
        .collect()
}

#[cfg(feature = "hdc")]
fn ensure_hdc_vector(mut entry: KnowledgeEntry) -> KnowledgeEntry {
    let has_valid_vector = entry
        .hdc_vector
        .as_ref()
        .is_some_and(|vector| vector.len() == HDC_VECTOR_BYTES);
    if !has_valid_vector {
        entry.hdc_vector = Some(fingerprint_entry(&entry).to_bytes().to_vec());
    }
    entry
}

fn normalize_entry_for_ingest(entry: KnowledgeEntry) -> KnowledgeEntry {
    let entry = normalize_entry_tier(normalize_entry_security(entry));
    #[cfg(feature = "hdc")]
    {
        ensure_hdc_vector(entry)
    }
    #[cfg(not(feature = "hdc"))]
    {
        entry
    }
}

fn normalize_entry_security(mut entry: KnowledgeEntry) -> KnowledgeEntry {
    let channel = entry
        .source
        .as_deref()
        .map(SourceChannel::from_source_label)
        .unwrap_or(SourceChannel::AgentOutput);
    apply_source_security_labels(std::slice::from_mut(&mut entry), channel);
    entry
}

fn coalesce_incoming_security_labels(entries: Vec<KnowledgeEntry>) -> Vec<KnowledgeEntry> {
    let mut coalesced: Vec<KnowledgeEntry> = Vec::with_capacity(entries.len());
    for entry in entries {
        if !entry.id.trim().is_empty()
            && let Some(existing) = coalesced.iter_mut().find(|item| item.id == entry.id)
        {
            existing.origin_taint = existing.origin_taint.max(entry.origin_taint);
            existing.classification = existing.classification.join(entry.classification);
        } else {
            coalesced.push(entry);
        }
    }
    coalesced
}

fn join_replayed_security_labels(
    existing: &mut [KnowledgeEntry],
    incoming: &[KnowledgeEntry],
) -> bool {
    let mut changed = false;
    for candidate in incoming {
        if candidate.id.trim().is_empty() {
            continue;
        }
        let Some(stored) = existing.iter_mut().find(|entry| entry.id == candidate.id) else {
            continue;
        };
        let joined_origin = stored.origin_taint.max(candidate.origin_taint);
        let joined_classification = stored.classification.join(candidate.classification);
        if joined_origin != stored.origin_taint || joined_classification != stored.classification {
            stored.origin_taint = joined_origin;
            stored.classification = joined_classification;
            changed = true;
        }
    }
    changed
}

fn normalize_entry_tier(mut entry: KnowledgeEntry) -> KnowledgeEntry {
    let inferred = inferred_retention_tier(&entry);
    if inferred.multiplier() > entry.tier.multiplier() {
        entry.tier = inferred;
    }
    entry
}

fn inferred_retention_tier(entry: &KnowledgeEntry) -> KnowledgeTier {
    let source_count = entry.source_episodes.len();
    let confidence = entry.confidence.clamp(0.0, 1.0);

    match entry.kind {
        KnowledgeKind::StrategyFragment if source_count >= 3 => KnowledgeTier::Persistent,
        KnowledgeKind::StrategyFragment => KnowledgeTier::Working,
        KnowledgeKind::Warning if source_count >= 2 || confidence >= 0.85 => {
            KnowledgeTier::Consolidated
        }
        KnowledgeKind::Warning => KnowledgeTier::Working,
        KnowledgeKind::AntiKnowledge => KnowledgeTier::Working,
        _ if source_count >= 4 || confidence >= 0.9 => KnowledgeTier::Consolidated,
        _ if source_count >= 2 || confidence >= 0.7 => KnowledgeTier::Working,
        _ => KnowledgeTier::Transient,
    }
}

fn compare_similarity_hits(
    left: &KnowledgeSimilarityHit,
    right: &KnowledgeSimilarityHit,
) -> std::cmp::Ordering {
    right
        .similarity
        .partial_cmp(&left.similarity)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            effective_confidence(&right.entry)
                .partial_cmp(&effective_confidence(&left.entry))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| right.entry.created_at.cmp(&left.entry.created_at))
        .then_with(|| left.entry.id.cmp(&right.entry.id))
}

#[cfg(feature = "hdc")]
fn compare_hits(left: &MemoryHit, right: &MemoryHit) -> std::cmp::Ordering {
    right
        .similarity
        .partial_cmp(&left.similarity)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            effective_confidence(&right.entry)
                .partial_cmp(&effective_confidence(&left.entry))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| right.entry.created_at.cmp(&left.entry.created_at))
        .then_with(|| left.entry.id.cmp(&right.entry.id))
}

fn normalize(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
}

fn tokenize(text: &str) -> Vec<String> {
    normalize(text)
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn keyword_score(entry: &KnowledgeEntry, terms: &[String], topic_norm: &str) -> f64 {
    let content = normalize(&entry.content);
    let tags: Vec<String> = entry.tags.iter().map(|tag| normalize(tag)).collect();

    let mut score = 0.0;
    if !topic_norm.is_empty() {
        if content.contains(topic_norm) {
            score += 1.0;
        }
        if tags
            .iter()
            .any(|tag| tag.contains(topic_norm) || topic_norm.contains(tag))
        {
            score += 1.0;
        }
    }

    for term in terms {
        if content.contains(term)
            || tags
                .iter()
                .any(|tag| tag.contains(term) || term.contains(tag))
        {
            score += 1.0;
        }
    }

    score
}

/// Compute recency factor with confirmation-adjusted decay.
///
/// Per spec: `weight = initialWeight * 0.5^(age/halfLife) * (1 + confirmations * 0.1)`
///
/// Confirmations extend the effective lifetime — each independent confirmation
/// adds 10% to the weight, rewarding knowledge that multiple episodes validate.
fn recency_factor(entry: &KnowledgeEntry, now: DateTime<Utc>) -> f64 {
    let age = now
        .signed_duration_since(entry.created_at)
        .num_seconds()
        .max(0) as f64
        / 86_400.0;
    let half_life = effective_half_life_days(entry);
    let base_decay = 0.5_f64.powf(age / half_life);
    let confirmation_adjustment = 1.0 + entry.confirmation_count as f64 * CONFIRMATION_DECAY_FACTOR;
    base_decay * confirmation_adjustment
}

/// Check if an entry has decayed below the death threshold.
///
/// Returns true if the entry's recency factor is below 1% of initial weight,
/// indicating it should enter the Death stage per the knowledge lifecycle spec.
pub fn is_dead(entry: &KnowledgeEntry, now: DateTime<Utc>) -> bool {
    let factor = recency_factor(entry, now);
    factor < DEATH_THRESHOLD
}

fn effective_half_life_days(entry: &KnowledgeEntry) -> f64 {
    entry.effective_half_life_days()
}

fn effective_confidence(entry: &KnowledgeEntry) -> f64 {
    bounded_confidence(entry) * confirmation_boost(entry) * entry.emotional_consolidation_boost()
}

fn bounded_confidence(entry: &KnowledgeEntry) -> f64 {
    let confidence = entry.confidence.clamp(0.0, 1.0);
    if entry.kind == KnowledgeKind::AntiKnowledge {
        confidence.max(ANTI_KNOWLEDGE_CONFIDENCE_FLOOR)
    } else {
        confidence
    }
}

fn confirmation_boost(entry: &KnowledgeEntry) -> f64 {
    if entry.source_episodes.len() >= 2 {
        CONFIRMATION_BOOST
    } else {
        1.0
    }
}

fn compare_hit_scores(left: &KnowledgeQueryHit, right: &KnowledgeQueryHit) -> std::cmp::Ordering {
    right
        .total_score
        .partial_cmp(&left.total_score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            right
                .breakdown
                .effective_confidence
                .partial_cmp(&left.breakdown.effective_confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| right.entry.created_at.cmp(&left.entry.created_at))
}

fn emotional_retrieval_boost(entry: &KnowledgeEntry) -> f64 {
    entry.emotional_retrieval_boost()
}

fn knowledge_kind_label(kind: KnowledgeKind) -> &'static str {
    kind.as_str()
}

fn similarity_against_entry(fingerprint: &[u8], entry: &KnowledgeEntry) -> Option<f32> {
    let stored = entry.hdc_vector.as_deref()?;
    if stored.len() != HDC_VECTOR_BYTES {
        return None;
    }

    let differing_bits = fingerprint
        .iter()
        .zip(stored.iter())
        .map(|(left, right)| (left ^ right).count_ones())
        .sum::<u32>();
    Some(1.0 - (differing_bits as f32 / (HDC_VECTOR_BYTES * 8) as f32))
}

#[cfg(feature = "hdc")]
fn hdc_similarity(entry: &KnowledgeEntry, topic: &str) -> f64 {
    let Some(vector) = entry.hdc_vector.as_deref() else {
        return 0.0;
    };
    let Ok(bytes) = <[u8; HDC_VECTOR_BYTES]>::try_from(vector) else {
        return 0.0;
    };
    let entry_vec = HdcVector::from_bytes(&bytes);
    let topic_vec = KnowledgeHdcEncoder.encode_query(topic);
    let raw_similarity = topic_vec.similarity(&entry_vec) as f64;
    if raw_similarity < HDC_QUERY_RELEVANCE_THRESHOLD {
        return 0.0;
    }
    raw_similarity - HDC_SIMILARITY_BASELINE
}

fn score_entry_for_query(
    entry: KnowledgeEntry,
    topic_terms: &[String],
    topic_norm: &str,
    _topic: &str,
    now: DateTime<Utc>,
) -> Option<KnowledgeQueryHit> {
    let keyword = keyword_score(&entry, topic_terms, topic_norm);
    let recency = recency_factor(&entry, now);
    let confidence = effective_confidence(&entry);
    let emotional = emotional_retrieval_boost(&entry);

    // NEURO-10: Additive balance/freshness boost so reinforced entries rank above
    // otherwise equivalent zero-balance entries.  Clamped to [0, BALANCE_FRESHNESS_WEIGHT]
    // so it acts as a tie-breaker rather than overriding keyword relevance.
    let balance_freshness_boost = BALANCE_FRESHNESS_WEIGHT * entry.freshness(now).clamp(0.0, 1.0);

    #[cfg(feature = "hdc")]
    let hdc = {
        let similarity = hdc_similarity(&entry, _topic);
        (similarity > 0.0).then_some(similarity)
    };
    #[cfg(feature = "hdc")]
    let hdc_contribution = hdc.unwrap_or(0.0);

    #[cfg(not(feature = "hdc"))]
    let hdc: Option<f64> = None;
    #[cfg(not(feature = "hdc"))]
    let hdc_contribution = 0.0;

    let relevance_score = keyword * confidence * recency * emotional + hdc_contribution;
    if relevance_score <= QUERY_SCORE_FLOOR {
        return None;
    }
    let total = relevance_score + balance_freshness_boost;
    Some(KnowledgeQueryHit {
        entry,
        total_score: total,
        breakdown: KnowledgeQueryBreakdown {
            keyword_score: keyword,
            effective_confidence: confidence,
            recency_factor: recency,
            emotional_boost: emotional,
            balance_freshness_boost,
            hdc_similarity: hdc,
        },
    })
}

fn entry_similarity(existing: &KnowledgeEntry, candidate: &KnowledgeEntry) -> f64 {
    if existing
        .content
        .trim()
        .eq_ignore_ascii_case(candidate.content.trim())
        && !existing.content.trim().is_empty()
    {
        return 1.0;
    }

    let existing_tags: HashSet<String> = existing.tags.iter().map(|tag| normalize(tag)).collect();
    let candidate_tags: HashSet<String> = candidate.tags.iter().map(|tag| normalize(tag)).collect();
    let tag_score = jaccard_similarity(&existing_tags, &candidate_tags);

    let existing_terms: HashSet<String> = tokenize(&existing.content).into_iter().collect();
    let candidate_terms: HashSet<String> = tokenize(&candidate.content).into_iter().collect();
    let keyword_score = jaccard_similarity(&existing_terms, &candidate_terms);

    (tag_score * 0.4 + keyword_score * 0.6).clamp(0.0, 1.0)
}

fn jaccard_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 0.0;
    }
    let intersection = left.intersection(right).count() as f64;
    let union = left.union(right).count() as f64;
    if union <= 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Compare two knowledge entries for topic-level similarity using tag
/// overlap and content keyword matching. This is deliberately lightweight
/// (no ML, no embedding) to keep `ingest()` fast.
fn entries_are_similar(existing: &KnowledgeEntry, new_entry: &KnowledgeEntry) -> bool {
    // Skip AntiKnowledge entries -- they are refutations, not confirmations.
    if existing.kind == KnowledgeKind::AntiKnowledge
        || new_entry.kind == KnowledgeKind::AntiKnowledge
    {
        return false;
    }

    // Tag overlap: normalize and intersect.
    let existing_tags: HashSet<String> = existing.tags.iter().map(|tag| normalize(tag)).collect();
    let new_tags: HashSet<String> = new_entry.tags.iter().map(|tag| normalize(tag)).collect();
    let tag_overlap = existing_tags.intersection(&new_tags).count();

    if tag_overlap < MIN_TAG_OVERLAP {
        return false;
    }

    // Content keyword overlap: tokenize and intersect.
    let existing_keywords: HashSet<String> = tokenize(&existing.content).into_iter().collect();
    let new_keywords: HashSet<String> = tokenize(&new_entry.content).into_iter().collect();
    let keyword_overlap = existing_keywords.intersection(&new_keywords).count();

    keyword_overlap >= MIN_KEYWORD_OVERLAP
}

fn dedupe_entries_for_ingest(
    entries: Vec<KnowledgeEntry>,
    existing: &[KnowledgeEntry],
) -> Vec<KnowledgeEntry> {
    let mut seen_ids = existing
        .iter()
        .filter(|entry| !entry.id.trim().is_empty())
        .map(|entry| entry.id.clone())
        .collect::<HashSet<_>>();

    entries
        .into_iter()
        .filter(|entry| {
            let id = entry.id.trim();
            id.is_empty() || seen_ids.insert(id.to_string())
        })
        .collect()
}

/// Scan new entries against existing entries to find confirmations.
///
/// Returns a list of confirmation records for each (existing, new) pair
/// where the entries are similar enough to indicate independent
/// confirmation of the same insight.
fn detect_confirmations(
    existing: &[KnowledgeEntry],
    new_entries: &[KnowledgeEntry],
) -> Vec<KnowledgeConfirmationRecord> {
    let now = Utc::now();
    let mut confirmations = Vec::new();

    for new_entry in new_entries {
        for existing_entry in existing {
            if existing_entry.id == new_entry.id {
                continue;
            }
            if !entries_are_similar(existing_entry, new_entry) {
                continue;
            }

            // Merge source episodes from both entries.
            let mut source_episodes: Vec<String> = existing_entry
                .source_episodes
                .iter()
                .chain(new_entry.source_episodes.iter())
                .cloned()
                .collect();
            source_episodes.sort();
            source_episodes.dedup();

            confirmations.push(KnowledgeConfirmationRecord {
                created_at: now,
                source_episodes,
                confirmed_entry_id: existing_entry.id.clone(),
                confirming_entry_id: new_entry.id.clone(),
            });
        }
    }

    confirmations
}

/// Check new non-AntiKnowledge entries against existing AntiKnowledge entries
/// using HDC similarity. Returns the filtered/modified list of entries:
/// - similarity > 0.9: entry rejected entirely
/// - similarity > 0.7: entry confidence discounted by 0.5x
/// - similarity > 0.5: warning logged
#[cfg(feature = "hdc")]
fn check_against_anti_knowledge(
    entries: Vec<KnowledgeEntry>,
    existing: &[KnowledgeEntry],
) -> Vec<KnowledgeEntry> {
    let anti_entries: Vec<_> = existing
        .iter()
        .filter(|e| e.kind == KnowledgeKind::AntiKnowledge)
        .collect();

    if anti_entries.is_empty() {
        return entries;
    }

    // Pre-encode all AntiKnowledge entries.
    let encoder = KnowledgeHdcEncoder;
    let anti_vectors: Vec<_> = anti_entries
        .iter()
        .map(|e| (e, fingerprint_entry(e), fingerprint_content(&e.content)))
        .collect();

    let mut result = Vec::with_capacity(entries.len());

    for mut entry in entries {
        if entry.kind == KnowledgeKind::AntiKnowledge {
            result.push(entry);
            continue;
        }

        let entry_vec = encoder.encode_entry(&entry);
        let entry_content_vec = fingerprint_content(&entry.content);
        let mut worst_similarity = 0.0_f64;
        let mut worst_anti_id = String::new();

        for (anti_entry, anti_vec, anti_content_vec) in &anti_vectors {
            let structured_similarity = entry_vec.similarity(anti_vec);
            let content_similarity = entry_content_vec.similarity(anti_content_vec);
            let sim = structured_similarity.max(content_similarity) as f64;
            if sim > worst_similarity {
                worst_similarity = sim;
                worst_anti_id = anti_entry.id.clone();
            }
        }

        if worst_similarity > ANTI_KNOWLEDGE_REJECT_THRESHOLD {
            tracing::warn!(
                entry_id = %entry.id,
                anti_knowledge_id = %worst_anti_id,
                similarity = worst_similarity,
                "rejecting entry: near-duplicate of refuted AntiKnowledge"
            );
            continue; // reject
        }

        if worst_similarity > ANTI_KNOWLEDGE_DISCOUNT_THRESHOLD {
            tracing::warn!(
                entry_id = %entry.id,
                anti_knowledge_id = %worst_anti_id,
                similarity = worst_similarity,
                "discounting entry confidence: conflicts with AntiKnowledge"
            );
            entry.confidence *= ANTI_KNOWLEDGE_DISCOUNT_FACTOR;
        } else if worst_similarity > ANTI_KNOWLEDGE_WARN_THRESHOLD {
            tracing::warn!(
                entry_id = %entry.id,
                anti_knowledge_id = %worst_anti_id,
                similarity = worst_similarity,
                "potential conflict with AntiKnowledge"
            );
        }

        result.push(entry);
    }

    result
}

/// Create an AntiKnowledge entry from a failed gate result.
///
/// The entry captures the task context, gate name, failure text, and agent
/// output snippet so future runs can avoid repeating the same mistake.
#[must_use]
pub fn extract_anti_pattern_from_failure(
    task_id: &str,
    task_prompt: &str,
    gate_name: &str,
    gate_error: &str,
    agent_output: Option<&str>,
) -> KnowledgeEntry {
    let created_at = Utc::now();
    let task_id = if task_id.trim().is_empty() {
        "unknown-task"
    } else {
        task_id.trim()
    };
    let gate_name = if gate_name.trim().is_empty() {
        "unknown-gate"
    } else {
        gate_name.trim()
    };
    let gate_name_norm = gate_name.to_ascii_lowercase();
    let task_prompt_text = task_prompt.trim();
    let gate_error_text = gate_error.trim();
    let agent_output_text = agent_output.map(|output| output.trim()).unwrap_or("");

    let task_prompt_snippet = if task_prompt_text.is_empty() {
        "unknown task prompt".to_string()
    } else {
        truncate_snippet(task_prompt_text, 100)
    };
    let gate_error_snippet = if gate_error_text.is_empty() {
        "unknown error".to_string()
    } else {
        truncate_snippet(gate_error_text, 240)
    };
    let agent_output_snippet = if agent_output_text.is_empty() {
        None
    } else {
        Some(truncate_snippet(agent_output_text, 200))
    };

    let mut content = format!(
        "Anti-pattern for task type '{task_prompt_snippet}': Gate '{gate_name}' failed with: {gate_error_snippet}."
    );
    if let Some(snippet) = &agent_output_snippet {
        content.push_str(" Agent output snippet: ");
        content.push_str(snippet);
    }

    let mut tags = vec![
        "bench".to_string(),
        format!("gate:{gate_name_norm}"),
        format!("task:{task_id}"),
    ];
    tags.extend(classify_compilation_error(gate_error_text));
    tags.extend(compilation_error_code_tags(gate_error_text));
    tags.sort();
    tags.dedup();

    let mut refutation_evidence = format!("Gate '{gate_name}' failed with: {gate_error_snippet}");
    if let Some(snippet) = &agent_output_snippet {
        refutation_evidence.push_str(" Agent output snippet: ");
        refutation_evidence.push_str(snippet);
    }

    let id_payload = format!(
        "{task_id}\x1f{task_prompt_text}\x1f{gate_name}\x1f{gate_error_text}\x1f{agent_output_text}"
    );

    KnowledgeEntry {
        id: format!("anti-{task_id}-{:016x}", stable_hash(id_payload.as_bytes())),
        kind: KnowledgeKind::AntiKnowledge,
        source: Some("bench-gate-failure".to_string()),
        origin_taint: Default::default(),
        classification: Default::default(),
        content,
        confidence: 0.6,
        confidence_weight: -0.6,
        refuted_insight_id: None,
        refutation_evidence: Some(refutation_evidence),
        source_episodes: Vec::new(),
        tags,
        source_model: None,
        model_generality: 1.0,
        created_at,
        half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
        tier: KnowledgeTier::Transient,
        emotional_tag: None,
        emotional_provenance: None,
        hdc_vector: None,
        confirmation_count: 0,
        distinct_contexts: Vec::new(),
        deprecated: false,
        balance: 1.0,
        frozen: false,
        balance_depleted_at: None,
        frozen_at: None,
        falsifier: None,
        catalytic_score: 0,
    }
}

/// Classify common rustc failure codes into semantic AntiKnowledge tags.
///
/// The returned tags are intentionally human-readable so future retrieval can
/// cluster related failures even when the exact compiler wording changes.
#[must_use]
pub fn classify_compilation_error(error: &str) -> Vec<String> {
    let mut tags = Vec::new();
    if error.contains("E0425") {
        tags.push("error:unresolved-name".to_string());
    }
    if error.contains("E0308") {
        tags.push("error:type-mismatch".to_string());
    }
    if error.contains("E0433") {
        tags.push("error:unresolved-import".to_string());
    }
    if error.contains("E0277") {
        tags.push("error:trait-not-satisfied".to_string());
    }
    tags
}

const ANTI_PATTERN_DUPLICATE_SIMILARITY_THRESHOLD: f64 = 0.45;

fn compilation_error_code_tags(error: &str) -> Vec<String> {
    let mut tags = Vec::new();
    if error.contains("E0425") {
        tags.push("error-code:E0425".to_string());
    }
    if error.contains("E0308") {
        tags.push("error-code:E0308".to_string());
    }
    if error.contains("E0433") {
        tags.push("error-code:E0433".to_string());
    }
    if error.contains("E0277") {
        tags.push("error-code:E0277".to_string());
    }
    tags
}

fn find_similar_anti_pattern_index(
    entries: &[KnowledgeEntry],
    candidate: &KnowledgeEntry,
) -> Option<usize> {
    let mut best_index: Option<usize> = None;
    let mut best_similarity = 0.0_f64;

    for (index, entry) in entries.iter().enumerate() {
        let similarity = anti_pattern_similarity(entry, candidate);
        if similarity >= ANTI_PATTERN_DUPLICATE_SIMILARITY_THRESHOLD && similarity > best_similarity
        {
            best_index = Some(index);
            best_similarity = similarity;
        }
    }

    best_index
}

fn anti_pattern_similarity(existing: &KnowledgeEntry, candidate: &KnowledgeEntry) -> f64 {
    if existing.kind != KnowledgeKind::AntiKnowledge
        || candidate.kind != KnowledgeKind::AntiKnowledge
    {
        return 0.0;
    }

    let Some(gate_tag) = candidate.tags.iter().find(|tag| tag.starts_with("gate:")) else {
        return 0.0;
    };
    if !entry_has_normalized_tag(existing, gate_tag) {
        return 0.0;
    }

    entry_similarity(existing, candidate)
}

fn entry_has_normalized_tag(entry: &KnowledgeEntry, tag: &str) -> bool {
    let normalized = normalize(tag);
    entry
        .tags
        .iter()
        .any(|entry_tag| normalize(entry_tag) == normalized)
}

fn reinforce_anti_pattern(existing: &mut KnowledgeEntry, candidate: &KnowledgeEntry) {
    existing.confidence = (existing.confidence + 0.1).clamp(0.6, 1.0);
    existing.confidence_weight = -existing.confidence;
    existing.confirmation_count = existing.confirmation_count.saturating_add(1);
    existing.half_life_days = KnowledgeKind::AntiKnowledge.default_half_life_days();

    if existing
        .source
        .as_deref()
        .is_none_or(|source| source.trim().is_empty())
    {
        existing.source.clone_from(&candidate.source);
    }
    if existing
        .refutation_evidence
        .as_deref()
        .is_none_or(|evidence| evidence.trim().is_empty())
    {
        existing
            .refutation_evidence
            .clone_from(&candidate.refutation_evidence);
    }

    if let Some(task_tag) = candidate
        .tags
        .iter()
        .find_map(|tag| tag.strip_prefix("task:"))
    {
        let task_tag = task_tag.trim();
        if !task_tag.is_empty()
            && !existing
                .distinct_contexts
                .iter()
                .any(|context| context.eq_ignore_ascii_case(task_tag))
        {
            existing.distinct_contexts.push(task_tag.to_string());
        }
    }

    if !candidate.source_episodes.is_empty() {
        let mut seen: HashSet<String> = existing.source_episodes.iter().cloned().collect();
        for source_episode in &candidate.source_episodes {
            if seen.insert(source_episode.clone()) {
                existing.source_episodes.push(source_episode.clone());
            }
        }
    }

    merge_tags(&mut existing.tags, &candidate.tags);
    KnowledgeStore::maybe_adjust_tier(existing);
}

fn merge_tags(existing: &mut Vec<String>, additional: &[String]) {
    let mut seen: HashSet<String> = existing.iter().cloned().collect();
    for tag in additional {
        if seen.insert(tag.clone()) {
            existing.push(tag.clone());
        }
    }
    existing.sort();
    existing.dedup();
}

fn truncate_snippet(content: &str, max_chars: usize) -> String {
    let mut chars = content.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_none() {
        content.to_string()
    } else {
        format!("{truncated}...")
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KnowledgeKind, KnowledgeTier};
    use chrono::Duration;
    use roko_core::extension::CamelTaintLevel;
    use roko_core::{PadVector, TaintLevel};
    use tempfile::TempDir;

    fn entry(
        kind: KnowledgeKind,
        id: &str,
        content: &str,
        tags: &[&str],
        confidence: f64,
        source_episodes: &[&str],
        created_at: DateTime<Utc>,
    ) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_owned(),
            kind,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: content.to_owned(),
            confidence,
            confidence_weight: confidence,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: source_episodes
                .iter()
                .map(|source| (*source).to_owned())
                .collect(),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            source_model: None,
            model_generality: 1.0,
            created_at,
            half_life_days: kind.default_half_life_days(),
            tier: KnowledgeTier::Consolidated,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,

            confirmation_count: 0,

            distinct_contexts: Vec::new(),

            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
        }
    }

    #[test]
    fn external_ingress_labels_survive_persistence_round_trip() {
        let temp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(temp.path().join("knowledge.jsonl"));
        let candidate = entry(
            KnowledgeKind::AntiKnowledge,
            "external-round-trip",
            "untrusted external observation",
            &["security"],
            0.9,
            &["episode-external"],
            Utc::now(),
        );

        store
            .ingest_with_source(vec![candidate], SourceChannel::ExternalApi)
            .expect("ingest external entry");

        let persisted = store.read_all().expect("read persisted entry");
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].source.as_deref(), Some("external-api"));
        assert_eq!(persisted[0].origin_taint, CamelTaintLevel::External);
        assert_eq!(persisted[0].classification, TaintLevel::Confidential);

        let raw = std::fs::read_to_string(store.path()).expect("read raw knowledge JSONL");
        assert!(raw.contains("\"origin_taint\":\"external\""));
        assert!(raw.contains("\"classification\":\"Confidential\""));
    }

    #[test]
    fn same_id_replay_can_raise_but_never_lower_security_labels() {
        let temp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(temp.path().join("knowledge.jsonl"));
        let original = entry(
            KnowledgeKind::AntiKnowledge,
            "monotonic-replay",
            "stable content",
            &["security"],
            0.9,
            &["episode-1"],
            Utc::now(),
        );
        store
            .ingest_with_source(vec![original], SourceChannel::GateVerdict)
            .expect("ingest trusted entry");

        let mut hostile_replay = entry(
            KnowledgeKind::AntiKnowledge,
            "monotonic-replay",
            "stable content",
            &["security"],
            0.9,
            &["episode-2"],
            Utc::now(),
        );
        hostile_replay.source = Some("user-external-import".to_string());
        hostile_replay.origin_taint = CamelTaintLevel::Trusted;
        hostile_replay.classification = TaintLevel::Public;
        store.add(hostile_replay).expect("replay external entry");

        let raised = store.read_all().expect("read raised label");
        assert_eq!(raised.len(), 1, "same-ID replay remains deduplicated");
        assert_eq!(raised[0].origin_taint, CamelTaintLevel::External);
        assert_eq!(raised[0].classification, TaintLevel::Confidential);

        let mut downgrade = raised[0].clone();
        downgrade.source = Some("manual-user".to_string());
        downgrade.origin_taint = CamelTaintLevel::Trusted;
        downgrade.classification = TaintLevel::Public;
        store.add(downgrade).expect("attempt lower-label replay");

        let retained = store.read_all().expect("read retained label");
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].origin_taint, CamelTaintLevel::External);
        assert_eq!(retained[0].classification, TaintLevel::Confidential);
    }

    #[test]
    fn extract_anti_pattern_from_failure_builds_anti_knowledge_entry() {
        let entry = extract_anti_pattern_from_failure(
            "task-42",
            "repair the compile failure in the Rust workspace",
            "compile",
            "error[E0425]: cannot find value `foo` in this scope\nerror[E0308]: mismatched types",
            Some("let bar = foo();"),
        );

        assert_eq!(entry.kind, KnowledgeKind::AntiKnowledge);
        assert_eq!(entry.tier, KnowledgeTier::Transient);
        assert!((entry.confidence - 0.6).abs() < f64::EPSILON);
        assert!((entry.confidence_weight + 0.6).abs() < f64::EPSILON);
        assert_eq!(entry.source.as_deref(), Some("bench-gate-failure"));
        assert!(entry.content.contains("Anti-pattern for task type"));
        assert!(entry.content.contains("Gate 'compile' failed with:"));
        assert!(entry.content.contains("Agent output snippet:"));
        assert!(entry.tags.contains(&"bench".to_string()));
        assert!(entry.tags.contains(&"gate:compile".to_string()));
        assert!(entry.tags.contains(&"task:task-42".to_string()));
        assert!(entry.tags.contains(&"error:unresolved-name".to_string()));
        assert!(entry.tags.contains(&"error:type-mismatch".to_string()));
        assert!(entry.tags.contains(&"error-code:E0425".to_string()));
        assert!(entry.tags.contains(&"error-code:E0308".to_string()));
    }

    #[test]
    fn classify_compilation_error_labels_common_codes() {
        let tags = classify_compilation_error(
            "error[E0425]: cannot find value `foo` in this scope\nerror[E0277]: trait bound not satisfied",
        );

        assert!(tags.contains(&"error:unresolved-name".to_string()));
        assert!(tags.contains(&"error:trait-not-satisfied".to_string()));
    }

    #[test]
    fn record_anti_pattern_from_failure_reinforces_existing_entry() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::for_roko_dir(tmp.path());
        let task_id = "task-42";
        let task_prompt = "repair the compile failure in the Rust workspace";
        let gate_name = "compile";
        let gate_error = "error[E0425]: cannot find value `foo` in this scope";
        let agent_output = Some("let bar = foo();");

        let initial = extract_anti_pattern_from_failure(
            task_id,
            task_prompt,
            gate_name,
            gate_error,
            agent_output,
        );

        store.add(initial.clone()).expect("seed anti knowledge");
        store
            .record_anti_pattern_from_failure(
                task_id,
                task_prompt,
                gate_name,
                gate_error,
                agent_output,
            )
            .expect("record repeated failure");

        let entries = store.read_all().expect("read entries");
        assert_eq!(entries.len(), 1);
        let updated = &entries[0];
        assert_eq!(updated.kind, KnowledgeKind::AntiKnowledge);
        assert!(updated.confidence > initial.confidence);
        assert!(updated.confidence_weight.is_sign_negative());
        assert_eq!(updated.confirmation_count, 1);
        assert!(updated.tags.contains(&"gate:compile".to_string()));
        assert!(updated.tags.contains(&"task:task-42".to_string()));
    }

    #[test]
    fn add_query_and_gc_roundtrip() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Rust async actors and memory stores",
                &["rust", "async"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add first");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k2",
                "Rust data pipelines",
                &["rust"],
                0.8,
                &["ep-b"],
                now - Duration::days(10),
            ))
            .expect("add second");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k3",
                "Completely unrelated note",
                &["misc"],
                0.01,
                &[],
                now,
            ))
            .expect("add third");

        let results = store.query("rust async", 2).expect("query");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "k1");
        assert_eq!(results[1].id, "k2");

        store.gc(DEFAULT_GC_MIN_CONFIDENCE).expect("gc");
        let all = store.read_all().expect("read after gc");
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|entry| entry.id != "k3"));
    }

    #[test]
    fn update_confidence_clamps_and_promotes_tier() {
        let tmp = TempDir::new().expect("tempdir");
        let mut store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let mut knowledge = entry(
            KnowledgeKind::Insight,
            "confidence-up",
            "Confidence reinforcement should promote validated knowledge.",
            &["confidence"],
            0.89,
            &["ep1"],
            Utc::now(),
        );
        knowledge.tier = KnowledgeTier::Working;
        store.add(knowledge).expect("add knowledge");

        assert!(
            store
                .update_confidence("confidence-up", 0.20)
                .expect("update confidence")
        );
        assert!(
            !store
                .update_confidence("missing", 0.20)
                .expect("missing update")
        );

        let all = store.read_all().expect("read all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].confidence, 1.0);
        assert_eq!(all[0].tier, KnowledgeTier::Consolidated);
    }

    #[test]
    fn record_usage_demotes_low_confidence_entry() {
        let tmp = TempDir::new().expect("tempdir");
        let mut store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let mut knowledge = entry(
            KnowledgeKind::Heuristic,
            "confidence-down",
            "Failed usage should reduce confidence.",
            &["confidence"],
            0.23,
            &["ep1"],
            Utc::now(),
        );
        knowledge.tier = KnowledgeTier::Persistent;
        store.add(knowledge).expect("add knowledge");

        store
            .record_usage("confidence-down", false)
            .expect("record usage");

        let all = store.read_all().expect("read all");
        assert!((all[0].confidence - 0.18).abs() < 1e-9);
        assert_eq!(all[0].tier, KnowledgeTier::Transient);
    }

    #[test]
    fn batch_record_usage_updates_once_and_shortens_weak_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let mut store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let mut weak = entry(
            KnowledgeKind::Warning,
            "weak",
            "Repeated misses should make this decay quickly.",
            &["confidence"],
            0.08,
            &["ep1"],
            Utc::now(),
        );
        weak.tier = KnowledgeTier::Working;
        weak.half_life_days = 30.0;
        let stable = entry(
            KnowledgeKind::Insight,
            "stable",
            "Successful usage should reinforce this entry.",
            &["confidence"],
            0.50,
            &["ep2"],
            Utc::now(),
        );
        store.add(weak).expect("add weak");
        store.add(stable).expect("add stable");

        let updated = store
            .batch_record_usage(&[
                ("weak".to_owned(), false),
                ("stable".to_owned(), true),
                ("missing".to_owned(), true),
            ])
            .expect("batch record usage");

        assert_eq!(updated, 2);
        let all = store.read_all().expect("read all");
        let weak = all.iter().find(|entry| entry.id == "weak").expect("weak");
        let stable = all
            .iter()
            .find(|entry| entry.id == "stable")
            .expect("stable");
        assert!((weak.confidence - 0.03).abs() < 1e-9);
        assert_eq!(weak.tier, KnowledgeTier::Transient);
        assert_eq!(weak.half_life_days, 1.0);
        assert!((stable.confidence - 0.52).abs() < 1e-9);
    }

    #[test]
    fn decay_reduces_old_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(30);

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "A durable heuristic",
                &["heuristic"],
                1.0,
                &["ep-a", "ep-b"],
                created_at,
            ))
            .expect("add");

        store.decay().expect("decay");
        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert!((all[0].confidence - 0.5).abs() < 0.05);
    }

    #[test]
    fn query_prefers_entries_validated_across_diverse_emotional_states() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        let mut high_diversity = entry(
            KnowledgeKind::Warning,
            "k-diverse",
            "Check rollback health before retrying a failed rollout",
            &["deploy", "rollback"],
            0.8,
            &["ep-a", "ep-b"],
            now,
        );
        high_diversity.emotional_provenance = Some(crate::EmotionalProvenance {
            average_pad: PadVector::new(-0.2, 0.3, 0.0),
            discovery_emotion: "negative_high_arousal".to_string(),
            validation_arc: Some(crate::ValidationArc::Redemptive),
            emotional_diversity: 1.0,
        });

        let mut low_diversity = entry(
            KnowledgeKind::Warning,
            "k-narrow",
            "Check rollback health before retrying a failed rollout after a database migration",
            &["deploy", "rollback"],
            0.8,
            &["ep-c", "ep-d"],
            now,
        );
        low_diversity.emotional_provenance = Some(crate::EmotionalProvenance {
            average_pad: PadVector::new(-0.2, 0.3, 0.0),
            discovery_emotion: "negative_high_arousal".to_string(),
            validation_arc: Some(crate::ValidationArc::Stable),
            emotional_diversity: 0.0,
        });

        store.add(low_diversity).expect("add narrow");
        store.add(high_diversity).expect("add diverse");

        let results = store
            .query("retry failed rollout rollback health", 2)
            .expect("query");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "k-diverse");
        assert_eq!(results[1].id, "k-narrow");
    }

    #[test]
    fn query_hits_expose_scoring_breakdown() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Rust async actors and memory stores",
                &["rust", "async"],
                0.9,
                &["ep-a", "ep-b"],
                now,
            ))
            .expect("add first");
        store
            .add(entry(
                KnowledgeKind::Warning,
                "k2",
                "Retry loops can amplify flaky async tests",
                &["testing"],
                0.8,
                &["ep-c"],
                now - Duration::days(10),
            ))
            .expect("add second");

        let hits = store.query_hits("rust async", 5).expect("query hits");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].entry.id, "k1");
        assert!(hits[0].total_score > QUERY_SCORE_FLOOR);
        assert!(hits[0].breakdown.keyword_score >= 2.0);
        assert!(hits[0].breakdown.effective_confidence > hits[0].entry.confidence);
        assert!(hits[0].breakdown.recency_factor > 0.9);
    }

    #[test]
    fn query_kind_hits_filter_by_kind() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Prefer small async state machines",
                &["async"],
                0.9,
                &["ep-a"],
                now,
            ))
            .expect("add insight");
        store
            .add(entry(
                KnowledgeKind::StrategyFragment,
                "k2",
                "Break async migrations into small compileable steps",
                &["async", "migration"],
                0.95,
                &["ep-b", "ep-c", "ep-d"],
                now,
            ))
            .expect("add strategy fragment");

        let hits = store
            .query_kind_hits("async migration", KnowledgeKind::StrategyFragment, 5)
            .expect("query kind hits");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "k2");
        assert_eq!(hits[0].entry.kind, KnowledgeKind::StrategyFragment);
    }

    #[test]
    fn query_similar_ranks_by_hamming_similarity() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        let mut exact = entry(
            KnowledgeKind::Insight,
            "k-exact",
            "Exact fingerprint match",
            &["fingerprint"],
            0.9,
            &["ep-a"],
            now,
        );
        exact.hdc_vector = Some(vec![0; HDC_VECTOR_BYTES]);

        let mut close = entry(
            KnowledgeKind::Insight,
            "k-close",
            "Close fingerprint match",
            &["fingerprint"],
            0.8,
            &["ep-b"],
            now,
        );
        let mut close_fp = vec![0; HDC_VECTOR_BYTES];
        close_fp[0] = 0b0000_0011;
        close.hdc_vector = Some(close_fp);

        let mut far = entry(
            KnowledgeKind::Insight,
            "k-far",
            "Far fingerprint match",
            &["fingerprint"],
            0.7,
            &["ep-c"],
            now,
        );
        far.hdc_vector = Some(vec![0xFF; HDC_VECTOR_BYTES]);

        store.ingest(vec![far, close, exact]).expect("ingest");

        let query = vec![0; HDC_VECTOR_BYTES];
        let hits = store.query_similar(&query, 3).expect("query similar");

        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].entry.id, "k-exact");
        assert_eq!(hits[1].entry.id, "k-close");
        assert_eq!(hits[2].entry.id, "k-far");
        assert!(hits[0].similarity > hits[1].similarity);
        assert!(hits[1].similarity > hits[2].similarity);
    }

    #[test]
    fn query_similar_rejects_invalid_fingerprint_length() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));

        let error = store
            .query_similar(&[0_u8; 16], 1)
            .expect_err("invalid fingerprint length should fail");

        assert!(
            error
                .to_string()
                .contains("knowledge fingerprints must be 1280 bytes")
        );
    }

    #[test]
    fn ingest_skips_duplicate_ids() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();
        let duplicate = entry(
            KnowledgeKind::Insight,
            "dup",
            "Keep one durable copy",
            &["dup"],
            0.8,
            &["ep-a"],
            now,
        );

        store
            .ingest(vec![duplicate.clone(), duplicate.clone()])
            .expect("ingest duplicates");
        store.add(duplicate).expect("add duplicate again");

        let all = store.read_all().expect("read all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "dup");
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn hdc_only_unrelated_entries_do_not_clear_query_floor() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Completely unrelated note about shell prompts",
                &["misc"],
                0.0,
                &["ep-a"],
                now,
            ))
            .expect("add unrelated");

        let hits = store
            .query_hits("database migrations", 5)
            .expect("query hits");
        assert!(hits.is_empty());
    }

    #[test]
    fn query_prefers_emotionally_reinforced_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        let mut neutral = entry(
            KnowledgeKind::Warning,
            "k-neutral",
            "Prefer the rollback path when rollout validation fails during a routine canary release",
            &["deploy", "rollback"],
            0.8,
            &["ep-a"],
            now,
        );
        neutral.emotional_provenance = Some(crate::EmotionalProvenance {
            average_pad: PadVector::new(-0.1, 0.2, 0.0),
            discovery_emotion: "neutral_mid_arousal".to_string(),
            validation_arc: Some(crate::ValidationArc::Stable),
            emotional_diversity: 0.0,
        });

        let mut reinforced = entry(
            KnowledgeKind::Warning,
            "k-reinforced",
            "Prefer the rollback path when rollout validation fails",
            &["deploy", "rollback"],
            0.8,
            &["ep-b"],
            now,
        );
        reinforced.emotional_tag = Some(roko_core::EmotionalTag::new(
            PadVector::new(-0.8, 0.4, 0.0),
            0.95,
            "rollback_failure",
            PadVector::new(-0.7, 0.3, 0.0),
        ));
        reinforced.emotional_provenance = Some(crate::EmotionalProvenance {
            average_pad: PadVector::new(-0.8, 0.4, 0.0),
            discovery_emotion: "negative_high_arousal".to_string(),
            validation_arc: Some(crate::ValidationArc::Redemptive),
            emotional_diversity: 1.0,
        });

        store.add(neutral).expect("add neutral");
        store.add(reinforced).expect("add reinforced");

        let results = store
            .query("rollback rollout validation failure", 2)
            .expect("query");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "k-reinforced");
    }

    #[test]
    fn decay_preserves_antiknowledge_confidence_floor() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(365);

        store
            .add(KnowledgeEntry {
                id: "anti-floor".to_owned(),
                kind: KnowledgeKind::AntiKnowledge,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "This previously successful pattern regressed badly.".to_owned(),
                confidence: 0.8,
                confidence_weight: -0.8,
                refuted_insight_id: Some("insight-1".to_owned()),
                refutation_evidence: Some("repeated gate failures".to_owned()),
                source_episodes: vec!["ep-a".to_owned()],
                tags: vec!["anti_knowledge".to_owned(), "regression".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at,
                half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
                tier: KnowledgeTier::Working,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
            })
            .expect("add anti knowledge");

        store.decay().expect("decay");
        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert!((all[0].confidence - ANTI_KNOWLEDGE_CONFIDENCE_FLOOR).abs() < f64::EPSILON);
    }

    #[test]
    fn decay_uses_kind_specific_half_lives() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(30);

        store
            .add(entry(
                KnowledgeKind::StrategyFragment,
                "strategy",
                "Reusable long-lived strategy fragment",
                &["strategy_fragment"],
                1.0,
                &[],
                created_at,
            ))
            .expect("add strategy fragment");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "insight",
                "Short-lived insight",
                &["insight"],
                1.0,
                &["ep-a", "ep-b"],
                created_at,
            ))
            .expect("add insight");
        store
            .add(entry(
                KnowledgeKind::Heuristic,
                "heuristic",
                "Mid-lived heuristic",
                &["heuristic"],
                1.0,
                &[],
                created_at,
            ))
            .expect("add heuristic");

        store.decay().expect("decay");
        let all = store.read_all().expect("read");
        let strategy = all
            .iter()
            .find(|entry| entry.id == "strategy")
            .expect("strategy");
        let insight = all
            .iter()
            .find(|entry| entry.id == "insight")
            .expect("insight");
        let heuristic = all
            .iter()
            .find(|entry| entry.id == "heuristic")
            .expect("heuristic");

        assert!(heuristic.confidence > insight.confidence);
        assert!(insight.confidence > strategy.confidence);
        assert!((insight.confidence - 0.5).abs() < 0.05);
        assert!((strategy.confidence - 0.22).abs() < 0.05);
        assert!((heuristic.confidence - 0.79).abs() < 0.05);
    }

    #[test]
    fn decay_drops_below_half_after_two_half_lives() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(60);

        store
            .add(entry(
                KnowledgeKind::Insight,
                "old-insight",
                "A stale but valid insight",
                &["insight"],
                1.0,
                &["ep-a", "ep-b"],
                created_at,
            ))
            .expect("add");

        store.decay().expect("decay");
        let all = store.read_all().expect("read");
        let confidence = all
            .iter()
            .find(|entry| entry.id == "old-insight")
            .expect("old insight")
            .confidence;
        assert!(confidence < 0.5);
        assert!((confidence - 0.25).abs() < 0.05);
    }

    #[test]
    fn confirmation_boost_retains_validated_entries_through_gc() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let created_at = Utc::now() - Duration::days(30);

        store
            .add(entry(
                KnowledgeKind::Insight,
                "single",
                "Single-source insight",
                &["insight"],
                0.4,
                &["ep-a"],
                created_at,
            ))
            .expect("add single");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "validated",
                "Validated insight",
                &["insight"],
                0.4,
                &["ep-a", "ep-b"],
                created_at,
            ))
            .expect("add validated");

        store.gc(0.5).expect("gc");
        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "validated");
    }

    #[test]
    fn gc_preserves_antiknowledge_even_below_threshold() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(KnowledgeEntry {
                id: "anti-gc".to_owned(),
                kind: KnowledgeKind::AntiKnowledge,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "This optimization path is deceptively harmful.".to_owned(),
                confidence: 0.01,
                confidence_weight: -0.4,
                refuted_insight_id: Some("insight-2".to_owned()),
                refutation_evidence: Some("caused repeated failures".to_owned()),
                source_episodes: vec!["ep-a".to_owned()],
                tags: vec!["anti_knowledge".to_owned(), "optimization".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
                tier: KnowledgeTier::Working,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
            })
            .expect("add anti knowledge");

        store.gc(0.95).expect("gc");
        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "anti-gc");
        assert!(
            (effective_confidence(&all[0]) - ANTI_KNOWLEDGE_CONFIDENCE_FLOOR).abs() < f64::EPSILON
        );
    }

    #[test]
    fn antiknowledge_halves_refuted_entry_confidence() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "insight-1",
                "A reusable insight",
                &["insight"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add original");
        store
            .add(KnowledgeEntry {
                id: "anti-1".to_owned(),
                kind: KnowledgeKind::AntiKnowledge,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "Previous insight insight-1 was wrong because it failed in practice."
                    .to_owned(),
                confidence: 0.9,
                confidence_weight: -0.9,
                refuted_insight_id: Some("insight-1".to_owned()),
                refutation_evidence: Some("it failed in practice".to_owned()),
                source_episodes: vec!["ep-b".to_owned()],
                tags: vec!["anti_knowledge".to_owned(), "insight".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
                tier: KnowledgeTier::Working,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
            })
            .expect("add anti knowledge");

        let all = store.read_all().expect("read");
        let original = all
            .iter()
            .find(|entry| entry.id == "insight-1")
            .expect("original");
        let anti = all.iter().find(|entry| entry.id == "anti-1").expect("anti");

        assert!((original.confidence - 0.5).abs() < f64::EPSILON);
        assert_eq!(anti.kind, KnowledgeKind::AntiKnowledge);
    }

    #[test]
    fn stats_aggregate_by_kind_and_age() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(KnowledgeEntry {
                id: "oldest".to_owned(),
                kind: KnowledgeKind::Insight,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "first".to_owned(),
                confidence: 0.8,
                confidence_weight: 0.8,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: Vec::new(),
                tags: Vec::new(),
                source_model: None,
                model_generality: 1.0,
                created_at: now - Duration::days(3),
                half_life_days: KnowledgeKind::Insight.default_half_life_days(),
                tier: KnowledgeTier::Consolidated,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
            })
            .expect("add oldest");
        store
            .add(KnowledgeEntry {
                id: "middle".to_owned(),
                kind: KnowledgeKind::StrategyFragment,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "second".to_owned(),
                confidence: 0.6,
                confidence_weight: 0.6,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: Vec::new(),
                tags: Vec::new(),
                source_model: None,
                model_generality: 1.0,
                created_at: now - Duration::days(1),
                half_life_days: KnowledgeKind::StrategyFragment.default_half_life_days(),
                tier: KnowledgeTier::Consolidated,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
            })
            .expect("add middle");
        store
            .add(KnowledgeEntry {
                id: "newest".to_owned(),
                kind: KnowledgeKind::Insight,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "third".to_owned(),
                confidence: 1.0,
                confidence_weight: 1.0,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: Vec::new(),
                tags: Vec::new(),
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::Insight.default_half_life_days(),
                tier: KnowledgeTier::Consolidated,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
            })
            .expect("add newest");

        let stats = store.stats().expect("stats");
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.kind_counts.get("insight"), Some(&2));
        assert_eq!(stats.kind_counts.get("strategy_fragment"), Some(&1));
        assert!((stats.average_confidence.expect("average") - 0.8).abs() < f64::EPSILON);
        assert_eq!(
            stats.oldest_entry.as_ref().map(|entry| entry.id.as_str()),
            Some("oldest")
        );
        assert_eq!(
            stats.newest_entry.as_ref().map(|entry| entry.id.as_str()),
            Some("newest")
        );
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn memory_index_search_prefers_matching_content() {
        let now = Utc::now();
        let index = MemoryIndex::from_entries(vec![
            entry(
                KnowledgeKind::Insight,
                "k1",
                "rust async memory retrieval",
                &["rust", "memory"],
                1.0,
                &["ep-a"],
                now,
            ),
            entry(
                KnowledgeKind::Insight,
                "k2",
                "postgres maintenance routine",
                &["db"],
                0.9,
                &[],
                now,
            ),
        ]);

        assert_eq!(index.len(), 2);
        assert!(!index.is_empty());

        let hits = index.search("rust async memory retrieval", 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "k1");
        assert!(hits[0].similarity >= 0.99);
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn knowledge_store_builds_memory_index() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "semantic retrieval over durable knowledge",
                &["memory"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add first");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k2",
                "completely unrelated topic",
                &["misc"],
                0.8,
                &[],
                now,
            ))
            .expect("add second");

        let index = store.memory_index().expect("index");
        assert_eq!(index.entries().len(), 2);
        let hits = index.search("semantic retrieval over durable knowledge", 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.id, "k1");
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn causal_links_match_queries_by_cause_and_effect() {
        let now = Utc::now();
        let index = MemoryIndex::from_entries(vec![
            entry(
                KnowledgeKind::CausalLink,
                "k1",
                "high complexity -> more review",
                &[
                    "cause:high complexity",
                    "effect:more review",
                    "domain:coding",
                ],
                0.9,
                &["ep-a"],
                now,
            ),
            entry(
                KnowledgeKind::Insight,
                "k2",
                "postgres vacuum keeps tables healthy",
                &["postgres"],
                0.9,
                &["ep-b"],
                now,
            ),
        ]);

        let cause_hits = index.search("high complexity", 1);
        assert_eq!(cause_hits.len(), 1);
        assert_eq!(cause_hits[0].entry.id, "k1");

        let effect_hits = index.search("more review", 1);
        assert_eq!(effect_hits.len(), 1);
        assert_eq!(effect_hits[0].entry.id, "k1");
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn ingest_populates_hdc_vector_when_feature_is_enabled() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "semantic retrieval over durable knowledge",
                &["memory"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add entry");

        let all = store.read_all().expect("read");
        let vector = all[0].hdc_vector.as_ref().expect("persisted hdc vector");
        assert_eq!(vector.len(), HDC_VECTOR_BYTES);
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn backfill_hdc_vectors_populates_missing_and_is_idempotent() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        // Write two entries directly without HDC vectors (bypassing ingest normalization)
        // by using rewrite_all on raw entries.
        let mut e1 = entry(
            KnowledgeKind::Insight,
            "k1",
            "distributed tracing improves observability",
            &["tracing", "observability"],
            0.9,
            &["ep-a"],
            now,
        );
        let mut e2 = entry(
            KnowledgeKind::Heuristic,
            "k2",
            "prefer idempotent operations in distributed systems",
            &["distributed", "idempotent"],
            0.8,
            &["ep-b"],
            now,
        );
        // Ensure hdc_vector is absent so backfill has work to do.
        e1.hdc_vector = None;
        e2.hdc_vector = None;
        store.rewrite_all(&[e1, e2]).expect("write raw entries");

        // First backfill: both entries lack vectors, so 2 must be updated.
        let changed = store
            .backfill_hdc_vectors()
            .expect("backfill_hdc_vectors first pass");
        assert_eq!(changed, 2, "both entries should receive HDC vectors");

        // Verify vectors were persisted with the correct byte length.
        let all = store.read_all().expect("read after backfill");
        for e in &all {
            let vec = e.hdc_vector.as_ref().expect("hdc_vector must be set");
            assert_eq!(
                vec.len(),
                HDC_VECTOR_BYTES,
                "entry {} must have {HDC_VECTOR_BYTES}-byte HDC vector",
                e.id
            );
        }

        // Second backfill: all entries already have valid vectors — 0 changes.
        let changed_again = store
            .backfill_hdc_vectors()
            .expect("backfill_hdc_vectors second pass");
        assert_eq!(changed_again, 0, "backfill must be idempotent");
    }

    // ── Confirmation detection tests ─────────────────────────────────

    #[test]
    fn entries_are_similar_detects_tag_and_keyword_overlap() {
        let now = Utc::now();
        let existing = entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful for concurrent pipelines",
            &["rust", "async", "concurrency"],
            1.0,
            &["ep-a"],
            now,
        );
        let similar = entry(
            KnowledgeKind::Insight,
            "k2",
            "Rust async runtime handles concurrent execution well",
            &["rust", "async"],
            0.9,
            &["ep-b"],
            now,
        );
        let unrelated = entry(
            KnowledgeKind::Insight,
            "k3",
            "PostgreSQL requires VACUUM for dead tuple cleanup",
            &["postgres", "maintenance"],
            0.8,
            &["ep-c"],
            now,
        );

        assert!(entries_are_similar(&existing, &similar));
        assert!(!entries_are_similar(&existing, &unrelated));
    }

    #[test]
    fn entries_are_similar_requires_minimum_keyword_overlap() {
        let now = Utc::now();
        let existing = entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful",
            &["rust"],
            1.0,
            &["ep-a"],
            now,
        );
        // Shares the tag "rust" but only one keyword overlap ("rust").
        let one_keyword = entry(
            KnowledgeKind::Insight,
            "k2",
            "Rust borrow checker prevents data races",
            &["rust"],
            0.9,
            &["ep-b"],
            now,
        );

        // Meets MIN_TAG_OVERLAP but not MIN_KEYWORD_OVERLAP.
        assert!(!entries_are_similar(&existing, &one_keyword));
    }

    #[test]
    fn entries_are_similar_skips_antiknowledge() {
        let now = Utc::now();
        let existing = entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful for concurrent pipelines",
            &["rust", "async"],
            1.0,
            &["ep-a"],
            now,
        );
        let anti = KnowledgeEntry {
            id: "anti-1".to_owned(),
            kind: KnowledgeKind::AntiKnowledge,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Rust async actors are not suitable for all concurrent pipelines".to_owned(),
            confidence: 0.9,
            confidence_weight: -0.9,
            refuted_insight_id: Some("k1".to_owned()),
            refutation_evidence: Some("test".to_owned()),
            source_episodes: vec!["ep-b".to_owned()],
            tags: vec!["rust".to_owned(), "async".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: now,
            half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,

            confirmation_count: 0,

            distinct_contexts: Vec::new(),

            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
        };

        assert!(!entries_are_similar(&existing, &anti));
    }

    #[test]
    fn detect_confirmations_finds_similar_entries() {
        let now = Utc::now();
        let existing = vec![entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful for concurrent pipelines",
            &["rust", "async"],
            1.0,
            &["ep-a"],
            now,
        )];
        let new_entries = vec![entry(
            KnowledgeKind::Insight,
            "k2",
            "Rust async runtime handles concurrent execution well",
            &["rust", "async"],
            0.9,
            &["ep-b"],
            now,
        )];

        let confirmations = detect_confirmations(&existing, &new_entries);
        assert_eq!(confirmations.len(), 1);
        assert_eq!(confirmations[0].confirmed_entry_id, "k1");
        assert_eq!(confirmations[0].confirming_entry_id, "k2");
        assert!(
            confirmations[0]
                .source_episodes
                .contains(&"ep-a".to_owned())
        );
        assert!(
            confirmations[0]
                .source_episodes
                .contains(&"ep-b".to_owned())
        );
    }

    #[test]
    fn detect_confirmations_skips_unrelated_entries() {
        let now = Utc::now();
        let existing = vec![entry(
            KnowledgeKind::Insight,
            "k1",
            "Rust async actors are useful for concurrent pipelines",
            &["rust", "async"],
            1.0,
            &["ep-a"],
            now,
        )];
        let new_entries = vec![entry(
            KnowledgeKind::Insight,
            "k3",
            "PostgreSQL requires VACUUM for dead tuple cleanup",
            &["postgres", "maintenance"],
            0.8,
            &["ep-c"],
            now,
        )];

        let confirmations = detect_confirmations(&existing, &new_entries);
        assert!(confirmations.is_empty());
    }

    #[test]
    fn ingest_writes_confirmation_records_for_similar_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        // Add first entry.
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Rust async actors are useful for concurrent pipelines",
                &["rust", "async"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add first");

        // No confirmations after first entry.
        let records = store.read_confirmations().expect("read confirmations");
        assert!(records.is_empty());

        // Add a similar entry.
        store
            .add(entry(
                KnowledgeKind::Insight,
                "k2",
                "Rust async runtime handles concurrent execution well",
                &["rust", "async"],
                0.9,
                &["ep-b"],
                now,
            ))
            .expect("add similar");

        // Now there should be a confirmation record.
        let records = store.read_confirmations().expect("read confirmations");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].confirmed_entry_id, "k1");
        assert_eq!(records[0].confirming_entry_id, "k2");
        assert!(records[0].source_episodes.contains(&"ep-a".to_owned()));
        assert!(records[0].source_episodes.contains(&"ep-b".to_owned()));
    }

    #[test]
    fn ingest_does_not_write_confirmations_for_unrelated_entries() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "Rust async actors are useful for concurrent pipelines",
                &["rust", "async"],
                1.0,
                &["ep-a"],
                now,
            ))
            .expect("add first");

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k3",
                "PostgreSQL requires VACUUM for dead tuple cleanup",
                &["postgres", "maintenance"],
                0.8,
                &["ep-c"],
                now,
            ))
            .expect("add unrelated");

        let records = store.read_confirmations().expect("read confirmations");
        assert!(records.is_empty());
    }

    #[test]
    fn confirmations_path_is_sibling_of_knowledge_path() {
        let store = KnowledgeStore::new("/some/path/neuro/knowledge.jsonl");
        assert_eq!(
            store.confirmations_path(),
            Path::new("/some/path/neuro/knowledge-confirmations.jsonl")
        );
    }

    #[test]
    fn ingest_promotes_high_support_entries_to_longer_tiers() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(KnowledgeEntry {
                id: "tiered".to_owned(),
                kind: KnowledgeKind::Insight,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "Repeatedly validated insight".to_owned(),
                confidence: 0.92,
                confidence_weight: 0.92,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: vec!["ep-a".to_owned(), "ep-b".to_owned(), "ep-c".to_owned()],
                tags: vec!["tier".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::Insight.default_half_life_days(),
                tier: KnowledgeTier::Transient,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
            })
            .expect("add tiered");

        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].tier, KnowledgeTier::Consolidated);
    }

    #[test]
    fn ingest_keeps_stronger_explicit_tiers() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(KnowledgeEntry {
                id: "persistent".to_owned(),
                kind: KnowledgeKind::StrategyFragment,
                source: None,
                origin_taint: Default::default(),
                classification: Default::default(),
                content: "A durable playbook fragment".to_owned(),
                confidence: 0.6,
                confidence_weight: 0.6,
                refuted_insight_id: None,
                refutation_evidence: None,
                source_episodes: vec!["ep-a".to_owned()],
                tags: vec!["strategy".to_owned()],
                source_model: None,
                model_generality: 1.0,
                created_at: now,
                half_life_days: KnowledgeKind::StrategyFragment.default_half_life_days(),
                tier: KnowledgeTier::Persistent,
                emotional_tag: None,
                emotional_provenance: None,
                hdc_vector: None,

                confirmation_count: 0,

                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                balance_depleted_at: None,
                frozen_at: None,
                falsifier: None,
                catalytic_score: 0,
            })
            .expect("add persistent");

        let all = store.read_all().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].tier, KnowledgeTier::Persistent);
    }

    #[test]
    fn stats_includes_tier_and_source_counts() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        // Use low confidence so normalize_entry_tier does not auto-promote.
        let mut e1 = entry(
            KnowledgeKind::Insight,
            "k1",
            "something useful",
            &["rust"],
            0.5,
            &["ep-a"],
            now,
        );
        e1.tier = KnowledgeTier::Working;
        e1.source = Some("local".to_owned());

        let mut e2 = entry(
            KnowledgeKind::AntiKnowledge,
            "k2",
            "do not retry on 5xx",
            &["http"],
            0.5,
            &["ep-b"],
            now,
        );
        e2.tier = KnowledgeTier::Working;

        store.add(e1).expect("add");
        store.add(e2).expect("add anti");

        let stats = store.stats().expect("stats");
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.anti_knowledge_count, 1);
        assert_eq!(stats.tier_counts.get("working"), Some(&2));
        assert_eq!(stats.source_counts.get("local"), Some(&1));
    }

    #[test]
    fn export_import_roundtrip_with_confidence_discount() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        let mut e = entry(
            KnowledgeKind::Insight,
            "k1",
            "important heuristic",
            &["rust"],
            0.5,
            &["ep-a"],
            now,
        );
        e.tier = KnowledgeTier::Consolidated;
        store.add(e).expect("add");

        // Export.
        let backup_path = tmp.path().join("backup.jsonl");
        let filter = ExportFilter::default();
        let count = store.export(&backup_path, &filter).expect("export");
        assert_eq!(count, 1);
        assert!(backup_path.exists());

        // Import into a fresh store.
        let store2 = KnowledgeStore::new(tmp.path().join("neuro2").join("knowledge.jsonl"));
        let options = ImportOptions {
            confidence_discount: 0.85,
            reset_tier: true,
            source_label: "backup-test".to_owned(),
            ..Default::default()
        };
        let imported = store2.import(&backup_path, &options).expect("import");
        assert_eq!(imported.imported, 1);

        let all = store2.read_all().expect("read");
        assert_eq!(all.len(), 1);
        // Confidence should be discounted: 0.5 * 0.85 = 0.425.
        assert!((all[0].confidence - 0.425).abs() < 0.01);
        // Tier should be reset to Transient (low confidence won't trigger promotion).
        assert_eq!(all[0].tier, KnowledgeTier::Transient);
        // Source label should be recorded.
        assert_eq!(all[0].source.as_deref(), Some("backup-test"));
    }

    #[test]
    fn export_filter_by_kind_and_confidence() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "high confidence insight",
                &["rust"],
                0.9,
                &["ep-a"],
                now,
            ))
            .expect("add");
        store
            .add(entry(
                KnowledgeKind::Warning,
                "k2",
                "low confidence warning",
                &["rust"],
                0.2,
                &["ep-b"],
                now,
            ))
            .expect("add");

        let backup_path = tmp.path().join("filtered.jsonl");
        let filter = ExportFilter {
            kinds: Some(vec![KnowledgeKind::Insight]),
            min_confidence: Some(0.5),
            ..Default::default()
        };
        let count = store.export(&backup_path, &filter).expect("export");
        assert_eq!(count, 1);
    }

    #[test]
    fn import_rejects_unsupported_version() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));

        let bad_backup = tmp.path().join("bad.jsonl");
        let header = BackupHeader {
            version: 99,
            created_at: Utc::now(),
            entry_count: 0,
            source_path: "test".to_owned(),
            merkle_root: String::new(),
        };
        std::fs::write(&bad_backup, serde_json::to_string(&header).unwrap() + "\n").unwrap();

        let result = store.import(&bad_backup, &ImportOptions::default());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unsupported backup version")
        );
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn anti_knowledge_check_rejects_near_duplicate() {
        // When a new entry has very high HDC similarity to an existing
        // AntiKnowledge entry, it should be filtered out.
        let anti = KnowledgeEntry {
            id: "anti-1".to_owned(),
            kind: KnowledgeKind::AntiKnowledge,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Never retry failed HTTP 5xx requests without backoff".to_owned(),
            confidence: 0.9,
            confidence_weight: 1.0,
            refuted_insight_id: Some("old-insight".to_owned()),
            refutation_evidence: Some("caused cascading failures".to_owned()),
            source_episodes: vec!["ep-a".to_owned()],
            tags: vec!["http".to_owned(), "retry".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
        };

        // A near-identical entry that should be rejected.
        let duplicate = KnowledgeEntry {
            id: "new-1".to_owned(),
            kind: KnowledgeKind::Insight,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Never retry failed HTTP 5xx requests without backoff".to_owned(),
            confidence: 0.8,
            confidence_weight: 1.0,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: vec!["ep-b".to_owned()],
            tags: vec!["http".to_owned(), "retry".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::Insight.default_half_life_days(),
            tier: KnowledgeTier::Transient,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
        };

        // An unrelated entry that should pass through.
        let unrelated = KnowledgeEntry {
            id: "new-2".to_owned(),
            kind: KnowledgeKind::Insight,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "PostgreSQL requires regular VACUUM for performance".to_owned(),
            confidence: 0.9,
            confidence_weight: 1.0,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: vec!["ep-c".to_owned()],
            tags: vec!["postgres".to_owned(), "maintenance".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::Insight.default_half_life_days(),
            tier: KnowledgeTier::Transient,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
        };

        let existing = vec![anti];
        let new_entries = prepare_entries_for_ingest(vec![duplicate, unrelated]);

        let result = check_against_anti_knowledge(new_entries, &existing);
        // The near-duplicate should be rejected, leaving only the unrelated entry.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "new-2");
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn anti_knowledge_check_passes_antiknowledge_entries_through() {
        // AntiKnowledge entries themselves should not be blocked by existing
        // AntiKnowledge.
        let existing_anti = KnowledgeEntry {
            id: "anti-1".to_owned(),
            kind: KnowledgeKind::AntiKnowledge,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Never retry failed HTTP 5xx requests without backoff".to_owned(),
            confidence: 0.9,
            confidence_weight: 1.0,
            refuted_insight_id: Some("old".to_owned()),
            refutation_evidence: None,
            source_episodes: vec!["ep-a".to_owned()],
            tags: vec!["http".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
        };

        let new_anti = KnowledgeEntry {
            id: "anti-2".to_owned(),
            kind: KnowledgeKind::AntiKnowledge,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: "Never retry failed HTTP 5xx requests without backoff -- updated".to_owned(),
            confidence: 0.95,
            confidence_weight: 1.0,
            refuted_insight_id: Some("other".to_owned()),
            refutation_evidence: None,
            source_episodes: vec!["ep-b".to_owned()],
            tags: vec!["http".to_owned()],
            source_model: None,
            model_generality: 1.0,
            created_at: Utc::now(),
            half_life_days: KnowledgeKind::AntiKnowledge.default_half_life_days(),
            tier: KnowledgeTier::Working,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
        };

        let existing = vec![existing_anti];
        let new_entries = prepare_entries_for_ingest(vec![new_anti]);
        let result = check_against_anti_knowledge(new_entries, &existing);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "anti-2");
    }

    // -----------------------------------------------------------------------
    // NEURO-10: Demurrage balance model and reinforcement signals
    // -----------------------------------------------------------------------

    #[test]
    fn reinforcement_bumps_balance() {
        let mut e = KnowledgeEntry {
            balance: 1.0,
            ..KnowledgeEntry::default()
        };
        let before = e.balance;
        e.reinforce(crate::ReinforcementSignal::Retrieved, 0.0);
        assert!(
            e.balance > before,
            "balance should increase after reinforcement"
        );

        // Novelty amplifies the bump.
        let mid = e.balance;
        e.reinforce(crate::ReinforcementSignal::Retrieved, 1.0);
        let bump_with_novelty = e.balance - mid;
        // Reset and do without novelty.
        e.balance = mid;
        e.reinforce(crate::ReinforcementSignal::Retrieved, 0.0);
        let bump_without_novelty = e.balance - mid;
        assert!(bump_with_novelty > bump_without_novelty);
    }

    #[test]
    fn balance_capped_at_five() {
        let mut e = KnowledgeEntry {
            balance: 4.95,
            ..KnowledgeEntry::default()
        };
        e.reinforce(crate::ReinforcementSignal::Surprised, 1.0);
        assert!(e.balance <= 5.0, "balance must not exceed 5.0");
    }

    #[test]
    fn demurrage_reduces_balance() {
        let mut e = KnowledgeEntry {
            balance: 1.0,
            ..KnowledgeEntry::default()
        };
        e.apply_demurrage(100.0);
        assert!(e.balance < 1.0, "demurrage should reduce balance");
        assert!(e.balance >= 0.0, "balance must not go negative");
    }

    #[test]
    fn demurrage_does_not_go_negative() {
        let mut e = KnowledgeEntry {
            balance: 0.01,
            ..KnowledgeEntry::default()
        };
        e.apply_demurrage(1_000_000.0);
        assert_eq!(e.balance, 0.0);
    }

    #[test]
    fn freshness_combines_balance_and_decay() {
        let now = Utc::now();
        let old = now - Duration::hours(24 * 30); // 30 days
        let mut e = KnowledgeEntry {
            balance: 1.0,
            half_life_days: 30.0,
            created_at: old,
            ..KnowledgeEntry::default()
        };
        let fresh_high = e.freshness(now);
        e.balance = 0.1;
        let fresh_low = e.freshness(now);
        assert!(fresh_high > fresh_low, "higher balance => higher freshness");
    }

    #[test]
    fn reinforcement_signal_base_values_positive() {
        for signal in &[
            crate::ReinforcementSignal::Retrieved,
            crate::ReinforcementSignal::Cited,
            crate::ReinforcementSignal::Gated,
            crate::ReinforcementSignal::Surprised,
            crate::ReinforcementSignal::AgentQuoted,
        ] {
            assert!(
                signal.base_value() > 0.0,
                "{:?} must have positive base_value",
                signal
            );
        }
    }

    // -----------------------------------------------------------------------
    // NEURO-10: Balance/freshness influence on query scoring
    // -----------------------------------------------------------------------

    /// Two entries with equal topic relevance, confidence, and recency: the one
    /// with higher balance should rank first because of the balance/freshness boost.
    #[test]
    fn query_prefers_balance_reinforced_entries() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let now = Utc::now();

        // Use distinct tags and unique content to prevent the confirmation-detection
        // path from running (entries_are_similar fires on shared tags + keywords).
        // Both entries match the query but are distinct enough to not confirm each other.
        let low_balance = {
            let mut e = entry(
                KnowledgeKind::Insight,
                "low-balance",
                "Run deploy jobs inside the integration gating pipeline",
                &["deploy-gate", "integration"],
                0.8,
                &["ep-x"],
                now,
            );
            e.balance = 0.0; // zero: no reinforcement history
            e
        };

        let high_balance = {
            let mut e = entry(
                KnowledgeKind::Insight,
                "high-balance",
                "Always validate deploy artifacts in the gating pipeline",
                &["deploy-validate", "pipeline"],
                0.8,
                &["ep-y"],
                now,
            );
            e.balance = 3.0; // reinforced: should get the balance/freshness boost
            e
        };

        store.add(low_balance).unwrap();
        store.add(high_balance).unwrap();

        let hits = store
            .query_hits("deploy gating pipeline", 2)
            .expect("query_hits");
        assert_eq!(hits.len(), 2, "both entries should score above the floor");

        assert_eq!(
            hits[0].entry.id, "high-balance",
            "reinforced (high-balance) entry must rank first"
        );
        assert!(
            hits[0].breakdown.balance_freshness_boost > hits[1].breakdown.balance_freshness_boost,
            "high-balance entry must have a larger balance_freshness_boost in the breakdown"
        );
    }

    #[test]
    fn store_reinforce_entry() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let mut e = entry(
            KnowledgeKind::Insight,
            "reinforce-me",
            "test entry",
            &["test"],
            0.8,
            &["ep1"],
            Utc::now(),
        );
        e.balance = 0.5;
        store.add(e).unwrap();

        let found = store
            .reinforce_entry("reinforce-me", crate::ReinforcementSignal::Gated, 0.2)
            .unwrap();
        assert!(found);

        let entries = store.read_all().unwrap();
        let updated = entries.iter().find(|e| e.id == "reinforce-me").unwrap();
        assert!(updated.balance > 0.5, "balance should have been bumped");
    }

    #[test]
    fn store_apply_demurrage() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let mut e = entry(
            KnowledgeKind::Insight,
            "demurrage-test",
            "test entry",
            &["test"],
            0.8,
            &["ep1"],
            Utc::now() - Duration::hours(100),
        );
        e.balance = 1.0;
        store.add(e).unwrap();

        let taxed = store.apply_demurrage().unwrap();
        assert_eq!(taxed, 1);

        let entries = store.read_all().unwrap();
        let updated = entries.iter().find(|e| e.id == "demurrage-test").unwrap();
        assert!(updated.balance < 1.0);
    }

    // -----------------------------------------------------------------------
    // NEURO-11: Cold-tier freeze/thaw
    // -----------------------------------------------------------------------

    #[test]
    fn freeze_and_thaw_entry() {
        let mut e = KnowledgeEntry {
            balance: 0.01,
            ..KnowledgeEntry::default()
        };
        assert!(!e.frozen);
        e.freeze();
        assert!(e.frozen);
        e.thaw(0.3);
        assert!(!e.frozen);
        assert!((e.balance - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn frozen_entries_excluded_from_hot_queries() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let mut e = entry(
            KnowledgeKind::Insight,
            "frozen-entry",
            "important knowledge about testing",
            &["testing"],
            0.8,
            &["ep1"],
            Utc::now(),
        );
        e.frozen = true;
        store.add(e).unwrap();

        let results = store.query("testing", 10).unwrap();
        assert!(
            results.is_empty(),
            "frozen entries should not appear in hot queries"
        );

        let cold = store.query_cold(10).unwrap();
        assert_eq!(
            cold.len(),
            1,
            "frozen entries should appear in cold queries"
        );
        assert_eq!(cold[0].id, "frozen-entry");
    }

    #[test]
    fn gc_with_freeze_freezes_low_confidence_entries() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());

        let low = entry(
            KnowledgeKind::Insight,
            "low-conf",
            "fading knowledge",
            &["test"],
            0.01,
            &["ep1"],
            Utc::now(),
        );
        store.add(low).unwrap();

        let removed = store.gc_with_freeze(0.05).unwrap();
        assert_eq!(removed, 0, "entry should be frozen, not removed");

        let entries = store.read_all().unwrap();
        let frozen_entry = entries.iter().find(|e| e.id == "low-conf").unwrap();
        assert!(frozen_entry.frozen, "entry should have been frozen");
    }

    #[test]
    fn gc_with_freeze_removes_already_frozen_below_threshold() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());

        // Already frozen entry below confidence threshold.
        let mut frozen = entry(
            KnowledgeKind::Insight,
            "already-frozen",
            "old frozen knowledge",
            &["test"],
            0.01,
            &["ep1"],
            Utc::now(),
        );
        frozen.frozen = true;
        store.add(frozen).unwrap();

        let removed = store.gc_with_freeze(0.05).unwrap();
        assert_eq!(
            removed, 1,
            "already-frozen entry below threshold should be permanently removed"
        );
    }

    #[test]
    fn store_thaw_entry() {
        let dir = TempDir::new().unwrap();
        let store = KnowledgeStore::for_roko_dir(dir.path());
        let mut e = entry(
            KnowledgeKind::Insight,
            "thaw-me",
            "frozen knowledge",
            &["test"],
            0.8,
            &["ep1"],
            Utc::now(),
        );
        e.frozen = true;
        e.balance = 0.0;
        store.add(e).unwrap();

        let thawed = store.thaw_entry("thaw-me", 0.3).unwrap();
        assert!(thawed);

        let entries = store.read_all().unwrap();
        let updated = entries.iter().find(|e| e.id == "thaw-me").unwrap();
        assert!(!updated.frozen);
        assert!((updated.balance - 0.3).abs() < f64::EPSILON);
    }
}

#[cfg(test)]
mod anti_pattern_tests {
    use super::*;
    use crate::{KnowledgeKind, KnowledgeTier};
    use chrono::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_extract_creates_anti_knowledge() {
        let entry = extract_anti_pattern_from_failure(
            "task-1",
            "Implement add function",
            "compile",
            "error[E0425]: cannot find value `x` in this scope",
            Some("fn add(a: i32, b: i32) -> i32 { x + y }"),
        );

        assert_eq!(entry.kind, KnowledgeKind::AntiKnowledge);
        assert_eq!(entry.tier, KnowledgeTier::Transient);
        assert!(entry.content.contains("compile"));
        assert!(entry.content.contains("E0425"));
        assert!(entry.tags.contains(&"gate:compile".to_string()));
        assert!(entry.tags.contains(&"task:task-1".to_string()));
        assert!(entry.confidence > 0.0 && entry.confidence <= 1.0);
    }

    #[test]
    fn test_extract_without_agent_output() {
        let entry = extract_anti_pattern_from_failure(
            "task-2",
            "Fix imports",
            "test",
            "test failed: expected true, got false",
            None,
        );

        assert_eq!(entry.kind, KnowledgeKind::AntiKnowledge);
        assert!(entry.content.contains("test"));
        assert!(!entry.content.contains("Agent output"));
    }

    #[test]
    fn test_extract_tags_include_error_codes() {
        let entry = extract_anti_pattern_from_failure(
            "task-3",
            "Type error task",
            "compile",
            "error[E0308]: mismatched types",
            None,
        );

        assert!(entry.tags.iter().any(|tag| tag.starts_with("error:")));
    }

    #[test]
    fn test_anti_pattern_is_queryable() {
        let dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(dir.path().join("knowledge.jsonl"));

        let entry = extract_anti_pattern_from_failure(
            "task-4",
            "Implement iterator",
            "compile",
            "error[E0277]: trait bound not satisfied",
            None,
        );

        store.add(entry).unwrap();

        let results = store
            .query_kind("Implement iterator", KnowledgeKind::AntiKnowledge, 5)
            .unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].kind, KnowledgeKind::AntiKnowledge);
        assert!(results[0].content.contains("E0277"));
    }

    // ── Merkle root tests (E43-T01) ───────────────────────────────────

    fn entry(
        kind: KnowledgeKind,
        id: &str,
        content: &str,
        tags: &[&str],
        confidence: f64,
        source_episodes: &[&str],
        created_at: DateTime<Utc>,
    ) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_owned(),
            kind,
            source: None,
            origin_taint: Default::default(),
            classification: Default::default(),
            content: content.to_owned(),
            confidence,
            confidence_weight: confidence,
            refuted_insight_id: None,
            refutation_evidence: None,
            source_episodes: source_episodes
                .iter()
                .map(|source| (*source).to_owned())
                .collect(),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            source_model: None,
            model_generality: 1.0,
            created_at,
            half_life_days: kind.default_half_life_days(),
            tier: KnowledgeTier::Consolidated,
            emotional_tag: None,
            emotional_provenance: None,
            hdc_vector: None,
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            deprecated: false,
            balance: 1.0,
            frozen: false,
            balance_depleted_at: None,
            frozen_at: None,
            falsifier: None,
            catalytic_score: 0,
        }
    }

    #[test]
    fn export_includes_merkle_root_in_header() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("neuro").join("knowledge.jsonl"));
        let now = Utc::now();

        store
            .add(entry(
                KnowledgeKind::Insight,
                "k1",
                "first entry",
                &["rust"],
                0.8,
                &["ep-a"],
                now,
            ))
            .expect("add");
        store
            .add(entry(
                KnowledgeKind::Heuristic,
                "k2",
                "second entry",
                &["tooling"],
                0.6,
                &["ep-b"],
                now,
            ))
            .expect("add");

        let backup_path = tmp.path().join("merkle.jsonl");
        let count = store
            .export(&backup_path, &ExportFilter::default())
            .expect("export");
        assert_eq!(count, 2);

        // Parse the header line and verify merkle_root is non-empty.
        let content = std::fs::read_to_string(&backup_path).expect("read");
        let header_line = content.lines().next().expect("header line");
        let header: BackupHeader = serde_json::from_str(header_line).expect("parse header");
        assert!(
            !header.merkle_root.is_empty(),
            "merkle_root must be non-empty for non-empty export"
        );
        // Must be a 64-char lowercase hex SHA-256.
        assert_eq!(
            header.merkle_root.len(),
            64,
            "merkle_root must be 64 hex chars (SHA-256)"
        );
        assert!(
            header.merkle_root.chars().all(|c| c.is_ascii_hexdigit()),
            "merkle_root must be hex"
        );
    }

    #[test]
    fn export_merkle_root_is_deterministic() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let now = Utc::now();

        let store_a = KnowledgeStore::new(tmp.path().join("a").join("knowledge.jsonl"));
        store_a
            .add(entry(
                KnowledgeKind::Insight,
                "id-1",
                "alpha",
                &["x"],
                0.9,
                &[],
                now,
            ))
            .expect("add");
        store_a
            .add(entry(
                KnowledgeKind::Insight,
                "id-2",
                "beta",
                &["y"],
                0.7,
                &[],
                now,
            ))
            .expect("add");

        let store_b = KnowledgeStore::new(tmp.path().join("b").join("knowledge.jsonl"));
        // Insert in reverse order.
        store_b
            .add(entry(
                KnowledgeKind::Insight,
                "id-2",
                "beta",
                &["y"],
                0.7,
                &[],
                now,
            ))
            .expect("add");
        store_b
            .add(entry(
                KnowledgeKind::Insight,
                "id-1",
                "alpha",
                &["x"],
                0.9,
                &[],
                now,
            ))
            .expect("add");

        let path_a = tmp.path().join("out_a.jsonl");
        let path_b = tmp.path().join("out_b.jsonl");
        store_a
            .export(&path_a, &ExportFilter::default())
            .expect("export a");
        store_b
            .export(&path_b, &ExportFilter::default())
            .expect("export b");

        let header_a: BackupHeader = serde_json::from_str(
            std::fs::read_to_string(&path_a)
                .expect("read a")
                .lines()
                .next()
                .expect("line"),
        )
        .expect("parse a");
        let header_b: BackupHeader = serde_json::from_str(
            std::fs::read_to_string(&path_b)
                .expect("read b")
                .lines()
                .next()
                .expect("line"),
        )
        .expect("parse b");

        assert_eq!(
            header_a.merkle_root, header_b.merkle_root,
            "merkle root must be deterministic regardless of insertion order"
        );
    }

    #[test]
    fn compute_merkle_root_empty_returns_empty_string() {
        assert_eq!(compute_merkle_root(&[]), "");
    }

    #[test]
    fn compute_merkle_root_single_entry() {
        let root = compute_merkle_root(&["only-entry".to_owned()]);
        // Single leaf: root = SHA-256("only-entry") as hex.
        assert_eq!(root.len(), 64);
        assert!(root.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn compute_merkle_root_order_independent() {
        let ids_a = vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()];
        let mut ids_b = ids_a.clone();
        ids_b.reverse();
        assert_eq!(
            compute_merkle_root(&ids_a),
            compute_merkle_root(&ids_b),
            "Merkle root must be order-independent (IDs are sorted internally)"
        );
    }

    fn write_canonical_test_backup(path: &Path, entries: &[KnowledgeEntry]) {
        let header = BackupHeader {
            version: KNOWLEDGE_BACKUP_VERSION,
            created_at: Utc::now(),
            entry_count: entries.len(),
            source_path: "test".to_owned(),
            merkle_root: compute_entry_merkle_root(entries).expect("compute root"),
        };
        let mut contents = serde_json::to_string(&header).expect("serialize header");
        contents.push('\n');
        for entry in entries {
            contents.push_str(&serde_json::to_string(entry).expect("serialize entry"));
            contents.push('\n');
        }
        std::fs::write(path, contents).expect("write canonical backup");
    }

    fn assert_failed_import_preserves_store(
        destination: &KnowledgeStore,
        backup: &Path,
        expected_error: &str,
    ) {
        let before = destination.read_all().expect("read before failed import");
        let error = destination
            .import(backup, &ImportOptions::default())
            .expect_err("import must fail");
        assert!(
            format!("{error:#}").contains(expected_error),
            "expected `{expected_error}` in `{error:#}`"
        );
        assert_eq!(
            destination.read_all().expect("read after failed import"),
            before,
            "validation failure must not partially write"
        );
    }

    #[test]
    fn export_default_filters_secrets_before_top_n() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));
        let now = Utc::now();
        store
            .add(entry(
                KnowledgeKind::Insight,
                "secret",
                "ANTHROPIC_API_KEY=private-value",
                &["api_key"],
                0.99,
                &[],
                now,
            ))
            .expect("add secret");
        store
            .add(entry(
                KnowledgeKind::Insight,
                "safe",
                "bounded retries improve reliability",
                &["reliability"],
                0.75,
                &[],
                now,
            ))
            .expect("add safe");

        let backup = tmp.path().join("export.jsonl");
        let count = store
            .export(
                &backup,
                &ExportFilter {
                    max_entries: Some(1),
                    ..Default::default()
                },
            )
            .expect("export");
        assert_eq!(count, 1);
        let (entries, legacy) = read_import_entries(&backup, false).expect("validate export");
        assert!(!legacy);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "safe");
    }

    #[test]
    fn import_rejects_content_and_id_tampering_without_partial_writes() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        destination
            .add(entry(
                KnowledgeKind::Insight,
                "existing",
                "existing durable knowledge",
                &["existing"],
                0.9,
                &[],
                Utc::now(),
            ))
            .expect("seed destination");
        let source = entry(
            KnowledgeKind::Insight,
            "source-id",
            "original content",
            &["source"],
            0.9,
            &[],
            Utc::now(),
        );

        for (name, needle, replacement) in [
            ("content", "original content", "tampered content"),
            ("id", "source-id", "tampered-id"),
        ] {
            let backup = tmp.path().join(format!("tampered-{name}.jsonl"));
            write_canonical_test_backup(&backup, std::slice::from_ref(&source));
            let contents = std::fs::read_to_string(&backup)
                .expect("read backup")
                .replace(needle, replacement);
            std::fs::write(&backup, contents).expect("tamper backup");
            assert_failed_import_preserves_store(
                &destination,
                &backup,
                "backup Merkle verification failed",
            );
        }
    }

    #[test]
    fn import_rejects_count_mismatch_malformed_and_truncated_inputs_without_writes() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        let source = entry(
            KnowledgeKind::Insight,
            "source",
            "valid import source",
            &["source"],
            0.9,
            &[],
            Utc::now(),
        );

        let count_mismatch = tmp.path().join("count-mismatch.jsonl");
        write_canonical_test_backup(&count_mismatch, std::slice::from_ref(&source));
        let contents = std::fs::read_to_string(&count_mismatch)
            .expect("read")
            .replacen("\"entry_count\":1", "\"entry_count\":2", 1);
        std::fs::write(&count_mismatch, contents).expect("write mismatch");
        assert_failed_import_preserves_store(
            &destination,
            &count_mismatch,
            "backup entry_count mismatch",
        );

        let malformed = tmp.path().join("malformed.jsonl");
        write_canonical_test_backup(&malformed, std::slice::from_ref(&source));
        let header = std::fs::read_to_string(&malformed)
            .expect("read")
            .lines()
            .next()
            .expect("header")
            .to_owned();
        std::fs::write(&malformed, format!("{header}\n{{\n")).expect("write malformed");
        assert_failed_import_preserves_store(&destination, &malformed, "malformed_entries=1");

        let truncated = tmp.path().join("truncated.jsonl");
        write_canonical_test_backup(&truncated, &[source]);
        let header = std::fs::read_to_string(&truncated)
            .expect("read")
            .lines()
            .next()
            .expect("header")
            .to_owned();
        std::fs::write(&truncated, format!("{header}\n")).expect("write truncated");
        assert_failed_import_preserves_store(
            &destination,
            &truncated,
            "backup entry_count mismatch",
        );
    }

    #[test]
    fn import_reports_exact_id_and_semantic_dedup_counts() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        destination
            .add(entry(
                KnowledgeKind::Insight,
                "existing",
                "semantic duplicate content",
                &["dedup"],
                0.9,
                &[],
                Utc::now(),
            ))
            .expect("seed destination");
        let duplicate_semantic = entry(
            KnowledgeKind::Insight,
            "semantic-copy",
            "semantic duplicate content",
            &["dedup"],
            0.8,
            &[],
            Utc::now(),
        );
        let first = entry(
            KnowledgeKind::Heuristic,
            "repeated-id",
            "first repeated ID entry",
            &["first"],
            0.8,
            &[],
            Utc::now(),
        );
        let second = entry(
            KnowledgeKind::Warning,
            "repeated-id",
            "second repeated ID entry",
            &["second"],
            0.8,
            &[],
            Utc::now(),
        );
        let backup = tmp.path().join("duplicates.jsonl");
        write_canonical_test_backup(&backup, &[duplicate_semantic, first, second]);

        let result = destination
            .import(&backup, &ImportOptions::default())
            .expect("import");
        assert_eq!(result.source_entries, 3);
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped_dedup, 2);
        assert_eq!(result.skipped_contradiction, 0);
        assert_eq!(destination.read_all().expect("read").len(), 2);
    }

    #[test]
    fn import_unconditionally_skips_high_confidence_contradictions() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        destination
            .add(entry(
                KnowledgeKind::AntiKnowledge,
                "refutation",
                "never retry an irreversible payment",
                &["payments", "retry"],
                0.95,
                &[],
                Utc::now(),
            ))
            .expect("seed AntiKnowledge");
        let candidate = entry(
            KnowledgeKind::Insight,
            "contradiction",
            "never retry an irreversible payment",
            &["payments", "retry"],
            0.95,
            &[],
            Utc::now(),
        );
        let backup = tmp.path().join("contradiction.jsonl");
        write_canonical_test_backup(&backup, &[candidate]);

        let result = destination
            .import(&backup, &ImportOptions::default())
            .expect("import");
        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped_contradiction, 1);
        assert_eq!(result.skipped_dedup, 0);
        assert_eq!(destination.read_all().expect("read").len(), 1);
    }

    #[test]
    fn import_default_discount_is_point_eight_and_legacy_requires_opt_in() {
        assert_eq!(ImportOptions::default().confidence_discount, 0.8);

        let tmp = TempDir::new().expect("tempdir");
        let raw = tmp.path().join("legacy.jsonl");
        let source = entry(
            KnowledgeKind::Insight,
            "legacy",
            "trusted legacy entry",
            &["legacy"],
            0.5,
            &[],
            Utc::now(),
        );
        std::fs::write(
            &raw,
            format!("{}\n", serde_json::to_string(&source).unwrap()),
        )
        .expect("write legacy");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        assert!(destination.import(&raw, &ImportOptions::default()).is_err());
        let result = destination
            .import(
                &raw,
                &ImportOptions {
                    allow_legacy: true,
                    ..Default::default()
                },
            )
            .expect("explicit legacy import");
        assert!(result.legacy_input);
        assert_eq!(result.imported, 1);
        let imported = destination.read_all().expect("read");
        assert!((imported[0].confidence - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn export_and_import_reject_the_live_store_as_their_transfer_path() {
        let tmp = TempDir::new().expect("tempdir");
        let empty_store = KnowledgeStore::new(tmp.path().join("empty").join("knowledge.jsonl"));
        let empty_error = empty_store
            .export(empty_store.path(), &ExportFilter::default())
            .expect_err("an absent self-export path must still fail");
        assert!(format!("{empty_error:#}").contains("live store"));
        assert!(
            !empty_store.path().exists(),
            "failed empty-store self-export must not create a backup header"
        );

        let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));
        store
            .add(entry(
                KnowledgeKind::Insight,
                "live",
                "live knowledge must remain a store",
                &["safety"],
                0.9,
                &[],
                Utc::now(),
            ))
            .expect("seed live store");
        let before = std::fs::read(store.path()).expect("read live bytes");

        let export_error = store
            .export(store.path(), &ExportFilter::default())
            .expect_err("self-export must fail");
        assert!(format!("{export_error:#}").contains("live store"));
        let import_error = store
            .import(
                store.path(),
                &ImportOptions {
                    allow_legacy: true,
                    ..Default::default()
                },
            )
            .expect_err("self-import must fail");
        assert!(format!("{import_error:#}").contains("live store"));
        assert_eq!(std::fs::read(store.path()).expect("read after"), before);
    }

    #[test]
    fn export_failure_preserves_an_existing_destination() {
        let tmp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(tmp.path().join("knowledge.jsonl"));
        std::fs::write(store.path(), b"not-json\n").expect("write corrupt source store");
        let output = tmp.path().join("existing-export.jsonl");
        std::fs::write(&output, b"previous valid artifact\n").expect("write prior export");

        let error = store
            .export(&output, &ExportFilter::default())
            .expect_err("corrupt source must fail export");
        assert!(format!("{error:#}").contains("decode knowledge line 1"));
        assert_eq!(
            std::fs::read(&output).expect("read preserved export"),
            b"previous valid artifact\n"
        );
    }

    #[test]
    fn import_rejects_a_corrupt_existing_store_without_losing_raw_records() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        let existing = entry(
            KnowledgeKind::Insight,
            "existing",
            "valid existing record",
            &["existing"],
            0.9,
            &[],
            Utc::now(),
        );
        let mut live_bytes = serde_json::to_vec(&existing).expect("serialize existing");
        live_bytes.extend_from_slice(b"\nnot-json\n");
        std::fs::write(destination.path(), &live_bytes).expect("write corrupt live store");

        let backup = tmp.path().join("source.jsonl");
        write_canonical_test_backup(
            &backup,
            &[entry(
                KnowledgeKind::Heuristic,
                "source",
                "valid source record",
                &["source"],
                0.8,
                &[],
                Utc::now(),
            )],
        );
        let error = destination
            .import(&backup, &ImportOptions::default())
            .expect_err("corrupt destination must fail closed");
        assert!(format!("{error:#}").contains("decode knowledge line 2"));
        assert_eq!(
            std::fs::read(destination.path()).expect("read preserved live store"),
            live_bytes
        );
    }

    #[test]
    fn imported_high_confidence_antiknowledge_blocks_regardless_of_record_order_or_decay() {
        let tmp = TempDir::new().expect("tempdir");
        let anti = entry(
            KnowledgeKind::AntiKnowledge,
            "anti",
            "never retry an irreversible payment",
            &["payments", "retry"],
            0.95,
            &[],
            Utc::now(),
        );
        let claim = entry(
            KnowledgeKind::Insight,
            "claim",
            "never retry an irreversible payment",
            &["payments", "retry"],
            0.95,
            &[],
            Utc::now(),
        );

        for (label, source) in [
            ("anti-first", vec![anti.clone(), claim.clone()]),
            ("anti-last", vec![claim.clone(), anti.clone()]),
        ] {
            let backup = tmp.path().join(format!("{label}.jsonl"));
            write_canonical_test_backup(&backup, &source);
            let destination = KnowledgeStore::new(tmp.path().join(format!("{label}-dest.jsonl")));
            let result = destination
                .import(&backup, &ImportOptions::default())
                .expect("import guarded bundle");
            assert_eq!(result.imported, 1, "{label}");
            assert_eq!(result.skipped_contradiction, 1, "{label}");
            let restored = destination.read_all().expect("read destination");
            assert_eq!(restored.len(), 1, "{label}");
            assert_eq!(restored[0].kind, KnowledgeKind::AntiKnowledge, "{label}");
            assert!(restored[0].confidence < 0.8, "default decay must apply");
        }
    }

    #[test]
    fn imported_antiknowledge_is_not_deduplicated_against_ordinary_knowledge() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = KnowledgeStore::new(tmp.path().join("destination.jsonl"));
        destination
            .add(entry(
                KnowledgeKind::Insight,
                "claim",
                "retrying an irreversible payment is unsafe",
                &["payments"],
                0.9,
                &[],
                Utc::now(),
            ))
            .expect("seed ordinary knowledge");
        let backup = tmp.path().join("anti.jsonl");
        write_canonical_test_backup(
            &backup,
            &[entry(
                KnowledgeKind::AntiKnowledge,
                "anti",
                "retrying an irreversible payment is unsafe",
                &["payments"],
                0.95,
                &[],
                Utc::now(),
            )],
        );

        let result = destination
            .import(&backup, &ImportOptions::default())
            .expect("import AntiKnowledge");
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped_dedup, 0);
        assert!(
            destination
                .read_all()
                .expect("read destination")
                .iter()
                .any(|entry| entry.kind == KnowledgeKind::AntiKnowledge)
        );
    }

    fn e24_store() -> (TempDir, KnowledgeStore) {
        let temp = TempDir::new().expect("tempdir");
        let store = KnowledgeStore::new(temp.path().join("knowledge.jsonl"));
        (temp, store)
    }

    fn e24_entry(id: &str) -> KnowledgeEntry {
        let mut value = entry(
            KnowledgeKind::AntiKnowledge,
            id,
            "Avoid repeating an observed verification failure",
            &["memory", "verification"],
            0.8,
            &["episode-1"],
            Utc::now(),
        );
        value.tier = KnowledgeTier::Transient;
        value
    }

    #[test]
    fn e24_demurrage_reduces_balance_and_halves_half_life_once() {
        let (_temp, store) = e24_store();
        let mut value = e24_entry("demurrage");
        value.balance = 0.004;
        value.half_life_days = 20.0;
        store.add(value).expect("add");

        assert_eq!(store.demurrage(0.005).expect("demurrage"), 1);
        let after = store.read_all().expect("read").remove(0);
        assert!((after.balance + 0.001).abs() < 1e-9);
        assert_eq!(after.half_life_days, 10.0);
        assert!(after.balance_depleted_at.is_some());

        store.demurrage(0.005).expect("second demurrage");
        let after = store.read_all().expect("read").remove(0);
        assert_eq!(
            after.half_life_days, 10.0,
            "half-life changes only at crossing"
        );
    }

    #[test]
    fn e24_demurrage_freezes_after_seven_depleted_days() {
        let (_temp, store) = e24_store();
        let mut value = e24_entry("freeze-after-grace");
        value.balance = 0.0;
        value.balance_depleted_at = Some(Utc::now() - Duration::days(8));
        store.add(value).expect("add");

        store.demurrage(0.005).expect("demurrage");
        let after = store.read_all().expect("read").remove(0);
        assert!(after.frozen);
        assert!(after.frozen_at.is_some());
    }

    #[test]
    fn e24_reinforce_uses_exact_signal_amounts() {
        let (_temp, store) = e24_store();
        let mut value = e24_entry("reinforcement");
        value.balance = 0.0;
        value.balance_depleted_at = Some(Utc::now());
        store.add(value).expect("add");
        let signals = [
            (crate::ReinforcementSignal::Retrieved, 0.05),
            (crate::ReinforcementSignal::Cited, 0.10),
            (crate::ReinforcementSignal::Gated, 0.15),
            (crate::ReinforcementSignal::Surprised, 0.08),
            (crate::ReinforcementSignal::AgentQuoted, 0.12),
        ];
        let mut expected = 0.0;
        for (signal, amount) in signals {
            store.reinforce("reinforcement", signal).expect("reinforce");
            expected += amount;
            let actual = store.read_all().expect("read")[0].balance;
            assert!((actual - expected).abs() < 1e-9);
        }
        assert!(
            store.read_all().expect("read")[0]
                .balance_depleted_at
                .is_none()
        );
    }

    #[test]
    fn e24_falsifier_survives_immunizes_and_discredits() {
        let (_temp, store) = e24_store();
        let mut survivor = e24_entry("survivor");
        survivor.falsifier = Some(Falsifier {
            predicate: "retries remain bounded".to_string(),
            observations: 0,
            violations: 0,
            last_checked: Utc::now(),
            active: true,
        });
        let mut discredited = e24_entry("discredited");
        discredited.content = "Avoid a separately falsified retry pattern".to_string();
        discredited.tags.push("separate".to_string());
        discredited.falsifier = survivor.falsifier.clone();
        store.ingest(vec![survivor, discredited]).expect("ingest");

        assert_eq!(
            store.check_falsifier("survivor", false).expect("check"),
            FalsifierOutcome::Survived
        );
        store.check_falsifier("survivor", false).expect("check");
        assert_eq!(
            store.check_falsifier("survivor", false).expect("check"),
            FalsifierOutcome::Immunized
        );
        assert_eq!(
            store.check_falsifier("discredited", true).expect("check"),
            FalsifierOutcome::Discredited
        );
        let entries = store.read_all().expect("read");
        let survivor = entries.iter().find(|entry| entry.id == "survivor").unwrap();
        assert_eq!(survivor.tier, KnowledgeTier::Consolidated);
        assert!(survivor.confidence >= 0.9);
        let discredited = entries
            .iter()
            .find(|entry| entry.id == "discredited")
            .unwrap();
        assert_eq!(discredited.confidence, 0.4);
        assert!(!discredited.falsifier.as_ref().unwrap().active);
    }

    #[test]
    fn e24_tier_progression_promotes_and_demotes() {
        let (_temp, store) = e24_store();
        let mut promote = e24_entry("promote");
        promote.kind = KnowledgeKind::Insight;
        promote.content = "Independent confirmations support bounded retries".to_string();
        promote.tags.push("promotion".to_string());
        promote.confirmation_count = 2;
        promote.confidence = 0.6;
        let mut demote = e24_entry("demote");
        demote.content = "A fragile working rule should leave working memory".to_string();
        demote.tags.push("demotion".to_string());
        demote.tier = KnowledgeTier::Working;
        demote.confidence = 0.1;
        store.ingest(vec![promote, demote]).expect("ingest");

        let report = store
            .apply_tier_progression(&crate::TierProgressionConfig::default())
            .expect("progression");
        assert!(
            report
                .promoted
                .contains(&("promote".to_string(), KnowledgeTier::Working))
        );
        assert!(
            report
                .demoted
                .contains(&("demote".to_string(), KnowledgeTier::Transient))
        );
    }

    #[test]
    fn e24_temporal_index_tracks_add_query_relation_and_gc() {
        let (temp, mut store) = e24_store();
        store.enable_temporal_index().expect("enable");
        let now = Utc::now();
        let mut epoch = KnowledgeEpoch::at(7, "test", now - Duration::seconds(1));
        epoch.close(now + Duration::seconds(1));
        assert!(store.add_temporal_epoch(epoch));
        let mut first = e24_entry("temporal-a");
        first.created_at = now - Duration::milliseconds(20);
        first.kind = KnowledgeKind::Insight;
        first.content = "Temporal entry alpha has unique context".to_string();
        first.tags.push("alpha".to_string());
        let mut second = e24_entry("temporal-b");
        second.kind = KnowledgeKind::Insight;
        second.content = "Temporal entry beta has distinct evidence".to_string();
        second.tags.push("beta".to_string());
        second.created_at = now;
        store.ingest(vec![first, second]).expect("ingest");
        assert_eq!(store.query_temporal(7).expect("query").len(), 2);
        assert!(
            store
                .query_temporal_relation("temporal-a", "temporal-b")
                .expect("relation")
                .is_some()
        );
        store
            .update_confidence("temporal-a", -1.0)
            .expect("confidence update");
        store.gc(0.5).expect("gc");
        assert!(
            store
                .query_temporal_relation("temporal-a", "temporal-b")
                .expect("relation")
                .is_none()
        );
        drop(temp);
    }

    #[cfg(feature = "hdc")]
    #[test]
    fn e24_query_hdc_is_similarity_sorted() {
        let (_temp, store) = e24_store();
        let query = HdcVector::from_seed(b"e24-exact");
        let mut exact = e24_entry("hdc-exact");
        exact.content = "Exact HDC query target".to_string();
        exact.tags.push("exact".to_string());
        exact.hdc_vector = Some(query.to_bytes().to_vec());
        let mut different = e24_entry("hdc-different");
        different.content = "Different HDC comparison target".to_string();
        different.tags.push("different".to_string());
        different.hdc_vector = Some(HdcVector::from_seed(b"e24-different").to_bytes().to_vec());
        store.ingest(vec![different, exact]).expect("ingest");

        let hits = store.query_hdc(&query, 2).expect("query hdc");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].entry.id, "hdc-exact");
        assert!(hits[0].total_score >= hits[1].total_score);
    }
}
