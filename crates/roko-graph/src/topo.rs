//! Topological sort: order graph nodes for execution, detect cycles, resolve dependencies.

use petgraph::Direction;
use petgraph::algo::toposort;

use crate::types::{Graph, GraphError, NodeId};

/// Perform a topological sort on the graph, returning node IDs in execution order.
///
/// Nodes with no dependencies come first; nodes that depend on others come later.
///
/// # Errors
/// Returns `GraphError::CycleDetected` if the graph contains a cycle.
pub fn topological_order(graph: &Graph) -> Result<Vec<NodeId>, GraphError> {
    let sorted = toposort(&graph.inner, None).map_err(|_| GraphError::CycleDetected)?;

    Ok(sorted
        .into_iter()
        .map(|idx| graph.inner[idx].id.clone())
        .collect())
}

/// Return the immediate dependencies (predecessors) of a node.
///
/// These are the nodes whose edges point *to* the given node.
#[must_use]
pub fn dependencies(graph: &Graph, node_id: &str) -> Vec<NodeId> {
    let Some(&idx) = graph.node_map.get(node_id) else {
        return vec![];
    };

    graph
        .inner
        .neighbors_directed(idx, Direction::Incoming)
        .map(|pred_idx| graph.inner[pred_idx].id.clone())
        .collect()
}

/// Return the immediate dependents (successors) of a node.
///
/// These are the nodes that depend on the given node.
#[must_use]
pub fn dependents(graph: &Graph, node_id: &str) -> Vec<NodeId> {
    let Some(&idx) = graph.node_map.get(node_id) else {
        return vec![];
    };

    graph
        .inner
        .neighbors_directed(idx, Direction::Outgoing)
        .map(|succ_idx| graph.inner[succ_idx].id.clone())
        .collect()
}

/// Check if the graph is a valid DAG (no cycles).
#[must_use]
pub fn is_dag(graph: &Graph) -> bool {
    toposort(&graph.inner, None).is_ok()
}

