//! Core types for the inline chat session.

use std::time::Instant;

use crate::auth_detect::AuthMethod;
use crate::dispatch_v2::DispatchResult;
use crate::inline::primitives::{CostMeter, StreamingState};

use crate::chat_session::ChatAgentSession;
use roko_learn::cost_table::CostTable;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Chat session phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Waiting for user input.
    Input,
    /// Sent message, waiting for first token.
    Thinking,
    /// Receiving streaming tokens.
    Streaming,
    /// Dispatch error — shows [r]etry / [s]witch / [q]uit options.
    Error { prompt: String, error: String },
    /// Session complete (user pressed Ctrl-D or /quit).
    Done,
}

/// All available slash commands for tab-completion.
pub(crate) const SLASH_COMMANDS: &[(&str, &str)] = &[
    // Session & display
    ("/help", "show available commands"),
    ("/version", "show version info"),
    ("/stats", "detailed session statistics"),
    ("/context", "show session context"),
    ("/history", "show conversation turns"),
    ("/input-history", "show typed input history"),
    ("/copy", "copy last response to clipboard"),
    ("/compact", "toggle compact output mode"),
    ("/quiet", "toggle per-turn usage summary"),
    ("/system", "set system message for session"),
    ("/reset", "clear conversation, fresh start"),
    ("/retry", "resend the last message"),
    ("/export", "export conversation (markdown/json)"),
    ("/cost", "session cost summary"),
    ("/tools", "list available tools"),
    ("/mcp", "show MCP config status"),
    ("/clear", "clear scrollback"),
    ("/quit", "exit the chat"),
    ("/exit", "exit the chat"),
    // Configuration
    ("/config", "show or set configuration"),
    ("/config providers", "list configured providers"),
    ("/config models", "list available models"),
    ("/config gates", "show gate configuration"),
    ("/model", "show or change model"),
    ("/provider", "show current auth/provider"),
    ("/auth", "show current auth/provider"),
    ("/effort", "set effort level (low/med/high/max)"),
    // Workspace & git
    ("/status", "workspace status"),
    ("/doctor", "health check"),
    ("/diff", "show git diff"),
    ("/git", "git status"),
    ("/log", "recent git commits"),
    ("/branch", "show current branch"),
    ("/changes", "changed files since last commit"),
    // File operations
    ("/file", "read and display a file"),
    ("/search", "grep workspace for pattern"),
    ("/find", "find files matching pattern"),
    ("/tree", "show directory tree"),
    // Agent & workflow
    ("/agent", "show/switch agent identity"),
    ("/agent list", "list configured agents"),
    ("/run", "execute prompt through universal loop"),
    ("/plan list", "list plans"),
    ("/plan run", "execute a plan"),
    ("/plan generate", "generate plan from prompt"),
    ("/gate", "toggle gates (compile/test/clippy)"),
    // Research
    ("/research", "research a topic"),
    // Knowledge & learning
    ("/knowledge", "query knowledge store"),
    ("/learn", "show learning state"),
];

/// How the session dispatches prompts.
#[derive(Debug, Clone)]
pub(crate) enum DispatchMode {
    /// HTTP backend (sidecar or serve).
    Http {
        client: reqwest::Client,
        backend_url: String,
        is_sidecar: bool,
    },
    /// Deprecated direct fallback. Kept only to make stale paths fail visibly.
    #[allow(dead_code)]
    Direct { auth: AuthMethod },
    /// Full agent session with system prompt, tools, MCP, safety.
    Session,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ChatInlineDispatchError {
    #[error(
        "interactive chat config load failed; refusing to fall back to deprecated dispatch_direct path: {source:#}"
    )]
    ConfigLoad {
        #[source]
        source: anyhow::Error,
    },
    #[error(
        "interactive chat model resolution failed; refusing to fall back to deprecated dispatch_direct path: {source:#}"
    )]
    ModelSelection {
        #[source]
        source: crate::model_selection::Error,
    },
    #[error(
        "ChatAgentSession initialization failed; refusing to fall back to deprecated dispatch_direct path: {source:#}"
    )]
    ChatSessionInit {
        #[source]
        source: anyhow::Error,
    },
    #[error("direct inline dispatch is disabled; ChatAgentSession is required for chat turns")]
    DirectDispatchDisabled,
}

/// A recorded conversation message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ConversationMessage {
    pub role: String, // "user" or "assistant"
    pub text: String,
    pub timestamp: String,
}

/// Serializable session snapshot for auto-save/resume.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct SessionSnapshot {
    pub turn_count: u32,
    pub total_cost: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model: String,
    pub agent_id: String,
    pub messages: Vec<ConversationMessage>,
    pub system_message: Option<String>,
    pub saved_at: String,
    pub first_user_message: Option<String>,
}

/// Full chat session state.
pub(crate) struct ChatSession {
    pub phase: Phase,
    pub input: super::input::InputState,
    pub streaming: StreamingState,
    pub cost: CostMeter,
    pub cost_table: CostTable,
    pub agent_id: String,
    pub tick: u64,
    pub started_at: Instant,
    pub dispatch: DispatchMode,
    /// Channel for receiving async responses from background dispatch calls.
    pub response_rx: Option<tokio::sync::mpsc::Receiver<Result<DispatchResult, String>>>,
    /// Channel for receiving live streaming events from the Session dispatch path.
    pub streaming_event_rx: Option<tokio::sync::mpsc::Receiver<roko_agent::AgentRuntimeEvent>>,
    /// Number of completed user-to-agent exchanges.
    pub turn_count: u32,
    /// When the current thinking phase began (for animated labels).
    pub thinking_started: Option<Instant>,
    /// Conversation transcript for export.
    pub conversation: Vec<ConversationMessage>,
    /// Last submitted prompt (for retry on error).
    pub last_prompt: Option<String>,
    /// Persistent system message for this session.
    pub system_message: Option<String>,
    /// Compact output mode.
    pub compact: bool,
    /// Suppress per-turn usage summary.
    pub quiet: bool,
    /// Full agent session (present when dispatch == `DispatchMode::Session`).
    pub agent_session: Option<ChatAgentSession>,
    /// When the last Ctrl-C was pressed (for double-tap exit).
    pub last_ctrl_c: Option<Instant>,
}

/// Reason a `/model` switch was rejected, with an optional hint.
pub(crate) struct ModelSwitchError {
    pub message: String,
    pub hint: Option<String>,
}

impl ModelSwitchError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }
}
