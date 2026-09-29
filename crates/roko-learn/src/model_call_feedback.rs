//! Durable model-call feedback recording helpers.
//!
//! This module centralizes the common persistence sequence used by direct
//! model-call surfaces: write efficiency feedback, update provider health, and
//! save cascade-router observations under `.roko/learn`. [`ModelCallJournal`]
//! journals those observations in the learning WAL before they are applied.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use parking_lot::Mutex;
use roko_core::Result;
use roko_core::foundation::{FeedbackEvent, FeedbackSink};

use crate::cascade_router::CascadeRouter;
use crate::error::LearnError;
use crate::feedback_service::FeedbackService;
use crate::model_router::CONTEXT_DIM;
use crate::provider_health::{ErrorClass, ProviderHealthRegistry};
use crate::wal::{self, WalEntry, WalSegment};

/// Metrics and identity for one model call.
#[derive(Debug, Clone)]
pub struct ModelCallFeedback {
    /// Optional workflow run id.
    pub run_id: Option<String>,
    /// Optional request id for correlating this call with caller logs.
    pub request_id: Option<String>,
    /// Prompt section ids included in the request.
    pub prompt_section_ids: Vec<String>,
    /// Knowledge ids included in the request.
    pub knowledge_ids: Vec<String>,
    /// Model slug used for learning and cascade-router observations.
    pub model: String,
    /// Provider id used for feedback and persisted provider health.
    pub provider: String,
    /// Caller role or surface, for example `dispatch_v2`.
    pub role: String,
    /// Input tokens reported by the provider.
    pub input_tokens: u64,
    /// Output tokens reported by the provider.
    pub output_tokens: u64,
    /// Cost reported by the provider.
    pub cost_usd: f64,
    /// End-to-end model-call latency in milliseconds.
    pub latency_ms: u64,
    /// Learning outcome for this call or workflow stage.
    pub success: bool,
    /// Provider transport outcome. When omitted, uses [`Self::success`].
    pub provider_success: Option<bool>,
    /// Classified error kind (e.g. `"rate_limit"`, `"timeout"`).
    /// `None` on success.
    pub error_class: Option<String>,
}

impl ModelCallFeedback {
    /// Total tokens reported by the provider.
    #[must_use]
    pub const fn token_usage(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    fn provider_health_success(&self) -> bool {
        self.provider_success.unwrap_or(self.success)
    }
}

/// Recorder for direct model-call learning persistence.
pub struct ModelCallFeedbackRecorder {
    learn_dir: PathBuf,
    cascade_journal: Arc<ModelCallJournal>,
    cascade_router: Option<Arc<CascadeRouter>>,
    save_cascade_router: bool,
}

impl ModelCallFeedbackRecorder {
    /// Create a recorder rooted at a workspace/project directory.
    #[must_use]
    pub fn from_workdir(workdir: &Path, model_slugs: Vec<String>) -> Self {
        Self::from_learn_dir(workdir.join(".roko").join("learn"), model_slugs)
    }

    /// Create a recorder rooted directly at a `.roko/learn` directory.
    #[must_use]
    pub fn from_learn_dir(learn_dir: PathBuf, model_slugs: Vec<String>) -> Self {
        let cascade_journal = ModelCallJournal::for_learn_dir(&learn_dir);
        let cascade_router = (!model_slugs.is_empty()).then(|| {
            Arc::new(CascadeRouter::load_or_new(
                cascade_journal.snapshot_path(),
                model_slugs,
            ))
        });
        Self {
            learn_dir,
            cascade_journal: Arc::new(cascade_journal),
            cascade_router,
            save_cascade_router: true,
        }
    }

    /// Create a recorder using an existing in-memory cascade router.
    ///
    /// This is useful for long-lived services that keep one router in memory
    /// and need direct model-call observations to update that same instance.
    #[must_use]
    pub fn with_cascade_router(learn_dir: PathBuf, cascade_router: Arc<CascadeRouter>) -> Self {
        Self {
            cascade_journal: Arc::new(ModelCallJournal::for_learn_dir(&learn_dir)),
            learn_dir,
            cascade_router: Some(cascade_router),
            save_cascade_router: true,
        }
    }

    /// Create a recorder without cascade-router observation.
    #[must_use]
    pub fn without_cascade_router(learn_dir: PathBuf) -> Self {
        Self {
            cascade_journal: Arc::new(ModelCallJournal::for_learn_dir(&learn_dir)),
            learn_dir,
            cascade_router: None,
            save_cascade_router: false,
        }
    }

