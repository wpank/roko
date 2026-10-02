//! Typed streaming events for provider adapters and tool loops.

use crate::tool_loop::{StreamEvent, StreamEventKind};
use crate::translate::{normalize_finish_reason, openai::parse_usage};
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
    /// The output of a completed tool call (correlates with a `ToolCallEnd`).
    ///
    /// Produced when a provider surfaces tool results on the stream.
    /// Most providers do not emit this; the ToolLoop injects results directly.
    ToolOutput {
        /// Provider-assigned tool call identifier.
        id: String,
        /// The text output returned by the tool.
        output: String,
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
            StreamEventKind::ToolCallStart { id, name } => {
                // Strip the index-key prefix if present (format: "__idx_N\0real_id").
                let clean_id = id
                    .find('\0')
                    .map(|sep| id[sep + 1..].to_string())
                    .unwrap_or(id);
                Some(Self::ToolCall {
                    id: clean_id,
                    name,
                    arguments: String::new(),
                })
            }
            StreamEventKind::ToolCallDelta {
                id: _,
                json_fragment,
            } => {
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
            StreamEventKind::ToolResult { id, output, .. } => Some(Self::ToolOutput { id, output }),
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
        parse_sse_line(line)
            .into_iter()
            .filter_map(UnifiedStreamEvent::from_stream_event)
            .collect()
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

/// The finish reason of a stream that ended without naming one. It
/// normalises to `FinishReason::Error("unknown")`, never to `Stop`, so a
/// cut connection or a missing finish reason does not read as a normal end
/// (backlog 1111).
pub const UNKNOWN_FINISH_REASON: &str = "unknown";

/// One parsed line of an OpenAI-compatible SSE stream.
#[derive(Debug)]
pub enum SseLine {
    /// The events a chunk carries: none for a non-`data:` line, a payload
    /// that is not JSON, or a chunk with nothing new.
    Events(Vec<StreamEvent>),
    /// The `[DONE]` terminator. It names no finish reason.
    Done,
    /// A provider `error` object sent in place of a chunk.
    Error(SseError),
}

/// A provider `error` object in an SSE stream (`data: {"error": {...}}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseError {
    /// `error.code`, as text: some providers send a number, some a name.
    pub code: Option<String>,
    /// `error.type`.
    pub error_type: Option<String>,
    /// `error.message`, or the raw error when it has none.
    pub message: String,
}

impl SseError {
    /// `error.code` read as an HTTP status, when it is one.
    #[must_use]
    pub fn status(&self) -> Option<u16> {
        self.code
            .as_deref()?
            .parse::<u16>()
            .ok()
            .filter(|status| (400..=599).contains(status))
    }
}

impl std::fmt::Display for SseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("provider stream error")?;
        if let Some(error_type) = &self.error_type {
            write!(f, " type={error_type}")?;
        }
        if let Some(code) = &self.code {
            write!(f, " code={code}")?;
        }
        write!(f, ": {}", self.message)
    }
}

/// Parse one SSE line of an OpenAI-compatible stream, telling a chunk's
/// events from the `[DONE]` terminator and from a provider error object.
///
/// A chunk yields every event it carries, in order: reasoning, content, one
/// tool-call start or delta per `tool_calls` element, usage, then the finish
/// reason (backlog 1110). Every event keeps the model and session ids its
/// chunk named.
pub fn parse_sse_frame(line: &str) -> SseLine {
    // Strip "data:" prefix; a non-data: line carries no events.
    let Some(rest) = line.strip_prefix("data:") else {
        return SseLine::Events(Vec::new());
    };
    // Strip exactly one leading space per RFC 8895 §9.2.6, matching the
    // shared sse::extract_sse_data / strip_one_space behaviour.
    let value = rest.strip_prefix(' ').unwrap_or(rest);

    if value == "[DONE]" {
        return SseLine::Done;
    }

    let Ok(json) = serde_json::from_str::<Value>(value) else {
        return SseLine::Events(Vec::new());
    };
    if let Some(error) = sse_error(&json) {
        return SseLine::Error(error);
    }
    // Each chunk names the model that serves it (bug-bfd241), and some name
    // the response, session and thread ids.
    let model = json
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string);
    let session = crate::tool_loop::session_ids(&json);
    SseLine::Events(
        parse_sse_chunk(&json)
            .into_iter()
            .map(|event| {
                event
                    .with_model(model.clone())
                    .with_session(session.clone())
            })
            .collect(),
    )
}

