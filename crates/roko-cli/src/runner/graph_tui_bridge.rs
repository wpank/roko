//! Bridge between the graph engine execution path and the TUI dashboard.
//!
//! The graph engine emits `ObservableEvent` telemetry into the Lens runtime
//! via `TelemetryEventSink`, but does not produce the `DashboardEvent`
//! variants that the TUI consumes. This module maps graph execution
//! lifecycle transitions to `DashboardEvent` publications so the TUI can
//! observe Graph engine plan runs the same way regardless of flags.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use roko_core::dashboard_snapshot::{
    TASK_OUTCOME_ACCEPTED_WITH_FAILURES, TASK_OUTCOME_ALREADY_SATISFIED, TASK_OUTCOME_PASSED,
    TASK_OUTCOME_UNVERIFIED,
};
use roko_core::{LensScope, ObservableEvent, Signal, TelemetryEventSink};
use roko_graph::cells::task_executor::TaskGateVerdict;
use roko_graph::engine::{GraphOutput, NodeStatus};

use crate::state_hub::StateHubSender;

use super::tui_bridge::TuiBridge;

/// Per-plan ETA tracking state shared across polling cycles.
#[derive(Debug)]
struct EtaTracker {
    /// When the plan started executing (used to compute elapsed time).
    started_at: Instant,
    /// Total number of tasks in the plan.
    total: usize,
    /// Number of tasks completed so far (success + failure + skip).
    done: usize,
}

impl EtaTracker {
    /// Proportional remaining minutes (at least 1), once at least one task
    /// is done and at least one remains.
    fn eta_minutes(&self) -> Option<u32> {
        let remaining = self.total.saturating_sub(self.done);
        if self.done == 0 || remaining == 0 {
            return None;
        }
        let elapsed_secs = self.started_at.elapsed().as_secs_f64();
        let per_task_secs = elapsed_secs / self.done as f64;
        let eta_minutes = (per_task_secs * remaining as f64 / 60.0).ceil() as u32;
        // Clamp to 1 minute minimum so the display is always useful.
        Some(eta_minutes.max(1))
    }
}

/// The run's remaining time: the longest estimate among running plans.
fn largest_eta_minutes(trackers: &HashMap<String, EtaTracker>) -> Option<u32> {
    trackers.values().filter_map(EtaTracker::eta_minutes).max()
}

/// Bridges passive observable telemetry into the runner's shared StateHub.
///
/// Moved from `runner/event_loop.rs` during Runner-v2 deletion.
pub struct StateHubTelemetrySink(StateHubSender);

impl StateHubTelemetrySink {
    /// Create a telemetry sink backed by a StateHub sender.
    pub fn new(sender: StateHubSender) -> Self {
        Self(sender)
    }
}

#[async_trait::async_trait]
impl TelemetryEventSink for StateHubTelemetrySink {
    async fn emit(
        &self,
        event: &ObservableEvent,
        ancestry: &[LensScope],
    ) -> roko_core::Result<Vec<Signal>> {
        let errors = self.0.emit_observable(event, ancestry);
        if errors.is_empty() {
            Ok(Vec::new())
        } else {
            Err(roko_core::RokoError::invalid(errors.join("; ")))
        }
    }
}

/// Adapter that maps graph engine lifecycle transitions to `DashboardEvent`
/// publications through the existing [`TuiBridge`].
///
/// Callers construct an instance before starting a graph plan and call its
/// methods at well-defined lifecycle points. ETA tracking is kept per plan
/// behind an internal mutex, so the adapter stays `Sync` and several plans
/// can run at once.
pub struct GraphTuiBridge {
    tui: TuiBridge,
    /// ETA tracking per running plan.
    eta: Arc<Mutex<HashMap<String, EtaTracker>>>,
}

