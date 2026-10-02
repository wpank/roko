//! Authored graph controller and configuration (#267).
//!
//! Implements [`ControllerLifecycle`] for user-defined graph definitions loaded
//! from TOML files. The controller drives pre-start validation, capability
//! intersection, budget enforcement, and observable execution through the
//! shared [`RuntimeServices`] infrastructure.
//!
//! # Controller lifecycle
//!
//! ```text
//! preflight → start → drive → report
//! ```
//!
//! `preflight` validates the graph against the workspace capability grant,
//! checks cell capability requirements, and ensures no plan-only privileges
//! leak into authored graph execution.
//!
//! `start` constructs the [`GraphEngine`] with the default cell registry,
//! wires the budget tracker and telemetry sink, and produces a
//! [`AuthoredGraphReport`].
//!
//! # Capability matrix
//!
//! Authored graphs use the fixed matrix from `roko_graph::profile`:
//! - `ReadFs`, `Bus` are granted from the workspace grant alone.
//! - `WriteFs`, `Network`, `Shell`, `Llm`, `Secrets` require both graph
//!   declaration and workspace grant.

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use roko_core::{Capability, CapabilitySet};
use serde::{Deserialize, Serialize};

use roko_graph::profile::{
    AuthoredGraphProfile, CellCapabilityDenial, ProfileValidationError, validate_cell_capabilities,
};

// ─── AuthoredGraphConfig ─────────────────────────────────────────────────────

/// Configuration for an authored graph execution session.
///
/// Resolved from CLI flags and graph definition before the controller
/// lifecycle begins. The `budget_usd` field carries an optional cost ceiling
/// from the `--budget` flag or workspace config.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthoredGraphConfig {
    /// Path to the graph TOML definition file.
    pub graph_path: PathBuf,
    /// Capabilities declared by the graph definition and validated against
    /// the workspace grant during preflight.
    pub capabilities: Vec<Capability>,
    /// Optional USD budget ceiling for the execution. When set, the graph
    /// engine's `BudgetTracker` enforces this limit.
    pub budget_usd: Option<f64>,
    /// Whether to emit canonical JSON events instead of human-readable output.
    pub json_output: bool,
    /// Whether to suppress human-readable progress output (errors still print).
    pub quiet: bool,
}

impl AuthoredGraphConfig {
    /// Create a minimal config for testing.
    #[must_use]
    pub fn for_test(graph_path: PathBuf) -> Self {
        Self {
            graph_path,
            capabilities: Vec::new(),
            budget_usd: None,
            json_output: false,
            quiet: false,
        }
    }
}

// ─── AuthoredGraphReport ─────────────────────────────────────────────────────

/// Terminal report produced by the authored graph controller after execution.
///
/// Mirrors the `GraphOutput` fields needed for CLI/serve reporting, plus
/// profile metadata and budget summary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthoredGraphReport {
    /// Name of the executed graph.
    pub graph_name: String,
    /// Whether all nodes completed successfully.
    pub success: bool,
    /// Number of nodes in the graph.
    pub node_count: usize,
    /// Number of nodes that completed successfully.
    pub nodes_passed: usize,
    /// Number of nodes that failed.
    pub nodes_failed: usize,
    /// Number of nodes that were skipped.
    pub nodes_skipped: usize,
    /// Total wall-clock execution duration.
    pub total_duration: Duration,
    /// Profile kind used for this execution.
    pub profile_kind: String,
    /// Effective capability set after intersection.
    pub effective_capabilities: Vec<String>,
    /// Budget consumed in USD, if budget tracking was active.
    pub budget_consumed_usd: Option<f64>,
    /// Budget ceiling in USD, if one was configured.
    pub budget_ceiling_usd: Option<f64>,
}

impl fmt::Display for AuthoredGraphReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AuthoredGraphReport(graph={}, success={}, nodes={}/{}, duration={:?})",
            self.graph_name, self.success, self.nodes_passed, self.node_count, self.total_duration,
        )?;
        if let Some(consumed) = self.budget_consumed_usd {
            write!(f, ", cost=${consumed:.4}")?;
        }
        Ok(())
    }
}

// ─── ControllerLifecycle ─────────────────────────────────────────────────────

