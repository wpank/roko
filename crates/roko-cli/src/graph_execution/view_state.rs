//! Graph-specific view state for TUI and HTTP dashboard projections (#248/#266).
//!
//! [`GraphViewState`] is the shared projection struct consumed by the TUI
//! graph tab and HTTP graph status routes. It is updated by the
//! [`GraphRuntimeEventAdapter`] during execution and read by presentation
//! layers.
//!
//! Both #248 (this adapter) and #266 (TUI rendering) reference this single
//! canonical definition. Any field changes must be coordinated across both.
//!
//! [`GraphRuntimeEventAdapter`]: super::runtime_event_adapter::GraphRuntimeEventAdapter

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Node status (presentation-layer)
// ---------------------------------------------------------------------------

/// Presentation-layer node status (distinct from roko_graph::engine::NodeStatus
/// to avoid coupling the TUI to engine internals).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeStatus {
    /// Not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed during execution.
    Failed,
    /// Skipped (dependency failure or condition unmet).
    Skipped,
}

impl std::fmt::Display for GraphNodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

// ---------------------------------------------------------------------------
// Node row
// ---------------------------------------------------------------------------

/// A single row in the graph node table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeRow {
    /// Node identifier within the graph.
    pub node_id: String,
    /// Cell type backing the node (e.g. "task-executor", "gate").
    pub cell_type: String,
    /// Current execution status.
    pub status: GraphNodeStatus,
    /// Zero-based wave index from topological sorting.
    pub wave_index: u32,
    /// Accumulated cost in micro-USD.
    pub cost_micro_usd: u64,
    /// Elapsed wall-clock time in milliseconds (set on completion).
    pub duration_ms: Option<u64>,
    /// Error message (set on failure).
    pub error: Option<String>,
    /// Node IDs this node depends on.
    pub dependencies: Vec<String>,
}

// ---------------------------------------------------------------------------
// Hot graph status
// ---------------------------------------------------------------------------

/// Live status of the graph engine Hot tick loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotGraphStatus {
    /// Whether the Hot loop is currently active.
    pub active: bool,
    /// Current tick number.
    pub current_tick: u64,
    /// Loop level name (e.g. "hot", "warm").
    pub loop_level: String,
    /// Tick interval in milliseconds.
    pub interval_ms: u64,
}

// ---------------------------------------------------------------------------
// GraphViewState
// ---------------------------------------------------------------------------

/// Materialized view of graph execution state for TUI and HTTP consumers.
///
/// Updated incrementally by the event adapter during execution.
/// Read by TUI render and HTTP status routes. Thread-safe via internal
/// `RwLock` (writer: event adapter; readers: TUI/HTTP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphViewState {
    /// Per-node status rows, ordered by wave then definition order.
    pub nodes: Vec<GraphNodeRow>,
    /// Currently selected node for detail view (TUI cursor).
    pub selected_node_id: Option<String>,
    /// Live Hot loop status, if active.
    pub hot_status: Option<HotGraphStatus>,
    /// Count of graph nodes not in the identity map.
    pub unknown_cell_count: usize,
    /// Stable fingerprint of the graph structure (for cache invalidation).
    pub graph_fingerprint: String,
    /// Run identifier, if a graph run is active.
    pub run_id: Option<String>,
}

