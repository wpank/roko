//! Claude CLI `stream-json` parser.
//!
//! Converts Claude CLI stream-json lines into [`HarnessEvent`]s.

use crate::process::stderr::{benign_stderr_warn_once, classify_benign_stderr};
use serde_json::Value;

use super::events::{EventParser, HarnessEvent};

/// Parser for the Claude CLI `--output-format stream-json` protocol.
///
/// Handles both stdout and stderr (Claude emits stream-json on both).
pub struct ClaudeStreamJsonParser {
    /// Agent name for log messages.
    name: String,
    /// Accumulated text bytes for progress reporting.
    text_bytes: usize,
    /// Count of tool calls seen.
    tool_count: usize,
    /// Whether debug mode is enabled (echo raw lines).
    debug: bool,
}

impl ClaudeStreamJsonParser {
    /// Create a new parser.
    pub fn new(name: impl Into<String>) -> Self {
        let debug = std::env::var_os("ROKO_DEBUG")
            .map(|v| {
                let s = v.to_string_lossy().trim().to_ascii_lowercase();
                matches!(s.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false);

        Self {
            name: name.into(),
            text_bytes: 0,
            tool_count: 0,
            debug,
        }
    }

    /// Try to parse a line as a stream-json event.
    fn parse_event(line: &str) -> Option<Value> {
        let value = serde_json::from_str::<Value>(line.trim()).ok()?;
        if value.get("type").and_then(Value::as_str).is_some() {
            Some(value)
        } else {
            None
        }
    }

    /// Convert a parsed stream-json event into HarnessEvents.
    fn event_to_harness_events(&mut self, event: &Value) -> Vec<HarnessEvent> {
        let mut out = Vec::new();
        let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");

        match event_type {
            // ── system ─────────────────────────────────────────────
            "system" => {
                let session_id = event
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let model = event
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                out.push(HarnessEvent::Output(format!(
                    "[system] session={session_id} model={model}"
                )));
            }

            // ── assistant ──────────────────────────────────────────
            "assistant" => {
                if let Some(content) = event
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(Value::as_array)
                {
                    for block in content {
                        let block_type = block.get("type").and_then(Value::as_str).unwrap_or("");
                        match block_type {
                            "text" => {
                                if let Some(text) = block.get("text").and_then(Value::as_str) {
                                    self.text_bytes += text.len();
                                    out.push(HarnessEvent::Output(text.to_string()));
                                }
                            }
                            "tool_use" => {
                                self.tool_count += 1;
                                let id = block
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string();
                                let name = block
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unknown")
                                    .to_string();
                                let arguments = block.get("input").cloned().unwrap_or(Value::Null);
                                eprintln!("[{}] tool: {name}", self.name);
                                out.push(HarnessEvent::ToolCall {
                                    id,
                                    name,
                                    arguments,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }

            // ── content_block_start ────────────────────────────────
            "content_block_start" => {
                if let Some(block) = event.get("content_block") {
                    let block_type = block.get("type").and_then(Value::as_str).unwrap_or("");
                    match block_type {
                        "tool_use" => {
                            self.tool_count += 1;
                            let id = block
                                .get("id")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string();
                            let name = block
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown")
                                .to_string();
                            eprintln!("[{}] tool: {name}", self.name);
                            out.push(HarnessEvent::ToolCall {
                                id,
                                name,
                                arguments: Value::Null,
                            });
                        }
                        "text" => {
                            eprintln!("[{}] generating text...", self.name);
                        }
                        _ => {}
                    }
                }
            }

            // ── content_block_delta ────────────────────────────────
            "content_block_delta" => {
                if let Some(delta) = event.get("delta") {
                    if let Some(text) = delta.get("text").and_then(Value::as_str) {
                        self.text_bytes += text.len();
                        out.push(HarnessEvent::Output(text.to_string()));
                    }
                }
            }

            // ── result ─────────────────────────────────────────────
            "result" => {
                // Usage extraction.
                if let Some(usage) = event.get("usage") {
                    let input_tokens = usage
                        .get("input_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let output_tokens = usage
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    out.push(HarnessEvent::Usage {
                        input_tokens,
                        output_tokens,
                    });
                }

                // Stop reason.
                if let Some(reason) = event
                    .get("stop_reason")
                    .or_else(|| event.get("result").and_then(|r| r.get("stop_reason")))
                    .and_then(Value::as_str)
                {
                    out.push(HarnessEvent::StopReason(reason.to_string()));
                }

                // Log summary.
                let summary = if self.tool_count > 0 {
                    format!(
                        "{} bytes text, {} tool calls",
                        self.text_bytes, self.tool_count
                    )
                } else {
                    format!("{} bytes text", self.text_bytes)
                };
                eprintln!("[{}] result received ({summary})", self.name);
            }

            // ── tool (subtype: result) ─────────────────────────────
            "tool" if event.get("subtype").and_then(Value::as_str) == Some("result") => {
                let id = event
                    .get("tool_use_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let progress = event
                    .get("content")
                    .and_then(|c| {
                        if let Some(arr) = c.as_array() {
                            arr.first()
                                .and_then(|b| b.get("text"))
                                .and_then(Value::as_str)
                                .map(String::from)
                        } else {
                            c.as_str().map(String::from)
                        }
                    })
                    .unwrap_or_default();
                out.push(HarnessEvent::ToolProgress { id, progress });
            }

            // ── error ──────────────────────────────────────────────
            "error" => {
                let msg = event
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .or_else(|| event.get("message").and_then(Value::as_str))
                    .unwrap_or("unknown error")
                    .to_string();
                out.push(HarnessEvent::Error(msg));
            }

            _ => {
                // Unknown event type -- ignore silently.
            }
        }

        out
    }
}

impl EventParser for ClaudeStreamJsonParser {
    fn parse_stdout_line(&mut self, line: &str) -> Vec<HarnessEvent> {
        if self.debug {
            eprintln!("{line}");
        }

        match Self::parse_event(line) {
            Some(event) => self.event_to_harness_events(&event),
            None => {
                // Non-JSON output (raw text from other agents).
                vec![HarnessEvent::Output(line.to_string())]
            }
        }
    }

    fn parse_stderr_line(&mut self, line: &str) -> Vec<HarnessEvent> {
        // First, try to parse as a stream-json event (Claude CLI
        // emits stream-json on BOTH stdout AND stderr).
        if let Some(event) = Self::parse_event(line) {
            if self.debug {
                eprintln!("{line}");
            }
            return self.event_to_harness_events(&event);
        }

        // Not stream-json -- classify as benign stderr.
        if let Some(benign) = classify_benign_stderr(line) {
            if benign_stderr_warn_once(benign.key) {
                eprintln!("[{}] {}", self.name, benign.summary);
            }
            // Suppressed (benign).
            return vec![];
        }

        // Real stderr line -- emit as Error.
        if self.debug {
            eprintln!("{line}");
        } else {
            eprintln!("[{}] {line}", self.name);
        }
        vec![HarnessEvent::Error(line.to_string())]
    }

    fn finalize(&mut self) -> Vec<HarnessEvent> {
        // No buffered state to flush.
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> ClaudeStreamJsonParser {
        ClaudeStreamJsonParser::new("test-agent")
    }

    #[test]
    fn parse_content_block_delta() {
        let mut p = parser();
        let line = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello world"}}"#;
        let events = p.parse_stdout_line(line);
        assert!(!events.is_empty());
        match &events[0] {
            HarnessEvent::Output(text) => assert_eq!(text, "Hello world"),
            other => panic!("expected Output, got: {other:?}"),
        }
    }

    #[test]
    fn parse_result_with_usage() {
        let mut p = parser();
        let line = r#"{"type":"result","usage":{"input_tokens":100,"output_tokens":50},"total_cost_usd":0.005,"model":"claude-sonnet-4-20250514"}"#;
        let events = p.parse_stdout_line(line);
        let usage_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, HarnessEvent::Usage { .. }))
            .collect();
        assert_eq!(usage_events.len(), 1);
        match usage_events[0] {
            HarnessEvent::Usage {
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(*input_tokens, 100);
                assert_eq!(*output_tokens, 50);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parse_tool_use_from_assistant() {
        let mut p = parser();
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool_1","name":"Read","input":{"file_path":"/tmp/test.rs"}}]}}"#;
        let events = p.parse_stdout_line(line);
        let tool_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, HarnessEvent::ToolCall { .. }))
            .collect();
        assert_eq!(tool_events.len(), 1);
        match &tool_events[0] {
            HarnessEvent::ToolCall {
                id,
                name,
                arguments,
            } => {
                assert_eq!(id, "tool_1");
                assert_eq!(name, "Read");
                assert!(arguments.get("file_path").is_some());
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parse_error_event() {
        let mut p = parser();
        let line = r#"{"type":"error","error":{"message":"rate limited"}}"#;
        let events = p.parse_stdout_line(line);
        assert!(!events.is_empty());
        match &events[0] {
            HarnessEvent::Error(msg) => {
                assert!(msg.contains("rate limited"));
            }
            other => panic!("expected Error, got: {other:?}"),
        }
    }

    #[test]
    fn parse_non_json_line() {
        let mut p = parser();
        let events = p.parse_stdout_line("this is plain text");
        assert_eq!(events.len(), 1);
        match &events[0] {
            HarnessEvent::Output(text) => {
                assert_eq!(text, "this is plain text");
            }
            other => panic!("expected Output, got: {other:?}"),
        }
    }

    #[test]
    fn stderr_real_error() {
        let mut p = parser();
        let events = p.parse_stderr_line("fatal: something went wrong");
        assert_eq!(events.len(), 1);
        match &events[0] {
            HarnessEvent::Error(msg) => {
                assert!(msg.contains("fatal"));
            }
            other => panic!("expected Error, got: {other:?}"),
        }
    }

    #[test]
    fn parse_system_init_event() {
        let mut p = parser();
        let line = r#"{"type":"system","subtype":"init","session_id":"abc123","model":"claude-sonnet-4-6","tools":[]}"#;
        let events = p.parse_stdout_line(line);
        assert_eq!(events.len(), 1, "expected exactly one Output event");
        match &events[0] {
            HarnessEvent::Output(text) => {
                assert!(
                    text.contains("session=abc123"),
                    "expected session_id in output, got: {text}"
                );
                assert!(
                    text.contains("model=claude-sonnet-4-6"),
                    "expected model in output, got: {text}"
                );
            }
            other => panic!("expected Output, got: {other:?}"),
        }
    }

    #[test]
    fn parse_system_init_event_missing_fields() {
        let mut p = parser();
        // A minimal system event with no session_id or model should still emit
        // an Output (with "unknown" fallbacks) rather than producing nothing.
        let line = r#"{"type":"system","subtype":"init"}"#;
        let events = p.parse_stdout_line(line);
        assert_eq!(events.len(), 1, "expected exactly one Output event");
        match &events[0] {
            HarnessEvent::Output(text) => {
                assert!(
                    text.contains("session=unknown"),
                    "expected fallback session in output, got: {text}"
                );
                assert!(
                    text.contains("model=unknown"),
                    "expected fallback model in output, got: {text}"
                );
            }
            other => panic!("expected Output, got: {other:?}"),
        }
    }

    // ── New tests for previously-untested code paths ───────────────────────

    /// `content_block_start` with a `tool_use` content block emits a `ToolCall`
    /// with `arguments: Value::Null` (arguments arrive in subsequent delta
    /// events, not in the start event itself).
    #[test]
    fn content_block_start_tool_use_emits_tool_call_with_null_arguments() {
        let mut p = parser();
        let line = r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"call_abc","name":"Bash"}}"#;
        let events = p.parse_stdout_line(line);
        assert_eq!(events.len(), 1, "expected exactly one ToolCall event");
        match &events[0] {
            HarnessEvent::ToolCall {
                id,
                name,
                arguments,
            } => {
                assert_eq!(id, "call_abc");
                assert_eq!(name, "Bash");
                assert_eq!(
                    *arguments,
                    serde_json::Value::Null,
                    "arguments should be Null for a content_block_start event"
                );
            }
            other => panic!("expected ToolCall, got: {other:?}"),
        }
    }

    /// A `tool` event with `subtype: "result"` emits a `ToolProgress` event
    /// carrying the tool-use id and the content text.
    #[test]
    fn tool_event_subtype_result_emits_tool_progress() {
        let mut p = parser();
        // Content as a plain string.
        let line =
            r#"{"type":"tool","subtype":"result","tool_use_id":"call_xyz","content":"exit 0"}"#;
        let events = p.parse_stdout_line(line);
        assert_eq!(events.len(), 1, "expected exactly one ToolProgress event");
        match &events[0] {
            HarnessEvent::ToolProgress { id, progress } => {
                assert_eq!(id, "call_xyz");
                assert_eq!(progress, "exit 0");
            }
            other => panic!("expected ToolProgress, got: {other:?}"),
        }
    }

    /// A `tool` event whose `content` field is an array of blocks uses the
    /// `text` field of the first block as the progress string.
    #[test]
    fn tool_event_result_array_content_uses_first_block_text() {
        let mut p = parser();
        let line = r#"{"type":"tool","subtype":"result","tool_use_id":"call_99","content":[{"type":"text","text":"compiled OK"}]}"#;
        let events = p.parse_stdout_line(line);
        assert_eq!(events.len(), 1, "expected exactly one ToolProgress event");
        match &events[0] {
            HarnessEvent::ToolProgress { id, progress } => {
                assert_eq!(id, "call_99");
                assert_eq!(progress, "compiled OK");
            }
            other => panic!("expected ToolProgress, got: {other:?}"),
        }
    }

    /// A `result` event that carries a `stop_reason` field emits a
    /// `StopReason` event (in addition to any `Usage` event when usage is
    /// present).
    #[test]
    fn result_event_with_stop_reason_emits_stop_reason() {
        let mut p = parser();
        let line = r#"{"type":"result","stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":5}}"#;
        let events = p.parse_stdout_line(line);

        let stop_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, HarnessEvent::StopReason(_)))
            .collect();
        assert_eq!(
            stop_events.len(),
            1,
            "expected exactly one StopReason event"
        );
        match stop_events[0] {
            HarnessEvent::StopReason(reason) => {
                assert_eq!(reason, "end_turn");
            }
            _ => unreachable!(),
        }
    }

    /// A benign stderr line (e.g. the Claude CLI startup message) is
    /// classified as benign and suppressed — the parser returns an empty vec.
    #[test]
    fn benign_stderr_line_returns_empty_vec() {
        let mut p = parser();
        // This matches the "claude-cli-startup" pattern in classify_benign_stderr.
        let events = p.parse_stderr_line("Claude CLI is starting...");
        assert!(
            events.is_empty(),
            "benign stderr should be suppressed (empty vec), got: {events:?}"
        );
    }

    /// A `content_block_start` with a `text` content block emits no events
    /// (it only logs a trace message).
    #[test]
    fn content_block_start_text_type_emits_no_events() {
        let mut p = parser();
        let line =
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#;
        let events = p.parse_stdout_line(line);
        assert!(
            events.is_empty(),
            "content_block_start with text type should emit no events, got: {events:?}"
        );
    }
}
