//! Episode sink — converts [`FeedbackEvent::TaskCompleted`] into a durable
//! [`Episode`] entry via [`EpisodeLogger`].
//!
//! This sink is the canonical replacement for the legacy
//! `learning_helpers::log_episode` path. It removes hardcoded `backend` /
//! `role` values: those now come from the [`AgentOutcome`] that the
//! dispatcher produced, so episodes correctly attribute work to the
//! provider that actually did it.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use roko_learn::episode_logger::{Episode, EpisodeGateVerdict, EpisodeLogger, Usage};
use roko_learn::hdc_fingerprint::{encode as encode_hdc_fingerprint, fingerprint_episode};
use roko_learn::hindsight::BLAMED_TASKS_KEY;

use super::{FeedbackEvent, FeedbackSink};

/// Sink that appends `task_completed` events to `.roko/episodes.jsonl`.
#[derive(Debug, Clone)]
pub struct EpisodeSink {
    logger: Arc<EpisodeLogger>,
}

impl EpisodeSink {
    /// Construct a sink writing to `path`.
    #[must_use]
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            logger: Arc::new(EpisodeLogger::new(path.into())),
        }
    }

    /// Wrap an existing logger (lets tests share state).
    #[must_use]
    pub fn from_logger(logger: Arc<EpisodeLogger>) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl FeedbackSink for EpisodeSink {
    fn name(&self) -> &'static str {
        "episodes"
    }

    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::TaskCompleted { .. })
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        let FeedbackEvent::TaskCompleted {
            plan_id,
            task_id,
            outcome,
            succeeded,
            prompt_text,
            cache_read_tokens,
            knowledge_ids,
            playbook_ids,
            initial_model,
            turns,
            failure_reason,
            settled,
            ..
        } = event
        else {
            return Ok(());
        };

        let mut episode = Episode::new(outcome.task_id.clone(), task_id.clone());
        // The attempt this episode records (S01): it joins the attempt's
        // verdict, efficiency and cost rows. The verdict's outcome, blame
        // and learning label ride along: `success` keeps its meaning (the
        // provider call succeeded and no verify step failed), so a learner
        // reading episodes reads `learning_label`, and `null` teaches it
        // nothing.
        if let Some(settled) = settled {
            episode.extra.insert(
                "attempt_key".into(),
                serde_json::Value::String(settled.identity.attempt_key.clone()),
            );
            let verdict = [
                ("outcome", serde_json::json!(settled.outcome)),
                ("blame", serde_json::json!(settled.blame)),
                ("learning_label", serde_json::json!(settled.learning_label)),
            ];
            for (key, value) in verdict {
                episode.extra.insert(key.into(), value);
            }
        }
        episode.success = *succeeded;
        episode.turns = *turns;
        if !*succeeded {
            episode.failure_reason = failure_reason.clone();
            if let Some((class, _)) = failure_reason.as_deref().and_then(|r| r.split_once(": ")) {
                episode.extra.insert(
                    "failure_class".into(),
                    serde_json::Value::String(class.to_string()),
                );
            }
            // An authored verify gate failed: record the verdict, and any
            // sibling task the failure is attributed to, for hindsight.
            if let Some(reason) = failure_reason
                .as_deref()
                .filter(|r| r.starts_with("verify: "))
            {
                episode
                    .gate_verdicts
                    .push(EpisodeGateVerdict::new("verify", false));
                let blamed = super::hindsight::blamed_tasks(plan_id, reason);
                if !blamed.is_empty() {
                    episode
                        .extra
                        .insert(BLAMED_TASKS_KEY.into(), serde_json::json!(blamed));
                }
            }
        }
        episode.usage = Usage {
            input_tokens: outcome.tokens_in,
            output_tokens: outcome.tokens_out,
            cost_usd: outcome.cost_usd,
            wall_ms: outcome.duration_ms,
            ..Default::default()
        };
        episode.tokens_used = outcome.total_tokens();
        episode.duration_secs = outcome.duration_ms as f64 / 1000.0;
        episode.backend = outcome.provider.clone();
        episode.model = outcome.model.clone();
        // Plan id is carried in the forward-compat `extra` bag — feedback
        // sinks can promote it to a first-class field once the schema
        // settles. See `.roko/GAPS.md`.
        episode
            .extra
            .insert("plan_id".into(), serde_json::Value::String(plan_id.clone()));

        // P0-07: Record knowledge injection provenance as a first-class field.
        episode.knowledge_ids_injected = knowledge_ids.clone();

        // ── Compounding metric keys (P3-4) ───────────────────────────────
        // Populate the episode `extra` bag with dispatch-time metadata so
        // downstream learning loops (c-factor, autocatalytic metrics,
        // playbook scoring) can observe non-zero values.
        episode.extra.insert(
            "knowledge_used".into(),
            serde_json::Value::Bool(!knowledge_ids.is_empty()),
        );
        // P0-06: Record the count of knowledge entries injected so
        // compounding metrics can measure reuse depth, not just presence.
        if !knowledge_ids.is_empty() {
            episode.extra.insert(
                "knowledge_count".into(),
                serde_json::Value::Number(serde_json::Number::from(knowledge_ids.len())),
            );
        }
        if let Some(first_pb) = playbook_ids.first() {
            episode.extra.insert(
                "playbook_id".into(),
                serde_json::Value::String(first_pb.clone()),
            );
        }
        // P0-04: Record the total playbook hit count so compounding metrics
        // can observe multi-playbook dispatch scenarios beyond the first ID.
        if !playbook_ids.is_empty() {
            episode.extra.insert(
                "playbook_hits".into(),
                serde_json::Value::Number(serde_json::Number::from(playbook_ids.len())),
            );
        }
        episode.extra.insert(
            "cache_hit".into(),
            serde_json::Value::Bool(*cache_read_tokens > 0),
        );
        if !initial_model.is_empty() {
            episode.extra.insert(
                "initial_model".into(),
                serde_json::Value::String(initial_model.clone()),
            );
        }
        // The `successful_model` is the model that actually produced the
        // output — already recorded in `episode.model` but also placed in
        // `extra` for uniform downstream consumption.
        episode.extra.insert(
            "successful_model".into(),
            serde_json::Value::String(outcome.model.clone()),
        );

        attach_episode_hdc_fingerprint(
            &mut episode,
            plan_id,
            task_id,
            outcome,
            *succeeded,
            prompt_text,
        );

        self.logger
            .append(&episode)
            .await
            .map_err(|err| anyhow::anyhow!("episode append failed: {err}"))?;
        Ok(())
    }
}

