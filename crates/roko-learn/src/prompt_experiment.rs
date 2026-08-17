//! Lightweight A/B testing framework for prompt section variants.
//!
//! Each experiment tests multiple variants of a prompt section (e.g. a
//! system-prompt paragraph). Variant selection is bandit-driven: exploration
//! favours under-sampled arms, then converges on the best performer once
//! evidence is strong.
//!
//! Persistence is a single JSON file managed by [`ExperimentStore`].

use roko_core::{ContentHash, ExperimentWinnerSummary};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;

use chrono::{DateTime, Utc};

/// Default path for persisted static overrides derived from concluded experiments.
pub const DEFAULT_STATIC_OVERRIDES_PATH: &str = ".roko/learn/static-overrides.json";

// ─── Types ──────────────────────────────────────────────────────────────────

/// A single prompt variant within an experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariant {
    /// Unique identifier for this variant (e.g. "concise-v2").
    pub id: String,
    /// Human-readable label.
    pub name: String,
    /// The prompt section name this replaces (e.g. "constraints").
    pub section_name: String,
    /// The actual prompt text content.
    pub content: String,
    /// Optional model slug when the experiment is selecting among models.
    #[serde(default)]
    pub slug: Option<String>,
    /// Whether this variant is still eligible for selection.
    pub active: bool,
}

/// Winner derived from a concluded prompt experiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentWinner {
    /// Experiment identifier that produced the winner.
    pub experiment_id: String,
    /// Parameter being overridden, typically a role or section name.
    pub parameter: String,
    /// Winning value that should become the new default.
    pub winning_value: String,
    /// Derived confidence in `[0.0, 1.0]`.
    pub confidence: f64,
}

/// Per-variant outcome tracker.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariantStats {
    /// Total number of times this variant has been assigned.
    pub trials: u64,
    /// Number of successful outcomes.
    pub successes: u64,
}

/// Immutable statistics captured when an experiment is auto-promoted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentArchive {
    /// Conclusion timestamp.
    pub concluded_at: DateTime<Utc>,
    /// Final per-variant counters.
    pub final_stats: HashMap<String, VariantStats>,
    /// Chi-squared p-value for the two leading variants.
    pub p_value: f64,
    /// Absolute success-rate difference between the leaders.
    pub effect_size: f64,
}

impl VariantStats {
    /// Empirical success rate.
    #[allow(clippy::cast_precision_loss)]
    pub fn success_rate(&self) -> f64 {
        if self.trials == 0 {
            0.0
        } else {
            self.successes as f64 / self.trials as f64
        }
    }

    /// UCB1-style score for arm selection (upper confidence bound).
    #[allow(clippy::cast_precision_loss)]
    fn ucb_score(&self, total_trials: u64) -> f64 {
        if self.trials == 0 {
            return f64::MAX; // Explore unsampled arms first.
        }
        let mean = self.successes as f64 / self.trials as f64;
        let exploration = (2.0 * (total_trials as f64).ln() / self.trials as f64).sqrt();
        mean + exploration
    }

    /// Wilson 95% confidence interval for the empirical success rate.
    #[allow(clippy::cast_precision_loss)]
    fn confidence_interval_95(&self) -> (f64, f64) {
        if self.trials == 0 {
            return (0.0, 0.0);
        }

        let n = self.trials as f64;
        let p = self.success_rate();
        let z = 1.96_f64;
        let z_sq = z * z;
        let denom = 1.0 + z_sq / n;
        let center = (p + z_sq / (2.0 * n)) / denom;
        let margin = (z / denom) * ((p * (1.0 - p) / n + z_sq / (4.0 * n * n)).sqrt());
        (
            (center - margin).clamp(0.0, 1.0),
            (center + margin).clamp(0.0, 1.0),
        )
    }
}

/// Per-variant metric tracker.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct VariantMetricStats {
    /// Number of metric observations.
    samples: u64,
    /// Sum of all recorded metric values.
    sum: f64,
    /// Most recent metric observation.
    last: Option<f64>,
}

impl VariantMetricStats {
    /// Record one metric observation.
    fn record(&mut self, value: f64) {
        self.samples += 1;
        self.sum += value;
        self.last = Some(value);
    }
}

/// Status of a prompt experiment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperimentStatus {
    /// Experiment is actively assigning variants.
    Running,
    /// A winner has been identified and the experiment is concluded.
    Concluded,
}

/// Durable identity of one runner attempt receiving prompt treatments.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PromptAttemptKey {
    /// Durable runner invocation id.
    pub run_id: String,
    /// Plan containing the task.
    pub plan_id: String,
    /// Task receiving the prompt.
    pub task_id: String,
    /// Monotonic task attempt number.
    pub attempt: u32,
}

impl PromptAttemptKey {
    /// Construct an attempt identity.
    pub fn new(
        run_id: impl Into<String>,
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        attempt: u32,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            attempt,
        }
    }
}

/// Durable lifecycle of a prompt-experiment assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptAssignmentState {
    /// Treatment selected and reserved, but no provider side effect started.
    Prepared,
    /// The exact composed prompt was handed to a provider launch boundary.
    Dispatched,
    /// A dispatched treatment received a terminal success/failure observation.
    Observed,
    /// The treatment never produced an eligible observation.
    Abandoned,
}

/// Attempt-level terminal disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentSettlement {
    /// The provider-backed attempt completed with this experiment outcome.
    Observed {
        /// Whether the attempt satisfied its outcome gate.
        success: bool,
    },
    /// The attempt was abandoned and must not update experiment statistics.
    Abandoned,
}

/// Durable assignment and audit receipt for one prompt treatment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptExperimentAssignment {
    /// Content-addressed deterministic assignment identifier.
    pub assignment_id: String,
    /// Runner attempt receiving this treatment.
    pub attempt_key: PromptAttemptKey,
    /// Experiment that selected the variant.
    pub experiment_id: String,
    /// Selected variant identifier.
    pub variant_id: String,
    /// Optional role scope of the selected experiment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Canonical prompt section replaced by the treatment.
    pub section_name: String,
    /// Exact treatment content retained until terminal settlement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_snapshot: Option<String>,
    /// Durable BLAKE3 hash of the treatment content.
    pub content_hash: String,
    /// Hash of the exact final prompt handed to the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
    /// Assignment lifecycle state.
    pub state: PromptAssignmentState,
    /// Observed outcome, present only for [`PromptAssignmentState::Observed`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// Whether this treatment was assigned while the experiment was running.
    /// Concluded sticky winners remain auditable but never reserve or update a
    /// learning trial.
    #[serde(default)]
    learning_eligible: bool,
}

/// Durable attempt bucket used to make preparation, dispatch, and settlement
/// idempotent across process crashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PromptAttemptAssignments {
    attempt_key: PromptAttemptKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(default)]
    eligible_sections: Vec<String>,
    #[serde(default)]
    assignments: Vec<PromptExperimentAssignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prompt_hash: Option<String>,
    /// Canonical assignment-id subset actually included in the dispatched
    /// prompt. `None` is reserved for stores written before subset tracking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    included_assignment_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    settlement: Option<AssignmentSettlement>,
}

/// Typed failures from durable assignment lifecycle operations.
#[derive(Debug, thiserror::Error)]
pub enum PromptAssignmentError {
    /// Filesystem or persisted-JSON failure.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// The request conflicts with already-durable attempt state.
    #[error("prompt assignment conflict: {0}")]
    Conflict(String),
    /// No durable assignment bucket exists for the requested attempt.
    #[error("prompt assignment attempt not found: {0}")]
    AttemptNotFound(String),
}

/// A prompt experiment tracks multiple variants for one prompt section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptExperiment {
    /// Unique experiment identifier.
    pub experiment_id: String,
    /// The prompt section under test.
    pub section_name: String,
    /// Optional agent role label when the experiment is selecting a role model.
    #[serde(default)]
    pub role: Option<String>,
    /// Available variants.
    pub variants: Vec<PromptVariant>,
    /// Per-variant statistics, keyed by variant id.
    pub stats: HashMap<String, VariantStats>,
    /// Per-variant metric observations, keyed by variant id.
    #[serde(default)]
    metric_stats: HashMap<String, VariantMetricStats>,
    /// Current experiment status.
    pub status: ExperimentStatus,
    /// Variant id of the winner, if concluded.
    pub winner_id: Option<String>,
    /// Minimum trials per variant before considering conclusion.
    pub min_trials_per_variant: u64,
    /// Required difference in success rate to declare a winner.
    pub min_effect_size: f64,
    /// Final statistics retained after automatic promotion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<ExperimentArchive>,
}

impl PromptExperiment {
    /// Create a new running experiment.
    pub fn new(
        experiment_id: impl Into<String>,
        section_name: impl Into<String>,
        variants: Vec<PromptVariant>,
    ) -> Self {
        let stats: HashMap<String, VariantStats> = variants
            .iter()
            .map(|v| (v.id.clone(), VariantStats::default()))
            .collect();
        Self {
            experiment_id: experiment_id.into(),
            section_name: section_name.into(),
            role: None,
            variants,
            stats,
            metric_stats: HashMap::new(),
            status: ExperimentStatus::Running,
            winner_id: None,
            min_trials_per_variant: 10,
            min_effect_size: 0.1,
            archive: None,
        }
    }

