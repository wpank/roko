//! The model that served a provider call (bug-31438d): the model the
//! provider reported, against the slug the bridge launched.
//!
//! A provider may serve another model than the one asked for: a model alias,
//! a router, a gateway that falls back. The attempt's records then carry
//! both names, its cost is priced by the model that served, and a `--model`
//! pin makes the substitution the attempt's error.

use roko_core::pricing_snapshot::PriceSnapshot;

use super::*;

/// `RokoError::Gateway` category for a pinned attempt the provider served
/// with another model. Non-retryable: a retry would be substituted again.
const MODEL_SUBSTITUTED_CATEGORY: &str = "model_substituted";

/// Whether `reported`, the model a provider said it served, is `launched`,
/// the slug the bridge asked for. Case, a provider prefix (`openai/gpt-4o`)
/// and a dated snapshot suffix (`gpt-4o-2024-08-06`,
/// `claude-sonnet-4-5-20250929`) do not make another model.
pub(super) fn same_model(launched: &str, reported: &str) -> bool {
    fn name(model: &str) -> String {
        let model = model.trim().to_ascii_lowercase();
        match model.rsplit_once('/') {
            Some((_, name)) => name.to_string(),
            None => model,
        }
    }
    fn snapshot_of(base: &str, model: &str) -> bool {
        model
            .strip_prefix(base)
            .and_then(|rest| rest.strip_prefix('-'))
            .is_some_and(|suffix| {
                suffix.chars().all(|c| c.is_ascii_digit() || c == '-')
                    && suffix.chars().filter(char::is_ascii_digit).count() >= 4
            })
    }
    let (launched, reported) = (name(launched), name(reported));
    launched == reported || snapshot_of(&launched, &reported) || snapshot_of(&reported, &launched)
}

/// CLI and ACP agents report their own cost; API providers' usage is priced
/// by roko from the model's rates.
pub(super) const fn is_cli_backend(kind: roko_core::ProviderKind) -> bool {
    matches!(
        kind,
        roko_core::ProviderKind::ClaudeCli
            | roko_core::ProviderKind::CodexCli
            | roko_core::ProviderKind::GeminiCli
            | roko_core::ProviderKind::CursorCli
            | roko_core::ProviderKind::CursorAcp
    )
}

/// The model that served one provider call, as its result reports it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ServedModel {
    /// The model the provider reported serving: the one its last response
    /// named. `None` when no response named one.
    pub(super) reported: Option<String>,
    /// Every model its responses named, in order, when they disagreed.
    pub(super) all_reported: Vec<String>,
    /// A reported model is not the one the bridge launched.
    pub(super) mismatch: bool,
}

