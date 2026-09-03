//! Authored graph production runtime profile (#267).
//!
//! Provides [`AuthoredGraphProfile`] and the fixed capability matrix for
//! `roko graph run`. This module lives in `roko-graph` (the #243 profile
//! module), not in `commands/graph.rs`, so the validation and enforcement
//! logic is reusable across CLI, serve, and trigger execution paths.
//!
//! # Capability matrix
//!
//! | Subcommand     | Granted capabilities                    |
//! |----------------|-----------------------------------------|
//! | `validate`     | None (load + validate only)             |
//! | `show`         | None (load + display only)              |
//! | `run`          | `ReadFs`, `Bus` from workspace config;  |
//! |                | `WriteFs`, `Network`, `Shell`, `Llm`,   |
//! |                | `Secrets` require **both** a graph      |
//! |                | declaration and the workspace grant     |
//!
//! Cells that need capabilities beyond what the profile grants are rejected
//! at pre-start validation time, not at dispatch time.

use std::fmt;

use roko_core::{Capability, CapabilitySet};
use serde::{Deserialize, Serialize};

use crate::types::GraphPolicy;

// ─── Baseline capabilities ──────────────────────────────────────────────────

/// Capabilities that a workspace may grant to any graph run without requiring
/// an explicit graph declaration. These are the "normal workspace" authorities.
const WORKSPACE_BASELINE: &[Capability] = &[Capability::ReadFs, Capability::Bus];

/// Capabilities that require **both** a graph policy declaration **and** a
/// workspace grant. A graph cannot receive these from the workspace alone.
const ELEVATED: &[Capability] = &[
    Capability::WriteFs,
    Capability::Network,
    Capability::Shell,
    Capability::Llm,
    Capability::Secrets,
];

// ─── RuntimeProfileKind ─────────────────────────────────────────────────────

/// Discriminant for the kind of runtime profile governing a graph execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProfileKind {
    /// An authored (standalone) graph loaded from a TOML definition file.
    /// Does **not** inherit plan or workspace privileges automatically.
    AuthoredGraph,
    /// A graph produced by converting a plan DAG. Inherits plan-scoped
    /// privileges from the runner-v2 session. (Placeholder for #285.)
    FullPlan,
}

impl fmt::Display for RuntimeProfileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthoredGraph => write!(f, "authored-graph"),
            Self::FullPlan => write!(f, "full-plan"),
        }
    }
}

// ─── CapabilityDenial ───────────────────────────────────────────────────────

/// A single capability that the graph requested but was denied by the
/// workspace grant or the fixed matrix rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityDenial {
    /// The capability that was requested.
    pub capability: Capability,
    /// Why it was denied.
    pub reason: DenialReason,
}

/// Reason a capability was denied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenialReason {
    /// The graph declared the capability but the workspace did not grant it.
    WorkspaceNotGranted,
    /// The capability is elevated and the graph did not declare it.
    NotDeclaredByGraph,
}

impl fmt::Display for CapabilityDenial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.reason {
            DenialReason::WorkspaceNotGranted => {
                write!(
                    f,
                    "{}: declared by graph but not granted by workspace",
                    self.capability
                )
            }
            DenialReason::NotDeclaredByGraph => {
                write!(
                    f,
                    "{}: requires explicit graph declaration (elevated capability)",
                    self.capability
                )
            }
        }
    }
}

// ─── AuthoredGraphProfile ───────────────────────────────────────────────────

/// Resolved production runtime profile for an authored graph.
///
/// Constructed via [`AuthoredGraphProfileBuilder`], which validates the
/// graph-declared capabilities against the workspace grant and produces a
/// pre-start error if any requested capability is denied.
#[derive(Clone, Debug)]
pub struct AuthoredGraphProfile {
    /// The runtime profile kind (always `AuthoredGraph`).
    kind: RuntimeProfileKind,
    /// Graph identifier for diagnostics.
    graph_id: String,
    /// The effective capability set after intersection.
    effective: CapabilitySet,
    /// Whether JSON output mode was requested.
    json_output: bool,
    /// Whether quiet mode was requested (suppress human progress, not errors).
    quiet: bool,
}

impl AuthoredGraphProfile {
    /// Start building a new profile.
    #[must_use]
    pub fn builder(graph_id: impl Into<String>) -> AuthoredGraphProfileBuilder {
        AuthoredGraphProfileBuilder {
            graph_id: graph_id.into(),
            graph_policy: None,
            workspace_grant: CapabilitySet::empty(),
            json_output: false,
            quiet: false,
        }
    }

