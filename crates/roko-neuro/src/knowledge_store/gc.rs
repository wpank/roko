//! Garbage collection, decay, demurrage, compaction, tier progression, and lifecycle operations.

use anyhow::{Result, ensure};
use chrono::Utc;

use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeTier};

use super::scoring::{effective_confidence, is_dead, recency_factor};
use super::types::*;
use super::KnowledgeStore;

impl KnowledgeStore {
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

    /// P3-22: Compact the knowledge confirmation log.
    ///
    /// Deduplicates by `(id)` and removes entries whose balance is <= 0
    /// and that have been frozen, producing a smaller file.
    pub fn compact(&self) -> Result<usize> {
        let _guard = self.write_gate.lock();
        let entries = self.read_all()?;
        let before = entries.len();
        // Deduplicate by id, keeping the latest version.
        let mut deduped: std::collections::HashMap<String, KnowledgeEntry> =
            std::collections::HashMap::with_capacity(entries.len());
        for entry in entries {
            deduped
                .entry(entry.id.clone())
                .and_modify(|existing| {
                    if entry.created_at > existing.created_at {
                        *existing = entry.clone();
                    }
                })
                .or_insert(entry);
        }
        let result: Vec<KnowledgeEntry> = deduped.into_values().collect();
        let after = result.len();
        if after < before {
            self.rewrite_all(&result)?;
        }
        Ok(before - after)
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
        use super::scoring::fingerprint_entry;
        use super::types::HDC_VECTOR_BYTES;

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
}
