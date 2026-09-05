//! Extension chain hook firing helpers.
//!
//! Extracted from `event_loop.rs` to reduce file size. These functions fire
//! pre/post-inference, gate, and error hooks through the extension chain
//! using non-blocking try_lock to avoid stalling the `select!` loop.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tracing::{info, warn};

use super::tui_bridge::TuiBridge;
use super::types::{GateCompletion, RunConfig};

/// Fire pre_inference extension hook (non-blocking try_lock to avoid stalling select).
pub(crate) async fn fire_pre_inference_hook(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    model: &str,
    role: &str,
    tui: &TuiBridge,
) {
    let Some(ext_chain) = &config.extension_chain else {
        return;
    };
    let Ok(chain) = ext_chain.try_lock() else {
        warn!("extension chain lock contended, skipping pre_inference hook");
        return;
    };
    let mut req = roko_core::extension::InferenceRequest {
        plan_id: plan_id.to_string(),
        task: task_id.to_string(),
        role: role.to_string(),
        model: model.to_string(),
        prompt_tokens: 0,
        extra: serde_json::Value::Null,
    };
    let success = chain.run_pre_inference(&mut req).await.is_ok();
    if !success {
        warn!("extension pre_inference hook failed");
    }
    tui.extension_hook(plan_id, task_id, "pre_inference", success);
}

/// Fire post_inference extension hook.
pub(crate) async fn fire_post_inference_hook(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    model: &str,
    role: &str,
    success: bool,
    cost_usd: f64,
    wall_ms: u64,
    tui: &TuiBridge,
) {
    let Some(ext_chain) = &config.extension_chain else {
        return;
    };
    let Ok(chain) = ext_chain.try_lock() else {
        warn!("extension chain lock contended, skipping post_inference hook");
        return;
    };
    let mut resp = roko_core::extension::InferenceResponse {
        plan_id: plan_id.to_string(),
        task: task_id.to_string(),
        role: role.to_string(),
        model: model.to_string(),
        success,
        cost_usd,
        wall_ms,
        extra: serde_json::Value::Null,
    };
    let hook_ok = chain.run_post_inference(&mut resp).await.is_ok();
    if !hook_ok {
        warn!("extension post_inference hook failed");
    }
    tui.extension_hook(plan_id, task_id, "post_inference", hook_ok);
}

/// Fire on_gate extension hook.
pub(crate) async fn fire_on_gate_hook(
    config: &RunConfig,
    completion: &GateCompletion,
    tui: &TuiBridge,
) {
    let Some(ext_chain) = &config.extension_chain else {
        return;
    };
    let Ok(chain) = ext_chain.try_lock() else {
        warn!("extension chain lock contended, skipping on_gate hook");
        return;
    };
    for verdict in &completion.verdicts {
        let mut event = roko_core::extension::GateEvent {
            plan_id: completion.plan_id.clone(),
            gate_name: verdict.gate_name.clone(),
            passed: verdict.passed,
            rung: format!("rung-{}", completion.rung),
            duration_ms: completion.duration_ms,
            details: serde_json::Value::Null,
        };
        let hook_ok = chain.run_on_gate(&mut event).await.is_ok();
        if !hook_ok {
            warn!(gate = %verdict.gate_name, "extension on_gate hook failed");
        }
        tui.extension_hook(
            &completion.plan_id,
            &completion.task_id,
            &format!("on_gate:{}", verdict.gate_name),
            hook_ok,
        );
    }
}

/// Fire on_error extension hook.
pub(crate) async fn fire_on_error_hook(
    config: &RunConfig,
    message: &str,
    source: &str,
    tui: &TuiBridge,
    plan_id: &str,
    task_id: &str,
) {
    let Some(ext_chain) = &config.extension_chain else {
        return;
    };
    let Ok(chain) = ext_chain.try_lock() else {
        warn!("extension chain lock contended, skipping on_error hook");
        return;
    };
    let event = roko_core::extension::ErrorEvent {
        error_message: message.to_string(),
        source: source.to_string(),
        extra: serde_json::Value::Null,
    };
    let hook_ok = chain.run_on_error(&event).await.is_ok();
    tui.extension_hook(plan_id, task_id, "on_error", hook_ok);
}

/// Initialize extensions, returning an error if a required extension fails.
pub async fn initialize_extensions(
    extension_chain: Option<&Arc<tokio::sync::Mutex<roko_core::extension::ExtensionChain>>>,
) -> Result<()> {
    let Some(extension_chain) = extension_chain else {
        return Ok(());
    };
    let mut chain = extension_chain.lock().await;
    let optional_by_name = chain
        .metadata()
        .into_iter()
        .map(|meta| (meta.name, meta.optional))
        .collect::<HashMap<_, _>>();
    let errors = chain.init_all().await;
    let mut required_errors = Vec::new();
    for (name, error) in errors {
        if optional_by_name.get(&name).copied().unwrap_or(false) {
            warn!(extension = %name, error = %error, "optional extension init failed; continuing");
            chain.disable_extension(&name);
        } else {
            required_errors.push(format!("{name}: {error}"));
        }
    }
    if required_errors.is_empty() {
        return Ok(());
    }
    for (name, error) in chain.shutdown_all().await {
        warn!(extension = %name, error = %error, "extension shutdown after startup failure failed");
    }
    Err(anyhow::anyhow!(
        "required extension initialization failed: {}",
        required_errors.join("; ")
    ))
}

/// Shutdown extension chain + persist cascade router.
pub(crate) async fn shutdown_subsystems(config: &RunConfig, tui: &TuiBridge) {
    // Extension chain shutdown.
    if let Some(ext_chain) = &config.extension_chain {
        let mut chain = ext_chain.lock().await;
        let errors = chain.shutdown_all().await;
        for (name, err) in &errors {
            warn!(extension = %name, error = %err, "extension shutdown failed");
        }
    }

    // Persist cascade router learned state.
    if let Some(router) = &config.cascade_router {
        let router_path = config.layout.cascade_router_path();
        if let Err(err) = router.save(&router_path) {
            warn!(error = %err, "failed to persist cascade router");
        } else {
            info!("cascade router state persisted");
            tui.cascade_router_updated(&router.snapshot_json());
        }
    }
}
