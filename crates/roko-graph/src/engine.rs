//! Graph execution engine: conditional sequential/parallel execution of Cell DAGs.
//!
//! The `GraphEngine` takes a `Graph` and a `CellRegistry`, topologically sorts
//! the nodes, and executes Cells sequentially or in bounded topological waves.
//! Only active conditional edges contribute upstream outputs to downstream Cells.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use petgraph::visit::EdgeRef as _;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use roko_core::{ContentHash, LensScope, ObservableEvent, TelemetryEventSink};

use crate::cell::{Cell, CellContext};
use crate::registry::CellRegistry;
use crate::replay::{ActivityRecorder, ActivityReplayer};
use crate::topo::{topological_order, topological_waves};
use crate::types::{EdgeCondition, ExecutionClass, Graph, GraphError, GraphPolicy, NodeId};

// ─── MergeEnqueuer trait ────────────────────────────────────────────────────

// MergeRequest and MergeEnqueuer are now defined in delivery.rs. Re-export
// them here for backward compatibility with existing callers.
pub use crate::delivery::{MergeEnqueuer, MergeRequest};

// ─── GraphSnapshot ──────────────────────────────────────────────────────────

/// Serializable snapshot of a graph execution in progress or completed (v2).
///
/// Captures per-node status, Activity node outputs, policy, budget state, and
/// a stable graph fingerprint so the engine can be resumed safely. Only
/// Activity node outputs are included -- Workflow node outputs are re-derived
/// on resume.
///
/// V2 adds `schema_version`, `graph_fingerprint`, budget tracking fields, and
/// `last_event_seq` for monotonic event replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshotV2 {
    /// On-disk schema version. Always `2` for this struct.
    #[serde(default = "default_snapshot_schema_version")]
    pub schema_version: u8,
    /// Name of the graph.
    pub graph_name: String,
    /// Graph ID (from metadata).
    pub graph_id: String,
    /// Stable BLAKE3 fingerprint of the execution-relevant graph definition.
    /// Used to reject resume after graph definition or policy drift.
    #[serde(default)]
    pub graph_fingerprint: String,
    /// Per-node execution status at snapshot time.
    pub node_statuses: HashMap<String, SerializableNodeStatus>,
    /// Activity node outputs. Workflow nodes are excluded (re-derived on resume).
    pub node_outputs: HashMap<String, Vec<SerializableSignal>>,
    /// Hot Graph tick count at snapshot time.
    pub tick_count: u64,
    /// Cumulative budget spent in micro-USD (1 USD = 1_000_000).
    #[serde(default)]
    pub budget_spent_micro_usd: u64,
    /// Budget reserved but not yet settled in micro-USD.
    #[serde(default)]
    pub budget_reserved_micro_usd: u64,
    /// Monotonic event sequence number at snapshot time. Replay must not emit
    /// events with sequence numbers at or below this value.
    #[serde(default)]
    pub last_event_seq: u64,
    /// Unix milliseconds when the snapshot was captured.
    pub created_at_ms: i64,
    /// Graph policy preserved for resume.
    pub policy: GraphPolicy,
}

/// Current schema version for [`GraphSnapshotV2`].
pub const GRAPH_SNAPSHOT_SCHEMA_VERSION: u8 = 2;

fn default_snapshot_schema_version() -> u8 {
    GRAPH_SNAPSHOT_SCHEMA_VERSION
}

/// Primary snapshot type. Callers use this alias; the underlying versioned
/// struct name is kept for migration clarity.
pub type GraphSnapshot = GraphSnapshotV2;

/// Serializable node status (mirrors [`NodeStatus`] but with serde support).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializableNodeStatus {
    /// Not yet started.
    Pending,
    /// Currently executing (treated as Pending on resume).
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

