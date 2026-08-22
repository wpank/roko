//! Codex CLI `exec --json` JSONL parser.
//!
//! Codex emits JSON-Lines on stdout when invoked with `codex exec --json`.
//! Events include `thread.started`, `turn.started`, `item.started`,
//! `item.completed`, and `turn.completed`. This module translates each line
//! into provider-neutral [`AgentRuntimeEvent`]s.

use serde::Deserialize;
use tracing::debug;

use crate::runtime_events::AgentRuntimeEvent;

// ── Wire types ──────────────────────────────────────────────────────────

/// Top-level Codex JSONL event (untagged because `type` values use dots).
#[derive(Debug, Deserialize)]
struct CodexEvent {
    #[serde(rename = "type")]
    event_type: String,
    /// Present on `thread.started`.
    #[serde(default)]
    thread_id: Option<String>,
    /// Present on `item.started` and `item.completed`.
    #[serde(default)]
    item: Option<CodexItem>,
    /// Present on `turn.completed`.
    #[serde(default)]
    usage: Option<CodexUsage>,
}

#[derive(Debug, Deserialize)]
struct CodexItem {
    #[serde(default)]
    id: String,
    #[serde(rename = "type", default)]
    item_type: String,
    /// Agent text message (on `agent_message` items).
    #[serde(default)]
    text: Option<String>,
    /// Command string (on `command_execution` items).
    #[serde(default)]
    command: Option<String>,
    /// Command output (on completed `command_execution` items).
    #[serde(default)]
    aggregated_output: Option<String>,
    /// Exit code (on completed `command_execution` items).
    #[serde(default)]
    exit_code: Option<i32>,
    /// File changes (on `file_change` items).
    #[serde(default)]
    changes: Option<Vec<CodexFileChange>>,
    /// Item status (`in_progress`, `completed`).
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexFileChange {
    #[serde(default)]
    path: String,
    #[serde(default)]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct CodexUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    reasoning_output_tokens: u64,
}

// ── Parser ──────────────────────────────────────────────────────────────

/// Parse one Codex `exec --json` JSONL line into canonical runtime events.
#[must_use]
pub fn parse_stream_line(line: &str) -> Vec<AgentRuntimeEvent> {
    let line = line.trim();
    if line.is_empty() {
        return Vec::new();
    }

    let event: CodexEvent = match serde_json::from_str(line) {
        Ok(e) => e,
        Err(e) => {
            debug!(line_len = line.len(), err = %e, "ignoring unparseable codex line");
            return Vec::new();
        }
    };

    match event.event_type.as_str() {
        "thread.started" => {
            let session_id = event.thread_id.unwrap_or_default();
            vec![AgentRuntimeEvent::SystemInit {
                session_id,
                model: String::new(),
            }]
        }

        "item.completed" => parse_item_completed(event.item),

        "item.started" => {
            // Emit a ToolCall for command_execution so the TUI can show it.
            if let Some(item) = &event.item {
                if item.item_type == "command_execution" {
                    let name = "command_execution".to_string();
                    return vec![AgentRuntimeEvent::ToolCall {
                        id: item.id.clone(),
                        name,
                    }];
                }
                if item.item_type == "file_change" {
                    let name = "file_change".to_string();
                    return vec![AgentRuntimeEvent::ToolCall {
                        id: item.id.clone(),
                        name,
                    }];
                }
            }
            Vec::new()
        }

        "turn.completed" => {
            let mut events = Vec::new();
            if let Some(usage) = event.usage {
                events.push(AgentRuntimeEvent::TokenUsage {
                    input_tokens: usage.input_tokens,
                    output_tokens: usage.output_tokens,
                    cache_read_tokens: usage.cached_input_tokens,
                    cache_write_tokens: 0,
                });
            }
            // Codex's turn.completed is the terminal event — synthesize
            // TurnCompleted + Exited so the runner knows the agent finished.
            events.push(AgentRuntimeEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: None,
                num_turns: None,
                is_error: false,
            });
            events.push(AgentRuntimeEvent::Exited {
                exit_code: Some(0),
            });
            events
        }

        "turn.started" => Vec::new(),

        other => {
            debug!(event_type = other, "ignoring unknown codex event type");
            Vec::new()
        }
    }
}

fn parse_item_completed(item: Option<CodexItem>) -> Vec<AgentRuntimeEvent> {
    let Some(item) = item else {
        return Vec::new();
    };

    match item.item_type.as_str() {
        "agent_message" => {
            if let Some(text) = item.text {
                if !text.is_empty() {
                    return vec![AgentRuntimeEvent::MessageDelta { text }];
                }
            }
            Vec::new()
        }

        "command_execution" => {
            let output = item.aggregated_output.unwrap_or_default();
            let truncated = if output.len() > 4096 {
                format!("{}\u{2026} [truncated]", &output[..4096])
            } else {
                output
            };
            vec![AgentRuntimeEvent::ToolOutput {
                id: item.id,
                output: truncated,
            }]
        }

        "file_change" => {
            let summary = item
                .changes
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|c| format!("{}: {}", c.kind, c.path))
                .collect::<Vec<_>>()
                .join(", ");
            vec![AgentRuntimeEvent::ToolOutput {
                id: item.id,
                output: summary,
            }]
        }

        other => {
            debug!(item_type = other, "ignoring unknown codex item type");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_started() {
        let events = parse_stream_line(
            r#"{"type":"thread.started","thread_id":"abc-123"}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentRuntimeEvent::SystemInit { session_id, .. } if session_id == "abc-123"));
    }

    #[test]
    fn agent_message() {
        let events = parse_stream_line(
            r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Hello"}}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentRuntimeEvent::MessageDelta { text } if text == "Hello"));
    }

    #[test]
    fn command_execution() {
        let events = parse_stream_line(
            r#"{"type":"item.completed","item":{"id":"item_2","type":"command_execution","command":"ls","aggregated_output":"file.txt\n","exit_code":0,"status":"completed"}}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentRuntimeEvent::ToolOutput { id, output } if id == "item_2" && output == "file.txt\n"));
    }

    #[test]
    fn turn_completed_with_usage() {
        let events = parse_stream_line(
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"cache_write_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":0}}"#,
        );
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], AgentRuntimeEvent::TokenUsage { input_tokens: 100, output_tokens: 10, .. }));
        assert!(matches!(&events[1], AgentRuntimeEvent::TurnCompleted { is_error: false, .. }));
        assert!(matches!(&events[2], AgentRuntimeEvent::Exited { exit_code: Some(0) }));
    }

    #[test]
    fn file_change() {
        let events = parse_stream_line(
            r#"{"type":"item.completed","item":{"id":"item_1","type":"file_change","changes":[{"path":"/tmp/test.txt","kind":"add"}],"status":"completed"}}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentRuntimeEvent::ToolOutput { output, .. } if output.contains("add: /tmp/test.txt")));
    }

    #[test]
    fn empty_and_unknown_lines() {
        assert!(parse_stream_line("").is_empty());
        assert!(parse_stream_line(r#"{"type":"turn.started"}"#).is_empty());
        assert!(parse_stream_line("not json").is_empty());
    }
}
