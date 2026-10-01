//! Runtime-facing learning orchestration helpers.
//!
//! This module provides a single integration point for CLI/orchestrator code:
//! pass one completed run, and the helper updates all configured learning
//! subsystems in a consistent order.
//!
//! The sub-modules separate concerns:
//!
//! - [`records`]: durable JSONL schemas, config types, and the error type.
//! - [`episode_helpers`]: pure extractors that pull typed values from episode metadata.
//! - [`routing`]: cascade-router and latency-aware reward helpers.
//! - [`persistence`]: async JSONL append/read, project snapshot readers, and query filters.
//! - [`cfactor_snapshot`]: C-Factor sub-score computation from episodes and knowledge records.

// ── Sub-modules ───────────────────────────────────────────────────────

pub mod cfactor_snapshot;
pub mod episode_helpers;
pub mod persistence;
pub mod records;
pub mod routing;

// ── Re-exports (preserves the original flat public API) ───────────────

pub use cfactor_snapshot::refresh_cfactor_snapshot;
pub use persistence::{
    ProjectLearningSnapshot, project_episode_paths, read_efficiency_events,
    read_efficiency_summaries, read_gate_outcomes, read_knowledge_seeds,
    read_project_efficiency_events, read_project_episodes_lossy, read_project_learning_snapshot,
    read_project_runtime_feedback_snapshot, read_retry_outcomes, read_runtime_feedback_snapshot,
    resolve_project_episode_path,
};
pub use records::{
    ApplyStatus, ArtifactValidationReport, CompletedRunInput, EfficiencyScope,
    EfficiencySummaryRecord, GateOutcomeRecord, GenerationOutcome, KnowledgeSeedEvidence,
    KnowledgeSeedRecord, LearningPaths, LearningRuntimeError, LearningUpdate,
    RUNTIME_FEEDBACK_SCHEMA_VERSION, RegressionConfig, RetryOutcomeRecord, RetryOutcomeStatus,
    RunnerFeedbackEvent, RuntimeFeedbackQuery, RuntimeFeedbackSnapshot, RuntimeFeedbackWrite,
    UpdateFrequency,
};

// ── Internal imports ──────────────────────────────────────────────────

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use tokio::sync::Mutex as AsyncMutex;

use roko_core::ConductorDecision;
use roko_core::DaimonPolicy;
use roko_core::agent::AgentRole;
use roko_core::metric::TaskMetric;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_daimon::{AffectEngine as _, AffectEvent, DaimonState, queue_wait_arousal};

use crate::cascade_router::{CascadeRouter, outcome_reward};
use crate::cfactor::CFactor;
use crate::context_pack_cache::ContextPackCache;
use crate::costs_db::CostsDb;
use crate::costs_log::CostsLog;
use crate::efficiency::AgentEfficiencyEvent;
use crate::episode_logger::{Episode, EpisodeLogger};
use crate::latency::LatencyRegistry;
use crate::local_reward::LocalRewardFunction;
use crate::model_router::RoutingContext;
use crate::playbook::PlaybookStore;
use crate::playbook_rules::PlaybookRules;
use crate::post_gate_reflection::{
    PostGateReflectionStore, ReflectionInput, ReflectionPromotionConfig,
};
use crate::prompt_experiment::{ExperimentStatus, ExperimentStore, PromptExperiment};
use crate::provider_health::{CircuitState, ProviderHealthRegistry, ProviderHealthTracker};
use crate::provider_model_outcome::{
    ProviderModelOutcomeRecord, ProviderModelOutcomeStore, ProviderModelPassRateReport,
    read_provider_model_outcomes, summarize_provider_model_outcomes,
};
use crate::regression::detect_regressions;
use crate::section_effect::SectionEffectivenessRegistry;
use crate::skill_library::{SkillLibrary, TemplatePatternGenerator};
use crate::wal::{self, WalEntry, WalSegment};

use episode_helpers::{
    GateCounts as GateCountsInner, backfill_gate_counts, derive_cost_record, extra_bool, extra_f64,
    extra_string, gate_counts_from_episode, load_local_rewards, parse_agent_role,
};
use persistence::{
    append_cfactor_snapshot, append_jsonl_record, append_task_metric, count_episode_records,
    load_task_metrics, read_efficiency_events as read_efficiency_events_impl,
    read_efficiency_summaries as read_efficiency_summaries_impl,
    read_gate_outcomes as read_gate_outcomes_impl,
    read_knowledge_seeds as read_knowledge_seeds_impl,
    read_retry_outcomes as read_retry_outcomes_impl,
};
use routing::{compute_reward_with_latency, sync_experiment_winner_artifact};

type EpisodeCompletionHook = Arc<dyn Fn(Episode) + Send + Sync>;

fn affect_state_path(learn_root: &Path) -> PathBuf {
    let root = learn_root.parent().unwrap_or(learn_root);
    root.join("daimon").join("affect.json")
}

// ── Regression helpers ────────────────────────────────────────────────

use crate::baseline;
use crate::regression::RegressionReport;

/// Compute a regression report using historical records.
///
/// Uses all-but-last-`current_window` records as baseline and the latest
/// window as current. Returns `None` when there is insufficient history.
fn compute_regression_report(
    metrics: &[TaskMetric],
    cfg: &RegressionConfig,
) -> Option<RegressionReport> {
    let min = cfg.thresholds.min_records;
    if metrics.len() < min.saturating_mul(2) {
        return None;
    }

    let window = cfg
        .current_window
        .max(min)
        .min(metrics.len().saturating_sub(min));
    if window == 0 || metrics.len() <= window {
        return None;
    }

    let split = metrics.len() - window;
    let baseline_records = &metrics[..split];
    let current_records = &metrics[split..];
    let baseline = baseline::compute_baseline(baseline_records, min);
    Some(detect_regressions(
        &baseline,
        current_records,
        &cfg.thresholds,
    ))
}

// ── WAL recovery + experiment merging ─────────────────────────────────

/// Save what the learning WAL holds but no snapshot does yet. Call it before
/// the runtime loads its snapshots, as a Graph run does before it loads its
/// router ([`crate::model_call_feedback::load_recovered_router`]).
///
/// The WAL is the shared `wal.jsonl` and the segments whose writers are gone.
/// A live writer's segment is left to that writer, which saves its entries
/// itself; replaying them as well would count them twice (bug-84de98).
/// Cascade observations are replayed by a router that tracks every model the
/// entries name, not the models this runtime routes between, so an opener
/// with a narrower model list keeps the other models' entries (bug-7a2630);
/// the snapshot merge leaves the models it does not track alone. The WAL is
/// emptied only once the snapshots that hold its entries are saved.
pub(crate) fn recover_wal(paths: &LearningPaths) {
    let mut entries = wal::replay_wal(&paths.wal_jsonl).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "[wal] shared WAL unreadable -- leaving it");
        Vec::new()
    });
    let shared_entries = entries.len();
    let orphans = wal::orphaned_segments(&paths.wal_segments_dir).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "[wal] segment scan failed -- replaying the shared WAL only");
        Vec::new()
    });
    for orphan in &orphans {
        entries.extend(orphan.entries().iter().cloned());
    }

    if !entries.is_empty() {
        tracing::info!(entries = entries.len(), "[wal] replaying learning WAL");
        let cascade_saved = save_recovered_observations(&paths.cascade_router_json, &entries);
        let experiments_saved =
            save_recovered_experiment_outcomes(&paths.experiments_json, &entries);
        if !(cascade_saved && experiments_saved) {
            tracing::warn!(
                "[wal] retaining entries because one or more replay snapshots did not commit"
            );
            return;
        }
    }
    if shared_entries > 0
        && let Err(e) = wal::truncate_wal(&paths.wal_jsonl)
    {
        tracing::warn!(error = %e, "[wal] truncate after replay failed");
    }
    for orphan in orphans {
        if let Err(e) = orphan.remove() {
            tracing::warn!(error = %e, "[wal] replayed segment not removed");
        }
    }
}

