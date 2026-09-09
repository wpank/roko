//! JSONL persistence and project-level snapshot readers.
//!
//! Provides the async append/read functions for the durable runtime-feedback
//! JSONL logs and the project-wide snapshot reader used by HTTP and CLI
//! surfaces.

use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::efficiency::AgentEfficiencyEvent;
use crate::episode_logger::{Episode, EpisodeLogger};
use crate::provider_model_outcome::{ProviderModelOutcomeRecord, read_provider_model_outcomes};
use roko_core::metric::TaskMetric;

use super::records::{
    EfficiencySummaryRecord, GateOutcomeRecord, KnowledgeSeedRecord, LearningPaths,
    LearningRuntimeError, RetryOutcomeRecord, RuntimeFeedbackQuery, RuntimeFeedbackSnapshot,
};

// ── Generic JSONL IO ──────────────────────────────────────────────────

pub(crate) async fn append_jsonl_record<T: Serialize + Sync + ?Sized>(
    path: &Path,
    value: &T,
) -> Result<(), LearningRuntimeError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    let path = path.to_path_buf();
    let max_mb = roko_core::config::ResourcesConfig::default().log_rotation_max_mb;
    tokio::task::spawn_blocking(move || {
        roko_fs::log_rotation::append_jsonl_line_sync(&path, line.as_bytes(), max_mb)
    })
    .await
    .map_err(|error| {
        LearningRuntimeError::Io(std::io::Error::other(format!(
            "learning JSONL append task failed: {error}"
        )))
    })??;
    Ok(())
}

pub(crate) async fn read_jsonl_lossy<T>(path: &Path) -> Result<Vec<T>, LearningRuntimeError>
where
    T: DeserializeOwned,
{
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(LearningRuntimeError::Io(err)),
    };
    let mut lines = BufReader::new(file).lines();
    let mut out = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<T>(trimmed) {
            out.push(record);
        }
    }
    Ok(out)
}

// ── TaskMetric IO ─────────────────────────────────────────────────────

/// Load `TaskMetric` records from a JSONL path, skipping malformed lines.
pub(crate) async fn load_task_metrics(path: &Path) -> io::Result<Vec<TaskMetric>> {
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let mut lines = BufReader::new(file).lines();
    let mut out = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(metric) = serde_json::from_str::<TaskMetric>(trimmed) {
            out.push(metric);
        }
    }
    Ok(out)
}

pub(crate) async fn count_episode_records(path: &Path) -> io::Result<u64> {
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(err) => return Err(err),
    };
    let mut lines = BufReader::new(file).lines();
    let mut count = 0_u64;
    while let Some(line) = lines.next_line().await? {
        if !line.trim().is_empty() {
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

/// Append one `TaskMetric` line to `path`.
pub(crate) async fn append_task_metric(path: &Path, metric: &TaskMetric) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let line =
        serde_json::to_string(metric).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    append_learning_jsonl(path, line).await?;
    Ok(())
}

/// Append one `CFactor` snapshot to `path`.
pub(crate) async fn append_cfactor_snapshot(
    path: &Path,
    snapshot: &crate::cfactor::CFactor,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let line = serde_json::to_string(snapshot)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    append_learning_jsonl(path, line).await?;
    Ok(())
}

pub(crate) async fn append_learning_jsonl(path: &Path, line: String) -> io::Result<()> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        roko_fs::log_rotation::append_jsonl_line_sync(
            &path,
            line.as_bytes(),
            roko_core::config::ResourcesConfig::default().log_rotation_max_mb,
        )
    })
    .await
    .map_err(|error| io::Error::other(format!("learning JSONL append task failed: {error}")))??;
    Ok(())
}

// ── Typed record readers ──────────────────────────────────────────────

/// Read efficiency events from a JSONL file. Returns empty vec if file missing.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or if any read operation
/// fails unexpectedly.
pub async fn read_efficiency_events(
    path: &Path,
) -> Result<Vec<AgentEfficiencyEvent>, LearningRuntimeError> {
    read_jsonl_lossy(path).await
}

