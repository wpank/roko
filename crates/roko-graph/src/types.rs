//! Core graph types: `Graph`, `Node`, `Edge`, `NodeId`, `EdgeCondition`, `GraphMetadata`,
//! `NodeOutput`, `NodeOutputStatus`, `GraphConfig`, `GraphPolicy`, `GraphMode`, `FailureStrategy`.

use std::collections::HashMap;
use std::time::Duration;

use indexmap::IndexMap;
use petgraph::graph::DiGraph;
use roko_core::{LensRegistry, TypeSchema};
use serde::{Deserialize, Serialize};

use roko_core::Capability;

use crate::hot::HotPolicy;

/// Unique identifier for a node within a graph.
pub type NodeId = String;

/// Condition that gates execution along an edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum EdgeCondition {
    /// Edge fires only if the source node succeeded.
    Success,
    /// Edge fires only if the source node failed.
    Failure,
    /// Edge fires only if the named output equals the given value.
    OutputEquals {
        /// Output key to check.
        key: String,
        /// Expected value.
        value: String,
    },
    /// Edge always fires (unconditional dependency).
    Always,
}

/// Metadata associated with a graph definition.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphMetadata {
    /// Human-readable name of the graph.
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional version string.
    #[serde(default)]
    pub version: Option<String>,
    /// Arbitrary key-value annotations.
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// Return an empty TOML table (used as default for node config).
fn default_config() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

/// Classification of a node for replay and snapshot purposes.
///
/// - `Workflow` nodes are deterministic (routing, scoring, composition): they can be
///   re-derived from their inputs on replay and do **not** need to be recorded.
/// - `Activity` nodes are non-deterministic (LLM calls, tool execution, external APIs):
///   their outputs must be recorded so replay can reproduce the same result without
///   re-invoking the external system.
///
/// The Graph Engine uses this flag when deciding which node outputs to include in a
/// snapshot (E21-T05).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionClass {
    /// Deterministic: routing, scoring, composition. Re-derived on replay.
    Workflow,
    /// Non-deterministic: LLM calls, tool execution, external APIs. Recorded for replay.
    #[default]
    Activity,
}

/// A node in the execution graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier within this graph.
    pub id: NodeId,
    /// Cell type name used to look up the factory in `CellRegistry`.
    pub cell_type: String,
    /// Configuration passed to the cell factory function.
    #[serde(default = "default_config")]
    pub config: toml::Value,
    /// Named inputs this node consumes (from upstream edges).
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Named outputs this node produces (for downstream edges).
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Execution classification for replay and snapshot decisions.
    ///
    /// Defaults to [`ExecutionClass::Activity`] because most graph nodes involve
    /// non-deterministic external computation (LLM calls, tool use, API calls).
    /// Set to [`ExecutionClass::Workflow`] for pure routing or scoring nodes whose
    /// outputs can be re-derived from inputs without re-running the external system.
    #[serde(default)]
    pub execution_class: ExecutionClass,
}

/// An edge connecting two nodes in the execution graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    /// Source node ID.
    pub from: NodeId,
    /// Target node ID.
    pub to: NodeId,
    /// Optional condition for this edge to fire.
    #[serde(default)]
    pub condition: Option<EdgeCondition>,
}

/// Index type used by petgraph for node indices.
pub type GraphNodeIdx = petgraph::graph::NodeIndex;

// ─── Graph execution policy ──────────────────────────────────────────────────

/// Execution mode for a graph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphMode {
    /// Execute the graph once and return. Default mode.
    #[default]
    OneShot,
    /// Execute the graph in a continuous tick loop (Hot Graph).
    Hot,
}

/// Strategy to apply when a node fails during execution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureStrategy {
    /// Abort the entire graph immediately on the first node failure. Default.
    #[default]
    FailFast,
    /// Skip failed nodes and continue executing independent successors.
    SkipFailed,
    /// Retry failed nodes up to `max_retries` times before giving up.
    Retry {
        /// Maximum number of retry attempts per failed node.
        max_retries: u32,
    },
}

