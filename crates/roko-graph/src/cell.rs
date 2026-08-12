//! The Cell trait -- universal computation unit for graph nodes.
//!
//! Every node in a `Graph` is backed by a Cell implementation. Cells are
//! instantiated from TOML config via the `CellRegistry` and executed by the
//! graph engine in topological order.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use roko_core::{ProtocolId, Signal, error::Result};

/// Semantic version tuple for Cell implementations.
pub type CellVersion = (u32, u32, u32);

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
}

impl CellContext {
    /// Construct a new `CellContext` with no trace or budget info.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            trace_id: None,
            run_id: None,
            budget_remaining: None,
            deadline_ms: None,
            parent_graph_id: None,
            cell_id: None,
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

    /// Execute this cell with the given input signals, producing output signals.
    ///
    /// The graph engine calls this in topological order, feeding outputs from
    /// upstream cells as inputs to downstream cells.
    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>>;
}
