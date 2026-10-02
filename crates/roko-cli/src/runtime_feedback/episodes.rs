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
use roko_learn::episode_logger::{
    Episode, EpisodeGateVerdict, EpisodeLogger, LEARNING_LABEL_KEY, Usage,
};
use roko_learn::hdc_fingerprint::{encode as encode_hdc_fingerprint, fingerprint_episode};
use roko_learn::hindsight::BLAMED_TASKS_KEY;
use roko_learn::telemetry::AttemptVerdictRecord;

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
            routing_context,
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
                (
                    LEARNING_LABEL_KEY,
                    serde_json::json!(settled.learning_label),
                ),
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
            // Every playbook dispatch credited, so a hindsight relabel can
            // retract each one (gap-b95d94).
            episode.extra.insert(
                super::hindsight::PLAYBOOK_IDS_KEY.into(),
                serde_json::json!(playbook_ids),
            );
        }
        // The category the routing sink counts this attempt's credit under,
        // so a hindsight relabel can retract it (gap-b95d94).
        let routing_category = routing_context.as_ref().map_or_else(
            || super::routing::build_fallback_routing_context(&outcome.model, None).task_category,
            |ctx| ctx.task_category,
        );
        episode.extra.insert(
            super::hindsight::ROUTING_CATEGORY_KEY.into(),
            serde_json::json!(routing_category),
        );
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
        if let Some(settled) = settled {
            attach_settled_attempt(&mut episode, settled);
        }

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