impl ServedModel {
    /// What served `dispatch`: the usage observation's model, and the
    /// `models_reported` tag roko's tool loop sets when its turns named
    /// different models.
    pub(super) fn of(dispatch: &crate::dispatch_v2::AgentResultDispatch) -> Self {
        let reported = dispatch
            .result
            .usage_obs
            .as_ref()
            .and_then(|usage| usage.model.clone())
            .filter(|model| !model.trim().is_empty());
        let all_reported: Vec<String> = dispatch
            .result
            .output
            .tag("models_reported")
            .map(|models| {
                models
                    .split(',')
                    .map(str::trim)
                    .filter(|model| !model.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let launched = &dispatch.target.model_slug;
        let mismatch = all_reported
            .iter()
            .chain(reported.iter())
            .any(|model| !same_model(launched, model));
        Self {
            reported,
            all_reported,
            mismatch,
        }
    }
}

impl GraphTaskDispatcher {
    /// The dated price snapshot this run prices model calls from (backlog
    /// 2114): the one decision 2113 picks for the workspace, loaded once.
    pub(super) fn pricing_snapshot(&self) -> Option<Arc<PriceSnapshot>> {
        crate::dispatch_v2::pricing_snapshot(&self.config.pricing, &self.workdir)
    }

    /// Check the model the provider reported serving `dispatch` against the
    /// slug the bridge launched. A substitution is logged at WARN and priced
    /// by the model that served ([`Self::price_by_served_model`]). Under a
    /// `--model` pin it is the attempt's error, returned here: the attempt
    /// still settles and records both models, and is not retried.
    pub(super) fn check_served_model(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        dispatch: &mut crate::dispatch_v2::AgentResultDispatch,
    ) -> Option<RokoError> {
        let served = ServedModel::of(dispatch);
        if !served.mismatch {
            return None;
        }
        let reported = served.reported.as_deref().unwrap_or_default();
        let launched = dispatch.target.model_slug.clone();
        let provider = dispatch.target.provider_id.clone();
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id,
            provider = %provider,
            launched = %launched,
            reported,
            models_reported = %served.all_reported.join(","),
            pinned = self.cli_model_override.is_some(),
            "the provider served another model than the one dispatched"
        );
        if let Some(reported) = served.reported.as_deref() {
            self.price_by_served_model(dispatch, reported);
        }
        let pin = self.cli_model_override.as_deref()?;
        Some(RokoError::Gateway {
            category: MODEL_SUBSTITUTED_CATEGORY,
            retryable: false,
            message: format!(
                "the --model pin `{pin}` dispatched `{launched}` on `{provider}`, but the provider \
                 reported serving `{reported}`. The attempt is not retried: drop --model to accept \
                 the substitution, or fix the provider's model routing."
            ),
        })
    }

    /// Price an API provider's usage at the rates of `served`, the model
    /// that served it: the run's price snapshot, else a `[models.*]` profile
    /// with that slug, else the built-in table. With no price for it the
    /// cost is unknown (0), not the launched model's. A CLI agent's own
    /// reported cost stands.
    pub(super) fn price_by_served_model(
        &self,
        dispatch: &mut crate::dispatch_v2::AgentResultDispatch,
        served: &str,
    ) {
        if is_cli_backend(dispatch.target.provider_kind) {
            return;
        }
        let models = self.config.effective_models();
        let profile = models
            .values()
            .find(|profile| profile.slug == served)
            .or_else(|| {
                models
                    .values()
                    .find(|profile| same_model(&profile.slug, served))
            });
        let usage = &mut dispatch.result.usage;
        usage.cost_usd = 0.0;
        let snapshot = self.pricing_snapshot();
        crate::dispatch_v2::fill_usage_cost_from_pricing(
            usage,
            snapshot.as_deref(),
            profile,
            served,
        );
        let cost_usd = f64::from(usage.cost_usd);
        if let Some(observation) = dispatch.result.usage_obs.as_mut() {
            observation.cost_usd = (cost_usd > 0.0).then_some(cost_usd);
        }
    }
}

#[cfg(test)]
mod tests {
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_learn::episode_logger::{Episode, EpisodeLogger};
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        FIXTURE_HANG_GUARD_SECS, FIXTURE_PROVIDER_TIMEOUT_MS, final_turn, jsonl_rows_where,
        make_bare_dispatcher, make_spec, make_task_def, recording_feedback, spawn_openai_mock,
        spawn_openai_stream_mock, tool_call_turn,
    };

    const RUN: &str = "graph-served-model-run";

    /// `gpt-oss-120b` on an OpenAI-compatible provider served by the mock at
    /// `base_url`; `glm-4.7` is priced but has no provider.
    fn openai_config(base_url: &str) -> RokoConfig {
        let model = |slug: &str, prices: (f64, f64)| ModelProfile {
            provider: "cerebras".to_string(),
            slug: slug.to_string(),
            context_window: 128_000,
            max_output: Some(1_024),
            max_tools: Some(32),
            supports_tools: true,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: Some(prices.0),
            cost_output_per_m: Some(prices.1),
            ..ModelProfile::default()
        };
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "gpt-oss-120b".to_string();
        config.agent.bare_mode = false;
        config.gates.cargo_fix_enabled = false;
        // `PATH` is always set, standing in for the provider's key.
        config.providers.insert(
            "cerebras".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some(base_url.to_string()),
                api_key_env: Some("PATH".to_string()),
                command: None,
                args: None,
                timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
            },
        );
        config.models.insert(
            "gpt-oss-120b".to_string(),
            model("gpt-oss-120b", (0.35, 0.75)),
        );
        config
            .models
            .insert("glm-4.7".to_string(), model("glm-4.7", (0.60, 2.20)));
        // The mock API answers without SSE. The stall watchdog would attach
        // live output, over which the tool loop streams, so keep it off.
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        config
    }

    fn served_as(model: &str, mut response: serde_json::Value) -> serde_json::Value {
        response["model"] = serde_json::json!(model);
        response
    }

    async fn episodes(workdir: &Path) -> Vec<Episode> {
        EpisodeLogger::read_all(&workdir.join(".roko/episodes.jsonl"))
            .await
            .expect("episodes")
    }

