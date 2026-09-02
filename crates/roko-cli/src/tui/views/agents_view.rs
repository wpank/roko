//! F3 Agents view -- Mori-style agent roster + output panel.
//!
//! Layout: left 32% (agent roster, summary line, token sparkline), a
//! one-cell VOID gutter, and right detail (role tabs + agent output).
//!
//! Renders rich gradient progress bars, context gauges, role-colored
//! tabs, and status chips matching the Mori Agents screen (F3).

use std::collections::HashMap;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::input::FocusZone;
use crate::tui::state::{AgentStatus, AgentTopologyStatus, TuiState, model_context_limit};
use crate::tui::util::truncate_middle;

// ---------------------------------------------------------------------------
// Role tab labels (fixed order, matching Mori)
// ---------------------------------------------------------------------------

pub(crate) const ROLE_TABS: &[(&str, &str)] = &[
    ("implementer", "1:impl"),
    ("strategist", "2:strat"),
    ("architect", "3:arch"),
    ("auditor", "4:audit"),
    ("critic", "5:crit"),
    ("conductor", "6:cond"),
    ("researcher", "7:res"),
];

// ---------------------------------------------------------------------------
// Public render
// ---------------------------------------------------------------------------

/// Render the full agents view.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    // Mori's master/detail split works well once both panes have enough room.
    // Below that point, stacking keeps the transcript wide enough to read and
    // gives the roster a short, useful overview instead of two clipped panes.
    if area.width < 104 {
        let roster_height = (area.height / 3)
            .clamp(4, 9)
            .min(area.height.saturating_sub(5).max(1));
        let panels = Layout::vertical([
            Constraint::Length(roster_height),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);
        render_left_panel(frame, panels[0], data, tui_state, view_state, theme);
        render_right_panel(frame, panels[2], tui_state, view_state, theme);
    } else {
        let panels = Layout::horizontal([
            Constraint::Percentage(32),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);
        render_left_panel(frame, panels[0], data, tui_state, view_state, theme);
        render_right_panel(frame, panels[2], tui_state, view_state, theme);
    }
}

// ---------------------------------------------------------------------------
// Left panel: agent roster + summary + token sparkline
// ---------------------------------------------------------------------------

fn render_left_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let has_token_data = tui_state.efficiency_summary.event_count > 0
        || tui_state.cumulative_input_tokens > 0
        || tui_state.cumulative_output_tokens > 0
        || !tui_state.efficiency_events.is_empty();
    // On short panes the sparkline merely repeats the summary and displaces the
    // selectable roster. Preserve it when it has room to be informative.
    let sparkline_height = if has_token_data && area.height >= 18 {
        6
    } else {
        0
    };
    let summary_height = u16::from(area.height >= 5);

    let sections = Layout::vertical([
        Constraint::Min(3),                   // agent roster (flexible)
        Constraint::Length(summary_height),   // compact summary line
        Constraint::Length(sparkline_height), // token sparkline
    ])
    .split(area);

    render_agent_roster(frame, sections[0], tui_state, view_state, theme);
    render_summary_line(frame, sections[1], tui_state, theme);
    if sparkline_height > 0 {
        crate::tui::widgets::token_sparkline::render_token_sparkline(
            frame,
            sections[2],
            data,
            tui_state,
        );
    }
}

// ---------------------------------------------------------------------------
// Agent roster
// ---------------------------------------------------------------------------

