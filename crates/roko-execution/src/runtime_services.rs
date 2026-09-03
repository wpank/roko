//! Runtime service facade for non-plan execution paths.
//!
//! This module defines the consumer-side contract that [`crate::profiles::RuntimeProfile::Workflow`],
//! [`crate::profiles::RuntimeProfile::ChatLight`], and ACP session paths will use to obtain
//! pre-built service handles. The actual builder lives in `roko-serve`'s `ServiceFactory`
//! today; backlog #243 will migrate construction into a profile-driven
//! `RuntimeServicesBuilder` in this crate.
//!
//! # Design
//!
//! Each non-plan surface creates an [`ExecutionOverrides`] at its host boundary and
//! passes it to the profile-specific constructor. The constructor validates the
//! override against the profile matrix (from [`crate::profiles::ProfileMatrix`]),
//! builds required bundles, and returns a typed service handle that the caller
//! stores for the session lifetime.
//!
//! Neither lane edits `commands/plan.rs`, `runner/event_loop.rs`, or any plan-path
//! type. API mismatches are reported as `SPEC_DRIFT` rather than repaired by
//! inventing a lane-local service.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::profiles::{ProfileMatrix, RuntimeProfile, ServiceBundleId};

// ---------------------------------------------------------------------------
// Execution overrides — host-boundary inputs for non-plan surfaces
// ---------------------------------------------------------------------------

/// Typed overrides resolved at the CLI/ACP/serve host boundary.
///
/// Each non-plan surface constructs this at its entry point before requesting
/// services. The fields map 1:1 to resolved CLI flags, ACP session params, or
/// serve request headers.
///
/// This struct does NOT carry raw CLI/ACP request types across the
/// `roko-execution` boundary — hosts translate their native types into this
/// shared representation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionOverrides {
    /// Explicit model key or slug override (e.g. `--model sonnet`).
    pub model: Option<String>,
    /// Explicit provider key override (e.g. `--provider anthropic_api`).
    pub provider: Option<String>,
    /// Explicit role override (e.g. `--role implementer`).
    pub role: Option<String>,
    /// Whether cascade routing is enabled.
    pub cascade_enabled: Option<bool>,
    /// Whether feedback recording should be active.
    pub feedback_enabled: Option<bool>,
    /// Whether affect modulation should be active.
    pub affect_enabled: Option<bool>,
    /// Explicit MCP config path override.
    pub mcp_config: Option<PathBuf>,
    /// Explicit run/session identifier.
    pub run_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Non-plan service request
// ---------------------------------------------------------------------------

/// Request to build runtime services for a non-plan execution surface.
///
/// Combines the target profile, workspace location, and host-resolved
/// overrides into a single validated construction request.
#[derive(Debug, Clone)]
pub struct NonPlanServiceRequest {
    /// Which execution profile this request targets.
    pub profile: RuntimeProfile,
    /// Workspace root directory.
    pub workdir: PathBuf,
    /// `.roko` directory (typically `workdir.join(".roko")`).
    pub roko_dir: PathBuf,
    /// Host-boundary overrides resolved from CLI flags or ACP params.
    pub overrides: ExecutionOverrides,
}

impl NonPlanServiceRequest {
    /// Construct a request with the standard `.roko` directory.
    pub fn new(profile: RuntimeProfile, workdir: PathBuf, overrides: ExecutionOverrides) -> Self {
        let roko_dir = workdir.join(".roko");
        Self {
            profile,
            workdir,
            roko_dir,
            overrides,
        }
    }