    /// A provider that answers as another model than the one dispatched:
    /// the verdict, episode, cost and efficiency rows name both, the cost is
    /// priced by the model that served, and a `--model` pin fails the
    /// attempt without a retry.
    #[tokio::test]
    async fn records_carry_the_provider_reported_model() {
        let temp = tempdir().expect("tempdir");
        let usage = |prompt: u64, completion: u64| {
            serde_json::json!({
                "prompt_tokens": prompt,
                "completion_tokens": completion,
                "total_tokens": prompt + completion
            })
        };
        let mut answer = served_as("glm-4.7", final_turn("done"));
        answer["usage"] = usage(1_000_000, 100_000);
        let (base_url, _requests) = spawn_openai_mock(vec![answer.clone(), answer]);
        let mut task = make_task_def("focused");
        task.model_hint = Some("gpt-oss-120b".to_string());
        task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        let dispatcher = make_bare_dispatcher(openai_config(&base_url), temp.path())
            .await
            .with_feedback(recording_feedback(temp.path()));
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the substituted answer is unverified, not failed");
        let budget = dispatcher.plan_budget_snapshot(&spec.plan_id);
        // Priced at glm-4.7's rates: 1M in at $0.60, 100k out at $2.20.
        assert!((budget.spent_usd - 0.82).abs() < 1e-4, "{budget:?}");
        drop(dispatcher);

        let verdicts = jsonl_rows_where(
            &temp
                .path()
                .join(".roko/runs")
                .join(RUN)
                .join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let executed = &verdicts[0]["executed"];
        assert_eq!(executed["model_dispatched"], "gpt-oss-120b");
        assert_eq!(executed["model_reported"], "glm-4.7");
        assert_eq!(executed["model_mismatch"], true);
        assert_eq!(executed["failover_chain"], serde_json::json!([]));

        let episode = episodes(temp.path()).await.remove(0);
        assert_eq!(episode.model, "gpt-oss-120b");
        assert_eq!(episode.extra["model_reported"], "glm-4.7");
        assert_eq!(episode.extra["model_mismatch"], true);
        assert!((episode.usage.cost_usd - 0.82).abs() < 1e-4, "{episode:?}");

        let costs =
            jsonl_rows_where(&temp.path().join(".roko/learn/costs.jsonl"), 1, |_| true).await;
        assert_eq!(costs[0]["model"], "gpt-oss-120b");
        assert_eq!(costs[0]["model_reported"], "glm-4.7");
        let efficiency = jsonl_rows_where(
            &temp.path().join(".roko/learn/efficiency.jsonl"),
            1,
            |row| row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA,
        )
        .await;
        assert_eq!(efficiency[0]["model"], "gpt-oss-120b");
        assert_eq!(efficiency[0]["model_reported"], "glm-4.7");
        assert_eq!(efficiency[0]["model_mismatch"], true);

        // Pinned with --model, the same substitution fails the attempt and
        // is not retried; its records still name both models.
        let pinned_dir = tempdir().expect("tempdir");
        let pinned = make_bare_dispatcher(openai_config(&base_url), pinned_dir.path())
            .await
            .with_cli_model_override(Some("gpt-oss-120b".to_string()))
            .with_feedback(recording_feedback(pinned_dir.path()));
        let error = pinned
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("a pinned model the provider did not serve fails");
        let RokoError::Gateway {
            category,
            retryable,
            message,
        } = &error
        else {
            panic!("expected a non-retryable gateway error, got {error:?}");
        };
        assert_eq!(*category, MODEL_SUBSTITUTED_CATEGORY);
        assert!(!retryable);
        assert!(message.contains("glm-4.7"), "{message}");
        let episode = episodes(pinned_dir.path()).await.remove(0);
        assert!(!episode.success);
        assert_eq!(episode.extra["model_reported"], "glm-4.7");
    }

    /// With the stall watchdog on, live output makes the tool loop stream
    /// its model calls. The model the stream's chunks name is the one the
    /// records report (bug-bfd241), as for an answer sent in one piece.
    #[tokio::test]
    async fn streamed_attempts_record_the_provider_reported_model() {
        let temp = tempdir().expect("tempdir");
        let (base_url, requests) = spawn_openai_stream_mock(vec![vec![
            serde_json::json!({
                "model": "glm-4.7",
                "choices": [{"index": 0, "delta": {"role": "assistant", "content": "done"}}]
            }),
            serde_json::json!({
                "model": "glm-4.7",
                "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]
            }),
            serde_json::json!({
                "model": "glm-4.7",
                "choices": [],
                "usage": {"prompt_tokens": 1_000, "completion_tokens": 100, "total_tokens": 1_100}
            }),
        ]]);
        let mut config = openai_config(&base_url);
        config.conductor = roko_core::config::schema::ConductorConfig::default();
        let mut task = make_task_def("focused");
        task.model_hint = Some("gpt-oss-120b".to_string());
        task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        let dispatcher = make_bare_dispatcher(config, temp.path())
            .await
            .with_feedback(recording_feedback(temp.path()));
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the streamed answer is unverified, not failed");
        assert_eq!(
            requests.lock()[0]["stream"],
            true,
            "the model call streamed"
        );
        drop(dispatcher);

        let verdicts = jsonl_rows_where(
            &temp
                .path()
                .join(".roko/runs")
                .join(RUN)
                .join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let executed = &verdicts[0]["executed"];
        assert_eq!(executed["model_dispatched"], "gpt-oss-120b");
        assert_eq!(executed["model_reported"], "glm-4.7", "{executed}");
        assert_eq!(executed["model_mismatch"], true);
        let costs =
            jsonl_rows_where(&temp.path().join(".roko/learn/costs.jsonl"), 1, |_| true).await;
        assert_eq!(costs[0]["model_reported"], "glm-4.7");
        assert_eq!(costs[0]["input_tokens"], 1_000);
    }

    /// Each model call of roko's tool loop is a turn: an OpenAI-compatible
    /// dispatch that calls the provider three times records three turns on
    /// its verdict, episode and efficiency row.
    #[tokio::test]
    async fn episode_turns_count_tool_loop_calls() {
        let temp = tempdir().expect("tempdir");
        std::fs::write(temp.path().join("notes.txt"), "draft notes\n").expect("seed notes");
        let read = |id: &str| {
            served_as(
                "gpt-oss-120b",
                tool_call_turn(id, "read_file", serde_json::json!({ "path": "notes.txt" })),
            )
        };
        let (base_url, requests) = spawn_openai_mock(vec![
            read("call-1"),
            read("call-2"),
            served_as("gpt-oss-120b", final_turn("read it twice")),
        ]);
        let mut task = make_task_def("focused");
        task.model_hint = Some("gpt-oss-120b".to_string());
        task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        let dispatcher = make_bare_dispatcher(openai_config(&base_url), temp.path())
            .await
            .with_feedback(recording_feedback(temp.path()));
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("three calls, then an answer");
        drop(dispatcher);
        assert_eq!(requests.lock().len(), 3, "the provider saw three calls");

        let verdicts = jsonl_rows_where(
            &temp
                .path()
                .join(".roko/runs")
                .join(RUN)
                .join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        assert_eq!(verdicts[0]["executed"]["turns"], 3);
        assert_eq!(verdicts[0]["executed"]["model_mismatch"], false);
        let episode = episodes(temp.path()).await.remove(0);
        assert_eq!(episode.turns, 3);
        assert!(!episode.extra.contains_key("turns_unknown"));
        let efficiency = jsonl_rows_where(
            &temp.path().join(".roko/learn/efficiency.jsonl"),
            1,
            |row| row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA,
        )
        .await;
        assert_eq!(efficiency[0]["turn_number"], 3);
    }

    #[test]
    fn a_dated_snapshot_or_provider_prefix_is_the_same_model() {
        for (launched, reported) in [
            ("gpt-oss-120b", "gpt-oss-120b"),
            ("gpt-4o", "gpt-4o-2024-08-06"),
            ("claude-sonnet-4-5-20250929", "claude-sonnet-4-5"),
            ("gpt-oss-120b", "openai/gpt-oss-120b"),
            ("GLM-4.7", "glm-4.7"),
        ] {
            assert!(same_model(launched, reported), "{launched} vs {reported}");
        }
        for (launched, reported) in [
            ("gpt-oss-120b", "glm-4.7"),
            ("gpt-4o", "gpt-4o-mini"),
            ("claude-sonnet-4", "claude-sonnet-4-5"),
            ("gpt-oss-120b", "gpt-oss-20b"),
        ] {
            assert!(!same_model(launched, reported), "{launched} vs {reported}");
        }
    }
}
