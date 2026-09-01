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
pub(crate) fn render_parallel_pool(
    frame: &mut Frame<'_>,
    area: Rect,
    agents: &[AgentRow],
    selected: usize,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("parallel agents")
        .border_style(Theme::unfocused_border_style())
        .title_style(Theme::unfocused_title_style())
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if agents.is_empty() {
        let empty = Paragraph::new("no parallel agents")
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
            let status = agent.status;
            Row::new(vec![
                Cell::from(truncate(&agent.id, 12)),
                Cell::from(truncate(&agent.role, 10)),
                Cell::from(truncate(&agent.model, 12)),
                Cell::from(truncate(&current_task, 18)),
                Cell::from(render_status_label(status, theme)),
                Cell::from(render_cumulative_usage(
                    agent.input_tokens,
                    agent.output_tokens,
                    theme,
                )),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Min(14),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from("agent id"),
            Cell::from("role"),
            Cell::from("model"),
            Cell::from("task"),
            Cell::from("progress"),
            Cell::from("cumulative usage"),
        ])
        .style(
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

fn render_cumulative_usage(input_tokens: u64, output_tokens: u64, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled("in ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{}k", input_tokens / 1000),
            Style::default().fg(theme.foreground),
        ),
        Span::styled(" out ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{}k", output_tokens / 1000),
            Style::default().fg(theme.foreground),
        ),
    ])
}
