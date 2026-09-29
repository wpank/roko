//! Snapshot persistence and migration helpers for the cascade router.

use roko_core::agent::AgentRole;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::types::StageTransition;

/// Serializable form of LinUCB arm parameters.
///
/// Persisting these avoids routing quality regression after restart:
/// without them, a stage-3 router (UCB mode) would have empty A/b
/// parameters and produce effectively random selections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinUCBSnapshot {
    /// Per-arm A matrix (flattened from `dim x dim`).
    pub a_matrices: Vec<Vec<f64>>,
    /// Per-arm b vector.
    pub b_vectors: Vec<Vec<f64>>,
    /// Dimensionality of the context feature vector.
    pub dim: usize,
    /// Total observations at snapshot time.
    pub observations: usize,
    /// Model slug of each arm, in the order of `a_matrices` and `b_vectors`.
    ///
    /// Arms are restored by slug, never by position (bug-605a8a). Snapshots
    /// written before this field existed leave it empty; their arms follow
    /// the cascade snapshot's `model_slugs`.
    #[serde(default)]
    pub slugs: Vec<String>,
}

impl LinUCBSnapshot {
    /// Whether [`Self::slugs`] names every persisted arm.
    #[must_use]
    pub fn names_every_arm(&self) -> bool {
        self.slugs.len() == self.a_matrices.len() && self.slugs.len() == self.b_vectors.len()
    }

    /// The arm persisted for `slug`: its `A` matrix, flattened row-major, and
    /// its `b` vector.
    ///
    /// `None` when no arm has that slug, when the arms are not named, or when
    /// the arm does not hold `dim` x `dim` and `dim` values.
    #[must_use]
    pub fn arm(&self, slug: &str) -> Option<(&[f64], &[f64])> {
        if !self.names_every_arm() {
            return None;
        }
        let index = self.slugs.iter().position(|candidate| candidate == slug)?;
        let a_matrix = self.a_matrices.get(index)?;
        let b_vector = self.b_vectors.get(index)?;
        (self.dim.checked_mul(self.dim) == Some(a_matrix.len()) && b_vector.len() == self.dim)
            .then_some((a_matrix.as_slice(), b_vector.as_slice()))
    }
}

/// Persisted snapshot of cascade router state.
#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct CascadeSnapshot {
    pub(crate) model_slugs: Vec<String>,
    #[serde(default)]
    pub(crate) role_table: HashMap<AgentRole, String>,
    pub(crate) confidence_stats: HashMap<String, PersistedModelStats>,
    /// Total observations across all models (used to restore cascade stage).
    ///
    /// Defaults to 0 for backward compatibility with snapshots written before
    /// this field was added; in that case `load_or_new` recomputes the total
    /// from the sum of per-model trials.
    #[serde(default)]
    pub(crate) total_observations: u64,
    #[serde(default)]
    pub(crate) stage_transitions: Vec<StageTransition>,
    /// LinUCB bandit state. `None` for snapshots written before this field
    /// was added; the router will start with fresh parameters in that case.
    #[serde(default)]
    pub(crate) linucb_state: Option<LinUCBSnapshot>,
    /// P3-08: Persisted Pareto frontier model slugs so the frontier survives
    /// restarts without requiring re-computation from scratch.
    #[serde(default)]
    pub(crate) pareto_frontier: Vec<String>,
}

impl CascadeSnapshot {
    /// Name the LinUCB arms of a snapshot written before arms carried their
    /// slugs. Its writer exported one arm per entry of `model_slugs`, in
    /// that order.
    pub(crate) fn name_legacy_arms(&mut self) {
        if let Some(state) = self.linucb_state.as_mut()
            && state.slugs.is_empty()
            && state.a_matrices.len() == self.model_slugs.len()
        {
            state.slugs.clone_from(&self.model_slugs);
        }
    }
}

