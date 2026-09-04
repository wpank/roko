//! Inspector for Hermes's custom SSE event `hermes.tool.progress`.
//!
//! Hermes Chat Completions streams emit standard `chat.completion.chunk`
//! events plus this custom event when a tool starts running. The
//! existing `OpenAiCompatLlmBackend` SSE parser emits nothing for
//! non-spec events. The inspector intercepts those and converts them
//! to `StreamEvent::TextDelta` for surfacing in the TUI / dashboard.
//!
//! This is the only Hermes-specific protocol code in v1. Everything
//! else is plain OpenAI Chat Completions.
//!
//! # SSE format
//!
//! ```text
//! event: hermes.tool.progress
//! data: {"tool": "terminal", "status": "start", "args": {"command": "ls"}}
//! ```

use crate::tool_loop::{StreamEvent, StreamEventKind};

/// The SSE event name that Hermes uses for tool progress updates.
pub const HERMES_TOOL_PROGRESS_EVENT: &str = "hermes.tool.progress";

/// Inspects `hermes.tool.progress` SSE events and converts them to
/// informational `StreamEvent::TextDelta` events.
pub struct ToolProgressInspector;

impl ToolProgressInspector {
    /// Inspect a named SSE event. Returns `Some(StreamEvent)` containing
    /// a `TextDelta` with tool progress info for `hermes.tool.progress`
    /// events, `None` for everything else.
    pub fn inspect(&self, event_name: &str, data: &serde_json::Value) -> Option<StreamEvent> {
        if event_name == HERMES_TOOL_PROGRESS_EVENT {
            let tool = data["tool"].as_str().unwrap_or("").to_string();
            let status = data["status"].as_str().unwrap_or("").to_string();
            Some(StreamEvent::now(StreamEventKind::TextDelta(format!(
                "[{tool}] {status}"
            ))))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recognizes_tool_progress_event() {
        let inspector = ToolProgressInspector;
        let data = json!({
            "tool": "terminal",
            "status": "start",
            "args": {"command": "ls -la"}
        });

        let event = inspector.inspect("hermes.tool.progress", &data);
        assert!(event.is_some());
        match &event.unwrap().kind {
            StreamEventKind::TextDelta(text) => {
                assert!(text.contains("terminal"));
                assert!(text.contains("start"));
            }
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[test]
    fn ignores_unknown_events() {
        let inspector = ToolProgressInspector;
        let data = json!({"foo": "bar"});

        assert!(inspector.inspect("some.other.event", &data).is_none());
    }

    #[test]
    fn handles_missing_fields_gracefully() {
        let inspector = ToolProgressInspector;
        let data = json!({});

        let event = inspector.inspect("hermes.tool.progress", &data);
        match &event.unwrap().kind {
            StreamEventKind::TextDelta(text) => {
                assert_eq!(text, "[] ");
            }
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }
}