impl GraphTuiBridge {
    /// Create a new bridge wrapping the shared `TuiBridge`.
    pub fn new(tui: TuiBridge) -> Self {
        Self {
            tui,
            eta: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // ── Plan-level events ────────────────────────────────────────────

    /// Emit `PlanStarted` before a graph plan begins execution.
    ///
    /// Also initialises the ETA tracker with the plan's task count and start
    /// time so subsequent `node_completed` calls can compute a proportional
    /// remaining-time estimate.
    pub fn plan_started(&self, plan_id: &str, task_count: usize) {
        self.tui.plan_started(plan_id, task_count);
        if let Ok(mut guard) = self.eta.lock() {
            guard.insert(
                plan_id.to_string(),
                EtaTracker {
                    started_at: Instant::now(),
                    total: task_count,
                    done: 0,
                },
            );
        }
    }

    /// Emit `PlanCompleted` after a graph plan finishes.
    pub fn plan_completed(&self, plan_id: &str, success: bool) {
        self.tui.plan_completed(plan_id, success);
        // Drop the plan's tracker so stale estimates don't linger, and show
        // what the plans still running need (none once the last finishes).
        let remaining = self.eta.lock().ok().and_then(|mut guard| {
            guard.remove(plan_id);
            largest_eta_minutes(&guard)
        });
        self.tui.critical_path_eta(plan_id, remaining);
    }

    // ── Node-level events (pre-execution) ────────────────────────────

    /// Emit `TaskStarted` when a graph node begins executing.
    pub fn node_started(&self, plan_id: &str, node_id: &str, title: &str) {
        self.tui
            .task_started(plan_id, node_id, title, "graph-executing");
    }

    // ── Node-level events (post-execution) ───────────────────────────

    /// Emit `TaskCompleted` after a graph node finishes (success or failure).
    /// Without a gate verdict, a completed node is reported as unverified.
    ///
    /// Also updates the ETA estimate and publishes a `CriticalPathEtaUpdated`
    /// event so the TUI progress card shows a live remaining-time estimate.
    pub fn node_completed(&self, plan_id: &str, node_id: &str, status: NodeStatus) {
        self.node_completed_with_verdict(plan_id, node_id, status, None);
    }

    /// Emit `TaskCompleted` with an outcome that honours the node's gate
    /// verdict: only a `passed` verdict is reported as a pass.
    pub fn node_completed_with_verdict(
        &self,
        plan_id: &str,
        node_id: &str,
        status: NodeStatus,
        verdict: Option<TaskGateVerdict>,
    ) {
        self.tui
            .task_completed(plan_id, node_id, node_outcome(status, verdict));

        // Update ETA tracking.  Only publish when there is a non-trivial
        // estimate: at least one task done and at least one remaining.  The
        // TUI progress card's proportional fallback handles the zero-done
        // case, so we skip publishing a None here to keep event volume low.
        // With several plans running, the run's estimate is the longest one.
        if let Ok(mut guard) = self.eta.lock() {
            let Some(tracker) = guard.get_mut(plan_id) else {
                return;
            };
            tracker.done = tracker.done.saturating_add(1);
            if tracker.eta_minutes().is_some() {
                self.tui
                    .critical_path_eta(plan_id, largest_eta_minutes(&guard));
            }
        }
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
            self.node_completed_with_verdict(
                plan_id,
                &result.node_id,
                result.status,
                output.gate_verdicts.get(&result.node_id).copied(),
            );
        }
    }

    // ── Status snapshot polling ──────────────────────────────────────

    /// Poll a live `FlowHandle` and emit events for any nodes whose status
    /// has changed since the last poll.
    ///
    /// Each completed node is reported with its gate verdict, looked up in
    /// the map `gate_verdicts` returns (bug-7e1b6b). It is called once, and
    /// only when some node finished since the last poll.
    ///
    /// Returns the new status map for the next polling cycle.
    ///
    /// Completions are always published before starts so that, when a
    /// predecessor completes and a successor starts in the same 100 ms tick,
    /// `task_completed` always precedes `task_started` in the event stream.
    /// This preserves the causal order visible to the portal's run view.
    pub fn poll_status_changes(
        &self,
        plan_id: &str,
        previous: &HashMap<String, NodeStatus>,
        current: &HashMap<String, NodeStatus>,
        node_titles: &HashMap<String, String>,
        gate_verdicts: impl FnOnce() -> BTreeMap<String, TaskGateVerdict>,
    ) -> Vec<(String, NodeStatus)> {
        // Collect transitions into two buckets so we can emit in the right
        // order regardless of HashMap iteration order.
        let mut completions: Vec<(&String, NodeStatus)> = Vec::new();
        let mut starts: Vec<(&String, NodeStatus)> = Vec::new();

        for (node_id, &new_status) in current {
            let old_status = previous.get(node_id).copied();
            if old_status.map_or(false, |old| old == new_status) {
                continue;
            }
            match new_status {
                NodeStatus::Running => starts.push((node_id, new_status)),
                NodeStatus::Complete
                | NodeStatus::Failed
                | NodeStatus::Skipped
                | NodeStatus::ConditionSkipped => completions.push((node_id, new_status)),
                NodeStatus::Pending => {}
            }
        }

        let verdicts = if completions.is_empty() {
            BTreeMap::new()
        } else {
            gate_verdicts()
        };
        let mut changes = Vec::new();
        // Emit completions first: in a serial DAG a node completing in this
        // tick is always the cause of any successor starting in the same tick.
        for (node_id, new_status) in completions {
            let verdict = verdicts.get(node_id).copied();
            self.node_completed_with_verdict(plan_id, node_id, new_status, verdict);
            changes.push((node_id.clone(), new_status));
        }
        for (node_id, new_status) in starts {
            let title = node_titles
                .get(node_id)
                .map(String::as_str)
                .unwrap_or(node_id);
            self.node_started(plan_id, node_id, title);
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

/// Dashboard outcome for a finished node.
///
/// Only a completed node whose gate verdict is `passed` is reported as
/// passed. One whose work was already there, so its verify steps passed on a
/// tree its attempt left unchanged, is `already_satisfied` (gap-9eb1e1). A
/// forced accept is accepted-with-failures, and a node that completed without
/// a verify step judging it is unverified (bug-7e1b6b).
fn node_outcome(status: NodeStatus, verdict: Option<TaskGateVerdict>) -> &'static str {
    match (status, verdict) {
        (NodeStatus::Complete, Some(TaskGateVerdict::Passed)) => TASK_OUTCOME_PASSED,
        (NodeStatus::Complete, Some(TaskGateVerdict::AlreadySatisfied)) => {
            TASK_OUTCOME_ALREADY_SATISFIED
        }
        (NodeStatus::Complete, Some(TaskGateVerdict::ForcedAccept)) => {
            TASK_OUTCOME_ACCEPTED_WITH_FAILURES
        }
        (NodeStatus::Complete, Some(TaskGateVerdict::Unverified) | None) => TASK_OUTCOME_UNVERIFIED,
        (NodeStatus::Failed, _) => "failed",
        (NodeStatus::Skipped, _) => "skipped",
        (NodeStatus::ConditionSkipped, _) => "condition-skipped",
        (NodeStatus::Pending | NodeStatus::Running, _) => "unknown",
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

    use roko_graph::engine::{NodeResult, NodeTiming};

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
            blocked_by: None,
            timing: NodeTiming::default(),
        }
    }

    #[test]
    fn eta_follows_the_plans_still_running() {
        let (hub, bridge) = make_bridge();
        let mut sub = hub.subscribe_events_from(0);
        let etas = |sub: &mut crate::state_hub::StateHubSubscription| {
            let mut etas = Vec::new();
            while let Ok(envelope) = sub.live.try_recv() {
                if let roko_core::DashboardEvent::CriticalPathEtaUpdated {
                    plan_id,
                    eta_minutes,
                } = envelope.payload
                {
                    etas.push((plan_id, eta_minutes));
                }
            }
            etas
        };

        bridge.plan_started("plan-a", 2);
        bridge.plan_started("plan-b", 3);
        bridge.node_completed("plan-b", "T1", NodeStatus::Complete);
        assert_eq!(etas(&mut sub), [("plan-b".to_string(), Some(1))]);

        // plan-b still has work left, so finishing plan-a keeps an estimate.
        bridge.plan_completed("plan-a", true);
        assert_eq!(etas(&mut sub), [("plan-a".to_string(), Some(1))]);

        bridge.plan_completed("plan-b", true);
        assert_eq!(etas(&mut sub), [("plan-b".to_string(), None)]);
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
            gate_verdicts: Default::default(),
        };

        emit_plan_lifecycle(&bridge, "test-plan", 3, &output, true);

        // Collect all events.
        let mut events = Vec::new();
        while let Ok(envelope) = sub.live.try_recv() {
            events.push(envelope.payload);
        }

        // PlanStarted + 3×TaskCompleted + 2×CriticalPathEtaUpdated (T01+T02 have
        // remaining tasks; T03 is the last so no ETA) + PlanCompleted +
        // CriticalPathEtaUpdated(None) from plan_completed = 8.
        assert_eq!(events.len(), 8, "expected 8 events, got {}", events.len());
    }

    #[test]
    fn forced_accept_verdict_is_not_reported_as_passed() {
        let (hub, bridge) = make_bridge();
        let mut sub = hub.subscribe_events_from(0);
        let output = GraphOutput {
            graph_name: "test-plan".to_string(),
            success: true,
            node_results: vec![
                make_node_result("T01", NodeStatus::Complete, 100),
                make_node_result("T02", NodeStatus::Complete, 100),
            ],
            total_duration: Duration::from_millis(200),
            gate_verdicts: [
                ("T01".to_string(), TaskGateVerdict::Passed),
                ("T02".to_string(), TaskGateVerdict::ForcedAccept),
            ]
            .into_iter()
            .collect(),
        };

        bridge.emit_graph_output("test-plan", &output);

        let mut outcomes = HashMap::new();
        while let Ok(envelope) = sub.live.try_recv() {
            if let roko_core::DashboardEvent::TaskCompleted {
                task_id, outcome, ..
            } = envelope.payload
            {
                outcomes.insert(task_id, outcome);
            }
        }
        assert_eq!(outcomes.get("T01").map(String::as_str), Some("passed"));
        assert_eq!(
            outcomes.get("T02").map(String::as_str),
            Some(TASK_OUTCOME_ACCEPTED_WITH_FAILURES)
        );
        // bug-7e1b6b: a task no verify step judged is unverified, not passed.
        for verdict in [Some(TaskGateVerdict::Unverified), None] {
            assert_eq!(
                node_outcome(NodeStatus::Complete, verdict),
                TASK_OUTCOME_UNVERIFIED
            );
        }
    }

    /// gap-9eb1e1: a task whose work was already there is its own outcome.
    #[test]
    fn already_satisfied_verdict_is_not_reported_as_passed() {
        assert_eq!(
            node_outcome(
                NodeStatus::Complete,
                Some(TaskGateVerdict::AlreadySatisfied),
            ),
            "already_satisfied"
        );
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
            gate_verdicts: Default::default(),
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

        let changes =
            bridge.poll_status_changes("plan-1", &previous, &current, &titles, BTreeMap::new);
        assert_eq!(changes.len(), 2);

        let mut events = Vec::new();
        while let Ok(envelope) = sub.live.try_recv() {
            events.push(envelope.payload);
        }
        // T01: Pending→Running = TaskStarted, T02: Running→Complete = TaskCompleted.
        assert_eq!(events.len(), 2);
    }

    /// bug-7e1b6b: each task that finishes is reported with the outcome its
    /// gate verdict earns, so only a verified pass counts as passed.
    #[test]
    fn poll_reports_each_completion_with_its_gate_verdict() {
        let (hub, bridge) = make_bridge();
        let mut sub = hub.subscribe_events_from(0);
        let running: HashMap<String, NodeStatus> = ["T1", "T2", "T3", "T4"]
            .into_iter()
            .map(|id| (id.to_string(), NodeStatus::Running))
            .collect();
        let finished: HashMap<String, NodeStatus> = [
            ("T1", NodeStatus::Complete),
            ("T2", NodeStatus::Complete),
            ("T3", NodeStatus::Failed),
            ("T4", NodeStatus::Skipped),
        ]
        .into_iter()
        .map(|(id, status)| (id.to_string(), status))
        .collect();
        let verdicts = || BTreeMap::from([("T1".to_string(), TaskGateVerdict::Passed)]);

        // Nothing finished yet: the verdicts are not read.
        bridge.poll_status_changes("plan-1", &running, &running, &HashMap::new(), || {
            panic!("no node finished")
        });
        bridge.poll_status_changes("plan-1", &running, &finished, &HashMap::new(), verdicts);

        let mut outcomes = BTreeMap::new();
        while let Ok(envelope) = sub.live.try_recv() {
            if let roko_core::DashboardEvent::TaskCompleted {
                task_id, outcome, ..
            } = envelope.payload
            {
                outcomes.insert(task_id, outcome);
            }
        }
        let outcomes: Vec<_> = outcomes
            .iter()
            .map(|(id, o)| (id.as_str(), o.as_str()))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("T1", "passed"),
                ("T2", TASK_OUTCOME_UNVERIFIED),
                ("T3", "failed"),
                ("T4", "skipped"),
            ]
        );
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
