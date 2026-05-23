//! Inspector for Hermes's custom SSE event `hermes.tool.progress`.
//!
//! Hermes Chat Completions streams emit standard `chat.completion.chunk`
//! events plus this custom event when a tool starts running. The
//! existing `OpenAiCompatLlmBackend` SSE parser emits nothing for
//! non-spec events. The inspector intercepts those and converts them
//! to `StreamChunk::ToolProgress` for surfacing in the TUI / dashboard.
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

use crate::streaming::StreamChunk;

/// The SSE event name that Hermes uses for tool progress updates.
pub const HERMES_TOOL_PROGRESS_EVENT: &str = "hermes.tool.progress";

/// Inspects `hermes.tool.progress` SSE events and converts them to
/// `StreamChunk::ToolProgress`.
pub struct ToolProgressInspector;

impl ToolProgressInspector {
    /// Inspect a named SSE event. Returns `Some(StreamChunk::ToolProgress)`
    /// for `hermes.tool.progress` events, `None` for everything else.
    pub fn inspect(&self, event_name: &str, data: &serde_json::Value) -> Option<StreamChunk> {
        if event_name == HERMES_TOOL_PROGRESS_EVENT {
            Some(StreamChunk::ToolProgress {
                tool: data["tool"].as_str().unwrap_or("").to_string(),
                status: data["status"].as_str().unwrap_or("").to_string(),
            })
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

        let chunk = inspector.inspect("hermes.tool.progress", &data);
        assert!(chunk.is_some());
        match chunk.unwrap() {
            StreamChunk::ToolProgress { tool, status } => {
                assert_eq!(tool, "terminal");
                assert_eq!(status, "start");
            }
            other => panic!("expected ToolProgress, got {other:?}"),
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

        let chunk = inspector.inspect("hermes.tool.progress", &data);
        match chunk.unwrap() {
            StreamChunk::ToolProgress { tool, status } => {
                assert!(tool.is_empty());
                assert!(status.is_empty());
            }
            other => panic!("expected ToolProgress, got {other:?}"),
        }
    }
}
