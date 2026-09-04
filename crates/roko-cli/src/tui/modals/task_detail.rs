//! Task detail modal.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::tui::dashboard::{GateSignalSummary, Theme};
use crate::tui::state::{TaskRow, TaskRowStatus};

/// Render the task detail modal for a task in the checklist.
pub fn render_task_detail_modal(
    frame: &mut Frame<'_>,
    area: Rect,
    task: &TaskRow,
    assigned_agents: &[String],
    gate_results: &[GateSignalSummary],
    scroll_offset: usize,
    theme: &Theme,
) {
    let popup = centered_rect(82, 78, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", task.id))
        .title_alignment(Alignment::Center)
        .border_style(theme.accent());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let sep_width = inner.width.saturating_sub(2) as usize;
    let mut lines: Vec<Line<'_>> = Vec::with_capacity(64);

    // ── Header: title + separator ────────────────────────────────────
    lines.push(Line::from(Span::styled(
        task.title.clone(),
        theme.section_header(),
    )));
    lines.push(Line::from(Span::styled(
        "\u{2550}".repeat(sep_width),
        theme.accent(),
    )));
    lines.push(Line::from(""));

    // ── Status section ─────────────────────────────────────────────────
    push_section_header(&mut lines, "Status", sep_width, theme);

    let (badge, status_style) = match task.status {
        TaskRowStatus::Done => ("\u{2713} done", theme.success()),
        TaskRowStatus::Active => ("\u{25b6} running", theme.info()),
        TaskRowStatus::Failed => ("\u{2717} failed", theme.danger()),
        TaskRowStatus::Blocked => ("\u{25a0} blocked", theme.warning()),
        TaskRowStatus::Pending => ("\u{25cb} pending", theme.muted()),
    };
    push_label_value(
        &mut lines,
        "Status",
        badge,
        theme.label(),
        status_style.add_modifier(Modifier::BOLD),
        theme,
    );
    push_label_value(
        &mut lines,
        "Elapsed",
        &format_elapsed(task.elapsed_secs),
        theme.label(),
        theme.value(),
        theme,
    );
    lines.push(Line::from(""));

    // ── Agent Info section ──────────────────────────────────────────────
    push_section_header(&mut lines, "Agent Info", sep_width, theme);

    let agent_str = if assigned_agents.is_empty() {
        "\u{2014}".to_string()
    } else {
        assigned_agents.join(", ")
    };
    let agent_style = if assigned_agents.is_empty() {
        theme.muted()
    } else {
        theme.value()
    };
    push_label_value(
        &mut lines,
        "Agent",
        &agent_str,
        theme.label(),
        agent_style,
        theme,
    );
    push_label_value(
        &mut lines,
        "Model",
        "\u{2014}",
        theme.label(),
        theme.muted(),
        theme,
    );
    push_label_value(
        &mut lines,
        "Attempt",
        "\u{2014}",
        theme.label(),
        theme.muted(),
        theme,
    );
    lines.push(Line::from(""));

    // ── Acceptance Criteria ──────────────────────────────────────────
    push_section_header(&mut lines, "Acceptance Criteria", sep_width, theme);
    // Placeholder: no criteria data on TaskRow
    lines.push(Line::from(vec![
        Span::styled("  1. ", theme.metadata()),
        Span::styled("\u{2014}", theme.muted()),
    ]));
    lines.push(Line::from(""));

    // ── Verify Command (code block) ─────────────────────────────────
    push_section_header(&mut lines, "Verify Command", sep_width, theme);
    push_code_block(&mut lines, "\u{2014}", sep_width, theme);
    lines.push(Line::from(""));

    // ── Files ────────────────────────────────────────────────────────
    push_section_header(&mut lines, "Files", sep_width, theme);
    lines.push(Line::from(Span::styled("  \u{2014}", theme.muted())));
    lines.push(Line::from(""));

    // ── Dependencies ─────────────────────────────────────────────────
    push_section_header(&mut lines, "Dependencies", sep_width, theme);
    lines.push(Line::from(Span::styled("  \u{2014}", theme.muted())));
    lines.push(Line::from(""));

    // ── Routing Context ──────────────────────────────────────────────
    push_section_header(&mut lines, "Routing Context", sep_width, theme);
    push_label_value(
        &mut lines,
        "  Category",
        "\u{2014}",
        theme.label(),
        theme.muted(),
        theme,
    );
    push_label_value(
        &mut lines,
        "  Reasoning",
        "\u{2014}",
        theme.label(),
        theme.muted(),
        theme,
    );
    push_label_value(
        &mut lines,
        "  Speed",
        "\u{2014}",
        theme.label(),
        theme.muted(),
        theme,
    );
    push_label_value(
        &mut lines,
        "  Quality",
        "\u{2014}",
        theme.label(),
        theme.muted(),
        theme,
    );
    push_label_value(
        &mut lines,
        "  Context",
        "\u{2014}",
        theme.label(),
        theme.muted(),
        theme,
    );
    push_label_value(
        &mut lines,
        "  Band",
        "\u{2014}",
        theme.label(),
        theme.muted(),
        theme,
    );
    lines.push(Line::from(""));

    // ── Gate Results ─────────────────────────────────────────────────
    push_section_header(&mut lines, "Gate Results", sep_width, theme);

    if gate_results.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No gate results recorded.",
            theme.muted(),
        )));
    } else {
        for gate in gate_results {
            let verdict_style = if gate.passed {
                theme.success()
            } else {
                theme.danger()
            };
            let icon = if gate.passed { "\u{2713}" } else { "\u{2717}" };
            let duration = if gate.duration_ms > 0 {
                format!("{}ms", gate.duration_ms)
            } else {
                "--".to_string()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {icon} "), verdict_style),
                Span::styled(
                    format!("{:<18}", gate.gate_name),
                    theme.text().add_modifier(Modifier::BOLD),
                ),
                Span::styled(duration, theme.muted()),
            ]));

            if !gate.excerpt.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("    ", theme.muted()),
                    Span::styled(gate.excerpt.clone(), theme.muted()),
                ]));
            }
        }
    }
    lines.push(Line::from(""));

    // ── Footer ───────────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(sep_width),
        theme.metadata(),
    )));
    lines.push(Line::from(vec![
        Span::styled("[Esc]", theme.accent()),
        Span::styled(" close  ", theme.muted()),
        Span::styled("[j/k]", theme.accent()),
        Span::styled(" scroll", theme.muted()),
    ]));

    let scroll = scroll_offset.min(u16::MAX as usize) as u16;
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        inner,
    );
}