/// Parse one SSE line of an OpenAI-compatible stream into its events, for a
/// caller that keeps no state across lines (see [`parse_sse_frame`]).
///
/// `[DONE]` yields `Done` with [`UNKNOWN_FINISH_REASON`]: the terminator
/// names no reason, and collecting the stream never lets `unknown` replace
/// a reason a chunk named. An error object yields nothing here; read it with
/// [`parse_sse_frame`].
pub fn parse_sse_line(line: &str) -> Vec<StreamEvent> {
    match parse_sse_frame(line) {
        SseLine::Events(events) => events,
        SseLine::Done => vec![StreamEvent::now(StreamEventKind::Done {
            finish_reason: UNKNOWN_FINISH_REASON.to_string(),
        })],
        SseLine::Error(_) => Vec::new(),
    }
}

/// The provider error object a chunk carries, if any: a non-null `error`
/// field, either an object with `message`, `type` and `code`, or a string.
fn sse_error(json: &Value) -> Option<SseError> {
    let error = json.get("error").filter(|error| !error.is_null())?;
    let text = |value: Option<&Value>| match value {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    };
    let message = match error {
        Value::String(message) => message.clone(),
        _ => text(error.get("message")).unwrap_or_else(|| error.to_string()),
    };
    Some(SseError {
        code: text(error.get("code")),
        error_type: text(error.get("type")),
        message,
    })
}

/// The events of an OpenAI-compatible stream chunk, parsed from its JSON.
///
/// A chunk can carry several fields at once: Z.ai sends its usage and
/// finish reason next to an empty `content`, and a tool call can share a
/// chunk with text or reasoning. None of them is dropped.
fn parse_sse_chunk(json: &Value) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    let delta = json.pointer("/choices/0/delta").unwrap_or(&Value::Null);

    // GLM streams reasoning before content, so surface that first.
    if let Some(reasoning) = delta.get("reasoning_content").and_then(Value::as_str)
        && !reasoning.is_empty()
    {
        events.push(StreamEvent::now(StreamEventKind::ReasoningDelta(
            reasoning.to_string(),
        )));
    }
    if let Some(content) = delta.get("content").and_then(Value::as_str)
        && !content.is_empty()
    {
        events.push(StreamEvent::now(StreamEventKind::TextDelta(
            content.to_string(),
        )));
    }
    if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
        events.extend(tool_calls.iter().map(tool_call_event));
    }
    // OpenAI sends `"usage": null` on every chunk but the last when usage is
    // requested; only a real usage block is an event.
    if json.get("usage").is_some_and(|usage| !usage.is_null()) {
        events.push(StreamEvent::now(StreamEventKind::Usage(parse_usage(json))));
    }
    if let Some(reason) = json
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
    {
        let finish_reason = normalize_finish_reason(reason);
        events.push(StreamEvent::now(StreamEventKind::Done {
            finish_reason: format!("{finish_reason:?}"),
        }));
    }
    events
}

/// The start or argument delta of one element of a chunk's `tool_calls`.
fn tool_call_event(tc: &Value) -> StreamEvent {
    // The `index` field is always present in OpenAI streaming deltas and
    // uniquely identifies each parallel tool call within a turn. The `id`
    // field is only present on the first chunk for each call.
    let index = tc.get("index").and_then(Value::as_u64).unwrap_or(0);
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

    // Use index as the stable key for linking start/delta events. The real
    // provider id is stored separately in the accumulator.
    let index_key = format!("__idx_{index}");

    // When id or name is present, this is a tool call start.
    if !id.is_empty() || !name.is_empty() {
        // Embed the real id after a NUL separator so the accumulator can
        // recover it: "__idx_0\0call_abc123". If there are initial arguments
        // in the same chunk, append them after a SOH (\x01) separator so the
        // accumulator can seed the entry: "__idx_0\0call_abc123\x01{\"value\":".
        let mut keyed_id = if id.is_empty() {
            index_key
        } else {
            format!("{index_key}\0{id}")
        };
        if !arguments.is_empty() {
            keyed_id.push('\x01');
            keyed_id.push_str(&arguments);
        }
        return StreamEvent::now(StreamEventKind::ToolCallStart { id: keyed_id, name });
    }
    // Otherwise it's a delta with partial arguments: use the same index key
    // so the accumulator can find the matching start.
    StreamEvent::now(StreamEventKind::ToolCallDelta {
        id: index_key,
        json_fragment: arguments,
    })
}