    /// Select the next variant to use via UCB1.
    ///
    /// Returns `None` if the experiment is concluded.
    pub fn assign_variant(&self) -> Option<&PromptVariant> {
        if self.status == ExperimentStatus::Concluded {
            // Return the winner if concluded.
            return self
                .winner_id
                .as_ref()
                .and_then(|wid| self.variants.iter().find(|v| v.id == *wid));
        }

        let total: u64 = self.stats.values().map(|s| s.trials).sum();
        let mut best_variant = None;
        let mut best_score = f64::NEG_INFINITY;

        for variant in &self.variants {
            if !variant.active {
                continue;
            }
            let stats = self.stats.get(&variant.id).cloned().unwrap_or_default();
            let score = stats.ucb_score(total);
            if score > best_score {
                best_score = score;
                best_variant = Some(variant);
            }
        }

        best_variant
    }

    /// Record an outcome for a variant. Returns true if the experiment concluded.
    pub fn record_outcome(&mut self, variant_id: &str, success: bool) -> bool {
        if let Some(stats) = self.stats.get_mut(variant_id) {
            stats.trials += 1;
            if success {
                stats.successes += 1;
            }
        }

        // Check for conclusion.
        if self.status == ExperimentStatus::Running {
            if let Some(winner) = self.check_conclusion() {
                self.status = ExperimentStatus::Concluded;
                self.winner_id = Some(winner);
                self.archive = Some(self.build_archive());
                return true;
            }
        }
        false
    }

    /// Record a numeric metric for a variant.
    pub fn record_metric(&mut self, variant_id: &str, metric: f64) {
        if !metric.is_finite() {
            return;
        }

        if self.stats.contains_key(variant_id) {
            self.metric_stats
                .entry(variant_id.to_string())
                .or_default()
                .record(metric);
        }
    }

    /// Check if we have enough data to declare a winner.
    ///
    /// Requires enough evidence, a practically meaningful effect, and either
    /// p < 0.05 or non-overlapping Wilson 95% intervals after 50 total trials.
    fn check_conclusion(&self) -> Option<String> {
        let active_stats: Vec<(&str, &VariantStats)> = self
            .variants
            .iter()
            .filter(|v| v.active)
            .filter_map(|v| self.stats.get(&v.id).map(|s| (v.id.as_str(), s)))
            .collect();

        if active_stats.len() < 2 {
            return active_stats.first().map(|(id, _)| (*id).to_string());
        }

        // All variants must meet minimum trials.
        if active_stats
            .iter()
            .any(|(_, s)| s.trials < self.min_trials_per_variant)
        {
            return None;
        }

        // Sort by success rate descending.
        let mut ranked: Vec<_> = active_stats
            .iter()
            .map(|(id, s)| (*id, s.success_rate()))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_id, best_rate) = ranked[0];
        let (_, second_rate) = ranked[1];

        let best_stats = active_stats
            .iter()
            .find(|(id, _)| *id == best_id)
            .map(|(_, stats)| *stats)?;
        let second_id = ranked[1].0;
        let second_stats = active_stats
            .iter()
            .find(|(id, _)| *id == second_id)
            .map(|(_, stats)| *stats)?;
        let (_, p_value) = chi_squared_test(best_stats, second_stats);
        let significant = p_value < 0.05;
        let early = self.early_stopping_check().as_deref() == Some(best_id);

        if best_rate - second_rate >= self.min_effect_size && (significant || early) {
            Some(best_id.to_string())
        } else {
            None
        }
    }

    /// Return the likely winner when Wilson 95% intervals no longer overlap
    /// after at least 50 observations across active variants.
    #[must_use]
    pub fn early_stopping_check(&self) -> Option<String> {
        let mut ranked = self
            .variants
            .iter()
            .filter(|variant| variant.active)
            .filter_map(|variant| {
                self.stats
                    .get(&variant.id)
                    .map(|stats| (&variant.id, stats))
            })
            .collect::<Vec<_>>();
        if ranked.len() < 2 || ranked.iter().map(|(_, stats)| stats.trials).sum::<u64>() < 50 {
            return None;
        }
        ranked.sort_by(|a, b| b.1.success_rate().total_cmp(&a.1.success_rate()));
        let best_interval = ranked[0].1.confidence_interval_95();
        let second_interval = ranked[1].1.confidence_interval_95();
        (best_interval.0 > second_interval.1).then(|| ranked[0].0.clone())
    }

    fn build_archive(&self) -> ExperimentArchive {
        let mut ranked = self
            .variants
            .iter()
            .filter(|variant| variant.active)
            .filter_map(|variant| self.stats.get(&variant.id))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.success_rate().total_cmp(&a.success_rate()));
        let (p_value, effect_size) = if ranked.len() >= 2 {
            let (_, p_value) = chi_squared_test(ranked[0], ranked[1]);
            (
                p_value,
                (ranked[0].success_rate() - ranked[1].success_rate()).abs(),
            )
        } else {
            (0.0, 1.0)
        };
        ExperimentArchive {
            concluded_at: Utc::now(),
            final_stats: self.stats.clone(),
            p_value,
            effect_size,
        }
    }

    /// Return a concluded winner when the experiment has enough evidence.
    #[must_use]
    pub fn concluded_winner(&self) -> Option<ExperimentWinner> {
        if self.status != ExperimentStatus::Concluded {
            return None;
        }

        let winner_id = self.winner_id.as_deref()?;
        let winner = self
            .variants
            .iter()
            .find(|variant| variant.id == winner_id)?;
        let confidence = self.winner_confidence(winner_id)?;
        if confidence < 0.95 {
            return None;
        }

        Some(ExperimentWinner {
            experiment_id: self.experiment_id.clone(),
            parameter: self
                .role
                .clone()
                .unwrap_or_else(|| self.section_name.clone()),
            winning_value: winner
                .slug
                .clone()
                .unwrap_or_else(|| winner.content.clone()),
            confidence,
        })
    }

    /// Return a detailed summary for dashboard rendering.
    #[must_use]
    pub fn winner_summary(&self) -> Option<ExperimentWinnerSummary> {
        let winner = self.concluded_winner()?;
        let winner_id = self.winner_id.as_deref()?;
        let winner_variant = self
            .variants
            .iter()
            .find(|variant| variant.id == winner_id)?;
        let winner_stats = self.stats.get(winner_id)?;
        let (ci_lower, ci_upper) = winner_stats.confidence_interval_95();

        Some(ExperimentWinnerSummary {
            experiment_id: self.experiment_id.clone(),
            parameter: winner.parameter,
            winner: winner_variant_label(winner_variant),
            winner_variant_id: winner_variant.id.clone(),
            win_rate: winner_stats.success_rate(),
            sample_size: winner_stats.trials,
            ci_lower,
            ci_upper,
            confidence: winner.confidence,
        })
    }

    fn winner_confidence(&self, winner_id: &str) -> Option<f64> {
        let mut ranked: Vec<(&str, &VariantStats, f64)> = self
            .variants
            .iter()
            .filter(|variant| variant.active)
            .filter_map(|variant| {
                self.stats
                    .get(&variant.id)
                    .map(|stats| (variant.id.as_str(), stats, stats.success_rate()))
            })
            .collect();
        if ranked.is_empty() {
            return None;
        }

        ranked.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        let (winner_ranked_id, winner_stats, winner_rate) = ranked
            .iter()
            .find(|(id, _, _)| *id == winner_id)
            .copied()
            .unwrap_or(ranked[0]);
        let second = ranked.iter().find(|(id, _, _)| *id != winner_ranked_id);
        let second_rate = second.map_or(0.0, |(_, _, rate)| *rate);

        let second_stats = second.map(|(_, stats, _)| *stats);
        let se = match second_stats {
            Some(second_stats) => {
                let winner_trials = winner_stats.trials.max(1) as f64;
                let second_trials = second_stats.trials.max(1) as f64;
                let winner_var = winner_rate * (1.0 - winner_rate) / winner_trials;
                let second_var = second_rate * (1.0 - second_rate) / second_trials;
                (winner_var + second_var).sqrt()
            }
            None => 0.0,
        };
        let gap = (winner_rate - second_rate).max(0.0);
        if se == 0.0 {
            Some(1.0)
        } else {
            Some((gap / (gap + se)).clamp(0.0, 1.0))
        }
    }
}

