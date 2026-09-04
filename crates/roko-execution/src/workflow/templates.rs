//! Named workflow graph templates (#257).
//!
//! Each canonical template maps to a pipeline band from the existing
//! `WorkflowConfig` infrastructure. Templates define acyclic subgraph
//! topologies that the [`WorkflowGraphController`](super::controller::WorkflowGraphController)
//! instantiates and executes.
//!
//! # Canonical Templates
//!
//! | Name | Review | Strategy | Max iter | Max autofix | Aliases |
//! |------|--------|----------|----------|-------------|---------|
//! | mechanical | no | no | 1 | 1 | express, standard |
//! | focused | yes | no | 2 | 2 | |
//! | integrative | yes | no | 3 | 2 | |
//! | architectural | yes | yes | 3 | 2 | full |

use std::fmt;

use roko_graph::types::{Edge, EdgeCondition, ExecutionClass, Graph, GraphMetadata, Node};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Template schema version. Included in idempotency keys.
pub const TEMPLATE_VERSION: u32 = 1;

/// All canonical template names, in order.
pub const CANONICAL_NAMES: &[&str] = &["mechanical", "focused", "integrative", "architectural"];

// ---------------------------------------------------------------------------
// Template descriptor
// ---------------------------------------------------------------------------

/// Descriptor for a named workflow template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTemplateDescriptor {
    /// Canonical template name.
    pub name: String,
    /// Template schema version.
    pub version: u32,
    /// Whether a review phase is included after successful gating.
    pub has_review: bool,
    /// Whether a strategy phase precedes implementation.
    pub has_strategy: bool,
    /// Maximum implement-gate(-review) iterations.
    pub max_iterations: u32,
    /// Maximum autofix attempts per gate failure within one iteration.
    pub max_autofix_attempts: u32,
    /// Whether git commit is enabled (can be disabled via no-commit mode).
    pub commit_enabled: bool,
}

impl WorkflowTemplateDescriptor {
    /// Create the `mechanical` template: implement -> gate -> commit.
    #[must_use]
    pub fn mechanical() -> Self {
        Self {
            name: "mechanical".to_string(),
            version: TEMPLATE_VERSION,
            has_review: false,
            has_strategy: false,
            max_iterations: 1,
            max_autofix_attempts: 1,
            commit_enabled: true,
        }
    }

    /// Create the `focused` template: implement -> gate -> review -> commit.
    #[must_use]
    pub fn focused() -> Self {
        Self {
            name: "focused".to_string(),
            version: TEMPLATE_VERSION,
            has_review: true,
            has_strategy: false,
            max_iterations: 2,
            max_autofix_attempts: 2,
            commit_enabled: true,
        }
    }

    /// Create the `integrative` template: implement -> gate -> review -> commit (more iterations).
    #[must_use]
    pub fn integrative() -> Self {
        Self {
            name: "integrative".to_string(),
            version: TEMPLATE_VERSION,
            has_review: true,
            has_strategy: false,
            max_iterations: 3,
            max_autofix_attempts: 2,
            commit_enabled: true,
        }
    }

    /// Create the `architectural` template: strategy -> implement -> gate -> review -> commit.
    #[must_use]
    pub fn architectural() -> Self {
        Self {
            name: "architectural".to_string(),
            version: TEMPLATE_VERSION,
            has_review: true,
            has_strategy: true,
            max_iterations: 3,
            max_autofix_attempts: 2,
            commit_enabled: true,
        }
    }

    /// Builder: disable the commit phase (no-commit mode).
    #[must_use]
    pub fn with_commit_disabled(mut self) -> Self {
        self.commit_enabled = false;
        self
    }
}

impl fmt::Display for WorkflowTemplateDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

// ---------------------------------------------------------------------------
// Alias resolution
// ---------------------------------------------------------------------------

/// Frozen alias table: maps legacy/convenience names to canonical names.
///
/// Unknown names are not resolved and produce an error listing all valid names.
const ALIAS_TABLE: &[(&str, &str)] = &[
    ("express", "mechanical"),
    ("standard", "mechanical"),
    ("full", "architectural"),
    // Canonical names map to themselves.
    ("mechanical", "mechanical"),
    ("focused", "focused"),
    ("integrative", "integrative"),
    ("architectural", "architectural"),
];

