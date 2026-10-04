//! Learning and efficiency helpers (originally extracted from the legacy orchestrator).
//!
//! Free functions for provider health recording, model slug resolution,
//! and episode distillation.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use roko_agent::model_call_service::ModelCallService;
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_core::foundation::ModelCaller;
use roko_learn::provider_health::ErrorClass;
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

/// Persist one provider-call outcome to `.roko/learn/provider-health.json`: a
/// success when `failure` is `None`, else a failure under the class the
/// shared failure classifier reads from its text, as Graph dispatch records
/// its calls' (bug-9ca6d7). Text the classifier does not know is
/// [`ErrorClass::Unknown`].
///
/// This writes the serialized registry used by config and TUI surfaces; it is
/// intentionally separate from the short-lived in-memory `ProviderHealthTracker`
/// used by `LearningRuntime` during a process.
pub(crate) fn record_persisted_provider_outcome(
    workdir: &Path,
    provider: &str,
    failure: Option<&str>,
) -> Result<()> {
    let error = failure.map_or(ErrorClass::Unknown, ErrorClass::from_failure_text);
    roko_learn::model_call_feedback::record_provider_outcome_at(
        &roko_fs::RokoLayout::for_project(workdir).learn_dir(),
        provider,
        failure.is_none(),
        error,
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

pub use roko_neuro::DISTILLATION_ROLE;

/// Distil each episode `runtime` logs into durable knowledge, in the
/// background, through `caller`, recording what each distillation call costs
/// (`roko_neuro::DistillationSpend`).
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
        roko_neuro::spawn_recorded_episode_distillation(
            workdir.clone(),
            episode,
            Arc::clone(&caller),
        );
    });
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use roko_core::foundation::{ModelCallRequest, ModelCallResponse, TokenUsage};
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
            "plan-generate",
            "plan:generate:demo",
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
        assert_eq!(costs[0]["task_id"], "plan:generate:demo");
        let efficiency = distiller_rows(tmp.path(), "efficiency.jsonl");
        assert_eq!(efficiency.len(), 1, "{efficiency:?}");
        assert_eq!(efficiency[0]["cost_usd"], DISTILL_COST_USD);
        assert_eq!(efficiency[0]["plan_id"], "demo");
    }
}