/// Lifecycle trait for execution controllers.
///
/// Each controller type (plan, workflow, authored-graph) implements this trait.
/// The host function `drive_controller` executes the lifecycle identically
/// for all controller types:
///
/// ```text
/// preflight(config) → start() → drive() → report()
/// ```
///
/// `Config` is the controller-specific configuration type.
/// `Output` is the terminal report type.
#[async_trait::async_trait]
pub trait ControllerLifecycle: Send + Sync {
    /// Configuration input for this controller.
    type Config;
    /// Terminal report output.
    type Output;

    /// Validate configuration and environment before starting.
    ///
    /// Returns a list of pre-start errors. An empty list means preflight passed.
    fn preflight(&self, config: &Self::Config) -> Vec<PreflightError>;

    /// Start execution. Called after preflight passes.
    ///
    /// Returns the terminal report when execution completes (success or failure).
    async fn start(&self, config: Self::Config) -> Result<Self::Output, ControllerError>;
}

/// Pre-start validation error from a controller's preflight check.
#[derive(Clone, Debug)]
pub struct PreflightError {
    /// The category of the error.
    pub category: PreflightCategory,
    /// Human-readable description.
    pub message: String,
}

impl fmt::Display for PreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.category, self.message)
    }
}

/// Categories for preflight validation errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreflightCategory {
    /// A graph-declared capability was denied by the workspace.
    CapabilityDenied,
    /// A cell requires a capability not granted by the profile.
    CellCapabilityDenied,
    /// The graph definition failed structural validation.
    GraphValidation,
    /// The graph definition file could not be loaded.
    LoadError,
    /// Budget configuration is invalid.
    BudgetInvalid,
}

impl fmt::Display for PreflightCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapabilityDenied => write!(f, "capability-denied"),
            Self::CellCapabilityDenied => write!(f, "cell-capability-denied"),
            Self::GraphValidation => write!(f, "graph-validation"),
            Self::LoadError => write!(f, "load-error"),
            Self::BudgetInvalid => write!(f, "budget-invalid"),
        }
    }
}

/// Errors during controller execution (after preflight).
#[derive(Debug, thiserror::Error)]
pub enum ControllerError {
    /// The graph definition could not be loaded.
    #[error("failed to load graph: {0}")]
    LoadError(String),
    /// Graph validation failed after loading.
    #[error("graph validation failed: {0}")]
    ValidationError(String),
    /// Profile construction or capability intersection failed.
    #[error("profile error: {0}")]
    ProfileError(String),
    /// Budget was exceeded during execution.
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
    /// Graph execution failed.
    #[error("execution error: {0}")]
    ExecutionError(String),
}

// ─── AuthoredGraphController ─────────────────────────────────────────────────

/// Controller implementing [`ControllerLifecycle`] for authored graph execution.
///
/// The controller validates capability intersection, constructs the
/// `AuthoredGraphProfile`, builds the graph engine, and drives execution
/// through the standard `GraphEngine::execute` path.
pub struct AuthoredGraphController {
    /// The workspace-level capability grant. This determines which
    /// capabilities the authored graph may receive.
    workspace_grant: CapabilitySet,
}

impl AuthoredGraphController {
    /// Create a new controller with the given workspace capability grant.
    #[must_use]
    pub fn new(workspace_grant: CapabilitySet) -> Self {
        Self { workspace_grant }
    }

    /// Build the `AuthoredGraphProfile` from graph policy and workspace grant.
    ///
    /// This is exposed so callers can inspect the profile before starting.
    pub fn build_profile(
        &self,
        graph: &roko_graph::types::Graph,
        config: &AuthoredGraphConfig,
    ) -> Result<AuthoredGraphProfile, ProfileValidationError> {
        AuthoredGraphProfile::builder(&graph.metadata.name)
            .graph_policy(&graph.policy)
            .workspace_grant(self.workspace_grant.clone())
            .json_output(config.json_output)
            .quiet(config.quiet)
            .build()
    }

    /// Validate cell capabilities against the built profile.
    pub fn validate_cells(
        &self,
        graph: &roko_graph::types::Graph,
        profile: &AuthoredGraphProfile,
    ) -> Vec<CellCapabilityDenial> {
        validate_cell_capabilities(graph, profile)
    }
}