#[cfg(test)]
mod tests {
    use super::{UnifiedStreamEvent, parse_sse_line};
    use crate::tool_loop::{StreamEvent, StreamEventKind};

    /// The first event `line` yields, if any.
    fn first_event(line: &str) -> Option<StreamEvent> {
        parse_sse_line(line).into_iter().next()
    }

    #[test]
    fn sse_parser_reads_reasoning_delta() {
        let event = first_event(
            r#"data: {"choices":[{"delta":{"reasoning_content":"Need to inspect the file."}}]}"#,
        );

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::ReasoningDelta(reasoning)) if reasoning == "Need to inspect the file."
        ));
    }

    #[test]
    fn sse_parser_reads_content_delta() {
        let event = first_event(r#"data: {"choices":[{"delta":{"content":"I can answer now."}}]}"#);

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::TextDelta(content)) if content == "I can answer now."
        ));
    }

    /// Each event keeps the model its chunk named (bug-bfd241).
    #[test]
    fn sse_parser_keeps_the_chunk_model() {
        let named =
            first_event(r#"data: {"model":"glm-4.7","choices":[{"delta":{"content":"hi"}}]}"#)
                .expect("a content chunk");
        assert_eq!(named.model.as_deref(), Some("glm-4.7"));

        let unnamed = first_event(r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#)
            .expect("a content chunk");
        assert_eq!(unnamed.model, None);
    }

    /// Each event carries the response, session and thread ids its chunk
    /// named (bug-ea7723).
    #[test]
    fn stream_events_carry_session_ids() {
        let named = first_event(
            r#"data: {"id":"chatcmpl-1","session_id":"sess-1","thread_id":"thread-1","choices":[{"delta":{"content":"hi"}}]}"#,
        )
        .expect("a content chunk");
        assert_eq!(
            named.session,
            Some(crate::translate::SessionState {
                session_id: Some("sess-1".to_string()),
                thread_id: Some("thread-1".to_string()),
                conversation_id: Some("chatcmpl-1".to_string()),
            })
        );

        let unnamed = first_event(r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#)
            .expect("a content chunk");
        assert_eq!(unnamed.session, None);
    }

    #[test]
    fn sse_parser_reads_tool_call_start() {
        let event = first_event(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"call_glm_","function":{"name":"edit_file","arguments":"{\"path\":\"note.txt\"}"}}]}}]}"#,
        );

        // The id embeds the index key, the real provider id after NUL,
        // and any initial argument fragment after SOH (\x01):
        // "__idx_1\0call_glm_\x01{\"path\":\"note.txt\"}".
        let kind = event.map(|e| e.kind);
        match kind {
            Some(StreamEventKind::ToolCallStart { ref id, ref name }) => {
                assert!(id.starts_with("__idx_1\0call_glm_"), "id={id:?}");
                assert_eq!(name, "edit_file");
                // Verify initial args are carried after SOH.
                assert!(
                    id.contains('\x01'),
                    "initial arguments should be encoded after SOH"
                );
            }
            other => panic!("expected ToolCallStart, got {other:?}"),
        }
    }

    #[test]
    fn sse_parser_reads_usage() {
        let event = first_event(
            r#"data: {"choices":[],"usage":{"prompt_tokens":21,"completion_tokens":9,"prompt_tokens_details":{"cached_tokens":4}}}"#,
        );

        // 4 of the 21 prompt tokens were cached (bug-b72a37).
        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::Usage(usage))
                if usage.input_tokens == 17
                    && usage.output_tokens == 9
                    && usage.cache_read_tokens == 4
        ));
    }

    /// backlog 2101: once usage is requested, OpenAI sends `"usage": null` on
    /// every chunk but the last. A finish chunk keeps its finish reason and
    /// yields no usage; the last chunk, with no choices, yields the usage.
    #[test]
    fn null_usage_chunk_keeps_its_finish_reason() {
        let finish = parse_sse_line(
            r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}],"usage":null}"#,
        );
        assert_eq!(finish.len(), 1, "{finish:?}");
        assert!(matches!(
            &finish[0].kind,
            StreamEventKind::Done { finish_reason } if finish_reason == "ToolCalls"
        ));

        let last = parse_sse_line(
            r#"data: {"choices":[],"usage":{"prompt_tokens":100,"completion_tokens":10,"prompt_tokens_details":{"cached_tokens":40}}}"#,
        );
        assert_eq!(last.len(), 1, "{last:?}");
        assert!(matches!(
            &last[0].kind,
            StreamEventKind::Usage(usage)
                if usage.input_tokens == 60
                    && usage.output_tokens == 10
                    && usage.cache_read_tokens == 40
        ));
    }

    #[test]
    fn sse_parser_reads_finish_reason() {
        let event = first_event(r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#);

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::Done { finish_reason }) if finish_reason == "ToolCalls"
        ));
    }

    /// `[DONE]` names no finish reason, so it does not read as `stop`
    /// (backlog 1111).
    #[test]
    fn sse_parser_reads_done_marker() {
        let event = first_event("data: [DONE]");

        assert!(matches!(
            event.map(|e| e.kind),
            Some(StreamEventKind::Done { finish_reason }) if finish_reason == "unknown"
        ));
        assert!(matches!(
            super::parse_sse_frame("data: [DONE]"),
            super::SseLine::Done
        ));
    }

    #[test]
    fn sse_parser_reports_error_objects() {
        let line = r#"data: {"error":{"message":"Rate limit reached","type":"rate_limit_error","code":429}}"#;
        let super::SseLine::Error(error) = super::parse_sse_frame(line) else {
            panic!("an error object is an error");
        };
        assert_eq!(error.message, "Rate limit reached");
        assert_eq!(error.error_type.as_deref(), Some("rate_limit_error"));
        assert_eq!(error.status(), Some(429));
        assert!(parse_sse_line(line).is_empty());

        let bare = super::parse_sse_frame(r#"data: {"error":"upstream closed"}"#);
        assert!(matches!(bare, super::SseLine::Error(error) if error.message == "upstream closed"));
    }

    /// backlog 1110: a chunk yields every field it carries, in order, and
    /// each event keeps the chunk's model. The chunks follow Z.ai's shapes.
    #[test]
    fn sse_chunk_keeps_usage_finish_and_every_tool_call() {
        // The final chunk: usage and finish reason next to an empty content.
        let last = parse_sse_line(
            r#"data: {"model":"glm-4.7","choices":[{"index":0,"delta":{"content":""},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":120,"completion_tokens":30,"total_tokens":150}}"#,
        );
        assert_eq!(last.len(), 2, "{last:?}");
        assert!(matches!(
            &last[0].kind,
            StreamEventKind::Usage(usage) if usage.input_tokens == 120 && usage.output_tokens == 30
        ));
        assert!(matches!(
            &last[1].kind,
            StreamEventKind::Done { finish_reason } if finish_reason == "ToolCalls"
        ));
        assert!(
            last.iter()
                .all(|event| event.model.as_deref() == Some("glm-4.7"))
        );

        // Reasoning and a tool call in one chunk: both survive, in order.
        let mixed = parse_sse_line(
            r#"data: {"choices":[{"delta":{"reasoning_content":"Need the file.","tool_calls":[{"index":0,"id":"call_1","function":{"name":"read_file","arguments":"{\"path\":\"a.txt\"}"}}]}}]}"#,
        );
        assert_eq!(mixed.len(), 2, "{mixed:?}");
        assert!(matches!(
            &mixed[0].kind,
            StreamEventKind::ReasoningDelta(reasoning) if reasoning == "Need the file."
        ));
        assert!(matches!(
            &mixed[1].kind,
            StreamEventKind::ToolCallStart { id, name }
                if id.starts_with("__idx_0\0call_1") && name == "read_file"
        ));

        // Two tool calls in one chunk: two starts, with distinct keys.
        let parallel = parse_sse_line(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_a","function":{"name":"read_file","arguments":""}},{"index":1,"id":"call_b","function":{"name":"write_file","arguments":""}}]}}]}"#,
        );
        let starts = parallel
            .iter()
            .filter_map(|event| match &event.kind {
                StreamEventKind::ToolCallStart { id, name } => Some((id.clone(), name.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(starts.len(), 2, "{parallel:?}");
        assert!(starts[0].0.starts_with("__idx_0\0call_a"));
        assert_eq!(starts[0].1, "read_file");
        assert!(starts[1].0.starts_with("__idx_1\0call_b"));
        assert_eq!(starts[1].1, "write_file");
    }

    #[test]
    fn sse_parser_ignores_non_data_lines() {
        assert!(first_event("event: message").is_none());
    }

    #[test]
    fn tool_result_stream_events_map_to_tool_output() {
        // A ToolResult stream event should surface as UnifiedStreamEvent::ToolOutput,
        // which is the UnifiedStreamEvent counterpart of AgentRuntimeEvent::ToolOutput.
        let event = StreamEvent::now(StreamEventKind::ToolResult {
            id: "call-1".to_string(),
            output: "hello".to_string(),
            is_error: false,
        });
        let unified = UnifiedStreamEvent::from_stream_event(event)
            .expect("ToolResult should produce a UnifiedStreamEvent");
        assert!(
            matches!(
                unified,
                UnifiedStreamEvent::ToolOutput { ref id, ref output }
                if id == "call-1" && output == "hello"
            ),
            "expected ToolOutput(call-1, hello), got {unified:?}"
        );
    }

    #[test]
    fn sse_parser_tool_call_start_embeds_index_and_real_id() {
        // First chunk of a tool call: has id, name, and index.
        let event = first_event(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_abc","function":{"name":"read_file","arguments":""}}]}}]}"#,
        );
        match event.map(|e| e.kind) {
            Some(StreamEventKind::ToolCallStart { id, name }) => {
                assert!(
                    id.starts_with("__idx_0\0"),
                    "id should embed index key: {id}"
                );
                assert!(id.ends_with("call_abc"), "id should embed real id: {id}");
                assert_eq!(name, "read_file");
            }
            other => panic!("expected ToolCallStart, got {other:?}"),
        }
    }

    #[test]
    fn sse_parser_tool_call_delta_uses_index_key() {
        // Subsequent chunk: no id, no name — just index and arguments.
        let event = first_event(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":"}}]}}]}"#,
        );
        match event.map(|e| e.kind) {
            Some(StreamEventKind::ToolCallDelta { id, json_fragment }) => {
                assert_eq!(id, "__idx_0", "delta should use index-based key");
                assert_eq!(json_fragment, r#"{"path":"#);
            }
            other => panic!("expected ToolCallDelta, got {other:?}"),
        }
    }

    #[test]
    fn sse_parser_parallel_tool_calls_use_distinct_index_keys() {
        // Two parallel tool calls use different indices.
        let event0 = first_event(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_a","function":{"name":"read","arguments":""}}]}}]}"#,
        ).unwrap();
        let event1 = first_event(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"call_b","function":{"name":"write","arguments":""}}]}}]}"#,
        ).unwrap();

        let id0 = match event0.kind {
            StreamEventKind::ToolCallStart { id, .. } => id,
            _ => panic!(),
        };
        let id1 = match event1.kind {
            StreamEventKind::ToolCallStart { id, .. } => id,
            _ => panic!(),
        };

        assert_ne!(id0, id1, "parallel tool calls must have distinct keys");
        assert!(id0.starts_with("__idx_0"));
        assert!(id1.starts_with("__idx_1"));
    }
}
