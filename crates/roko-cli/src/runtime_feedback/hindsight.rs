//! Hindsight sink — relabels an earlier success when a later gate failure
//! is attributed to that task.
//!
//! When a verify step fails while sibling tasks edit the same working tree,
//! the dispatcher waits for them to settle and re-runs the step. A failure
//! that persists with every located error in a sibling's declared files
//! names it: the failure reason leads with `blocked_by_sibling = <task>`.
//! That is retrospective evidence against the sibling's latest success: its
//! edits break a gate it did not run. [`super::EpisodeSink`] records the
//! blame on the failed episode (`extra.blamed_tasks`); this sink then runs
//! [`HindsightRelabeler`] over the plan's episodes and appends each new
//! correction once to `.roko/learn/episode-adjustments.jsonl`.
//!
//! Episode readers apply the log as they load (the prompt caches, `roko learn
//! episodes` and dream replay, through `roko_learn::hindsight`). The credit
//! the success earned at completion is retracted here, once per new
//! correction: every playbook it used, and the router's category and
//! confidence counts (`CascadeRouter::retract_success`; the `LinUCB` bandit
//! keeps its update). The router retraction is journaled in the run's
//! learning WAL, as the routing outcome was, so it survives a crash before
//! the run saves its router (bug-583e50).
//!
//! Nothing else relabels. On the Graph path every episode is a fresh attempt
//! that edits files, so a later failure of the same task, or of one that
//! declares the same files, does not show that earlier work regressed.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use roko_core::task::TaskCategory;
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use roko_learn::hindsight::{
    AdjustmentKind, EpisodeAdjustment, HindsightRelabeler, append_new_adjustments,
};
use roko_learn::model_call_feedback::ModelCallJournal;
use roko_learn::playbook::PlaybookStore;

use super::{FeedbackEvent, FeedbackSink};

/// Failure-reason prefix of a verify failure attributed to sibling tasks.
const BLAME_PREFIX: &str = "verify: blocked_by_sibling = ";

/// Episode `extra` key listing every playbook dispatch credited the attempt
/// with; `extra.playbook_id` holds only the first.
pub(crate) const PLAYBOOK_IDS_KEY: &str = "playbook_ids";

/// Episode `extra` key holding the task category the routing sink credited
/// the attempt under.
pub(crate) const ROUTING_CATEGORY_KEY: &str = "routing_category";

/// Tasks a verify failure reason blames, as `"{plan_id}/{task_id}"` keys.
///
/// Only a reason that leads with the blame counts; a sibling label naming
/// another plan is already `"{plan_id}/{task_id}"`.
#[must_use]
pub fn blamed_tasks(plan_id: &str, failure_reason: &str) -> Vec<String> {
    let Some((labels, _)) = failure_reason
        .strip_prefix(BLAME_PREFIX)
        .and_then(|rest| rest.split_once(": "))
    else {
        return Vec::new();
    };
    labels
        .split(", ")
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(|label| {
            if label.contains('/') {
                label.to_string()
            } else {
                format!("{plan_id}/{label}")
            }
        })
        .collect()
}

/// Sink that relabels episodes when a verify failure blames a sibling task.
///
/// Register it after [`super::EpisodeSink`] on the same log: it reads back
/// the failed episode that sink just wrote.
#[derive(Debug)]
pub struct HindsightSink {
    episodes_path: PathBuf,
    adjustments_path: PathBuf,
    /// The playbook store beside the adjustments log: `.roko/learn/playbooks/`
    /// for `.roko/learn/episode-adjustments.jsonl`.
    playbook_dir: PathBuf,
    /// The router the run's routing sink credits, when the run has one.
    router: Option<Arc<CascadeRouter>>,
    /// The journal the run saves `router` through, when it has one.
    journal: Option<Arc<ModelCallJournal>>,
    /// Serializes appends so concurrent failures record a correction once.
    serial: Arc<tokio::sync::Mutex<()>>,
}

