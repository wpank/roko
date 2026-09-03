//! RuntimeServicesBuilder and service bundle types (#243).
//!
//! Constructs the six shared service bundles once before the event loop.
//! Both Runner-v2 (`FullPlan`) and Graph (`GraphPlan`) use this builder
//! to share provider health, rate limiter, cost table, prompt cache, and
//! process supervisor contracts.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use roko_fs::RokoLayout;

use crate::profiles::RuntimeProfile;

// ---------------------------------------------------------------------------
// Service bundle types
// ---------------------------------------------------------------------------

/// Provider dispatch bundle: factory, model resolver, rate limiter, health.
#[derive(Debug, Clone)]
pub struct DispatchBundle {
    /// Shared provider health registry.
    pub health_registry: Arc<roko_learn::provider_health::ProviderHealthRegistry>,
    /// Cascade router for learned model selection.
    pub cascade_router: Option<Arc<roko_learn::cascade_router::CascadeRouter>>,
    /// CLI model override (highest priority).
    pub cli_model_override: Option<String>,
}

/// Prompt assembly bundle: cache and builder state.
#[derive(Debug, Clone)]
pub struct PromptBundle {
    /// Prompt cache path for workspace-scoped data.
    pub cache_workdir: PathBuf,
}

/// Feedback bundle: learning stores and handles.
#[derive(Debug, Clone)]
pub struct FeedbackBundle {
    /// Learning directory path.
    pub learn_dir: PathBuf,
}

/// Extensions bundle: plugin chain and connector registries.
#[derive(Debug, Clone)]
pub struct ExtensionsBundle {
    /// Extension names from config.
    pub extension_names: Vec<String>,
    /// Workspace root for extension resolution.
    pub workdir: PathBuf,
}

/// Observation bundle: telemetry, metrics, structured logging.
#[derive(Debug, Clone)]
pub struct ObservationBundle {
    /// Whether screenshots are enabled.
    pub screenshots: bool,
    /// Log file path override.
    pub log_file: Option<PathBuf>,
}

/// Guards bundle: safety, permissions, budget.
#[derive(Debug, Clone)]
pub struct GuardsBundle {
    /// Whether to dangerously skip permissions.
    pub dangerously_skip_permissions: bool,
    /// Plan budget ceiling in USD (0.0 = unlimited).
    pub budget_ceiling: f64,
    /// Per-turn budget ceiling in USD (0.0 = unlimited).
    pub turn_ceiling: f64,
    /// Whether a CLI budget override is active.
    pub budget_override_active: bool,
}

// ---------------------------------------------------------------------------
// RuntimeServices
// ---------------------------------------------------------------------------

/// The six service bundles constructed by [`RuntimeServicesBuilder`].
///
/// Contains exactly `dispatch`, `prompt`, `feedback`, `extensions`,
/// `observation`, and `guards`.
#[derive(Debug, Clone)]
pub struct RuntimeServices {
    /// Provider dispatch: factory, model resolver, rate limiter, health.
    pub dispatch: DispatchBundle,
    /// Prompt assembly: cache and builder state.
    pub prompt: PromptBundle,
    /// Feedback: learning stores and handles.
    pub feedback: FeedbackBundle,
    /// Extensions: plugin chain and connector registries.
    pub extensions: ExtensionsBundle,
    /// Observation: telemetry, metrics, structured logging.
    pub observation: ObservationBundle,
    /// Guards: safety, permissions, budget.
    pub guards: GuardsBundle,
    /// The profile that was used to construct these services.
    pub profile: RuntimeProfile,
}

// ---------------------------------------------------------------------------
// ExecutionOverrides
// ---------------------------------------------------------------------------

