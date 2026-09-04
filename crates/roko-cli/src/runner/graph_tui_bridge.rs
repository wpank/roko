//! Bridge between the graph engine execution path and the TUI dashboard.
//!
//! The graph engine emits `ObservableEvent` telemetry into the Lens runtime
//! via `TelemetryEventSink`, but does not produce the `DashboardEvent`
//! variants that the TUI consumes. This module maps graph execution
//! lifecycle transitions to `DashboardEvent` publications so the TUI can
//! observe `--engine graph` runs identically to runner-v2 plan runs.

use std::collections::HashMap;

use roko_graph::engine::{GraphOutput, NodeStatus};

use super::tui_bridge::TuiBridge;

/// Adapter that maps graph engine lifecycle transitions to `DashboardEvent`
/// publications through the existing [`TuiBridge`].
///
/// Callers construct an instance before starting a graph plan and call its
/// methods at well-defined lifecycle points. The adapter keeps no mutable
/// state beyond what `TuiBridge` maintains internally.
pub struct GraphTuiBridge {
    tui: TuiBridge,
}

impl GraphTuiBridge {
    /// Create a new bridge wrapping the shared `TuiBridge`.
    pub fn new(tui: TuiBridge) -> Self {
        Self { tui }
    }

    // ── Plan-level events ────────────────────────────────────────────

    /// Emit `PlanStarted` before a graph plan begins execution.
    pub fn plan_started(&self, plan_id: &str, task_count: usize) {
        self.tui.plan_started(plan_id, task_count);
    }

    /// Emit `PlanCompleted` after a graph plan finishes.
    pub fn plan_completed(&self, plan_id: &str, success: bool) {
        self.tui.plan_completed(plan_id, success);
    }

    // ── Node-level events (pre-execution) ────────────────────────────

    /// Emit `TaskStarted` when a graph node begins executing.
    pub fn node_started(&self, plan_id: &str, node_id: &str, title: &str) {
        self.tui
            .task_started(plan_id, node_id, title, "graph-executing");
    }

    // ── Node-level events (post-execution) ───────────────────────────

    /// Emit `TaskCompleted` after a graph node finishes (success or failure).
    pub fn node_completed(&self, plan_id: &str, node_id: &str, status: NodeStatus) {
        let outcome = match status {
            NodeStatus::Complete => "passed",
            NodeStatus::Failed => "failed",
            NodeStatus::Skipped => "skipped",
            NodeStatus::ConditionSkipped => "condition-skipped",
            NodeStatus::Pending | NodeStatus::Running => "unknown",
        };
        self.tui.task_completed(plan_id, node_id, outcome);
    }

    // ── Batch post-execution summary ─────────────────────────────────

    /// Emit events from a completed `GraphOutput` for all nodes.
    ///
    /// This is the primary integration point: called once after
    /// `engine.execute()` returns, it retroactively publishes the
    /// per-node events that the TUI needs to render the plan tree and
    /// task progress.
    pub fn emit_graph_output(&self, plan_id: &str, output: &GraphOutput) {
        // Emit per-node results.
        for result in &output.node_results {
            self.node_completed(plan_id, &result.node_id, result.status);
        }
    }

    // ── Status snapshot polling ──────────────────────────────────────

    /// Poll a live `FlowHandle` and emit events for any nodes whose status
    /// has changed since the last poll.
    ///
    /// Returns the new status map for the next polling cycle.
    pub fn poll_status_changes(
        &self,
        plan_id: &str,
        previous: &HashMap<String, NodeStatus>,
        current: &HashMap<String, NodeStatus>,
        node_titles: &HashMap<String, String>,
    ) -> Vec<(String, NodeStatus)> {
        let mut changes = Vec::new();
        for (node_id, &new_status) in current {
            let old_status = previous.get(node_id).copied();
            let changed = old_status.map_or(true, |old| old != new_status);
            if !changed {
                continue;
            }
            match new_status {
                NodeStatus::Running => {
                    let title = node_titles
                        .get(node_id)
                        .map(String::as_str)
                        .unwrap_or(node_id);
                    self.node_started(plan_id, node_id, title);
                }
                NodeStatus::Complete
                | NodeStatus::Failed
                | NodeStatus::Skipped
                | NodeStatus::ConditionSkipped => {
                    self.node_completed(plan_id, node_id, new_status);
                }
                NodeStatus::Pending => {}
            }
            changes.push((node_id.clone(), new_status));
        }
        changes
    }

    /// Emit an event log entry for graph engine diagnostics.
    pub fn log_event(&self, event_type: &str, message: &str) {
        self.tui.status(event_type, message);
    }

    /// Emit an error event.
    pub fn error(&self, message: &str) {
        self.tui.error(message);
    }
}

/// Collect node ID to title mappings from plan tasks for status polling.
///
/// Used by the graph execution path to build the title lookup table
/// that `poll_status_changes` needs.
pub fn build_node_title_map(
    tasks: &[(String, roko_graph::convert::PlanTaskInfo)],
) -> HashMap<String, String> {
    tasks
        .iter()
        .map(|(id, info)| (id.clone(), info.title.clone()))
        .collect()
}