impl Default for GraphViewState {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            selected_node_id: None,
            hot_status: None,
            unknown_cell_count: 0,
            graph_fingerprint: String::new(),
            run_id: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared handle
// ---------------------------------------------------------------------------

/// Thread-safe shared handle to a [`GraphViewState`].
///
/// The event adapter holds the write side; TUI and HTTP hold read-only
/// clones of the `Arc`.
#[derive(Debug, Clone)]
pub struct SharedGraphViewState {
    inner: Arc<RwLock<GraphViewState>>,
}

impl SharedGraphViewState {
    /// Create a new shared handle with default state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(GraphViewState::default())),
        }
    }

    /// Create from an existing state (e.g. loaded from disk).
    pub fn from_state(state: GraphViewState) -> Self {
        Self {
            inner: Arc::new(RwLock::new(state)),
        }
    }

    /// Read the current state.
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, GraphViewState> {
        self.inner.read()
    }

    /// Borrow the state mutably for updates.
    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, GraphViewState> {
        self.inner.write()
    }

    /// Clone the current state as an owned snapshot.
    #[must_use]
    pub fn snapshot(&self) -> GraphViewState {
        self.inner.read().clone()
    }

    /// Update a single node's status.
    pub fn update_node_status(&self, node_id: &str, status: GraphNodeStatus) {
        let mut state = self.inner.write();
        if let Some(node) = state.nodes.iter_mut().find(|n| n.node_id == node_id) {
            node.status = status;
        }
    }

    /// Update a node's cost.
    pub fn add_node_cost(&self, node_id: &str, micro_usd: u64) {
        let mut state = self.inner.write();
        if let Some(node) = state.nodes.iter_mut().find(|n| n.node_id == node_id) {
            node.cost_micro_usd += micro_usd;
        }
    }

    /// Set a node's completion duration.
    pub fn set_node_duration(&self, node_id: &str, duration_ms: u64) {
        let mut state = self.inner.write();
        if let Some(node) = state.nodes.iter_mut().find(|n| n.node_id == node_id) {
            node.duration_ms = Some(duration_ms);
        }
    }

    /// Set a node's error message.
    pub fn set_node_error(&self, node_id: &str, error: String) {
        let mut state = self.inner.write();
        if let Some(node) = state.nodes.iter_mut().find(|n| n.node_id == node_id) {
            node.error = Some(error);
        }
    }

    /// Initialize node rows from the identity map.
    pub fn initialize_from_identity_map(
        &self,
        identity_map: &super::identity_map::GraphIdentityMap,
        run_id: &str,
        graph_fingerprint: &str,
    ) {
        let mut state = self.inner.write();
        state.run_id = Some(run_id.to_string());
        state.graph_fingerprint = graph_fingerprint.to_string();
        state.nodes.clear();

        // Collect and sort by wave_index then task_id for stable ordering.
        let mut entries: Vec<_> = identity_map
            .iter()
            .map(|(node_id, identity)| GraphNodeRow {
                node_id: node_id.to_string(),
                cell_type: "task-executor".to_string(),
                status: GraphNodeStatus::Pending,
                wave_index: identity.wave_index,
                cost_micro_usd: 0,
                duration_ms: None,
                error: None,
                dependencies: Vec::new(),
            })
            .collect();
        entries.sort_by(|a, b| {
            a.wave_index
                .cmp(&b.wave_index)
                .then(a.node_id.cmp(&b.node_id))
        });
        state.nodes = entries;
    }

    /// Update the Hot loop status.
    pub fn set_hot_status(&self, status: Option<HotGraphStatus>) {
        let mut state = self.inner.write();
        state.hot_status = status;
    }

    /// Build a status summary from the current state.
    #[must_use]
    pub fn status_summary(&self) -> GraphStatusSummary {
        let state = self.inner.read();
        let mut pending = 0usize;
        let mut running = 0usize;
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut skipped = 0usize;
        let mut total_cost_micro_usd = 0u64;

        for node in &state.nodes {
            match node.status {
                GraphNodeStatus::Pending => pending += 1,
                GraphNodeStatus::Running => running += 1,
                GraphNodeStatus::Completed => completed += 1,
                GraphNodeStatus::Failed => failed += 1,
                GraphNodeStatus::Skipped => skipped += 1,
            }
            total_cost_micro_usd += node.cost_micro_usd;
        }

        GraphStatusSummary {
            total: state.nodes.len(),
            pending,
            running,
            completed,
            failed,
            skipped,
            total_cost_micro_usd,
        }
    }
}

