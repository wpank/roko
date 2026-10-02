//! Profile-driven runtime service builder for Roko execution surfaces (#243).
//!
//! This crate provides [`RuntimeServicesBuilder`] which constructs shared
//! service bundles (dispatch, prompt, feedback, extensions, observation,
//! guards) from a validated config and a [`RuntimeProfile`]. Both Runner-v2
//! and Graph plan engines consume the same builder so that provider health,
//! rate limiter, cost table, prompt cache, and shutdown/process supervisor
//! contracts are shared.
//!
//! # Layer
//!
//! This crate is layer 3. It may depend on layer 0-3 crates but must never
//! depend on `roko-cli`, `roko-serve`, or `roko-acp` (all layer 4).
//!
//! # Modules
//!
//! - [`builder`] -- RuntimeServicesBuilder and service bundle types.
//! - [`diagnostics`] -- Shared diagnostic and preflight service.
//! - [`dispatch`] -- Layer-3 dispatch factory, model resolver, and request types.
//! - [`extensions`] -- Extensions bundle for MCP and plugin runtimes.
//! - [`feedback`] -- Feedback receipt and settlement pipeline.
//! - [`guards`] -- Guards bundle for safety, budget, and process supervision.
//! - [`lifecycle`] -- Runner lifecycle event types for envelope publication.
//! - [`observation`] -- Observation bundle for event publication.
//! - [`overrides`] -- Layer-safe ExecutionOverrides value object with policy enums.
//! - [`profiles`] -- RuntimeProfile enum and profile bundle matrix.
//! - [`prompt`] -- Layer-3 prompt assembly handles and cache.
//! - [`authored_graph`] -- AuthoredGraph controller lifecycle and config (#267).
//! - [`replan_controller`] -- Durable Graph gate-failure replan controller.
//! - [`runtime_services`] -- Non-plan service construction for workflow/chat/ACP.
//! - [`plan_generator`] -- Plan-generation value types (#280).

pub mod authored_graph;
pub mod builder;
pub mod diagnostics;
pub mod dispatch;
pub mod extensions;
pub mod feedback;
pub mod guards;
pub mod lifecycle;
pub mod observation;
pub mod overrides;
pub mod plan_generator;
pub mod profiles;
pub mod prompt;
pub mod replan_controller;
pub mod runtime_services;

// ---- Builder-level re-exports ------------------------------------------------

pub use builder::{
    BuilderError, FeedbackBundle, PromptBundle, RuntimeServices, RuntimeServicesBuilder,
    RuntimeServicesSummary,
};
pub use lifecycle::RunnerLifecycleEvent;
pub use profiles::{ProfileBundleManifest, RuntimeProfile, profile_bundle_manifest};

// ---- Profile matrix re-exports -----------------------------------------------

pub use profiles::{BundleRequirement, ProfileMatrix, ServiceBundleId};

// ---- Non-plan service re-exports ---------------------------------------------

pub use runtime_services::{
    CostSettlement, CostSettlementError, NonPlanServiceHandle, NonPlanServiceRequest,
    ServiceConstructionError, ShutdownRegistration, build_non_plan_services, overrides_for_acp,
    overrides_for_chat, overrides_for_workflow, validate_service_request,
};

// ---- Module-level bundle re-exports ------------------------------------------

pub use dispatch::factory::DispatchFactory;
pub use dispatch::model_resolver::ModelResolverHandle;
pub use dispatch::request::DispatchRequest;
pub use extensions::ExtensionsBundle;
pub use guards::{CostLedger, GuardsBundle};
pub use observation::{ObservationBundle, ObservationPublisher};
pub use overrides::ExecutionOverrides as DetailedExecutionOverrides;
pub use plan_generator::{
    PlanGenError, PlanGeneratorAdapter, PlanGeneratorOutcome, PlanGeneratorOverrides,
    PlanGeneratorRequest, PlanSource, ValidatedPlan, ValidationEvidence,
};
pub use prompt::builder::PromptBuildHandle;
pub use prompt::cache::PromptCacheHandle;

// ---- Authored graph controller re-exports (#267) ----------------------------

pub use authored_graph::{
    AuthoredGraphConfig, AuthoredGraphController, AuthoredGraphReport, ControllerError,
    ControllerLifecycle, PreflightCategory, PreflightError, drive_controller,
};
