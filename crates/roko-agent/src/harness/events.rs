//! Harness event types and the `EventParser` trait.
//!
//! A harness adapter parses raw stdout/stderr lines into [`HarnessEvent`]s,
//! which are then converted into an [`AgentResult`] by
//! [`harness_events_to_agent_result()`].

use crate::agent::AgentResult;
use crate::usage::Usage;
use roko_core::{Body, Kind, Provenance, Signal};
use serde_json::Value;

/// A normalized event emitted by a harness subprocess.
///
/// All agent-specific stream protocols (Claude stream-json, Cursor JSON-RPC,
/// OpenAI SSE, plain text) are parsed into this common representation.
#[derive(Debug, Clone)]
pub enum HarnessEvent {
    /// Text output from the agent.
    Output(String),
    /// A tool call request from the agent.
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
    /// Progress update on a running tool.
    ToolProgress { id: String, progress: String },
    /// Token usage report.
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// Error message from the agent.
    Error(String),
    /// Stop reason from the agent.
    StopReason(String),
}

/// Trait for parsing raw subprocess output lines into [`HarnessEvent`]s.
///
/// Implementations are protocol-specific: `ClaudeStreamJsonParser` for
/// Claude CLI, a future `AcpParser` for ACP, etc.
pub trait EventParser: Send {
    /// Parse a line from stdout. Returns zero or more events.
    fn parse_stdout_line(&mut self, line: &str) -> Vec<HarnessEvent>;

    /// Parse a line from stderr. Returns zero or more events.
    ///
    /// Default implementation emits a single `Error` event.
    fn parse_stderr_line(&mut self, line: &str) -> Vec<HarnessEvent> {
        vec![HarnessEvent::Error(line.to_string())]
    }

    /// Called after the subprocess exits. Returns any buffered events.
    ///
    /// Default implementation returns an empty vec (no buffered state).
    fn finalize(&mut self) -> Vec<HarnessEvent> {
        vec![]
    }
}

