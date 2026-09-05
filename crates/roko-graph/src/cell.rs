//! The Cell trait -- universal computation unit for graph nodes.
//!
//! Every node in a `Graph` is backed by a Cell implementation. Cells are
//! instantiated from TOML config via the `CellRegistry` and executed by the
//! graph engine in topological order.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use roko_core::{PredictionRecord, ProtocolId, SharedGateEvaluator, Signal, error::Result};

/// Semantic version tuple for Cell implementations.
pub type CellVersion = (u32, u32, u32);

/// Shared service handles injected into `CellContext` by the graph engine.
///
/// The engine populates these before each Cell execution. Cells access
/// services through `ctx.resources` in their `execute` implementation.
#[derive(Clone, Default)]
pub struct CellResources {
    /// Shared gate evaluator for cells that need to run verification.
    ///
    /// Populated by the engine from the host-supplied gate service.
    /// `GatePipelineCell` accesses this as `ctx.resources.gates`.
    pub gates: Option<Arc<dyn SharedGateEvaluator>>,
}

impl std::fmt::Debug for CellResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellResources")
            .field("gates", &self.gates.is_some())
            .finish()
    }
}

/// Runtime context passed to `Cell::execute()`.
///
/// Provides the cell with access to shared infrastructure (cancel tokens,
/// budgets, trace context) without cells needing to manage their own handles.
#[derive(Debug, Clone)]
pub struct CellContext {
    /// Trace context for observability.
    pub trace_id: Option<String>,
    /// Run identifier (if executing within a Graph/Flow).
    pub run_id: Option<String>,
    /// Remaining budget for this execution (USD).
    pub budget_remaining: Option<f64>,
    /// Unix millisecond deadline for this execution scope.
    ///
    /// When `Some`, the cell should not begin work after this timestamp.
    /// Use [`CellContext::time_remaining_ms`] to check how much time is left.
    pub deadline_ms: Option<i64>,
    /// ID of the enclosing `Graph` (for nested execution tracing).
    pub parent_graph_id: Option<String>,
    /// The ID of the Cell currently being executed.
    ///
    /// Set by the engine immediately before calling [`Cell::execute`].
    pub cell_id: Option<String>,
    /// Effective capability intersection for this execution scope.
    /// `None` preserves the unscoped workspace behavior.
    pub capabilities: Option<roko_core::CapabilitySet>,
    /// Zero-based wave index within the current graph execution.
    ///
    /// Set by the engine during parallel wave execution. `None` for
    /// sequential execution or when the wave context is not available.
    /// Added by #246; wired by #256.
    pub wave_index: Option<u32>,
    /// Total number of waves in the current graph execution.
    ///
    /// Set by the engine during parallel wave execution. `None` for
    /// sequential execution or when the wave context is not available.
    /// Added by #246; wired by #256.
    pub total_waves: Option<u32>,
    /// Shared service handles injected by the graph engine (#250).
    ///
    /// Cells access specific services through typed fields, e.g.
    /// `ctx.resources.gates` for gate evaluation.
    pub resources: CellResources,
    /// Shared cancellation flag for cooperative cell shutdown (#255).
    ///
    /// When `Some` and the inner `AtomicBool` is `true`, the cell should
    /// abort work as soon as practical. Checked via [`CellContext::is_cancelled`].
    pub cancel_flag: Option<Arc<AtomicBool>>,
    /// Shared pause flag for cooperative cell suspension (#255).
    ///
    /// When `Some` and the inner `AtomicBool` is `true`, the cell should
    /// pause new work and yield. Checked via [`CellContext::is_paused`].
    pub pause_flag: Option<Arc<AtomicBool>>,
}

impl CellContext {
    /// Construct a new `CellContext` with no trace or budget info.
    #[must_use]
    pub fn new() -> Self {
        Self {
            trace_id: None,
            run_id: None,
            budget_remaining: None,
            deadline_ms: None,
            parent_graph_id: None,
            cell_id: None,
            capabilities: None,
            wave_index: None,
            total_waves: None,
            resources: CellResources::default(),
            cancel_flag: None,
            pause_flag: None,
        }
    }

    /// Builder: set the trace ID.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Builder: set the run ID.
    #[must_use]
    pub fn with_run_id(mut self, run_id: String) -> Self {
        self.run_id = Some(run_id);
        self
    }

    /// Builder: set the remaining budget.
    #[must_use]
    pub const fn with_budget(mut self, budget: f64) -> Self {
        self.budget_remaining = Some(budget);
        self
    }

    /// Returns `true` if the budget has been exhausted.
    ///
    /// Specifically, returns `true` when `budget_remaining` is `Some(x)` and
    /// `x <= 0.0`. Returns `false` when no budget limit is set.
    #[must_use]
    pub fn is_over_budget(&self) -> bool {
        self.budget_remaining.is_some_and(|b| b <= 0.0)
    }

    /// Returns the number of milliseconds remaining before the deadline.
    ///
    /// Returns `None` when no deadline is set. Returns a negative value if the
    /// deadline has already passed.
    #[must_use]
    pub fn time_remaining_ms(&self) -> Option<i64> {
        let deadline = self.deadline_ms?;
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Some(deadline - now_ms)
    }

    /// Builder: set a Unix millisecond deadline for this execution scope.
    #[must_use]
    pub fn with_deadline(mut self, ms: i64) -> Self {
        self.deadline_ms = Some(ms);
        self
    }

    /// Builder: record the ID of the enclosing Graph.
    #[must_use]
    pub fn with_parent_graph(mut self, id: String) -> Self {
        self.parent_graph_id = Some(id);
        self
    }