/// Pearson chi-squared test for two binary-outcome variants.
///
/// Returns `(statistic, p_value)` with one degree of freedom. Degenerate
/// tables return a non-significant p-value instead of producing NaN.
#[must_use]
pub fn chi_squared_test(stats_a: &VariantStats, stats_b: &VariantStats) -> (f64, f64) {
    if stats_a.trials == 0 || stats_b.trials == 0 {
        return (0.0, 1.0);
    }
    let a_success = stats_a.successes.min(stats_a.trials) as f64;
    let b_success = stats_b.successes.min(stats_b.trials) as f64;
    let a_total = stats_a.trials as f64;
    let b_total = stats_b.trials as f64;
    let successes = a_success + b_success;
    let total = a_total + b_total;
    let failures = total - successes;
    if successes <= f64::EPSILON || failures <= f64::EPSILON {
        return (0.0, 1.0);
    }
    let expected_a_success = a_total * successes / total;
    let expected_b_success = b_total * successes / total;
    let cells = [
        (a_success, expected_a_success),
        (a_total - a_success, a_total - expected_a_success),
        (b_success, expected_b_success),
        (b_total - b_success, b_total - expected_b_success),
    ];
    let statistic = cells
        .iter()
        .filter(|(_, expected)| *expected > f64::EPSILON)
        .map(|(observed, expected)| (observed - expected).powi(2) / expected)
        .sum::<f64>();
    (statistic, erfc((statistic / 2.0).sqrt()).clamp(0.0, 1.0))
}

// Abramowitz and Stegun 7.1.26; ample precision for an experiment gate.
fn erfc(value: f64) -> f64 {
    let x = value.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let polynomial =
        (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t
            + 0.254_829_592)
            * t;
    let erf = 1.0 - polynomial * (-x * x).exp();
    if value >= 0.0 { 1.0 - erf } else { 1.0 + erf }
}

// ─── Store ──────────────────────────────────────────────────────────────────

/// Persisted experiment store: manages all active and concluded experiments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentStore {
    experiments: HashMap<String, PromptExperiment>,
    /// Attempt-scoped assignment receipts. The map key is a deterministic hash
    /// of run/plan/task/attempt; the full identity is retained in each bucket.
    #[serde(default)]
    attempt_assignments: BTreeMap<String, PromptAttemptAssignments>,
}

