//! Parser for OpenClaw's stable infer JSON envelope.
//!
//! Schema: openclaw/docs/cli/infer.md, "JSON output" section.
//! The envelope has exactly 8 stable top-level fields:
//! `ok`, `capability`, `transport`, `provider`, `model`, `attempts`,
//! `outputs`, `error`.
//!
//! The `outputs` array contains objects with `type`, `value`, `path`,
//! `mime_type`, `size`. The `error` object has `kind` and `message`.
//!
//! **Important**: The envelope does NOT include a `usage` field.
//! Token accounting for OpenClaw infer uses character-count estimation.

use serde::Deserialize;
use std::path::PathBuf;

use crate::harness::error::HarnessError;
use crate::harness::events::{EventParser, HarnessEvent};

/// OpenClaw's stable 8-field JSON envelope from `openclaw infer ... --json`.
///
/// Example:
/// ```json
/// {
///   "ok": true,
///   "capability": "model",
///   "transport": "api",
///   "provider": "anthropic",
///   "model": "claude-sonnet-4-5-20250514",
///   "attempts": 1,
///   "outputs": [{"type": "text", "value": "Hello!"}],
///   "error": null
/// }
/// ```
#[derive(Clone, Debug, Deserialize)]
pub struct InferEnvelope {
    /// Whether the inference succeeded.
    pub ok: bool,

    /// The capability that was invoked (e.g. `"model"`).
    pub capability: String,

    /// Transport used: `"api"` or `"gateway"`.
    pub transport: String,

    /// Provider name (e.g. `"openai"`, `"anthropic"`).
    pub provider: Option<String>,

    /// Model identifier that was used.
    pub model: Option<String>,

    /// Number of retry attempts made.
    #[serde(default)]
    pub attempts: u32,

    /// Result objects. For text inference, typically one entry with
    /// `type=text` and `value` containing the completion text.
    #[serde(default)]
    pub outputs: Vec<InferOutput>,

    /// Error details when `ok` is `false`. `None` when `ok` is `true`.
    pub error: Option<InferError>,
}

/// A single output entry in the infer envelope.
#[derive(Clone, Debug, Deserialize)]
pub struct InferOutput {
    /// Output type: `"text"` or `"file"`.
    #[serde(rename = "type")]
    pub kind: String,

    /// Text content (present when `kind == "text"`).
    pub value: Option<String>,

    /// File path (present when `kind == "file"`).
    pub path: Option<PathBuf>,

    /// MIME type of the output.
    pub mime_type: Option<String>,

    /// Byte size of the output.
    pub size: Option<u64>,
}

/// Error details from a failed infer invocation.
#[derive(Clone, Debug, Deserialize)]
pub struct InferError {
    /// Error category: `"auth"`, `"unknown_model"`, `"timeout"`,
    /// `"rate_limit"`, `"context_length"`, `"server_error"`, `"network"`.
    pub kind: String,

    /// Human-readable error description.
    pub message: String,
}

impl InferEnvelope {
    /// Extract the final text output from the envelope.
    ///
    /// Returns the `value` of the first output with `type == "text"`,
    /// or `None` if no text output exists.
    pub fn final_text(&self) -> Option<&str> {
        self.outputs
            .iter()
            .find(|o| o.kind == "text")
            .and_then(|o| o.value.as_deref())
    }

    /// Map an envelope-level error to a `HarnessError`.
    ///
    /// Called when `ok == false` to produce a typed error for the
    /// adapter's error path.
    ///
    /// ## Error mapping
    ///
    /// | Envelope `error.kind` | `HarnessError` variant |
    /// |---|---|
    /// | `"auth"` | `HarnessError::Auth(message)` |
    /// | `"timeout"` | `HarnessError::Timeout { ... }` |
    /// | any other kind | `HarnessError::Protocol(formatted message)` |
    pub fn to_harness_error(&self) -> Option<HarnessError> {
        self.error.as_ref().map(|e| match e.kind.as_str() {
            "auth" => HarnessError::Auth(e.message.clone()),
            "timeout" => HarnessError::Timeout {
                elapsed: std::time::Duration::ZERO,
                configured: std::time::Duration::ZERO,
            },
            other => {
                HarnessError::Protocol(format!("openclaw infer error [{}]: {}", other, e.message))
            }
        })
    }

