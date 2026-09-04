//! Graph node identity resolution for canonical runtime event emission (#248).
//!
//! [`GraphIdentityMap`] maps graph `node_id` values to plan/task metadata so
//! the [`GraphRuntimeEventAdapter`] can populate [`RuntimeEventEnvelope`]
//! fields (`plan_id`, `task_id`, `node_id`) without requiring the graph engine
//! to know about plan-layer identity.
//!
//! The map is built once from the converted graph before execution and is
//! immutable thereafter.
//!
//! [`GraphRuntimeEventAdapter`]: super::runtime_event_adapter::GraphRuntimeEventAdapter
//! [`RuntimeEventEnvelope`]: roko_core::runtime_event::RuntimeEventEnvelope

use std::collections::HashMap;

use roko_graph::convert::PlanTaskInfo;

// ---------------------------------------------------------------------------
// Identity record
// ---------------------------------------------------------------------------

/// Resolved identity for a single graph node.
#[derive(Debug, Clone)]
pub struct NodeIdentity {
    /// Plan identifier (from the plan directory name or graph metadata).
    pub plan_id: String,
    /// Task identifier (the original task ID from `tasks.toml`).
    pub task_id: String,
    /// Human-readable task title.
    pub title: String,
    /// Role (implementer, researcher, reviewer, etc.), if defined.
    pub role: Option<String>,
    /// Zero-based wave index assigned during topological sorting.
    pub wave_index: u32,
}

// ---------------------------------------------------------------------------
// Identity map
// ---------------------------------------------------------------------------

/// Immutable map from graph `node_id` to plan/task identity.
///
/// Built once before execution and queried by [`GraphRuntimeEventAdapter`]
/// on every event. Unknown `node_id` values are handled gracefully with
/// graph-scoped fallback labels.
///
/// [`GraphRuntimeEventAdapter`]: super::runtime_event_adapter::GraphRuntimeEventAdapter
#[derive(Debug, Clone)]
pub struct GraphIdentityMap {
    /// The plan-level identifier (e.g. the plan directory name).
    graph_plan_id: String,
    /// Per-node identity records.
    entries: HashMap<String, NodeIdentity>,
}

impl GraphIdentityMap {
    /// Build the identity map from converted plan tasks and their wave assignments.
    ///
    /// `plan_id` is the plan-level identifier (typically the plan directory name).
    /// `tasks` is the `(node_id, PlanTaskInfo)` pairs from conversion.
    /// `wave_assignments` maps `node_id` to zero-based wave index; nodes without
    /// an assignment default to wave 0.
    pub fn build(
        plan_id: &str,
        tasks: &[(String, PlanTaskInfo)],
        wave_assignments: &HashMap<String, u32>,
    ) -> Self {
        let mut entries = HashMap::with_capacity(tasks.len());
        for (node_id, info) in tasks {
            let wave_index = wave_assignments.get(node_id).copied().unwrap_or(0);
            entries.insert(
                node_id.clone(),
                NodeIdentity {
                    plan_id: plan_id.to_string(),
                    task_id: node_id.clone(),
                    title: info.title.clone(),
                    role: info.role.clone(),
                    wave_index,
                },
            );
        }
        Self {
            graph_plan_id: plan_id.to_string(),
            entries,
        }
    }

    /// Look up the identity for a graph node.
    ///
    /// Returns `None` for unknown authored-graph nodes; callers should use
    /// [`fallback_identity`] to construct a graph-scoped label.
    ///
    /// [`fallback_identity`]: Self::fallback_identity
    pub fn get(&self, node_id: &str) -> Option<&NodeIdentity> {
        self.entries.get(node_id)
    }

    /// Construct a fallback identity for an unknown graph node.
    ///
    /// Uses `plan_id = graph_id` (the graph-level identifier), no task ID,
    /// and title `graph node <node_id>`.
    pub fn fallback_identity(&self, node_id: &str) -> NodeIdentity {
        NodeIdentity {
            plan_id: self.graph_plan_id.clone(),
            task_id: String::new(),
            title: format!("graph node {node_id}"),
            role: None,
            wave_index: 0,
        }
    }

    /// The plan-level identifier for the graph.
    pub fn plan_id(&self) -> &str {
        &self.graph_plan_id
    }

    /// Total number of known nodes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all known `(node_id, identity)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &NodeIdentity)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task_info(title: &str, role: Option<&str>) -> PlanTaskInfo {
        PlanTaskInfo {
            title: title.to_string(),
            description: None,
            role: role.map(|r| r.to_string()),
            tier: "focused".to_string(),
            model_hint: None,
            files: Vec::new(),
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            timeout_secs: 60,
            max_retries: 0,
            domain: None,
            sequence: 0,
            full_config_json: serde_json::Value::Null,
        }
    }

    #[test]
    fn build_and_lookup() {
        let tasks = vec![
            (
                "T01".to_string(),
                make_task_info("Compile", Some("implementer")),
            ),
            ("T02".to_string(), make_task_info("Test", Some("reviewer"))),
        ];
        let waves: HashMap<String, u32> = [("T01".to_string(), 0), ("T02".to_string(), 1)]
            .into_iter()
            .collect();

        let map = GraphIdentityMap::build("my-plan", &tasks, &waves);
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
        assert_eq!(map.plan_id(), "my-plan");

        let t01 = map.get("T01").expect("T01 should exist");
        assert_eq!(t01.plan_id, "my-plan");
        assert_eq!(t01.task_id, "T01");
        assert_eq!(t01.title, "Compile");
        assert_eq!(t01.role.as_deref(), Some("implementer"));
        assert_eq!(t01.wave_index, 0);

        let t02 = map.get("T02").expect("T02 should exist");
        assert_eq!(t02.wave_index, 1);
    }

    #[test]
    fn unknown_node_returns_none() {
        let map = GraphIdentityMap::build("plan", &[], &HashMap::new());
        assert!(map.get("unknown").is_none());
    }

    #[test]
    fn fallback_identity_uses_graph_scoped_label() {
        let map = GraphIdentityMap::build("graph-42", &[], &HashMap::new());
        let fallback = map.fallback_identity("mystery-node");
        assert_eq!(fallback.plan_id, "graph-42");
        assert!(fallback.task_id.is_empty());
        assert_eq!(fallback.title, "graph node mystery-node");
        assert!(fallback.role.is_none());
        assert_eq!(fallback.wave_index, 0);
    }

    #[test]
    fn missing_wave_assignment_defaults_to_zero() {
        let tasks = vec![("T01".to_string(), make_task_info("First", None))];
        // No wave assignment for T01.
        let map = GraphIdentityMap::build("plan", &tasks, &HashMap::new());
        let t01 = map.get("T01").unwrap();
        assert_eq!(t01.wave_index, 0);
    }

    #[test]
    fn iter_yields_all_entries() {
        let tasks = vec![
            ("A".to_string(), make_task_info("Alpha", None)),
            ("B".to_string(), make_task_info("Beta", None)),
        ];
        let map = GraphIdentityMap::build("plan", &tasks, &HashMap::new());
        let ids: Vec<&str> = map.iter().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"A"));
        assert!(ids.contains(&"B"));
    }
}
