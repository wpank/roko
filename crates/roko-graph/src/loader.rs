//! TOML loader: parse graph definitions from TOML files into `Graph` structs.

use std::path::Path;

use roko_core::{LensConfig, LensRegistry};
use serde::Deserialize;

use crate::types::{
    Edge, EdgeCondition, ExecutionClass, Graph, GraphError, GraphMetadata, GraphPolicy, Node,
};

/// Raw TOML representation of a graph file (for deserialization).
#[derive(Debug, Deserialize)]
struct RawGraphFile {
    graph: RawGraphMeta,
    #[serde(default)]
    nodes: Vec<RawNode>,
    #[serde(default)]
    edges: Vec<RawEdge>,
    /// Top-level `[[lenses]]` entries from the telemetry specification.
    #[serde(default)]
    lenses: Vec<LensConfig>,
}

/// Raw metadata section (including optional policy subsection).
#[derive(Debug, Deserialize)]
struct RawGraphMeta {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    labels: Option<std::collections::HashMap<String, String>>,
    /// Optional `[graph.policy]` section. Absent → default policy.
    #[serde(default)]
    policy: Option<GraphPolicy>,
}

/// Raw node definition from TOML.
#[derive(Debug, Deserialize)]
struct RawNode {
    id: String,
    cell_type: String,
    #[serde(default)]
    config: Option<toml::Value>,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    outputs: Vec<String>,
    /// Execution classification for replay and snapshot decisions.
    /// Defaults to `Activity` (non-deterministic) when absent from TOML.
    #[serde(default)]
    execution_class: ExecutionClass,
}

/// Raw edge definition from TOML.
#[derive(Debug, Deserialize)]
struct RawEdge {
    from: String,
    to: String,
    #[serde(default)]
    condition: Option<RawEdgeCondition>,
}

/// Raw edge condition from TOML.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum RawEdgeCondition {
    #[serde(alias = "success")]
    Success,
    #[serde(alias = "failure")]
    Failure,
    #[serde(alias = "always")]
    Always,
    #[serde(alias = "output_equals")]
    OutputEquals { key: String, value: String },
}

impl From<RawEdgeCondition> for EdgeCondition {
    fn from(raw: RawEdgeCondition) -> Self {
        match raw {
            RawEdgeCondition::Success => Self::Success,
            RawEdgeCondition::Failure => Self::Failure,
            RawEdgeCondition::Always => Self::Always,
            RawEdgeCondition::OutputEquals { key, value } => Self::OutputEquals { key, value },
        }
    }
}

/// Load a graph from a TOML string.
///
/// # Errors
/// Returns `GraphError::LoaderError` on parse failures, or `GraphError::DuplicateNode` /
/// `GraphError::NodeNotFound` if the graph definition is inconsistent.
pub fn load_from_str(toml_str: &str) -> Result<Graph, GraphError> {
    let raw: RawGraphFile =
        toml::from_str(toml_str).map_err(|e| GraphError::LoaderError(e.to_string()))?;

    let metadata = GraphMetadata {
        name: raw.graph.name,
        description: raw.graph.description,
        version: raw.graph.version,
        labels: raw.graph.labels.unwrap_or_default(),
    };

    // Use the `[graph.policy]` section if present; otherwise fall back to default.
    let policy = raw.graph.policy.unwrap_or_default();

    let lenses = load_lenses(raw.lenses)?;
    let mut graph = Graph::new(metadata).with_policy(policy).with_lenses(lenses);

    // Add all nodes first.
    for raw_node in raw.nodes {
        let node = Node {
            id: raw_node.id,
            cell_type: raw_node.cell_type,
            config: raw_node
                .config
                .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new())),
            inputs: raw_node.inputs,
            outputs: raw_node.outputs,
            execution_class: raw_node.execution_class,
        };
        graph.add_node(node)?;
    }

    // Then add edges (referencing existing nodes).
    for raw_edge in raw.edges {
        let edge = Edge {
            from: raw_edge.from,
            to: raw_edge.to,
            condition: raw_edge.condition.map(Into::into),
        };
        graph.add_edge(edge)?;
    }

    Ok(graph)
}

fn load_lenses(configs: Vec<LensConfig>) -> Result<LensRegistry, GraphError> {
    let mut registry = LensRegistry::new();
    for config in configs {
        let name = config.name.clone();
        registry
            .register(config)
            .map_err(|error| GraphError::LoaderError(format!("invalid lens `{name}`: {error}")))?;
    }
    registry
        .validate()
        .map_err(|error| GraphError::LoaderError(format!("invalid lens composition: {error}")))?;
    Ok(registry)
}