    /// Record model-call feedback, provider health, and cascade observation.
    ///
    /// The cascade observation is journaled in the learning WAL before it is
    /// applied, and dropped from the journal once the router snapshot is
    /// saved.
    ///
    /// # Errors
    ///
    /// Returns an error if any durable write fails.
    pub async fn record(&self, feedback: ModelCallFeedback) -> Result<()> {
        self.record_provider_health(&feedback)?;

        let mut feedback_service = FeedbackService::new(self.learn_dir.clone());
        if let Some(router) = &self.cascade_router {
            feedback_service = feedback_service
                .with_cascade_router(Arc::clone(router))
                .with_cascade_journal(Arc::clone(&self.cascade_journal));
        }

        let token_usage = feedback.token_usage();
        feedback_service
            .record(FeedbackEvent::ModelCall {
                run_id: feedback.run_id,
                request_id: feedback.request_id,
                prompt_section_ids: feedback.prompt_section_ids,
                knowledge_ids: feedback.knowledge_ids,
                model: Some(feedback.model),
                provider: Some(feedback.provider),
                token_usage: Some(token_usage),
                cost: Some(feedback.cost_usd),
                role: feedback.role,
                input_tokens: feedback.input_tokens,
                output_tokens: feedback.output_tokens,
                cost_usd: feedback.cost_usd,
                latency_ms: feedback.latency_ms,
                success: feedback.success,
                error_class: feedback.error_class.clone(),
            })
            .await?;
        feedback_service.flush_async().await?;

        if self.save_cascade_router
            && let Some(router) = &self.cascade_router
        {
            self.cascade_journal.save(router).map_err(|e| {
                roko_core::error::RokoError::Io(std::io::Error::other(e.to_string()))
            })?;
        }

        Ok(())
    }

    fn record_provider_health(&self, feedback: &ModelCallFeedback) -> Result<()> {
        record_provider_health_at(
            &self.learn_dir,
            &feedback.provider,
            feedback.provider_health_success(),
        )
    }
}

/// Persist one provider-health outcome under a workspace `.roko/learn` tree.
///
/// # Errors
///
/// Returns an error when the health registry directory or JSON file cannot be
/// written.
pub fn record_provider_health_for_workdir(
    workdir: &Path,
    provider: &str,
    success: bool,
) -> Result<()> {
    record_provider_health_at(&workdir.join(".roko").join("learn"), provider, success)
}

/// Persist one provider-health outcome under a `.roko/learn` directory.
///
/// # Errors
///
/// Returns an error when the health registry directory or JSON file cannot be
/// written.
pub fn record_provider_health_at(learn_dir: &Path, provider: &str, success: bool) -> Result<()> {
    let provider = provider.trim();
    if provider.is_empty() {
        return Ok(());
    }

    std::fs::create_dir_all(learn_dir)?;
    let path = learn_dir.join("provider-health.json");
    let registry = ProviderHealthRegistry::load_or_new(&path);
    if success {
        registry.record_success(provider);
    } else {
        registry.record_failure(provider, ErrorClass::Unknown);
    }
    registry.save(&path)?;
    Ok(())
}

/// Record a model-call reward observation on an existing cascade router.
///
/// Nothing is journaled: a router whose owner saves it should observe through
/// [`ModelCallJournal::observe_model_call`] instead.
pub fn observe_model_call_on_router(
    router: &CascadeRouter,
    model: &str,
    role: &str,
    success: bool,
    latency_ms: u64,
) {
    let Some(model_idx) = router.model_index_for_slug(model) else {
        tracing::debug!("model {model} not in cascade router slug list, skipping observe");
        return;
    };

    router.observe_outcome(
        model_call_context_vec(role, latency_ms),
        model_idx,
        model_call_reward(success),
        success,
    );
}

/// Write-ahead journal for the cascade observations of model-call surfaces
/// (feedback Path B: chat, direct dispatch, ACP, the vision loop, serve).
///
/// Those surfaces load `cascade-router.json`, observe calls and save the
/// snapshot again. An observation applied but not yet saved used to be lost
/// on a crash or a failed save (find-0dc1d5). The journal appends each
/// observation to its own WAL segment beside the snapshot before applying
/// it, and truncates the segment once a saved snapshot holds its
/// observations. The segment stays locked while the journal lives, so
/// `LearningRuntime` replays it only once its writer is gone: each
/// observation is saved by its writer or replayed after a crash, never both
/// (bug-84de98).
#[derive(Debug)]
pub struct ModelCallJournal {
    snapshot_path: PathBuf,
    segments_dir: PathBuf,
    /// The journal's WAL segment, created on its first observation. Holding
    /// the lock also keeps `save` from truncating an observation that the
    /// snapshot it writes does not hold.
    segment: Mutex<Option<WalSegment>>,
}

impl ModelCallJournal {
    /// Journal for the router saved at `snapshot_path`, in a WAL segment
    /// beside it.
    #[must_use]
    pub fn for_snapshot(snapshot_path: &Path) -> Self {
        Self {
            snapshot_path: snapshot_path.to_path_buf(),
            segments_dir: snapshot_path.with_file_name(wal::SEGMENTS_DIR),
            segment: Mutex::new(None),
        }
    }

