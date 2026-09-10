//! Entry points (`run_chat_inline`, `run_unified_inline`) and the per-key input handler.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::commands::{handle_agent_session_slash_command, handle_slash_command};
use super::dispatch::{cost_from_result, dispatch_prompt, naive_opus_cost};
use super::input::InputState;
use super::output::{push_agent_response, push_error_with_suggestions, push_tool_outputs, push_usage_line};
use super::render::render_viewport;
use super::session::{
    build_unified_inline_agent_session, format_time, load_history, load_last_session_summary,
    save_history_entry, save_session, session_banner_label,
};
use super::types::{ChatSession, ConversationMessage, DispatchMode, Phase};

use crate::auth_detect::AuthMethod;
use crate::chat;
use crate::inline::primitives::{CostMeter, StreamingState};
use crate::inline::styled;
use crate::inline::symbols;
use crate::inline::terminal::InlineTerminal;
use crate::tui::Theme;
use crate::auth as auth;

use roko_learn::cost_table::CostTable;

// ---------------------------------------------------------------------------
// Backend resolution
// ---------------------------------------------------------------------------

/// Resolve the best backend URL for an agent.
///
/// Priority: sidecar (from `.roko/runtime/agents.json`) -> serve URL.
fn resolve_chat_backend(agent_id: &str, serve_url: &str) -> (String, bool) {
    let workdir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    // Try sidecar first
    if let Some(sidecar_url) = chat::lookup_sidecar_url(agent_id, &workdir) {
        return (sidecar_url, true);
    }
    (serve_url.to_string(), false)
}

// ---------------------------------------------------------------------------
// run_chat_inline
// ---------------------------------------------------------------------------