/// Convert a vec of [`HarnessEvent`]s into an [`AgentResult`].
///
/// # Arguments
///
/// * `events` - Events collected from `run_one_shot()`.
/// * `input` - The original input `Signal` (used for `derive()`).
/// * `agent_name` - Name of the agent (for provenance tagging).
/// * `wall_ms` - Wall-clock duration of the subprocess run.
///
/// # Returns
///
/// An `AgentResult` with:
/// - `output`: Signal containing concatenated text from `Output` events.
/// - `usage`: Usage from `Usage` events (last one wins).
/// - `trace`: Signals from `Error` events.
/// - `success`: `true` if at least one `Output` event was emitted.
pub fn harness_events_to_agent_result(
    events: &[HarnessEvent],
    input: &Signal,
    agent_name: &str,
    wall_ms: u64,
) -> AgentResult {
    let mut text_parts: Vec<String> = Vec::new();
    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;
    let mut trace_signals: Vec<Signal> = Vec::new();
    let mut stop_reason: Option<String> = None;

    for event in events {
        match event {
            HarnessEvent::Output(text) => {
                text_parts.push(text.clone());
            }
            HarnessEvent::Usage {
                input_tokens: it,
                output_tokens: ot,
            } => {
                // Last usage report wins (result event comes last).
                input_tokens = *it;
                output_tokens = *ot;
            }
            HarnessEvent::Error(msg) => {
                let signal = Signal::builder(Kind::AgentMessage)
                    .body(Body::text(msg))
                    .provenance(Provenance::agent(agent_name))
                    .tag("stream", "stderr")
                    .build();
                trace_signals.push(signal);
            }
            HarnessEvent::StopReason(reason) => {
                stop_reason = Some(reason.clone());
            }
            HarnessEvent::ToolCall { .. } | HarnessEvent::ToolProgress { .. } => {
                // Tool events are informational during streaming.
                // The final text output already includes tool results.
            }
        }
    }

    let output_text = text_parts.join("");

    let usage = Usage {
        input_tokens: u32::try_from(input_tokens).unwrap_or(u32::MAX),
        output_tokens: u32::try_from(output_tokens).unwrap_or(u32::MAX),
        cache_read_tokens: 0,
        cache_create_tokens: 0,
        cost_usd: 0.0,
        wall_ms,
    };

    if output_text.trim().is_empty() {
        let reason = stop_reason.as_deref().unwrap_or("empty response");
        let output = input
            .derive(Kind::AgentOutput, Body::text(reason))
            .provenance(Provenance::agent(agent_name))
            .tag("agent", agent_name)
            .tag("failed", "true")
            .build();
        AgentResult::fail(output)
            .with_usage(usage)
            .with_trace(trace_signals)
    } else {
        let output = input
            .derive(Kind::AgentOutput, Body::text(&output_text))
            .provenance(Provenance::agent(agent_name))
            .tag("agent", agent_name)
            .build();
        AgentResult::ok(output)
            .with_usage(usage)
            .with_trace(trace_signals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a minimal input signal for testing.
    fn test_input_signal() -> Signal {
        Signal::builder(Kind::Task)
            .body(Body::text("test prompt"))
            .provenance(Provenance::agent("test"))
            .build()
    }

    #[test]
    fn harness_events_to_agent_result_text_only() {
        let input = test_input_signal();
        let events = vec![
            HarnessEvent::Output("Hello ".to_string()),
            HarnessEvent::Output("world!".to_string()),
        ];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 100);

        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap(), "Hello world!");
        assert!(result.trace.is_empty());
        assert_eq!(result.usage.wall_ms, 100);
    }

    #[test]
    fn harness_events_to_agent_result_with_error() {
        let input = test_input_signal();
        let events = vec![
            HarnessEvent::Output("partial output".to_string()),
            HarnessEvent::Error("something went wrong".to_string()),
        ];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 200);

        // Should still succeed because there is Output text.
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap(), "partial output");
        // The error event should appear as a trace signal.
        assert_eq!(result.trace.len(), 1);
        assert_eq!(
            result.trace[0].body.as_text().unwrap(),
            "something went wrong"
        );
    }

    #[test]
    fn harness_events_to_agent_result_with_usage() {
        let input = test_input_signal();
        let events = vec![
            HarnessEvent::Output("response".to_string()),
            HarnessEvent::Usage {
                input_tokens: 500,
                output_tokens: 200,
            },
        ];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 300);

        assert!(result.success);
        assert_eq!(result.usage.input_tokens, 500);
        assert_eq!(result.usage.output_tokens, 200);
        assert_eq!(result.usage.wall_ms, 300);
    }

    #[test]
    fn harness_events_to_agent_result_empty() {
        let input = test_input_signal();
        let events: Vec<HarnessEvent> = vec![];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 50);

        // No output events means failure.
        assert!(!result.success);
        // The output body should contain the default stop reason.
        assert_eq!(result.output.body.as_text().unwrap(), "empty response");
        assert_eq!(result.usage.wall_ms, 50);
    }

    /// When output is empty and a StopReason event is present, the stop_reason
    /// string should appear in the failure body instead of "empty response".
    #[test]
    fn harness_events_to_agent_result_stop_reason_no_output() {
        let input = test_input_signal();
        let events = vec![HarnessEvent::StopReason("max_tokens".to_string())];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 75);

        assert!(!result.success);
        assert_eq!(result.output.body.as_text().unwrap(), "max_tokens");
        assert_eq!(result.usage.wall_ms, 75);
    }

    /// When output is non-empty, a StopReason event should be silently dropped;
    /// only the output text should appear in the result body.
    #[test]
    fn harness_events_to_agent_result_stop_reason_with_output() {
        let input = test_input_signal();
        let events = vec![
            HarnessEvent::Output("actual text".to_string()),
            HarnessEvent::StopReason("end_turn".to_string()),
        ];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 120);

        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap(), "actual text");
        // stop_reason must not leak into the output body.
        assert!(!result.output.body.as_text().unwrap().contains("end_turn"));
        assert_eq!(result.usage.wall_ms, 120);
    }

    /// When multiple Usage events are present, only the last one should be
    /// reflected in the result (last-wins semantics).
    #[test]
    fn harness_events_to_agent_result_multiple_usage_last_wins() {
        let input = test_input_signal();
        let events = vec![
            HarnessEvent::Output("response".to_string()),
            HarnessEvent::Usage {
                input_tokens: 100,
                output_tokens: 50,
            },
            HarnessEvent::Usage {
                input_tokens: 800,
                output_tokens: 320,
            },
        ];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 400);

        assert!(result.success);
        // The second Usage event must overwrite the first.
        assert_eq!(result.usage.input_tokens, 800);
        assert_eq!(result.usage.output_tokens, 320);
    }

    /// ToolCall and ToolProgress events are informational only; they must not
    /// contribute any text to the output body.
    #[test]
    fn harness_events_to_agent_result_tool_events_silently_skipped() {
        let input = test_input_signal();
        let events = vec![
            HarnessEvent::ToolCall {
                id: "call-1".to_string(),
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path": "/tmp/foo"}),
            },
            HarnessEvent::ToolProgress {
                id: "call-1".to_string(),
                progress: "50%".to_string(),
            },
            HarnessEvent::Output("final answer".to_string()),
        ];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 250);

        assert!(result.success);
        // Only the Output event should appear in the body.
        assert_eq!(result.output.body.as_text().unwrap(), "final answer");
        // Tool events must not leak into trace either.
        assert!(result.trace.is_empty());
    }

    /// When only Error events are present (no Output), the result must be a
    /// failure and the error messages must appear as trace signals.
    #[test]
    fn harness_events_to_agent_result_only_errors() {
        let input = test_input_signal();
        let events = vec![
            HarnessEvent::Error("connection refused".to_string()),
            HarnessEvent::Error("timeout after 30s".to_string()),
        ];
        let result = harness_events_to_agent_result(&events, &input, "test-agent", 30_000);

        assert!(!result.success);
        // Both error messages must be recorded as trace signals.
        assert_eq!(result.trace.len(), 2);
        assert_eq!(
            result.trace[0].body.as_text().unwrap(),
            "connection refused"
        );
        assert_eq!(result.trace[1].body.as_text().unwrap(), "timeout after 30s");
        assert_eq!(result.usage.wall_ms, 30_000);
    }
}
