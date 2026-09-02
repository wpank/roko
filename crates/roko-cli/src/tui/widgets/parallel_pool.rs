//! Parallel agent roster table widget.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use super::super::dashboard::Theme;
use super::super::state::{AgentRow, AgentStatus};

/// Render a table of parallel agent instances.
///
/// Active agents are sorted first. The selected row is highlighted.
/// Includes agent name, role/model combined column, current task,
/// a mini context-usage progress bar, and compact token usage.
pub(crate) fn render_parallel_pool(
    frame: &mut Frame<'_>,
    area: Rect,
    agents: &[AgentRow],
    selected: usize,
    theme: &Theme,
) {
    let active_count = agents.iter().filter(|a| a.active).count();
    let title = if active_count > 0 {
        format!("Agents ({active_count} active)")
    } else {
        "Agents".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Theme::unfocused_border_style())
        .title_style(Theme::unfocused_title_style())
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if agents.is_empty() {
        let empty =
            Paragraph::new("No agents running \u{2014} agents spawn when plans execute")
                .style(theme.muted())
                .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    }

    // Sort: active first, then by role.
    let mut sorted: Vec<(usize, &AgentRow)> = agents.iter().enumerate().collect();
    sorted.sort_by(|(_, a), (_, b)| {
        b.active
            .cmp(&a.active)
            .then_with(|| a.role.cmp(&b.role))
            .then_with(|| a.id.cmp(&b.id))
    });

    let wide = inner.width >= 80;
    let medium = inner.width >= 50;

    let rows: Vec<Row<'_>> = sorted
        .iter()
        .map(|(orig_idx, agent)| {
            let is_selected = *orig_idx == selected;
            let row_style = if is_selected {
                theme.selection()
            } else {
                Style::default()
            };

            let current_task = if !agent.current_task.is_empty() {
                agent.current_task.clone()
            } else if !agent.current_plan.is_empty() {
                agent.current_plan.clone()
            } else {
                "-".to_string()
            };

            // Combined role/model cell for density.
            let role_model = if medium {
                format!(
                    "{}/{}",
                    truncate(&agent.role, 8),
                    shorten_model_name(&agent.model)
                )
            } else {
                truncate(&agent.role, 10)
            };

            let status = agent.status;
            let task_w = if wide { 20 } else { 14 };
            let mut cells = vec![
                Cell::from(truncate(&agent.id, 10)),
                Cell::from(Span::styled(
                    truncate(&role_model, if wide { 20 } else { 14 }),
                    Style::default().fg(theme.foreground),
                )),
                Cell::from(truncate(&current_task, task_w)),
                Cell::from(render_status_label(status, theme)),
                Cell::from(render_context_bar(agent, theme)),
            ];
            if wide {
                cells.push(Cell::from(render_compact_usage(
                    agent.input_tokens,
                    agent.output_tokens,
                    theme,
                )));
            }
            Row::new(cells).style(row_style)
        })
        .collect();

    let task_w = if wide { 20u16 } else { 14 };
    let role_w = if wide { 20u16 } else { 14 };

    let mut widths = vec![
        Constraint::Length(10),
        Constraint::Length(role_w),
        Constraint::Min(task_w.min(10)),
        Constraint::Length(8),
        Constraint::Length(10),
    ];
    if wide {
        widths.push(Constraint::Min(12));
    }

    let mut header_cells = vec![
        Cell::from("agent"),
        Cell::from("role/model"),
        Cell::from("task"),
        Cell::from("status"),
        Cell::from("ctx"),
    ];
    if wide {
        header_cells.push(Cell::from("tokens"));
    }

    let table = Table::new(rows, widths)
        .header(
            Row::new(header_cells).style(
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .column_spacing(1);

    frame.render_widget(table, inner);
}

use crate::tui::display_utils::truncate;

fn render_status_label(status: AgentStatus, theme: &Theme) -> Line<'static> {
    let (label, color) = match status {
        AgentStatus::Active => ("active", theme.accent),
        AgentStatus::Idle => ("idle", theme.muted),
        AgentStatus::Done => ("done", theme.success),
        AgentStatus::Failed => ("failed", theme.danger),
    };

    Line::from(vec![Span::styled(
        format!("{:^8}", label),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )])
}

/// Mini context-usage progress bar: `[=====>   ] 42%`
fn render_context_bar(agent: &AgentRow, theme: &Theme) -> Line<'static> {
    let total = agent.input_tokens + agent.output_tokens;
    let limit = agent.context_limit;
    if limit == 0 || total == 0 {
        return Line::from(Span::styled("-", Style::default().fg(theme.muted)));
    }
    let ratio = (total as f64 / limit as f64).clamp(0.0, 1.0);
    let pct = (ratio * 100.0).round() as u64;
    let bar_w = 6;
    let filled = (ratio * bar_w as f64).round() as usize;
    let bar = format!(
        "{}{}",
        "\u{2588}".repeat(filled.min(bar_w)),
        "\u{2591}".repeat(bar_w.saturating_sub(filled)),
    );
    let color = if ratio >= 0.8 {
        theme.danger
    } else if ratio >= 0.5 {
        theme.warning
    } else {
        theme.info
    };
    Line::from(vec![
        Span::styled(bar, Style::default().fg(color)),
        Span::styled(format!("{pct:>3}%"), Style::default().fg(theme.muted)),
    ])
}

/// Compact token usage: `12k/4k` (input/output).
fn render_compact_usage(input_tokens: u64, output_tokens: u64, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            fmt_k(input_tokens),
            Style::default().fg(theme.foreground),
        ),
        Span::styled("/", Style::default().fg(theme.muted)),
        Span::styled(
            fmt_k(output_tokens),
            Style::default().fg(theme.foreground),
        ),
    ])
}

fn fmt_k(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

/// Shorten model slug for compact display: "claude-sonnet-4-20250514" -> "sonnet-4".
fn shorten_model_name(model: &str) -> String {
    // Strip common prefixes and date suffixes.
    let s = model
        .strip_prefix("claude-")
        .or_else(|| model.strip_prefix("gpt-"))
        .unwrap_or(model);
    // Remove date suffix (e.g. "-20250514").
    let s = if s.len() > 10 {
        s.split('-')
            .take_while(|part| part.len() < 8 || part.parse::<u64>().is_err())
            .collect::<Vec<_>>()
            .join("-")
    } else {
        s.to_string()
    };
    if s.len() > 12 {
        s[..12].to_string()
    } else {
        s
    }
}
