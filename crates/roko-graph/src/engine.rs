//! Graph execution engine: conditional sequential/parallel execution of Cell DAGs.
//!
//! The `GraphEngine` takes a `Graph` and a `CellRegistry`, topologically sorts
//! the nodes, and executes Cells sequentially or with bounded parallelism,
//! starting each Cell as soon as the nodes it depends on have settled.
//! Only active conditional edges contribute upstream outputs to downstream Cells.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use petgraph::visit::EdgeRef as _;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use roko_core::{ContentHash, LensScope, ObservableEvent, TelemetryEventSink};

use crate::cell::{Cell, CellContext};
use crate::cells::task_executor::TaskGateVerdict;
use crate::registry::CellRegistry;
use crate::replay::{ActivityRecorder, ActivityReplayer};
use crate::topo::topological_order;
use crate::types::{
    EdgeCondition, ExecutionClass, Graph, GraphError, GraphNodeIdx, GraphPolicy, Node, NodeId,
};

// ─── MergeEnqueuer trait ────────────────────────────────────────────────────

// MergeRequest and MergeEnqueuer are now defined in delivery.rs. Re-export
// them here for backward compatibility with existing callers.
pub use crate::delivery::{MergeEnqueuer, MergeRequest};

// ─── GraphSnapshot ──────────────────────────────────────────────────────────

// Snapshot types are now defined in snapshot.rs (#251). Re-export them here
// for backward compatibility with existing callers.
pub use crate::snapshot::{
    GRAPH_SNAPSHOT_SCHEMA_VERSION, GraphSnapshot, GraphSnapshotV2, SerializableNodeStatus,
    SerializableSignal,
};

// Re-export reconciliation helper. The snapshot module returns ReconcileAction;
// this wrapper preserves the original NodeStatus return for existing callers.
/// Reconcile an ambiguous `Running` status from a restored snapshot.
///
/// Graph callers that do not have a registered reconciliation owner should
/// call this to convert `Running` to `Pending` before resume. This preserves
/// backward compatibility for callers that do not implement owner-based
/// reconciliation.
pub fn reconcile_running_status(status: SerializableNodeStatus) -> NodeStatus {
    match status {
        SerializableNodeStatus::Running => NodeStatus::Pending,
        other => other.into(),
    }
}

// ─── SerializableNodeStatus <-> NodeStatus conversions ──────────────────────

impl From<NodeStatus> for SerializableNodeStatus {
    fn from(s: NodeStatus) -> Self {
        match s {
            NodeStatus::Pending => Self::Pending,
            NodeStatus::Running => Self::Running,
            NodeStatus::Complete => Self::Complete,
            NodeStatus::Failed => Self::Failed,
            NodeStatus::Skipped => Self::Skipped,
            NodeStatus::ConditionSkipped => Self::ConditionSkipped,
        }
    }
}

impl From<SerializableNodeStatus> for NodeStatus {
    fn from(s: SerializableNodeStatus) -> Self {
        match s {
            SerializableNodeStatus::Pending => Self::Pending,
            // Running is preserved so a registered reconciliation owner can
            // decide how to handle it, rather than blindly resetting to Pending.
            SerializableNodeStatus::Running => Self::Running,
            SerializableNodeStatus::Complete => Self::Complete,
            SerializableNodeStatus::Failed => Self::Failed,
            SerializableNodeStatus::Skipped => Self::Skipped,
            SerializableNodeStatus::ConditionSkipped => Self::ConditionSkipped,
        }
    }
}

// ─── Node types ─────────────────────────────────────────────────────────────

/// Status of a node during graph execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    /// Not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Complete,
    /// Failed during execution.
    Failed,
    /// Skipped because an upstream node failed.
    Skipped,
    /// Skipped because no incoming conditional route selected this node.
    ConditionSkipped,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Complete => write!(f, "complete"),
            Self::Failed => write!(f, "FAILED"),
            Self::Skipped => write!(f, "skipped"),
            Self::ConditionSkipped => write!(f, "not-selected"),
        }
    }
}

/// Decision made for a node after evaluating all of its incoming edges.
enum NodeActivation {
    /// A root node, supplied by the graph ingress boundary.
    Root,
    /// The node is selected and receives outputs only from active edges.
    Ready(Vec<roko_core::Signal>),
    /// No conditional route selected the node. This is a successful no-op.
    ConditionSkipped(String),
    /// A required dependency did not complete successfully.
    UpstreamFailed {
        /// Why the node cannot run.
        reason: String,
        /// The required dependency that did not complete, if any.
        dependency: Option<NodeId>,
    },
}

/// Execution result for a single node.
#[derive(Debug, Clone)]
pub struct NodeResult {
    /// Node identifier.
    pub node_id: NodeId,
    /// Cell type that was executed.
    pub cell_type: String,
    /// Final status after execution.
    pub status: NodeStatus,
    /// Wall-clock duration of execution (zero for skipped nodes).
    pub duration: Duration,
    /// Failure or skip diagnostic, when applicable.
    pub error: Option<String>,
    /// Number of output signals produced.
    pub output_count: usize,
    /// Whether the cell backing this node is a stub/placeholder.
    pub is_stub: bool,
    /// For a node skipped because a node it depends on failed: the failed
    /// node behind it, followed through any skipped dependencies in between.
    pub blocked_by: Option<NodeId>,
    /// When the node became ready and when its cell started.
    pub timing: NodeTiming,
}

/// When a node became ready to run and when it was dispatched, in
/// milliseconds since the Unix epoch.
///
/// A node is ready once every node it depends on has settled, or when the
/// run starts for a node with no dependencies. It is dispatched when it gets
/// a slot and its cell starts, so the difference is the time it waited for a
/// slot. Both are `None` for a node that started no cell: skipped,
/// condition-skipped, or replayed from the Activity log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NodeTiming {
    /// When every node it depends on had settled.
    pub ready_at_ms: Option<u64>,
    /// When its cell started.
    pub dispatched_at_ms: Option<u64>,
}

impl NodeTiming {
    /// A node that became ready at `ready_at_ms` and is dispatched now.
    fn dispatched_now(ready_at_ms: u64) -> Self {
        Self {
            ready_at_ms: Some(ready_at_ms),
            dispatched_at_ms: Some(crate::control::now_ms()),
        }
    }

    /// How long the node waited for a slot once it was ready.
    #[must_use]
    pub fn slot_wait_ms(&self) -> Option<u64> {
        Some(self.dispatched_at_ms?.saturating_sub(self.ready_at_ms?))
    }
}

/// Output of a full graph execution.
#[derive(Debug, Clone)]
pub struct GraphOutput {
    /// Name of the graph that was executed.
    pub graph_name: String,
    /// Whether the graph completed without failures (untaken routes are allowed).
    pub success: bool,
    /// Per-node execution results in topological order.
    pub node_results: Vec<NodeResult>,
    /// Total wall-clock duration for the full graph execution.
    pub total_duration: Duration,
    /// Gate verdicts stamped on node outputs (live or replayed), keyed by
    /// node. Nodes whose outputs carry no verdict tag are absent.
    pub gate_verdicts: BTreeMap<NodeId, TaskGateVerdict>,
}

impl GraphOutput {
    /// Return a human-readable summary of the graph execution.
    #[must_use]
    pub fn summary(&self) -> String {
        use std::fmt::Write;

        let mut s = String::new();
        let _ = writeln!(s, "Graph: {}", self.graph_name);
        let _ = writeln!(
            s,
            "Status: {}",
            if self.success { "SUCCESS" } else { "FAILED" }
        );
        let _ = writeln!(s, "Duration: {:?}", self.total_duration);
        let _ = writeln!(s, "Nodes: {}", self.node_results.len());
        s.push('\n');
        for result in &self.node_results {
            let stub_marker = if result.is_stub { " [STUB]" } else { "" };
            let dur = if result.duration > Duration::ZERO {
                format!(" ({:?})", result.duration)
            } else {
                String::new()
            };
            let _ = writeln!(
                s,
                "  [{:>8}] {} ({}){}{stub_marker}",
                result.status, result.node_id, result.cell_type, dur
            );
            if let Some(err) = &result.error {
                let _ = writeln!(s, "             error: {err}");
            }
        }

        let stub_count = self.node_results.iter().filter(|r| r.is_stub).count();
        if stub_count > 0 {
            let _ = writeln!(
                s,
                "\nWARNING: {stub_count} node(s) used stub/passthrough cells. \
                 These need real implementations before production use."
            );
        }

        s
    }
}

/// A snapshot of a live graph execution, returned by [`FlowHandle::status`].
#[derive(Debug, Clone)]
pub struct FlowStatus {
    /// Per-node status snapshot at the time of the call.
    pub node_statuses: HashMap<NodeId, NodeStatus>,
    /// Wall-clock time elapsed since the execution started.
    pub elapsed: Duration,
    /// Total budget consumed so far, in microdollars (multiply by 1e-6 for USD).
    pub budget_consumed_microdollars: u64,
}

/// A handle to a live graph execution spawned by [`GraphEngine::start`].
///
/// Provides non-blocking access to per-node status, budget consumption, and
/// cancellation. Follows the same pattern as [`crate::hot::HotGraphHandle`].
pub struct FlowHandle {
    /// Unique identifier for this execution run.
    pub run_id: String,
    /// Name of the graph being executed (from [`crate::types::GraphMetadata::name`]).
    pub graph_id: String,
    /// Wall-clock instant when execution started.
    pub started_at: Instant,
    /// Per-node status, updated atomically as nodes start/complete/fail.
    node_statuses: Arc<parking_lot::Mutex<HashMap<NodeId, NodeStatus>>>,
    /// Total budget consumed in microdollars (1 USD = 1_000_000 microdollars).
    budget_consumed: Arc<AtomicU64>,
    /// Cancellation token -- call [`FlowHandle::cancel`] to request early stop.
    cancel: CancellationToken,
    /// Final graph output, set once the background task finishes.
    result: Arc<parking_lot::Mutex<Option<GraphOutput>>>,
    /// Background task handle.
    join_handle: parking_lot::Mutex<Option<JoinHandle<()>>>,
}

impl FlowHandle {
    /// Return a point-in-time snapshot of execution status.
    pub fn status(&self) -> FlowStatus {
        FlowStatus {
            node_statuses: self.node_statuses.lock().clone(),
            elapsed: self.started_at.elapsed(),
            budget_consumed_microdollars: self.budget_consumed.load(Ordering::Relaxed),
        }
    }

    /// Request cancellation of the running graph execution.
    ///
    /// No further node starts. Nodes already running are not interrupted:
    /// they see the cancellation through [`CellContext::is_cancelled`] and
    /// start no further work (bug-ceb581), and the background task stops
    /// once they complete.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Asynchronously wait for the graph execution to complete.
    ///
    /// Returns the [`GraphOutput`] from the finished run, or `None` if the
    /// background task was dropped or panicked before producing a result.
    ///
    /// Can be called multiple times; only the first call actually awaits the
    /// background task. Subsequent calls return the cached result immediately.
    pub async fn await_completion(&self) -> Option<GraphOutput> {
        let handle = self.join_handle.lock().take();
        if let Some(h) = handle {
            let _ = h.await;
        }
        self.result.lock().clone()
    }

    /// Check whether the background execution task is still running.
    pub fn is_running(&self) -> bool {
        let guard = self.join_handle.lock();
        match &*guard {
            Some(h) => !h.is_finished(),
            None => false,
        }
    }
}

/// A graph whose edges have been validated for type-schema compatibility.
///
/// Produced by [`GraphEngine::validate_for_start`]. All engine entry points
/// (`execute`, `execute_parallel`, `execute_at_tick`, `execute_parallel_at_tick`,
/// `resume_from`, `start`) require this proof token so validation cannot be
/// accidentally skipped.
///
/// This is a zero-cost wrapper; it borrows the engine that already owns the graph.
#[derive(Debug)]
pub struct ValidatedGraph {
    _private: (),
}

/// Asked just before a node's cell starts; see [`GraphEngine::with_dispatch_stop`].
///
/// `Some(reason)` stops the run: no further node starts, nodes already
/// running finish, and every node that has not started is skipped with
/// `reason`.
pub type DispatchStop = Arc<dyn Fn() -> Option<String> + Send + Sync>;

/// The graph execution engine. Holds a graph and registry, executing nodes
/// sequentially or with bounded parallelism according to policy.
pub struct GraphEngine {
    graph: Graph,
    registry: CellRegistry,
    /// Signals supplied by the caller to every root node in the Graph.
    ///
    /// Root nodes have no predecessor outputs to consume, so this is the
    /// ingress boundary for manual, trigger, and nested-Graph executions.
    root_inputs: Vec<roko_core::Signal>,
    /// Optional recorder — when present, Activity node outputs are appended to
    /// a JSONL file after each successful execution.
    recorder: Option<parking_lot::Mutex<ActivityRecorder>>,
    /// Optional replayer — when present, Activity node outputs are read from
    /// the JSONL file instead of re-executing the cell.
    replayer: Option<ActivityReplayer>,
    /// Optional merge queue — when present, a [`MergeRequest`] is enqueued
    /// after a successful graph execution that represents a plan.
    merge_queue: Option<Arc<dyn MergeEnqueuer>>,
    /// Optional passive lifecycle-event sink for the telemetry Lens runtime.
    telemetry: Option<Arc<dyn TelemetryEventSink>>,
    /// Optional graph execution event sink (#246).
    ///
    /// When present, all execution paths (sequential, parallel, `start()`,
    /// resume, Hot Graph) emit rich lifecycle events via a shared helper.
    /// `TelemetryEventSink` is kept unchanged; the engine emits to both sinks.
    event_sink: Option<Arc<dyn crate::events::GraphEventSink>>,
    /// Monotonic sequence counter for graph event emission.
    event_seq: crate::events::EventSeqCounter,
    /// Last complete per-node outputs for stateful Hot Graph ticks.
    tick_state: parking_lot::Mutex<HashMap<NodeId, Vec<roko_core::Signal>>>,
    /// Set to `true` after [`validate_for_start`] succeeds, so Hot Graph tick
    /// loops do not re-validate on every iteration.
    pre_validated: std::sync::atomic::AtomicBool,
    /// When `true`, test-stub descriptors are allowed in the graph. Production
    /// starts set this to `false` (the default) and reject any graph containing
    /// stub descriptors.
    allow_test_stubs: bool,
    /// Asked before each node's cell starts; `Some(reason)` stops the run.
    dispatch_stop: Option<DispatchStop>,
}

impl GraphEngine {
    /// Create a new engine for the given graph and cell registry.
    #[must_use]
    pub fn new(graph: Graph, registry: CellRegistry) -> Self {
        Self {
            graph,
            registry,
            root_inputs: Vec::new(),
            recorder: None,
            replayer: None,
            merge_queue: None,
            telemetry: None,
            event_sink: None,
            event_seq: crate::events::EventSeqCounter::new(),
            tick_state: parking_lot::Mutex::new(HashMap::new()),
            pre_validated: std::sync::atomic::AtomicBool::new(false),
            allow_test_stubs: false,
            dispatch_stop: None,
        }
    }

    /// Supply the input Signals delivered to each root node.
    ///
    /// A Graph may have more than one root. Each root receives an independent
    /// clone of this collection; downstream nodes continue to receive only
    /// their predecessors' outputs.
    #[must_use]
    pub fn with_root_inputs(mut self, inputs: Vec<roko_core::Signal>) -> Self {
        self.root_inputs = inputs;
        self
    }

    /// Restore the last complete per-node outputs for a stateful Hot Graph.
    ///
    /// Unknown node IDs are rejected so a checkpoint from a drifted Graph
    /// cannot silently inject state. The state is consumed only when the
    /// Graph's Hot policy enables `persist_tick_state`.
    pub fn restore_tick_state(
        &self,
        state: HashMap<NodeId, Vec<roko_core::Signal>>,
    ) -> std::result::Result<(), GraphError> {
        if let Some(node_id) = state
            .keys()
            .find(|node_id| !self.graph.node_map.contains_key(*node_id))
        {
            return Err(GraphError::InvalidGraph {
                reason: format!("Hot checkpoint contains unknown node state '{node_id}'"),
            });
        }
        *self.tick_state.lock() = state;
        Ok(())
    }

    /// Snapshot the last complete per-node outputs for a stateful Hot Graph.
    #[must_use]
    pub fn tick_state_snapshot(&self) -> HashMap<NodeId, Vec<roko_core::Signal>> {
        self.tick_state.lock().clone()
    }

    /// Attach an [`ActivityRecorder`] to this engine.
    ///
    /// After every successful Activity node execution the outputs will be
    /// appended to the recorder's JSONL file. Workflow nodes are never recorded.
    #[must_use]
    pub fn with_recorder(mut self, recorder: ActivityRecorder) -> Self {
        self.recorder = Some(parking_lot::Mutex::new(recorder));
        self
    }

    /// Attach an [`ActivityReplayer`] to this engine.
    ///
    /// When a replayer is present and contains a matching entry for an Activity
    /// node at the current tick, the recorded outputs are used directly without
    /// re-executing the cell. Workflow nodes always re-execute.
    #[must_use]
    pub fn with_replayer(mut self, replayer: ActivityReplayer) -> Self {
        self.replayer = Some(replayer);
        self
    }

    /// Attach a [`MergeEnqueuer`] to this engine.
    ///
    /// After a successful graph execution, the engine will enqueue a
    /// [`MergeRequest`] containing the graph name as `plan_id` and any
    /// `files_changed` collected from Activity node outputs. The caller
    /// (typically the plan runner) is responsible for providing an
    /// implementation that bridges to the real merge queue.
    #[must_use]
    pub fn with_merge_queue(mut self, queue: Arc<dyn MergeEnqueuer>) -> Self {
        self.merge_queue = Some(queue);
        self
    }

