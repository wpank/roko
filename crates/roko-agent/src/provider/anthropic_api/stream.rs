// Anthropic Messages SSE streaming decoder.
//
// Implements the Anthropic streaming protocol: parses SSE frames from
// the Messages API and emits canonical `StreamEvent` values.
//
// # Anthropic SSE event types
//
// - `message_start` — initial message metadata + usage
// - `content_block_start` — new content block (text, thinking, or tool_use)
// - `content_block_delta` — incremental content for a block
// - `content_block_stop` — block is complete
// - `message_delta` — final usage + stop reason
// - `message_stop` — stream is done
// - `error` — provider error
// - `ping` — keep-alive (ignored)

use std::collections::HashMap;

use serde_json::Value;

use crate::tool_loop::{LlmError, StreamEvent, StreamEventKind};
use crate::usage::Usage;

/// Maximum size of a single SSE frame in bytes.
const MAX_SSE_FRAME_BYTES: usize = 1024 * 1024; // 1 MiB

/// Maximum size of accumulated tool-use JSON per tool block.
const MAX_TOOL_JSON_BYTES: usize = 1024 * 1024; // 1 MiB

/// Maximum number of concurrent tool blocks tracked.
const MAX_CONCURRENT_TOOL_BLOCKS: usize = 64;

/// Incremental SSE line decoder that handles arbitrary chunk boundaries.
///
/// Feed raw bytes via [`push`](Self::push); call [`drain_lines`](Self::drain_lines)
/// to extract complete SSE lines. Handles both `\n` and `\r\n` line endings.
pub(crate) struct SseLineDecoder {
    buffer: Vec<u8>,
}

impl SseLineDecoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(4096),
        }
    }

    /// Append raw bytes from a chunk.
    ///
    /// Returns `Err` if the accumulated buffer exceeds the frame size limit.
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), LlmError> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > MAX_SSE_FRAME_BYTES {
            return Err(LlmError::Backend(format!(
                "SSE frame exceeds {MAX_SSE_FRAME_BYTES} byte limit"
            )));
        }
        Ok(())
    }

    /// Drain all complete lines from the buffer.
    ///
    /// Returns a vector of lines with trailing CR/LF stripped.
    pub fn drain_lines(&mut self) -> Vec<String> {
        let mut lines = Vec::new();
        loop {
            let newline_pos = self.buffer.iter().position(|&b| b == b'\n');
            let Some(pos) = newline_pos else {
                break;
            };

            let line_bytes = &self.buffer[..pos];
            // Strip trailing \r if present (CRLF).
            let line = if line_bytes.last() == Some(&b'\r') {
                String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]).to_string()
            } else {
                String::from_utf8_lossy(line_bytes).to_string()
            };
            lines.push(line);
            self.buffer.drain(..=pos);
        }
        lines
    }

    /// Flush any remaining unterminated content as a final line.
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            return None;
        }
        let line = String::from_utf8_lossy(&self.buffer).to_string();
        self.buffer.clear();
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.to_string())
    }
}

/// State machine that tracks Anthropic content blocks and produces
/// canonical [`StreamEvent`] values.
pub(crate) struct AnthropicStreamState {
    /// Map of block index -> in-progress tool block state.
    tool_blocks: HashMap<u64, ToolBlockState>,
    /// Initial usage from `message_start` (input tokens, cache counts).
    initial_usage: Option<InitialUsage>,
    /// Whether we have emitted a Done event.
    done_emitted: bool,
}

/// Tracked state for one tool_use content block.
struct ToolBlockState {
    id: String,
    name: String,
    json_acc: String,
    started: bool,
}

/// Usage reported in `message_start`.
struct InitialUsage {
    input_tokens: u32,
    cache_read_tokens: u32,
    cache_create_tokens: u32,
}

impl AnthropicStreamState {
    pub fn new() -> Self {
        Self {
            tool_blocks: HashMap::new(),
            initial_usage: None,
            done_emitted: false,
        }
    }

    /// Returns true if a `Done` event has been emitted.
    pub fn is_done(&self) -> bool {
        self.done_emitted
    }

