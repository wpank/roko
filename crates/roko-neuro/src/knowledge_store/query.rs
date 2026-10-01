//! Knowledge store query, search, and statistics operations.

use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use chrono::Utc;

use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeTier};

#[cfg(feature = "hdc")]
use crate::hdc::{ResonanceDetector, ResonancePair, RoleFillerEncoder};
#[cfg(feature = "hdc")]
use anyhow::Context;
#[cfg(feature = "hdc")]
use roko_primitives::hdc::HdcVector;
#[cfg(feature = "hdc")]
use std::io::{self, BufRead, BufReader};

use super::KnowledgeStore;
use super::scoring::*;
use super::types::*;

impl KnowledgeStore {
    /// Query the store for entries relevant to `topic`.
    ///
    /// Relevance is scored by keyword overlap in tags/content, multiplied
    /// by confidence, recency, and a 1.5x confirmation boost for entries
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

    /// P1-34: Query with PAD affect state bias.
    ///
    /// Performs a standard keyword query, then re-scores results using PAD
    /// similarity between the current affect state and each entry's emotional
    /// provenance. Entries discovered in similar emotional states get a boost.
    ///
    /// The `pad_weight` parameter controls how much influence affect has:
    /// 0.0 = no affect bias, 1.0 = strong affect bias.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn query_with_affect(
        &self,
        topic: &str,
        limit: usize,
        affect: &roko_core::affect::PadVector,
        pad_weight: f64,
    ) -> Result<Vec<KnowledgeEntry>> {
        let pad_weight = pad_weight.clamp(0.0, 1.0);
        // Fetch more candidates than needed so affect bias can reshuffle.
        let oversampled = (limit * 2).max(10);
        let mut hits = self.query_hits(topic, oversampled)?;

        if pad_weight > f64::EPSILON {
            for hit in &mut hits {
                let pad_bonus = pad_affinity(affect, &hit.entry) * pad_weight * 0.25;
                hit.total_score += pad_bonus;
            }
            hits.sort_by(|left, right| {
                right
                    .total_score
                    .partial_cmp(&left.total_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.entry.id.cmp(&right.entry.id))
            });
        }

        hits.truncate(limit);
        Ok(hits.into_iter().map(|hit| hit.entry).collect())
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

    /// Every entry a hot query considers, unranked. Prompt caches load these
    /// once and rank them for each task (bug-86117a).
    ///
    /// # Errors
    ///
    /// Returns an error if the backing file cannot be read.
    pub fn hot_entries(&self) -> Result<Vec<KnowledgeEntry>> {
        let mut entries = self.read_all()?;
        entries.retain(is_hot);
        Ok(entries)
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
        let file = match std::fs::File::open(&self.path) {
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
            // P3-11: Skip entries whose HDC vector was encoded with a
            // different version to avoid spurious similarity matches.
            if entry.hdc_encoder_version != 0
                && entry.hdc_encoder_version != roko_core::engram::ENCODER_VERSION_TEXT_V1
            {
                tracing::debug!(
                    entry_id = %entry.id,
                    stored_version = entry.hdc_encoder_version,
                    current_version = roko_core::engram::ENCODER_VERSION_TEXT_V1,
                    "P3-11: skipping HDC entry with mismatched encoder version"
                );
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

        // P2-29: HDC telemetry -- log query metrics for Lens/tracing consumption.
        let top_score = hits.first().map(|h| h.total_score).unwrap_or(0.0);
        tracing::info!(
            monotonic_counter.roko_hdc_queries_total = 1_u64,
            result_count = hits.len(),
            top_similarity = %format!("{top_score:.4}"),
            top_k,
            "HDC query completed"
        );

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

    pub(crate) fn query_hits_filtered<F>(
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
                if !is_hot(&entry) || !include(&entry) {
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

    #[cfg(feature = "hdc")]
    /// Build an in-memory HDC index over the current knowledge store.
    ///
    /// The index fingerprints each entry's content once and keeps the
    /// resulting vectors resident for fast similarity search.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read.
    pub fn memory_index(&self) -> Result<super::memory_index::MemoryIndex> {
        Ok(super::memory_index::MemoryIndex::from_entries(
            self.read_all()?,
        ))
    }
}

/// Whether hot queries consider `entry`. Frozen entries belong to the cold
/// tier (NEURO-11), and entries with blank content are noise (Audit #80).
fn is_hot(entry: &KnowledgeEntry) -> bool {
    !entry.frozen && !entry.content.trim().is_empty()
}