    /// Attach a passive lifecycle-event sink.
    ///
    /// Telemetry failures are logged and never change Graph or Cell outcomes.
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: Arc<dyn TelemetryEventSink>) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Attach a graph execution event sink (#246).
    ///
    /// When present, all execution paths emit rich lifecycle events via a
    /// shared helper. `TelemetryEventSink` is kept unchanged; the engine
    /// emits to both sinks independently.
    #[must_use]
    pub fn with_event_sink(mut self, sink: Arc<dyn crate::events::GraphEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Allow test-stub descriptors to pass validation.
    ///
    /// By default, `validate_for_start` rejects graphs containing nodes whose
    /// registry descriptors have `is_stub = true`. Call this method with `true`
    /// to permit stubs (test environments only).
    #[must_use]
    pub fn with_allow_test_stubs(mut self, allow: bool) -> Self {
        self.allow_test_stubs = allow;
        self
    }

    /// Stop starting nodes once `stop` returns a reason.
    ///
    /// `stop` is asked just before each node's cell starts, in every
    /// execution path. Once it returns `Some(reason)`, no further node
    /// starts: nodes already running finish, and each node that has not
    /// started is skipped with `reason`. Replayed Activity outputs start no
    /// cell, so they are not held back. Plan runs use this to stop at a spent
    /// budget.
    #[must_use]
    pub fn with_dispatch_stop(mut self, stop: DispatchStop) -> Self {
        self.dispatch_stop = Some(stop);
        self
    }

    /// Why no further node may start, when the dispatch stop says so.
    fn dispatch_stopped(&self) -> Option<String> {
        self.dispatch_stop.as_ref().and_then(|stop| stop())
    }

    /// Append a completed Activity node's outputs to the attached recorder,
    /// with when the node became ready and was dispatched, so a resumed run
    /// can replay them.
    fn record_activity(
        &self,
        node_id: &str,
        tick: u64,
        outputs: &[roko_core::Signal],
        timing: NodeTiming,
    ) -> Result<(), GraphError> {
        let Some(recorder) = &self.recorder else {
            return Ok(());
        };
        let graph_name = &self.graph.metadata.name;
        recorder
            .lock()
            .record_timed(graph_name, node_id, tick, outputs.to_vec(), timing)
            .map_err(|error| GraphError::NodeFailed {
                node_id: node_id.to_string(),
                reason: format!("persist Activity checkpoint: {error}"),
            })
    }

    /// Return a reference to the graph event sequence counter.
    ///
    /// Useful for callers that need to pre-allocate sequence numbers or
    /// inspect the current sequence state.
    #[must_use]
    pub fn event_seq(&self) -> &crate::events::EventSeqCounter {
        &self.event_seq
    }

    /// Validate the graph's edges for type-schema compatibility and return
    /// a [`ValidatedGraph`] proof token.
    ///
    /// This performs side-effect-free introspection via [`CellDescriptor`]
    /// metadata in the registry. No Cells are constructed. The validation
    /// is performed once per start/resume, not on every node dispatch.
    ///
    /// All entry points (`execute`, `execute_parallel`, `execute_at_tick`,
    /// `execute_parallel_at_tick`, `resume_from`, `start`) require the
    /// returned token so validation cannot be accidentally skipped.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::EdgeValidationFailed`] if any edge has
    /// incompatible type schemas between its source output and target input.
    /// The error includes the count and the first mismatch description.
    ///
    /// Returns [`GraphError::InvalidGraph`] if a production start encounters
    /// a graph containing test-stub descriptors.
    pub fn validate_for_start(&self) -> Result<ValidatedGraph, GraphError> {
        // Skip if already validated (Hot Graph tick loops call this path once).
        if self.pre_validated.load(Ordering::Acquire) {
            return Ok(ValidatedGraph { _private: () });
        }

        // Reject test-stub descriptors in production mode.
        if !self.allow_test_stubs {
            let stub_nodes: Vec<String> = self
                .graph
                .inner
                .node_weights()
                .filter_map(|node| {
                    self.registry
                        .descriptor(&node.cell_type)
                        .filter(|d| d.is_stub)
                        .map(|_| node.id.clone())
                })
                .collect();
            if !stub_nodes.is_empty() {
                return Err(GraphError::InvalidGraph {
                    reason: format!(
                        "graph contains {} test-stub node(s): {}",
                        stub_nodes.len(),
                        stub_nodes.join(", ")
                    ),
                });
            }
        }

        // Validate edge type compatibility using descriptor introspection.
        let edge_errors = self.graph.validate_edges(&self.registry);
        if !edge_errors.is_empty() {
            let first = edge_errors[0].to_string();
            return Err(GraphError::EdgeValidationFailed {
                count: edge_errors.len(),
                first_error: first,
            });
        }

        self.pre_validated.store(true, Ordering::Release);
        Ok(ValidatedGraph { _private: () })
    }

    /// Execute the graph using the configured concurrency policy.
    ///
    /// Validates all edges for type-schema compatibility before executing any
    /// node. If validation fails, returns immediately without spawning work.
    ///
    /// Each node is instantiated from the registry, executed with inputs from
    /// upstream nodes, and its outputs are stored for downstream consumption.
    /// If a node fails, all its transitive dependents are marked as Skipped.
    ///
    /// When a [`ActivityReplayer`] is attached (via [`GraphEngine::with_replayer`]),
    /// Activity nodes whose outputs have been previously recorded are substituted
    /// with those recorded outputs instead of re-executing the cell.
    ///
    /// When an [`ActivityRecorder`] is attached (via [`GraphEngine::with_recorder`]),
    /// every successful Activity node execution is written to the JSONL log.
    ///
    /// Workflow nodes always re-execute regardless of recorder/replayer state.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeValidationFailed` if edges have incompatible schemas,
    /// `GraphError::CycleDetected` if the graph contains a cycle, or
    /// `GraphError::UnknownCellType` if a node references an unregistered cell type.
    pub async fn execute(&self, ctx: &CellContext) -> Result<GraphOutput, GraphError> {
        let _validated = self.validate_for_start()?;
        if self.graph.policy.max_concurrent_nodes > 1 {
            self.execute_parallel_at_tick_validated(ctx, 0).await
        } else {
            // tick = 0 for one-shot (non-Hot) graph executions.
            self.execute_at_tick_validated(ctx, 0).await
        }
    }

    /// Execute the graph at a specific tick index.
    ///
    /// Validates all edges for type-schema compatibility before executing any
    /// node. Used internally by [`GraphEngine::execute`] (tick 0) and by Hot Graph
    /// tick loops (tick N). The tick is threaded through to the recorder/replayer
    /// so multi-tick runs can store and retrieve per-tick Activity outputs.
    #[allow(clippy::too_many_lines)]
    pub async fn execute_at_tick(
        &self,
        ctx: &CellContext,
        tick: u64,
    ) -> Result<GraphOutput, GraphError> {
        let _validated = self.validate_for_start()?;
        self.execute_at_tick_validated(ctx, tick).await
    }

    /// Internal: execute at tick after validation has been performed.
    #[allow(clippy::too_many_lines)]
    async fn execute_at_tick_validated(
        &self,
        ctx: &CellContext,
        tick: u64,
    ) -> Result<GraphOutput, GraphError> {
        let start = Instant::now();
        let graph_name = self.graph.metadata.name.clone();
        let run_id = ctx.run_id.clone().unwrap_or_else(|| graph_name.clone());
        let graph_ancestry = [LensScope::Graph(graph_name.clone())];
        self.emit_telemetry(
            &ObservableEvent::GraphStarted {
                graph: graph_name.clone(),
                run: run_id.clone(),
                input_hash: input_signal_hash(&self.root_inputs),
            },
            &graph_ancestry,
        )
        .await;
        let mut total_cost_usd = 0.0;

        // 1. Topological sort
        let order = topological_order(&self.graph)?;

        // 2. Track outputs and terminal status for conditional routing.
        let mut outputs = self.initial_tick_outputs();
        let mut statuses: HashMap<NodeId, NodeStatus> = HashMap::new();
        let mut results: Vec<NodeResult> = Vec::with_capacity(order.len());
        // How many results the event sink has been told about.
        let mut published = 0;
        // Once set, no further node starts: why the rest are skipped.
        let mut abort: Option<String> = None;
        let mut resumed_emitted = false;
        let mut clock = SettleClock::start();
        let mut previous: Option<&NodeId> = None;

        // 3. Execute each node in order
        for node_id in &order {
            // The previous node settled when the loop moved on from it.
            if let Some(previous) = previous.replace(node_id) {
                clock.settle(previous);
            }
            published = self
                .publish_settled(&run_id, &results, published, &outputs)
                .await;
            // SAFETY: topological_order only returns IDs that are in the graph.
            let Some(node) = self.graph.get_node(node_id) else {
                continue;
            };

            if let Some(reason) = &abort {
                let result = skipped_result(node, NodeStatus::Skipped, reason.clone());
                statuses.insert(node_id.clone(), result.status);
                results.push(result);
                continue;
            }

            let input = match evaluate_node_activation(&self.graph, node_id, &statuses, &outputs) {
                NodeActivation::Root => self.root_tick_inputs(node_id, &outputs),
                NodeActivation::Ready(input) => input,
                NodeActivation::ConditionSkipped(reason) => {
                    let result = NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::ConditionSkipped,
                        duration: Duration::ZERO,
                        error: Some(reason),
                        output_count: 0,
                        is_stub: false,
                        blocked_by: None,
                        timing: NodeTiming::default(),
                    };
                    statuses.insert(node_id.clone(), result.status);
                    results.push(result);
                    continue;
                }
                NodeActivation::UpstreamFailed { reason, dependency } => {
                    let result = NodeResult {
                        blocked_by: dependency
                            .as_deref()
                            .and_then(|dependency| failed_root(dependency, &statuses, &results)),
                        ..skipped_result(node, NodeStatus::Skipped, reason)
                    };
                    statuses.insert(node_id.clone(), result.status);
                    results.push(result);
                    continue;
                }
            };

            let is_activity = node.execution_class == ExecutionClass::Activity;

            // For Activity nodes: check replayer for a pre-recorded result.
            if is_activity
                && let Some(replayer) = &self.replayer
                && let Some(recorded) = replayer.lookup(node_id, tick)
            {
                let mut recorded = recorded.clone();
                propagate_input_taint(&input, &mut recorded, node_id);
                let count = recorded.len();
                if !resumed_emitted {
                    self.emit_telemetry(
                        &ObservableEvent::GraphResumed {
                            graph: graph_name.clone(),
                            run: run_id.clone(),
                        },
                        &graph_ancestry,
                    )
                    .await;
                    resumed_emitted = true;
                }
                info!(
                    node_id = %node_id,
                    tick,
                    outputs = count,
                    "replay: substituting recorded Activity output"
                );
                outputs.insert(node_id.clone(), recorded);
                statuses.insert(node_id.clone(), NodeStatus::Complete);
                results.push(NodeResult {
                    node_id: node_id.clone(),
                    cell_type: node.cell_type.clone(),
                    status: NodeStatus::Complete,
                    duration: Duration::ZERO,
                    error: None,
                    output_count: count,
                    is_stub: false,
                    blocked_by: None,
                    timing: NodeTiming::default(),
                });
                continue;
            }

            if let Some(reason) = self.dispatch_stopped() {
                info!(node_id = %node_id, %reason, "dispatch stopped: starting no further nodes");
                let result = skipped_result(node, NodeStatus::Skipped, reason.clone());
                statuses.insert(node_id.clone(), result.status);
                results.push(result);
                abort = Some(reason);
                continue;
            }

            // Instantiate cell from registry
            let cell: Box<dyn Cell> = self.registry.create(&node.cell_type, node.config.clone())?;

            let input_hash = input_signal_hash(&input);
            let cell_is_stub = cell.is_stub();
            let estimated_cost_usd = cell.estimated_cost().unwrap_or_default();
            let cell_ancestry = [
                LensScope::Cell(node_id.clone()),
                LensScope::Graph(graph_name.clone()),
            ];
            self.emit_telemetry(
                &ObservableEvent::CellStarted {
                    block: node_id.clone(),
                    run: run_id.clone(),
                    input_hash,
                },
                &cell_ancestry,
            )
            .await;
            self.publish_node_started(&run_id, node).await;

            info!(node_id = %node_id, cell_type = %node.cell_type, "executing node");
            let timing = NodeTiming::dispatched_now(clock.ready_at_ms(&self.graph, node_id));
            let node_start = Instant::now();

            // Execute the cell, applying the graph's retry policy without
            // treating intermediate failures as terminal Cell failures.
            let (execution, attempts) = execute_cell_with_retries(
                cell.as_ref(),
                input,
                ctx,
                max_retries(&self.graph.policy),
                self.telemetry.as_ref(),
                node_id,
                &run_id,
                &cell_ancestry,
            )
            .await;
            match execution {
                Ok(output_signals) => {
                    let duration = node_start.elapsed();
                    let duration_ms = duration_ms(duration);
                    let count = output_signals.len();
                    total_cost_usd += estimated_cost_usd * f64::from(attempts);
                    info!(
                        node_id = %node_id,
                        outputs = count,
                        duration_ms = duration.as_millis(),
                        "node complete"
                    );

                    // For Activity nodes: record the output if a recorder is present.
                    if is_activity {
                        self.record_activity(node_id, tick, &output_signals, timing)?;
                    }

                    self.emit_telemetry(
                        &ObservableEvent::CellCompleted {
                            block: node_id.clone(),
                            run: run_id.clone(),
                            duration_ms,
                            cost_usd: estimated_cost_usd * f64::from(attempts),
                        },
                        &cell_ancestry,
                    )
                    .await;
                    self.emit_telemetry(
                        &ObservableEvent::GraphNodeCompleted {
                            graph: graph_name.clone(),
                            run: run_id.clone(),
                            node: node_id.clone(),
                            duration_ms,
                        },
                        &cell_ancestry,
                    )
                    .await;

                    outputs.insert(node_id.clone(), output_signals);
                    statuses.insert(node_id.clone(), NodeStatus::Complete);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Complete,
                        duration,
                        error: None,
                        output_count: count,
                        is_stub: cell_is_stub,
                        blocked_by: None,
                        timing,
                    });
                }
                Err(e) => {
                    let duration = node_start.elapsed();
                    let msg = e.to_string();
                    total_cost_usd += estimated_cost_usd * f64::from(attempts);
                    warn!(
                        node_id = %node_id,
                        error = %msg,
                        duration_ms = duration.as_millis(),
                        "node failed"
                    );
                    statuses.insert(node_id.clone(), NodeStatus::Failed);
                    if matches!(
                        self.graph.policy.failure_strategy,
                        crate::types::FailureStrategy::FailFast
                    ) {
                        abort.get_or_insert_with(|| "aborted after graph failure".to_string());
                    }
                    self.emit_telemetry(
                        &ObservableEvent::CellFailed {
                            block: node_id.clone(),
                            run: run_id.clone(),
                            error: msg.clone(),
                        },
                        &cell_ancestry,
                    )
                    .await;
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Failed,
                        duration,
                        error: Some(msg),
                        output_count: 0,
                        is_stub: cell_is_stub,
                        blocked_by: None,
                        timing,
                    });
                }
            }
        }
        self.publish_settled(&run_id, &results, published, &outputs)
            .await;

        let total_duration = start.elapsed();
        let success = graph_execution_succeeded(&results);

        if success {
            self.emit_telemetry(
                &ObservableEvent::GraphCompleted {
                    graph: graph_name.clone(),
                    run: run_id,
                    duration_ms: duration_ms(total_duration),
                    cost_usd: total_cost_usd,
                },
                &graph_ancestry,
            )
            .await;
        } else {
            self.emit_telemetry(
                &ObservableEvent::GraphFailed {
                    graph: graph_name.clone(),
                    run: run_id,
                    error: "one or more graph nodes failed".to_string(),
                },
                &graph_ancestry,
            )
            .await;
        }

        // After successful execution, enqueue a merge request if a merge queue
        // is attached. Collect files_changed from Activity node outputs via
        // the "files_changed" tag convention.
        if success && let Some(merge_queue) = &self.merge_queue {
            let files_changed = Self::collect_files_changed(&outputs);
            if !files_changed.is_empty() {
                let request = MergeRequest {
                    plan_id: graph_name.clone(),
                    branch_name: String::new(), // caller sets via merge queue impl
                    files_changed,
                    priority: 0,
                };
                let accepted = merge_queue.enqueue(request);
                info!(
                    graph = %graph_name,
                    accepted,
                    "merge request enqueued after successful execution"
                );
            }
        }

        self.persist_tick_outputs(&outputs);

        Ok(GraphOutput {
            graph_name,
            success,
            node_results: results,
            total_duration,
            gate_verdicts: collect_gate_verdicts(&outputs),
        })
    }

    /// Execute the graph with bounded parallel node execution.
    ///
    /// Validates all edges for type-schema compatibility before executing any
    /// node.
    ///
    /// Each node starts as soon as every node it depends on has settled, on a
    /// `tokio::task::JoinSet`, with at most
    /// [`GraphPolicy::max_concurrent_nodes`] running at once. A node never
    /// waits for nodes it does not depend on. A failed node blocks only its
    /// dependants, unless the failure strategy is `FailFast`: then no further
    /// node starts after the first failure, and nodes already running finish.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeValidationFailed` if edges have incompatible schemas,
    /// `GraphError::CycleDetected` if the graph contains a cycle, or
    /// `GraphError::UnknownCellType` if a node references an unregistered cell type.
    #[allow(clippy::too_many_lines)]
    pub async fn execute_parallel(&self, ctx: &CellContext) -> Result<GraphOutput, GraphError> {
        self.execute_parallel_at_tick(ctx, 0).await
    }

    /// Execute with bounded parallelism at a specific Hot Graph tick.
    ///
    /// Validates all edges for type-schema compatibility before executing any
    /// node. The tick is part of Activity replay/record identity; keeping it
    /// explicit prevents multi-tick runs from reusing tick-zero evidence.
    #[allow(clippy::too_many_lines)]
    pub async fn execute_parallel_at_tick(
        &self,
        ctx: &CellContext,
        tick: u64,
    ) -> Result<GraphOutput, GraphError> {
        let _validated = self.validate_for_start()?;
        self.execute_parallel_at_tick_validated(ctx, tick).await
    }

    /// Internal: execute parallel at tick after validation has been performed.
    async fn execute_parallel_at_tick_validated(
        &self,
        ctx: &CellContext,
        tick: u64,
    ) -> Result<GraphOutput, GraphError> {
        let start = Instant::now();
        let graph_name = self.graph.metadata.name.clone();
        let run_id = ctx.run_id.clone().unwrap_or_else(|| graph_name.clone());
        let graph_ancestry = [LensScope::Graph(graph_name.clone())];
        self.emit_telemetry(
            &ObservableEvent::GraphStarted {
                graph: graph_name.clone(),
                run: run_id.clone(),
                input_hash: input_signal_hash(&self.root_inputs),
            },
            &graph_ancestry,
        )
        .await;

        let statuses = parking_lot::Mutex::new(HashMap::new());
        let ReadyQueueRun {
            outputs,
            results,
            total_cost_usd,
            ..
        } = self
            .execute_ready_queue(
                ctx,
                ReadyQueueOptions {
                    tick,
                    statuses: &statuses,
                    cancel: None,
                    announce_resume: true,
                },
            )
            .await?;

        let total_duration = start.elapsed();
        let success = graph_execution_succeeded(&results);

        if success {
            self.emit_telemetry(
                &ObservableEvent::GraphCompleted {
                    graph: graph_name.clone(),
                    run: run_id,
                    duration_ms: duration_ms(total_duration),
                    cost_usd: total_cost_usd,
                },
                &graph_ancestry,
            )
            .await;
        } else {
            self.emit_telemetry(
                &ObservableEvent::GraphFailed {
                    graph: graph_name.clone(),
                    run: run_id,
                    error: "one or more graph nodes failed".to_string(),
                },
                &graph_ancestry,
            )
            .await;
        }

        if success && let Some(merge_queue) = &self.merge_queue {
            let files_changed = Self::collect_files_changed(&outputs);
            if !files_changed.is_empty() {
                let accepted = merge_queue.enqueue(MergeRequest {
                    plan_id: graph_name.clone(),
                    branch_name: String::new(),
                    files_changed,
                    priority: 0,
                });
                info!(graph = %graph_name, accepted, "parallel merge request enqueued");
            }
        }

        self.persist_tick_outputs(&outputs);

        Ok(GraphOutput {
            graph_name,
            success,
            node_results: results,
            total_duration,
            gate_verdicts: collect_gate_verdicts(&outputs),
        })
    }

    /// Run the Graph with bounded parallelism, starting each node as soon as
    /// every one of its predecessors has settled.
    ///
    /// A node is decided once all of its predecessors are terminal: it is
    /// condition-skipped, skipped after a failed dependency, replayed from the
    /// Activity log, or queued to run. Queued nodes start in topological order
    /// whenever fewer than `max_concurrent_nodes` cells are running, so a node
    /// never waits for nodes it does not depend on.
    ///
    /// The one exception is [`Node::exclusive`]: a queued node whose paths
    /// overlap a running node's waits until that node finishes. It holds no
    /// slot while it waits, so queued nodes after it may start first.
    ///
    /// A failed node blocks only its dependants, unless the policy is
    /// `FailFast`: then no further node starts after the first failure. No
    /// further node starts once `cancel` fires either. Nodes already running
    /// always finish, and their Activity outputs are recorded, so a resumed
    /// run can replay them.
    #[allow(clippy::too_many_lines)]
    async fn execute_ready_queue(
        &self,
        ctx: &CellContext,
        options: ReadyQueueOptions<'_>,
    ) -> Result<ReadyQueueRun, GraphError> {
        use tokio::task::JoinSet;

        let ReadyQueueOptions {
            tick,
            statuses,
            cancel,
            announce_resume,
        } = options;
        let graph_name = self.graph.metadata.name.clone();
        let run_id = ctx.run_id.clone().unwrap_or_else(|| graph_name.clone());
        let max_concurrent = self.graph.policy.max_concurrent_nodes.max(1);
        let max_retries = max_retries(&self.graph.policy);
        let fail_fast = matches!(
            self.graph.policy.failure_strategy,
            crate::types::FailureStrategy::FailFast
        );

        let mut queue = ReadyQueue::new(&self.graph)?;
        {
            let mut statuses = statuses.lock();
            for pos in 0..queue.len() {
                statuses.insert(queue.node(pos).id.clone(), NodeStatus::Pending);
            }
        }
        let mut outputs = self.initial_tick_outputs();
        let mut results: Vec<NodeResult> = Vec::with_capacity(queue.len());
        // How many results the event sink has been told about.
        let mut published = 0;
        let mut total_cost_usd = 0.0;
        let mut resumed_emitted = false;
        let mut was_cancelled = false;
        let mut halt: Option<Halt> = None;
        // Nodes cleared to run that wait for a free slot, or for a running
        // node that holds their exclusive paths, with their input.
        let mut runnable: BTreeMap<usize, Vec<roko_core::Signal>> = BTreeMap::new();
        // The queued nodes held back by exclusive paths, with the running
        // node each was last seen waiting for.
        let mut waiting: BTreeMap<usize, usize> = BTreeMap::new();
        let mut running: JoinSet<NodeRun> = JoinSet::new();
        let mut running_nodes: HashMap<tokio::task::Id, usize> = HashMap::new();
        // When each running node became ready and was dispatched.
        let mut timings: HashMap<usize, NodeTiming> = HashMap::new();

        loop {
            if !matches!(halt, Some(Halt::Cancelled))
                && cancel.is_some_and(CancellationToken::is_cancelled)
            {
                info!(graph = %graph_name, "flow cancelled: starting no further nodes");
                halt = Some(Halt::Cancelled);
            }
            if halt.is_some() {
                // Queued nodes will not start: decide them again as halted.
                queue
                    .ready
                    .extend(std::mem::take(&mut runnable).into_keys());
            }

            // Decide every ready node in topological order. Settling a node
            // can make its dependants ready; they are decided in this pass.
            while let Some(pos) = queue.ready.pop_first() {
                let node = queue.node(pos);
                if let Some(halt) = &halt {
                    statuses.lock().insert(node.id.clone(), NodeStatus::Skipped);
                    match halt {
                        Halt::Cancelled => {
                            was_cancelled = true;
                            self.emit_telemetry(
                                &ObservableEvent::CellCancelled {
                                    block: node.id.clone(),
                                    run: run_id.clone(),
                                },
                                &[
                                    LensScope::Cell(node.id.clone()),
                                    LensScope::Graph(graph_name.clone()),
                                ],
                            )
                            .await;
                        }
                        Halt::FailFast => results.push(skipped_result(
                            node,
                            NodeStatus::Skipped,
                            "aborted after graph failure".to_string(),
                        )),
                        Halt::Stopped(reason) => {
                            results.push(skipped_result(node, NodeStatus::Skipped, reason.clone()));
                        }
                    }
                    queue.settle(pos);
                    continue;
                }

                let activation = {
                    let status_guard = statuses.lock();
                    evaluate_node_activation(&self.graph, &node.id, &status_guard, &outputs)
                };
                let input = match activation {
                    NodeActivation::Root => self.root_tick_inputs(&node.id, &outputs),
                    NodeActivation::Ready(input) => input,
                    NodeActivation::ConditionSkipped(reason) => {
                        statuses
                            .lock()
                            .insert(node.id.clone(), NodeStatus::ConditionSkipped);
                        results.push(skipped_result(node, NodeStatus::ConditionSkipped, reason));
                        queue.settle(pos);
                        continue;
                    }
                    NodeActivation::UpstreamFailed { reason, dependency } => {
                        let mut status_guard = statuses.lock();
                        let blocked_by = dependency.as_deref().and_then(|dependency| {
                            failed_root(dependency, &status_guard, &results)
                        });
                        status_guard.insert(node.id.clone(), NodeStatus::Skipped);
                        drop(status_guard);
                        results.push(NodeResult {
                            blocked_by,
                            ..skipped_result(node, NodeStatus::Skipped, reason)
                        });
                        queue.settle(pos);
                        continue;
                    }
                };

                // Activity nodes with a recorded output are replayed, not run.
                if node.execution_class == ExecutionClass::Activity
                    && let Some(replayer) = &self.replayer
                    && let Some(recorded) = replayer.lookup(&node.id, tick)
                {
                    let mut recorded = recorded.clone();
                    propagate_input_taint(&input, &mut recorded, &node.id);
                    if announce_resume && !resumed_emitted {
                        self.emit_telemetry(
                            &ObservableEvent::GraphResumed {
                                graph: graph_name.clone(),
                                run: run_id.clone(),
                            },
                            &[LensScope::Graph(graph_name.clone())],
                        )
                        .await;
                        resumed_emitted = true;
                    }
                    let count = recorded.len();
                    info!(
                        node_id = %node.id,
                        tick,
                        outputs = count,
                        "replay: substituting recorded Activity output"
                    );
                    outputs.insert(node.id.clone(), recorded);
                    statuses
                        .lock()
                        .insert(node.id.clone(), NodeStatus::Complete);
                    results.push(NodeResult {
                        node_id: node.id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Complete,
                        duration: Duration::ZERO,
                        error: None,
                        output_count: count,
                        is_stub: false,
                        blocked_by: None,
                        timing: NodeTiming::default(),
                    });
                    queue.settle(pos);
                    continue;
                }

                runnable.insert(pos, input);
            }
            published = self
                .publish_settled(&run_id, &results, published, &outputs)
                .await;

            // Start queued nodes, in topological order, while slots are free.
            while running.len() < max_concurrent {
                // Pass over each node whose exclusive paths overlap a running
                // node's. It keeps its place in the queue and holds no slot.
                let mut busy: Vec<usize> = running_nodes.values().copied().collect();
                busy.sort_unstable();
                let mut startable = None;
                for &pos in runnable.keys() {
                    let Some((holder, path, held)) = queue.exclusion_conflict(pos, &busy) else {
                        startable = Some(pos);
                        break;
                    };
                    if waiting.insert(pos, holder) != Some(holder) {
                        info!(
                            graph = %graph_name,
                            node_id = %queue.node(pos).id,
                            waits_for = %queue.node(holder).id,
                            path,
                            held,
                            "exclusive paths overlap a running node: waiting for it to finish"
                        );
                    }
                }
                let Some(pos) = startable else {
                    break;
                };
                if let Some(reason) = self.dispatch_stopped() {
                    info!(graph = %graph_name, %reason, "dispatch stopped: starting no further nodes");
                    halt = Some(Halt::Stopped(reason));
                    break;
                }
                let input = runnable.remove(&pos).unwrap_or_default();
                let node = queue.node(pos);
                if waiting.remove(&pos).is_some() {
                    info!(
                        graph = %graph_name,
                        node_id = %node.id,
                        "exclusive paths free: starting the node that waited"
                    );
                }
                let cell = self.registry.create(&node.cell_type, node.config.clone())?;
                statuses.lock().insert(node.id.clone(), NodeStatus::Running);
                self.publish_node_started(&run_id, node).await;
                timings.insert(pos, NodeTiming::dispatched_now(queue.ready_at_ms(pos)));
                let task = running.spawn(run_node(NodeLaunch {
                    node_id: node.id.clone(),
                    cell_type: node.cell_type.clone(),
                    cell,
                    input,
                    ctx: ctx.clone(),
                    graph_name: graph_name.clone(),
                    run_id: run_id.clone(),
                    telemetry: self.telemetry.clone(),
                    max_retries,
                }));
                running_nodes.insert(task.id(), pos);
            }

            if halt.is_some() && !runnable.is_empty() {
                // The dispatch stop just fired: settle the queued nodes.
                continue;
            }
            // Nothing is running, so nothing can become ready: every node
            // has settled.
            if running.is_empty() {
                break;
            }

            // Wait for the next node to finish, or for cancellation.
            let next = match cancel {
                Some(cancel) if !matches!(halt, Some(Halt::Cancelled)) => {
                    tokio::select! {
                        next = running.join_next_with_id() => next,
                        () = cancel.cancelled() => continue,
                    }
                }
                _ => running.join_next_with_id().await,
            };
            let (task, outcome) = match next {
                Some(Ok((task, run))) => (task, Ok(run)),
                Some(Err(error)) => (error.id(), Err(error)),
                None => continue,
            };
            let Some(pos) = running_nodes.remove(&task) else {
                continue;
            };
            let node = queue.node(pos);
            let mut run = match outcome {
                Ok(run) => run,
                Err(error) => {
                    // A panicking cell fails its node, so its dependants are
                    // skipped rather than left waiting.
                    let error = format!("node task panicked: {error}");
                    self.emit_telemetry(
                        &ObservableEvent::CellFailed {
                            block: node.id.clone(),
                            run: run_id.clone(),
                            error: error.clone(),
                        },
                        &[
                            LensScope::Cell(node.id.clone()),
                            LensScope::Graph(graph_name.clone()),
                        ],
                    )
                    .await;
                    NodeRun {
                        result: NodeResult {
                            node_id: node.id.clone(),
                            cell_type: node.cell_type.clone(),
                            status: NodeStatus::Failed,
                            duration: Duration::ZERO,
                            error: Some(error),
                            output_count: 0,
                            is_stub: false,
                            blocked_by: None,
                            timing: NodeTiming::default(),
                        },
                        cost_usd: 0.0,
                        outputs: Vec::new(),
                    }
                }
            };

            run.result.timing = timings.remove(&pos).unwrap_or_default();
            total_cost_usd += run.cost_usd;
            if run.result.status == NodeStatus::Complete {
                // Persist Activity outputs so a future --resume-plan can
                // substitute them instead of re-calling the provider.
                if node.execution_class == ExecutionClass::Activity {
                    self.record_activity(&node.id, tick, &run.outputs, run.result.timing)?;
                }
                outputs.insert(node.id.clone(), run.outputs);
            } else {
                warn!(
                    node_id = %node.id,
                    error = run.result.error.as_deref().unwrap_or_default(),
                    "parallel node failed"
                );
                if fail_fast && halt.is_none() {
                    halt = Some(Halt::FailFast);
                }
            }
            statuses.lock().insert(node.id.clone(), run.result.status);
            results.push(run.result);
            queue.settle(pos);
            published = self
                .publish_settled(&run_id, &results, published, &outputs)
                .await;
        }
        self.publish_settled(&run_id, &results, published, &outputs)
            .await;

        results.sort_by_key(|result| queue.position_of(&result.node_id));
        Ok(ReadyQueueRun {
            outputs,
            results,
            total_cost_usd,
            was_cancelled,
        })
    }

    /// Capture a serializable snapshot of the current execution state.
    ///
    /// Only Activity node outputs are included; Workflow node outputs are
    /// omitted because they can be re-derived from inputs on resume.
    #[must_use]
    pub fn snapshot(
        &self,
        node_statuses: &HashMap<NodeId, NodeStatus>,
        node_outputs: &HashMap<NodeId, Vec<roko_core::Signal>>,
        tick: u64,
    ) -> GraphSnapshot {
        self.snapshot_with_budget(node_statuses, node_outputs, tick, 0, 0, 0)
    }

    /// Capture a serializable snapshot with budget and event sequence state.
    ///
    /// Like [`snapshot`](Self::snapshot) but records cumulative spend,
    /// reservations, and the last emitted event sequence number for monotonic
    /// replay guarantees.
    #[must_use]
    pub fn snapshot_with_budget(
        &self,
        node_statuses: &HashMap<NodeId, NodeStatus>,
        node_outputs: &HashMap<NodeId, Vec<roko_core::Signal>>,
        tick: u64,
        budget_spent_micro_usd: u64,
        budget_reserved_micro_usd: u64,
        last_event_seq: u64,
    ) -> GraphSnapshot {
        let mut snap_statuses = HashMap::new();
        for (id, status) in node_statuses {
            snap_statuses.insert(id.clone(), SerializableNodeStatus::from(*status));
        }

        let mut snap_outputs = HashMap::new();
        for (id, signals) in node_outputs {
            // Only snapshot Activity node outputs.
            if let Some(node) = self.graph.get_node(id)
                && node.execution_class == ExecutionClass::Activity
            {
                let serialized: Vec<SerializableSignal> = signals
                    .iter()
                    .filter_map(|e| {
                        serde_json::to_value(e)
                            .ok()
                            .map(|json| SerializableSignal { json })
                    })
                    .collect();
                if !serialized.is_empty() {
                    snap_outputs.insert(id.clone(), serialized);
                }
            }
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let graph_fingerprint =
            crate::fingerprint::graph_execution_fingerprint(&self.graph).unwrap_or_default();

        GraphSnapshot {
            schema_version: GRAPH_SNAPSHOT_SCHEMA_VERSION,
            graph_name: self.graph.metadata.name.clone(),
            graph_id: self.graph.metadata.name.clone(),
            graph_fingerprint,
            node_statuses: snap_statuses,
            node_outputs: snap_outputs,
            tick_count: tick,
            budget_spent_micro_usd,
            budget_reserved_micro_usd,
            last_event_seq,
            created_at_ms: now,
            policy: self.graph.policy.clone(),
        }
    }

    /// Resume a graph engine from a previously captured snapshot.
    ///
    /// Validates all edges for type-schema compatibility before resuming any
    /// node execution.
    ///
    /// Activity nodes that were `Complete` are restored without re-execution.
    /// Completed Workflow nodes are re-derived because snapshots intentionally
    /// omit their outputs. Pending and Running nodes are also re-executed.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeValidationFailed` if edges have incompatible schemas,
    /// or an error if the graph contains a cycle or references unknown cell types.
    #[allow(clippy::too_many_lines)]
    pub async fn resume_from(
        snapshot: &GraphSnapshot,
        graph: Graph,
        registry: CellRegistry,
        ctx: &CellContext,
    ) -> Result<GraphOutput, GraphError> {
        // Reject test-stub descriptors (resume is always a production path).
        let stub_nodes: Vec<String> = graph
            .inner
            .node_weights()
            .filter_map(|node| {
                registry
                    .descriptor(&node.cell_type)
                    .filter(|d| d.is_stub)
                    .map(|_| node.id.clone())
            })
            .collect();
        if !stub_nodes.is_empty() {
            return Err(GraphError::InvalidGraph {
                reason: format!(
                    "graph contains {} test-stub node(s): {}",
                    stub_nodes.len(),
                    stub_nodes.join(", ")
                ),
            });
        }

        // Validate edges before any resumption work.
        let edge_errors = graph.validate_edges(&registry);
        if !edge_errors.is_empty() {
            let first = edge_errors[0].to_string();
            return Err(GraphError::EdgeValidationFailed {
                count: edge_errors.len(),
                first_error: first,
            });
        }

        let start = Instant::now();
        let graph_name = graph.metadata.name.clone();

        let order = topological_order(&graph)?;

        let mut outputs: HashMap<NodeId, Vec<roko_core::Signal>> = HashMap::new();
        let mut statuses: HashMap<NodeId, NodeStatus> = HashMap::new();
        let mut results: Vec<NodeResult> = Vec::with_capacity(order.len());

        // Restore completed Activity node outputs from the snapshot.
        for (node_id, serialized_signals) in &snapshot.node_outputs {
            let signals: Vec<roko_core::Signal> = serialized_signals
                .iter()
                .filter_map(|se| serde_json::from_value(se.json.clone()).ok())
                .collect();
            if !signals.is_empty() {
                outputs.insert(node_id.clone(), signals);
            }
        }
        let mut clock = SettleClock::start();
        let mut previous: Option<&NodeId> = None;

        for node_id in &order {
            // The previous node settled when the loop moved on from it.
            if let Some(previous) = previous.replace(node_id) {
                clock.settle(previous);
            }
            let Some(node) = graph.get_node(node_id) else {
                continue;
            };

            // Restore terminal Activity and route statuses. Workflow outputs
            // are not snapshotted, so completed Workflow nodes re-execute.
            // Running nodes without a registered reconciliation owner are
            // treated as Pending and re-executed.
            if let Some(snap_status) = snapshot.node_statuses.get(node_id) {
                let status: NodeStatus = reconcile_running_status(*snap_status);
                if status == NodeStatus::Complete
                    && node.execution_class == ExecutionClass::Activity
                {
                    let output_count = outputs.get(node_id).map_or(0, Vec::len);
                    statuses.insert(node_id.clone(), NodeStatus::Complete);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Complete,
                        duration: Duration::ZERO,
                        error: None,
                        output_count,
                        is_stub: false,
                        blocked_by: None,
                        timing: NodeTiming::default(),
                    });
                    continue;
                }
                if status == NodeStatus::ConditionSkipped {
                    statuses.insert(node_id.clone(), NodeStatus::ConditionSkipped);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::ConditionSkipped,
                        duration: Duration::ZERO,
                        error: Some("conditional route was not selected in snapshot".to_string()),
                        output_count: 0,
                        is_stub: false,
                        blocked_by: None,
                        timing: NodeTiming::default(),
                    });
                    continue;
                }
                if status == NodeStatus::Skipped {
                    statuses.insert(node_id.clone(), NodeStatus::Skipped);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Skipped,
                        duration: Duration::ZERO,
                        error: Some("skipped in snapshot".to_string()),
                        output_count: 0,
                        is_stub: false,
                        blocked_by: None,
                        timing: NodeTiming::default(),
                    });
                    continue;
                }
                if status == NodeStatus::Failed {
                    statuses.insert(node_id.clone(), NodeStatus::Failed);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Failed,
                        duration: Duration::ZERO,
                        error: Some("failed in snapshot".to_string()),
                        output_count: 0,
                        is_stub: false,
                        blocked_by: None,
                        timing: NodeTiming::default(),
                    });
                    continue;
                }
            }

            let input = match evaluate_node_activation(&graph, node_id, &statuses, &outputs) {
                NodeActivation::Root => Vec::new(),
                NodeActivation::Ready(input) => input,
                NodeActivation::ConditionSkipped(reason) => {
                    statuses.insert(node_id.clone(), NodeStatus::ConditionSkipped);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::ConditionSkipped,
                        duration: Duration::ZERO,
                        error: Some(reason),
                        output_count: 0,
                        is_stub: false,
                        blocked_by: None,
                        timing: NodeTiming::default(),
                    });
                    continue;
                }
                NodeActivation::UpstreamFailed { reason, dependency } => {
                    let blocked_by = dependency
                        .as_deref()
                        .and_then(|dependency| failed_root(dependency, &statuses, &results));
                    statuses.insert(node_id.clone(), NodeStatus::Skipped);
                    results.push(NodeResult {
                        blocked_by,
                        ..skipped_result(node, NodeStatus::Skipped, reason)
                    });
                    continue;
                }
            };

            // Re-execute pending nodes and all Workflow nodes.
            let cell: Box<dyn Cell> = registry.create(&node.cell_type, node.config.clone())?;
            let cell_is_stub = cell.is_stub();

            info!(node_id = %node_id, cell_type = %node.cell_type, "resume: executing node");
            let timing = NodeTiming::dispatched_now(clock.ready_at_ms(&graph, node_id));
            let node_start = Instant::now();

            let input_taint = input.clone();
            match cell.execute(input, ctx).await {
                Ok(mut output_signals) => {
                    propagate_input_taint(&input_taint, &mut output_signals, node_id);
                    let duration = node_start.elapsed();
                    let count = output_signals.len();
                    outputs.insert(node_id.clone(), output_signals);
                    statuses.insert(node_id.clone(), NodeStatus::Complete);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Complete,
                        duration,
                        error: None,
                        output_count: count,
                        is_stub: cell_is_stub,
                        blocked_by: None,
                        timing,
                    });
                }
                Err(e) => {
                    let duration = node_start.elapsed();
                    let msg = e.to_string();
                    statuses.insert(node_id.clone(), NodeStatus::Failed);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Failed,
                        duration,
                        error: Some(msg),
                        output_count: 0,
                        is_stub: cell_is_stub,
                        blocked_by: None,
                        timing,
                    });
                }
            }
        }

        let total_duration = start.elapsed();
        let success = graph_execution_succeeded(&results);

        Ok(GraphOutput {
            graph_name,
            success,
            node_results: results,
            total_duration,
            gate_verdicts: collect_gate_verdicts(&outputs),
        })
    }

    /// Validate the graph without executing: check for cycles, unknown cell types,
    /// and unresolved edge references.
    ///
    /// # Errors
    /// Returns a list of validation issues.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for cycles
        if topological_order(&self.graph).is_err() {
            issues.push("graph contains a cycle".to_string());
        }

        // Check all node cell types are registered
        for (node_id, idx) in &self.graph.node_map {
            let node = &self.graph.inner[*idx];
            if !self.registry.contains(&node.cell_type) {
                issues.push(format!(
                    "node '{}' references unknown cell type '{}'",
                    node_id, node.cell_type
                ));
            }
        }

        issues
    }

    /// Start the graph execution on a background tokio task, returning a
    /// [`FlowHandle`] immediately.
    ///
    /// Validates all edges for type-schema compatibility before spawning the
    /// background task. If validation fails, the `FlowHandle` is returned with
    /// the failure immediately available via `await_completion`.
    ///
    /// This is the async alternative to [`GraphEngine::execute`]. The caller
    /// receives a handle while execution continues in the background. Use
    /// [`FlowHandle::await_completion`] to wait for the final result, or
    /// [`FlowHandle::cancel`] to request early termination.
    ///
    /// A unique `run_id` is generated automatically using a random UUID-like
    /// string derived from the current timestamp and a counter.
    pub fn start(self, ctx: CellContext) -> FlowHandle {
        // Validate before spawning work. If validation fails, we still return
        // a FlowHandle but the result will be None (the task logs the error).
        if let Err(e) = self.validate_for_start() {
            warn!(error = %e, "graph edge validation failed before start");
            let graph_id = self.graph.metadata.name.clone();
            let cancel = CancellationToken::new();
            return FlowHandle {
                run_id: format!("flow-validation-failed-{graph_id}"),
                graph_id,
                started_at: Instant::now(),
                node_statuses: Arc::new(parking_lot::Mutex::new(HashMap::new())),
                budget_consumed: Arc::new(AtomicU64::new(0)),
                cancel,
                result: Arc::new(parking_lot::Mutex::new(None)),
                join_handle: parking_lot::Mutex::new(None),
            };
        }
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let run_id = format!(
            "flow-{}-{:04}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            seq
        );
        let graph_id = self.graph.metadata.name.clone();
        let started_at = Instant::now();

        let cancel = CancellationToken::new();
        // Running cells see the cancellation (bug-ceb581).
        let ctx = ctx.with_run_cancel(cancel.clone());
        let node_statuses: Arc<parking_lot::Mutex<HashMap<NodeId, NodeStatus>>> =
            Arc::new(parking_lot::Mutex::new(HashMap::new()));
        let budget_consumed = Arc::new(AtomicU64::new(0));
        let result: Arc<parking_lot::Mutex<Option<GraphOutput>>> =
            Arc::new(parking_lot::Mutex::new(None));

        // Clone handles for the background task.
        let cancel_clone = cancel.clone();
        let node_statuses_clone = node_statuses.clone();
        let result_clone = result.clone();
        let run_id_clone = run_id.clone();
        let graph_id_clone = graph_id.clone();

        let join_handle = tokio::spawn(async move {
            info!(run_id = %run_id_clone, graph = %graph_id_clone, "flow started");

            // Execute the graph, propagating per-node status updates.
            let graph_output = self
                .execute_with_status_tracking(&ctx, &node_statuses_clone, &cancel_clone)
                .await;

            match &graph_output {
                Ok(output) => {
                    info!(
                        run_id = %run_id_clone,
                        graph = %graph_id_clone,
                        success = output.success,
                        nodes = output.node_results.len(),
                        "flow completed"
                    );
                }
                Err(e) => {
                    warn!(
                        run_id = %run_id_clone,
                        graph = %graph_id_clone,
                        error = %e,
                        "flow failed"
                    );
                }
            }

            if let Ok(output) = graph_output {
                *result_clone.lock() = Some(output);
            }
        });

        FlowHandle {
            run_id,
            graph_id,
            started_at,
            node_statuses,
            budget_consumed,
            cancel,
            result,
            join_handle: parking_lot::Mutex::new(Some(join_handle)),
        }
    }

    /// Internal: execute the graph while publishing per-node status into `node_statuses`.
    ///
    /// Respects the cancellation token: once it is cancelled no further node
    /// starts, and nodes already running finish.
    ///
    /// When `policy.max_concurrent_nodes > 1` nodes run with bounded
    /// parallelism, each starting as soon as its predecessors have settled
    /// (the scheduler `execute_parallel_at_tick_validated` uses). When
    /// `max_concurrent_nodes == 1` nodes run sequentially in topological order.
    #[allow(clippy::too_many_lines)] // Keep status transitions adjacent to graph execution.
    async fn execute_with_status_tracking(
        &self,
        ctx: &CellContext,
        node_statuses: &Arc<parking_lot::Mutex<HashMap<NodeId, NodeStatus>>>,
        cancel: &CancellationToken,
    ) -> Result<GraphOutput, GraphError> {
        if self.graph.policy.max_concurrent_nodes > 1 {
            self.execute_with_status_tracking_parallel(ctx, node_statuses, cancel)
                .await
        } else {
            self.execute_with_status_tracking_sequential(ctx, node_statuses, cancel)
                .await
        }
    }

    /// Sequential variant of status-tracking execution (max_concurrent_nodes == 1).
    #[allow(clippy::too_many_lines)]
    async fn execute_with_status_tracking_sequential(
        &self,
        ctx: &CellContext,
        node_statuses: &Arc<parking_lot::Mutex<HashMap<NodeId, NodeStatus>>>,
        cancel: &CancellationToken,
    ) -> Result<GraphOutput, GraphError> {
        let start = Instant::now();
        let graph_name = self.graph.metadata.name.clone();
        let run_id = ctx.run_id.clone().unwrap_or_else(|| graph_name.clone());
        let graph_ancestry = [LensScope::Graph(graph_name.clone())];

        let order = topological_order(&self.graph)?;

        self.emit_telemetry(
            &ObservableEvent::GraphStarted {
                graph: graph_name.clone(),
                run: run_id.clone(),
                input_hash: input_signal_hash(&self.root_inputs),
            },
            &graph_ancestry,
        )
        .await;

        let mut outputs = self.initial_tick_outputs();
        let mut results: Vec<NodeResult> = Vec::with_capacity(order.len());
        // How many results the event sink has been told about.
        let mut published = 0;
        let mut total_cost_usd = 0.0;
        let mut was_cancelled = false;
        // Once set, no further node starts: why the rest are skipped.
        let mut abort: Option<String> = None;

        // Seed all nodes as Pending.
        {
            let mut statuses = node_statuses.lock();
            for node_id in &order {
                statuses.insert(node_id.clone(), NodeStatus::Pending);
            }
        }
        let mut clock = SettleClock::start();
        let mut previous: Option<&NodeId> = None;

        for node_id in &order {
            // The previous node settled when the loop moved on from it.
            if let Some(previous) = previous.replace(node_id) {
                clock.settle(previous);
            }
            published = self
                .publish_settled(&run_id, &results, published, &outputs)
                .await;
            // Honour cancellation between nodes.
            if cancel.is_cancelled() {
                info!(node_id = %node_id, "flow cancelled before node");
                self.emit_telemetry(
                    &ObservableEvent::CellCancelled {
                        block: node_id.clone(),
                        run: run_id.clone(),
                    },
                    &[
                        LensScope::Cell(node_id.clone()),
                        LensScope::Graph(graph_name.clone()),
                    ],
                )
                .await;
                was_cancelled = true;
                break;
            }

            let Some(node) = self.graph.get_node(node_id) else {
                continue;
            };

            if let Some(reason) = &abort {
                node_statuses
                    .lock()
                    .insert(node_id.clone(), NodeStatus::Skipped);
                results.push(skipped_result(node, NodeStatus::Skipped, reason.clone()));
                continue;
            }

            let input = {
                let statuses = node_statuses.lock();
                match evaluate_node_activation(&self.graph, node_id, &statuses, &outputs) {
                    NodeActivation::Root => self.root_tick_inputs(node_id, &outputs),
                    NodeActivation::Ready(input) => input,
                    NodeActivation::ConditionSkipped(reason) => {
                        drop(statuses);
                        node_statuses
                            .lock()
                            .insert(node_id.clone(), NodeStatus::ConditionSkipped);
                        results.push(NodeResult {
                            node_id: node_id.clone(),
                            cell_type: node.cell_type.clone(),
                            status: NodeStatus::ConditionSkipped,
                            duration: Duration::ZERO,
                            error: Some(reason),
                            output_count: 0,
                            is_stub: false,
                            blocked_by: None,
                            timing: NodeTiming::default(),
                        });
                        continue;
                    }
                    NodeActivation::UpstreamFailed { reason, dependency } => {
                        let blocked_by = dependency
                            .as_deref()
                            .and_then(|dependency| failed_root(dependency, &statuses, &results));
                        drop(statuses);
                        node_statuses
                            .lock()
                            .insert(node_id.clone(), NodeStatus::Skipped);
                        results.push(NodeResult {
                            blocked_by,
                            ..skipped_result(node, NodeStatus::Skipped, reason)
                        });
                        continue;
                    }
                }
            };

            node_statuses
                .lock()
                .insert(node_id.clone(), NodeStatus::Running);

            // For Activity nodes: check the replayer for a pre-recorded result.
            // `start()` always drives tick 0; skip cell instantiation when a
            // recording is available so the provider is not called again.
            let is_activity = node.execution_class == ExecutionClass::Activity;
            if is_activity
                && let Some(replayer) = &self.replayer
                && let Some(recorded) = replayer.lookup(node_id, 0)
            {
                let mut recorded = recorded.clone();
                propagate_input_taint(&input, &mut recorded, node_id);
                let count = recorded.len();
                info!(
                    node_id = %node_id,
                    outputs = count,
                    "flow-sequential: substituting recorded Activity output"
                );
                outputs.insert(node_id.clone(), recorded);
                node_statuses
                    .lock()
                    .insert(node_id.clone(), NodeStatus::Complete);
                results.push(NodeResult {
                    node_id: node_id.clone(),
                    cell_type: node.cell_type.clone(),
                    status: NodeStatus::Complete,
                    duration: Duration::ZERO,
                    error: None,
                    output_count: count,
                    is_stub: false,
                    blocked_by: None,
                    timing: NodeTiming::default(),
                });
                continue;
            }

            if let Some(reason) = self.dispatch_stopped() {
                info!(node_id = %node_id, %reason, "flow: dispatch stopped, starting no further nodes");
                node_statuses
                    .lock()
                    .insert(node_id.clone(), NodeStatus::Skipped);
                results.push(skipped_result(node, NodeStatus::Skipped, reason.clone()));
                abort = Some(reason);
                continue;
            }

            let cell: Box<dyn Cell> = self.registry.create(&node.cell_type, node.config.clone())?;
            let cell_is_stub = cell.is_stub();
            let estimated_cost_usd = cell.estimated_cost().unwrap_or_default();
            let ancestry = [
                LensScope::Cell(node_id.clone()),
                LensScope::Graph(graph_name.clone()),
            ];
            self.emit_telemetry(
                &ObservableEvent::CellStarted {
                    block: node_id.clone(),
                    run: run_id.clone(),
                    input_hash: input_signal_hash(&input),
                },
                &ancestry,
            )
            .await;
            self.publish_node_started(&run_id, node).await;

            info!(node_id = %node_id, cell_type = %node.cell_type, "flow: executing node");
            let timing = NodeTiming::dispatched_now(clock.ready_at_ms(&self.graph, node_id));
            let node_start = Instant::now();

            let (execution, attempts) = execute_cell_with_retries(
                cell.as_ref(),
                input,
                ctx,
                max_retries(&self.graph.policy),
                self.telemetry.as_ref(),
                node_id,
                &run_id,
                &ancestry,
            )
            .await;
            match execution {
                Ok(output_signals) => {
                    let duration = node_start.elapsed();
                    let duration_ms = duration_ms(duration);
                    let count = output_signals.len();
                    total_cost_usd += estimated_cost_usd * f64::from(attempts);

                    // For Activity nodes: persist the output so a future
                    // --resume-plan can substitute it instead of re-calling
                    // the provider.
                    if is_activity {
                        self.record_activity(node_id, 0, &output_signals, timing)?;
                    }

                    self.emit_telemetry(
                        &ObservableEvent::CellCompleted {
                            block: node_id.clone(),
                            run: run_id.clone(),
                            duration_ms,
                            cost_usd: estimated_cost_usd * f64::from(attempts),
                        },
                        &ancestry,
                    )
                    .await;
                    self.emit_telemetry(
                        &ObservableEvent::GraphNodeCompleted {
                            graph: graph_name.clone(),
                            run: run_id.clone(),
                            node: node_id.clone(),
                            duration_ms,
                        },
                        &ancestry,
                    )
                    .await;
                    node_statuses
                        .lock()
                        .insert(node_id.clone(), NodeStatus::Complete);
                    outputs.insert(node_id.clone(), output_signals);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Complete,
                        duration,
                        error: None,
                        output_count: count,
                        is_stub: cell_is_stub,
                        blocked_by: None,
                        timing,
                    });
                }
                Err(e) => {
                    let duration = node_start.elapsed();
                    let msg = e.to_string();
                    total_cost_usd += estimated_cost_usd * f64::from(attempts);
                    self.emit_telemetry(
                        &ObservableEvent::CellFailed {
                            block: node_id.clone(),
                            run: run_id.clone(),
                            error: msg.clone(),
                        },
                        &ancestry,
                    )
                    .await;
                    warn!(node_id = %node_id, error = %msg, "flow: node failed");
                    node_statuses
                        .lock()
                        .insert(node_id.clone(), NodeStatus::Failed);
                    if matches!(
                        self.graph.policy.failure_strategy,
                        crate::types::FailureStrategy::FailFast
                    ) {
                        abort.get_or_insert_with(|| "aborted after graph failure".to_string());
                    }
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Failed,
                        duration,
                        error: Some(msg),
                        output_count: 0,
                        is_stub: cell_is_stub,
                        blocked_by: None,
                        timing,
                    });
                }
            }
        }
        self.publish_settled(&run_id, &results, published, &outputs)
            .await;

        let total_duration = start.elapsed();
        let success = !was_cancelled && graph_execution_succeeded(&results);

        if was_cancelled {
            self.emit_telemetry(
                &ObservableEvent::GraphPaused {
                    graph: graph_name.clone(),
                    run: run_id,
                    reason: "cancelled".to_string(),
                },
                &graph_ancestry,
            )
            .await;
        } else if success {
            self.emit_telemetry(
                &ObservableEvent::GraphCompleted {
                    graph: graph_name.clone(),
                    run: run_id,
                    duration_ms: duration_ms(total_duration),
                    cost_usd: total_cost_usd,
                },
                &graph_ancestry,
            )
            .await;
        } else {
            self.emit_telemetry(
                &ObservableEvent::GraphFailed {
                    graph: graph_name.clone(),
                    run: run_id,
                    error: "one or more graph nodes failed".to_string(),
                },
                &graph_ancestry,
            )
            .await;
        }

        Ok(GraphOutput {
            graph_name,
            success,
            node_results: results,
            total_duration,
            gate_verdicts: collect_gate_verdicts(&outputs),
        })
    }

    /// Parallel variant of status-tracking execution (max_concurrent_nodes > 1).
    ///
    /// Runs the ready-queue scheduler shared with
    /// `execute_parallel_at_tick_validated`: each node starts as soon as its
    /// predecessors have settled, within `max_concurrent_nodes`. The shared
    /// `node_statuses` map is updated as nodes start, complete, or fail, so
    /// `FlowHandle::status()` reflects live parallel progress. Once `cancel`
    /// fires no further node starts; nodes already running finish.
    async fn execute_with_status_tracking_parallel(
        &self,
        ctx: &CellContext,
        node_statuses: &Arc<parking_lot::Mutex<HashMap<NodeId, NodeStatus>>>,
        cancel: &CancellationToken,
    ) -> Result<GraphOutput, GraphError> {
        let start = Instant::now();
        let graph_name = self.graph.metadata.name.clone();
        let run_id = ctx.run_id.clone().unwrap_or_else(|| graph_name.clone());
        let graph_ancestry = [LensScope::Graph(graph_name.clone())];

        self.emit_telemetry(
            &ObservableEvent::GraphStarted {
                graph: graph_name.clone(),
                run: run_id.clone(),
                input_hash: input_signal_hash(&self.root_inputs),
            },
            &graph_ancestry,
        )
        .await;

        // `start()` always drives tick 0.
        let ReadyQueueRun {
            outputs,
            results,
            total_cost_usd,
            was_cancelled,
        } = self
            .execute_ready_queue(
                ctx,
                ReadyQueueOptions {
                    tick: 0,
                    statuses: node_statuses,
                    cancel: Some(cancel),
                    announce_resume: false,
                },
            )
            .await?;

        let total_duration = start.elapsed();
        let success = !was_cancelled && graph_execution_succeeded(&results);

        if was_cancelled {
            self.emit_telemetry(
                &ObservableEvent::GraphPaused {
                    graph: graph_name.clone(),
                    run: run_id,
                    reason: "cancelled".to_string(),
                },
                &graph_ancestry,
            )
            .await;
        } else if success {
            self.emit_telemetry(
                &ObservableEvent::GraphCompleted {
                    graph: graph_name.clone(),
                    run: run_id,
                    duration_ms: duration_ms(total_duration),
                    cost_usd: total_cost_usd,
                },
                &graph_ancestry,
            )
            .await;
        } else {
            self.emit_telemetry(
                &ObservableEvent::GraphFailed {
                    graph: graph_name.clone(),
                    run: run_id,
                    error: "one or more graph nodes failed".to_string(),
                },
                &graph_ancestry,
            )
            .await;
        }

        Ok(GraphOutput {
            graph_name,
            success,
            node_results: results,
            total_duration,
            gate_verdicts: collect_gate_verdicts(&outputs),
        })
    }

    fn persist_tick_state_enabled(&self) -> bool {
        self.graph
            .policy
            .hot
            .as_ref()
            .is_some_and(|policy| policy.persist_tick_state)
    }

    fn initial_tick_outputs(&self) -> HashMap<NodeId, Vec<roko_core::Signal>> {
        if self.persist_tick_state_enabled() {
            self.tick_state.lock().clone()
        } else {
            HashMap::new()
        }
    }

    fn root_tick_inputs(
        &self,
        node_id: &str,
        outputs: &HashMap<NodeId, Vec<roko_core::Signal>>,
    ) -> Vec<roko_core::Signal> {
        let mut input = self.root_inputs.clone();
        if self.persist_tick_state_enabled()
            && let Some(previous) = outputs.get(node_id)
        {
            input.extend(previous.iter().cloned());
        }
        input
    }

    fn persist_tick_outputs(&self, outputs: &HashMap<NodeId, Vec<roko_core::Signal>>) {
        if self.persist_tick_state_enabled() {
            *self.tick_state.lock() = outputs.clone();
        }
    }

    async fn emit_telemetry(&self, event: &ObservableEvent, ancestry: &[LensScope]) {
        let Some(telemetry) = &self.telemetry else {
            return;
        };
        if let Err(error) = telemetry.emit(event, ancestry).await {
            warn!(%error, event_kind = ?event.kind(), "passive telemetry delivery failed");
        }
    }

    /// Publish one graph execution event to the event sink, when one is
    /// attached (reg-cbfff6). A failed delivery is logged: the sink observes
    /// the run and does not stop it.
    async fn publish_graph_event(&self, event: crate::events::GraphExecutionEvent) {
        let Some(sink) = &self.event_sink else {
            return;
        };
        if let Err(error) = sink.publish(&event).await {
            warn!(%error, event = event.variant_name(), "graph event delivery failed");
        }
    }

    /// Publish that `node`'s cell started running (reg-cbfff6).
    async fn publish_node_started(&self, run_id: &str, node: &Node) {
        if self.event_sink.is_none() {
            return;
        }
        let event = crate::events::GraphExecutionEvent::NodeStarted {
            common: crate::events::make_common(run_id, &self.graph.metadata.name, &self.event_seq),
            node: crate::events::make_node_fields(
                &node.id,
                &node.cell_type,
                node.execution_class,
                0,
            ),
        };
        self.publish_graph_event(event).await;
    }

    /// Publish how each node result from index `published` on settled, and
    /// return how many results are now published (reg-cbfff6). A completed
    /// task node carries the outcome its gate verdict earns.
    async fn publish_settled(
        &self,
        run_id: &str,
        results: &[NodeResult],
        published: usize,
        outputs: &HashMap<NodeId, Vec<roko_core::Signal>>,
    ) -> usize {
        if self.event_sink.is_some() {
            for result in results.iter().skip(published) {
                let event = self.settled_event(run_id, result, outputs);
                self.publish_graph_event(event).await;
            }
        }
        results.len()
    }

    /// The event saying how `result`'s node settled: completed, failed or
    /// skipped.
    fn settled_event(
        &self,
        run_id: &str,
        result: &NodeResult,
        outputs: &HashMap<NodeId, Vec<roko_core::Signal>>,
    ) -> crate::events::GraphExecutionEvent {
        use crate::events::GraphExecutionEvent;

        let execution_class = self
            .graph
            .get_node(&result.node_id)
            .map(|node| node.execution_class)
            .unwrap_or_default();
        let common = crate::events::make_common(run_id, &self.graph.metadata.name, &self.event_seq);
        let node =
            crate::events::make_node_fields(&result.node_id, &result.cell_type, execution_class, 0);
        let elapsed_ms = duration_ms(result.duration);
        let reason = result.error.clone().unwrap_or_default();
        match result.status {
            NodeStatus::Complete => GraphExecutionEvent::NodeCompleted {
                common,
                node,
                elapsed_ms,
                outcome: completed_outcome(
                    &result.cell_type,
                    outputs.get(&result.node_id).map_or(&[][..], Vec::as_slice),
                ),
            },
            NodeStatus::Failed => GraphExecutionEvent::NodeFailed {
                common,
                node,
                elapsed_ms,
                error: reason,
            },
            NodeStatus::Skipped
            | NodeStatus::ConditionSkipped
            | NodeStatus::Pending
            | NodeStatus::Running => GraphExecutionEvent::NodeSkipped {
                common,
                node,
                reason: match &result.blocked_by {
                    Some(blocker) => format!("blocked by failed node '{blocker}': {reason}"),
                    None => reason,
                },
            },
        }
    }

    /// Extract `files_changed` from completed node outputs.
    ///
    /// Convention: nodes that modify files include a `"files_changed"` tag in
    /// their output signals. The tag value is a comma-separated list of file
    /// paths. This method scans all node outputs and collects those paths.
    fn collect_files_changed(outputs: &HashMap<NodeId, Vec<roko_core::Signal>>) -> Vec<String> {
        let mut files = Vec::new();
        for signals in outputs.values() {
            for signal in signals {
                if let Some(value) = signal.tags.get("files_changed") {
                    for path in value.split(',') {
                        let trimmed = path.trim();
                        if !trimmed.is_empty() {
                            files.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
        files.sort();
        files.dedup();
        files
    }
}

// ─── Node timing ────────────────────────────────────────────────────────────

/// When the nodes of a sequential run settled, so that a node's ready time
/// is known when the loop reaches it: the latest of its predecessors' settle
/// times, or the run's start for a node with none.
struct SettleClock {
    started_at_ms: u64,
    settled_at_ms: HashMap<NodeId, u64>,
}

impl SettleClock {
    fn start() -> Self {
        Self {
            started_at_ms: crate::control::now_ms(),
            settled_at_ms: HashMap::new(),
        }
    }

    /// Record that `node_id` settled now.
    fn settle(&mut self, node_id: &str) {
        self.settled_at_ms
            .insert(node_id.to_string(), crate::control::now_ms());
    }

    /// When every node that `node_id` depends on had settled.
    fn ready_at_ms(&self, graph: &Graph, node_id: &str) -> u64 {
        crate::topo::dependencies(graph, node_id)
            .iter()
            .filter_map(|dependency| self.settled_at_ms.get(dependency).copied())
            .max()
            .unwrap_or(self.started_at_ms)
    }
}

// ─── Ready-queue scheduling ─────────────────────────────────────────────────

/// Settings that differ between the bounded-parallel entry points.
struct ReadyQueueOptions<'a> {
    /// Tick under which Activity outputs are replayed and recorded.
    tick: u64,
    /// Live per-node status map. `start()` shares it with its [`FlowHandle`].
    statuses: &'a parking_lot::Mutex<HashMap<NodeId, NodeStatus>>,
    /// Once cancelled, no further node starts (`start()` only).
    cancel: Option<&'a CancellationToken>,
    /// Emit `GraphResumed` before the first replayed Activity output.
    announce_resume: bool,
}

/// What a bounded-parallel run produced, before Graph-level telemetry.
struct ReadyQueueRun {
    /// Outputs of the completed nodes.
    outputs: HashMap<NodeId, Vec<roko_core::Signal>>,
    /// Per-node results in topological order. Cancelled nodes have none.
    results: Vec<NodeResult>,
    /// Estimated cost of every attempt of every started node.
    total_cost_usd: f64,
    /// Whether cancellation kept at least one node from starting.
    was_cancelled: bool,
}

/// Why the ready-queue scheduler starts no further node.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Halt {
    /// A node failed under the `FailFast` failure strategy.
    FailFast,
    /// The engine's dispatch stop gave this reason.
    Stopped(String),
    /// The run was cancelled.
    Cancelled,
}

/// Dependency bookkeeping for [`GraphEngine::execute_ready_queue`].
///
/// Nodes are addressed by topological position, so ready nodes are decided
/// and started in a deterministic order.
struct ReadyQueue<'g> {
    graph: &'g Graph,
    /// Node indices in topological order.
    order: Vec<GraphNodeIdx>,
    /// Topological position of each node, indexed by its petgraph index.
    position: Vec<usize>,
    /// Incoming edges of each node whose source has not settled yet.
    unsettled_inputs: Vec<usize>,
    /// Nodes whose predecessors have all settled and that are not decided yet.
    ready: BTreeSet<usize>,
    /// When each node became ready (Unix ms): when the run started for a
    /// root, else when its last predecessor settled. Zero until then.
    ready_at_ms: Vec<u64>,
}

impl<'g> ReadyQueue<'g> {
    fn new(graph: &'g Graph) -> Result<Self, GraphError> {
        let order = topological_order(graph)?
            .iter()
            .map(|node_id| {
                graph
                    .node_map
                    .get(node_id)
                    .copied()
                    .ok_or_else(|| GraphError::NodeNotFound(node_id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut position = vec![0; graph.inner.node_count()];
        for (pos, idx) in order.iter().enumerate() {
            position[idx.index()] = pos;
        }
        let unsettled_inputs: Vec<usize> = order
            .iter()
            .map(|&idx| {
                graph
                    .inner
                    .edges_directed(idx, petgraph::Direction::Incoming)
                    .count()
            })
            .collect();
        let ready: BTreeSet<usize> = (0..order.len())
            .filter(|&pos| unsettled_inputs[pos] == 0)
            .collect();
        let started_at_ms = crate::control::now_ms();
        let mut ready_at_ms = vec![0; order.len()];
        for &pos in &ready {
            ready_at_ms[pos] = started_at_ms;
        }
        Ok(Self {
            graph,
            order,
            position,
            unsettled_inputs,
            ready,
            ready_at_ms,
        })
    }

    fn len(&self) -> usize {
        self.order.len()
    }

    /// The node at topological position `pos`.
    fn node(&self, pos: usize) -> &'g Node {
        &self.graph.inner[self.order[pos]]
    }

    /// Record that the node at `pos` reached a terminal status. Each dependant
    /// whose last unsettled input this was becomes ready.
    fn settle(&mut self, pos: usize) {
        for edge in self
            .graph
            .inner
            .edges_directed(self.order[pos], petgraph::Direction::Outgoing)
        {
            let target = self.position[edge.target().index()];
            self.unsettled_inputs[target] -= 1;
            if self.unsettled_inputs[target] == 0 {
                self.ready.insert(target);
                self.ready_at_ms[target] = crate::control::now_ms();
            }
        }
    }

    /// When the node at `pos` became ready (Unix ms).
    fn ready_at_ms(&self, pos: usize) -> u64 {
        self.ready_at_ms[pos]
    }

    /// Topological position of `node_id`, used to order results.
    fn position_of(&self, node_id: &str) -> usize {
        self.graph
            .node_map
            .get(node_id)
            .map_or(usize::MAX, |idx| self.position[idx.index()])
    }

    /// The first node of `running` whose exclusive paths overlap those of the
    /// node at `pos`, with the overlapping paths: the node's own, then the
    /// running node's.
    fn exclusion_conflict(
        &self,
        pos: usize,
        running: &[usize],
    ) -> Option<(usize, &'g str, &'g str)> {
        let wanted = &self.node(pos).exclusive;
        if wanted.is_empty() {
            return None;
        }
        running.iter().find_map(|&other| {
            crate::exclusion::first_overlap(wanted, &self.node(other).exclusive)
                .map(|(path, held)| (other, path, held))
        })
    }
}

/// A node the scheduler starts on its own task, with everything that task needs.
struct NodeLaunch {
    node_id: NodeId,
    cell_type: String,
    cell: Box<dyn Cell>,
    input: Vec<roko_core::Signal>,
    ctx: CellContext,
    graph_name: String,
    run_id: String,
    telemetry: Option<Arc<dyn TelemetryEventSink>>,
    max_retries: u32,
}

/// How a started node ended.
struct NodeRun {
    result: NodeResult,
    /// Estimated cost of all of the node's attempts.
    cost_usd: f64,
    /// Output signals; empty unless the node completed.
    outputs: Vec<roko_core::Signal>,
}

/// Execute a started node's cell under the Graph's retry policy, emitting
/// its Cell lifecycle telemetry.
async fn run_node(launch: NodeLaunch) -> NodeRun {
    let NodeLaunch {
        node_id,
        cell_type,
        cell,
        input,
        ctx,
        graph_name,
        run_id,
        telemetry,
        max_retries,
    } = launch;
    let is_stub = cell.is_stub();
    let estimated_cost_usd = cell.estimated_cost().unwrap_or_default();
    let ancestry = [
        LensScope::Cell(node_id.clone()),
        LensScope::Graph(graph_name.clone()),
    ];
    emit_telemetry_to(
        telemetry.as_ref(),
        &ObservableEvent::CellStarted {
            block: node_id.clone(),
            run: run_id.clone(),
            input_hash: input_signal_hash(&input),
        },
        &ancestry,
    )
    .await;

    let node_start = Instant::now();
    let (execution, attempts) = execute_cell_with_retries(
        cell.as_ref(),
        input,
        &ctx,
        max_retries,
        telemetry.as_ref(),
        &node_id,
        &run_id,
        &ancestry,
    )
    .await;
    let duration = node_start.elapsed();
    let cost_usd = estimated_cost_usd * f64::from(attempts);
    let (status, error, outputs) = match execution {
        Ok(outputs) => {
            let duration_ms = duration_ms(duration);
            emit_telemetry_to(
                telemetry.as_ref(),
                &ObservableEvent::CellCompleted {
                    block: node_id.clone(),
                    run: run_id.clone(),
                    duration_ms,
                    cost_usd,
                },
                &ancestry,
            )
            .await;
            emit_telemetry_to(
                telemetry.as_ref(),
                &ObservableEvent::GraphNodeCompleted {
                    graph: graph_name,
                    run: run_id,
                    node: node_id.clone(),
                    duration_ms,
                },
                &ancestry,
            )
            .await;
            (NodeStatus::Complete, None, outputs)
        }
        Err(error) => {
            let error = error.to_string();
            emit_telemetry_to(
                telemetry.as_ref(),
                &ObservableEvent::CellFailed {
                    block: node_id.clone(),
                    run: run_id,
                    error: error.clone(),
                },
                &ancestry,
            )
            .await;
            (NodeStatus::Failed, Some(error), Vec::new())
        }
    };
    NodeRun {
        result: NodeResult {
            node_id,
            cell_type,
            status,
            duration,
            error,
            output_count: outputs.len(),
            is_stub,
            blocked_by: None,
            timing: NodeTiming::default(),
        },
        cost_usd,
        outputs,
    }
}

/// The failed node that blocks a node whose required `dependency` did not
/// complete: the dependency itself when it failed, or the failed node that
/// blocked it when it was skipped in turn.
fn failed_root(
    dependency: &str,
    statuses: &HashMap<NodeId, NodeStatus>,
    results: &[NodeResult],
) -> Option<NodeId> {
    match statuses.get(dependency) {
        Some(NodeStatus::Failed) => Some(dependency.to_string()),
        Some(NodeStatus::Skipped) => results
            .iter()
            .find(|result| result.node_id == dependency)
            .and_then(|result| result.blocked_by.clone()),
        _ => None,
    }
}

/// Result for a node that settled without running its cell.
fn skipped_result(node: &Node, status: NodeStatus, reason: String) -> NodeResult {
    NodeResult {
        node_id: node.id.clone(),
        cell_type: node.cell_type.clone(),
        status,
        duration: Duration::ZERO,
        error: Some(reason),
        output_count: 0,
        is_stub: false,
        blocked_by: None,
        timing: NodeTiming::default(),
    }
}

/// Evaluate the incoming edge set for one node.
///
/// Unconditional and `Always` edges are required dependencies (AND). The
/// remaining conditional edges are routes (OR): at least one route must fire,
/// and only outputs carried by fired edges are passed to the target node.
fn evaluate_node_activation(
    graph: &Graph,
    node_id: &str,
    statuses: &HashMap<NodeId, NodeStatus>,
    outputs: &HashMap<NodeId, Vec<roko_core::Signal>>,
) -> NodeActivation {
    use petgraph::Direction;

    let Some(&idx) = graph.node_map.get(node_id) else {
        return NodeActivation::UpstreamFailed {
            reason: format!("node `{node_id}` is not in the graph"),
            dependency: None,
        };
    };

    let mut has_incoming = false;
    let mut has_conditional = false;
    let mut conditional_fired = false;
    let mut input = Vec::new();

    for edge_ref in graph.inner.edges_directed(idx, Direction::Incoming) {
        has_incoming = true;
        let edge = edge_ref.weight();
        let source_id = &graph.inner[edge_ref.source()].id;
        let status = statuses
            .get(source_id)
            .copied()
            .unwrap_or(NodeStatus::Pending);
        let source_outputs = outputs.get(source_id).map_or(&[][..], Vec::as_slice);

        match edge.condition.as_ref() {
            None | Some(EdgeCondition::Always) => match status {
                NodeStatus::Complete => input.extend(source_outputs.iter().cloned()),
                NodeStatus::ConditionSkipped => {
                    return NodeActivation::ConditionSkipped(format!(
                        "required route from `{source_id}` was not selected"
                    ));
                }
                NodeStatus::Failed | NodeStatus::Skipped => {
                    return NodeActivation::UpstreamFailed {
                        reason: format!("required dependency `{source_id}` did not complete"),
                        dependency: Some(source_id.clone()),
                    };
                }
                NodeStatus::Pending | NodeStatus::Running => {
                    return NodeActivation::UpstreamFailed {
                        reason: format!("required dependency `{source_id}` is not complete"),
                        dependency: Some(source_id.clone()),
                    };
                }
            },
            Some(condition) => {
                has_conditional = true;
                let fires = match condition {
                    EdgeCondition::Success => status == NodeStatus::Complete,
                    EdgeCondition::Failure => status == NodeStatus::Failed,
                    EdgeCondition::OutputEquals { key, value } => {
                        status == NodeStatus::Complete
                            && source_outputs
                                .iter()
                                .any(|signal| signal_output_equals(signal, key, value))
                    }
                    EdgeCondition::Always => unreachable!("handled as a required edge"),
                };
                if fires {
                    conditional_fired = true;
                    input.extend(source_outputs.iter().cloned());
                }
            }
        }
    }

    if !has_incoming {
        NodeActivation::Root
    } else if has_conditional && !conditional_fired {
        NodeActivation::ConditionSkipped("no incoming conditional edge fired".to_string())
    } else {
        NodeActivation::Ready(input)
    }
}

fn signal_output_equals(signal: &roko_core::Signal, key: &str, expected: &str) -> bool {
    if let Some(tag_key) = key.strip_prefix("tags.") {
        return signal
            .tags
            .get(tag_key)
            .is_some_and(|value| value == expected);
    }

    if let roko_core::Body::Json(value) = &signal.body
        && let Some(actual) = json_path(value, key)
        && json_value_equals(actual, expected)
    {
        return true;
    }

    if matches!(key, "body" | "text" | "value")
        && let roko_core::Body::Text(actual) = &signal.body
    {
        return actual == expected;
    }

    signal.tags.get(key).is_some_and(|value| value == expected)
}

fn json_path<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    if key.is_empty() || key == "body" {
        return Some(value);
    }
    key.split('.')
        .try_fold(value, |current, segment| match current {
            serde_json::Value::Object(map) => map.get(segment),
            serde_json::Value::Array(values) => segment
                .parse::<usize>()
                .ok()
                .and_then(|index| values.get(index)),
            _ => None,
        })
}

fn json_value_equals(actual: &serde_json::Value, expected: &str) -> bool {
    match actual {
        serde_json::Value::String(value) => value == expected,
        serde_json::Value::Number(value) => value.to_string() == expected,
        serde_json::Value::Bool(value) => value.to_string() == expected,
        serde_json::Value::Null => expected == "null",
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::to_string(actual).is_ok_and(|value| value == expected)
        }
    }
}

fn graph_execution_succeeded(results: &[NodeResult]) -> bool {
    results.iter().all(|result| {
        matches!(
            result.status,
            NodeStatus::Complete | NodeStatus::ConditionSkipped
        )
    })
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn input_signal_hash(input: &[roko_core::Signal]) -> String {
    let bytes = input
        .iter()
        .flat_map(|signal| signal.id.0)
        .collect::<Vec<_>>();
    ContentHash::of(&bytes).to_hex()
}

fn max_retries(policy: &GraphPolicy) -> u32 {
    match policy.failure_strategy {
        crate::types::FailureStrategy::Retry { max_retries } => max_retries,
        crate::types::FailureStrategy::FailFast | crate::types::FailureStrategy::SkipFailed => 0,
    }
}

#[allow(clippy::too_many_arguments)]
async fn execute_cell_with_retries(
    cell: &dyn Cell,
    input: Vec<roko_core::Signal>,
    ctx: &CellContext,
    max_retries: u32,
    telemetry: Option<&Arc<dyn TelemetryEventSink>>,
    block: &str,
    run: &str,
    ancestry: &[LensScope],
) -> (roko_core::Result<Vec<roko_core::Signal>>, u32) {
    let prediction = cell.predict(&input);
    if let Some(prediction) = prediction.as_ref() {
        let serialized = serde_json::to_string(prediction)
            .unwrap_or_else(|error| format!("prediction serialization failed: {error}"));
        emit_telemetry_to(
            telemetry,
            &ObservableEvent::CellPredictionPublished {
                block: block.to_string(),
                prediction: serialized,
            },
            ancestry,
        )
        .await;
    }
    let mut retry_attempt = 0_u32;
    loop {
        match cell.execute(input.clone(), ctx).await {
            Ok(mut output) => {
                propagate_input_taint(&input, &mut output, block);
                if let Some(prediction) = prediction.as_ref() {
                    let calibration_error = cell.calibration_error(prediction, &output);
                    cell.correct(prediction, &output);
                    if let Some(error) = calibration_error {
                        emit_telemetry_to(
                            telemetry,
                            &ObservableEvent::CellCalibrationReceived {
                                block: block.to_string(),
                                error: error.clamp(0.0, 1.0),
                            },
                            ancestry,
                        )
                        .await;
                    }
                }
                return (Ok(output), retry_attempt.saturating_add(1));
            }
            Err(error) if retry_attempt < max_retries => {
                retry_attempt = retry_attempt.saturating_add(1);
                emit_telemetry_to(
                    telemetry,
                    &ObservableEvent::CellRetried {
                        block: block.to_string(),
                        run: run.to_string(),
                        attempt: retry_attempt,
                        reason: error.to_string(),
                    },
                    ancestry,
                )
                .await;
            }
            Err(error) => return (Err(error), retry_attempt.saturating_add(1)),
        }
    }
}

/// Cell type of a plan task node.
const TASK_EXECUTOR_CELL_TYPE: &str = "task-executor";

/// The dashboard outcome a completed node settled with (reg-cbfff6): the one
/// its gate verdict earns, `unverified` for a task node that carries no
/// verdict, and none for any other node.
fn completed_outcome(cell_type: &str, outputs: &[roko_core::Signal]) -> Option<String> {
    use roko_core::dashboard_snapshot::{
        TASK_OUTCOME_ACCEPTED_WITH_FAILURES, TASK_OUTCOME_ALREADY_SATISFIED, TASK_OUTCOME_PASSED,
        TASK_OUTCOME_PASSED_WITH_PREEXISTING_FAILURES, TASK_OUTCOME_UNVERIFIED,
    };

    let outcome = match TaskGateVerdict::from_signals(outputs) {
        Some(TaskGateVerdict::Passed) => TASK_OUTCOME_PASSED,
        Some(TaskGateVerdict::PassedWithPreexistingFailures) => {
            TASK_OUTCOME_PASSED_WITH_PREEXISTING_FAILURES
        }
        Some(TaskGateVerdict::AlreadySatisfied) => TASK_OUTCOME_ALREADY_SATISFIED,
        Some(TaskGateVerdict::ForcedAccept) => TASK_OUTCOME_ACCEPTED_WITH_FAILURES,
        Some(TaskGateVerdict::Unverified) => TASK_OUTCOME_UNVERIFIED,
        None if cell_type == TASK_EXECUTOR_CELL_TYPE => TASK_OUTCOME_UNVERIFIED,
        None => return None,
    };
    Some(outcome.to_string())
}

/// Collect the gate verdicts stamped on each node's outputs.
fn collect_gate_verdicts(
    outputs: &HashMap<NodeId, Vec<roko_core::Signal>>,
) -> BTreeMap<NodeId, TaskGateVerdict> {
    outputs
        .iter()
        .filter_map(|(node_id, signals)| {
            TaskGateVerdict::from_signals(signals).map(|verdict| (node_id.clone(), verdict))
        })
        .collect()
}

/// Enforce the Graph IFC boundary after every Cell execution and replay.
///
/// Cells may preserve or raise their own output classification, but cannot
/// lower it below the join of their inputs. The engine owns this invariant so
/// it also applies to third-party Cells that do not use `Signal::derive`.
fn propagate_input_taint(
    input: &[roko_core::Signal],
    output: &mut [roko_core::Signal],
    block: &str,
) {
    let inherited = input
        .iter()
        .fold(roko_core::TaintLevel::Public, |level, signal| {
            level.join(signal.provenance.effective_taint())
        });

    for signal in output {
        let current = signal.provenance.effective_taint();
        if !inherited.can_flow_to(current) {
            signal.provenance.taint_level = signal.provenance.taint_level.join(inherited);
            info!(
                block,
                signal = %signal.id,
                input_taint = ?inherited,
                output_taint = ?signal.provenance.effective_taint(),
                "raised Cell output taint to preserve monotonic Graph flow"
            );
        }
    }
}

async fn emit_telemetry_to(
    telemetry: Option<&Arc<dyn TelemetryEventSink>>,
    event: &ObservableEvent,
    ancestry: &[LensScope],
) {
    let Some(telemetry) = telemetry else {
        return;
    };
    if let Err(error) = telemetry.emit(event, ancestry).await {
        warn!(%error, event_kind = ?event.kind(), "passive telemetry delivery failed");
    }
}

/// Build the default cell registry with standard gate and utility cells.
///
/// Registered cell types:
/// - `gate.compile` -- `CompileGate` (cargo check)
/// - `gate.test` -- `TestGate` (cargo test)
/// - `gate.clippy` -- `ClippyGate` (cargo clippy)
/// - `security.verify.*` -- independently hosted corrigibility Verify Cells
/// - `security.immune.*` -- ordered runtime immune-pipeline Cells
/// - `noop` -- `NoopCell` (passes input through unchanged, useful for testing)
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn default_registry() -> CellRegistry {
    let mut registry = CellRegistry::new();
    crate::cells::register_corrigibility_cells(&mut registry);
    crate::cells::register_immune_cells(&mut registry);

    registry.register("gate.compile", |_config| {
        Box::new(ShellCell::new(
            "gate.compile",
            "CompileGate",
            "cargo",
            &["check", "--workspace"],
        ))
    });

    registry.register("gate.test", |_config| {
        Box::new(ShellCell::new(
            "gate.test",
            "TestGate",
            "cargo",
            &["test", "--workspace"],
        ))
    });

    registry.register("gate.clippy", |_config| {
        Box::new(ShellCell::new(
            "gate.clippy",
            "ClippyGate",
            "cargo",
            &["clippy", "--workspace", "--no-deps", "--", "-D", "warnings"],
        ))
    });

    // All typed registrations below use CellDescriptor for side-effect-free
    // edge validation (backlog #271).
    use crate::registry::CellDescriptor;
    use roko_core::{Kind, TypeSchema};

    registry.register_with_descriptor("noop", CellDescriptor::test_stub("noop"), |_config| {
        Box::new(NoopCell::default())
    });

    // Cognitive loop cells (E22-T01): real typed Cell implementations
    // with explicit CellDescriptors for side-effect-free edge validation.
    // Wave 12 (#268): all cognitive descriptors now carry protocol and predictive metadata.
    use roko_core::ProtocolId;

    registry.register_with_descriptor(
        "sense",
        CellDescriptor::new(
            "sense",
            (0, 2, 0),
            None,
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
        )
        .with_protocols(vec![ProtocolId::Observe])
        .with_display_name("SenseCell"),
        |_config| Box::new(crate::cells::cognitive::SenseCell::new()),
    );
    registry.register_with_descriptor(
        "assess",
        CellDescriptor::new(
            "assess",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
        )
        .with_protocols(vec![ProtocolId::Score])
        .with_predictive(true)
        .with_display_name("AssessCell"),
        |_config| Box::new(crate::cells::cognitive::AssessCell::new()),
    );
    // "score" is an alias for "assess" in legacy graph definitions.
    registry.register_with_descriptor(
        "score",
        CellDescriptor::new(
            "score",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
        )
        .with_protocols(vec![ProtocolId::Score])
        .with_predictive(true)
        .with_display_name("AssessCell (score alias)"),
        |_config| Box::new(crate::cells::cognitive::AssessCell::new()),
    );
    registry.register_with_descriptor(
        "compose",
        CellDescriptor::new(
            "compose",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
            Some(TypeSchema::OfKind(Kind::Prompt)),
        )
        .with_protocols(vec![ProtocolId::Compose])
        .with_display_name("CognitiveComposeCell"),
        |_config| Box::new(crate::cells::cognitive::CognitiveComposeCell::new()),
    );
    registry.register_with_descriptor(
        "act",
        CellDescriptor::new(
            "act",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::Prompt)),
            Some(TypeSchema::OfKind(Kind::Episode)),
        )
        .with_protocols(vec![ProtocolId::Connect])
        .with_display_name("ActCell"),
        |_config| Box::new(crate::cells::cognitive::ActCell::new()),
    );
    registry.register_with_descriptor(
        "verify",
        CellDescriptor::new(
            "verify",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::Episode)),
            Some(TypeSchema::OfKind(Kind::GateVerdict)),
        )
        .with_protocols(vec![ProtocolId::Verify])
        .with_display_name("VerifyCell"),
        |_config| Box::new(crate::cells::cognitive::VerifyCell::new()),
    );
    registry.register_with_descriptor(
        "persist",
        CellDescriptor::new(
            "persist",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::GateVerdict)),
            None,
        )
        .with_protocols(vec![ProtocolId::Store])
        .with_display_name("PersistCell"),
        |_config| Box::new(crate::cells::cognitive::PersistCell::new()),
    );
    registry.register_with_descriptor(
        "react",
        CellDescriptor::new("react", (0, 2, 0), None, None)
            .with_protocols(vec![ProtocolId::React, ProtocolId::Trigger])
            .with_display_name("ReactCell"),
        |_config| Box::new(crate::cells::cognitive::ReactCell::new()),
    );

    // Task executor cell for plan-to-graph converted tasks (task 101).
    registry.register("task-executor", |config| {
        Box::new(crate::cells::task_executor::TaskExecutorCell::unconfigured(
            config,
        ))
    });

    // Legacy cognitive loop aliases -- delegate to the real cognitive Cell
    // implementations so graph definitions that reference old names
    // (signal-reader, etc.) get full typed execution instead of stubs.
    registry.register_with_descriptor(
        "signal-reader",
        CellDescriptor::new(
            "signal-reader",
            (0, 2, 0),
            None,
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
        )
        .with_protocols(vec![ProtocolId::Observe])
        .with_display_name("SenseCell (signal-reader alias)"),
        |_config| Box::new(crate::cells::cognitive::SenseCell::new()),
    );
    registry.register_with_descriptor(
        "relevance-scorer",
        CellDescriptor::new(
            "relevance-scorer",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
        )
        .with_protocols(vec![ProtocolId::Score])
        .with_predictive(true)
        .with_display_name("AssessCell (relevance-scorer alias)"),
        |_config| Box::new(crate::cells::cognitive::AssessCell::new()),
    );
    registry.register_with_descriptor(
        "system-prompt-builder",
        CellDescriptor::new(
            "system-prompt-builder",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
            Some(TypeSchema::OfKind(Kind::Prompt)),
        )
        .with_protocols(vec![ProtocolId::Compose])
        .with_display_name("CognitiveComposeCell (system-prompt-builder alias)"),
        |_config| Box::new(crate::cells::cognitive::CognitiveComposeCell::new()),
    );
    registry.register_with_descriptor(
        "claude-agent",
        CellDescriptor::new(
            "claude-agent",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::Prompt)),
            Some(TypeSchema::OfKind(Kind::Episode)),
        )
        .with_protocols(vec![ProtocolId::Connect])
        .with_display_name("ActCell (claude-agent alias)"),
        |_config| Box::new(crate::cells::cognitive::ActCell::new()),
    );
    registry.register_with_descriptor(
        "gate-pipeline",
        CellDescriptor::new(
            "gate-pipeline",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::Episode)),
            Some(TypeSchema::OfKind(Kind::GateVerdict)),
        )
        .with_protocols(vec![ProtocolId::Verify])
        .with_display_name("VerifyCell (gate-pipeline alias)"),
        |_config| Box::new(crate::cells::cognitive::VerifyCell::new()),
    );
    registry.register_with_descriptor(
        "store-writer",
        CellDescriptor::new(
            "store-writer",
            (0, 2, 0),
            Some(TypeSchema::OfKind(Kind::GateVerdict)),
            None,
        )
        .with_protocols(vec![ProtocolId::Store])
        .with_display_name("PersistCell (store-writer alias)"),
        |_config| Box::new(crate::cells::cognitive::PersistCell::new()),
    );
    registry.register_with_descriptor(
        "event-publisher",
        CellDescriptor::new("event-publisher", (0, 2, 0), None, None)
            .with_protocols(vec![ProtocolId::React, ProtocolId::Trigger])
            .with_display_name("ReactCell (event-publisher alias)"),
        |_config| Box::new(crate::cells::cognitive::ReactCell::new()),
    );

    registry
}

