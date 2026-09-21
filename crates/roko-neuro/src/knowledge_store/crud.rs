//! Insert, update, delete, upsert, and reinforcement operations.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;

use anyhow::{Context, Result, ensure};
use chrono::Utc;

use crate::admission::evaluate_admission;
use crate::{
    Falsifier, KnowledgeEntry, KnowledgeKind, KnowledgeTier, SourceChannel,
    apply_source_security_labels,
};

use super::KnowledgeStore;
use super::anti_pattern::{
    extract_anti_pattern_from_failure, find_similar_anti_pattern_index, reinforce_anti_pattern,
};
use super::scoring::*;
use super::types::*;

impl KnowledgeStore {
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

        if let Some(index) = find_similar_anti_pattern_index(&entries, &candidate) {
            reinforce_anti_pattern(&mut entries[index], &candidate);
            self.rewrite_all(&entries)?;
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

                    // Auto-promote based on thresholds (P2-15: log tier progressions).
                    let old_tier = entry.tier;
                    match entry.tier {
                        KnowledgeTier::Transient if entry.confirmation_count >= 2 => {
                            entry.tier = KnowledgeTier::Working;
                        }
                        KnowledgeTier::Working if entry.distinct_contexts.len() >= 3 => {
                            entry.tier = KnowledgeTier::Consolidated;
                        }
                        _ => {}
                    }
                    if entry.tier != old_tier {
                        tracing::info!(
                            knowledge_id = %entry.id,
                            from_tier = ?old_tier,
                            to_tier = ?entry.tier,
                            confirmations = entry.confirmation_count,
                            distinct_contexts = entry.distinct_contexts.len(),
                            "knowledge tier progression"
                        );
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

    /// P3-13: Record a retrieval-access for a batch of entry IDs.
    ///
    /// Increments `access_count` and sets `last_accessed` for each matched
    /// entry. Spaced retrieval strengthening: when `access_count > 1`,
    /// the half-life is extended by a spacing factor proportional to the
    /// log of the access count, rewarding entries accessed at wider intervals.
    pub fn record_access(&self, entry_ids: &[&str]) -> Result<usize> {
        let id_set: std::collections::HashSet<&str> = entry_ids.iter().copied().collect();
        if id_set.is_empty() {
            return Ok(0);
        }
        let now = chrono::Utc::now();
        self.update_entries(|entry| {
            if id_set.contains(entry.id.as_str()) {
                entry.access_count = entry.access_count.saturating_add(1);
                // Spacing factor: extend half-life proportionally to log(access_count).
                if entry.access_count > 1 {
                    let spacing = (entry.access_count as f64).ln();
                    entry.half_life_days *= 1.0 + 0.05 * spacing;
                }
                entry.last_accessed = Some(now);
                true
            } else {
                false
            }
        })
    }
}
