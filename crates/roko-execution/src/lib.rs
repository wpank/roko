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
//! - [`replan_controller`] -- Durable Graph gate-failure replan controller.
//! - [`runtime_services`] -- Non-plan service construction for workflow/chat/ACP.
//! - [`workflow`] -- Workflow graph cells and templates.

pub mod builder;
pub mod diagnostics;
pub mod dispatch;
pub mod extensions;
pub mod feedback;
pub mod guards;
pub mod lifecycle;
pub mod observation;
pub mod overrides;
pub mod profiles;
pub mod prompt;
pub mod replan_controller;
pub mod runtime_services;
pub mod workflow;

// ---- Builder-level re-exports ------------------------------------------------

pub use builder::{
    BuilderError, DispatchBundle, ExecutionOverrides, ExtensionsBundle, FeedbackBundle,
    GuardsBundle, ObservationBundle, PromptBundle, RuntimeServices, RuntimeServicesBuilder,
};
pub use lifecycle::RunnerLifecycleEvent;
pub use profiles::{ProfileBundleManifest, RuntimeProfile, profile_bundle_manifest};

// ---- Profile matrix re-exports -----------------------------------------------

pub use profiles::{BundleRequirement, ProfileMatrix, ServiceBundleId};

// ---- Non-plan service re-exports ---------------------------------------------

pub use runtime_services::{
    CostSettlement, CostSettlementError, NonPlanServiceHandle, NonPlanServiceRequest,
    ServiceConstructionError, ShutdownRegistration, build_non_plan_services,
    overrides_for_acp, overrides_for_chat, overrides_for_workflow, validate_service_request,
};

// ---- Workflow re-exports -----------------------------------------------------

pub use workflow::{
    ActivityScope, ControllerAction, PhaseInput, PhaseReceipt, ReviewVerdict,
    WorkflowGraphController, WorkflowPhase, WorkflowTemplateDescriptor, WorkflowTermination,
    build_report, idempotency_key, parse_review, resolve_template,
};

// ---- Detailed service bundle re-exports --------------------------------------

pub use dispatch::factory::DispatchFactory;
pub use dispatch::model_resolver::ModelResolverHandle;
pub use dispatch::request::DispatchRequest;
pub use guards::CostLedger;
pub use observation::ObservationPublisher;
pub use overrides::ExecutionOverrides as DetailedExecutionOverrides;
pub use prompt::builder::PromptBuildHandle;
pub use prompt::cache::PromptCacheHandle;
