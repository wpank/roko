//! Dense agent status grid widget.
//!
//! Shows one compact cell per active agent with:
//! - Selection cursor
//! - Status icon (spinner/check/x/dot) and label
//! - Agent role (color-coded)
//! - Model name (truncated)
//! - Token counts (input/output, compact)
//! - Cost ($X.XX)
//! - Elapsed time
//! - Context usage percent
//! - Effort level

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

use crate::tui::dashboard::Theme;
use crate::tui::state::{AgentStatus, TuiState, model_context_limit};
use crate::tui::util::truncate_middle;

/// Render the agent status grid.
///
/// Each agent occupies a single line showing status icon, role, model,
/// tokens, cost, elapsed, context %, and effort level. Agents are sorted
/// with active first, then idle, then done, then failed. Supports compact
/// mode for narrow terminals and virtual scrolling for 30+ agents.
pub fn render_agent_status_grid(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let has_errors = tui_state
        .agent_summaries
        .iter()
        .any(|a| AgentStatus::from(a.status.as_str()).is_failed());

    let active_count = tui_state
        .agent_summaries
        .iter()
        .filter(|a| AgentStatus::from(a.status.as_str()).is_active())
        .count();
    let total = tui_state.agent_summaries.len();

    let title = format!(" Agent Grid ({active_count}/{total} active) ");
    let border_style = if has_errors {
        theme.danger()
    } else {
        Theme::unfocused_border_style()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            if active_count > 0 {
                theme.section_header()
            } else {
                theme.muted()
            },
        ))
        .border_style(border_style)
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if tui_state.agent_summaries.is_empty() {
        let empty =
            Paragraph::new(Span::styled("  no agents", theme.muted())).wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    // Build and sort entries: active first, then idle, then done, then failed.
    let mut entries: Vec<(usize, &crate::tui::dashboard::AgentSummary)> =
        tui_state.agent_summaries.iter().enumerate().collect();
    entries.sort_by(|(_, a), (_, b)| {
        agent_sort_key(&a.status)
            .cmp(&agent_sort_key(&b.status))
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });

    // Build activity snapshot for turn/cost data.
    let activity = crate::tui::dashboard::build_agent_activity_snapshot(
        &tui_state.agent_summaries,
        &tui_state.efficiency_events,
    );

    let compact = inner.width < 52;
    let show_header = inner.height > 2;
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Header line -- adapt columns to available width.
    if show_header {
        let hdr = theme.label();
        let header = if compact {
            Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(format!("{:<10}", "role"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:>5}", "ctx%"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:<6}", "state"), hdr),
            ])
        } else {
            Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(format!("{:<12}", "role"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:<14}", "model"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:>11}", "tokens"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:>6}", "cost"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:>6}", "time"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:>5}", "ctx%"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:<8}", "task"), hdr),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:<8}", "status"), hdr),
            ])
        };
        lines.push(header);
    }

    let header_height = usize::from(show_header);
    let footer_height = if compact { 0 } else { 1 };
    let visible_rows = (inner.height as usize)
        .saturating_sub(header_height)
        .saturating_sub(footer_height);

    // Virtual scrolling: keep the selected agent visible, or the first active.
    let scroll_start = if entries.len() > visible_rows {
        // If we have a valid selection, scroll to keep it visible.
        let focus_idx = entries
            .iter()
            .position(|(orig_idx, _)| *orig_idx == tui_state.selected_agent)
            .or_else(|| {
                entries
                    .iter()
                    .position(|(_, a)| AgentStatus::from(a.status.as_str()).is_active())
            })
            .unwrap_or(0);
        focus_idx.min(entries.len().saturating_sub(visible_rows))
    } else {
        0
    };

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Aggregate totals for the summary footer.
    let mut total_cost: f64 = 0.0;
    let mut total_tokens_sum: u64 = 0;

    for (row_num, (orig_idx, agent)) in entries.iter().skip(scroll_start).take(visible_rows).enumerate() {
        let status = AgentStatus::from(agent.status.as_str());
        let is_selected = *orig_idx == tui_state.selected_agent;

        // Animated spinner for active agents, static icon otherwise.
        let (icon, icon_style) = status_icon_animated(status, theme, &tui_state.atmosphere);

        let role_w = if compact { 10 } else { 12 };
        let role_display = truncate_middle(&agent.label, role_w);
        let role_color = grid_role_accent(&agent.label);

        let activity_row = activity
            .as_ref()
            .and_then(|snap| snap.active_agents.iter().find(|r| r.agent_id == agent.id));
        let agent_row = tui_state.agents.iter().find(|row| row.id == agent.id);

        // Fallback chain: activity -> agent_row -> em-dash placeholder.
        let model_raw = activity_row
            .map(|r| r.model.as_str())
            .or_else(|| agent_row.map(|r| r.model.as_str()))
            .unwrap_or("");
        let model_display = if model_raw.is_empty() {
            "\u{2014}".to_string()
        } else {
            truncate_middle(model_raw, 14)
        };
        let model_color = model_name_color(model_raw);

        let _turns = activity_row.map_or_else(
            || {
                agent_row
                    .map(|r| r.input_tokens.saturating_add(r.output_tokens) / 4000)
                    .filter(|&t| t > 0)
                    .unwrap_or(0) as usize
            },
            |r| r.turns,
        );

        // Per-agent input/output tokens.
        let (input_tokens, output_tokens) = agent_row
            .map(|row| (row.input_tokens, row.output_tokens))
            .unwrap_or((0, 0));

        let total_tokens = activity_row.map_or_else(
            || agent_row.map_or(0, |row| row.input_tokens + row.output_tokens),
            |row| row.tokens_used,
        );
        total_tokens_sum += total_tokens;

        let ctx_limit = agent_row
            .map(|row| row.context_limit)
            .filter(|limit| *limit > 0)
            .or_else(|| activity_row.map(|r| model_context_limit(&r.model)))
            .unwrap_or_else(|| model_context_limit(""));
        let ctx_pct = if ctx_limit > 0 {
            (total_tokens as f64 / ctx_limit as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        let ctx_color = if ctx_pct >= 90.0 {
            theme.danger
        } else if ctx_pct >= 70.0 {
            theme.warning
        } else {
            theme.success
        };

        // Cost from activity snapshot.
        let cost_usd = activity_row.map_or(0.0, |r| r.cost_usd);
        total_cost += cost_usd;

        // Elapsed time from agent_row spawn time.
        let elapsed_str = agent_row
            .filter(|row| row.spawned_at_ms > 0)
            .map(|row| format_elapsed_ms(now_ms.saturating_sub(row.spawned_at_ms)))
            .unwrap_or_else(|| "\u{2014}".to_string());

        // Task progress: look up the plan this agent is working on.
        let task_progress_str = agent_row
            .filter(|r| !r.current_plan.is_empty())
            .and_then(|r| {
                tui_state
                    .plans
                    .iter()
                    .find(|p| p.id == r.current_plan)
                    .filter(|p| p.tasks_total > 0)
                    .map(|p| (p.tasks_done, p.tasks_total))
            });

        // Selection cursor.
        let cursor = if is_selected { "\u{25b6} " } else { "  " };
        // Alternating row backgrounds for readability.
        let row_bg = if is_selected {
            Theme::BG_HIGHLIGHT
        } else if row_num % 2 == 1 {
            Theme::BG_RAISED
        } else {
            Theme::BG
        };

        if compact {
            let (state_label, state_color) = status_label_color(status, theme);
            lines.push(Line::from(vec![
                Span::styled(
                    cursor,
                    Style::default()
                        .fg(theme.accent)
                        .bg(row_bg),
                ),
                Span::styled(icon.clone(), icon_style.bg(row_bg)),
                Span::styled(
                    format!("{:<10}", role_display),
                    Style::default()
                        .fg(role_color)
                        .bg(row_bg)
                        .add_modifier(if status.is_active() {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{ctx_pct:>4.0}%"),
                    Style::default().fg(ctx_color).bg(row_bg),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{state_label:<6}"),
                    Style::default().fg(state_color).bg(row_bg),
                ),
            ]));
        } else {
            let (state_label, state_color) = status_label_color(status, theme);
            let tokens_str = format!(
                "{}/{}",
                format_compact(input_tokens),
                format_compact(output_tokens),
            );
            let cost_str = if cost_usd > 0.001 {
                format!("${cost_usd:.2}")
            } else {
                "\u{2014}".to_string()
            };
            // Inline task progress: "3/5 |||.." or em-dash if unavailable.
            let task_col = if let Some((done, total)) = task_progress_str {
                format_task_progress(done, total)
            } else {
                "\u{2014}".to_string()
            };
            let task_color = task_progress_str
                .map(|(done, total)| Theme::semantic_color(done as f64 / total as f64))
                .unwrap_or(theme.muted);
            lines.push(Line::from(vec![
                Span::styled(
                    cursor,
                    Style::default()
                        .fg(theme.accent)
                        .bg(row_bg),
                ),
                Span::styled(icon.clone(), icon_style.bg(row_bg)),
                Span::styled(
                    format!("{:<12}", role_display),
                    Style::default()
                        .fg(role_color)
                        .bg(row_bg)
                        .add_modifier(if status.is_active() {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{:<14}", model_display),
                    Style::default().fg(model_color).bg(row_bg),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{tokens_str:>11}"),
                    Style::default().fg(theme.foreground).bg(row_bg),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{cost_str:>6}"),
                    Style::default()
                        .fg(if cost_usd > 1.0 {
                            theme.warning
                        } else {
                            theme.foreground
                        })
                        .bg(row_bg),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{elapsed_str:>6}"),
                    Style::default().fg(theme.foreground).bg(row_bg),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{ctx_pct:>4.0}%"),
                    Style::default().fg(ctx_color).bg(row_bg),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{task_col:<8}"),
                    Style::default().fg(task_color).bg(row_bg),
                ),
                Span::styled(" ", Style::default().bg(row_bg)),
                Span::styled(
                    format!("{state_label:<8}"),
                    Style::default().fg(state_color).bg(row_bg),
                ),
            ]));
        }
    }

    // Accumulate costs/tokens from agents not in the visible window for the footer.
    for (_, agent) in entries.iter().take(scroll_start).chain(
        entries.iter().skip(scroll_start + visible_rows),
    ) {
        let activity_row = activity
            .as_ref()
            .and_then(|snap| snap.active_agents.iter().find(|r| r.agent_id == agent.id));
        let agent_row = tui_state.agents.iter().find(|row| row.id == agent.id);
        total_cost += activity_row.map_or(0.0, |r| r.cost_usd);
        total_tokens_sum += activity_row.map_or_else(
            || agent_row.map_or(0, |row| row.input_tokens + row.output_tokens),
            |row| row.tokens_used,
        );
    }

    // Overflow indicator with directional hints.
    if entries.len() > visible_rows && lines.len() > header_height {
        let hidden_after = entries.len().saturating_sub(scroll_start + visible_rows);
        if hidden_after > 0 {
            lines.pop();
            lines.push(Line::from(Span::styled(
                format!("  ... +{hidden_after}\u{2193} more"),
                theme.muted(),
            )));
        }
    }

    // Summary footer line (space was reserved by footer_height above).
    if !compact && footer_height > 0 {
        let cost_str = if total_cost > 0.001 {
            format!("${total_cost:.2}")
        } else {
            "$0.00".to_string()
        };
        let tokens_str = format_compact(total_tokens_sum);
        let footer = format!(
            "  {total} agents \u{00b7} {active_count} active \u{00b7} {cost_str} \u{00b7} {tokens_str} tokens"
        );
        lines.push(Line::from(Span::styled(footer, theme.label())));
    }

    frame.render_widget(Paragraph::new(lines), inner);

    // Scrollbar for large agent lists.
    if entries.len() > visible_rows && visible_rows > 0 {
        let sb_area = Rect::new(
            inner.x,
            inner.y + header_height as u16,
            inner.width,
            inner.height.saturating_sub(header_height as u16),
        );
        let mut sb_state = ScrollbarState::new(entries.len()).position(scroll_start);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(theme.accent))
            .track_style(Style::default().fg(Theme::TEXT_PHANTOM))
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, sb_area, &mut sb_state);
    }
}

/// Compact variant for embedding in the F1 dashboard.
pub fn render_agent_status_grid_compact(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let active_count = tui_state
        .agent_summaries
        .iter()
        .filter(|a| AgentStatus::from(a.status.as_str()).is_active())
        .count();
    let total = tui_state.agent_summaries.len();

    let title = format!(" Agents ({active_count}/{total}) ");
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, theme.accent()))
        .border_style(Theme::unfocused_border_style())
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if tui_state.agent_summaries.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("  no agents", theme.muted())),
            inner,
        );
        return;
    }

    let mut entries: Vec<&crate::tui::dashboard::AgentSummary> =
        tui_state.agent_summaries.iter().collect();
    entries.sort_by(|a, b| {
        agent_sort_key(&a.status)
            .cmp(&agent_sort_key(&b.status))
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });

    let mut lines: Vec<Line<'_>> = Vec::new();
    for agent in entries.iter().take(inner.height as usize) {
        let status = AgentStatus::from(agent.status.as_str());
        let (icon, icon_style) = status_icon_animated(status, theme, &tui_state.atmosphere);
        let role_color = grid_role_accent(&agent.label);
        let (state_label, state_color) = status_label_color(status, theme);
        lines.push(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(icon, icon_style),
            Span::styled(
                truncate_middle(&agent.label, 10),
                Style::default().fg(role_color),
            ),
            Span::styled(
                format!(" {state_label}"),
                Style::default().fg(state_color),
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Animated status icon: active agents get a rotating braille spinner driven
/// by the atmosphere clock, giving real-time visual feedback that the agent
/// is working.
fn status_icon_animated(
    status: AgentStatus,
    theme: &Theme,
    atm: &crate::tui::atmosphere::Atmosphere,
) -> (String, Style) {
    match status {
        AgentStatus::Active => {
            let ch = atm.spinner();
            (
                format!("{ch} "),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )
        }
        _ => {
            let (s, style) = status_icon(status, theme);
            (s.to_string(), style)
        }
    }
}

fn status_icon(status: AgentStatus, theme: &Theme) -> (&'static str, Style) {
    match status {
        AgentStatus::Active => (
            "\u{25cf} ", // filled circle -- static fallback
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        AgentStatus::Idle => (
            "\u{25cb} ", // empty circle
            Style::default().fg(theme.muted),
        ),
        AgentStatus::Done => (
            "\u{2713} ", // checkmark
            Style::default().fg(theme.success),
        ),
        AgentStatus::Failed => (
            "\u{2717} ", // cross
            Style::default()
                .fg(theme.danger)
                .add_modifier(Modifier::BOLD),
        ),
    }
}

/// Status label and color for display columns.
fn status_label_color(status: AgentStatus, theme: &Theme) -> (&'static str, ratatui::style::Color) {
    match status {
        AgentStatus::Active => ("\u{25b6} active", theme.warning),
        AgentStatus::Done => ("\u{2713} done", theme.success),
        AgentStatus::Failed => ("\u{2717} failed", theme.danger),
        AgentStatus::Idle => ("\u{25cb} idle", theme.muted),
    }
}

/// Role-specific accent color for the agent grid.
fn grid_role_accent(role: &str) -> ratatui::style::Color {
    let r = role.to_lowercase();
    if r.contains("impl") || r.contains("implement") {
        Theme::SAGE
    } else if r.contains("strat") {
        Theme::DREAM
    } else if r.contains("arch") {
        Theme::LAVENDER
    } else if r.contains("audit") {
        Theme::WARNING
    } else if r.contains("crit") {
        Theme::EMBER
    } else if r.contains("cond") {
        Theme::TEAL
    } else if r.contains("verif") {
        Theme::TEXT_DIM
    } else if r.contains("research") {
        Theme::DREAM
    } else {
        Theme::TEXT_DIM
    }
}

/// Model-family color coding: opus/o1→purple, sonnet/gpt→blue, haiku/mini→green,
/// gemini→teal, others→dim.
fn model_name_color(model: &str) -> ratatui::style::Color {
    let m = model.to_lowercase();
    if m.contains("opus") || m.contains("o1-") || m.contains("o3-") {
        Theme::LAVENDER
    } else if m.contains("sonnet") || m.contains("gpt-4") || m.contains("gpt-5") {
        Theme::DREAM
    } else if m.contains("haiku") || m.contains("mini") || m.contains("flash") {
        Theme::SAGE
    } else if m.contains("gemini") {
        Theme::TEAL
    } else if m.contains("claude") {
        Theme::DREAM
    } else {
        Theme::TEXT_DIM
    }
}

/// Format task progress as a compact inline bar: "3/5 |||.."
fn format_task_progress(done: usize, total: usize) -> String {
    let bar_w = 4usize.min(total);
    let filled = if total > 0 {
        (done * bar_w + total / 2) / total
    } else {
        0
    };
    let empty = bar_w.saturating_sub(filled);
    let bar: String = "\u{2502}".repeat(filled) + &".".repeat(empty);
    format!("{done}/{total}{bar}")
}

fn agent_sort_key(status_str: &str) -> u8 {
    match AgentStatus::from(status_str) {
        AgentStatus::Active => 0,
        AgentStatus::Idle => 1,
        AgentStatus::Done => 2,
        AgentStatus::Failed => 3,
    }
}

/// Format a number in compact form: 0-999 as-is, 1k-999k, 1.0M+.
fn format_compact(n: u64) -> String {
    if n == 0 {
        "-".to_string()
    } else if n < 1_000 {
        format!("{n}")
    } else if n < 10_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else if n < 1_000_000 {
        format!("{}k", n / 1_000)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

/// Format elapsed milliseconds as compact duration string (e.g. `2m30s`, `45s`).
fn format_elapsed_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    if total_secs == 0 {
        return "<1s".to_string();
    }
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{hours}h{mins:02}m")
    } else if mins > 0 {
        format!("{mins}m{secs:02}s")
    } else {
        format!("{secs}s")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::dashboard::AgentSummary;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn make_state_with_agents(agents: Vec<AgentSummary>) -> TuiState {
        let mut state = TuiState::new();
        state.agent_summaries = agents;
        state
    }

    #[test]
    fn agent_status_grid_renders_without_panic() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();

        let agents = vec![
            AgentSummary {
                id: "agent-1".into(),
                label: "implementer".into(),
                plan_id: Some("plan-1".into()),
                status: "running".into(),
            },
            AgentSummary {
                id: "agent-2".into(),
                label: "auditor".into(),
                plan_id: None,
                status: "done".into(),
            },
            AgentSummary {
                id: "agent-3".into(),
                label: "critic".into(),
                plan_id: None,
                status: "failed".into(),
            },
        ];
        let state = make_state_with_agents(agents);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_agent_status_grid(frame, area, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn agent_status_grid_empty() {
        let backend = TestBackend::new(60, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let state = make_state_with_agents(Vec::new());
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_agent_status_grid(frame, area, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn compact_grid_renders_without_panic() {
        let backend = TestBackend::new(40, 6);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();

        let agents = vec![AgentSummary {
            id: "a1".into(),
            label: "strategist".into(),
            plan_id: None,
            status: "active".into(),
        }];
        let state = make_state_with_agents(agents);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_agent_status_grid_compact(frame, area, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn grid_scrolls_with_many_agents() {
        let backend = TestBackend::new(80, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();

        // 40 agents but only 8 rows tall (6 inner) -- must scroll
        let agents: Vec<AgentSummary> = (0..40)
            .map(|i| AgentSummary {
                id: format!("agent-{i:02}"),
                label: if i % 3 == 0 {
                    "implementer".into()
                } else if i % 3 == 1 {
                    "auditor".into()
                } else {
                    "critic".into()
                },
                plan_id: None,
                status: if i < 5 { "running" } else { "done" }.into(),
            })
            .collect();
        let state = make_state_with_agents(agents);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_agent_status_grid(frame, area, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn format_compact_values() {
        assert_eq!(format_compact(0), "-");
        assert_eq!(format_compact(500), "500");
        assert_eq!(format_compact(1_500), "1.5k");
        assert_eq!(format_compact(12_500), "12k");
        assert_eq!(format_compact(999_000), "999k");
        assert_eq!(format_compact(1_500_000), "1.5M");
    }

    #[test]
    fn format_elapsed_values() {
        assert_eq!(format_elapsed_ms(0), "<1s");
        assert_eq!(format_elapsed_ms(500), "<1s");
        assert_eq!(format_elapsed_ms(45_000), "45s");
        assert_eq!(format_elapsed_ms(150_000), "2m30s");
        assert_eq!(format_elapsed_ms(3_661_000), "1h01m");
    }

    #[test]
    fn status_labels_correct() {
        let theme = Theme::dark();
        let (label, color) = status_label_color(AgentStatus::Active, &theme);
        assert!(label.contains("active"));
        assert_eq!(color, theme.warning);

        let (label, color) = status_label_color(AgentStatus::Done, &theme);
        assert!(label.contains("done"));
        assert_eq!(color, theme.success);

        let (label, color) = status_label_color(AgentStatus::Failed, &theme);
        assert!(label.contains("failed"));
        assert_eq!(color, theme.danger);

        let (label, color) = status_label_color(AgentStatus::Idle, &theme);
        assert!(label.contains("idle"));
        assert_eq!(color, theme.muted);
    }

    #[test]
    fn grid_role_colors() {
        assert_eq!(grid_role_accent("implementer"), Theme::SAGE);
        assert_eq!(grid_role_accent("strategist"), Theme::DREAM);
        assert_eq!(grid_role_accent("architect"), Theme::LAVENDER);
        assert_eq!(grid_role_accent("auditor"), Theme::WARNING);
        assert_eq!(grid_role_accent("critic"), Theme::EMBER);
        assert_eq!(grid_role_accent("conductor"), Theme::TEAL);
        assert_eq!(grid_role_accent("verifier"), Theme::TEXT_DIM);
    }

    #[test]
    fn model_name_colors() {
        assert_eq!(model_name_color("claude-opus-4-6"), Theme::LAVENDER);
        assert_eq!(model_name_color("claude-sonnet-4-6"), Theme::DREAM);
        assert_eq!(model_name_color("claude-haiku-4-5"), Theme::SAGE);
        assert_eq!(model_name_color("gpt-4o"), Theme::DREAM);
        assert_eq!(model_name_color("gpt-5.6-sol"), Theme::DREAM);
        assert_eq!(model_name_color("o1-preview"), Theme::LAVENDER);
        assert_eq!(model_name_color("gemini-2.5-flash"), Theme::SAGE);
        assert_eq!(model_name_color("gemini-2.0-pro"), Theme::TEAL);
        assert_eq!(model_name_color("unknown-model"), Theme::TEXT_DIM);
    }

    #[test]
    fn task_progress_formatting() {
        assert_eq!(format_task_progress(0, 5), "0/5....");
        assert_eq!(format_task_progress(5, 5), "5/5\u{2502}\u{2502}\u{2502}\u{2502}");
        assert_eq!(format_task_progress(3, 4), "3/4\u{2502}\u{2502}\u{2502}.");
        assert_eq!(format_task_progress(0, 0), "0/0");
    }
}