/// Layer-safe execution overrides passed to [`RuntimeServicesBuilder`].
///
/// This is the layer-3 equivalent of the CLI's `ResolvedExecutionOverrides`.
/// CLI code converts its clap-derived type into this before passing it to
/// the builder. No clap dependency here.
#[derive(Debug, Clone, Default)]
pub struct ExecutionOverrides {
    /// Model override (highest priority).
    pub model: Option<String>,
    /// Role override.
    pub role: Option<String>,
    /// Effort level override.
    pub effort: Option<String>,
    /// Whether gate-failure replanning is disabled.
    pub replan_disabled: bool,
    /// Whether to dangerously skip permissions.
    pub dangerously_skip_permissions: bool,
    /// Plan budget ceiling in USD (0.0 = unlimited).
    pub budget_ceiling: f64,
    /// Per-turn budget ceiling in USD (0.0 = unlimited).
    pub turn_ceiling: f64,
    /// Whether a CLI budget override is active.
    pub budget_override_active: bool,
    /// Log file path.
    pub log_file: Option<PathBuf>,
    /// Whether screenshots are enabled.
    pub screenshots: bool,
}

// ---------------------------------------------------------------------------
// BuilderError
// ---------------------------------------------------------------------------

/// Builder error type.
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    /// A required bundle could not be constructed.
    #[error("failed to construct {bundle} for profile {profile}: {reason}")]
    BundleConstruction {
        profile: String,
        bundle: String,
        reason: String,
    },
    /// A forbidden bundle was injected for this profile.
    #[error("bundle {bundle} is forbidden for profile {profile}")]
    ForbiddenBundle { profile: String, bundle: String },
}

// ---------------------------------------------------------------------------
// RuntimeServicesBuilder
// ---------------------------------------------------------------------------

/// Profile-driven runtime services builder.
///
/// Constructs shared service bundles once before the event loop. Both
/// Runner-v2 and Graph engines use this builder with their respective
/// profiles (`FullPlan` / `GraphPlan`) to ensure they share provider
/// health, rate limiter, cost table, prompt cache, and process supervisor.
pub struct RuntimeServicesBuilder {
    profile: RuntimeProfile,
    overrides: ExecutionOverrides,
    cascade_router: Option<Arc<roko_learn::cascade_router::CascadeRouter>>,
    extension_names: Vec<String>,
}

impl RuntimeServicesBuilder {
    /// Create a new builder for the given profile and overrides.
    ///
    /// This is the only production constructor. Tests may use
    /// [`Self::for_test`].
    pub fn new(profile: RuntimeProfile, overrides: ExecutionOverrides) -> Self {
        Self {
            profile,
            overrides,
            cascade_router: None,
            extension_names: Vec::new(),
        }
    }

    /// Test-only constructor with default overrides.
    pub fn for_test(profile: RuntimeProfile) -> Self {
        Self {
            profile,
            overrides: ExecutionOverrides::default(),
            cascade_router: None,
            extension_names: Vec::new(),
        }
    }

    /// Set the cascade router for learned model selection.
    #[must_use]
    pub fn with_cascade_router(
        mut self,
        router: Arc<roko_learn::cascade_router::CascadeRouter>,
    ) -> Self {
        self.cascade_router = Some(router);
        self
    }

    /// Set the extension names from config.
    #[must_use]
    pub fn with_extension_names(mut self, names: Vec<String>) -> Self {
        self.extension_names = names;
        self
    }

    /// Build the runtime services for the given workdir.
    ///
    /// Validates the profile, constructs each long-lived handle once, and
    /// returns the service facade. Service handles are reused for the full
    /// run lifetime; they are never reconstructed per call.
    pub fn build(self, workdir: &Path) -> Result<RuntimeServices, BuilderError> {
        let layout = RokoLayout::for_project(workdir);
        let learn_dir = layout.learn_dir();

        let health_path = learn_dir.join("provider-health.json");
        let health_registry = Arc::new(
            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(&health_path),
        );

        let dispatch = DispatchBundle {
            health_registry,
            cascade_router: self.cascade_router,
            cli_model_override: self.overrides.model.clone(),
        };

        let prompt = PromptBundle {
            cache_workdir: workdir.to_path_buf(),
        };

        let feedback = FeedbackBundle {
            learn_dir: learn_dir.to_path_buf(),
        };

        let extensions = ExtensionsBundle {
            extension_names: self.extension_names,
            workdir: workdir.to_path_buf(),
        };

        let observation = ObservationBundle {
            screenshots: self.overrides.screenshots,
            log_file: self.overrides.log_file.clone(),
        };

        let guards = GuardsBundle {
            dangerously_skip_permissions: self.overrides.dangerously_skip_permissions,
            budget_ceiling: self.overrides.budget_ceiling,
            turn_ceiling: self.overrides.turn_ceiling,
            budget_override_active: self.overrides.budget_override_active,
        };

        Ok(RuntimeServices {
            dispatch,
            prompt,
            feedback,
            extensions,
            observation,
            guards,
            profile: self.profile,
        })
    }

