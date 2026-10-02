//! ACP protocol types: errors, cognitive events, stream results.

use std::sync::{Arc, Mutex, OnceLock};

use thiserror::Error;

use crate::transport::TransportError;
use crate::types::{
    ContentBlock, INTERNAL_ERROR, INVALID_PARAMS, McpServerStatus, PermissionAction,
    PermissionDecision, PlanEntry, SESSION_BUDGET_EXCEEDED, SESSION_BUSY, SessionPromptResult,
    StopReason, ToolCallKind, ToolCallStatus, UsageInfo,
};

// ── Error types ──────────────────────────────────────────────────────

/// Errors produced while bridging cognitive events to ACP session updates.
#[derive(Debug, Error)]
pub enum BridgeEventsError {
    /// The target session already has an active prompt in flight.
    #[error("session '{0}' already has an active prompt")]
    SessionBusy(String),
    /// JSON serialization for an outbound session update failed.
    #[error("failed to serialize ACP session update: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Writing to the ACP stdio transport failed.
    #[error("failed to send ACP session update: {0}")]
    Transport(#[from] TransportError),
    /// The spawned cognitive task terminated unexpectedly.
    #[error("ACP cognitive task failed: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    /// A pipeline runner error.
    #[error("ACP pipeline error: {0}")]
    Pipeline(#[from] anyhow::Error),
    /// Prompt content is not supported by the selected model/dispatch path.
    #[error("unsupported ACP prompt content: {0}")]
    UnsupportedPromptContent(String),
    /// The persisted ACP session cost budget has been exhausted.
    #[error(
        "ACP session budget exceeded: spent ${accumulated_cost_usd:.6} of ${cost_budget_usd:.6} USD"
    )]
    BudgetExceeded {
        /// Configured session ceiling in USD.
        cost_budget_usd: f64,
        /// Persisted spend accumulated by completed efficiency events.
        accumulated_cost_usd: f64,
    },
}

impl BridgeEventsError {
    /// Returns a JSON-RPC error tuple when the failure maps to a client-visible ACP error.
    #[must_use]
    pub fn rpc_error(&self) -> Option<(i32, String)> {
        match self {
            Self::SessionBusy(session_id) => Some((
                SESSION_BUSY,
                format!("session '{session_id}' already has an active prompt"),
            )),
            Self::Serialize(e) => Some((INTERNAL_ERROR, format!("serialization error: {e}"))),
            Self::Transport(e) => Some((INTERNAL_ERROR, format!("transport error: {e}"))),
            Self::TaskJoin(e) => Some((INTERNAL_ERROR, format!("task failed: {e}"))),
            Self::Pipeline(e) => Some((INTERNAL_ERROR, format!("pipeline error: {e}"))),
            Self::UnsupportedPromptContent(message) => Some((INVALID_PARAMS, message.clone())),
            Self::BudgetExceeded {
                cost_budget_usd,
                accumulated_cost_usd,
            } => Some((
                SESSION_BUDGET_EXCEEDED,
                format!(
                    "ACP session budget exceeded: spent ${accumulated_cost_usd:.6} of ${cost_budget_usd:.6} USD; increase budget.max_plan_usd or start an unlimited session"
                ),
            )),
        }
    }
}

/// Result alias for ACP event bridge operations.
pub type Result<T> = std::result::Result<T, BridgeEventsError>;

pub(crate) static CASCADE_ROUTER_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
pub(crate) static EXPERIMENT_STORE_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

// ── Cognitive events ─────────────────────────────────────────────────

