//! Inline interactive chat REPL.
//!
//! Split into focused sub-modules:
//! - `types` -- core types (`Phase`, `ChatSession`, `DispatchMode`, etc.)
//! - `input` -- input buffer, completion dropdown, history search, command palette
//! - `session` -- session construction, persistence, model helpers
//! - `commands` -- slash command handling
//! - `dispatch` -- prompt dispatch and backend HTTP communication
//! - `render` -- viewport rendering
//! - `output` -- scrollback output helpers (agent responses, tool outputs, errors)
//! - `event_loop` -- entry points and main event loop

mod commands;
mod decompose;
mod dispatch;
mod event_loop;
mod input;
mod output;
mod render;
mod session;
mod types;

// Public API -- these are the only two entry points used externally.
pub use event_loop::{run_chat_inline, run_unified_inline};

// Re-exports consumed only by the #[cfg(test)] module below.
#[cfg(test)]
pub(crate) use dispatch::dispatch_prompt;
#[cfg(test)]
pub(crate) use input::{CommandPalette, CompletionState, HistorySearch, InputState, fuzzy_match};
#[cfg(test)]
pub(crate) use output::{error_suggestions, reading_time};
#[cfg(test)]
pub(crate) use session::{
    active_model_name, apply_model_switch, session_banner_label, thinking_label, truncate_str,
    turn_result_to_dispatch_result,
};
#[cfg(test)]
pub(crate) use types::{ChatInlineDispatchError, ChatSession, DispatchMode, Phase, SLASH_COMMANDS};

#[cfg(test)]
mod tests;
