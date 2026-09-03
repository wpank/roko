//! Graph node-to-plan/task identity map (#248).
//!
//! [`GraphIdentityMap`] is built from the converted graph before execution and
//! maps `node_id -> { plan_id, task_id, title, role, wave_index }`. The adapter
//! in [`super::runtime_event_adapter`] uses this map to stamp plan/task
//! identity onto canonical [`RuntimeEventEnvelope`] outputs.
//!
//! Unknown authored nodes (absent from the map) use `plan_id = graph_id`,
//! empty task ID, and title `"graph node <node_id>"`.
//!
//! [`RuntimeEventEnvelope`]: roko_core::runtime_event::RuntimeEventEnvelope

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Identity entry
// ---------------------------------------------------------------------------

/// Identity metadata for a single graph node, resolved before execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeIdentity {
    /// Plan identifier this node belongs to.
    pub plan_id: String,
    /// Task identifier within the plan (empty for non-task nodes).
    pub task_id: String,
    /// Human-readable title for dashboard display.
    pub title: String,
    /// Role label (e.g. `"implementer"`, `"researcher"`).
    pub role: String,
    /// Zero-based wave index in the topological sort.
    pub wave_index: u32,
}

// ---------------------------------------------------------------------------
// Lightweight task entry (avoids coupling to PlanTaskInfo)
// ---------------------------------------------------------------------------

/// Minimal task metadata passed when building the identity map.
///
/// Callers convert from `roko_graph::convert::PlanTaskInfo` at the call site
/// so this module remains free of a direct `PlanTaskInfo` dependency.
#[derive(Debug, Clone)]
pub struct TaskEntry {
    /// Task ID within the plan.
    pub task_id: String,
    /// Human-readable title.
    pub title: String,
    /// Role label.
    pub role: String,
}