/// Resolve a template name (canonical or alias) to its canonical name.
///
/// Returns `None` for unknown names.
#[must_use]
pub fn resolve_template_name(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    ALIAS_TABLE
        .iter()
        .find(|(alias, _)| *alias == lower.as_str())
        .map(|(_, canonical)| *canonical)
}

/// Resolve a template name to its descriptor.
///
/// # Errors
///
/// Returns an error listing all valid canonical names and aliases if the name
/// is not recognized.
pub fn resolve_template(name: &str) -> Result<WorkflowTemplateDescriptor, TemplateResolutionError> {
    let canonical = resolve_template_name(name).ok_or_else(|| TemplateResolutionError {
        requested: name.to_string(),
        valid_canonical: CANONICAL_NAMES.iter().map(|s| (*s).to_string()).collect(),
        valid_aliases: ALIAS_TABLE
            .iter()
            .filter(|(alias, canonical)| alias != canonical)
            .map(|(alias, canonical)| format!("{alias} -> {canonical}"))
            .collect(),
    })?;

    Ok(match canonical {
        "mechanical" => WorkflowTemplateDescriptor::mechanical(),
        "focused" => WorkflowTemplateDescriptor::focused(),
        "integrative" => WorkflowTemplateDescriptor::integrative(),
        "architectural" => WorkflowTemplateDescriptor::architectural(),
        _ => unreachable!("canonical name validated by resolve_template_name"),
    })
}

/// Error returned when a template name cannot be resolved.
#[derive(Debug, Clone, thiserror::Error)]
#[error(
    "unknown workflow template '{requested}'; valid canonical names: {valid_canonical:?}; \
     aliases: {valid_aliases:?}"
)]
pub struct TemplateResolutionError {
    /// The name that was requested.
    pub requested: String,
    /// Valid canonical template names.
    pub valid_canonical: Vec<String>,
    /// Valid aliases with their targets.
    pub valid_aliases: Vec<String>,
}

// ---------------------------------------------------------------------------
// Graph topology builder
// ---------------------------------------------------------------------------

/// Node IDs used in workflow subgraphs.
pub mod node_ids {
    /// Prompt composition node.
    pub const COMPOSE: &str = "compose";
    /// Implementation agent dispatch.
    pub const IMPLEMENT: &str = "implement";
    /// Gate verification pipeline.
    pub const GATE: &str = "gate";
    /// Review agent dispatch (when review is enabled).
    pub const REVIEW: &str = "review";
    /// Git commit delivery (when commit is enabled).
    pub const COMMIT: &str = "commit";
}

/// Cell type names for workflow nodes.
pub mod cell_types {
    /// Prompt composition cell.
    pub const COMPOSE: &str = "workflow.compose";
    /// Implementation agent cell.
    pub const IMPLEMENT: &str = "workflow.implement";
    /// Gate pipeline cell.
    pub const GATE: &str = "workflow.gate";
    /// Review agent cell.
    pub const REVIEW: &str = "workflow.review";
    /// Commit delivery cell.
    pub const COMMIT: &str = "workflow.commit";
}