    /// Builder: record the ID of the Cell being executed.
    #[must_use]
    pub fn with_cell_id(mut self, id: String) -> Self {
        self.cell_id = Some(id);
        self
    }

    /// Builder: attach the effective Space/Graph capability intersection.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: roko_core::CapabilitySet) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Builder: attach shared service resources.
    #[must_use]
    pub fn with_resources(mut self, resources: CellResources) -> Self {
        self.resources = resources;
        self
    }

    /// Builder: attach a shared cancellation flag (#255).
    ///
    /// When the flag is set to `true`, [`is_cancelled`](Self::is_cancelled)
    /// will return `true` and the cell should abort work.
    #[must_use]
    pub fn with_cancel_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancel_flag = Some(flag);
        self
    }

    /// Builder: attach a shared pause flag (#255).
    ///
    /// When the flag is set to `true`, [`is_paused`](Self::is_paused)
    /// will return `true` and the cell should yield.
    #[must_use]
    pub fn with_pause_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.pause_flag = Some(flag);
        self
    }

    /// Returns `true` if cancellation has been requested (#255).
    ///
    /// Returns `false` when no cancel flag is set. Cells should check
    /// this periodically during long-running operations and abort if true.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Acquire))
    }

    /// Returns `true` if the executor is paused (#255).
    ///
    /// Returns `false` when no pause flag is set. Cells should check
    /// this to delay starting new work until unpaused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.pause_flag
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Acquire))
    }
}

impl Default for CellContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Universal computation unit. Every graph node is backed by a Cell implementation.
///
/// The Cell trait provides identity, cost estimation, and an async execute method.
/// Implementations include gates (compile, test, clippy), agent dispatch, compose
/// steps, and user-defined cells registered via `CellRegistry`.
#[async_trait]
pub trait Cell: Send + Sync + 'static {
    /// Unique identifier for this cell instance.
    fn cell_id(&self) -> &str;

    /// Human-readable name for display and logging.
    fn cell_name(&self) -> &str;

    /// Semantic version of this cell's implementation.
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }

    /// Protocol conformances this cell declares (typed).
    fn protocols(&self) -> Vec<ProtocolId> {
        Vec::new()
    }
    /// Convenience: check if this cell conforms to a given protocol.
    fn has_protocol(&self, id: ProtocolId) -> bool {
        self.protocols().contains(&id)
    }

    /// Returns `true` when this cell is a stub/placeholder rather than a real
    /// implementation.  The graph engine uses this to tag [`NodeResult`]s and
    /// emit a warning in [`GraphOutput::summary`] so operators know which nodes
    /// still need real implementations.
    fn is_stub(&self) -> bool {
        false
    }

    /// Estimated USD cost per invocation, when known.
    fn estimated_cost(&self) -> Option<f64> {
        None
    }

    /// Estimated wall-clock duration per invocation, when known.
    fn estimated_duration(&self) -> Option<Duration> {
        None
    }

    /// Describes the input type this cell expects. `None` means untyped (Any).
    fn input_schema(&self) -> Option<&roko_core::TypeSchema> {
        None
    }

    /// Describes the output type this cell produces. `None` means untyped (Any).
    fn output_schema(&self) -> Option<&roko_core::TypeSchema> {
        None
    }

    /// Predict the expected outcome before execution.
    ///
    /// Returning `Some` activates the Graph Engine's canonical
    /// predict-publish-correct lifecycle for this Cell invocation.
    fn predict(&self, input: &[Signal]) -> Option<PredictionRecord> {
        let _ = input;
        None
    }

    /// Compute a normalized calibration error after successful execution.
    ///
    /// Predictive Cells return `Some(error)` in `[0.0, 1.0]` when their
    /// prediction schema supports a meaningful comparison with actual output.
    fn calibration_error(&self, prediction: &PredictionRecord, actual: &[Signal]) -> Option<f64> {
        let _ = (prediction, actual);
        None
    }

    /// Receive the completed prediction and actual output for online learning.
    fn correct(&self, prediction: &PredictionRecord, actual: &[Signal]) {
        let _ = (prediction, actual);
    }

    /// Execute this cell with the given input signals, producing output signals.
    ///
    /// The graph engine calls this in topological order, feeding outputs from
    /// upstream cells as inputs to downstream cells.
    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_resources_default_has_no_gates() {
        let r = CellResources::default();
        assert!(r.gates.is_none());
    }

    #[test]
    fn cell_resources_debug_shows_presence() {
        let r = CellResources::default();
        let debug = format!("{r:?}");
        assert!(debug.contains("gates: false"));
    }

    #[test]
    fn cell_context_new_has_default_resources() {
        let ctx = CellContext::new();
        assert!(ctx.resources.gates.is_none());
    }

    #[test]
    fn cell_context_with_resources() {
        // Verify the builder method works.
        let r = CellResources::default();
        let ctx = CellContext::new().with_resources(r);
        assert!(ctx.resources.gates.is_none());
    }

    #[test]
    fn cell_context_with_gates_resource() {
        use roko_core::{
            SharedGateError, SharedGateEvaluator, SharedGateRequest, SharedGateVerdict,
        };

        struct MockEvaluator;

        #[async_trait]
        impl SharedGateEvaluator for MockEvaluator {
            async fn verify_rung(
                &self,
                _request: &SharedGateRequest,
            ) -> std::result::Result<SharedGateVerdict, SharedGateError> {
                Ok(SharedGateVerdict::pass("mock"))
            }
        }

        let r = CellResources {
            gates: Some(Arc::new(MockEvaluator)),
        };
        let ctx = CellContext::new().with_resources(r);
        assert!(ctx.resources.gates.is_some());

        let debug = format!("{:?}", ctx.resources);
        assert!(debug.contains("gates: true"));
    }
}