fn render_agent_roster(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let focused = matches!(tui_state.focus, FocusZone::PlanTree);
    let mut agents: Vec<(usize, &crate::tui::dashboard::AgentSummary)> =
        tui_state.agent_summaries.iter().enumerate().collect();
    agents.sort_by(|(idx_a, a), (idx_b, b)| {
        agent_status_rank(&a.status)
            .cmp(&agent_status_rank(&b.status))
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| idx_a.cmp(idx_b))
    });

    let active_count = agents
        .iter()
        .filter(|a| AgentStatus::from(a.1.status.as_str()).is_active())
        .count();
    let title = format!(" Agents · {active_count}/{} active ", agents.len());

    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::unfocused_border_style()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else if active_count > 0 {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Theme::unfocused_title_style()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, title_style))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if agents.is_empty() {
        let empty_lines = if inner.height >= 5 {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No agents online",
                    Style::default()
                        .fg(theme.muted)
                        .add_modifier(Modifier::ITALIC),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Agents appear when plans execute or when",
                    Style::default().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    "started with: roko agent start --name <id>",
                    Style::default().fg(theme.muted),
                )),
            ]
        } else {
            vec![Line::from(Span::styled(
                "No agents online \u{2014} start a plan or roko agent start",
                Style::default().fg(theme.muted),
            ))]
        };
        let empty = Paragraph::new(empty_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    let content_width = inner.width as usize;
    let activity = crate::tui::dashboard::build_agent_activity_snapshot(
        &tui_state.agent_summaries,
        &tui_state.efficiency_events,
    );

    let density = RosterDensity::for_width(inner.width);
    let show_header = inner.height >= 3;
    let mut rows: Vec<(usize, Line<'_>)> = Vec::with_capacity(agents.len());

    for (idx, agent) in agents {
        let is_selected = idx == view_state.selected;
        let status = AgentStatus::from(agent.status.as_str());
        let is_active = status.is_active();
        let is_done = status.is_done();
        let is_failed = status.is_failed();

        // Status icon
        let (icon, icon_style) = if is_active {
            (
                "\u{25b6}", // ▶
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_done {
            ("\u{2713}", Style::default().fg(theme.success)) // ✓
        } else if is_failed {
            (
                "\u{2717}", // ✗
                Style::default()
                    .fg(theme.danger)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("\u{25cb}", Style::default().fg(theme.muted)) // ○
        };

        // Role accent color
        let accent = role_accent(&agent.label, theme);
        let bg = if is_selected {
            theme.selection_background
        } else {
            Color::Reset
        };
        let cursor = if is_selected { "\u{25b6}" } else { " " };
        let activity_row = activity
            .as_ref()
            .and_then(|snap| snap.active_agents.iter().find(|r| r.agent_id == agent.id));
        let agent_row = tui_state.agents.iter().find(|row| row.id == agent.id);
        let model = display_model(
            activity_row
                .map(|r| r.model.as_str())
                .or_else(|| agent_row.map(|row| row.model.as_str())),
        );
        let task = activity_row
            .map(|r| r.task.clone())
            .or_else(|| {
                agent_row
                    .map(|row| row.current_task.clone())
                    .filter(|task| !task.is_empty())
            })
            .or_else(|| agent.plan_id.as_deref().map(ToOwned::to_owned))
            .unwrap_or_else(|| "-".to_string());
        let total_tokens = activity_row.map_or_else(
            || agent_row.map_or(0, |row| row.input_tokens + row.output_tokens),
            |row| row.tokens_used,
        );
        let tokens_str = format_tokens(total_tokens);

        // Context gauge — use tokens against the model's context window
        let ctx_limit = tui_state
            .agents
            .iter()
            .find(|row| row.id == agent.id)
            .map(|row| row.context_limit)
            .filter(|limit| *limit > 0)
            .or_else(|| activity_row.map(|row| model_context_limit(&row.model)))
            .unwrap_or_else(|| model_context_limit(""));
        let fill_pct = (total_tokens as f64 / ctx_limit as f64).clamp(0.0, 1.0);
        let state_label = if is_active {
            "RUN"
        } else if is_done {
            "DONE"
        } else if is_failed {
            "FAIL"
        } else {
            "IDLE"
        };
        let status_color = if is_active {
            accent
        } else if is_done {
            theme.success
        } else if is_failed {
            theme.danger
        } else {
            Color::Reset
        };

        let attempt = agent_row.map_or(0, |r| r.attempt);
        let row = roster_row(RosterRow {
            width: content_width,
            density,
            cursor,
            icon,
            icon_style,
            id: &agent.id,
            model: &model,
            state_label,
            status_color,
            task: &task,
            tokens: &tokens_str,
            context_pct: (fill_pct * 100.0).round() as u64,
            accent,
            background: bg,
            theme,
            active: is_active,
            attempt,
        });
        rows.push((idx, row));
    }

    let header_height = usize::from(show_header);
    let capacity = (inner.height as usize).saturating_sub(header_height).max(1);
    let total_rows = rows.len();
    let selected_pos = rows
        .iter()
        .position(|(idx, _)| *idx == view_state.selected)
        .unwrap_or(0);
    let max_start = total_rows.saturating_sub(capacity);
    let start = selected_pos
        .saturating_sub(capacity.saturating_sub(1))
        .min(max_start);
    let mut lines = Vec::with_capacity(capacity + header_height);
    if show_header {
        lines.push(roster_header(content_width, density, theme));
    }
    lines.extend(
        rows.into_iter()
            .skip(start)
            .take(capacity)
            .map(|(_, line)| line),
    );
    frame.render_widget(Paragraph::new(lines), inner);

    // Scrollbar when roster overflows.
    if total_rows > capacity && capacity > 0 {
        let sb_area = Rect::new(
            inner.x,
            inner.y + header_height as u16,
            inner.width,
            inner.height.saturating_sub(header_height as u16),
        );
        let mut sb_state = ScrollbarState::new(total_rows).position(start);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(theme.accent))
            .track_style(Style::default().fg(Theme::TEXT_PHANTOM))
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, sb_area, &mut sb_state);
    }
}

#[derive(Debug, Clone, Copy)]
enum RosterDensity {
    Compact,
    Standard,
    Wide,
}

impl RosterDensity {
    const fn for_width(width: u16) -> Self {
        if width < 42 {
            Self::Compact
        } else if width < 65 {
            Self::Standard
        } else {
            Self::Wide
        }
    }
}

struct RosterRow<'a> {
    width: usize,
    density: RosterDensity,
    cursor: &'a str,
    icon: &'a str,
    icon_style: Style,
    id: &'a str,
    model: &'a str,
    state_label: &'a str,
    status_color: Color,
    task: &'a str,
    tokens: &'a str,
    context_pct: u64,
    accent: Color,
    background: Color,
    theme: &'a Theme,
    active: bool,
    attempt: u32,
}

fn roster_row(row: RosterRow<'_>) -> Line<'static> {
    let id_style = Style::default()
        .fg(row.accent)
        .bg(row.background)
        .add_modifier(if row.active {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    let status_style = Style::default()
        .fg(if row.status_color == Color::Reset {
            row.theme.muted
        } else {
            Theme::VOID
        })
        .bg(row.status_color)
        .add_modifier(Modifier::BOLD);
    let base = vec![
        Span::styled(
            row.cursor.to_string(),
            Style::default().fg(row.theme.accent).bg(row.background),
        ),
        Span::styled(row.icon.to_string(), row.icon_style.bg(row.background)),
        Span::styled(" ", Style::default().bg(row.background)),
    ];

    // Attempt badge: shown as "R2", "R3", etc. when attempt > 1.
    let attempt_badge = if row.attempt > 1 {
        format!("R{}", row.attempt)
    } else {
        String::new()
    };
    let attempt_style = Style::default()
        .fg(row.theme.warning)
        .bg(row.background)
        .add_modifier(Modifier::BOLD);

    let mut spans = base;
    match row.density {
        RosterDensity::Compact => {
            let id_w = (row.width / 2).clamp(8, 16);
            let task_w = row.width.saturating_sub(id_w + 10).max(1);
            spans.extend([
                Span::styled(
                    format!("{:<id_w$}", truncate_middle(row.id, id_w)),
                    id_style,
                ),
                Span::styled(format!(" {:<4} ", row.state_label), status_style),
            ]);
            if !attempt_badge.is_empty() {
                let remaining_w = task_w.saturating_sub(attempt_badge.len() + 1);
                spans.extend([
                    Span::styled(attempt_badge.clone(), attempt_style),
                    Span::styled(
                        format!(" {}", truncate_middle(row.task, remaining_w)),
                        Style::default().fg(row.theme.foreground).bg(row.background),
                    ),
                ]);
            } else {
                spans.push(Span::styled(
                    truncate_middle(row.task, task_w),
                    Style::default().fg(row.theme.foreground).bg(row.background),
                ));
            }
        }
        RosterDensity::Standard => {
            let id_w = 12.min(row.width / 3);
            let attempt_extra = if attempt_badge.is_empty() { 0 } else { 3 };
            let task_w = row
                .width
                .saturating_sub(id_w + 20 + attempt_extra)
                .max(1);
            spans.extend([
                Span::styled(
                    format!("{:<id_w$}", truncate_middle(row.id, id_w)),
                    id_style,
                ),
                Span::styled(format!(" {:<4} ", row.state_label), status_style),
            ]);
            if !attempt_badge.is_empty() {
                spans.push(Span::styled(
                    format!("{attempt_badge} "),
                    attempt_style,
                ));
            }
            spans.extend([
                Span::styled(
                    format!(" {:<task_w$}", truncate_middle(row.task, task_w)),
                    Style::default().fg(row.theme.foreground).bg(row.background),
                ),
                Span::styled(
                    format!(" {:>6}", row.tokens),
                    Style::default().fg(row.theme.muted).bg(row.background),
                ),
            ]);
        }
        RosterDensity::Wide => {
            let id_w = 14;
            let model_w = 10;
            let attempt_extra = if attempt_badge.is_empty() { 0 } else { 3 };
            let task_w = row
                .width
                .saturating_sub(53 + attempt_extra)
                .max(8);
            spans.extend([
                Span::styled(
                    format!("{:<id_w$}", truncate_middle(row.id, id_w)),
                    id_style,
                ),
                Span::styled(
                    format!(" {:<model_w$}", truncate_middle(row.model, model_w)),
                    Style::default().fg(row.theme.muted).bg(row.background),
                ),
                Span::styled(format!(" {:<4} ", row.state_label), status_style),
            ]);
            if !attempt_badge.is_empty() {
                spans.push(Span::styled(
                    format!("{attempt_badge} "),
                    attempt_style,
                ));
            }
            spans.extend([
                Span::styled(
                    format!(" {:<task_w$}", truncate_middle(row.task, task_w)),
                    Style::default().fg(row.theme.foreground).bg(row.background),
                ),
                Span::styled(
                    format!(" {:>6}", row.tokens),
                    Style::default().fg(row.theme.foreground).bg(row.background),
                ),
                Span::styled(
                    format!(" {:>3}%", row.context_pct),
                    Style::default().fg(row.theme.muted).bg(row.background),
                ),
            ]);
        }
    }
    Line::from(spans)
}

fn roster_header(width: usize, density: RosterDensity, theme: &Theme) -> Line<'static> {
    let label = match density {
        RosterDensity::Compact => "   agent            state  task",
        RosterDensity::Standard => "   agent        state  task             tokens",
        RosterDensity::Wide => {
            "   agent          model      state  task                 tokens ctx"
        }
    };
    Line::from(Span::styled(
        truncate_middle(label, width),
        Style::default().fg(theme.muted),
    ))
}

// ---------------------------------------------------------------------------
// Summary line
// ---------------------------------------------------------------------------

fn render_summary_line(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let active_count = tui_state
        .agent_summaries
        .iter()
        .filter(|a| AgentStatus::from(a.status.as_str()).is_active())
        .count();
    let total_agents = tui_state.agent_summaries.len();
    let total_tokens = tui_state.cumulative_input_tokens + tui_state.cumulative_output_tokens;
    let cost = tui_state.cost_dollars;

    let line1 = Line::from(vec![
        Span::styled(" agents: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{active_count}/{total_agents}"),
            Style::default()
                .fg(if active_count > 0 {
                    theme.accent
                } else {
                    theme.foreground
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  tokens: ", Style::default().fg(theme.muted)),
        Span::styled(
            format_tokens(total_tokens),
            Style::default().fg(theme.foreground),
        ),
        Span::styled("  cost: ", Style::default().fg(theme.muted)),
        Span::styled(
            if cost > 0.001 {
                format!("${:.2}", cost)
            } else {
                "-".to_string()
            },
            Style::default().fg(if cost > 1.0 {
                theme.warning
            } else {
                theme.foreground
            }),
        ),
    ]);

    let para = Paragraph::new(line1);
    frame.render_widget(para, area);
}

// ---------------------------------------------------------------------------
// Right panel: role tabs + agent output
// ---------------------------------------------------------------------------

fn render_right_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);

    // -- Tab bar --
    render_role_tabs(frame, layout[0], tui_state, view_state, theme);

    // -- Output body --
    render_output_body(frame, layout[1], tui_state, view_state, theme);
}

// ---------------------------------------------------------------------------
// Role tabs
// ---------------------------------------------------------------------------

fn render_role_tabs(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    // Determine which roles have active agents
    let agent_roles: Vec<&str> = tui_state
        .agent_summaries
        .iter()
        .map(|a| a.label.as_str())
        .collect();

    // Selected role from sub_tab
    let selected_role = ROLE_TABS
        .get(view_state.sub_tab.min(ROLE_TABS.len().saturating_sub(1)))
        .map(|(role, _)| *role)
        .unwrap_or("");

    let bg = Theme::BG_RAISED;
    let mut spans: Vec<Span<'_>> = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(bg)));

    let compact = area.width < 64;
    let visible_tabs = ROLE_TABS.iter().filter(|(role, _)| {
        !compact
            || *role == selected_role
            || agent_roles.iter().any(|agent_role| agent_role == role)
    });
    let mut visible_count = 0usize;
    for &(role, label) in visible_tabs {
        visible_count += 1;
        let is_active = role == selected_role;
        let has_agent = agent_roles.iter().any(|r| *r == role);

        let accent = role_accent(role, theme);
        let style = if is_active {
            Style::default()
                .fg(Theme::VOID)
                .bg(accent)
                .add_modifier(Modifier::BOLD)
        } else if has_agent {
            Style::default().fg(accent).bg(bg)
        } else {
            Style::default().fg(theme.muted).bg(bg)
        };

        spans.push(Span::styled(format!(" {label} "), style));
        spans.push(Span::styled(" ", Style::default().bg(bg)));
    }

    if compact && visible_count < ROLE_TABS.len() {
        spans.push(Span::styled(
            " 1-7:roles ",
            Style::default().fg(theme.muted).bg(bg),
        ));
    }

    let used: usize = spans.iter().map(|span| span.content.chars().count()).sum();
    spans.push(Span::styled(
        " ".repeat((area.width as usize).saturating_sub(used)),
        Style::default().bg(bg),
    ));

    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, area);
}

// ---------------------------------------------------------------------------
// Output body
// ---------------------------------------------------------------------------

fn render_output_body(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    if tui_state.agent_topology_visible {
        render_agent_topology_panel(frame, area, tui_state, theme);
        return;
    }

    let selected_agent = tui_state.agent_summaries.get(view_state.selected);
    let selected_row = selected_agent
        .and_then(|agent| tui_state.agents.iter().find(|row| row.id == agent.id))
        .or_else(|| tui_state.agents.get(view_state.selected));
    let selected_id = selected_agent
        .map(|agent| agent.id.as_str())
        .or_else(|| selected_row.map(|row| row.id.as_str()))
        .unwrap_or("");
    let selected_status = selected_agent
        .map(|agent| agent.status.as_str())
        .or_else(|| selected_row.map(|row| row.status.label()))
        .unwrap_or("idle");
    let selected_role = selected_agent
        .map(|agent| agent.label.as_str())
        .or_else(|| selected_row.map(|row| row.role.as_str()))
        .unwrap_or("");
    let accent = role_accent(selected_role, theme);
    let focused = matches!(
        tui_state.focus,
        FocusZone::AgentOutput | FocusZone::RightPanel
    );

    // P7.4: Append attempt info to the title when available.
    let attempt_suffix = selected_row
        .filter(|row| row.attempt > 0)
        .map(|row| format!(" (attempt {})", row.attempt))
        .unwrap_or_default();

    let title_label = if selected_id.is_empty() {
        "Agent Output".to_string()
    } else if area.width < 72 {
        format!("Output · {}", truncate_middle(selected_id, 24))
    } else {
        format!(
            "Output \u{00b7} {} \u{00b7} {}{}",
            selected_id, selected_status, attempt_suffix
        )
    };

    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::unfocused_border_style()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else if selected_agent.is_some_and(|a| AgentStatus::from(a.status.as_str()).is_active()) {
        Style::default().fg(accent).add_modifier(Modifier::BOLD)
    } else {
        Theme::unfocused_title_style()
    };

    let collected = collect_agent_output_lines(tui_state, view_state.selected);
    let output_lines = if collected.is_empty() {
        Vec::new()
    } else {
        tui_state.render_agent_output_lines(selected_id, &collected, theme)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // The main transcript is the dominant surface. A second stream pane is
    // useful only when a sidecar stream actually exists; runner-mode output is
    // already present above and rendering it twice wastes the most valuable
    // part of the screen.
    let show_stream_panel = inner.height >= 24
        && inner.width >= 72
        && !selected_id.is_empty()
        && tui_state.agent_streams.contains_key(selected_id);
    let layout = if show_stream_panel {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(7),
        ])
        .split(inner)
    } else {
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner)
    };
    let output_area = layout[1];
    let stream_area = if show_stream_panel {
        Some(layout[2])
    } else {
        None
    };

    render_route_metrics_bar(frame, layout[0], tui_state, view_state, theme);

    if output_area.width == 0 || output_area.height == 0 {
        return;
    }

    let visible_height = output_area.height as usize;
    let output_paragraph = Paragraph::new(output_lines.clone())
        .style(theme.text())
        .wrap(Wrap { trim: false });
    // Paragraph::scroll is expressed in rendered rows, not source lines.
    // Counting wrapped rows is what keeps the live tail on the newest text.
    let total_lines = output_paragraph.line_count(output_area.width);
    let max_scroll = total_lines
        .saturating_sub(visible_height)
        .min(u16::MAX as usize);
    let scroll = tui_state.agent_scroll.unwrap_or(max_scroll).min(max_scroll);
    let is_following = tui_state.agent_scroll.is_none();
    let is_agent_active = selected_agent
        .is_some_and(|a| AgentStatus::from(a.status.as_str()).is_active());
    let tail_indicator = if is_following {
        if is_agent_active {
            "\u{25cf} FOLLOWING".to_string()
        } else {
            "TAIL".to_string()
        }
    } else {
        format!("PINNED line {}", scroll.saturating_add(1))
    };
    let tail_style = if is_following {
        if is_agent_active {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.success)
        }
    } else {
        Style::default().fg(theme.warning)
    };
    let block = block.border_style(border_style).title(vec![
        Span::styled(format!(" {title_label}"), title_style),
        Span::styled(format!(" [{tail_indicator}] "), tail_style),
    ]);
    frame.render_widget(block, area);

    if output_lines.is_empty() {
        // Centered empty state
        let v_pad = output_area.height / 2;
        let mut empty_lines: Vec<Line<'_>> = Vec::new();
        for _ in 0..v_pad.saturating_sub(2) {
            empty_lines.push(Line::from(""));
        }
        empty_lines.push(Line::from(Span::styled(
            "waiting for agent output...",
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::ITALIC),
        )));
        empty_lines.push(Line::from(""));
        empty_lines.push(Line::from(Span::styled(
            "output will stream here when agents are active",
            Style::default().fg(theme.muted),
        )));
        let empty = Paragraph::new(empty_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, output_area);
    } else {
        let paragraph = output_paragraph.scroll((scroll as u16, 0));
        frame.render_widget(paragraph, output_area);

        // Scrollbar when content overflows.
        if total_lines > visible_height && visible_height > 0 {
            let mut sb_state = ScrollbarState::new(total_lines).position(scroll);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(Theme::ROSE))
                .track_style(Style::default().fg(Theme::TEXT_PHANTOM))
                .begin_symbol(None)
                .end_symbol(None);
            frame.render_stateful_widget(scrollbar, output_area, &mut sb_state);
        }
    }

    if let Some(stream_area) = stream_area {
        render_live_stream_panel(frame, stream_area, selected_id, tui_state, theme);
    }
}

