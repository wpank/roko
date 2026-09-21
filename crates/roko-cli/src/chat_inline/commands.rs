//! Slash command handling (`/help`, `/model`, `/status`, etc.).

use anyhow::Result;
use ratatui::text::{Line, Span};
use serde_json::json;

use super::session::{
    active_model_name, active_system_prompt, apply_model_switch, current_model_name, preview_text,
    session_banner_label,
};
use super::types::{ChatSession, DispatchMode, Phase};
use crate::auth_detect::AuthMethod;
use crate::chat_session::SlashResult;
use crate::inline::primitives::CostMeter;
use crate::inline::styled;
use crate::inline::symbols;
use crate::inline::terminal::InlineTerminal;
use crate::tui::Theme;

// ---------------------------------------------------------------------------
// Shell helpers
// ---------------------------------------------------------------------------

/// Run a shell command and return stdout (or stderr if stdout is empty).
fn shell_output(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .current_dir(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            if stdout.trim().is_empty() {
                stderr
            } else {
                stdout
            }
        })
        .unwrap_or_else(|e| format!("error: {e}"))
}

/// Push shell output as scrollback lines (truncated to max_lines).
fn push_shell_output(
    term: &mut InlineTerminal,
    theme: &Theme,
    label: &str,
    output: &str,
    max_lines: usize,
) -> std::io::Result<()> {
    let lines: Vec<&str> = output.lines().collect();
    let truncated = lines.len() > max_lines;
    let mut styled_lines = vec![styled::section_start(theme, label, "", None)];
    for line in lines.iter().take(max_lines) {
        styled_lines.push(Line::from(vec![
            Span::styled(format!("{} ", symbols::BAR), theme.muted()),
            Span::styled(line.to_string(), theme.text()),
        ]));
    }
    if truncated {
        styled_lines.push(styled::continuation(
            theme,
            "",
            &format!("... ({} more lines)", lines.len() - max_lines),
            None,
        ));
    }
    styled_lines.push(Line::from(vec![Span::styled(
        symbols::END.to_string(),
        theme.muted(),
    )]));
    term.push_lines(&styled_lines)
}

/// Read roko.toml and return its content.
fn read_roko_toml() -> Option<String> {
    let path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("roko.toml");
    std::fs::read_to_string(path).ok()
}

// ---------------------------------------------------------------------------
// Agent session slash commands
// ---------------------------------------------------------------------------