    /// Estimate token usage from the prompt and output text.
    ///
    /// OpenClaw's `infer --json` does not report token usage. This method
    /// uses the standard heuristic of 1 token per 4 characters.
    ///
    /// Formula:
    /// - `input_tokens = max(prompt_chars / 4, 1)`
    /// - `output_tokens = max(output_chars / 4, 1)`
    pub fn estimate_usage(&self, prompt: &str) -> (u64, u64) {
        let output_text = self.final_text().unwrap_or("");
        let input_tokens = (prompt.len() as u64 / 4).max(1);
        let output_tokens = (output_text.len() as u64 / 4).max(1);
        (input_tokens, output_tokens)
    }
}

/// `EventParser` implementation for OpenClaw's `infer --json` output.
///
/// The `infer --json` command outputs a single JSON object to stdout
/// upon completion. The parser accumulates stdout lines into a buffer
/// and parses the complete output as an `InferEnvelope` when
/// `finalize()` is called after the child exits.
///
/// Stderr output is emitted as `HarnessEvent::Error` strings (OpenClaw
/// may emit progress messages or Node.js deprecation notices on stderr).
pub struct InferEventParser {
    /// Accumulated stdout content.
    stdout_buf: String,
    /// Accumulated stderr lines for diagnostics.
    stderr_lines: Vec<String>,
}

impl InferEventParser {
    /// Create a new parser.
    pub fn new() -> Self {
        Self {
            stdout_buf: String::new(),
            stderr_lines: Vec::new(),
        }
    }

    /// Parse the accumulated stdout buffer as an `InferEnvelope`.
    ///
    /// This is called from `finalize()` after the child process has
    /// finished writing all output.
    fn parse_envelope(&self) -> Result<InferEnvelope, HarnessError> {
        let trimmed = self.stdout_buf.trim();
        if trimmed.is_empty() {
            return Err(HarnessError::Protocol(
                "openclaw infer produced no stdout output".to_string(),
            ));
        }
        serde_json::from_str(trimmed).map_err(|e| {
            HarnessError::Protocol(format!("failed to parse openclaw infer JSON envelope: {e}"))
        })
    }
}

impl Default for InferEventParser {
    fn default() -> Self {
        Self::new()
    }
}

impl EventParser for InferEventParser {
    /// Accumulate stdout lines. The infer --json output is a single
    /// JSON object that may span multiple lines (pretty-printed).
    /// No events are emitted line-by-line -- the envelope is parsed
    /// as a whole in `finalize()`.
    fn parse_stdout_line(&mut self, line: &str) -> Vec<HarnessEvent> {
        self.stdout_buf.push_str(line);
        self.stdout_buf.push('\n');
        // No events emitted until finalize() -- the JSON envelope
        // must be parsed as a complete unit.
        Vec::new()
    }