    /// The runtime profile kind.
    #[must_use]
    pub const fn kind(&self) -> RuntimeProfileKind {
        self.kind
    }

    /// The graph identifier.
    #[must_use]
    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    /// The effective capability set for this execution.
    #[must_use]
    pub const fn effective(&self) -> &CapabilitySet {
        &self.effective
    }

    /// Whether JSON output mode is active.
    #[must_use]
    pub const fn json_output(&self) -> bool {
        self.json_output
    }

    /// Whether quiet mode is active (suppress human progress, not errors).
    #[must_use]
    pub const fn quiet(&self) -> bool {
        self.quiet
    }

    /// Return `true` if the effective set permits the given capability.
    #[must_use]
    pub fn permits(&self, cap: Capability) -> bool {
        self.effective.has(cap)
    }
}

// ─── Builder ────────────────────────────────────────────────────────────────

/// Builder for [`AuthoredGraphProfile`].
#[derive(Clone, Debug)]
pub struct AuthoredGraphProfileBuilder {
    graph_id: String,
    graph_policy: Option<GraphPolicy>,
    workspace_grant: CapabilitySet,
    json_output: bool,
    quiet: bool,
}

impl AuthoredGraphProfileBuilder {
    /// Set the graph policy (which carries the declared capabilities).
    #[must_use]
    pub fn graph_policy(mut self, policy: &GraphPolicy) -> Self {
        self.graph_policy = Some(policy.clone());
        self
    }

    /// Set the workspace-level capability grant.
    #[must_use]
    pub fn workspace_grant(mut self, grant: CapabilitySet) -> Self {
        self.workspace_grant = grant;
        self
    }

    /// Enable JSON output mode.
    #[must_use]
    pub fn json_output(mut self, enabled: bool) -> Self {
        self.json_output = enabled;
        self
    }

    /// Enable quiet mode.
    #[must_use]
    pub fn quiet(mut self, enabled: bool) -> Self {
        self.quiet = enabled;
        self
    }

    /// Build the profile, validating capabilities against the workspace grant.
    ///
    /// # Errors
    ///
    /// Returns the list of denied capabilities if any graph-declared capability
    /// is not granted by the workspace, or if a graph tries to use elevated
    /// capabilities without declaring them.
    pub fn build(self) -> Result<AuthoredGraphProfile, ProfileValidationError> {
        let graph_declared: CapabilitySet = self
            .graph_policy
            .as_ref()
            .map(|p| CapabilitySet::from(p.capabilities.iter().copied()))
            .unwrap_or_else(CapabilitySet::empty);

        let mut denials = Vec::new();

        // Check: every graph-declared capability must be present in the
        // workspace grant, otherwise the workspace operator has not authorized
        // it.
        for cap in graph_declared.iter() {
            if !self.workspace_grant.has(*cap) {
                denials.push(CapabilityDenial {
                    capability: *cap,
                    reason: DenialReason::WorkspaceNotGranted,
                });
            }
        }

        if !denials.is_empty() {
            return Err(ProfileValidationError {
                graph_id: self.graph_id,
                denials,
            });
        }

        // Compute the effective set using the fixed matrix:
        // 1. Baseline capabilities (ReadFs, Bus) are granted if the workspace
        //    permits them, regardless of whether the graph declared them.
        // 2. Elevated capabilities (WriteFs, Network, Shell, Llm, Secrets)
        //    require BOTH graph declaration AND workspace grant.
        let mut effective = CapabilitySet::empty();

        // Grant baseline capabilities from workspace
        for &cap in WORKSPACE_BASELINE {
            if self.workspace_grant.has(cap) {
                effective = CapabilitySet::from(effective.iter().copied().chain(Some(cap)));
            }
        }

        // Grant elevated capabilities only when both declared and granted
        for &cap in ELEVATED {
            if graph_declared.has(cap) && self.workspace_grant.has(cap) {
                effective = CapabilitySet::from(effective.iter().copied().chain(Some(cap)));
            }
        }

        Ok(AuthoredGraphProfile {
            kind: RuntimeProfileKind::AuthoredGraph,
            graph_id: self.graph_id,
            effective,
            json_output: self.json_output,
            quiet: self.quiet,
        })
    }
}

// ─── ProfileValidationError ─────────────────────────────────────────────────

/// Pre-start error indicating the graph requested capabilities that are
/// not authorized by the workspace.
#[derive(Clone, Debug)]
pub struct ProfileValidationError {
    /// The graph that failed validation.
    pub graph_id: String,
    /// The capabilities that were denied.
    pub denials: Vec<CapabilityDenial>,
}

