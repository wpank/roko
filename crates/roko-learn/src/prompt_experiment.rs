//! Lightweight A/B testing framework for prompt section variants.
//!
//! Each experiment tests multiple variants of a prompt section (e.g. a
//! system-prompt paragraph). Each attempt draws its variant uniformly at
//! random through S01's assignment draw ([`crate::telemetry::assign`]) on the
//! `prompt_variant` layer, and its assignment logs the draw's propensity. An
//! experiment concludes once two-arm difference confidence sequences over
//! those draws ([`DifferenceCs`]) put one variant above every other. The
//! sequences stay valid however often they are checked, so monitoring after
//! every outcome cannot manufacture a winner (S03 T8, backlog 5118).
//!
//! Persistence is a single JSON file managed by [`ExperimentStore`].

use roko_core::{ContentHash, ExperimentWinnerSummary};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::loop_audit::cs::DifferenceCs;
use crate::telemetry::assign::draw;
use crate::telemetry::{AssignmentUnit, AttemptKey};

/// Default path for persisted static overrides derived from concluded experiments.
pub const DEFAULT_STATIC_OVERRIDES_PATH: &str = ".roko/learn/static-overrides.json";

/// The layer prompt experiments draw their variants on (S03 §4.2).
pub const PROMPT_VARIANT_LAYER: &str = "prompt_variant";

/// The family-wise level of the conclusion rule, split over the experiments
/// running at once.
pub const EXPERIMENT_ALPHA: f64 = 0.05;

/// The seed of experiment draws: the experiment id keys the layer, and the
/// attempt key differs from draw to draw.
const DRAW_SEED: u64 = 0;

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

/// One outcome the conclusion rule counts: an attempt's result under the
/// variant a randomized draw picked, with the propensity the draw logged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantObservation {
    /// The variant the draw picked.
    pub variant_id: String,
    /// Whether the attempt succeeded.
    pub success: bool,
    /// P(the draw picks this variant), logged when it was drawn.
    pub propensity: f64,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// P(this variant) under the draw that picked it, logged when it was
    /// prepared: 1/k over the k active variants of a running experiment, 1
    /// for a concluded experiment's winner. `None` on receipts written before
    /// backlog 5118.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propensity: Option<f64>,
    /// Whether this treatment was assigned while the experiment was running.
    /// Concluded sticky winners remain auditable but never reserve or update a
    /// learning trial.
    #[serde(default)]
    learning_eligible: bool,
}

/// Durable attempt bucket used to make preparation, dispatch, and settlement
/// idempotent across process crashes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// This experiment's share of [`EXPERIMENT_ALPHA`]: α over the
    /// experiments running when the store last split it, never raised.
    #[serde(default = "default_alpha")]
    pub alpha: f64,
    /// The randomized observations the conclusion rule counts, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observations: Vec<VariantObservation>,
    /// The conclusion rule over `observations`, rebuilt after a load.
    #[serde(skip)]
    rule: Option<ConclusionRule>,
}

