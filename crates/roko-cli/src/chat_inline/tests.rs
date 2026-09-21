//! Tests for the inline chat module.

use super::*;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::chat_session::ChatAgentSession;
use crate::inline::primitives::{CostMeter, StreamingState};
use roko_learn::cost_table::CostTable;

#[test]
fn input_state_basic() {
    let mut input = InputState::new();
    input.insert('h');
    input.insert('i');
    assert_eq!(input.buffer, "hi");
    assert_eq!(input.cursor, 2);
}

#[test]
fn input_state_backspace() {
    let mut input = InputState::new();
    input.insert('a');
    input.insert('b');
    input.backspace();
    assert_eq!(input.buffer, "a");
    assert_eq!(input.cursor, 1);
}

#[test]
fn input_state_history() {
    let mut input = InputState::new();
    input.buffer = "first".into();
    input.submit();
    input.buffer = "second".into();
    input.submit();
    assert!(input.is_empty());

    input.history_up();
    assert_eq!(input.buffer, "second");
    input.history_up();
    assert_eq!(input.buffer, "first");
    input.history_down();
    assert_eq!(input.buffer, "second");
    input.history_down();
    assert!(input.is_empty());
}

#[test]
fn input_state_submit() {
    let mut input = InputState::new();
    input.insert('t');
    input.insert('e');
    input.insert('s');
    input.insert('t');
    let text = input.submit();
    assert_eq!(text, "test");
    assert!(input.is_empty());
    assert_eq!(input.history.len(), 1);
}

#[test]
fn turn_result_to_dispatch_result_keeps_model_and_tool_preview() {
    let turn = crate::chat_session::TurnResult {
        text: "hello".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        input_tokens: 12,
        output_tokens: 34,
        tool_calls: vec![crate::chat_session::ToolCallSummary {
            name: "Read".to_string(),
            input_abbrev: "file contents here".to_string(),
            success: true,
        }],
        session_id: Some("sess-123".to_string()),
        duration: Duration::from_millis(42),
        cancelled: false,
    };

    let result = turn_result_to_dispatch_result(turn);
    assert_eq!(result.model, "claude-sonnet-4-6");
    assert_eq!(result.session_id.as_deref(), Some("sess-123"));
    assert_eq!(result.tool_outputs.len(), 1);
    assert_eq!(result.tool_outputs[0].tool_name.as_deref(), Some("Read"));
    assert_eq!(result.tool_outputs[0].content, "file contents here");
}

#[test]
fn chat_session_init_error_blocks_dispatch_direct_fallback() {
    let err = ChatInlineDispatchError::ChatSessionInit {
        source: anyhow::anyhow!("synthetic init failure"),
    };
    let message = err.to_string();

    assert!(message.contains("ChatAgentSession initialization failed"));
    assert!(message.contains("refusing to fall back"));
    assert!(message.contains("dispatch_direct"));
    assert!(message.contains("synthetic init failure"));
}

#[test]
fn direct_dispatch_mode_returns_visible_failure() {
    use crate::auth_detect::AuthMethod;

    let mut session = ChatSession {
        phase: Phase::Input,
        input: InputState::new(),
        streaming: StreamingState::new(""),
        cost: CostMeter::new(),
        cost_table: CostTable {
            models: HashMap::new(),
        }
        .with_defaults(),
        agent_id: "roko".to_string(),
        tick: 0,
        started_at: Instant::now(),
        dispatch: DispatchMode::Direct {
            auth: AuthMethod::ClaudeCli,
        },
        response_rx: None,
        streaming_event_rx: None,
        turn_count: 0,
        thinking_started: None,
        conversation: Vec::new(),
        last_prompt: None,
        system_message: None,
        compact: false,
        quiet: false,
        agent_session: None,
        last_ctrl_c: None,
    };

    dispatch_prompt(&mut session, "hello");
    let err = session
        .response_rx
        .as_mut()
        .expect("response channel")
        .try_recv()
        .expect("direct failure result")
        .expect_err("direct dispatch should fail");

    assert!(err.contains("direct inline dispatch is disabled"));
    assert!(err.contains("ChatAgentSession"));
}

