//! F9 Atelier view -- PRD and plan workshop.
//!
//! Layout: top 3-line stats bar + left 40% (PRD list) + right 60% (plan detail).
//!
//! Data source: `roko_core::DashboardSnapshot` via StateHub.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap};

use super::{SubView, ViewState};
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::empty_state;
use crate::tui::state::TuiState;
use crate::tui::tabs::Tab;

type PrdEntry = roko_core::PrdSummary;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum PrdStatus {
    #[default]
    Idea,
    Draft,
    Published,
    Planned,
}

impl PrdStatus {
    fn from_str(status: &str) -> Self {
        match status.trim().to_ascii_lowercase().as_str() {
            "published" | "publish" => Self::Published,
            "draft" => Self::Draft,
            "planned" | "plan" => Self::Planned,
            _ => Self::Idea,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Idea => "idea",
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Planned => "planned",
        }
    }

    /// Four-character Unicode badge shown in the PRD list.
    fn badge(self) -> &'static str {
        match self {
            Self::Idea => "IDEA",
            Self::Draft => "DRFT",
            Self::Published => "PUBL",
            Self::Planned => "PLAN",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum TaskState {
    #[default]
    Pending,
    Running,
    Done,
    Failed,
}

impl TaskState {
    fn from_str(status: &str) -> Self {
        match status.trim().to_ascii_lowercase().as_str() {
            "done" | "completed" | "passed" | "skipped" => Self::Done,
            "running" | "active" | "executing" | "in_progress" | "implementing" | "gating"
            | "verifying" | "reviewing" => Self::Running,
            "failed" | "error" | "gate_rejected" => Self::Failed,
            _ => Self::Pending,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Pending => "[ ]",
            Self::Running => "[>]",
            Self::Done => "[x]",
            Self::Failed => "[!]",
        }
    }
}

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

/// Render the full atelier view.
///
/// Handles empty state and terminal resize via percentage constraints.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let prds = &tui_state.atelier_prds;

    let rows = Layout::vertical([
        Constraint::Length(1), // Sub-tab bar
        Constraint::Length(3), // Stats bar
        Constraint::Min(0),    // Main content
    ])
    .split(area);

    render_sub_tab_bar(frame, rows[0], view_state, theme);
    render_stats_bar(frame, rows[1], prds, data, theme);

    if prds.is_empty() {
        empty_state::render_empty_state(frame, rows[2], Tab::Atelier, &tui_state.atmosphere);
        return;
    }

    let selected = view_state.selected.min(prds.len().saturating_sub(1));
    match view_state.active_sub_view(Tab::Atelier) {
        SubView::PlanExplorer => {
            render_plan_detail(frame, rows[2], prds, selected, data, tui_state, theme);
        }
        _ => {
            let (sidebar, detail) =
                crate::tui::layout::responsive_panel_split(rows[2], 40, 100, rows[2].height / 3);
            render_prd_list(frame, sidebar, prds, selected, theme);
            render_plan_detail(frame, detail, prds, selected, data, tui_state, theme);
        }
    }
}

fn render_sub_tab_bar(frame: &mut Frame<'_>, area: Rect, view_state: &ViewState, theme: &Theme) {
    let label = SubView::bar_label(Tab::Atelier, view_state.sub_tab);
    let bar = Paragraph::new(Line::from(Span::styled(label, theme.muted())))
        .alignment(Alignment::Center)
        .style(ratatui::style::Style::default().bg(Theme::BG_RAISED));
    frame.render_widget(bar, area);
}

// ---------------------------------------------------------------------------
// Stats bar
// ---------------------------------------------------------------------------

fn render_stats_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    prds: &[PrdEntry],
    data: &DashboardData,
    theme: &Theme,
) {
    let block = Block::bordered().border_style(theme.muted());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let plan_count = prds.iter().filter(|p| p.plan_count > 0).count();
    let done_tasks: usize = prds.iter().map(|p| p.task_done).sum();
    let total_tasks: usize = prds.iter().map(|p| p.task_total).sum();

    let all_done = total_tasks > 0 && done_tasks == total_tasks;
    let tasks_style = if all_done {
        theme.success()
    } else {
        theme.text()
    };

    let cols = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ])
    .split(inner);

    let stat = |label_text: &str, value: String, style| {
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{label_text}: "), theme.label()),
            Span::styled(value, style),
        ]))
        .alignment(Alignment::Center)
    };

    frame.render_widget(stat("PRDs", prds.len().to_string(), theme.value()), cols[0]);
    frame.render_widget(stat("Plans", plan_count.to_string(), theme.info()), cols[1]);
    frame.render_widget(
        stat("Tasks", format!("{done_tasks}/{total_tasks}"), tasks_style),
        cols[2],
    );
    frame.render_widget(
        stat("Agents", data.agents.len().to_string(), theme.value()),
        cols[3],
    );
    frame.render_widget(
        stat(
            "Episodes",
            data.efficiency.event_count.to_string(),
            theme.metadata(),
        ),
        cols[4],
    );
}