fn render_agent_topology_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let focused = matches!(
        tui_state.focus,
        FocusZone::AgentOutput | FocusZone::RightPanel
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focused {
            Theme::focused_border_style()
        } else {
            theme.muted()
        })
        .title(Span::styled(
            " Agent Topology ",
            if focused {
                Theme::focused_title_style()
            } else {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            },
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let sections = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);
    let status_text = topology_status_text(tui_state);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(status_text, Style::default().fg(theme.muted)),
        ])),
        sections[0],
    );

    let body_lines = agent_topology_lines(tui_state)
        .into_iter()
        .map(Line::from)
        .collect::<Vec<_>>();
    let viewport_height = sections[1].height as usize;
    let max_scroll = body_lines
        .len()
        .saturating_sub(viewport_height)
        .min(u16::MAX as usize);
    let scroll = tui_state.agent_topology_scroll_offset.min(max_scroll);
    frame.render_widget(
        Paragraph::new(body_lines)
            .style(theme.text())
            .wrap(Wrap { trim: false })
            .scroll((scroll as u16, 0)),
        sections[1],
    );
}

fn topology_status_text(tui_state: &TuiState) -> String {
    match &tui_state.agent_topology_status {
        AgentTopologyStatus::Idle => "press Ctrl+T to load topology".to_string(),
        AgentTopologyStatus::Loading => "loading topology...".to_string(),
        AgentTopologyStatus::Ready => format!(
            "{} nodes · {} edges · Ctrl+T closes",
            tui_state.agent_topology.nodes.len(),
            tui_state.agent_topology.edges.len()
        ),
        AgentTopologyStatus::Unavailable => {
            "topology not available from this roko serve".to_string()
        }
        AgentTopologyStatus::Error(message) => {
            format!("topology fetch failed · {}", truncate_middle(message, 48))
        }
    }
}

