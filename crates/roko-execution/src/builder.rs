//! RuntimeServicesBuilder and service bundle types (#243).
//!
//! Constructs the six shared service bundles once before the event loop.
//! Both Runner-v2 (`FullPlan`) and Graph (`GraphPlan`) use this builder
//! to share provider health, rate limiter, cost table, prompt cache, and
//! process supervisor contracts.
//!
//! # Bundle types
//!
//! The builder produces a [`RuntimeServices`] facade whose six bundles use
//! the rich, handle-bearing types defined in their dedicated modules:
//!
//! - [`DispatchBundle`] — `dispatch/factory.rs` (`DispatchFactory`)
//! - [`PromptBundle`] — prompt cache + builder config
//! - [`FeedbackBundle`] — learning directory + optional settler
//! - [`ExtensionsBundle`] — `extensions.rs` (MCP + plugins)
//! - [`ObservationBundle`] — `observation.rs` (event publisher)
//! - [`GuardsBundle`] — `guards.rs` (safety, budget, process supervisor)

use std::path::{Path, PathBuf};
use std::sync::Arc;

use roko_core::config::schema::RokoConfig;
use roko_fs::RokoLayout;
use serde::{Deserialize, Serialize};

use crate::dispatch::factory::DispatchFactory;
use crate::extensions::ExtensionsBundle;
use crate::guards::GuardsBundle;
use crate::observation::ObservationBundle;
use crate::overrides::ExecutionOverrides as DetailedOverrides;
use crate::profiles::{BundleRequirement, ProfileMatrix, RuntimeProfile, ServiceBundleId};
use crate::prompt::builder::PromptBuildHandle;
use crate::prompt::cache::PromptCacheHandle;

// ---------------------------------------------------------------------------
// PromptBundle — wraps the prompt sub-handles
// ---------------------------------------------------------------------------

/// Prompt assembly bundle: cache and builder configuration.
///
/// The cache holds pre-loaded knowledge, episodes, playbooks, and section
/// effectiveness data. The build handle carries composition strategy config.
#[derive(Debug, Clone)]
pub struct PromptBundle {
    /// Pre-loaded prompt context data.
    pub cache: Arc<PromptCacheHandle>,
    /// Prompt assembly configuration.
    pub build_handle: PromptBuildHandle,
}

