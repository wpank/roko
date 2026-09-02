//! Dense agent status grid widget.
//!
//! Shows one compact cell per active agent with:
//! - Status icon (spinner/check/x/dot)
//! - Agent role
//! - Model name (truncated)
//! - Turn count
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
/// turns, context %, and effort level. Agents are sorted with active
/// first, then idle, then done/failed. Supports compact mode for narrow
/// terminals and virtual scrolling for 30+ agents.
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
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
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
        let header = if compact {
            Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled(format!("{:<10}", "role"), Style::default().fg(theme.muted)),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:>5}", "ctx%"), Style::default().fg(theme.muted)),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:<6}", "state"), Style::default().fg(theme.muted)),
            ])
        } else {
            Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled(format!("{:<12}", "role"), Style::default().fg(theme.muted)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{:<14}", "model"),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{:>5}", "turns"),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{:>5}", "ctx%"), Style::default().fg(theme.muted)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{:<8}", "effort"),
                    Style::default().fg(theme.muted),
                ),
            ])
        };
        lines.push(header);
    }

    let header_height = usize::from(show_header);
    let visible_rows = (inner.height as usize).saturating_sub(header_height);

    // Virtual scrolling: keep the first active agent visible, or start at 0.
    let scroll_start = if entries.len() > visible_rows {
        entries
            .iter()
            .position(|(_, a)| AgentStatus::from(a.status.as_str()).is_active())
            .unwrap_or(0)
            .min(entries.len().saturating_sub(visible_rows))
    } else {
        0
    };

    for (_, agent) in entries.iter().skip(scroll_start).take(visible_rows) {
        let status = AgentStatus::from(agent.status.as_str());
        // Animated spinner for active agents, static icon otherwise.
        let (icon, icon_style) = status_icon_animated(status, theme, &tui_state.atmosphere);

        let role_w = if compact { 10 } else { 12 };
        let role_display = truncate_middle(&agent.label, role_w);
        let role_color = Theme::role_accent(&agent.label);

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

        let turns = activity_row.map_or_else(
            || {
                agent_row
                    .map(|r| r.input_tokens.saturating_add(r.output_tokens) / 4000)
                    .filter(|&t| t > 0)
                    .unwrap_or(0) as usize
            },
            |r| r.turns,
        );

        let total_tokens = activity_row.map_or_else(
            || agent_row.map_or(0, |row| row.input_tokens + row.output_tokens),
            |row| row.tokens_used,
        );
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

        if compact {
            let state_label = match status {
                AgentStatus::Active => "LIVE",
                AgentStatus::Idle => "idle",
                AgentStatus::Done => "done",
                AgentStatus::Failed => "FAIL",
            };
            let state_color = match status {
                AgentStatus::Active => theme.accent,
                AgentStatus::Done => theme.success,
                AgentStatus::Failed => theme.danger,
                AgentStatus::Idle => theme.muted,
            };
            lines.push(Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(icon.clone(), icon_style),
                Span::styled(
                    format!("{:<10}", role_display),
                    Style::default()
                        .fg(role_color)
                        .add_modifier(if status.is_active() {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{ctx_pct:>4.0}%"), Style::default().fg(ctx_color)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{state_label:<6}"),
                    Style::default().fg(state_color),
                ),
            ]));
        } else {
            let effort_label = effort_level_label(tui_state, &agent.id);
            lines.push(Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(icon.clone(), icon_style),
                Span::styled(
                    format!("{:<12}", role_display),
                    Style::default()
                        .fg(role_color)
                        .add_modifier(if status.is_active() {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{:<14}", model_display),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(
                    if turns > 0 {
                        format!("{turns:>5}")
                    } else {
                        "    \u{2014}".to_string()
                    },
                    Style::default().fg(theme.foreground),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(format!("{ctx_pct:>4.0}%"), Style::default().fg(ctx_color)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{effort_label:<8}"),
                    effort_style(effort_label, theme),
                ),
            ]));
        }
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
        let role_color = Theme::role_accent(&agent.label);
        lines.push(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(icon, icon_style),
            Span::styled(
                truncate_middle(&agent.label, 10),
                Style::default().fg(role_color),
            ),
            Span::styled(
                format!(" {}", status.label()),
                Style::default().fg(theme.muted),
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
            Style::default().fg(theme.warning),
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

fn agent_sort_key(status_str: &str) -> u8 {
    match AgentStatus::from(status_str) {
        AgentStatus::Active => 0,
        AgentStatus::Idle => 1,
        AgentStatus::Done => 2,
        AgentStatus::Failed => 3,
    }
}

fn effort_level_label(tui_state: &TuiState, agent_id: &str) -> &'static str {
    let agent_row = tui_state.agents.iter().find(|r| r.id == agent_id);
    if let Some(row) = agent_row {
        let total_tokens = row.input_tokens + row.output_tokens;
        let ctx_limit = row.context_limit.max(1);
        let usage = total_tokens as f64 / ctx_limit as f64;
        if usage >= 0.8 {
            "high"
        } else if usage >= 0.4 {
            "medium"
        } else if total_tokens > 0 {
            "low"
        } else {
            "\u{2014}"
        }
    } else {
        "\u{2014}"
    }
}

fn effort_style(label: &str, theme: &Theme) -> Style {
    match label {
        "high" => Style::default()
            .fg(theme.danger)
            .add_modifier(Modifier::BOLD),
        "medium" => Style::default().fg(theme.warning),
        "low" => Style::default().fg(theme.success),
        _ => theme.muted(),
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
}