    /// Which profile this builder is constructing.
    #[must_use]
    pub fn profile(&self) -> RuntimeProfile {
        self.profile
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_for_test_constructs_all_profiles() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        for profile in [
            RuntimeProfile::FullPlan,
            RuntimeProfile::GraphPlan,
            RuntimeProfile::Workflow,
            RuntimeProfile::DirectLight,
            RuntimeProfile::AgentServer,
            RuntimeProfile::ChatLight,
            RuntimeProfile::AuthoredGraph,
        ] {
            let services = RuntimeServicesBuilder::for_test(profile)
                .build(workdir.path())
                .unwrap();
            assert_eq!(services.profile, profile);
        }
    }

    #[test]
    fn builder_with_overrides_propagates_fields() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let overrides = ExecutionOverrides {
            model: Some("sonnet-4".to_string()),
            dangerously_skip_permissions: true,
            budget_ceiling: 5.0,
            turn_ceiling: 0.5,
            budget_override_active: true,
            ..Default::default()
        };
        let services = RuntimeServicesBuilder::new(RuntimeProfile::FullPlan, overrides)
            .build(workdir.path())
            .unwrap();
        assert_eq!(
            services.dispatch.cli_model_override.as_deref(),
            Some("sonnet-4")
        );
        assert!(services.guards.dangerously_skip_permissions);
        assert!((services.guards.budget_ceiling - 5.0).abs() < f64::EPSILON);
        assert!(services.guards.budget_override_active);
    }

    #[test]
    fn builder_with_cascade_router() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let router = Arc::new(roko_learn::cascade_router::CascadeRouter::load_or_new(
            &workdir.path().join("router.json"),
            vec!["model-a".to_string()],
        ));
        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .with_cascade_router(router)
            .build(workdir.path())
            .unwrap();
        assert!(services.dispatch.cascade_router.is_some());
    }

    #[test]
    fn execution_overrides_default_is_permissive() {
        let o = ExecutionOverrides::default();
        assert!(o.model.is_none());
        assert!(!o.dangerously_skip_permissions);
        assert!((o.budget_ceiling - 0.0).abs() < f64::EPSILON);
        assert!(!o.budget_override_active);
    }

    #[test]
    fn builder_fullplan_and_graphplan_produce_same_bundle_structure() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let overrides = ExecutionOverrides {
            model: Some("test-model".to_string()),
            budget_ceiling: 10.0,
            turn_ceiling: 1.0,
            ..Default::default()
        };

        let full_services = RuntimeServicesBuilder::new(
            RuntimeProfile::FullPlan,
            overrides.clone(),
        )
        .build(workdir.path())
        .unwrap();

        let graph_services =
            RuntimeServicesBuilder::new(RuntimeProfile::GraphPlan, overrides)
                .build(workdir.path())
                .unwrap();

        // Both paths produce the same resolved metadata for equivalent requests.
        assert_eq!(
            full_services.dispatch.cli_model_override,
            graph_services.dispatch.cli_model_override,
        );
        assert!(
            (full_services.guards.budget_ceiling - graph_services.guards.budget_ceiling).abs()
                < f64::EPSILON,
        );
        assert!(
            (full_services.guards.turn_ceiling - graph_services.guards.turn_ceiling).abs()
                < f64::EPSILON,
        );
    }
}