    /// Process one SSE line pair (event type + data) and return events.
    ///
    /// An SSE frame consists of an `event:` line followed by a `data:` line.
    /// This method takes both and returns zero or more canonical events.
    pub fn process_sse_event(
        &mut self,
        event_type: &str,
        data: &str,
    ) -> Result<Vec<StreamEvent>, LlmError> {
        match event_type {
            "message_start" => self.handle_message_start(data),
            "content_block_start" => self.handle_content_block_start(data),
            "content_block_delta" => self.handle_content_block_delta(data),
            "content_block_stop" => self.handle_content_block_stop(data),
            "message_delta" => self.handle_message_delta(data),
            "message_stop" => self.handle_message_stop(),
            "ping" => Ok(Vec::new()),
            "error" => self.handle_error(data),
            _ => {
                // Unknown event types are silently ignored per SSE spec.
                Ok(Vec::new())
            }
        }
    }

    fn handle_message_start(&mut self, data: &str) -> Result<Vec<StreamEvent>, LlmError> {
        let json: Value = serde_json::from_str(data)
            .map_err(|e| LlmError::Backend(format!("parse message_start: {e}")))?;

        // Extract initial usage (input tokens).
        if let Some(usage) = json.pointer("/message/usage") {
            let input_tokens = usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let cache_read_tokens = usage
                .get("cache_read_input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let cache_create_tokens = usage
                .get("cache_creation_input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;

            self.initial_usage = Some(InitialUsage {
                input_tokens,
                cache_read_tokens,
                cache_create_tokens,
            });

            // Emit an initial Usage event with input token counts.
            return Ok(vec![StreamEvent::now(StreamEventKind::Usage(Usage {
                input_tokens,
                output_tokens: 0,
                cache_read_tokens,
                cache_create_tokens,
                ..Default::default()
            }))]);
        }

        Ok(Vec::new())
    }

    fn handle_content_block_start(&mut self, data: &str) -> Result<Vec<StreamEvent>, LlmError> {
        let json: Value = serde_json::from_str(data)
            .map_err(|e| LlmError::Backend(format!("parse content_block_start: {e}")))?;

        let index = json.get("index").and_then(Value::as_u64).unwrap_or(0);

        let block = json.get("content_block").unwrap_or(&Value::Null);
        let block_type = block.get("type").and_then(Value::as_str).unwrap_or("");

        match block_type {
            "tool_use" => {
                if self.tool_blocks.len() >= MAX_CONCURRENT_TOOL_BLOCKS {
                    return Err(LlmError::Backend(format!(
                        "exceeded {MAX_CONCURRENT_TOOL_BLOCKS} concurrent tool blocks"
                    )));
                }

                let id = block
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();

                self.tool_blocks.insert(
                    index,
                    ToolBlockState {
                        id: id.clone(),
                        name: name.clone(),
                        json_acc: String::new(),
                        started: true,
                    },
                );

                Ok(vec![StreamEvent::now(StreamEventKind::ToolCallStart {
                    id,
                    name,
                })])
            }
            // text and thinking blocks don't need explicit start events;
            // their deltas carry the content directly.
            _ => Ok(Vec::new()),
        }
    }

    fn handle_content_block_delta(&mut self, data: &str) -> Result<Vec<StreamEvent>, LlmError> {
        let json: Value = serde_json::from_str(data)
            .map_err(|e| LlmError::Backend(format!("parse content_block_delta: {e}")))?;

        let index = json.get("index").and_then(Value::as_u64).unwrap_or(0);

        let delta = json.get("delta").unwrap_or(&Value::Null);
        let delta_type = delta.get("type").and_then(Value::as_str).unwrap_or("");

        match delta_type {
            "text_delta" => {
                let text = delta
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                Ok(vec![StreamEvent::now(StreamEventKind::TextDelta(text))])
            }
            "thinking" => {
                let thinking = delta
                    .get("thinking")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                Ok(vec![StreamEvent::now(StreamEventKind::ReasoningDelta(
                    thinking,
                ))])
            }
            "input_json_delta" => {
                let partial_json = delta
                    .get("partial_json")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                if let Some(tool_block) = self.tool_blocks.get_mut(&index) {
                    tool_block.json_acc.push_str(partial_json);

                    if tool_block.json_acc.len() > MAX_TOOL_JSON_BYTES {
                        return Err(LlmError::Backend(format!(
                            "tool JSON for block {} exceeds {MAX_TOOL_JSON_BYTES} byte limit",
                            index
                        )));
                    }

                    Ok(vec![StreamEvent::now(StreamEventKind::ToolCallDelta {
                        id: tool_block.id.clone(),
                        json_fragment: partial_json.to_string(),
                    })])
                } else {
                    // Delta for unknown block index -- skip rather than error.
                    Ok(Vec::new())
                }
            }
            _ => {
                // Unknown delta types are silently ignored.
                Ok(Vec::new())
            }
        }
    }

    fn handle_content_block_stop(&mut self, data: &str) -> Result<Vec<StreamEvent>, LlmError> {
        let json: Value = serde_json::from_str(data)
            .map_err(|e| LlmError::Backend(format!("parse content_block_stop: {e}")))?;

        let index = json.get("index").and_then(Value::as_u64).unwrap_or(0);

        if let Some(tool_block) = self.tool_blocks.remove(&index) {
            // Parse accumulated JSON into a Value.
            let args: Value = if tool_block.json_acc.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&tool_block.json_acc)
                    .unwrap_or_else(|_| Value::String(tool_block.json_acc.clone()))
            };

            Ok(vec![StreamEvent::now(StreamEventKind::ToolCallEnd {
                id: tool_block.id,
                name: tool_block.name,
                args,
            })])
        } else {
            // Text/thinking blocks closing -- no event needed.
            Ok(Vec::new())
        }
    }

