//! Model A/B experiments.
//!
//! Each task draws its model variant uniformly at random through S01's
//! assignment draw on the `model_experiment` layer, keyed by the task id, and
//! the draw's propensity goes back to the caller to log. An experiment
//! concludes on the same two-arm difference confidence sequences as prompt
//! experiments ([`crate::prompt_experiment`]; S03 T8, backlog 5119), which
//! stay valid however often they are checked. No dispatch path consults model
//! experiments yet; `roko experiment model` manages them.

use crate::cascade_router::CascadeRouter;
use crate::prompt_experiment::{
    ConclusionRule, EXPERIMENT_ALPHA, ExperimentStatus, VariantObservation, draw_uniform,
};
use roko_core::agent::AgentRole;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The layer model experiments draw their variants on.
pub const MODEL_EXPERIMENT_LAYER: &str = "model_experiment";

/// A model A/B experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelExperiment {
    /// Unique experiment identifier.
    pub experiment_id: String,
    /// Human-readable description of the experiment.
    pub description: String,
    /// Optional role scope for the experiment.
    pub role: Option<String>,
    /// Optional task category scope for the experiment.
    pub task_category: Option<String>,
    /// Variants available in the experiment.
    pub variants: Vec<ModelVariant>,
    /// Per-variant statistics keyed by variant id.
    pub stats: HashMap<String, ModelVariantStats>,
    /// Current experiment status.
    pub status: ExperimentStatus,
    /// Winner variant id, if concluded.
    pub winner_id: Option<String>,
    /// Minimum trials per variant before the experiment can conclude.
    pub min_trials_per_variant: u64,
    /// Minimum effect size required to declare a winner.
    pub min_effect_size: f64,
    /// Experiment creation timestamp in ISO-8601 format.
    pub created_at: String,
    /// This experiment's share of [`EXPERIMENT_ALPHA`]: α over the
    /// experiments running when the store last split it, never raised.
    #[serde(default = "crate::prompt_experiment::default_alpha")]
    pub alpha: f64,
    /// The randomized observations the conclusion rule counts, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observations: Vec<VariantObservation>,
    /// The conclusion rule over `observations`, rebuilt after a load.
    #[serde(skip)]
    rule: Option<ConclusionRule>,
}

/// A single model variant participating in an experiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelVariant {
    /// Unique identifier for this variant.
    pub id: String,
    /// Key into the `[models.*]` configuration table.
    pub model_key: String,
    /// API model slug.
    pub slug: String,
    /// Provider key for the model.
    pub provider: String,
}

/// A model experiment's assignment for one task: the experiment, the variant,
/// and the propensity of the draw that picked it, for the caller to log.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelAssignment {
    /// The experiment that drew the variant.
    pub experiment_id: String,
    /// The variant drawn.
    pub variant: ModelVariant,
    /// P(the draw picks this variant): 1/k over the experiment's k variants.
    pub propensity: f64,
}

/// Per-variant stats for a model experiment.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelVariantStats {
    /// Number of trials run for this variant.
    pub trials: u64,
    /// Number of successful trials.
    pub successes: u64,
    /// Total cost accumulated in USD.
    pub total_cost_usd: f64,
    /// Total tokens consumed.
    pub total_tokens: u64,
    /// Total duration accumulated in milliseconds.
    pub total_duration_ms: u64,
    /// Success rate derived from `successes / trials`.
    pub pass_rate: f64,
    /// Average cost per trial in USD.
    pub avg_cost_usd: f64,
    /// Cost per successful trial in USD.
    pub cost_per_success: f64,
    /// Average duration per trial in milliseconds.
    pub avg_duration_ms: f64,
}

impl ModelVariantStats {
    /// Recompute derived metrics from the accumulated counters.
    fn recalculate(&mut self) {
        if self.trials == 0 {
            self.pass_rate = 0.0;
            self.avg_cost_usd = 0.0;
            self.cost_per_success = 0.0;
            self.avg_duration_ms = 0.0;
            return;
        }

        self.pass_rate = self.successes as f64 / self.trials as f64;
        self.avg_cost_usd = self.total_cost_usd / self.trials as f64;
        self.avg_duration_ms = self.total_duration_ms as f64 / self.trials as f64;
        self.cost_per_success = if self.successes == 0 {
            0.0
        } else {
            self.total_cost_usd / self.successes as f64
        };
    }
}

