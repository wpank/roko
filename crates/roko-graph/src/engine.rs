//! Graph execution engine: sequential topological execution of cell DAGs.
//!
//! The `GraphEngine` takes a `Graph` and a `CellRegistry`, topologically sorts
//! the nodes, and executes each cell sequentially. Outputs from upstream nodes
//! are passed as inputs to downstream nodes via an internal context map.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::cell::{Cell, CellContext};
use crate::registry::CellRegistry;
use crate::replay::{ActivityRecorder, ActivityReplayer};
use crate::topo::topological_order;
use crate::types::{ExecutionClass, Graph, GraphError, NodeId};

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
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Complete => write!(f, "complete"),
            Self::Failed => write!(f, "FAILED"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
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
    /// Error message if status is Failed.
    pub error: Option<String>,
    /// Number of output engrams produced.
    pub output_count: usize,
}

/// Output of a full graph execution.
#[derive(Debug, Clone)]
pub struct GraphOutput {
    /// Name of the graph that was executed.
    pub graph_name: String,
    /// Whether the entire graph completed successfully (all nodes Complete).
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
            let dur = if result.duration > Duration::ZERO {
                format!(" ({:?})", result.duration)
            } else {
                String::new()
            };
            let _ = writeln!(
                s,
                "  [{:>8}] {} ({}){}",
                result.status, result.node_id, result.cell_type, dur
            );
            if let Some(err) = &result.error {
                let _ = writeln!(s, "             error: {err}");
            }
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

/// The graph execution engine. Holds a graph and registry, executes nodes
/// sequentially in topological order.
pub struct GraphEngine {
    graph: Graph,
    registry: CellRegistry,
    /// Optional recorder — when present, Activity node outputs are appended to
    /// a JSONL file after each successful execution.
    recorder: Option<parking_lot::Mutex<ActivityRecorder>>,
    /// Optional replayer — when present, Activity node outputs are read from
    /// the JSONL file instead of re-executing the cell.
    replayer: Option<ActivityReplayer>,
}

impl GraphEngine {
    /// Create a new engine for the given graph and cell registry.
    #[must_use]
    pub fn new(graph: Graph, registry: CellRegistry) -> Self {
        Self {
            graph,
            registry,
            recorder: None,
            replayer: None,
        }
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

    /// Execute the graph sequentially in topological order.
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
    /// Returns `GraphError::CycleDetected` if the graph contains a cycle, or
    /// `GraphError::UnknownCellType` if a node references an unregistered cell type.
    pub async fn execute(&self, ctx: &CellContext) -> Result<GraphOutput, GraphError> {
        // tick = 0 for one-shot (non-Hot) graph executions.
        self.execute_at_tick(ctx, 0).await
    }

    /// Execute the graph at a specific tick index.
    ///
    /// Used internally by [`GraphEngine::execute`] (tick 0) and by Hot Graph
    /// tick loops (tick N). The tick is threaded through to the recorder/replayer
    /// so multi-tick runs can store and retrieve per-tick Activity outputs.
    #[allow(clippy::too_many_lines)]
    pub async fn execute_at_tick(
        &self,
        ctx: &CellContext,
        tick: u64,
    ) -> Result<GraphOutput, GraphError> {
        let start = Instant::now();
        let graph_name = self.graph.metadata.name.clone();

        // 1. Topological sort
        let order = topological_order(&self.graph)?;

        // 2. Track outputs per node and failed-set for skip propagation
        let mut outputs: HashMap<NodeId, Vec<roko_core::Engram>> = HashMap::new();
        let mut failed_nodes: HashSet<NodeId> = HashSet::new();
        let mut results: Vec<NodeResult> = Vec::with_capacity(order.len());

        // 3. Execute each node in order
        for node_id in &order {
            // SAFETY: topological_order only returns IDs that are in the graph.
            let Some(node) = self.graph.get_node(node_id) else {
                continue;
            };

            // Check if any upstream dependency failed -> skip
            if self.has_failed_ancestor(node_id, &failed_nodes) {
                results.push(NodeResult {
                    node_id: node_id.clone(),
                    cell_type: node.cell_type.clone(),
                    status: NodeStatus::Skipped,
                    duration: Duration::ZERO,
                    error: Some("upstream dependency failed".to_string()),
                    output_count: 0,
                });
                failed_nodes.insert(node_id.clone());
                continue;
            }

            let is_activity = node.execution_class == ExecutionClass::Activity;

            // For Activity nodes: check replayer for a pre-recorded result.
            if is_activity {
                if let Some(replayer) = &self.replayer {
                    if let Some(recorded) = replayer.lookup(node_id, tick) {
                        let count = recorded.len();
                        info!(
                            node_id = %node_id,
                            tick,
                            outputs = count,
                            "replay: substituting recorded Activity output"
                        );
                        outputs.insert(node_id.clone(), recorded.clone());
                        results.push(NodeResult {
                            node_id: node_id.clone(),
                            cell_type: node.cell_type.clone(),
                            status: NodeStatus::Complete,
                            duration: Duration::ZERO,
                            error: None,
                            output_count: count,
                        });
                        continue;
                    }
                }
            }

            // Instantiate cell from registry
            let cell: Box<dyn Cell> = self.registry.create(&node.cell_type, node.config.clone())?;

            // Gather inputs from upstream nodes
            let input = self.gather_inputs(node_id, &outputs);

            info!(node_id = %node_id, cell_type = %node.cell_type, "executing node");
            let node_start = Instant::now();

            // Execute the cell
            match cell.execute(input, ctx).await {
                Ok(output_engrams) => {
                    let duration = node_start.elapsed();
                    let count = output_engrams.len();
                    info!(
                        node_id = %node_id,
                        outputs = count,
                        duration_ms = duration.as_millis(),
                        "node complete"
                    );

                    // For Activity nodes: record the output if a recorder is present.
                    if is_activity {
                        if let Some(recorder) = &self.recorder {
                            if let Err(e) = recorder.lock().record(
                                &graph_name,
                                node_id,
                                tick,
                                output_engrams.clone(),
                            ) {
                                warn!(
                                    node_id = %node_id,
                                    error = %e,
                                    "replay recorder: failed to write entry"
                                );
                            }
                        }
                    }

                    outputs.insert(node_id.clone(), output_engrams);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Complete,
                        duration,
                        error: None,
                        output_count: count,
                    });
                }
                Err(e) => {
                    let duration = node_start.elapsed();
                    let msg = e.to_string();
                    warn!(
                        node_id = %node_id,
                        error = %msg,
                        duration_ms = duration.as_millis(),
                        "node failed"
                    );
                    failed_nodes.insert(node_id.clone());
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Failed,
                        duration,
                        error: Some(msg),
                        output_count: 0,
                    });
                }
            }
        }

        let total_duration = start.elapsed();
        let success = results.iter().all(|r| r.status == NodeStatus::Complete);

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
    /// This is the async alternative to [`GraphEngine::execute`]. The caller
    /// receives a handle while execution continues in the background. Use
    /// [`FlowHandle::await_completion`] to wait for the final result, or
    /// [`FlowHandle::cancel`] to request early termination.
    ///
    /// A unique `run_id` is generated automatically using a random UUID-like
    /// string derived from the current timestamp and a counter.
    pub fn start(self, ctx: CellContext) -> FlowHandle {
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
    async fn execute_with_status_tracking(
        &self,
        ctx: &CellContext,
        node_statuses: &Arc<parking_lot::Mutex<HashMap<NodeId, NodeStatus>>>,
        cancel: &CancellationToken,
    ) -> Result<GraphOutput, GraphError> {
        let start = Instant::now();

        let order = topological_order(&self.graph)?;

        let mut outputs: HashMap<NodeId, Vec<roko_core::Engram>> = HashMap::new();
        let mut failed_nodes: HashSet<NodeId> = HashSet::new();
        let mut results: Vec<NodeResult> = Vec::with_capacity(order.len());

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
                break;
            }

            let Some(node) = self.graph.get_node(node_id) else {
                continue;
            };

            if self.has_failed_ancestor(node_id, &failed_nodes) {
                node_statuses
                    .lock()
                    .insert(node_id.clone(), NodeStatus::Skipped);
                results.push(NodeResult {
                    node_id: node_id.clone(),
                    cell_type: node.cell_type.clone(),
                    status: NodeStatus::Skipped,
                    duration: Duration::ZERO,
                    error: Some("upstream dependency failed".to_string()),
                    output_count: 0,
                });
                failed_nodes.insert(node_id.clone());
                continue;
            }

            node_statuses
                .lock()
                .insert(node_id.clone(), NodeStatus::Running);

            let cell: Box<dyn Cell> = self.registry.create(&node.cell_type, node.config.clone())?;
            let input = self.gather_inputs(node_id, &outputs);

            info!(node_id = %node_id, cell_type = %node.cell_type, "flow: executing node");
            let node_start = Instant::now();

            match cell.execute(input, ctx).await {
                Ok(output_engrams) => {
                    let duration = node_start.elapsed();
                    let count = output_engrams.len();
                    node_statuses
                        .lock()
                        .insert(node_id.clone(), NodeStatus::Complete);
                    outputs.insert(node_id.clone(), output_engrams);
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Complete,
                        duration,
                        error: None,
                        output_count: count,
                    });
                }
                Err(e) => {
                    let duration = node_start.elapsed();
                    let msg = e.to_string();
                    warn!(node_id = %node_id, error = %msg, "flow: node failed");
                    node_statuses
                        .lock()
                        .insert(node_id.clone(), NodeStatus::Failed);
                    failed_nodes.insert(node_id.clone());
                    results.push(NodeResult {
                        node_id: node_id.clone(),
                        cell_type: node.cell_type.clone(),
                        status: NodeStatus::Failed,
                        duration,
                        error: Some(msg),
                        output_count: 0,
                    });
                }
            }
        }

        let total_duration = start.elapsed();
        let success = results.iter().all(|r| r.status == NodeStatus::Complete);

        Ok(GraphOutput {
            graph_name: self.graph.metadata.name.clone(),
            success,
            node_results: results,
            total_duration,
        })
    }

    /// Check if a node has any failed ancestor in the DAG.
    fn has_failed_ancestor(&self, node_id: &str, failed: &HashSet<NodeId>) -> bool {
        use petgraph::Direction;

        let Some(&idx) = self.graph.node_map.get(node_id) else {
            return false;
        };

        // Check all incoming neighbors (direct parents)
        for pred_idx in self
            .graph
            .inner
            .neighbors_directed(idx, Direction::Incoming)
        {
            let pred_id = &self.graph.inner[pred_idx].id;
            if failed.contains(pred_id) {
                return true;
            }
        }
        false
    }

    /// Gather output engrams from all upstream (predecessor) nodes as input.
    fn gather_inputs(
        &self,
        node_id: &str,
        outputs: &HashMap<NodeId, Vec<roko_core::Engram>>,
    ) -> Vec<roko_core::Engram> {
        use petgraph::Direction;

        let Some(&idx) = self.graph.node_map.get(node_id) else {
            return vec![];
        };

        let mut input = Vec::new();
        for pred_idx in self
            .graph
            .inner
            .neighbors_directed(idx, Direction::Incoming)
        {
            let pred_id = &self.graph.inner[pred_idx].id;
            if let Some(engrams) = outputs.get(pred_id) {
                input.extend(engrams.iter().cloned());
            }
        }
        input
    }
}