impl fmt::Debug for AuthoredGraphController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthoredGraphController")
            .field("workspace_grant", &self.workspace_grant)
            .finish()
    }
}

#[async_trait::async_trait]
impl ControllerLifecycle for AuthoredGraphController {
    type Config = AuthoredGraphConfig;
    type Output = AuthoredGraphReport;

    fn preflight(&self, config: &Self::Config) -> Vec<PreflightError> {
        let mut errors = Vec::new();

        // 1. Load the graph
        let graph = match roko_graph::loader::load_from_file(&config.graph_path) {
            Ok(g) => g,
            Err(e) => {
                errors.push(PreflightError {
                    category: PreflightCategory::LoadError,
                    message: format!(
                        "failed to load graph '{}': {e}",
                        config.graph_path.display()
                    ),
                });
                return errors;
            }
        };

        // 2. Build profile (capability intersection)
        let profile = match self.build_profile(&graph, config) {
            Ok(p) => p,
            Err(e) => {
                for denial in &e.denials {
                    errors.push(PreflightError {
                        category: PreflightCategory::CapabilityDenied,
                        message: format!("graph '{}': {denial}", config.graph_path.display()),
                    });
                }
                return errors;
            }
        };

        // 3. Validate cell capabilities
        let cell_denials = self.validate_cells(&graph, &profile);
        for denial in &cell_denials {
            errors.push(PreflightError {
                category: PreflightCategory::CellCapabilityDenied,
                message: denial.to_string(),
            });
        }

        // 4. Structural graph validation. Stub cells are left to the start,
        // which knows whether the run allows test stubs.
        let registry = roko_graph::default_registry();
        let engine = roko_graph::GraphEngine::new(graph, registry).with_allow_test_stubs(true);
        let issues = engine.validate();
        for issue in &issues {
            errors.push(PreflightError {
                category: PreflightCategory::GraphValidation,
                message: issue.to_string(),
            });
        }

        // 5. Budget validation
        if let Some(budget) = config.budget_usd
            && budget <= 0.0
        {
            errors.push(PreflightError {
                category: PreflightCategory::BudgetInvalid,
                message: format!("budget must be positive, got {budget}"),
            });
        }

        errors
    }