// ---------------------------------------------------------------------------
// Left panel: PRD list
// ---------------------------------------------------------------------------

fn render_prd_list(
    frame: &mut Frame<'_>,
    area: Rect,
    prds: &[PrdEntry],
    selected: usize,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            format!(" PRDs ({}) ", prds.len()),
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 10 {
        return;
    }

    let visible_height = inner.height as usize;
    let scroll = if selected >= visible_height {
        selected - visible_height + 1
    } else {
        0
    };

    let items: Vec<ListItem<'_>> = prds
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, prd)| {
            let is_sel = i == selected;
            let status = PrdStatus::from_str(&prd.status);
            let badge_style = match status {
                PrdStatus::Idea => theme.badge_pending(),
                PrdStatus::Draft => theme.badge_running(),
                PrdStatus::Published => theme.badge_complete(),
                PrdStatus::Planned => theme.badge_complete(),
            };

            let progress = if prd.task_total > 0 {
                format!(" {}/{}", prd.task_done, prd.task_total)
            } else {
                String::new()
            };

            let title_max = (inner.width as usize).saturating_sub(12 + progress.len());
            let row_style = if is_sel {
                theme.selection()
            } else {
                ratatui::style::Style::default()
                    .fg(Theme::BONE_BRIGHT)
                    .add_modifier(Modifier::BOLD)
            };

            let mut spans = vec![
                Span::styled(format!(" {} ", status.badge()), badge_style),
                Span::raw(" "),
                Span::styled(truncate(&prd.title, title_max), row_style),
            ];

            if !progress.is_empty() {
                let progress_style = if prd.task_failed > 0 {
                    theme.danger()
                } else if prd.task_done == prd.task_total {
                    theme.success()
                } else {
                    theme.muted()
                };
                spans.push(Span::styled(progress, progress_style));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

// ---------------------------------------------------------------------------
// Right panel: plan detail with task list
// ---------------------------------------------------------------------------

fn render_plan_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    prds: &[PrdEntry],
    selected: usize,
    _data: &DashboardData,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let Some(prd) = prds.get(selected) else {
        return;
    };

    let block = Block::bordered()
        .title(Span::styled(
            format!(" {} ", truncate(&prd.title, 40)),
            ratatui::style::Style::default()
                .fg(Theme::BONE_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 20 {
        return;
    }

    // Compute how many lines the actions section needs.
    let status = PrdStatus::from_str(&prd.status);
    let actions_height: u16 = match status {
        PrdStatus::Idea | PrdStatus::Draft => 5,
        PrdStatus::Published => 5,
        PrdStatus::Planned => 4,
    };

    let sections = Layout::vertical([
        Constraint::Length(6),              // PRD metadata + separator
        Constraint::Length(actions_height), // CLI actions + separator
        Constraint::Min(0),                 // Task list
        Constraint::Length(1),              // Keybinding hints
    ])
    .split(inner);

    // PRD metadata
    let status_badge_style = match status {
        PrdStatus::Idea => theme.badge_pending(),
        PrdStatus::Draft => theme.badge_running(),
        PrdStatus::Published => theme.badge_complete(),
        PrdStatus::Planned => theme.badge_complete(),
    };
    let status_text_style = match status {
        PrdStatus::Idea => theme.muted(),
        PrdStatus::Draft => theme.warning(),
        PrdStatus::Published => theme.success(),
        PrdStatus::Planned => theme.info(),
    };
    let completion = if prd.task_total > 0 {
        format!(
            "{:.0}%",
            prd.task_done as f64 / prd.task_total as f64 * 100.0
        )
    } else {
        "\u{2014}".to_string() // em dash
    };

    let sep_width = inner.width as usize;
    let separator = "\u{2500}".repeat(sep_width.min(120));
    let sep_line = Line::from(Span::styled(
        separator.clone(),
        ratatui::style::Style::default().fg(Theme::SEPARATOR),
    ));

    let meta_lines = vec![
        Line::from(vec![
            Span::styled("slug:       ", theme.label()),
            Span::styled(&prd.slug, theme.value()),
        ]),
        Line::from(vec![
            Span::styled("status:     ", theme.label()),
            Span::styled(format!(" {} ", status.badge()), status_badge_style),
            Span::raw(" "),
            Span::styled(status.label(), status_text_style),
        ]),
        Line::from(vec![
            Span::styled("tasks:      ", theme.label()),
            Span::styled(
                format!("{}/{}", prd.task_done, prd.task_total),
                theme.value(),
            ),
            Span::styled(format!("  ({completion})"), theme.metadata()),
        ]),
        Line::from(vec![
            Span::styled("failed:     ", theme.label()),
            Span::styled(
                prd.task_failed.to_string(),
                if prd.task_failed > 0 {
                    theme.danger()
                } else {
                    theme.metadata()
                },
            ),
        ]),
        sep_line.clone(),
    ];
    frame.render_widget(
        Paragraph::new(meta_lines).wrap(Wrap { trim: false }),
        sections[0],
    );

    // CLI actions block: show actionable commands based on PRD status.
    let mut action_lines: Vec<Line<'_>> =
        vec![Line::from(Span::styled("Actions", theme.section_header()))];
    match status {
        PrdStatus::Idea | PrdStatus::Draft => {
            action_lines.push(Line::from(Span::styled(
                "  roko prd draft promote    # publish this draft".to_string(),
                theme.metadata(),
            )));
            action_lines.push(Line::from(Span::styled(
                format!(
                    "  roko prd plan {:<12}# generate implementation plan",
                    &prd.slug
                ),
                theme.metadata(),
            )));
        }
        PrdStatus::Published => {
            action_lines.push(Line::from(Span::styled(
                format!(
                    "  roko prd plan {:<12}# generate implementation plan",
                    &prd.slug
                ),
                theme.metadata(),
            )));
            action_lines.push(Line::from(Span::styled(
                "  roko plan run plans/      # execute generated plan".to_string(),
                theme.metadata(),
            )));
        }
        PrdStatus::Planned => {
            action_lines.push(Line::from(Span::styled(
                "  roko plan run plans/      # execute the plan".to_string(),
                theme.metadata(),
            )));
        }
    }
    action_lines.push(sep_line);
    frame.render_widget(
        Paragraph::new(action_lines).wrap(Wrap { trim: false }),
        sections[1],
    );

    // Task list: read from cached atelier tasks in TuiState.
    let empty_tasks = Vec::new();
    let tasks = tui_state
        .atelier_tasks_by_slug
        .get(&prd.slug)
        .unwrap_or(&empty_tasks);

    let visible_task_rows = (sections[2].height.saturating_sub(2)) as usize; // border + header
    let overflow = tasks.len() > visible_task_rows;
    let task_title = if overflow {
        format!(
            " Tasks ({}) [{} hidden] ",
            tasks.len(),
            tasks.len() - visible_task_rows
        )
    } else {
        format!(" Tasks ({}) ", tasks.len())
    };
    let task_block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(task_title, theme.section_header()))
        .border_style(ratatui::style::Style::default().fg(Theme::SEPARATOR));
    let task_inner = task_block.inner(sections[2]);
    frame.render_widget(task_block, sections[2]);

    if tasks.is_empty() {
        frame.render_widget(
            Paragraph::new("no tasks -- run 'roko prd plan <slug>' to generate")
                .style(theme.muted())
                .wrap(Wrap { trim: false }),
            task_inner,
        );
        return;
    }

    let title_max = (task_inner.width as usize).saturating_sub(24);
    let rows: Vec<Row<'_>> = tasks
        .iter()
        .map(|task| {
            let status = TaskState::from_str(&task.status);
            let icon_style = match status {
                TaskState::Pending => theme.muted(),
                TaskState::Running => theme.warning(),
                TaskState::Done => theme.success(),
                TaskState::Failed => theme.danger(),
            };
            Row::new(vec![
                Cell::from(Span::styled(status.icon(), icon_style)),
                Cell::from(Span::styled(truncate(&task.id, 8), theme.metadata())),
                Cell::from(Span::styled(
                    truncate(&task.title, title_max),
                    theme.value(),
                )),
                Cell::from(Span::styled(truncate(&task.agent, 12), theme.metadata())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Min(10),
        Constraint::Length(12),
    ];
    frame.render_widget(
        Table::new(rows, widths)
            .header(Row::new(["", "id", "title", "agent"]).style(theme.label()))
            .column_spacing(1),
        task_inner,
    );

    // Bottom keybinding hints (active keys only)
    let hint_line = Line::from(vec![
        Span::styled(" j/k", theme.accent()),
        Span::styled(":navigate  ", theme.muted()),
        Span::styled("Enter", theme.accent()),
        Span::styled(":expand  ", theme.muted()),
        Span::styled("r", theme.accent()),
        Span::styled(":refresh", theme.muted()),
    ]);
    frame.render_widget(
        Paragraph::new(hint_line).alignment(Alignment::Center),
        sections[3],
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

use crate::tui::display_utils::truncate;