/// Top-level execution policy for a graph definition.
///
/// Parsed from `[graph.policy]` in TOML graph definitions. If the section is
/// absent, all fields use their defaults (OneShot, FailFast, 4 concurrent nodes).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphPolicy {
    /// Whether to run once (`OneShot`) or continuously tick (`Hot`).
    #[serde(default)]
    pub mode: GraphMode,
    /// What to do when a node fails.
    #[serde(default)]
    pub failure_strategy: FailureStrategy,
    /// Maximum number of nodes that may execute concurrently within one wave.
    #[serde(default = "default_max_concurrent_nodes")]
    pub max_concurrent_nodes: usize,
    /// Optional wall-clock timeout for the entire graph execution (milliseconds).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Hot Graph tick policy. Only relevant when `mode = Hot`.
    #[serde(default)]
    pub hot: Option<HotPolicy>,
    /// Capabilities declared by this graph definition.
    ///
    /// Missing `capabilities` deserializes to an empty vector, meaning the graph
    /// requests no privileged authorities. A live run may receive `ReadFs` and
    /// `Bus` from normal workspace configuration; `WriteFs`, `Network`, `Shell`,
    /// `Llm`, and `Secrets` require both a graph declaration here **and** the
    /// corresponding workspace grant.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

fn default_max_concurrent_nodes() -> usize {
    4
}

impl Default for GraphPolicy {
    fn default() -> Self {
        Self {
            mode: GraphMode::OneShot,
            failure_strategy: FailureStrategy::FailFast,
            max_concurrent_nodes: default_max_concurrent_nodes(),
            timeout_ms: None,
            hot: None,
            capabilities: Vec::new(),
        }
    }
}

/// A directed acyclic graph of execution nodes, backed by petgraph.
#[derive(Debug, Clone)]
pub struct Graph {
    /// Graph metadata (name, description, labels).
    pub metadata: GraphMetadata,
    /// Execution policy controlling mode, failure handling, and concurrency.
    pub policy: GraphPolicy,
    /// Validated telemetry Lens routing and composition declared by
    /// top-level `[[lenses]]` entries in the graph definition.
    pub lenses: LensRegistry,
    /// The underlying petgraph directed graph.
    pub inner: DiGraph<Node, Edge>,
    /// Mapping from `NodeId` (string) to petgraph node index.
    pub node_map: IndexMap<NodeId, GraphNodeIdx>,
}

impl Graph {
    /// Create a new empty graph with the given metadata and default policy.
    #[must_use]
    pub fn new(metadata: GraphMetadata) -> Self {
        Self {
            metadata,
            policy: GraphPolicy::default(),
            lenses: LensRegistry::new(),
            inner: DiGraph::new(),
            node_map: IndexMap::new(),
        }
    }

    /// Builder method to override the graph's execution policy.
    #[must_use]
    pub fn with_policy(mut self, policy: GraphPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Builder method to attach a validated telemetry Lens registry.
    #[must_use]
    pub fn with_lenses(mut self, lenses: LensRegistry) -> Self {
        self.lenses = lenses;
        self
    }

    /// Add a node to the graph. Returns the petgraph index.
    ///
    /// # Errors
    /// Returns an error if a node with the same ID already exists.
    pub fn add_node(&mut self, node: Node) -> Result<GraphNodeIdx, GraphError> {
        if self.node_map.contains_key(&node.id) {
            return Err(GraphError::DuplicateNode(node.id));
        }
        let id = node.id.clone();
        let idx = self.inner.add_node(node);
        self.node_map.insert(id, idx);
        Ok(idx)
    }

    /// Add an edge between two existing nodes.
    ///
    /// # Errors
    /// Returns an error if either node ID is not found.
    pub fn add_edge(&mut self, edge: Edge) -> Result<(), GraphError> {
        let from_idx = self
            .node_map
            .get(&edge.from)
            .copied()
            .ok_or_else(|| GraphError::NodeNotFound(edge.from.clone()))?;
        let to_idx = self
            .node_map
            .get(&edge.to)
            .copied()
            .ok_or_else(|| GraphError::NodeNotFound(edge.to.clone()))?;
        self.inner.add_edge(from_idx, to_idx, edge);
        Ok(())
    }

    /// Get a node by its string ID.
    #[must_use]
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.node_map.get(id).map(|idx| &self.inner[*idx])
    }

