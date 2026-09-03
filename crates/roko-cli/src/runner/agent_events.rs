//! Agent event handler — updates `RunState` and `TuiBridge` in response
//! to `AgentEvent`s from the stream parser.
//!
//! When streaming output is enabled, key events are rendered through the
//! runner's output sink (`RunOutputSink`) so the operator sees what the
//! agent is doing instead of a static spinner.

use tracing::{debug, info};

use roko_gate::review_verdict::{IssueCategory, ReviewVerdictContext};
use roko_gate::{ParsedReviewVerdict, parse_structured_review_verdict};

use super::output_sink::{RunOutputSink, TokenUsage};
use super::state::RunState;
use super::tui_bridge::TuiBridge;
use super::types::{AgentEvent, StderrSeverity};

/// Maximum bytes retained in `agent_output`. When exceeded, the buffer is
/// trimmed to keep the tail (most recent output), which is what replan
/// context and diagnostics need.
const MAX_AGENT_OUTPUT: usize = 32_768;
/// Tool transcripts projected into the live dashboard are bounded separately
/// from the runner's durable evidence so a single compiler dump cannot flood
/// the watch snapshot or make rendering unresponsive.
const MAX_TUI_TOOL_OUTPUT: usize = 8_192;
/// Internal framing used when a provider exposes reasoning but the legacy
/// runtime event enum only has `MessageDelta`. It is removed before normal
/// transcript/state accumulation and emitted as a semantic StateHub record.
pub(crate) const REASONING_DELTA_PREFIX: &str = "\u{001f}roko.reasoning.v1 ";