/// Run the inline chat REPL.
///
/// If stdout is not a TTY, falls back to the legacy line-oriented REPL.
pub async fn run_chat_inline(agent_id: &str, serve_url: &str) -> Result<()> {
    if !crate::inline::should_use_inline() {
        return chat::run_chat_repl(agent_id, serve_url).await;
    }

    // --- All fallible setup BEFORE entering raw mode ---
    let credential = auth::resolve_api_key(&roko_core::config::ServeAuthConfig::default(), None);

    let mut client_builder = reqwest::Client::builder();
    if let Some(ref credential) = credential {
        client_builder = client_builder.default_headers(credential.headers());
    }
    let client = client_builder.build().context("build HTTP client")?;

    // Discover sidecar or fall back to serve
    let (backend_url, is_sidecar) = resolve_chat_backend(agent_id, serve_url);
    let backend_label = if is_sidecar {
        format!("sidecar @ {backend_url}")
    } else {
        backend_url.clone()
    };

    // --- NOW create the terminal (enters raw mode) ---
    let mut term = InlineTerminal::new().context("init inline terminal")?;
    let theme = *term.theme();

    // Push welcome banner
    let version = env!("CARGO_PKG_VERSION");
    term.push_lines(&[
        styled::section_start(
            &theme,
            "roko chat",
            &format!("v{version}  {}  {agent_id}", symbols::SEP),
            Some(&backend_label),
        ),
        Line::from(vec![
            Span::styled(symbols::END.to_string(), theme.muted()),
            Span::raw(" "),
            Span::styled(
                "Type a message. Ctrl-D to exit. /help for commands.".to_string(),
                Style::default().fg(Theme::TEXT_DIM),
            ),
        ]),
    ])?;
    if let Some((saved_at, turns, cost, topic)) = load_last_session_summary() {
        term.push_lines(&[
            Line::from(vec![
                Span::styled(format!("{} ", symbols::BAR), theme.muted()),
                Span::styled(
                    format!(
                        "Last: {saved_at}  {}  {turns} turns  {}  ${cost:.4}",
                        symbols::SEP,
                        symbols::SEP
                    ),
                    Style::default().fg(Theme::TEXT_GHOST),
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("{} ", symbols::BAR), theme.muted()),
                Span::styled(
                    format!("\"{topic}\""),
                    Style::default().fg(Theme::TEXT_GHOST),
                ),
            ]),
        ])?;
    }
    term.push_lines(&[Line::raw("")])?;

    let cost_table = CostTable {
        models: HashMap::new(),
    }
    .with_defaults();

    let mut input = InputState::new();
    input.history = load_history();

    let mut session = ChatSession {
        phase: Phase::Input,
        input,
        streaming: StreamingState::new(""),
        cost: CostMeter::new(),
        cost_table,
        agent_id: agent_id.to_string(),
        tick: 0,
        started_at: Instant::now(),
        dispatch: DispatchMode::Http {
            client,
            backend_url,
            is_sidecar,
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

    run_main_loop(&mut session, &mut term, &theme).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// run_unified_inline
// ---------------------------------------------------------------------------

/// Run the unified inline chat through `ChatAgentSession`.
///
/// This is the primary entry point for `roko` with no subcommand.
/// Session setup failures are returned instead of downgrading to direct dispatch.
pub async fn run_unified_inline(_auth: &AuthMethod) -> Result<()> {
    if !crate::inline::should_use_inline() {
        eprintln!("hint: stdin is not a TTY — use `roko \"prompt\"` for one-shot mode");
        return Ok(());
    }

    // --- All fallible setup BEFORE entering raw mode ---
    let workdir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let (agent_session, system_message, cost_table) =
        build_unified_inline_agent_session(workdir.clone()).map_err(|error| {
            tracing::warn!("{error:#}");
            error
        })?;

    // --- NOW create the terminal (enters raw mode) ---
    let mut term = InlineTerminal::new().context("init inline terminal")?;
    let theme = *term.theme();

    // Push welcome banner
    let version = env!("CARGO_PKG_VERSION");
    let workspace = workdir.display().to_string();
    let roko_initialized = workdir.join(".roko").exists();
    let init_status = if roko_initialized {
        ".roko/ initialized"
    } else {
        ".roko/ not found"
    };
    term.push_lines(&[
        styled::section_start(
            &theme,
            "roko",
            &format!(
                "v{version}  {}  {}",
                symbols::SEP,
                session_banner_label(&agent_session.model_selection)
            ),
            None,
        ),
        styled::continuation(&theme, "workspace", &workspace, Some(init_status)),
        Line::from(vec![
            Span::styled(symbols::END.to_string(), theme.muted()),
            Span::raw(" "),
            Span::styled(
                "Type a message. Ctrl-D to exit. /help for commands.".to_string(),
                Style::default().fg(Theme::TEXT_DIM),
            ),
        ]),
    ])?;
    if let Some((saved_at, turns, cost, topic)) = load_last_session_summary() {
        term.push_lines(&[
            Line::from(vec![
                Span::styled(format!("{} ", symbols::BAR), theme.muted()),
                Span::styled(
                    format!(
                        "Last: {saved_at}  {}  {turns} turns  {}  ${cost:.4}",
                        symbols::SEP,
                        symbols::SEP
                    ),
                    Style::default().fg(Theme::TEXT_GHOST),
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("{} ", symbols::BAR), theme.muted()),
                Span::styled(
                    format!("\"{topic}\""),
                    Style::default().fg(Theme::TEXT_GHOST),
                ),
            ]),
        ])?;
    }
    term.push_lines(&[Line::raw("")])?;

    let mut input = InputState::new();
    input.history = load_history();

    let mut session = ChatSession {
        phase: Phase::Input,
        input,
        streaming: StreamingState::new(""),
        cost: CostMeter::new(),
        cost_table,
        agent_id: "roko".to_string(),
        tick: 0,
        started_at: Instant::now(),
        dispatch: DispatchMode::Session,
        response_rx: None,
        streaming_event_rx: None,
        turn_count: 0,
        thinking_started: None,
        conversation: Vec::new(),
        last_prompt: None,
        system_message,
        compact: false,
        quiet: false,
        agent_session: Some(agent_session),
        last_ctrl_c: None,
    };

    run_main_loop(&mut session, &mut term, &theme).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Main event loop (shared by both entry points)
// ---------------------------------------------------------------------------

async fn run_main_loop(
    session: &mut ChatSession,
    term: &mut InlineTerminal,
    theme: &Theme,
) -> Result<()> {
    loop {
        // Draw the viewport
        term.draw(|frame| {
            render_viewport(frame, session, theme);
        })?;

        // Poll for events (30fps = ~33ms)
        if event::poll(Duration::from_millis(33)).context("poll events")? {
            if let Event::Key(key) = event::read().context("read event")? {
                match session.phase {
                    Phase::Input => {
                        if handle_input_key(key, session, term, theme).await? {
                            break;
                        }
                    }
                    Phase::Thinking | Phase::Streaming => {
                        // Ctrl+C interrupts generation; double-tap exits
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            let now = Instant::now();
                            let double_tap = session.last_ctrl_c.map_or(false, |prev| {
                                now.duration_since(prev) < Duration::from_millis(500)
                            });
                            session.last_ctrl_c = Some(now);

                            // Push partial output to scrollback
                            if !session.streaming.is_empty() {
                                let text = session.streaming.take_buffer();
                                push_agent_response(term, theme, &text, &session.agent_id)?;
                            }
                            if double_tap {
                                term.push_lines(&[styled::continuation(
                                    theme,
                                    "",
                                    "[exiting]",
                                    None,
                                )])?;
                                session.phase = Phase::Done;
                                break;
                            }
                            term.push_lines(&[styled::continuation(
                                theme,
                                "",
                                "[interrupted — Ctrl-C again to exit]",
                                None,
                            )])?;
                            session.phase = Phase::Input;
                        }
                    }
                    Phase::Error { ref prompt, .. } => {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                session.phase = Phase::Done;
                                break;
                            }
                            KeyCode::Char('r') => {
                                let retry_prompt = prompt.clone();
                                session.phase = Phase::Thinking;
                                session.thinking_started = Some(Instant::now());
                                dispatch_prompt(session, &retry_prompt);
                            }
                            KeyCode::Char('q') | KeyCode::Esc => {
                                term.push_blank()?;
                                session.phase = Phase::Input;
                            }
                            _ => {}
                        }
                    }
                    Phase::Done => break,
                }
            }
        }

        // Forward live streaming events from the Session dispatch path.
        if let Some(ref mut event_rx) = session.streaming_event_rx {
            loop {
                match event_rx.try_recv() {
                    Ok(event) => match &event {
                        roko_agent::AgentRuntimeEvent::MessageDelta { text } => {
                            if session.phase == Phase::Thinking {
                                session.phase = Phase::Streaming;
                            }
                            session.streaming.append(text);
                        }
                        roko_agent::AgentRuntimeEvent::ToolCall { name, .. } => {
                            session.streaming.set_phase(format!("tool:{name}"));
                        }
                        roko_agent::AgentRuntimeEvent::ToolOutput { .. } => {
                            session.streaming.set_phase("Streaming");
                        }
                        _ => {}
                    },
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
                }
            }
        }

        // Check for async response from background dispatch
        if let Some(ref mut rx) = session.response_rx {
            match rx.try_recv() {
                Ok(Ok(result)) => {
                    let latency = session
                        .thinking_started
                        .map(|t| t.elapsed().as_secs_f64())
                        .unwrap_or(0.0);
                    push_tool_outputs(term, theme, &result.tool_outputs)?;
                    push_agent_response(term, theme, &result.text, &session.agent_id)?;
                    let ts = format_time(Instant::now());
                    term.push_lines(&[Line::from(vec![Span::styled(
                        format!("  {ts} ({latency:.1}s)"),
                        Style::default().fg(Theme::TEXT_GHOST),
                    )])])?;
                    session.conversation.push(ConversationMessage {
                        role: "assistant".into(),
                        text: result.text.clone(),
                        timestamp: format!("{ts} ({latency:.1}s)"),
                    });
                    let cost = cost_from_result(&session.cost_table, &result);
                    let naive = naive_opus_cost(result.input_tokens, result.output_tokens);
                    session.cost.record_run(
                        cost,
                        result.input_tokens,
                        result.output_tokens,
                        &result.model,
                        naive,
                    );
                    if matches!(&session.dispatch, DispatchMode::Session) {
                        if let Some(agent_session) = session.agent_session.as_mut() {
                            if let Some(session_id) = result.session_id.clone() {
                                agent_session.session_id = Some(session_id);
                            }
                        }
                    }
                    session.turn_count += 1;
                    session.thinking_started = None;
                    if !session.quiet {
                        push_usage_line(
                            term,
                            theme,
                            &result.model,
                            result.input_tokens,
                            result.output_tokens,
                            cost,
                            latency,
                        )?;
                    }
                    if latency > 10.0 {
                        print!("\x07");
                    }
                    // Auto-save every 5 turns
                    if session.turn_count % 5 == 0 {
                        save_session(session);
                    }
                    session.phase = Phase::Input;
                    session.response_rx = None;
                    session.streaming_event_rx = None;
                    term.push_blank()?;
                }
                Ok(Err(err)) => {
                    if err == "__cancelled__" {
                        session.thinking_started = None;
                        session.phase = Phase::Input;
                        session.response_rx = None;
                        session.streaming_event_rx = None;
                    } else {
                        push_error_with_suggestions(term, theme, &err)?;
                        let prompt = session.last_prompt.clone().unwrap_or_default();
                        session.thinking_started = None;
                        session.phase = Phase::Error { prompt, error: err };
                        session.response_rx = None;
                        session.streaming_event_rx = None;
                    }
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // Still waiting -- spinner continues
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    term.push_lines(&[styled::continuation(
                        theme,
                        "error",
                        "response channel closed",
                        None,
                    )])?;
                    session.thinking_started = None;
                    session.phase = Phase::Input;
                    session.response_rx = None;
                    session.streaming_event_rx = None;
                }
            }
        }

        session.tick += 1;
        session.streaming.tick();
    }

    // Push session summary before exit
    if session.cost.run_count > 0 {
        term.push_blank()?;
        let ratio = session.cost.savings_ratio();
        let summary_line = if ratio > 1.5 {
            format!(
                "{} turns  {}  ${:.4} total  {}  {:.1}x savings vs baseline",
                session.cost.run_count,
                symbols::SEP,
                session.cost.total_cost,
                symbols::SEP,
                ratio,
            )
        } else {
            format!(
                "{} turns  {}  ${:.4} total",
                session.cost.run_count,
                symbols::SEP,
                session.cost.total_cost,
            )
        };
        term.push_lines(&[styled::section_start(
            theme,
            "session",
            &summary_line,
            None,
        )])?;
    }

    term.push_blank()?;
    save_session(session);
    Ok(())
}

// ---------------------------------------------------------------------------
// Per-key input handler
// ---------------------------------------------------------------------------

/// Returns true if the session should exit.
async fn handle_input_key(
    key: KeyEvent,
    session: &mut ChatSession,
    term: &mut InlineTerminal,
    theme: &Theme,
) -> Result<bool> {
    // --- Reverse history search mode (Ctrl+R) ---
    if session.input.search.active {
        match key.code {
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                session.input.search.next_match();
                if let Some(entry) = session.input.search.current_match(&session.input.history) {
                    session.input.buffer = entry.to_string();
                    session.input.cursor = session.input.buffer.len();
                }
            }
            KeyCode::Enter => {
                if let Some(idx) = session.input.search.accept() {
                    if let Some(entry) = session.input.history.get(idx) {
                        session.input.buffer = entry.clone();
                        session.input.cursor = session.input.buffer.len();
                    }
                } else {
                    session.input.search.deactivate();
                }
            }
            KeyCode::Esc | KeyCode::Char('c')
                if key.code == KeyCode::Esc || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                session.input.search.deactivate();
                session.input.buffer.clear();
                session.input.cursor = 0;
            }
            KeyCode::Backspace => {
                session.input.search.query.pop();
                session.input.search.match_idx = 0;
                session.input.search.update(&session.input.history);
                if let Some(entry) = session.input.search.current_match(&session.input.history) {
                    session.input.buffer = entry.to_string();
                    session.input.cursor = session.input.buffer.len();
                }
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                session.input.search.query.push(ch);
                session.input.search.match_idx = 0;
                session.input.search.update(&session.input.history);
                if let Some(entry) = session.input.search.current_match(&session.input.history) {
                    session.input.buffer = entry.to_string();
                    session.input.cursor = session.input.buffer.len();
                }
            }
            _ => {
                if let Some(idx) = session.input.search.accept() {
                    if let Some(entry) = session.input.history.get(idx) {
                        session.input.buffer = entry.clone();
                        session.input.cursor = session.input.buffer.len();
                    }
                } else {
                    session.input.search.deactivate();
                }
            }
        }
        return Ok(false);
    }

    // --- Command palette mode (Ctrl+K) ---
    if session.input.palette.active {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                session.input.palette.dismiss();
                let now = Instant::now();
                let double_tap = session.last_ctrl_c.map_or(false, |prev| {
                    now.duration_since(prev) < Duration::from_millis(500)
                });
                session.last_ctrl_c = Some(now);
                if session.input.is_empty() || double_tap {
                    session.phase = Phase::Done;
                    return Ok(true);
                }
                session.input.clear();
                session.input.completion.dismiss();
                return Ok(false);
            }
            KeyCode::Esc => {
                session.input.palette.dismiss();
            }
            KeyCode::Enter => {
                if let Some(cmd) = session.input.palette.accept() {
                    return handle_slash_command(&cmd, session, term, theme);
                }
            }
            KeyCode::Up => session.input.palette.select_prev(),
            KeyCode::Down => session.input.palette.select_next(),
            KeyCode::Backspace => {
                if session.input.palette.query.is_empty() {
                    session.input.palette.dismiss();
                } else {
                    session.input.palette.backspace();
                }
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                session.input.palette.type_char(ch);
            }
            _ => {}
        }
        return Ok(false);
    }

    let dropdown_visible = session.input.completion.visible;

    match key.code {
        // --- Shift+Enter: insert newline ---
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            session.input.insert_newline();
            session.input.completion.dismiss();
        }

        // --- Enter ---
        KeyCode::Enter => {
            if dropdown_visible {
                if let Some(cmd) = session.input.completion.accept() {
                    session.input.buffer = cmd;
                    session.input.cursor = session.input.buffer.len();
                }
                return Ok(false);
            }
            if session.input.is_empty() {
                return Ok(false);
            }
            session.input.completion.dismiss();
            let text = session.input.submit();

            // Persist to history file
            save_history_entry(&text);

            if let Some(handled) = handle_agent_session_slash_command(&text, session, term, theme)?
            {
                return Ok(handled);
            }

            if text.starts_with('/') {
                return handle_slash_command(&text, session, term, theme);
            }

            // Push user message to scrollback with timestamp
            let timestamp = format_time(Instant::now());
            if text.contains('\n') {
                let msg_lines: Vec<&str> = text.split('\n').collect();
                let mut scroll_lines = vec![Line::from(vec![
                    Span::styled(
                        format!("{} ", symbols::PROMPT),
                        Style::default().fg(Theme::ROSE),
                    ),
                    Span::styled(msg_lines[0].to_string(), Style::default().fg(Theme::BONE)),
                    Span::styled(
                        format!("  {timestamp}"),
                        Style::default().fg(Theme::TEXT_GHOST),
                    ),
                ])];
                for line in &msg_lines[1..] {
                    scroll_lines.push(Line::from(vec![
                        Span::styled(
                            format!("{} ", symbols::BAR),
                            Style::default().fg(Theme::TEXT_DIM),
                        ),
                        Span::styled(line.to_string(), Style::default().fg(Theme::BONE)),
                    ]));
                }
                term.push_lines(&scroll_lines)?;
            } else {
                term.push_lines(&[Line::from(vec![
                    Span::styled(
                        format!("{} ", symbols::PROMPT),
                        Style::default().fg(Theme::ROSE),
                    ),
                    Span::styled(text.clone(), Style::default().fg(Theme::BONE)),
                    Span::styled(
                        format!("  {timestamp}"),
                        Style::default().fg(Theme::TEXT_GHOST),
                    ),
                ])])?;
            }

            session.conversation.push(ConversationMessage {
                role: "user".into(),
                text: text.clone(),
                timestamp: timestamp.clone(),
            });

            session.last_prompt = Some(text.clone());

            session.phase = Phase::Thinking;
            session.thinking_started = Some(Instant::now());
            dispatch_prompt(session, &text);
        }

        // --- Escape: dismiss dropdown ---
        KeyCode::Esc => {
            session.input.completion.dismiss();
        }

        // --- Exit ---
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            session.phase = Phase::Done;
            return Ok(true);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let now = Instant::now();
            let double_tap = session.last_ctrl_c.map_or(false, |prev| {
                now.duration_since(prev) < Duration::from_millis(500)
            });
            session.last_ctrl_c = Some(now);

            if session.input.is_empty() || double_tap {
                session.phase = Phase::Done;
                return Ok(true);
            }
            session.input.clear();
            session.input.completion.dismiss();
        }

        // --- Editing ---
        KeyCode::Backspace => {
            session.input.backspace();
            session.input.completion.update(&session.input.buffer);
        }
        KeyCode::Delete => session.input.delete(),
        KeyCode::Left => {
            session.input.move_left();
            session.input.completion.dismiss();
        }
        KeyCode::Right => {
            if !dropdown_visible
                && session.input.cursor == session.input.buffer.len()
                && session.input.accept_ghost()
            {
                // Accepted ghost suggestion
            } else {
                session.input.move_right();
                session.input.completion.dismiss();
            }
        }
        KeyCode::Home | KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            session.input.home();
            session.input.completion.dismiss();
        }
        KeyCode::End | KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !dropdown_visible
                && session.input.cursor == session.input.buffer.len()
                && session.input.accept_ghost()
            {
                // Accepted ghost suggestion
            } else {
                session.input.end();
                session.input.completion.dismiss();
            }
        }

        // --- Tab / Shift+Tab: completion navigation ---
        KeyCode::Tab => {
            if dropdown_visible {
                session.input.completion.select_next();
            } else {
                session.input.completion.update(&session.input.buffer);
            }
        }
        KeyCode::BackTab => {
            if dropdown_visible {
                session.input.completion.select_prev();
            }
        }

        // --- Up / Down: dropdown nav or history ---
        KeyCode::Up => {
            if dropdown_visible {
                session.input.completion.select_prev();
            } else {
                session.input.history_up();
            }
        }
        KeyCode::Down => {
            if dropdown_visible {
                session.input.completion.select_next();
            } else {
                session.input.history_down();
            }
        }

        // --- Clear line ---
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            session.input.clear();
            session.input.completion.dismiss();
        }

        // --- Clear screen ---
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..term.viewport_height() {
                term.push_blank()?;
            }
        }

        // --- Command palette ---
        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            session.input.completion.dismiss();
            session.input.palette.open();
        }

        // --- Reverse history search ---
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            session.input.search.active = true;
            session.input.search.query.clear();
            session.input.search.match_idx = 0;
            session.input.search.matches.clear();
            session.input.completion.dismiss();
        }

        // --- Regular character input ---
        KeyCode::Char(ch) => {
            session.input.insert(ch);
            if session.input.buffer.trim_start().starts_with('/') {
                session.input.completion.update(&session.input.buffer);
            } else {
                session.input.completion.dismiss();
            }
        }

        _ => {}
    }

    Ok(false)
}