/// A cascade router update journaled in the WAL.
enum RecoveredUpdate<'a> {
    /// An observation.
    Observation(RecoveredObservation<'a>),
    /// A success that a hindsight relabel retracted from the model's
    /// confidence stats (bug-583e50).
    Retraction { model_slug: &'a str },
}

impl<'a> RecoveredUpdate<'a> {
    /// The model the update is for.
    fn model_slug(&self) -> &'a str {
        match self {
            Self::Observation(observation) => observation.model_slug,
            Self::Retraction { model_slug } => *model_slug,
        }
    }
}

/// A cascade observation journaled in the WAL.
struct RecoveredObservation<'a> {
    model_slug: &'a str,
    context_features: &'a [f64],
    reward: f64,
    success: bool,
    /// Share of a full observation its `LinUCB` update carried.
    weight: f64,
}

/// The cascade router update `entry` journals, unless a saved snapshot
/// already holds it.
fn recovered_update<'a>(
    entry: &'a WalEntry,
    folded: &HashSet<&str>,
) -> Option<RecoveredUpdate<'a>> {
    // A retraction undoes a success replayed before it, or one that the
    // snapshot holds already.
    if let WalEntry::SuccessRetraction { model_slug, .. } = entry {
        return Some(RecoveredUpdate::Retraction { model_slug });
    }
    recovered_observation(entry, folded).map(RecoveredUpdate::Observation)
}

/// The cascade observation `entry` journals, unless a saved snapshot already
/// holds it.
fn recovered_observation<'a>(
    entry: &'a WalEntry,
    folded: &HashSet<&str>,
) -> Option<RecoveredObservation<'a>> {
    match entry {
        WalEntry::CascadeObservation {
            model_slug,
            context_features,
            reward,
            success,
            ..
        } => Some(RecoveredObservation {
            model_slug,
            context_features,
            reward: *reward,
            success: *success,
            weight: 1.0,
        }),
        // A model-call surface or a Graph run journaled this observation,
        // but no saved snapshot contains it (find-0dc1d5, bug-dfb28f).
        WalEntry::ModelCallObservation {
            id,
            model_slug,
            context_features,
            reward,
            success,
            weight,
            ..
        } if !folded.contains(id.as_str()) => Some(RecoveredObservation {
            model_slug,
            context_features,
            reward: *reward,
            success: *success,
            weight: *weight,
        }),
        _ => None,
    }
}

/// Replay the cascade router updates in `entries`, in order, into the
/// snapshot at `snapshot_path`, and report whether the snapshot now holds
/// them.
///
/// The replaying router tracks exactly the models the entries name, so no
/// entry is skipped as an unknown model (bug-7a2630).
fn save_recovered_observations(snapshot_path: &Path, entries: &[WalEntry]) -> bool {
    let folded = wal::folded_model_call_ids(entries);
    let updates = entries
        .iter()
        .filter_map(|entry| recovered_update(entry, &folded))
        .collect::<Vec<_>>();
    let mut models: Vec<String> = Vec::new();
    for update in &updates {
        if !models.iter().any(|model| model == update.model_slug()) {
            models.push(update.model_slug().to_string());
        }
    }
    if models.is_empty() {
        return true;
    }

    let router = CascadeRouter::load_or_new(snapshot_path, models);
    for update in &updates {
        let observation = match update {
            RecoveredUpdate::Observation(observation) => observation,
            RecoveredUpdate::Retraction { model_slug } => {
                router.replay_retraction(model_slug);
                continue;
            }
        };
        let Some(model_idx) = router.model_index_for_slug(observation.model_slug) else {
            continue;
        };
        router.replay_weighted_observation(
            observation.model_slug,
            observation.context_features,
            model_idx,
            observation.reward,
            observation.success,
            observation.weight,
        );
    }
    match router.save(snapshot_path) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(error = %e, "[wal] cascade-router snapshot after replay failed");
            false
        }
    }
}