    /// Return the number of nodes in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    /// Return the number of edges in the graph.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// Validate all edges for TypeSchema compatibility between source output and
    /// target input schemas.
    ///
    /// Uses side-effect-free [`CellDescriptor`] introspection from the registry
    /// rather than constructing live Cell instances. This ensures validation
    /// never triggers factory side effects.
    ///
    /// Rules:
    /// - Edges where either cell is missing from the registry produce an error.
    /// - Edges where either cell returns `None` for its schema (i.e. untyped /
    ///   "Any") are always considered valid.
    /// - ALL errors are collected — validation does not fail fast.
    #[must_use]
    pub fn validate_edges(
        &self,
        registry: &crate::registry::CellRegistry,
    ) -> Vec<EdgeValidationError> {
        let mut errors = Vec::new();

        for edge_ref in self.inner.edge_references() {
            let edge = edge_ref.weight();
            let from_id = &edge.from;
            let to_id = &edge.to;

            // Look up source node's cell type.
            let source_cell_type = match self.get_node(from_id) {
                Some(n) => n.cell_type.clone(),
                None => {
                    errors.push(EdgeValidationError {
                        edge_from: from_id.clone(),
                        edge_to: to_id.clone(),
                        source_schema: None,
                        target_schema: None,
                        reason: format!("source node '{from_id}' not found in graph"),
                    });
                    continue;
                }
            };

            // Look up target node's cell type.
            let target_cell_type = match self.get_node(to_id) {
                Some(n) => n.cell_type.clone(),
                None => {
                    errors.push(EdgeValidationError {
                        edge_from: from_id.clone(),
                        edge_to: to_id.clone(),
                        source_schema: None,
                        target_schema: None,
                        reason: format!("target node '{to_id}' not found in graph"),
                    });
                    continue;
                }
            };

            // Retrieve source descriptor (side-effect-free, no Cell construction).
            let source_desc = match registry.descriptor(&source_cell_type) {
                Some(d) => d,
                None => {
                    errors.push(EdgeValidationError {
                        edge_from: from_id.clone(),
                        edge_to: to_id.clone(),
                        source_schema: None,
                        target_schema: None,
                        reason: format!(
                            "source node '{from_id}' cell type '{source_cell_type}' not in registry"
                        ),
                    });
                    continue;
                }
            };

            // Retrieve target descriptor (side-effect-free, no Cell construction).
            let target_desc = match registry.descriptor(&target_cell_type) {
                Some(d) => d,
                None => {
                    errors.push(EdgeValidationError {
                        edge_from: from_id.clone(),
                        edge_to: to_id.clone(),
                        source_schema: None,
                        target_schema: None,
                        reason: format!(
                            "target node '{to_id}' cell type '{target_cell_type}' not in registry"
                        ),
                    });
                    continue;
                }
            };

            // Get schemas from descriptors. None means untyped (Any) — always valid.
            let source_schema = source_desc.output_schema.clone();
            let target_schema = target_desc.input_schema.clone();

            let compatible = match (&source_schema, &target_schema) {
                // Either side is None (untyped / Any) -> always valid.
                (None, _) | (_, None) => true,
                (Some(src), Some(tgt)) => src.is_compatible_with(tgt),
            };

            if !compatible {
                errors.push(EdgeValidationError {
                    edge_from: from_id.clone(),
                    edge_to: to_id.clone(),
                    source_schema,
                    target_schema,
                    reason: format!(
                        "output schema of '{from_id}' is not compatible with input schema of '{to_id}'"
                    ),
                });
            }
        }

        errors
    }
}

