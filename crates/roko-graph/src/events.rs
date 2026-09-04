//! Rich graph execution event contract (backlog #246).
//!
//! This module defines the graph-local event vocabulary, delivery classes,
//! sink trait, and disposition types. Graph identity lives here; mapping to
//! canonical/presentation events belongs exclusively to #248.
//!
//! # Delivery classes
//!
//! Events are classified as either **Reliable** or **BestEffort**:
//!
//! - **Reliable** events must be acknowledged before the owning safe boundary
//!   commits. A reliable publish may not be silently dropped. Terminal, usage,
//!   receipt, gate, and control events are all reliable.
//!
//! - **BestEffort** events are bounded/coalescible: `NodeProgress`,
//!   `CellProgress`, `AgentText`, and `GateRungOutput`. A slow TUI never
//!   blocks execution. If a best-effort event is dropped, the engine must
//!   synchronously publish one reliable `Gap` before the next event.
//!
//! # Ordering
//!
//! Within a single `run_id`, the `seq` field is strictly monotonic. Replay
//! preserves original `seq` values.
//!
//! # Terminal invariants
//!
//! Every dispatched node has one start and one terminal event. A conditionally
//! skipped node has one skip terminal and zero live-start events. Resume emits
//! replay/resume events without reporting a new provider execution.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::ExecutionClass;

/// Schema version for graph execution events. Bumped on breaking changes.
pub const GRAPH_EVENT_SCHEMA_VERSION: u8 = 1;

// ---------------------------------------------------------------------------
// Delivery classification
// ---------------------------------------------------------------------------

/// Delivery class for a graph execution event.
///
/// The engine uses this to decide whether a slow observer can block execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEventDelivery {
    /// The engine must receive `Acknowledged` before committing at the next
    /// node/wave/terminal boundary. Reliable publish failure stops the graph.
    Reliable,
    /// Fire-and-forget: the engine emits and does not wait. May be dropped or
    /// coalesced by a slow observer, but the observer must then emit a `Gap`.
    BestEffort,
}

// ---------------------------------------------------------------------------
// Disposition
// ---------------------------------------------------------------------------

/// Result of publishing a single graph execution event to a sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEventDisposition {
    /// Event was durably received by the observer.
    Acknowledged,
    /// Best-effort event was accepted but may be dropped under load.
    AcceptedBestEffort,
    /// Event was merged with a preceding event of the same variant/scope.
    Coalesced,
    /// Event was discarded. Legal only for best-effort events. The engine
    /// must synchronously publish one reliable `Gap` before the next event.
    Dropped,
}

// ---------------------------------------------------------------------------
// Sink trait
// ---------------------------------------------------------------------------

/// Async trait for receiving graph execution events.
///
/// Implementations must not mutate graph or cell state. The engine depends
/// only on this passive contract; routing, projection, and overhead enforcement
/// are owned by downstream consumers. `TelemetryEventSink` is kept unchanged
/// and the engine emits to both sinks via a private helper.
#[async_trait]
pub trait GraphEventSink: Send + Sync {
    /// Publish a single graph execution event.
    ///
    /// # Errors
    ///
    /// Returns an error when a reliable event cannot be delivered. The engine
    /// treats reliable publish failure as a stop condition at the next safe
    /// commit boundary (node/wave/terminal).
    async fn publish(
        &self,
        event: &GraphExecutionEvent,
    ) -> std::result::Result<GraphEventDisposition, GraphEventError>;
}

/// Error returned by [`GraphEventSink::publish`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphEventError {
    /// The sink rejected or failed to deliver a reliable event.
    #[error("graph event delivery failed: {reason}")]
    DeliveryFailed {
        /// Human-readable explanation.
        reason: String,
    },
    /// The sink is permanently closed.
    #[error("graph event sink closed")]
    SinkClosed,
}

// ---------------------------------------------------------------------------
// Shared field groups
// ---------------------------------------------------------------------------

/// Fields present on every graph execution event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonFields {
    /// Schema version (always [`GRAPH_EVENT_SCHEMA_VERSION`]).
    pub schema_version: u8,
    /// Unique run identifier.
    pub run_id: String,
    /// Graph identifier (from metadata).
    pub graph_id: String,
    /// Strictly monotonic sequence number within this `run_id`.
    pub seq: u64,
}

/// Additional fields for node/dispatch/gate/cell variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeFields {
    /// Node identifier within the graph.
    pub node_id: String,
    /// Cell type backing the node.
    pub cell_type: String,
    /// Whether the node is Workflow or Activity.
    pub execution_class: ExecutionClass,
    /// Retry attempt number (0 for first attempt).
    pub attempt: u32,
}

/// Additional fields for wave variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaveFields {
    /// Zero-based index of the current wave.
    pub wave_index: u32,
    /// Total number of waves in the graph.
    pub total_waves: u32,
}

/// Additional fields for dispatch (agent/tool) variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchFields {
    /// Stable attempt identity across retries.
    pub attempt_id: String,
    /// Optional agent identifier.
    pub agent_id: Option<String>,
}

/// Terminal statistics included in graph/wave completion events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalStats {
    /// Elapsed wall-clock time in milliseconds.
    pub elapsed_ms: u64,
    /// Number of nodes that completed successfully.
    pub completed_nodes: u32,
    /// Total number of nodes in scope.
    pub total_nodes: u32,
}

/// Budget amounts in micro-USD (1 USD = 1_000_000 micro-USD).
///
/// All fields are `u64` as mandated by the spec: do not use `f64` in
/// graph-local accounting events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetAmounts {
    /// Estimated cost so far.
    pub estimated_micro_usd: u64,
    /// Reserved but not yet spent.
    pub reserved_micro_usd: u64,
    /// Actually spent (provider-reported).
    pub actual_micro_usd: u64,
    /// Remaining budget.
    pub remaining_micro_usd: u64,
}