/// Load a graph from a TOML file on disk.
///
/// # Errors
/// Returns `GraphError::LoaderError` if the file cannot be read or parsed.
pub fn load_from_file(path: &Path) -> Result<Graph, GraphError> {
    let content =
        std::fs::read_to_string(path).map_err(|e| GraphError::LoaderError(e.to_string()))?;
    load_from_str(&content)
}

#[cfg(test)]
mod tests {
    use crate::types::{FailureStrategy, GraphMode};

    use super::*;

    const SAMPLE_GRAPH: &str = r#"
[graph]
name = "ci-pipeline"
description = "Compile then test"
version = "1.0.0"

[[nodes]]
id = "compile"
cell_type = "gate.compile"
inputs = []
outputs = ["artifact"]
[nodes.config]
workspace = "."

[[nodes]]
id = "test"
cell_type = "gate.test"
inputs = ["compile.artifact"]
outputs = ["report"]
[nodes.config]
timeout_secs = 300

[[edges]]
from = "compile"
to = "test"
[edges.condition]
type = "success"
"#;

    #[test]
    fn load_basic_graph_from_str() {
        let graph = load_from_str(SAMPLE_GRAPH).unwrap();
        assert_eq!(graph.metadata.name, "ci-pipeline");
        assert_eq!(
            graph.metadata.description.as_deref(),
            Some("Compile then test")
        );
        assert_eq!(graph.metadata.version.as_deref(), Some("1.0.0"));
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        let compile = graph.get_node("compile").unwrap();
        assert_eq!(compile.cell_type, "gate.compile");
        assert_eq!(compile.outputs, vec!["artifact"]);

        let test = graph.get_node("test").unwrap();
        assert_eq!(test.cell_type, "gate.test");
        assert_eq!(test.inputs, vec!["compile.artifact"]);
    }

    #[test]
    fn load_graph_without_policy_uses_defaults() {
        let graph = load_from_str(SAMPLE_GRAPH).unwrap();
        // No [graph.policy] section -> defaults applied.
        assert_eq!(graph.policy.mode, GraphMode::OneShot);
        assert_eq!(graph.policy.failure_strategy, FailureStrategy::FailFast);
        assert_eq!(graph.policy.max_concurrent_nodes, 4);
        assert!(graph.policy.timeout_ms.is_none());
        assert!(graph.policy.hot.is_none());
    }

    #[test]
    fn load_graph_with_policy_section() {
        let toml_str = r#"
[graph]
name = "hot-pipeline"

[graph.policy]
mode = "hot"
failure_strategy = "skip_failed"
max_concurrent_nodes = 8
timeout_ms = 60000

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        assert_eq!(graph.policy.mode, GraphMode::Hot);
        assert_eq!(graph.policy.failure_strategy, FailureStrategy::SkipFailed);
        assert_eq!(graph.policy.max_concurrent_nodes, 8);
        assert_eq!(graph.policy.timeout_ms, Some(60_000));
    }