impl ExperimentStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            experiments: HashMap::new(),
            attempt_assignments: BTreeMap::new(),
        }
    }

    /// Strictly load a store, returning an empty store only when it is absent.
    ///
    /// # Errors
    ///
    /// Malformed JSON is returned as [`io::ErrorKind::InvalidData`]. The source
    /// file is never rewritten by this read-only operation.
    pub fn load_strict(path: &Path) -> io::Result<Self> {
        roko_fs::read_json_or_default_strict(path)
    }

    /// Strict read/mutate/atomic-write transaction under the store's stable
    /// sibling advisory lock.
    ///
    /// # Errors
    ///
    /// Returns strict load, mutation, or atomic publication errors. Malformed
    /// input and failed mutations leave the existing file untouched.
    pub fn transaction<R>(
        path: &Path,
        transaction: impl FnOnce(&mut Self) -> io::Result<R>,
    ) -> io::Result<R> {
        roko_fs::with_locked_json_transaction::<Self, R, io::Error, _>(path, transaction)
    }

    /// Load from a JSON file, or create empty if missing/corrupt.
    pub fn load_or_new(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save to a JSON file (atomic write).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be serialized or if the output
    /// file cannot be created, written, or renamed.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        roko_fs::atomic_write_json(path, self)
    }

    /// Register a new experiment. No-op if `experiment_id` already exists.
    pub fn register(&mut self, experiment: PromptExperiment) {
        self.experiments
            .entry(experiment.experiment_id.clone())
            .or_insert(experiment);
    }

    /// Look up an experiment by id.
    pub fn get(&self, experiment_id: &str) -> Option<&PromptExperiment> {
        self.experiments.get(experiment_id)
    }

    /// Find a running experiment for the given prompt section name.
    pub fn active_for_section(&self, section_name: &str) -> Option<&PromptExperiment> {
        self.experiments
            .values()
            .find(|e| e.section_name == section_name && e.status == ExperimentStatus::Running)
    }

    /// Assign a variant for a given prompt section, if an active experiment exists.
    ///
    /// Returns `(variant_id, variant_content)` or `None` if no experiment.
    pub fn assign_variant(&self, experiment_name: &str) -> Option<(String, String)> {
        let experiment = self
            .experiments
            .values()
            .find(|e| e.experiment_id == experiment_name || e.section_name == experiment_name)?;
        let variant = experiment.assign_variant()?;
        Some((variant.id.clone(), variant.content.clone()))
    }

    /// Assign a variant for a given prompt section, if an active experiment exists.
    ///
    /// Returns `(variant_id, variant_content)` or `None` if no experiment.
    pub fn assign_variant_for_section(&self, section_name: &str) -> Option<(String, String)> {
        self.assign_variant(section_name)
    }

    /// Prepare deterministic role/section treatments for one durable attempt.
    ///
    /// Preparation reserves running-experiment arms without counting a trial.
    /// Repeating the identical request returns the same content snapshots.
    /// Multiple applicable experiments for one role/section are rejected
    /// instead of selecting by hash-map iteration order. Concluded experiments
    /// return their persisted winner as a sticky, non-learning treatment.
    ///
    /// # Errors
    ///
    /// Returns strict persistence failures, invalid attempt identity, overlap,
    /// or a request that conflicts with an existing attempt bucket.
    pub fn prepare_attempt_assignments(
        path: &Path,
        attempt_key: &PromptAttemptKey,
        role: Option<&str>,
        eligible_sections: &[&str],
    ) -> Result<Vec<PromptExperimentAssignment>, PromptAssignmentError> {
        validate_attempt_key(attempt_key)?;
        let role = normalize_optional(role);
        let eligible_sections = normalize_sections(eligible_sections)?;
        roko_fs::with_locked_json_transaction::<Self, _, PromptAssignmentError, _>(path, |store| {
            store.prepare_attempt_assignments_unlocked(
                attempt_key,
                role.as_deref(),
                &eligible_sections,
            )
        })
    }

    /// Mark the exact subset of prepared treatments included in the final
    /// prompt as dispatched immediately before provider launch.
    ///
    /// Assignment ids must be sorted, unique, and belong to this attempt.
    /// Repeating the same hash and subset is idempotent. A different hash or
    /// subset, missing preparation, or terminal attempt is a conflict.
    pub fn mark_attempt_dispatched(
        path: &Path,
        attempt_key: &PromptAttemptKey,
        prompt_hash: &str,
        included_assignment_ids: &[&str],
    ) -> Result<Vec<PromptExperimentAssignment>, PromptAssignmentError> {
        let prompt_hash = prompt_hash.trim();
        if prompt_hash.is_empty() {
            return Err(PromptAssignmentError::Conflict(
                "dispatch prompt hash cannot be empty".to_string(),
            ));
        }
        let included_assignment_ids = validate_included_assignment_ids(included_assignment_ids)?;
        let bucket_id = attempt_bucket_id(attempt_key);
        roko_fs::with_locked_json_transaction::<Self, _, PromptAssignmentError, _>(path, |store| {
            let bucket = store
                .attempt_assignments
                .get_mut(&bucket_id)
                .ok_or_else(|| PromptAssignmentError::AttemptNotFound(bucket_id.clone()))?;
            if bucket.attempt_key != *attempt_key {
                return Err(PromptAssignmentError::Conflict(format!(
                    "attempt bucket collision for {bucket_id}"
                )));
            }
            if bucket.settlement.is_some() {
                return Err(PromptAssignmentError::Conflict(format!(
                    "attempt {bucket_id} is already settled"
                )));
            }

            let mut known_assignment_ids = bucket
                .assignments
                .iter()
                .map(|assignment| assignment.assignment_id.as_str())
                .collect::<Vec<_>>();
            known_assignment_ids.sort_unstable();
            if known_assignment_ids
                .windows(2)
                .any(|pair| pair[0] == pair[1])
            {
                return Err(PromptAssignmentError::Conflict(format!(
                    "attempt {bucket_id} contains duplicate assignment ids"
                )));
            }
            if let Some(unknown_id) = included_assignment_ids.iter().find(|assignment_id| {
                known_assignment_ids
                    .binary_search(&assignment_id.as_str())
                    .is_err()
            }) {
                return Err(PromptAssignmentError::Conflict(format!(
                    "assignment {unknown_id:?} does not belong to attempt {bucket_id}"
                )));
            }

            if let Some(existing_hash) = bucket.prompt_hash.as_deref() {
                if existing_hash != prompt_hash {
                    return Err(PromptAssignmentError::Conflict(format!(
                        "attempt {bucket_id} was dispatched with a different prompt hash"
                    )));
                }

                // Ledgers written before subset tracking can recover the
                // original full inclusion set from dispatched assignment rows.
                let existing_included =
                    bucket.included_assignment_ids.clone().unwrap_or_else(|| {
                        let mut ids = bucket
                            .assignments
                            .iter()
                            .filter(|assignment| {
                                assignment.state == PromptAssignmentState::Dispatched
                            })
                            .map(|assignment| assignment.assignment_id.clone())
                            .collect::<Vec<_>>();
                        ids.sort();
                        ids
                    });
                if existing_included != included_assignment_ids {
                    return Err(PromptAssignmentError::Conflict(format!(
                        "attempt {bucket_id} was dispatched with a different assignment subset"
                    )));
                }
                if bucket.assignments.iter().any(|assignment| {
                    let included = included_assignment_ids
                        .binary_search(&assignment.assignment_id)
                        .is_ok();
                    if included {
                        assignment.state != PromptAssignmentState::Dispatched
                            || assignment.prompt_hash.as_deref() != Some(prompt_hash)
                    } else {
                        assignment.state != PromptAssignmentState::Prepared
                            || assignment.prompt_hash.is_some()
                    }
                }) {
                    return Err(PromptAssignmentError::Conflict(format!(
                        "attempt {bucket_id} has inconsistent dispatched assignments"
                    )));
                }
                bucket.included_assignment_ids = Some(included_assignment_ids.clone());
                return Ok(bucket.assignments.clone());
            }

            if bucket
                .assignments
                .iter()
                .any(|assignment| assignment.state != PromptAssignmentState::Prepared)
            {
                return Err(PromptAssignmentError::Conflict(format!(
                    "attempt {bucket_id} contains a non-prepared assignment"
                )));
            }
            bucket.prompt_hash = Some(prompt_hash.to_string());
            bucket.included_assignment_ids = Some(included_assignment_ids.clone());
            for assignment in &mut bucket.assignments {
                if included_assignment_ids
                    .binary_search(&assignment.assignment_id)
                    .is_ok()
                {
                    assignment.state = PromptAssignmentState::Dispatched;
                    assignment.prompt_hash = Some(prompt_hash.to_string());
                }
            }
            Ok(bucket.assignments.clone())
        })
    }

    /// Atomically settle every treatment belonging to one attempt.
    ///
    /// Prepared treatments become abandoned and never count a trial. Dispatched
    /// observed treatments update their exact experiment/variant once;
    /// dispatched abandoned treatments do not. Repeating the same settlement is
    /// a no-op, while a different terminal result is rejected.
    pub fn settle_attempt(
        path: &Path,
        attempt_key: &PromptAttemptKey,
        settlement: AssignmentSettlement,
    ) -> Result<Vec<PromptExperimentAssignment>, PromptAssignmentError> {
        let bucket_id = attempt_bucket_id(attempt_key);
        roko_fs::with_locked_json_transaction::<Self, _, PromptAssignmentError, _>(path, |store| {
            store.settle_attempt_unlocked(&bucket_id, attempt_key, settlement)
        })
    }

    /// Return durable assignment receipts for an attempt.
    #[must_use]
    pub fn assignments_for_attempt(
        &self,
        attempt_key: &PromptAttemptKey,
    ) -> Option<&[PromptExperimentAssignment]> {
        let bucket = self
            .attempt_assignments
            .get(&attempt_bucket_id(attempt_key))?;
        (bucket.attempt_key == *attempt_key).then_some(bucket.assignments.as_slice())
    }

    fn prepare_attempt_assignments_unlocked(
        &mut self,
        attempt_key: &PromptAttemptKey,
        role: Option<&str>,
        eligible_sections: &[String],
    ) -> Result<Vec<PromptExperimentAssignment>, PromptAssignmentError> {
        let bucket_id = attempt_bucket_id(attempt_key);
        if let Some(existing) = self.attempt_assignments.get(&bucket_id) {
            if existing.attempt_key != *attempt_key {
                return Err(PromptAssignmentError::Conflict(format!(
                    "attempt bucket collision for {bucket_id}"
                )));
            }
            if existing.role.as_deref() != role || existing.eligible_sections != eligible_sections {
                return Err(PromptAssignmentError::Conflict(format!(
                    "attempt {bucket_id} was prepared with a different role or section set"
                )));
            }
            return Ok(existing.assignments.clone());
        }

        // Resolve every treatment before inserting anything, so overlap or a
        // malformed experiment cannot publish a partial reservation.
        let mut assignments = Vec::new();
        for section_name in eligible_sections {
            let mut candidates = self
                .experiments
                .values()
                .filter(|experiment| experiment.section_name == *section_name)
                .filter(|experiment| experiment_matches_role(experiment, role))
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| left.experiment_id.cmp(&right.experiment_id));
            if candidates.len() > 1 {
                let experiment_ids = candidates
                    .iter()
                    .map(|experiment| experiment.experiment_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(PromptAssignmentError::Conflict(format!(
                    "role {:?} section {section_name:?} is covered by overlapping experiments: {experiment_ids}",
                    role
                )));
            }
            let Some(experiment) = candidates.first().copied() else {
                continue;
            };

            let learning_eligible = experiment.status == ExperimentStatus::Running;
            let variant = if learning_eligible {
                self.select_variant_with_reservations(experiment, &attempt_key.run_id)
                    .ok_or_else(|| {
                        PromptAssignmentError::Conflict(format!(
                            "running experiment {:?} has no active variant",
                            experiment.experiment_id
                        ))
                    })?
            } else {
                experiment
                    .winner_id
                    .as_deref()
                    .and_then(|winner_id| {
                        experiment
                            .variants
                            .iter()
                            .find(|variant| variant.id == winner_id)
                    })
                    .cloned()
                    .ok_or_else(|| {
                        PromptAssignmentError::Conflict(format!(
                            "concluded experiment {:?} has no persisted winner variant",
                            experiment.experiment_id
                        ))
                    })?
            };
            if variant.section_name != *section_name {
                return Err(PromptAssignmentError::Conflict(format!(
                    "variant {:?} targets section {:?}, not experiment section {:?}",
                    variant.id, variant.section_name, section_name
                )));
            }

            assignments.push(PromptExperimentAssignment {
                assignment_id: assignment_id(
                    attempt_key,
                    &experiment.experiment_id,
                    &variant.id,
                    role,
                    section_name,
                ),
                attempt_key: attempt_key.clone(),
                experiment_id: experiment.experiment_id.clone(),
                variant_id: variant.id,
                role: role.map(str::to_string),
                section_name: section_name.clone(),
                content_hash: ContentHash::of(variant.content.as_bytes()).to_hex(),
                content_snapshot: Some(variant.content),
                prompt_hash: None,
                state: PromptAssignmentState::Prepared,
                success: None,
                learning_eligible,
            });
        }
        assignments.sort_by(|left, right| {
            left.section_name
                .cmp(&right.section_name)
                .then_with(|| left.experiment_id.cmp(&right.experiment_id))
                .then_with(|| left.variant_id.cmp(&right.variant_id))
        });

        // The runner asks for treatments whenever an experiments store exists.
        // Do not permanently grow it for unrelated roles/sections.
        if assignments.is_empty() {
            return Ok(assignments);
        }

        self.attempt_assignments.insert(
            bucket_id,
            PromptAttemptAssignments {
                attempt_key: attempt_key.clone(),
                role: role.map(str::to_string),
                eligible_sections: eligible_sections.to_vec(),
                assignments: assignments.clone(),
                prompt_hash: None,
                included_assignment_ids: None,
                settlement: None,
            },
        );
        Ok(assignments)
    }

    fn select_variant_with_reservations(
        &self,
        experiment: &PromptExperiment,
        current_run_id: &str,
    ) -> Option<PromptVariant> {
        let mut variants = experiment
            .variants
            .iter()
            .filter(|variant| variant.active)
            .collect::<Vec<_>>();
        variants.sort_by(|left, right| left.id.cmp(&right.id));
        let effective_trials = variants
            .iter()
            .map(|variant| {
                experiment
                    .stats
                    .get(&variant.id)
                    .map_or(0, |stats| stats.trials)
                    + self.outstanding_reservations(
                        current_run_id,
                        &experiment.experiment_id,
                        &variant.id,
                    )
            })
            .sum::<u64>();

        variants
            .into_iter()
            .map(|variant| {
                let stats = experiment
                    .stats
                    .get(&variant.id)
                    .cloned()
                    .unwrap_or_default();
                let reserved = self.outstanding_reservations(
                    current_run_id,
                    &experiment.experiment_id,
                    &variant.id,
                );
                let effective_variant_trials = stats.trials + reserved;
                let score = if effective_variant_trials == 0 {
                    f64::MAX
                } else {
                    let exploration = (2.0 * (effective_trials.max(1) as f64).ln()
                        / effective_variant_trials as f64)
                        .sqrt();
                    stats.success_rate() + exploration
                };
                (variant, score)
            })
            .max_by(|(left_variant, left_score), (right_variant, right_score)| {
                left_score
                    .total_cmp(right_score)
                    // `max_by` keeps the greater item; reverse the id tie-break
                    // so deterministic preparation chooses the lower id.
                    .then_with(|| right_variant.id.cmp(&left_variant.id))
            })
            .map(|(variant, _)| variant.clone())
    }

    fn outstanding_reservations(
        &self,
        current_run_id: &str,
        experiment_id: &str,
        variant_id: &str,
    ) -> u64 {
        self.attempt_assignments
            .values()
            .filter(|bucket| bucket.attempt_key.run_id == current_run_id)
            .flat_map(|bucket| &bucket.assignments)
            .filter(|assignment| assignment.learning_eligible)
            .filter(|assignment| assignment.experiment_id == experiment_id)
            .filter(|assignment| assignment.variant_id == variant_id)
            .filter(|assignment| {
                matches!(
                    assignment.state,
                    PromptAssignmentState::Prepared | PromptAssignmentState::Dispatched
                )
            })
            .count() as u64
    }

    fn settle_attempt_unlocked(
        &mut self,
        bucket_id: &str,
        attempt_key: &PromptAttemptKey,
        settlement: AssignmentSettlement,
    ) -> Result<Vec<PromptExperimentAssignment>, PromptAssignmentError> {
        let bucket = self
            .attempt_assignments
            .get(bucket_id)
            .ok_or_else(|| PromptAssignmentError::AttemptNotFound(bucket_id.to_string()))?;
        if bucket.attempt_key != *attempt_key {
            return Err(PromptAssignmentError::Conflict(format!(
                "attempt bucket collision for {bucket_id}"
            )));
        }
        if let Some(existing) = bucket.settlement {
            if existing == settlement {
                return Ok(bucket.assignments.clone());
            }
            return Err(PromptAssignmentError::Conflict(format!(
                "attempt {bucket_id} already has a different settlement"
            )));
        }

        let observations = match settlement {
            AssignmentSettlement::Observed { success } => bucket
                .assignments
                .iter()
                .filter(|assignment| {
                    assignment.state == PromptAssignmentState::Dispatched
                        && assignment.learning_eligible
                })
                .map(|assignment| {
                    (
                        assignment.experiment_id.clone(),
                        assignment.variant_id.clone(),
                        success,
                    )
                })
                .collect::<Vec<_>>(),
            AssignmentSettlement::Abandoned => Vec::new(),
        };

        // Validate every scoped target before changing any statistics.
        for (experiment_id, variant_id, _) in &observations {
            let experiment = self.experiments.get(experiment_id).ok_or_else(|| {
                PromptAssignmentError::Conflict(format!(
                    "assignment references missing experiment {experiment_id:?}"
                ))
            })?;
            if !experiment.stats.contains_key(variant_id) {
                return Err(PromptAssignmentError::Conflict(format!(
                    "assignment references missing variant {variant_id:?} in experiment {experiment_id:?}"
                )));
            }
        }
        for (experiment_id, variant_id, success) in &observations {
            if !self.record_outcome_for_experiment(experiment_id, variant_id, *success) {
                return Err(PromptAssignmentError::Conflict(format!(
                    "could not settle {experiment_id:?}/{variant_id:?}"
                )));
            }
        }

        let bucket = self
            .attempt_assignments
            .get_mut(bucket_id)
            .expect("validated attempt bucket remains present");
        for assignment in &mut bucket.assignments {
            match assignment.state {
                PromptAssignmentState::Prepared => {
                    assignment.state = PromptAssignmentState::Abandoned;
                    assignment.success = None;
                }
                PromptAssignmentState::Dispatched => match settlement {
                    AssignmentSettlement::Observed { success } => {
                        assignment.state = PromptAssignmentState::Observed;
                        assignment.success = Some(success);
                    }
                    AssignmentSettlement::Abandoned => {
                        assignment.state = PromptAssignmentState::Abandoned;
                        assignment.success = None;
                    }
                },
                PromptAssignmentState::Observed | PromptAssignmentState::Abandoned => {
                    return Err(PromptAssignmentError::Conflict(format!(
                        "attempt {bucket_id} contains terminal assignments without a settlement"
                    )));
                }
            }
            assignment.content_snapshot = None;
        }
        bucket.settlement = Some(settlement);
        Ok(bucket.assignments.clone())
    }

    /// Return all concluded experiments with sufficiently high confidence.
    #[must_use]
    pub fn concluded_winners(&self) -> Vec<ExperimentWinner> {
        let mut winners: Vec<_> = self
            .experiments
            .values()
            .filter_map(PromptExperiment::concluded_winner)
            .collect();
        winners.sort_by(|a, b| {
            b.confidence
                .total_cmp(&a.confidence)
                .then_with(|| a.experiment_id.cmp(&b.experiment_id))
        });
        winners
    }

    /// Return concluded winners with confidence intervals for dashboard rendering.
    #[must_use]
    pub fn winner_summaries(&self) -> Vec<ExperimentWinnerSummary> {
        let mut winners = self
            .experiments
            .values()
            .filter_map(PromptExperiment::winner_summary)
            .collect::<Vec<_>>();
        winners.sort_by(|a, b| a.experiment_id.cmp(&b.experiment_id));
        winners
    }

    /// Return the winning variant of a concluded experiment, if it reached
    /// statistical significance (confidence >= 0.95).
    ///
    /// This is a convenience accessor; the auto-promotion in
    /// `LearningRuntime::record_completed_run` calls `on_experiment_concluded`
    /// which already promotes winners into the cascade router. This method
    /// exposes the winner for callers that need the variant content directly.
    pub fn promote_winner(&self, experiment_id: &str) -> Option<ExperimentWinner> {
        let experiment = self.experiments.get(experiment_id)?;
        let winner = experiment.concluded_winner()?;
        if winner.confidence >= 0.95 {
            Some(winner)
        } else {
            None
        }
    }

    /// Write concluded winners to the static-overrides file.
    ///
    /// # Errors
    ///
    /// Returns an error if the static-overrides file cannot be written.
    pub fn apply_winners(&self, winners: &[ExperimentWinner]) -> io::Result<()> {
        self.apply_winners_to(winners, Path::new(DEFAULT_STATIC_OVERRIDES_PATH))
    }

    /// Write concluded winners to `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the existing overrides cannot be parsed, or if the
    /// new overrides cannot be serialized, written, or renamed.
    pub fn apply_winners_to(&self, winners: &[ExperimentWinner], path: &Path) -> io::Result<()> {
        if winners.is_empty() {
            return Ok(());
        }

        let mut overrides: BTreeMap<String, String> = self
            .load_static_overrides_path(path)
            .unwrap_or_default()
            .into_iter()
            .collect();

        for winner in winners.iter().filter(|winner| winner.confidence >= 0.95) {
            overrides.insert(winner.parameter.clone(), winner.winning_value.clone());
        }

        write_static_overrides(path, &overrides)
    }

    fn load_static_overrides_path(&self, path: &Path) -> io::Result<HashMap<String, String>> {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(err) => return Err(err),
        };
        let map = serde_json::from_str::<HashMap<String, String>>(&contents)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        Ok(map)
    }

    /// Promote all concluded experiment winners to the static config overrides
    /// file (INT-10: Experiments -> Static config).
    ///
    /// Returns the number of winners promoted. Only winners with confidence
    /// >= 0.95 are written.
    ///
    /// # Errors
    ///
    /// Returns an error if the static-overrides file cannot be written.
    pub fn promote_all_to_config(&self) -> io::Result<usize> {
        self.promote_all_to_config_at(Path::new(DEFAULT_STATIC_OVERRIDES_PATH))
    }

    /// Promote all concluded experiment winners to a specific path.
    ///
    /// # Errors
    ///
    /// Returns an error if the overrides file cannot be written.
    pub fn promote_all_to_config_at(&self, path: &Path) -> io::Result<usize> {
        let winners = self.concluded_winners();
        let promotable: Vec<_> = winners
            .into_iter()
            .filter(|w| w.confidence >= 0.95)
            .collect();
        let count = promotable.len();
        self.apply_winners_to(&promotable, path)?;
        Ok(count)
    }

    /// Record an outcome by `variant_id` (searches all experiments).
    pub fn record_outcome(&mut self, variant_id: &str, success: bool) {
        for experiment in self.experiments.values_mut() {
            if experiment.stats.contains_key(variant_id) {
                experiment.record_outcome(variant_id, success);
                return;
            }
        }
    }

    /// Record an outcome for a variant in one specific experiment.
    ///
    /// Returns `false` when either identifier is unknown. Prefer this scoped
    /// form when the caller retained the experiment id at assignment time, so
    /// identical variant ids in separate experiments cannot be conflated.
    pub fn record_outcome_for_experiment(
        &mut self,
        experiment_id: &str,
        variant_id: &str,
        success: bool,
    ) -> bool {
        let Some(experiment) = self.experiments.get_mut(experiment_id) else {
            return false;
        };
        if !experiment.stats.contains_key(variant_id) {
            return false;
        }
        experiment.record_outcome(variant_id, success);
        true
    }

    /// Apply a WAL-replayed experiment outcome. Does NOT write a WAL entry.
    ///
    /// Identical to [`Self::record_outcome`] but named distinctly so callers
    /// cannot accidentally bypass WAL writes during normal operation.
    pub fn replay_outcome(&mut self, variant_id: &str, success: bool) {
        self.record_outcome(variant_id, success);
    }

    /// Record a numeric metric for a variant within a specific experiment.
    pub fn record_metric(&mut self, experiment_id: &str, variant_id: &str, metric: f64) {
        if let Some(experiment) = self.experiments.get_mut(experiment_id) {
            experiment.record_metric(variant_id, metric);
        }
    }

    /// All experiments (for reporting).
    #[must_use]
    pub const fn experiments(&self) -> &HashMap<String, PromptExperiment> {
        &self.experiments
    }

    /// Running experiments count.
    pub fn running_count(&self) -> usize {
        self.experiments
            .values()
            .filter(|e| e.status == ExperimentStatus::Running)
            .count()
    }

    /// Concluded experiments count.
    pub fn concluded_count(&self) -> usize {
        self.experiments
            .values()
            .filter(|e| e.status == ExperimentStatus::Concluded)
            .count()
    }

    /// Iterate over all experiments.
    pub fn iter(&self) -> impl Iterator<Item = &PromptExperiment> {
        self.experiments.values()
    }
}

