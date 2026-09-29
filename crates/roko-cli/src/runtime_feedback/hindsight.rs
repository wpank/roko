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
//! Nothing else relabels. On the Graph path every episode is a fresh attempt
//! that edits files, so a later failure of the same task, or of one that
//! declares the same files, does not show that earlier work regressed.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use roko_learn::episode_logger::EpisodeLogger;
use roko_learn::hindsight::{HindsightRelabeler, append_new_adjustments};

use super::{FeedbackEvent, FeedbackSink};

/// Failure-reason prefix of a verify failure attributed to sibling tasks.
const BLAME_PREFIX: &str = "verify: blocked_by_sibling = ";

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
    /// Serializes appends so concurrent failures record a correction once.
    serial: Arc<tokio::sync::Mutex<()>>,
}

impl HindsightSink {
    /// Sink scanning `episodes_path` and recording to `adjustments_path`.
    #[must_use]
    pub fn new(episodes_path: impl Into<PathBuf>, adjustments_path: impl Into<PathBuf>) -> Self {
        Self {
            episodes_path: episodes_path.into(),
            adjustments_path: adjustments_path.into(),
            serial: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
}

#[async_trait]
impl FeedbackSink for HindsightSink {
    fn name(&self) -> &'static str {
        "hindsight"
    }

    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(
            event,
            FeedbackEvent::TaskCompleted {
                plan_id,
                succeeded: false,
                failure_reason: Some(reason),
                ..
            } if !blamed_tasks(plan_id, reason).is_empty()
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
        if appended > 0 {
            tracing::info!(
                plan_id = %plan_id,
                appended,
                "hindsight relabeled earlier successes blamed by a later verify failure"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{AgentOutcome, ModelChoiceSource};
    use crate::runtime_feedback::{EpisodeSink, FeedbackFacade};
    use roko_learn::hindsight::{AdjustmentKind, BLAMED_TASKS_KEY, read_adjustments};
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
            settled: None,
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
}