impl ModelExperiment {
    /// A running experiment over `variants`, created now, with no role or
    /// task category scope: 20 trials per variant and a 0.05 lead conclude.
    pub fn new(
        experiment_id: impl Into<String>,
        description: impl Into<String>,
        variants: Vec<ModelVariant>,
    ) -> Self {
        Self {
            experiment_id: experiment_id.into(),
            description: description.into(),
            role: None,
            task_category: None,
            variants,
            stats: HashMap::new(),
            status: ExperimentStatus::Running,
            winner_id: None,
            min_trials_per_variant: 20,
            min_effect_size: 0.05,
            created_at: chrono::Utc::now().to_rfc3339(),
            alpha: EXPERIMENT_ALPHA,
            observations: Vec::new(),
            rule: None,
        }
    }

    /// Total trials across all variants for this experiment.
    pub fn total_trials(&self) -> u64 {
        self.stats.values().map(|s| s.trials).sum()
    }

    /// Select the model variant for `task_id`.
    ///
    /// Concluded experiments always return the winner. Running experiments
    /// return the variant [`Self::draw_variant`] draws for the task, so the
    /// same task receives the same variant across plan resumes.
    pub fn assign_variant(&self, task_id: &str) -> Option<&ModelVariant> {
        if self.status == ExperimentStatus::Concluded {
            return self
                .variants
                .iter()
                .find(|variant| Some(&variant.id) == self.winner_id.as_ref());
        }
        self.draw_variant(task_id).map(|(variant, _)| variant)
    }

    /// Draw the variant for `unit_key` (the task id): uniform over the
    /// variants in id order, on [`MODEL_EXPERIMENT_LAYER`] keyed by the
    /// experiment id. Returns the variant and its propensity 1/k for the
    /// caller to log, or `None` without a variant.
    #[must_use]
    pub fn draw_variant(&self, unit_key: &str) -> Option<(&ModelVariant, f64)> {
        let arms = self.arms();
        let (index, propensity) = draw_uniform(
            MODEL_EXPERIMENT_LAYER,
            &self.experiment_id,
            unit_key,
            arms.len(),
        )?;
        Some((arms[index], propensity))
    }

    /// The variants in id order: the arms of the draw and of the rule.
    fn arms(&self) -> Vec<&ModelVariant> {
        let mut arms: Vec<&ModelVariant> = self.variants.iter().collect();
        arms.sort_by(|left, right| left.id.cmp(&right.id));
        arms
    }

    /// Record an outcome for a model variant and update experiment state.
    /// Returns true if the experiment concluded.
    ///
    /// The outcome counts as a trial, but it carries no logged propensity, so
    /// the conclusion rule never sees it ([`Self::record_observation`]).
    pub fn record_outcome(
        &mut self,
        variant_id: &str,
        success: bool,
        cost_usd: f64,
        tokens: u64,
        duration_ms: u64,
    ) -> bool {
        self.record(variant_id, success, None, cost_usd, tokens, duration_ms)
    }

    /// Record a randomized observation: the outcome of a task under the
    /// variant [`Self::draw_variant`] drew, at the propensity it returned.
    /// Returns true if the experiment concluded.
    pub fn record_observation(
        &mut self,
        variant_id: &str,
        success: bool,
        propensity: f64,
        cost_usd: f64,
        tokens: u64,
        duration_ms: u64,
    ) -> bool {
        self.record(
            variant_id,
            success,
            Some(propensity),
            cost_usd,
            tokens,
            duration_ms,
        )
    }