/// Read normalized efficiency summaries from a JSONL file.
///
/// Missing files produce an empty vector and malformed lines are skipped.
///
/// # Errors
///
/// Returns an error only for file open/read failures.
pub async fn read_efficiency_summaries(
    path: &Path,
) -> Result<Vec<EfficiencySummaryRecord>, LearningRuntimeError> {
    read_jsonl_lossy(path).await
}

/// Read gate outcomes from a JSONL file.
///
/// Missing files produce an empty vector and malformed lines are skipped.
///
/// # Errors
///
/// Returns an error only for file open/read failures.
pub async fn read_gate_outcomes(
    path: &Path,
) -> Result<Vec<GateOutcomeRecord>, LearningRuntimeError> {
    read_jsonl_lossy(path).await
}

/// Read retry outcomes from a JSONL file.
///
/// Missing files produce an empty vector and malformed lines are skipped.
///
/// # Errors
///
/// Returns an error only for file open/read failures.
pub async fn read_retry_outcomes(
    path: &Path,
) -> Result<Vec<RetryOutcomeRecord>, LearningRuntimeError> {
    read_jsonl_lossy(path).await
}

/// Read knowledge seeds from a JSONL file.
///
/// Missing files produce an empty vector and malformed lines are skipped.
///
/// # Errors
///
/// Returns an error only for file open/read failures.
pub async fn read_knowledge_seeds(
    path: &Path,
) -> Result<Vec<KnowledgeSeedRecord>, LearningRuntimeError> {
    read_jsonl_lossy(path).await
}

pub(crate) async fn count_jsonl_records(path: &Path) -> Result<usize, LearningRuntimeError> {
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(err) => return Err(LearningRuntimeError::Io(err)),
    };
    let mut lines = BufReader::new(file).lines();
    let mut count = 0;
    while let Some(line) = lines.next_line().await? {
        if !line.trim().is_empty() {
            count += 1;
        }
    }
    Ok(count)
}

// ── Project-level snapshot loader ─────────────────────────────────────

/// Learning artifacts discovered for a project workdir.
///
/// This is intentionally read-only and tolerant. It lets HTTP and CLI surfaces
/// show runner-produced durable feedback from the canonical episode log while
/// retaining a read-only fallback for pre-V3 workspaces.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectLearningSnapshot {
    /// Episode records from the canonical log or one legacy fallback.
    pub episodes: Vec<Episode>,
    /// Efficiency events from `.roko/learn/efficiency.jsonl`.
    pub efficiency_events: Vec<AgentEfficiencyEvent>,
    /// Provider/model outcomes from `.roko/learn/provider-model-outcomes.jsonl`.
    pub provider_model_outcomes: Vec<ProviderModelOutcomeRecord>,
    /// Efficiency summaries from `.roko/learn/efficiency-summaries.jsonl`.
    pub efficiency_summaries: Vec<EfficiencySummaryRecord>,
    /// Gate outcomes from `.roko/learn/gate-outcomes.jsonl`.
    pub gate_outcomes: Vec<GateOutcomeRecord>,
    /// Retry outcomes from `.roko/learn/retry-outcomes.jsonl`.
    pub retry_outcomes: Vec<RetryOutcomeRecord>,
    /// Knowledge seeds from `.roko/learn/knowledge-seeds.jsonl`.
    pub knowledge_seeds: Vec<KnowledgeSeedRecord>,
    /// Parsed cascade router snapshot from `.roko/learn/cascade-router.json`.
    pub cascade_router: Option<serde_json::Value>,
    /// Number of durable knowledge entries in `.roko/neuro/knowledge.jsonl`.
    pub knowledge_entries: usize,
    /// Episode files that existed and were read.
    pub episode_paths: Vec<PathBuf>,
    /// Efficiency log path.
    pub efficiency_path: PathBuf,
    /// Provider/model outcome log path.
    pub provider_model_outcomes_path: PathBuf,
    /// Efficiency summary log path.
    pub efficiency_summaries_path: PathBuf,
    /// Gate outcome log path.
    pub gate_outcomes_path: PathBuf,
    /// Retry outcome log path.
    pub retry_outcomes_path: PathBuf,
    /// Knowledge seed log path.
    pub knowledge_seeds_path: PathBuf,
    /// Cascade router snapshot path.
    pub cascade_router_path: PathBuf,
    /// Durable knowledge JSONL path.
    pub knowledge_path: PathBuf,
}