/// A new experiment's α share: all of it.
pub(crate) const fn default_alpha() -> f64 {
    EXPERIMENT_ALPHA
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
            alpha: EXPERIMENT_ALPHA,
            observations: Vec::new(),
            rule: None,
        }
    }

    /// The variant for a caller without an attempt key: a concluded
    /// experiment's winner, else the control, its first active variant.
    /// Without a key there is no draw to log, so an unkeyed caller never runs
    /// a treatment and its outcomes never reach the conclusion rule.
    ///
    /// Returns `None` when the experiment has no such variant.
    pub fn assign_variant(&self) -> Option<&PromptVariant> {
        if self.status == ExperimentStatus::Concluded {
            // Return the winner if concluded.
            return self
                .winner_id
                .as_ref()
                .and_then(|wid| self.variants.iter().find(|v| v.id == *wid));
        }
        self.variants.iter().find(|variant| variant.active)
    }

    /// Draw the variant of a running experiment for `attempt_key`: uniform
    /// over the active variants in id order, on [`PROMPT_VARIANT_LAYER`] keyed
    /// by the experiment id, with the attempt as the unit. Returns the variant
    /// and its propensity 1/k, or `None` without an active variant.
    #[must_use]
    pub fn draw_variant(&self, attempt_key: &PromptAttemptKey) -> Option<(&PromptVariant, f64)> {
        let arms = self.arms();
        let unit = AssignmentUnit::Attempt.unit_key(&AttemptKey::from(attempt_key.clone()));
        let (index, propensity) =
            draw_uniform(PROMPT_VARIANT_LAYER, &self.experiment_id, &unit, arms.len())?;
        Some((arms[index], propensity))
    }

    /// The active variants with statistics, in id order: the arms of the draw
    /// and of the conclusion rule.
    fn arms(&self) -> Vec<&PromptVariant> {
        let mut arms: Vec<&PromptVariant> = self
            .variants
            .iter()
            .filter(|variant| variant.active && self.stats.contains_key(&variant.id))
            .collect();
        arms.sort_by(|left, right| left.id.cmp(&right.id));
        arms
    }

    /// Record an outcome for a variant. Returns true if the experiment concluded.
    ///
    /// The outcome counts as a trial, but it carries no logged propensity, so
    /// the conclusion rule never sees it ([`Self::record_observation`]).
    pub fn record_outcome(&mut self, variant_id: &str, success: bool) -> bool {
        self.record(variant_id, success, None)
    }

    /// Record a randomized observation: the outcome of an attempt under the
    /// variant [`Self::draw_variant`] picked, at the propensity its assignment
    /// logged. Returns true if the experiment concluded.
    pub fn record_observation(&mut self, variant_id: &str, success: bool, propensity: f64) -> bool {
        self.record(variant_id, success, Some(propensity))
    }

    /// Count a trial for `variant_id`, log a running experiment's observation
    /// when it has a propensity in (0, 1], and conclude when the rule
    /// declares a winner.
    fn record(&mut self, variant_id: &str, success: bool, propensity: Option<f64>) -> bool {
        let Some(stats) = self.stats.get_mut(variant_id) else {
            return false;
        };
        stats.trials += 1;
        if success {
            stats.successes += 1;
        }
        if self.status != ExperimentStatus::Running {
            return false;
        }
        if let Some(propensity) = propensity.filter(|p| *p > 0.0 && *p <= 1.0) {
            self.observations.push(VariantObservation {
                variant_id: variant_id.to_string(),
                success,
                propensity,
            });
        }
        let Some(winner) = self.conclusion() else {
            return false;
        };
        self.status = ExperimentStatus::Concluded;
        self.winner_id = Some(winner);
        self.archive = Some(self.build_archive());
        true
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

    /// The winner the rule declares, if any. A lone active variant wins at
    /// once, as there is nothing to compare it with. Otherwise, once every
    /// active variant has `min_trials_per_variant` trials, the winner is the
    /// variant whose difference CS against every other lies above 0, with an
    /// estimated lead of at least `min_effect_size` over each.
    fn conclusion(&mut self) -> Option<String> {
        let arms: Vec<String> = self
            .arms()
            .into_iter()
            .map(|variant| variant.id.clone())
            .collect();
        if arms.len() < 2 {
            return arms.into_iter().next();
        }
        let min_trials = self.min_trials_per_variant;
        let trials = |id: &String| self.stats.get(id).map_or(0, |stats| stats.trials);
        if arms.iter().any(|id| trials(id) < min_trials) {
            return None;
        }
        let (alpha, min_effect) = (self.alpha, self.min_effect_size);
        ConclusionRule::judge(&mut self.rule, arms, alpha, &self.observations, min_effect)
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

/// Draw one of `arms` arms uniformly for `unit_key` on `layer`, with
/// `experiment_id` as the epoch, through S01's assignment draw: the index and
/// its propensity 1/`arms`, or `None` without an arm.
pub(crate) fn draw_uniform(
    layer: &str,
    experiment_id: &str,
    unit_key: &str,
    arms: usize,
) -> Option<(usize, f64)> {
    if arms == 0 {
        return None;
    }
    let u = draw(DRAW_SEED, layer, experiment_id, unit_key);
    let index = ((u * arms as f64) as usize).min(arms - 1);
    Some((index, 1.0 / arms as f64))
}

/// The conclusion rule over an experiment's randomized observations: one
/// two-arm difference CS ([`DifferenceCs`]) per pair of arms, each at the
/// experiment's α share split over the pairs, fed in observation order.
#[derive(Debug, Clone)]
pub(crate) struct ConclusionRule {
    /// The arms (variant ids, in id order) the pairs are over.
    arms: Vec<String>,
    /// The α share the rule was built at.
    alpha: f64,
    /// Observations taken so far, counted or skipped.
    consumed: usize,
    /// One CS per pair of arms.
    pairs: Vec<ArmPair>,
}

/// One pair of arms: the CS on the first arm's lead over the second, and the
/// count and sum of its IPW increments.
#[derive(Debug, Clone)]
struct ArmPair {
    first: usize,
    second: usize,
    cs: DifferenceCs,
    n: u64,
    sum: f64,
}

impl ConclusionRule {
    /// The winner the rule declares over `observations` for `arms`, at α
    /// share `alpha` (capped at [`EXPERIMENT_ALPHA`]). The cached `rule` is
    /// rebuilt when the arms or the share changed, then fed the observations
    /// it has not taken yet.
    pub(crate) fn judge(
        rule: &mut Option<Self>,
        arms: Vec<String>,
        alpha: f64,
        observations: &[VariantObservation],
        min_effect: f64,
    ) -> Option<String> {
        let alpha = if alpha > 0.0 && alpha < EXPERIMENT_ALPHA {
            alpha
        } else {
            EXPERIMENT_ALPHA
        };
        let stale = rule.as_ref().is_none_or(|rule| {
            rule.arms != arms || rule.alpha != alpha || rule.consumed > observations.len()
        });
        if stale {
            *rule = None;
        }
        let rule = rule.get_or_insert_with(|| Self::new(arms, alpha));
        for observation in &observations[rule.consumed..] {
            rule.push(observation);
        }
        rule.winner(min_effect).map(str::to_string)
    }

    /// A rule over `arms` at level `alpha`, split over the pairs.
    fn new(arms: Vec<String>, alpha: f64) -> Self {
        let count = arms.len();
        let level = alpha / (count * count.saturating_sub(1) / 2).max(1) as f64;
        let mut pairs = Vec::new();
        for first in 0..count {
            for second in first + 1..count {
                pairs.push(ArmPair {
                    first,
                    second,
                    cs: DifferenceCs::new(level, 1.0),
                    n: 0,
                    sum: 0.0,
                });
            }
        }
        Self {
            arms,
            alpha,
            consumed: 0,
            pairs,
        }
    }

    /// Take the next observation. One of a variant outside the arms, or one
    /// drawn over another number of arms (its propensity is not 1/k), is
    /// skipped.
    fn push(&mut self, observation: &VariantObservation) {
        self.consumed += 1;
        let variant = observation.variant_id.as_str();
        let Some(arm) = self.arms.iter().position(|id| id == variant) else {
            return;
        };
        if (observation.propensity * self.arms.len() as f64 - 1.0).abs() > 1e-9 {
            return;
        }
        let y = if observation.success { 1.0 } else { 0.0 };
        for pair in &mut self.pairs {
            if arm != pair.first && arm != pair.second {
                continue;
            }
            // Uniform draws give both arms the logged propensity, so within
            // the pair each arm has conditional propensity 1/2.
            let first = arm == pair.first;
            pair.cs.push(y, first, 0.5);
            pair.n += 1;
            pair.sum += if first { 2.0 * y } else { -2.0 * y };
        }
    }

    /// The arm whose CS against every other arm lies above 0, with an
    /// estimated lead of at least `min_effect` over each.
    fn winner(&self, min_effect: f64) -> Option<&str> {
        if self.pairs.is_empty() {
            return None;
        }
        (0..self.arms.len())
            .find(|arm| {
                self.pairs
                    .iter()
                    .filter(|pair| pair.first == *arm || pair.second == *arm)
                    .all(|pair| pair.leads(*arm, min_effect))
            })
            .map(|arm| self.arms[arm].as_str())
    }
}

impl ArmPair {
    /// Whether `arm`, one of the pair, leads the other: the CS on its lead
    /// lies above 0, and the estimated lead is at least `min_effect`.
    fn leads(&self, arm: usize, min_effect: f64) -> bool {
        let Some((low, high)) = self.cs.interval() else {
            return false;
        };
        if self.n == 0 {
            return false;
        }
        let estimate = self.sum / self.n as f64;
        let (low, estimate) = if arm == self.first {
            (low, estimate)
        } else {
            (-high, -estimate)
        };
        low > 0.0 && estimate >= min_effect
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
        self.split_alpha();
    }

    /// Split [`EXPERIMENT_ALPHA`] over the running experiments: each share
    /// drops to α over their count and never rises again, so the experiments
    /// running at once share α. The retired retrieval-strategy experiment
    /// takes no share.
    fn split_alpha(&mut self) {
        let running = self
            .experiments
            .values()
            .filter(|experiment| experiment.status == ExperimentStatus::Running)
            .filter(|experiment| experiment.experiment_id != Self::RETRIEVAL_STRATEGY_EXPERIMENT_ID)
            .count();
        let share = EXPERIMENT_ALPHA / running.max(1) as f64;
        for experiment in self.experiments.values_mut() {
            if experiment.status == ExperimentStatus::Running {
                experiment.alpha = experiment.alpha.min(share);
            }
        }
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
    /// Returns `(variant_id, section_name, variant_content)` so callers can
    /// perform canonical section replacement rather than appending.
    pub fn assign_variant_with_section(
        &self,
        experiment_name: &str,
    ) -> Option<(String, String, String)> {
        let experiment = self
            .experiments
            .values()
            .find(|e| e.experiment_id == experiment_name || e.section_name == experiment_name)?;
        let variant = experiment.assign_variant()?;
        Some((
            variant.id.clone(),
            experiment.section_name.clone(),
            variant.content.clone(),
        ))
    }

    /// Assign a variant for a given prompt section, if an active experiment exists.
    ///
    /// Returns `(variant_id, variant_content)` or `None` if no experiment.
    pub fn assign_variant_for_section(&self, section_name: &str) -> Option<(String, String)> {
        self.assign_variant(section_name)
    }

    /// Prepare deterministic role/section treatments for one durable attempt.
    ///
    /// Preparation draws each running experiment's variant from the attempt
    /// key ([`PromptExperiment::draw_variant`]), logs the draw's propensity on
    /// the assignment, and counts no trial. Prepared and dispatched
    /// assignments reserve their treatment until settlement, but they never
    /// steer a later draw. Repeating the identical request returns the same
    /// content snapshots.
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
            let (variant, propensity) = if learning_eligible {
                experiment
                    .draw_variant(attempt_key)
                    .map(|(variant, propensity)| (variant.clone(), propensity))
                    .ok_or_else(|| {
                        PromptAssignmentError::Conflict(format!(
                            "running experiment {:?} has no active variant",
                            experiment.experiment_id
                        ))
                    })?
            } else {
                let winner = experiment
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
                    })?;
                (winner, 1.0)
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
                propensity: Some(propensity),
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
                        assignment.propensity,
                    )
                })
                .collect::<Vec<_>>(),
            AssignmentSettlement::Abandoned => Vec::new(),
        };

        // Validate every scoped target before changing any statistics.
        for (experiment_id, variant_id, _, _) in &observations {
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
        for (experiment_id, variant_id, success, propensity) in &observations {
            let recorded = match propensity {
                Some(propensity) => self.record_observation_for_experiment(
                    experiment_id,
                    variant_id,
                    *success,
                    *propensity,
                ),
                // Receipts written before backlog 5118 logged no propensity.
                None => self.record_outcome_for_experiment(experiment_id, variant_id, *success),
            };
            if !recorded {
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
        self.split_alpha();
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
        self.split_alpha();
        let Some(experiment) = self.experiments.get_mut(experiment_id) else {
            return false;
        };
        if !experiment.stats.contains_key(variant_id) {
            return false;
        }
        experiment.record_outcome(variant_id, success);
        true
    }

    /// Record a randomized observation for a variant in one experiment, at
    /// the propensity its assignment logged
    /// ([`PromptExperiment::record_observation`]).
    ///
    /// Returns `false` when either identifier is unknown.
    pub fn record_observation_for_experiment(
        &mut self,
        experiment_id: &str,
        variant_id: &str,
        success: bool,
        propensity: f64,
    ) -> bool {
        self.split_alpha();
        let Some(experiment) = self.experiments.get_mut(experiment_id) else {
            return false;
        };
        if !experiment.stats.contains_key(variant_id) {
            return false;
        }
        experiment.record_observation(variant_id, success, propensity);
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

    /// Force-conclude a running experiment by picking the best-performing active variant.
    ///
    /// Returns the winning variant id on success. Errors if the experiment is
    /// already concluded, not found, or has no active variants with stats.
    pub fn force_conclude(&mut self, experiment_id: &str) -> io::Result<String> {
        let exp = self.experiments.get_mut(experiment_id).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("experiment '{experiment_id}' not found"),
            )
        })?;

        if exp.status == ExperimentStatus::Concluded {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "experiment '{experiment_id}' is already concluded with winner '{}'",
                    exp.winner_id.as_deref().unwrap_or("?")
                ),
            ));
        }

        // Pick the active variant with the highest success rate.
        // Ties are broken by variant id lexicographic order.
        let active_ids: Vec<String> = exp
            .variants
            .iter()
            .filter(|v| v.active)
            .map(|v| v.id.clone())
            .collect();

        let winner_id = active_ids
            .iter()
            .filter_map(|id| exp.stats.get(id.as_str()).map(|s| (id, s)))
            .max_by(|a, b| {
                a.1.success_rate()
                    .partial_cmp(&b.1.success_rate())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(b.0))
            })
            .map(|(id, _)| id.clone())
            .or_else(|| active_ids.into_iter().next())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("experiment '{experiment_id}' has no active variants"),
                )
            })?;

        exp.status = ExperimentStatus::Concluded;
        exp.winner_id = Some(winner_id.clone());
        exp.archive = Some(exp.build_archive());
        Ok(winner_id)
    }

    // ── RAG-11: the retired retrieval strategy experiment ─────────────────

    /// Id of the RAG-11 retrieval-strategy experiment. Graph dispatch no
    /// longer draws its arms (4105): they labelled attempts without changing
    /// retrieval. Stores written before then may still hold it, and prompt
    /// treatments ignore it.
    pub const RETRIEVAL_STRATEGY_EXPERIMENT_ID: &'static str = "retrieval-strategy";
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
    use crate::loop_audit::sim::SplitMix64;

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
    fn unkeyed_assignment_serves_the_control_then_the_winner() {
        let mut exp = PromptExperiment::new("test-1", "constraints", make_variants("constraints"));
        assert_eq!(exp.assign_variant().map(|v| v.id.as_str()), Some("a"));
        exp.status = ExperimentStatus::Concluded;
        exp.winner_id = Some("b".into());
        assert_eq!(exp.assign_variant().map(|v| v.id.as_str()), Some("b"));
    }

    #[test]
    fn experiment_concludes_when_the_sequence_separates_the_arms() {
        let mut exp = PromptExperiment::new("test-2", "style", make_variants("style"));
        exp.min_trials_per_variant = 5;
        exp.min_effect_size = 0.1;

        // Variant "a" always succeeds and "b" always fails, drawn 50/50.
        let mut concluded_at = None;
        for observation in 1..=40_u32 {
            let (variant, success) = if observation % 2 == 1 {
                ("a", true)
            } else {
                ("b", false)
            };
            if exp.record_observation(variant, success, 0.5) {
                concluded_at = Some(observation);
                break;
            }
        }

        // Ten successes against nine failures first put the sequence's
        // lower bound above 0.
        assert_eq!(concluded_at, Some(19));
        assert_eq!(exp.status, ExperimentStatus::Concluded);
        assert_eq!(exp.winner_id.as_deref(), Some("a"));
        assert!(
            exp.archive
                .as_ref()
                .is_some_and(|archive| archive.p_value < 0.05)
        );
    }

    /// An outcome without a logged propensity counts a trial, but the
    /// conclusion rule never sees it: a one-sided record of 50 outcomes per
    /// variant leaves the experiment running.
    #[test]
    fn unlogged_outcomes_count_trials_but_never_conclude() {
        let mut exp = PromptExperiment::new("unlogged", "style", make_variants("style"));
        for _ in 0..50 {
            assert!(!exp.record_outcome("a", true));
            assert!(!exp.record_outcome("b", false));
        }
        assert_eq!(exp.status, ExperimentStatus::Running);
        assert_eq!(exp.stats["a"].trials, 50);
        assert!(exp.observations.is_empty());
    }

    #[test]
    fn chi_squared_test_flags_a_large_gap() {
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
    }

    /// S03 T8: over 10³ simulated A/A experiments (two variants with one pass
    /// rate, 400 attempts each, drawn and judged by the rule itself) at most
    /// 5% declare a winner. The legacy rule declared one in about a third
    /// (`loop_audit::sim::legacy_false_winners`).
    #[test]
    fn prompt_experiment_aa_false_winner_rate_below_alpha() {
        const EXPERIMENTS: u64 = 1_000;
        const ATTEMPTS: u32 = 400;
        let mut winners = 0_u64;
        for rep in 0..EXPERIMENTS {
            let mut experiment = PromptExperiment::new("aa", "aa", make_variants("aa"));
            let mut outcomes = SplitMix64::new(rep);
            for task in 1..=ATTEMPTS {
                let key = PromptAttemptKey::new(format!("aa-{rep}"), "p", format!("t{task}"), 1);
                let (variant, propensity) = experiment
                    .draw_variant(&key)
                    .map(|(variant, propensity)| (variant.id.clone(), propensity))
                    .expect("a running experiment draws a variant");
                let success = outcomes.next_f64() < 0.5;
                if experiment.record_observation(&variant, success, propensity) {
                    winners += 1;
                    break;
                }
            }
        }
        let rate = winners as f64 / EXPERIMENTS as f64;
        println!("prompt experiments: A/A false-winner rate {rate:.3} ({winners}/{EXPERIMENTS})");
        assert!(
            rate <= 0.05,
            "{winners} of {EXPERIMENTS} A/A experiments declared a winner"
        );
    }

    /// Every assignment logs its draw's propensity: 1/k for a running
    /// experiment's k active variants, 1 for a concluded experiment's winner.
    /// Settlement carries it into the observation the rule counts, and the
    /// draws spread evenly over the variants.
    #[test]
    fn prompt_assignment_logs_propensity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new(
            "pair",
            "constraints",
            make_variants("constraints"),
        ));
        let mut triple = make_variants("style");
        triple.push(PromptVariant {
            id: "c".into(),
            name: "Variant C".into(),
            section_name: "style".into(),
            content: "Be playful.".into(),
            slug: None,
            active: true,
        });
        store.register(PromptExperiment::new("triple", "style", triple));
        let mut done = PromptExperiment::new("done", "tone", make_variants("tone"));
        done.status = ExperimentStatus::Concluded;
        done.winner_id = Some("b".into());
        store.register(done);
        store.save(&path).unwrap();

        let key = attempt("run-logged", 1);
        let prepared = ExperimentStore::prepare_attempt_assignments(
            &path,
            &key,
            None,
            &["constraints", "style", "tone"],
        )
        .unwrap();
        let logged = |experiment_id: &str| {
            prepared
                .iter()
                .find(|assignment| assignment.experiment_id == experiment_id)
                .unwrap()
        };
        assert_eq!(logged("pair").propensity, Some(0.5));
        assert_eq!(logged("triple").propensity, Some(1.0 / 3.0));
        assert_eq!(logged("done").propensity, Some(1.0));
        assert_eq!(logged("done").variant_id, "b");
        for experiment_id in ["pair", "triple"] {
            let drawn = store
                .get(experiment_id)
                .and_then(|experiment| experiment.draw_variant(&key))
                .map(|(variant, _)| variant.id.clone());
            assert_eq!(drawn.as_ref(), Some(&logged(experiment_id).variant_id));
        }

        let mut ids = prepared
            .iter()
            .map(|assignment| assignment.assignment_id.as_str())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ExperimentStore::mark_attempt_dispatched(&path, &key, "logged-prompt", &ids).unwrap();
        ExperimentStore::settle_attempt(
            &path,
            &key,
            AssignmentSettlement::Observed { success: true },
        )
        .unwrap();
        let reopened = ExperimentStore::load_strict(&path).unwrap();
        let observed =
            |experiment_id: &str| reopened.get(experiment_id).unwrap().observations.clone();
        assert_eq!(
            observed("triple"),
            [VariantObservation {
                variant_id: logged("triple").variant_id.clone(),
                success: true,
                propensity: 1.0 / 3.0,
            }]
        );
        assert_eq!(observed("pair")[0].propensity, 0.5);
        assert!(
            observed("done").is_empty(),
            "a sticky winner is not observed"
        );

        let experiment = reopened.get("triple").unwrap();
        let mut counts = HashMap::new();
        for task in 0..3_000 {
            let key = PromptAttemptKey::new("uniform", "p", format!("t{task}"), 1);
            let (variant, _) = experiment.draw_variant(&key).unwrap();
            *counts.entry(variant.id.clone()).or_insert(0_u32) += 1;
        }
        for id in ["a", "b", "c"] {
            let share = f64::from(counts[id]) / 3_000.0;
            assert!((0.30..0.37).contains(&share), "{id}: {share}");
        }
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
    fn preparation_is_idempotent_and_draws_from_the_attempt_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let mut store = ExperimentStore::new();
        let mut experiment =
            PromptExperiment::new("exp", "constraints", make_variants("constraints"));
        experiment.role = Some("implementer".into());
        let drawn = |key: &PromptAttemptKey| {
            experiment
                .draw_variant(key)
                .map(|(variant, _)| variant.id.clone())
        };
        let first_key = attempt("run-1", 1);
        let second_key = attempt("run-1", 2);
        let (first_draw, second_draw) = (drawn(&first_key), drawn(&second_key));
        store.register(experiment);
        store.save(&path).unwrap();

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
        assert_eq!(first[0].propensity, Some(0.5));
        assert_eq!(Some(&first[0].variant_id), first_draw.as_ref());

        let replay = ExperimentStore::prepare_attempt_assignments(
            &path,
            &first_key,
            Some("implementer"),
            &["unrelated", "constraints"],
        )
        .unwrap();
        assert_eq!(replay, first);

        // The first attempt's reservation does not steer the second draw.
        let second = ExperimentStore::prepare_attempt_assignments(
            &path,
            &second_key,
            Some("implementer"),
            &["constraints"],
        )
        .unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(Some(&second[0].variant_id), second_draw.as_ref());

        let reopened = ExperimentStore::load_strict(&path).unwrap();
        assert_eq!(
            reopened.assignments_for_attempt(&first_key),
            Some(first.as_slice())
        );
        let stats = &reopened.get("exp").unwrap().stats;
        assert_eq!(stats["a"].trials + stats["b"].trials, 0);
    }

    #[test]
    fn outstanding_reservations_never_change_a_draw() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        let experiment = PromptExperiment::new("exp", "constraints", make_variants("constraints"));
        let mut store = ExperimentStore::new();
        store.register(experiment.clone());
        store.save(&path).unwrap();

        for index in 1..=5 {
            ExperimentStore::prepare_attempt_assignments(
                &path,
                &attempt("old-run", index),
                None,
                &["constraints"],
            )
            .unwrap();
        }
        for index in 1..=5 {
            let key = attempt("new-run", index);
            let prepared =
                ExperimentStore::prepare_attempt_assignments(&path, &key, None, &["constraints"])
                    .unwrap();
            let drawn = experiment
                .draw_variant(&key)
                .map(|(variant, _)| variant.id.clone());
            assert_eq!(Some(&prepared[0].variant_id), drawn.as_ref());
        }

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