    /// Journal for the router saved as `cascade-router.json` under a
    /// `.roko/learn` directory.
    #[must_use]
    pub fn for_learn_dir(learn_dir: &Path) -> Self {
        Self::for_snapshot(&learn_dir.join("cascade-router.json"))
    }

    /// The router snapshot that [`Self::save`] writes.
    #[must_use]
    pub fn snapshot_path(&self) -> &Path {
        &self.snapshot_path
    }

    /// Journal one model call, then apply it to `router`: reward 1 on
    /// success, a failed trial with reward 0 otherwise.
    pub fn observe_model_call(
        &self,
        router: &CascadeRouter,
        model: &str,
        role: &str,
        success: bool,
        latency_ms: u64,
    ) {
        self.observe(
            router,
            model,
            model_call_context_vec(role, latency_ms),
            model_call_reward(success),
            success,
        );
    }

    /// Journal an observation, then apply it to `router`.
    ///
    /// When the WAL cannot be written the observation is still applied, and
    /// is then only as durable as the next snapshot save.
    pub fn observe(
        &self,
        router: &CascadeRouter,
        model_slug: &str,
        context_features: Vec<f64>,
        reward: f64,
        success: bool,
    ) {
        let Some(model_idx) = router.model_index_for_slug(model_slug) else {
            tracing::debug!("model {model_slug} not in cascade router slug list, skipping observe");
            return;
        };

        let entry = WalEntry::ModelCallObservation {
            id: uuid::Uuid::new_v4().to_string(),
            model_slug: model_slug.to_string(),
            context_features: context_features.clone(),
            model_idx,
            reward,
            success,
            ts_ms: Utc::now().timestamp_millis(),
        };

        // Journal and apply under the lock, so `save` never truncates an
        // observation that the snapshot it writes does not hold. This is the
        // update `CascadeRouter::replay_observation` repeats on replay.
        let mut segment = self.segment.lock();
        if let Err(error) = wal::append_to_segment(&mut segment, &self.segments_dir, &entry) {
            tracing::warn!(
                dir = %self.segments_dir.display(),
                %error,
                "[wal] model-call observation not journaled -- learning not durable this entry"
            );
        }
        router.observe_outcome(context_features, model_idx, reward, success);
    }

