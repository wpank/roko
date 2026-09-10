//! Scrollback output helpers: agent responses, tool outputs, usage lines,
//! error messages with suggestions.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::dispatch_v2::ToolOutput;
use crate::inline::styled;
use crate::inline::symbols;
use crate::inline::terminal::InlineTerminal;
use crate::tui::Theme;

// ---------------------------------------------------------------------------
// Reading time
// ---------------------------------------------------------------------------

/// Estimate reading time for a text response.
pub(crate) fn reading_time(text: &str) -> Option<String> {
    let words = text.split_whitespace().count();
    if words < 100 {
        return None;
    }
    let minutes = words as f64 / 200.0; // avg reading speed
    if minutes < 1.0 {
        Some(format!("~{words} words"))
    } else {
        Some(format!("~{} min read", minutes.ceil() as u32))
    }
}

// ---------------------------------------------------------------------------
// Usage line
// ---------------------------------------------------------------------------

/// Print a compact per-turn usage summary to the inline terminal scrollback.
pub(crate) fn push_usage_line(
    term: &mut InlineTerminal,
    _theme: &Theme,
    model_name: &str,
    input_tokens: u64,
    output_tokens: u64,
    cost: f64,
    latency_s: f64,
) -> std::io::Result<()> {
    let total_tokens = input_tokens + output_tokens;
    let usage_text = if total_tokens == 0 {
        format!("  [{model_name} | usage: unknown | {latency_s:.1}s]")
    } else {
        let cost_text = if cost == 0.0 {
            "free".to_string()
        } else {
            format!("${cost:.4}")
        };
        format!("  [{model_name} | {total_tokens} tokens | {cost_text} | {latency_s:.1}s]")
    };

    term.push_lines(&[Line::from(vec![Span::styled(
        usage_text,
        Style::default().fg(Theme::TEXT_GHOST),
    )])])
}

// ---------------------------------------------------------------------------
// Tool outputs
// ---------------------------------------------------------------------------