/// A type-schema mismatch detected between two connected graph nodes.
///
/// Produced by [`Graph::validate_edges`] when the source node's `output_schema`
/// is not compatible with the target node's `input_schema`.
#[derive(Debug, Clone)]
pub struct EdgeValidationError {
    /// ID of the source (upstream) node.
    pub edge_from: String,
    /// ID of the target (downstream) node.
    pub edge_to: String,
    /// Output schema declared by the source cell (`None` = untyped).
    pub source_schema: Option<TypeSchema>,
    /// Input schema declared by the target cell (`None` = untyped).
    pub target_schema: Option<TypeSchema>,
    /// Human-readable explanation of why the edge is invalid.
    pub reason: String,
}

impl std::fmt::Display for EdgeValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "edge {} -> {}: {}",
            self.edge_from, self.edge_to, self.reason
        )
    }
}

/// Errors specific to graph construction, validation, and execution.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    /// A node with this ID already exists.
    #[error("duplicate node: `{0}`")]
    DuplicateNode(NodeId),
    /// Referenced node ID does not exist in the graph.
    #[error("node not found: `{0}`")]
    NodeNotFound(NodeId),
    /// The graph contains a cycle and cannot be topologically sorted.
    #[error("cycle detected in graph")]
    CycleDetected,
    /// TOML parsing or schema error.
    #[error("loader error: {0}")]
    LoaderError(String),
    /// Cell type not found in registry.
    #[error("unknown cell type: `{0}`")]
    UnknownCellType(String),
    /// A node failed during execution.
    #[error("node '{node_id}' failed: {reason}")]
    NodeFailed {
        /// The node that failed.
        node_id: String,
        /// Description of the failure.
        reason: String,
    },
    /// An edge references a non-existent source or target.
    #[error("edge references unknown node '{node_id}'")]
    InvalidEdge {
        /// The referenced node ID.
        node_id: String,
    },
    /// Budget exceeded (tokens, cost, or deadline).
    #[error("budget exceeded: {reason}")]
    BudgetExceeded {
        /// Description of which limit was breached.
        reason: String,
    },
    /// A condition expression failed to evaluate.
    #[error("condition evaluation failed for edge {from} -> {to}: {reason}")]
    ConditionError {
        /// Source node.
        from: String,
        /// Target node.
        to: String,
        /// What went wrong.
        reason: String,
    },
    /// The graph definition is invalid.
    #[error("invalid graph definition: {reason}")]
    InvalidGraph {
        /// What is wrong with the graph.
        reason: String,
    },
    /// One or more edges failed type-schema validation.
    #[error("edge validation failed ({count} error(s)): {first_error}")]
    EdgeValidationFailed {
        /// Number of invalid edges detected.
        count: usize,
        /// Human-readable description of the first error (for Display).
        first_error: String,
    },
}

// ─── Node output types (used by condition evaluation & cell modules) ─────────

/// Status of a node's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeOutputStatus {
    /// Node completed successfully.
    Success,
    /// Node failed during execution.
    Failed,
    /// Node was skipped (e.g., budget exceeded, upstream failure).
    Skipped,
}

impl NodeOutputStatus {
    /// Returns `true` if this status represents success.
    #[must_use]
    pub fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    /// Returns `true` if this status represents failure.
    #[must_use]
    pub fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }

    /// Returns `true` if this status represents a skipped node.
    #[must_use]
    pub fn is_skipped(self) -> bool {
        matches!(self, Self::Skipped)
    }
}

/// Output produced by a single node execution, used for condition evaluation
/// and inter-cell communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    /// The node that produced this output.
    pub node_id: NodeId,
    /// Execution status.
    pub status: NodeOutputStatus,
    /// Structured output data (JSON).
    pub data: serde_json::Value,
    /// Error message if status is `Failed` or `Skipped`.
    pub error: Option<String>,
    /// Tokens consumed during execution.
    #[serde(default)]
    pub tokens_used: u64,
    /// Estimated cost in USD.
    #[serde(default)]
    pub cost_usd: f64,
    /// Wall-clock duration of execution.
    #[serde(default, with = "duration_millis")]
    pub duration: Duration,
}