    /// Count a trial for `variant_id`, log a running experiment's observation
    /// when it has a propensity in (0, 1], and conclude when the rule
    /// declares a winner.
    fn record(
        &mut self,
        variant_id: &str,
        success: bool,
        propensity: Option<f64>,
        cost_usd: f64,
        tokens: u64,
        duration_ms: u64,
    ) -> bool {
        let stats = self.stats.entry(variant_id.to_string()).or_default();
        stats.trials += 1;
        if success {
            stats.successes += 1;
        }
        stats.total_cost_usd += cost_usd;
        stats.total_tokens += tokens;
        stats.total_duration_ms += duration_ms;
        stats.recalculate();

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
        let Some(winner_id) = self.conclusion() else {
            return false;
        };
        self.status = ExperimentStatus::Concluded;
        self.winner_id = Some(winner_id);
        true
    }

    /// The winner the rule declares once every variant has
    /// `min_trials_per_variant` trials: a lone variant, or the variant whose
    /// difference CS against every other lies above 0 with an estimated lead
    /// of at least `min_effect_size` over each.
    fn conclusion(&mut self) -> Option<String> {
        let arms: Vec<String> = self
            .arms()
            .into_iter()
            .map(|variant| variant.id.clone())
            .collect();
        let min_trials = self.min_trials_per_variant;
        let trials = |id: &String| self.stats.get(id).map_or(0, |stats| stats.trials);
        if arms.is_empty() || arms.iter().any(|id| trials(id) < min_trials) {
            return None;
        }
        if arms.len() == 1 {
            return arms.into_iter().next();
        }
        let (alpha, min_effect) = (self.alpha, self.min_effect_size);
        ConclusionRule::judge(&mut self.rule, arms, alpha, &self.observations, min_effect)
    }
}

// ─── Store ──────────────────────────────────────────────────────────────────

/// Persisted registry of model experiments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelExperimentStore {
    /// All experiments keyed by experiment id.
    #[serde(default)]
    experiments: HashMap<String, ModelExperiment>,
}

impl ModelExperimentStore {
    /// Load a store from disk, or create an empty store if the file is missing
    /// or invalid.
    pub fn load_or_new(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save the store to disk using an atomic rename.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be serialized, if the target
    /// directory or snapshot file cannot be written, or if syncing the
    /// cascade-router mirror fails.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, path)?;
        self.sync_cascade_router(path)?;
        Ok(())
    }

    /// Register a new experiment if it is not already present.
    pub fn register(&mut self, experiment: ModelExperiment) {
        self.experiments
            .entry(experiment.experiment_id.clone())
            .or_insert(experiment);
        self.split_alpha();
    }

    /// Split [`EXPERIMENT_ALPHA`] over the running experiments: each share
    /// drops to α over their count and never rises again.
    fn split_alpha(&mut self) {
        let share = EXPERIMENT_ALPHA / self.running_count().max(1) as f64;
        for experiment in self.experiments.values_mut() {
            if experiment.status == ExperimentStatus::Running {
                experiment.alpha = experiment.alpha.min(share);
            }
        }
    }

    /// Find an active experiment scoped to a specific role.
    ///
    /// When multiple experiments target the same role, selects the one with
    /// fewest total trials to prevent starvation.
    pub fn active_for_role(&self, role: &str) -> Option<&ModelExperiment> {
        self.experiments
            .values()
            .filter(|experiment| {
                experiment.status == ExperimentStatus::Running
                    && experiment.role.as_deref() == Some(role)
            })
            .min_by_key(|experiment| experiment.total_trials())
    }

    /// Find an active experiment scoped to a specific task category.
    ///
    /// When multiple experiments target the same category, selects the one
    /// with fewest total trials to prevent starvation.
    pub fn active_for_category(&self, category: &str) -> Option<&ModelExperiment> {
        self.experiments
            .values()
            .filter(|experiment| {
                experiment.status == ExperimentStatus::Running
                    && experiment.task_category.as_deref() == Some(category)
            })
            .min_by_key(|experiment| experiment.total_trials())
    }

    fn applicable_experiment(&self, role: &str, category: &str) -> Option<&ModelExperiment> {
        self.experiments
            .values()
            .find(|experiment| {
                experiment.status == ExperimentStatus::Running
                    && experiment.role.as_deref() == Some(role)
                    && experiment.task_category.as_deref() == Some(category)
            })
            .or_else(|| self.active_for_role(role))
            .or_else(|| self.active_for_category(category))
    }