    /// Validate that the requested profile is appropriate for a non-plan
    /// surface. Returns an error if the profile is `FullPlan` or `GraphPlan`.
    pub fn validate_non_plan(&self) -> Result<(), ServiceConstructionError> {
        match self.profile {
            RuntimeProfile::FullPlan | RuntimeProfile::GraphPlan => {
                Err(ServiceConstructionError::ProfileMismatch {
                    profile: self.profile,
                    reason: format!(
                        "{} is a plan profile; use the plan runner instead",
                        self.profile
                    ),
                })
            }
            _ => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Service handle — the returned facade
// ---------------------------------------------------------------------------

/// Opaque service handle returned by the runtime services facade.
///
/// This is the consumer-side handle that callers store for the session
/// lifetime. It carries the profile that was used to construct it, the
/// validated bundle set, and the original overrides for introspection.
///
/// When #243 lands, this will wrap the concrete `ServiceBundle` from
/// `roko-serve::ServiceFactory`. Until then, callers use it as a
/// validation-only type that proves the profile matrix was satisfied.
#[derive(Debug, Clone)]
pub struct NonPlanServiceHandle {
    /// Profile used to construct this handle.
    profile: RuntimeProfile,
    /// Bundle IDs that were required and resolved.
    required_bundles: Vec<ServiceBundleId>,
    /// Bundle IDs that were optional and resolved (if available).
    optional_bundles: Vec<ServiceBundleId>,
    /// Original overrides for introspection / logging.
    overrides: ExecutionOverrides,
    /// Stable identifier for this service instance.
    instance_id: String,
}

impl NonPlanServiceHandle {
    /// The profile this handle was constructed for.
    #[must_use]
    pub fn profile(&self) -> RuntimeProfile {
        self.profile
    }

    /// The set of required bundles that were resolved.
    #[must_use]
    pub fn required_bundles(&self) -> &[ServiceBundleId] {
        &self.required_bundles
    }

    /// The set of optional bundles that were resolved.
    #[must_use]
    pub fn optional_bundles(&self) -> &[ServiceBundleId] {
        &self.optional_bundles
    }

    /// The overrides that were applied.
    #[must_use]
    pub fn overrides(&self) -> &ExecutionOverrides {
        &self.overrides
    }

    /// Stable identifier for correlating cost settlement and process
    /// registration with this service instance.
    #[must_use]
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Whether feedback recording is active for this instance.
    #[must_use]
    pub fn feedback_enabled(&self) -> bool {
        self.overrides.feedback_enabled.unwrap_or(true)
    }

    /// Whether cascade routing is active for this instance.
    #[must_use]
    pub fn cascade_enabled(&self) -> bool {
        self.overrides.cascade_enabled.unwrap_or(true)
    }
}

impl fmt::Display for NonPlanServiceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NonPlanServiceHandle({}, id={}, required={}, optional={})",
            self.profile,
            self.instance_id,
            self.required_bundles.len(),
            self.optional_bundles.len(),
        )
    }
}

// ---------------------------------------------------------------------------
// Construction errors
// ---------------------------------------------------------------------------

/// Errors during non-plan service construction.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ServiceConstructionError {
    /// The requested profile is not valid for a non-plan surface.
    #[error("profile mismatch: {profile} -- {reason}")]
    ProfileMismatch {
        profile: RuntimeProfile,
        reason: String,
    },
    /// A required bundle could not be constructed.
    #[error("required bundle {bundle} failed for profile {profile}: {reason}")]
    RequiredBundleMissing {
        profile: RuntimeProfile,
        bundle: ServiceBundleId,
        reason: String,
    },
    /// Configuration is missing or invalid for the requested profile.
    #[error("configuration error for profile {profile}: {reason}")]
    ConfigError {
        profile: RuntimeProfile,
        reason: String,
    },
    /// A forbidden bundle was injected (internal invariant violation).
    #[error("forbidden bundle {bundle} injected for profile {profile}")]
    ForbiddenBundleInjected {
        profile: RuntimeProfile,
        bundle: ServiceBundleId,
    },
    /// The #243 `RuntimeServicesBuilder` is not yet available.
    ///
    /// This is returned by `build_non_plan_services` until the builder is
    /// implemented. Callers should fall back to their current `ServiceFactory`
    /// construction path.
    #[error("SPEC_DRIFT: RuntimeServicesBuilder not yet available (blocked on #243); \
             profile={profile}, use ServiceFactory::build as interim")]
    BuilderNotAvailable { profile: RuntimeProfile },
}

// ---------------------------------------------------------------------------
// Validation — can be used today without the builder
// ---------------------------------------------------------------------------

/// Validate that a non-plan service request satisfies the profile matrix.
///
/// This is the consumer-side validation that callers can use today. It
/// checks that the profile is valid for a non-plan surface and that all
/// required bundles are declared in the profile matrix.
///
/// Returns the set of required and optional bundle IDs for the profile.
pub fn validate_service_request(
    request: &NonPlanServiceRequest,
) -> Result<NonPlanServiceHandle, ServiceConstructionError> {
    request.validate_non_plan()?;

    let matrix = ProfileMatrix::canonical();
    let required: Vec<ServiceBundleId> = matrix
        .required_bundles(request.profile)
        .into_iter()
        .collect();
    let optional: Vec<ServiceBundleId> = matrix
        .optional_bundles(request.profile)
        .into_iter()
        .collect();

    let instance_id = request
        .overrides
        .run_id
        .clone()
        .unwrap_or_else(|| format!("{}_{}", request.profile, chrono_millis()));

    Ok(NonPlanServiceHandle {
        profile: request.profile,
        required_bundles: required,
        optional_bundles: optional,
        overrides: request.overrides.clone(),
        instance_id,
    })
}