// ---------------------------------------------------------------------------
// The event enum
// ---------------------------------------------------------------------------

/// Graph execution event -- the complete local vocabulary.
///
/// Every variant carries [`CommonFields`] via the shared `common` field.
/// Node/dispatch/gate/cell variants additionally carry [`NodeFields`].
/// Wave variants carry [`WaveFields`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant, missing_docs)]
pub enum GraphExecutionEvent {
    // ── Graph lifecycle ──────────────────────────────────────────────────
    /// Graph execution has started.
    GraphStarted {
        #[serde(flatten)]
        common: CommonFields,
    },
    /// Graph execution completed successfully.
    GraphCompleted {
        #[serde(flatten)]
        common: CommonFields,
        /// Terminal statistics.
        stats: TerminalStats,
    },
    /// Graph execution failed.
    GraphFailed {
        #[serde(flatten)]
        common: CommonFields,
        /// Terminal statistics.
        stats: TerminalStats,
        /// Concise error description.
        error: String,
    },
    /// Graph execution was cancelled.
    GraphCancelled {
        #[serde(flatten)]
        common: CommonFields,
        /// Terminal statistics.
        stats: TerminalStats,
    },

    // ── Wave lifecycle ───────────────────────────────────────────────────
    /// A topological wave has started executing.
    WaveStarted {
        #[serde(flatten)]
        common: CommonFields,
        /// Wave metadata.
        wave: WaveFields,
    },
    /// A topological wave has completed.
    WaveCompleted {
        #[serde(flatten)]
        common: CommonFields,
        /// Wave metadata.
        wave: WaveFields,
        /// Duration of this wave.
        elapsed_ms: u64,
    },

    // ── Node lifecycle ───────────────────────────────────────────────────
    /// A node has started executing.
    NodeStarted {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
    },
    /// A node was skipped (conditional route not selected or upstream failed).
    NodeSkipped {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Why the node was skipped.
        reason: String,
    },
    /// A node is being retried after a transient failure.
    NodeRetrying {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Error from the previous attempt.
        error: String,
    },
    /// Incremental progress within a node (best-effort).
    NodeProgress {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Bounded progress message.
        message: String,
        /// Steps completed so far.
        completed: u32,
        /// Total steps expected.
        total: u32,
    },
    /// A node completed successfully.
    NodeCompleted {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Duration of this node execution.
        elapsed_ms: u64,
    },
    /// A node failed (all retries exhausted).
    NodeFailed {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Duration of this node execution.
        elapsed_ms: u64,
        /// Concise error description.
        error: String,
    },

    // ── Dispatch (agent/tool) lifecycle ───────────────────────────────────
    /// An agent dispatch has started.
    AgentStarted {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Dispatch identity.
        dispatch: DispatchFields,
        /// Provider name (e.g. "anthropic", "openai").
        provider: String,
        /// Model identifier.
        model: String,
    },
    /// A chunk of agent text output (best-effort, streaming).
    AgentText {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Dispatch identity.
        dispatch: DispatchFields,
        /// One bounded text chunk.
        chunk: String,
    },
    /// A tool call has started within an agent dispatch.
    ToolStarted {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Dispatch identity.
        dispatch: DispatchFields,
        /// Tool name.
        tool_name: String,
    },
    /// A tool call has completed within an agent dispatch.
    ToolCompleted {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Dispatch identity.
        dispatch: DispatchFields,
        /// Tool name.
        tool_name: String,
        /// Whether the tool call succeeded.
        success: bool,
        /// Duration of the tool call in milliseconds.
        duration_ms: u64,
    },
    /// Token usage and cost recorded for an agent dispatch.
    UsageRecorded {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Dispatch identity.
        dispatch: DispatchFields,
        /// Input tokens consumed.
        input_tokens: u64,
        /// Output tokens produced.
        output_tokens: u64,
        /// Actual cost in micro-USD.
        actual_micro_usd: u64,
    },
    /// An agent dispatch has completed.
    AgentCompleted {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Dispatch identity.
        dispatch: DispatchFields,
        /// Provider name.
        provider: String,
        /// Model identifier.
        model: String,
        /// Duration of the dispatch in milliseconds.
        elapsed_ms: u64,
    },

    // ── Gate/cell detail ─────────────────────────────────────────────────
    /// A gate rung has started evaluating.
    GateRungStarted {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Zero-based rung index.
        rung_index: u32,
        /// Human-readable rung name.
        rung_name: String,
    },
    /// Intermediate output from a gate rung (best-effort).
    GateRungOutput {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Zero-based rung index.
        rung_index: u32,
        /// Human-readable rung name.
        rung_name: String,
        /// Bounded output text.
        output: String,
    },
    /// A gate rung has completed.
    GateRungCompleted {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Zero-based rung index.
        rung_index: u32,
        /// Human-readable rung name.
        rung_name: String,
        /// Whether the rung was selected (true) or skipped (false).
        selected: bool,
        /// Whether the rung was skipped entirely.
        skipped: bool,
        /// Whether the rung passed validation.
        pass: bool,
        /// Duration of the rung in milliseconds.
        duration_ms: u64,
        /// Reference to evidence (path, hash, etc.).
        evidence_ref: Option<String>,
    },
    /// Incremental progress within a cell (best-effort).
    CellProgress {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Bounded progress message.
        message: String,
        /// Steps completed so far.
        completed: u32,
        /// Total steps expected.
        total: u32,
    },

    // ── Accounting ───────────────────────────────────────────────────────
    /// Budget state has been updated.
    BudgetUpdated {
        #[serde(flatten)]
        common: CommonFields,
        /// Current budget amounts (all micro-USD, no f64).
        amounts: BudgetAmounts,
    },