/// Events emitted by the cognitive loop and mapped to ACP session updates.
#[derive(Debug, Clone)]
pub enum CognitiveEvent {
    /// A streamed agent-visible text chunk.
    TokenChunk(String),
    /// A streamed internal reasoning chunk.
    ThinkingChunk(String),
    /// A tool call has started running.
    ToolCallStart {
        tool_call_id: String,
        title: String,
        kind: ToolCallKind,
        locations: Option<Vec<crate::types::ToolCallLocation>>,
    },
    /// A tool call has finished with rendered content.
    ToolCallComplete {
        tool_call_id: String,
        status: ToolCallStatus,
        content: Vec<ContentBlock>,
    },
    /// A plan update with structured entries (shown as progress in editor).
    PlanUpdate { entries: Vec<PlanEntry> },
    /// MCP server discovery results.
    McpStatus { statuses: Vec<McpServerStatus> },
    /// Prompt execution completed normally.
    Complete {
        stop_reason: StopReason,
        usage: Option<UsageInfo>,
    },
    /// Prompt execution failed before normal completion.
    Failure { message: String },
    /// Prompt execution stopped because the token budget was exhausted.
    MaxTokens,
    /// A spawned tool loop is requesting permission from the parent session.
    ///
    /// The reply channel carries a [`PermissionDecision`]; the parent loop
    /// sends exactly one, either with [`PermissionReplyChannel::reply`] or on
    /// the sender from [`PermissionReplyChannel::take_sender`].  If the
    /// channel is dropped without a reply, the tool loop should treat it as
    /// `PermissionDecision::Reject` (fail-closed).
    PermissionRequest {
        /// What the tool loop wants to do and why.
        payload: PermissionRequestPayload,
        /// One-shot reply channel back to the requesting tool loop.
        reply: PermissionReplyChannel,
    },
}

impl CognitiveEvent {
    /// Whether this event ends the turn. The editor waits for one, so these are
    /// never dropped.
    pub(crate) const fn ends_turn(&self) -> bool {
        matches!(
            self,
            Self::Complete { .. } | Self::Failure { .. } | Self::MaxTokens
        )
    }
}

/// Parameters describing what a tool loop wants permission to do.
#[derive(Debug, Clone)]
pub struct PermissionRequestPayload {
    /// The kind of action that needs authorisation (e.g. `FileEdit`, `TerminalCommand`).
    pub action: PermissionAction,
    /// Short human-readable title for the permission dialog (e.g. "Edit src/main.rs").
    pub title: String,
    /// Longer description of why the action is needed.
    pub detail: String,
}

/// Clone-safe wrapper around a `oneshot::Sender<PermissionDecision>`.
///
/// Because `oneshot::Sender` is not `Clone`, the sender is held behind
/// `Arc<Mutex<Option<…>>>`.  The first call to [`reply`](Self::reply)
/// takes the sender and sends the decision.  Subsequent calls (or calls
/// after a clone) return `false`.
#[derive(Debug, Clone)]
pub struct PermissionReplyChannel {
    inner: Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<PermissionDecision>>>>,
}

impl PermissionReplyChannel {
    /// Create a new reply channel from a raw oneshot sender.
    pub fn new(sender: tokio::sync::oneshot::Sender<PermissionDecision>) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(Some(sender))),
        }
    }

    /// Send a decision back to the requesting tool loop.
    ///
    /// Returns `true` if the decision was delivered, `false` if the
    /// channel was already consumed or the receiver was dropped.
    pub fn reply(&self, decision: PermissionDecision) -> bool {
        match self.take_sender() {
            Some(tx) => tx.send(decision).is_ok(),
            None => false,
        }
    }

    /// Takes the sender out of the channel, consuming it. A waiter holds the
    /// sender to await `closed()`, which resolves when the requesting tool stops
    /// waiting, without keeping the lock. `None` if the channel was consumed.
    pub fn take_sender(&self) -> Option<tokio::sync::oneshot::Sender<PermissionDecision>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    /// Returns `true` if the reply channel has already been consumed.
    pub fn is_consumed(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_none()
    }
}

// ── Stream events → editor ───────────────────────────────────────────

/// Result of streaming events: the prompt result, accumulated assistant text,
/// and any provider-reported usage.
pub struct StreamResult {
    pub prompt_result: SessionPromptResult,
    /// Accumulated assistant text from TokenChunk events.
    pub assistant_text: String,
    /// Usage reported by the provider, if any.
    pub usage: Option<UsageInfo>,
}