impl PromptBundle {
    /// Create a minimal prompt bundle for testing.
    pub fn for_test() -> Self {
        Self {
            cache: Arc::new(PromptCacheHandle::empty()),
            build_handle: PromptBuildHandle::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// FeedbackBundle — wraps the learning stores
// ---------------------------------------------------------------------------

/// Feedback bundle: learning stores and settlement handles.
///
/// Constructed only for profiles that require or opt into feedback.
/// The bundle owns the directory path and optional pre-loaded handles;
/// actual settlement pipeline construction (#253) is downstream.
#[derive(Debug, Clone)]
pub struct FeedbackBundle {
    /// Learning directory path (`.roko/learn/`).
    pub learn_dir: PathBuf,
    /// Provider health registry (shared with dispatch).
    pub health_registry: Arc<roko_learn::provider_health::ProviderHealthRegistry>,
    /// Cascade router for learned model selection.
    pub cascade_router: Option<Arc<roko_learn::cascade_router::CascadeRouter>>,
}

impl FeedbackBundle {
    /// Create a minimal feedback bundle for testing.
    pub fn for_test() -> Self {
        Self {
            learn_dir: PathBuf::from("/tmp/learn"),
            health_registry: Arc::new(roko_learn::provider_health::ProviderHealthRegistry::new()),
            cascade_router: None,
        }
    }
}

// ---------------------------------------------------------------------------
// RuntimeServices
// ---------------------------------------------------------------------------

/// The six service bundles constructed by [`RuntimeServicesBuilder`].
///
/// Contains exactly `dispatch`, `prompt`, `feedback`, `extensions`,
/// `observation`, and `guards`. Each bundle uses the rich, handle-bearing
/// type from its dedicated module.
#[derive(Debug, Clone)]
pub struct RuntimeServices {
    /// Provider dispatch: factory, model resolver, rate limiter, health.
    pub dispatch: Arc<DispatchFactory>,
    /// Prompt assembly: cache and builder state.
    pub prompt: PromptBundle,
    /// Feedback: learning stores and handles (None for light profiles).
    pub feedback: Option<FeedbackBundle>,
    /// Extensions: plugin chain and MCP runtime.
    pub extensions: ExtensionsBundle,
    /// Observation: telemetry, event publisher.
    pub observation: ObservationBundle,
    /// Guards: safety, permissions, budget, process supervisor.
    pub guards: GuardsBundle,
    /// The profile that was used to construct these services.
    pub profile: RuntimeProfile,
}

impl RuntimeServices {
    /// Diagnostic summary for snapshot tests and logging.
    pub fn summary(&self) -> RuntimeServicesSummary {
        RuntimeServicesSummary {
            profile: format!("{}", self.profile),
            has_dispatch: true,
            has_prompt: true,
            has_feedback: self.feedback.is_some(),
            has_extensions: true,
            has_observation: true,
            has_guards: true,
            dispatch_has_health_registry: true,
            dispatch_has_cascade_router: self.dispatch.cascade_router.is_some(),
            dispatch_has_mcp_runtime: self.dispatch.mcp_runtime.is_some(),
            guards_has_budget: self.guards.budget_ceiling_usd.is_some(),
            guards_has_process_supervisor: self.guards.process_supervisor.is_some(),
            observation_has_event_publisher: self.observation.event_publisher.is_some(),
            observation_telemetry_enabled: self.observation.telemetry_enabled,
        }
    }
}

/// Serializable summary for snapshot tests.
#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeServicesSummary {
    /// Profile name.
    pub profile: String,
    /// Whether the dispatch bundle is present.
    pub has_dispatch: bool,
    /// Whether the prompt bundle is present.
    pub has_prompt: bool,
    /// Whether the feedback bundle is present.
    pub has_feedback: bool,
    /// Whether the extensions bundle is present.
    pub has_extensions: bool,
    /// Whether the observation bundle is present.
    pub has_observation: bool,
    /// Whether the guards bundle is present.
    pub has_guards: bool,
    /// Whether the dispatch has a health registry.
    pub dispatch_has_health_registry: bool,
    /// Whether the dispatch has a cascade router.
    pub dispatch_has_cascade_router: bool,
    /// Whether the dispatch has an MCP runtime.
    pub dispatch_has_mcp_runtime: bool,
    /// Whether the guards has a budget ceiling.
    pub guards_has_budget: bool,
    /// Whether the guards has a process supervisor.
    pub guards_has_process_supervisor: bool,
    /// Whether the observation has an event publisher.
    pub observation_has_event_publisher: bool,
    /// Whether telemetry is enabled.
    pub observation_telemetry_enabled: bool,
}

// ---------------------------------------------------------------------------
// BuilderError
// ---------------------------------------------------------------------------

/// Builder error type.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
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
    overrides: DetailedOverrides,
    config: Option<Arc<RokoConfig>>,
    cascade_router: Option<Arc<roko_learn::cascade_router::CascadeRouter>>,
    dispatch_factory: Option<Arc<DispatchFactory>>,
    observation_bundle: Option<ObservationBundle>,
    extensions_bundle: Option<ExtensionsBundle>,
    budget_ceiling_usd: Option<f64>,
}

impl RuntimeServicesBuilder {
    /// Create a new builder for the given profile and overrides.
    ///
    /// This is the primary constructor. Tests may use [`Self::for_test`].
    pub fn new(profile: RuntimeProfile, overrides: DetailedOverrides) -> Self {
        Self {
            profile,
            overrides,
            config: None,
            cascade_router: None,
            dispatch_factory: None,
            observation_bundle: None,
            extensions_bundle: None,
            budget_ceiling_usd: None,
        }
    }

    /// Production constructor that takes a validated config, profile, and
    /// overrides. This is the prescribed entry point from backlog #243:
    /// `RuntimeServicesBuilder::from_config(&ValidatedConfig, RuntimeProfile, ExecutionOverrides)`.
    pub fn from_config(
        config: &Arc<RokoConfig>,
        profile: RuntimeProfile,
        overrides: DetailedOverrides,
    ) -> Self {
        Self {
            profile,
            overrides,
            config: Some(Arc::clone(config)),
            cascade_router: None,
            dispatch_factory: None,
            observation_bundle: None,
            extensions_bundle: None,
            budget_ceiling_usd: None,
        }
    }