impl Default for ExperimentStore {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_attempt_key(attempt_key: &PromptAttemptKey) -> Result<(), PromptAssignmentError> {
    for (name, value) in [
        ("run_id", attempt_key.run_id.as_str()),
        ("plan_id", attempt_key.plan_id.as_str()),
        ("task_id", attempt_key.task_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(PromptAssignmentError::Conflict(format!(
                "attempt {name} cannot be empty"
            )));
        }
    }
    Ok(())
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_sections(sections: &[&str]) -> Result<Vec<String>, PromptAssignmentError> {
    let mut normalized = Vec::with_capacity(sections.len());
    for section in sections {
        let section = section.trim();
        if section.is_empty() {
            return Err(PromptAssignmentError::Conflict(
                "eligible prompt sections cannot contain an empty name".to_string(),
            ));
        }
        normalized.push(section.to_string());
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn validate_included_assignment_ids(
    assignment_ids: &[&str],
) -> Result<Vec<String>, PromptAssignmentError> {
    let mut canonical = Vec::with_capacity(assignment_ids.len());
    for assignment_id in assignment_ids {
        if assignment_id.is_empty() || assignment_id.trim() != *assignment_id {
            return Err(PromptAssignmentError::Conflict(
                "included assignment ids must be non-empty exact ids".to_string(),
            ));
        }
        if canonical
            .last()
            .is_some_and(|previous: &String| previous.as_str() >= *assignment_id)
        {
            return Err(PromptAssignmentError::Conflict(
                "included assignment ids must be lexicographically sorted and unique".to_string(),
            ));
        }
        canonical.push((*assignment_id).to_string());
    }
    Ok(canonical)
}

fn experiment_matches_role(experiment: &PromptExperiment, requested_role: Option<&str>) -> bool {
    let experiment_role = experiment
        .role
        .as_deref()
        .map(str::trim)
        .filter(|role| !role.is_empty());
    experiment_role.is_none() || experiment_role == requested_role
}

fn attempt_bucket_id(attempt_key: &PromptAttemptKey) -> String {
    stable_assignment_hash(&[
        "prompt-attempt",
        &attempt_key.run_id,
        &attempt_key.plan_id,
        &attempt_key.task_id,
        &attempt_key.attempt.to_string(),
    ])
}

fn assignment_id(
    attempt_key: &PromptAttemptKey,
    experiment_id: &str,
    variant_id: &str,
    role: Option<&str>,
    section_name: &str,
) -> String {
    format!(
        "prompt-assignment-{}",
        stable_assignment_hash(&[
            "prompt-assignment",
            &attempt_key.run_id,
            &attempt_key.plan_id,
            &attempt_key.task_id,
            &attempt_key.attempt.to_string(),
            experiment_id,
            variant_id,
            role.unwrap_or(""),
            section_name,
        ])
    )
}

fn stable_assignment_hash(parts: &[&str]) -> String {
    let mut canonical = Vec::new();
    for part in parts {
        canonical.extend_from_slice(&(part.len() as u64).to_le_bytes());
        canonical.extend_from_slice(part.as_bytes());
    }
    ContentHash::of(&canonical).to_hex()
}

fn winner_variant_label(variant: &PromptVariant) -> String {
    variant
        .slug
        .clone()
        .filter(|slug| !slug.trim().is_empty())
        .or_else(|| (!variant.name.trim().is_empty()).then(|| variant.name.clone()))
        .unwrap_or_else(|| variant.id.clone())
}

fn write_static_overrides(path: &Path, overrides: &BTreeMap<String, String>) -> io::Result<()> {
    let json = serde_json::to_string_pretty(overrides)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_variants(section: &str) -> Vec<PromptVariant> {
        vec![
            PromptVariant {
                id: "a".into(),
                name: "Variant A".into(),
                section_name: section.into(),
                content: "Be concise.".into(),
                slug: None,
                active: true,
            },
            PromptVariant {
                id: "b".into(),
                name: "Variant B".into(),
                section_name: section.into(),
                content: "Be verbose and thorough.".into(),
                slug: None,
                active: true,
            },
        ]
    }

    fn attempt(run_id: &str, attempt: u32) -> PromptAttemptKey {
        PromptAttemptKey::new(run_id, "plan-1", "task-1", attempt)
    }

    #[test]
    fn experiment_selects_unsampled_first() {
        let exp = PromptExperiment::new("test-1", "constraints", make_variants("constraints"));
        // Both unsampled — should return first variant.
        let v = exp.assign_variant().unwrap();
        assert!(v.id == "a" || v.id == "b");
    }

    #[test]
    fn experiment_concludes_when_gap_sufficient() {
        let mut exp = PromptExperiment::new("test-2", "style", make_variants("style"));
        exp.min_trials_per_variant = 5;
        exp.min_effect_size = 0.1;

        // Give variant "a" 100% success, "b" 0%.
        for _ in 0..5 {
            exp.record_outcome("a", true);
            exp.record_outcome("b", false);
        }

        assert_eq!(exp.status, ExperimentStatus::Concluded);
        assert_eq!(exp.winner_id.as_deref(), Some("a"));
        assert!(
            exp.archive
                .as_ref()
                .is_some_and(|archive| archive.p_value < 0.05)
        );
    }

    #[test]
    fn chi_squared_significance_and_early_stopping_identify_winner() {
        let strong = VariantStats {
            trials: 50,
            successes: 48,
        };
        let weak = VariantStats {
            trials: 50,
            successes: 10,
        };
        let (statistic, p_value) = chi_squared_test(&strong, &weak);
        assert!(statistic > 10.0);
        assert!(p_value < 0.05);

        let mut experiment = PromptExperiment::new("early", "style", make_variants("style"));
        experiment.stats.insert("a".into(), strong);
        experiment.stats.insert("b".into(), weak);
        assert_eq!(experiment.early_stopping_check().as_deref(), Some("a"));
    }

    #[test]
    fn store_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");

        let mut store = ExperimentStore::new();
        let exp = PromptExperiment::new("exp-1", "constraints", make_variants("constraints"));
        store.register(exp);

        store.save(&path).unwrap();
        let loaded = ExperimentStore::load_or_new(&path);
        assert_eq!(loaded.experiments().len(), 1);
        assert!(loaded.get("exp-1").is_some());
    }

    #[test]
    fn concluded_winners_only_return_high_confidence_results() {
        let mut store = ExperimentStore::new();
        let mut exp = PromptExperiment::new(
            "exp-role",
            "model-routing",
            vec![PromptVariant {
                id: "winner".into(),
                name: "Winner".into(),
                section_name: "model-routing".into(),
                content: "claude-sonnet-4-6".into(),
                slug: Some("claude-sonnet-4-6".into()),
                active: true,
            }],
        );
        exp.role = Some("implementer".into());
        exp.status = ExperimentStatus::Concluded;
        exp.winner_id = Some("winner".into());
        store.register(exp);

        let winners = store.concluded_winners();
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].parameter, "implementer");
        assert_eq!(winners[0].winning_value, "claude-sonnet-4-6");
        assert!(winners[0].confidence >= 0.95);
    }