// ─── Built-in cell implementations ──────────────────────────────────────────

/// A no-op cell that passes its input through unchanged. Useful for testing
/// and as a placeholder in graph definitions.
struct NoopCell {
    id: &'static str,
    name: &'static str,
}

impl NoopCell {
    #[cfg(test)]
    const fn with_id_and_name(id: &'static str, name: &'static str) -> Self {
        Self { id, name }
    }
}

impl Default for NoopCell {
    fn default() -> Self {
        Self {
            id: "noop",
            name: "NoopCell",
        }
    }
}

#[async_trait::async_trait]
impl Cell for NoopCell {
    fn cell_id(&self) -> &str {
        self.id
    }
    fn cell_name(&self) -> &str {
        self.name
    }
    fn cell_version(&self) -> crate::cell::CellVersion {
        (0, 1, 0)
    }
    fn is_stub(&self) -> bool {
        true
    }
    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        Vec::new()
    }
    fn estimated_cost(&self) -> Option<f64> {
        None
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(1))
    }
    async fn execute(
        &self,
        input: Vec<roko_core::Signal>,
        _ctx: &CellContext,
    ) -> roko_core::error::Result<Vec<roko_core::Signal>> {
        Ok(input)
    }
}

/// A cell that runs a shell command. Used for gate implementations (compile, test, clippy).
/// Succeeds if the command exits with status 0, fails otherwise.
struct ShellCell {
    id: &'static str,
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

impl ShellCell {
    const fn new(
        id: &'static str,
        name: &'static str,
        program: &'static str,
        args: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            name,
            program,
            args,
        }
    }
}

