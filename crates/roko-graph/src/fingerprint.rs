//! Stable identity for execution-relevant Graph definitions.

use std::collections::BTreeMap;

use roko_core::ContentHash;
use serde::Serialize;

use crate::types::Graph;

const FINGERPRINT_SCHEMA_VERSION: u32 = 1;

/// Compute a stable BLAKE3 identity for the execution-relevant parts of a Graph.
///
/// Node and edge insertion order and metadata-label map order do not affect the
/// result. Checkpoint callers use this identity to reject replay after graph
/// definition or policy drift.
pub fn graph_execution_fingerprint(graph: &Graph) -> Result<String, serde_json::Error> {
    #[derive(Serialize)]
    struct Fingerprint<'a> {
        schema: u32,
        name: &'a str,
        description: &'a Option<String>,
        version: &'a Option<String>,
        labels: BTreeMap<&'a str, &'a str>,
        policy: &'a crate::types::GraphPolicy,
        nodes: Vec<&'a crate::types::Node>,
        edges: Vec<&'a crate::types::Edge>,
    }

    let mut nodes: Vec<_> = graph.inner.node_weights().collect();
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut edges: Vec<_> = graph.inner.edge_weights().collect();
    edges.sort_by(|left, right| {
        left.from
            .cmp(&right.from)
            .then_with(|| left.to.cmp(&right.to))
            .then_with(|| format!("{:?}", left.condition).cmp(&format!("{:?}", right.condition)))
    });
    let identity = Fingerprint {
        schema: FINGERPRINT_SCHEMA_VERSION,
        name: &graph.metadata.name,
        description: &graph.metadata.description,
        version: &graph.metadata.version,
        labels: graph
            .metadata
            .labels
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect(),
        policy: &graph.policy,
        nodes,
        edges,
    };
    let encoded = serde_json::to_vec(&identity)?;
    Ok(ContentHash::of(&encoded).to_hex())
}

#[cfg(test)]
mod tests {
    use crate::types::{ExecutionClass, GraphMetadata, Node};

    use super::*;

    fn graph(config_value: i64) -> Graph {
        let mut graph = Graph::new(GraphMetadata {
            name: "stable".to_string(),
            labels: [
                ("z".to_string(), "last".to_string()),
                ("a".to_string(), "first".to_string()),
            ]
            .into_iter()
            .collect(),
            ..GraphMetadata::default()
        });
        graph
            .add_node(Node {
                id: "node".to_string(),
                cell_type: "noop".to_string(),
                config: toml::Value::Table(toml::map::Map::from_iter([(
                    "value".to_string(),
                    toml::Value::Integer(config_value),
                )])),
                inputs: Vec::new(),
                outputs: Vec::new(),
                execution_class: ExecutionClass::Workflow,
            })
            .expect("node");
        graph
    }

    #[test]
    fn fingerprint_is_stable_and_sensitive_to_execution_config() {
        let first = graph_execution_fingerprint(&graph(1)).expect("fingerprint");
        assert_eq!(first, graph_execution_fingerprint(&graph(1)).unwrap());
        assert_ne!(first, graph_execution_fingerprint(&graph(2)).unwrap());
    }
}
