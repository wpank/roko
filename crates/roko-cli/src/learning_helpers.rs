//! Learning and efficiency helpers (originally extracted from the legacy orchestrator).
//!
//! Free functions for provider health recording, model slug resolution,
//! and episode distillation.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use roko_agent::model_call_service::ModelCallService;
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_core::foundation::{ModelCallRequest, ModelCallResponse, ModelCaller};
use roko_learn::costs_db::CostRecord;
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::episode_logger::Episode;
use roko_learn::runtime_feedback::LearningRuntime;

/// Resolve a configured model key or slug into the API slug learning stores use.
///
/// If `model` is empty, falls back to the config default model. Returns `None`
/// only when neither source yields a non-empty model slug.
pub(crate) fn resolve_capture_model_slug(
    config: &RokoConfig,
    model: Option<&str>,
) -> Option<String> {
    let requested = model.filter(|value| !value.trim().is_empty()).or_else(|| {
        let default_model = config.agent.default_model.trim();
        (!default_model.is_empty()).then_some(default_model)
    })?;
    let slug = resolve_model(config, requested).slug;
    (!slug.trim().is_empty()).then_some(slug)
}

/// Resolve the configured provider id for a model key or API slug.
pub(crate) fn provider_id_for_model(
    config: &RokoConfig,
    model_key_or_slug: &str,
) -> Option<String> {
    let models = config.effective_models();
    models
        .get(model_key_or_slug)
        .or_else(|| {
            models
                .values()
                .find(|profile| profile.slug == model_key_or_slug)
        })
        .map(|profile| profile.provider.clone())
        .filter(|provider| !provider.trim().is_empty())
}

/// Return the stable cascade model universe for a direct one-shot capture.
pub(crate) fn capture_runtime_model_slugs(config: &RokoConfig, episode_model: &str) -> Vec<String> {
    let mut model_slugs = config.model_slugs_for_cascade();
    if !episode_model.trim().is_empty() && !model_slugs.iter().any(|slug| slug == episode_model) {
        model_slugs.push(episode_model.to_string());
    }
    model_slugs.sort();
    model_slugs.dedup();
    model_slugs
}

/// Persist one provider-health outcome to `.roko/learn/provider-health.json`.
///
/// This writes the serialized registry used by config and TUI surfaces; it is
/// intentionally separate from the short-lived in-memory `ProviderHealthTracker`
/// used by `LearningRuntime` during a process.
pub(crate) fn record_persisted_provider_health(
    workdir: &Path,
    provider: &str,
    success: bool,
) -> Result<()> {
    roko_learn::model_call_feedback::record_provider_health_for_workdir(
        workdir, provider, success,
    )?;
    Ok(())
}

// ─── Distillation ────────────────────────────────────────────────────────

/// Build a [`ModelCaller`] configured for episode distillation.
///
/// Uses the workspace's configured default model so distillation works in
/// environments that don't have Anthropic providers (e.g. Zhipu-only deploys).
///
/// This is `pub` (not `pub(crate)`) because the binary target and the library
/// target are compiled as separate crates — `pub(crate)` would make the
/// function invisible to the binary.
pub fn distillation_model_caller(workdir: &Path) -> Arc<dyn ModelCaller> {
    let config = roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
    let model = config.agent.default_model.clone();
    Arc::new(
        ModelCallService::new(model)
            .with_config(config)
            .with_working_dir(workdir)
            .with_immune_root(workdir),
    )
}

/// Role under which the spend of an episode-distillation call is recorded.
pub const DISTILLATION_ROLE: &str = "episode-distiller";

/// Distil each episode `runtime` logs into durable knowledge, in the
/// background, through `caller`, recording what each distillation call costs
/// ([`DistillationSpend`]).
///
/// The capture paths (`agent_exec::persist_capture_episode` and the binary's
/// `commands::util::persist_capture_episode`) install this as their
/// episode-completion hook.
pub fn install_capture_distillation(
    runtime: &mut LearningRuntime,
    workdir: &Path,
    caller: Arc<dyn ModelCaller>,
) {
    let workdir = workdir.to_path_buf();
    runtime.set_episode_completion_hook(move |episode| {
        let recorded: Arc<dyn ModelCaller> =
            Arc::new(DistillationSpend::new(&workdir, &episode, Arc::clone(&caller)));
        roko_neuro::spawn_episode_distillation(workdir.clone(), episode, Some(recorded));
    });
}

