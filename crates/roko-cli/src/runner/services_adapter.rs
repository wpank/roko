//! Layer-4 adapter: converts [`RuntimeServices`] from roko-execution into
//! the CLI-specific handles consumed by Runner-v2 and Graph bootstraps (#244).
//!
//! This module replaces the duplicated service-construction blocks in
//! `event_loop.rs` and `commands/plan.rs` with a single call to
//! [`build_shared_services`]. The scheduling implementations remain
//! separate until cutover.

use std::path::Path;
use std::sync::Arc;

use roko_execution::{
    BuilderError, ExecutionOverrides, RuntimeProfile, RuntimeServices, RuntimeServicesBuilder,
};
use roko_learn::cascade_router::CascadeRouter;
use tracing::info;

/// Build shared runtime services for a plan execution surface.
///
/// Both Runner-v2 (`FullPlan`) and Graph (`GraphPlan`) call this once
/// before their event loops. The returned [`RuntimeServices`] provides
/// the shared health registry, cascade router, budget, and dispatch
/// metadata that was previously duplicated across the two bootstrap paths.
///
/// # Arguments
///
/// * `profile` - `FullPlan` for Runner-v2, `GraphPlan` for Graph engine.
/// * `workdir` - Workspace root.
/// * `overrides` - Resolved execution overrides from CLI flags (#262).
/// * `cascade_router` - Optional pre-loaded cascade router from `RunConfig`.
/// * `extension_names` - Extension names from project config.
pub fn build_shared_services(
    profile: RuntimeProfile,
    workdir: &Path,
    overrides: ExecutionOverrides,
    cascade_router: Option<Arc<CascadeRouter>>,
    extension_names: Vec<String>,
) -> Result<RuntimeServices, BuilderError> {
    let mut builder = RuntimeServicesBuilder::new(profile, overrides);

    if let Some(router) = cascade_router {
        builder = builder.with_cascade_router(router);
    }

    if !extension_names.is_empty() {
        builder = builder.with_extension_names(extension_names);
    }

    let services = builder.build(workdir)?;

    info!(
        profile = %services.profile,
        has_cascade_router = services.dispatch.cascade_router.is_some(),
        has_model_override = services.dispatch.cli_model_override.is_some(),
        budget_ceiling = services.guards.budget_ceiling,
        dangerously_skip_permissions = services.guards.dangerously_skip_permissions,
        "shared runtime services constructed for plan execution"
    );

    Ok(services)
}

/// Convert CLI-resolved plan budget parameters into an [`ExecutionOverrides`].
///
/// This replaces the manual budget resolution that was duplicated in both
/// `event_loop.rs` and `commands/plan.rs`.
pub fn overrides_from_plan_config(
    cli_model_override: Option<String>,
    dangerously_skip_permissions: bool,
    budget_ceiling: f64,
    turn_ceiling: f64,
    budget_override_active: bool,
    screenshots: bool,
    log_file: Option<std::path::PathBuf>,
) -> ExecutionOverrides {
    ExecutionOverrides {
        model: cli_model_override,
        dangerously_skip_permissions,
        budget_ceiling,
        turn_ceiling,
        budget_override_active,
        screenshots,
        log_file,
        ..Default::default()
    }
}