/// Return known episode JSONL locations for `workdir`, canonical paths first,
/// followed by legacy migration inputs.
///
/// Order: root episodes (canonical) -> learn dir -> memory dir.
#[must_use]
pub fn project_episode_paths(workdir: impl AsRef<Path>) -> Vec<PathBuf> {
    let roko = workdir.as_ref().join(".roko");
    vec![
        // Canonical: root episodes.jsonl
        roko.join("episodes.jsonl"),
        // Legacy pre-V3 learning-runtime sink.
        roko.join("learn").join("episodes.jsonl"),
        // Legacy pre-V2 memory sink.
        roko.join("memory").join("episodes.jsonl"),
    ]
}

/// Resolve the active project episode log, preferring the canonical root and
/// falling back to historical locations only when it is absent.
#[must_use]
pub fn resolve_project_episode_path(workdir: impl AsRef<Path>) -> PathBuf {
    let paths = project_episode_paths(workdir);
    paths
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .unwrap_or_else(|| paths[0].clone())
}

/// Read all valid project episodes from the canonical log or one legacy
/// fallback.
///
/// # Errors
///
/// Returns an error only for filesystem read failures from an existing file.
pub async fn read_project_episodes_lossy(
    workdir: impl AsRef<Path>,
) -> Result<Vec<Episode>, LearningRuntimeError> {
    Ok(EpisodeLogger::read_all_lossy(&resolve_project_episode_path(workdir)).await?)
}

/// Read project efficiency events from `.roko/learn/efficiency.jsonl`.
///
/// # Errors
///
/// Returns an error if the efficiency log cannot be read.
pub async fn read_project_efficiency_events(
    workdir: impl AsRef<Path>,
) -> Result<Vec<AgentEfficiencyEvent>, LearningRuntimeError> {
    read_efficiency_events(&workdir.as_ref().join(".roko/learn/efficiency.jsonl")).await
}

