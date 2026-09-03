//! Typed streaming events for provider adapters and tool loops.

use crate::tool_loop::{StreamEvent, StreamEventKind};
use crate::translate::{normalize_finish_reason, openai::parse_usage};
use crate::usage::Usage;
use serde_json::Value;

/// Provider-neutral stream event covering both OpenAI SSE and Claude CLI protocols.
#[derive(Debug, Clone)]
pub enum UnifiedStreamEvent {
    /// Incremental content text.
    ContentDelta(String),
    /// Incremental reasoning/thinking text.
    ReasoningDelta(String),
    /// Tool call information.
    ToolCall {
        /// Provider-assigned tool call identifier.
        id: String,
        /// Tool/function name.
        name: String,
        /// JSON argument text.
        arguments: String,
    },
    /// Token usage accounting.
    Usage {
        /// Input/prompt tokens.
        input_tokens: u64,
        /// Output/completion tokens.
        output_tokens: u64,
    },
    /// Stream completed successfully.
    Done,
    /// Stream error.
    Error(String),
    /// System/init event (session info, model announcement).
    SystemInit {
        /// Provider session id.
        session_id: String,
        /// Effective model name.
        model: String,
    },
}

impl UnifiedStreamEvent {
    /// Try to convert an [`AgentRuntimeEvent`](crate::runtime_events::AgentRuntimeEvent)
    /// into a [`UnifiedStreamEvent`].
    ///
    /// Returns `None` for events that do not map to provider-neutral stream
    /// output, such as tool results or lifecycle start events.
    #[must_use]
    pub fn try_from_runtime_event(event: crate::runtime_events::AgentRuntimeEvent) -> Option<Self> {
        use crate::runtime_events::AgentRuntimeEvent;

        match event {
            AgentRuntimeEvent::SystemInit { session_id, model } => {
                Some(Self::SystemInit { session_id, model })
            }
            AgentRuntimeEvent::MessageDelta { text } => Some(Self::ContentDelta(text)),
            AgentRuntimeEvent::ToolCall { id, name } => Some(Self::ToolCall {
                id,
                name,
                arguments: String::new(),
            }),
            AgentRuntimeEvent::TokenUsage {
                input_tokens,
                output_tokens,
                ..
            } => Some(Self::Usage {
                input_tokens,
                output_tokens,
            }),
            AgentRuntimeEvent::TurnCompleted { is_error, .. } => {
                if is_error {
                    Some(Self::Error("agent turn completed with error".to_string()))
                } else {
                    Some(Self::Done)
                }
            }
            AgentRuntimeEvent::Error { message } => Some(Self::Error(message)),
            AgentRuntimeEvent::Started { .. }
            | AgentRuntimeEvent::ToolOutput { .. }
            | AgentRuntimeEvent::Exited { .. } => None,
        }
    }

    /// Convert a [`StreamEvent`] into a [`UnifiedStreamEvent`].
    #[must_use]
    pub fn from_stream_event(event: StreamEvent) -> Option<Self> {
        match event.kind {
            StreamEventKind::TextDelta(text) => Some(Self::ContentDelta(text)),
            StreamEventKind::ReasoningDelta(text) => Some(Self::ReasoningDelta(text)),
            StreamEventKind::ToolCallStart { id, name } => Some(Self::ToolCall {
                id,
                name,
                arguments: String::new(),
            }),
            StreamEventKind::ToolCallDelta { id: _, json_fragment } => {
                // Deltas carry partial arguments; map to an empty-named ToolCall
                // for accumulators that need argument fragments.
                Some(Self::ToolCall {
                    id: String::new(),
                    name: String::new(),
                    arguments: json_fragment,
                })
            }
            StreamEventKind::ToolCallEnd { id, name, args } => Some(Self::ToolCall {
                id,
                name,
                arguments: args.to_string(),
            }),
            StreamEventKind::Usage(usage) => Some(Self::Usage {
                input_tokens: u64::from(usage.input_tokens),
                output_tokens: u64::from(usage.output_tokens),
            }),
            StreamEventKind::Done { .. } => Some(Self::Done),
        }
    }
}

/// Unified trait for parsing streaming JSON lines from any LLM provider.
///
/// Each provider's wire format is different (OpenAI uses SSE `data:` prefixed
/// lines, Claude CLI uses bare JSON-Lines), but both produce sequences of
/// typed events. This trait normalizes the parsing interface.
pub trait StreamJsonParser: Send + Sync {
    /// Parse a single line of streaming output into zero or more events.
    ///
    /// Returns an empty vec for keep-alive lines, comment lines, or
    /// lines that don't produce actionable events.
    fn parse_line(&self, line: &str) -> Vec<UnifiedStreamEvent>;

    /// Human-readable name of this parser (for diagnostics).
    fn parser_name(&self) -> &str;
}

/// Parser for OpenAI-compatible SSE streams (`data: {...}` lines).
///
/// Wraps the existing [`parse_sse_line`] function and converts
/// [`StreamEvent`] values into [`UnifiedStreamEvent`].
pub struct OpenAiSseParser;