impl fmt::Display for ProfileValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "graph '{}' requests {} denied capability(ies):",
            self.graph_id,
            self.denials.len()
        )?;
        for denial in &self.denials {
            write!(f, " [{denial}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProfileValidationError {}

// ─── Cell capability validation ─────────────────────────────────────────────

/// Validate that all cells in a graph have the capabilities they require
/// under the given profile.
///
/// This is a pre-start check: it inspects cell types and their known
/// requirements against the profile's effective capability set.
///
/// # Known cell requirements
///
/// - `task-executor` requires `Llm` (dispatches to a language model)
/// - Shell-based cells (`gate.compile`, `gate.test`, `gate.clippy`) require
///   `Shell` and `ReadFs`
///
/// Returns a list of denials for cells whose requirements are not met.
pub fn validate_cell_capabilities(
    graph: &crate::types::Graph,
    profile: &AuthoredGraphProfile,
) -> Vec<CellCapabilityDenial> {
    let mut denials = Vec::new();

    for (_node_id, idx) in &graph.node_map {
        let node = &graph.inner[*idx];
        let required = cell_type_required_capabilities(&node.cell_type);

        for cap in required {
            if !profile.permits(cap) {
                denials.push(CellCapabilityDenial {
                    node_id: node.id.clone(),
                    cell_type: node.cell_type.clone(),
                    capability: cap,
                });
            }
        }
    }

    denials
}

/// A cell in the graph that requires a capability not granted by the profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellCapabilityDenial {
    /// The node ID in the graph.
    pub node_id: String,
    /// The cell type name.
    pub cell_type: String,
    /// The required capability that is not granted.
    pub capability: Capability,
}

impl fmt::Display for CellCapabilityDenial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "node '{}' (cell_type '{}') requires {} which is not granted by the profile",
            self.node_id, self.cell_type, self.capability
        )
    }
}