/// Return nodes that have no incoming edges (root/entry nodes).
#[must_use]
pub fn root_nodes(graph: &Graph) -> Vec<NodeId> {
    graph
        .node_map
        .iter()
        .filter(|(_, idx)| {
            graph
                .inner
                .neighbors_directed(**idx, Direction::Incoming)
                .next()
                .is_none()
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Group topologically sorted nodes into parallel execution waves.
///
/// Within each wave, all nodes are independent of each other (no edges between
/// them). Nodes in wave N+1 depend only on nodes in waves 0..N. This makes it
/// safe to execute all nodes within a single wave concurrently.
///
/// Returns a `Vec<Vec<NodeId>>` where each inner `Vec` is one wave.
///
/// # Errors
/// Returns `GraphError::CycleDetected` if the graph contains a cycle.
pub fn topological_waves(graph: &Graph) -> Result<Vec<Vec<NodeId>>, GraphError> {
    use std::collections::HashMap;

    // Compute the longest-path depth for each node. Roots have depth 0.
    let order = topological_order(graph)?;

    let mut depth: HashMap<&str, usize> = HashMap::new();
    let mut max_depth: usize = 0;

    for node_id in &order {
        let Some(&idx) = graph.node_map.get(node_id) else {
            continue;
        };

        // Depth = max(depth of predecessors) + 1, or 0 if no predecessors.
        let mut node_depth = 0;
        for pred_idx in graph.inner.neighbors_directed(idx, Direction::Incoming) {
            let pred_id = &graph.inner[pred_idx].id;
            if let Some(&pd) = depth.get(pred_id.as_str()) {
                node_depth = node_depth.max(pd + 1);
            }
        }
        depth.insert(node_id, node_depth);
        max_depth = max_depth.max(node_depth);
    }

    // Group nodes by depth level.
    let mut waves: Vec<Vec<NodeId>> = vec![Vec::new(); max_depth + 1];
    for node_id in &order {
        if let Some(&d) = depth.get(node_id.as_str()) {
            waves[d].push(node_id.clone());
        }
    }

    // Remove empty waves (shouldn't happen, but be safe).
    waves.retain(|w| !w.is_empty());

    Ok(waves)
}

/// Return nodes that have no outgoing edges (leaf/terminal nodes).
#[must_use]
pub fn leaf_nodes(graph: &Graph) -> Vec<NodeId> {
    graph
        .node_map
        .iter()
        .filter(|(_, idx)| {
            graph
                .inner
                .neighbors_directed(**idx, Direction::Outgoing)
                .next()
                .is_none()
        })
        .map(|(id, _)| id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::types::{Edge, GraphMetadata, Node};

    use super::*;

    fn make_node(id: &str) -> Node {
        Node {
            id: id.to_string(),
            cell_type: "noop".to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec![],
            outputs: vec![],
            execution_class: crate::types::ExecutionClass::default(),
        }
    }

    fn make_edge(from: &str, to: &str) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
            condition: None,
        }
    }

    #[test]
    fn topological_sort_linear_chain() {
        let mut graph = Graph::new(GraphMetadata {
            name: "chain".to_string(),
            ..Default::default()
        });
        graph.add_node(make_node("a")).unwrap();
        graph.add_node(make_node("b")).unwrap();
        graph.add_node(make_node("c")).unwrap();
        graph.add_edge(make_edge("a", "b")).unwrap();
        graph.add_edge(make_edge("b", "c")).unwrap();

        let order = topological_order(&graph).unwrap();
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn topological_sort_diamond() {
        // a -> b, a -> c, b -> d, c -> d
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("a")).unwrap();
        graph.add_node(make_node("b")).unwrap();
        graph.add_node(make_node("c")).unwrap();
        graph.add_node(make_node("d")).unwrap();
        graph.add_edge(make_edge("a", "b")).unwrap();
        graph.add_edge(make_edge("a", "c")).unwrap();
        graph.add_edge(make_edge("b", "d")).unwrap();
        graph.add_edge(make_edge("c", "d")).unwrap();

        let order = topological_order(&graph).unwrap();
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();
        let pos_d = order.iter().position(|x| x == "d").unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
    }

    #[test]
    fn topological_sort_detects_cycle() {
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("a")).unwrap();
        graph.add_node(make_node("b")).unwrap();
        graph.add_edge(make_edge("a", "b")).unwrap();
        graph.add_edge(make_edge("b", "a")).unwrap();

        let result = topological_order(&graph);
        assert_eq!(result, Err(GraphError::CycleDetected));
    }

    #[test]
    fn is_dag_true_for_valid_graph() {
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("x")).unwrap();
        graph.add_node(make_node("y")).unwrap();
        graph.add_edge(make_edge("x", "y")).unwrap();
        assert!(is_dag(&graph));
    }

    #[test]
    fn is_dag_false_for_cyclic_graph() {
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("x")).unwrap();
        graph.add_node(make_node("y")).unwrap();
        graph.add_edge(make_edge("x", "y")).unwrap();
        graph.add_edge(make_edge("y", "x")).unwrap();
        assert!(!is_dag(&graph));
    }

    #[test]
    fn dependencies_and_dependents() {
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("a")).unwrap();
        graph.add_node(make_node("b")).unwrap();
        graph.add_node(make_node("c")).unwrap();
        graph.add_edge(make_edge("a", "b")).unwrap();
        graph.add_edge(make_edge("a", "c")).unwrap();

        let deps_of_b = dependencies(&graph, "b");
        assert_eq!(deps_of_b, vec!["a"]);

        let mut deps_of_a = dependents(&graph, "a");
        deps_of_a.sort();
        assert_eq!(deps_of_a, vec!["b", "c"]);

        // Non-existent node returns empty.
        assert!(dependencies(&graph, "nonexistent").is_empty());
        assert!(dependents(&graph, "nonexistent").is_empty());
    }

    #[test]
    fn root_and_leaf_nodes() {
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("a")).unwrap();
        graph.add_node(make_node("b")).unwrap();
        graph.add_node(make_node("c")).unwrap();
        graph.add_edge(make_edge("a", "b")).unwrap();
        graph.add_edge(make_edge("b", "c")).unwrap();

        let roots = root_nodes(&graph);
        assert_eq!(roots, vec!["a"]);

        let leaves = leaf_nodes(&graph);
        assert_eq!(leaves, vec!["c"]);
    }

    #[test]
    fn topological_waves_linear() {
        // a -> b -> c should produce 3 waves of 1 node each
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("a")).unwrap();
        graph.add_node(make_node("b")).unwrap();
        graph.add_node(make_node("c")).unwrap();
        graph.add_edge(make_edge("a", "b")).unwrap();
        graph.add_edge(make_edge("b", "c")).unwrap();

        let waves = topological_waves(&graph).unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0], vec!["a"]);
        assert_eq!(waves[1], vec!["b"]);
        assert_eq!(waves[2], vec!["c"]);
    }

    #[test]
    fn topological_waves_diamond() {
        // a -> b, a -> c, b -> d, c -> d
        // Wave 0: [a], Wave 1: [b, c], Wave 2: [d]
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("a")).unwrap();
        graph.add_node(make_node("b")).unwrap();
        graph.add_node(make_node("c")).unwrap();
        graph.add_node(make_node("d")).unwrap();
        graph.add_edge(make_edge("a", "b")).unwrap();
        graph.add_edge(make_edge("a", "c")).unwrap();
        graph.add_edge(make_edge("b", "d")).unwrap();
        graph.add_edge(make_edge("c", "d")).unwrap();

        let waves = topological_waves(&graph).unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0], vec!["a"]);
        assert_eq!(waves[1].len(), 2);
        assert!(waves[1].contains(&"b".to_string()));
        assert!(waves[1].contains(&"c".to_string()));
        assert_eq!(waves[2], vec!["d"]);
    }

    #[test]
    fn topological_waves_independent_roots() {
        // Two independent nodes should be in the same wave
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("x")).unwrap();
        graph.add_node(make_node("y")).unwrap();

        let waves = topological_waves(&graph).unwrap();
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].len(), 2);
    }

    #[test]
    fn single_node_graph() {
        let mut graph = Graph::new(GraphMetadata::default());
        graph.add_node(make_node("solo")).unwrap();

        let order = topological_order(&graph).unwrap();
        assert_eq!(order, vec!["solo"]);
        assert_eq!(root_nodes(&graph), vec!["solo"]);
        assert_eq!(leaf_nodes(&graph), vec!["solo"]);
        assert!(is_dag(&graph));
    }
}