impl StreamJsonParser for OpenAiSseParser {
    fn parse_line(&self, line: &str) -> Vec<UnifiedStreamEvent> {
        match parse_sse_line(line) {
            Some(event) => {
                if let Some(unified) = UnifiedStreamEvent::from_stream_event(event) {
                    vec![unified]
                } else {
                    Vec::new()
                }
            }
            None => Vec::new(),
        }
    }

    fn parser_name(&self) -> &str {
        "openai-sse"
    }
}

/// Parser for Claude CLI `--output-format stream-json` lines.
///
/// Wraps the existing `parse_stream_line()` function and translates
/// `AgentRuntimeEvent` variants into [`UnifiedStreamEvent`].
pub struct ClaudeCliParser;

impl StreamJsonParser for ClaudeCliParser {
    fn parse_line(&self, line: &str) -> Vec<UnifiedStreamEvent> {
        use crate::provider::claude_cli::stream::parse_stream_line;

        parse_stream_line(line)
            .into_iter()
            .filter_map(UnifiedStreamEvent::try_from_runtime_event)
            .collect()
    }

    fn parser_name(&self) -> &str {
        "claude-cli"
    }
}

/// Parse a single OpenAI-compatible SSE line into a canonical stream event.
///
/// Returns a [`StreamEvent`] ready for direct use with [`crate::tool_loop::collect_stream_to_response`]
/// and the `stream_turn` API.
#[must_use]
pub fn parse_sse_line(line: &str) -> Option<StreamEvent> {
    let line = line.strip_prefix("data:")?.trim_start();
    if line == "[DONE]" {
        return Some(StreamEvent::now(StreamEventKind::Done {
            finish_reason: "stop".to_string(),
        }));
    }

    let json: Value = serde_json::from_str(line).ok()?;
    let delta = json.pointer("/choices/0/delta").unwrap_or(&Value::Null);

    // GLM streams reasoning before content, so surface that first.
    if let Some(reasoning) = delta.get("reasoning_content").and_then(Value::as_str) {
        return Some(StreamEvent::now(StreamEventKind::ReasoningDelta(
            reasoning.to_string(),
        )));
    }
    if let Some(content) = delta.get("content").and_then(Value::as_str) {
        return Some(StreamEvent::now(StreamEventKind::TextDelta(
            content.to_string(),
        )));
    }
    if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
        for tc in tool_calls {
            let id = tc
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_default();
            let name = tc
                .pointer("/function/name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_default();
            let arguments = tc
                .pointer("/function/arguments")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            // When both id and name are present, this is a tool call start.
            if !id.is_empty() || !name.is_empty() {
                return Some(StreamEvent::now(StreamEventKind::ToolCallStart {
                    id,
                    name,
                }));
            }
            // Otherwise it's a delta with partial arguments.
            return Some(StreamEvent::now(StreamEventKind::ToolCallDelta {
                id: String::new(),
                json_fragment: arguments,
            }));
        }
    }
    if json.get("usage").is_some() {
        return Some(StreamEvent::now(StreamEventKind::Usage(parse_usage(&json))));
    }
    if let Some(reason) = json
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
    {
        let finish_reason = normalize_finish_reason(reason);
        return Some(StreamEvent::now(StreamEventKind::Done {
            finish_reason: format!("{finish_reason:?}"),
        }));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::parse_sse_line;
    use crate::tool_loop::StreamEventKind;

    #[test]
    fn sse_parser_reads_reasoning_delta() {
        let event = parse_sse_line(
            r#"data: {"choices":[{"delta":{"reasoning_content":"Need to inspect the file."}}]}"#,
        );

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::ReasoningDelta(reasoning)) if reasoning == "Need to inspect the file."
        ));
    }

    #[test]
    fn sse_parser_reads_content_delta() {
        let event =
            parse_sse_line(r#"data: {"choices":[{"delta":{"content":"I can answer now."}}]}"#);

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::TextDelta(content)) if content == "I can answer now."
        ));
    }

    #[test]
    fn sse_parser_reads_tool_call_start() {
        let event = parse_sse_line(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"call_glm_","function":{"name":"edit_file","arguments":"{\"path\":\"note.txt\"}"}}]}}]}"#,
        );

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::ToolCallStart { id, name })
                if id == "call_glm_" && name == "edit_file"
        ));
    }

    #[test]
    fn sse_parser_reads_usage() {
        let event = parse_sse_line(
            r#"data: {"choices":[],"usage":{"prompt_tokens":21,"completion_tokens":9,"prompt_tokens_details":{"cached_tokens":4}}}"#,
        );

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::Usage(usage))
                if usage.input_tokens == 21
                    && usage.output_tokens == 9
                    && usage.cache_read_tokens == 4
        ));
    }

    #[test]
    fn sse_parser_reads_finish_reason() {
        let event =
            parse_sse_line(r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#);

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::Done { finish_reason }) if finish_reason == "ToolCalls"
        ));
    }

    #[test]
    fn sse_parser_reads_done_marker() {
        let event = parse_sse_line("data: [DONE]");

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::Done { finish_reason }) if finish_reason == "stop"
        ));
    }

    #[test]
    fn sse_parser_ignores_non_data_lines() {
        assert!(parse_sse_line("event: message").is_none());
    }
}