impl HindsightSink {
    /// Sink scanning `episodes_path` and recording to `adjustments_path`.
    #[must_use]
    pub fn new(episodes_path: impl Into<PathBuf>, adjustments_path: impl Into<PathBuf>) -> Self {
        let adjustments_path = adjustments_path.into();
        let playbook_dir = adjustments_path.with_file_name("playbooks");
        Self {
            episodes_path: episodes_path.into(),
            adjustments_path,
            playbook_dir,
            router: None,
            journal: None,
            serial: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Also retract, from `router`, the routing credit each relabeled
    /// success earned at completion.
    #[must_use]
    pub fn with_router(mut self, router: Option<Arc<CascadeRouter>>) -> Self {
        self.router = router;
        self
    }

    /// Journal each router retraction in `journal`, the journal the run
    /// saves its router through, before applying it (bug-583e50).
    #[must_use]
    pub fn with_journal(mut self, journal: Option<Arc<ModelCallJournal>>) -> Self {
        self.journal = journal;
        self
    }

    /// Retract the credit each newly relabeled success earned at completion:
    /// every playbook it used becomes a failure, and so does its router
    /// credit. Best-effort: a failed playbook update is logged.
    async fn retract_credit(&self, episodes: &[Episode], appended: &[EpisodeAdjustment]) {
        let store = PlaybookStore::new(&self.playbook_dir);
        let regressions = appended
            .iter()
            .filter(|adjustment| adjustment.adjustment_kind == AdjustmentKind::Regression);
        for adjustment in regressions {
            let Some(episode) = episodes
                .iter()
                .find(|episode| episode.id == adjustment.original_episode_id)
            else {
                continue;
            };
            for playbook_id in episode_playbook_ids(episode) {
                if let Err(error) = store.relabel_success_as_failure(&playbook_id).await {
                    tracing::warn!(
                        %playbook_id,
                        %error,
                        "hindsight playbook retraction failed (best-effort)"
                    );
                }
            }
            if let Some(router) = &self.router
                && let Some((model, category)) = routing_credit(episode)
            {
                self.retract_router_credit(router, model, category).await;
            }
        }
    }

    /// Retract a success the routing sink credited `model` with in
    /// `category` from `router`, journaled first when the run has a journal.
    async fn retract_router_credit(
        &self,
        router: &Arc<CascadeRouter>,
        model: String,
        category: TaskCategory,
    ) {
        let router = Arc::clone(router);
        let journal = self.journal.clone();
        // The WAL write syncs to disk, so it runs off the reactor.
        let retracted = tokio::task::spawn_blocking(move || match journal {
            Some(journal) => journal.retract_success(&router, &model, category),
            None => router.retract_success(&model, category),
        })
        .await;
        if let Err(error) = retracted {
            tracing::warn!(%error, "hindsight router retraction failed (best-effort)");
        }
    }
}

/// Every playbook a Graph episode used: `extra.playbook_ids`, or, in an
/// episode written before that key, the first one alone (`extra.playbook_id`).
fn episode_playbook_ids(episode: &Episode) -> Vec<String> {
    if let Some(ids) = episode
        .extra
        .get(PLAYBOOK_IDS_KEY)
        .and_then(serde_json::Value::as_array)
    {
        return ids
            .iter()
            .filter_map(serde_json::Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect();
    }
    episode
        .extra
        .get("playbook_id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .into_iter()
        .collect()
}

/// The routing credit the routing sink gave `episode`'s attempt: its model,
/// and the category it counted under. An attempt that failed over ran a
/// model the router did not pick, so it earned none; an episode with no
/// recorded category counted under the routing sink's fallback category.
fn routing_credit(episode: &Episode) -> Option<(String, TaskCategory)> {
    if episode.extra.contains_key("failover_chain") {
        return None;
    }
    let category = episode
        .extra
        .get(ROUTING_CATEGORY_KEY)
        .and_then(|value| serde_json::from_value::<TaskCategory>(value.clone()).ok())
        .unwrap_or_else(|| {
            super::routing::build_fallback_routing_context(&episode.model, None).task_category
        });
    Some((episode.model.clone(), category))
}

#[async_trait]
impl FeedbackSink for HindsightSink {
    fn name(&self) -> &'static str {
        "hindsight"
    }

    /// A failure of the agent's work (learning label 0) whose verify
    /// failure blames a sibling task.
    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(
            event,
            FeedbackEvent::TaskCompleted {
                plan_id,
                failure_reason: Some(reason),
                ..
            } if event.learning_success() == Some(false)
                && !blamed_tasks(plan_id, reason).is_empty()
        )
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        if !self.interested(event) {
            return Ok(());
        }
        let FeedbackEvent::TaskCompleted { plan_id, .. } = event else {
            return Ok(());
        };
        let _serial = self.serial.lock().await;
        let episodes = EpisodeLogger::read_all_lossy(&self.episodes_path)
            .await
            .map_err(|error| anyhow::anyhow!("hindsight: read episodes: {error}"))?
            .into_iter()
            .filter(|episode| {
                episode
                    .extra
                    .get("plan_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(plan_id.as_str())
            })
            .collect::<Vec<_>>();
        let adjustments = HindsightRelabeler::new()
            .attributed_only()
            .scan(&episodes, &[]);
        if adjustments.is_empty() {
            return Ok(());
        }
        let path = self.adjustments_path.clone();
        let appended =
            tokio::task::spawn_blocking(move || append_new_adjustments(&path, &adjustments))
                .await
                .map_err(|error| anyhow::anyhow!("hindsight append task join: {error}"))??;
        if appended.is_empty() {
            return Ok(());
        }
        tracing::info!(
            plan_id = %plan_id,
            appended = appended.len(),
            "hindsight relabeled earlier successes blamed by a later verify failure"
        );
        self.retract_credit(&episodes, &appended).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{AgentOutcome, ModelChoiceSource};
    use crate::runtime_feedback::{
        EpisodeSink, FeedbackFacade, RoutingObservationSink, settled_as,
    };
    use roko_learn::hindsight::{AdjustmentKind, BLAMED_TASKS_KEY, read_adjustments};
    use roko_learn::telemetry::AttemptOutcome;
    use tempfile::tempdir;

    #[test]
    fn blame_is_read_only_from_a_leading_marker() {
        assert_eq!(
            blamed_tasks(
                "p",
                "verify: blocked_by_sibling = T12: 1/1 verify step(s) failed"
            ),
            ["p/T12"]
        );
        assert_eq!(
            blamed_tasks(
                "p",
                "verify: blocked_by_sibling = T12, other/T3: 2/2 verify step(s) failed"
            ),
            ["p/T12", "other/T3"]
        );
        assert!(blamed_tasks("p", "verify: 1/1 verify step(s) failed").is_empty());
        assert!(blamed_tasks("p", "provider: blocked_by_sibling = T12: no").is_empty());
        assert!(
            blamed_tasks(
                "p",
                "verify: 1/1 failed\nblocked_by_sibling = T12: every located error"
            )
            .is_empty()
        );
    }

    fn completed(task_id: &str, succeeded: bool, failure_reason: Option<&str>) -> FeedbackEvent {
        FeedbackEvent::TaskCompleted {
            plan_id: "plan-h".into(),
            task_id: task_id.into(),
            outcome: AgentOutcome {
                task_id: task_id.into(),
                plan_id: "plan-h".into(),
                model: "claude-sonnet-4-6".into(),
                provider: "claude_cli".into(),
                output: "done".into(),
                tokens_in: 10,
                tokens_out: 5,
                cost_usd: 0.0,
                duration_ms: 10,
                exit_code: Some(i32::from(!succeeded)),
                is_error: !succeeded,
            },
            model_source: ModelChoiceSource::Router,
            succeeded,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
            turns: 1,
            failure_reason: failure_reason.map(str::to_string),
            settled: settled_as(
                if succeeded {
                    AttemptOutcome::Passed
                } else {
                    AttemptOutcome::GateFailed
                },
                true,
            ),
        }
    }

    #[tokio::test]
    async fn blamed_sibling_success_is_relabeled_once() {
        let dir = tempdir().unwrap();
        let episodes = dir.path().join("episodes.jsonl");
        let adjustments = dir.path().join("learn").join("episode-adjustments.jsonl");
        let facade = FeedbackFacade::new()
            .with_sink(Arc::new(EpisodeSink::at(&episodes)))
            .with_sink(Arc::new(HindsightSink::new(&episodes, &adjustments)));
        let blame = "verify: blocked_by_sibling = T12: 1/1 verify step(s) failed for task `T2`:\n\n\
                     verify[0:check] `tsc --noEmit` failed";

        facade
            .on_event(&completed("T12", true, None))
            .await
            .unwrap();
        // An unattributed verify failure relabels nothing.
        facade
            .on_event(&completed(
                "T3",
                false,
                Some("verify: 1/1 verify step(s) failed"),
            ))
            .await
            .unwrap();
        assert!(!adjustments.exists());

        facade
            .on_event(&completed("T2", false, Some(blame)))
            .await
            .unwrap();
        facade
            .on_event(&completed("T2", false, Some(blame)))
            .await
            .unwrap();

        let logged = EpisodeLogger::read_all(&episodes).await.unwrap();
        let sibling_success = logged
            .iter()
            .find(|episode| episode.task_id == "T12")
            .unwrap();
        let failure = logged
            .iter()
            .find(|episode| episode.task_id == "T2")
            .unwrap();
        assert_eq!(
            failure.extra[BLAMED_TASKS_KEY],
            serde_json::json!(["plan-h/T12"])
        );
        assert!(failure.gate_verdicts.iter().any(|verdict| !verdict.passed));

        let recorded = read_adjustments(&adjustments).unwrap();
        assert_eq!(recorded.len(), 1, "{recorded:?}");
        assert_eq!(recorded[0].original_episode_id, sibling_success.id);
        assert_eq!(recorded[0].adjustment_kind, AdjustmentKind::Regression);
    }

    /// gap-5be28d: a success that a later verify failure blames stops
    /// counting as one. The prompt cache reads it as a failure, and the
    /// playbook it used moves that success to its failures.
    #[tokio::test]
    async fn a_relabeled_success_no_longer_counts_as_a_success() {
        let dir = tempdir().unwrap();
        let workdir = dir.path();
        let layout = roko_fs::RokoLayout::for_project(workdir);
        let episodes = layout.root_episodes_path();
        let playbooks = PlaybookStore::new(layout.playbooks_dir());
        let playbook = roko_learn::playbook::Playbook::new("pb-wiring", "Wire the module");
        playbooks.save(&playbook).await.unwrap();
        let adjustments = roko_learn::hindsight::workspace_adjustments_path(workdir);
        let facade = FeedbackFacade::new()
            .with_sink(Arc::new(EpisodeSink::at(&episodes)))
            .with_sink(Arc::new(HindsightSink::new(&episodes, &adjustments)));

        // T1 passes with the playbook, which dispatch credits at completion.
        let mut success = completed("T1", true, None);
        if let FeedbackEvent::TaskCompleted { playbook_ids, .. } = &mut success {
            *playbook_ids = vec!["pb-wiring".to_string()];
        }
        facade.on_event(&success).await.unwrap();
        playbooks.record_outcome("pb-wiring", true).await.unwrap();

        // Then T2's verify failure is blamed on T1.
        let blame = "verify: blocked_by_sibling = T1: 1/1 verify step(s) failed for task `T2`";
        facade
            .on_event(&completed("T2", false, Some(blame)))
            .await
            .unwrap();

        let cache = crate::dispatch::PromptCache::load(workdir);
        let relabeled = cache
            .episodes
            .iter()
            .find(|episode| episode.task_id == "T1")
            .expect("T1's episode");
        assert!(!relabeled.success);
        assert_eq!(relabeled.learning_success(), Some(false));
        assert!(
            relabeled
                .extra
                .contains_key(roko_learn::hindsight::HINDSIGHT_ADJUSTMENT_KEY)
        );
        let playbook = playbooks.load("pb-wiring").await.unwrap().unwrap();
        assert_eq!((playbook.success_count, playbook.failure_count), (0, 1));
    }

    /// The router's (trials, successes) for the fixture model.
    fn sonnet_confidence(router: &CascadeRouter) -> Option<(u64, u64)> {
        router
            .confidence_snapshot()
            .get("claude-sonnet-4-6")
            .copied()
    }

    /// gap-b95d94: a relabeled success gives back every credit it earned at
    /// completion: each playbook it used, and its router success.
    #[tokio::test]
    async fn hindsight_retracts_everywhere_playbooks_and_router() {
        let dir = tempdir().unwrap();
        let workdir = dir.path();
        let layout = roko_fs::RokoLayout::for_project(workdir);
        let episodes = layout.root_episodes_path();
        let playbooks = PlaybookStore::new(layout.playbooks_dir());
        let used = ["pb-first", "pb-second"];
        for id in used {
            let playbook = roko_learn::playbook::Playbook::new(id, "Wire the module");
            playbooks.save(&playbook).await.unwrap();
        }
        let models = vec!["claude-sonnet-4-6".to_string()];
        let router = Arc::new(CascadeRouter::new(models));
        let adjustments = roko_learn::hindsight::workspace_adjustments_path(workdir);
        let hindsight =
            HindsightSink::new(&episodes, &adjustments).with_router(Some(Arc::clone(&router)));
        let facade = FeedbackFacade::new()
            .with_sink(Arc::new(EpisodeSink::at(&episodes)))
            .with_sink(Arc::new(hindsight))
            .with_sink(Arc::new(RoutingObservationSink::new(Arc::clone(&router))));

        // T1 passes with both playbooks: dispatch credits them, and the
        // routing sink credits the router.
        let mut success = completed("T1", true, None);
        if let FeedbackEvent::TaskCompleted { playbook_ids, .. } = &mut success {
            *playbook_ids = used.iter().map(|id| (*id).to_string()).collect();
        }
        facade.on_event(&success).await.unwrap();
        for id in used {
            playbooks.record_outcome(id, true).await.unwrap();
        }
        assert_eq!(sonnet_confidence(&router), Some((1, 1)));

        // Then T2's verify failure is blamed on T1.
        let blame = "verify: blocked_by_sibling = T1: 1/1 verify step(s) failed for task `T2`";
        facade
            .on_event(&completed("T2", false, Some(blame)))
            .await
            .unwrap();

        for id in used {
            let playbook = playbooks.load(id).await.unwrap().unwrap();
            assert_eq!(
                (playbook.success_count, playbook.failure_count),
                (0, 1),
                "{id}"
            );
        }
        // T1's success is retracted; T2's failure is a trial of its own.
        assert_eq!(sonnet_confidence(&router), Some((2, 0)));
    }
}