#[test]
fn input_state_navigation() {
    let mut input = InputState::new();
    for ch in "hello".chars() {
        input.insert(ch);
    }
    assert_eq!(input.cursor, 5);
    input.home();
    assert_eq!(input.cursor, 0);
    input.end();
    assert_eq!(input.cursor, 5);
    input.move_left();
    assert_eq!(input.cursor, 4);
    input.move_right();
    assert_eq!(input.cursor, 5);
}

// -----------------------------------------------------------------------
// fuzzy_match tests
// -----------------------------------------------------------------------

#[test]
fn fuzzy_match_exact() {
    let (score, indices) = fuzzy_match("model", "model").unwrap();
    assert!(score > 0);
    assert_eq!(indices, vec![0, 1, 2, 3, 4]);
}

#[test]
fn fuzzy_match_prefix() {
    let (score, indices) = fuzzy_match("mo", "model").unwrap();
    assert!(score > 0);
    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn fuzzy_match_subsequence() {
    let result = fuzzy_match("mdl", "model");
    assert!(result.is_some());
    let (_, indices) = result.unwrap();
    assert_eq!(indices.len(), 3);
    assert_eq!(indices[0], 0); // m
}

#[test]
fn fuzzy_match_no_match() {
    assert!(fuzzy_match("xyz", "model").is_none());
}

#[test]
fn fuzzy_match_empty_query() {
    let (score, indices) = fuzzy_match("", "model").unwrap();
    assert_eq!(score, 0);
    assert!(indices.is_empty());
}

#[test]
fn fuzzy_match_case_insensitive() {
    let result = fuzzy_match("MO", "model");
    assert!(result.is_some());
    let (_, indices) = result.unwrap();
    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn fuzzy_match_word_boundary_bonus() {
    let (boundary_score, _) = fuzzy_match("h", "/help").unwrap();
    let (mid_score, _) = fuzzy_match("e", "/help").unwrap();
    assert!(boundary_score > mid_score);
}

// -----------------------------------------------------------------------
// CompletionState tests
// -----------------------------------------------------------------------

#[test]
fn completion_state_slash_trigger() {
    let mut cs = CompletionState::new();
    cs.update("/");
    assert!(cs.visible);
    assert_eq!(cs.matches.len(), SLASH_COMMANDS.len());
}

#[test]
fn completion_state_filtering() {
    let mut cs = CompletionState::new();
    cs.update("/he");
    assert!(cs.visible);
    assert!(cs.matches.iter().any(|m| m.command == "/help"));
}

#[test]
fn completion_state_no_match() {
    let mut cs = CompletionState::new();
    cs.update("/zzzzz");
    assert!(!cs.visible);
    assert!(cs.matches.is_empty());
}

#[test]
fn completion_state_navigation_wrap() {
    let mut cs = CompletionState::new();
    cs.update("/");
    let count = cs.matches.len();
    assert!(count > 1);

    for _ in 0..count {
        cs.select_next();
    }
    assert_eq!(cs.selected, 0);

    cs.select_prev();
    assert_eq!(cs.selected, count - 1);
}

#[test]
fn completion_state_accept() {
    let mut cs = CompletionState::new();
    cs.update("/he");
    assert!(cs.visible);
    let accepted = cs.accept();
    assert!(accepted.is_some());
    assert!(accepted.unwrap().starts_with('/'));
    assert!(!cs.visible);
}

#[test]
fn completion_state_dismiss() {
    let mut cs = CompletionState::new();
    cs.update("/");
    assert!(cs.visible);
    cs.dismiss();
    assert!(!cs.visible);
    assert!(cs.matches.is_empty());
}

#[test]
fn completion_state_non_slash_dismissed() {
    let mut cs = CompletionState::new();
    cs.update("hello");
    assert!(!cs.visible);
}

// -----------------------------------------------------------------------
// ghost_suggestion tests
// -----------------------------------------------------------------------

#[test]
fn ghost_suggestion_from_history() {
    let mut input = InputState::new();
    input.buffer = "hello world".into();
    input.submit();
    input.buffer = "hel".into();
    input.cursor = 3;
    let ghost = input.ghost_suggestion();
    assert_eq!(ghost, Some("lo world"));
}

#[test]
fn ghost_suggestion_none_when_empty() {
    let input = InputState::new();
    assert!(input.ghost_suggestion().is_none());
}

#[test]
fn ghost_suggestion_none_for_slash() {
    let mut input = InputState::new();
    input.buffer = "/mo".into();
    input.cursor = 3;
    assert!(input.ghost_suggestion().is_none());
}

#[test]
fn ghost_suggestion_prefers_recent() {
    let mut input = InputState::new();
    input.buffer = "test alpha".into();
    input.submit();
    input.buffer = "test beta".into();
    input.submit();
    input.buffer = "test".into();
    input.cursor = 4;
    assert_eq!(input.ghost_suggestion(), Some(" beta"));
}

#[test]
fn ghost_suggestion_none_cursor_not_at_end() {
    let mut input = InputState::new();
    input.buffer = "hello world".into();
    input.submit();
    input.buffer = "hello".into();
    input.cursor = 3;
    assert!(input.ghost_suggestion().is_none());
}

#[test]
fn ghost_suggestion_accept() {
    let mut input = InputState::new();
    input.buffer = "hello world".into();
    input.submit();
    input.buffer = "hel".into();
    input.cursor = 3;
    assert!(input.accept_ghost());
    assert_eq!(input.buffer, "hello world");
    assert_eq!(input.cursor, 11);
}

// -----------------------------------------------------------------------
// thinking_label tests
// -----------------------------------------------------------------------

#[test]
fn thinking_label_phases() {
    assert_eq!(thinking_label(0.5), "Connecting...");
    assert_eq!(thinking_label(1.9), "Connecting...");
    assert_eq!(thinking_label(2.0), "Thinking...");
    assert_eq!(thinking_label(5.0), "Thinking...");
    assert_eq!(thinking_label(8.0), "Still thinking...");
    assert_eq!(thinking_label(12.0), "Still thinking...");
    assert_eq!(thinking_label(15.0), "Deep in thought...");
    assert_eq!(thinking_label(30.0), "Deep in thought...");
}

// -----------------------------------------------------------------------
// history persistence tests
// -----------------------------------------------------------------------

#[test]
fn history_loaded_into_input() {
    let mut input = InputState::new();
    input.history = vec!["first".to_string(), "second".to_string()];
    input.history_up();
    assert_eq!(input.buffer, "second");
    input.history_up();
    assert_eq!(input.buffer, "first");
}

#[test]
fn ghost_works_with_loaded_history() {
    let mut input = InputState::new();
    input.history = vec![
        "cargo test --workspace".to_string(),
        "cargo build --release".to_string(),
    ];
    input.buffer = "cargo".into();
    input.cursor = 5;
    assert_eq!(input.ghost_suggestion(), Some(" build --release"));
}

// -----------------------------------------------------------------------
// multi-line input tests
// -----------------------------------------------------------------------

#[test]
fn insert_newline_basic() {
    let mut input = InputState::new();
    input.insert('a');
    input.insert_newline();
    input.insert('b');
    assert_eq!(input.buffer, "a\nb");
    assert_eq!(input.cursor, 3);
    assert_eq!(input.line_count(), 2);
}

#[test]
fn line_count_single() {
    let mut input = InputState::new();
    input.buffer = "hello world".into();
    assert_eq!(input.line_count(), 1);
}

#[test]
fn line_count_multiple() {
    let mut input = InputState::new();
    input.buffer = "line1\nline2\nline3".into();
    assert_eq!(input.line_count(), 3);
}

#[test]
fn cursor_line_col_first_line() {
    let mut input = InputState::new();
    input.buffer = "hello\nworld".into();
    input.cursor = 3;
    assert_eq!(input.cursor_line_col(), (0, 3));
}

#[test]
fn cursor_line_col_second_line() {
    let mut input = InputState::new();
    input.buffer = "hello\nworld".into();
    input.cursor = 8;
    assert_eq!(input.cursor_line_col(), (1, 2));
}

#[test]
fn cursor_line_col_at_newline() {
    let mut input = InputState::new();
    input.buffer = "hello\nworld".into();
    input.cursor = 6;
    assert_eq!(input.cursor_line_col(), (1, 0));
}

// -----------------------------------------------------------------------
// completion with /export
// -----------------------------------------------------------------------

#[test]
fn completion_includes_export() {
    let mut cs = CompletionState::new();
    cs.update("/exp");
    assert!(cs.visible);
    assert!(cs.matches.iter().any(|m| m.command == "/export"));
}

// -----------------------------------------------------------------------
// reading_time tests
// -----------------------------------------------------------------------

#[test]
fn reading_time_short() {
    assert!(reading_time("hello world").is_none());
}

#[test]
fn reading_time_medium() {
    let text = "word ".repeat(150);
    let rt = reading_time(&text);
    assert!(rt.is_some());
    assert!(rt.unwrap().contains("words"));
}

#[test]
fn reading_time_long() {
    let text = "word ".repeat(500);
    let rt = reading_time(&text);
    assert!(rt.is_some());
    assert!(rt.unwrap().contains("min read"));
}

// -----------------------------------------------------------------------
// error_suggestions tests
// -----------------------------------------------------------------------

#[test]
fn error_suggestions_connection() {
    let s = error_suggestions("connection refused");
    assert!(!s.is_empty());
    assert!(s.iter().any(|(_, cmd)| cmd.contains("serve")));
}

#[test]
fn error_suggestions_auth() {
    let s = error_suggestions("401 unauthorized");
    assert!(!s.is_empty());
    assert!(s.iter().any(|(label, _)| label.contains("key")));
}

#[test]
fn error_suggestions_rate_limit() {
    let s = error_suggestions("429 too many requests");
    assert!(!s.is_empty());
}

#[test]
fn error_suggestions_unknown() {
    let s = error_suggestions("something weird happened");
    assert!(s.is_empty());
}

// -----------------------------------------------------------------------
// history search (Ctrl+R) tests
// -----------------------------------------------------------------------

#[test]
fn history_search_basic() {
    let history = vec![
        "cargo build".to_string(),
        "cargo test --workspace".to_string(),
        "fix the login bug".to_string(),
    ];
    let mut search = HistorySearch::new();
    search.active = true;
    search.query = "cargo".to_string();
    search.update(&history);
    assert_eq!(search.matches.len(), 2);
    assert_eq!(
        search.current_match(&history),
        Some("cargo test --workspace")
    );
}

#[test]
fn history_search_cycle() {
    let history = vec![
        "alpha one".to_string(),
        "alpha two".to_string(),
        "alpha three".to_string(),
    ];
    let mut search = HistorySearch::new();
    search.query = "alpha".to_string();
    search.update(&history);
    assert_eq!(search.matches.len(), 3);
    assert_eq!(search.current_match(&history), Some("alpha three"));
    search.next_match();
    assert_eq!(search.current_match(&history), Some("alpha two"));
    search.next_match();
    assert_eq!(search.current_match(&history), Some("alpha one"));
    search.next_match();
    assert_eq!(search.current_match(&history), Some("alpha three"));
}

#[test]
fn history_search_no_match() {
    let history = vec!["hello".to_string()];
    let mut search = HistorySearch::new();
    search.query = "xyz".to_string();
    search.update(&history);
    assert!(search.matches.is_empty());
    assert!(search.current_match(&history).is_none());
}

#[test]
fn history_search_accept() {
    let history = vec!["cargo build".to_string(), "cargo test".to_string()];
    let mut search = HistorySearch::new();
    search.active = true;
    search.query = "test".to_string();
    search.update(&history);
    let idx = search.accept();
    assert_eq!(idx, Some(1));
    assert!(!search.active);
}

#[test]
fn history_search_case_insensitive() {
    let history = vec!["Fix THE bug".to_string()];
    let mut search = HistorySearch::new();
    search.query = "fix the".to_string();
    search.update(&history);
    assert_eq!(search.matches.len(), 1);
}

// -----------------------------------------------------------------------
// Command palette tests
// -----------------------------------------------------------------------

#[test]
fn palette_open_shows_all() {
    let mut p = CommandPalette::new();
    p.open();
    assert!(p.active);
    assert_eq!(p.matches.len(), SLASH_COMMANDS.len());
}

#[test]
fn palette_filter() {
    let mut p = CommandPalette::new();
    p.open();
    p.type_char('h');
    p.type_char('e');
    p.type_char('l');
    assert!(p.matches.iter().any(|m| m.command == "/help"));
    assert!(p.matches.len() < SLASH_COMMANDS.len());
}

#[test]
fn palette_navigation() {
    let mut p = CommandPalette::new();
    p.open();
    assert_eq!(p.selected, 0);
    p.select_next();
    assert_eq!(p.selected, 1);
    p.select_prev();
    assert_eq!(p.selected, 0);
    p.select_prev();
    assert_eq!(p.selected, p.matches.len() - 1);
}

#[test]
fn palette_accept() {
    let mut p = CommandPalette::new();
    p.open();
    let cmd = p.accept();
    assert!(cmd.is_some());
    assert!(cmd.unwrap().starts_with('/'));
    assert!(!p.active);
}

#[test]
fn palette_dismiss() {
    let mut p = CommandPalette::new();
    p.open();
    p.type_char('x');
    p.dismiss();
    assert!(!p.active);
    assert!(p.query.is_empty());
}

#[test]
fn palette_backspace() {
    let mut p = CommandPalette::new();
    p.open();
    p.type_char('h');
    p.type_char('e');
    let after_he = p.matches.len();
    p.backspace();
    assert!(p.matches.len() >= after_he);
}

#[test]
fn palette_searches_description() {
    let mut p = CommandPalette::new();
    p.open();
    p.type_char('v');
    p.type_char('e');
    p.type_char('r');
    assert!(p.matches.iter().any(|m| m.command == "/version"));
}

// -----------------------------------------------------------------------
// truncate_str tests
// -----------------------------------------------------------------------

#[test]
fn truncate_short() {
    assert_eq!(truncate_str("hello", 10), "hello");
}

#[test]
fn truncate_long() {
    assert_eq!(truncate_str("hello world", 8), "hello...");
}

// -----------------------------------------------------------------------
// /model atomic switch tests
// -----------------------------------------------------------------------

fn make_session(dispatch: DispatchMode, agent_session: Option<ChatAgentSession>) -> ChatSession {
    ChatSession {
        phase: Phase::Input,
        input: InputState::new(),
        streaming: StreamingState::new(""),
        cost: CostMeter::new(),
        cost_table: CostTable::from_config(&indexmap::IndexMap::new()).with_defaults(),
        agent_id: "test".to_string(),
        tick: 0,
        started_at: Instant::now(),
        dispatch,
        response_rx: None,
        streaming_event_rx: None,
        turn_count: 0,
        thinking_started: None,
        conversation: Vec::new(),
        last_prompt: None,
        system_message: None,
        compact: false,
        quiet: false,
        agent_session,
        last_ctrl_c: None,
    }
}

fn make_agent_session(workdir: std::path::PathBuf) -> ChatAgentSession {
    use crate::model_selection::{EffectiveModelSelection, SelectionSource};
    ChatAgentSession {
        workdir,
        model: "model-original".to_string(),
        model_selection: EffectiveModelSelection {
            requested_model: Some("model-original".to_string()),
            effective_model_key: "model-original".to_string(),
            provider_key: "provider-original".to_string(),
            provider_kind: "anthropic_api".to_string(),
            backend_slug: "backend-original".to_string(),
            source: SelectionSource::ProjectDefault,
            reason: "test fixture".to_string(),
        },
        effort: "medium".to_string(),
        system_prompt: String::new(),
        allowed_tools_csv: String::new(),
        mcp_config: None,
        session_id: None,
        api_history: Vec::new(),
        settings_json: None,
        timeout: None,
        provider_base_url: None,
        provider_api_key_env: None,
    }
}

#[test]
fn apply_model_switch_rejects_empty_arg() {
    use crate::auth_detect::AuthMethod;
    let mut session = make_session(
        DispatchMode::Direct {
            auth: AuthMethod::AnthropicApi {
                key: "sk".to_string(),
                model: Some("claude-sonnet-4-6".to_string()),
            },
        },
        None,
    );
    let err = apply_model_switch(&mut session, "   ").unwrap_err();
    assert!(err.message.contains("empty"));
    if let DispatchMode::Direct {
        auth: AuthMethod::AnthropicApi { model, .. },
    } = &session.dispatch
    {
        assert_eq!(model.as_deref(), Some("claude-sonnet-4-6"));
    } else {
        panic!("dispatch mode should still be Direct/AnthropicApi");
    }
}

#[test]
fn apply_model_switch_session_failure_leaves_selection_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let agent_session = make_agent_session(dir.path().to_path_buf());
    let original_backend = agent_session.model_selection.backend_slug.clone();
    let original_effective = agent_session.model_selection.effective_model_key.clone();
    let original_provider = agent_session.model_selection.provider_key.clone();
    let original_model = agent_session.model.clone();

    let mut session = make_session(DispatchMode::Session, Some(agent_session));

    let err = apply_model_switch(&mut session, "no-such-bogus-model").unwrap_err();
    assert!(!err.message.is_empty(), "error must surface a message");

    let restored = session
        .agent_session
        .as_ref()
        .expect("agent_session preserved");
    assert_eq!(
        restored.model_selection.backend_slug, original_backend,
        "backend_slug must be unchanged on failure"
    );
    assert_eq!(
        restored.model_selection.effective_model_key, original_effective,
        "effective_model_key must be unchanged on failure"
    );
    assert_eq!(
        restored.model_selection.provider_key, original_provider,
        "provider_key must be unchanged on failure"
    );
    assert_eq!(
        restored.model, original_model,
        "agent_session.model must be unchanged on failure"
    );
}

#[test]
fn apply_model_switch_direct_unsupported_auth_returns_hint() {
    use crate::auth_detect::AuthMethod;
    let mut session = make_session(
        DispatchMode::Direct {
            auth: AuthMethod::ClaudeCli,
        },
        None,
    );
    let err = apply_model_switch(&mut session, "claude-haiku-4-5").unwrap_err();
    assert!(
        err.hint.is_some(),
        "ClaudeCli switch should suggest setting an API key"
    );
}

#[test]
fn session_banner_label_formats_model_and_provider_kind() {
    use crate::model_selection::{EffectiveModelSelection, SelectionSource};
    let selection = EffectiveModelSelection {
        requested_model: None,
        effective_model_key: "claude-sonnet-4-6".to_string(),
        provider_key: "anthropic_prod".to_string(),
        provider_kind: "anthropic-api".to_string(),
        backend_slug: "claude-sonnet-4-6".to_string(),
        source: SelectionSource::ProjectDefault,
        reason: "test".to_string(),
    };
    let label = session_banner_label(&selection);
    assert_eq!(label, "claude-sonnet-4-6 (anthropic-api)");
}

#[test]
fn session_banner_label_with_claude_cli_provider() {
    use crate::model_selection::{EffectiveModelSelection, SelectionSource};
    let selection = EffectiveModelSelection {
        requested_model: Some("claude-opus-4-6".to_string()),
        effective_model_key: "claude-opus-4-6".to_string(),
        provider_key: "claude_cli".to_string(),
        provider_kind: "claude_cli".to_string(),
        backend_slug: "claude-opus-4-6".to_string(),
        source: SelectionSource::CliOverride,
        reason: "cli override".to_string(),
    };
    let label = session_banner_label(&selection);
    assert_eq!(label, "claude-opus-4-6 (claude_cli)");
}

#[test]
fn session_banner_label_with_openai_compat_provider() {
    use crate::model_selection::{EffectiveModelSelection, SelectionSource};
    let selection = EffectiveModelSelection {
        requested_model: Some("gpt-5.4-mini".to_string()),
        effective_model_key: "gpt-5.4-mini".to_string(),
        provider_key: "openai_prod".to_string(),
        provider_kind: "openai_compat".to_string(),
        backend_slug: "gpt-5.4-mini".to_string(),
        source: SelectionSource::ProjectDefault,
        reason: "project default".to_string(),
    };
    let label = session_banner_label(&selection);
    assert_eq!(label, "gpt-5.4-mini (openai_compat)");
}

#[test]
fn session_banner_label_uses_effective_model_key_not_requested() {
    use crate::model_selection::{EffectiveModelSelection, SelectionSource};
    let selection = EffectiveModelSelection {
        requested_model: Some("sonnet".to_string()),
        effective_model_key: "claude-sonnet-4-6".to_string(),
        provider_key: "anthropic_api".to_string(),
        provider_kind: "anthropic_api".to_string(),
        backend_slug: "claude-sonnet-4-6-20250514".to_string(),
        source: SelectionSource::CliOverride,
        reason: "cli override".to_string(),
    };
    let label = session_banner_label(&selection);
    assert_eq!(label, "claude-sonnet-4-6 (anthropic_api)");
    assert!(!label.contains("sonnet ("), "should not use the raw alias");
}

#[test]
fn session_banner_label_with_ollama_provider() {
    use crate::model_selection::{EffectiveModelSelection, SelectionSource};
    let selection = EffectiveModelSelection {
        requested_model: Some("llama3:70b".to_string()),
        effective_model_key: "llama3:70b".to_string(),
        provider_key: "local_ollama".to_string(),
        provider_kind: "ollama".to_string(),
        backend_slug: "llama3:70b".to_string(),
        source: SelectionSource::BuiltInDefault,
        reason: "built-in default".to_string(),
    };
    let label = session_banner_label(&selection);
    assert_eq!(label, "llama3:70b (ollama)");
}

#[test]
fn session_banner_label_with_empty_strings_does_not_panic() {
    use crate::model_selection::{EffectiveModelSelection, SelectionSource};
    let selection = EffectiveModelSelection {
        requested_model: None,
        effective_model_key: String::new(),
        provider_key: String::new(),
        provider_kind: String::new(),
        backend_slug: String::new(),
        source: SelectionSource::BuiltInDefault,
        reason: String::new(),
    };
    let label = session_banner_label(&selection);
    assert_eq!(label, " ()");
}

#[test]
fn session_banner_label_source_does_not_affect_output() {
    use crate::model_selection::{EffectiveModelSelection, SelectionSource};
    let sources = [
        SelectionSource::CliOverride,
        SelectionSource::ProviderOverride,
        SelectionSource::TaskModel,
        SelectionSource::RoleConfig,
        SelectionSource::CascadeRouter,
        SelectionSource::ProjectDefault,
        SelectionSource::BuiltInDefault,
    ];
    for source in sources {
        let selection = EffectiveModelSelection {
            requested_model: None,
            effective_model_key: "test-model".to_string(),
            provider_key: "test_provider".to_string(),
            provider_kind: "test_kind".to_string(),
            backend_slug: "test-slug".to_string(),
            source,
            reason: "test".to_string(),
        };
        let label = session_banner_label(&selection);
        assert_eq!(
            label, "test-model (test_kind)",
            "label should be independent of source {:?}",
            source
        );
    }
}

#[test]
fn active_model_name_returns_agent_session_model_for_session_dispatch() {
    let workdir = std::path::PathBuf::from("/tmp/test");
    let agent_session = make_agent_session(workdir);
    let session = make_session(DispatchMode::Session, Some(agent_session));
    let name = active_model_name(&session);
    assert_eq!(name, "model-original");
}

#[test]
fn active_model_name_returns_fallback_for_session_dispatch_without_agent_session() {
    let session = make_session(DispatchMode::Session, None);
    let name = active_model_name(&session);
    assert_eq!(name, "session");
}
