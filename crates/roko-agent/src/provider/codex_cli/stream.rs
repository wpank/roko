//! Codex CLI `exec --json` JSONL parser.
//!
//! Codex emits JSON-Lines on stdout when invoked with `codex exec --json`.
//! Events include `thread.started`, `turn.started`, `item.started`,
//! `item.completed`, and `turn.completed`. This module translates each line
//! into provider-neutral [`AgentRuntimeEvent`]s.

use std::sync::LazyLock;

use roko_core::pricing_snapshot::{PriceRow, PriceSnapshot, TokenCounts};
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
    /// Total output tokens; reasoning tokens are a subset of this total,
    /// not an addition to it (OpenAI Responses API convention).
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    /// Reasoning (thinking) tokens, already included in `output_tokens`.
    /// Forwarded on the wire via `AgentRuntimeEvent::TokenUsage::reasoning_tokens`.
    #[serde(default)]
    reasoning_output_tokens: u64,
}

// ── Cost estimation ─────────────────────────────────────────────────────

/// The built-in price snapshot, its default id, read once: Codex reports no
/// USD figure of its own, so a turn's cost is its tokens at this snapshot's
/// rates (backlog 6106).
static DEFAULT_SNAPSHOT: LazyLock<Option<PriceSnapshot>> =
    LazyLock::new(|| PriceSnapshot::builtin().ok());

/// The row of the built-in price snapshot that prices Codex turns on
/// `model`. `None` for an empty slug or one the snapshot does not list,
/// whose turns then have an unknown cost, never another model's (backlog
/// 6106). Resolve it once per dispatch and parse with
/// [`parse_stream_line_priced`].
#[must_use]
pub fn codex_price_row(model: Option<&str>) -> Option<PriceRow> {
    let slug = model.map(str::trim).filter(|slug| !slug.is_empty())?;
    DEFAULT_SNAPSHOT.as_ref()?.row(slug).cloned()
}

/// What a Codex turn's `usage` costs at `row`'s rates: its uncached input,
/// cached input and output, plus its reasoning tokens at the output rate
/// only when the row says they are not inside the output (backlog 6106).
fn estimate_codex_cost(usage: &CodexUsage, row: &PriceRow) -> f64 {
    let tokens = TokenCounts {
        input: usage.input_tokens.saturating_sub(usage.cached_input_tokens),
        cache_read: usage.cached_input_tokens,
        cache_write_5m: 0,
        cache_write_1h: 0,
        output: usage.output_tokens,
        reasoning: usage.reasoning_output_tokens,
    };
    row.price(&tokens).api_equiv_usd
}

// ── Parser ──────────────────────────────────────────────────────────────

/// Parse one Codex `exec --json` JSONL line into canonical runtime events.
///
/// It names no model, so a turn's cost is unknown.
#[must_use]
pub fn parse_stream_line(line: &str) -> Vec<AgentRuntimeEvent> {
    parse_stream_line_priced(line, None)
}

/// Parse one Codex `exec --json` JSONL line into canonical runtime events,
/// pricing turns on `model` at the built-in price snapshot
/// ([`codex_price_row`]).
///
/// `model` only affects the `TurnCompleted.total_cost_usd` estimate, which
/// is `None` for an empty slug or one the snapshot does not list.
#[must_use]
pub fn parse_stream_line_with_model(line: &str, model: Option<&str>) -> Vec<AgentRuntimeEvent> {
    parse_stream_line_priced(line, codex_price_row(model).as_ref())
}

