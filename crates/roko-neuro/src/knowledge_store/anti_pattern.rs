//! Anti-pattern extraction and classification from gate failures.

use std::collections::HashSet;

use chrono::Utc;

use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeTier};

use super::KnowledgeStore;
use super::scoring::{entry_similarity, normalize, stable_hash, truncate_snippet};

pub(crate) const ANTI_PATTERN_DUPLICATE_SIMILARITY_THRESHOLD: f64 = 0.45;

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
        hdc_encoder_version: 0,
        access_count: 0,
        last_accessed: None,
        contradiction_count: 0,
        activation_conditions: Vec::new(),
        commit_batch: None,
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

pub(crate) fn compilation_error_code_tags(error: &str) -> Vec<String> {
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

pub(crate) fn find_similar_anti_pattern_index(
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

pub(crate) fn reinforce_anti_pattern(existing: &mut KnowledgeEntry, candidate: &KnowledgeEntry) {
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

pub(crate) fn merge_tags(existing: &mut Vec<String>, additional: &[String]) {
    let mut seen: HashSet<String> = existing.iter().cloned().collect();
    for tag in additional {
        if seen.insert(tag.clone()) {
            existing.push(tag.clone());
        }
    }
    existing.sort();
    existing.dedup();
}