    // ── Completion delivery (#254) ──────────────────────────────────────
    /// A completion delivery lifecycle has started.
    DeliveryStarted {
        #[serde(flatten)]
        common: CommonFields,
        /// Delivery identifier (idempotency key).
        delivery_id: String,
        /// Plan identifier.
        plan_id: String,
        /// Branch being delivered.
        branch: String,
        /// Whether publication is requested.
        publish: bool,
    },
    /// A completion delivery state machine has advanced.
    DeliveryStateAdvanced {
        #[serde(flatten)]
        common: CommonFields,
        /// Delivery identifier.
        delivery_id: String,
        /// Plan identifier.
        plan_id: String,
        /// Previous state.
        from_state: String,
        /// New state after the transition.
        to_state: String,
        /// Optional merge commit OID (set after merge).
        merge_commit: Option<String>,
        /// Optional publication reference (set after publish).
        publication_ref: Option<String>,
    },
    /// A completion delivery has reached terminal success.
    DeliveryCompleted {
        #[serde(flatten)]
        common: CommonFields,
        /// Delivery identifier.
        delivery_id: String,
        /// Plan identifier.
        plan_id: String,
        /// Release policy for the workspace.
        release_policy: String,
    },
    /// A completion delivery has reached a terminal failure state.
    DeliveryFailed {
        #[serde(flatten)]
        common: CommonFields,
        /// Delivery identifier.
        delivery_id: String,
        /// Plan identifier.
        plan_id: String,
        /// Terminal failure state (conflict, regression_failed, terminal_failed).
        failure_state: String,
        /// Error details.
        error: String,
        /// Release policy for the workspace.
        release_policy: String,
    },

    // ── Feedback settlement (#253) ─────────────────────────────────────
    /// A feedback sink has been successfully settled for a task attempt.
    FeedbackSinkSettled {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Receipt idempotency key.
        idempotency_key: String,
        /// Sink key (e.g. "episode", "routing", "knowledge").
        sink_key: String,
        /// Row index in the 12-row settlement order (0-11).
        row: u32,
    },
    /// A feedback sink failed during settlement for a task attempt.
    FeedbackSinkFailed {
        #[serde(flatten)]
        common: CommonFields,
        /// Node identity and metadata.
        node: NodeFields,
        /// Receipt idempotency key.
        idempotency_key: String,
        /// Sink key that failed.
        sink_key: String,
        /// Row index in the 12-row settlement order (0-11).
        row: u32,
        /// Whether this was a critical failure (rows 0-2).
        critical: bool,
        /// Error message from the sink.
        error: String,
    },

    // ── Delivery/replay ──────────────────────────────────────────────────
    /// Replay of a previous run has started.
    ReplayStarted {
        #[serde(flatten)]
        common: CommonFields,
    },
    /// Replay of a previous run has completed.
    ReplayCompleted {
        #[serde(flatten)]
        common: CommonFields,
    },
    /// One or more events were lost (explicit gap marker).
    ///
    /// Emitted as a reliable event after any best-effort drop so downstream
    /// consumers can detect and account for the loss.
    Gap {
        #[serde(flatten)]
        common: CommonFields,
        /// Number of events lost.
        lost_count: u64,
    },
}

impl GraphExecutionEvent {
    /// Return the delivery class for this event variant.
    ///
    /// Only `NodeProgress`, `CellProgress`, `AgentText`, and `GateRungOutput`
    /// are best-effort. All other variants -- including `UsageRecorded` and
    /// `Gap` -- are reliable.
    #[must_use]
    pub const fn delivery(&self) -> GraphEventDelivery {
        match self {
            Self::NodeProgress { .. }
            | Self::CellProgress { .. }
            | Self::AgentText { .. }
            | Self::GateRungOutput { .. } => GraphEventDelivery::BestEffort,
            _ => GraphEventDelivery::Reliable,
        }
    }

    /// Return the common fields shared by every variant.
    #[must_use]
    pub fn common(&self) -> &CommonFields {
        match self {
            Self::GraphStarted { common, .. }
            | Self::GraphCompleted { common, .. }
            | Self::GraphFailed { common, .. }
            | Self::GraphCancelled { common, .. }
            | Self::WaveStarted { common, .. }
            | Self::WaveCompleted { common, .. }
            | Self::NodeStarted { common, .. }
            | Self::NodeSkipped { common, .. }
            | Self::NodeRetrying { common, .. }
            | Self::NodeProgress { common, .. }
            | Self::NodeCompleted { common, .. }
            | Self::NodeFailed { common, .. }
            | Self::AgentStarted { common, .. }
            | Self::AgentText { common, .. }
            | Self::ToolStarted { common, .. }
            | Self::ToolCompleted { common, .. }
            | Self::UsageRecorded { common, .. }
            | Self::AgentCompleted { common, .. }
            | Self::GateRungStarted { common, .. }
            | Self::GateRungOutput { common, .. }
            | Self::GateRungCompleted { common, .. }
            | Self::CellProgress { common, .. }
            | Self::BudgetUpdated { common, .. }
            | Self::DeliveryStarted { common, .. }
            | Self::DeliveryStateAdvanced { common, .. }
            | Self::DeliveryCompleted { common, .. }
            | Self::DeliveryFailed { common, .. }
            | Self::FeedbackSinkSettled { common, .. }
            | Self::FeedbackSinkFailed { common, .. }
            | Self::ReplayStarted { common, .. }
            | Self::ReplayCompleted { common, .. }
            | Self::Gap { common, .. } => common,
        }
    }