/// What the attempt's verdict records that the event's legacy fields cannot
/// say: the model the provider reported serving (bug-31438d), the failover
/// that replaced the planned model (bug-35379d), a turn count the agent
/// never reported (bug-55fd84), and the helper model calls made for the
/// attempt (bug-62e3f4), which stay out of `usage`: that is the agent run's.
fn attach_settled_attempt(episode: &mut Episode, settled: &AttemptVerdictRecord) {
    let executed = &settled.executed;
    episode.extra.insert(
        "model_reported".into(),
        executed
            .model_reported
            .clone()
            .map_or(serde_json::Value::Null, serde_json::Value::String),
    );
    episode.extra.insert(
        "model_mismatch".into(),
        serde_json::Value::Bool(executed.model_mismatch),
    );
    if !executed.models_reported.is_empty() {
        episode.extra.insert(
            "models_reported".into(),
            serde_json::json!(executed.models_reported),
        );
    }
    if let Some(planned) = executed.failover_chain.first() {
        episode.extra.insert(
            "substituted_from".into(),
            serde_json::Value::String(planned.clone()),
        );
        episode.extra.insert(
            "failover_chain".into(),
            serde_json::json!(executed.failover_chain),
        );
        if let Some(reason) = &executed.failover_reason {
            episode.extra.insert(
                "failover_reason".into(),
                serde_json::Value::String(reason.clone()),
            );
        }
        if !executed.failover_refusals.is_empty() {
            episode.extra.insert(
                "failover_refusals".into(),
                serde_json::json!(executed.failover_refusals),
            );
        }
    }
    // The sampling parameters the requests carried; empty when the
    // provider's defaults applied (gap-13bbbd).
    episode
        .extra
        .insert("sampling".into(), serde_json::json!(executed.sampling));
    // An unreported count is unknown, not one turn.
    match executed.turns {
        Some(turns) => episode.turns = u64::from(turns),
        None => {
            episode.turns = 0;
            episode
                .extra
                .insert("turns_unknown".into(), serde_json::Value::Bool(true));
        }
    }
    if let Some(helpers) = &settled.helpers {
        episode
            .extra
            .insert("helper_calls".into(), serde_json::json!(helpers.calls));
        episode.extra.insert(
            "helper_cost_usd".into(),
            serde_json::json!(helpers.cost_usd),
        );
        episode.extra.insert(
            "helper_tokens_in".into(),
            serde_json::json!(helpers.tokens_in),
        );
        episode.extra.insert(
            "helper_tokens_out".into(),
            serde_json::json!(helpers.tokens_out),
        );
        if helpers.unpriced_calls > 0 {
            episode.extra.insert(
                "helper_unpriced_calls".into(),
                serde_json::json!(helpers.unpriced_calls),
            );
        }
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

    /// The settled verdict says what the event's legacy fields cannot: the
    /// model the provider reported, the failover, the helper calls, and a
    /// turn count the agent never reported, which is unknown rather than
    /// the event's fallback of one.
    #[tokio::test]
    async fn settled_attempt_records_served_model_turns_and_helpers() {
        use roko_learn::telemetry::{
            AttemptIdentity, AttemptKey, AttemptOutcome, HelperCallsUsage,
        };

        let dir = tempdir().unwrap();
        let path = dir.path().join("episodes.jsonl");
        let sink = EpisodeSink::at(&path);
        let key = AttemptKey::new("run-1", "plan-1", "task-1", 1);
        let mut verdict = AttemptVerdictRecord::settle(
            AttemptIdentity::new(&key),
            AttemptOutcome::Unverified,
            true,
        );
        verdict.executed.model_reported = Some("glm-4.7".into());
        verdict.executed.model_mismatch = true;
        verdict.executed.failover_chain = vec!["claude-sonnet".into()];
        verdict.executed.failover_reason =
            Some("`claude-sonnet` on `claude_cli`: out of usage".into());
        verdict.helpers = Some(HelperCallsUsage {
            calls: 2,
            tokens_in: 40,
            tokens_out: 8,
            cost_usd: 0.25,
            ..HelperCallsUsage::default()
        });
        sink.on_event(&FeedbackEvent::TaskCompleted {
            turns: 1,
            failure_reason: None,
            settled: Some(Arc::new(verdict)),
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            outcome: outcome(),
            model_source: ModelChoiceSource::Router,
            succeeded: true,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: "claude-sonnet-4-6".into(),
        })
        .await
        .unwrap();

        let episode = EpisodeLogger::read_all(&path).await.unwrap().remove(0);
        assert_eq!(episode.turns, 0);
        assert_eq!(episode.extra["turns_unknown"], true);
        assert_eq!(episode.model, "claude-sonnet-4-6");
        assert_eq!(episode.extra["model_reported"], "glm-4.7");
        assert_eq!(episode.extra["model_mismatch"], true);
        assert_eq!(episode.extra["substituted_from"], "claude-sonnet");
        assert_eq!(
            episode.extra["failover_reason"],
            "`claude-sonnet` on `claude_cli`: out of usage"
        );
        assert_eq!(episode.extra["helper_calls"], 2);
        assert_eq!(episode.extra["helper_cost_usd"], 0.25);
        assert!(
            (episode.usage.cost_usd - 0.003).abs() < 1e-9,
            "helper cost stays out of the agent run's usage"
        );
    }

    /// gap-13bbbd: an episode names the sampling its requests carried, and
    /// records an empty map when the provider's defaults applied.
    #[tokio::test]
    async fn episodes_record_the_sampling_sent() {
        use roko_learn::telemetry::{AttemptIdentity, AttemptKey, AttemptOutcome};

        let dir = tempdir().unwrap();
        let path = dir.path().join("episodes.jsonl");
        let sink = EpisodeSink::at(&path);
        for (task_id, temperature) in [("sampled", Some(0.2)), ("defaults", None)] {
            let key = AttemptKey::new("run-1", "plan-1", task_id, 1);
            let mut verdict = AttemptVerdictRecord::settle(
                AttemptIdentity::new(&key),
                AttemptOutcome::Unverified,
                true,
            );
            if let Some(temperature) = temperature {
                verdict
                    .executed
                    .sampling
                    .insert("temperature".into(), serde_json::json!(temperature));
            }
            sink.on_event(&FeedbackEvent::TaskCompleted {
                turns: 1,
                failure_reason: None,
                settled: Some(Arc::new(verdict)),
                plan_id: "plan-1".into(),
                task_id: task_id.into(),
                outcome: outcome(),
                model_source: ModelChoiceSource::Router,
                succeeded: true,
                routing_context: None,
                prompt_text: None,
                cache_read_tokens: 0,
                knowledge_ids: vec![],
                playbook_ids: vec![],
                initial_model: "gpt-oss-120b".into(),
            })
            .await
            .unwrap();
        }

        let episodes = EpisodeLogger::read_all(&path).await.unwrap();
        assert_eq!(
            episodes[0].extra["sampling"],
            serde_json::json!({ "temperature": 0.2 })
        );
        assert_eq!(episodes[1].extra["sampling"], serde_json::json!({}));
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
