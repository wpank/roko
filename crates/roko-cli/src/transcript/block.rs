//! Semantic block model for transcript rendering.
//!
//! A [`TranscriptBlock`] is the unit that renderers (inline or TUI) iterate
//! over. Each variant carries exactly the data needed for display without
//! requiring the renderer to know about provider wire formats.

use serde::{Deserialize, Serialize};

// ─── ToolBlockStatus ────────────────────────────────────────────────────

/// Display-oriented status for a tool call block.
///
/// This is a simplified projection of [`roko_core::tool::transcript::ToolLifecycleStatus`]
/// for rendering; the full lifecycle enum carries admission and execution
/// states that renderers do not need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolBlockStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
    Denied,
}

impl ToolBlockStatus {
    /// Whether this status represents a terminal (settled) state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending | Self::Running)
    }

    /// Whether the tool call ended in an error-like state.
    #[must_use]
    pub const fn is_error(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::TimedOut | Self::Cancelled | Self::Denied
        )
    }
}

// ─── SubagentBlockStatus ────────────────────────────────────────────────

/// Display-oriented status for a subagent block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentBlockStatus {
    Starting,
    Running,
    Completed,
    Failed,
}

impl SubagentBlockStatus {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

// ─── MessageLevel ───────────────────────────────────────────────────────

/// Severity level for system messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
}

// ─── TranscriptBlock ────────────────────────────────────────────────────

/// A semantic block for transcript rendering.
///
/// Each variant represents one logical unit that a renderer can display.
/// Blocks are produced by [`super::convert::blocks_from_records`] from the
/// raw [`TranscriptRecord`] stream and consumed by both the inline CLI
/// and the TUI transcript panes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "block_type", rename_all = "snake_case")]
pub enum TranscriptBlock {
    /// Accumulated assistant text (may still be streaming).
    AssistantText { text: String, is_streaming: bool },

    /// Extended thinking / reasoning content.
    Reasoning { text: String, is_streaming: bool },

    /// A tool invocation with its lifecycle.
    ToolCall {
        /// Provider-assigned call ID (correlation key).
        call_id: String,
        /// Canonical tool name.
        tool_name: String,
        /// Short preview of arguments (e.g. file path, command).
        arguments_preview: Option<String>,
        /// Current display status.
        status: ToolBlockStatus,
        /// Execution wall time in milliseconds.
        duration_ms: Option<u64>,
        /// Truncated result preview for collapsed rendering.
        result_preview: Option<String>,
        /// Error message if the tool failed.
        error: Option<String>,
        /// Whether the result was truncated for display.
        truncated: bool,
        /// Whether the result was redacted by policy.
        redacted: bool,
    },

    /// A todo/task list update from the provider.
    TodoUpdate {
        todo_id: String,
        title: String,
        status: String,
        progress: Option<f64>,
    },

    /// A subagent invocation with nested child blocks.
    SubagentBlock {
        agent_id: String,
        agent_name: String,
        status: SubagentBlockStatus,
        children: Vec<TranscriptBlock>,
    },

    /// A system-level message (info, warning, error).
    SystemMessage { level: MessageLevel, text: String },

    /// Token usage report.
    UsageReport {
        input_tokens: u64,
        output_tokens: u64,
        cache_tokens: Option<u64>,
    },

    /// The active provider/model changed mid-run.
    ProviderChange {
        from: String,
        to: String,
        reason: String,
    },
}

impl TranscriptBlock {
    /// Returns the block type as a stable string for filtering.
    #[must_use]
    pub fn block_type(&self) -> &'static str {
        match self {
            Self::AssistantText { .. } => "assistant_text",
            Self::Reasoning { .. } => "reasoning",
            Self::ToolCall { .. } => "tool_call",
            Self::TodoUpdate { .. } => "todo_update",
            Self::SubagentBlock { .. } => "subagent",
            Self::SystemMessage { .. } => "system_message",
            Self::UsageReport { .. } => "usage_report",
            Self::ProviderChange { .. } => "provider_change",
        }
    }

    /// Check if this block contains `needle` (case-insensitive).
    #[must_use]
    pub fn contains_text(&self, needle: &str) -> bool {
        let needle_lower = needle.to_ascii_lowercase();
        match self {
            Self::AssistantText { text, .. } | Self::Reasoning { text, .. } => {
                text.to_ascii_lowercase().contains(&needle_lower)
            }
            Self::ToolCall {
                tool_name,
                arguments_preview,
                result_preview,
                error,
                ..
            } => {
                tool_name.to_ascii_lowercase().contains(&needle_lower)
                    || arguments_preview
                        .as_deref()
                        .map_or(false, |s| s.to_ascii_lowercase().contains(&needle_lower))
                    || result_preview
                        .as_deref()
                        .map_or(false, |s| s.to_ascii_lowercase().contains(&needle_lower))
                    || error
                        .as_deref()
                        .map_or(false, |s| s.to_ascii_lowercase().contains(&needle_lower))
            }
            Self::TodoUpdate { title, status, .. } => {
                title.to_ascii_lowercase().contains(&needle_lower)
                    || status.to_ascii_lowercase().contains(&needle_lower)
            }
            Self::SubagentBlock {
                agent_name,
                children,
                ..
            } => {
                agent_name.to_ascii_lowercase().contains(&needle_lower)
                    || children.iter().any(|c| c.contains_text(needle))
            }
            Self::SystemMessage { text, .. } => text.to_ascii_lowercase().contains(&needle_lower),
            Self::UsageReport { .. } => false,
            Self::ProviderChange {
                from, to, reason, ..
            } => {
                from.to_ascii_lowercase().contains(&needle_lower)
                    || to.to_ascii_lowercase().contains(&needle_lower)
                    || reason.to_ascii_lowercase().contains(&needle_lower)
            }
        }
    }
}