    fn handle_message_delta(&mut self, data: &str) -> Result<Vec<StreamEvent>, LlmError> {
        let json: Value = serde_json::from_str(data)
            .map_err(|e| LlmError::Backend(format!("parse message_delta: {e}")))?;

        let mut events = Vec::new();

        // Extract cumulative usage from message_delta.
        // The message_delta carries the cumulative output token count.
        // We combine it with the initial input counts to produce a final Usage.
        if let Some(usage) = json.get("usage") {
            let output_tokens = usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;

            let (input_tokens, cache_read_tokens, cache_create_tokens) =
                if let Some(ref initial) = self.initial_usage {
                    (
                        initial.input_tokens,
                        initial.cache_read_tokens,
                        initial.cache_create_tokens,
                    )
                } else {
                    (0, 0, 0)
                };

            events.push(StreamEvent::now(StreamEventKind::Usage(Usage {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_create_tokens,
                ..Default::default()
            })));
        }

        // Extract stop reason from message_delta.delta.stop_reason.
        if let Some(stop_reason) = json.pointer("/delta/stop_reason").and_then(Value::as_str) {
            let finish_reason = match stop_reason {
                "end_turn" => "stop",
                "tool_use" => "tool_calls",
                "max_tokens" => "length",
                "stop_sequence" => "stop_sequence",
                other => other,
            };
            events.push(StreamEvent::now(StreamEventKind::Done {
                finish_reason: finish_reason.to_string(),
            }));
            self.done_emitted = true;
        }

        Ok(events)
    }

    fn handle_message_stop(&mut self) -> Result<Vec<StreamEvent>, LlmError> {
        if !self.done_emitted {
            self.done_emitted = true;
            Ok(vec![StreamEvent::now(StreamEventKind::Done {
                finish_reason: "stop".to_string(),
            })])
        } else {
            Ok(Vec::new())
        }
    }

    fn handle_error(&mut self, data: &str) -> Result<Vec<StreamEvent>, LlmError> {
        let json: Value = serde_json::from_str(data)
            .unwrap_or_else(|_| serde_json::json!({"error": {"message": data}}));

        let message = json
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error")
            .to_string();

        // Error events never emit Done -- prior deltas remain observable.
        Err(LlmError::Backend(format!(
            "Anthropic stream error: {message}"
        )))
    }
}