/// Build an acyclic subgraph for one generation of the given template.
///
/// The topology is always:
/// `Compose -> Implement(Activity) -> Gate(Activity) -> [Review(Activity)] -> [Commit(Activity)]`
///
/// Review is included only when `descriptor.has_review` is true.
/// Commit is included only when `descriptor.commit_enabled` is true.
///
/// The `generation` index is embedded in node IDs to prevent aliasing across
/// generations.
#[must_use]
pub fn build_generation_subgraph(
    descriptor: &WorkflowTemplateDescriptor,
    generation: u32,
) -> Graph {
    let prefix = format!("gen{generation}");
    let mut graph = Graph::new(GraphMetadata {
        name: format!("{}-gen{generation}", descriptor.name),
        description: Some(format!(
            "Workflow generation {generation} for template {}",
            descriptor.name
        )),
        ..Default::default()
    });

    // Always present: compose -> implement -> gate.
    let compose_id = format!("{prefix}/{}", node_ids::COMPOSE);
    let implement_id = format!("{prefix}/{}", node_ids::IMPLEMENT);
    let gate_id = format!("{prefix}/{}", node_ids::GATE);

    graph
        .add_node(Node {
            id: compose_id.clone(),
            cell_type: cell_types::COMPOSE.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec!["prompt".to_string()],
            execution_class: ExecutionClass::Workflow,
        })
        .expect("compose node is first, cannot duplicate");

    graph
        .add_node(Node {
            id: implement_id.clone(),
            cell_type: cell_types::IMPLEMENT.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec!["prompt".to_string()],
            outputs: vec!["output".to_string()],
            execution_class: ExecutionClass::Activity,
        })
        .expect("implement node unique within generation");

    graph
        .add_node(Node {
            id: gate_id.clone(),
            cell_type: cell_types::GATE.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec!["output".to_string()],
            outputs: vec!["verdict".to_string()],
            execution_class: ExecutionClass::Activity,
        })
        .expect("gate node unique within generation");

    // Edges: compose -> implement -> gate.
    graph
        .add_edge(Edge {
            from: compose_id,
            to: implement_id.clone(),
            condition: None,
        })
        .expect("valid edge");
    graph
        .add_edge(Edge {
            from: implement_id,
            to: gate_id.clone(),
            condition: Some(EdgeCondition::Success),
        })
        .expect("valid edge");

    // Track the "last node" for chaining.
    let mut last_id = gate_id;

    // Optional review node.
    if descriptor.has_review {
        let review_id = format!("{prefix}/{}", node_ids::REVIEW);
        graph
            .add_node(Node {
                id: review_id.clone(),
                cell_type: cell_types::REVIEW.to_string(),
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec!["verdict".to_string()],
                outputs: vec!["review_verdict".to_string()],
                execution_class: ExecutionClass::Activity,
            })
            .expect("review node unique within generation");

        graph
            .add_edge(Edge {
                from: last_id,
                to: review_id.clone(),
                condition: Some(EdgeCondition::Success),
            })
            .expect("valid edge");

        last_id = review_id;
    }

    // Optional commit node.
    if descriptor.commit_enabled {
        let commit_id = format!("{prefix}/{}", node_ids::COMMIT);
        graph
            .add_node(Node {
                id: commit_id.clone(),
                cell_type: cell_types::COMMIT.to_string(),
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec![],
                outputs: vec!["commit_hash".to_string()],
                execution_class: ExecutionClass::Activity,
            })
            .expect("commit node unique within generation");

        graph
            .add_edge(Edge {
                from: last_id,
                to: commit_id,
                condition: Some(EdgeCondition::Success),
            })
            .expect("valid edge");
    }

    graph
}