/// Build the default cell registry with standard gate and utility cells.
///
/// Registered cell types:
/// - `gate.compile` -- `CompileGate` (cargo check)
/// - `gate.test` -- `TestGate` (cargo test)
/// - `gate.clippy` -- `ClippyGate` (cargo clippy)
/// - `noop` -- `NoopCell` (passes input through unchanged, useful for testing)
#[must_use]
pub fn default_registry() -> CellRegistry {
    let mut registry = CellRegistry::new();

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

    registry.register("noop", |_config| Box::new(NoopCell::default()));

    registry.register("score", |_config| {
        Box::new(NoopCell::with_id_and_name("score", "ScoreCell"))
    });

    registry.register("compose", |_config| {
        Box::new(NoopCell::with_id_and_name("compose", "ComposeCell"))
    });

    registry.register("act", |_config| {
        Box::new(NoopCell::with_id_and_name("act", "ActCell"))
    });

    // Task executor cell for plan-to-graph converted tasks (task 101).
    registry.register("task-executor", |_config| {
        Box::new(crate::cells::task_executor::TaskExecutorCell::default())
    });

    // Cognitive loop stub cells (task 103).
    // These are passthrough stubs registered so cognitive loop TOML graphs
    // validate without error. Real implementations are future tasks.
    for name in crate::cells::stubs::COGNITIVE_LOOP_STUBS {
        let cell_name = (*name).to_string();
        registry.register(name, move |_config| {
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
        input: Vec<roko_core::Engram>,
        _ctx: &CellContext,
    ) -> roko_core::error::Result<Vec<roko_core::Engram>> {
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
        Some(Duration::from_secs(60))
    }
    async fn execute(
        &self,
        input: Vec<roko_core::Engram>,
        _ctx: &CellContext,
    ) -> roko_core::error::Result<Vec<roko_core::Engram>> {
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
}