/// Read the current project learning artifacts for CLI/API presentation.
///
/// # Errors
///
/// Returns an error if an existing artifact cannot be read.
pub async fn read_project_learning_snapshot(
    workdir: impl AsRef<Path>,
) -> Result<ProjectLearningSnapshot, LearningRuntimeError> {
    let workdir = workdir.as_ref();
    let roko = workdir.join(".roko");
    let efficiency_path = roko.join("learn").join("efficiency.jsonl");
    let provider_model_outcomes_path = roko.join("learn").join("provider-model-outcomes.jsonl");
    let efficiency_summaries_path = roko.join("learn").join("efficiency-summaries.jsonl");
    let gate_outcomes_path = roko.join("learn").join("gate-outcomes.jsonl");
    let retry_outcomes_path = roko.join("learn").join("retry-outcomes.jsonl");
    let knowledge_seeds_path = roko.join("learn").join("knowledge-seeds.jsonl");
    let cascade_router_path = roko.join("learn").join("cascade-router.json");
    let knowledge_path = roko.join("neuro").join("knowledge.jsonl");

    let episodes = read_project_episodes_lossy(workdir).await?;
    let resolved_episode_path = resolve_project_episode_path(workdir);
    let episode_paths = resolved_episode_path
        .exists()
        .then_some(resolved_episode_path);
    let efficiency_events = read_efficiency_events(&efficiency_path).await?;
    let provider_model_outcomes = read_provider_model_outcomes(&provider_model_outcomes_path)
        .await
        .map_err(LearningRuntimeError::Io)?;
    let efficiency_summaries = read_efficiency_summaries(&efficiency_summaries_path).await?;
    let gate_outcomes = read_gate_outcomes(&gate_outcomes_path).await?;
    let retry_outcomes = read_retry_outcomes(&retry_outcomes_path).await?;
    let knowledge_seeds = read_knowledge_seeds(&knowledge_seeds_path).await?;
    let cascade_router = match tokio::fs::read_to_string(&cascade_router_path).await {
        Ok(contents) => serde_json::from_str(&contents).ok(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(LearningRuntimeError::Io(err)),
    };
    let knowledge_entries = count_jsonl_records(&knowledge_path).await?;

    Ok(ProjectLearningSnapshot {
        episodes,
        efficiency_events,
        provider_model_outcomes,
        efficiency_summaries,
        gate_outcomes,
        retry_outcomes,
        knowledge_seeds,
        cascade_router,
        knowledge_entries,
        episode_paths: episode_paths.into_iter().collect(),
        efficiency_path,
        provider_model_outcomes_path,
        efficiency_summaries_path,
        gate_outcomes_path,
        retry_outcomes_path,
        knowledge_seeds_path,
        cascade_router_path,
        knowledge_path,
    })
}

// ── Query & filtering ─────────────────────────────────────────────────

use super::episode_helpers::{
    episode_model, episode_provider, episode_source_id, extra_string,
};

/// Query canonical feedback logs under `paths`.
///
/// # Errors
///
/// Returns an error if an existing log cannot be opened or read.
pub async fn read_runtime_feedback_snapshot(
    paths: &LearningPaths,
    query: &RuntimeFeedbackQuery,
) -> Result<RuntimeFeedbackSnapshot, LearningRuntimeError> {
    let mut episodes = EpisodeLogger::read_all_lossy(&paths.episodes_jsonl).await?;
    episodes.retain(|episode| episode_matches_query(episode, query));
    apply_latest_limit(&mut episodes, query.limit);

    let mut provider_model_outcomes =
        read_provider_model_outcomes(&paths.provider_model_outcomes_jsonl)
            .await
            .map_err(LearningRuntimeError::Io)?;
    provider_model_outcomes.retain(|record| provider_model_outcome_matches_query(record, query));
    apply_latest_limit(&mut provider_model_outcomes, query.limit);

    let mut efficiency_summaries =
        read_efficiency_summaries(&paths.efficiency_summaries_jsonl).await?;
    efficiency_summaries.retain(|record| efficiency_summary_matches_query(record, query));
    apply_latest_limit(&mut efficiency_summaries, query.limit);

    let mut gate_outcomes = read_gate_outcomes(&paths.gate_outcomes_jsonl).await?;
    gate_outcomes.retain(|record| gate_outcome_matches_query(record, query));
    apply_latest_limit(&mut gate_outcomes, query.limit);

    let mut retry_outcomes = read_retry_outcomes(&paths.retry_outcomes_jsonl).await?;
    retry_outcomes.retain(|record| retry_outcome_matches_query(record, query));
    apply_latest_limit(&mut retry_outcomes, query.limit);

    let mut knowledge_seeds = read_knowledge_seeds(&paths.knowledge_seeds_jsonl).await?;
    knowledge_seeds.retain(|record| knowledge_seed_matches_query(record, query));
    apply_latest_limit(&mut knowledge_seeds, query.limit);

    Ok(RuntimeFeedbackSnapshot {
        episodes,
        provider_model_outcomes,
        efficiency_summaries,
        gate_outcomes,
        retry_outcomes,
        knowledge_seeds,
    })
}

/// Query canonical project feedback logs using default `.roko/learn` paths.
///
/// This reads episodes from the root log (or one legacy fallback), then reads
/// the canonical derived feedback streams from `.roko/learn`.
///
/// # Errors
///
/// Returns an error if an existing log cannot be opened or read.
pub async fn read_project_runtime_feedback_snapshot(
    workdir: impl AsRef<Path>,
    query: &RuntimeFeedbackQuery,
) -> Result<RuntimeFeedbackSnapshot, LearningRuntimeError> {
    let workdir = workdir.as_ref();
    let paths = LearningPaths::for_project(workdir);
    let mut snapshot = read_runtime_feedback_snapshot(&paths, query).await?;

    let mut project_episodes = read_project_episodes_lossy(workdir).await?;
    project_episodes.retain(|episode| episode_matches_query(episode, query));
    apply_latest_limit(&mut project_episodes, query.limit);
    snapshot.episodes = project_episodes;

    Ok(snapshot)
}

fn apply_latest_limit<T>(items: &mut Vec<T>, limit: Option<usize>) {
    let Some(limit) = limit else {
        return;
    };
    if items.len() > limit {
        let drop_count = items.len() - limit;
        items.drain(0..drop_count);
    }
}

fn query_matches(value: &str, expected: Option<&String>) -> bool {
    expected
        .map(String::as_str)
        .is_none_or(|expected| value.trim() == expected.trim())
}

fn query_matches_option(value: Option<&str>, expected: Option<&String>) -> bool {
    expected
        .map(String::as_str)
        .is_none_or(|expected| value.is_some_and(|value| value.trim() == expected.trim()))
}

fn episode_matches_query(episode: &Episode, query: &RuntimeFeedbackQuery) -> bool {
    query_matches(
        extra_string(episode, "plan_id")
            .unwrap_or_default()
            .as_str(),
        query.plan_id.as_ref(),
    ) && query_matches(&episode.task_id, query.task_id.as_ref())
        && query_matches(episode_source_id(episode), query.episode_id.as_ref())
        && query_matches(episode_provider(episode).as_str(), query.provider.as_ref())
        && query_matches(episode_model(episode).as_str(), query.model.as_ref())
}

fn provider_model_outcome_matches_query(
    record: &ProviderModelOutcomeRecord,
    query: &RuntimeFeedbackQuery,
) -> bool {
    query_matches_option(record.run_id.as_deref(), query.plan_id.as_ref())
        && query_matches(&record.task_id, query.task_id.as_ref())
        && query_matches_option(record.run_id.as_deref(), query.episode_id.as_ref())
        && query_matches(&record.provider, query.provider.as_ref())
        && query_matches(&record.model, query.model.as_ref())
}

fn efficiency_summary_matches_query(
    record: &EfficiencySummaryRecord,
    query: &RuntimeFeedbackQuery,
) -> bool {
    query_matches(&record.plan_id, query.plan_id.as_ref())
        && query_matches(&record.task_id, query.task_id.as_ref())
        && query_matches_option(record.episode_id.as_deref(), query.episode_id.as_ref())
        && query_matches(&record.provider, query.provider.as_ref())
        && query_matches(&record.model, query.model.as_ref())
}

fn gate_outcome_matches_query(record: &GateOutcomeRecord, query: &RuntimeFeedbackQuery) -> bool {
    query_matches(&record.plan_id, query.plan_id.as_ref())
        && query_matches(&record.task_id, query.task_id.as_ref())
        && query_matches_option(record.episode_id.as_deref(), query.episode_id.as_ref())
        && query_matches_option(record.provider.as_deref(), query.provider.as_ref())
        && query_matches_option(record.model.as_deref(), query.model.as_ref())
}

fn retry_outcome_matches_query(record: &RetryOutcomeRecord, query: &RuntimeFeedbackQuery) -> bool {
    query_matches(&record.plan_id, query.plan_id.as_ref())
        && query_matches(&record.task_id, query.task_id.as_ref())
        && query_matches_option(record.episode_id.as_deref(), query.episode_id.as_ref())
        && query_matches_option(record.provider.as_deref(), query.provider.as_ref())
        && query_matches_option(record.model.as_deref(), query.model.as_ref())
}

fn knowledge_seed_matches_query(
    record: &KnowledgeSeedRecord,
    query: &RuntimeFeedbackQuery,
) -> bool {
    let provider = record
        .metadata
        .get("provider")
        .and_then(serde_json::Value::as_str);
    let model = record.source_model.as_deref().or_else(|| {
        record
            .metadata
            .get("model")
            .and_then(serde_json::Value::as_str)
    });
    query_matches(&record.plan_id, query.plan_id.as_ref())
        && query_matches(&record.task_id, query.task_id.as_ref())
        && query
            .episode_id
            .as_deref()
            .is_none_or(|episode_id| record.source_episodes.iter().any(|id| id == episode_id))
        && query_matches_option(provider, query.provider.as_ref())
        && query_matches_option(model, query.model.as_ref())
}