/// Return the capabilities required by a known cell type.
///
/// Unknown cell types return an empty list (fail open at the type level;
/// the cell itself is responsible for checking capabilities at execute time
/// via `CellContext::capabilities`).
fn cell_type_required_capabilities(cell_type: &str) -> Vec<Capability> {
    match cell_type {
        // Task executor dispatches to a language model
        "task-executor" => vec![Capability::Llm],

        // Shell-based gate cells
        "gate.compile" | "gate.test" | "gate.clippy" => {
            vec![Capability::Shell, Capability::ReadFs]
        }

        // Noop and unknown types have no inherent requirements
        _ => Vec::new(),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GraphMetadata, GraphPolicy};

    fn workspace_all() -> CapabilitySet {
        CapabilitySet::all()
    }

    fn workspace_baseline_only() -> CapabilitySet {
        CapabilitySet::from([Capability::ReadFs, Capability::Bus])
    }

    fn graph_policy_with_caps(caps: &[Capability]) -> GraphPolicy {
        GraphPolicy {
            capabilities: caps.to_vec(),
            ..Default::default()
        }
    }

    // ── Fixed matrix tests ──────────────────────────────────────────────

    #[test]
    fn baseline_caps_granted_without_graph_declaration() {
        let profile = AuthoredGraphProfile::builder("test-graph")
            .workspace_grant(workspace_all())
            .build()
            .expect("should succeed with no declarations");

        assert!(profile.permits(Capability::ReadFs));
        assert!(profile.permits(Capability::Bus));
        // Elevated caps should NOT be granted without declaration
        assert!(!profile.permits(Capability::WriteFs));
        assert!(!profile.permits(Capability::Network));
        assert!(!profile.permits(Capability::Shell));
        assert!(!profile.permits(Capability::Llm));
        assert!(!profile.permits(Capability::Secrets));
    }

    #[test]
    fn elevated_caps_require_both_declaration_and_grant() {
        let policy = graph_policy_with_caps(&[Capability::Llm, Capability::Shell]);
        let profile = AuthoredGraphProfile::builder("test-graph")
            .graph_policy(&policy)
            .workspace_grant(workspace_all())
            .build()
            .expect("should succeed when both declared and granted");

        assert!(profile.permits(Capability::Llm));
        assert!(profile.permits(Capability::Shell));
        assert!(profile.permits(Capability::ReadFs)); // baseline
        assert!(profile.permits(Capability::Bus)); // baseline
        assert!(!profile.permits(Capability::Network)); // not declared
        assert!(!profile.permits(Capability::WriteFs)); // not declared
        assert!(!profile.permits(Capability::Secrets)); // not declared
    }

    #[test]
    fn graph_declared_but_workspace_denied_is_pre_start_error() {
        let policy =
            graph_policy_with_caps(&[Capability::Llm, Capability::Network, Capability::Shell]);
        let error = AuthoredGraphProfile::builder("overprivileged-graph")
            .graph_policy(&policy)
            .workspace_grant(workspace_baseline_only())
            .build()
            .expect_err("should fail when workspace denies");

        assert_eq!(error.graph_id, "overprivileged-graph");
        assert_eq!(error.denials.len(), 3);
        let denied_caps: Vec<_> = error.denials.iter().map(|d| d.capability).collect();
        assert!(denied_caps.contains(&Capability::Llm));
        assert!(denied_caps.contains(&Capability::Network));
        assert!(denied_caps.contains(&Capability::Shell));
        assert!(error
            .denials
            .iter()
            .all(|d| d.reason == DenialReason::WorkspaceNotGranted));
    }

    #[test]
    fn empty_graph_no_workspace_grants_nothing() {
        let profile = AuthoredGraphProfile::builder("empty-graph")
            .workspace_grant(CapabilitySet::empty())
            .build()
            .expect("should succeed with no declarations and no grants");

        assert!(!profile.permits(Capability::ReadFs));
        assert!(!profile.permits(Capability::Bus));
        assert!(profile.effective().is_empty());
    }

    #[test]
    fn profile_kind_is_authored_graph() {
        let profile = AuthoredGraphProfile::builder("test")
            .workspace_grant(workspace_baseline_only())
            .build()
            .unwrap();

        assert_eq!(profile.kind(), RuntimeProfileKind::AuthoredGraph);
    }

    #[test]
    fn json_and_quiet_flags_propagate() {
        let profile = AuthoredGraphProfile::builder("test")
            .workspace_grant(workspace_baseline_only())
            .json_output(true)
            .quiet(true)
            .build()
            .unwrap();

        assert!(profile.json_output());
        assert!(profile.quiet());
    }

    #[test]
    fn runtime_profile_kind_display() {
        assert_eq!(RuntimeProfileKind::AuthoredGraph.to_string(), "authored-graph");
        assert_eq!(RuntimeProfileKind::FullPlan.to_string(), "full-plan");
    }

    #[test]
    fn denial_display_workspace_not_granted() {
        let denial = CapabilityDenial {
            capability: Capability::Llm,
            reason: DenialReason::WorkspaceNotGranted,
        };
        assert!(denial
            .to_string()
            .contains("declared by graph but not granted by workspace"));
    }

    #[test]
    fn denial_display_not_declared() {
        let denial = CapabilityDenial {
            capability: Capability::Shell,
            reason: DenialReason::NotDeclaredByGraph,
        };
        assert!(denial
            .to_string()
            .contains("requires explicit graph declaration"));
    }

    #[test]
    fn validation_error_display_lists_all_denials() {
        let error = ProfileValidationError {
            graph_id: "bad-graph".to_string(),
            denials: vec![
                CapabilityDenial {
                    capability: Capability::Llm,
                    reason: DenialReason::WorkspaceNotGranted,
                },
                CapabilityDenial {
                    capability: Capability::Shell,
                    reason: DenialReason::WorkspaceNotGranted,
                },
            ],
        };
        let msg = error.to_string();
        assert!(msg.contains("bad-graph"));
        assert!(msg.contains("2 denied"));
    }

    // ── Cell capability validation tests ────────────────────────────────

    #[test]
    fn task_executor_requires_llm() {
        let required = cell_type_required_capabilities("task-executor");
        assert!(required.contains(&Capability::Llm));
    }

    #[test]
    fn gate_cells_require_shell_and_read_fs() {
        for cell_type in &["gate.compile", "gate.test", "gate.clippy"] {
            let required = cell_type_required_capabilities(cell_type);
            assert!(required.contains(&Capability::Shell), "{cell_type} needs Shell");
            assert!(
                required.contains(&Capability::ReadFs),
                "{cell_type} needs ReadFs"
            );
        }
    }

    #[test]
    fn noop_and_unknown_require_nothing() {
        assert!(cell_type_required_capabilities("noop").is_empty());
        assert!(cell_type_required_capabilities("my-custom-cell").is_empty());
    }

    #[test]
    fn validate_cell_capabilities_catches_task_executor_without_llm() {
        use crate::types::{Graph, Node};

        let mut graph = Graph::new(GraphMetadata {
            name: "needs-llm".to_string(),
            ..Default::default()
        });
        graph
            .add_node(Node {
                id: "task-1".to_string(),
                cell_type: "task-executor".to_string(),
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec![],
                outputs: vec![],
                execution_class: Default::default(),
            })
            .unwrap();

        // Profile without Llm
        let profile = AuthoredGraphProfile::builder("needs-llm")
            .workspace_grant(workspace_baseline_only())
            .build()
            .unwrap();

        let denials = validate_cell_capabilities(&graph, &profile);
        assert_eq!(denials.len(), 1);
        assert_eq!(denials[0].node_id, "task-1");
        assert_eq!(denials[0].capability, Capability::Llm);
    }

    #[test]
    fn validate_cell_capabilities_passes_with_adequate_grants() {
        use crate::types::{Graph, Node};

        let mut graph = Graph::new(GraphMetadata {
            name: "adequate".to_string(),
            ..Default::default()
        });
        graph
            .add_node(Node {
                id: "task-1".to_string(),
                cell_type: "task-executor".to_string(),
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec![],
                outputs: vec![],
                execution_class: Default::default(),
            })
            .unwrap();

        let policy = graph_policy_with_caps(&[Capability::Llm]);
        let profile = AuthoredGraphProfile::builder("adequate")
            .graph_policy(&policy)
            .workspace_grant(workspace_all())
            .build()
            .unwrap();

        let denials = validate_cell_capabilities(&graph, &profile);
        assert!(denials.is_empty(), "should pass: {denials:?}");
    }

    #[test]
    fn validate_cell_capabilities_gate_compile_without_shell() {
        use crate::types::{Graph, Node};

        let mut graph = Graph::new(GraphMetadata {
            name: "no-shell".to_string(),
            ..Default::default()
        });
        graph
            .add_node(Node {
                id: "compile".to_string(),
                cell_type: "gate.compile".to_string(),
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec![],
                outputs: vec![],
                execution_class: Default::default(),
            })
            .unwrap();

        // Only baseline caps, no Shell
        let profile = AuthoredGraphProfile::builder("no-shell")
            .workspace_grant(workspace_baseline_only())
            .build()
            .unwrap();

        let denials = validate_cell_capabilities(&graph, &profile);
        // gate.compile needs Shell -- which is not in baseline
        assert!(
            denials.iter().any(|d| d.capability == Capability::Shell),
            "should deny Shell: {denials:?}"
        );
    }

    // ── Malicious/overprivileged graph denial tests ─────────────────────

    #[test]
    fn malicious_graph_requesting_all_elevated_denied_by_readonly_workspace() {
        let policy = graph_policy_with_caps(&[
            Capability::WriteFs,
            Capability::Network,
            Capability::Shell,
            Capability::Llm,
            Capability::Secrets,
        ]);
        let error = AuthoredGraphProfile::builder("malicious-graph")
            .graph_policy(&policy)
            .workspace_grant(workspace_baseline_only())
            .build()
            .expect_err("should deny all elevated");

        assert_eq!(error.denials.len(), 5);
        assert_eq!(error.graph_id, "malicious-graph");
    }

    #[test]
    fn graph_cannot_inherit_full_plan_privileges() {
        // A graph with no capabilities declaration against a full workspace
        // should only get baseline, never the full plan set.
        let profile = AuthoredGraphProfile::builder("standalone")
            .workspace_grant(workspace_all())
            .build()
            .unwrap();

        assert_eq!(profile.kind(), RuntimeProfileKind::AuthoredGraph);
        // Only baseline should be granted
        assert!(profile.permits(Capability::ReadFs));
        assert!(profile.permits(Capability::Bus));
        assert!(!profile.permits(Capability::WriteFs));
        assert!(!profile.permits(Capability::Llm));
        assert!(!profile.permits(Capability::Secrets));
    }

    #[test]
    fn partial_workspace_grant_only_grants_intersection() {
        // Workspace grants Shell but not Llm; graph declares both
        let policy = graph_policy_with_caps(&[Capability::Shell, Capability::Llm]);
        let workspace = CapabilitySet::from([
            Capability::ReadFs,
            Capability::Bus,
            Capability::Shell,
            // Llm is NOT granted
        ]);

        let error = AuthoredGraphProfile::builder("partial-grant")
            .graph_policy(&policy)
            .workspace_grant(workspace)
            .build()
            .expect_err("Llm should be denied");

        assert_eq!(error.denials.len(), 1);
        assert_eq!(error.denials[0].capability, Capability::Llm);
    }

    #[test]
    fn cell_denial_display_includes_node_and_capability() {
        let denial = CellCapabilityDenial {
            node_id: "build-step".to_string(),
            cell_type: "gate.compile".to_string(),
            capability: Capability::Shell,
        };
        let msg = denial.to_string();
        assert!(msg.contains("build-step"));
        assert!(msg.contains("gate.compile"));
        assert!(msg.contains("Shell"));
    }
}