    /// Assign a model variant for the current role/category, if an active
    /// experiment applies.
    ///
    /// `task_id` keys the experiment's uniform draw.
    pub fn assign_model(&self, role: &str, category: &str, task_id: &str) -> Option<ModelVariant> {
        self.assign_model_with_experiment(role, category, task_id)
            .map(|assignment| assignment.variant)
    }

    /// Assign a model variant for the current role/category, with the owning
    /// experiment and the propensity of the draw, for the caller to log and
    /// pass back to [`Self::record_observation`].
    ///
    /// `task_id` keys the experiment's uniform draw.
    pub fn assign_model_with_experiment(
        &self,
        role: &str,
        category: &str,
        task_id: &str,
    ) -> Option<ModelAssignment> {
        let experiment = self.applicable_experiment(role, category)?;
        let (variant, propensity) = experiment.draw_variant(task_id)?;
        Some(ModelAssignment {
            experiment_id: experiment.experiment_id.clone(),
            variant: variant.clone(),
            propensity,
        })
    }

    /// Record an outcome for a variant within a specific experiment. It
    /// counts as a trial, but the conclusion rule never sees it.
    pub fn record_outcome(
        &mut self,
        experiment_id: &str,
        variant_id: &str,
        success: bool,
        cost: f64,
        tokens: u64,
        duration: u64,
    ) {
        let usage = (cost, tokens, duration);
        self.record(experiment_id, variant_id, success, None, usage);
    }

    /// Record a randomized observation for a variant within a specific
    /// experiment, at the propensity its assignment returned.
    pub fn record_observation(
        &mut self,
        experiment_id: &str,
        variant_id: &str,
        success: bool,
        propensity: f64,
        cost: f64,
        tokens: u64,
        duration: u64,
    ) {
        let usage = (cost, tokens, duration);
        self.record(experiment_id, variant_id, success, Some(propensity), usage);
    }

    /// Record one outcome, `usage` being its cost, tokens and duration, and
    /// report a conclusion.
    fn record(
        &mut self,
        experiment_id: &str,
        variant_id: &str,
        success: bool,
        propensity: Option<f64>,
        usage: (f64, u64, u64),
    ) {
        self.split_alpha();
        let Some(experiment) = self.experiments.get_mut(experiment_id) else {
            return;
        };
        let (cost, tokens, duration) = usage;
        let concluded = experiment.record(variant_id, success, propensity, cost, tokens, duration);
        if concluded {
            let experiment = experiment.clone();
            self.on_conclusion(&experiment);
        }
    }

    /// Number of currently running experiments.
    pub fn running_count(&self) -> usize {
        self.experiments
            .values()
            .filter(|experiment| experiment.status == ExperimentStatus::Running)
            .count()
    }

    /// Look up an experiment by id.
    pub fn get(&self, experiment_id: &str) -> Option<&ModelExperiment> {
        self.experiments.get(experiment_id)
    }

    /// All concluded experiments.
    pub fn concluded_experiments(&self) -> Vec<&ModelExperiment> {
        self.experiments
            .values()
            .filter(|experiment| experiment.status == ExperimentStatus::Concluded)
            .collect()
    }

    /// Iterate over all experiments.
    pub fn iter(&self) -> impl Iterator<Item = &ModelExperiment> {
        self.experiments.values()
    }

    fn on_conclusion(&self, experiment: &ModelExperiment) {
        if let Some(ref winner_id) = experiment.winner_id {
            tracing::info!(
                experiment = %experiment.experiment_id,
                winner = %winner_id,
                "model experiment concluded"
            );
        }
    }