pub(crate) fn agent_topology_lines(tui_state: &TuiState) -> Vec<String> {
    match &tui_state.agent_topology_status {
        AgentTopologyStatus::Idle => vec!["topology not loaded yet".to_string()],
        AgentTopologyStatus::Loading => vec!["loading topology...".to_string()],
        AgentTopologyStatus::Unavailable => {
            vec!["topology not available from this roko serve".to_string()]
        }
        AgentTopologyStatus::Error(message) => vec![
            "topology fetch failed".to_string(),
            truncate_middle(message, 72),
        ],
        AgentTopologyStatus::Ready => build_agent_topology_lines(tui_state),
    }
}

fn build_agent_topology_lines(tui_state: &TuiState) -> Vec<String> {
    if tui_state.agent_topology.nodes.is_empty() {
        return vec!["no topology nodes reported".to_string()];
    }

    let mut tasks_by_agent: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for task in &tui_state.active_task_summaries {
        for agent_id in &task.assigned_agents {
            tasks_by_agent
                .entry(agent_id.clone())
                .or_default()
                .push((task.task_id.clone(), task.status.clone()));
        }
    }
    for tasks in tasks_by_agent.values_mut() {
        tasks.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    }

    let mut nodes = tui_state.agent_topology.nodes.clone();
    nodes.sort_by(|lhs, rhs| lhs.id.cmp(&rhs.id));

    let mut lines = vec!["└── pool: default".to_string()];
    for (idx, node) in nodes.iter().enumerate() {
        let is_last_node = idx + 1 == nodes.len();
        let node_branch = if is_last_node {
            "    └──"
        } else {
            "    ├──"
        };
        let child_prefix = if is_last_node {
            "        "
        } else {
            "    │   "
        };
        let status = tui_state
            .agents
            .iter()
            .find(|agent| agent.id == node.id)
            .map(|agent| agent.status.label())
            .unwrap_or("idle");
        lines.push(format!("{node_branch} {} [{}]", node.id, status));

        let mut children = Vec::new();
        if let Some(tasks) = tasks_by_agent.get(&node.id) {
            for (task_idx, (task_id, task_status)) in tasks.iter().enumerate() {
                let branch = if task_idx + 1 == tasks.len() && node.address.is_empty() {
                    "└──"
                } else {
                    "├──"
                };
                children.push(format!(
                    "{child_prefix}{branch} task: {} ({})",
                    truncate_middle(task_id, 36),
                    task_status
                ));
            }
        }
        if !node.address.is_empty() {
            children.push(format!(
                "{child_prefix}└── addr: {}",
                truncate_middle(&node.address, 42)
            ));
        }
        if children.is_empty() {
            children.push(format!("{child_prefix}└── no active tasks"));
        }
        lines.extend(children);
    }

    lines
}