/// A [`ModelCaller`] that records the spend of each call it makes to distil
/// one episode.
///
/// [`roko_neuro::spawn_episode_distillation`] calls the model detached from
/// the command that logged the episode, so nothing else sees that call's
/// usage. Each call that returns is recorded the way
/// [`crate::plan_authoring::AuthoringSpend`] records an authoring call: a cost
/// record in `.roko/learn/costs.jsonl` and an efficiency row in
/// `.roko/learn/efficiency.jsonl`, under role [`DISTILLATION_ROLE`] and the
/// episode's plan and task ids.
pub struct DistillationSpend {
    inner: Arc<dyn ModelCaller>,
    workdir: PathBuf,
    episode_id: String,
    plan_id: String,
    task_id: String,
}

impl DistillationSpend {
    /// Record, in `workdir`'s learning logs, the calls `inner` makes to distil
    /// `episode`.
    #[must_use]
    pub fn new(workdir: &Path, episode: &Episode, inner: Arc<dyn ModelCaller>) -> Self {
        let plan_id = episode
            .extra
            .get("plan_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        Self {
            inner,
            workdir: workdir.to_path_buf(),
            episode_id: episode.id.clone(),
            plan_id,
            task_id: episode.task_id.clone(),
        }
    }

    /// Append the cost record and the efficiency row of one returned call.
    /// Best-effort: a failed write is logged and never fails the call.
    async fn record(&self, response: &ModelCallResponse, duration_ms: u64) {
        let config =
            roko_core::config::loader::load_config_unified(&self.workdir).unwrap_or_default();
        let provider = provider_id_for_model(&config, &response.model)
            .unwrap_or_else(|| "unknown-provider".to_string());
        let usage = &response.usage;
        let timestamp = chrono::Utc::now().to_rfc3339();

        let cost_record = CostRecord {
            timestamp: timestamp.clone(),
            model: response.model.clone(),
            provider: provider.clone(),
            role: DISTILLATION_ROLE.to_string(),
            plan_id: self.plan_id.clone(),
            task_id: self.task_id.clone(),
            complexity_band: "standard".to_string(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cached_tokens: 0,
            cost_usd: usage.cost_usd,
            duration_ms,
            success: true,
            session_id: String::new(),
            // A `ModelCallResponse` does not say where its usage came from.
            cost_source: roko_learn::telemetry::CostSource::Unknown,
        };
        let efficiency_event = AgentEfficiencyEvent {
            agent_id: format!("{}/{DISTILLATION_ROLE}", self.episode_id),
            role: DISTILLATION_ROLE.to_string(),
            backend: provider,
            model: response.model.clone(),
            plan_id: self.plan_id.clone(),
            task_id: self.task_id.clone(),
            attempt_id: format!("{}/{DISTILLATION_ROLE}", self.episode_id),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cost_usd: usage.cost_usd,
            cost_usd_without_cache: usage.cost_usd,
            total_prompt_tokens: usage.input_tokens,
            wall_time_ms: duration_ms,
            duration_ms,
            iteration: 1,
            outcome: "success".to_string(),
            model_used: response.model.clone(),
            frequency: roko_core::OperatingFrequency::Theta,
            timestamp,
            ..AgentEfficiencyEvent::default()
        };

        let learn_dir = roko_fs::RokoLayout::for_project(&self.workdir).learn_dir();
        for (file_name, line) in [
            ("costs.jsonl", serde_json::to_string(&cost_record)),
            ("efficiency.jsonl", serde_json::to_string(&efficiency_event)),
        ] {
            let path = learn_dir.join(file_name);
            if let Err(error) = append_learn_line(&path, line).await {
                tracing::warn!(
                    path = %path.display(),
                    task_id = %self.task_id,
                    %error,
                    "distillation spend write failed (best-effort)"
                );
            }
        }
    }
}

#[async_trait]
impl ModelCaller for DistillationSpend {
    async fn call(&self, req: ModelCallRequest) -> roko_core::Result<ModelCallResponse> {
        let started = std::time::Instant::now();
        let response = self.inner.call(req).await?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.record(&response, duration_ms).await;
        Ok(response)
    }
}

/// Append one serialized record to a `.roko/learn` JSONL log, rotating it at
/// the default size.
async fn append_learn_line(path: &Path, line: serde_json::Result<String>) -> std::io::Result<()> {
    let line = line.map_err(std::io::Error::other)?;
    let max_mb = roko_core::config::ResourcesConfig::default().log_rotation_max_mb;
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        roko_fs::log_rotation::append_jsonl_line_sync(&path, line.as_bytes(), max_mb)
    })
    .await
    .map_err(std::io::Error::other)?
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use roko_core::foundation::TokenUsage;
    use roko_learn::runtime_feedback::CompletedRunInput;

    use super::*;

    /// What the fake distiller charges per call. A power of two, so it
    /// compares exactly.
    const DISTILL_COST_USD: f64 = 0.0625;

    /// A distillation model that reports a known cost and distils nothing.
    struct FakeDistiller;

    #[async_trait]
    impl ModelCaller for FakeDistiller {
        async fn call(&self, _req: ModelCallRequest) -> roko_core::Result<ModelCallResponse> {
            Ok(ModelCallResponse {
                content: r#"{"entries": []}"#.to_string(),
                model: "fake-distiller".to_string(),
                usage: TokenUsage {
                    input_tokens: 900,
                    output_tokens: 40,
                    total_tokens: 940,
                    cost_usd: DISTILL_COST_USD,
                },
                stop_reason: Some("end_turn".to_string()),
                request_id: None,
            })
        }
    }

    /// The rows of `.roko/learn/<file>` recorded under the distiller's role.
    fn distiller_rows(workdir: &Path, file: &str) -> Vec<serde_json::Value> {
        std::fs::read_to_string(workdir.join(".roko").join("learn").join(file))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|row| row["role"] == DISTILLATION_ROLE)
            .collect()
    }

    /// bug-0f8948: logging a capture episode starts a detached distillation
    /// call. Its spend lands once in the cost log and once in the efficiency
    /// log, attributed to the episode's plan and task.
    #[tokio::test]
    async fn distillation_spend_is_recorded() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut runtime = LearningRuntime::open_for_project(tmp.path())
            .await
            .expect("open learning runtime");
        install_capture_distillation(&mut runtime, tmp.path(), Arc::new(FakeDistiller));
        let (episode, _) = crate::agent_episode::build_capture_episode(
            "claude",
            Some("claude-sonnet-4-6"),
            "prd-plan-generate",
            "prd:plan:demo",
            "prompt body",
            "output body",
            true,
            42,
            None,
        );
        runtime
            .record_completed_run(CompletedRunInput::from_episode(episode))
            .await
            .expect("record capture episode");

        // The distillation runs detached; the efficiency row is written last.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while distiller_rows(tmp.path(), "efficiency.jsonl").is_empty()
            && std::time::Instant::now() < deadline
        {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let costs = distiller_rows(tmp.path(), "costs.jsonl");
        assert_eq!(costs.len(), 1, "{costs:?}");
        assert_eq!(costs[0]["cost_usd"], DISTILL_COST_USD);
        assert_eq!(costs[0]["input_tokens"], 900);
        assert_eq!(costs[0]["output_tokens"], 40);
        assert_eq!(costs[0]["model"], "fake-distiller");
        assert_eq!(costs[0]["plan_id"], "demo");
        assert_eq!(costs[0]["task_id"], "prd:plan:demo");
        let efficiency = distiller_rows(tmp.path(), "efficiency.jsonl");
        assert_eq!(efficiency.len(), 1, "{efficiency:?}");
        assert_eq!(efficiency[0]["cost_usd"], DISTILL_COST_USD);
        assert_eq!(efficiency[0]["plan_id"], "demo");
    }
}