/// Process a single agent event, updating state and publishing to TUI.
///
/// Output rendering is delegated to the provided `sink`. The sink handles
/// text buffering internally (e.g., `StderrSink` accumulates deltas and
/// flushes on structural boundaries).
pub(crate) fn handle_agent_event(
    event: &AgentEvent,
    state: &mut RunState,
    tui: &TuiBridge,
    sink: &dyn RunOutputSink,
) {
    let plan_id = &state.plan_id;
    let task_id = &state.current_task;

    match event {
        AgentEvent::Started {
            agent_id: _,
            provider,
            model,
            pid,
        } => {
            state.agent_active = true;
            state.agent_model = model.clone();
            state.agent_provider = provider.clone();
            state.agent_pid = *pid;
            sink.agent_started(plan_id, task_id, provider, model, *pid);
        }

        AgentEvent::SystemInit { session_id, model } => {
            state.agent_active = true;
            // Defensive: some providers (e.g. codex) emit init events that
            // carry no model. Never let an empty model overwrite the slug
            // recorded by `AgentEvent::Started` — an empty model gets
            // episodes dropped downstream and blanks the TUI.
            if !model.is_empty() {
                state.agent_model = model.clone();
            }
            state.session_id = Some(session_id.clone());
            debug!(model = %model, session_id = %session_id, "agent initialized");
        }

        AgentEvent::MessageDelta { text } => {
            if let Some(reasoning) = text.strip_prefix(REASONING_DELTA_PREFIX) {
                let agent_id = agent_id_for_state(state);
                let attempt = state.iteration_for(plan_id, task_id);
                tui.agent_reasoning_delta(&agent_id, plan_id, task_id, attempt, reasoning);
                sink.agent_text_delta(plan_id, task_id, reasoning);
                return;
            }
            state.agent_output.push_str(text);
            if state.agent_output.len() > MAX_AGENT_OUTPUT {
                let trim_point = state.agent_output.len() - MAX_AGENT_OUTPUT / 2;
                let boundary = state.agent_output.ceil_char_boundary(trim_point);
                let omitted_lines = state.agent_output[..boundary].lines().count();
                state.agent_output = format!(
                    "[output truncated: {} lines omitted]\n{}",
                    omitted_lines,
                    &state.agent_output[boundary..],
                );
                debug!(
                    trimmed_to = state.agent_output.len(),
                    "agent_output exceeded cap, trimmed to tail"
                );
            }
            let agent_id = agent_id_for_state(state);
            let attempt = state.iteration_for(plan_id, task_id);
            tui.agent_text_delta(&agent_id, plan_id, task_id, attempt, text);

            sink.agent_text_delta(plan_id, task_id, text);
        }

        AgentEvent::ToolCall { id, name } => {
            let marker = format!("\n[tool: {name}]\n");
            state.agent_output.push_str(&marker);

            let agent_id = agent_id_for_state(state);
            let attempt = state.iteration_for(plan_id, task_id);
            tui.tool_call(&agent_id, plan_id, task_id, attempt, id, name);
            sink.tool_call(plan_id, task_id, id, name);
        }

        AgentEvent::ToolOutput { id, output } => {
            // Truncate tool output in the accumulated buffer.
            let limit = roko_core::defaults::DEFAULT_TOOL_OUTPUT_TRUNCATE_AT;
            let (truncated, state_was_truncated) = bounded_utf8(output, limit);
            state.agent_output.push_str(truncated);
            if state_was_truncated {
                let omitted_lines = output[truncated.len()..].lines().count();
                state.agent_output.push_str(&format!(
                    "\n[output truncated: {omitted_lines} lines omitted]\n"
                ));
            }
            state.agent_output.push('\n');

            let (visible, visible_was_truncated) = bounded_utf8(output, MAX_TUI_TOOL_OUTPUT);
            let mut dashboard_output = visible.to_string();
            if visible_was_truncated {
                let omitted_lines = output[visible.len()..].lines().count();
                dashboard_output
                    .push_str(&format!("\n[output truncated: {omitted_lines} lines omitted]\n"));
            } else if !dashboard_output.ends_with('\n') {
                dashboard_output.push('\n');
            }
            let agent_id = agent_id_for_state(state);
            let attempt = state.iteration_for(plan_id, task_id);
            tui.tool_output(&agent_id, plan_id, task_id, attempt, id, &dashboard_output);

            sink.tool_output(plan_id, task_id, id, output);
        }

        AgentEvent::TokenUsage {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            reasoning_tokens,
        } => {
            state.tokens_in += input_tokens;
            state.tokens_out += output_tokens;
            state.cache_read_tokens += cache_read_tokens;
            state.cache_write_tokens += cache_write_tokens;
            state.reasoning_tokens += reasoning_tokens;
            // Token counts are accumulated here; authoritative cost comes from
            // TurnCompleted.total_cost_usd which overwrites state.cost_usd.

            sink.token_usage(
                plan_id,
                task_id,
                TokenUsage {
                    input_tokens: *input_tokens,
                    output_tokens: *output_tokens,
                    cache_read_tokens: *cache_read_tokens,
                    cache_write_tokens: *cache_write_tokens,
                },
            );
            tui.token_usage(
                plan_id,
                task_id,
                *input_tokens,
                *output_tokens,
                *cache_read_tokens,
                *cache_write_tokens,
            );
        }

        AgentEvent::TurnCompleted {
            session_id,
            total_cost_usd,
            num_turns: _,
            is_error,
        } => {
            state.agent_active = false;
            state.agent_turn_completed = true;
            if let Some(sid) = session_id {
                state.session_id = Some(sid.clone());
            }
            if let Some(cost) = total_cost_usd {
                // Use the authoritative cost from the result event.
                state.cost_usd = *cost;
            }
            if *is_error {
                state.agent_output.push_str("\n[agent error]\n");
            }

            // Parse a structured review verdict from the accumulated agent
            // output. Failures are handled by the FailClosed fallback inside
            // parse_structured_review_verdict — never a hard error here.
            let verdict = parse_review_verdict(state);
            state.express_mode = is_quick_fixable(&verdict);
            state.parsed_review_verdict = Some(verdict);

            let agent_id = agent_id_for_state(state);
            let attempt = state.iteration_for(plan_id, task_id);
            tui.agent_completed(&agent_id, plan_id, task_id, attempt);
            let cost_display = format!("{:.4}", state.cost_usd);
            info!(
                task = %state.current_task,
                plan_id = %state.plan_id,
                tokens_in = state.tokens_in,
                tokens_out = state.tokens_out,
                cache_read = state.cache_read_tokens,
                cache_write = state.cache_write_tokens,
                cost_usd = %cost_display,
                model = %state.agent_model,
                is_error = *is_error,
                express_mode = state.express_mode,
                "agent turn completed"
            );

            sink.agent_turn_completed(
                plan_id,
                task_id,
                *total_cost_usd,
                *is_error,
                &state.agent_model,
                state.tokens_in,
                state.tokens_out,
            );
            tui.efficiency_event(plan_id, task_id, "cost_usd", state.cost_usd);
        }

        AgentEvent::Error { message } => {
            let severity = StderrSeverity::from_message(message);
            state
                .agent_output
                .push_str(&format!("\n[error: {message}]\n"));
            tui.error(message);

            sink.agent_error(plan_id, task_id, message, severity);
        }

        AgentEvent::Exited { exit_code } => {
            state.agent_active = false;
            state.agent_pid = None;
            debug!(exit_code = ?exit_code, task = %state.current_task, "agent process exited");
        }
    }
}

