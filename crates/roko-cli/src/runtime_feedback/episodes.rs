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
use roko_learn::episode_logger::{Episode, EpisodeLogger, Usage};
use roko_learn::hdc_fingerprint::{encode as encode_hdc_fingerprint, fingerprint_episode};

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
            ..
        } = event
        else {
            return Ok(());
        };

        let mut episode = Episode::new(outcome.task_id.clone(), task_id.clone());
        episode.success = *succeeded;
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

        // ── Compounding metric keys (P3-4) ───────────────────────────────
        // Populate the episode `extra` bag with dispatch-time metadata so
        // downstream learning loops (c-factor, autocatalytic metrics,
        // playbook scoring) can observe non-zero values.
        episode.extra.insert(
            "knowledge_used".into(),
            serde_json::Value::Bool(!knowledge_ids.is_empty()),
        );
        if let Some(first_pb) = playbook_ids.first() {
            episode
                .extra
                .insert("playbook_id".into(), serde_json::Value::String(first_pb.clone()));
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