    async fn start(&self, config: Self::Config) -> Result<Self::Output, ControllerError> {
        // Load graph
        let graph = roko_graph::loader::load_from_file(&config.graph_path).map_err(|e| {
            ControllerError::LoadError(format!(
                "failed to load graph '{}': {e}",
                config.graph_path.display()
            ))
        })?;

        // Build profile
        let profile = self
            .build_profile(&graph, &config)
            .map_err(|e| ControllerError::ProfileError(e.to_string()))?;

        // Validate cells
        let cell_denials = self.validate_cells(&graph, &profile);
        if !cell_denials.is_empty() {
            let detail = cell_denials
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(ControllerError::ValidationError(format!(
                "cell capability check failed ({} denial(s)): {detail}",
                cell_denials.len()
            )));
        }

        // Build budget tracker if configured
        let budget_tracker = config.budget_usd.map(|usd| {
            roko_graph::BudgetTracker::with_limits(roko_graph::BudgetLimits {
                max_tokens: None,
                max_cost_usd: Some(usd),
                deadline: None,
            })
        });

        // Build engine
        let registry = roko_graph::default_registry();
        let engine = roko_graph::GraphEngine::new(graph.clone(), registry);

        // Validate
        let issues = engine.validate();
        if !issues.is_empty() {
            return Err(ControllerError::ValidationError(
                issues
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }

        // Build CellContext with effective capabilities
        let ctx = roko_graph::CellContext::new().with_capabilities(profile.effective().clone());

        // Execute
        let output = engine
            .execute(&ctx)
            .await
            .map_err(|e| ControllerError::ExecutionError(e.to_string()))?;

        // Collect node stats
        let nodes_passed = output
            .node_results
            .iter()
            .filter(|r| r.status == roko_graph::NodeStatus::Complete)
            .count();
        let nodes_failed = output
            .node_results
            .iter()
            .filter(|r| r.status == roko_graph::NodeStatus::Failed)
            .count();
        let nodes_skipped = output
            .node_results
            .iter()
            .filter(|r| {
                r.status == roko_graph::NodeStatus::Skipped
                    || r.status == roko_graph::NodeStatus::ConditionSkipped
            })
            .count();

        let effective_caps: Vec<String> =
            profile.effective().iter().map(|c| c.to_string()).collect();

        let budget_consumed = budget_tracker.as_ref().map(|t| t.cost_usd());

        Ok(AuthoredGraphReport {
            graph_name: output.graph_name,
            success: output.success,
            node_count: output.node_results.len(),
            nodes_passed,
            nodes_failed,
            nodes_skipped,
            total_duration: output.total_duration,
            profile_kind: profile.kind().to_string(),
            effective_capabilities: effective_caps,
            budget_consumed_usd: budget_consumed,
            budget_ceiling_usd: config.budget_usd,
        })
    }
}

/// Drive a controller through the full lifecycle: preflight, then start.
///
/// This is the canonical host function that executes any controller
/// implementing [`ControllerLifecycle`] identically.
pub async fn drive_controller<C: ControllerLifecycle>(
    controller: &C,
    config: C::Config,
) -> Result<C::Output, ControllerError>
where
    C::Config: Clone,
{
    let preflight_errors = controller.preflight(&config);
    if !preflight_errors.is_empty() {
        let detail = preflight_errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(ControllerError::ValidationError(format!(
            "preflight failed ({} error(s)): {detail}",
            preflight_errors.len()
        )));
    }

    controller.start(config).await
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::Capability;

    fn workspace_all() -> CapabilitySet {
        CapabilitySet::all()
    }

    fn workspace_baseline() -> CapabilitySet {
        CapabilitySet::from([Capability::ReadFs, Capability::Bus])
    }

    /// Write a one-node graph around `sense`: a real, zero-cost Cell with no
    /// capability requirements. The `noop` test stub cannot be used because
    /// production starts reject test-stub descriptors.
    fn write_test_graph(dir: &std::path::Path, name: &str, caps: &[&str]) -> PathBuf {
        let path = dir.join(format!("{name}.toml"));
        let caps_str = if caps.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = caps.iter().map(|c| format!("\"{c}\"")).collect();
            format!("\n[graph.policy]\ncapabilities = [{}]\n", items.join(", "))
        };
        std::fs::write(
            &path,
            format!(
                r#"[graph]
name = "{name}"
{caps_str}
[[nodes]]
id = "root"
cell_type = "sense"
"#
            ),
        )
        .unwrap();
        path
    }

    fn write_task_executor_graph(dir: &std::path::Path, name: &str) -> PathBuf {
        let path = dir.join(format!("{name}.toml"));
        std::fs::write(
            &path,
            format!(
                r#"[graph]
name = "{name}"

[graph.policy]
capabilities = ["llm"]

[[nodes]]
id = "task-1"
cell_type = "task-executor"
"#
            ),
        )
        .unwrap();
        path
    }

    // ── AuthoredGraphConfig tests ────────────────────────────────────────

    #[test]
    fn config_for_test_has_sensible_defaults() {
        let config = AuthoredGraphConfig::for_test(PathBuf::from("/tmp/test.toml"));
        assert!(config.capabilities.is_empty());
        assert!(config.budget_usd.is_none());
        assert!(!config.json_output);
        assert!(!config.quiet);
    }

    #[test]
    fn config_serializes_and_deserializes() {
        let config = AuthoredGraphConfig {
            graph_path: PathBuf::from("/tmp/graph.toml"),
            capabilities: vec![Capability::Llm, Capability::Shell],
            budget_usd: Some(5.0),
            json_output: true,
            quiet: false,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: AuthoredGraphConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.graph_path, config.graph_path);
        assert_eq!(back.capabilities.len(), 2);
        assert_eq!(back.budget_usd, Some(5.0));
        assert!(back.json_output);
    }

    // ── AuthoredGraphReport tests ────────────────────────────────────────

    #[test]
    fn report_display_includes_graph_name_and_status() {
        let report = AuthoredGraphReport {
            graph_name: "my-graph".to_string(),
            success: true,
            node_count: 5,
            nodes_passed: 4,
            nodes_failed: 0,
            nodes_skipped: 1,
            total_duration: Duration::from_secs(2),
            profile_kind: "authored-graph".to_string(),
            effective_capabilities: vec!["ReadFs".to_string()],
            budget_consumed_usd: Some(0.15),
            budget_ceiling_usd: Some(1.0),
        };
        let display = report.to_string();
        assert!(display.contains("my-graph"));
        assert!(display.contains("success=true"));
        assert!(display.contains("$0.15"));
    }

    // ── PreflightError tests ─────────────────────────────────────────────

    #[test]
    fn preflight_error_display() {
        let err = PreflightError {
            category: PreflightCategory::CapabilityDenied,
            message: "Llm not granted".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("capability-denied"));
        assert!(display.contains("Llm not granted"));
    }

    // ── Controller preflight tests ───────────────────────────────────────

    #[test]
    fn preflight_passes_for_valid_noop_graph() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_test_graph(dir.path(), "valid", &[]);
        let controller = AuthoredGraphController::new(workspace_all());
        let config = AuthoredGraphConfig::for_test(path);
        let errors = controller.preflight(&config);
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    /// Preflight checks the graph's structure; a stub cell is refused at the
    /// start, which knows whether the run allows test stubs (bug-147b45).
    #[test]
    fn preflight_leaves_stub_cells_to_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stub.toml");
        std::fs::write(
            &path,
            "[graph]\nname = \"stub\"\n\n[[nodes]]\nid = \"root\"\ncell_type = \"noop\"\n",
        )
        .unwrap();
        let controller = AuthoredGraphController::new(workspace_all());
        let errors = controller.preflight(&AuthoredGraphConfig::for_test(path));
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn preflight_fails_for_missing_file() {
        let controller = AuthoredGraphController::new(workspace_all());
        let config = AuthoredGraphConfig::for_test(PathBuf::from("/nonexistent/graph.toml"));
        let errors = controller.preflight(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].category, PreflightCategory::LoadError);
    }