    /// Save `router` to the snapshot, then truncate the journal's segment:
    /// the snapshot holds its observations now.
    ///
    /// # Errors
    ///
    /// Returns an error when the snapshot cannot be written. The segment then
    /// keeps the observations, and `LearningRuntime` replays them if this
    /// writer is gone before a later save holds them.
    pub fn save(&self, router: &CascadeRouter) -> std::result::Result<(), LearnError> {
        let mut segment = self.segment.lock();
        router.save(&self.snapshot_path)?;
        if let Some(segment) = segment.as_mut()
            && let Err(error) = segment.truncate()
        {
            tracing::warn!(
                path = %segment.path().display(),
                %error,
                "[wal] segment not truncated -- a replay may count these observations twice"
            );
        }
        Ok(())
    }
}

fn model_call_reward(success: bool) -> f64 {
    if success { 1.0 } else { 0.0 }
}

fn model_call_context_vec(role: &str, latency_ms: u64) -> Vec<f64> {
    let role_feature = simple_role_hash(role);
    let latency_feature = (latency_ms as f64 / 60_000.0).min(1.0);
    let mut context_vec = vec![0.0; CONTEXT_DIM];

    context_vec[0] = role_feature;
    context_vec[1] = latency_feature;
    context_vec[16] = 1.0;

    context_vec
}

fn simple_role_hash(role: &str) -> f64 {
    let hash: u32 = role.bytes().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u32::from(b))
    });
    f64::from(hash % 1000) / 1000.0
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use roko_core::foundation::{FeedbackEvent, FeedbackSink};
    use tempfile::tempdir;

    use super::{ModelCallFeedback, ModelCallFeedbackRecorder, ModelCallJournal};
    use crate::cascade_router::CascadeRouter;
    use crate::feedback_service::FeedbackService;
    use crate::runtime_feedback::LearningRuntime;
    use crate::wal::replay_wal;

    fn model_call(model: &str, success: bool) -> FeedbackEvent {
        FeedbackEvent::ModelCall {
            run_id: None,
            request_id: None,
            prompt_section_ids: Vec::new(),
            knowledge_ids: Vec::new(),
            model: Some(model.to_string()),
            provider: None,
            token_usage: None,
            cost: None,
            role: "implementer".to_string(),
            input_tokens: 100,
            output_tokens: 20,
            cost_usd: 0.01,
            latency_ms: 1_500,
            success,
            error_class: None,
        }
    }

    async fn reopen(learn_dir: &Path, models: Vec<String>) -> LearningRuntime {
        LearningRuntime::open_under_with_models(learn_dir, models)
            .await
            .expect("open learning runtime")
    }

    /// Entries journaled in the WAL segments under `learn_dir`, whether their
    /// writers live or not.
    fn journaled_entries(learn_dir: &Path) -> usize {
        let Ok(listing) = std::fs::read_dir(learn_dir.join(crate::wal::SEGMENTS_DIR)) else {
            return 0;
        };
        listing
            .map(|segment| {
                let path = segment.expect("segment").path();
                replay_wal(&path).expect("read segment").len()
            })
            .sum()
    }

    #[tokio::test]
    async fn model_call_observation_replayed_from_wal() {
        // find-0dc1d5: Path B observations that no saved snapshot contains
        // (the process died before saving) are replayed from the WAL.
        let tmp = tempdir().expect("tempdir");
        let learn_dir = tmp.path().join("learn");
        std::fs::create_dir_all(&learn_dir).expect("learn dir");
        let models = vec!["model-a".to_string(), "model-b".to_string()];
        {
            let router = Arc::new(CascadeRouter::new(models.clone()));
            let journal = Arc::new(ModelCallJournal::for_learn_dir(&learn_dir));
            let service = FeedbackService::new(learn_dir.clone())
                .with_cascade_router(Arc::clone(&router))
                .with_cascade_journal(journal);
            service
                .record(model_call("model-a", false))
                .await
                .expect("record failed call");
            service
                .record(model_call("model-b", true))
                .await
                .expect("record successful call");
            assert_eq!(router.total_observations(), 2, "applied in memory");
        }

        let runtime = reopen(&learn_dir, models).await;
        let router = runtime.cascade_router();
        assert_eq!(router.total_observations(), 2);
        let confidence = router.confidence_snapshot();
        assert_eq!(confidence["model-a"], (1, 0));
        assert_eq!(confidence["model-b"], (1, 1));
        assert_eq!(
            journaled_entries(&learn_dir),
            0,
            "the replayed segment is removed once the snapshot holds it"
        );
    }

    #[tokio::test]
    async fn model_call_observation_replayed_from_wal_only_until_folded() {
        // find-0dc1d5: once a saved snapshot contains an observation, the
        // save drops it from the journal, so a replay cannot count it twice.
        let tmp = tempdir().expect("tempdir");
        let learn_dir = tmp.path().join("learn");
        let models = vec!["model-a".to_string()];
        let router = CascadeRouter::new(models.clone());
        let journal = ModelCallJournal::for_learn_dir(&learn_dir);

        journal.observe_model_call(&router, "model-a", "implementer", true, 1_000);
        journal.save(&router).expect("save snapshot");
        journal.observe_model_call(&router, "model-a", "implementer", false, 1_000);
        // The process dies before the failure reaches a snapshot.
        drop(journal);

        let runtime = reopen(&learn_dir, models).await;
        assert_eq!(runtime.cascade_router().total_observations(), 2);
        assert_eq!(
            runtime.cascade_router().confidence_snapshot()["model-a"],
            (2, 1)
        );
    }

    #[tokio::test]
    async fn a_running_writers_unsaved_observations_are_counted_once() {
        // bug-84de98: an opener leaves a live writer's journaled observations
        // to that writer, which saves them once.
        let tmp = tempdir().expect("tempdir");
        let learn_dir = tmp.path().join("learn");
        let models = vec!["model-a".to_string()];
        let router = CascadeRouter::new(models.clone());
        let journal = ModelCallJournal::for_learn_dir(&learn_dir);
        journal.observe_model_call(&router, "model-a", "implementer", true, 1_000);
        journal.observe_model_call(&router, "model-a", "implementer", false, 1_000);

        // Another process opens its learning runtime while the writer runs.
        let opener = reopen(&learn_dir, models.clone()).await;
        assert_eq!(
            opener.cascade_router().total_observations(),
            0,
            "a live writer's entries are its own to save"
        );
        drop(opener);

        journal.save(&router).expect("the writer saves");
        drop(journal);
        let after = reopen(&learn_dir, models).await;
        assert_eq!(after.cascade_router().total_observations(), 2);
        let confidence = after.cascade_router().confidence_snapshot();
        assert_eq!(confidence["model-a"], (2, 1));
    }

    #[test]
    fn the_wal_is_truncated_after_every_save() {
        // bug-7a2630: a long-lived writer such as serve truncates its segment
        // after every save, so the WAL stays bounded while it runs.
        let tmp = tempdir().expect("tempdir");
        let learn_dir = tmp.path().join("learn");
        let router = CascadeRouter::new(vec!["model-a".to_string()]);
        let journal = ModelCallJournal::for_learn_dir(&learn_dir);
        for round in 1..=3 {
            journal.observe_model_call(&router, "model-a", "implementer", true, 1_000);
            journal.observe_model_call(&router, "model-a", "implementer", false, 1_000);
            assert_eq!(
                journaled_entries(&learn_dir),
                2,
                "round {round}: journaled until saved"
            );
            journal.save(&router).expect("save");
            assert_eq!(
                journaled_entries(&learn_dir),
                0,
                "round {round}: the save truncates the segment"
            );
        }
        let saved = CascadeRouter::load_or_new(journal.snapshot_path(), vec!["model-a".to_string()]);
        assert_eq!(saved.confidence_snapshot()["model-a"], (6, 3));
    }

    #[tokio::test]
    async fn wal_replay_keeps_entries_for_untracked_models() {
        // bug-7a2630: an opener that tracks fewer models than the WAL names
        // still saves every entry, so a wider router later sees them all.
        let tmp = tempdir().expect("tempdir");
        let learn_dir = tmp.path().join("learn");
        let models = vec!["model-a".to_string(), "model-b".to_string()];
        {
            let router = CascadeRouter::new(models.clone());
            let journal = ModelCallJournal::for_learn_dir(&learn_dir);
            journal.observe_model_call(&router, "model-a", "implementer", true, 1_000);
            journal.observe_model_call(&router, "model-b", "implementer", false, 1_000);
            // The writer dies before it saves.
        }

        let narrow = reopen(&learn_dir, vec!["model-a".to_string()]).await;
        let narrow_confidence = narrow.cascade_router().confidence_snapshot();
        assert_eq!(narrow_confidence["model-a"], (1, 1));
        drop(narrow);

        let wide = reopen(&learn_dir, models).await;
        let confidence = wide.cascade_router().confidence_snapshot();
        assert_eq!(confidence["model-a"], (1, 1));
        assert_eq!(
            confidence["model-b"],
            (1, 0),
            "the narrower opener kept model-b's entry"
        );
        assert_eq!(wide.cascade_router().total_observations(), 2);
    }

    #[tokio::test]
    async fn recorder_writes_feedback_health_and_cascade_router() {
        let tmp = tempdir().expect("tempdir");
        let recorder =
            ModelCallFeedbackRecorder::from_workdir(tmp.path(), vec!["model-slug".to_string()]);

        recorder
            .record(ModelCallFeedback {
                run_id: None,
                request_id: Some("request-1".to_string()),
                prompt_section_ids: Vec::new(),
                knowledge_ids: Vec::new(),
                model: "model-slug".to_string(),
                provider: "provider-id".to_string(),
                role: "test_role".to_string(),
                input_tokens: 12,
                output_tokens: 34,
                cost_usd: 0.056,
                latency_ms: 789,
                success: true,
                provider_success: None,
                error_class: None,
            })
            .await
            .expect("record feedback");

        let learn_dir = tmp.path().join(".roko/learn");
        let efficiency =
            std::fs::read_to_string(learn_dir.join("efficiency.jsonl")).expect("efficiency");
        assert!(efficiency.contains("\"kind\":\"model_call\""));
        assert!(efficiency.contains("\"model\":\"model-slug\""));
        assert!(efficiency.contains("\"provider\":\"provider-id\""));

        let provider_health =
            std::fs::read_to_string(learn_dir.join("provider-health.json")).expect("health");
        assert!(provider_health.contains("provider_id"));

        let cascade =
            std::fs::read_to_string(learn_dir.join("cascade-router.json")).expect("cascade");
        assert!(cascade.contains("model-slug"));
    }
}