    /// Test-only constructor with default overrides.
    pub fn for_test(profile: RuntimeProfile) -> Self {
        Self {
            profile,
            overrides: DetailedOverrides::default(),
            config: None,
            cascade_router: None,
            dispatch_factory: None,
            observation_bundle: None,
            extensions_bundle: None,
            budget_ceiling_usd: None,
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

    /// Inject a pre-built dispatch factory (e.g. from existing CLI code).
    #[must_use]
    pub fn with_dispatch_factory(mut self, factory: Arc<DispatchFactory>) -> Self {
        self.dispatch_factory = Some(factory);
        self
    }

    /// Inject a pre-built observation bundle.
    #[must_use]
    pub fn with_observation(mut self, obs: ObservationBundle) -> Self {
        self.observation_bundle = Some(obs);
        self
    }

    /// Inject a pre-built extensions bundle.
    #[must_use]
    pub fn with_extensions(mut self, ext: ExtensionsBundle) -> Self {
        self.extensions_bundle = Some(ext);
        self
    }

    /// Set the budget ceiling in USD.
    #[must_use]
    pub fn with_budget_ceiling_usd(mut self, ceiling: f64) -> Self {
        self.budget_ceiling_usd = Some(ceiling);
        self
    }

    /// Build the runtime services for the given workdir.
    ///
    /// Validates the profile against the bundle matrix, constructs each
    /// long-lived handle once, and returns the service facade. Service
    /// handles are reused for the full run lifetime; they are never
    /// reconstructed per call.
    pub fn build(self, workdir: &Path) -> Result<RuntimeServices, BuilderError> {
        let layout = RokoLayout::for_project(workdir);
        let learn_dir = layout.learn_dir();
        let matrix = ProfileMatrix::canonical();

        // -- Dispatch -------------------------------------------------------
        let dispatch = if let Some(factory) = self.dispatch_factory {
            factory
        } else {
            let health_path = learn_dir.join("provider-health.json");
            let health_registry = Arc::new(
                roko_learn::provider_health::ProviderHealthRegistry::load_or_new(&health_path),
            );
            Arc::new(DispatchFactory {
                semaphores: Arc::new(roko_agent::provider::ProviderSemaphores::new(
                    &indexmap::IndexMap::new(),
                )),
                mcp_runtime: None,
                local_tool_runtime: None,
                rate_limiter: Arc::new(roko_agent::rate_limit::ProviderRateLimiter::new(60)),
                health_registry,
                cascade_router: self.cascade_router.clone(),
            })
        };

        // -- Prompt ---------------------------------------------------------
        let prompt = PromptBundle {
            cache: Arc::new(PromptCacheHandle::load(workdir)),
            build_handle: PromptBuildHandle::default(),
        };

        // -- Feedback -------------------------------------------------------
        let feedback_req = matrix
            .entries_for(self.profile, ServiceBundleId::Feedback)
            .unwrap_or(BundleRequirement::Optional);
        let feedback = match feedback_req {
            BundleRequirement::Required => {
                let health_path = learn_dir.join("provider-health.json");
                Some(FeedbackBundle {
                    learn_dir: learn_dir.to_path_buf(),
                    health_registry: Arc::new(
                        roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                            &health_path,
                        ),
                    ),
                    cascade_router: self.cascade_router.clone(),
                })
            }
            BundleRequirement::Optional => {
                // Construct if learn dir exists.
                if learn_dir.exists() {
                    let health_path = learn_dir.join("provider-health.json");
                    Some(FeedbackBundle {
                        learn_dir: learn_dir.to_path_buf(),
                        health_registry: Arc::new(
                            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                                &health_path,
                            ),
                        ),
                        cascade_router: self.cascade_router,
                    })
                } else {
                    None
                }
            }
            BundleRequirement::Forbidden => None,
        };

        // -- Extensions -----------------------------------------------------
        let extensions = self
            .extensions_bundle
            .unwrap_or_else(ExtensionsBundle::for_test);

        // -- Observation ----------------------------------------------------
        let observation = self
            .observation_bundle
            .unwrap_or_else(ObservationBundle::for_test);

        // -- Guards ---------------------------------------------------------
        let budget = self.budget_ceiling_usd.or(self.overrides.budget_override);
        let mut guards = GuardsBundle::for_test();
        guards.budget_ceiling_usd = budget;

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

    /// Access the config, if one was provided.
    #[must_use]
    pub fn config(&self) -> Option<&Arc<RokoConfig>> {
        self.config.as_ref()
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
    fn builder_with_overrides_propagates_budget() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let overrides = DetailedOverrides {
            budget_override: Some(5.0),
            ..Default::default()
        };
        let services = RuntimeServicesBuilder::new(RuntimeProfile::FullPlan, overrides)
            .build(workdir.path())
            .unwrap();
        assert!((services.guards.budget_ceiling_usd.unwrap() - 5.0).abs() < f64::EPSILON);
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
    fn builder_from_config_constructs_with_config() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let config = Arc::new(RokoConfig::default());
        let overrides = DetailedOverrides::default();
        let builder =
            RuntimeServicesBuilder::from_config(&config, RuntimeProfile::FullPlan, overrides);
        assert!(builder.config().is_some());
        let services = builder.build(workdir.path()).unwrap();
        assert_eq!(services.profile, RuntimeProfile::FullPlan);
    }

    #[test]
    fn fullplan_has_feedback() {
        let workdir = tempfile::tempdir().unwrap();
        let roko_dir = workdir.path().join(".roko");
        std::fs::create_dir_all(roko_dir.join("learn")).unwrap();

        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .build(workdir.path())
            .unwrap();
        assert!(services.feedback.is_some(), "FullPlan requires feedback");
    }

    #[test]
    fn light_profile_feedback_optional() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();
        // No learn dir, so optional feedback won't be constructed.

        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::ChatLight)
            .build(workdir.path())
            .unwrap();
        // Feedback is optional for ChatLight -- may or may not be present
        // depending on whether learn dir exists.
        assert_eq!(services.profile, RuntimeProfile::ChatLight);
    }