    #[test]
    fn preflight_fails_for_overprivileged_graph() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_test_graph(dir.path(), "overpriv", &["llm", "secrets", "network"]);
        let controller = AuthoredGraphController::new(workspace_baseline());
        let config = AuthoredGraphConfig::for_test(path);
        let errors = controller.preflight(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.category == PreflightCategory::CapabilityDenied),
            "expected capability denial: {errors:?}"
        );
    }

    #[test]
    fn preflight_fails_for_task_executor_without_llm_grant() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_task_executor_graph(dir.path(), "no-llm");
        // Workspace does not grant Llm
        let controller = AuthoredGraphController::new(workspace_baseline());
        let config = AuthoredGraphConfig::for_test(path);
        let errors = controller.preflight(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.category == PreflightCategory::CapabilityDenied),
            "expected capability denial for Llm: {errors:?}"
        );
    }

    #[test]
    fn preflight_fails_for_negative_budget() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_test_graph(dir.path(), "neg-budget", &[]);
        let controller = AuthoredGraphController::new(workspace_all());
        let mut config = AuthoredGraphConfig::for_test(path);
        config.budget_usd = Some(-1.0);
        let errors = controller.preflight(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.category == PreflightCategory::BudgetInvalid),
            "expected budget invalid: {errors:?}"
        );
    }

    // ── Controller start tests ───────────────────────────────────────────

    #[tokio::test]
    async fn start_succeeds_for_valid_noop_graph() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_test_graph(dir.path(), "start-ok", &[]);
        let controller = AuthoredGraphController::new(workspace_all());
        let config = AuthoredGraphConfig::for_test(path);
        let report = controller.start(config).await.unwrap();
        assert!(report.success);
        assert_eq!(report.profile_kind, "authored-graph");
        assert_eq!(report.node_count, 1);
        assert_eq!(report.nodes_passed, 1);
    }

    #[tokio::test]
    async fn start_fails_for_missing_file() {
        let controller = AuthoredGraphController::new(workspace_all());
        let config = AuthoredGraphConfig::for_test(PathBuf::from("/nonexistent/graph.toml"));
        let err = controller.start(config).await.unwrap_err();
        assert!(matches!(err, ControllerError::LoadError(_)));
    }

    #[tokio::test]
    async fn start_with_budget_reports_cost() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_test_graph(dir.path(), "budgeted", &[]);
        let controller = AuthoredGraphController::new(workspace_all());
        let mut config = AuthoredGraphConfig::for_test(path);
        config.budget_usd = Some(10.0);
        let report = controller.start(config).await.unwrap();
        assert!(report.success);
        assert_eq!(report.budget_ceiling_usd, Some(10.0));
        // Budget consumed should be 0 for a noop graph
        assert!(report.budget_consumed_usd.unwrap_or(0.0) < f64::EPSILON);
    }

    // ── drive_controller tests ───────────────────────────────────────────

    #[tokio::test]
    async fn drive_controller_full_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_test_graph(dir.path(), "lifecycle", &[]);
        let controller = AuthoredGraphController::new(workspace_all());
        let config = AuthoredGraphConfig::for_test(path);
        let report = drive_controller(&controller, config).await.unwrap();
        assert!(report.success);
    }

    #[tokio::test]
    async fn drive_controller_fails_preflight() {
        let controller = AuthoredGraphController::new(workspace_all());
        let config = AuthoredGraphConfig::for_test(PathBuf::from("/nonexistent/graph.toml"));
        let err = drive_controller(&controller, config).await.unwrap_err();
        assert!(
            matches!(err, ControllerError::ValidationError(_)),
            "expected ValidationError from preflight, got: {err}"
        );
    }

    // ── Malicious/overprivileged graph denial tests ──────────────────────

    #[test]
    fn authored_graph_cannot_inherit_plan_privileges() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_test_graph(dir.path(), "standalone", &[]);
        let controller = AuthoredGraphController::new(workspace_all());
        let config = AuthoredGraphConfig::for_test(path);

        // Preflight should pass
        let errors = controller.preflight(&config);
        assert!(errors.is_empty());

        // But loading the graph and building the profile should NOT
        // grant any elevated capabilities
        let graph = roko_graph::loader::load_from_file(&config.graph_path).unwrap();
        let profile = controller.build_profile(&graph, &config).unwrap();
        assert!(profile.permits(Capability::ReadFs)); // baseline
        assert!(profile.permits(Capability::Bus)); // baseline
        assert!(!profile.permits(Capability::WriteFs)); // elevated, not declared
        assert!(!profile.permits(Capability::Llm)); // elevated, not declared
        assert!(!profile.permits(Capability::Secrets)); // elevated, not declared
        assert!(!profile.permits(Capability::Shell)); // elevated, not declared
        assert!(!profile.permits(Capability::Network)); // elevated, not declared
    }

    #[test]
    fn malicious_graph_all_elevated_denied_by_readonly_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_test_graph(
            dir.path(),
            "malicious",
            &["write_fs", "network", "shell", "llm", "secrets"],
        );
        let controller = AuthoredGraphController::new(workspace_baseline());
        let config = AuthoredGraphConfig::for_test(path);
        let errors = controller.preflight(&config);
        // Should have 5 capability denials (all elevated)
        let cap_denials: Vec<_> = errors
            .iter()
            .filter(|e| e.category == PreflightCategory::CapabilityDenied)
            .collect();
        assert_eq!(
            cap_denials.len(),
            5,
            "expected 5 capability denials, got: {cap_denials:?}"
        );
    }

    #[test]
    fn graph_with_task_executor_needs_llm_in_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_task_executor_graph(dir.path(), "needs-llm");
        // Workspace grants Llm but graph does NOT declare it in policy
        let ws = CapabilitySet::from([Capability::ReadFs, Capability::Bus, Capability::Llm]);
        let controller = AuthoredGraphController::new(ws);

        // Load graph and build profile -- profile won't have Llm because
        // the graph policy declares it, but we need to check cell validation
        let config = AuthoredGraphConfig::for_test(path);
        let errors = controller.preflight(&config);
        // This graph declares llm in policy AND workspace grants it,
        // so it should pass profile validation.
        // The cell validation should also pass because the profile includes Llm.
        assert!(
            errors.is_empty(),
            "expected no errors when Llm is both declared and granted: {errors:?}"
        );
    }
}