/// Attempt to build runtime services for a non-plan surface.
///
/// **Status: blocked on #243.** Returns `Err(BuilderNotAvailable)` until
/// the `RuntimeServicesBuilder` is implemented. Callers should:
///
/// 1. Call `validate_service_request()` to get a validated handle.
/// 2. Fall back to `ServiceFactory::build(ServiceConfig { ... })` for the
///    actual bundle construction.
/// 3. Store the handle for cost settlement and process registration
///    correlation.
///
/// When #243 lands, this function will call `RuntimeServicesBuilder::build()`
/// directly and return the concrete `ServiceBundle`.
pub fn build_non_plan_services(
    request: &NonPlanServiceRequest,
) -> Result<NonPlanServiceHandle, ServiceConstructionError> {
    // Phase 1: validate the request against the profile matrix.
    let handle = validate_service_request(request)?;

    // Phase 2: actual construction — blocked on #243.
    // TODO(#243): Replace with RuntimeServicesBuilder::build(request).
    tracing::debug!(
        profile = %request.profile,
        instance_id = %handle.instance_id,
        required = ?handle.required_bundles,
        "validated non-plan service request (builder pending #243)"
    );

    Ok(handle)
}

/// Millisecond timestamp for generating unique instance IDs.
fn chrono_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Host-boundary adapters — per-surface conversion helpers
// ---------------------------------------------------------------------------

/// Build [`ExecutionOverrides`] from workflow (run.rs) CLI flags.
///
/// Maps the `CliOverrides` struct used by `run_with_workflow_engine` to
/// the shared `ExecutionOverrides` type.
pub fn overrides_for_workflow(
    model: Option<String>,
    role: Option<String>,
    provider: Option<String>,
    cascade_enabled: Option<bool>,
    mcp_config: Option<PathBuf>,
) -> ExecutionOverrides {
    ExecutionOverrides {
        model,
        provider,
        role,
        cascade_enabled,
        feedback_enabled: Some(true),
        affect_enabled: Some(true),
        mcp_config,
        run_id: None,
    }
}

/// Build [`ExecutionOverrides`] from chat session parameters.
///
/// Chat sessions enable feedback but disable affect modulation by default.
/// Cascade routing follows the workspace default unless overridden.
pub fn overrides_for_chat(
    model: Option<String>,
    provider: Option<String>,
) -> ExecutionOverrides {
    ExecutionOverrides {
        model,
        provider,
        role: None,
        cascade_enabled: None,
        feedback_enabled: Some(true),
        affect_enabled: Some(false),
        mcp_config: None,
        run_id: None,
    }
}

/// Build [`ExecutionOverrides`] from ACP session parameters.
///
/// ACP sessions enable feedback and cascade routing, disable affect modulation,
/// and carry a session-scoped run ID.
pub fn overrides_for_acp(
    session_id: &str,
    model_key: Option<String>,
    mcp_config: Option<PathBuf>,
) -> ExecutionOverrides {
    ExecutionOverrides {
        model: model_key,
        provider: None,
        role: None,
        cascade_enabled: Some(true),
        feedback_enabled: Some(true),
        affect_enabled: Some(false),
        mcp_config,
        run_id: Some(format!("acp_workflow_{session_id}")),
    }
}

// ---------------------------------------------------------------------------
// Cost settlement contract
// ---------------------------------------------------------------------------

/// Marker trait for types that settle cost exactly once per provider call.
///
/// Each non-plan surface must implement this on its feedback adapter to
/// satisfy the shared checklist requirement: "every completed provider call
/// settles actual usage/cost exactly once."
///
/// The trait is object-safe and async to accommodate both synchronous file
/// writers and async HTTP reporters.
#[async_trait::async_trait]
pub trait CostSettlement: Send + Sync {
    /// Settle the cost for one completed provider call.
    ///
    /// Implementations must be idempotent: calling settle twice with the
    /// same `request_id` must not double-count.
    async fn settle_cost(
        &self,
        request_id: &str,
        model: &str,
        provider: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        latency_ms: u64,
    ) -> Result<(), CostSettlementError>;

    /// Flush any buffered settlements to durable storage.
    async fn flush(&self) -> Result<(), CostSettlementError>;
}

/// Errors from cost settlement.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CostSettlementError {
    /// The settlement was rejected as a duplicate.
    #[error("duplicate settlement for request_id={request_id}")]
    Duplicate { request_id: String },

    /// I/O or persistence error.
    #[error("settlement persistence failed: {reason}")]
    PersistenceError { reason: String },
}

// ---------------------------------------------------------------------------
// Shutdown registration contract
// ---------------------------------------------------------------------------

/// Marker trait for types that register processes with the shared shutdown
/// supervisor.
///
/// Each non-plan surface must register spawned agent processes so that
/// `GracefulShutdown` can terminate them on SIGTERM / Ctrl-C.
pub trait ShutdownRegistration: Send + Sync {
    /// Register a process with the shutdown supervisor.
    fn register_process(&self, pid: u32, label: &str);

    /// Deregister a process after it exits normally.
    fn deregister_process(&self, pid: u32);
}