impl Default for SharedGraphViewState {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary counts from a [`GraphViewState`].
///
/// Built by [`SharedGraphViewState::status_summary`] for dashboards and
/// progress reporting.
#[derive(Debug, Clone)]
pub struct GraphStatusSummary {
    /// Total number of graph nodes.
    pub total: usize,
    /// Nodes not yet started.
    pub pending: usize,
    /// Nodes currently executing.
    pub running: usize,
    /// Nodes that completed successfully.
    pub completed: usize,
    /// Nodes that failed during execution.
    pub failed: usize,
    /// Nodes skipped due to dependency failure or condition.
    pub skipped: usize,
    /// Accumulated cost across all nodes, in micro-USD.
    pub total_cost_micro_usd: u64,
}

// ---------------------------------------------------------------------------
// Projector: GraphExecutionEvent → GraphViewState updates
// ---------------------------------------------------------------------------

/// Updates [`SharedGraphViewState`] from [`GraphExecutionEvent`] values.
///
/// This is the write-side companion to [`GraphRuntimeEventAdapter`]: the
/// adapter converts graph events to runtime envelopes for dashboard/SSE/JSONL,
/// while this projector updates the graph-specific view state for the TUI
/// graph tab and HTTP graph routes.
#[derive(Debug, Clone)]
pub struct GraphViewStateProjector {
    state: SharedGraphViewState,
}

impl GraphViewStateProjector {
    /// Create a new projector writing to the given shared state.
    pub fn new(state: SharedGraphViewState) -> Self {
        Self { state }
    }

    /// Process a graph execution event and update the view state.
    pub fn update(&self, event: &roko_graph::events::GraphExecutionEvent) {
        use roko_graph::events::GraphExecutionEvent;

        match event {
            GraphExecutionEvent::GraphStarted { common } => {
                let mut s = self.state.write();
                s.run_id = Some(common.run_id.clone());
            }
            GraphExecutionEvent::GraphCompleted { .. }
            | GraphExecutionEvent::GraphFailed { .. }
            | GraphExecutionEvent::GraphCancelled { .. } => {
                self.state.set_hot_status(None);
            }

            GraphExecutionEvent::NodeStarted { node, .. } => {
                self.state
                    .update_node_status(&node.node_id, GraphNodeStatus::Running);
            }
            GraphExecutionEvent::NodeCompleted {
                node, elapsed_ms, ..
            } => {
                self.state
                    .update_node_status(&node.node_id, GraphNodeStatus::Completed);
                self.state.set_node_duration(&node.node_id, *elapsed_ms);
            }
            GraphExecutionEvent::NodeFailed {
                node,
                elapsed_ms,
                error,
                ..
            } => {
                self.state
                    .update_node_status(&node.node_id, GraphNodeStatus::Failed);
                self.state.set_node_duration(&node.node_id, *elapsed_ms);
                self.state.set_node_error(&node.node_id, error.clone());
            }
            GraphExecutionEvent::NodeSkipped { node, .. } => {
                self.state
                    .update_node_status(&node.node_id, GraphNodeStatus::Skipped);
            }
            GraphExecutionEvent::NodeRetrying { node, .. } => {
                // Back to running status on retry.
                self.state
                    .update_node_status(&node.node_id, GraphNodeStatus::Running);
            }

            GraphExecutionEvent::UsageRecorded {
                node,
                actual_micro_usd,
                ..
            } => {
                self.state.add_node_cost(&node.node_id, *actual_micro_usd);
            }

            // Wave, agent, tool, gate, cell progress, budget, delivery,
            // feedback, replay, gap events do not update the node table
            // directly (they are handled by the runtime event projector
            // for dashboard updates).
            _ => {}
        }
    }

