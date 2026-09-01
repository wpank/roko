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
    /// Total output tokens; reasoning tokens are a subset of this total,
    /// not an addition to it (OpenAI Responses API convention).
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    /// Reasoning (thinking) tokens, already included in `output_tokens`.
    /// The canonical `AgentRuntimeEvent::TokenUsage` has no reasoning field,
    /// so this is surfaced via telemetry rather than forwarded on the wire.
    #[serde(default)]
    reasoning_output_tokens: u64,
}

// ── Cost estimation ─────────────────────────────────────────────────────

/// Per-million-token pricing for a codex-family model.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CodexPricing {
    /// USD per million uncached input tokens.
    input_per_m: f64,
    /// USD per million cached (cache-read) input tokens.
    cached_input_per_m: f64,
    /// USD per million output tokens. Codex reports reasoning tokens as a
    /// subset of `output_tokens`, so reasoning is billed at the output rate
    /// here, matching how OpenAI invoices it.
    output_per_m: f64,
}

/// Pricing for `gpt-5.6-sol`, the configured default codex CLI model.
///
/// This is also the fallback for unknown codex slugs: it preserves the rates
/// this module applied to *every* codex turn before pricing became
/// model-aware.
const GPT_5_6_SOL_PRICING: CodexPricing = CodexPricing {
    input_per_m: 2.0,
    cached_input_per_m: 0.50,
    output_per_m: 8.0,
};

/// Resolve per-token pricing for a codex model slug.
///
/// Rates mirror the gpt-5.x tiers in roko-learn's `CostTable::with_defaults`
/// (roko-agent must not depend on roko-learn, so the rows are duplicated
/// here): full-size gpt-5.x ≈ $2.50/$10.00 per M, mini tier ≈ $0.40/$1.60.
/// Unknown slugs fall back to [`GPT_5_6_SOL_PRICING`]. Configured
/// `[models.*].cost_*_per_m` values take precedence downstream whenever the
/// runner prices from the `ModelProfile`; this estimate only feeds the
/// stream-level `TurnCompleted.total_cost_usd`.
fn codex_pricing_for_model(model: Option<&str>) -> CodexPricing {
    let Some(slug) = model.map(str::trim).filter(|slug| !slug.is_empty()) else {
        return GPT_5_6_SOL_PRICING;
    };
    match slug {
        "gpt-5.6-sol" => GPT_5_6_SOL_PRICING,
        "gpt-5-codex" => CodexPricing {
            input_per_m: 2.50,
            cached_input_per_m: 0.63,
            output_per_m: 10.0,
        },
        other if other.contains("mini") => CodexPricing {
            input_per_m: 0.40,
            cached_input_per_m: 0.10,
            output_per_m: 1.60,
        },
        _ => GPT_5_6_SOL_PRICING,
    }
}

/// Estimate the USD cost of a Codex turn from its token usage.
fn estimate_codex_cost(usage: &CodexUsage, pricing: &CodexPricing) -> f64 {
    let uncached = usage.input_tokens.saturating_sub(usage.cached_input_tokens);
    uncached as f64 * pricing.input_per_m / 1_000_000.0
        + usage.cached_input_tokens as f64 * pricing.cached_input_per_m / 1_000_000.0
        + usage.output_tokens as f64 * pricing.output_per_m / 1_000_000.0
}

// ── Parser ──────────────────────────────────────────────────────────────

/// Parse one Codex `exec --json` JSONL line into canonical runtime events.
///
/// Equivalent to [`parse_stream_line_with_model`] with no model hint: turn
/// costs are estimated at the fallback `gpt-5.6-sol` rates.
#[must_use]
pub fn parse_stream_line(line: &str) -> Vec<AgentRuntimeEvent> {
    parse_stream_line_with_model(line, None)
}

/// Parse one Codex `exec --json` JSONL line into canonical runtime events,
/// resolving turn-cost estimates from the configured model slug.
///
/// `model` only affects the `TurnCompleted.total_cost_usd` estimate; pass
/// `None` (or an empty slug) to use the fallback pricing.
#[must_use]
pub fn parse_stream_line_with_model(line: &str, model: Option<&str>) -> Vec<AgentRuntimeEvent> {
    let pricing = codex_pricing_for_model(model);
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
                .map(|usage| estimate_codex_cost(usage, &pricing));
            if let Some(usage) = event.usage {
                if usage.reasoning_output_tokens > 0 {
                    // Reasoning tokens are a subset of `output_tokens` (already
                    // billed at the output rate by the estimate above). The
                    // canonical `TokenUsage` event has no reasoning field, so
                    // the count is surfaced here instead of being dropped.
                    debug!(
                        reasoning_output_tokens = usage.reasoning_output_tokens,
                        "codex turn included reasoning tokens"
                    );
                }
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
        let events = parse_stream_line(
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"cache_write_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":0}}"#,
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

    #[test]
    fn turn_cost_uses_fallback_pricing_without_model_hint() {
        // 100 input (50 cached) + 10 output at gpt-5.6-sol rates:
        // 50*$2 + 50*$0.50 + 10*$8 per M = $0.000205.
        let events = parse_stream_line(
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":10}}"#,
        );
        let cost = events.iter().find_map(|e| match e {
            AgentRuntimeEvent::TurnCompleted { total_cost_usd, .. } => *total_cost_usd,
            _ => None,
        });
        let cost = cost.expect("cost estimate");
        assert!((cost - 0.000205).abs() < 1e-12, "cost was {cost}");
    }

    #[test]
    fn turn_cost_resolves_pricing_from_model_slug() {
        let line = r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":10}}"#;
        let cost_for = |model: Option<&str>| {
            parse_stream_line_with_model(line, model)
                .into_iter()
                .find_map(|e| match e {
                    AgentRuntimeEvent::TurnCompleted { total_cost_usd, .. } => total_cost_usd,
                    _ => None,
                })
                .expect("cost estimate")
        };
        let assert_cost = |model: Option<&str>, expected: f64| {
            let cost = cost_for(model);
            assert!(
                (cost - expected).abs() < 1e-12,
                "model {model:?}: expected {expected}, got {cost}"
            );
        };

        // gpt-5-codex: 50*$2.50 + 50*$0.63 + 10*$10 per M = $0.0002565.
        assert_cost(Some("gpt-5-codex"), 0.0002565);
        // codex-mini: 50*$0.40 + 50*$0.10 + 10*$1.60 per M = $0.000041.
        assert_cost(Some("codex-mini"), 0.000041);
        // Unknown and empty slugs fall back to gpt-5.6-sol rates.
        assert_cost(Some("gpt-9-future"), 0.000205);
        assert_cost(Some(""), 0.000205);
        assert_cost(None, 0.000205);
    }

    #[test]
    fn reasoning_tokens_do_not_inflate_output_totals() {
        // `reasoning_output_tokens` is a subset of `output_tokens`; the
        // canonical event must carry the provider-reported total unchanged.
        let events = parse_stream_line(
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":40,"reasoning_output_tokens":25}}"#,
        );
        assert!(matches!(
            &events[0],
            AgentRuntimeEvent::TokenUsage {
                output_tokens: 40,
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