    /// Return the node fields, if this variant carries them.
    #[must_use]
    pub fn node(&self) -> Option<&NodeFields> {
        match self {
            Self::NodeStarted { node, .. }
            | Self::NodeSkipped { node, .. }
            | Self::NodeRetrying { node, .. }
            | Self::NodeProgress { node, .. }
            | Self::NodeCompleted { node, .. }
            | Self::NodeFailed { node, .. }
            | Self::AgentStarted { node, .. }
            | Self::AgentText { node, .. }
            | Self::ToolStarted { node, .. }
            | Self::ToolCompleted { node, .. }
            | Self::UsageRecorded { node, .. }
            | Self::AgentCompleted { node, .. }
            | Self::GateRungStarted { node, .. }
            | Self::GateRungOutput { node, .. }
            | Self::GateRungCompleted { node, .. }
            | Self::CellProgress { node, .. }
            | Self::FeedbackSinkSettled { node, .. }
            | Self::FeedbackSinkFailed { node, .. } => Some(node),
            _ => None,
        }
    }

    /// Whether this is a terminal event for its scope.
    ///
    /// Graph-terminal: `GraphCompleted`, `GraphFailed`, `GraphCancelled`.
    /// Node-terminal: `NodeCompleted`, `NodeFailed`, `NodeSkipped`.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::GraphCompleted { .. }
                | Self::GraphFailed { .. }
                | Self::GraphCancelled { .. }
                | Self::NodeCompleted { .. }
                | Self::NodeFailed { .. }
                | Self::NodeSkipped { .. }
                | Self::DeliveryCompleted { .. }
                | Self::DeliveryFailed { .. }
        )
    }

    /// Whether this event carries a `DispatchFields` block.
    #[must_use]
    pub fn dispatch(&self) -> Option<&DispatchFields> {
        match self {
            Self::AgentStarted { dispatch, .. }
            | Self::AgentText { dispatch, .. }
            | Self::ToolStarted { dispatch, .. }
            | Self::ToolCompleted { dispatch, .. }
            | Self::UsageRecorded { dispatch, .. }
            | Self::AgentCompleted { dispatch, .. } => Some(dispatch),
            _ => None,
        }
    }

    /// Human-readable variant name for logging.
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::GraphStarted { .. } => "GraphStarted",
            Self::GraphCompleted { .. } => "GraphCompleted",
            Self::GraphFailed { .. } => "GraphFailed",
            Self::GraphCancelled { .. } => "GraphCancelled",
            Self::WaveStarted { .. } => "WaveStarted",
            Self::WaveCompleted { .. } => "WaveCompleted",
            Self::NodeStarted { .. } => "NodeStarted",
            Self::NodeSkipped { .. } => "NodeSkipped",
            Self::NodeRetrying { .. } => "NodeRetrying",
            Self::NodeProgress { .. } => "NodeProgress",
            Self::NodeCompleted { .. } => "NodeCompleted",
            Self::NodeFailed { .. } => "NodeFailed",
            Self::AgentStarted { .. } => "AgentStarted",
            Self::AgentText { .. } => "AgentText",
            Self::ToolStarted { .. } => "ToolStarted",
            Self::ToolCompleted { .. } => "ToolCompleted",
            Self::UsageRecorded { .. } => "UsageRecorded",
            Self::AgentCompleted { .. } => "AgentCompleted",
            Self::GateRungStarted { .. } => "GateRungStarted",
            Self::GateRungOutput { .. } => "GateRungOutput",
            Self::GateRungCompleted { .. } => "GateRungCompleted",
            Self::CellProgress { .. } => "CellProgress",
            Self::BudgetUpdated { .. } => "BudgetUpdated",
            Self::DeliveryStarted { .. } => "DeliveryStarted",
            Self::DeliveryStateAdvanced { .. } => "DeliveryStateAdvanced",
            Self::DeliveryCompleted { .. } => "DeliveryCompleted",
            Self::DeliveryFailed { .. } => "DeliveryFailed",
            Self::FeedbackSinkSettled { .. } => "FeedbackSinkSettled",
            Self::FeedbackSinkFailed { .. } => "FeedbackSinkFailed",
            Self::ReplayStarted { .. } => "ReplayStarted",
            Self::ReplayCompleted { .. } => "ReplayCompleted",
            Self::Gap { .. } => "Gap",
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: construct CommonFields with auto-incrementing seq
// ---------------------------------------------------------------------------

/// Monotonic sequence counter for graph event emission.
///
/// Each `GraphEngine` instance holds one of these. The counter is shared
/// across sequential/parallel/start/resume/Hot execution paths so all events
/// for a single run share one strictly increasing sequence.
#[derive(Debug)]
pub struct EventSeqCounter {
    next: std::sync::atomic::AtomicU64,
}