    fn sync_cascade_router(&self, experiment_store_path: &Path) -> Result<(), std::io::Error> {
        let mut role_winners: HashMap<AgentRole, (String, String, String)> = HashMap::new();

        for experiment in self.experiments.values() {
            if experiment.status != ExperimentStatus::Concluded {
                continue;
            }

            let Some(role_raw) = experiment.role.as_deref() else {
                continue;
            };
            let Some(role) = parse_agent_role(role_raw) else {
                tracing::warn!(
                    experiment = %experiment.experiment_id,
                    role = role_raw,
                    "skipping concluded model experiment with unrecognized role"
                );
                continue;
            };
            let Some(winner_id) = experiment.winner_id.as_deref() else {
                continue;
            };
            let Some(winner) = experiment
                .variants
                .iter()
                .find(|variant| variant.id == winner_id)
            else {
                tracing::warn!(
                    experiment = %experiment.experiment_id,
                    winner = winner_id,
                    "skipping concluded model experiment with missing winner variant"
                );
                continue;
            };

            let should_replace = role_winners
                .get(&role)
                .map(|(created_at, _, _)| experiment.created_at >= *created_at)
                .unwrap_or(true);
            if should_replace {
                role_winners.insert(
                    role,
                    (
                        experiment.created_at.clone(),
                        winner.slug.clone(),
                        experiment.experiment_id.clone(),
                    ),
                );
            }
        }

        if role_winners.is_empty() {
            return Ok(());
        }

        let mut model_slugs: Vec<String> = self
            .experiments
            .values()
            .flat_map(|experiment| {
                experiment
                    .variants
                    .iter()
                    .map(|variant| variant.slug.clone())
                    .collect::<Vec<_>>()
            })
            .collect();
        model_slugs.sort();
        model_slugs.dedup();

        let router_path = experiment_store_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("cascade-router.json");
        let mut router = CascadeRouter::load_or_new(&router_path, model_slugs);

        for (role, (_, slug, experiment_id)) in role_winners {
            router.set_static_role_model(role, slug.clone());
            tracing::info!(
                experiment = %experiment_id,
                role = role.label(),
                winner_model = %slug,
                cascade_router = %router_path.display(),
                "updated cascade router static role mapping from concluded experiment"
            );
        }

        router
            .save(&router_path)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

fn parse_agent_role(raw: &str) -> Option<AgentRole> {
    if let Ok(role) = serde_json::from_str::<AgentRole>(&format!("\"{raw}\"")) {
        return Some(role);
    }

    std::iter::once(AgentRole::Conductor)
        .chain(AgentRole::ALL_AGENTS.iter().copied())
        .find(|role| raw == format!("{role:?}"))
}

impl Default for ModelExperimentStore {
    fn default() -> Self {
        Self {
            experiments: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cascade_router::{CascadeRouter, CascadeStage};
    use crate::loop_audit::sim::SplitMix64;
    use crate::model_router::RoutingContext;
    use roko_core::agent::AgentRole;
    use roko_core::task::{TaskCategory, TaskComplexityBand};

    fn make_variants() -> Vec<ModelVariant> {
        vec![
            ModelVariant {
                id: "a".into(),
                model_key: "model-a".into(),
                slug: "model-a".into(),
                provider: "provider-a".into(),
            },
            ModelVariant {
                id: "b".into(),
                model_key: "model-b".into(),
                slug: "model-b".into(),
                provider: "provider-b".into(),
            },
        ]
    }

    fn make_experiment(
        experiment_id: &str,
        role: Option<&str>,
        category: Option<&str>,
    ) -> ModelExperiment {
        let mut experiment = ModelExperiment::new(
            experiment_id,
            format!("Experiment {experiment_id}"),
            make_variants(),
        );
        experiment.role = role.map(str::to_string);
        experiment.task_category = category.map(str::to_string);
        experiment.min_trials_per_variant = 1;
        experiment.created_at = "2026-04-11T00:00:00Z".into();
        experiment
    }

    /// Record randomized observations in `experiment` of `store`, "a"
    /// succeeding and "b" failing in turn, until it concludes; the count.
    fn separate(store: &mut ModelExperimentStore, experiment: &str) -> usize {
        for observation in 0..40 {
            let (variant, success) = if observation % 2 == 0 {
                ("a", true)
            } else {
                ("b", false)
            };
            store.record_observation(experiment, variant, success, 0.5, 1.25, 120, 900);
            if store.running_count() == 0 {
                return observation + 1;
            }
        }
        panic!("{experiment} did not conclude");
    }

    #[test]
    fn model_experiment_types() {
        let mut experiment = ModelExperiment::new(
            "glm-vs-kimi",
            "Compare models for implementer tasks",
            vec![
                ModelVariant {
                    id: "glm".into(),
                    model_key: "glm-5-1".into(),
                    slug: "glm-5.1".into(),
                    provider: "zai".into(),
                },
                ModelVariant {
                    id: "kimi".into(),
                    model_key: "kimi-k2-5".into(),
                    slug: "kimi-k2.5".into(),
                    provider: "moonshot".into(),
                },
            ],
        );
        experiment.role = Some("implementer".into());
        experiment.task_category = Some("implementation".into());
        experiment.stats = HashMap::from([(
            "glm".into(),
            ModelVariantStats {
                trials: 12,
                successes: 9,
                total_cost_usd: 2.4,
                total_tokens: 18_000,
                total_duration_ms: 54_000,
                pass_rate: 0.75,
                avg_cost_usd: 0.2,
                cost_per_success: 0.266_666_666_7,
                avg_duration_ms: 4_500.0,
            },
        )]);

        let json = serde_json::to_string(&experiment).expect("serialize");
        let decoded: ModelExperiment = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(decoded.experiment_id, "glm-vs-kimi");
        assert_eq!(decoded.variants.len(), 2);
        assert_eq!(decoded.stats["glm"].trials, 12);
        assert_eq!(decoded.status, ExperimentStatus::Running);

        // A store written before backlog 5119 has neither an α share nor
        // observations; it still loads.
        let legacy = serde_json::json!({
            "experiment_id": "legacy",
            "description": "Written before 5119",
            "role": null,
            "task_category": null,
            "variants": [],
            "stats": {},
            "status": "Running",
            "winner_id": null,
            "min_trials_per_variant": 20,
            "min_effect_size": 0.05,
            "created_at": "2026-04-11T00:00:00Z",
        });
        let legacy: ModelExperiment = serde_json::from_value(legacy).expect("legacy JSON");
        assert_eq!(legacy.alpha, EXPERIMENT_ALPHA);
        assert!(legacy.observations.is_empty());
    }

    /// A task draws one variant at propensity 1/k; outcomes without a
    /// logged propensity count trials only, and randomized observations
    /// conclude once the sequence separates the variants.
    #[test]
    fn model_experiment_draws_and_concludes_on_the_sequence() {
        let mut experiment = make_experiment("glm-vs-kimi", Some("implementer"), None);
        let (drawn, propensity) = experiment
            .draw_variant("T1")
            .map(|(variant, propensity)| (variant.id.clone(), propensity))
            .expect("a draw");
        assert_eq!(propensity, 0.5);
        assert_eq!(
            experiment.assign_variant("T1").map(|v| v.id.clone()),
            Some(drawn)
        );

        experiment.record_outcome("a", true, 1.0, 100, 1_000);
        experiment.record_outcome("b", false, 1.0, 100, 1_000);
        assert_eq!(experiment.status, ExperimentStatus::Running);

        let mut concluded_at = None;
        for observation in 0..40_u32 {
            let (variant, success) = if observation % 2 == 0 {
                ("a", true)
            } else {
                ("b", false)
            };
            if experiment.record_observation(variant, success, 0.5, 1.0, 100, 1_000) {
                concluded_at = Some(observation + 1);
                break;
            }
        }

        // Ten successes against nine failures first separate the variants.
        assert_eq!(concluded_at, Some(19));
        assert_eq!(experiment.winner_id.as_deref(), Some("a"));
        assert_eq!(
            experiment.assign_variant("T1").map(|v| v.id.as_str()),
            Some("a")
        );
        assert_eq!(experiment.stats["a"].pass_rate, 1.0);
        assert_eq!(experiment.stats["a"].avg_cost_usd, 1.0);
        assert_eq!(experiment.stats["a"].cost_per_success, 1.0);
        assert_eq!(experiment.stats["a"].avg_duration_ms, 1_000.0);
    }

    /// S03 T8: over 10³ simulated A/A model experiments (two variants with
    /// one pass rate, 400 tasks each, drawn and judged by the rule itself) at
    /// most 5% declare a winner.
    #[test]
    fn model_experiment_aa_false_winner_rate_below_alpha() {
        const EXPERIMENTS: u64 = 1_000;
        const TASKS: u32 = 400;
        let mut winners = 0_u64;
        for rep in 0..EXPERIMENTS {
            let mut experiment = ModelExperiment::new("aa", "A/A", make_variants());
            let mut outcomes = SplitMix64::new(rep);
            for task in 1..=TASKS {
                let (variant, propensity) = experiment
                    .draw_variant(&format!("aa-{rep}-t{task}"))
                    .map(|(variant, propensity)| (variant.id.clone(), propensity))
                    .expect("a running experiment draws a variant");
                let success = outcomes.next_f64() < 0.5;
                if experiment.record_observation(&variant, success, propensity, 0.0, 0, 0) {
                    winners += 1;
                    break;
                }
            }
        }
        let rate = winners as f64 / EXPERIMENTS as f64;
        println!("model experiments: A/A false-winner rate {rate:.3} ({winners}/{EXPERIMENTS})");
        assert!(
            rate <= 0.05,
            "{winners} of {EXPERIMENTS} A/A experiments declared a winner"
        );
    }

    #[test]
    fn model_experiment_store_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model-experiments.json");

        let mut store = ModelExperimentStore::default();
        let mut concluded = make_experiment("category-exp", None, Some("implementation"));
        concluded.status = ExperimentStatus::Concluded;
        concluded.winner_id = Some("b".into());
        store.register(make_experiment("role-exp", Some("implementer"), None));
        store.register(concluded);

        store.save(&path).unwrap();

        let loaded = ModelExperimentStore::load_or_new(&path);
        assert_eq!(loaded.running_count(), 1);
        assert_eq!(loaded.concluded_experiments().len(), 1);
        assert_eq!(
            loaded
                .active_for_role("implementer")
                .map(|exp| exp.experiment_id.as_str()),
            Some("role-exp")
        );
        assert_eq!(
            loaded
                .active_for_category("implementation")
                .map(|exp| exp.experiment_id.as_str()),
            None
        );
        let drawn = loaded
            .get("role-exp")
            .and_then(|experiment| experiment.draw_variant(""))
            .map(|(variant, _)| variant.id.clone());
        assert_eq!(
            loaded
                .assign_model("implementer", "implementation", "")
                .map(|variant| variant.id),
            drawn
        );
    }

    #[test]
    fn model_experiment_store_prefers_role_scope() {
        let mut store = ModelExperimentStore::default();
        store.register(make_experiment("role-exp", Some("implementer"), None));
        store.register(make_experiment(
            "category-exp",
            None,
            Some("implementation"),
        ));

        let assigned = store
            .assign_model_with_experiment("implementer", "implementation", "")
            .expect("an assignment");
        assert_eq!(assigned.experiment_id, "role-exp");
        assert_eq!(assigned.propensity, 0.5);
        assert_eq!(
            store
                .active_for_role("implementer")
                .map(|exp| exp.experiment_id.as_str()),
            Some("role-exp")
        );
        assert_eq!(
            store
                .active_for_category("implementation")
                .map(|exp| exp.experiment_id.as_str()),
            Some("category-exp")
        );
    }

    #[test]
    fn model_experiment_store_records_outcomes() {
        let mut store = ModelExperimentStore::default();
        let experiment =
            make_experiment("glm-vs-kimi", Some("implementer"), Some("implementation"));
        store.register(experiment);

        store.record_outcome("glm-vs-kimi", "a", true, 1.25, 120, 900);
        store.record_outcome("glm-vs-kimi", "b", false, 2.50, 240, 1_800);
        assert_eq!(store.running_count(), 1, "unlogged outcomes never conclude");

        assert_eq!(separate(&mut store, "glm-vs-kimi"), 19);
        let concluded = store.concluded_experiments();
        assert_eq!(concluded.len(), 1);
        let experiment = concluded[0];
        assert_eq!(experiment.status, ExperimentStatus::Concluded);
        assert_eq!(experiment.winner_id.as_deref(), Some("a"));
        assert_eq!(experiment.stats["a"].trials, 11);
        assert_eq!(experiment.stats["a"].successes, 11);
        assert_eq!(experiment.stats["a"].total_cost_usd, 13.75);
        assert_eq!(experiment.stats["a"].total_tokens, 1_320);
        assert_eq!(experiment.stats["a"].total_duration_ms, 9_900);
        assert_eq!(experiment.stats["a"].pass_rate, 1.0);
        assert_eq!(experiment.stats["b"].trials, 10);
        assert_eq!(experiment.stats["b"].successes, 0);
        assert_eq!(experiment.observations.len(), 19);
        assert_eq!(store.running_count(), 0);
    }

    #[test]
    fn experiment_conclusion_updates_static_role_table() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("model-experiments.json");
        let router_path = dir.path().join("cascade-router.json");
        let router = CascadeRouter::new(vec![
            "claude-haiku-4-5".to_string(),
            "claude-sonnet-4-5".to_string(),
            "model-a".to_string(),
            "model-b".to_string(),
        ]);
        router.save(&router_path).unwrap();

        let mut store = ModelExperimentStore::default();
        store.register(make_experiment(
            "glm-vs-kimi",
            Some("implementer"),
            Some("implementation"),
        ));

        separate(&mut store, "glm-vs-kimi");
        store.save(&store_path).unwrap();

        let reloaded = CascadeRouter::load_or_new(
            &router_path,
            vec![
                "claude-haiku-4-5".to_string(),
                "claude-sonnet-4-5".to_string(),
                "model-a".to_string(),
                "model-b".to_string(),
            ],
        );
        let routed = reloaded.route(&RoutingContext {
            task_category: TaskCategory::Implementation,
            complexity: TaskComplexityBand::Standard,
            iteration: 0,
            role: AgentRole::Implementer,
            crate_familiarity: 0.5,
            has_prior_failure: false,
            conductor_load: 0.0,
            active_agents: 0,
            ready_queue_depth: 0,
            max_queue_wait_hours: 0.0,
            daimon_policy: roko_core::DaimonPolicy::default(),
            thinking_level: None,
            temperament: None,
            previous_model: None,
            plan_context_tokens: None,
            tier_thresholds: None,
            cfactor: None,
        });

        assert_eq!(routed.stage, CascadeStage::Static);
        assert_eq!(routed.primary.slug, "model-a");
    }

    #[test]
    fn assignment_varies_by_task_id() {
        // Each task id keys its own uniform draw, so ten task ids reach both
        // variants.
        let experiment = make_experiment("exp", Some("implementer"), Some("implementation"));
        let ids: Vec<String> = (0..10).map(|i| format!("task-{i}")).collect();
        let selected: std::collections::HashSet<&str> = ids
            .iter()
            .filter_map(|task_id| experiment.assign_variant(task_id).map(|v| v.id.as_str()))
            .collect();
        assert!(
            selected.len() >= 2,
            "expected at least 2 distinct variants across 10 task_ids, got {selected:?}"
        );
    }

    #[test]
    fn assignment_stable_across_outcome_recording() {
        // The draw depends on the task id alone, so recording outcomes never
        // moves a task's variant: a resumed plan keeps its assignment.
        let mut experiment = make_experiment("exp", Some("implementer"), Some("implementation"));
        let task_id = "T01";

        let first = experiment
            .assign_variant(task_id)
            .map(|v| v.id.clone())
            .expect("initial assignment");

        let other_id = if first == "a" { "b" } else { "a" };
        experiment.record_outcome(other_id, true, 1.0, 100, 1_000);
        experiment.record_outcome(&first, false, 1.0, 100, 1_000);

        let second = experiment
            .assign_variant(task_id)
            .map(|v| v.id.clone())
            .expect("post-outcome assignment");

        assert_eq!(
            first, second,
            "variant for task {task_id} changed after recording outcomes"
        );
    }
}