fn attach_episode_hdc_fingerprint(
    episode: &mut Episode,
    plan_id: &str,
    task_id: &str,
    outcome: &crate::dispatch::AgentOutcome,
    succeeded: bool,
    prompt_text: &Option<String>,
) {
    let prompt = prompt_text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{plan_id}/{task_id}"));
    let outcome_text = if outcome.output.trim().is_empty() {
        format!(
            "succeeded={} model={} provider={}",
            succeeded, &outcome.model, &outcome.provider
        )
    } else {
        outcome.output.clone()
    };
    let fingerprint = fingerprint_episode(&prompt, &outcome_text);
    episode.hdc_fingerprint = Some(encode_hdc_fingerprint(&fingerprint));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{AgentOutcome, ModelChoiceSource};
    use tempfile::tempdir;

    fn outcome() -> AgentOutcome {
        AgentOutcome {
            task_id: "task-1".into(),
            plan_id: "plan-1".into(),
            model: "claude-sonnet-4-6".into(),
            provider: "claude_cli".into(),
            output: "ok".into(),
            tokens_in: 200,
            tokens_out: 80,
            cost_usd: 0.003,
            duration_ms: 1234,
            exit_code: Some(0),
            is_error: false,
        }
    }

    #[tokio::test]
    async fn task_completed_writes_episode_with_provider_attribution() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("episodes.jsonl");
        let sink = EpisodeSink::at(&path);
        let event = FeedbackEvent::TaskCompleted {
            turns: 0,
            failure_reason: None,
            settled: None,
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            outcome: outcome(),
            model_source: ModelChoiceSource::Router,
            succeeded: true,
            routing_context: None,
            prompt_text: Some("system prompt\n\nuser prompt".into()),
            cache_read_tokens: 0,
            knowledge_ids: vec!["k-1".into()],
            playbook_ids: vec!["pb-1".into()],
            initial_model: "claude-sonnet-4-6".into(),
        };
        sink.on_event(&event).await.unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"backend\":\"claude_cli\""));
        assert!(contents.contains("\"model\":\"claude-sonnet-4-6\""));
        assert!(
            contents.contains("\"plan_id\":\"plan-1\""),
            "extra carries plan id"
        );
        assert!(contents.contains("\"task_id\":\"task-1\""));
        assert!(contents.contains("\"hdc_fingerprint\""));
        // P3-4: episode extra keys for compounding metrics
        assert!(
            contents.contains("\"knowledge_used\":true"),
            "extra should contain knowledge_used"
        );
        assert!(
            contents.contains("\"playbook_id\":\"pb-1\""),
            "extra should contain playbook_id"
        );
        assert!(
            contents.contains("\"cache_hit\":false"),
            "extra should contain cache_hit (false when cache_read_tokens=0)"
        );
        assert!(
            contents.contains("\"initial_model\":\"claude-sonnet-4-6\""),
            "extra should contain initial_model"
        );
        assert!(
            contents.contains("\"successful_model\":\"claude-sonnet-4-6\""),
            "extra should contain successful_model"
        );
    }

    #[tokio::test]
    async fn verify_failure_keeps_the_full_reason_and_a_failed_verdict() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("episodes.jsonl");
        let sink = EpisodeSink::at(&path);
        let reason = "verify: 1/2 verify step(s) failed for task `Greet`:\n\n\
                      verify[1:test] `cargo test` failed: exit code: 101\n\
                      thread 'greets' panicked at src/lib.rs:4:5";
        let mut failed = outcome();
        failed.is_error = true;
        sink.on_event(&FeedbackEvent::TaskCompleted {
            turns: 3,
            failure_reason: Some(reason.into()),
            settled: None,
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            outcome: failed,
            model_source: ModelChoiceSource::Router,
            succeeded: false,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: String::new(),
        })
        .await
        .unwrap();

        let episode = EpisodeLogger::read_all(&path).await.unwrap().remove(0);
        assert_eq!(episode.failure_reason.as_deref(), Some(reason));
        assert_eq!(episode.extra["failure_class"], "verify");
        assert_eq!(
            episode.gate_verdicts,
            [EpisodeGateVerdict::new("verify", false)]
        );
        assert!(!episode.extra.contains_key(BLAMED_TASKS_KEY));
    }

    /// Every attempt still gets its episode, but a learner reading episodes
    /// can tell an attempt that teaches nothing: an unverified attempt keeps
    /// `success` (its provider call succeeded and no verify step failed) and
    /// carries a `null` learning label.
    #[tokio::test]
    async fn episodes_carry_the_learning_label_learners_read() {
        use crate::runtime_feedback::settled_as;
        use roko_learn::telemetry::AttemptOutcome;

        let dir = tempdir().unwrap();
        let path = dir.path().join("episodes.jsonl");
        let sink = EpisodeSink::at(&path);
        let cases = [
            (
                AttemptOutcome::Unverified,
                true,
                "none",
                serde_json::Value::Null,
            ),
            (AttemptOutcome::Passed, true, "none", serde_json::json!(1)),
            (
                AttemptOutcome::GateFailed,
                false,
                "agent",
                serde_json::json!(0),
            ),
            (
                AttemptOutcome::ProviderError,
                false,
                "infra",
                serde_json::Value::Null,
            ),
        ];
        for (verdict, succeeded, _, _) in &cases {
            sink.on_event(&FeedbackEvent::TaskCompleted {
                turns: 1,
                failure_reason: None,
                settled: settled_as(*verdict, true),
                plan_id: "plan-1".into(),
                task_id: "task-1".into(),
                outcome: outcome(),
                model_source: ModelChoiceSource::Router,
                succeeded: *succeeded,
                routing_context: None,
                prompt_text: None,
                cache_read_tokens: 0,
                knowledge_ids: vec![],
                playbook_ids: vec![],
                initial_model: String::new(),
            })
            .await
            .unwrap();
        }

        let episodes = EpisodeLogger::read_all(&path).await.unwrap();
        assert_eq!(episodes.len(), cases.len());
        for (episode, (verdict, succeeded, blame, label)) in episodes.iter().zip(&cases) {
            let wire = serde_json::to_value(verdict).unwrap();
            assert_eq!(episode.extra["outcome"], wire);
            assert_eq!(episode.success, *succeeded, "{wire}");
            assert_eq!(episode.extra["blame"], *blame, "{wire}");
            assert_eq!(episode.extra["learning_label"], *label, "{wire}");
        }
    }

    #[tokio::test]
    async fn sink_ignores_non_task_events() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("episodes.jsonl");
        let sink = EpisodeSink::at(&path);
        let event = FeedbackEvent::IdleTick {
            ticks_since_last_work: 1,
        };
        assert!(!sink.interested(&event));
        // on_event must still be safe to call — should be a no-op.
        sink.on_event(&event).await.unwrap();
        // No file should have been created.
        assert!(!path.exists() || std::fs::read(&path).unwrap().is_empty());
    }
}
