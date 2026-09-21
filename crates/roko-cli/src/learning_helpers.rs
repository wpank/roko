//! Learning and efficiency helpers (originally extracted from the legacy orchestrator).
//!
//! Free functions for provider health recording, model slug resolution,
//! and episode distillation.

use std::{path::Path, sync::Arc};

use anyhow::Result;
use roko_agent::model_call_service::ModelCallService;
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_core::foundation::ModelCaller;

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