    #[test]
    fn winner_summaries_include_ci_and_stable_ordering() {
        let mut store = ExperimentStore::new();

        let mut exp_b = PromptExperiment::new("exp-b", "constraints", make_variants("constraints"));
        exp_b.status = ExperimentStatus::Concluded;
        exp_b.winner_id = Some("b".into());
        exp_b.stats.insert(
            "a".into(),
            VariantStats {
                trials: 80,
                successes: 8,
            },
        );
        exp_b.stats.insert(
            "b".into(),
            VariantStats {
                trials: 80,
                successes: 76,
            },
        );

        let mut exp_a = PromptExperiment::new("exp-a", "constraints", make_variants("constraints"));
        exp_a.status = ExperimentStatus::Concluded;
        exp_a.winner_id = Some("a".into());
        exp_a.stats.insert(
            "a".into(),
            VariantStats {
                trials: 96,
                successes: 92,
            },
        );
        exp_a.stats.insert(
            "b".into(),
            VariantStats {
                trials: 96,
                successes: 12,
            },
        );

        store.register(exp_b);
        store.register(exp_a);

        let winners = store.winner_summaries();
        assert_eq!(winners.len(), 2);
        assert_eq!(winners[0].experiment_id, "exp-a");
        assert_eq!(winners[1].experiment_id, "exp-b");
        assert_eq!(winners[0].winner_variant_id, "a");
        assert_eq!(winners[0].winner, "Variant A");
        assert_eq!(winners[0].sample_size, 96);
        assert!((winners[0].win_rate - (92.0 / 96.0)).abs() < f64::EPSILON);
        assert!(winners[0].ci_lower <= winners[0].win_rate);
        assert!(winners[0].ci_upper >= winners[0].win_rate);
    }

