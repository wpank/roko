//! Gemini CLI headless stream-JSON translation.

use serde::Deserialize;
use tracing::debug;

use crate::runtime_events::AgentRuntimeEvent;

const TOOL_OUTPUT_TRUNCATE_AT: usize = roko_core::defaults::DEFAULT_TOOL_OUTPUT_TRUNCATE_AT;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GeminiStreamEvent {
    Init {
        #[serde(default)]
        session_id: String,
        #[serde(default)]
        model: String,
    },
    Message {
        #[serde(default)]
        role: String,
        #[serde(default)]
        content: String,
    },
    ToolUse {
        #[serde(default)]
        tool_name: String,
        #[serde(default)]
        tool_id: String,
    },
    ToolResult {
        #[serde(default)]
        tool_id: String,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        error: Option<GeminiError>,
    },
    Result {
        #[serde(default)]
        status: String,
        #[serde(default)]
        error: Option<GeminiError>,
        #[serde(default)]
        stats: Option<GeminiStats>,
    },
    Error {
        #[serde(default)]
        severity: String,
        #[serde(default)]
        message: String,
    },
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct GeminiStats {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cached: u64,
}

/// Parse one Gemini `--output-format stream-json` line into canonical events.
#[must_use]
pub fn parse_stream_line(line: &str) -> Vec<AgentRuntimeEvent> {
    let line = line.trim();
    if line.is_empty() {
        return Vec::new();
    }
    let event = match serde_json::from_str::<GeminiStreamEvent>(line) {
        Ok(event) => event,
        Err(error) => {
            debug!(line_len = line.len(), %error, "ignoring unparseable Gemini stream line");
            return Vec::new();
        }
    };

    match event {
        GeminiStreamEvent::Init { session_id, model } => {
            vec![AgentRuntimeEvent::SystemInit { session_id, model }]
        }
        GeminiStreamEvent::Message { role, content } if role == "assistant" => {
            vec![AgentRuntimeEvent::MessageDelta { text: content }]
        }
        GeminiStreamEvent::Message { .. } => Vec::new(),
        GeminiStreamEvent::ToolUse { tool_name, tool_id } => vec![AgentRuntimeEvent::ToolCall {
            id: tool_id,
            name: tool_name,
        }],
        GeminiStreamEvent::ToolResult {
            tool_id,
            output,
            error,
        } => {
            let output = output
                .or_else(|| error.map(|error| error.message))
                .unwrap_or_default();
            vec![AgentRuntimeEvent::ToolOutput {
                id: tool_id,
                output: truncate_tool_output(output),
            }]
        }
        GeminiStreamEvent::Result {
            status,
            error,
            stats,
        } => {
            let is_error = status != "success";
            let mut events = Vec::new();
            if let Some(stats) = stats {
                events.push(AgentRuntimeEvent::TokenUsage {
                    input_tokens: stats.input_tokens,
                    output_tokens: stats.output_tokens,
                    cache_read_tokens: stats.cached,
                    cache_write_tokens: 0,
                    reasoning_tokens: 0,
                });
            }
            if is_error {
                events.push(AgentRuntimeEvent::Error {
                    message: error
                        .map(|error| error.message)
                        .filter(|message| !message.is_empty())
                        .unwrap_or_else(|| "Gemini CLI reported an error result".to_string()),
                });
            }
            events.push(AgentRuntimeEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: None,
                num_turns: None,
                is_error,
            });
            events
        }
        GeminiStreamEvent::Error { severity, .. } if severity == "warning" => Vec::new(),
        GeminiStreamEvent::Error { message, .. } => {
            vec![AgentRuntimeEvent::Error { message }]
        }
    }
}

fn truncate_tool_output(output: String) -> String {
    if output.len() <= TOOL_OUTPUT_TRUNCATE_AT {
        return output;
    }
    let boundary = output
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= TOOL_OUTPUT_TRUNCATE_AT)
        .last()
        .unwrap_or(0);
    format!("{}… [truncated]", &output[..boundary])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_lifecycle_messages_and_usage() {
        assert_eq!(
            parse_stream_line(
                r#"{"type":"init","session_id":"session-1","model":"gemini-2.5-pro"}"#
            ),
            vec![AgentRuntimeEvent::SystemInit {
                session_id: "session-1".into(),
                model: "gemini-2.5-pro".into(),
            }]
        );
        assert_eq!(
            parse_stream_line(
                r#"{"type":"message","role":"assistant","content":"done","delta":true}"#
            ),
            vec![AgentRuntimeEvent::MessageDelta {
                text: "done".into()
            }]
        );
        assert_eq!(
            parse_stream_line(
                r#"{"type":"result","status":"success","stats":{"input_tokens":11,"output_tokens":7,"cached":3}}"#
            ),
            vec![
                AgentRuntimeEvent::TokenUsage {
                    input_tokens: 11,
                    output_tokens: 7,
                    cache_read_tokens: 3,
                    cache_write_tokens: 0,
                    reasoning_tokens: 0,
                },
                AgentRuntimeEvent::TurnCompleted {
                    session_id: None,
                    total_cost_usd: None,
                    num_turns: None,
                    is_error: false,
                },
            ]
        );
    }

    #[test]
    fn translates_tool_calls_results_and_terminal_errors() {
        assert_eq!(
            parse_stream_line(
                r#"{"type":"tool_use","tool_name":"demo.echo","tool_id":"call-1","parameters":{"text":"hi"}}"#
            ),
            vec![AgentRuntimeEvent::ToolCall {
                id: "call-1".into(),
                name: "demo.echo".into(),
            }]
        );
        assert_eq!(
            parse_stream_line(
                r#"{"type":"tool_result","tool_id":"call-1","status":"success","output":"hi"}"#
            ),
            vec![AgentRuntimeEvent::ToolOutput {
                id: "call-1".into(),
                output: "hi".into(),
            }]
        );
        let events = parse_stream_line(
            r#"{"type":"result","status":"error","error":{"type":"AUTH","message":"login required"}}"#,
        );
        assert!(
            matches!(events[0], AgentRuntimeEvent::Error { ref message } if message == "login required")
        );
        assert!(matches!(
            events[1],
            AgentRuntimeEvent::TurnCompleted { is_error: true, .. }
        ));
    }

    #[test]
    fn ignores_echoed_user_messages_and_unparseable_lines() {
        assert!(
            parse_stream_line(r#"{"type":"message","role":"user","content":"secret"}"#).is_empty()
        );
        assert!(
            parse_stream_line(r#"{"type":"error","severity":"warning","message":"loop detected"}"#)
                .is_empty()
        );
        assert!(parse_stream_line("not json").is_empty());
    }
}