impl TaskEntry {
    /// Create a new task entry.
    #[must_use]
    pub fn new(
        task_id: impl Into<String>,
        title: impl Into<String>,
        role: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            title: title.into(),
            role: role.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Identity map
// ---------------------------------------------------------------------------

/// Maps `node_id -> NodeIdentity`, built from the converted plan graph before
/// execution. Immutable after construction via `from_plan_tasks`.
///
/// Also maintains a reverse lookup (`(plan_slug, task_id) -> node_id`)
/// for the runner-v2 bridge path that needs the opposite direction.
#[derive(Debug, Clone, Default)]
pub struct GraphIdentityMap {
    /// Fallback plan identifier used for unknown nodes.
    graph_id: String,
    /// Node-ID-keyed identity entries.
    entries: HashMap<String, NodeIdentity>,
    /// Runner task key `(plan_slug, task_id)` -> graph node ID.
    runner_to_graph: HashMap<(String, String), String>,
    /// Graph node ID -> runner task key.
    graph_to_runner: HashMap<String, (String, String)>,
}

impl GraphIdentityMap {
    /// Create an empty map with the given graph-level fallback identifier.
    #[must_use]
    pub fn new(graph_id: impl Into<String>) -> Self {
        Self {
            graph_id: graph_id.into(),
            entries: HashMap::new(),
            runner_to_graph: HashMap::new(),
            graph_to_runner: HashMap::new(),
        }
    }

    /// Build from an iterator of `(node_id, TaskEntry)` pairs produced by
    /// the plan-to-graph conversion.
    ///
    /// `wave_assignments` supplies the zero-based wave index for each node.
    /// Nodes absent from `wave_assignments` default to wave 0.
    pub fn from_plan_tasks(
        graph_id: impl Into<String>,
        plan_id: impl Into<String>,
        tasks: impl IntoIterator<Item = (String, TaskEntry)>,
        wave_assignments: &HashMap<String, u32>,
    ) -> Self {
        let plan_id = plan_id.into();
        let graph_id = graph_id.into();
        let mut entries = HashMap::new();
        let mut runner_to_graph = HashMap::new();
        let mut graph_to_runner = HashMap::new();

        for (node_id, entry) in tasks {
            let wave_index = wave_assignments.get(&node_id).copied().unwrap_or(0);
            let identity = NodeIdentity {
                plan_id: plan_id.clone(),
                task_id: entry.task_id.clone(),
                title: entry.title,
                role: entry.role,
                wave_index,
            };
            let runner_key = (plan_id.clone(), entry.task_id);
            runner_to_graph.insert(runner_key.clone(), node_id.clone());
            graph_to_runner.insert(node_id.clone(), runner_key);
            entries.insert(node_id, identity);
        }

        Self {
            graph_id,
            entries,
            runner_to_graph,
            graph_to_runner,
        }
    }

    /// Look up identity for a node. Returns `None` for unknown nodes.
    #[must_use]
    pub fn get(&self, node_id: &str) -> Option<&NodeIdentity> {
        self.entries.get(node_id)
    }

    /// Look up identity, returning a fallback for unknown authored nodes.
    ///
    /// The fallback uses `plan_id = graph_id`, empty task ID, and
    /// `title = "graph node <node_id>"`.
    #[must_use]
    pub fn get_or_fallback(&self, node_id: &str) -> NodeIdentity {
        if let Some(identity) = self.entries.get(node_id) {
            identity.clone()
        } else {
            NodeIdentity {
                plan_id: self.graph_id.clone(),
                task_id: String::new(),
                title: format!("graph node {node_id}"),
                role: String::new(),
                wave_index: 0,
            }
        }
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

    /// Register a single mapping between a runner task and a graph node.
    pub fn insert_mapping(&mut self, plan_slug: &str, task_id: &str, graph_node_id: &str) {
        let runner_key = (plan_slug.to_string(), task_id.to_string());
        let graph_key = graph_node_id.to_string();
        self.runner_to_graph
            .insert(runner_key.clone(), graph_key.clone());
        self.graph_to_runner.insert(graph_key, runner_key);
    }

    /// Insert or replace a node identity entry.
    pub fn insert(&mut self, node_id: impl Into<String>, identity: NodeIdentity) {
        let node_id = node_id.into();
        let runner_key = (identity.plan_id.clone(), identity.task_id.clone());
        self.runner_to_graph
            .insert(runner_key.clone(), node_id.clone());
        self.graph_to_runner.insert(node_id.clone(), runner_key);
        self.entries.insert(node_id, identity);
    }

    /// The graph-level identifier used for fallback plan IDs.
    #[must_use]
    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    /// Number of known node identities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the map is empty (no known nodes).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tasks() -> Vec<(String, TaskEntry)> {
        vec![
            (
                "T01".to_string(),
                TaskEntry::new("compile", "Compile the project", "implementer"),
            ),
            (
                "T02".to_string(),
                TaskEntry::new("test", "Run test suite", "tester"),
            ),
            (
                "T03".to_string(),
                TaskEntry::new("lint", "Run clippy", "reviewer"),
            ),
        ]
    }

    fn sample_waves() -> HashMap<String, u32> {
        [
            ("T01".to_string(), 0),
            ("T02".to_string(), 1),
            ("T03".to_string(), 1),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn build_from_plan_tasks() {
        let map = GraphIdentityMap::from_plan_tasks(
            "graph-1",
            "plan-alpha",
            sample_tasks(),
            &sample_waves(),
        );

        assert_eq!(map.len(), 3);
        assert!(!map.is_empty());
        assert_eq!(map.graph_id(), "graph-1");

        let t01 = map.get("T01").expect("T01 should exist");
        assert_eq!(t01.plan_id, "plan-alpha");
        assert_eq!(t01.task_id, "compile");
        assert_eq!(t01.title, "Compile the project");
        assert_eq!(t01.role, "implementer");
        assert_eq!(t01.wave_index, 0);

        let t02 = map.get("T02").expect("T02 should exist");
        assert_eq!(t02.wave_index, 1);
    }

    #[test]
    fn fallback_for_unknown_node() {
        let map = GraphIdentityMap::from_plan_tasks(
            "graph-1",
            "plan-alpha",
            sample_tasks(),
            &sample_waves(),
        );

        assert!(map.get("T99").is_none());

        let fallback = map.get_or_fallback("T99");
        assert_eq!(fallback.plan_id, "graph-1");
        assert_eq!(fallback.task_id, "");
        assert_eq!(fallback.title, "graph node T99");
        assert_eq!(fallback.role, "");
        assert_eq!(fallback.wave_index, 0);
    }

    #[test]
    fn empty_map_fallback() {
        let map = GraphIdentityMap::new("standalone-graph");
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);

        let fallback = map.get_or_fallback("any-node");
        assert_eq!(fallback.plan_id, "standalone-graph");
        assert_eq!(fallback.title, "graph node any-node");
    }

    #[test]
    fn bidirectional_lookup() {
        let map = GraphIdentityMap::from_plan_tasks(
            "g1",
            "plan-a",
            vec![
                ("N1".to_string(), TaskEntry::new("T1", "First", "impl")),
                ("N2".to_string(), TaskEntry::new("T2", "Second", "test")),
            ],
            &HashMap::new(),
        );

        assert_eq!(map.graph_node_id("plan-a", "T1"), Some("N1"));
        assert_eq!(map.graph_node_id("plan-a", "T2"), Some("N2"));
        assert_eq!(map.runner_task_key("N1"), Some(("plan-a", "T1")));
        assert_eq!(map.runner_task_key("N2"), Some(("plan-a", "T2")));
        assert!(map.graph_node_id("plan-a", "T99").is_none());
        assert!(map.runner_task_key("N99").is_none());
    }

    #[test]
    fn insert_updates_both_directions() {
        let mut map = GraphIdentityMap::new("g1");
        let identity = NodeIdentity {
            plan_id: "p1".to_string(),
            task_id: "t1".to_string(),
            title: "Original".to_string(),
            role: "impl".to_string(),
            wave_index: 0,
        };
        map.insert("N1", identity);
        assert_eq!(map.get("N1").unwrap().title, "Original");
        assert_eq!(map.graph_node_id("p1", "t1"), Some("N1"));
        assert_eq!(map.runner_task_key("N1"), Some(("p1", "t1")));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn missing_wave_defaults_to_zero() {
        let waves: HashMap<String, u32> = HashMap::new();
        let map = GraphIdentityMap::from_plan_tasks(
            "g1",
            "p1",
            vec![("N1".to_string(), TaskEntry::new("t1", "Task", "impl"))],
            &waves,
        );
        assert_eq!(map.get("N1").unwrap().wave_index, 0);
    }

    #[test]
    fn node_identity_serde_roundtrip() {
        let identity = NodeIdentity {
            plan_id: "plan-x".to_string(),
            task_id: "task-y".to_string(),
            title: "Compile step".to_string(),
            role: "implementer".to_string(),
            wave_index: 2,
        };
        let json = serde_json::to_string(&identity).expect("serialize");
        let deser: NodeIdentity = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(identity, deser);
    }
}