/// Let `ChatAgentSession` handle slash commands that mutate its own state.
pub(crate) fn handle_agent_session_slash_command(
    text: &str,
    session: &mut ChatSession,
    term: &mut InlineTerminal,
    theme: &Theme,
) -> Result<Option<bool>> {
    let trimmed = text.trim();
    if !trimmed.starts_with('/') {
        return Ok(None);
    }

    let result = {
        session
            .agent_session
            .as_mut()
            .map(|agent_session| agent_session.handle_slash_command(trimmed))
    };

    let Some(result) = result else {
        return Ok(None);
    };

    match result {
        SlashResult::Updated(msg) => {
            if trimmed.starts_with("/system") || trimmed.starts_with("/reset") {
                session.system_message = active_system_prompt(session);
            }
            if trimmed.starts_with("/reset") {
                session.conversation.clear();
                session.turn_count = 0;
                session.cost = CostMeter::new();
                session.last_prompt = None;
            }
            term.push_lines(&[styled::continuation(theme, "config", &msg, None)])?;
            Ok(Some(false))
        }
        SlashResult::Display(text) => {
            let mut lines: Vec<Line<'static>> = Vec::new();
            for (idx, line) in text.lines().enumerate() {
                if idx == 0 {
                    let parts: Vec<&str> = line.splitn(2, "  ").collect();
                    let (label, value) = if parts.len() == 2 {
                        (parts[0].to_string(), parts[1].to_string())
                    } else {
                        (String::new(), line.to_string())
                    };
                    lines.push(styled::section_start(theme, &label, &value, None));
                } else if line.trim().is_empty() {
                    continue;
                } else {
                    let trimmed_line = line.trim_start_matches("  ");
                    let parts: Vec<&str> = trimmed_line.splitn(2, "  ").collect();
                    let (label, value) = if parts.len() == 2 {
                        (parts[0].trim().to_string(), parts[1].trim().to_string())
                    } else {
                        (String::new(), trimmed_line.to_string())
                    };
                    lines.push(styled::continuation(theme, &label, &value, None));
                }
            }
            term.push_lines(&lines)?;
            Ok(Some(false))
        }
        SlashResult::Error(msg) => {
            term.push_lines(&[styled::continuation(theme, "error", &msg, None)])?;
            Ok(Some(false))
        }
        SlashResult::Unknown(_) | SlashResult::NotACommand => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Main slash command handler
// ---------------------------------------------------------------------------

/// Handle `/` commands. Returns true if the session should exit.
pub(crate) fn handle_slash_command(
    text: &str,
    session: &mut ChatSession,
    term: &mut InlineTerminal,
    theme: &Theme,
) -> Result<bool> {
    let cmd = text.trim();
    if let Some(exit) = handle_agent_session_slash_command(cmd, session, term, theme)? {
        return Ok(exit);
    }
    match cmd {
        // =================================================================
        // Session & display
        // =================================================================
        "/quit" | "/exit" | "/q" => {
            session.phase = Phase::Done;
            return Ok(true);
        }
        "/help" | "/h" => {
            term.push_lines(&[
                styled::section_start(theme, "help", "session & display", None),
                styled::continuation(theme, "/model <name>", "show or change model", None),
                styled::continuation(theme, "/provider", "show auth/provider info", None),
                styled::continuation(theme, "/cost", "session cost summary", None),
                styled::continuation(theme, "/stats", "detailed session statistics", None),
                styled::continuation(
                    theme,
                    "/context",
                    "session context (model, tools, mcp)",
                    None,
                ),
                styled::continuation(theme, "/tools", "list available tools", None),
                styled::continuation(theme, "/mcp", "MCP config status", None),
                styled::continuation(theme, "/version", "version info", None),
                styled::continuation(theme, "/history", "show conversation turns", None),
                styled::continuation(theme, "/input-history", "show typed input history", None),
                styled::continuation(theme, "/copy", "copy last response to clipboard", None),
                styled::continuation(theme, "/compact", "toggle compact output", None),
                styled::continuation(theme, "/quiet", "toggle per-turn usage summary", None),
                styled::continuation(theme, "/system <text>", "set system message", None),
                styled::continuation(theme, "/reset", "clear conversation", None),
                styled::continuation(theme, "/retry", "resend last message", None),
                styled::continuation(theme, "/export [md|json]", "export conversation", None),
                styled::continuation(theme, "/clear", "clear scrollback", None),
            ])?;
            term.push_lines(&[
                styled::section_start(theme, "", "configuration", None),
                styled::continuation(theme, "/config", "show config summary", None),
                styled::continuation(theme, "/config providers", "list providers", None),
                styled::continuation(theme, "/config models", "list all models", None),
                styled::continuation(theme, "/config gates", "gate configuration", None),
                styled::continuation(theme, "/config set <k> <v>", "set config value", None),
                styled::continuation(
                    theme,
                    "/effort <level>",
                    "set effort (low/med/high/max)",
                    None,
                ),
                styled::continuation(theme, "/gate <name> on|off", "toggle gate", None),
            ])?;
            term.push_lines(&[
                styled::section_start(theme, "", "workspace & git", None),
                styled::continuation(theme, "/status", "workspace status", None),
                styled::continuation(theme, "/doctor", "health check", None),
                styled::continuation(theme, "/diff", "git diff", None),
                styled::continuation(theme, "/git", "git status", None),
                styled::continuation(theme, "/log [n]", "recent commits", None),
                styled::continuation(theme, "/branch", "current branch", None),
                styled::continuation(theme, "/changes", "changed files", None),
            ])?;
            term.push_lines(&[
                styled::section_start(theme, "", "files & search", None),
                styled::continuation(theme, "/file <path>", "read a file", None),
                styled::continuation(theme, "/search <pattern>", "grep workspace", None),
                styled::continuation(theme, "/find <pattern>", "find files", None),
                styled::continuation(theme, "/tree [path]", "directory tree", None),
            ])?;
            term.push_lines(&[
                styled::section_start(theme, "", "agents & workflows", None),
                styled::continuation(theme, "/agent [name]", "show/switch agent", None),
                styled::continuation(theme, "/run <prompt>", "universal loop", None),
                styled::continuation(theme, "/plan list|run|generate", "plan management", None),
                styled::continuation(theme, "/prd idea|list", "PRD management", None),
                styled::continuation(theme, "/research <query>", "research a topic", None),
                styled::continuation(theme, "/knowledge <query>", "query knowledge", None),
                styled::continuation(theme, "/learn", "learning state", None),
                styled::section_end(theme, "/quit", "exit the chat"),
            ])?;
        }
        "/version" | "/v" => {
            let version = env!("CARGO_PKG_VERSION");
            let rustc = shell_output("rustc", &["--version"]);
            term.push_lines(&[
                styled::section_start(theme, "version", "", None),
                styled::continuation(theme, "roko", &format!("v{version}"), None),
                styled::continuation(theme, "rustc", rustc.trim(), None),
                styled::continuation(theme, "platform", std::env::consts::OS, None),
                styled::section_end(theme, "arch", std::env::consts::ARCH),
            ])?;
        }
        "/stats" => {
            let elapsed = session.started_at.elapsed();
            let mins = elapsed.as_secs() / 60;
            let secs = elapsed.as_secs() % 60;
            let avg_cost = if session.turn_count > 0 {
                session.cost.total_cost / session.turn_count as f64
            } else {
                0.0
            };
            let avg_tokens = if session.turn_count > 0 {
                (session.cost.input_tokens + session.cost.output_tokens) / session.turn_count as u64
            } else {
                0
            };
            term.push_lines(&[
                styled::section_start(theme, "stats", "session details", None),
                styled::continuation(theme, "elapsed", &format!("{mins}m {secs}s"), None),
                styled::continuation(theme, "turns", &session.turn_count.to_string(), None),
                styled::continuation(
                    theme,
                    "total cost",
                    &format!("${:.4}", session.cost.total_cost.max(0.0)),
                    None,
                ),
                styled::continuation(
                    theme,
                    "avg/turn",
                    &format!("${:.4} cost, {avg_tokens} tokens", avg_cost.max(0.0)),
                    None,
                ),
                styled::continuation(
                    theme,
                    "tokens in",
                    &session.cost.input_tokens.to_string(),
                    None,
                ),
                styled::continuation(
                    theme,
                    "tokens out",
                    &session.cost.output_tokens.to_string(),
                    None,
                ),
                styled::continuation(
                    theme,
                    "messages",
                    &session.conversation.len().to_string(),
                    None,
                ),
                styled::section_end(
                    theme,
                    "savings",
                    &format!("{:.1}x vs baseline", session.cost.savings_ratio()),
                ),
            ])?;
        }
        "/context" => {
            let model = current_model_name(session);
            let provider: String = match &session.dispatch {
                DispatchMode::Direct { auth } => match auth {
                    AuthMethod::AnthropicApi { .. } => "anthropic-api".to_string(),
                    AuthMethod::ClaudeCli => "claude-cli".to_string(),
                    AuthMethod::CliProvider { label } => format!("{label}-cli"),
                    AuthMethod::OpenAiCompat { .. } => "openai-compat".to_string(),
                    AuthMethod::NeedsSetup => "not configured".to_string(),
                },
                DispatchMode::Http { is_sidecar, .. } => {
                    if *is_sidecar {
                        "agent-sidecar".to_string()
                    } else {
                        "roko-serve".to_string()
                    }
                }
                DispatchMode::Session => "session".to_string(),
            };
            let workdir = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string());
            let mcp_status: String = if std::path::Path::new(".roko/mcp.json").exists() {
                ".roko/mcp.json".to_string()
            } else if std::path::Path::new(".roko/mcp.toml").exists() {
                ".roko/mcp.toml".to_string()
            } else {
                "none".to_string()
            };
            let system_preview: String = match &session.system_message {
                Some(s) if !s.is_empty() => {
                    if s.len() > 200 {
                        format!("{}... [{} chars]", &s[..200], s.len())
                    } else {
                        s.clone()
                    }
                }
                _ => "(none)".to_string(),
            };
            let total_tokens = session.cost.input_tokens + session.cost.output_tokens;
            use roko_std::tool::builtin::BUILTIN_TOOL_NAMES;
            let tool_count = BUILTIN_TOOL_NAMES.len();
            term.push_lines(&[
                styled::section_start(theme, "context", "session", None),
                styled::continuation(theme, "workdir", &workdir, None),
                styled::continuation(theme, "model", &model, None),
                styled::continuation(theme, "provider", &provider, None),
                styled::continuation(theme, "agent", &session.agent_id, None),
                styled::continuation(theme, "turns", &session.turn_count.to_string(), None),
                styled::continuation(
                    theme,
                    "history",
                    &format!("{} entries", session.input.history.len()),
                    None,
                ),
                styled::continuation(theme, "tools", &format!("{tool_count} available"), None),
                styled::continuation(theme, "mcp", &mcp_status, None),
                styled::continuation(theme, "tokens", &total_tokens.to_string(), None),
                styled::continuation(theme, "system", &system_preview, None),
                styled::section_end(
                    theme,
                    "cost",
                    &format!("${:.4}", session.cost.total_cost.max(0.0)),
                ),
            ])?;
        }
        "/history" => {
            let is_claude_cli = matches!(
                &session.dispatch,
                DispatchMode::Direct {
                    auth: AuthMethod::ClaudeCli
                }
            );

            if session.conversation.is_empty() {
                let mut lines = vec![styled::section_start(
                    theme,
                    "history",
                    "no turns yet",
                    None,
                )];
                if is_claude_cli {
                    lines.push(styled::continuation(
                        theme,
                        "note",
                        "Claude CLI manages its own session history",
                        None,
                    ));
                }
                lines.push(styled::section_end(theme, "tip", "send a message to start"));
                term.push_lines(&lines)?;
            } else {
                let cli_badge = if is_claude_cli {
                    Some("CLI-managed")
                } else {
                    None
                };
                let mut lines = vec![styled::section_start(
                    theme,
                    "history",
                    &format!("{} turns", session.turn_count),
                    cli_badge,
                )];
                let start = session.conversation.len().saturating_sub(20);
                for (i, msg) in session.conversation[start..].iter().enumerate() {
                    let turn_num = start + i + 1;
                    let preview = preview_text(&msg.text, 50);
                    let badge = format!("{} chars", msg.text.chars().count());
                    lines.push(styled::continuation(
                        theme,
                        &format!("#{turn_num} {}", msg.role),
                        &preview,
                        Some(badge.as_str()),
                    ));
                }
                if is_claude_cli {
                    lines.push(styled::continuation(
                        theme,
                        "note",
                        "full history managed by Claude CLI process",
                        None,
                    ));
                }
                lines.push(Line::from(vec![Span::styled(
                    symbols::END.to_string(),
                    theme.muted(),
                )]));
                term.push_lines(&lines)?;
            }
        }
        "/input-history" => {
            let history = &session.input.history;
            if history.is_empty() {
                term.push_lines(&[styled::continuation(
                    theme,
                    "input-history",
                    "no input history",
                    None,
                )])?;
            } else {
                let start = history.len().saturating_sub(20);
                let mut lines = vec![styled::section_start(
                    theme,
                    "input-history",
                    &format!(
                        "{} entries (showing last {})",
                        history.len(),
                        history.len() - start
                    ),
                    None,
                )];
                for (i, entry) in history[start..].iter().enumerate() {
                    let display = preview_text(entry, 60);
                    lines.push(styled::continuation(
                        theme,
                        &format!("{}", start + i + 1),
                        &display,
                        None,
                    ));
                }
                lines.push(Line::from(vec![Span::styled(
                    symbols::END.to_string(),
                    theme.muted(),
                )]));
                term.push_lines(&lines)?;
            }
        }
        "/copy" => {
            if let Some(last) = session
                .conversation
                .iter()
                .rev()
                .find(|m| m.role == "assistant")
            {
                let result = std::process::Command::new("pbcopy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .and_then(|mut child| {
                        if let Some(ref mut stdin) = child.stdin {
                            use std::io::Write;
                            stdin.write_all(last.text.as_bytes())?;
                        }
                        child.wait()
                    });
                match result {
                    Ok(status) if status.success() => {
                        let preview = if last.text.len() > 50 {
                            format!("{}...", &last.text[..47])
                        } else {
                            last.text.clone()
                        };
                        term.push_lines(&[styled::continuation(
                            theme,
                            "copy",
                            "copied to clipboard",
                            Some(&preview),
                        )])?;
                    }
                    _ => {
                        term.push_lines(&[styled::continuation(
                            theme,
                            "copy",
                            "clipboard not available",
                            Some("pbcopy/xclip not found"),
                        )])?;
                    }
                }
            } else {
                term.push_lines(&[styled::continuation(
                    theme,
                    "copy",
                    "no response to copy",
                    None,
                )])?;
            }
        }
        "/compact" => {
            session.compact = !session.compact;
            let state = if session.compact { "on" } else { "off" };
            term.push_lines(&[styled::continuation(theme, "compact", state, None)])?;
        }
        "/quiet" => {
            session.quiet = !session.quiet;
            let state = if session.quiet { "on" } else { "off" };
            term.push_lines(&[styled::continuation(
                theme,
                "quiet",
                &format!("usage line {state}"),
                None,
            )])?;
        }
        _ if cmd.starts_with("/system") => {
            let msg = cmd.strip_prefix("/system").unwrap().trim();
            if msg.is_empty() {
                if let Some(sys) = active_system_prompt(session) {
                    term.push_lines(&[styled::continuation(theme, "system", &sys, None)])?;
                } else {
                    term.push_lines(&[styled::continuation(
                        theme,
                        "system",
                        "no system message set",
                        Some("/system <text>"),
                    )])?;
                }
            } else {
                session.system_message = Some(msg.to_string());
                if let Some(agent_session) = session.agent_session.as_mut() {
                    agent_session.system_prompt = msg.to_string();
                }
                term.push_lines(&[styled::continuation(theme, "system", "set", Some(msg))])?;
            }
        }
        "/reset" => {
            session.conversation.clear();
            session.turn_count = 0;
            session.cost = CostMeter::new();
            session.last_prompt = None;
            session.system_message = if session.agent_session.is_some() {
                active_system_prompt(session)
            } else {
                None
            };
            term.push_lines(&[styled::continuation(
                theme,
                "reset",
                "conversation cleared",
                None,
            )])?;
        }
        "/cost" => {
            let ratio = session.cost.savings_ratio();
            term.push_lines(&[
                styled::section_start(theme, "cost", "session summary", None),
                styled::continuation(theme, "turns", &session.cost.run_count.to_string(), None),
                styled::continuation(
                    theme,
                    "total",
                    &format!("${:.4}", session.cost.total_cost.max(0.0)),
                    None,
                ),
                styled::continuation(
                    theme,
                    "tokens",
                    &format!(
                        "{} in / {} out",
                        session.cost.input_tokens, session.cost.output_tokens
                    ),
                    None,
                ),
                styled::section_end(theme, "savings", &format!("{ratio:.1}x vs baseline")),
            ])?;
        }
        "/provider" | "/auth" => {
            let info = match &session.dispatch {
                DispatchMode::Direct { auth } => auth.label(),
                DispatchMode::Http {
                    backend_url,
                    is_sidecar,
                    ..
                } => {
                    format!(
                        "HTTP {} ({})",
                        backend_url,
                        if *is_sidecar { "sidecar" } else { "serve" }
                    )
                }
                DispatchMode::Session => session
                    .agent_session
                    .as_ref()
                    .map(|agent_session| session_banner_label(&agent_session.model_selection))
                    .unwrap_or_else(|| "ChatAgentSession".to_string()),
            };
            term.push_lines(&[styled::continuation(theme, "provider", &info, None)])?;
        }
        _ if cmd.starts_with("/model") => {
            let arg = cmd.strip_prefix("/model").unwrap().trim();
            if arg.is_empty() {
                let current = match &session.dispatch {
                    DispatchMode::Direct { auth } => match auth {
                        AuthMethod::ClaudeCli => "claude CLI (auto)".to_string(),
                        AuthMethod::CliProvider { label } => format!("{label} CLI (auto)"),
                        AuthMethod::AnthropicApi { model, .. } => {
                            let m = model.as_deref().unwrap_or("claude-sonnet-4-6");
                            format!("{m} (Anthropic API)")
                        }
                        AuthMethod::OpenAiCompat {
                            model, base_url, ..
                        } => {
                            let m = model.as_deref().unwrap_or("gpt-5.4-mini");
                            format!("{m} ({base_url})")
                        }
                        AuthMethod::NeedsSetup => "none".to_string(),
                    },
                    DispatchMode::Http { .. } => "HTTP backend (model set server-side)".to_string(),
                    DispatchMode::Session => active_model_name(session),
                };
                term.push_lines(&[styled::continuation(theme, "model", &current, None)])?;
            } else {
                match apply_model_switch(session, arg) {
                    Ok(label) => {
                        term.push_lines(&[styled::continuation(
                            theme,
                            "model",
                            &format!("switched to {label}"),
                            None,
                        )])?;
                    }
                    Err(error) => {
                        term.push_lines(&[styled::continuation(
                            theme,
                            "model",
                            &error.message,
                            error.hint.as_deref(),
                        )])?;
                    }
                }
            }
        }
        _ if cmd.starts_with("/effort") => {
            let arg = cmd.strip_prefix("/effort").unwrap().trim();
            if arg.is_empty() {
                let current = session
                    .agent_session
                    .as_ref()
                    .map(|agent_session| agent_session.effort.as_str())
                    .unwrap_or("medium");
                term.push_lines(&[styled::continuation(
                    theme,
                    "effort",
                    &format!("current: {current}"),
                    Some("use: low, medium, high, max"),
                )])?;
            } else {
                match arg {
                    "low" | "medium" | "med" | "high" | "max" => {
                        if let Some(agent_session) = session.agent_session.as_mut() {
                            agent_session.effort = arg.to_string();
                        }
                        term.push_lines(&[styled::continuation(
                            theme,
                            "effort",
                            &format!("set to {arg}"),
                            None,
                        )])?;
                    }
                    _ => {
                        term.push_lines(&[styled::continuation(
                            theme,
                            "effort",
                            &format!("unknown level: {arg}"),
                            Some("use: low, medium, high, max"),
                        )])?;
                    }
                }
            }
        }
        _ if cmd.starts_with("/gate") => {
            let arg = cmd.strip_prefix("/gate").unwrap().trim();
            if arg.is_empty() {
                if let Some(toml) = read_roko_toml() {
                    let clippy = if toml.contains("clippy_enabled = true") {
                        "on"
                    } else {
                        "off"
                    };
                    let tests = if toml.contains("skip_tests = false") {
                        "on"
                    } else {
                        "off"
                    };
                    term.push_lines(&[
                        styled::section_start(theme, "gates", "", None),
                        styled::continuation(theme, "compile", "on (always)", None),
                        styled::continuation(theme, "test", tests, None),
                        styled::continuation(theme, "clippy", clippy, None),
                        styled::section_end(theme, "max iter", "3"),
                    ])?;
                } else {
                    term.push_lines(&[styled::continuation(
                        theme,
                        "gates",
                        "no roko.toml found",
                        None,
                    )])?;
                }
            } else {
                term.push_lines(&[styled::continuation(
                    theme,
                    "gate",
                    &format!("gate toggle: {arg}"),
                    Some("update roko.toml"),
                )])?;
            }
        }

        // =================================================================
        // Configuration
        // =================================================================
        "/config" => {
            if let Some(toml) = read_roko_toml() {
                let mut lines = vec![styled::section_start(theme, "config", "roko.toml", None)];
                for section in &[
                    "[project]",
                    "[agent]",
                    "[routing]",
                    "[gates]",
                    "[budget]",
                    "[conductor]",
                ] {
                    if let Some(pos) = toml.find(section) {
                        let chunk: String =
                            toml[pos..].lines().take(6).collect::<Vec<_>>().join("\n");
                        for line in chunk.lines().take(5) {
                            if !line.trim().is_empty() {
                                lines.push(Line::from(vec![
                                    Span::styled(format!("{} ", symbols::BAR), theme.muted()),
                                    Span::styled(line.to_string(), theme.text()),
                                ]));
                            }
                        }
                    }
                }
                lines.push(Line::from(vec![Span::styled(
                    symbols::END.to_string(),
                    theme.muted(),
                )]));
                term.push_lines(&lines)?;
            } else {
                term.push_lines(&[styled::continuation(
                    theme,
                    "config",
                    "no roko.toml found",
                    Some("run: roko init"),
                )])?;
            }
        }
        "/config providers" => {
            if let Some(toml) = read_roko_toml() {
                let mut lines = vec![styled::section_start(theme, "providers", "", None)];
                for line in toml.lines() {
                    if line.starts_with("[providers.") {
                        let name = line.trim_start_matches("[providers.").trim_end_matches(']');
                        let next_lines: Vec<&str> = toml[toml.find(line).unwrap()..]
                            .lines()
                            .skip(1)
                            .take(4)
                            .collect();
                        let env_key = next_lines
                            .iter()
                            .find(|l| l.contains("api_key_env"))
                            .and_then(|l| l.split('"').nth(1))
                            .unwrap_or("");
                        let has_key = if env_key.is_empty() {
                            "no key needed"
                        } else if std::env::var(env_key).is_ok() {
                            "key set"
                        } else {
                            "key missing"
                        };
                        let default_model = next_lines
                            .iter()
                            .find(|l| l.contains("default_model"))
                            .and_then(|l| l.split('"').nth(1))
                            .unwrap_or("?");
                        lines.push(styled::continuation(
                            theme,
                            name,
                            default_model,
                            Some(has_key),
                        ));
                    }
                }
                lines.push(Line::from(vec![Span::styled(
                    symbols::END.to_string(),
                    theme.muted(),
                )]));
                term.push_lines(&lines)?;
            } else {
                term.push_lines(&[styled::continuation(
                    theme,
                    "providers",
                    "no roko.toml",
                    None,
                )])?;
            }
        }
        "/config models" => {
            if let Some(toml) = read_roko_toml() {
                let mut lines = vec![styled::section_start(
                    theme,
                    "models",
                    "all configured models",
                    None,
                )];
                for line in toml.lines() {
                    if line.starts_with("[models.") && !line.starts_with("[models]") {
                        let alias = line.trim_start_matches("[models.").trim_end_matches(']');
                        let next_lines: Vec<&str> = toml[toml.find(line).unwrap()..]
                            .lines()
                            .skip(1)
                            .take(3)
                            .collect();
                        let provider = next_lines
                            .iter()
                            .find(|l| l.contains("provider"))
                            .and_then(|l| l.split('"').nth(1))
                            .unwrap_or("?");
                        let slug = next_lines
                            .iter()
                            .find(|l| l.contains("slug"))
                            .and_then(|l| l.split('"').nth(1))
                            .unwrap_or("?");
                        lines.push(styled::continuation(theme, alias, slug, Some(provider)));
                    }
                }
                lines.push(Line::from(vec![Span::styled(
                    symbols::END.to_string(),
                    theme.muted(),
                )]));
                term.push_lines(&lines)?;
            } else {
                term.push_lines(&[styled::continuation(theme, "models", "no roko.toml", None)])?;
            }
        }
        "/config gates" => {
            // Delegate to /gate
            return handle_slash_command("/gate", session, term, theme);
        }
        _ if cmd.starts_with("/config set ") => {
            let rest = cmd.strip_prefix("/config set ").unwrap().trim();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() < 2 {
                term.push_lines(&[styled::continuation(
                    theme,
                    "config",
                    "usage: /config set <key> <value>",
                    None,
                )])?;
            } else {
                let (key, value) = (parts[0], parts[1]);
                term.push_lines(&[styled::continuation(
                    theme,
                    "config",
                    &format!("set {key} = {value}"),
                    Some("edit roko.toml to persist"),
                )])?;
            }
        }

        // =================================================================
        // Workspace & git
        // =================================================================
        "/status" => {
            let roko_dir = std::path::Path::new(".roko");
            let has_roko = roko_dir.exists();
            let signal_count = std::fs::read_to_string(".roko/engrams.jsonl")
                .map(|s| s.lines().count())
                .unwrap_or(0);
            let episode_count = std::fs::read_to_string(".roko/episodes.jsonl")
                .map(|s| s.lines().count())
                .unwrap_or(0);
            let plan_count = std::fs::read_dir(".roko/plans")
                .map(|d| d.count())
                .unwrap_or(0);
            let branch = shell_output("git", &["branch", "--show-current"]);
            term.push_lines(&[
                styled::section_start(theme, "status", "", None),
                styled::continuation(
                    theme,
                    ".roko",
                    if has_roko { "initialized" } else { "not found" },
                    None,
                ),
                styled::continuation(theme, "branch", branch.trim(), None),
                styled::continuation(theme, "signals", &signal_count.to_string(), None),
                styled::continuation(theme, "episodes", &episode_count.to_string(), None),
                styled::section_end(theme, "plans", &plan_count.to_string()),
            ])?;
        }
        "/doctor" => {
            use roko_execution::diagnostics::{
                DiagnosticCheckId, DiagnosticRequest, DiagnosticService, DiagnosticSeverity,
            };

            let selected = [
                DiagnosticCheckId::Config,
                DiagnosticCheckId::Workspace,
                DiagnosticCheckId::Git,
                DiagnosticCheckId::Credentials,
            ]
            .into_iter()
            .collect();

            let report = DiagnosticService::run(&DiagnosticRequest {
                workdir: std::env::current_dir().unwrap_or_default(),
                selected,
                profile: None,
                allow_repairs: false,
            });

            let mut lines = vec![styled::section_start(
                theme,
                "doctor",
                "workspace health",
                None,
            )];
            for finding in &report.findings {
                let icon = match finding.severity {
                    DiagnosticSeverity::Info => symbols::PASS,
                    DiagnosticSeverity::Warning => symbols::WARN,
                    DiagnosticSeverity::Error => symbols::FAIL,
                };
                let label = format!("{}: {}", finding.check_id, finding.message);
                lines.push(styled::continuation(theme, icon, &label, None));
            }
            lines.push(Line::from(vec![Span::styled(
                symbols::END.to_string(),
                theme.muted(),
            )]));
            term.push_lines(&lines)?;
        }
        "/diff" => {
            let output = shell_output("git", &["diff", "--stat", "--color=never"]);
            if output.trim().is_empty() {
                term.push_lines(&[styled::continuation(theme, "diff", "no changes", None)])?;
            } else {
                push_shell_output(term, theme, "diff", &output, 30)?;
            }
        }
        "/git" => {
            let output = shell_output("git", &["status", "--short"]);
            if output.trim().is_empty() {
                term.push_lines(&[styled::continuation(
                    theme,
                    "git",
                    "clean working tree",
                    None,
                )])?;
            } else {
                push_shell_output(term, theme, "git", &output, 25)?;
            }
        }
        _ if cmd.starts_with("/log") => {
            let arg = cmd.strip_prefix("/log").unwrap().trim();
            let n = arg.parse::<usize>().unwrap_or(5);
            let output = shell_output("git", &["log", &format!("-{n}"), "--oneline", "--decorate"]);
            push_shell_output(term, theme, "log", &output, n + 1)?;
        }
        "/branch" => {
            let output = shell_output("git", &["branch", "-v", "--color=never"]);
            push_shell_output(term, theme, "branch", &output, 15)?;
        }
        "/changes" => {
            let output = shell_output("git", &["diff", "--name-status", "HEAD"]);
            if output.trim().is_empty() {
                term.push_lines(&[styled::continuation(
                    theme,
                    "changes",
                    "no changes since last commit",
                    None,
                )])?;
            } else {
                push_shell_output(term, theme, "changes", &output, 30)?;
            }
        }

        // =================================================================
        // File operations
        // =================================================================
        _ if cmd.starts_with("/file ") => {
            let path = cmd.strip_prefix("/file ").unwrap().trim();
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    push_shell_output(term, theme, &format!("file: {path}"), &content, 40)?;
                }
                Err(e) => {
                    term.push_lines(&[styled::continuation(
                        theme,
                        "error",
                        &format!("{path}: {e}"),
                        None,
                    )])?;
                }
            }
        }
        _ if cmd.starts_with("/search ") => {
            let pattern = cmd.strip_prefix("/search ").unwrap().trim();
            let output = shell_output(
                "grep",
                &[
                    "-rn",
                    "--include=*.rs",
                    "--include=*.toml",
                    "--include=*.md",
                    pattern,
                    ".",
                ],
            );
            if output.trim().is_empty() {
                term.push_lines(&[styled::continuation(
                    theme,
                    "search",
                    &format!("no matches for '{pattern}'"),
                    None,
                )])?;
            } else {
                push_shell_output(term, theme, &format!("search: {pattern}"), &output, 25)?;
            }
        }
        _ if cmd.starts_with("/find ") => {
            let pattern = cmd.strip_prefix("/find ").unwrap().trim();
            let output = shell_output(
                "find",
                &[
                    ".",
                    "-name",
                    pattern,
                    "-not",
                    "-path",
                    "*/target/*",
                    "-not",
                    "-path",
                    "*/.git/*",
                ],
            );
            if output.trim().is_empty() {
                term.push_lines(&[styled::continuation(
                    theme,
                    "find",
                    &format!("no files matching '{pattern}'"),
                    None,
                )])?;
            } else {
                push_shell_output(term, theme, &format!("find: {pattern}"), &output, 25)?;
            }
        }
        _ if cmd.starts_with("/tree") => {
            let arg = cmd.strip_prefix("/tree").unwrap().trim();
            let path = if arg.is_empty() { "." } else { arg };
            let output = shell_output(
                "find",
                &[
                    path,
                    "-maxdepth",
                    "3",
                    "-not",
                    "-path",
                    "*/target/*",
                    "-not",
                    "-path",
                    "*/.git/*",
                ],
            );
            push_shell_output(term, theme, &format!("tree: {path}"), &output, 30)?;
        }

        // =================================================================
        // Agent & workflow
        // =================================================================
        "/agent" | "/agent list" => {
            let agent_id = &session.agent_id;
            term.push_lines(&[
                styled::section_start(theme, "agent", "", None),
                styled::continuation(theme, "current", agent_id, None),
                styled::continuation(
                    theme,
                    "roles",
                    "implementer, strategist, architect, auditor, researcher, scribe, critic",
                    None,
                ),
                styled::section_end(theme, "switch", "/agent <name>"),
            ])?;
        }
        _ if cmd.starts_with("/agent ") => {
            let name = cmd.strip_prefix("/agent ").unwrap().trim();
            if name != "list" {
                session.agent_id = name.to_string();
                let color = Theme::role_accent(name);
                let color_name = if color == Theme::ROSE {
                    "rose"
                } else if color == Theme::DREAM {
                    "dream"
                } else if color == Theme::BONE {
                    "bone"
                } else if color == Theme::SAGE {
                    "sage"
                } else if color == Theme::EMBER {
                    "ember"
                } else {
                    "default"
                };
                term.push_lines(&[styled::continuation(
                    theme,
                    "agent",
                    &format!("switched to {name}"),
                    Some(color_name),
                )])?;
            }
        }
        _ if cmd.starts_with("/run ") => {
            let prompt = cmd.strip_prefix("/run ").unwrap().trim();
            term.push_lines(&[
                styled::section_start(theme, "run", "universal loop", None),
                styled::continuation(theme, "prompt", prompt, None),
                styled::continuation(theme, "command", &format!("roko run \"{prompt}\""), None),
                styled::section_end(theme, "tip", "run this in a terminal for full output"),
            ])?;
        }
        "/plan list" => {
            let output = shell_output(
                "find",
                &[
                    ".roko/plans",
                    "-name",
                    "*.toml",
                    "-not",
                    "-path",
                    "*/target/*",
                ],
            );
            if output.trim().is_empty() {
                let output2 = shell_output("find", &["plans", "-name", "*.toml"]);
                if output2.trim().is_empty() {
                    term.push_lines(&[styled::continuation(
                        theme,
                        "plans",
                        "no plans found",
                        None,
                    )])?;
                } else {
                    push_shell_output(term, theme, "plans", &output2, 20)?;
                }
            } else {
                push_shell_output(term, theme, "plans", &output, 20)?;
            }
        }
        _ if cmd.starts_with("/plan run ") => {
            let dir = cmd.strip_prefix("/plan run ").unwrap().trim();
            term.push_lines(&[styled::continuation(
                theme,
                "plan",
                &format!("roko plan run {dir}"),
                Some("run in terminal"),
            )])?;
        }
        _ if cmd.starts_with("/plan generate ") => {
            let prompt = cmd.strip_prefix("/plan generate ").unwrap().trim();
            term.push_lines(&[styled::continuation(
                theme,
                "plan",
                &format!("roko plan generate \"{prompt}\""),
                Some("run in terminal"),
            )])?;
        }
        "/plan" => {
            term.push_lines(&[
                styled::section_start(theme, "plan", "subcommands", None),
                styled::continuation(theme, "/plan list", "list plans", None),
                styled::continuation(theme, "/plan run <dir>", "execute a plan", None),
                styled::section_end(theme, "/plan generate <prompt>", "generate plan"),
            ])?;
        }

        // =================================================================
        // PRD & research
        // =================================================================
        _ if cmd.starts_with("/prd idea ") => {
            let idea = cmd.strip_prefix("/prd idea ").unwrap().trim();
            term.push_lines(&[styled::continuation(
                theme,
                "prd",
                &format!("roko prd idea \"{idea}\""),
                Some("run in terminal"),
            )])?;
        }
        "/prd list" => {
            let prd_dir = std::path::Path::new(".roko/prd");
            if prd_dir.exists() {
                let output = shell_output("ls", &["-la", ".roko/prd/"]);
                push_shell_output(term, theme, "PRDs", &output, 20)?;
            } else {
                term.push_lines(&[styled::continuation(
                    theme,
                    "prd",
                    "no PRDs found",
                    Some(".roko/prd/ not found"),
                )])?;
            }
        }
        "/prd" => {
            term.push_lines(&[
                styled::section_start(theme, "prd", "subcommands", None),
                styled::continuation(theme, "/prd idea <text>", "capture idea", None),
                styled::continuation(theme, "/prd list", "list PRDs", None),
                styled::section_end(theme, "cli", "roko prd draft|plan|status"),
            ])?;
        }
        _ if cmd.starts_with("/research ") => {
            let query = cmd.strip_prefix("/research ").unwrap().trim();
            term.push_lines(&[styled::continuation(
                theme,
                "research",
                &format!("roko research topic \"{query}\""),
                Some("run in terminal"),
            )])?;
        }

        // =================================================================
        // Knowledge & learning
        // =================================================================
        _ if cmd.starts_with("/knowledge ") => {
            let query = cmd.strip_prefix("/knowledge ").unwrap().trim();
            if query == "stats" {
                let neuro_dir = std::path::Path::new(".roko/neuro");
                if neuro_dir.exists() {
                    let output = shell_output("ls", &["-la", ".roko/neuro/"]);
                    push_shell_output(term, theme, "knowledge", &output, 15)?;
                } else {
                    term.push_lines(&[styled::continuation(
                        theme,
                        "knowledge",
                        "store not initialized",
                        None,
                    )])?;
                }
            } else {
                term.push_lines(&[styled::continuation(
                    theme,
                    "knowledge",
                    &format!("roko knowledge query \"{query}\""),
                    Some("run in terminal"),
                )])?;
            }
        }
        "/knowledge" => {
            term.push_lines(&[
                styled::section_start(theme, "knowledge", "subcommands", None),
                styled::continuation(theme, "/knowledge <query>", "query store", None),
                styled::continuation(theme, "/knowledge stats", "store stats", None),
                styled::section_end(theme, "cli", "roko knowledge query|stats|gc"),
            ])?;
        }
        "/learn" => {
            let learn_dir = std::path::Path::new(".roko/learn");
            if learn_dir.exists() {
                let files: Vec<String> = std::fs::read_dir(learn_dir)
                    .map(|d| {
                        d.filter_map(|e| e.ok())
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                let mut lines = vec![styled::section_start(
                    theme,
                    "learn",
                    "learning state",
                    None,
                )];
                for f in &files {
                    lines.push(styled::continuation(theme, "", f, None));
                }
                lines.push(Line::from(vec![Span::styled(
                    symbols::END.to_string(),
                    theme.muted(),
                )]));
                term.push_lines(&lines)?;
            } else {
                term.push_lines(&[styled::continuation(
                    theme,
                    "learn",
                    "no learning data",
                    Some(".roko/learn/ not found"),
                )])?;
            }
        }

        // =================================================================
        // Export & retry
        // =================================================================
        _ if cmd.starts_with("/export") => {
            let arg = cmd.strip_prefix("/export").unwrap().trim();
            let format = if arg.is_empty() { "markdown" } else { arg };
            if session.conversation.is_empty() {
                term.push_lines(&[styled::continuation(
                    theme,
                    "export",
                    "no messages to export",
                    None,
                )])?;
            } else {
                let exports_dir = std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join(".roko")
                    .join("exports");
                let _ = std::fs::create_dir_all(&exports_dir);
                let ts = chrono::Local::now().format("%Y-%m-%d-%H%M");
                match format {
                    "markdown" | "md" => {
                        let path = exports_dir.join(format!("chat-{ts}.md"));
                        let model_name = current_model_name(session);
                        let mut md = format!(
                            "# Roko Chat — {}\n\n**Model**: {} | **Turns**: {} | **Cost**: ${:.4}\n\n---\n\n",
                            chrono::Local::now().format("%Y-%m-%d %H:%M"),
                            model_name,
                            session.turn_count,
                            session.cost.total_cost,
                        );
                        for msg in &session.conversation {
                            let role = if msg.role == "user" { "User" } else { "Roko" };
                            md.push_str(&format!("## {role}\n\n{}\n\n---\n\n", msg.text));
                        }
                        match std::fs::write(&path, &md) {
                            Ok(()) => term.push_lines(&[styled::continuation(
                                theme,
                                "export",
                                &format!("saved to {}", path.display()),
                                None,
                            )])?,
                            Err(e) => term.push_lines(&[styled::continuation(
                                theme,
                                "error",
                                &format!("export failed: {e}"),
                                None,
                            )])?,
                        }
                    }
                    "json" => {
                        let path = exports_dir.join(format!("chat-{ts}.json"));
                        let messages: Vec<serde_json::Value> = session.conversation.iter()
                            .map(|m| json!({ "role": m.role, "text": m.text, "timestamp": m.timestamp }))
                            .collect();
                        let export = json!({
                            "turns": session.turn_count, "cost": session.cost.total_cost,
                            "tokens_in": session.cost.input_tokens, "tokens_out": session.cost.output_tokens,
                            "messages": messages,
                        });
                        match std::fs::write(
                            &path,
                            serde_json::to_string_pretty(&export).unwrap_or_default(),
                        ) {
                            Ok(()) => term.push_lines(&[styled::continuation(
                                theme,
                                "export",
                                &format!("saved to {}", path.display()),
                                None,
                            )])?,
                            Err(e) => term.push_lines(&[styled::continuation(
                                theme,
                                "error",
                                &format!("export failed: {e}"),
                                None,
                            )])?,
                        }
                    }
                    _ => {
                        term.push_lines(&[styled::continuation(
                            theme,
                            "export",
                            &format!("unknown format: {format}"),
                            Some("use: markdown, json"),
                        )])?;
                    }
                }
            }
        }
        "/retry" => {
            if let Some(ref prompt) = session.last_prompt {
                session.input.buffer = prompt.clone();
                session.input.cursor = session.input.buffer.len();
                term.push_lines(&[styled::continuation(
                    theme,
                    "retry",
                    "loaded last message — press Enter to send",
                    None,
                )])?;
            } else {
                term.push_lines(&[styled::continuation(
                    theme,
                    "retry",
                    "no previous message to retry",
                    None,
                )])?;
            }
        }
        "/clear" => {
            for _ in 0..term.viewport_height() {
                term.push_blank()?;
            }
        }

        "/tools" => {
            use roko_std::tool::builtin::BUILTIN_TOOL_NAMES;

            let names = BUILTIN_TOOL_NAMES.as_slice();
            let mut lines = vec![styled::section_start(
                theme,
                "tools",
                &format!("{} builtin tools", names.len()),
                None,
            )];

            for chunk in names.chunks(4) {
                lines.push(styled::continuation(theme, "tool", &chunk.join("  "), None));
            }

            lines.push(styled::section_end(
                theme,
                "tip",
                "resolved from roko-std builtins",
            ));
            term.push_lines(&lines)?;
        }

        "/mcp" | "/mcp reload" => {
            let reload = cmd == "/mcp reload";
            let candidates = [".roko/mcp.json", ".roko/mcp.toml", "mcp.json"];
            let found: Option<&str> = candidates
                .iter()
                .copied()
                .find(|p| std::path::Path::new(p).exists());

            if let Some(path) = found {
                let mut lines = vec![styled::section_start(
                    theme,
                    "mcp",
                    if reload { "reloaded" } else { "configured" },
                    None,
                )];
                lines.push(styled::continuation(theme, "config", path, None));

                if path.ends_with(".json") {
                    if let Ok(raw) = std::fs::read_to_string(path) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                            if let Some(servers) = val.get("mcpServers").and_then(|s| s.as_object())
                            {
                                for name in servers.keys() {
                                    lines.push(styled::continuation(
                                        theme,
                                        "server",
                                        name,
                                        Some("configured"),
                                    ));
                                }
                            }
                        }
                    }
                }

                lines.push(styled::section_end(
                    theme,
                    "tip",
                    "use --mcp-config in roko.toml to override",
                ));
                term.push_lines(&lines)?;
            } else {
                term.push_lines(&[
                    styled::section_start(theme, "mcp", "not configured", None),
                    styled::continuation(theme, "hint", "set agent.mcp_config in roko.toml", None),
                    styled::section_end(theme, "docs", "see: roko config show"),
                ])?;
            }
        }

        _ => {
            term.push_lines(&[styled::continuation(
                theme,
                "command",
                &format!("command: {cmd}"),
                Some("try /help"),
            )])?;
        }
    }
    Ok(false)
}