/// Lightweight serializable signal reference for snapshots.
///
/// Full [`roko_core::Signal`] is already serde-compatible, but we wrap the
/// JSON representation to keep the snapshot format stable even if Signal
/// internals change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableSignal {
    /// JSON-serialized signal.
    pub json: serde_json::Value,
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
    UpstreamFailed(String),
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
    /// The background task will stop after the currently-executing node
    /// completes. Already-started nodes are not interrupted.
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

/// The graph execution engine. Holds a graph and registry, executing nodes
/// sequentially or in bounded parallel topological waves according to policy.
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
        let mut fail_fast_abort = false;
        let mut resumed_emitted = false;

        // 3. Execute each node in order
        for node_id in &order {
            // SAFETY: topological_order only returns IDs that are in the graph.
            let Some(node) = self.graph.get_node(node_id) else {
                continue;
            };

            if fail_fast_abort {
                let result = NodeResult {
                    node_id: node_id.clone(),
                    cell_type: node.cell_type.clone(),
                    status: NodeStatus::Skipped,
                    duration: Duration::ZERO,
                    error: Some("aborted after graph failure".to_string()),
                    output_count: 0,
                    is_stub: false,
                };
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
                    };
                    statuses.insert(node_id.clone(), result.status);
                    results.push(result);
                    continue;
                }
                NodeActivation::UpstreamFailed(reason) => {
                    let result = NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Skipped,
                        duration: Duration::ZERO,
                        error: Some(reason),
                        output_count: 0,
                        is_stub: false,
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
                });
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

            info!(node_id = %node_id, cell_type = %node.cell_type, "executing node");
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
                    if is_activity
                        && let Some(recorder) = &self.recorder
                        && let Err(error) = recorder.lock().record(
                            &graph_name,
                            node_id,
                            tick,
                            output_signals.clone(),
                        )
                    {
                        return Err(GraphError::NodeFailed {
                            node_id: node_id.clone(),
                            reason: format!("persist Activity checkpoint: {error}"),
                        });
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
                    fail_fast_abort = matches!(
                        self.graph.policy.failure_strategy,
                        crate::types::FailureStrategy::FailFast
                    );
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
                    });
                }
            }
        }

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
        })
    }

    /// Execute the graph with parallel node execution within topological waves.
    ///
    /// Validates all edges for type-schema compatibility before executing any
    /// node.
    ///
    /// Nodes are grouped into waves using [`topological_waves`]. Within each
    /// wave, nodes execute concurrently via `tokio::task::JoinSet`, limited by
    /// [`GraphPolicy::max_concurrent_nodes`] through a [`tokio::sync::Semaphore`].
    ///
    /// Between waves, execution is sequential: wave N+1 only starts after all
    /// nodes in wave N have completed. If any node fails and the failure
    /// strategy is `FailFast`, remaining waves are skipped.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeValidationFailed` if edges have incompatible schemas,
    /// `GraphError::CycleDetected` if the graph contains a cycle, or
    /// `GraphError::UnknownCellType` if a node references an unregistered cell type.
    #[allow(clippy::too_many_lines)]
    pub async fn execute_parallel(&self, ctx: &CellContext) -> Result<GraphOutput, GraphError> {
        self.execute_parallel_at_tick(ctx, 0).await
    }

    /// Execute bounded parallel topological waves at a specific Hot Graph tick.
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
    #[allow(clippy::too_many_lines)]
    async fn execute_parallel_at_tick_validated(
        &self,
        ctx: &CellContext,
        tick: u64,
    ) -> Result<GraphOutput, GraphError> {
        use tokio::task::JoinSet;

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
        let max_concurrent = self.graph.policy.max_concurrent_nodes.max(1);

        // 1. Compute waves
        let waves = topological_waves(&self.graph)?;

        // 2. Track outputs and failures
        let outputs: Arc<parking_lot::Mutex<HashMap<NodeId, Vec<roko_core::Signal>>>> =
            Arc::new(parking_lot::Mutex::new(self.initial_tick_outputs()));
        let statuses: Arc<parking_lot::Mutex<HashMap<NodeId, NodeStatus>>> =
            Arc::new(parking_lot::Mutex::new(HashMap::new()));
        let failed_nodes: Arc<parking_lot::Mutex<HashSet<NodeId>>> =
            Arc::new(parking_lot::Mutex::new(HashSet::new()));
        let mut results: Vec<NodeResult> = Vec::new();
        let mut resumed_emitted = false;

        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        let mut total_cost_usd = 0.0;

        // 3. Execute wave by wave
        for wave in &waves {
            let mut join_set: JoinSet<(NodeResult, f64, Vec<roko_core::Signal>, bool)> =
                JoinSet::new();
            for node_id in wave {
                let Some(node) = self.graph.get_node(node_id) else {
                    continue;
                };

                let activation = {
                    let status_guard = statuses.lock();
                    let output_guard = outputs.lock();
                    evaluate_node_activation(&self.graph, node_id, &status_guard, &output_guard)
                };
                let input = match activation {
                    NodeActivation::Root => {
                        let output_guard = outputs.lock();
                        self.root_tick_inputs(node_id, &output_guard)
                    }
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
                        };
                        statuses.lock().insert(node_id.clone(), result.status);
                        results.push(result);
                        continue;
                    }
                    NodeActivation::UpstreamFailed(reason) => {
                        let result = NodeResult {
                            node_id: node_id.clone(),
                            cell_type: node.cell_type.clone(),
                            status: NodeStatus::Skipped,
                            duration: Duration::ZERO,
                            error: Some(reason),
                            output_count: 0,
                            is_stub: false,
                        };
                        statuses.lock().insert(node_id.clone(), result.status);
                        results.push(result);
                        continue;
                    }
                };
                let input_hash = input_signal_hash(&input);
                let is_activity = node.execution_class == ExecutionClass::Activity;

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
                    outputs.lock().insert(node_id.clone(), recorded);
                    statuses
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
                    });
                    continue;
                }

                // Instantiate cell from registry
                let cell: Arc<dyn Cell> = self
                    .registry
                    .create(&node.cell_type, node.config.clone())?
                    .into();

                let sem = semaphore.clone();
                let node_id = node_id.clone();
                let cell_type = node.cell_type.clone();
                let cell_is_stub = cell.is_stub();
                let ctx = ctx.clone();
                let graph_name = graph_name.clone();
                let run_id = run_id.clone();
                let telemetry = self.telemetry.clone();
                let estimated_cost_usd = cell.estimated_cost().unwrap_or_default();
                let max_retries = max_retries(&self.graph.policy);
                statuses.lock().insert(node_id.clone(), NodeStatus::Running);

                join_set.spawn(async move {
                    let Ok(_permit) = sem.acquire().await else {
                        return (
                            NodeResult {
                                node_id: node_id.clone(),
                                cell_type,
                                status: NodeStatus::Failed,
                                duration: Duration::ZERO,
                                error: Some("semaphore closed".into()),
                                output_count: 0,
                                is_stub: cell_is_stub,
                            },
                            0.0,
                            Vec::new(),
                            is_activity,
                        );
                    };

                    let ancestry = [
                        LensScope::Cell(node_id.clone()),
                        LensScope::Graph(graph_name.clone()),
                    ];
                    emit_telemetry_to(
                        telemetry.as_ref(),
                        &ObservableEvent::CellStarted {
                            block: node_id.clone(),
                            run: run_id.clone(),
                            input_hash,
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
                    let attempt_cost = estimated_cost_usd * f64::from(attempts);
                    let (result, output_signals) = match execution {
                        Ok(output_signals) => {
                            let duration = node_start.elapsed();
                            let duration_ms = duration_ms(duration);
                            let count = output_signals.len();
                            emit_telemetry_to(
                                telemetry.as_ref(),
                                &ObservableEvent::CellCompleted {
                                    block: node_id.clone(),
                                    run: run_id.clone(),
                                    duration_ms,
                                    cost_usd: attempt_cost,
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
                            (
                                NodeResult {
                                    node_id: node_id.clone(),
                                    cell_type,
                                    status: NodeStatus::Complete,
                                    duration,
                                    error: None,
                                    output_count: count,
                                    is_stub: cell_is_stub,
                                },
                                output_signals,
                            )
                        }
                        Err(e) => {
                            let duration = node_start.elapsed();
                            let error = e.to_string();
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
                            (
                                NodeResult {
                                    node_id: node_id.clone(),
                                    cell_type,
                                    status: NodeStatus::Failed,
                                    duration,
                                    error: Some(error),
                                    output_count: 0,
                                    is_stub: cell_is_stub,
                                },
                                Vec::new(),
                            )
                        }
                    };
                    (result, attempt_cost, output_signals, is_activity)
                });
            }

            // Await all tasks in this wave
            while let Some(join_result) = join_set.join_next().await {
                match join_result {
                    Ok((node_result, attempt_cost, output_signals, is_activity)) => {
                        total_cost_usd += attempt_cost;
                        if node_result.status == NodeStatus::Failed {
                            failed_nodes.lock().insert(node_result.node_id.clone());
                            statuses
                                .lock()
                                .insert(node_result.node_id.clone(), NodeStatus::Failed);
                        } else if node_result.status == NodeStatus::Complete {
                            if is_activity
                                && let Some(recorder) = &self.recorder
                                && let Err(error) = recorder.lock().record(
                                    &graph_name,
                                    &node_result.node_id,
                                    tick,
                                    output_signals.clone(),
                                )
                            {
                                return Err(GraphError::NodeFailed {
                                    node_id: node_result.node_id,
                                    reason: format!("persist Activity checkpoint: {error}"),
                                });
                            }
                            outputs
                                .lock()
                                .insert(node_result.node_id.clone(), output_signals);
                            statuses
                                .lock()
                                .insert(node_result.node_id.clone(), NodeStatus::Complete);
                        }
                        results.push(node_result);
                    }
                    Err(join_err) => {
                        warn!(error = %join_err, "parallel node task panicked");
                    }
                }
            }

            // Check if we need to abort (FailFast with any failure in this wave)
            if matches!(
                self.graph.policy.failure_strategy,
                crate::types::FailureStrategy::FailFast
            ) && failed_nodes.lock().iter().next().is_some()
            {
                // Mark remaining waves as skipped
                for remaining_wave in waves.iter().skip_while(|w| *w != wave).skip(1) {
                    for node_id in remaining_wave {
                        if let Some(node) = self.graph.get_node(node_id) {
                            let result = NodeResult {
                                node_id: node_id.clone(),
                                cell_type: node.cell_type.clone(),
                                status: NodeStatus::Skipped,
                                duration: Duration::ZERO,
                                error: Some("aborted: upstream wave had failure".to_string()),
                                output_count: 0,
                                is_stub: false,
                            };
                            statuses.lock().insert(node_id.clone(), result.status);
                            results.push(result);
                        }
                    }
                }
                break;
            }
        }

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
            let files_changed = Self::collect_files_changed(&outputs.lock());
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

        self.persist_tick_outputs(&outputs.lock());

        Ok(GraphOutput {
            graph_name,
            success,
            node_results: results,
            total_duration,
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

        let graph_fingerprint = crate::fingerprint::graph_execution_fingerprint(&self.graph)
            .unwrap_or_default();

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

        for node_id in &order {
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
                    });
                    continue;
                }
                NodeActivation::UpstreamFailed(reason) => {
                    statuses.insert(node_id.clone(), NodeStatus::Skipped);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Skipped,
                        duration: Duration::ZERO,
                        error: Some(reason),
                        output_count: 0,
                        is_stub: false,
                    });
                    continue;
                }
            };

            // Re-execute pending nodes and all Workflow nodes.
            let cell: Box<dyn Cell> = registry.create(&node.cell_type, node.config.clone())?;
            let cell_is_stub = cell.is_stub();

            info!(node_id = %node_id, cell_type = %node.cell_type, "resume: executing node");
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
    /// Respects the cancellation token -- stops after the current node if cancelled.
    #[allow(clippy::too_many_lines)] // Keep status transitions adjacent to graph execution.
    async fn execute_with_status_tracking(
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
        let mut total_cost_usd = 0.0;
        let mut was_cancelled = false;
        let mut fail_fast_abort = false;

        // Seed all nodes as Pending.
        {
            let mut statuses = node_statuses.lock();
            for node_id in &order {
                statuses.insert(node_id.clone(), NodeStatus::Pending);
            }
        }

        for node_id in &order {
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

            if fail_fast_abort {
                node_statuses
                    .lock()
                    .insert(node_id.clone(), NodeStatus::Skipped);
                results.push(NodeResult {
                    node_id: node_id.clone(),
                    cell_type: node.cell_type.clone(),
                    status: NodeStatus::Skipped,
                    duration: Duration::ZERO,
                    error: Some("aborted after graph failure".to_string()),
                    output_count: 0,
                    is_stub: false,
                });
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
                        });
                        continue;
                    }
                    NodeActivation::UpstreamFailed(reason) => {
                        drop(statuses);
                        node_statuses
                            .lock()
                            .insert(node_id.clone(), NodeStatus::Skipped);
                        results.push(NodeResult {
                            node_id: node_id.clone(),
                            cell_type: node.cell_type.clone(),
                            status: NodeStatus::Skipped,
                            duration: Duration::ZERO,
                            error: Some(reason),
                            output_count: 0,
                            is_stub: false,
                        });
                        continue;
                    }
                }
            };

            node_statuses
                .lock()
                .insert(node_id.clone(), NodeStatus::Running);

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

            info!(node_id = %node_id, cell_type = %node.cell_type, "flow: executing node");
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
                    fail_fast_abort = matches!(
                        self.graph.policy.failure_strategy,
                        crate::types::FailureStrategy::FailFast
                    );
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Failed,
                        duration,
                        error: Some(msg),
                        output_count: 0,
                        is_stub: cell_is_stub,
                    });
                }
            }
        }

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

    /// Emit a graph execution event to the optional sink (#246).
    ///
    /// This is the single emission path shared by sequential, parallel,
    /// `start()`, resume, and Hot Graph. It does NOT replace telemetry;
    /// the engine emits to both sinks independently.
    async fn emit_graph_event(&self, event: &crate::events::GraphExecutionEvent) {
        crate::events::emit_graph_event(self.event_sink.as_ref(), event).await;
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
        return NodeActivation::UpstreamFailed(format!("node `{node_id}` is not in the graph"));
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
                    return NodeActivation::UpstreamFailed(format!(
                        "required dependency `{source_id}` did not complete"
                    ));
                }
                NodeStatus::Pending | NodeStatus::Running => {
                    return NodeActivation::UpstreamFailed(format!(
                        "required dependency `{source_id}` is not complete"
                    ));
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

    registry.register_with_descriptor(
        "noop",
        CellDescriptor {
            id: "noop".to_string(),
            version: (0, 1, 0),
            input_schema: None,
            output_schema: None,
            is_stub: true,
        },
        |_config| Box::new(NoopCell::default()),
    );

    // Cognitive loop cells (E22-T01): real typed Cell implementations
    // with explicit CellDescriptors for side-effect-free edge validation.

    registry.register_with_descriptor(
        "sense",
        CellDescriptor::new("sense", (0, 1, 0), None, Some(TypeSchema::OfKind(Kind::AgentMessage))),
        |_config| Box::new(crate::cells::cognitive::SenseCell::new()),
    );
    registry.register_with_descriptor(
        "assess",
        CellDescriptor::new(
            "assess",
            (0, 1, 0),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
        ),
        |_config| Box::new(crate::cells::cognitive::AssessCell::new()),
    );
    // "score" is an alias for "assess" in legacy graph definitions.
    registry.register_with_descriptor(
        "score",
        CellDescriptor::new(
            "score",
            (0, 1, 0),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
        ),
        |_config| Box::new(crate::cells::cognitive::AssessCell::new()),
    );
    registry.register_with_descriptor(
        "compose",
        CellDescriptor::new(
            "compose",
            (0, 1, 0),
            Some(TypeSchema::OfKind(Kind::AgentMessage)),
            Some(TypeSchema::OfKind(Kind::Prompt)),
        ),
        |_config| Box::new(crate::cells::cognitive::CognitiveComposeCell::new()),
    );
    registry.register_with_descriptor(
        "act",
        CellDescriptor::new(
            "act",
            (0, 1, 0),
            Some(TypeSchema::OfKind(Kind::Prompt)),
            Some(TypeSchema::OfKind(Kind::Episode)),
        ),
        |_config| Box::new(crate::cells::cognitive::ActCell::new()),
    );
    registry.register_with_descriptor(
        "verify",
        CellDescriptor::new(
            "verify",
            (0, 1, 0),
            Some(TypeSchema::OfKind(Kind::Episode)),
            Some(TypeSchema::OfKind(Kind::GateVerdict)),
        ),
        |_config| Box::new(crate::cells::cognitive::VerifyCell::new()),
    );
    registry.register_with_descriptor(
        "persist",
        CellDescriptor::new("persist", (0, 1, 0), Some(TypeSchema::OfKind(Kind::GateVerdict)), None),
        |_config| Box::new(crate::cells::cognitive::PersistCell::new()),
    );
    registry.register("react", |_config| {
        Box::new(crate::cells::cognitive::ReactCell::new())
    });

    // Task executor cell for plan-to-graph converted tasks (task 101).
    registry.register("task-executor", |config| {
        Box::new(crate::cells::task_executor::TaskExecutorCell::unconfigured(
            config,
        ))
    });

    // Legacy cognitive loop stub aliases -- keep PassthroughCell stubs for
    // graph definitions that still reference old names (signal-reader, etc.).
    for name in crate::cells::stubs::COGNITIVE_LOOP_STUBS {
        let cell_name = (*name).to_string();
        let desc = CellDescriptor {
            id: cell_name.clone(),
            version: (0, 1, 0),
            input_schema: None,
            output_schema: None,
            is_stub: true,
        };
        registry.register_with_descriptor(name, desc, move |_config| {
            Box::new(crate::cells::stubs::PassthroughCell::new(cell_name.clone()))
        });
    }

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
                |_| Box::new(NoopCell::with_id_and_name("agent-msg-source", "AgentMsgSource")),
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
            graph.add_node(make_node("src", "agent-msg-source")).unwrap();
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
            graph.add_node(make_node("src", "agent-msg-source")).unwrap();
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
            graph.add_node(make_node("my-source", "agent-msg-source")).unwrap();
            graph.add_node(make_node("my-target", "episode-sink")).unwrap();
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
            graph.add_node(make_node("src", "agent-msg-source")).unwrap();
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
        fn graph_validation_test_stub_descriptor() {
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
            // Stub descriptors have no schemas, so edge validation passes.
            assert!(engine.validate_for_start().is_ok());
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
            for (from, to) in [
                ("s", "a"),
                ("a", "c"),
                ("c", "x"),
                ("x", "v"),
                ("v", "p"),
            ] {
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
        let deserialized: GraphSnapshotV2 =
            serde_json::from_str(&json).expect("deserialize");

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

        let snap: GraphSnapshotV2 =
            serde_json::from_value(json).expect("deserialize old format");

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
}
