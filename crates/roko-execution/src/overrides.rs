//! Layer-safe execution overrides value object.
//!
//! [`ExecutionOverrides`] carries the resolved, typed flags that runtime
//! service construction needs. It is the layer-3 counterpart of the
//! CLI-specific `ResolvedExecutionOverrides` defined in `roko-cli`.
//!
//! # Boundary
//!
//! This type carries only the fields that affect service construction and
//! dispatch behavior -- it does **not** import `Cli`, `PlanRunArgs`, or any
//! clap types. CLI code converts its parsed flags into this object before
//! passing it to [`RuntimeServicesBuilder`](crate::builder::RuntimeServicesBuilder).

use std::num::NonZeroUsize;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Policy enums (layer-safe mirrors of CLI-side policies)
// ---------------------------------------------------------------------------

/// Whether adaptive replanning is enabled for this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplanPolicy {
    /// Use the configured default.
    Default,
    /// Explicitly disabled by the user.
    DisabledByUser,
}

/// Whether cascade (multi-model) routing is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CascadePolicy {
    /// Use the configured cascade routing behavior.
    Default,
    /// Cascade routing explicitly disabled.
    DisabledByUser,
}

/// Dry-run / read-only preview mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DryRunPolicy {
    /// Execute normally.
    Execute,
    /// Preview without mutation.
    ReadOnlyNoMutation,
}

/// How structural validation behaves before execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationPolicy {
    /// Run the full validation pipeline.
    Full,
    /// Skip structure-only validation.
    SkipStructureOnly,
}

/// Unsafe permission override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionConsent {
    /// Normal interactive permission prompts.
    PromptUser,
    /// Skip all permission prompts (dangerously-skip-permissions / yes).
    ExplicitUnsafeConsent,
}

// ---------------------------------------------------------------------------
// Main overrides object
// ---------------------------------------------------------------------------

/// Typed execution overrides used by [`RuntimeServicesBuilder`].
///
/// CLI code converts parsed clap flags into this struct before handing it to
/// the builder. Serve and ACP callers construct it directly from their own
/// request parameters.
///
/// This struct carries only the information that runtime service construction
/// and dispatch behavior need. Presentation (TUI vs text), output formatting
/// (JSON, color), and interaction mode (headless) remain in the CLI layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOverrides {
    // -- Model / provider ------------------------------------------------
    /// Canonical model override (highest-priority operator decision).
    pub model: Option<String>,

    /// Explicit provider override.
    pub provider: Option<String>,

    // -- Execution policies ----------------------------------------------
    /// Replanning policy.
    pub replan: ReplanPolicy,

    /// Structural validation policy.
    pub validation: ValidationPolicy,

    /// Dry-run / preview mode.
    pub dry_run: DryRunPolicy,

    /// Cascade routing policy.
    pub cascade_policy: CascadePolicy,

    // -- Role / effort ---------------------------------------------------
    /// Agent role override.
    pub role: Option<String>,

    /// Reasoning effort level.
    pub effort: Option<String>,

    // -- Plan-run specifics ----------------------------------------------
    /// Maximum retry attempts per task.
    pub max_retries: Option<u32>,

    /// Maximum concurrent tasks (0 = config default).
    pub max_tasks: usize,

    /// Budget override for this run (Some(0.0) = unlimited).
    pub budget_override: Option<f64>,

    /// Re-queue drifted tasks on resume.
    pub force_resume: bool,

    /// Resume from engine state path.
    pub resume_plan: Option<PathBuf>,

    /// Skip disk-space pre-check.
    pub force_disk_check: bool,

    /// Skip preflight environment checks.
    pub skip_preflight: bool,

    /// Unsafe permission override.
    pub permission_consent: PermissionConsent,

    /// Review checkpoint batch size.
    pub batch_size: Option<NonZeroUsize>,
}

impl Default for ExecutionOverrides {
    fn default() -> Self {
        Self {
            model: None,
            provider: None,
            replan: ReplanPolicy::Default,
            validation: ValidationPolicy::Full,
            dry_run: DryRunPolicy::Execute,
            cascade_policy: CascadePolicy::Default,
            role: None,
            effort: None,
            max_retries: None,
            max_tasks: 0,
            budget_override: None,
            force_resume: false,
            resume_plan: None,
            force_disk_check: false,
            skip_preflight: false,
            permission_consent: PermissionConsent::PromptUser,
            batch_size: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_overrides_are_non_destructive() {
        let ov = ExecutionOverrides::default();
        assert_eq!(ov.dry_run, DryRunPolicy::Execute);
        assert_eq!(ov.replan, ReplanPolicy::Default);
        assert_eq!(ov.cascade_policy, CascadePolicy::Default);
        assert_eq!(ov.validation, ValidationPolicy::Full);
        assert_eq!(ov.permission_consent, PermissionConsent::PromptUser);
        assert!(ov.model.is_none());
        assert!(ov.provider.is_none());
        assert!(!ov.skip_preflight);
        assert!(!ov.force_disk_check);
    }

    #[test]
    fn overrides_serialize_roundtrip() {
        let ov = ExecutionOverrides {
            model: Some("claude-sonnet-4-6".into()),
            provider: Some("anthropic-api".into()),
            replan: ReplanPolicy::DisabledByUser,
            dry_run: DryRunPolicy::ReadOnlyNoMutation,
            cascade_policy: CascadePolicy::DisabledByUser,
            max_retries: Some(3),
            max_tasks: 4,
            budget_override: Some(10.0),
            ..ExecutionOverrides::default()
        };
        let json = serde_json::to_string_pretty(&ov).unwrap();
        let deser: ExecutionOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(deser.replan, ReplanPolicy::DisabledByUser);
        assert_eq!(deser.max_retries, Some(3));
    }
}