    /// Access the underlying shared state.
    pub fn state(&self) -> &SharedGraphViewState {
        &self.state
    }
}

// ---------------------------------------------------------------------------
// Build node dependencies from the identity map
// ---------------------------------------------------------------------------

/// Populate node dependency lists from `PlanTaskInfo.depends_on`.
pub fn populate_dependencies(
    state: &SharedGraphViewState,
    tasks: &[(String, roko_graph::convert::PlanTaskInfo)],
) {
    let dep_map: HashMap<String, Vec<String>> = tasks
        .iter()
        .map(|(id, info)| (id.clone(), info.depends_on.clone()))
        .collect();

    let mut s = state.write();
    for node in &mut s.nodes {
        if let Some(deps) = dep_map.get(&node.node_id) {
            node.dependencies = deps.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use roko_graph::convert::PlanTaskInfo;
    use roko_graph::events::*;
    use roko_graph::types::ExecutionClass;

    use super::*;
    use crate::graph_execution::identity_map::GraphIdentityMap;

    fn make_task_info(title: &str) -> PlanTaskInfo {
        PlanTaskInfo {
            title: title.to_string(),
            description: None,
            role: None,
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

    fn setup() -> (
        SharedGraphViewState,
        GraphViewStateProjector,
        GraphIdentityMap,
    ) {
        let tasks = vec![
            ("T01".to_string(), make_task_info("Compile")),
            ("T02".to_string(), make_task_info("Test")),
            ("T03".to_string(), make_task_info("Lint")),
        ];
        let waves: HashMap<String, u32> = [
            ("T01".to_string(), 0),
            ("T02".to_string(), 1),
            ("T03".to_string(), 1),
        ]
        .into_iter()
        .collect();
        let identity_map = GraphIdentityMap::build("test-plan", &tasks, &waves);

        let state = SharedGraphViewState::new();
        state.initialize_from_identity_map(&identity_map, "run-1", "fp-abc");

        let projector = GraphViewStateProjector::new(state.clone());
        (state, projector, identity_map)
    }

    fn make_common(seq: u64) -> CommonFields {
        CommonFields {
            schema_version: GRAPH_EVENT_SCHEMA_VERSION,
            run_id: "run-1".to_string(),
            graph_id: "test-plan".to_string(),
            seq,
        }
    }

    fn make_node(node_id: &str) -> NodeFields {
        NodeFields {
            node_id: node_id.to_string(),
            cell_type: "task-executor".to_string(),
            execution_class: ExecutionClass::Activity,
            attempt: 0,
        }
    }

    #[test]
    fn initial_state_has_pending_nodes() {
        let (state, _, _) = setup();
        let snap = state.snapshot();
        assert_eq!(snap.nodes.len(), 3);
        assert!(
            snap.nodes
                .iter()
                .all(|n| n.status == GraphNodeStatus::Pending)
        );
        assert_eq!(snap.run_id.as_deref(), Some("run-1"));
        assert_eq!(snap.graph_fingerprint, "fp-abc");
    }

    #[test]
    fn nodes_sorted_by_wave_then_id() {
        let (state, _, _) = setup();
        let snap = state.snapshot();
        assert_eq!(snap.nodes[0].node_id, "T01");
        assert_eq!(snap.nodes[0].wave_index, 0);
        // T02 and T03 are both wave 1, sorted by node_id.
        assert_eq!(snap.nodes[1].node_id, "T02");
        assert_eq!(snap.nodes[2].node_id, "T03");
    }

    #[test]
    fn node_started_updates_status() {
        let (state, projector, _) = setup();
        projector.update(&GraphExecutionEvent::NodeStarted {
            common: make_common(1),
            node: make_node("T01"),
        });
        let snap = state.snapshot();
        let t01 = snap.nodes.iter().find(|n| n.node_id == "T01").unwrap();
        assert_eq!(t01.status, GraphNodeStatus::Running);
    }

    #[test]
    fn node_completed_sets_duration() {
        let (state, projector, _) = setup();
        projector.update(&GraphExecutionEvent::NodeCompleted {
            common: make_common(2),
            node: make_node("T01"),
            elapsed_ms: 1500,
            outcome: None,
        });
        let snap = state.snapshot();
        let t01 = snap.nodes.iter().find(|n| n.node_id == "T01").unwrap();
        assert_eq!(t01.status, GraphNodeStatus::Completed);
        assert_eq!(t01.duration_ms, Some(1500));
    }

    #[test]
    fn node_failed_sets_error() {
        let (state, projector, _) = setup();
        projector.update(&GraphExecutionEvent::NodeFailed {
            common: make_common(3),
            node: make_node("T02"),
            elapsed_ms: 500,
            error: "tests failed".to_string(),
        });
        let snap = state.snapshot();
        let t02 = snap.nodes.iter().find(|n| n.node_id == "T02").unwrap();
        assert_eq!(t02.status, GraphNodeStatus::Failed);
        assert_eq!(t02.error.as_deref(), Some("tests failed"));
    }

    #[test]
    fn usage_accumulates_cost() {
        let (state, projector, _) = setup();
        projector.update(&GraphExecutionEvent::UsageRecorded {
            common: make_common(4),
            node: make_node("T01"),
            dispatch: DispatchFields {
                attempt_id: "a1".to_string(),
                agent_id: None,
            },
            input_tokens: 100,
            output_tokens: 50,
            actual_micro_usd: 50_000,
        });
        projector.update(&GraphExecutionEvent::UsageRecorded {
            common: make_common(5),
            node: make_node("T01"),
            dispatch: DispatchFields {
                attempt_id: "a2".to_string(),
                agent_id: None,
            },
            input_tokens: 200,
            output_tokens: 100,
            actual_micro_usd: 100_000,
        });
        let snap = state.snapshot();
        let t01 = snap.nodes.iter().find(|n| n.node_id == "T01").unwrap();
        assert_eq!(t01.cost_micro_usd, 150_000);
    }

    #[test]
    fn status_summary_counts() {
        let (state, projector, _) = setup();
        projector.update(&GraphExecutionEvent::NodeCompleted {
            common: make_common(1),
            node: make_node("T01"),
            elapsed_ms: 100,
            outcome: None,
        });
        projector.update(&GraphExecutionEvent::NodeFailed {
            common: make_common(2),
            node: make_node("T02"),
            elapsed_ms: 200,
            error: "oops".to_string(),
        });
        let summary = state.status_summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.pending, 1); // T03 is still pending
    }

    #[test]
    fn hot_status_can_be_set_and_cleared() {
        let (state, projector, _) = setup();
        state.set_hot_status(Some(HotGraphStatus {
            active: true,
            current_tick: 42,
            loop_level: "hot".to_string(),
            interval_ms: 100,
        }));
        assert!(state.snapshot().hot_status.is_some());

        projector.update(&GraphExecutionEvent::GraphCompleted {
            common: make_common(10),
            stats: TerminalStats {
                elapsed_ms: 5000,
                completed_nodes: 3,
                total_nodes: 3,
            },
        });
        assert!(state.snapshot().hot_status.is_none());
    }

    #[test]
    fn populate_dependencies_fills_deps() {
        let info1 = make_task_info("Compile");
        let mut info2 = make_task_info("Test");
        info2.depends_on = vec!["T01".to_string()];

        let tasks = vec![("T01".to_string(), info1), ("T02".to_string(), info2)];
        let waves: HashMap<String, u32> = [("T01".to_string(), 0), ("T02".to_string(), 1)]
            .into_iter()
            .collect();
        let identity_map = GraphIdentityMap::build("plan", &tasks, &waves);

        let state = SharedGraphViewState::new();
        state.initialize_from_identity_map(&identity_map, "run", "fp");

        populate_dependencies(&state, &tasks);

        let snap = state.snapshot();
        let t01 = snap.nodes.iter().find(|n| n.node_id == "T01").unwrap();
        assert!(t01.dependencies.is_empty());
        let t02 = snap.nodes.iter().find(|n| n.node_id == "T02").unwrap();
        assert_eq!(t02.dependencies, vec!["T01"]);
    }
}