    #[test]
    fn apply_winners_writes_static_overrides() {
        let store = ExperimentStore::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("static-overrides.json");
        let winners = vec![ExperimentWinner {
            experiment_id: "exp-role".into(),
            parameter: "implementer".into(),
            winning_value: "claude-sonnet-4-6".into(),
            confidence: 0.99,
        }];

        store.apply_winners_to(&winners, &path).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let overrides: HashMap<String, String> = serde_json::from_str(&contents).unwrap();
        assert_eq!(
            overrides.get("implementer"),
            Some(&"claude-sonnet-4-6".to_string())
        );
    }

    #[test]
    fn assign_variant_for_section_works() {
        let mut store = ExperimentStore::new();
        let exp = PromptExperiment::new("exp-1", "constraints", make_variants("constraints"));
        store.register(exp);

        let result = store.assign_variant_for_section("constraints");
        assert!(result.is_some());
        let (id, _content) = result.unwrap();
        assert!(id == "a" || id == "b");

        // No experiment for unknown section.
        assert!(store.assign_variant_for_section("unknown").is_none());
    }

    #[test]
    fn record_metric_updates_existing_variant_only() {
        let mut store = ExperimentStore::new();
        let exp = PromptExperiment::new("exp-1", "constraints", make_variants("constraints"));
        store.register(exp);

        store.record_metric("exp-1", "a", 0.75);
        store.record_metric("exp-1", "missing", 0.2);

        let experiment = store.get("exp-1").expect("experiment exists");
        let stats = experiment.metric_stats.get("a").expect("variant metrics");
        assert_eq!(stats.samples, 1);
        assert_eq!(stats.last, Some(0.75));
        assert_eq!(stats.sum, 0.75);
        assert!(!experiment.metric_stats.contains_key("missing"));
    }

    #[test]
    fn replay_outcome_updates_stats_identically_to_record_outcome() {
        let mut store = ExperimentStore::default();
        let exp = PromptExperiment::new("exp-1", "style", make_variants("style"));
        store.register(exp);

        // Use replay_outcome and verify it behaves like record_outcome.
        store.replay_outcome("a", true);
        store.replay_outcome("a", false);
        store.replay_outcome("b", true);

        let experiment = store.get("exp-1").expect("experiment exists");
        let stats_a = experiment.stats.get("a").expect("variant a stats");
        assert_eq!(stats_a.trials, 2);
        assert_eq!(stats_a.successes, 1);

        let stats_b = experiment.stats.get("b").expect("variant b stats");
        assert_eq!(stats_b.trials, 1);
        assert_eq!(stats_b.successes, 1);
    }

    #[test]
    fn legacy_store_json_loads_strictly_with_empty_assignment_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let experiment =
            PromptExperiment::new("legacy", "constraints", make_variants("constraints"));
        let legacy = serde_json::json!({
            "experiments": { "legacy": experiment }
        });
        std::fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

        let loaded = ExperimentStore::load_strict(&path).expect("legacy JSON remains readable");
        assert!(loaded.get("legacy").is_some());
        assert!(loaded.attempt_assignments.is_empty());
    }

    #[test]
    fn malformed_store_is_preserved_and_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let malformed = b"{ not valid experiment state";
        std::fs::write(&path, malformed).unwrap();