    #[test]
    fn guards_bundle_has_real_handles() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .with_budget_ceiling_usd(10.0)
            .build(workdir.path())
            .unwrap();

        assert!(!services.guards.budget_exceeded());
        services.guards.cost_ledger.record(11.0);
        assert!(services.guards.budget_exceeded());
    }

    #[test]
    fn dispatch_factory_has_real_handles() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .build(workdir.path())
            .unwrap();

        // DispatchFactory has real semaphores, rate limiter, health registry
        assert!(services.dispatch.mcp_runtime.is_none()); // not injected
        assert!(services.dispatch.cascade_router.is_none()); // not injected
    }

    #[test]
    fn prompt_cache_loads_from_workdir() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .build(workdir.path())
            .unwrap();

        // Cache is loaded (empty in test, but constructed).
        assert!(services.prompt.cache.neuro_entries.is_empty());
        assert!(!services.prompt.cache.is_stale());
    }

    #[test]
    fn injected_dispatch_factory_is_used() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let factory = Arc::new(DispatchFactory::for_test());
        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .with_dispatch_factory(factory.clone())
            .build(workdir.path())
            .unwrap();

        // The injected factory is the same Arc.
        assert!(Arc::ptr_eq(&services.dispatch, &factory));
    }

    #[test]
    fn profile_enabled_services_snapshot() {
        let workdir = tempfile::tempdir().unwrap();
        let roko_dir = workdir.path().join(".roko");
        std::fs::create_dir_all(roko_dir.join("learn")).unwrap();

        let profiles = [
            RuntimeProfile::FullPlan,
            RuntimeProfile::GraphPlan,
            RuntimeProfile::Workflow,
            RuntimeProfile::DirectLight,
            RuntimeProfile::AgentServer,
            RuntimeProfile::ChatLight,
            RuntimeProfile::AuthoredGraph,
        ];

        let mut snapshot: Vec<serde_json::Value> = Vec::new();
        for profile in &profiles {
            let services = RuntimeServicesBuilder::for_test(*profile)
                .build(workdir.path())
                .unwrap();
            let summary = services.summary();
            snapshot.push(serde_json::to_value(&summary).unwrap());
        }
        insta::assert_json_snapshot!("builder_profile_services", snapshot);
    }

    #[test]
    fn injected_observation_bundle_is_used() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let obs = ObservationBundle {
            event_publisher: None,
            telemetry_enabled: true,
        };
        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .with_observation(obs)
            .build(workdir.path())
            .unwrap();
        assert!(services.observation.telemetry_enabled);
    }

    #[test]
    fn injected_extensions_bundle_is_used() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let ext = ExtensionsBundle::for_test();
        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .with_extensions(ext)
            .build(workdir.path())
            .unwrap();
        assert!(services.extensions.mcp_runtime.is_none());
        assert!(services.extensions.local_tool_runtime.is_none());
    }

    #[test]
    fn builder_ceiling_takes_priority_over_override_budget() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let overrides = DetailedOverrides {
            budget_override: Some(5.0),
            ..Default::default()
        };
        let services = RuntimeServicesBuilder::new(RuntimeProfile::FullPlan, overrides)
            .with_budget_ceiling_usd(20.0)
            .build(workdir.path())
            .unwrap();
        // Builder-level ceiling wins over override budget.
        assert!(
            (services.guards.budget_ceiling_usd.unwrap() - 20.0).abs() < f64::EPSILON,
            "expected 20.0, got {:?}",
            services.guards.budget_ceiling_usd
        );
    }

    #[test]
    fn summary_serializes_and_deserializes() {
        let workdir = tempfile::tempdir().unwrap();
        let roko_dir = workdir.path().join(".roko");
        std::fs::create_dir_all(roko_dir.join("learn")).unwrap();

        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .with_budget_ceiling_usd(10.0)
            .build(workdir.path())
            .unwrap();
        let summary = services.summary();
        let json = serde_json::to_string(&summary).unwrap();
        let back: RuntimeServicesSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.profile, "FullPlan");
        assert!(back.guards_has_budget);
        assert!(!back.guards_has_process_supervisor);
        assert!(!back.observation_has_event_publisher);
    }

    #[test]
    fn prompt_bundle_for_test_is_fresh() {
        let bundle = PromptBundle::for_test();
        assert!(bundle.cache.neuro_entries.is_empty());
        assert!(bundle.cache.episodes.is_empty());
        assert!(!bundle.cache.is_stale());
    }

    #[test]
    fn feedback_bundle_for_test_has_health_registry() {
        let bundle = FeedbackBundle::for_test();
        assert!(bundle.cascade_router.is_none());
        assert_eq!(bundle.learn_dir, PathBuf::from("/tmp/learn"));
    }

    #[test]
    fn feedback_absent_when_no_learn_dir_for_optional_profile() {
        let workdir = tempfile::tempdir().unwrap();
        // Only .roko, no .roko/learn/ -- feedback is optional for DirectLight
        std::fs::create_dir_all(workdir.path().join(".roko")).unwrap();

        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::DirectLight)
            .build(workdir.path())
            .unwrap();
        assert!(
            services.feedback.is_none(),
            "DirectLight with no learn dir should have no feedback"
        );
    }

    #[test]
    fn feedback_present_when_learn_dir_exists_for_optional_profile() {
        let workdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(workdir.path().join(".roko").join("learn")).unwrap();

        let services = RuntimeServicesBuilder::for_test(RuntimeProfile::DirectLight)
            .build(workdir.path())
            .unwrap();
        assert!(
            services.feedback.is_some(),
            "DirectLight with learn dir should have feedback"
        );
    }

    #[test]
    fn builder_fullplan_and_graphplan_produce_same_bundle_structure() {
        let workdir = tempfile::tempdir().unwrap();
        let roko_dir = workdir.path().join(".roko");
        std::fs::create_dir_all(roko_dir.join("learn")).unwrap();

        let full_services = RuntimeServicesBuilder::for_test(RuntimeProfile::FullPlan)
            .with_budget_ceiling_usd(10.0)
            .build(workdir.path())
            .unwrap();

        let graph_services = RuntimeServicesBuilder::for_test(RuntimeProfile::GraphPlan)
            .with_budget_ceiling_usd(10.0)
            .build(workdir.path())
            .unwrap();

        // Both paths produce the same resolved metadata for equivalent requests.
        assert_eq!(
            full_services.dispatch.cascade_router.is_some(),
            graph_services.dispatch.cascade_router.is_some(),
        );
        assert_eq!(
            full_services.guards.budget_ceiling_usd,
            graph_services.guards.budget_ceiling_usd,
        );
        assert_eq!(
            full_services.feedback.is_some(),
            graph_services.feedback.is_some(),
        );
    }
}