impl EventSeqCounter {
    /// Create a new counter starting at 1 (seq 0 is reserved for internal use).
    #[must_use]
    pub fn new() -> Self {
        Self {
            next: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Fetch and increment the sequence number.
    pub fn next(&self) -> u64 {
        self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for EventSeqCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to construct [`CommonFields`] with auto-incrementing seq.
#[must_use]
pub fn make_common(run_id: &str, graph_id: &str, seq_counter: &EventSeqCounter) -> CommonFields {
    CommonFields {
        schema_version: GRAPH_EVENT_SCHEMA_VERSION,
        run_id: run_id.to_string(),
        graph_id: graph_id.to_string(),
        seq: seq_counter.next(),
    }
}

/// Helper to construct [`NodeFields`].
#[must_use]
pub fn make_node_fields(
    node_id: &str,
    cell_type: &str,
    execution_class: ExecutionClass,
    attempt: u32,
) -> NodeFields {
    NodeFields {
        node_id: node_id.to_string(),
        cell_type: cell_type.to_string(),
        execution_class,
        attempt,
    }
}

/// Helper to construct [`DispatchFields`].
#[must_use]
pub fn make_dispatch_fields(
    attempt_id: impl Into<String>,
    agent_id: Option<String>,
) -> DispatchFields {
    DispatchFields {
        attempt_id: attempt_id.into(),
        agent_id,
    }
}

/// Helper to construct [`TerminalStats`].
#[must_use]
pub fn make_terminal_stats(elapsed: Duration, completed: u32, total: u32) -> TerminalStats {
    TerminalStats {
        elapsed_ms: elapsed.as_millis() as u64,
        completed_nodes: completed,
        total_nodes: total,
    }
}

// ---------------------------------------------------------------------------
// Engine emission helper
// ---------------------------------------------------------------------------

/// Private helper: emit to a graph event sink if present, logging failures.
///
/// This is the single emission path shared by sequential, parallel, `start()`,
/// resume, and Hot Graph execution. It does NOT replace `TelemetryEventSink`;
/// the engine emits to both sinks independently.
#[allow(dead_code)]
pub(crate) async fn emit_graph_event(
    sink: Option<&Arc<dyn GraphEventSink>>,
    event: &GraphExecutionEvent,
) {
    let Some(sink) = sink else {
        return;
    };
    match sink.publish(event).await {
        Ok(GraphEventDisposition::Dropped) => {
            // Best-effort drop is legal only for best-effort events.
            // The caller is responsible for emitting a Gap if needed.
            if event.delivery() == GraphEventDelivery::Reliable {
                tracing::error!(
                    variant = event.variant_name(),
                    seq = event.common().seq,
                    "reliable graph event was dropped by sink -- this is a contract violation"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                variant = event.variant_name(),
                seq = event.common().seq,
                error = %e,
                "graph event sink publish failed"
            );
        }
        Ok(_) => {}
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_common() -> CommonFields {
        CommonFields {
            schema_version: GRAPH_EVENT_SCHEMA_VERSION,
            run_id: "run-1".to_string(),
            graph_id: "graph-1".to_string(),
            seq: 1,
        }
    }

    fn test_node() -> NodeFields {
        NodeFields {
            node_id: "node-1".to_string(),
            cell_type: "agent".to_string(),
            execution_class: ExecutionClass::Activity,
            attempt: 0,
        }
    }

    fn test_wave() -> WaveFields {
        WaveFields {
            wave_index: 0,
            total_waves: 3,
        }
    }

    fn test_dispatch() -> DispatchFields {
        DispatchFields {
            attempt_id: "attempt-1".to_string(),
            agent_id: Some("agent-1".to_string()),
        }
    }

    fn test_stats() -> TerminalStats {
        TerminalStats {
            elapsed_ms: 1500,
            completed_nodes: 3,
            total_nodes: 5,
        }
    }

    fn test_budget() -> BudgetAmounts {
        BudgetAmounts {
            estimated_micro_usd: 100_000,
            reserved_micro_usd: 50_000,
            actual_micro_usd: 80_000,
            remaining_micro_usd: 920_000,
        }
    }

    // ── Delivery classification ──────────────────────────────────────────

    #[test]
    fn graph_lifecycle_events_are_reliable() {
        let common = test_common();
        let stats = test_stats();

        let events = [
            GraphExecutionEvent::GraphStarted {
                common: common.clone(),
            },
            GraphExecutionEvent::GraphCompleted {
                common: common.clone(),
                stats: stats.clone(),
            },
            GraphExecutionEvent::GraphFailed {
                common: common.clone(),
                stats: stats.clone(),
                error: "boom".into(),
            },
            GraphExecutionEvent::GraphCancelled {
                common: common.clone(),
                stats,
            },
        ];
        for event in &events {
            assert_eq!(
                event.delivery(),
                GraphEventDelivery::Reliable,
                "{} should be reliable",
                event.variant_name()
            );
        }
    }

    #[test]
    fn wave_events_are_reliable() {
        let common = test_common();
        let wave = test_wave();
        let events = [
            GraphExecutionEvent::WaveStarted {
                common: common.clone(),
                wave: wave.clone(),
            },
            GraphExecutionEvent::WaveCompleted {
                common,
                wave,
                elapsed_ms: 500,
            },
        ];
        for event in &events {
            assert_eq!(
                event.delivery(),
                GraphEventDelivery::Reliable,
                "{} should be reliable",
                event.variant_name()
            );
        }
    }

    #[test]
    fn node_lifecycle_events_are_reliable_except_progress() {
        let common = test_common();
        let node = test_node();

        let reliable = [
            GraphExecutionEvent::NodeStarted {
                common: common.clone(),
                node: node.clone(),
            },
            GraphExecutionEvent::NodeSkipped {
                common: common.clone(),
                node: node.clone(),
                reason: "upstream failed".into(),
            },
            GraphExecutionEvent::NodeRetrying {
                common: common.clone(),
                node: node.clone(),
                error: "timeout".into(),
            },
            GraphExecutionEvent::NodeCompleted {
                common: common.clone(),
                node: node.clone(),
                elapsed_ms: 1000,
            },
            GraphExecutionEvent::NodeFailed {
                common: common.clone(),
                node: node.clone(),
                elapsed_ms: 500,
                error: "permanent".into(),
            },
        ];
        for event in &reliable {
            assert_eq!(
                event.delivery(),
                GraphEventDelivery::Reliable,
                "{} should be reliable",
                event.variant_name()
            );
        }

        let progress = GraphExecutionEvent::NodeProgress {
            common,
            node,
            message: "step 2/5".into(),
            completed: 2,
            total: 5,
        };
        assert_eq!(progress.delivery(), GraphEventDelivery::BestEffort);
    }

    #[test]
    fn dispatch_events_delivery_classes() {
        let common = test_common();
        let node = test_node();
        let dispatch = test_dispatch();

        let reliable = [
            GraphExecutionEvent::AgentStarted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                provider: "anthropic".into(),
                model: "claude-4".into(),
            },
            GraphExecutionEvent::ToolStarted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                tool_name: "read_file".into(),
            },
            GraphExecutionEvent::ToolCompleted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                tool_name: "read_file".into(),
                success: true,
                duration_ms: 50,
            },
            GraphExecutionEvent::UsageRecorded {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                input_tokens: 1000,
                output_tokens: 500,
                actual_micro_usd: 5000,
            },
            GraphExecutionEvent::AgentCompleted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                provider: "anthropic".into(),
                model: "claude-4".into(),
                elapsed_ms: 3000,
            },
        ];
        for event in &reliable {
            assert_eq!(
                event.delivery(),
                GraphEventDelivery::Reliable,
                "{} should be reliable",
                event.variant_name()
            );
        }

        let text = GraphExecutionEvent::AgentText {
            common,
            node,
            dispatch,
            chunk: "hello".into(),
        };
        assert_eq!(text.delivery(), GraphEventDelivery::BestEffort);
    }

    #[test]
    fn gate_events_delivery_classes() {
        let common = test_common();
        let node = test_node();

        let reliable = [
            GraphExecutionEvent::GateRungStarted {
                common: common.clone(),
                node: node.clone(),
                rung_index: 0,
                rung_name: "compile".into(),
            },
            GraphExecutionEvent::GateRungCompleted {
                common: common.clone(),
                node: node.clone(),
                rung_index: 0,
                rung_name: "compile".into(),
                selected: true,
                skipped: false,
                pass: true,
                duration_ms: 200,
                evidence_ref: None,
            },
        ];
        for event in &reliable {
            assert_eq!(
                event.delivery(),
                GraphEventDelivery::Reliable,
                "{} should be reliable",
                event.variant_name()
            );
        }

        let output = GraphExecutionEvent::GateRungOutput {
            common: common.clone(),
            node: node.clone(),
            rung_index: 0,
            rung_name: "compile".into(),
            output: "Compiling roko-cli...".into(),
        };
        assert_eq!(output.delivery(), GraphEventDelivery::BestEffort);

        let cell_progress = GraphExecutionEvent::CellProgress {
            common,
            node,
            message: "processing".into(),
            completed: 1,
            total: 3,
        };
        assert_eq!(cell_progress.delivery(), GraphEventDelivery::BestEffort);
    }

    #[test]
    fn accounting_and_delivery_events_are_reliable() {
        let common = test_common();

        let events = [
            GraphExecutionEvent::BudgetUpdated {
                common: common.clone(),
                amounts: test_budget(),
            },
            GraphExecutionEvent::ReplayStarted {
                common: common.clone(),
            },
            GraphExecutionEvent::ReplayCompleted {
                common: common.clone(),
            },
            GraphExecutionEvent::Gap {
                common,
                lost_count: 3,
            },
        ];
        for event in &events {
            assert_eq!(
                event.delivery(),
                GraphEventDelivery::Reliable,
                "{} should be reliable",
                event.variant_name()
            );
        }
    }

    // ── Variant count ────────────────────────────────────────────────────

    #[test]
    fn all_26_variants_have_delivery_and_variant_name() {
        // Construct one of each variant and verify both methods work.
        let common = test_common();
        let node = test_node();
        let wave = test_wave();
        let dispatch = test_dispatch();
        let stats = test_stats();
        let budget = test_budget();

        let all: Vec<GraphExecutionEvent> = vec![
            GraphExecutionEvent::GraphStarted {
                common: common.clone(),
            },
            GraphExecutionEvent::GraphCompleted {
                common: common.clone(),
                stats: stats.clone(),
            },
            GraphExecutionEvent::GraphFailed {
                common: common.clone(),
                stats: stats.clone(),
                error: "e".into(),
            },
            GraphExecutionEvent::GraphCancelled {
                common: common.clone(),
                stats,
            },
            GraphExecutionEvent::WaveStarted {
                common: common.clone(),
                wave: wave.clone(),
            },
            GraphExecutionEvent::WaveCompleted {
                common: common.clone(),
                wave,
                elapsed_ms: 1,
            },
            GraphExecutionEvent::NodeStarted {
                common: common.clone(),
                node: node.clone(),
            },
            GraphExecutionEvent::NodeSkipped {
                common: common.clone(),
                node: node.clone(),
                reason: "r".into(),
            },
            GraphExecutionEvent::NodeRetrying {
                common: common.clone(),
                node: node.clone(),
                error: "e".into(),
            },
            GraphExecutionEvent::NodeProgress {
                common: common.clone(),
                node: node.clone(),
                message: "m".into(),
                completed: 0,
                total: 0,
            },
            GraphExecutionEvent::NodeCompleted {
                common: common.clone(),
                node: node.clone(),
                elapsed_ms: 1,
            },
            GraphExecutionEvent::NodeFailed {
                common: common.clone(),
                node: node.clone(),
                elapsed_ms: 1,
                error: "e".into(),
            },
            GraphExecutionEvent::AgentStarted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                provider: "p".into(),
                model: "m".into(),
            },
            GraphExecutionEvent::AgentText {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                chunk: "c".into(),
            },
            GraphExecutionEvent::ToolStarted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                tool_name: "t".into(),
            },
            GraphExecutionEvent::ToolCompleted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                tool_name: "t".into(),
                success: true,
                duration_ms: 1,
            },
            GraphExecutionEvent::UsageRecorded {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                input_tokens: 0,
                output_tokens: 0,
                actual_micro_usd: 0,
            },
            GraphExecutionEvent::AgentCompleted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                provider: "p".into(),
                model: "m".into(),
                elapsed_ms: 1,
            },
            GraphExecutionEvent::GateRungStarted {
                common: common.clone(),
                node: node.clone(),
                rung_index: 0,
                rung_name: "r".into(),
            },
            GraphExecutionEvent::GateRungOutput {
                common: common.clone(),
                node: node.clone(),
                rung_index: 0,
                rung_name: "r".into(),
                output: "o".into(),
            },
            GraphExecutionEvent::GateRungCompleted {
                common: common.clone(),
                node: node.clone(),
                rung_index: 0,
                rung_name: "r".into(),
                selected: true,
                skipped: false,
                pass: true,
                duration_ms: 1,
                evidence_ref: None,
            },
            GraphExecutionEvent::CellProgress {
                common: common.clone(),
                node: node.clone(),
                message: "m".into(),
                completed: 0,
                total: 0,
            },
            GraphExecutionEvent::BudgetUpdated {
                common: common.clone(),
                amounts: budget,
            },
            GraphExecutionEvent::DeliveryStarted {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "plan-a".into(),
                branch: "roko/plan-a".into(),
                publish: true,
            },
            GraphExecutionEvent::DeliveryStateAdvanced {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "plan-a".into(),
                from_state: "Prepared".into(),
                to_state: "Queued".into(),
                merge_commit: None,
                publication_ref: None,
            },
            GraphExecutionEvent::DeliveryCompleted {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "plan-a".into(),
                release_policy: "Delete".into(),
            },
            GraphExecutionEvent::DeliveryFailed {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "plan-a".into(),
                failure_state: "Conflict".into(),
                error: "conflict".into(),
                release_policy: "RetainForReview".into(),
            },
            GraphExecutionEvent::FeedbackSinkSettled {
                common: common.clone(),
                node: node.clone(),
                idempotency_key: "run:plan:task:0".into(),
                sink_key: "episode".into(),
                row: 3,
            },
            GraphExecutionEvent::FeedbackSinkFailed {
                common: common.clone(),
                node: node.clone(),
                idempotency_key: "run:plan:task:0".into(),
                sink_key: "routing".into(),
                row: 5,
                critical: false,
                error: "router unavailable".into(),
            },
            GraphExecutionEvent::ReplayStarted {
                common: common.clone(),
            },
            GraphExecutionEvent::ReplayCompleted {
                common: common.clone(),
            },
            GraphExecutionEvent::Gap {
                common,
                lost_count: 0,
            },
        ];

        assert_eq!(all.len(), 32, "expected exactly 32 variants");

        for event in &all {
            // delivery() doesn't panic
            let _ = event.delivery();
            // variant_name() doesn't panic
            let name = event.variant_name();
            assert!(!name.is_empty());
            // common() doesn't panic
            let c = event.common();
            assert_eq!(c.schema_version, GRAPH_EVENT_SCHEMA_VERSION);
        }
    }

    // ── Terminal classification ───────────────────────────────────────────

    #[test]
    fn terminal_events_are_correctly_classified() {
        let common = test_common();
        let node = test_node();
        let stats = test_stats();

        let terminals = [
            GraphExecutionEvent::GraphCompleted {
                common: common.clone(),
                stats: stats.clone(),
            },
            GraphExecutionEvent::GraphFailed {
                common: common.clone(),
                stats: stats.clone(),
                error: "e".into(),
            },
            GraphExecutionEvent::GraphCancelled {
                common: common.clone(),
                stats,
            },
            GraphExecutionEvent::NodeCompleted {
                common: common.clone(),
                node: node.clone(),
                elapsed_ms: 1,
            },
            GraphExecutionEvent::NodeFailed {
                common: common.clone(),
                node: node.clone(),
                elapsed_ms: 1,
                error: "e".into(),
            },
            GraphExecutionEvent::NodeSkipped {
                common: common.clone(),
                node,
                reason: "r".into(),
            },
            GraphExecutionEvent::DeliveryCompleted {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "p1".into(),
                release_policy: "Delete".into(),
            },
            GraphExecutionEvent::DeliveryFailed {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "p1".into(),
                failure_state: "Conflict".into(),
                error: "conflict".into(),
                release_policy: "RetainForReview".into(),
            },
        ];
        for event in &terminals {
            assert!(
                event.is_terminal(),
                "{} should be terminal",
                event.variant_name()
            );
        }

        let non_terminals = [
            GraphExecutionEvent::GraphStarted {
                common: common.clone(),
            },
            GraphExecutionEvent::NodeStarted {
                common: common.clone(),
                node: test_node(),
            },
            GraphExecutionEvent::DeliveryStarted {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "p1".into(),
                branch: "b".into(),
                publish: true,
            },
            GraphExecutionEvent::DeliveryStateAdvanced {
                common,
                delivery_id: "d1".into(),
                plan_id: "p1".into(),
                from_state: "Prepared".into(),
                to_state: "Queued".into(),
                merge_commit: None,
                publication_ref: None,
            },
        ];
        for event in &non_terminals {
            assert!(
                !event.is_terminal(),
                "{} should not be terminal",
                event.variant_name()
            );
        }
    }

    // ── Accessor coverage ────────────────────────────────────────────────

    #[test]
    fn node_accessor_returns_none_for_graph_events() {
        let event = GraphExecutionEvent::GraphStarted {
            common: test_common(),
        };
        assert!(event.node().is_none());
    }

    #[test]
    fn node_accessor_returns_some_for_node_events() {
        let event = GraphExecutionEvent::NodeStarted {
            common: test_common(),
            node: test_node(),
        };
        let n = event.node().unwrap();
        assert_eq!(n.node_id, "node-1");
    }

    #[test]
    fn dispatch_accessor_returns_none_for_non_dispatch_events() {
        let event = GraphExecutionEvent::NodeStarted {
            common: test_common(),
            node: test_node(),
        };
        assert!(event.dispatch().is_none());
    }

    #[test]
    fn dispatch_accessor_returns_some_for_dispatch_events() {
        let event = GraphExecutionEvent::AgentStarted {
            common: test_common(),
            node: test_node(),
            dispatch: test_dispatch(),
            provider: "p".into(),
            model: "m".into(),
        };
        let d = event.dispatch().unwrap();
        assert_eq!(d.attempt_id, "attempt-1");
    }

    // ── Serde roundtrip ──────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip_all_variants() {
        let common = test_common();
        let node = test_node();
        let wave = test_wave();
        let dispatch = test_dispatch();
        let stats = test_stats();

        let events: Vec<GraphExecutionEvent> = vec![
            GraphExecutionEvent::GraphStarted {
                common: common.clone(),
            },
            GraphExecutionEvent::GraphCompleted {
                common: common.clone(),
                stats: stats.clone(),
            },
            GraphExecutionEvent::GraphFailed {
                common: common.clone(),
                stats: stats.clone(),
                error: "boom".into(),
            },
            GraphExecutionEvent::NodeStarted {
                common: common.clone(),
                node: node.clone(),
            },
            GraphExecutionEvent::AgentStarted {
                common: common.clone(),
                node: node.clone(),
                dispatch: dispatch.clone(),
                provider: "anthropic".into(),
                model: "claude-4".into(),
            },
            GraphExecutionEvent::WaveStarted {
                common: common.clone(),
                wave: wave.clone(),
            },
            GraphExecutionEvent::WaveCompleted {
                common: common.clone(),
                wave,
                elapsed_ms: 500,
            },
            GraphExecutionEvent::BudgetUpdated {
                common: common.clone(),
                amounts: test_budget(),
            },
            GraphExecutionEvent::DeliveryStarted {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "plan-a".into(),
                branch: "roko/plan-a".into(),
                publish: true,
            },
            GraphExecutionEvent::DeliveryStateAdvanced {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "plan-a".into(),
                from_state: "Prepared".into(),
                to_state: "Queued".into(),
                merge_commit: Some("abc123".into()),
                publication_ref: None,
            },
            GraphExecutionEvent::DeliveryCompleted {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "plan-a".into(),
                release_policy: "Delete".into(),
            },
            GraphExecutionEvent::DeliveryFailed {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "plan-a".into(),
                failure_state: "Conflict".into(),
                error: "merge conflict".into(),
                release_policy: "RetainForReview".into(),
            },
            GraphExecutionEvent::Gap {
                common,
                lost_count: 7,
            },
        ];

        for event in &events {
            let json = serde_json::to_string(event).expect("serialize");
            let deser: GraphExecutionEvent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(
                event.variant_name(),
                deser.variant_name(),
                "roundtrip mismatch for {}",
                event.variant_name()
            );
            assert_eq!(event.common().seq, deser.common().seq);
        }
    }

    // ── EventSeqCounter ──────────────────────────────────────────────────

    #[test]
    fn seq_counter_starts_at_1_and_increments() {
        let counter = EventSeqCounter::new();
        assert_eq!(counter.next(), 1);
        assert_eq!(counter.next(), 2);
        assert_eq!(counter.next(), 3);
    }

    // ── Helper constructors ──────────────────────────────────────────────

    #[test]
    fn make_common_sets_schema_version() {
        let counter = EventSeqCounter::new();
        let c = make_common("run-1", "graph-1", &counter);
        assert_eq!(c.schema_version, GRAPH_EVENT_SCHEMA_VERSION);
        assert_eq!(c.run_id, "run-1");
        assert_eq!(c.graph_id, "graph-1");
        assert_eq!(c.seq, 1);
    }

    #[test]
    fn make_terminal_stats_computes_elapsed_ms() {
        let s = make_terminal_stats(std::time::Duration::from_millis(1234), 5, 10);
        assert_eq!(s.elapsed_ms, 1234);
        assert_eq!(s.completed_nodes, 5);
        assert_eq!(s.total_nodes, 10);
    }

    // ── Budget uses u64, not f64 ─────────────────────────────────────────

    #[test]
    fn budget_amounts_are_integer_micro_usd() {
        let b = test_budget();
        // Verify the fields are u64 by doing integer arithmetic.
        let total = b.actual_micro_usd + b.remaining_micro_usd;
        assert_eq!(total, 1_000_000);
    }

    // ── Delivery event classification (#254) ─────────────────────────────

    #[test]
    fn delivery_events_are_reliable() {
        let common = test_common();
        let events = [
            GraphExecutionEvent::DeliveryStarted {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "p1".into(),
                branch: "b".into(),
                publish: true,
            },
            GraphExecutionEvent::DeliveryStateAdvanced {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "p1".into(),
                from_state: "Prepared".into(),
                to_state: "Queued".into(),
                merge_commit: None,
                publication_ref: None,
            },
            GraphExecutionEvent::DeliveryCompleted {
                common: common.clone(),
                delivery_id: "d1".into(),
                plan_id: "p1".into(),
                release_policy: "Delete".into(),
            },
            GraphExecutionEvent::DeliveryFailed {
                common,
                delivery_id: "d1".into(),
                plan_id: "p1".into(),
                failure_state: "Conflict".into(),
                error: "conflict".into(),
                release_policy: "RetainForReview".into(),
            },
        ];
        for event in &events {
            assert_eq!(
                event.delivery(),
                GraphEventDelivery::Reliable,
                "{} should be reliable",
                event.variant_name()
            );
        }
    }

    #[test]
    fn delivery_events_have_no_node_or_dispatch_fields() {
        let common = test_common();
        let event = GraphExecutionEvent::DeliveryStarted {
            common,
            delivery_id: "d1".into(),
            plan_id: "p1".into(),
            branch: "b".into(),
            publish: true,
        };
        assert!(event.node().is_none());
        assert!(event.dispatch().is_none());
    }
}