        let error = ExperimentStore::prepare_attempt_assignments(
            &path,
            &attempt("run-malformed", 1),
            Some("implementer"),
            &["constraints"],
        )
        .expect_err("malformed state must not become a default store");
        assert!(matches!(
            error,
            PromptAssignmentError::Io(ref source)
                if source.kind() == io::ErrorKind::InvalidData
        ));
        assert_eq!(std::fs::read(path).unwrap(), malformed);
    }

    #[test]
    fn preparation_is_idempotent_and_outstanding_reservations_spread_arms() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        let mut experiment =
            PromptExperiment::new("exp", "constraints", make_variants("constraints"));
        experiment.role = Some("implementer".into());
        store.register(experiment);
        store.save(&path).unwrap();

        let first_key = attempt("run-1", 1);
        let first = ExperimentStore::prepare_attempt_assignments(
            &path,
            &first_key,
            Some("implementer"),
            &["constraints", "unrelated"],
        )
        .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].state, PromptAssignmentState::Prepared);
        assert_eq!(first[0].content_hash.len(), 64);
        assert!(first[0].content_snapshot.is_some());
        assert_eq!(first[0].section_name, "constraints");

        let replay = ExperimentStore::prepare_attempt_assignments(
            &path,
            &first_key,
            Some("implementer"),
            &["unrelated", "constraints"],
        )
        .unwrap();
        assert_eq!(replay, first);

        let second = ExperimentStore::prepare_attempt_assignments(
            &path,
            &attempt("run-1", 2),
            Some("implementer"),
            &["constraints"],
        )
        .unwrap();
        assert_eq!(second.len(), 1);
        assert_ne!(second[0].variant_id, first[0].variant_id);

        let reopened = ExperimentStore::load_strict(&path).unwrap();
        assert_eq!(
            reopened.assignments_for_attempt(&first_key),
            Some(first.as_slice())
        );
        let stats = &reopened.get("exp").unwrap().stats;
        assert_eq!(stats["a"].trials + stats["b"].trials, 0);
    }

    #[test]
    fn orphaned_reservations_from_an_older_run_do_not_bias_a_new_run() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "exp",
            "constraints",
            make_variants("constraints"),
        ));
        store.save(&path).unwrap();

        let old = ExperimentStore::prepare_attempt_assignments(
            &path,
            &attempt("old-run", 1),
            None,
            &["constraints"],
        )
        .unwrap();
        let first_in_new_run = ExperimentStore::prepare_attempt_assignments(
            &path,
            &attempt("new-run", 1),
            None,
            &["constraints"],
        )
        .unwrap();
        assert_eq!(first_in_new_run[0].variant_id, old[0].variant_id);

        let second_in_new_run = ExperimentStore::prepare_attempt_assignments(
            &path,
            &attempt("new-run", 2),
            None,
            &["constraints"],
        )
        .unwrap();
        assert_ne!(
            second_in_new_run[0].variant_id,
            first_in_new_run[0].variant_id
        );

        let reopened = ExperimentStore::load_strict(&path).unwrap();
        assert_eq!(
            reopened
                .assignments_for_attempt(&attempt("old-run", 1))
                .unwrap()[0]
                .state,
            PromptAssignmentState::Prepared
        );
    }

    #[test]
    fn overlapping_role_section_experiments_are_rejected_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "global",
            "constraints",
            make_variants("constraints"),
        ));
        let mut scoped =
            PromptExperiment::new("scoped", "constraints", make_variants("constraints"));
        scoped.role = Some("implementer".into());
        store.register(scoped);
        store.save(&path).unwrap();
        let key = attempt("run-overlap", 1);

        let error = ExperimentStore::prepare_attempt_assignments(
            &path,
            &key,
            Some("implementer"),
            &["constraints"],
        )
        .expect_err("global and role-specific treatment overlap");
        assert!(matches!(error, PromptAssignmentError::Conflict(_)));
        assert!(
            ExperimentStore::load_strict(&path)
                .unwrap()
                .assignments_for_attempt(&key)
                .is_none()
        );
    }

    #[test]
    fn unrelated_attempt_does_not_create_an_empty_durable_bucket() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        let mut experiment =
            PromptExperiment::new("scoped", "constraints", make_variants("constraints"));
        experiment.role = Some("implementer".into());
        store.register(experiment);
        store.save(&path).unwrap();
        let key = attempt("run-unrelated", 1);

        let prepared = ExperimentStore::prepare_attempt_assignments(
            &path,
            &key,
            Some("reviewer"),
            &["constraints", "context"],
        )
        .unwrap();

        assert!(prepared.is_empty());
        let reopened = ExperimentStore::load_strict(&path).unwrap();
        assert!(reopened.attempt_assignments.is_empty());
        assert!(reopened.assignments_for_attempt(&key).is_none());
    }

    #[test]
    fn dispatch_and_scoped_settlement_are_idempotent_and_auditable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "exp",
            "constraints",
            make_variants("constraints"),
        ));
        store.save(&path).unwrap();
        let key = attempt("run-observed", 1);
        let prepared =
            ExperimentStore::prepare_attempt_assignments(&path, &key, None, &["constraints"])
                .unwrap();
        let content_hash = prepared[0].content_hash.clone();
        let included_assignment_ids = [prepared[0].assignment_id.as_str()];

        let dispatched = ExperimentStore::mark_attempt_dispatched(
            &path,
            &key,
            "full-prompt-hash",
            &included_assignment_ids,
        )
        .unwrap();
        assert_eq!(dispatched[0].state, PromptAssignmentState::Dispatched);
        assert_eq!(
            ExperimentStore::mark_attempt_dispatched(
                &path,
                &key,
                "full-prompt-hash",
                &included_assignment_ids,
            )
            .unwrap(),
            dispatched
        );
        assert!(matches!(
            ExperimentStore::mark_attempt_dispatched(
                &path,
                &key,
                "changed-hash",
                &included_assignment_ids,
            ),
            Err(PromptAssignmentError::Conflict(_))
        ));

        let settlement = AssignmentSettlement::Observed { success: true };
        let observed = ExperimentStore::settle_attempt(&path, &key, settlement).unwrap();
        assert_eq!(observed[0].state, PromptAssignmentState::Observed);
        assert_eq!(observed[0].success, Some(true));
        assert!(observed[0].content_snapshot.is_none());
        assert_eq!(observed[0].content_hash, content_hash);
        assert_eq!(observed[0].prompt_hash.as_deref(), Some("full-prompt-hash"));
        assert_eq!(
            ExperimentStore::settle_attempt(&path, &key, settlement).unwrap(),
            observed
        );
        assert!(matches!(
            ExperimentStore::settle_attempt(
                &path,
                &key,
                AssignmentSettlement::Observed { success: false }
            ),
            Err(PromptAssignmentError::Conflict(_))
        ));

        let reopened = ExperimentStore::load_strict(&path).unwrap();
        let variant = &observed[0].variant_id;
        let stats = &reopened.get("exp").unwrap().stats[variant];
        assert_eq!(stats.trials, 1);
        assert_eq!(stats.successes, 1);
    }

    #[test]
    fn dispatch_tracks_exact_included_subset_and_excluded_treatment_gets_zero_credit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "included",
            "constraints",
            make_variants("constraints"),
        ));
        store.register(PromptExperiment::new(
            "excluded",
            "style",
            make_variants("style"),
        ));
        store.save(&path).unwrap();
        let key = attempt("run-subset", 1);
        let prepared = ExperimentStore::prepare_attempt_assignments(
            &path,
            &key,
            None,
            &["constraints", "style"],
        )
        .unwrap();
        assert_eq!(prepared.len(), 2);

        let included = prepared
            .iter()
            .find(|assignment| assignment.experiment_id == "included")
            .unwrap();
        let excluded = prepared
            .iter()
            .find(|assignment| assignment.experiment_id == "excluded")
            .unwrap();
        let included_assignment_id = included.assignment_id.clone();
        let excluded_assignment_id = excluded.assignment_id.clone();
        let included_variant_id = included.variant_id.clone();
        let excluded_variant_id = excluded.variant_id.clone();
        let excluded_content_hash = excluded.content_hash.clone();

        let mut all_ids = [
            included_assignment_id.as_str(),
            excluded_assignment_id.as_str(),
        ];
        all_ids.sort_unstable();
        let unsorted_ids = [all_ids[1], all_ids[0]];
        assert!(matches!(
            ExperimentStore::mark_attempt_dispatched(&path, &key, "prompt-hash", &unsorted_ids,),
            Err(PromptAssignmentError::Conflict(_))
        ));
        assert!(matches!(
            ExperimentStore::mark_attempt_dispatched(
                &path,
                &key,
                "prompt-hash",
                &[all_ids[0], all_ids[0]],
            ),
            Err(PromptAssignmentError::Conflict(_))
        ));
        assert!(matches!(
            ExperimentStore::mark_attempt_dispatched(
                &path,
                &key,
                "prompt-hash",
                &["prompt-assignment-not-in-attempt"],
            ),
            Err(PromptAssignmentError::Conflict(_))
        ));

        let included_ids = [included_assignment_id.as_str()];
        let dispatched =
            ExperimentStore::mark_attempt_dispatched(&path, &key, "prompt-hash", &included_ids)
                .unwrap();
        let dispatched_included = dispatched
            .iter()
            .find(|assignment| assignment.experiment_id == "included")
            .unwrap();
        let still_prepared = dispatched
            .iter()
            .find(|assignment| assignment.experiment_id == "excluded")
            .unwrap();
        assert_eq!(dispatched_included.state, PromptAssignmentState::Dispatched);
        assert_eq!(
            dispatched_included.prompt_hash.as_deref(),
            Some("prompt-hash")
        );
        assert_eq!(still_prepared.state, PromptAssignmentState::Prepared);
        assert!(still_prepared.prompt_hash.is_none());
        assert_eq!(
            ExperimentStore::mark_attempt_dispatched(&path, &key, "prompt-hash", &included_ids,)
                .unwrap(),
            dispatched
        );
        assert!(matches!(
            ExperimentStore::mark_attempt_dispatched(
                &path,
                &key,
                "prompt-hash",
                &[excluded_assignment_id.as_str()],
            ),
            Err(PromptAssignmentError::Conflict(_))
        ));

        let settled = ExperimentStore::settle_attempt(
            &path,
            &key,
            AssignmentSettlement::Observed { success: true },
        )
        .unwrap();
        let observed = settled
            .iter()
            .find(|assignment| assignment.experiment_id == "included")
            .unwrap();
        let abandoned = settled
            .iter()
            .find(|assignment| assignment.experiment_id == "excluded")
            .unwrap();
        assert_eq!(observed.state, PromptAssignmentState::Observed);
        assert_eq!(observed.success, Some(true));
        assert_eq!(abandoned.state, PromptAssignmentState::Abandoned);
        assert_eq!(abandoned.success, None);
        assert!(abandoned.content_snapshot.is_none());
        assert_eq!(abandoned.content_hash, excluded_content_hash);
        assert!(abandoned.prompt_hash.is_none());

        let reopened = ExperimentStore::load_strict(&path).unwrap();
        assert_eq!(
            reopened.get("included").unwrap().stats[&included_variant_id].trials,
            1
        );
        assert_eq!(
            reopened.get("included").unwrap().stats[&included_variant_id].successes,
            1
        );
        assert_eq!(
            reopened.get("excluded").unwrap().stats[&excluded_variant_id].trials,
            0
        );
        assert_eq!(
            reopened.get("excluded").unwrap().stats[&excluded_variant_id].successes,
            0
        );
    }

    #[test]
    fn settling_prepared_assignment_abandons_without_counting_trial() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "exp",
            "constraints",
            make_variants("constraints"),
        ));
        store.save(&path).unwrap();
        let key = attempt("run-prepared", 1);
        ExperimentStore::prepare_attempt_assignments(&path, &key, None, &["constraints"]).unwrap();

        let settled = ExperimentStore::settle_attempt(
            &path,
            &key,
            AssignmentSettlement::Observed { success: true },
        )
        .unwrap();
        assert_eq!(settled[0].state, PromptAssignmentState::Abandoned);
        assert_eq!(settled[0].success, None);
        assert!(settled[0].content_snapshot.is_none());
        let reopened = ExperimentStore::load_strict(&path).unwrap();
        assert!(
            reopened
                .get("exp")
                .unwrap()
                .stats
                .values()
                .all(|stats| stats.trials == 0)
        );
    }

    #[test]
    fn concluded_winner_is_sticky_and_never_changes_stats() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut experiment =
            PromptExperiment::new("done", "constraints", make_variants("constraints"));
        experiment.status = ExperimentStatus::Concluded;
        experiment.winner_id = Some("b".into());
        experiment.stats.get_mut("a").unwrap().trials = 10;
        experiment.stats.get_mut("b").unwrap().trials = 10;
        let original_stats = experiment.stats.clone();
        let mut store = ExperimentStore::new();
        store.register(experiment);
        store.save(&path).unwrap();
        let key = attempt("run-winner", 1);

        let prepared =
            ExperimentStore::prepare_attempt_assignments(&path, &key, None, &["constraints"])
                .unwrap();
        assert_eq!(prepared[0].variant_id, "b");
        assert_eq!(
            prepared[0].content_snapshot.as_deref(),
            Some("Be verbose and thorough.")
        );
        let reopened_prepared = ExperimentStore::load_strict(&path).unwrap();
        assert_eq!(
            reopened_prepared.assignments_for_attempt(&key),
            Some(prepared.as_slice())
        );

        ExperimentStore::mark_attempt_dispatched(
            &path,
            &key,
            "winner-prompt",
            &[prepared[0].assignment_id.as_str()],
        )
        .unwrap();
        ExperimentStore::settle_attempt(
            &path,
            &key,
            AssignmentSettlement::Observed { success: true },
        )
        .unwrap();
        let reopened = ExperimentStore::load_strict(&path).unwrap();
        let reopened_stats = &reopened.get("done").unwrap().stats;
        for (variant_id, expected) in original_stats {
            assert_eq!(reopened_stats[&variant_id].trials, expected.trials);
            assert_eq!(reopened_stats[&variant_id].successes, expected.successes);
        }
    }
}