fn bounded_utf8(input: &str, max_bytes: usize) -> (&str, bool) {
    if input.len() <= max_bytes {
        return (input, false);
    }
    let mut boundary = max_bytes.min(input.len());
    while boundary > 0 && !input.is_char_boundary(boundary) {
        boundary -= 1;
    }
    (&input[..boundary], true)
}

/// Derive an agent identifier from the current state.
fn agent_id_for_state(state: &RunState) -> String {
    format!("{}/{}", state.plan_id, state.current_task)
}

/// Parse a structured review verdict from the agent's accumulated output.
///
/// Constructs a [`ReviewVerdictContext`] from the current [`RunState`] fields
/// and delegates to [`parse_structured_review_verdict`]. The parser handles
/// malformed or free-text output via the `FailClosed` fallback — callers
/// must never treat a parse failure as a fatal error.
fn parse_review_verdict(state: &RunState) -> ParsedReviewVerdict {
    let ctx = ReviewVerdictContext {
        verdict_id: format!("{}/{}", state.plan_id, state.current_task),
        batch_id: state.plan_id.clone(),
        task_id: state.current_task.clone(),
        reviewer_role_id: state.agent_model.clone(),
        raw_output_ref: format!(".roko/runs/{}/{}.raw", state.plan_id, state.current_task),
        created_at: chrono_now_rfc3339(),
    };
    parse_structured_review_verdict(&state.agent_output, ctx)
}