/// Emit the full lifecycle events for a synchronous graph plan execution.
///
/// This is a convenience wrapper that emits `PlanStarted`, per-node
/// status from the output, and `PlanCompleted` in one call. Used by
/// `cmd_plan_run_engine` after `engine.execute()` returns.
pub fn emit_plan_lifecycle(
    bridge: &GraphTuiBridge,
    plan_id: &str,
    _task_count: usize,
    output: &GraphOutput,
    execution_succeeded: bool,
) {
    // PlanStarted was already emitted before execute(); emit results.
    bridge.emit_graph_output(plan_id, output);
    bridge.plan_completed(plan_id, execution_succeeded);
}

/// Compute a `NodeStatus`-keyed summary from graph output for efficiency
/// event reporting.
pub fn status_summary(output: &GraphOutput) -> StatusSummary {
    let mut complete = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut total_duration_ms = 0u64;

    for result in &output.node_results {
        match result.status {
            NodeStatus::Complete => complete += 1,
            NodeStatus::Failed => failed += 1,
            NodeStatus::Skipped | NodeStatus::ConditionSkipped => skipped += 1,
            NodeStatus::Pending | NodeStatus::Running => {}
        }
        total_duration_ms += result.duration.as_millis() as u64;
    }

    StatusSummary {
        total: output.node_results.len(),
        complete,
        failed,
        skipped,
        total_duration_ms,
    }
}

/// Aggregate status counts from a graph execution.
#[derive(Debug, Clone, Copy)]
pub struct StatusSummary {
    pub total: usize,
    pub complete: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use roko_graph::engine::NodeResult;

    use super::*;
    use crate::state_hub::StateHub;

    fn make_bridge() -> (StateHub, GraphTuiBridge) {
        let hub = StateHub::default_capacity();
        let tui = TuiBridge::new(hub.sender());
        let bridge = GraphTuiBridge::new(tui);
        (hub, bridge)
    }

    fn make_node_result(node_id: &str, status: NodeStatus, duration_ms: u64) -> NodeResult {
        NodeResult {
            node_id: node_id.to_string(),
            cell_type: "task-executor".to_string(),
            status,
            duration: Duration::from_millis(duration_ms),
            error: None,
            output_count: 1,
            is_stub: false,
        }
    }

    #[test]
    fn plan_lifecycle_emits_all_events() {
        let (hub, bridge) = make_bridge();
        let mut sub = hub.subscribe_events_from(0);

        bridge.plan_started("test-plan", 3);

        let output = GraphOutput {
            graph_name: "test-plan".to_string(),
            success: true,
            node_results: vec![
                make_node_result("T01", NodeStatus::Complete, 100),
                make_node_result("T02", NodeStatus::Complete, 200),
                make_node_result("T03", NodeStatus::Skipped, 0),
            ],
            total_duration: Duration::from_millis(300),
        };

        emit_plan_lifecycle(&bridge, "test-plan", 3, &output, true);

        // Collect all events.
        let mut events = Vec::new();
        while let Ok(envelope) = sub.live.try_recv() {
            events.push(envelope.payload);
        }

        // Should have: PlanStarted + 3 TaskCompleted + PlanCompleted = 5.
        assert_eq!(events.len(), 5, "expected 5 events, got {}", events.len());
    }

    #[test]
    fn status_summary_counts_correctly() {
        let output = GraphOutput {
            graph_name: "test".to_string(),
            success: false,
            node_results: vec![
                make_node_result("T01", NodeStatus::Complete, 100),
                make_node_result("T02", NodeStatus::Failed, 50),
                make_node_result("T03", NodeStatus::Skipped, 0),
                make_node_result("T04", NodeStatus::ConditionSkipped, 0),
                make_node_result("T05", NodeStatus::Complete, 200),
            ],
            total_duration: Duration::from_millis(350),
        };

        let summary = status_summary(&output);
        assert_eq!(summary.total, 5);
        assert_eq!(summary.complete, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 2);
        assert_eq!(summary.total_duration_ms, 350);
    }

    #[test]
    fn poll_detects_status_transitions() {
        let (hub, bridge) = make_bridge();
        let mut sub = hub.subscribe_events_from(0);

        let previous: HashMap<String, NodeStatus> = [
            ("T01".to_string(), NodeStatus::Pending),
            ("T02".to_string(), NodeStatus::Running),
        ]
        .into_iter()
        .collect();

        let current: HashMap<String, NodeStatus> = [
            ("T01".to_string(), NodeStatus::Running),
            ("T02".to_string(), NodeStatus::Complete),
        ]
        .into_iter()
        .collect();

        let titles: HashMap<String, String> = [
            ("T01".to_string(), "First task".to_string()),
            ("T02".to_string(), "Second task".to_string()),
        ]
        .into_iter()
        .collect();

        let changes = bridge.poll_status_changes("plan-1", &previous, &current, &titles);
        assert_eq!(changes.len(), 2);

        let mut events = Vec::new();
        while let Ok(envelope) = sub.live.try_recv() {
            events.push(envelope.payload);
        }
        // T01: Pending→Running = TaskStarted, T02: Running→Complete = TaskCompleted.
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn build_node_title_map_collects_titles() {
        let tasks = vec![(
            "T01".to_string(),
            roko_graph::convert::PlanTaskInfo {
                title: "First".to_string(),
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
            },
        )];

        let map = build_node_title_map(&tasks);
        assert_eq!(map.get("T01").map(String::as_str), Some("First"));
    }
}