/// Build a gate-failure recovery subgraph (autofix generation).
///
/// Topology: `Compose(failure evidence) -> AutoFix(Activity) -> Gate -> [Review] -> [Commit]`
#[must_use]
pub fn build_autofix_subgraph(descriptor: &WorkflowTemplateDescriptor, generation: u32) -> Graph {
    let prefix = format!("gen{generation}");
    let mut graph = Graph::new(GraphMetadata {
        name: format!("{}-autofix-gen{generation}", descriptor.name),
        description: Some(format!(
            "Autofix generation {generation} for template {}",
            descriptor.name
        )),
        ..Default::default()
    });

    let compose_id = format!("{prefix}/{}", node_ids::COMPOSE);
    let autofix_id = format!("{prefix}/autofix");
    let gate_id = format!("{prefix}/{}", node_ids::GATE);

    graph
        .add_node(Node {
            id: compose_id.clone(),
            cell_type: cell_types::COMPOSE.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec!["prompt".to_string()],
            execution_class: ExecutionClass::Workflow,
        })
        .expect("compose node first");

    graph
        .add_node(Node {
            id: autofix_id.clone(),
            cell_type: cell_types::IMPLEMENT.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec!["prompt".to_string()],
            outputs: vec!["output".to_string()],
            execution_class: ExecutionClass::Activity,
        })
        .expect("autofix node unique");

    graph
        .add_node(Node {
            id: gate_id.clone(),
            cell_type: cell_types::GATE.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec!["output".to_string()],
            outputs: vec!["verdict".to_string()],
            execution_class: ExecutionClass::Activity,
        })
        .expect("gate node unique");

    graph
        .add_edge(Edge {
            from: compose_id,
            to: autofix_id.clone(),
            condition: None,
        })
        .expect("valid edge");
    graph
        .add_edge(Edge {
            from: autofix_id,
            to: gate_id.clone(),
            condition: Some(EdgeCondition::Success),
        })
        .expect("valid edge");

    let mut last_id = gate_id;

    if descriptor.has_review {
        let review_id = format!("{prefix}/{}", node_ids::REVIEW);
        graph
            .add_node(Node {
                id: review_id.clone(),
                cell_type: cell_types::REVIEW.to_string(),
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec!["verdict".to_string()],
                outputs: vec!["review_verdict".to_string()],
                execution_class: ExecutionClass::Activity,
            })
            .expect("review node unique");

        graph
            .add_edge(Edge {
                from: last_id,
                to: review_id.clone(),
                condition: Some(EdgeCondition::Success),
            })
            .expect("valid edge");
        last_id = review_id;
    }

    if descriptor.commit_enabled {
        let commit_id = format!("{prefix}/{}", node_ids::COMMIT);
        graph
            .add_node(Node {
                id: commit_id.clone(),
                cell_type: cell_types::COMMIT.to_string(),
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec![],
                outputs: vec!["commit_hash".to_string()],
                execution_class: ExecutionClass::Activity,
            })
            .expect("commit node unique");

        graph
            .add_edge(Edge {
                from: last_id,
                to: commit_id,
                condition: Some(EdgeCondition::Success),
            })
            .expect("valid edge");
    }

    graph
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Alias resolution ───────────────────────────────────────────────

    #[test]
    fn resolve_canonical_names() {
        for name in CANONICAL_NAMES {
            assert_eq!(resolve_template_name(name), Some(*name));
        }
    }

    #[test]
    fn resolve_aliases() {
        assert_eq!(resolve_template_name("express"), Some("mechanical"));
        assert_eq!(resolve_template_name("standard"), Some("mechanical"));
        assert_eq!(resolve_template_name("full"), Some("architectural"));
    }

    #[test]
    fn resolve_case_insensitive() {
        assert_eq!(resolve_template_name("MECHANICAL"), Some("mechanical"));
        assert_eq!(resolve_template_name("Express"), Some("mechanical"));
        assert_eq!(resolve_template_name("FULL"), Some("architectural"));
    }

    #[test]
    fn resolve_unknown_returns_none() {
        assert_eq!(resolve_template_name("bogus"), None);
    }

    #[test]
    fn resolve_template_unknown_returns_error() {
        let err = resolve_template("bogus").unwrap_err();
        assert_eq!(err.requested, "bogus");
        assert_eq!(err.valid_canonical.len(), 4);
        assert!(!err.valid_aliases.is_empty());
    }

    #[test]
    fn resolve_template_all_canonical() {
        for name in CANONICAL_NAMES {
            let desc = resolve_template(name).unwrap();
            assert_eq!(desc.name, *name);
            assert_eq!(desc.version, TEMPLATE_VERSION);
        }
    }

    #[test]
    fn resolve_template_alias_express() {
        let desc = resolve_template("express").unwrap();
        assert_eq!(desc.name, "mechanical");
    }

    #[test]
    fn resolve_template_alias_full() {
        let desc = resolve_template("full").unwrap();
        assert_eq!(desc.name, "architectural");
    }

    // ── Template properties ────────────────────────────────────────────

    #[test]
    fn mechanical_properties() {
        let d = WorkflowTemplateDescriptor::mechanical();
        assert!(!d.has_review);
        assert!(!d.has_strategy);
        assert_eq!(d.max_iterations, 1);
        assert_eq!(d.max_autofix_attempts, 1);
        assert!(d.commit_enabled);
    }

    #[test]
    fn focused_properties() {
        let d = WorkflowTemplateDescriptor::focused();
        assert!(d.has_review);
        assert!(!d.has_strategy);
        assert_eq!(d.max_iterations, 2);
    }

    #[test]
    fn integrative_properties() {
        let d = WorkflowTemplateDescriptor::integrative();
        assert!(d.has_review);
        assert!(!d.has_strategy);
        assert_eq!(d.max_iterations, 3);
    }

    #[test]
    fn architectural_properties() {
        let d = WorkflowTemplateDescriptor::architectural();
        assert!(d.has_review);
        assert!(d.has_strategy);
        assert_eq!(d.max_iterations, 3);
    }

    #[test]
    fn no_commit_mode() {
        let d = WorkflowTemplateDescriptor::mechanical().with_commit_disabled();
        assert!(!d.commit_enabled);
    }

    #[test]
    fn display_format() {
        let d = WorkflowTemplateDescriptor::mechanical();
        assert_eq!(d.to_string(), "mechanical@1");
    }

    // ── Subgraph topology ──────────────────────────────────────────────

    #[test]
    fn mechanical_subgraph_has_3_nodes() {
        let d = WorkflowTemplateDescriptor::mechanical();
        let g = build_generation_subgraph(&d, 0);
        // compose, implement, gate, commit = 4 nodes (no review).
        assert_eq!(g.node_count(), 4);
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn focused_subgraph_has_review() {
        let d = WorkflowTemplateDescriptor::focused();
        let g = build_generation_subgraph(&d, 0);
        // compose, implement, gate, review, commit = 5 nodes.
        assert_eq!(g.node_count(), 5);
        assert_eq!(g.edge_count(), 4);
    }

    #[test]
    fn no_commit_subgraph_omits_commit() {
        let d = WorkflowTemplateDescriptor::mechanical().with_commit_disabled();
        let g = build_generation_subgraph(&d, 0);
        // compose, implement, gate = 3 nodes.
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn generation_ids_are_scoped() {
        let d = WorkflowTemplateDescriptor::mechanical();
        let g0 = build_generation_subgraph(&d, 0);
        let g1 = build_generation_subgraph(&d, 1);
        assert!(g0.get_node("gen0/compose").is_some());
        assert!(g1.get_node("gen1/compose").is_some());
        // No cross-generation aliasing.
        assert!(g0.get_node("gen1/compose").is_none());
    }

    #[test]
    fn autofix_subgraph_topology() {
        let d = WorkflowTemplateDescriptor::focused();
        let g = build_autofix_subgraph(&d, 2);
        // compose, autofix, gate, review, commit = 5 nodes.
        assert_eq!(g.node_count(), 5);
        assert!(g.get_node("gen2/autofix").is_some());
        assert!(g.get_node("gen2/gate").is_some());
    }

    #[test]
    fn subgraph_is_acyclic() {
        for name in CANONICAL_NAMES {
            let d = resolve_template(name).unwrap();
            let g = build_generation_subgraph(&d, 0);
            let order = roko_graph::topo::topological_order(&g);
            assert!(order.is_ok(), "cycle detected in {name} subgraph");
        }
    }

    #[test]
    fn autofix_subgraph_is_acyclic() {
        for name in CANONICAL_NAMES {
            let d = resolve_template(name).unwrap();
            let g = build_autofix_subgraph(&d, 0);
            let order = roko_graph::topo::topological_order(&g);
            assert!(order.is_ok(), "cycle detected in {name} autofix subgraph");
        }
    }

    // ── Serde ──────────────────────────────────────────────────────────

    #[test]
    fn descriptor_serde_roundtrip() {
        for name in CANONICAL_NAMES {
            let d = resolve_template(name).unwrap();
            let json = serde_json::to_string(&d).unwrap();
            let back: WorkflowTemplateDescriptor = serde_json::from_str(&json).unwrap();
            assert_eq!(back, d);
        }
    }
}