/// Serializable form of per-model confidence stats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct PersistedModelStats {
    pub(crate) trials: u64,
    pub(crate) successes: u64,
    #[serde(default)]
    pub(crate) total_citations: u64,
    #[serde(default)]
    pub(crate) total_search_latency_ms: u64,
    #[serde(default)]
    pub(crate) total_cost_usd: f64,
    #[serde(default)]
    pub(crate) perplexity_requests: u64,
    #[serde(default)]
    pub(crate) total_gemini_thinking_tokens: u64,
    #[serde(default)]
    pub(crate) total_gemini_cached_tokens: u64,
    #[serde(default)]
    pub(crate) total_gemini_grounding_queries: u64,
    #[serde(default)]
    pub(crate) gemini_code_execution_successes: u64,
    #[serde(default)]
    pub(crate) gemini_code_execution_failures: u64,
    #[serde(default)]
    pub(crate) gemini_context_window_le_200k_requests: u64,
    #[serde(default)]
    pub(crate) gemini_context_window_gt_200k_requests: u64,
    #[serde(default)]
    pub(crate) gemini_requests: u64,
}

impl PersistedModelStats {
    pub(crate) fn weighted_half(self) -> Self {
        Self {
            trials: self.trials / 2,
            successes: self.successes / 2,
            ..self
        }
    }

    /// What these counters gained since `base`, an earlier reading of them.
    pub(crate) fn learned_since(self, base: Self) -> Self {
        Self {
            trials: self.trials.saturating_sub(base.trials),
            successes: self.successes.saturating_sub(base.successes),
            total_citations: self.total_citations.saturating_sub(base.total_citations),
            total_search_latency_ms: self
                .total_search_latency_ms
                .saturating_sub(base.total_search_latency_ms),
            total_cost_usd: (self.total_cost_usd - base.total_cost_usd).max(0.0),
            perplexity_requests: self
                .perplexity_requests
                .saturating_sub(base.perplexity_requests),
            total_gemini_thinking_tokens: self
                .total_gemini_thinking_tokens
                .saturating_sub(base.total_gemini_thinking_tokens),
            total_gemini_cached_tokens: self
                .total_gemini_cached_tokens
                .saturating_sub(base.total_gemini_cached_tokens),
            total_gemini_grounding_queries: self
                .total_gemini_grounding_queries
                .saturating_sub(base.total_gemini_grounding_queries),
            gemini_code_execution_successes: self
                .gemini_code_execution_successes
                .saturating_sub(base.gemini_code_execution_successes),
            gemini_code_execution_failures: self
                .gemini_code_execution_failures
                .saturating_sub(base.gemini_code_execution_failures),
            gemini_context_window_le_200k_requests: self
                .gemini_context_window_le_200k_requests
                .saturating_sub(base.gemini_context_window_le_200k_requests),
            gemini_context_window_gt_200k_requests: self
                .gemini_context_window_gt_200k_requests
                .saturating_sub(base.gemini_context_window_gt_200k_requests),
            gemini_requests: self.gemini_requests.saturating_sub(base.gemini_requests),
        }
    }