fn render_live_stream_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    agent_id: &str,
    tui_state: &TuiState,
    theme: &Theme,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let stream = (!agent_id.is_empty())
        .then(|| tui_state.agent_streams.get(agent_id))
        .flatten();
    let has_runner_output = stream.is_none()
        && !agent_id.is_empty()
        && !collect_agent_output_lines(tui_state, tui_state.selected_agent).is_empty();
    let (status_label, title_style) = match stream {
        Some(stream) if stream.connected => (
            "connected",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Some(stream) if stream.completed => ("done", Style::default().fg(theme.success)),
        Some(_) => ("connecting...", Style::default().fg(theme.warning)),
        None if has_runner_output => ("output (from runner)", Style::default().fg(theme.accent)),
        None => ("no stream", Style::default().fg(theme.muted)),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::unfocused_border_style())
        .title(vec![
            Span::styled(" Live Stream ", title_style),
            Span::styled(
                format!(" {status_label} "),
                Style::default().fg(theme.muted),
            ),
        ]);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Determine whether the stream is actively receiving.
    let is_streaming = stream.is_some_and(|s| s.connected && !s.completed);

    let body = if agent_id.is_empty() {
        "select an agent to view the live tail".to_string()
    } else if let Some(stream) = stream {
        let chunks = stream.chunks.iter().cloned().collect::<Vec<_>>();
        if chunks.is_empty() {
            if stream.connected {
                "waiting for live chunks...".to_string()
            } else {
                "connecting...".to_string()
            }
        } else {
            let mut joined = chunks.join("\n");
            // Append a blinking cursor when actively streaming.
            if is_streaming {
                let cursor = if tui_state.atmosphere.frame_count % 8 < 4 {
                    "\u{2588}" // full block
                } else {
                    " "
                };
                joined.push_str(cursor);
            }
            joined
        }
    } else {
        // No WebSocket stream -- fall back to agent output collected by the
        // runner (approval / plan-run mode without a sidecar).
        let collected = collect_agent_output_lines(tui_state, tui_state.selected_agent);
        if collected.is_empty() {
            "no live stream (output appears in Output tab)".to_string()
        } else {
            collected.join("\n")
        }
    };

    let paragraph = Paragraph::new(body)
        .style(theme.text())
        .wrap(Wrap { trim: false });
    let max_scroll = paragraph
        .line_count(inner.width)
        .saturating_sub(inner.height as usize)
        .min(u16::MAX as usize) as u16;
    frame.render_widget(paragraph.scroll((max_scroll, 0)), inner);
}