/// Return an RFC 3339 timestamp for the current moment.
///
/// Uses only `std` to avoid pulling in additional crate dependencies.
fn chrono_now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Minimal RFC 3339 — seconds precision, UTC only.
    // Format: YYYY-MM-DDTHH:MM:SSZ
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400; // days since 1970-01-01
    // Gregorian calendar calculation (ignores leap seconds).
    let (y, mo, d) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Convert a count of days since 1970-01-01 to (year, month, day).
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    loop {
        let leap = is_leap(y);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut mo = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        mo += 1;
    }
    (y, mo, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// Determine whether a parsed verdict's issues are all in quick-fixable categories.
///
/// Express mode is enabled when:
/// - The verdict contains at least one blocking issue, AND
/// - **All** blocking issues fall in quick-fixable categories:
///   [`IssueCategory::CompileError`], [`IssueCategory::LintViolation`],
///   [`IssueCategory::FormatViolation`], or [`IssueCategory::SymbolMissing`].
///
/// The following categories are explicitly **not** quick-fixable and will
/// prevent express mode even if they are the only issues:
/// [`IssueCategory::TestFailure`], [`IssueCategory::SecurityIssue`],
/// [`IssueCategory::PerformanceRegression`], [`IssueCategory::NeedsHumanReview`],
/// [`IssueCategory::IncompleteImpl`], [`IssueCategory::IntegrationFailure`].
pub(crate) fn is_quick_fixable(verdict: &ParsedReviewVerdict) -> bool {
    // A passed verdict needs no express dispatch.
    if verdict.passed() {
        return false;
    }
    // If there are no blocking findings, there is nothing to fix quickly.
    if verdict.evidence.blocking_findings.is_empty() {
        return false;
    }
    // If parse failed (FailClosed), we don't have typed issues — fall back
    // to full strategist flow.
    if verdict.parse_error.is_some() {
        return false;
    }
    // Without typed ReviewIssue data we cannot classify. The ParsedReviewVerdict
    // evidence carries blocking_findings as strings, not typed IssueCategory
    // values. Express mode therefore requires that the verdict was parsed from
    // the agent output as a ReviewVerdict with typed issues, which is stored
    // separately in RunState via parse_review_verdict(). Here we do a
    // best-effort heuristic on the evidence's blocking findings text.
    for finding in &verdict.evidence.blocking_findings {
        if !quick_fixable_by_text(finding) {
            return false;
        }
    }
    true
}

/// Heuristic to classify a blocking-finding string as quick-fixable.
///
/// Checks for keyword patterns that correspond to quick-fixable
/// [`IssueCategory`] variants. This is a text-based fallback because
/// [`ParsedReviewVerdict`] carries string findings, not typed
/// [`IssueCategory`] values.
fn quick_fixable_by_text(finding: &str) -> bool {
    let f = finding.to_ascii_lowercase();
    // Quick-fixable keywords
    let quick = [
        "compile error",
        "compilation error",
        "compile_error",
        "error[e", // rustc error codes like E0433
        "lint",
        "clippy",
        "format",
        "fmt",
        "symbol missing",
        "symbol_missing",
        "unresolved import",
        "cannot find",
    ];
    // Non-quick-fixable keywords — checked first to prevent false positives.
    let not_quick = [
        "test failure",
        "test_failure",
        "tests fail",
        "security",
        "performance regression",
        "performance_regression",
        "needs human",
        "needs_human",
        "human review",
        "integration",
        "incomplete",
    ];
    if not_quick.iter().any(|k| f.contains(k)) {
        return false;
    }
    quick.iter().any(|k| f.contains(k))
}

/// Classify a [`ReviewIssue`]'s category as quick-fixable.
///
/// This typed variant is used in tests and by code that has access to a
/// full [`ReviewVerdict`] (not just the string evidence in
/// [`ParsedReviewVerdict`]).
#[allow(dead_code)]
pub(crate) fn issue_category_is_quick_fixable(cat: &IssueCategory) -> bool {
    matches!(
        cat,
        IssueCategory::CompileError
            | IssueCategory::LintViolation
            | IssueCategory::FormatViolation
            | IssueCategory::SymbolMissing
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::output_sink::NoopSink;
    use crate::runner::state::RunState;
    use crate::runner::tui_bridge::TuiBridge;
    use crate::runner::types::AgentEvent;
    use crate::state_hub::StateHub;
    use roko_gate::review_verdict::IssueCategory;

    fn noop_bridge() -> (StateHub, TuiBridge) {
        let hub = StateHub::default_capacity();
        let tui = TuiBridge::new(hub.sender());
        (hub, tui)
    }

    fn make_state(plan_id: &str, task_id: &str) -> RunState {
        let mut state = RunState::new(1);
        state.plan_id = plan_id.to_string();
        state.current_task = task_id.to_string();
        state
    }

    // T1 / SH04-T01: structured attribution -- plan_id and task_id flow from
    // `RunState` fields, not by parsing slash-separated display IDs.
    #[test]
    fn message_delta_publishes_output_attributed_to_plan_and_task() {
        let (hub, tui) = noop_bridge();
        let mut state = make_state("plan-alpha", "T42");
        let sink = NoopSink;
        tui.agent_spawned(
            "plan-alpha/T42",
            "plan-alpha",
            "T42",
            1,
            "implementer",
            "test-model",
            "test-provider",
        );

        handle_agent_event(
            &AgentEvent::MessageDelta {
                text: "hello world".to_string(),
            },
            &mut state,
            &tui,
            &sink,
        );

        let snap = hub.snapshot().borrow().clone();
        // The agent output is stored in the snapshot under the structured
        // current task supplied by AgentSpawned.
        let agent_key = "plan-alpha/T42";
        assert!(snap.agents.contains_key(agent_key));
        // Stream records carry a semantic prefix; verify the payload
        // round-trips through the StateHub snapshot.
        let last_line = snap
            .task_outputs
            .get("T42")
            .and_then(|lines| lines.back())
            .expect("task_outputs should contain T42");
        assert!(
            last_line.starts_with(crate::runner::tui_bridge::STREAM_RECORD_PREFIX),
            "task output must be a stream record"
        );
        let json_str = last_line
            .strip_prefix(crate::runner::tui_bridge::STREAM_RECORD_PREFIX)
            .unwrap();
        let record: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(record["kind"], "text");
        assert_eq!(record["payload"]["text"], "hello world");
        assert!(
            state.agent_output.contains("hello world"),
            "agent_output in RunState must accumulate the delta"
        );
    }

    #[test]
    fn tool_activity_is_projected_to_connected_dashboard_with_a_bound() {
        let (hub, tui) = noop_bridge();
        let mut state = make_state("plan-alpha", "T42");
        let sink = NoopSink;
        tui.agent_spawned(
            "plan-alpha/T42",
            "plan-alpha",
            "T42",
            1,
            "implementer",
            "test-model",
            "test-provider",
        );

        handle_agent_event(
            &AgentEvent::ToolCall {
                id: "tool-1".to_string(),
                name: "cargo check".to_string(),
            },
            &mut state,
            &tui,
            &sink,
        );
        handle_agent_event(
            &AgentEvent::ToolOutput {
                id: "tool-1".to_string(),
                output: "x".repeat(MAX_TUI_TOOL_OUTPUT + 100),
            },
            &mut state,
            &tui,
            &sink,
        );

        let snap = hub.snapshot().borrow().clone();
        let lines = snap.task_outputs.get("T42").expect("task output ring");
        // Stream records carry the semantic prefix; parse and check payloads.
        let has_tool_start = lines.iter().any(|line| {
            line.strip_prefix(crate::runner::tui_bridge::STREAM_RECORD_PREFIX)
                .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
                .map_or(false, |v| {
                    v["kind"] == "tool_start" && v["payload"]["tool"] == "cargo check"
                })
        });
        assert!(
            has_tool_start,
            "tool_start record with name 'cargo check' must be projected"
        );
        let has_truncated = lines.iter().any(|line| {
            line.strip_prefix(crate::runner::tui_bridge::STREAM_RECORD_PREFIX)
                .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
                .map_or(false, |v| {
                    v["payload"]["output"]
                        .as_str()
                        .map_or(false, |s| s.contains("output truncated:"))
                })
        });
        assert!(
            has_truncated,
            "tool_result record must contain truncation marker"
        );
        // Stream record envelope adds JSON framing (~150B per record × 2 records).
        assert!(
            snap.agents["plan-alpha/T42"].output_bytes <= MAX_TUI_TOOL_OUTPUT + 512,
            "output_bytes {} must stay bounded (max {})",
            snap.agents["plan-alpha/T42"].output_bytes,
            MAX_TUI_TOOL_OUTPUT + 512
        );
    }

    // T1 / SH04-T01: structured attribution via agent_id_for_state never depends
    // on slash-parsing; it derives directly from plan_id and current_task.
    #[test]
    fn agent_id_derives_from_structured_fields_not_display_id_parsing() {
        let mut state = make_state("my-plan", "task:colon-in-name");
        // Even when the task_id contains a colon the agent_id is a predictable
        // slash-joined string -- no further parsing is applied.
        let id = agent_id_for_state(&state);
        assert_eq!(id, "my-plan/task:colon-in-name");

        state.plan_id = "plan/with/slashes".to_string();
        state.current_task = "T1".to_string();
        let id2 = agent_id_for_state(&state);
        assert_eq!(id2, "plan/with/slashes/T1");
    }

    // Codex regression: a `SystemInit` with an empty model must not wipe the
    // model slug recorded by `Started` (empty models get episodes dropped and
    // blank the TUI). A non-empty init model still wins.
    #[test]
    fn system_init_with_empty_model_preserves_started_model() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T1");
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::Started {
                agent_id: "plan/T1".to_string(),
                provider: "codex-cli".to_string(),
                model: "gpt-5.6-sol".to_string(),
                pid: Some(42),
            },
            &mut state,
            &tui,
            &sink,
        );
        assert_eq!(state.agent_model, "gpt-5.6-sol");

        handle_agent_event(
            &AgentEvent::SystemInit {
                session_id: "thread-1".to_string(),
                model: String::new(),
            },
            &mut state,
            &tui,
            &sink,
        );
        assert_eq!(
            state.agent_model, "gpt-5.6-sol",
            "empty SystemInit model must not overwrite the Started model"
        );
        assert_eq!(state.session_id.as_deref(), Some("thread-1"));

        handle_agent_event(
            &AgentEvent::SystemInit {
                session_id: "thread-1".to_string(),
                model: "gpt-5.6-sol-mini".to_string(),
            },
            &mut state,
            &tui,
            &sink,
        );
        assert_eq!(
            state.agent_model, "gpt-5.6-sol-mini",
            "a non-empty SystemInit model still updates state"
        );
    }

    // T2 / SH04-T05: token usage accumulates in state without double-counting.
    #[test]
    fn token_usage_accumulates_in_state_without_double_count() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T1");
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_tokens: 10,
                cache_write_tokens: 5,
                reasoning_tokens: 0,
            },
            &mut state,
            &tui,
            &sink,
        );
        assert_eq!(state.tokens_in, 100, "tokens_in must equal the event value");
        assert_eq!(
            state.tokens_out, 50,
            "tokens_out must equal the event value"
        );
        assert_eq!(state.cache_read_tokens, 10);
        assert_eq!(state.cache_write_tokens, 5);

        // A second usage event accumulates on top -- no reset between events.
        handle_agent_event(
            &AgentEvent::TokenUsage {
                input_tokens: 20,
                output_tokens: 10,
                cache_read_tokens: 2,
                cache_write_tokens: 1,
                reasoning_tokens: 0,
            },
            &mut state,
            &tui,
            &sink,
        );
        assert_eq!(state.tokens_in, 120, "tokens_in must accumulate, not reset");
        assert_eq!(state.tokens_out, 60);
    }

    // T2 / SH04-T05: authoritative cost comes from TurnCompleted, not per-event
    // token accumulation, to avoid double-counting.
    #[test]
    fn turn_completed_overwrites_cost_with_authoritative_value() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T1");
        let sink = NoopSink;

        // Simulate some prior token accumulation.
        state.cost_usd = 0.99;

        handle_agent_event(
            &AgentEvent::TurnCompleted {
                session_id: Some("sess-1".to_string()),
                total_cost_usd: Some(0.0042),
                num_turns: Some(3),
                is_error: false,
            },
            &mut state,
            &tui,
            &sink,
        );

        assert!(
            (state.cost_usd - 0.0042).abs() < 1e-9,
            "cost_usd must be overwritten by the authoritative TurnCompleted value"
        );
        assert!(
            !state.agent_active,
            "agent must be inactive after TurnCompleted"
        );
        assert!(
            state.agent_turn_completed,
            "agent_turn_completed flag must be set"
        );
    }

    // T3 / SH04-T02: error events include severity classification; the error
    // message is stored in agent_output and published through the sink without
    // also becoming a fatal process exit.
    #[test]
    fn error_event_classifies_severity_and_does_not_exit_agent() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T1");
        state.agent_active = true;
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::Error {
                message: "warning: unused variable".to_string(),
            },
            &mut state,
            &tui,
            &sink,
        );

        assert!(
            state.agent_output.contains("warning: unused variable"),
            "error message must be buffered in agent_output"
        );
        // agent_active is NOT cleared by Error -- only by Exited or TurnCompleted.
        assert!(
            state.agent_active,
            "an Error event must not terminate the agent process"
        );
    }

    // T3 / SH04-T02: Exited event clears active flag but does not duplicate
    // the error path -- there is no additional error record written.
    #[test]
    fn exited_event_clears_active_flag_without_writing_error() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T1");
        state.agent_active = true;
        state.agent_pid = Some(12345);
        let sink = NoopSink;
        let prior_output = state.agent_output.clone();

        handle_agent_event(
            &AgentEvent::Exited { exit_code: Some(0) },
            &mut state,
            &tui,
            &sink,
        );

        assert!(!state.agent_active, "agent must be inactive after Exited");
        assert!(
            state.agent_pid.is_none(),
            "agent_pid must be cleared after Exited"
        );
        assert_eq!(
            state.agent_output, prior_output,
            "Exited must not append to agent_output"
        );
    }

    // ─── E45-T01: Review verdict parsing + express mode ───────────────

    // After TurnCompleted, the parsed verdict must be stored in state.
    // Free-text output triggers the FailClosed path which is NOT a hard
    // error — the verdict is stored but express_mode must be false.
    #[test]
    fn turn_completed_parses_review_verdict_and_stores_it() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T1");
        // Agent output is free text — will be parsed as FailClosed.
        state.agent_output = "LGTM, all looks good to me".to_string();
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: Some(0.001),
                num_turns: Some(1),
                is_error: false,
            },
            &mut state,
            &tui,
            &sink,
        );

        assert!(
            state.parsed_review_verdict.is_some(),
            "parsed_review_verdict must be Some after TurnCompleted"
        );
        // FailClosed is a legitimate non-error result.
        let verdict = state.parsed_review_verdict.as_ref().unwrap();
        assert!(
            verdict.parse_error.is_some(),
            "free-text output should produce a FailClosed verdict with parse_error set"
        );
        // Free-text output cannot activate express mode.
        assert!(
            !state.express_mode,
            "express_mode must be false when verdict is FailClosed"
        );
    }

    // Structured JSON output that passes the review must produce a passed
    // verdict and NOT activate express mode (there is nothing to fix).
    #[test]
    fn passed_verdict_does_not_activate_express_mode() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T2");
        state.agent_output = r#"{
            "status": "passed",
            "confidence": 0.95,
            "blocking_findings": [],
            "non_blocking_findings": [],
            "required_next_action": "none",
            "evidence_refs": []
        }"#
        .to_string();
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: Some(0.002),
                num_turns: Some(2),
                is_error: false,
            },
            &mut state,
            &tui,
            &sink,
        );

        let verdict = state
            .parsed_review_verdict
            .as_ref()
            .expect("verdict must be stored");
        assert!(verdict.passed(), "parsed verdict must be passed");
        assert!(
            !state.express_mode,
            "express_mode must be false when verdict passes (nothing to fix)"
        );
    }

    // A verdict with only quick-fixable blocking findings (compile error)
    // must activate express mode.
    #[test]
    fn compile_error_finding_activates_express_mode() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T3");
        state.agent_output = r#"{
            "status": "failed",
            "confidence": 0.6,
            "blocking_findings": ["compile error: error[E0433] unresolved import"],
            "non_blocking_findings": [],
            "required_next_action": "retry",
            "evidence_refs": []
        }"#
        .to_string();
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: None,
                num_turns: Some(1),
                is_error: false,
            },
            &mut state,
            &tui,
            &sink,
        );

        assert!(
            state.express_mode,
            "express_mode must be true for a compile-error-only verdict"
        );
    }

    // A verdict with a TestFailure blocking finding must NOT activate express
    // mode — test failures are never quick-fixable.
    #[test]
    fn test_failure_finding_does_not_activate_express_mode() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T4");
        state.agent_output = r#"{
            "status": "failed",
            "confidence": 0.5,
            "blocking_findings": ["test failure: 3 tests failed"],
            "non_blocking_findings": [],
            "required_next_action": "retry",
            "evidence_refs": []
        }"#
        .to_string();
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: None,
                num_turns: Some(1),
                is_error: false,
            },
            &mut state,
            &tui,
            &sink,
        );

        assert!(
            !state.express_mode,
            "express_mode must be false for test-failure blocking findings"
        );
    }

    // A verdict with a SecurityIssue finding must NOT activate express mode.
    #[test]
    fn security_issue_finding_does_not_activate_express_mode() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T5");
        state.agent_output = r#"{
            "status": "failed",
            "confidence": 0.4,
            "blocking_findings": ["security vulnerability detected in dependency"],
            "non_blocking_findings": [],
            "required_next_action": "retry",
            "evidence_refs": []
        }"#
        .to_string();
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: None,
                num_turns: Some(1),
                is_error: false,
            },
            &mut state,
            &tui,
            &sink,
        );

        assert!(
            !state.express_mode,
            "express_mode must be false for security-issue blocking findings"
        );
    }

    // Typed IssueCategory classification: quick-fixable categories map correctly.
    #[test]
    fn issue_category_quick_fixable_matches_spec() {
        assert!(issue_category_is_quick_fixable(
            &IssueCategory::CompileError
        ));
        assert!(issue_category_is_quick_fixable(
            &IssueCategory::LintViolation
        ));
        assert!(issue_category_is_quick_fixable(
            &IssueCategory::FormatViolation
        ));
        assert!(issue_category_is_quick_fixable(
            &IssueCategory::SymbolMissing
        ));

        assert!(!issue_category_is_quick_fixable(
            &IssueCategory::TestFailure
        ));
        assert!(!issue_category_is_quick_fixable(
            &IssueCategory::SecurityIssue
        ));
        assert!(!issue_category_is_quick_fixable(
            &IssueCategory::PerformanceRegression
        ));
        assert!(!issue_category_is_quick_fixable(
            &IssueCategory::NeedsHumanReview
        ));
        assert!(!issue_category_is_quick_fixable(
            &IssueCategory::IncompleteImpl
        ));
        assert!(!issue_category_is_quick_fixable(
            &IssueCategory::IntegrationFailure
        ));
    }

    // Format and lint findings are quick-fixable.
    #[test]
    fn lint_and_format_findings_activate_express_mode() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T6");
        state.agent_output = r#"{
            "status": "failed",
            "confidence": 0.7,
            "blocking_findings": ["lint: clippy warning - unused variable"],
            "non_blocking_findings": [],
            "required_next_action": "retry",
            "evidence_refs": []
        }"#
        .to_string();
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: None,
                num_turns: Some(1),
                is_error: false,
            },
            &mut state,
            &tui,
            &sink,
        );

        assert!(
            state.express_mode,
            "express_mode must be true for lint-only blocking findings"
        );
    }

    // A mix of quick-fixable and non-quick-fixable findings must NOT
    // activate express mode — ALL blocking findings must be quick-fixable.
    #[test]
    fn mixed_findings_do_not_activate_express_mode() {
        let (_hub, tui) = noop_bridge();
        let mut state = make_state("plan", "T7");
        state.agent_output = r#"{
            "status": "failed",
            "confidence": 0.5,
            "blocking_findings": [
                "compile error: error[E0433] unresolved import",
                "test failure: integration test panicked"
            ],
            "non_blocking_findings": [],
            "required_next_action": "retry",
            "evidence_refs": []
        }"#
        .to_string();
        let sink = NoopSink;

        handle_agent_event(
            &AgentEvent::TurnCompleted {
                session_id: None,
                total_cost_usd: None,
                num_turns: Some(1),
                is_error: false,
            },
            &mut state,
            &tui,
            &sink,
        );

        assert!(
            !state.express_mode,
            "express_mode must be false when any blocking finding is not quick-fixable"
        );
    }
}