// ---------------------------------------------------------------------------
// Arc-wrapped type aliases for ergonomic consumption
// ---------------------------------------------------------------------------

/// Shared cost settlement handle.
pub type SharedCostSettlement = Arc<dyn CostSettlement>;

/// Shared shutdown registration handle.
pub type SharedShutdownRegistration = Arc<dyn ShutdownRegistration>;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_request_validates() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::Workflow,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        assert!(request.validate_non_plan().is_ok());
    }

    #[test]
    fn chat_light_request_validates() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::ChatLight,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        assert!(request.validate_non_plan().is_ok());
    }

    #[test]
    fn direct_light_request_validates() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::DirectLight,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        assert!(request.validate_non_plan().is_ok());
    }

    #[test]
    fn agent_server_request_validates() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::AgentServer,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        assert!(request.validate_non_plan().is_ok());
    }

    #[test]
    fn full_plan_request_rejected() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::FullPlan,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        assert!(request.validate_non_plan().is_err());
    }

    #[test]
    fn graph_plan_request_rejected() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::GraphPlan,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        assert!(request.validate_non_plan().is_err());
    }

    #[test]
    fn validate_returns_handle_with_correct_profile() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::Workflow,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides {
                model: Some("sonnet".to_string()),
                ..Default::default()
            },
        );
        let handle = validate_service_request(&request).unwrap();
        assert_eq!(handle.profile(), RuntimeProfile::Workflow);
        assert_eq!(handle.overrides().model.as_deref(), Some("sonnet"));
        assert!(!handle.required_bundles().is_empty());
    }

    #[test]
    fn handle_display_includes_profile() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::ChatLight,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        let handle = validate_service_request(&request).unwrap();
        let display = format!("{handle}");
        assert!(display.contains("chat_light"), "display: {display}");
    }

    #[test]
    fn workflow_overrides_enables_affect() {
        let overrides = overrides_for_workflow(None, None, None, None, None);
        assert_eq!(overrides.affect_enabled, Some(true));
        assert_eq!(overrides.feedback_enabled, Some(true));
    }

    #[test]
    fn chat_overrides_disables_affect() {
        let overrides = overrides_for_chat(None, None);
        assert_eq!(overrides.affect_enabled, Some(false));
        assert_eq!(overrides.feedback_enabled, Some(true));
    }

    #[test]
    fn acp_overrides_carries_session_id() {
        let overrides = overrides_for_acp("session-123", None, None);
        assert_eq!(
            overrides.run_id.as_deref(),
            Some("acp_workflow_session-123")
        );
        assert_eq!(overrides.cascade_enabled, Some(true));
        assert_eq!(overrides.affect_enabled, Some(false));
    }

    #[test]
    fn build_non_plan_services_returns_handle() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::Workflow,
            PathBuf::from("/tmp/test"),
            overrides_for_workflow(
                Some("sonnet".to_string()),
                None,
                None,
                Some(true),
                None,
            ),
        );
        let handle = build_non_plan_services(&request).unwrap();
        assert_eq!(handle.profile(), RuntimeProfile::Workflow);
        assert!(handle.cascade_enabled());
        assert!(handle.feedback_enabled());
    }

    #[test]
    fn build_rejects_plan_profiles() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::FullPlan,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        let err = build_non_plan_services(&request).unwrap_err();
        assert!(
            matches!(err, ServiceConstructionError::ProfileMismatch { .. }),
            "expected ProfileMismatch, got: {err}"
        );
    }

    #[test]
    fn instance_id_uses_run_id_when_provided() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::Workflow,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides {
                run_id: Some("my-run-123".to_string()),
                ..Default::default()
            },
        );
        let handle = validate_service_request(&request).unwrap();
        assert_eq!(handle.instance_id(), "my-run-123");
    }

    #[test]
    fn instance_id_generated_when_not_provided() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::ChatLight,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        let handle = validate_service_request(&request).unwrap();
        assert!(
            handle.instance_id().starts_with("chat_light_"),
            "expected auto-generated ID starting with 'chat_light_', got: {}",
            handle.instance_id()
        );
    }

    #[test]
    fn workflow_profile_requires_dispatch_and_prompt() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::Workflow,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        let handle = validate_service_request(&request).unwrap();
        assert!(
            handle
                .required_bundles()
                .contains(&ServiceBundleId::Dispatch),
            "workflow must require dispatch"
        );
        assert!(
            handle.required_bundles().contains(&ServiceBundleId::Prompt),
            "workflow must require prompt"
        );
    }

    #[test]
    fn authored_graph_request_validates() {
        let request = NonPlanServiceRequest::new(
            RuntimeProfile::AuthoredGraph,
            PathBuf::from("/tmp/test"),
            ExecutionOverrides::default(),
        );
        assert!(request.validate_non_plan().is_ok());
    }
}
