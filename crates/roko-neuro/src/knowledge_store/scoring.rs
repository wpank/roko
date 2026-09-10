//! Scoring, similarity, normalization, and comparison helpers for knowledge queries.

use std::collections::HashSet;

use chrono::{DateTime, Utc};

#[cfg(feature = "hdc")]
use crate::hdc::KnowledgeHdcEncoder;
use crate::{KnowledgeEntry, KnowledgeKind};

use super::types::*;

#[cfg(feature = "hdc")]
use roko_primitives::hdc::{HdcVector, text_fingerprint};

// ── Text helpers ─────────────────────────────────────────────────────

pub(crate) fn normalize(text: &str) -> String {
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

pub(crate) fn tokenize(text: &str) -> Vec<String> {
    normalize(text)
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

// ── Scoring primitives ───────────────────────────────────────────────

pub(crate) fn keyword_score(entry: &KnowledgeEntry, terms: &[String], topic_norm: &str) -> f64 {
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
/// Confirmations extend the effective lifetime -- each independent confirmation
/// adds 10% to the weight, rewarding knowledge that multiple episodes validate.
pub(crate) fn recency_factor(entry: &KnowledgeEntry, now: DateTime<Utc>) -> f64 {
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

pub(crate) fn effective_half_life_days(entry: &KnowledgeEntry) -> f64 {
    entry.effective_half_life_days()
}

pub(crate) fn effective_confidence(entry: &KnowledgeEntry) -> f64 {
    bounded_confidence(entry) * confirmation_boost(entry) * entry.emotional_consolidation_boost()
}

pub(crate) fn bounded_confidence(entry: &KnowledgeEntry) -> f64 {
    let confidence = entry.confidence.clamp(0.0, 1.0);
    if entry.kind == KnowledgeKind::AntiKnowledge {
        confidence.max(ANTI_KNOWLEDGE_CONFIDENCE_FLOOR)
    } else {
        confidence
    }
}

pub(crate) fn confirmation_boost(entry: &KnowledgeEntry) -> f64 {
    if entry.source_episodes.len() >= 2 {
        CONFIRMATION_BOOST
    } else {
        1.0
    }
}

pub(crate) fn compare_hit_scores(
    left: &KnowledgeQueryHit,
    right: &KnowledgeQueryHit,
) -> std::cmp::Ordering {
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

pub(crate) fn emotional_retrieval_boost(entry: &KnowledgeEntry) -> f64 {
    entry.emotional_retrieval_boost()
}

/// P1-34: Compute PAD affinity between the current affect state and a
/// knowledge entry's emotional provenance. Returns 0.0 if the entry
/// has no emotional provenance. Returns a value in [0.0, 1.0] where
/// 1.0 = identical PAD vectors.
pub(crate) fn pad_affinity(
    current: &roko_core::affect::PadVector,
    entry: &KnowledgeEntry,
) -> f64 {
    let Some(provenance) = entry.emotional_provenance.as_ref() else {
        return 0.0;
    };
    let avg = &provenance.average_pad;
    let dp = current.pleasure - avg.pleasure;
    let da = current.arousal - avg.arousal;
    let dd = current.dominance - avg.dominance;
    let distance = (dp * dp + da * da + dd * dd).sqrt();
    // Max possible distance is sqrt(12) ~ 3.46 (each dimension in [-1, 1]).
    // Normalize to [0, 1] where 0 = max distance, 1 = identical.
    (1.0 - distance / 3.46).clamp(0.0, 1.0)
}

pub(crate) fn knowledge_kind_label(kind: KnowledgeKind) -> &'static str {
    kind.as_str()
}

pub(crate) fn similarity_against_entry(fingerprint: &[u8], entry: &KnowledgeEntry) -> Option<f32> {
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
pub(crate) fn hdc_similarity(entry: &KnowledgeEntry, topic: &str) -> f64 {
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

pub(crate) fn score_entry_for_query(
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

pub(crate) fn entry_similarity(existing: &KnowledgeEntry, candidate: &KnowledgeEntry) -> f64 {
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

pub(crate) fn jaccard_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
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
pub(crate) fn entries_are_similar(
    existing: &KnowledgeEntry,
    new_entry: &KnowledgeEntry,
) -> bool {
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

pub(crate) fn compare_similarity_hits(
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
pub(crate) fn compare_hits(
    left: &super::memory_index::MemoryHit,
    right: &super::memory_index::MemoryHit,
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

// ── HDC fingerprinting ───────────────────────────────────────────────

#[cfg(feature = "hdc")]
pub(crate) fn fingerprint_entry(entry: &KnowledgeEntry) -> HdcVector {
    if let Some(vector) = entry.hdc_vector.as_deref()
        && let Ok(bytes) = <[u8; HDC_VECTOR_BYTES]>::try_from(vector)
    {
        return HdcVector::from_bytes(&bytes);
    }
    KnowledgeHdcEncoder.encode_entry(entry)
}

#[cfg(feature = "hdc")]
pub(crate) fn fingerprint_content(content: &str) -> HdcVector {
    text_fingerprint(&normalize(content))
}

// ── Ingest helpers ───────────────────────────────────────────────────

#[cfg(feature = "hdc")]
pub(crate) fn prepare_entries_for_ingest(
    entries: Vec<KnowledgeEntry>,
) -> Vec<KnowledgeEntry> {
    entries
        .into_iter()
        .map(normalize_entry_for_ingest)
        .collect()
}

#[cfg(not(feature = "hdc"))]
pub(crate) fn prepare_entries_for_ingest(
    entries: Vec<KnowledgeEntry>,
) -> Vec<KnowledgeEntry> {
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

pub(crate) fn normalize_entry_for_ingest(entry: KnowledgeEntry) -> KnowledgeEntry {
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

pub(crate) fn normalize_entry_security(mut entry: KnowledgeEntry) -> KnowledgeEntry {
    let channel = entry
        .source
        .as_deref()
        .map(crate::SourceChannel::from_source_label)
        .unwrap_or(crate::SourceChannel::AgentOutput);
    crate::apply_source_security_labels(std::slice::from_mut(&mut entry), channel);
    entry
}

pub(crate) fn coalesce_incoming_security_labels(
    entries: Vec<KnowledgeEntry>,
) -> Vec<KnowledgeEntry> {
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

pub(crate) fn join_replayed_security_labels(
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
    // Mesh-sourced entries have an explicit tier set by the sync protocol's
    // receive policy.  Do not auto-promote them -- the peer-assigned tier
    // must be earned through local confirmation/progression instead.
    let is_mesh = entry
        .source
        .as_deref()
        .is_some_and(|s| s.starts_with("mesh:"));
    if is_mesh {
        return entry;
    }

    let inferred = inferred_retention_tier(&entry);
    if inferred.multiplier() > entry.tier.multiplier() {
        entry.tier = inferred;
    }
    entry
}

fn inferred_retention_tier(entry: &KnowledgeEntry) -> crate::KnowledgeTier {
    use crate::KnowledgeTier;
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

// ── Deduplication and confirmation detection ─────────────────────────

pub(crate) fn dedupe_entries_for_ingest(
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
pub(crate) fn detect_confirmations(
    existing: &[KnowledgeEntry],
    new_entries: &[KnowledgeEntry],
) -> Vec<KnowledgeConfirmationRecord> {
    let now = chrono::Utc::now();
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
pub(crate) fn check_against_anti_knowledge(
    entries: Vec<KnowledgeEntry>,
    existing: &[KnowledgeEntry],
) -> Vec<KnowledgeEntry> {
    use crate::hdc::KnowledgeHdcEncoder;

    let anti_entries: Vec<_> = existing
        .iter()
        .filter(|e| e.kind == KnowledgeKind::AntiKnowledge)
        .collect();

    if anti_entries.is_empty() {
        return entries;
    }

    // Pre-encode all AntiKnowledge entries.
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

        let entry_vec = KnowledgeHdcEncoder.encode_entry(&entry);
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

pub(crate) fn truncate_snippet(content: &str, max_chars: usize) -> String {
    let mut chars = content.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_none() {
        content.to_string()
    } else {
        format!("{truncated}...")
    }
}

pub(crate) fn stable_hash(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}