#[async_trait::async_trait]
impl Cell for ShellCell {
    fn cell_id(&self) -> &str {
        self.id
    }
    fn cell_name(&self) -> &str {
        self.name
    }
    fn cell_version(&self) -> crate::cell::CellVersion {
        (0, 1, 0)
    }
    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::Verify]
    }
    fn estimated_cost(&self) -> Option<f64> {
        None
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_mins(1))
    }
    async fn execute(
        &self,
        input: Vec<roko_core::Signal>,
        ctx: &CellContext,
    ) -> roko_core::error::Result<Vec<roko_core::Signal>> {
        if let Some(capabilities) = &ctx.capabilities {
            for required in [
                roko_core::Capability::Execute,
                roko_core::Capability::FileSystem,
            ] {
                if !capabilities.contains(required) {
                    return Err(roko_core::error::RokoError::invalid(format!(
                        "cell '{}' requires capability {required}",
                        self.id
                    )));
                }
            }
        }
        let output = tokio::process::Command::new(self.program)
            .args(self.args)
            .output()
            .await
            .map_err(|e| roko_core::error::RokoError::Verify {
                gate: self.name.to_string(),
                message: format!("failed to spawn '{}': {}", self.program, e),
            })?;

        if output.status.success() {
            Ok(input)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if stderr.is_empty() {
                stdout.to_string()
            } else {
                stderr.to_string()
            };
            // Truncate to avoid massive error messages
            let detail = if detail.len() > 2000 {
                format!("{}...(truncated)", &detail[..2000])
            } else {
                detail
            };
            Err(roko_core::error::RokoError::Verify {
                gate: self.name.to_string(),
                message: format!(
                    "{} exited with code {}: {}",
                    self.program,
                    output.status.code().unwrap_or(-1),
                    detail
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_from_str;

    struct CaptureCell {
        received: Arc<std::sync::Mutex<Vec<roko_core::Signal>>>,
    }

    #[async_trait::async_trait]
    impl Cell for CaptureCell {
        fn cell_id(&self) -> &str {
            "capture"
        }

        fn cell_name(&self) -> &str {
            "CaptureCell"
        }

        async fn execute(
            &self,
            input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::error::Result<Vec<roko_core::Signal>> {
            *self
                .received
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = input.clone();
            Ok(input)
        }
    }

    struct TaintLoweringCell;

    #[async_trait::async_trait]
    impl Cell for TaintLoweringCell {
        fn cell_id(&self) -> &str {
            "taint-lowering"
        }

        fn cell_name(&self) -> &str {
            "TaintLoweringCell"
        }

        async fn execute(
            &self,
            _input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            Ok(vec![
                roko_core::Signal::builder(roko_core::Kind::Prompt)
                    .body(roko_core::Body::text("fresh public output"))
                    .provenance(roko_core::Provenance::trusted("unsafe-cell"))
                    .build(),
            ])
        }
    }

    struct JsonOutputCell {
        value: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl Cell for JsonOutputCell {
        fn cell_id(&self) -> &str {
            "json-output"
        }

        fn cell_name(&self) -> &str {
            "JsonOutputCell"
        }

        async fn execute(
            &self,
            _input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            Ok(vec![
                roko_core::Signal::builder(roko_core::Kind::Custom("route.output".into()))
                    .body(roko_core::Body::Json(self.value.clone()))
                    .build(),
            ])
        }
    }

    struct AlwaysFailCell;

    #[async_trait::async_trait]
    impl Cell for AlwaysFailCell {
        fn cell_id(&self) -> &str {
            "always-fail"
        }

        fn cell_name(&self) -> &str {
            "AlwaysFailCell"
        }

        async fn execute(
            &self,
            _input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            Err(roko_core::RokoError::invalid("intentional test failure"))
        }
    }

    struct TickIncrementCell;

    #[async_trait::async_trait]
    impl Cell for TickIncrementCell {
        fn cell_id(&self) -> &str {
            "tick-increment"
        }

        fn cell_name(&self) -> &str {
            "TickIncrementCell"
        }

        async fn execute(
            &self,
            input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            let previous = input
                .iter()
                .filter_map(|signal| match &signal.body {
                    roko_core::Body::Json(value) => value.get("tick")?.as_u64(),
                    _ => None,
                })
                .max()
                .unwrap_or(0);
            Ok(vec![
                roko_core::Signal::builder(roko_core::Kind::Custom("tick.state".into()))
                    .body(roko_core::Body::Json(serde_json::json!({
                        "tick": previous + 1
                    })))
                    .build(),
            ])
        }
    }

    #[derive(Default)]
    struct RecordingTelemetry {
        events: std::sync::Mutex<Vec<(ObservableEvent, Vec<LensScope>)>>,
        fail: bool,
    }

    #[async_trait::async_trait]
    impl TelemetryEventSink for RecordingTelemetry {
        async fn emit(
            &self,
            event: &ObservableEvent,
            ancestry: &[LensScope],
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            self.events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((event.clone(), ancestry.to_vec()));
            if self.fail {
                Err(roko_core::RokoError::invalid("test telemetry failure"))
            } else {
                Ok(Vec::new())
            }
        }
    }

    struct FailThenSucceedCell {
        attempts: Arc<AtomicU64>,
        failures_before_success: u64,
        corrections: Arc<AtomicU64>,
    }

    #[async_trait::async_trait]
    impl Cell for FailThenSucceedCell {
        fn cell_id(&self) -> &str {
            "flaky"
        }

        fn cell_name(&self) -> &str {
            "FailThenSucceedCell"
        }

        fn predict(&self, input: &[roko_core::Signal]) -> Option<roko_core::PredictionRecord> {
            Some(roko_core::PredictionRecord {
                cell_id: self.cell_id().to_string(),
                predicted_outcome: serde_json::json!({"output_count": input.len()}),
                confidence: 1.0,
                timestamp_ms: 0,
            })
        }

        fn calibration_error(
            &self,
            _prediction: &roko_core::PredictionRecord,
            _actual: &[roko_core::Signal],
        ) -> Option<f64> {
            Some(0.0)
        }

        fn correct(
            &self,
            _prediction: &roko_core::PredictionRecord,
            _actual: &[roko_core::Signal],
        ) {
            self.corrections.fetch_add(1, Ordering::SeqCst);
        }

        async fn execute(
            &self,
            input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
            if attempt < self.failures_before_success {
                Err(roko_core::RokoError::invalid(format!(
                    "transient failure {}",
                    attempt + 1
                )))
            } else {
                Ok(input)
            }
        }
    }

    fn noop_registry() -> CellRegistry {
        let mut r = CellRegistry::new();
        r.register("noop", |_| Box::new(NoopCell::default()));
        r.register("gate.compile", |_| {
            Box::new(NoopCell::with_id_and_name("gate.compile", "CompileGate"))
        });
        r.register("gate.test", |_| {
            Box::new(NoopCell::with_id_and_name("gate.test", "TestGate"))
        });
        r.register("gate.clippy", |_| {
            Box::new(NoopCell::with_id_and_name("gate.clippy", "ClippyGate"))
        });
        r
    }

    fn result_status(output: &GraphOutput, node_id: &str) -> NodeStatus {
        output
            .node_results
            .iter()
            .find(|result| result.node_id == node_id)
            .unwrap_or_else(|| panic!("missing result for node `{node_id}`"))
            .status
    }

    #[tokio::test]
    async fn execute_linear_graph() {
        let toml_str = r#"
[graph]
name = "linear"

[[nodes]]
id = "a"
cell_type = "noop"

[[nodes]]
id = "b"
cell_type = "noop"

[[nodes]]
id = "c"
cell_type = "noop"

[[edges]]
from = "a"
to = "b"

[[edges]]
from = "b"
to = "c"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let engine = GraphEngine::new(graph, noop_registry());
        let ctx = CellContext::new();
        let output = engine.execute(&ctx).await.unwrap();

        assert!(output.success);
        assert_eq!(output.node_results.len(), 3);
        assert!(
            output
                .node_results
                .iter()
                .all(|r| r.status == NodeStatus::Complete)
        );
    }

    #[tokio::test]
    async fn execute_single_node() {
        let toml_str = r#"
[graph]
name = "single"

[[nodes]]
id = "only"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let engine = GraphEngine::new(graph, noop_registry());
        let ctx = CellContext::new();
        let output = engine.execute(&ctx).await.unwrap();

        assert!(output.success);
        assert_eq!(output.node_results.len(), 1);
        assert_eq!(output.node_results[0].status, NodeStatus::Complete);
    }

    #[tokio::test]
    async fn root_inputs_reach_root_cells_and_flow_downstream() {
        let graph = load_from_str(
            r#"
[graph]
name = "root-input"

[[nodes]]
id = "root"
cell_type = "noop"

[[nodes]]
id = "sink"
cell_type = "capture"

[[edges]]
from = "root"
to = "sink"
"#,
        )
        .unwrap();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = Arc::clone(&received);
        let mut registry = noop_registry();
        registry.register("capture", move |_| {
            Box::new(CaptureCell {
                received: Arc::clone(&capture),
            })
        });
        let signal =
            roko_core::Signal::builder(roko_core::Kind::Custom("trigger.input".to_string()))
                .body(roko_core::Body::Json(serde_json::json!({
                    "inputs": {"branch": "main"}
                })))
                .build();

        let output = GraphEngine::new(graph, registry)
            .with_root_inputs(vec![signal.clone()])
            .execute(&CellContext::new())
            .await
            .unwrap();

        assert!(output.success);
        assert_eq!(output.node_results[0].output_count, 1);
        assert_eq!(output.node_results[1].output_count, 1);
        assert_eq!(
            *received
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            vec![signal]
        );
    }

    #[tokio::test]
    async fn parallel_execution_preserves_signal_flow_between_waves() {
        let graph = load_from_str(
            r#"
[graph]
name = "parallel-signal-flow"

[graph.policy]
max_concurrent_nodes = 2

[[nodes]]
id = "root"
cell_type = "noop"

[[nodes]]
id = "sink"
cell_type = "capture"

[[edges]]
from = "root"
to = "sink"
"#,
        )
        .unwrap();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = Arc::clone(&received);
        let mut registry = noop_registry();
        registry.register("capture", move |_| {
            Box::new(CaptureCell {
                received: Arc::clone(&capture),
            })
        });
        let signal = roko_core::Signal::builder(roko_core::Kind::Task)
            .body(roko_core::Body::text("flow between waves"))
            .build();

        let output = GraphEngine::new(graph, registry)
            .with_root_inputs(vec![signal.clone()])
            .execute_parallel(&CellContext::new())
            .await
            .unwrap();

        assert!(output.success);
        assert_eq!(
            *received
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            vec![signal]
        );
    }

    #[tokio::test]
    async fn sequential_output_condition_selects_only_matching_branch() {
        let graph = load_from_str(
            r#"
[graph]
name = "conditional-sequential"

[graph.policy]
max_concurrent_nodes = 1

[[nodes]]
id = "route"
cell_type = "json-output"

[[nodes]]
id = "left"
cell_type = "noop"

[[nodes]]
id = "right"
cell_type = "noop"

[[nodes]]
id = "right-child"
cell_type = "noop"

[[edges]]
from = "route"
to = "left"
[edges.condition]
type = "output_equals"
key = "route"
value = "left"

[[edges]]
from = "route"
to = "right"
[edges.condition]
type = "output_equals"
key = "route"
value = "right"

[[edges]]
from = "right"
to = "right-child"
"#,
        )
        .unwrap();
        let mut registry = noop_registry();
        registry.register("json-output", |_| {
            Box::new(JsonOutputCell {
                value: serde_json::json!({"route": "left"}),
            })
        });

        let output = GraphEngine::new(graph, registry)
            .execute(&CellContext::new())
            .await
            .unwrap();

        assert!(output.success);
        assert_eq!(result_status(&output, "route"), NodeStatus::Complete);
        assert_eq!(result_status(&output, "left"), NodeStatus::Complete);
        assert_eq!(
            result_status(&output, "right"),
            NodeStatus::ConditionSkipped
        );
        assert_eq!(
            result_status(&output, "right-child"),
            NodeStatus::ConditionSkipped
        );
    }

    #[tokio::test]
    async fn parallel_output_condition_matches_nested_json_with_sequential_parity() {
        let graph = load_from_str(
            r#"
[graph]
name = "conditional-parallel"

[graph.policy]
max_concurrent_nodes = 4

[[nodes]]
id = "route"
cell_type = "json-output"

[[nodes]]
id = "selected"
cell_type = "noop"

[[nodes]]
id = "unselected"
cell_type = "noop"

[[edges]]
from = "route"
to = "selected"
[edges.condition]
type = "output_equals"
key = "decision.status"
value = "go"

[[edges]]
from = "route"
to = "unselected"
[edges.condition]
type = "output_equals"
key = "decision.status"
value = "stop"
"#,
        )
        .unwrap();
        let mut registry = noop_registry();
        registry.register("json-output", |_| {
            Box::new(JsonOutputCell {
                value: serde_json::json!({"decision": {"status": "go"}}),
            })
        });

        let output = GraphEngine::new(graph, registry)
            .execute(&CellContext::new())
            .await
            .unwrap();

        assert!(output.success);
        assert_eq!(result_status(&output, "selected"), NodeStatus::Complete);
        assert_eq!(
            result_status(&output, "unselected"),
            NodeStatus::ConditionSkipped
        );
    }

    #[tokio::test]
    async fn failure_condition_runs_handler_under_skip_failed_policy() {
        let graph = load_from_str(
            r#"
[graph]
name = "failure-route"

[graph.policy]
failure_strategy = "skip_failed"
max_concurrent_nodes = 1

[[nodes]]
id = "source"
cell_type = "always-fail"

[[nodes]]
id = "recovery"
cell_type = "noop"

[[nodes]]
id = "success-only"
cell_type = "noop"

[[edges]]
from = "source"
to = "recovery"
[edges.condition]
type = "failure"

[[edges]]
from = "source"
to = "success-only"
[edges.condition]
type = "success"
"#,
        )
        .unwrap();
        let mut registry = noop_registry();
        registry.register("always-fail", |_| Box::new(AlwaysFailCell));

        let output = GraphEngine::new(graph, registry)
            .execute(&CellContext::new())
            .await
            .unwrap();

        assert!(!output.success);
        assert_eq!(result_status(&output, "source"), NodeStatus::Failed);
        assert_eq!(result_status(&output, "recovery"), NodeStatus::Complete);
        assert_eq!(
            result_status(&output, "success-only"),
            NodeStatus::ConditionSkipped
        );
    }

    #[tokio::test]
    async fn resume_uses_restored_activity_output_for_conditional_routing() {
        let graph = load_from_str(
            r#"
[graph]
name = "conditional-resume"

[[nodes]]
id = "route"
cell_type = "noop"

[[nodes]]
id = "left"
cell_type = "noop"

[[nodes]]
id = "right"
cell_type = "noop"

[[edges]]
from = "route"
to = "left"
[edges.condition]
type = "output_equals"
key = "route"
value = "left"

[[edges]]
from = "route"
to = "right"
[edges.condition]
type = "output_equals"
key = "route"
value = "right"
"#,
        )
        .unwrap();
        let mut statuses = HashMap::new();
        statuses.insert("route".to_string(), NodeStatus::Complete);
        statuses.insert("left".to_string(), NodeStatus::Pending);
        statuses.insert("right".to_string(), NodeStatus::Pending);
        let mut outputs = HashMap::new();
        outputs.insert(
            "route".to_string(),
            vec![
                roko_core::Signal::builder(roko_core::Kind::Custom("route.output".into()))
                    .body(roko_core::Body::Json(serde_json::json!({"route": "left"})))
                    .build(),
            ],
        );
        let snapshot =
            GraphEngine::new(graph.clone(), noop_registry()).snapshot(&statuses, &outputs, 0);

        let output =
            GraphEngine::resume_from(&snapshot, graph, noop_registry(), &CellContext::new())
                .await
                .unwrap();

        assert!(output.success);
        assert_eq!(result_status(&output, "route"), NodeStatus::Complete);
        assert_eq!(result_status(&output, "left"), NodeStatus::Complete);
        assert_eq!(
            result_status(&output, "right"),
            NodeStatus::ConditionSkipped
        );
    }

    #[tokio::test]
    async fn live_flow_applies_conditional_routing() {
        let graph = load_from_str(
            r#"
[graph]
name = "conditional-flow"

[[nodes]]
id = "route"
cell_type = "json-output"

[[nodes]]
id = "selected"
cell_type = "noop"

[[nodes]]
id = "unselected"
cell_type = "noop"

[[edges]]
from = "route"
to = "selected"
[edges.condition]
type = "success"

[[edges]]
from = "route"
to = "unselected"
[edges.condition]
type = "failure"
"#,
        )
        .unwrap();
        let mut registry = noop_registry();
        registry.register("json-output", |_| {
            Box::new(JsonOutputCell {
                value: serde_json::json!({"ok": true}),
            })
        });

        let output = GraphEngine::new(graph, registry)
            .start(CellContext::new())
            .await_completion()
            .await
            .expect("flow output");

        assert!(output.success);
        assert_eq!(result_status(&output, "selected"), NodeStatus::Complete);
        assert_eq!(
            result_status(&output, "unselected"),
            NodeStatus::ConditionSkipped
        );
    }

    #[tokio::test]
    async fn hot_policy_persists_cell_outputs_between_ticks() {
        let graph = load_from_str(
            r#"
[graph]
name = "stateful-hot"

[graph.policy]
mode = "hot"
max_concurrent_nodes = 2

[graph.policy.hot]
persist_tick_state = true

[[nodes]]
id = "counter"
cell_type = "tick-increment"

[[nodes]]
id = "sink"
cell_type = "capture"

[[edges]]
from = "counter"
to = "sink"
"#,
        )
        .unwrap();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = Arc::clone(&received);
        let mut registry = noop_registry();
        registry.register("tick-increment", |_| Box::new(TickIncrementCell));
        registry.register("capture", move |_| {
            Box::new(CaptureCell {
                received: Arc::clone(&capture),
            })
        });
        let engine = GraphEngine::new(graph, registry);

        assert!(
            engine
                .execute_parallel_at_tick(&CellContext::new(), 0)
                .await
                .unwrap()
                .success
        );
        assert!(
            engine
                .execute_parallel_at_tick(&CellContext::new(), 1)
                .await
                .unwrap()
                .success
        );

        let signals = received
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(signals.len(), 1);
        let roko_core::Body::Json(body) = &signals[0].body else {
            panic!("tick state must remain structured JSON");
        };
        assert_eq!(body["tick"], serde_json::json!(2));
    }

    #[tokio::test]
    async fn graph_boundary_prevents_cell_from_lowering_input_taint() {
        let graph = load_from_str(
            r#"
[graph]
name = "taint-flow"

[[nodes]]
id = "lowering"
cell_type = "taint-lowering"

[[nodes]]
id = "sink"
cell_type = "capture"

[[edges]]
from = "lowering"
to = "sink"
"#,
        )
        .unwrap();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture = Arc::clone(&received);
        let mut registry = noop_registry();
        registry.register("taint-lowering", |_| Box::new(TaintLoweringCell));
        registry.register("capture", move |_| {
            Box::new(CaptureCell {
                received: Arc::clone(&capture),
            })
        });
        let classified = roko_core::Signal::builder(roko_core::Kind::Task)
            .provenance(
                roko_core::Provenance::external("webhook")
                    .with_taint_level(roko_core::TaintLevel::Secret),
            )
            .build();

        let output = GraphEngine::new(graph, registry)
            .with_root_inputs(vec![classified])
            .execute(&CellContext::new())
            .await
            .unwrap();

        assert!(output.success);
        let signals = received
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(signals.len(), 1);
        assert_eq!(
            signals[0].provenance.effective_taint(),
            roko_core::TaintLevel::Secret
        );
    }

    #[tokio::test]
    async fn graph_execution_emits_scoped_lifecycle_events() {
        let graph = load_from_str(
            r#"
[graph]
name = "observed"

[[nodes]]
id = "only"
cell_type = "noop"
"#,
        )
        .unwrap();
        let telemetry = Arc::new(RecordingTelemetry::default());
        let engine = GraphEngine::new(graph, noop_registry()).with_telemetry(telemetry.clone());
        let output = engine
            .execute(&CellContext::new().with_run_id("run-1".into()))
            .await
            .unwrap();
        assert!(output.success);

        let events = telemetry
            .events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(matches!(
            events.first().map(|entry| &entry.0),
            Some(ObservableEvent::GraphStarted { run, .. }) if run == "run-1"
        ));
        assert!(events.iter().any(|(event, ancestry)| {
            matches!(event, ObservableEvent::CellCompleted { block, .. } if block == "only")
                && ancestry
                    == &[
                        LensScope::Cell("only".into()),
                        LensScope::Graph("observed".into()),
                    ]
        }));
        assert!(matches!(
            events.last().map(|entry| &entry.0),
            Some(ObservableEvent::GraphCompleted { graph, .. }) if graph == "observed"
        ));
    }

    #[tokio::test]
    async fn predictive_cell_publishes_once_and_receives_one_terminal_calibration() {
        let graph = load_from_str(
            r#"
[graph]
name = "predictive"

[[nodes]]
id = "assess-node"
cell_type = "assess"
"#,
        )
        .expect("predictive graph");
        let telemetry = Arc::new(RecordingTelemetry::default());
        let input = roko_core::Signal::builder(roko_core::Kind::AgentMessage)
            .body(roko_core::Body::text("one input"))
            .build();
        let output = GraphEngine::new(graph, default_registry())
            .with_root_inputs(vec![input])
            .with_telemetry(telemetry.clone())
            .execute(&CellContext::new().with_run_id("prediction-run".into()))
            .await
            .expect("predictive execution");
        assert!(output.success);

        let events = telemetry
            .events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let published = events
            .iter()
            .position(|(event, ancestry)| {
                matches!(
                    event,
                    ObservableEvent::CellPredictionPublished { block, prediction }
                        if block == "assess-node"
                            && serde_json::from_str::<roko_core::PredictionRecord>(prediction)
                                .is_ok_and(|record| {
                                    record.cell_id == "assess"
                                        && record.predicted_outcome["output_count"] == 1
                                })
                ) && ancestry
                    == &[
                        LensScope::Cell("assess-node".into()),
                        LensScope::Graph("predictive".into()),
                    ]
            })
            .expect("prediction publication");
        let calibrated = events
            .iter()
            .position(|(event, ancestry)| {
                matches!(
                    event,
                    ObservableEvent::CellCalibrationReceived { block, error }
                        if block == "assess-node" && *error == 0.0
                ) && ancestry
                    == &[
                        LensScope::Cell("assess-node".into()),
                        LensScope::Graph("predictive".into()),
                    ]
            })
            .expect("calibration receipt");
        let completed = events
            .iter()
            .position(|(event, _)| {
                matches!(event, ObservableEvent::CellCompleted { block, .. } if block == "assess-node")
            })
            .expect("cell completion");
        assert!(published < calibrated);
        assert!(calibrated < completed);
        assert_eq!(
            events
                .iter()
                .filter(|(event, _)| matches!(
                    event,
                    ObservableEvent::CellPredictionPublished { .. }
                ))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|(event, _)| matches!(
                    event,
                    ObservableEvent::CellCalibrationReceived { .. }
                ))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn replay_emits_graph_resumed_once_without_reexecuting_activity() {
        let graph = load_from_str(
            r#"
[graph]
name = "resumed"

[[nodes]]
id = "only"
cell_type = "noop"
"#,
        )
        .unwrap();
        let recording = tempfile::NamedTempFile::new().unwrap();
        let mut recorder = ActivityRecorder::create_fresh("resume-run", recording.path()).unwrap();
        recorder.record("resumed", "only", 0, Vec::new()).unwrap();
        drop(recorder);
        let replayer =
            ActivityReplayer::load_scoped(recording.path(), "resumed", "resume-run").unwrap();
        let telemetry = Arc::new(RecordingTelemetry::default());

        let output = GraphEngine::new(graph, noop_registry())
            .with_replayer(replayer)
            .with_telemetry(telemetry.clone())
            .execute(&CellContext::new().with_run_id("resume-run".into()))
            .await
            .unwrap();

        assert!(output.success);
        let events = telemetry
            .events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(
            events
                .iter()
                .filter(|(event, _)| matches!(event, ObservableEvent::GraphResumed { .. }))
                .count(),
            1
        );
        assert!(
            !events
                .iter()
                .any(|(event, _)| matches!(event, ObservableEvent::CellStarted { .. }))
        );
        assert!(matches!(
            events.first().map(|entry| &entry.0),
            Some(ObservableEvent::GraphStarted { .. })
        ));
    }

    #[tokio::test]
    async fn telemetry_failure_never_changes_graph_outcome() {
        let graph = load_from_str(
            r#"
[graph]
name = "passive"

[[nodes]]
id = "only"
cell_type = "noop"
"#,
        )
        .unwrap();
        let telemetry = Arc::new(RecordingTelemetry {
            fail: true,
            ..RecordingTelemetry::default()
        });
        let output = GraphEngine::new(graph, noop_registry())
            .with_telemetry(telemetry)
            .execute(&CellContext::new())
            .await
            .unwrap();
        assert!(output.success);
    }

    #[tokio::test]
    async fn retry_policy_executes_retries_and_emits_each_transition() {
        let graph = load_from_str(
            r#"
[graph]
name = "retry-observed"

[graph.policy]
failure_strategy = { retry = { max_retries = 2 } }

[[nodes]]
id = "flaky"
cell_type = "flaky"
"#,
        )
        .unwrap();
        let attempts = Arc::new(AtomicU64::new(0));
        let corrections = Arc::new(AtomicU64::new(0));
        let cell_attempts = Arc::clone(&attempts);
        let cell_corrections = Arc::clone(&corrections);
        let mut registry = CellRegistry::new();
        registry.register("flaky", move |_| {
            Box::new(FailThenSucceedCell {
                attempts: Arc::clone(&cell_attempts),
                failures_before_success: 2,
                corrections: Arc::clone(&cell_corrections),
            })
        });
        let telemetry = Arc::new(RecordingTelemetry::default());

        let output = GraphEngine::new(graph, registry)
            .with_telemetry(telemetry.clone())
            .execute(&CellContext::new().with_run_id("retry-run".into()))
            .await
            .unwrap();

        assert!(output.success);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(corrections.load(Ordering::SeqCst), 1);
        let events = telemetry
            .events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let retries = events
            .iter()
            .filter_map(|(event, ancestry)| match event {
                ObservableEvent::CellRetried {
                    block,
                    run,
                    attempt,
                    reason,
                } => Some((block, run, *attempt, reason, ancestry)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(retries.len(), 2);
        assert_eq!(retries[0].2, 1);
        assert_eq!(retries[1].2, 2);
        assert_eq!(retries[0].0, "flaky");
        assert_eq!(retries[0].1, "retry-run");
        assert!(retries[0].3.contains("transient failure 1"));
        assert_eq!(
            retries[0].4,
            &[
                LensScope::Cell("flaky".into()),
                LensScope::Graph("retry-observed".into()),
            ]
        );
        assert!(
            !events
                .iter()
                .any(|(event, _)| matches!(event, ObservableEvent::CellFailed { .. }))
        );
        assert_eq!(
            events
                .iter()
                .filter(|(event, _)| matches!(
                    event,
                    ObservableEvent::CellPredictionPublished { .. }
                ))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|(event, _)| matches!(
                    event,
                    ObservableEvent::CellCalibrationReceived { .. }
                ))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn exhausted_retry_policy_emits_one_terminal_failure() {
        let graph = load_from_str(
            r#"
[graph]
name = "retry-exhausted"

[graph.policy]
failure_strategy = { retry = { max_retries = 2 } }

[[nodes]]
id = "flaky"
cell_type = "flaky"
"#,
        )
        .unwrap();
        let attempts = Arc::new(AtomicU64::new(0));
        let corrections = Arc::new(AtomicU64::new(0));
        let cell_attempts = Arc::clone(&attempts);
        let cell_corrections = Arc::clone(&corrections);
        let mut registry = CellRegistry::new();
        registry.register("flaky", move |_| {
            Box::new(FailThenSucceedCell {
                attempts: Arc::clone(&cell_attempts),
                failures_before_success: u64::MAX,
                corrections: Arc::clone(&cell_corrections),
            })
        });
        let telemetry = Arc::new(RecordingTelemetry::default());

        let output = GraphEngine::new(graph, registry)
            .with_telemetry(telemetry.clone())
            .execute(&CellContext::new())
            .await
            .unwrap();

        assert!(!output.success);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(corrections.load(Ordering::SeqCst), 0);
        let events = telemetry
            .events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(
            events
                .iter()
                .filter(|(event, _)| matches!(event, ObservableEvent::CellRetried { .. }))
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|(event, _)| matches!(event, ObservableEvent::CellFailed { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|(event, _)| matches!(
                    event,
                    ObservableEvent::CellPredictionPublished { .. }
                ))
                .count(),
            1
        );
        assert!(
            !events
                .iter()
                .any(|(event, _)| matches!(event, ObservableEvent::CellCalibrationReceived { .. }))
        );
        assert!(matches!(
            events.last().map(|entry| &entry.0),
            Some(ObservableEvent::GraphFailed { .. })
        ));
    }

    #[tokio::test]
    async fn cancelling_flow_emits_cell_cancelled_before_graph_paused() {
        let graph = load_from_str(
            r#"
[graph]
name = "cancel-observed"

[[nodes]]
id = "pending"
cell_type = "noop"
"#,
        )
        .unwrap();
        let telemetry = Arc::new(RecordingTelemetry::default());
        let handle = GraphEngine::new(graph, noop_registry())
            .with_telemetry(telemetry.clone())
            .start(CellContext::new().with_run_id("cancel-run".into()));
        handle.cancel();
        let output = handle.await_completion().await.expect("cancelled output");

        assert!(!output.success);
        let events = telemetry
            .events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cancelled = events
            .iter()
            .position(|(event, ancestry)| {
                matches!(
                    event,
                    ObservableEvent::CellCancelled { block, run }
                        if block == "pending" && run == "cancel-run"
                ) && ancestry
                    == &[
                        LensScope::Cell("pending".into()),
                        LensScope::Graph("cancel-observed".into()),
                    ]
            })
            .expect("CellCancelled event");
        let paused = events
            .iter()
            .position(|(event, _)| matches!(event, ObservableEvent::GraphPaused { .. }))
            .expect("GraphPaused event");
        assert!(cancelled < paused);
    }

    #[tokio::test]
    async fn validate_missing_cell_type() {
        let toml_str = r#"
[graph]
name = "bad"

[[nodes]]
id = "a"
cell_type = "nonexistent"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let engine = GraphEngine::new(graph, noop_registry());
        let issues = engine.validate();
        assert!(!issues.is_empty());
        assert!(issues[0].contains("nonexistent"));
    }

    #[tokio::test]
    async fn validate_valid_graph() {
        let toml_str = r#"
[graph]
name = "valid"

[[nodes]]
id = "a"
cell_type = "noop"

[[nodes]]
id = "b"
cell_type = "gate.compile"

[[edges]]
from = "a"
to = "b"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let engine = GraphEngine::new(graph, noop_registry());
        let issues = engine.validate();
        assert!(issues.is_empty());
    }

    // ─── validate_for_start / graph_validation tests ──────────────────────────

    mod graph_validation {
        use super::*;
        use crate::GraphMetadata;
        use crate::registry::CellDescriptor;
        use roko_core::{Kind, TypeSchema};

        /// Build a minimal registry with a single untyped noop entry.
        fn untyped_registry() -> CellRegistry {
            let mut reg = CellRegistry::new();
            reg.register("noop", |_| Box::new(NoopCell::default()));
            reg
        }

        /// Build a registry with typed descriptors for edge validation.
        fn typed_registry() -> CellRegistry {
            let mut reg = CellRegistry::new();
            reg.register_with_descriptor(
                "agent-msg-source",
                CellDescriptor::new(
                    "agent-msg-source",
                    (1, 0, 0),
                    None,
                    Some(TypeSchema::OfKind(Kind::AgentMessage)),
                ),
                |_| {
                    Box::new(NoopCell::with_id_and_name(
                        "agent-msg-source",
                        "AgentMsgSource",
                    ))
                },
            );
            reg.register_with_descriptor(
                "agent-msg-sink",
                CellDescriptor::new(
                    "agent-msg-sink",
                    (1, 0, 0),
                    Some(TypeSchema::OfKind(Kind::AgentMessage)),
                    None,
                ),
                |_| Box::new(NoopCell::with_id_and_name("agent-msg-sink", "AgentMsgSink")),
            );
            reg.register_with_descriptor(
                "episode-sink",
                CellDescriptor::new(
                    "episode-sink",
                    (1, 0, 0),
                    Some(TypeSchema::OfKind(Kind::Episode)),
                    None,
                ),
                |_| Box::new(NoopCell::with_id_and_name("episode-sink", "EpisodeSink")),
            );
            reg.register("noop", |_| Box::new(NoopCell::default()));
            reg
        }

        fn make_node(id: &str, cell_type: &str) -> crate::types::Node {
            crate::types::Node {
                id: id.to_string(),
                cell_type: cell_type.to_string(),
                config: toml::Value::Table(toml::map::Map::new()),
                inputs: vec![],
                outputs: vec![],
                execution_class: crate::types::ExecutionClass::default(),
                exclusive: vec![],
            }
        }

        fn make_edge(from: &str, to: &str) -> crate::types::Edge {
            crate::types::Edge {
                from: from.to_string(),
                to: to.to_string(),
                condition: None,
            }
        }

        #[test]
        fn graph_validation_compatible_edges_pass() {
            let registry = typed_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "compatible".to_string(),
                ..Default::default()
            });
            graph
                .add_node(make_node("src", "agent-msg-source"))
                .unwrap();
            graph.add_node(make_node("tgt", "agent-msg-sink")).unwrap();
            graph.add_edge(make_edge("src", "tgt")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let result = engine.validate_for_start();
            assert!(result.is_ok(), "compatible typed edges should pass");
        }

        #[test]
        fn graph_validation_mismatched_types_fail() {
            let registry = typed_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "mismatch".to_string(),
                ..Default::default()
            });
            graph
                .add_node(make_node("src", "agent-msg-source"))
                .unwrap();
            graph.add_node(make_node("tgt", "episode-sink")).unwrap();
            graph.add_edge(make_edge("src", "tgt")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let result = engine.validate_for_start();
            assert!(result.is_err(), "incompatible types should fail validation");
            let err = result.unwrap_err();
            assert!(
                matches!(err, GraphError::EdgeValidationFailed { count: 1, .. }),
                "expected EdgeValidationFailed, got: {err:?}"
            );
        }

        #[test]
        fn graph_validation_missing_registry_entry_fails() {
            let registry = CellRegistry::new(); // empty
            let mut graph = Graph::new(GraphMetadata {
                name: "missing".to_string(),
                ..Default::default()
            });
            graph.add_node(make_node("a", "nonexistent")).unwrap();
            graph.add_node(make_node("b", "also-nonexistent")).unwrap();
            graph.add_edge(make_edge("a", "b")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let result = engine.validate_for_start();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                matches!(err, GraphError::EdgeValidationFailed { .. }),
                "expected EdgeValidationFailed, got: {err:?}"
            );
        }

        #[test]
        fn graph_validation_untyped_edges_always_pass() {
            let registry = untyped_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "untyped".to_string(),
                ..Default::default()
            });
            graph.add_node(make_node("a", "noop")).unwrap();
            graph.add_node(make_node("b", "noop")).unwrap();
            graph.add_edge(make_edge("a", "b")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let result = engine.validate_for_start();
            assert!(
                result.is_ok(),
                "untyped (None schema) edges should always pass"
            );
        }

        #[test]
        fn graph_validation_no_edges_passes() {
            let registry = untyped_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "no-edges".to_string(),
                ..Default::default()
            });
            graph.add_node(make_node("a", "noop")).unwrap();
            graph.add_node(make_node("b", "noop")).unwrap();
            // No edges at all

            let engine = GraphEngine::new(graph, registry);
            let result = engine.validate_for_start();
            assert!(result.is_ok(), "graph with no edges should pass");
        }

        #[test]
        fn graph_validation_error_includes_node_names() {
            let registry = typed_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "names".to_string(),
                ..Default::default()
            });
            graph
                .add_node(make_node("my-source", "agent-msg-source"))
                .unwrap();
            graph
                .add_node(make_node("my-target", "episode-sink"))
                .unwrap();
            graph.add_edge(make_edge("my-source", "my-target")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let err = engine.validate_for_start().unwrap_err();
            let display = err.to_string();
            assert!(
                display.contains("my-source") || display.contains("my-target"),
                "error should include node names: {display}"
            );
        }

        #[test]
        fn graph_validation_collects_all_errors() {
            let registry = typed_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "multi-error".to_string(),
                ..Default::default()
            });
            graph
                .add_node(make_node("src", "agent-msg-source"))
                .unwrap();
            graph.add_node(make_node("tgt1", "episode-sink")).unwrap();
            graph.add_node(make_node("tgt2", "episode-sink")).unwrap();
            graph.add_edge(make_edge("src", "tgt1")).unwrap();
            graph.add_edge(make_edge("src", "tgt2")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let err = engine.validate_for_start().unwrap_err();
            assert!(
                matches!(err, GraphError::EdgeValidationFailed { count: 2, .. }),
                "expected 2 errors, got: {err:?}"
            );
        }

        #[test]
        fn graph_validation_idempotent_after_success() {
            let registry = untyped_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "idempotent".to_string(),
                ..Default::default()
            });
            graph.add_node(make_node("a", "noop")).unwrap();
            graph.add_node(make_node("b", "noop")).unwrap();
            graph.add_edge(make_edge("a", "b")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            // First call validates.
            assert!(engine.validate_for_start().is_ok());
            // Second call returns immediately (cached).
            assert!(engine.validate_for_start().is_ok());
        }

        #[test]
        fn graph_validation_rejects_stub_descriptors_by_default() {
            let mut registry = CellRegistry::new();
            registry.register_with_descriptor(
                "stub-cell",
                CellDescriptor::test_stub("stub-cell"),
                |_| Box::new(NoopCell::default()),
            );

            let mut graph = Graph::new(GraphMetadata {
                name: "stub".to_string(),
                ..Default::default()
            });
            graph.add_node(make_node("a", "stub-cell")).unwrap();
            graph.add_node(make_node("b", "stub-cell")).unwrap();
            graph.add_edge(make_edge("a", "b")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let err = engine.validate_for_start().unwrap_err();
            assert!(
                matches!(err, GraphError::InvalidGraph { ref reason } if reason.contains("test-stub")),
                "expected InvalidGraph with stub mention, got: {err:?}"
            );
        }

        #[test]
        fn graph_validation_allows_stubs_when_flag_set() {
            let mut registry = CellRegistry::new();
            registry.register_with_descriptor(
                "stub-cell",
                CellDescriptor::test_stub("stub-cell"),
                |_| Box::new(NoopCell::default()),
            );

            let mut graph = Graph::new(GraphMetadata {
                name: "stub-allowed".to_string(),
                ..Default::default()
            });
            graph.add_node(make_node("a", "stub-cell")).unwrap();
            graph.add_node(make_node("b", "stub-cell")).unwrap();
            graph.add_edge(make_edge("a", "b")).unwrap();

            let engine = GraphEngine::new(graph, registry).with_allow_test_stubs(true);
            assert!(
                engine.validate_for_start().is_ok(),
                "stubs should pass when allow_test_stubs is true"
            );
        }

        #[test]
        fn graph_validation_stub_error_lists_node_ids() {
            let mut registry = CellRegistry::new();
            registry.register_with_descriptor(
                "stub-cell",
                CellDescriptor::test_stub("stub-cell"),
                |_| Box::new(NoopCell::default()),
            );

            let mut graph = Graph::new(GraphMetadata {
                name: "stub-names".to_string(),
                ..Default::default()
            });
            graph.add_node(make_node("my-stub-a", "stub-cell")).unwrap();
            graph.add_node(make_node("my-stub-b", "stub-cell")).unwrap();
            graph.add_edge(make_edge("my-stub-a", "my-stub-b")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let err = engine.validate_for_start().unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("my-stub-a") && msg.contains("my-stub-b"),
                "error should list stub node IDs: {msg}"
            );
        }

        #[test]
        fn graph_validation_descriptor_introspection_is_side_effect_free() {
            use std::sync::atomic::{AtomicU32, Ordering};

            // Track how many times the factory is called.
            let call_count = Arc::new(AtomicU32::new(0));
            let count_clone = call_count.clone();

            let mut registry = CellRegistry::new();
            registry.register_with_descriptor(
                "tracked",
                CellDescriptor::new(
                    "tracked",
                    (0, 1, 0),
                    Some(TypeSchema::OfKind(Kind::Task)),
                    Some(TypeSchema::OfKind(Kind::Episode)),
                ),
                move |_| {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                    Box::new(NoopCell::default())
                },
            );

            let mut graph = Graph::new(GraphMetadata {
                name: "no-side-effects".to_string(),
                ..Default::default()
            });
            graph.add_node(make_node("a", "tracked")).unwrap();
            graph.add_node(make_node("b", "tracked")).unwrap();
            graph.add_edge(make_edge("a", "b")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let _ = engine.validate_for_start();

            assert_eq!(
                call_count.load(Ordering::Relaxed),
                0,
                "validate_for_start must not call the cell factory"
            );
        }

        #[test]
        fn graph_validation_default_registry_passes() {
            // The default registry should produce valid descriptors for all
            // production cognitive loop edges.
            let registry = default_registry();

            // Build a cognitive loop graph: sense -> assess -> compose -> act ->
            //   verify -> persist -> react
            let mut graph = Graph::new(GraphMetadata {
                name: "cognitive-loop".to_string(),
                ..Default::default()
            });
            for (id, ct) in [
                ("s", "sense"),
                ("a", "assess"),
                ("c", "compose"),
                ("x", "act"),
                ("v", "verify"),
                ("p", "persist"),
                ("r", "react"),
            ] {
                graph.add_node(make_node(id, ct)).unwrap();
            }
            for (from, to) in [("s", "a"), ("a", "c"), ("c", "x"), ("x", "v"), ("v", "p")] {
                graph.add_edge(make_edge(from, to)).unwrap();
            }

            let engine = GraphEngine::new(graph, registry);
            let result = engine.validate_for_start();
            assert!(
                result.is_ok(),
                "default registry cognitive loop should validate: {:?}",
                result.err()
            );
        }

        #[test]
        fn graph_validation_conditional_edge_type_mismatch() {
            let registry = typed_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "conditional-mismatch".to_string(),
                ..Default::default()
            });
            graph
                .add_node(make_node("src", "agent-msg-source"))
                .unwrap();
            graph.add_node(make_node("tgt", "episode-sink")).unwrap();
            graph
                .add_edge(crate::types::Edge {
                    from: "src".to_string(),
                    to: "tgt".to_string(),
                    condition: Some(crate::types::EdgeCondition::OutputEquals {
                        key: "status".to_string(),
                        value: "ok".to_string(),
                    }),
                })
                .unwrap();

            let engine = GraphEngine::new(graph, registry).with_allow_test_stubs(true);
            let result = engine.validate_for_start();
            assert!(
                result.is_err(),
                "conditional edges with incompatible schemas should still fail"
            );
        }

        #[test]
        fn graph_validation_conditional_edge_compatible_passes() {
            let registry = typed_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "conditional-ok".to_string(),
                ..Default::default()
            });
            graph
                .add_node(make_node("src", "agent-msg-source"))
                .unwrap();
            graph.add_node(make_node("tgt", "agent-msg-sink")).unwrap();
            graph
                .add_edge(crate::types::Edge {
                    from: "src".to_string(),
                    to: "tgt".to_string(),
                    condition: Some(crate::types::EdgeCondition::Success),
                })
                .unwrap();

            let engine = GraphEngine::new(graph, registry).with_allow_test_stubs(true);
            assert!(
                engine.validate_for_start().is_ok(),
                "conditional edge with compatible schemas should pass"
            );
        }

        #[test]
        fn graph_validation_mixed_stub_and_production_rejected() {
            let mut registry = typed_registry();
            registry.register_with_descriptor(
                "stub-cell",
                CellDescriptor::test_stub("stub-cell"),
                |_| Box::new(NoopCell::default()),
            );

            let mut graph = Graph::new(GraphMetadata {
                name: "mixed".to_string(),
                ..Default::default()
            });
            graph
                .add_node(make_node("prod", "agent-msg-source"))
                .unwrap();
            graph.add_node(make_node("stub", "stub-cell")).unwrap();
            graph.add_edge(make_edge("prod", "stub")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            let err = engine.validate_for_start().unwrap_err();
            assert!(
                matches!(err, GraphError::InvalidGraph { ref reason } if reason.contains("stub")),
                "mixed graph should be rejected: {err:?}"
            );
        }

        #[test]
        fn graph_validation_production_only_descriptors_pass() {
            let registry = typed_registry();
            let mut graph = Graph::new(GraphMetadata {
                name: "production-only".to_string(),
                ..Default::default()
            });
            graph
                .add_node(make_node("src", "agent-msg-source"))
                .unwrap();
            graph.add_node(make_node("tgt", "agent-msg-sink")).unwrap();
            graph.add_edge(make_edge("src", "tgt")).unwrap();

            let engine = GraphEngine::new(graph, registry);
            assert!(
                engine.validate_for_start().is_ok(),
                "production-only graph should pass"
            );
        }
    }

    // ─── GraphSnapshotV2 tests ──────────────────────────────────────────

    #[test]
    fn snapshot_v2_serde_roundtrip() {
        let snap = GraphSnapshotV2 {
            schema_version: GRAPH_SNAPSHOT_SCHEMA_VERSION,
            graph_name: "test".into(),
            graph_id: "test".into(),
            graph_fingerprint: "abc123".into(),
            node_statuses: HashMap::from([
                ("a".into(), SerializableNodeStatus::Complete),
                ("b".into(), SerializableNodeStatus::Running),
                ("c".into(), SerializableNodeStatus::Pending),
            ]),
            node_outputs: HashMap::new(),
            tick_count: 5,
            budget_spent_micro_usd: 123_456,
            budget_reserved_micro_usd: 50_000,
            last_event_seq: 42,
            created_at_ms: 1_000_000,
            policy: GraphPolicy::default(),
        };

        let json = serde_json::to_string(&snap).expect("serialize");
        let deserialized: GraphSnapshotV2 = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.schema_version, GRAPH_SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(deserialized.graph_fingerprint, "abc123");
        assert_eq!(deserialized.budget_spent_micro_usd, 123_456);
        assert_eq!(deserialized.budget_reserved_micro_usd, 50_000);
        assert_eq!(deserialized.last_event_seq, 42);
        assert_eq!(deserialized.tick_count, 5);
    }

    #[test]
    fn snapshot_v2_backward_compat_missing_new_fields() {
        // Simulate a v1 snapshot (pre-v2) missing the new fields.
        let json = serde_json::json!({
            "graph_name": "old",
            "graph_id": "old",
            "node_statuses": {},
            "node_outputs": {},
            "tick_count": 0,
            "created_at_ms": 1000,
            "policy": {
                "mode": "one_shot",
                "failure_strategy": "fail_fast",
                "max_concurrent_nodes": 4
            }
        });

        let snap: GraphSnapshotV2 = serde_json::from_value(json).expect("deserialize old format");

        assert_eq!(snap.schema_version, GRAPH_SNAPSHOT_SCHEMA_VERSION);
        assert!(snap.graph_fingerprint.is_empty());
        assert_eq!(snap.budget_spent_micro_usd, 0);
        assert_eq!(snap.budget_reserved_micro_usd, 0);
        assert_eq!(snap.last_event_seq, 0);
    }

    #[test]
    fn running_status_preserved_through_serializable_roundtrip() {
        let serializable = SerializableNodeStatus::Running;
        let node_status: NodeStatus = serializable.into();
        assert_eq!(node_status, NodeStatus::Running);
    }

    #[test]
    fn reconcile_running_converts_to_pending() {
        assert_eq!(
            reconcile_running_status(SerializableNodeStatus::Running),
            NodeStatus::Pending
        );
    }

    #[test]
    fn reconcile_complete_stays_complete() {
        assert_eq!(
            reconcile_running_status(SerializableNodeStatus::Complete),
            NodeStatus::Complete
        );
    }

    #[test]
    fn snapshot_type_alias_is_v2() {
        // Ensure GraphSnapshot and GraphSnapshotV2 are the same type.
        fn assert_same_type(_snap: GraphSnapshot) {
            let _v2: GraphSnapshotV2 = _snap;
        }
        let snap = GraphSnapshotV2 {
            schema_version: 2,
            graph_name: "t".into(),
            graph_id: "t".into(),
            graph_fingerprint: String::new(),
            node_statuses: HashMap::new(),
            node_outputs: HashMap::new(),
            tick_count: 0,
            budget_spent_micro_usd: 0,
            budget_reserved_micro_usd: 0,
            last_event_seq: 0,
            created_at_ms: 0,
            policy: GraphPolicy::default(),
        };
        assert_same_type(snap);
    }

    #[test]
    fn snapshot_with_budget_records_all_fields() {
        let toml_str = r#"
            [graph]
            name = "budget-test"

            [[nodes]]
            id = "n1"
            cell_type = "noop"
        "#;
        let graph = load_from_str(toml_str).expect("parse");
        let mut registry = CellRegistry::new();
        registry.register("noop", |_| {
            Box::new(CaptureCell {
                received: Arc::new(std::sync::Mutex::new(Vec::new())),
            })
        });
        let engine = GraphEngine::new(graph, registry);

        let statuses = HashMap::from([("n1".to_string(), NodeStatus::Complete)]);
        let outputs = HashMap::new();

        let snap = engine.snapshot_with_budget(&statuses, &outputs, 3, 100_000, 25_000, 17);

        assert_eq!(snap.schema_version, 2);
        assert_eq!(snap.budget_spent_micro_usd, 100_000);
        assert_eq!(snap.budget_reserved_micro_usd, 25_000);
        assert_eq!(snap.last_event_seq, 17);
        assert_eq!(snap.tick_count, 3);
        assert!(!snap.graph_fingerprint.is_empty());
    }

    // ─── ISSUE-27 regression: start() must dispatch independent nodes concurrently ──

    /// A cell that records its start and end timestamps into a shared vec so the
    /// test can verify that two cells ran concurrently (overlapping wall-clock
    /// intervals) when `max_concurrent_nodes = 2`.
    struct TimestampCell {
        id: String,
        intervals: Arc<parking_lot::Mutex<Vec<(String, Instant, Instant)>>>,
        delay: Duration,
    }

    #[async_trait::async_trait]
    impl Cell for TimestampCell {
        fn cell_id(&self) -> &str {
            &self.id
        }

        fn cell_name(&self) -> &str {
            "TimestampCell"
        }

        async fn execute(
            &self,
            input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            let begin = Instant::now();
            tokio::time::sleep(self.delay).await;
            let end = Instant::now();
            self.intervals.lock().push((self.id.clone(), begin, end));
            Ok(input)
        }
    }

    /// ISSUE-27 regression: `start()` with `max_concurrent_nodes = 2` must run
    /// two independent nodes concurrently. Before the fix, `execute_with_status_tracking`
    /// always ran sequentially, so the second node didn't start until the first finished.
    #[tokio::test]
    async fn start_with_max_parallel_2_dispatches_independent_nodes_concurrently() {
        // Two root nodes with no edges between them — they are in the same
        // topological wave and must run in parallel when max_concurrent_nodes = 2.
        let graph = load_from_str(
            r#"
[graph]
name = "parallel-flow-start"

[graph.policy]
max_concurrent_nodes = 2

[[nodes]]
id = "left"
cell_type = "timed-left"

[[nodes]]
id = "right"
cell_type = "timed-right"
"#,
        )
        .unwrap();

        let intervals: Arc<parking_lot::Mutex<Vec<(String, Instant, Instant)>>> =
            Arc::new(parking_lot::Mutex::new(Vec::new()));
        let iv_left = Arc::clone(&intervals);
        let iv_right = Arc::clone(&intervals);

        let delay = Duration::from_millis(50);

        let mut registry = CellRegistry::new();
        registry.register("timed-left", move |_| {
            Box::new(TimestampCell {
                id: "left".into(),
                intervals: Arc::clone(&iv_left),
                delay,
            })
        });
        registry.register("timed-right", move |_| {
            Box::new(TimestampCell {
                id: "right".into(),
                intervals: Arc::clone(&iv_right),
                delay,
            })
        });

        let output = GraphEngine::new(graph, registry)
            .with_allow_test_stubs(true)
            .start(CellContext::new())
            .await_completion()
            .await
            .expect("flow output");

        assert!(output.success, "both nodes should complete successfully");
        assert_eq!(
            output.node_results.len(),
            2,
            "both nodes must appear in results"
        );
        for r in &output.node_results {
            assert_eq!(
                r.status,
                NodeStatus::Complete,
                "node {} must complete",
                r.node_id
            );
        }

        // Verify the two cells actually ran concurrently: the start of the later-
        // starting cell must be BEFORE the end of the earlier-ending cell.
        let iv = intervals.lock();
        assert_eq!(iv.len(), 2, "both cells must have recorded intervals");
        let (_, begin_0, end_0) = &iv[0];
        let (_, begin_1, end_1) = &iv[1];

        // If they ran sequentially the second began after the first ended.
        // If concurrent the second begins before the first ends (overlap).
        let sequential = begin_1 >= end_0 || begin_0 >= end_1;
        assert!(
            !sequential,
            "nodes ran sequentially but should have overlapped: \
             left [{begin_0:?}..{end_0:?}] right [{begin_1:?}..{end_1:?}]"
        );
    }

    // ─── gap-4d835d: ready-queue dispatch ────────────────────────────────────

    /// Shared record of how [`SleepCell`] runs interleave.
    #[derive(Default)]
    struct SleepLog {
        /// `<label>:start` and `<label>:end`, in the order they happened.
        events: parking_lot::Mutex<Vec<String>>,
        running: AtomicU64,
        max_running: AtomicU64,
    }

    impl SleepLog {
        fn events(&self) -> Vec<String> {
            self.events.lock().clone()
        }

        fn saw(&self, event: &str) -> bool {
            self.events().iter().any(|seen| seen == event)
        }

        fn position(&self, event: &str) -> usize {
            let events = self.events();
            events
                .iter()
                .position(|seen| seen == event)
                .unwrap_or_else(|| panic!("`{event}` never happened: {events:?}"))
        }
    }

    /// Sleeps for `delay_ms` from its node config, logging `<label>:start` and
    /// `<label>:end`, then fails when `fail = true` and succeeds otherwise.
    struct SleepCell {
        label: String,
        delay: Duration,
        fail: bool,
        log: Arc<SleepLog>,
    }

    #[async_trait::async_trait]
    impl Cell for SleepCell {
        fn cell_id(&self) -> &str {
            "sleep"
        }

        fn cell_name(&self) -> &str {
            "SleepCell"
        }

        async fn execute(
            &self,
            input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            self.log.events.lock().push(format!("{}:start", self.label));
            let running = self.log.running.fetch_add(1, Ordering::SeqCst) + 1;
            self.log.max_running.fetch_max(running, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            self.log.running.fetch_sub(1, Ordering::SeqCst);
            self.log.events.lock().push(format!("{}:end", self.label));
            if self.fail {
                Err(roko_core::RokoError::invalid(format!(
                    "{} failed on purpose",
                    self.label
                )))
            } else {
                Ok(input)
            }
        }
    }

    fn sleep_registry(log: &Arc<SleepLog>) -> CellRegistry {
        let log = Arc::clone(log);
        let mut registry = noop_registry();
        registry.register("sleep", move |config| {
            Box::new(SleepCell {
                label: config
                    .get("label")
                    .and_then(toml::Value::as_str)
                    .unwrap_or("unlabelled")
                    .to_string(),
                delay: Duration::from_millis(
                    config
                        .get("delay_ms")
                        .and_then(toml::Value::as_integer)
                        .map_or(0, i64::unsigned_abs),
                ),
                fail: config
                    .get("fail")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false),
                log: Arc::clone(&log),
            })
        });
        registry
    }

    /// gap-4d835d: a node starts as soon as its own dependencies finish, not
    /// once every node of the previous topological wave has.
    ///
    /// `slow` and `fast` form wave 0; `after-fast` needs only `fast`. It must
    /// start and finish while `slow` still runs, through `execute` (graph
    /// runs, Hot Graphs) and through `start` (plan runs).
    #[tokio::test(start_paused = true)]
    async fn a_ready_node_does_not_wait_for_its_wave() {
        let graph = load_from_str(
            r#"
[graph]
name = "ready-queue"

[graph.policy]
max_concurrent_nodes = 4

[[nodes]]
id = "slow"
cell_type = "sleep"
config = { label = "slow", delay_ms = 60000 }

[[nodes]]
id = "fast"
cell_type = "sleep"
config = { label = "fast", delay_ms = 10 }

[[nodes]]
id = "after-fast"
cell_type = "sleep"
config = { label = "after-fast", delay_ms = 10 }

[[edges]]
from = "fast"
to = "after-fast"
"#,
        )
        .unwrap();

        let log = Arc::new(SleepLog::default());
        let output = GraphEngine::new(graph.clone(), sleep_registry(&log))
            .execute(&CellContext::new())
            .await
            .unwrap();
        assert!(output.success);
        assert!(
            log.position("after-fast:end") < log.position("slow:end"),
            "after-fast waited for slow: {:?}",
            log.events()
        );

        let log = Arc::new(SleepLog::default());
        let handle = GraphEngine::new(graph, sleep_registry(&log)).start(CellContext::new());
        tokio::time::sleep(Duration::from_secs(1)).await;
        let statuses = handle.status().node_statuses;
        assert_eq!(statuses["slow"], NodeStatus::Running);
        assert_eq!(statuses["after-fast"], NodeStatus::Complete);
        let output = handle.await_completion().await.expect("flow output");
        assert!(output.success);
        assert!(log.position("after-fast:end") < log.position("slow:end"));
    }

    /// A failed node blocks only its own dependants. Nodes that do not depend
    /// on it still run, including one that becomes ready after the failure,
    /// and the graph reports failure once everything has settled.
    #[tokio::test(start_paused = true)]
    async fn a_failed_node_blocks_only_its_dependants() {
        let graph = load_from_str(
            r#"
[graph]
name = "failure-isolation"

[graph.policy]
failure_strategy = "skip_failed"
max_concurrent_nodes = 4

[[nodes]]
id = "broken"
cell_type = "sleep"
config = { label = "broken", delay_ms = 10, fail = true }

[[nodes]]
id = "after-broken"
cell_type = "sleep"
config = { label = "after-broken" }

[[nodes]]
id = "slow"
cell_type = "sleep"
config = { label = "slow", delay_ms = 1000 }

[[nodes]]
id = "after-slow"
cell_type = "sleep"
config = { label = "after-slow", delay_ms = 10 }

[[edges]]
from = "broken"
to = "after-broken"

[[edges]]
from = "slow"
to = "after-slow"
"#,
        )
        .unwrap();

        for through_start in [false, true] {
            let log = Arc::new(SleepLog::default());
            let engine = GraphEngine::new(graph.clone(), sleep_registry(&log));
            let output = if through_start {
                engine
                    .start(CellContext::new())
                    .await_completion()
                    .await
                    .expect("flow output")
            } else {
                engine.execute(&CellContext::new()).await.unwrap()
            };

            assert!(!output.success, "through_start = {through_start}");
            assert_eq!(result_status(&output, "broken"), NodeStatus::Failed);
            assert_eq!(result_status(&output, "after-broken"), NodeStatus::Skipped);
            assert_eq!(result_status(&output, "slow"), NodeStatus::Complete);
            assert_eq!(result_status(&output, "after-slow"), NodeStatus::Complete);
            assert!(!log.saw("after-broken:start"));
            assert!(
                log.position("broken:end") < log.position("after-slow:start"),
                "after-slow should start after the failure: {:?}",
                log.events()
            );
        }
    }

    /// Under `FailFast` (the default) no node starts after the first failure,
    /// but a node already running finishes.
    #[tokio::test(start_paused = true)]
    async fn fail_fast_starts_no_node_after_the_first_failure() {
        let graph = load_from_str(
            r#"
[graph]
name = "fail-fast"

[graph.policy]
max_concurrent_nodes = 4

[[nodes]]
id = "broken"
cell_type = "sleep"
config = { label = "broken", delay_ms = 10, fail = true }

[[nodes]]
id = "slow"
cell_type = "sleep"
config = { label = "slow", delay_ms = 1000 }

[[nodes]]
id = "after-slow"
cell_type = "sleep"
config = { label = "after-slow" }

[[edges]]
from = "slow"
to = "after-slow"
"#,
        )
        .unwrap();
        let log = Arc::new(SleepLog::default());

        let output = GraphEngine::new(graph, sleep_registry(&log))
            .execute(&CellContext::new())
            .await
            .unwrap();

        assert!(!output.success);
        assert_eq!(result_status(&output, "broken"), NodeStatus::Failed);
        assert_eq!(result_status(&output, "slow"), NodeStatus::Complete);
        assert_eq!(result_status(&output, "after-slow"), NodeStatus::Skipped);
        assert!(!log.saw("after-slow:start"));
    }

    /// Ready nodes never run more than `max_concurrent_nodes` at once, and a
    /// node waiting for a slot stays `Pending` rather than `Running`.
    #[tokio::test(start_paused = true)]
    async fn ready_nodes_respect_max_concurrent_nodes() {
        let graph = load_from_str(
            r#"
[graph]
name = "bounded"

[graph.policy]
max_concurrent_nodes = 2

[[nodes]]
id = "a"
cell_type = "sleep"
config = { label = "a", delay_ms = 100 }

[[nodes]]
id = "b"
cell_type = "sleep"
config = { label = "b", delay_ms = 100 }

[[nodes]]
id = "c"
cell_type = "sleep"
config = { label = "c", delay_ms = 100 }

[[nodes]]
id = "d"
cell_type = "sleep"
config = { label = "d", delay_ms = 100 }
"#,
        )
        .unwrap();
        let log = Arc::new(SleepLog::default());

        let handle = GraphEngine::new(graph, sleep_registry(&log)).start(CellContext::new());
        tokio::time::sleep(Duration::from_millis(50)).await;
        let statuses = handle.status().node_statuses;
        let count = |status| statuses.values().filter(|s| **s == status).count();
        assert_eq!(count(NodeStatus::Running), 2, "{statuses:?}");
        assert_eq!(count(NodeStatus::Pending), 2, "{statuses:?}");

        let output = handle.await_completion().await.expect("flow output");
        assert!(output.success);
        assert_eq!(log.max_running.load(Ordering::SeqCst), 2);
    }

    /// Cancelling a parallel flow starts no further node, even under
    /// `SkipFailed` after a failure. A node already running finishes, and the
    /// nodes that wait on it are cancelled.
    #[tokio::test(start_paused = true)]
    async fn cancelling_a_parallel_flow_starts_no_further_node() {
        let graph = load_from_str(
            r#"
[graph]
name = "cancel-parallel"

[graph.policy]
failure_strategy = "skip_failed"
max_concurrent_nodes = 4

[[nodes]]
id = "broken"
cell_type = "sleep"
config = { label = "broken", delay_ms = 10, fail = true }

[[nodes]]
id = "slow"
cell_type = "sleep"
config = { label = "slow", delay_ms = 1000 }

[[nodes]]
id = "after-slow"
cell_type = "sleep"
config = { label = "after-slow" }

[[edges]]
from = "slow"
to = "after-slow"
"#,
        )
        .unwrap();
        let log = Arc::new(SleepLog::default());
        let telemetry = Arc::new(RecordingTelemetry::default());

        let handle = GraphEngine::new(graph, sleep_registry(&log))
            .with_telemetry(telemetry.clone())
            .start(CellContext::new().with_run_id("cancel-parallel-run".into()));
        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.cancel();
        let output = handle.await_completion().await.expect("flow output");

        assert!(!output.success);
        assert_eq!(result_status(&output, "broken"), NodeStatus::Failed);
        assert_eq!(result_status(&output, "slow"), NodeStatus::Complete);
        assert!(!log.saw("after-slow:start"));
        assert_eq!(
            handle.status().node_statuses["after-slow"],
            NodeStatus::Skipped
        );
        let events = telemetry
            .events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(events.iter().any(|(event, _)| matches!(
            event,
            ObservableEvent::CellCancelled { block, .. } if block == "after-slow"
        )));
        assert!(matches!(
            events.last().map(|entry| &entry.0),
            Some(ObservableEvent::GraphPaused { .. })
        ));
    }

    struct PanicCell;

    #[async_trait::async_trait]
    impl Cell for PanicCell {
        fn cell_id(&self) -> &str {
            "panic"
        }

        fn cell_name(&self) -> &str {
            "PanicCell"
        }

        async fn execute(
            &self,
            _input: Vec<roko_core::Signal>,
            _ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            panic!("cell panicked on purpose");
        }
    }

    /// A cell that panics fails its node, so the graph reports the node and
    /// skips its dependants instead of losing track of them.
    #[tokio::test]
    async fn a_panicking_cell_fails_its_node() {
        let graph = load_from_str(
            r#"
[graph]
name = "panic"

[graph.policy]
failure_strategy = "skip_failed"
max_concurrent_nodes = 2

[[nodes]]
id = "boom"
cell_type = "panic"

[[nodes]]
id = "after-boom"
cell_type = "noop"

[[nodes]]
id = "other"
cell_type = "noop"

[[edges]]
from = "boom"
to = "after-boom"
"#,
        )
        .unwrap();
        let mut registry = noop_registry();
        registry.register("panic", |_| Box::new(PanicCell));

        let output = GraphEngine::new(graph, registry)
            .execute(&CellContext::new())
            .await
            .unwrap();

        assert!(!output.success);
        let boom = output
            .node_results
            .iter()
            .find(|result| result.node_id == "boom")
            .expect("boom result");
        assert_eq!(boom.status, NodeStatus::Failed);
        assert!(
            boom.error
                .as_deref()
                .is_some_and(|e| e.contains("panicked")),
            "{:?}",
            boom.error
        );
        assert_eq!(result_status(&output, "after-boom"), NodeStatus::Skipped);
        assert_eq!(result_status(&output, "other"), NodeStatus::Complete);
    }

    /// A node skipped because a dependency failed names the failed node,
    /// even through a dependency that was skipped in turn, whichever
    /// execution path runs the graph.
    #[tokio::test(start_paused = true)]
    async fn a_skipped_node_names_the_failed_node_that_blocked_it() {
        let graph = load_from_str(
            r#"
[graph]
name = "blocked-by"

[graph.policy]
failure_strategy = "skip_failed"

[[nodes]]
id = "broken"
cell_type = "sleep"
config = { label = "broken", fail = true }

[[nodes]]
id = "child"
cell_type = "sleep"
config = { label = "child" }

[[nodes]]
id = "grandchild"
cell_type = "sleep"
config = { label = "grandchild" }

[[nodes]]
id = "other"
cell_type = "sleep"
config = { label = "other" }

[[edges]]
from = "broken"
to = "child"

[[edges]]
from = "child"
to = "grandchild"
"#,
        )
        .unwrap();

        for max_concurrent_nodes in [1, 4] {
            for through_start in [false, true] {
                let case =
                    format!("max_concurrent_nodes {max_concurrent_nodes}, start {through_start}");
                let mut graph = graph.clone();
                graph.policy.max_concurrent_nodes = max_concurrent_nodes;
                let log = Arc::new(SleepLog::default());
                let engine = GraphEngine::new(graph, sleep_registry(&log));
                let output = if through_start {
                    engine
                        .start(CellContext::new())
                        .await_completion()
                        .await
                        .expect("flow output")
                } else {
                    engine.execute(&CellContext::new()).await.unwrap()
                };
                let blocked_by = |node_id: &str| {
                    output
                        .node_results
                        .iter()
                        .find(|result| result.node_id == node_id)
                        .and_then(|result| result.blocked_by.clone())
                };

                assert!(!output.success, "{case}");
                assert_eq!(
                    result_status(&output, "other"),
                    NodeStatus::Complete,
                    "{case}"
                );
                assert_eq!(blocked_by("broken"), None, "{case}");
                assert_eq!(blocked_by("child").as_deref(), Some("broken"), "{case}");
                assert_eq!(
                    blocked_by("grandchild").as_deref(),
                    Some("broken"),
                    "{case}"
                );
            }
        }
    }

    /// A graph event sink that records every event it receives.
    #[derive(Default)]
    struct RecordingGraphSink {
        events: parking_lot::Mutex<Vec<crate::events::GraphExecutionEvent>>,
    }

    #[async_trait::async_trait]
    impl crate::events::GraphEventSink for RecordingGraphSink {
        async fn publish(
            &self,
            event: &crate::events::GraphExecutionEvent,
        ) -> std::result::Result<crate::events::GraphEventDisposition, crate::events::GraphEventError>
        {
            self.events.lock().push(event.clone());
            Ok(crate::events::GraphEventDisposition::Acknowledged)
        }
    }

    /// reg-cbfff6: a sink attached with `with_event_sink` receives each
    /// node's lifecycle while the graph runs, whichever execution path runs
    /// it: a start before the node's cell runs, then how the node settled. A
    /// node blocked by a failed one is skipped with the blocker in the reason.
    #[tokio::test(start_paused = true)]
    async fn the_event_sink_receives_node_lifecycle_events() {
        use crate::events::GraphExecutionEvent;

        let graph = load_from_str(
            r#"
[graph]
name = "lifecycle"

[graph.policy]
failure_strategy = "skip_failed"

[[nodes]]
id = "broken"
cell_type = "sleep"
config = { label = "broken", fail = true }

[[nodes]]
id = "child"
cell_type = "sleep"
config = { label = "child" }

[[nodes]]
id = "grandchild"
cell_type = "sleep"
config = { label = "grandchild" }

[[nodes]]
id = "other"
cell_type = "sleep"
config = { label = "other" }

[[edges]]
from = "broken"
to = "child"

[[edges]]
from = "child"
to = "grandchild"
"#,
        )
        .unwrap();

        for max_concurrent_nodes in [1, 4] {
            for through_start in [false, true] {
                let case =
                    format!("max_concurrent_nodes {max_concurrent_nodes}, start {through_start}");
                let mut graph = graph.clone();
                graph.policy.max_concurrent_nodes = max_concurrent_nodes;
                let log = Arc::new(SleepLog::default());
                let sink = Arc::new(RecordingGraphSink::default());
                let engine =
                    GraphEngine::new(graph, sleep_registry(&log)).with_event_sink(sink.clone());
                let output = if through_start {
                    engine
                        .start(CellContext::new())
                        .await_completion()
                        .await
                        .expect("flow output")
                } else {
                    engine.execute(&CellContext::new()).await.unwrap()
                };
                assert!(!output.success, "{case}");

                let events = sink.events.lock().clone();
                let seqs: Vec<u64> = events.iter().map(|event| event.common().seq).collect();
                assert!(
                    seqs.windows(2).all(|pair| pair[0] < pair[1]),
                    "{case}: {seqs:?}"
                );
                let lifecycle = |node_id: &str| -> Vec<&'static str> {
                    events
                        .iter()
                        .filter(|event| event.node().is_some_and(|node| node.node_id == node_id))
                        .map(GraphExecutionEvent::variant_name)
                        .collect()
                };
                assert_eq!(
                    lifecycle("other"),
                    ["NodeStarted", "NodeCompleted"],
                    "{case}"
                );
                assert_eq!(lifecycle("broken"), ["NodeStarted", "NodeFailed"], "{case}");
                for blocked in ["child", "grandchild"] {
                    assert_eq!(lifecycle(blocked), ["NodeSkipped"], "{case}: {blocked}");
                }
                let skip_reason = |node_id: &str| {
                    events.iter().find_map(|event| match event {
                        GraphExecutionEvent::NodeSkipped { node, reason, .. }
                            if node.node_id == node_id =>
                        {
                            Some(reason.clone())
                        }
                        _ => None,
                    })
                };
                assert!(
                    skip_reason("grandchild").is_some_and(|reason| reason.contains("'broken'")),
                    "{case}: {events:?}"
                );
                // A cell that is not a plan task has no task outcome.
                assert!(
                    events.iter().all(|event| !matches!(
                        event,
                        GraphExecutionEvent::NodeCompleted {
                            outcome: Some(_),
                            ..
                        }
                    )),
                    "{case}: {events:?}"
                );
            }
        }
    }

    /// reg-cbfff6: a completed plan task node carries the outcome its gate
    /// verdict earns, one with no verdict is unverified, and any other node
    /// has no task outcome.
    #[test]
    fn a_completed_task_node_carries_its_gate_outcome() {
        let mut signals =
            vec![roko_core::Signal::builder(roko_core::Kind::Custom("task.output".into())).build()];
        assert_eq!(
            completed_outcome(TASK_EXECUTOR_CELL_TYPE, &signals).as_deref(),
            Some("unverified")
        );
        assert_eq!(completed_outcome("sleep", &signals), None);
        TaskGateVerdict::Passed.stamp(&mut signals);
        assert_eq!(
            completed_outcome(TASK_EXECUTOR_CELL_TYPE, &signals).as_deref(),
            Some("passed")
        );
        TaskGateVerdict::PassedWithPreexistingFailures.stamp(&mut signals);
        assert_eq!(
            completed_outcome(TASK_EXECUTOR_CELL_TYPE, &signals).as_deref(),
            Some("passed_with_preexisting_failures")
        );
        TaskGateVerdict::ForcedAccept.stamp(&mut signals);
        assert_eq!(
            completed_outcome(TASK_EXECUTOR_CELL_TYPE, &signals).as_deref(),
            Some("accepted_with_failures")
        );
    }

    /// The dispatch stop halts a `SkipFailed` run the way a spent plan budget
    /// must: once it fires no further node starts, nodes already running
    /// finish, and every node that had not started is skipped with its
    /// reason.
    #[tokio::test(start_paused = true)]
    async fn a_dispatch_stop_starts_no_further_node() {
        let graph = load_from_str(
            r#"
[graph]
name = "dispatch-stop"

[graph.policy]
failure_strategy = "skip_failed"
max_concurrent_nodes = 3

[[nodes]]
id = "broken"
cell_type = "sleep"
config = { label = "broken", delay_ms = 10, fail = true }

[[nodes]]
id = "spender"
cell_type = "sleep"
config = { label = "spender", delay_ms = 50 }

[[nodes]]
id = "long"
cell_type = "sleep"
config = { label = "long", delay_ms = 200 }

[[nodes]]
id = "after-spender"
cell_type = "sleep"
config = { label = "after-spender" }

[[nodes]]
id = "after-long"
cell_type = "sleep"
config = { label = "after-long" }

[[edges]]
from = "spender"
to = "after-spender"

[[edges]]
from = "long"
to = "after-long"
"#,
        )
        .unwrap();

        for through_start in [false, true] {
            let log = Arc::new(SleepLog::default());
            let spent = Arc::clone(&log);
            let engine = GraphEngine::new(graph.clone(), sleep_registry(&log)).with_dispatch_stop(
                Arc::new(move || {
                    spent
                        .saw("spender:end")
                        .then(|| "plan budget exhausted".to_string())
                }),
            );
            let output = if through_start {
                engine
                    .start(CellContext::new())
                    .await_completion()
                    .await
                    .expect("flow output")
            } else {
                engine.execute(&CellContext::new()).await.unwrap()
            };

            assert!(!output.success, "through_start = {through_start}");
            assert_eq!(result_status(&output, "broken"), NodeStatus::Failed);
            assert_eq!(result_status(&output, "spender"), NodeStatus::Complete);
            assert_eq!(result_status(&output, "long"), NodeStatus::Complete);
            for waiting in ["after-spender", "after-long"] {
                let result = output
                    .node_results
                    .iter()
                    .find(|result| result.node_id == waiting)
                    .expect("result");
                assert_eq!(result.status, NodeStatus::Skipped);
                assert_eq!(result.error.as_deref(), Some("plan budget exhausted"));
                assert!(!log.saw(&format!("{waiting}:start")));
            }
        }
    }

    /// A sequential run (one node at a time) honours the dispatch stop too.
    #[tokio::test(start_paused = true)]
    async fn a_dispatch_stop_halts_a_sequential_run() {
        let graph = load_from_str(
            r#"
[graph]
name = "dispatch-stop-sequential"

[graph.policy]
failure_strategy = "skip_failed"
max_concurrent_nodes = 1

[[nodes]]
id = "first"
cell_type = "sleep"
config = { label = "first", delay_ms = 10 }

[[nodes]]
id = "second"
cell_type = "sleep"
config = { label = "second", delay_ms = 10 }

[[nodes]]
id = "third"
cell_type = "sleep"
config = { label = "third", delay_ms = 10 }

[[edges]]
from = "first"
to = "second"

[[edges]]
from = "second"
to = "third"
"#,
        )
        .unwrap();

        for through_start in [false, true] {
            let log = Arc::new(SleepLog::default());
            let spent = Arc::clone(&log);
            let engine = GraphEngine::new(graph.clone(), sleep_registry(&log)).with_dispatch_stop(
                Arc::new(move || {
                    spent
                        .saw("first:end")
                        .then(|| "plan budget exhausted".to_string())
                }),
            );
            let output = if through_start {
                engine
                    .start(CellContext::new())
                    .await_completion()
                    .await
                    .expect("flow output")
            } else {
                engine.execute(&CellContext::new()).await.unwrap()
            };

            assert!(!output.success, "through_start = {through_start}");
            assert_eq!(result_status(&output, "first"), NodeStatus::Complete);
            for waiting in ["second", "third"] {
                let result = output
                    .node_results
                    .iter()
                    .find(|result| result.node_id == waiting)
                    .expect("result");
                assert_eq!(result.status, NodeStatus::Skipped);
                assert_eq!(result.error.as_deref(), Some("plan budget exhausted"));
            }
            assert!(!log.saw("second:start"));
        }
    }

    // ─── gap-439794: exclusive paths ─────────────────────────────────────────

    /// Whether the [`SleepCell`] runs labelled `left` and `right` overlapped.
    fn ran_together(log: &SleepLog, left: &str, right: &str) -> bool {
        log.position(&format!("{left}:start")) < log.position(&format!("{right}:end"))
            && log.position(&format!("{right}:start")) < log.position(&format!("{left}:end"))
    }

    /// gap-439794: two ready nodes whose exclusive paths overlap never run at
    /// the same time, though a slot is free for each. `plan-view` writes a
    /// file inside the directory `stage` writes. The node that waits stays
    /// `Pending`, through `execute` (graph runs, Hot Graphs) and through
    /// `start` (plan runs).
    #[tokio::test(start_paused = true)]
    async fn same_wave_tasks_with_overlapping_files_are_serialized() {
        let graph = load_from_str(
            r#"
[graph]
name = "overlapping-files"

[graph.policy]
max_concurrent_nodes = 2

[[nodes]]
id = "stage"
cell_type = "sleep"
config = { label = "stage", delay_ms = 100 }
exclusive = ["web/src/stage"]

[[nodes]]
id = "plan-view"
cell_type = "sleep"
config = { label = "plan-view", delay_ms = 100 }
exclusive = ["./web/src/stage/PlanView.tsx"]
"#,
        )
        .unwrap();

        let log = Arc::new(SleepLog::default());
        let output = GraphEngine::new(graph.clone(), sleep_registry(&log))
            .execute(&CellContext::new())
            .await
            .unwrap();
        assert!(output.success);
        assert_eq!(
            log.max_running.load(Ordering::SeqCst),
            1,
            "{:?}",
            log.events()
        );
        assert!(
            !ran_together(&log, "stage", "plan-view"),
            "{:?}",
            log.events()
        );

        let log = Arc::new(SleepLog::default());
        let handle = GraphEngine::new(graph, sleep_registry(&log)).start(CellContext::new());
        tokio::time::sleep(Duration::from_millis(50)).await;
        let statuses = handle.status().node_statuses;
        let count = |status| statuses.values().filter(|s| **s == status).count();
        assert_eq!(count(NodeStatus::Running), 1, "{statuses:?}");
        assert_eq!(count(NodeStatus::Pending), 1, "{statuses:?}");
        let output = handle.await_completion().await.expect("flow output");
        assert!(output.success);
        assert_eq!(result_status(&output, "stage"), NodeStatus::Complete);
        assert_eq!(result_status(&output, "plan-view"), NodeStatus::Complete);
        assert_eq!(
            log.max_running.load(Ordering::SeqCst),
            1,
            "{:?}",
            log.events()
        );
        assert!(
            !ran_together(&log, "stage", "plan-view"),
            "{:?}",
            log.events()
        );
    }

    /// Nodes whose exclusive paths do not overlap still run side by side:
    /// the directory `src/app` does not hold the file `src/app.rs`.
    #[tokio::test(start_paused = true)]
    async fn same_wave_tasks_with_disjoint_files_run_in_parallel() {
        let graph = load_from_str(
            r#"
[graph]
name = "disjoint-files"

[graph.policy]
max_concurrent_nodes = 2

[[nodes]]
id = "app-dir"
cell_type = "sleep"
config = { label = "app-dir", delay_ms = 100 }
exclusive = ["src/app"]

[[nodes]]
id = "app-file"
cell_type = "sleep"
config = { label = "app-file", delay_ms = 100 }
exclusive = ["src/app.rs"]
"#,
        )
        .unwrap();

        for through_start in [false, true] {
            let log = Arc::new(SleepLog::default());
            let engine = GraphEngine::new(graph.clone(), sleep_registry(&log));
            let output = if through_start {
                engine
                    .start(CellContext::new())
                    .await_completion()
                    .await
                    .expect("flow output")
            } else {
                engine.execute(&CellContext::new()).await.unwrap()
            };

            assert!(output.success, "through_start = {through_start}");
            assert_eq!(
                log.max_running.load(Ordering::SeqCst),
                2,
                "{:?}",
                log.events()
            );
            assert!(
                ran_together(&log, "app-dir", "app-file"),
                "{:?}",
                log.events()
            );
        }
    }

    /// A node waiting for its exclusive paths holds no slot. `lib-a` and
    /// `lib-b` write the same file; with two slots, `docs` runs beside the
    /// one that starts first instead of queueing behind the one that waits.
    /// Both listing orders run, so `docs` comes last in topological order in
    /// one of them: there a waiting node that took a slot would block it.
    #[tokio::test(start_paused = true)]
    async fn a_node_waiting_for_its_files_holds_no_slot() {
        const HEADER: &str = r#"
[graph]
name = "waiting-holds-no-slot"

[graph.policy]
max_concurrent_nodes = 2
"#;
        const LIB_A: &str = r#"
[[nodes]]
id = "lib-a"
cell_type = "sleep"
config = { label = "lib-a", delay_ms = 1000 }
exclusive = ["src/lib.rs"]
"#;
        const LIB_B: &str = r#"
[[nodes]]
id = "lib-b"
cell_type = "sleep"
config = { label = "lib-b", delay_ms = 1000 }
exclusive = ["src/lib.rs"]
"#;
        const DOCS: &str = r#"
[[nodes]]
id = "docs"
cell_type = "sleep"
config = { label = "docs", delay_ms = 10 }
exclusive = ["docs/guide.md"]
"#;

        for listing in [[LIB_A, LIB_B, DOCS], [DOCS, LIB_B, LIB_A]] {
            let graph = load_from_str(&format!("{HEADER}{}", listing.concat())).unwrap();
            for through_start in [false, true] {
                let log = Arc::new(SleepLog::default());
                let engine = GraphEngine::new(graph.clone(), sleep_registry(&log));
                let output = if through_start {
                    engine
                        .start(CellContext::new())
                        .await_completion()
                        .await
                        .expect("flow output")
                } else {
                    engine.execute(&CellContext::new()).await.unwrap()
                };

                assert!(output.success, "through_start = {through_start}");
                assert!(!ran_together(&log, "lib-a", "lib-b"), "{:?}", log.events());
                let first_writer_end = log.position("lib-a:end").min(log.position("lib-b:end"));
                assert!(
                    log.position("docs:start") < first_writer_end,
                    "docs waited behind the writers: {:?}",
                    log.events()
                );
            }
        }
    }

    /// Exclusive paths order nodes but are not dependencies. Under
    /// `SkipFailed`, when the node holding the paths fails, the node that
    /// waited for them still runs, and nothing reports it as blocked.
    #[tokio::test(start_paused = true)]
    async fn a_file_conflict_is_not_a_dependency() {
        let graph = load_from_str(
            r#"
[graph]
name = "conflict-is-not-a-dependency"

[graph.policy]
failure_strategy = "skip_failed"
max_concurrent_nodes = 2

[[nodes]]
id = "broken"
cell_type = "sleep"
config = { label = "broken", delay_ms = 100, fail = true }
exclusive = ["src/lib.rs"]

[[nodes]]
id = "setup"
cell_type = "sleep"
config = { label = "setup", delay_ms = 10 }

[[nodes]]
id = "fixer"
cell_type = "sleep"
config = { label = "fixer", delay_ms = 10 }
exclusive = ["src/lib.rs"]

[[edges]]
from = "setup"
to = "fixer"
"#,
        )
        .unwrap();

        for through_start in [false, true] {
            let log = Arc::new(SleepLog::default());
            let engine = GraphEngine::new(graph.clone(), sleep_registry(&log));
            let output = if through_start {
                engine
                    .start(CellContext::new())
                    .await_completion()
                    .await
                    .expect("flow output")
            } else {
                engine.execute(&CellContext::new()).await.unwrap()
            };

            assert!(!output.success, "through_start = {through_start}");
            assert_eq!(result_status(&output, "broken"), NodeStatus::Failed);
            assert_eq!(result_status(&output, "fixer"), NodeStatus::Complete);
            let fixer = output
                .node_results
                .iter()
                .find(|result| result.node_id == "fixer")
                .expect("result");
            assert_eq!(fixer.blocked_by, None);
            assert!(
                log.position("broken:end") < log.position("fixer:start"),
                "fixer ran beside broken: {:?}",
                log.events()
            );
        }
    }

    // ─── gap-3006e9: ready and dispatch times ───────────────────────────────

    /// gap-3006e9: every node that runs records when it became ready and when
    /// it was dispatched, in its result and in the Activity log, so the time
    /// it waited for a slot can be measured. Three 100 ms roots compete for
    /// the slots, and `after-first` becomes ready only once `first` has
    /// finished. Covers the sequential loops (one slot) and the ready queue
    /// (two slots), through `execute` and through `start`. Real time: the
    /// stamps are wall-clock times.
    #[tokio::test]
    async fn tasks_record_when_they_became_ready_and_were_dispatched() {
        let graph = load_from_str(
            r#"
[graph]
name = "timing"

[[nodes]]
id = "first"
cell_type = "sleep"
config = { label = "first", delay_ms = 100 }

[[nodes]]
id = "second"
cell_type = "sleep"
config = { label = "second", delay_ms = 100 }

[[nodes]]
id = "third"
cell_type = "sleep"
config = { label = "third", delay_ms = 100 }

[[nodes]]
id = "after-first"
cell_type = "sleep"
config = { label = "after-first", delay_ms = 10 }

[[edges]]
from = "first"
to = "after-first"
"#,
        )
        .unwrap();

        for max_concurrent_nodes in [1_usize, 2] {
            for through_start in [false, true] {
                let case = format!("{max_concurrent_nodes} slots, start {through_start}");
                let mut graph = graph.clone();
                graph.policy.max_concurrent_nodes = max_concurrent_nodes;
                let dir = tempfile::tempdir().unwrap();
                let activities = dir.path().join("activities.jsonl");
                let log = Arc::new(SleepLog::default());
                let engine = GraphEngine::new(graph, sleep_registry(&log))
                    .with_recorder(ActivityRecorder::create("timing", &activities).unwrap());
                let output = if through_start {
                    engine
                        .start(CellContext::new())
                        .await_completion()
                        .await
                        .expect("flow output")
                } else {
                    engine.execute(&CellContext::new()).await.unwrap()
                };
                assert!(output.success, "{case}");

                let timing_of = |node_id: &str| {
                    output
                        .node_results
                        .iter()
                        .find(|result| result.node_id == node_id)
                        .map(|result| result.timing)
                        .unwrap_or_else(|| panic!("{case}: no result for {node_id}"))
                };
                for node_id in ["first", "second", "third", "after-first"] {
                    let node = timing_of(node_id);
                    assert!(node.ready_at_ms.is_some(), "{case}: {node_id} {node:?}");
                    assert!(
                        node.dispatched_at_ms >= node.ready_at_ms,
                        "{case}: {node_id} {node:?}"
                    );
                }

                // With every slot taken, the last root to start waited for
                // one 100 ms root per slot ahead of it.
                let last_root_wait = ["first", "second", "third"]
                    .into_iter()
                    .filter_map(|node_id| timing_of(node_id).slot_wait_ms())
                    .max()
                    .unwrap_or_default();
                let expected_wait = 100 * (3 - max_concurrent_nodes as u64);
                assert!(
                    last_root_wait + 10 >= expected_wait,
                    "{case}: the last root waited {last_root_wait} ms"
                );

                // `after-first` became ready only once `first` had finished.
                let (first, after_first) = (timing_of("first"), timing_of("after-first"));
                assert!(
                    after_first.ready_at_ms >= first.dispatched_at_ms.map(|at| at + 90),
                    "{case}: {first:?} {after_first:?}"
                );

                // The Activity log records the same times.
                let recorded = std::fs::read_to_string(&activities).unwrap();
                let entries: Vec<crate::replay::RecordEntry> = recorded
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .collect();
                assert_eq!(entries.len(), 4, "{case}");
                for entry in entries {
                    let expected = timing_of(&entry.node_id);
                    assert_eq!(entry.ready_at_ms, expected.ready_at_ms, "{case}");
                    assert_eq!(entry.dispatched_at_ms, expected.dispatched_at_ms, "{case}");
                }
            }
        }
    }

    /// Waits until its context reports its run cancelled, then succeeds,
    /// recording that it saw the cancellation.
    struct WaitsForCancelCell {
        started: Arc<std::sync::atomic::AtomicBool>,
        saw_cancel: Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait::async_trait]
    impl Cell for WaitsForCancelCell {
        fn cell_id(&self) -> &str {
            "waits-for-cancel"
        }

        fn cell_name(&self) -> &str {
            "WaitsForCancelCell"
        }

        async fn execute(
            &self,
            input: Vec<roko_core::Signal>,
            ctx: &CellContext,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            self.started.store(true, Ordering::SeqCst);
            for _ in 0..500 {
                if ctx.is_cancelled() {
                    self.saw_cancel.store(true, Ordering::SeqCst);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Ok(input)
        }
    }

    /// bug-ceb581: `FlowHandle::cancel` reaches a cell that is already
    /// running, through its context, so it can start no further work.
    #[tokio::test]
    async fn flow_cancel_reaches_a_running_cell() {
        let graph = load_from_str(
            r#"
[graph]
name = "cancel-reaches-cell"

[[nodes]]
id = "waiting"
cell_type = "waits-for-cancel"
"#,
        )
        .unwrap();
        let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let saw_cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut registry = noop_registry();
        let cell_started = Arc::clone(&started);
        let cell_saw_cancel = Arc::clone(&saw_cancel);
        registry.register("waits-for-cancel", move |_| {
            Box::new(WaitsForCancelCell {
                started: Arc::clone(&cell_started),
                saw_cancel: Arc::clone(&cell_saw_cancel),
            })
        });

        let flow = GraphEngine::new(graph, registry).start(CellContext::new());
        for _ in 0..500 {
            if started.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(started.load(Ordering::SeqCst), "the cell is running");
        flow.cancel();
        let _output = flow.await_completion().await.expect("flow output");
        assert!(
            saw_cancel.load(Ordering::SeqCst),
            "the running cell saw the cancellation"
        );
    }
}