    /// Add counters learned elsewhere.
    pub(crate) fn absorb(&mut self, learned: Self) {
        self.trials += learned.trials;
        self.successes += learned.successes;
        self.total_citations += learned.total_citations;
        self.total_search_latency_ms += learned.total_search_latency_ms;
        self.total_cost_usd += learned.total_cost_usd;
        self.perplexity_requests += learned.perplexity_requests;
        self.total_gemini_thinking_tokens += learned.total_gemini_thinking_tokens;
        self.total_gemini_cached_tokens += learned.total_gemini_cached_tokens;
        self.total_gemini_grounding_queries += learned.total_gemini_grounding_queries;
        self.gemini_code_execution_successes += learned.gemini_code_execution_successes;
        self.gemini_code_execution_failures += learned.gemini_code_execution_failures;
        self.gemini_context_window_le_200k_requests +=
            learned.gemini_context_window_le_200k_requests;
        self.gemini_context_window_gt_200k_requests +=
            learned.gemini_context_window_gt_200k_requests;
        self.gemini_requests += learned.gemini_requests;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VersionChange {
    Added(String),
    Removed(String),
    Upgraded { old: String, new: String },
}

pub(crate) fn detect_version_changes(
    persisted_slugs: &[String],
    current_slugs: &[String],
) -> Vec<VersionChange> {
    let mut changes = Vec::new();
    let persisted_set: HashSet<&str> = persisted_slugs.iter().map(String::as_str).collect();
    let current_set: HashSet<&str> = current_slugs.iter().map(String::as_str).collect();

    for slug in current_slugs {
        if !persisted_set.contains(slug.as_str()) {
            let prefix = slug
                .rsplit_once('-')
                .map_or(slug.as_str(), |(prefix, _)| prefix);
            if let Some(old) = persisted_slugs
                .iter()
                .find(|candidate| candidate.starts_with(prefix))
            {
                changes.push(VersionChange::Upgraded {
                    old: old.clone(),
                    new: slug.clone(),
                });
            } else {
                changes.push(VersionChange::Added(slug.clone()));
            }
        }
    }

    for slug in persisted_slugs {
        if !current_set.contains(slug.as_str()) {
            changes.push(VersionChange::Removed(slug.clone()));
        }
    }

    changes
}

pub(crate) fn migrated_confidence_stats(
    persisted_stats: &HashMap<String, PersistedModelStats>,
    changes: &[VersionChange],
    active_slugs: &[String],
) -> HashMap<String, PersistedModelStats> {
    let active_set: HashSet<&str> = active_slugs.iter().map(String::as_str).collect();
    let mut migrated = persisted_stats
        .iter()
        .filter(|(slug, _)| active_set.contains(slug.as_str()))
        .map(|(slug, stats)| (slug.clone(), *stats))
        .collect::<HashMap<_, _>>();

    for change in changes {
        if let VersionChange::Upgraded { old, new } = change {
            let Some(old_stats) = persisted_stats.get(old) else {
                continue;
            };
            let transferred = old_stats.weighted_half();
            if transferred.trials == 0 && transferred.successes == 0 {
                continue;
            }

            let entry = migrated
                .entry(new.clone())
                .or_insert(PersistedModelStats::default());
            entry.trials += transferred.trials;
            entry.successes += transferred.successes;
        }
    }

    migrated
}

pub(crate) fn remap_role_table_entry(slug: String, changes: &[VersionChange]) -> String {
    for change in changes {
        if let VersionChange::Upgraded { old, new } = change {
            if slug == *old {
                return new.clone();
            }
        }
    }

    slug
}

/// Add what `current` learned since `base` to `latest`, the snapshot on disk
/// (bug-9c88ac).
///
/// `base` is the state the router last loaded or saved, so `current - base`
/// is its own new learning. Confidence counters and LinUCB `A`/`b` sums are
/// additive, so adding that difference model by model keeps everything other
/// writers saved meanwhile, and a model only `latest` has keeps its state
/// (bug-605a8a). Role-table entries and a Pareto frontier the router changed
/// replace the persisted ones; new models and stage transitions are appended.
pub(crate) fn merge_learning(
    latest: &mut CascadeSnapshot,
    current: &CascadeSnapshot,
    base: &CascadeSnapshot,
) {
    if latest.total_observations == 0 {
        // Snapshots written before the total was persisted count trials, as
        // `CascadeRouter::load_or_new` does.
        latest.total_observations = latest
            .confidence_stats
            .values()
            .map(|stats| stats.trials)
            .sum();
    }
    latest.total_observations += current
        .total_observations
        .saturating_sub(base.total_observations);

    for slug in &current.model_slugs {
        if !latest.model_slugs.contains(slug) {
            latest.model_slugs.push(slug.clone());
        }
    }

    for (slug, stats) in &current.confidence_stats {
        let learned = match base.confidence_stats.get(slug) {
            Some(base_stats) => stats.learned_since(*base_stats),
            None => *stats,
        };
        if learned != PersistedModelStats::default() {
            latest
                .confidence_stats
                .entry(slug.clone())
                .or_default()
                .absorb(learned);
        }
    }

    merge_linucb_arms(latest, current, base);

    let new_transitions = current
        .stage_transitions
        .get(base.stage_transitions.len()..)
        .unwrap_or_default();
    for transition in new_transitions {
        let recorded = latest
            .stage_transitions
            .iter()
            .any(|persisted| persisted.from == transition.from && persisted.to == transition.to);
        if !recorded {
            latest.stage_transitions.push(transition.clone());
        }
    }

    for (role, slug) in &current.role_table {
        if base.role_table.get(role) == Some(slug) {
            latest
                .role_table
                .entry(*role)
                .or_insert_with(|| slug.clone());
        } else {
            latest.role_table.insert(*role, slug.clone());
        }
    }

    if current.pareto_frontier != base.pareto_frontier {
        latest.pareto_frontier.clone_from(&current.pareto_frontier);
    }
}

/// Add the LinUCB `A`/`b` updates `current` made since `base` to the arms in
/// `latest`, matched by slug, and write one arm per model in `model_slugs`
/// order.
///
/// Persisted arms that cannot be matched to a model, or that have another
/// context dimension, cannot be combined with the router's and are reset.
fn merge_linucb_arms(
    latest: &mut CascadeSnapshot,
    current: &CascadeSnapshot,
    base: &CascadeSnapshot,
) {
    let Some(learned) = current.linucb_state.as_ref() else {
        return;
    };
    let dim = learned.dim;
    let fresh_a = identity_matrix(dim);
    let fresh_b = vec![0.0; dim];

    latest.name_legacy_arms();
    let mut arms: HashMap<String, (Vec<f64>, Vec<f64>)> = HashMap::new();
    let mut observations = 0;
    if let Some(persisted) = latest.linucb_state.take() {
        if persisted.dim == dim && persisted.names_every_arm() {
            for slug in &persisted.slugs {
                if let Some((a_matrix, b_vector)) = persisted.arm(slug) {
                    arms.insert(slug.clone(), (a_matrix.to_vec(), b_vector.to_vec()));
                }
            }
            observations = persisted.observations;
        } else {
            tracing::warn!(
                persisted_dim = persisted.dim,
                dim,
                "persisted LinUCB arms cannot be matched to models -- resetting them"
            );
        }
    }

    let base_arms = base.linucb_state.as_ref();
    for slug in &learned.slugs {
        let Some((now_a, now_b)) = learned.arm(slug) else {
            continue;
        };
        let (then_a, then_b) = base_arms
            .and_then(|state| state.arm(slug))
            .unwrap_or((fresh_a.as_slice(), fresh_b.as_slice()));
        if now_a == then_a && now_b == then_b {
            continue;
        }
        let (a_matrix, b_vector) = arms
            .entry(slug.clone())
            .or_insert_with(|| (fresh_a.clone(), fresh_b.clone()));
        add_difference(a_matrix, now_a, then_a);
        add_difference(b_vector, now_b, then_b);
    }
    observations += learned
        .observations
        .saturating_sub(base_arms.map_or(0, |state| state.observations));

    // An arm whose model is missing from the list keeps its state.
    let mut unlisted: Vec<String> = arms
        .keys()
        .filter(|slug| !latest.model_slugs.contains(slug))
        .cloned()
        .collect();
    unlisted.sort();
    latest.model_slugs.extend(unlisted);

    let mut merged = LinUCBSnapshot {
        a_matrices: Vec::with_capacity(latest.model_slugs.len()),
        b_vectors: Vec::with_capacity(latest.model_slugs.len()),
        dim,
        observations,
        slugs: Vec::with_capacity(latest.model_slugs.len()),
    };
    for slug in &latest.model_slugs {
        let (a_matrix, b_vector) = arms
            .remove(slug)
            .unwrap_or_else(|| (fresh_a.clone(), fresh_b.clone()));
        merged.a_matrices.push(a_matrix);
        merged.b_vectors.push(b_vector);
        merged.slugs.push(slug.clone());
    }
    latest.linucb_state = Some(merged);
}

/// A `dim` x `dim` identity matrix, flattened row-major: a fresh arm's `A`.
fn identity_matrix(dim: usize) -> Vec<f64> {
    let mut matrix = vec![0.0; dim * dim];
    for diagonal in matrix.iter_mut().step_by(dim + 1) {
        *diagonal = 1.0;
    }
    matrix
}

/// Add `now - then` to `target`, element by element.
fn add_difference(target: &mut [f64], now: &[f64], then: &[f64]) {
    for ((value, now), then) in target.iter_mut().zip(now).zip(then) {
        *value += now - then;
    }
}