/// Push a bold section header with a thin separator line below it.
fn push_section_header(lines: &mut Vec<Line<'_>>, title: &str, width: usize, theme: &Theme) {
    lines.push(Line::from(Span::styled(
        title.to_string(),
        theme.section_header(),
    )));
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(width),
        theme.metadata(),
    )));
}

/// Push a `Label: Value` line with distinct styles for each part.
fn push_label_value(
    lines: &mut Vec<Line<'_>>,
    label: &str,
    value: &str,
    label_style: ratatui::style::Style,
    value_style: ratatui::style::Style,
    _theme: &Theme,
) {
    lines.push(Line::from(vec![
        Span::styled(format!("  {label:<12}"), label_style),
        Span::styled(value.to_string(), value_style),
    ]));
}

/// Push a code block with box-drawing border around the content.
fn push_code_block(lines: &mut Vec<Line<'_>>, content: &str, width: usize, theme: &Theme) {
    let inner_w = width.saturating_sub(4);
    let padded = format!("{content:<inner_w$}");

    // Top border: ┌─ ... ─┐
    let top_fill = inner_w.saturating_sub(0);
    lines.push(Line::from(Span::styled(
        format!("  \u{250c}{}\u{2510}", "\u{2500}".repeat(top_fill)),
        theme.metadata(),
    )));

    // Content line: │ text │
    lines.push(Line::from(vec![
        Span::styled("  \u{2502}", theme.metadata()),
        Span::styled(padded, theme.code_block()),
        Span::styled("\u{2502}", theme.metadata()),
    ]));

    // Bottom border: └─ ... ─┘
    lines.push(Line::from(Span::styled(
        format!("  \u{2514}{}\u{2518}", "\u{2500}".repeat(top_fill)),
        theme.metadata(),
    )));
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn format_elapsed(elapsed_secs: f64) -> String {
    let elapsed_secs = elapsed_secs.max(0.0).round() as u64;
    let hours = elapsed_secs / 3600;
    let minutes = (elapsed_secs % 3600) / 60;
    let seconds = elapsed_secs % 60;

    if hours > 0 {
        format!("{hours}h {minutes:02}m {seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}