/// Parse one Codex `exec --json` JSONL line into canonical runtime events,
/// pricing turns at `row`, the price snapshot row the caller resolved once
/// for its dispatch. With no row, a turn's cost is unknown.
#[must_use]
pub fn parse_stream_line_priced(line: &str, row: Option<&PriceRow>) -> Vec<AgentRuntimeEvent> {
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
            // Codex's `thread.started` carries only a thread id — never a
            // model slug. Emitting `SystemInit { model: "" }` here would
            // overwrite the requested model in runner state (episodes get
            // dropped on the empty-model guard and the TUI model blanks),
            // so no event is emitted for this line.
            debug!(
                thread_id = event.thread_id.as_deref().unwrap_or(""),
                "codex thread started"
            );
            Vec::new()
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
            let total_cost_usd = event
                .usage
                .as_ref()
                .zip(row)
                .map(|(usage, row)| estimate_codex_cost(usage, row));
            if let Some(usage) = event.usage {
                events.push(AgentRuntimeEvent::TokenUsage {
                    input_tokens: usage.input_tokens,
                    output_tokens: usage.output_tokens,
                    cache_read_tokens: usage.cached_input_tokens,
                    cache_write_tokens: 0,
                    // Reasoning tokens are a subset of `output_tokens`
                    // (already billed at the output rate by the estimate
                    // above); forward the count instead of dropping it.
                    reasoning_tokens: usage.reasoning_output_tokens,
                });
            }
            // Codex's turn.completed is the terminal event — synthesize
            // TurnCompleted + Exited so the runner knows the agent finished.
            events.push(AgentRuntimeEvent::TurnCompleted {
                session_id: None,
                total_cost_usd,
                num_turns: None,
                is_error: false,
            });
            events.push(AgentRuntimeEvent::Exited { exit_code: Some(0) });
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
            if let Some(text) = item.text
                && !text.is_empty()
            {
                return vec![AgentRuntimeEvent::MessageDelta { text }];
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
        // `thread.started` carries no model slug, so the parser must not
        // emit a `SystemInit` with an empty model (it would wipe the
        // requested model from runner state).
        let events = parse_stream_line(r#"{"type":"thread.started","thread_id":"abc-123"}"#);
        assert!(events.is_empty());
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, AgentRuntimeEvent::SystemInit { .. }))
        );
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
        assert!(
            matches!(&events[0], AgentRuntimeEvent::ToolOutput { id, output } if id == "item_2" && output == "file.txt\n")
        );
    }

    #[test]
    fn turn_completed_with_usage() {
        let events = parse_stream_line_with_model(
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"cache_write_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":0}}"#,
            Some("gpt-5.5"),
        );
        assert_eq!(events.len(), 3);
        assert!(matches!(
            &events[0],
            AgentRuntimeEvent::TokenUsage {
                input_tokens: 100,
                output_tokens: 10,
                ..
            }
        ));
        assert!(matches!(
            &events[1],
            AgentRuntimeEvent::TurnCompleted {
                is_error: false,
                total_cost_usd: Some(cost),
                ..
            } if *cost > 0.0
        ));
        assert!(matches!(
            &events[2],
            AgentRuntimeEvent::Exited { exit_code: Some(0) }
        ));
    }

    #[test]
    fn file_change() {
        let events = parse_stream_line(
            r#"{"type":"item.completed","item":{"id":"item_1","type":"file_change","changes":[{"path":"/tmp/test.txt","kind":"add"}],"status":"completed"}}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0], AgentRuntimeEvent::ToolOutput { output, .. } if output.contains("add: /tmp/test.txt"))
        );
    }

    /// The turn the snapshot tests price: 1,000 input tokens (400 cached)
    /// and 100 output tokens, 30 of them reasoning.
    const PRICED_TURN: &str = r#"{"type":"turn.completed","usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":100,"reasoning_output_tokens":30}}"#;

    /// The cost a turn's events report.
    fn turn_cost(events: &[AgentRuntimeEvent]) -> Option<f64> {
        events.iter().find_map(|event| match event {
            AgentRuntimeEvent::TurnCompleted { total_cost_usd, .. } => *total_cost_usd,
            _ => None,
        })
    }

    /// backlog 6106: a Codex turn is priced at its model's price snapshot
    /// row, its reasoning inside the output.
    #[test]
    fn codex_cost_uses_snapshot() {
        // gpt-5.5: 600 uncached in at $5, 400 cached at $0.50 and 100 out at
        // $30 per million.
        let cost = turn_cost(&parse_stream_line_with_model(PRICED_TURN, Some("gpt-5.5")));
        let cost = cost.expect("the snapshot lists gpt-5.5");
        let expected = (600.0 * 5.0 + 400.0 * 0.50 + 100.0 * 30.0) / 1e6;
        assert!((cost - expected).abs() < 1e-12, "{cost}");
    }

    /// backlog 6106: a model the snapshot does not list, an empty slug or no
    /// model at all has an unknown cost, never the Codex CLI default's.
    #[test]
    fn an_unlisted_codex_model_has_no_cost() {
        for model in [Some("gpt-5.6-sol"), Some("codex-mini"), Some(""), None] {
            let events = parse_stream_line_with_model(PRICED_TURN, model);
            assert_eq!(turn_cost(&events), None, "{model:?}");
        }
        assert_eq!(turn_cost(&parse_stream_line(PRICED_TURN)), None);
    }

    /// backlog 6106: a row that bills reasoning apart from the output adds
    /// the reasoning tokens at the output rate.
    #[test]
    fn reasoning_billed_apart_adds_to_the_cost() {
        let mut row = codex_price_row(Some("gpt-5.5")).expect("the gpt-5.5 row");
        let inside = turn_cost(&parse_stream_line_priced(PRICED_TURN, Some(&row)));
        row.reasoning_in_output = false;
        let apart = turn_cost(&parse_stream_line_priced(PRICED_TURN, Some(&row)));
        let (inside, apart) = (inside.expect("a cost"), apart.expect("a cost"));
        // 30 reasoning tokens at gpt-5.5's $30 output rate.
        let reasoning = 30.0 * 30.0 / 1e6;
        assert!((apart - inside - reasoning).abs() < 1e-12, "{apart}");
    }

    #[test]
    fn reasoning_tokens_do_not_inflate_output_totals() {
        // `reasoning_output_tokens` is a subset of `output_tokens`; the
        // canonical event must carry the provider-reported total unchanged
        // and forward the reasoning count.
        let events = parse_stream_line(
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":40,"reasoning_output_tokens":25}}"#,
        );
        assert!(matches!(
            &events[0],
            AgentRuntimeEvent::TokenUsage {
                output_tokens: 40,
                reasoning_tokens: 25,
                ..
            }
        ));
    }

    #[test]
    fn empty_and_unknown_lines() {
        assert!(parse_stream_line("").is_empty());
        assert!(parse_stream_line(r#"{"type":"turn.started"}"#).is_empty());
        assert!(parse_stream_line("not json").is_empty());
    }
}
