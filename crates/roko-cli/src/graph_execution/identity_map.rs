//! Graph identity mapping between runner-v2 task/plan IDs and Graph node IDs.
//!
//! The runner-v2 event loop uses string-based plan/task identifiers while
//! the Graph engine uses typed node identifiers. This module provides the
//! bidirectional mapping without coupling either layer to the other's ID
//! scheme.

use std::collections::HashMap;

/// Bidirectional map between runner-v2 task IDs and Graph node identifiers.
///
/// Runner-v2 uses `(plan_slug, task_id)` pairs. Graph uses opaque `NodeId`
/// strings. This map allows both sides to look up the other's identifier
/// without knowing its format.
#[derive(Debug, Clone, Default)]
pub struct GraphIdentityMap {
    /// Runner task key -> Graph node ID.
    runner_to_graph: HashMap<(String, String), String>,
    /// Graph node ID -> Runner task key.
    graph_to_runner: HashMap<String, (String, String)>,
}

impl GraphIdentityMap {
    /// Create an empty identity map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a mapping between a runner task and a graph node.
    pub fn insert(&mut self, plan_slug: &str, task_id: &str, graph_node_id: &str) {
        let runner_key = (plan_slug.to_string(), task_id.to_string());
        let graph_key = graph_node_id.to_string();
        self.runner_to_graph
            .insert(runner_key.clone(), graph_key.clone());
        self.graph_to_runner.insert(graph_key, runner_key);
    }

    /// Look up the Graph node ID for a runner task.
    #[must_use]
    pub fn graph_node_id(&self, plan_slug: &str, task_id: &str) -> Option<&str> {
        let key = (plan_slug.to_string(), task_id.to_string());
        self.runner_to_graph.get(&key).map(String::as_str)
    }

    /// Look up the runner task key for a Graph node ID.
    #[must_use]
    pub fn runner_task_key(&self, graph_node_id: &str) -> Option<(&str, &str)> {
        self.graph_to_runner
            .get(graph_node_id)
            .map(|(slug, task_id)| (slug.as_str(), task_id.as_str()))
    }

    /// Number of registered mappings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.runner_to_graph.len()
    }

    /// Whether the map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.runner_to_graph.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_mapping() {
        let mut map = GraphIdentityMap::new();
        map.insert("my-plan", "T1", "node-abc-123");

        assert_eq!(map.graph_node_id("my-plan", "T1"), Some("node-abc-123"));
        assert_eq!(
            map.runner_task_key("node-abc-123"),
            Some(("my-plan", "T1"))
        );
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn missing_lookup_returns_none() {
        let map = GraphIdentityMap::new();
        assert!(map.graph_node_id("missing", "T1").is_none());
        assert!(map.runner_task_key("missing-node").is_none());
        assert!(map.is_empty());
    }

    #[test]
    fn multiple_entries() {
        let mut map = GraphIdentityMap::new();
        map.insert("plan-a", "T1", "node-1");
        map.insert("plan-a", "T2", "node-2");
        map.insert("plan-b", "T1", "node-3");

        assert_eq!(map.len(), 3);
        assert_eq!(map.graph_node_id("plan-a", "T1"), Some("node-1"));
        assert_eq!(map.graph_node_id("plan-a", "T2"), Some("node-2"));
        assert_eq!(map.graph_node_id("plan-b", "T1"), Some("node-3"));
        assert_eq!(map.runner_task_key("node-2"), Some(("plan-a", "T2")));
    }
}