/// Replay the prompt experiment outcomes in `entries` into the store at
/// `experiments_path`, and report whether the store now holds them.
fn save_recovered_experiment_outcomes(experiments_path: &Path, entries: &[WalEntry]) -> bool {
    let outcomes = entries
        .iter()
        .filter_map(|entry| match entry {
            WalEntry::ExperimentOutcome {
                variant_id,
                success,
                ..
            } => Some((variant_id.as_str(), *success)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if outcomes.is_empty() {
        return true;
    }
    match ExperimentStore::transaction(experiments_path, |latest| {
        // Legacy WAL records predate attempt/experiment scoping. Preserve
        // their historical global lookup during replay; new assignments
        // settle through the scoped attempt API.
        for (variant_id, success) in &outcomes {
            latest.replay_outcome(variant_id, *success);
        }
        Ok(())
    }) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(error = %e, "[wal] experiment transaction after replay failed");
            false
        }
    }
}

fn merge_missing_experiments(target: &mut ExperimentStore, source: &ExperimentStore) {
    for experiment in source.iter().cloned() {
        target.register(experiment);
    }
}

fn commit_experiment_snapshot(path: &Path, local: &ExperimentStore) -> io::Result<ExperimentStore> {
    ExperimentStore::transaction(path, |latest| {
        merge_missing_experiments(latest, local);
        Ok(latest.clone())
    })
}

/// Bootstrap the learning runtime's legacy in-memory health view from the
/// persisted registry. The provider-call boundary remains the sole owner of
/// canonical persisted outcomes; replaying state here avoids double-counting
/// when `record_completed_run` updates its local learning view.
fn provider_health_tracker_from_persisted(root: &Path) -> ProviderHealthTracker {
    let health_path = root.join("provider-health.json");
    let registry = ProviderHealthRegistry::load_or_new(&health_path);
    let snapshot = registry.snapshot();
    let tracker = ProviderHealthTracker::new();
    for (provider_id, health) in &snapshot {
        match health.state {
            CircuitState::Open => {
                tracker.record_failure(provider_id);
                tracker.record_failure(provider_id);
                tracker.record_failure(provider_id);
            }
            CircuitState::Closed | CircuitState::HalfOpen => {
                tracker.record_success(provider_id);
            }
        }
    }
    tracker
}

// ── LearningRuntime ───────────────────────────────────────────────────

/// The central learning runtime that aggregates all subsystem feedback from
/// completed agent runs.
pub struct LearningRuntime {
    paths: LearningPaths,
    episode_logger: EpisodeLogger,
    update_frequency: UpdateFrequency,
    episode_count: AtomicU64,
    affect_engine: parking_lot::Mutex<DaimonState>,
    costs_log: CostsLog,
    costs_db: CostsDb,
    provider_health: ProviderHealthTracker,
    skill_library: SkillLibrary,
    pub(crate) playbook_store: PlaybookStore,
    pub(crate) playbook_rules: PlaybookRules,
    regression: RegressionConfig,
    task_metrics: AsyncMutex<Vec<TaskMetric>>,
    pub(crate) latency_registry: LatencyRegistry,
    cascade_router: CascadeRouter,
    context_pack_cache: ContextPackCache,
    experiment_store: parking_lot::Mutex<ExperimentStore>,
    local_rewards: parking_lot::Mutex<HashMap<String, LocalRewardFunction>>,
    pub(crate) section_effectiveness: parking_lot::Mutex<SectionEffectivenessRegistry>,
    provider_model_outcomes: ProviderModelOutcomeStore,
    episode_completion_hook: Option<EpisodeCompletionHook>,
    /// The runtime's WAL segment, created on its first entry.
    wal: parking_lot::Mutex<Option<WalSegment>>,
}

impl LearningRuntime {
    /// Open a runtime at `paths` and preload persisted state.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence files cannot be read/initialized.
    pub async fn open(
        paths: LearningPaths,
        regression: RegressionConfig,
    ) -> Result<Self, LearningRuntimeError> {
        tokio::fs::create_dir_all(&paths.root).await?;
        tokio::fs::create_dir_all(&paths.playbooks_dir).await?;
        let affect_path = affect_state_path(&paths.root);
        if let Some(parent) = affect_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let episode_logger = EpisodeLogger::new(&paths.episodes_jsonl);
        let costs_log = CostsLog::open_creating(&paths.costs_jsonl).await?;
        let costs_db = CostsDb::new();
        let existing_costs = costs_log.read_all().await?;
        costs_db.insert_batch(existing_costs);
        let episode_count = count_episode_records(&paths.episodes_jsonl).await?;

        let skill_library = SkillLibrary::new(&paths.skills_json).await?;
        let playbook_store = PlaybookStore::new(&paths.playbooks_dir);
        let playbook_rules = PlaybookRules::open(&paths.playbook_rules_toml)?;
        let task_metrics = load_task_metrics(&paths.task_metrics_jsonl).await?;

        let latency_registry = LatencyRegistry::load_or_new(&paths.latency_stats_json);
        // Save what the WAL holds but the snapshots don't, before loading them.
        recover_wal(&paths);
        let cascade_router = CascadeRouter::load_or_new(
            &paths.cascade_router_json,
            vec!["claude-sonnet-4-5".into(), "claude-haiku-4-5".into()],
        );
        let context_pack_cache = ContextPackCache::new(256, paths.root.join("context-cache.json"));
        let experiment_store = ExperimentStore::load_or_new(&paths.experiments_json);
        let local_rewards = load_local_rewards(&paths.local_rewards_json);
        let section_effectiveness =
            SectionEffectivenessRegistry::load_or_new(&paths.section_effects_json);
        let provider_model_outcomes =
            ProviderModelOutcomeStore::open_creating(&paths.provider_model_outcomes_jsonl).await?;

        sync_experiment_winner_artifact(&paths.experiment_winners_json, &experiment_store)?;

        let provider_health = provider_health_tracker_from_persisted(&paths.root);

        Ok(Self {
            paths,
            episode_logger,
            update_frequency: UpdateFrequency::default(),
            episode_count: AtomicU64::new(episode_count),
            affect_engine: parking_lot::Mutex::new(DaimonState::load_or_new(&affect_path)),
            costs_log,
            costs_db,
            provider_health,
            skill_library,
            playbook_store,
            playbook_rules,
            regression,
            task_metrics: AsyncMutex::new(task_metrics),
            latency_registry,
            cascade_router,
            context_pack_cache,
            experiment_store: parking_lot::Mutex::new(experiment_store),
            local_rewards: parking_lot::Mutex::new(local_rewards),
            section_effectiveness: parking_lot::Mutex::new(section_effectiveness),
            provider_model_outcomes,
            episode_completion_hook: None,
            wal: parking_lot::Mutex::new(None),
        })
    }

    /// Open a runtime with a custom model list for the cascade router.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence files cannot be read/initialized.
    pub async fn open_with_models(
        paths: LearningPaths,
        regression: RegressionConfig,
        models: Vec<String>,
    ) -> Result<Self, LearningRuntimeError> {
        tokio::fs::create_dir_all(&paths.root).await?;
        tokio::fs::create_dir_all(&paths.playbooks_dir).await?;
        let affect_path = affect_state_path(&paths.root);
        if let Some(parent) = affect_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let episode_logger = EpisodeLogger::new(&paths.episodes_jsonl);
        let costs_log = CostsLog::open_creating(&paths.costs_jsonl).await?;
        let costs_db = CostsDb::new();
        let existing_costs = costs_log.read_all().await?;
        costs_db.insert_batch(existing_costs);
        let episode_count = count_episode_records(&paths.episodes_jsonl).await?;

        let skill_library = SkillLibrary::new(&paths.skills_json).await?;
        let playbook_store = PlaybookStore::new(&paths.playbooks_dir);
        let playbook_rules = PlaybookRules::open(&paths.playbook_rules_toml)?;
        let task_metrics = load_task_metrics(&paths.task_metrics_jsonl).await?;

        let latency_registry = LatencyRegistry::load_or_new(&paths.latency_stats_json);
        // Save what the WAL holds but the snapshots don't, before loading them.
        recover_wal(&paths);
        let cascade_router = CascadeRouter::load_or_new(&paths.cascade_router_json, models);
        let context_pack_cache = ContextPackCache::new(256, paths.root.join("context-cache.json"));
        let experiment_store = ExperimentStore::load_or_new(&paths.experiments_json);
        let local_rewards = load_local_rewards(&paths.local_rewards_json);
        let section_effectiveness =
            SectionEffectivenessRegistry::load_or_new(&paths.section_effects_json);
        let provider_model_outcomes =
            ProviderModelOutcomeStore::open_creating(&paths.provider_model_outcomes_jsonl).await?;

        sync_experiment_winner_artifact(&paths.experiment_winners_json, &experiment_store)?;

        let provider_health = provider_health_tracker_from_persisted(&paths.root);

        Ok(Self {
            paths,
            episode_logger,
            update_frequency: UpdateFrequency::default(),
            episode_count: AtomicU64::new(episode_count),
            affect_engine: parking_lot::Mutex::new(DaimonState::load_or_new(&affect_path)),
            costs_log,
            costs_db,
            provider_health,
            skill_library,
            playbook_store,
            playbook_rules,
            regression,
            task_metrics: AsyncMutex::new(task_metrics),
            latency_registry,
            cascade_router,
            context_pack_cache,
            experiment_store: parking_lot::Mutex::new(experiment_store),
            local_rewards: parking_lot::Mutex::new(local_rewards),
            section_effectiveness: parking_lot::Mutex::new(section_effectiveness),
            provider_model_outcomes,
            episode_completion_hook: None,
            wal: parking_lot::Mutex::new(None),
        })
    }

    /// Convenience constructor using default paths under `root` and default regression config.
    pub async fn open_under(root: impl Into<PathBuf>) -> Result<Self, LearningRuntimeError> {
        Self::open(
            LearningPaths::for_runtime_root(root),
            RegressionConfig::default(),
        )
        .await
    }

    /// Open a runtime at `root` with a custom model list for the cascade router.
    pub async fn open_under_with_models(
        root: impl Into<PathBuf>,
        models: Vec<String>,
    ) -> Result<Self, LearningRuntimeError> {
        Self::open_with_models(
            LearningPaths::for_runtime_root(root),
            RegressionConfig::default(),
            models,
        )
        .await
    }

    /// Open the learning runtime for a project, writing episodes only to the
    /// canonical root `.roko/episodes.jsonl` log.
    pub async fn open_for_project(workdir: impl AsRef<Path>) -> Result<Self, LearningRuntimeError> {
        let workdir = workdir.as_ref();
        roko_fs::RokoLayout::for_project(workdir)
            .ensure_dirs()
            .await?;
        Self::open(
            LearningPaths::for_project(workdir),
            RegressionConfig::default(),
        )
        .await
    }

    /// Open the project learning runtime with an explicit model set.
    pub async fn open_for_project_with_models(
        workdir: impl AsRef<Path>,
        models: Vec<String>,
    ) -> Result<Self, LearningRuntimeError> {
        let workdir = workdir.as_ref();
        roko_fs::RokoLayout::for_project(workdir)
            .ensure_dirs()
            .await?;
        Self::open_with_models(
            LearningPaths::for_project(workdir),
            RegressionConfig::default(),
            models,
        )
        .await
    }

    // ── Accessors ─────────────────────────────────────────────────────

    /// Borrow configured paths.
    #[must_use]
    pub const fn paths(&self) -> &LearningPaths {
        &self.paths
    }
    /// Borrow the configured subsystem update cadences.
    #[must_use]
    pub const fn update_frequency(&self) -> &UpdateFrequency {
        &self.update_frequency
    }
    /// Override the subsystem update cadences for this runtime.
    pub fn set_update_frequency(&mut self, update_frequency: UpdateFrequency) {
        self.update_frequency = update_frequency;
    }
    /// Borrow in-memory costs DB.
    #[must_use]
    pub const fn costs_db(&self) -> &CostsDb {
        &self.costs_db
    }
    /// Borrow provider health tracker.
    #[must_use]
    pub const fn provider_health(&self) -> &ProviderHealthTracker {
        &self.provider_health
    }
    /// Borrow skill library.
    #[must_use]
    pub const fn skill_library(&self) -> &SkillLibrary {
        &self.skill_library
    }
    /// Mutably borrow the skill library (e.g. for recording outcomes).
    pub const fn skill_library_mut(&mut self) -> &mut SkillLibrary {
        &mut self.skill_library
    }
    /// Borrow playbook rules.
    #[must_use]
    pub const fn playbook_rules(&self) -> &PlaybookRules {
        &self.playbook_rules
    }
    /// Borrow the latency registry used for routing feedback.
    #[must_use]
    pub const fn latency_registry(&self) -> &LatencyRegistry {
        &self.latency_registry
    }
    /// Borrow cascade router.
    #[must_use]
    pub const fn cascade_router(&self) -> &CascadeRouter {
        &self.cascade_router
    }
    /// Borrow context pack cache.
    #[must_use]
    pub const fn context_pack_cache(&self) -> &ContextPackCache {
        &self.context_pack_cache
    }
    /// Borrow experiment store (behind `parking_lot::Mutex`).
    #[must_use]
    pub const fn experiment_store(&self) -> &parking_lot::Mutex<ExperimentStore> {
        &self.experiment_store
    }

    /// Return a snapshot of the learned section-effectiveness registry.
    #[must_use]
    pub fn section_effectiveness_snapshot(&self) -> SectionEffectivenessRegistry {
        self.section_effectiveness.lock().clone()
    }

    /// Inject config-sourced model tiers into the cascade router.
    ///
    /// Call this after construction when the `RokoConfig` is available,
    /// so the router uses explicit `tier` fields from `roko.toml` instead
    /// of substring heuristics.
    pub fn set_model_tiers(
        &mut self,
        models: &indexmap::IndexMap<String, roko_core::config::ModelProfile>,
    ) {
        self.cascade_router.set_model_tiers(models);
    }

    /// Filter model slugs down to those whose providers are currently healthy.
    pub fn healthy_model_slugs<F>(&self, all_model_slugs: &[String], provider_of: F) -> Vec<String>
    where
        F: Fn(&str) -> String,
    {
        let healthy_models = self
            .provider_health
            .filter_arms_or_best(all_model_slugs, provider_of);
        if healthy_models.is_empty() {
            all_model_slugs.to_vec()
        } else {
            healthy_models
        }
    }

    // ── Local reward ──────────────────────────────────────────────────

    /// Query the local reward score for a subsystem decision.
    pub fn local_reward_score(&self, subsystem: &str, decision_key: &str) -> f64 {
        self.local_rewards
            .lock()
            .get(subsystem)
            .map_or(0.5, |reward| reward.score(decision_key))
    }

    /// Record a local decision outcome against global task success for the
    /// named subsystem.
    fn observe_local_reward(&self, subsystem: &str, decision_key: &str, global_success: bool) {
        self.local_rewards
            .lock()
            .entry(subsystem.to_owned())
            .or_default()
            .observe(decision_key, global_success);
    }

    /// Persist local reward functions to disk.
    fn save_local_rewards(&self) {
        let rewards = self.local_rewards.lock();
        if let Ok(json) = serde_json::to_string_pretty(&*rewards) {
            let _ = std::fs::write(&self.paths.local_rewards_json, json);
        }
    }

    // ── Episode hook ──────────────────────────────────────────────────

    /// Install a callback that runs after a completed episode is
    /// persisted.
    ///
    /// The callback is synchronous so it can enqueue background work
    /// without holding up the learning runtime.
    pub fn set_episode_completion_hook<F>(&mut self, hook: F)
    where
        F: Fn(Episode) + Send + Sync + 'static,
    {
        self.episode_completion_hook = Some(Arc::new(hook));
    }

    // ── Runner event facade ───────────────────────────────────────────

    /// Consume one normalized runner event and append all canonical feedback records.
    pub async fn record_runner_event(
        &self,
        event: RunnerFeedbackEvent,
    ) -> Result<RuntimeFeedbackWrite, LearningRuntimeError> {
        let mut write = RuntimeFeedbackWrite::default();

        match event {
            RunnerFeedbackEvent::CompletedRun { input } => {
                let update = self.record_completed_run(*input).await?;
                write.provider_model_outcomes +=
                    usize::from(update.provider_model_outcome_recorded == ApplyStatus::Applied);
                write.efficiency_summaries +=
                    usize::from(update.efficiency_summary_recorded == ApplyStatus::Applied);
                write.gate_outcomes += update.gate_outcomes_recorded;
                write.retry_outcomes +=
                    usize::from(update.retry_outcome_recorded == ApplyStatus::Applied);
                write.knowledge_seeds +=
                    usize::from(update.knowledge_seed_recorded == ApplyStatus::Applied);
                write.learning_update = Some(update);
            }
            RunnerFeedbackEvent::Episode { episode } => {
                self.append_episode(&episode).await?;
                write.episode_appended = true;
                write.merge(self.append_derived_episode_feedback(&episode, true).await?);
            }
            RunnerFeedbackEvent::EfficiencyEvent { event, scope } => {
                let provider_model_outcome =
                    ProviderModelOutcomeRecord::from_efficiency_event(&event).is_some();
                self.append_efficiency_event_with_scope(&event, scope)
                    .await?;
                write.efficiency_events = 1;
                write.efficiency_summaries = 1;
                write.provider_model_outcomes = usize::from(provider_model_outcome);
            }
            RunnerFeedbackEvent::ProviderModelOutcome { outcome } => {
                self.append_provider_model_outcome(&outcome).await?;
                write.provider_model_outcomes = 1;
            }
            RunnerFeedbackEvent::EfficiencySummary { summary } => {
                self.append_efficiency_summary(&summary).await?;
                write.efficiency_summaries = 1;
            }
            RunnerFeedbackEvent::GateOutcome { outcome } => {
                self.append_gate_outcome(&outcome).await?;
                write.gate_outcomes = 1;
            }
            RunnerFeedbackEvent::RetryOutcome { outcome } => {
                self.append_retry_outcome(&outcome).await?;
                write.retry_outcomes = 1;
            }
            RunnerFeedbackEvent::KnowledgeSeed { seed } => {
                self.append_knowledge_seed(&seed).await?;
                write.knowledge_seeds = 1;
            }
        }

        Ok(write)
    }

    // ── Generation outcome ────────────────────────────────────────────

    /// Record a generation outcome while distinguishing process success from artifact validity.
    pub async fn record_generation_outcome(
        &self,
        task_id: &str,
        model: &str,
        outcome: &GenerationOutcome,
    ) -> Result<(), LearningRuntimeError> {
        let mut episode = Episode::new("roko-cli", task_id);
        episode.kind = "generation".to_string();
        episode.agent_template = "generator".to_string();
        episode.model = model.to_string();
        episode.trigger_kind = "generation".to_string();
        episode.success = outcome.fully_successful();
        episode.failure_reason = if outcome.process_success && !outcome.artifact_valid {
            Some("artifact validation failed".to_string())
        } else if !outcome.process_success {
            Some("generation process failed".to_string())
        } else {
            None
        };
        episode.extra.insert(
            "process_success".to_string(),
            serde_json::json!(outcome.process_success),
        );
        episode.extra.insert(
            "artifact_valid".to_string(),
            serde_json::json!(outcome.artifact_valid),
        );
        episode.extra.insert(
            "generation_status".to_string(),
            serde_json::json!(outcome.status_label()),
        );
        if let Some(report) = &outcome.validation_report {
            episode
                .extra
                .insert("validation_report".to_string(), report.clone());
        }
        episode.attach_all_fingerprints();

        self.record_runner_event(RunnerFeedbackEvent::Episode {
            episode: Box::new(episode),
        })
        .await
        .map(|_| ())
    }

    // ── Efficiency / summary / gate / retry / seed append ─────────────

    /// Append an efficiency event to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error on write failure.
    pub async fn append_efficiency_event(
        &self,
        event: &AgentEfficiencyEvent,
    ) -> Result<(), LearningRuntimeError> {
        self.append_efficiency_event_with_scope(event, EfficiencyScope::Turn)
            .await
    }

    async fn append_efficiency_event_with_scope(
        &self,
        event: &AgentEfficiencyEvent,
        scope: EfficiencyScope,
    ) -> Result<(), LearningRuntimeError> {
        append_jsonl_record(&self.paths.efficiency_jsonl, event).await?;
        self.record_latency_from_efficiency_event(event)?;
        self.record_section_effectiveness_from_efficiency_event(event)?;
        let summary = EfficiencySummaryRecord::from_efficiency_event(event, scope);
        self.append_efficiency_summary(&summary).await?;
        if let Some(outcome) = ProviderModelOutcomeRecord::from_efficiency_event(event) {
            self.append_provider_model_outcome(&outcome).await?;
        }
        Ok(())
    }

    /// Append a normalized efficiency summary to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error on serialization or write failure.
    pub async fn append_efficiency_summary(
        &self,
        summary: &EfficiencySummaryRecord,
    ) -> Result<(), LearningRuntimeError> {
        append_jsonl_record(&self.paths.efficiency_summaries_jsonl, summary).await
    }

    /// Append a provider/model outcome to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error on serialization or write failure.
    pub async fn append_provider_model_outcome(
        &self,
        outcome: &ProviderModelOutcomeRecord,
    ) -> Result<(), LearningRuntimeError> {
        self.provider_model_outcomes.append(outcome).await?;
        Ok(())
    }

    /// Append a gate outcome to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error on serialization or write failure.
    pub async fn append_gate_outcome(
        &self,
        outcome: &GateOutcomeRecord,
    ) -> Result<(), LearningRuntimeError> {
        append_jsonl_record(&self.paths.gate_outcomes_jsonl, outcome).await
    }

    /// Append multiple gate outcomes to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error on the first serialization or write failure.
    pub async fn append_gate_outcomes(
        &self,
        outcomes: &[GateOutcomeRecord],
    ) -> Result<(), LearningRuntimeError> {
        for outcome in outcomes {
            self.append_gate_outcome(outcome).await?;
        }
        Ok(())
    }

    /// Append a retry outcome to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error on serialization or write failure.
    pub async fn append_retry_outcome(
        &self,
        outcome: &RetryOutcomeRecord,
    ) -> Result<(), LearningRuntimeError> {
        append_jsonl_record(&self.paths.retry_outcomes_jsonl, outcome).await
    }

    /// Append a knowledge seed to the JSONL log.
    ///
    /// # Errors
    ///
    /// Returns an error on serialization or write failure.
    pub async fn append_knowledge_seed(
        &self,
        seed: &KnowledgeSeedRecord,
    ) -> Result<(), LearningRuntimeError> {
        append_jsonl_record(&self.paths.knowledge_seeds_jsonl, seed).await
    }

    // ── Routing reward ────────────────────────────────────────────────

    /// Compute a latency-aware routing reward for a model/provider observation.
    ///
    /// The current wall-clock latency comes from the efficiency event emitted
    /// for this turn. When that timing is unavailable, the historical p50 for
    /// the same `(model, provider)` pair is used as a fallback.
    #[must_use]
    pub fn compute_routing_reward_with_latency(
        &self,
        gate_passed: bool,
        cost_usd: f64,
        wall_time_ms: u64,
        model: &str,
        provider: &str,
    ) -> f64 {
        compute_reward_with_latency(
            gate_passed,
            cost_usd,
            wall_time_ms,
            &self.latency_registry,
            model,
            provider,
        )
    }

    // ── Read helpers (delegate to persistence) ────────────────────────

    /// Read all persisted efficiency events from the JSONL log.
    ///
    /// Returns an empty vec if the file does not exist.
    pub async fn read_efficiency_events(
        &self,
    ) -> Result<Vec<AgentEfficiencyEvent>, LearningRuntimeError> {
        read_efficiency_events_impl(&self.paths.efficiency_jsonl).await
    }

    /// Read all persisted provider/model outcome telemetry records.
    ///
    /// Returns an empty vec if the file does not exist.
    pub async fn read_provider_model_outcomes(
        &self,
    ) -> Result<Vec<ProviderModelOutcomeRecord>, LearningRuntimeError> {
        read_provider_model_outcomes(&self.paths.provider_model_outcomes_jsonl)
            .await
            .map_err(LearningRuntimeError::Io)
    }

    /// Read all persisted efficiency summaries.
    ///
    /// Returns an empty vec if the file does not exist.
    pub async fn read_efficiency_summaries(
        &self,
    ) -> Result<Vec<EfficiencySummaryRecord>, LearningRuntimeError> {
        read_efficiency_summaries_impl(&self.paths.efficiency_summaries_jsonl).await
    }

    /// Read all persisted gate outcomes.
    ///
    /// Returns an empty vec if the file does not exist.
    pub async fn read_gate_outcomes(&self) -> Result<Vec<GateOutcomeRecord>, LearningRuntimeError> {
        read_gate_outcomes_impl(&self.paths.gate_outcomes_jsonl).await
    }

    /// Read all persisted retry outcomes.
    ///
    /// Returns an empty vec if the file does not exist.
    pub async fn read_retry_outcomes(
        &self,
    ) -> Result<Vec<RetryOutcomeRecord>, LearningRuntimeError> {
        read_retry_outcomes_impl(&self.paths.retry_outcomes_jsonl).await
    }

    /// Read all persisted knowledge seeds.
    ///
    /// Returns an empty vec if the file does not exist.
    pub async fn read_knowledge_seeds(
        &self,
    ) -> Result<Vec<KnowledgeSeedRecord>, LearningRuntimeError> {
        read_knowledge_seeds_impl(&self.paths.knowledge_seeds_jsonl).await
    }

    /// Query all canonical runtime feedback streams for this runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if an existing log cannot be opened or read.
    pub async fn query_feedback(
        &self,
        query: &RuntimeFeedbackQuery,
    ) -> Result<RuntimeFeedbackSnapshot, LearningRuntimeError> {
        persistence::read_runtime_feedback_snapshot(&self.paths, query).await
    }

    /// Return rolling provider/model pass-rate summaries.
    ///
    /// # Errors
    ///
    /// Returns an error if the outcome log cannot be opened or read.
    pub async fn provider_model_pass_rates(
        &self,
        window_size: usize,
    ) -> Result<ProviderModelPassRateReport, LearningRuntimeError> {
        let records = self.read_provider_model_outcomes().await?;
        Ok(summarize_provider_model_outcomes(&records, window_size))
    }

    /// Read the latest persisted C-Factor snapshot, if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot file cannot be read.
    pub async fn latest_cfactor(&self) -> Result<Option<CFactor>, LearningRuntimeError> {
        let contents = match tokio::fs::read_to_string(&self.paths.cfactor_jsonl).await {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(LearningRuntimeError::Io(err)),
        };
        Ok(contents
            .lines()
            .rev()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .and_then(|line| serde_json::from_str::<CFactor>(line).ok()))
    }

    // ── WAL ───────────────────────────────────────────────────────────

    /// Append a learning event to the runtime's WAL segment for crash-safe
    /// durability.
    ///
    /// If the segment's entry count reaches the configured max, an automatic
    /// compaction (snapshot + truncate) is triggered.
    fn wal_append(&self, entry: WalEntry) {
        let mut segment = self.wal.lock();
        let dir = &self.paths.wal_segments_dir;
        if let Err(e) = wal::append_to_segment(&mut segment, dir, &entry) {
            tracing::warn!(error = %e, "[wal] append failed -- learning not durable this entry");
            return;
        }
        if let Some(segment) = segment.as_mut()
            && segment.entry_count() >= roko_core::defaults::DEFAULT_LEARN_WAL_MAX_ENTRIES
        {
            self.compact_wal_locked(segment);
        }
    }

    /// Record a gate threshold EMA update in the WAL.
    ///
    /// Gate thresholds live in `roko-gate` which must not depend on
    /// `roko-learn`. This public method lets the runner event loop
    /// write a gate threshold WAL entry after updating the in-memory
    /// threshold state.
    pub fn wal_append_gate_threshold(&self, rung: u32, passed: bool) {
        self.wal_append(WalEntry::GateThresholdUpdate {
            rung,
            passed,
            ts_ms: Utc::now().timestamp_millis(),
        });
    }

    /// Atomically write pre-serialized adaptive gate thresholds JSON to
    /// the learn directory (`gate-thresholds.json`).
    ///
    /// This is the incremental flush path: callers serialize their
    /// `AdaptiveThresholds` (which lives in `roko-gate`) and hand the JSON
    /// string to the learning runtime, which owns the output path.
    ///
    /// # Errors
    ///
    /// Returns an IO error if the parent directory cannot be created or
    /// the atomic write fails.
    pub fn flush_gate_thresholds_json(&self, json: &str) -> std::io::Result<()> {
        if let Some(parent) = self.paths.gate_thresholds_json.parent() {
            std::fs::create_dir_all(parent)?;
        }
        roko_fs::atomic_write_bytes(&self.paths.gate_thresholds_json, json.as_bytes())
    }

    fn compact_wal_locked(&self, wal: &mut WalSegment) {
        let cascade_saved = match self.cascade_router.save(&self.paths.cascade_router_json) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!(error = %e, "[wal] cascade-router snapshot failed during compaction");
                false
            }
        };
        let local = self.experiment_store.lock().clone();
        let experiments_saved = match commit_experiment_snapshot(
            &self.paths.experiments_json,
            &local,
        ) {
            Ok(committed) => {
                *self.experiment_store.lock() = committed;
                true
            }
            Err(e) => {
                tracing::warn!(error = %e, "[wal] experiment transaction failed during compaction");
                false
            }
        };
        if cascade_saved && experiments_saved {
            if let Err(e) = wal.truncate() {
                tracing::warn!(error = %e, "[wal] truncate failed during compaction");
            }
        } else {
            tracing::warn!("[wal] retaining entries because compaction snapshots did not commit");
        }
    }

    fn commit_experiment_snapshot(&self) -> io::Result<()> {
        let local = self.experiment_store.lock().clone();
        let committed = commit_experiment_snapshot(&self.paths.experiments_json, &local)?;
        *self.experiment_store.lock() = committed;
        Ok(())
    }

    fn truncate_wal_after_experiment_snapshot(&self, wal: &mut WalSegment) {
        if let Err(e) = self.commit_experiment_snapshot() {
            tracing::warn!(error = %e, "[wal] experiment transaction failed during cascade-router save");
            return;
        }
        if let Err(e) = wal.truncate() {
            tracing::warn!(error = %e, "[wal] truncate failed during compaction");
        }
    }

    /// Save cascade router observations to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the cascade router snapshot cannot be written.
    pub fn save_cascade_router(&self) -> Result<(), LearningRuntimeError> {
        // Held across the save, so the truncation below never drops an entry
        // appended after the snapshot was taken.
        let mut segment = self.wal.lock();
        self.cascade_router.save(&self.paths.cascade_router_json)?;
        if let Some(segment) = segment.as_mut() {
            self.truncate_wal_after_experiment_snapshot(segment);
        }
        Ok(())
    }

    // ── Conductor intervention ────────────────────────────────────────

    /// Record conductor-driven negative feedback for the routed model.
    ///
    /// Restart/fail interventions indicate the selected model failed to make
    /// acceptable progress for the current routing context, so they are fed
    /// back into the cascade router as a zero-reward failure.
    pub fn record_conductor_intervention(
        &self,
        routing_context: &RoutingContext,
        model_slug: &str,
        intervention: &ConductorDecision,
    ) -> bool {
        if !matches!(
            intervention,
            ConductorDecision::Restart { .. } | ConductorDecision::Fail { .. }
        ) {
            return false;
        }
        let context_features = routing_context.to_features();
        let model_idx = self
            .cascade_router
            .model_index_for_slug(model_slug)
            .unwrap_or(0);
        self.cascade_router
            .record_observation(routing_context, model_slug, 0.0, false);
        self.wal_append(WalEntry::CascadeObservation {
            model_slug: model_slug.to_string(),
            context_features,
            model_idx,
            reward: 0.0,
            success: false,
            ts_ms: Utc::now().timestamp_millis(),
        });
        if let Err(err) = self.save_cascade_router() {
            eprintln!("[learn] cascade router save failed after conductor intervention: {err}");
        }
        true
    }

    // ── Episode persistence ───────────────────────────────────────────

    /// Append one raw episode record without triggering any learning updates.
    ///
    /// # Errors
    ///
    /// Returns an error if the episode cannot be appended to the persisted log.
    pub async fn append_episode(&self, episode: &Episode) -> Result<(), LearningRuntimeError> {
        let mut episode = episode.clone();
        self.apply_affect_signature(&mut episode);
        self.episode_logger.append(&episode).await?;
        Ok(())
    }

    async fn append_derived_episode_feedback(
        &self,
        episode: &Episode,
        include_provider_model_outcome: bool,
    ) -> Result<RuntimeFeedbackWrite, LearningRuntimeError> {
        let mut write = RuntimeFeedbackWrite::default();

        if include_provider_model_outcome
            && let Some(outcome) = ProviderModelOutcomeRecord::from_episode(episode, None)
        {
            self.append_provider_model_outcome(&outcome).await?;
            write.provider_model_outcomes = 1;
        }

        let summary = EfficiencySummaryRecord::from_episode(episode);
        self.append_efficiency_summary(&summary).await?;
        write.efficiency_summaries = 1;

        let gate_outcomes = GateOutcomeRecord::from_episode(episode);
        self.append_gate_outcomes(&gate_outcomes).await?;
        write.gate_outcomes = gate_outcomes.len();

        if let Some(retry_outcome) = RetryOutcomeRecord::from_episode(episode) {
            self.append_retry_outcome(&retry_outcome).await?;
            write.retry_outcomes = 1;
        }

        let artifact_valid = extra_bool(episode, "artifact_valid").unwrap_or(true);
        if artifact_valid {
            if let Some(seed) = KnowledgeSeedRecord::from_successful_episode(episode) {
                self.append_knowledge_seed(&seed).await?;
                write.knowledge_seeds = 1;
            }
        } else {
            tracing::info!(
                task_id = %episode.task_id,
                episode_id = %episode.episode_id,
                "Withholding knowledge seed: artifact_valid=false in episode extra"
            );
        }

        Ok(write)
    }

    // ── Core: record_completed_run ────────────────────────────────────

    /// Persist one completed run and update all available learning subsystems.
    pub async fn record_completed_run(
        &self,
        mut input: CompletedRunInput,
    ) -> Result<LearningUpdate, LearningRuntimeError> {
        let mut update = LearningUpdate::default();

        let gate_counts = gate_counts_from_episode(&input.episode);
        let skip_only = gate_counts.is_some_and(GateCountsInner::has_only_skipped);
        if let Some(counts) = gate_counts {
            backfill_gate_counts(&mut input.episode, counts);
        }
        if skip_only {
            input.episode.success = false;
            if input
                .episode
                .failure_reason
                .as_ref()
                .is_none_or(|reason| reason.trim().is_empty())
            {
                input.episode.failure_reason = Some("all gates skipped".to_string());
            }
            input.episode.extra.insert(
                "provider_model_outcome_status".to_string(),
                serde_json::json!("blocked"),
            );
        }

        input.episode.attach_all_fingerprints();
        self.apply_affect_signature(&mut input.episode);
        self.episode_logger.append(&input.episode).await?;
        update.episode_logged = ApplyStatus::Applied;
        if let Some(hook) = &self.episode_completion_hook {
            hook(input.episode.clone());
        }
        let episode_count = self.episode_count.fetch_add(1, Ordering::Relaxed) + 1;

        if !skip_only && let Some(reflection_input) = ReflectionInput::from_episode(&input.episode)
        {
            let mut reflection_store =
                PostGateReflectionStore::load(&self.paths.post_gate_reflections_json);
            let observation =
                reflection_store.observe(reflection_input, ReflectionPromotionConfig::default());
            reflection_store.save(&self.paths.post_gate_reflections_json)?;
            update.reflection_recorded = ApplyStatus::Applied;
            if observation.candidate.is_some() {
                update.reflection_candidate_updated = ApplyStatus::Applied;
            }
        }

        if input.playbook_id.is_none() {
            input.playbook_id = extra_string(&input.episode, "playbook_id");
        }
        if input.playbook_rule_id.is_none() {
            input.playbook_rule_id = extra_string(&input.episode, "playbook_rule_id");
        }
        if input.matched_skill_id.is_none() {
            input.matched_skill_id = extra_string(&input.episode, "skill_name")
                .or_else(|| extra_string(&input.episode, "matched_skill_id"));
        }

        let cost_record = match input.cost_record {
            Some(record) => Some(record),
            None => derive_cost_record(&input.episode, input.provider.as_deref()),
        };

        if let Some(record) = cost_record {
            self.costs_db.insert(record.clone());
            self.costs_log.append(&record).await?;
            update.cost_logged = ApplyStatus::Applied;
            if input.provider.is_none() {
                input.provider = Some(record.provider.clone());
            }
        }

        let provider_for_outcome = input.provider.clone();

        if !skip_only && let Some(provider) = input.provider {
            if input.episode.success {
                self.provider_health.record_success(&provider);
            } else {
                self.provider_health.record_failure(&provider);
            }
            update.provider_updated = ApplyStatus::Applied;
        }
        // An attempt without a learning label (S01 §4.1) has told provider
        // health how the provider did, but, like one whose gates all
        // skipped, it teaches no learner.
        let skip_only = skip_only || input.episode.learning_success().is_none();

        if let Some(outcome) = ProviderModelOutcomeRecord::from_episode(
            &input.episode,
            provider_for_outcome.as_deref(),
        ) {
            self.provider_model_outcomes.append(&outcome).await?;
            update.provider_model_outcome_recorded = ApplyStatus::Applied;
        }

        let derived_feedback = self
            .append_derived_episode_feedback(&input.episode, false)
            .await?;
        if derived_feedback.efficiency_summaries > 0 {
            update.efficiency_summary_recorded = ApplyStatus::Applied;
        }
        update.gate_outcomes_recorded = derived_feedback.gate_outcomes;
        if derived_feedback.retry_outcomes > 0 {
            update.retry_outcome_recorded = ApplyStatus::Applied;
        }
        if derived_feedback.knowledge_seeds > 0 {
            update.knowledge_seed_recorded = ApplyStatus::Applied;
        }

        if !skip_only && let Some(playbook_id) = input.playbook_id {
            if self
                .playbook_store
                .record_outcome(&playbook_id, input.episode.success)
                .await?
            {
                update.playbook_updated = ApplyStatus::Applied;
            }
        }

        let local_reward_rule_id = input.playbook_rule_id.clone();
        let local_reward_skill_id = input.matched_skill_id.clone();

        if !skip_only && let Some(rule_id) = input.playbook_rule_id {
            self.playbook_rules
                .record_outcome(&rule_id, input.episode.success);
            self.playbook_rules.save()?;
            update.playbook_rule_updated = ApplyStatus::Applied;
        }

        if !skip_only
            && let Some(skill_id) = input.matched_skill_id
            && self.skill_library.get(&skill_id).is_some()
        {
            self.skill_library
                .record_outcome(&skill_id, input.episode.success)
                .await?;
            update.matched_skill_updated = ApplyStatus::Applied;
        }

        let generator = TemplatePatternGenerator;
        if !skip_only
            && self.update_frequency.skill_mining_due(episode_count)
            && let Some(skill) = self.skill_library.extract(&input.episode, &generator).await
        {
            update.extracted_skill_id = Some(skill.name);
        }

        if !skip_only && let Some(metric) = input.task_metric {
            append_task_metric(&self.paths.task_metrics_jsonl, &metric).await?;
            let metrics_snapshot = {
                let mut guard = self.task_metrics.lock().await;
                guard.push(metric);
                guard.clone()
            };
            update.regression_report =
                compute_regression_report(&metrics_snapshot, &self.regression);
        }

        if !skip_only && self.update_frequency.distiller_due(episode_count) {
            self.append_cfactor_snapshot().await?;
        }

        // Cascade router observation
        let artifact_valid_for_router =
            extra_bool(&input.episode, "artifact_valid").unwrap_or(true);
        if !skip_only
            && self.update_frequency.router_due(episode_count)
            && artifact_valid_for_router
        {
            update.router_updated = self.update_cascade_router(&input.episode);
        } else if !artifact_valid_for_router {
            tracing::debug!(
                task_id = %input.episode.task_id,
                model = ?extra_string(&input.episode, "model"),
                "Cascade router: skipping positive observation -- artifact_valid=false"
            );
        }

        if update.router_updated {
            if let Err(e) = self.save_cascade_router() {
                eprintln!("[learn] cascade router save failed: {e}");
            }
        }

        // Prompt experiment outcome
        if !skip_only
            && self.update_frequency.experiments_due(episode_count)
            && let Some(ref variant_id) = input.experiment_variant_id
        {
            let local = self.experiment_store.lock().clone();
            let experiment_id = input.experiment_id.as_deref();
            let transaction = ExperimentStore::transaction(
                &self.paths.experiments_json,
                |latest| {
                    merge_missing_experiments(latest, &local);
                    let matched_id = if let Some(experiment_id) = experiment_id {
                        let was_running = latest.get(experiment_id).is_some_and(|experiment| {
                            experiment.status == ExperimentStatus::Running
                        });
                        if !latest.record_outcome_for_experiment(
                            experiment_id,
                            variant_id,
                            input.episode.success,
                        ) {
                            return Err(io::Error::new(
                                io::ErrorKind::NotFound,
                                format!(
                                    "experiment '{experiment_id}' variant '{variant_id}' disappeared before outcome recording"
                                ),
                            ));
                        }
                        Some((experiment_id.to_string(), was_running))
                    } else {
                        let matched = latest
                            .iter()
                            .find(|experiment| experiment.stats.contains_key(variant_id))
                            .map(|experiment| {
                                (
                                    experiment.experiment_id.clone(),
                                    experiment.status == ExperimentStatus::Running,
                                )
                            });
                        latest.record_outcome(variant_id, input.episode.success);
                        matched
                    };
                    Ok((latest.clone(), matched_id))
                },
            );

            match transaction {
                Ok((committed, matched)) => {
                    *self.experiment_store.lock() = committed.clone();
                    let static_table_updated =
                        matched.is_some_and(|(experiment_id, was_running)| {
                            was_running
                                && committed.get(&experiment_id).is_some_and(|experiment| {
                                    self.on_experiment_concluded(experiment)
                                })
                        });
                    if let Err(e) = sync_experiment_winner_artifact(
                        &self.paths.experiment_winners_json,
                        &committed,
                    ) {
                        eprintln!("[learn] experiment winner artifact save failed: {e}");
                    }
                    if static_table_updated && let Err(e) = self.save_cascade_router() {
                        eprintln!(
                            "[learn] cascade router save failed after experiment conclusion: {e}"
                        );
                    }
                }
                Err(e) => eprintln!("[learn] experiment store transaction failed: {e}"),
            }
        }

        // Local reward observations
        let success = input.episode.success;
        if !skip_only && let Some(model) = extra_string(&input.episode, "model") {
            self.observe_local_reward("router", &model, success);
        }
        if !skip_only && let Some(ref skill_id) = local_reward_skill_id {
            self.observe_local_reward("skill", skill_id, success);
        }
        if !skip_only && let Some(ref rule_id) = local_reward_rule_id {
            self.observe_local_reward("playbook_rule", rule_id, success);
        }
        if !skip_only {
            self.save_local_rewards();
        }

        // Adaptive gate threshold flush cadence
        if !skip_only && self.update_frequency.gate_thresholds_due(episode_count) {
            update.gate_thresholds_flush_due = true;
            if let Some(ref snapshot_json) = input.gate_thresholds_snapshot {
                match self.flush_gate_thresholds_json(snapshot_json) {
                    Ok(()) => {
                        update.gate_thresholds_flushed = true;
                        tracing::debug!(
                            episode_count,
                            "incremental adaptive threshold flush to {}",
                            self.paths.gate_thresholds_json.display()
                        );
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "incremental adaptive threshold flush failed");
                    }
                }
            }
        }

        Ok(update)
    }

    // ── Affect ────────────────────────────────────────────────────────

    fn apply_affect_signature(&self, episode: &mut Episode) {
        let task_key = if episode.task_id.trim().is_empty() {
            episode.agent_id.clone()
        } else {
            episode.task_id.clone()
        };
        let mut engine = self.affect_engine.lock();
        // An attempt without a learning label (S01 §4.1) moves no affect.
        let skip_only = episode.learning_success().is_none()
            || gate_counts_from_episode(episode).is_some_and(GateCountsInner::has_only_skipped);
        if !skip_only {
            for (rung, verdict) in episode.gate_verdicts.iter().enumerate() {
                let _ = engine.appraise(AffectEvent::GateResult {
                    plan_id: String::new(),
                    task_id: task_key.clone(),
                    passed: verdict.passed,
                    rung: rung as u32,
                });
            }
            if episode.success {
                let _ = engine.appraise(AffectEvent::TaskOutcome {
                    task_id: task_key.clone(),
                    succeeded: true,
                });
            } else {
                let _ = engine.appraise(AffectEvent::TaskOutcome {
                    task_id: task_key.clone(),
                    succeeded: false,
                });
            }
        }
        let state = engine.query();
        episode.extra.insert("pad".to_string(), serde_json::json!({ "pleasure": state.pad.pleasure, "arousal": state.pad.arousal, "dominance": state.pad.dominance }));
        episode.extra.insert(
            "affect_confidence".to_string(),
            serde_json::json!(state.confidence),
        );
    }

    // ── Cascade router update ─────────────────────────────────────────

    fn update_cascade_router(&self, episode: &Episode) -> bool {
        let role_str = extra_string(episode, "role");
        let model_slug = extra_string(episode, "model");
        let Some(slug) = model_slug else { return false };
        let role = role_str
            .as_deref()
            .and_then(parse_agent_role)
            .unwrap_or(AgentRole::Implementer);
        let category_str =
            extra_string(episode, "task_category").unwrap_or_else(|| "implementation".to_string());
        let cat_json = format!("\"{category_str}\"");
        let task_category =
            serde_json::from_str::<TaskCategory>(&cat_json).unwrap_or(TaskCategory::Implementation);
        let complexity_str =
            extra_string(episode, "complexity_band").unwrap_or_else(|| "standard".to_string());
        let cplx_json = format!("\"{complexity_str}\"");
        let complexity = serde_json::from_str::<TaskComplexityBand>(&cplx_json)
            .unwrap_or(TaskComplexityBand::Standard);
        let crate_familiarity = extra_f64(episode, "crate_familiarity").unwrap_or(0.5);

        let ctx = RoutingContext {
            task_category,
            complexity,
            iteration: 0,
            role,
            crate_familiarity,
            has_prior_failure: !episode.success,
            conductor_load: 0.0,
            active_agents: 0,
            ready_queue_depth: 0,
            max_queue_wait_hours: 0.0,
            daimon_policy: DaimonPolicy::new(
                extra_f64(episode, "affect_confidence").unwrap_or(0.5),
                roko_core::BehavioralState::Engaged,
            ),
            thinking_level: None,
            temperament: None,
            previous_model: None,
            plan_context_tokens: None,
            tier_thresholds: None,
            cfactor: None,
        };
        if episode
            .extra
            .get("cascade_router_observed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            return false;
        }
        let provider = extra_string(episode, "provider")
            .or_else(|| extra_string(episode, "backend"))
            .unwrap_or_else(|| "unknown-provider".to_string());
        // A failure earns 0 however cheap and fast it was, as on every other
        // path (bug-3ea1f5), and the journal records the reward applied.
        let reward = outcome_reward(
            episode.success,
            self.compute_routing_reward_with_latency(
                episode.success,
                episode.usage.cost_usd,
                episode.usage.wall_ms,
                &slug,
                &provider,
            ),
        );
        let context_features = ctx.to_features();
        let model_idx = self.cascade_router.model_index_for_slug(&slug).unwrap_or(0);
        self.cascade_router
            .record_observation(&ctx, &slug, reward, episode.success);
        self.wal_append(WalEntry::CascadeObservation {
            model_slug: slug,
            context_features,
            model_idx,
            reward,
            success: episode.success,
            ts_ms: Utc::now().timestamp_millis(),
        });
        true
    }

    fn on_experiment_concluded(&self, experiment: &PromptExperiment) -> bool {
        let (Some(winner_id), Some(role_raw)) =
            (experiment.winner_id.as_deref(), experiment.role.as_deref())
        else {
            return false;
        };
        let Some(role) = parse_agent_role(role_raw) else {
            return false;
        };
        let Some(winner_slug) = experiment
            .variants
            .iter()
            .find(|variant| variant.id == winner_id)
            .and_then(|variant| variant.slug.as_deref())
        else {
            return false;
        };
        if !self.cascade_router.update_static_table(role, winner_slug) {
            return false;
        }
        eprintln!(
            "[learn] experiment concluded -- updated static routing table: experiment={} winner={} role={}",
            experiment.experiment_id, winner_slug, role_raw
        );
        true
    }

    // ── Affect queries ────────────────────────────────────────────────

    /// Return the current arousal value tracked for a task key.
    pub fn task_arousal(&self, task_id: impl AsRef<str>) -> f64 {
        let _ = task_id.as_ref();
        self.affect_engine.lock().query().pad.arousal
    }

    /// Return the current task confidence tracked for a task key.
    pub fn task_confidence(&self, task_id: impl AsRef<str>) -> f64 {
        let _ = task_id.as_ref();
        self.affect_engine.lock().query().confidence
    }

    /// Return the current task arousal with queue-wait motivation applied.
    pub fn task_arousal_with_queue_wait(&self, task_id: impl AsRef<str>, queued_hours: f64) -> f64 {
        let base = self.task_arousal(task_id);
        let bump = queue_wait_arousal(queued_hours);
        (base + bump).clamp(-1.0, 1.0)
    }

    async fn append_cfactor_snapshot(&self) -> Result<(), LearningRuntimeError> {
        let snapshot = cfactor_snapshot::compute_cfactor_snapshot(&self.paths.root).await?;
        append_cfactor_snapshot(&self.paths.cfactor_jsonl, &snapshot).await?;
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "runtime_feedback_tests.rs"]
mod tests;