    #[test]
    fn load_graph_without_edges() {
        let toml_str = r#"
[graph]
name = "solo"

[[nodes]]
id = "only"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.lenses.registrations().is_empty());
    }

    #[test]
    fn load_graph_with_lenses_preserves_params_filters_and_chain_order() {
        let toml_str = r#"
[graph]
name = "observed-pipeline"

[[nodes]]
id = "build"
cell_type = "noop"

[[lenses]]
name = "cost"
block = "roko:cost-lens@^1.0"
scope = "graph:observed-pipeline"
[lenses.params]
interval = "60s"
budget_warn_pct = 0.8

[[lenses]]
name = "trend"
block = "roko:trend-lens@^1.0"
scope = "lens:cost"
[lenses.params]
metric = "total_usd"
"#;

        let graph = load_from_str(toml_str).unwrap();
        assert_eq!(graph.lenses.registrations().len(), 2);
        let cost = graph.lenses.get("cost").unwrap();
        assert_eq!(cost.config.params["interval"].as_str(), Some("60s"));
        assert_eq!(
            cost.observes,
            [
                roko_core::ObservableEventKind::CellLifecycle,
                roko_core::ObservableEventKind::GraphLifecycle,
                roko_core::ObservableEventKind::AgentLifecycle,
            ]
        );
        assert_eq!(graph.lenses.chain_order().unwrap(), ["cost", "trend"]);
    }

    #[test]
    fn load_graph_allows_forward_referenced_lens_chains() {
        let toml_str = r#"
[graph]
name = "forward-chain"

[[lenses]]
name = "anomaly"
block = "roko:anomaly-lens@^1.0"
scope = "lens:trend"

[[lenses]]
name = "trend"
block = "roko:trend-lens@^1.0"
scope = "lens:cost"

[[lenses]]
name = "cost"
block = "roko:cost-lens@^1.0"
scope = "graph"
"#;

        let graph = load_from_str(toml_str).unwrap();
        assert_eq!(
            graph.lenses.chain_order().unwrap(),
            ["cost", "trend", "anomaly"]
        );
        assert_eq!(graph.lenses.chains()["cost"], ["trend"]);
        assert_eq!(graph.lenses.chains()["trend"], ["anomaly"]);
    }

    #[test]
    fn load_graph_rejects_invalid_lens_config_and_duplicates() {
        let invalid_scope = r#"
[graph]
name = "bad-lens"

[[lenses]]
name = "cost"
block = "roko:cost-lens@^1.0"
scope = "universe"
"#;
        let error = load_from_str(invalid_scope).unwrap_err();
        assert!(matches!(error, GraphError::LoaderError(_)));
        assert!(error.to_string().contains("invalid lens `cost`"));
        assert!(error.to_string().contains("unknown lens scope `universe`"));

        let duplicate = r#"
[graph]
name = "duplicate-lens"

[[lenses]]
name = "cost"
block = "roko:cost-lens@^1.0"
scope = "graph"

[[lenses]]
name = "cost"
block = "roko:cost-lens@^2.0"
scope = "global"
"#;
        let error = load_from_str(duplicate).unwrap_err();
        assert!(error.to_string().contains("duplicate lens name `cost`"));
    }

    #[test]
    fn load_graph_rejects_lens_cycles_with_path() {
        let toml_str = r#"
[graph]
name = "cyclic-lenses"

[[lenses]]
name = "a"
block = "plugin:a@1"
scope = "lens:b"

[[lenses]]
name = "b"
block = "plugin:b@1"
scope = "lens:c"

[[lenses]]
name = "c"
block = "plugin:c@1"
scope = "lens:a"
"#;

        let error = load_from_str(toml_str).unwrap_err();
        assert!(matches!(error, GraphError::LoaderError(_)));
        assert!(error.to_string().contains("lens chain cycle detected"));
        assert!(error.to_string().contains("a -> c -> b -> a"));
    }

    #[test]
    fn load_graph_rejects_unresolved_lens_forward_reference() {
        let toml_str = r#"
[graph]
name = "missing-upstream"

[[lenses]]
name = "trend"
block = "roko:trend-lens@^1.0"
scope = "lens:cost"
"#;

        let error = load_from_str(toml_str).unwrap_err();
        assert!(matches!(error, GraphError::LoaderError(_)));
        assert!(error.to_string().contains("unresolved upstream lens: cost"));
    }

    #[test]
    fn load_graph_edge_to_missing_node_fails() {
        let toml_str = r#"
[graph]
name = "bad"

[[nodes]]
id = "a"
cell_type = "noop"

[[edges]]
from = "a"
to = "b"
"#;
        let result = load_from_str(toml_str);
        assert!(matches!(
            result,
            Err(GraphError::NodeNotFound(ref id)) if id == "b"
        ));
    }

    #[test]
    fn load_invalid_toml_fails() {
        let result = load_from_str("not valid toml {{{{");
        assert!(matches!(result, Err(GraphError::LoaderError(_))));
    }

    #[test]
    fn load_from_file_works() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("graph.toml");
        std::fs::write(&path, SAMPLE_GRAPH).unwrap();

        let graph = load_from_file(&path).unwrap();
        assert_eq!(graph.metadata.name, "ci-pipeline");
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn load_graph_with_labels() {
        let toml_str = r#"
[graph]
name = "labeled"
[graph.labels]
team = "platform"
priority = "high"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        assert_eq!(graph.metadata.labels.get("team").unwrap(), "platform");
        assert_eq!(graph.metadata.labels.get("priority").unwrap(), "high");
    }
}