    /// Capture stderr lines. OpenClaw may emit progress messages,
    /// Node.js deprecation notices, or diagnostic output on stderr.
    /// Non-empty lines are emitted as `HarnessEvent::Error` for
    /// upstream logging.
    fn parse_stderr_line(&mut self, line: &str) -> Vec<HarnessEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        self.stderr_lines.push(trimmed.to_string());
        vec![HarnessEvent::Error(format!("[openclaw stderr] {trimmed}"))]
    }

    /// Parse the accumulated stdout buffer as an `InferEnvelope` and
    /// convert it to `HarnessEvent` values.
    ///
    /// This is called by `ChildProcessRunner` after the child process
    /// has exited and all stdout/stderr has been consumed.
    ///
    /// On success (`ok == true`):
    /// - Emits `HarnessEvent::Output(text)` with the final text.
    /// - Emits `HarnessEvent::StopReason("end_turn".to_string())`.
    ///
    /// On failure (`ok == false`):
    /// - Emits `HarnessEvent::Error(message)` with the error details.
    /// - Emits `HarnessEvent::StopReason("error".to_string())`.
    ///
    /// If the JSON cannot be parsed at all:
    /// - Emits `HarnessEvent::Error(parse_error_message)`.
    fn finalize(&mut self) -> Vec<HarnessEvent> {
        let mut events = Vec::new();

        match self.parse_envelope() {
            Ok(envelope) => {
                if envelope.ok {
                    // Success path: emit the text output.
                    let text = envelope
                        .final_text()
                        .unwrap_or("(no text output)")
                        .to_string();
                    events.push(HarnessEvent::Output(text));
                    events.push(HarnessEvent::StopReason("end_turn".to_string()));
                } else {
                    // Error path: map envelope error to HarnessError string.
                    let error_msg = match &envelope.error {
                        Some(e) => format!("openclaw infer error [{}]: {}", e.kind, e.message),
                        None => {
                            "openclaw infer failed with ok=false but no error details".to_string()
                        }
                    };
                    events.push(HarnessEvent::Error(error_msg));
                    events.push(HarnessEvent::StopReason("error".to_string()));
                }
            }
            Err(e) => {
                events.push(HarnessEvent::Error(format!("{e}")));
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASIC_ENVELOPE: &str = r#"{
        "ok": true,
        "capability": "model",
        "transport": "api",
        "provider": "anthropic",
        "model": "claude-sonnet-4-5-20250514",
        "attempts": 1,
        "outputs": [{"type": "text", "value": "Paris is the capital of France."}],
        "error": null
    }"#;

    const ERROR_ENVELOPE: &str = r#"{
        "ok": false,
        "capability": "model",
        "transport": "api",
        "provider": "anthropic",
        "model": "claude-sonnet-4-5-20250514",
        "attempts": 3,
        "outputs": [],
        "error": {"kind": "auth", "message": "API key invalid"}
    }"#;

    const UNKNOWN_MODEL_ENVELOPE: &str = r#"{
        "ok": false,
        "capability": "model",
        "transport": "api",
        "provider": "openai",
        "model": "nonexistent/model-9",
        "attempts": 2,
        "outputs": [],
        "error": {"kind": "unknown_model", "message": "Model 'nonexistent/model-9' not found"}
    }"#;

    const EMPTY_OUTPUTS_ENVELOPE: &str = r#"{
        "ok": true,
        "capability": "model",
        "transport": "gateway",
        "provider": "anthropic",
        "model": "claude-sonnet-4-5-20250514",
        "attempts": 1,
        "outputs": [],
        "error": null
    }"#;

    const THINKING_ENVELOPE: &str = r#"{
        "ok": true,
        "capability": "model",
        "transport": "api",
        "provider": "openai",
        "model": "gpt-5.5",
        "attempts": 1,
        "outputs": [
            {"type": "text", "value": "The answer is 42."}
        ],
        "error": null
    }"#;

    const RATE_LIMIT_ENVELOPE: &str = r#"{
        "ok": false,
        "capability": "model",
        "transport": "api",
        "provider": "openai",
        "model": "gpt-5.5",
        "attempts": 3,
        "outputs": [],
        "error": {"kind": "rate_limit", "message": "Rate limit exceeded, retry after 30s"}
    }"#;

    const NETWORK_ERROR_ENVELOPE: &str = r#"{
        "ok": false,
        "capability": "model",
        "transport": "api",
        "provider": "anthropic",
        "model": "claude-sonnet-4-5-20250514",
        "attempts": 1,
        "outputs": [],
        "error": {"kind": "network", "message": "Connection refused"}
    }"#;

    #[test]
    fn parse_basic_envelope() {
        let env: InferEnvelope = serde_json::from_str(BASIC_ENVELOPE).unwrap();
        assert!(env.ok);
        assert_eq!(env.capability, "model");
        assert_eq!(env.transport, "api");
        assert_eq!(env.provider.as_deref(), Some("anthropic"));
        assert_eq!(env.model.as_deref(), Some("claude-sonnet-4-5-20250514"));
        assert_eq!(env.attempts, 1);
        assert_eq!(env.outputs.len(), 1);
        assert_eq!(env.outputs[0].kind, "text");
        assert_eq!(
            env.outputs[0].value.as_deref(),
            Some("Paris is the capital of France.")
        );
        assert!(env.error.is_none());
    }

    #[test]
    fn final_text_extracts_first_text_output() {
        let env: InferEnvelope = serde_json::from_str(BASIC_ENVELOPE).unwrap();
        assert_eq!(env.final_text(), Some("Paris is the capital of France."));
    }

    #[test]
    fn final_text_returns_none_for_empty_outputs() {
        let env: InferEnvelope = serde_json::from_str(EMPTY_OUTPUTS_ENVELOPE).unwrap();
        assert!(env.final_text().is_none());
    }

    #[test]
    fn parse_error_envelope() {
        let env: InferEnvelope = serde_json::from_str(ERROR_ENVELOPE).unwrap();
        assert!(!env.ok);
        assert!(env.outputs.is_empty());
        let err = env.error.as_ref().unwrap();
        assert_eq!(err.kind, "auth");
        assert!(err.message.contains("API key"));
    }

    #[test]
    fn parse_unknown_model_envelope() {
        let env: InferEnvelope = serde_json::from_str(UNKNOWN_MODEL_ENVELOPE).unwrap();
        assert!(!env.ok);
        let err = env.error.as_ref().unwrap();
        assert_eq!(err.kind, "unknown_model");
        assert_eq!(env.attempts, 2);
    }

    #[test]
    fn parse_thinking_envelope() {
        let env: InferEnvelope = serde_json::from_str(THINKING_ENVELOPE).unwrap();
        assert!(env.ok);
        assert_eq!(env.final_text(), Some("The answer is 42."));
    }

    #[test]
    fn all_eight_fields_present() {
        // Verify all 8 stable fields are accessible after deserialization.
        let env: InferEnvelope = serde_json::from_str(BASIC_ENVELOPE).unwrap();
        let _ = env.ok;
        let _ = &env.capability;
        let _ = &env.transport;
        let _ = &env.provider;
        let _ = &env.model;
        let _ = env.attempts;
        let _ = &env.outputs;
        let _ = &env.error;
    }

    #[test]
    fn error_to_harness_error_auth() {
        let env: InferEnvelope = serde_json::from_str(ERROR_ENVELOPE).unwrap();
        let err = env.to_harness_error().unwrap();
        assert!(matches!(err, HarnessError::Auth(_)));
    }

    #[test]
    fn error_to_harness_error_timeout() {
        let json = r#"{
            "ok": false, "capability": "model", "transport": "api",
            "provider": "openai", "model": "gpt-5.5", "attempts": 1,
            "outputs": [],
            "error": {"kind": "timeout", "message": "Request timed out after 90s"}
        }"#;
        let env: InferEnvelope = serde_json::from_str(json).unwrap();
        let err = env.to_harness_error().unwrap();
        assert!(matches!(err, HarnessError::Timeout { .. }));
    }

    #[test]
    fn error_to_harness_error_rate_limit() {
        let env: InferEnvelope = serde_json::from_str(RATE_LIMIT_ENVELOPE).unwrap();
        let err = env.to_harness_error().unwrap();
        // rate_limit maps to Protocol since HarnessError has no RateLimit variant
        assert!(matches!(err, HarnessError::Protocol(_)));
    }

    #[test]
    fn error_to_harness_error_network() {
        let env: InferEnvelope = serde_json::from_str(NETWORK_ERROR_ENVELOPE).unwrap();
        let err = env.to_harness_error().unwrap();
        // network maps to Protocol since HarnessError has no Network variant
        assert!(matches!(err, HarnessError::Protocol(_)));
    }

    #[test]
    fn error_to_harness_error_unknown_model() {
        let env: InferEnvelope = serde_json::from_str(UNKNOWN_MODEL_ENVELOPE).unwrap();
        let err = env.to_harness_error().unwrap();
        // unknown_model maps to Protocol since HarnessError has no UnknownModel variant
        assert!(matches!(err, HarnessError::Protocol(_)));
    }

    #[test]
    fn error_to_harness_error_unknown_kind_falls_back_to_protocol() {
        let json = r#"{
            "ok": false, "capability": "model", "transport": "api",
            "provider": "openai", "model": "gpt-5.5", "attempts": 1,
            "outputs": [],
            "error": {"kind": "something_new", "message": "unexpected error"}
        }"#;
        let env: InferEnvelope = serde_json::from_str(json).unwrap();
        let err = env.to_harness_error().unwrap();
        assert!(matches!(err, HarnessError::Protocol(_)));
    }

    #[test]
    fn estimate_usage_basic() {
        let env: InferEnvelope = serde_json::from_str(BASIC_ENVELOPE).unwrap();
        let (input, output) = env.estimate_usage("What is the capital of France?");
        // "What is the capital of France?" = 31 chars -> 31/4 = 7
        assert_eq!(input, 7);
        // "Paris is the capital of France." = 31 chars -> 31/4 = 7
        assert_eq!(output, 7);
    }

    #[test]
    fn estimate_usage_minimum_is_one() {
        let env: InferEnvelope = serde_json::from_str(EMPTY_OUTPUTS_ENVELOPE).unwrap();
        let (input, output) = env.estimate_usage("hi");
        // "hi" = 2 chars -> 2/4 = 0 -> clamped to 1
        assert_eq!(input, 1);
        // no text output -> 0/4 = 0 -> clamped to 1
        assert_eq!(output, 1);
    }

    #[test]
    fn event_parser_accumulates_stdout() {
        let mut parser = InferEventParser::new();
        // Feed the envelope line by line -- no events emitted mid-stream
        let lines: Vec<&str> = BASIC_ENVELOPE.lines().collect();
        for line in &lines {
            let events = parser.parse_stdout_line(line);
            assert!(events.is_empty(), "no events should be emitted mid-stream");
        }
    }

    #[test]
    fn event_parser_finalize_success() {
        let mut parser = InferEventParser::new();
        // Feed entire envelope
        for line in BASIC_ENVELOPE.lines() {
            parser.parse_stdout_line(line);
        }

        let events = parser.finalize();

        let has_output = events
            .iter()
            .any(|e| matches!(e, HarnessEvent::Output(t) if t.contains("Paris")));
        let has_stop = events
            .iter()
            .any(|e| matches!(e, HarnessEvent::StopReason(r) if r == "end_turn"));
        assert!(has_output, "should have emitted an Output event");
        assert!(has_stop, "should have emitted a StopReason event");
    }

    #[test]
    fn event_parser_finalize_error() {
        let mut parser = InferEventParser::new();
        for line in ERROR_ENVELOPE.lines() {
            parser.parse_stdout_line(line);
        }

        let events = parser.finalize();

        let has_error = events
            .iter()
            .any(|e| matches!(e, HarnessEvent::Error(msg) if msg.contains("auth")));
        let has_stop = events
            .iter()
            .any(|e| matches!(e, HarnessEvent::StopReason(r) if r == "error"));
        assert!(has_error, "should have emitted an Error event");
        assert!(has_stop, "should have emitted a StopReason(error) event");
    }

    #[test]
    fn event_parser_finalize_empty_stdout() {
        let mut parser = InferEventParser::new();
        let events = parser.finalize();
        let has_error = events
            .iter()
            .any(|e| matches!(e, HarnessEvent::Error(msg) if msg.contains("no stdout")));
        assert!(has_error, "should report empty stdout as error");
    }

    #[test]
    fn event_parser_stderr_produces_error_event() {
        let mut parser = InferEventParser::new();
        let events = parser.parse_stderr_line("(node:12345) ExperimentalWarning: ...");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            HarnessEvent::Error(msg) if msg.contains("ExperimentalWarning")
        ));
    }

    #[test]
    fn event_parser_empty_stderr_ignored() {
        let mut parser = InferEventParser::new();
        let events = parser.parse_stderr_line("   ");
        assert!(events.is_empty());
    }
}