/// Parse SSE lines into (event_type, data) pairs.
///
/// SSE frames consist of `event: <type>` and `data: <json>` lines
/// separated by blank lines. This function processes a batch of lines
/// and yields all complete event/data pairs.
pub(crate) fn parse_sse_frames(lines: &[String]) -> Vec<(String, String)> {
    let mut frames = Vec::new();
    let mut current_event = String::new();
    let mut current_data = Vec::new();

    for line in lines {
        if line.is_empty() {
            // End of frame: emit if we have data.
            if !current_data.is_empty() {
                let event_type = if current_event.is_empty() {
                    "message".to_string()
                } else {
                    current_event.clone()
                };
                frames.push((event_type, current_data.join("\n")));
            }
            current_event.clear();
            current_data.clear();
            continue;
        }

        if line.starts_with(':') {
            // SSE comment -- ignore.
            continue;
        }

        if let Some(rest) = line.strip_prefix("event:") {
            current_event = rest.trim_start().to_string();
        } else if let Some(rest) = line.strip_prefix("data:") {
            current_data.push(rest.trim_start().to_string());
        }
        // Other field names (id:, retry:) are ignored.
    }

    // Flush any unterminated frame (final event without trailing blank line).
    if !current_data.is_empty() {
        let event_type = if current_event.is_empty() {
            "message".to_string()
        } else {
            current_event
        };
        frames.push((event_type, current_data.join("\n")));
    }

    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── SseLineDecoder tests ───────────────────────────────────────

    #[test]
    fn decoder_splits_lines_on_lf() {
        let mut decoder = SseLineDecoder::new();
        decoder.push(b"line1\nline2\nline3\n").unwrap();
        let lines = decoder.drain_lines();
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn decoder_splits_lines_on_crlf() {
        let mut decoder = SseLineDecoder::new();
        decoder.push(b"line1\r\nline2\r\n").unwrap();
        let lines = decoder.drain_lines();
        assert_eq!(lines, vec!["line1", "line2"]);
    }

    #[test]
    fn decoder_handles_partial_chunks() {
        let mut decoder = SseLineDecoder::new();
        decoder.push(b"event: mess").unwrap();
        assert!(decoder.drain_lines().is_empty());
        decoder.push(b"age_start\ndata: {}\n").unwrap();
        let lines = decoder.drain_lines();
        assert_eq!(lines, vec!["event: message_start", "data: {}"]);
    }

    #[test]
    fn decoder_flush_returns_unterminated_content() {
        let mut decoder = SseLineDecoder::new();
        decoder.push(b"data: partial").unwrap();
        assert!(decoder.drain_lines().is_empty());
        let flushed = decoder.flush();
        assert_eq!(flushed, Some("data: partial".to_string()));
    }

    #[test]
    fn decoder_flush_returns_none_when_empty() {
        let mut decoder = SseLineDecoder::new();
        assert!(decoder.flush().is_none());
    }

    #[test]
    fn decoder_rejects_oversized_frame() {
        let mut decoder = SseLineDecoder::new();
        let oversized = vec![b'x'; MAX_SSE_FRAME_BYTES + 1];
        let result = decoder.push(&oversized);
        assert!(result.is_err());
    }

    // ─── parse_sse_frames tests ─────────────────────────────────────

    #[test]
    fn parse_frames_extracts_event_and_data() {
        let lines = vec![
            "event: message_start".to_string(),
            "data: {\"message\":{}}".to_string(),
            "".to_string(),
            "event: ping".to_string(),
            "data: {}".to_string(),
            "".to_string(),
        ];
        let frames = parse_sse_frames(&lines);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].0, "message_start");
        assert_eq!(frames[0].1, "{\"message\":{}}");
        assert_eq!(frames[1].0, "ping");
    }

    #[test]
    fn parse_frames_ignores_comments() {
        let lines = vec![
            ": keep-alive".to_string(),
            "event: ping".to_string(),
            "data: {}".to_string(),
            "".to_string(),
        ];
        let frames = parse_sse_frames(&lines);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].0, "ping");
    }

    #[test]
    fn parse_frames_handles_multiline_data() {
        let lines = vec![
            "event: content_block_delta".to_string(),
            "data: {\"index\":0,".to_string(),
            "data: \"delta\":{}}".to_string(),
            "".to_string(),
        ];
        let frames = parse_sse_frames(&lines);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].1, "{\"index\":0,\n\"delta\":{}}");
    }

    #[test]
    fn parse_frames_flushes_unterminated_frame() {
        let lines = vec!["event: message_stop".to_string(), "data: {}".to_string()];
        let frames = parse_sse_frames(&lines);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].0, "message_stop");
    }

    // ─── AnthropicStreamState tests ─────────────────────────────────

    #[test]
    fn anthropic_stream_text_delta() {
        let mut state = AnthropicStreamState::new();
        let events = state
            .process_sse_event(
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::TextDelta(text) if text == "Hello"
        ));
    }

    #[test]
    fn anthropic_stream_thinking_delta() {
        let mut state = AnthropicStreamState::new();
        let events = state
            .process_sse_event(
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"thinking","thinking":"reasoning step"}}"#,
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::ReasoningDelta(text) if text == "reasoning step"
        ));
    }

    #[test]
    fn anthropic_stream_message_start_usage() {
        let mut state = AnthropicStreamState::new();
        let events = state
            .process_sse_event(
                "message_start",
                r#"{"message":{"usage":{"input_tokens":100,"cache_read_input_tokens":20,"cache_creation_input_tokens":5}}}"#,
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::Usage(usage) if usage.input_tokens == 100
                && usage.cache_read_tokens == 20
                && usage.cache_create_tokens == 5
                && usage.output_tokens == 0
        ));
    }

    #[test]
    fn anthropic_stream_tool_lifecycle() {
        let mut state = AnthropicStreamState::new();

        // Tool start
        let events = state
            .process_sse_event(
                "content_block_start",
                r#"{"index":1,"content_block":{"type":"tool_use","id":"call_1","name":"read_file"}}"#,
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::ToolCallStart { id, name }
                if id == "call_1" && name == "read_file"
        ));

        // Tool delta
        let events = state
            .process_sse_event(
                "content_block_delta",
                r#"{"index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":"}}"#,
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::ToolCallDelta { id, json_fragment }
                if id == "call_1" && json_fragment == "{\"path\":"
        ));

        // Another tool delta
        let events = state
            .process_sse_event(
                "content_block_delta",
                r#"{"index":1,"delta":{"type":"input_json_delta","partial_json":"\"foo.txt\"}"}}"#,
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::ToolCallDelta { id, json_fragment }
                if id == "call_1" && json_fragment == "\"foo.txt\"}"
        ));

        // Tool stop
        let events = state
            .process_sse_event("content_block_stop", r#"{"index":1}"#)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::ToolCallEnd { id, name, args }
                if id == "call_1"
                && name == "read_file"
                && args == &serde_json::json!({"path": "foo.txt"})
        ));
    }

    #[test]
    fn anthropic_stream_parallel_tools() {
        let mut state = AnthropicStreamState::new();

        // Start two tools
        state
            .process_sse_event(
                "content_block_start",
                r#"{"index":0,"content_block":{"type":"tool_use","id":"call_a","name":"read_file"}}"#,
            )
            .unwrap();
        state
            .process_sse_event(
                "content_block_start",
                r#"{"index":1,"content_block":{"type":"tool_use","id":"call_b","name":"write_file"}}"#,
            )
            .unwrap();

        // Interleaved deltas
        state
            .process_sse_event(
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"input_json_delta","partial_json":"{\"p\":\"a\"}"}}"#,
            )
            .unwrap();
        state
            .process_sse_event(
                "content_block_delta",
                r#"{"index":1,"delta":{"type":"input_json_delta","partial_json":"{\"p\":\"b\"}"}}"#,
            )
            .unwrap();

        // Stop first tool
        let events = state
            .process_sse_event("content_block_stop", r#"{"index":0}"#)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::ToolCallEnd { id, name, args }
                if id == "call_a"
                && name == "read_file"
                && args == &serde_json::json!({"p": "a"})
        ));

        // Stop second tool
        let events = state
            .process_sse_event("content_block_stop", r#"{"index":1}"#)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::ToolCallEnd { id, name, args }
                if id == "call_b"
                && name == "write_file"
                && args == &serde_json::json!({"p": "b"})
        ));
    }

    #[test]
    fn anthropic_stream_message_delta_usage_and_stop() {
        let mut state = AnthropicStreamState::new();

        // Set initial usage
        state
            .process_sse_event(
                "message_start",
                r#"{"message":{"usage":{"input_tokens":50,"cache_read_input_tokens":10,"cache_creation_input_tokens":0}}}"#,
            )
            .unwrap();

        // Message delta with cumulative output tokens + stop reason
        let events = state
            .process_sse_event(
                "message_delta",
                r#"{"delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}"#,
            )
            .unwrap();

        assert_eq!(events.len(), 2);
        // First event: final cumulative usage
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::Usage(usage)
                if usage.input_tokens == 50
                && usage.output_tokens == 25
                && usage.cache_read_tokens == 10
        ));
        // Second event: done with stop reason
        assert!(matches!(
            &events[1].kind,
            StreamEventKind::Done { finish_reason }
                if finish_reason == "stop"
        ));
        assert!(state.is_done());
    }

    #[test]
    fn anthropic_stream_tool_use_stop_reason() {
        let mut state = AnthropicStreamState::new();
        let events = state
            .process_sse_event(
                "message_delta",
                r#"{"delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":10}}"#,
            )
            .unwrap();

        let done = events
            .iter()
            .find(|e| matches!(e.kind, StreamEventKind::Done { .. }));
        assert!(matches!(
            &done.unwrap().kind,
            StreamEventKind::Done { finish_reason } if finish_reason == "tool_calls"
        ));
    }

    #[test]
    fn anthropic_stream_max_tokens_stop_reason() {
        let mut state = AnthropicStreamState::new();
        let events = state
            .process_sse_event(
                "message_delta",
                r#"{"delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":4096}}"#,
            )
            .unwrap();

        let done = events
            .iter()
            .find(|e| matches!(e.kind, StreamEventKind::Done { .. }));
        assert!(matches!(
            &done.unwrap().kind,
            StreamEventKind::Done { finish_reason } if finish_reason == "length"
        ));
    }

    #[test]
    fn anthropic_stream_message_stop_without_prior_done() {
        let mut state = AnthropicStreamState::new();
        assert!(!state.is_done());

        let events = state.process_sse_event("message_stop", "{}").unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            StreamEventKind::Done { finish_reason } if finish_reason == "stop"
        ));
        assert!(state.is_done());
    }

    #[test]
    fn anthropic_stream_message_stop_after_done_emits_nothing() {
        let mut state = AnthropicStreamState::new();

        // First Done from message_delta
        state
            .process_sse_event(
                "message_delta",
                r#"{"delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":1}}"#,
            )
            .unwrap();
        assert!(state.is_done());

        // message_stop should not emit a duplicate
        let events = state.process_sse_event("message_stop", "{}").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn anthropic_stream_error_event() {
        let mut state = AnthropicStreamState::new();
        let result = state.process_sse_event(
            "error",
            r#"{"error":{"type":"overloaded_error","message":"API is overloaded"}}"#,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("API is overloaded"));
        // Error should NOT set done_emitted
        assert!(!state.is_done());
    }

    #[test]
    fn anthropic_stream_ping_emits_nothing() {
        let mut state = AnthropicStreamState::new();
        let events = state.process_sse_event("ping", "{}").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn anthropic_stream_usage_no_double_count() {
        let mut state = AnthropicStreamState::new();

        // message_start: input=100, cache_read=20
        state
            .process_sse_event(
                "message_start",
                r#"{"message":{"usage":{"input_tokens":100,"cache_read_input_tokens":20,"cache_creation_input_tokens":0}}}"#,
            )
            .unwrap();

        // message_delta: cumulative output=50
        let events = state
            .process_sse_event(
                "message_delta",
                r#"{"delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":50}}"#,
            )
            .unwrap();

        let usage_event = events
            .iter()
            .find(|e| matches!(e.kind, StreamEventKind::Usage(_)))
            .expect("should have usage event");

        // Verify: input_tokens=100 (from message_start), output_tokens=50 (from message_delta)
        // No double counting.
        if let StreamEventKind::Usage(usage) = &usage_event.kind {
            assert_eq!(usage.input_tokens, 100);
            assert_eq!(usage.output_tokens, 50);
            assert_eq!(usage.cache_read_tokens, 20);
        } else {
            panic!("expected Usage event");
        }
    }

    #[test]
    fn anthropic_stream_tool_json_bounds_limit() {
        let mut state = AnthropicStreamState::new();

        state
            .process_sse_event(
                "content_block_start",
                r#"{"index":0,"content_block":{"type":"tool_use","id":"call_1","name":"tool"}}"#,
            )
            .unwrap();

        // Send a huge JSON fragment that exceeds the limit.
        let huge = "x".repeat(MAX_TOOL_JSON_BYTES + 1);
        let result = state.process_sse_event(
            "content_block_delta",
            &format!(
                r#"{{"index":0,"delta":{{"type":"input_json_delta","partial_json":"{huge}"}}}}"#
            ),
        );
        assert!(result.is_err());
    }

    #[test]
    fn anthropic_stream_concurrent_tool_block_limit() {
        let mut state = AnthropicStreamState::new();

        for i in 0..MAX_CONCURRENT_TOOL_BLOCKS {
            state
                .process_sse_event(
                    "content_block_start",
                    &format!(
                        r#"{{"index":{i},"content_block":{{"type":"tool_use","id":"call_{i}","name":"tool_{i}"}}}}"#
                    ),
                )
                .unwrap();
        }

        // One more should fail.
        let result = state.process_sse_event(
            "content_block_start",
            &format!(
                r#"{{"index":{},"content_block":{{"type":"tool_use","id":"call_extra","name":"tool_extra"}}}}"#,
                MAX_CONCURRENT_TOOL_BLOCKS
            ),
        );
        assert!(result.is_err());
    }

    // ─── End-to-end fixture tests ────────────────────────────────────

    /// Simulate a complete text-only stream.
    #[test]
    fn anthropic_stream_fixture_text_only() {
        let sse = "\
event: message_start\n\
data: {\"message\":{\"id\":\"msg_1\",\"usage\":{\"input_tokens\":25,\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}}}\n\
\n\
event: content_block_start\n\
data: {\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\", world!\"}}\n\
\n\
event: content_block_stop\n\
data: {\"index\":0}\n\
\n\
event: message_delta\n\
data: {\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":12}}\n\
\n\
event: message_stop\n\
data: {}\n\
\n";

        let mut decoder = SseLineDecoder::new();
        decoder.push(sse.as_bytes()).unwrap();
        let lines = decoder.drain_lines();
        let frames = parse_sse_frames(&lines);
        let mut state = AnthropicStreamState::new();

        let mut all_events = Vec::new();
        for (event_type, data) in &frames {
            let events = state.process_sse_event(event_type, data).unwrap();
            all_events.extend(events);
        }

        // Should have: Usage(initial), TextDelta("Hello"), TextDelta(", world!"),
        //              Usage(final), Done
        let text_deltas: Vec<_> = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::TextDelta(t) = &e.kind {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(text_deltas, vec!["Hello", ", world!"]);

        let usage_events: Vec<_> = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::Usage(u) = &e.kind {
                    Some(u)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(usage_events.len(), 2);
        assert_eq!(usage_events[0].input_tokens, 25);
        assert_eq!(usage_events[0].output_tokens, 0);
        assert_eq!(usage_events[1].input_tokens, 25);
        assert_eq!(usage_events[1].output_tokens, 12);

        assert!(state.is_done());
    }

    /// Simulate a stream with parallel tool calls.
    #[test]
    fn anthropic_stream_fixture_parallel_tools() {
        let sse = "\
event: message_start\n\
data: {\"message\":{\"id\":\"msg_2\",\"usage\":{\"input_tokens\":50,\"cache_read_input_tokens\":5,\"cache_creation_input_tokens\":0}}}\n\
\n\
event: content_block_start\n\
data: {\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"read_file\"}}\n\
\n\
event: content_block_start\n\
data: {\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t2\",\"name\":\"write_file\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"a.txt\\\"}\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"b.txt\\\",\\\"content\\\":\\\"hi\\\"}\"}}\n\
\n\
event: content_block_stop\n\
data: {\"index\":0}\n\
\n\
event: content_block_stop\n\
data: {\"index\":1}\n\
\n\
event: message_delta\n\
data: {\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":30}}\n\
\n\
event: message_stop\n\
data: {}\n\
\n";

        let mut decoder = SseLineDecoder::new();
        decoder.push(sse.as_bytes()).unwrap();
        let lines = decoder.drain_lines();
        let frames = parse_sse_frames(&lines);
        let mut state = AnthropicStreamState::new();

        let mut all_events = Vec::new();
        for (event_type, data) in &frames {
            let events = state.process_sse_event(event_type, data).unwrap();
            all_events.extend(events);
        }

        // Verify tool starts
        let starts: Vec<_> = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::ToolCallStart { id, name } = &e.kind {
                    Some((id.as_str(), name.as_str()))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(starts, vec![("t1", "read_file"), ("t2", "write_file")]);

        // Verify tool ends with correct JSON
        let ends: Vec<_> = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::ToolCallEnd { id, name, args } = &e.kind {
                    Some((id.as_str(), name.as_str(), args.clone()))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(ends.len(), 2);
        assert_eq!(ends[0].0, "t1");
        assert_eq!(ends[0].1, "read_file");
        assert_eq!(ends[0].2, serde_json::json!({"path": "a.txt"}));
        assert_eq!(ends[1].0, "t2");
        assert_eq!(ends[1].1, "write_file");
        assert_eq!(
            ends[1].2,
            serde_json::json!({"path": "b.txt", "content": "hi"})
        );

        // Verify final usage
        let last_usage = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::Usage(u) = &e.kind {
                    Some(u)
                } else {
                    None
                }
            })
            .last()
            .expect("should have usage");
        assert_eq!(last_usage.input_tokens, 50);
        assert_eq!(last_usage.output_tokens, 30);
        assert_eq!(last_usage.cache_read_tokens, 5);

        // Verify stop reason
        let done = all_events
            .iter()
            .find(|e| matches!(e.kind, StreamEventKind::Done { .. }))
            .expect("should have done");
        assert!(matches!(
            &done.kind,
            StreamEventKind::Done { finish_reason } if finish_reason == "tool_calls"
        ));
    }

    /// Simulate an error frame mid-stream.
    #[test]
    fn anthropic_stream_fixture_error_preserves_prior_deltas() {
        let sse = "\
event: message_start\n\
data: {\"message\":{\"id\":\"msg_3\",\"usage\":{\"input_tokens\":10,\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}}}\n\
\n\
event: content_block_start\n\
data: {\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial output\"}}\n\
\n\
event: error\n\
data: {\"error\":{\"type\":\"server_error\",\"message\":\"Internal server error\"}}\n\
\n";

        let mut decoder = SseLineDecoder::new();
        decoder.push(sse.as_bytes()).unwrap();
        let lines = decoder.drain_lines();
        let frames = parse_sse_frames(&lines);
        let mut state = AnthropicStreamState::new();

        let mut all_events = Vec::new();
        let mut error = None;
        for (event_type, data) in &frames {
            match state.process_sse_event(event_type, data) {
                Ok(events) => all_events.extend(events),
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }

        // Prior deltas should be observable.
        let text_deltas: Vec<_> = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::TextDelta(t) = &e.kind {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(text_deltas, vec!["partial output"]);

        // Error should be present.
        assert!(error.is_some());
        assert!(error.unwrap().to_string().contains("Internal server error"));

        // Done should NOT have been emitted.
        assert!(!state.is_done());
    }

    /// Simulate a stream with thinking/reasoning deltas.
    #[test]
    fn anthropic_stream_fixture_thinking() {
        let sse = "\
event: message_start\n\
data: {\"message\":{\"id\":\"msg_4\",\"usage\":{\"input_tokens\":30,\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}}}\n\
\n\
event: content_block_start\n\
data: {\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"thinking\",\"thinking\":\"Let me think\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"thinking\",\"thinking\":\" about this.\"}}\n\
\n\
event: content_block_stop\n\
data: {\"index\":0}\n\
\n\
event: content_block_start\n\
data: {\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"The answer is 42.\"}}\n\
\n\
event: content_block_stop\n\
data: {\"index\":1}\n\
\n\
event: message_delta\n\
data: {\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":20}}\n\
\n\
event: message_stop\n\
data: {}\n\
\n";

        let mut decoder = SseLineDecoder::new();
        decoder.push(sse.as_bytes()).unwrap();
        let lines = decoder.drain_lines();
        let frames = parse_sse_frames(&lines);
        let mut state = AnthropicStreamState::new();

        let mut all_events = Vec::new();
        for (event_type, data) in &frames {
            let events = state.process_sse_event(event_type, data).unwrap();
            all_events.extend(events);
        }

        // Verify reasoning deltas
        let reasoning: Vec<_> = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::ReasoningDelta(t) = &e.kind {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(reasoning, vec!["Let me think", " about this."]);

        // Verify text delta
        let text: Vec<_> = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::TextDelta(t) = &e.kind {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(text, vec!["The answer is 42."]);
    }

    /// Simulate a truncated stream (no message_stop).
    #[test]
    fn anthropic_stream_fixture_truncation() {
        let sse = "\
event: message_start\n\
data: {\"message\":{\"id\":\"msg_5\",\"usage\":{\"input_tokens\":10,\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}}}\n\
\n\
event: content_block_start\n\
data: {\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"truncated\"}}\n\
\n";

        let mut decoder = SseLineDecoder::new();
        decoder.push(sse.as_bytes()).unwrap();
        let lines = decoder.drain_lines();
        let frames = parse_sse_frames(&lines);
        let mut state = AnthropicStreamState::new();

        let mut all_events = Vec::new();
        for (event_type, data) in &frames {
            let events = state.process_sse_event(event_type, data).unwrap();
            all_events.extend(events);
        }

        // Text delta should be observable
        let text: Vec<_> = all_events
            .iter()
            .filter_map(|e| {
                if let StreamEventKind::TextDelta(t) = &e.kind {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(text, vec!["truncated"]);

        // No Done should have been emitted
        assert!(!state.is_done());
        let done_count = all_events
            .iter()
            .filter(|e| matches!(e.kind, StreamEventKind::Done { .. }))
            .count();
        assert_eq!(done_count, 0);
    }
}