/// Publish a [`RunnerLifecycleEvent`] through the canonical #208 envelope.
///
/// This is the layer-4 adapter that wraps lifecycle events into the
/// `RuntimeEventEnvelope` format without adding Runner variants to core.
/// The event is logged via tracing and, when an HTTP sink is available,
/// forwarded to the running `roko serve` process.
pub fn emit_lifecycle_event(
    event: &roko_execution::RunnerLifecycleEvent,
    http_sink: Option<&roko_runtime::HttpEventSink>,
) {
    let json = match serde_json::to_string(event) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!(error = %e, "failed to serialize runner lifecycle event");
            return;
        }
    };
    tracing::debug!(event = %json, "runner lifecycle event");

    if let Some(sink) = http_sink {
        // Fire-and-forget: the HTTP sink is non-blocking. If the serve
        // process is not running, the send silently fails.
        // Emit a lifecycle log event through the HTTP sink.
        // The event bus carries structured RuntimeEvent variants; for
        // unstructured lifecycle JSON we use the Log variant.
        let envelope = roko_core::runtime_event::RuntimeEvent::Extension {
            namespace: "runner.lifecycle".to_string(),
            version: "1".to_string(),
            value: serde_json::Value::String(json),
        };
        sink.emit(envelope);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_shared_services_fullplan() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let overrides = ExecutionOverrides {
            model: Some("test-model".to_string()),
            budget_ceiling: 10.0,
            turn_ceiling: 1.0,
            ..Default::default()
        };

        let services = build_shared_services(
            RuntimeProfile::FullPlan,
            workdir.path(),
            overrides,
            None,
            Vec::new(),
        )
        .unwrap();

        assert_eq!(services.profile, RuntimeProfile::FullPlan);
        assert_eq!(
            services.dispatch.cli_model_override.as_deref(),
            Some("test-model")
        );
        assert!((services.guards.budget_ceiling - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_shared_services_graphplan() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let overrides = ExecutionOverrides {
            model: Some("test-model".to_string()),
            budget_ceiling: 10.0,
            turn_ceiling: 1.0,
            ..Default::default()
        };

        let services = build_shared_services(
            RuntimeProfile::GraphPlan,
            workdir.path(),
            overrides,
            None,
            Vec::new(),
        )
        .unwrap();

        assert_eq!(services.profile, RuntimeProfile::GraphPlan);
    }

    #[test]
    fn fullplan_and_graphplan_produce_same_resolved_metadata() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let overrides = ExecutionOverrides {
            model: Some("shared-model".to_string()),
            budget_ceiling: 5.0,
            turn_ceiling: 0.5,
            dangerously_skip_permissions: true,
            ..Default::default()
        };

        let full = build_shared_services(
            RuntimeProfile::FullPlan,
            workdir.path(),
            overrides.clone(),
            None,
            Vec::new(),
        )
        .unwrap();

        let graph = build_shared_services(
            RuntimeProfile::GraphPlan,
            workdir.path(),
            overrides,
            None,
            Vec::new(),
        )
        .unwrap();

        // Acceptance criterion: both engines emit the same resolved
        // provider/model metadata for an equivalent dispatch request.
        assert_eq!(
            full.dispatch.cli_model_override,
            graph.dispatch.cli_model_override
        );
        assert!((full.guards.budget_ceiling - graph.guards.budget_ceiling).abs() < f64::EPSILON);
        assert!((full.guards.turn_ceiling - graph.guards.turn_ceiling).abs() < f64::EPSILON);
        assert_eq!(
            full.guards.dangerously_skip_permissions,
            graph.guards.dangerously_skip_permissions
        );
        assert_eq!(
            full.guards.budget_override_active,
            graph.guards.budget_override_active
        );
    }

    #[test]
    fn overrides_from_plan_config_propagates() {
        let overrides = overrides_from_plan_config(
            Some("sonnet-4".to_string()),
            true,
            10.0,
            1.0,
            true,
            true,
            Some(std::path::PathBuf::from("/tmp/events.jsonl")),
        );
        assert_eq!(overrides.model.as_deref(), Some("sonnet-4"));
        assert!(overrides.dangerously_skip_permissions);
        assert!((overrides.budget_ceiling - 10.0).abs() < f64::EPSILON);
        assert!(overrides.screenshots);
        assert!(overrides.log_file.is_some());
    }

    #[test]
    fn build_with_cascade_router_and_extensions() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let router = Arc::new(CascadeRouter::load_or_new(
            &workdir.path().join("router.json"),
            vec!["model-a".to_string()],
        ));

        let services = build_shared_services(
            RuntimeProfile::FullPlan,
            workdir.path(),
            ExecutionOverrides::default(),
            Some(router),
            vec!["ext-1".to_string(), "ext-2".to_string()],
        )
        .unwrap();

        assert!(services.dispatch.cascade_router.is_some());
        assert_eq!(services.extensions.extension_names.len(), 2);
    }

    #[test]
    fn emit_lifecycle_event_without_sink() {
        // Should not panic when no HTTP sink is provided.
        let event = roko_execution::RunnerLifecycleEvent::Started {
            profile: "FullPlan".to_string(),
            plan_count: 1,
            task_count: 5,
        };
        emit_lifecycle_event(&event, None);
    }
}