/// Render tool execution outputs above the agent response.
/// Each tool output is shown as a collapsed summary with the tool name and
/// a preview of the output content, similar to mori's CommandOutput panel.
pub(crate) fn push_tool_outputs(
    term: &mut InlineTerminal,
    theme: &Theme,
    tool_outputs: &[ToolOutput],
) -> std::io::Result<()> {
    if tool_outputs.is_empty() {
        return Ok(());
    }
    let use_unicode = crate::inline::should_use_inline();
    for output in tool_outputs {
        let tool_label = output.tool_name.as_deref().unwrap_or("tool");

        // Infer success/failure from the tool content.
        let is_error = output.content.is_empty()
            || output.content.starts_with("Error:")
            || output.content.starts_with("error:");

        let line_count = output.content.lines().count();
        let summary = if is_error {
            let first = output.content.lines().next().unwrap_or("failed");
            let mut chars = first.chars();
            let truncated: String = chars.by_ref().take(60).collect();
            if chars.next().is_some() {
                format!("{truncated}...")
            } else {
                truncated
            }
        } else if line_count > 1 {
            format!("{line_count} lines")
        } else if output.content.is_empty() {
            "ok".to_string()
        } else {
            format!("{} bytes", output.content.len())
        };

        let (success_symbol, failure_symbol) = if use_unicode {
            (symbols::PASS, symbols::FAIL)
        } else {
            ("+", "x")
        };
        let (indicator, indicator_style, summary_style) = if is_error {
            (
                failure_symbol,
                Style::default().fg(Theme::EMBER),
                Style::default().fg(Theme::EMBER),
            )
        } else {
            (
                success_symbol,
                Style::default().fg(Theme::SAGE),
                Style::default().fg(Theme::TEXT_DIM),
            )
        };
        term.push_lines(&[Line::from(vec![
            Span::styled(format!("  {indicator} "), indicator_style),
            Span::styled(
                format!("[{tool_label}]"),
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {summary}"), summary_style),
        ])])?;
    }
    term.push_lines(&[Line::raw("")])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Agent response
// ---------------------------------------------------------------------------

pub(crate) fn push_agent_response(
    term: &mut InlineTerminal,
    theme: &Theme,
    text: &str,
    agent_id: &str,
) -> std::io::Result<()> {
    let mut lines = Vec::new();

    // Agent header (with role-specific color and optional reading time)
    let agent_color = Theme::role_accent(agent_id);
    let agent_color = if agent_color == Theme::TEXT_DIM {
        theme.info
    } else {
        agent_color
    };
    let mut header_spans = vec![
        Span::styled(symbols::START.to_string(), Style::default().fg(agent_color)),
        Span::raw(" "),
        Span::styled(
            agent_id.to_string(),
            Style::default()
                .fg(agent_color)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(rt) = reading_time(text) {
        header_spans.push(Span::styled(
            format!("  {}", rt),
            Style::default().fg(Theme::TEXT_GHOST),
        ));
    }
    lines.push(Line::from(header_spans));

    // Response body -- rendered as markdown with bar prefix
    let md_lines = crate::inline::markdown::render_markdown_with_bar(text, theme);
    lines.extend(md_lines);

    // Close
    lines.push(Line::from(vec![Span::styled(
        symbols::END.to_string(),
        theme.muted(),
    )]));

    term.push_lines(&lines)
}

// ---------------------------------------------------------------------------
// Error suggestions
// ---------------------------------------------------------------------------

/// Suggest recovery actions for common error patterns.
pub(crate) fn error_suggestions(err: &str) -> Vec<(&'static str, &'static str)> {
    let err_lower = err.to_lowercase();
    let mut suggestions = Vec::new();

    if err_lower.contains("connection refused") || err_lower.contains("connect error") {
        suggestions.push(("start server", "roko serve"));
        suggestions.push(("check port", "lsof -i :6677"));
        suggestions.push(("use direct mode", "roko (no subcommand)"));
    } else if err_lower.contains("unauthorized")
        || err_lower.contains("401")
        || err_lower.contains("invalid api key")
        || err_lower.contains("authentication")
    {
        suggestions.push(("check key", "echo $ANTHROPIC_API_KEY | head -c 10"));
        suggestions.push(("set key", "export ANTHROPIC_API_KEY=sk-ant-..."));
    } else if err_lower.contains("rate limit")
        || err_lower.contains("429")
        || err_lower.contains("too many requests")
    {
        suggestions.push(("wait & retry", "try again in 30 seconds"));
        suggestions.push(("switch model", "/model claude-haiku-4-5-20251001"));
    } else if err_lower.contains("timeout") || err_lower.contains("timed out") {
        suggestions.push(("retry", "press Enter to resend"));
        suggestions.push(("switch model", "/model claude-haiku-4-5-20251001"));
    } else if err_lower.contains("model") && err_lower.contains("not found") {
        suggestions.push(("list models", "/model"));
        suggestions.push(("use default", "/model claude-sonnet-4-6"));
    } else if err_lower.contains("context")
        && (err_lower.contains("length") || err_lower.contains("too long"))
    {
        suggestions.push(("clear context", "/clear"));
        suggestions.push(("start fresh", "exit and restart"));
    }

    suggestions
}

/// Push an error with contextual recovery suggestions.
pub(crate) fn push_error_with_suggestions(
    term: &mut InlineTerminal,
    theme: &Theme,
    err: &str,
) -> std::io::Result<()> {
    let mut lines = vec![styled::continuation(theme, "error", err, None)];

    let suggestions = error_suggestions(err);
    if !suggestions.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            symbols::BAR.to_string(),
            theme.muted(),
        )]));
        for (label, cmd) in &suggestions {
            lines.push(Line::from(vec![
                Span::styled(format!("{} ", symbols::BAR), theme.muted()),
                Span::styled(format!("  {label}: "), Style::default().fg(Theme::WARNING)),
                Span::styled(cmd.to_string(), Style::default().fg(Theme::BONE)),
            ]));
        }
    }

    term.push_lines(&lines)
}