pub(crate) fn collect_agent_output_lines(tui_state: &TuiState, selected: usize) -> Vec<String> {
    let selected_agent = tui_state.agent_summaries.get(selected);

    // Priority (item 41 fix):
    //   0. Live push-mode task_output_tails for the agent's current task
    //   1. Selected agent's live row data from tui_state.agents
    //   2. current_plan_execution.agent_output_tail (pull-mode fallback)
    //   3. episode output text

    // 0. Live push-mode tail for the agent's current task (item 41).
    if let Some(agent_summary) = selected_agent {
        if let Some(agent_row) = tui_state
            .agents
            .iter()
            .find(|row| row.id == agent_summary.id)
        {
            if !agent_row.current_task.is_empty() {
                if let Some(live) = tui_state
                    .task_output_tails
                    .get(&agent_row.current_task)
                    .filter(|lines| !lines.is_empty())
                {
                    return live.clone();
                }
            }
        }
    }

    if let Some(agent_summary) = selected_agent {
        if let Some(agent_row) = tui_state
            .agents
            .iter()
            .find(|row| row.id == agent_summary.id)
        {
            if !agent_row.output_lines.is_empty() {
                return agent_row.output_lines.clone();
            }
            if !agent_row.last_output_line.is_empty() {
                return vec![agent_row.last_output_line.clone()];
            }
            if !agent_row.current_task.is_empty() {
                let task_output = tui_state
                    .task_output_tails
                    .get(&agent_row.current_task)
                    .cloned()
                    .unwrap_or_default();
                if !task_output.is_empty() {
                    return task_output;
                }
            }
        }

        for episode in &tui_state.episodes_cache {
            if episode.agent_id != agent_summary.id {
                continue;
            }
            for key in [
                "stderr",
                "agent_stderr",
                "output",
                "stdout",
                "agent_output",
                "output_tail",
            ] {
                if let Some(text) = episode.extra.get(key).and_then(|v| v.as_str()) {
                    if !text.trim().is_empty() {
                        return text.lines().map(String::from).collect();
                    }
                }
            }
        }
    }

    Vec::new()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Role-specific accent color.
fn role_accent(role: &str, theme: &Theme) -> Color {
    match role.to_lowercase().as_str() {
        "implementer" | "impl" => Theme::ROSE,
        "strategist" | "strat" => Theme::DREAM,
        "architect" | "arch" => Theme::SAGE,
        "auditor" | "audit" => Theme::WARNING,
        "critic" | "crit" => Theme::EMBER,
        "conductor" | "cond" => Theme::LAVENDER,
        "researcher" | "res" => Theme::TEAL,
        _ => theme.accent,
    }
}

/// Format a token count as compact string.
fn format_tokens(n: u64) -> String {
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

fn render_route_metrics_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let selected_agent = tui_state.agent_summaries.get(view_state.selected);
    let selected_id = selected_agent
        .map(|a| a.id.as_str())
        .or_else(|| {
            tui_state
                .agents
                .get(view_state.selected)
                .map(|agent| agent.id.as_str())
        })
        .unwrap_or("");
    let agent_row = tui_state.agents.iter().find(|row| row.id == selected_id);
    let metrics = tui_state.route_metrics.get(selected_id);

    let model = metrics
        .map(|metric| metric.model.as_str())
        .filter(|model| !model.is_empty())
        .or_else(|| {
            agent_row
                .map(|row| row.model.as_str())
                .filter(|model| !model.is_empty())
        })
        .unwrap_or("");
    let model_label = display_model(Some(model));
    let context_used = metrics
        .map(|metric| metric.context_used)
        .unwrap_or_else(|| agent_row.map_or(0, |row| row.input_tokens + row.output_tokens));
    let context_limit = metrics
        .map(|metric| metric.context_limit)
        .filter(|limit| *limit > 0)
        .or_else(|| {
            agent_row
                .map(|row| row.context_limit)
                .filter(|limit| *limit > 0)
        })
        .unwrap_or_else(|| model_context_limit(model));
    let utilization = if context_limit > 0 {
        (context_used as f64 / context_limit as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let focus_score = metrics.map_or(0.0, |metric| metric.focus_score);
    let tier = metrics
        .map(|metric| metric.tier.as_str())
        .filter(|tier| !tier.is_empty())
        .unwrap_or("balanced");

    let usage_color = if utilization >= 0.8 {
        theme.danger
    } else if utilization >= 0.5 {
        theme.warning
    } else {
        theme.success
    };
    let focus_color = if focus_score >= 0.75 {
        theme.foreground
    } else if focus_score >= 0.4 {
        theme.muted
    } else {
        Theme::FOCUS_LOW
    };

    let model_style = Style::default().add_modifier(Modifier::BOLD);
    let usage_style = Style::default()
        .fg(usage_color)
        .add_modifier(Modifier::BOLD);
    let line = if area.width < 52 {
        Line::from(vec![
            Span::styled(
                format!(" [{}]", truncate_middle(&model_label, 16)),
                model_style,
            ),
            Span::styled("  ctx ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{:>3}%", (utilization * 100.0).round() as u64),
                usage_style,
            ),
            Span::styled("  ", Style::default()),
            Span::styled(
                truncate_middle(tier, 12),
                Style::default().fg(theme.foreground),
            ),
        ])
    } else if area.width < 78 {
        Line::from(vec![
            Span::styled(
                format!(" [{}]", truncate_middle(&model_label, 18)),
                model_style,
            ),
            Span::styled("  ctx ", Style::default().fg(theme.muted)),
            Span::styled(
                format!(
                    "{}/{}",
                    format_tokens(context_used),
                    format_tokens(context_limit)
                ),
                usage_style,
            ),
            Span::styled("  ·  ", Style::default().fg(theme.muted)),
            Span::styled(
                truncate_middle(tier, 12),
                Style::default().fg(theme.foreground),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(format!("[{}]", model_label), model_style),
            Span::styled("  ctx ", Style::default().fg(theme.muted)),
            Span::styled(
                format!(
                    "{}/{}",
                    format_tokens(context_used),
                    format_tokens(context_limit)
                ),
                usage_style,
            ),
            Span::styled("  ·  focus ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{:.2}", focus_score),
                Style::default()
                    .fg(focus_color)
                    .add_modifier(if focus_score >= 0.75 {
                        Modifier::BOLD
                    } else {
                        Modifier::DIM
                    }),
            ),
            Span::styled("  ·  ", Style::default().fg(theme.muted)),
            Span::styled(tier.to_string(), Style::default().fg(theme.foreground)),
        ])
    };

    frame.render_widget(Paragraph::new(line), area);
}

fn is_agent_active(status: &str) -> bool {
    matches!(status, "running" | "active")
}

fn is_agent_done(status: &str) -> bool {
    matches!(status, "done" | "completed")
}

fn is_agent_failed(status: &str) -> bool {
    matches!(status, "error" | "failed")
}

fn agent_status_rank(status: &str) -> u8 {
    if is_agent_active(status) {
        0
    } else if matches!(status, "idle" | "waiting") {
        1
    } else if is_agent_done(status) {
        2
    } else if is_agent_failed(status) {
        3
    } else {
        4
    }
}

use crate::tui::display_utils::display_model;

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::tui::dashboard::AgentSummary;
    use crate::tui::state::AgentRow;

    fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn populated_state() -> TuiState {
        let mut state = TuiState::default();
        for idx in 0..10 {
            let id = format!("agent-{idx:02}");
            state.agent_summaries.push(AgentSummary {
                id: id.clone(),
                label: if idx % 2 == 0 {
                    "implementer".to_string()
                } else {
                    "auditor".to_string()
                },
                plan_id: Some(format!("plan-{idx:02}")),
                status: "active".to_string(),
            });
            state.agents.push(AgentRow {
                id,
                active: true,
                status: AgentStatus::Active,
                role: "implementer".to_string(),
                model: "gpt-5.6-sol".to_string(),
                input_tokens: 24_000 + idx as u64,
                output_tokens: 3_000,
                context_limit: 200_000,
                current_plan: format!("plan-{idx:02}"),
                current_task: format!("T{idx}: keep the selected work visible"),
                output_lines: if idx == 9 {
                    (0..18)
                        .map(|line| {
                            let suffix = if line == 17 { " TAIL_MARKER" } else { "" };
                            format!(
                                "line {line:02} {}{suffix}",
                                "wrapped agent transcript ".repeat(5)
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                },
                ..Default::default()
            });
        }
        state
    }

    fn render_at(width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let data = DashboardData::default();
        let state = populated_state();
        let view_state = ViewState {
            selected: 9,
            auto_tail: true,
            ..ViewState::default()
        };
        let theme = Theme::dark();
        terminal
            .draw(|frame| render(frame, frame.area(), &data, &state, &view_state, &theme))
            .unwrap();
        rendered_text(&terminal)
    }

    #[test]
    fn agents_view_preserves_selection_and_wrapped_tail_at_common_sizes() {
        for (width, height) in [(80, 24), (120, 40), (200, 60)] {
            let rendered = render_at(width, height);
            assert!(
                rendered.contains("agent-09"),
                "selected agent missing at {width}x{height}:\n{rendered}"
            );
            assert!(
                rendered.contains("TAIL_MARKER"),
                "wrapped transcript tail missing at {width}x{height}:\n{rendered}"
            );
            // Active agents show "FOLLOWING"; idle shows "TAIL".
            assert!(
                rendered.contains("FOLLOWING") || rendered.contains("[TAIL]"),
                "tail indicator missing at {width}x{height}"
            );
        }
    }

    #[test]
    fn narrow_agents_view_uses_stacked_readable_layout() {
        let rendered = render_at(80, 24);
        assert!(rendered.contains("Agents"));
        assert!(rendered.contains("Output"));
        assert!(rendered.contains("5.6-sol"));
        assert!(
            !rendered.contains("Live Stream"),
            "duplicate stream panel should yield to transcript on a short terminal"
        );
    }
}