impl NodeOutput {
    /// Create a successful output with the given data.
    pub fn success(node_id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            node_id: node_id.into(),
            status: NodeOutputStatus::Success,
            data,
            error: None,
            tokens_used: 0,
            cost_usd: 0.0,
            duration: Duration::ZERO,
        }
    }

    /// Create a failed output with the given error message.
    pub fn failed(node_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            status: NodeOutputStatus::Failed,
            data: serde_json::Value::Null,
            error: Some(error.into()),
            tokens_used: 0,
            cost_usd: 0.0,
            duration: Duration::ZERO,
        }
    }

    /// Create a skipped output with the given reason.
    pub fn skipped(node_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            status: NodeOutputStatus::Skipped,
            data: serde_json::Value::Null,
            error: Some(reason.into()),
            tokens_used: 0,
            cost_usd: 0.0,
            duration: Duration::ZERO,
        }
    }
}

/// Serde helper: serialize/deserialize `Duration` as milliseconds (u64).
mod duration_millis {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Duration, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_u64(d.as_millis() as u64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(de)?;
        Ok(Duration::from_millis(ms))
    }
}

// ─── Graph-level configuration (used by budget tracker) ─────────────────────

/// Graph-level configuration for resource limits and execution policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphConfig {
    /// Maximum total tokens allowed.
    #[serde(default)]
    pub max_tokens: Option<u64>,
    /// Maximum total cost in USD.
    #[serde(default)]
    pub max_cost_usd: Option<f64>,
    /// Maximum wall-clock time.
    #[serde(default)]
    pub deadline: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_empty_graph() {
        let meta = GraphMetadata {
            name: "test-graph".to_string(),
            ..Default::default()
        };
        let graph = Graph::new(meta);
        assert_eq!(graph.metadata.name, "test-graph");
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn graph_default_policy() {
        let graph = Graph::new(GraphMetadata::default());
        assert_eq!(graph.policy.mode, GraphMode::OneShot);
        assert_eq!(graph.policy.failure_strategy, FailureStrategy::FailFast);
        assert_eq!(graph.policy.max_concurrent_nodes, 4);
        assert!(graph.policy.timeout_ms.is_none());
        assert!(graph.policy.hot.is_none());
        assert!(graph.policy.capabilities.is_empty());
        assert!(graph.lenses.registrations().is_empty());
    }

    #[test]
    fn graph_with_policy_builder() {
        let policy = GraphPolicy {
            mode: GraphMode::Hot,
            failure_strategy: FailureStrategy::SkipFailed,
            max_concurrent_nodes: 8,
            timeout_ms: Some(30_000),
            hot: None,
            capabilities: vec![Capability::ReadFs, Capability::Llm],
        };
        let graph = Graph::new(GraphMetadata::default()).with_policy(policy);
        assert_eq!(graph.policy.mode, GraphMode::Hot);
        assert_eq!(graph.policy.failure_strategy, FailureStrategy::SkipFailed);
        assert_eq!(graph.policy.max_concurrent_nodes, 8);
        assert_eq!(graph.policy.timeout_ms, Some(30_000));
        assert_eq!(graph.policy.capabilities.len(), 2);
    }

    #[test]
    fn add_nodes_and_edges() {
        let mut graph = Graph::new(GraphMetadata {
            name: "pipeline".to_string(),
            ..Default::default()
        });

        let compile_node = Node {
            id: "compile".to_string(),
            cell_type: "gate.compile".to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec!["success".to_string()],
            execution_class: ExecutionClass::default(),
        };
        let test_node = Node {
            id: "test".to_string(),
            cell_type: "gate.test".to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec!["compile.success".to_string()],
            outputs: vec![],
            execution_class: ExecutionClass::default(),
        };

        graph.add_node(compile_node).unwrap();
        graph.add_node(test_node).unwrap();

        let edge = Edge {
            from: "compile".to_string(),
            to: "test".to_string(),
            condition: Some(EdgeCondition::Success),
        };
        graph.add_edge(edge).unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.get_node("compile").is_some());
        assert!(graph.get_node("test").is_some());
        assert!(graph.get_node("nonexistent").is_none());
    }

    #[test]
    fn duplicate_node_errors() {
        let mut graph = Graph::new(GraphMetadata::default());
        let node = Node {
            id: "a".to_string(),
            cell_type: "noop".to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::default(),
        };
        graph.add_node(node.clone()).unwrap();
        let result = graph.add_node(node);
        assert_eq!(result, Err(GraphError::DuplicateNode("a".to_string())));
    }

    #[test]
    fn edge_to_nonexistent_node_errors() {
        let mut graph = Graph::new(GraphMetadata::default());
        let node = Node {
            id: "a".to_string(),
            cell_type: "noop".to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::default(),
        };
        graph.add_node(node).unwrap();

        let edge = Edge {
            from: "a".to_string(),
            to: "b".to_string(),
            condition: None,
        };
        assert_eq!(
            graph.add_edge(edge),
            Err(GraphError::NodeNotFound("b".to_string()))
        );
    }

    #[test]
    fn edge_condition_serde_roundtrip() {
        let cond = EdgeCondition::OutputEquals {
            key: "status".to_string(),
            value: "ok".to_string(),
        };
        let serialized = toml::to_string(&cond).unwrap();
        let deserialized: EdgeCondition = toml::from_str(&serialized).unwrap();
        assert_eq!(cond, deserialized);
    }

    #[test]
    fn node_output_success() {
        let output = NodeOutput::success("n1", serde_json::json!({"result": "ok"}));
        assert!(output.status.is_success());
        assert!(!output.status.is_failed());
        assert!(!output.status.is_skipped());
        assert_eq!(output.node_id, "n1");
        assert!(output.error.is_none());
    }

    #[test]
    fn node_output_failed() {
        let output = NodeOutput::failed("n1", "boom");
        assert!(output.status.is_failed());
        assert!(!output.status.is_success());
        assert_eq!(output.error.as_deref(), Some("boom"));
    }

    #[test]
    fn node_output_skipped() {
        let output = NodeOutput::skipped("n1", "budget exceeded");
        assert!(output.status.is_skipped());
        assert!(!output.status.is_success());
        assert!(!output.status.is_failed());
    }

    // ─── validate_edges tests ─────────────────────────────────────────────────

    use roko_core::TypeSchema;

    use crate::registry::{CellDescriptor, CellRegistry};

    /// Dummy noop cell used only to satisfy the factory requirement.
    struct DummyCell;

    #[async_trait::async_trait]
    impl crate::cell::Cell for DummyCell {
        fn cell_id(&self) -> &str {
            "dummy"
        }
        fn cell_name(&self) -> &str {
            "DummyCell"
        }
        async fn execute(
            &self,
            input: Vec<roko_core::Signal>,
            _ctx: &crate::cell::CellContext,
        ) -> roko_core::error::Result<Vec<roko_core::Signal>> {
            Ok(input)
        }
    }

    fn make_test_node(id: &str, cell_type: &str) -> Node {
        Node {
            id: id.to_string(),
            cell_type: cell_type.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec![],
            execution_class: ExecutionClass::default(),
        }
    }

    fn make_test_edge(from: &str, to: &str) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
            condition: None,
        }
    }

    #[test]
    fn validate_edges_no_schemas_always_valid() {
        let mut registry = CellRegistry::new();
        // Untyped descriptor (no schemas) -- always valid.
        registry.register("untyped", |_| Box::new(DummyCell));

        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_test_node("a", "untyped")).unwrap();
        graph.add_node(make_test_node("b", "untyped")).unwrap();
        graph.add_edge(make_test_edge("a", "b")).unwrap();

        let errors = graph.validate_edges(&registry);
        assert!(
            errors.is_empty(),
            "untyped cells should always be valid: {errors:?}"
        );
    }

    #[test]
    fn validate_edges_incompatible_schemas_produces_error() {
        let mut registry = CellRegistry::new();
        registry.register_with_descriptor(
            "task-out",
            CellDescriptor::new(
                "task-out",
                (0, 1, 0),
                None,
                Some(TypeSchema::OfKind(roko_core::Kind::Task)),
            ),
            |_| Box::new(DummyCell),
        );
        registry.register_with_descriptor(
            "episode-in",
            CellDescriptor::new(
                "episode-in",
                (0, 1, 0),
                Some(TypeSchema::OfKind(roko_core::Kind::Episode)),
                None,
            ),
            |_| Box::new(DummyCell),
        );

        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_test_node("src", "task-out")).unwrap();
        graph.add_node(make_test_node("tgt", "episode-in")).unwrap();
        graph.add_edge(make_test_edge("src", "tgt")).unwrap();

        let errors = graph.validate_edges(&registry);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].edge_from, "src");
        assert_eq!(errors[0].edge_to, "tgt");
    }

    #[test]
    fn validate_edges_missing_cell_type_produces_error() {
        let registry = CellRegistry::new(); // empty

        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_test_node("a", "unknown-type")).unwrap();
        graph.add_node(make_test_node("b", "unknown-type")).unwrap();
        graph.add_edge(make_test_edge("a", "b")).unwrap();

        let errors = graph.validate_edges(&registry);
        assert_eq!(
            errors.len(),
            1,
            "one error for the missing source cell type"
        );
        assert!(errors[0].reason.contains("not in registry"));
    }

    #[test]
    fn validate_edges_compatible_same_kind_no_error() {
        let mut registry = CellRegistry::new();
        registry.register_with_descriptor(
            "task-out",
            CellDescriptor::new(
                "task-out",
                (0, 1, 0),
                None,
                Some(TypeSchema::OfKind(roko_core::Kind::Task)),
            ),
            |_| Box::new(DummyCell),
        );
        registry.register_with_descriptor(
            "task-in",
            CellDescriptor::new(
                "task-in",
                (0, 1, 0),
                Some(TypeSchema::OfKind(roko_core::Kind::Task)),
                None,
            ),
            |_| Box::new(DummyCell),
        );

        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_test_node("src", "task-out")).unwrap();
        graph.add_node(make_test_node("tgt", "task-in")).unwrap();
        graph.add_edge(make_test_edge("src", "tgt")).unwrap();

        let errors = graph.validate_edges(&registry);
        assert!(
            errors.is_empty(),
            "same Kind should be compatible: {errors:?}"
        );
    }

    #[test]
    fn validate_edges_collects_all_errors() {
        let mut registry = CellRegistry::new();
        registry.register_with_descriptor(
            "task-out",
            CellDescriptor::new(
                "task-out",
                (0, 1, 0),
                None,
                Some(TypeSchema::OfKind(roko_core::Kind::Task)),
            ),
            |_| Box::new(DummyCell),
        );
        registry.register_with_descriptor(
            "episode-in",
            CellDescriptor::new(
                "episode-in",
                (0, 1, 0),
                Some(TypeSchema::OfKind(roko_core::Kind::Episode)),
                None,
            ),
            |_| Box::new(DummyCell),
        );

        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_test_node("a", "task-out")).unwrap();
        graph.add_node(make_test_node("b", "episode-in")).unwrap();
        graph.add_node(make_test_node("c", "episode-in")).unwrap();
        graph.add_edge(make_test_edge("a", "b")).unwrap();
        graph.add_edge(make_test_edge("a", "c")).unwrap();

        let errors = graph.validate_edges(&registry);
        assert_eq!(
            errors.len(),
            2,
            "both incompatible edges should be reported"
        );
    }
}
