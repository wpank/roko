//! F8 Marketplace view -- job board browser.
//!
//! Layout: left 35% (job list) | right 65% (job detail).
//!
//! Data source: `.roko/jobs/*.json` files (no roko-serve required).
//! Job type tags: research = rose, coding_task = bone, other = muted.
//! Status icons: pending = open circle, active = play, done = check, failed = cross.
//!
//! Keyboard:
//!   j/k     -- navigate list (wraps at boundaries)
//!   Enter   -- focus detail panel
//!   r       -- signal refresh (next file poll picks up changes)

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap};

use super::{SubView, ViewState};
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::state::{JobFormField, TuiState};
use crate::tui::tabs::Tab;

type Job = roko_core::MarketplaceJob;

/// Canonical status string, preferring `status` over `state`.
fn effective_status(job: &Job) -> &str {
    if !job.status.is_empty() {
        &job.status
    } else if !job.state.is_empty() {
        &job.state
    } else {
        "unknown"
    }
}

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

/// Render the full marketplace view.
///
/// Handles terminal resize: the layout uses percentage constraints so it
/// adapts automatically to any terminal width.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
    render_sub_tab_bar(frame, rows[0], view_state, theme);

    let content = rows[1];
    let jobs = &tui_state.marketplace_jobs;

    if jobs.is_empty() {
        match view_state.active_sub_view(Tab::Marketplace) {
            SubView::CreateJob => render_create_job(frame, content, tui_state, theme),
            _ => crate::tui::empty_state::render_empty_state(
                frame,
                content,
                Tab::Marketplace,
                &tui_state.atmosphere,
            ),
        }
        return;
    }

    let selected = view_state.selected.min(jobs.len().saturating_sub(1));
    match view_state.active_sub_view(Tab::Marketplace) {
        SubView::JobDetail => {
            if let Some(job) = jobs.get(selected) {
                render_job_detail(frame, content, job, tui_state, theme);
            }
        }
        SubView::CreateJob => render_create_job(frame, content, tui_state, theme),
        _ => {
            let (sidebar, detail) =
                crate::tui::layout::responsive_panel_split(content, 35, 100, content.height / 3);
            render_job_list(frame, sidebar, jobs, selected, theme);
            if let Some(job) = jobs.get(selected) {
                render_job_detail(frame, detail, job, tui_state, theme);
            }
        }
    }
}

fn render_sub_tab_bar(frame: &mut Frame<'_>, area: Rect, view_state: &ViewState, theme: &Theme) {
    let label = SubView::bar_label(Tab::Marketplace, view_state.sub_tab);
    let bar = Paragraph::new(Line::from(Span::styled(label, theme.muted())))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Theme::BG_RAISED));
    frame.render_widget(bar, area);
}

// Empty state is now handled by `crate::tui::empty_state::render_empty_state`.

// ---------------------------------------------------------------------------
// Left panel: job list
// ---------------------------------------------------------------------------

fn render_job_list(
    frame: &mut Frame<'_>,
    area: Rect,
    jobs: &[Job],
    selected: usize,
    theme: &Theme,
) {
    // Count by canonical status for the header badge.
    let pending = jobs
        .iter()
        .filter(|job| matches!(effective_status(job), "open" | "pending" | "assigned"))
        .count();
    let active = jobs
        .iter()
        .filter(|job| matches!(effective_status(job), "active" | "running" | "in_progress"))
        .count();
    let done = jobs
        .iter()
        .filter(|job| matches!(effective_status(job), "done" | "completed" | "evaluated"))
        .count();

    let block = Block::bordered()
        .title(Line::from(vec![
            Span::styled(format!(" Jobs ({}) ", jobs.len()), theme.section_header()),
            Span::styled(format!(" {pending}P "), theme.badge_pending()),
            Span::raw(" "),
            Span::styled(format!(" {active}A "), theme.badge_running()),
            Span::raw(" "),
            Span::styled(format!(" {done}D "), theme.badge_complete()),
            Span::raw(" "),
        ]))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 10 {
        return;
    }

    let visible_height = inner.height as usize;
    // Scroll to keep `selected` visible.
    let scroll = if selected >= visible_height {
        selected - visible_height + 1
    } else {
        0
    };

    let items: Vec<ListItem<'_>> = jobs
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, job)| {
            let is_sel = i == selected;
            let status = effective_status(job);

            let (icon, status_style) = match status {
                "open" | "pending" => ("\u{25cb}", theme.muted()), // open circle
                "assigned" => ("\u{25d4}", theme.info()),          // half circle
                "in_progress" | "active" | "running" => ("\u{25b6}", theme.warning()), // play
                "submitted" => ("\u{25d1}", theme.info()),         // half circle
                "done" | "completed" | "evaluated" => ("\u{2713}", theme.success()), // check
                "failed" | "cancelled" => ("\u{2717}", theme.danger()), // cross
                _ => ("\u{25cb}", theme.muted()),                  // open circle
            };

            // Job type color tag (research=rose, coding_task=bone/dim, other=muted).
            let type_style = match job.job_type.as_str() {
                "research" => Style::default().fg(Theme::ROSE),
                "coding_task" | "coding" => Style::default().fg(Theme::BONE_DIM),
                _ => theme.muted(),
            };

            let avail_width = (inner.width as usize).saturating_sub(8);
            let title = truncate(&job.title, avail_width);
            let row_style = if is_sel {
                theme.selection()
            } else {
                theme.text()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {icon} "), status_style),
                // Small type indicator
                Span::styled(
                    format!("[{}] ", &job.job_type.chars().take(3).collect::<String>()),
                    type_style,
                ),
                Span::styled(title, row_style),
            ]))
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

// ---------------------------------------------------------------------------
// Right panel: job detail
// ---------------------------------------------------------------------------

fn render_job_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    job: &Job,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            format!(" {} ", truncate(&job.title, 40)),
            theme.section_header(),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 20 {
        return;
    }

    let is_in_progress = matches!(effective_status(job), "in_progress" | "active" | "running");
    let has_progress = is_in_progress && tui_state.job_progress.contains_key(&job.id);

    let sections = Layout::vertical([
        Constraint::Length(8),                                // Metadata table
        Constraint::Min(0),                                   // Description (word-wrapped)
        Constraint::Length(if has_progress { 4 } else { 0 }), // Progress bar
        Constraint::Length(3),                                // Keybinding hints + assign prompt
    ])
    .split(inner);

    let status = effective_status(job);
    let status_badge_style = match status {
        "open" | "pending" => theme.badge_pending(),
        "assigned" | "submitted" => theme.badge_running(),
        "in_progress" | "active" | "running" => theme.badge_running(),
        "done" | "completed" | "evaluated" => theme.badge_complete(),
        "failed" | "cancelled" => theme.badge_failed(),
        _ => theme.muted(),
    };
    let priority_style = match job.priority.as_str() {
        "critical" | "p0" => theme.danger(),
        "high" | "p1" => theme.warning(),
        "medium" | "p2" | "" => theme.muted(),
        _ => theme.muted(),
    };

    // Build valid-transitions hint
    let parsed_status = roko_core::JobStatus::parse(status).unwrap_or(roko_core::JobStatus::Open);
    let transitions = parsed_status.valid_transitions();
    let transition_hint = if transitions.is_empty() {
        "(terminal)".to_string()
    } else {
        transitions
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let col_widths = [Constraint::Length(11), Constraint::Min(0)];

    let meta_rows = vec![
        Row::new([
            Cell::from(Span::styled("id:", theme.label())),
            Cell::from(Span::styled(&job.id, theme.metadata())),
        ]),
        Row::new([
            Cell::from(Span::styled("status:", theme.label())),
            Cell::from(Line::from(vec![
                Span::styled(format!(" {status} "), status_badge_style),
                Span::styled(format!("  \u{2192} {transition_hint}"), theme.metadata()),
            ])),
        ]),
        Row::new([
            Cell::from(Span::styled("type:", theme.label())),
            Cell::from(Span::styled(&job.job_type, theme.value())),
        ]),
        Row::new([
            Cell::from(Span::styled("priority:", theme.label())),
            Cell::from(Span::styled(
                if job.priority.is_empty() {
                    "\u{2014}"
                } else {
                    &job.priority
                },
                priority_style,
            )),
        ]),
        Row::new([
            Cell::from(Span::styled("posted by:", theme.label())),
            Cell::from(if job.posted_by.is_empty() {
                Span::styled("\u{2014}", theme.metadata())
            } else {
                Span::styled(&job.posted_by, theme.value())
            }),
        ]),
        Row::new([
            Cell::from(Span::styled("assigned:", theme.label())),
            Cell::from(if job.assigned_to.is_empty() {
                Span::styled("(unassigned)", theme.metadata())
            } else {
                Span::styled(&job.assigned_to, theme.info())
            }),
        ]),
        Row::new([
            Cell::from(Span::styled("created:", theme.label())),
            Cell::from(Span::styled(
                if job.created_at.is_empty() {
                    "\u{2014}"
                } else {
                    &job.created_at
                },
                theme.metadata(),
            )),
        ]),
        Row::new([
            Cell::from(Span::styled("tags:", theme.label())),
            Cell::from(Span::styled(
                if job.tags.is_empty() {
                    "(none)".to_string()
                } else {
                    job.tags.join(", ")
                },
                theme.metadata(),
            )),
        ]),
    ];

    frame.render_widget(
        Table::new(meta_rows, col_widths).column_spacing(1),
        sections[0],
    );

    // Description with proper word-wrap using ratatui's Wrap widget.
    let desc_block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(" Description ", theme.section_header()))
        .border_style(Style::default().fg(Theme::SEPARATOR));
    let desc_inner = desc_block.inner(sections[1]);
    frame.render_widget(desc_block, sections[1]);

    let desc_text = if job.description.is_empty() {
        "No description provided.".to_string()
    } else {
        job.description.clone()
    };
    frame.render_widget(
        Paragraph::new(desc_text)
            .style(theme.text())
            .wrap(Wrap { trim: false }),
        desc_inner,
    );

    // Progress bar for in-progress jobs.
    if has_progress {
        if let Some(progress) = tui_state.job_progress.get(&job.id) {
            let prog_block = Block::default()
                .borders(Borders::TOP)
                .title(Span::styled(" Progress ", theme.section_header()))
                .border_style(Style::default().fg(Theme::SEPARATOR));
            let prog_inner = prog_block.inner(sections[2]);
            frame.render_widget(prog_block, sections[2]);

            let bar_width = (prog_inner.width as usize).saturating_sub(10);
            let filled = (progress.percent as usize * bar_width) / 100;
            let empty = bar_width.saturating_sub(filled);
            let bar_line = Line::from(vec![
                Span::styled(" [", theme.muted()),
                Span::styled("\u{2588}".repeat(filled), theme.success()),
                Span::styled("\u{2500}".repeat(empty), theme.muted()),
                Span::styled(format!("] {}%", progress.percent), theme.muted()),
            ]);
            let agent_hint = if progress.agent_id.is_empty() {
                String::new()
            } else {
                format!("  agent: {}", progress.agent_id)
            };
            let msg_line = Line::from(vec![
                Span::styled(
                    format!(
                        " {}",
                        truncate(&progress.message, prog_inner.width as usize - 2)
                    ),
                    theme.text(),
                ),
                Span::styled(agent_hint, theme.muted()),
            ]);
            frame.render_widget(Paragraph::new(vec![bar_line, msg_line]), prog_inner);
        }
    }

    // Section index for hints/assign prompt shifts when progress is shown.
    let hints_section = if has_progress { 3 } else { 2 };

    // Bottom hints or assign prompt
    if tui_state.job_assign_editing {
        // Show the assign-agent inline prompt
        let assign_block = Block::bordered()
            .title(Span::styled(" Assign to agent: ", theme.section_header()))
            .border_style(theme.warning());
        let assign_inner = assign_block.inner(sections[hints_section]);
        frame.render_widget(assign_block, sections[hints_section]);
        frame.render_widget(
            Paragraph::new(format!("{}\u{2588}", tui_state.job_assign_buffer)).style(theme.text()),
            assign_inner,
        );
    } else {
        // Show keybinding hints (active keys only; planned keys dimmed)
        let hint_line = Line::from(vec![
            Span::styled(" j/k", theme.accent()),
            Span::styled(":navigate  ", theme.muted()),
            Span::styled("Enter", theme.accent()),
            Span::styled(":detail  ", theme.muted()),
            Span::styled("n", theme.accent()),
            Span::styled(":new  ", theme.muted()),
            Span::styled("r", theme.accent()),
            Span::styled(":refresh", theme.muted()),
        ]);
        frame.render_widget(
            Paragraph::new(hint_line).alignment(Alignment::Center),
            sections[hints_section],
        );
    }
}

fn render_create_job(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let block = Block::bordered()
        .title(Span::styled(" New Job ", theme.section_header()))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 10 || inner.width < 30 {
        frame.render_widget(
            Paragraph::new("Terminal too small for form.")
                .style(theme.muted())
                .alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let sections = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Length(3), // Type
        Constraint::Length(3), // Priority
        Constraint::Length(5), // Description (taller)
        Constraint::Length(3), // Buttons / hints
        Constraint::Min(0),    // Padding
    ])
    .split(inner);

    let fields: &[(JobFormField, &str, &str)] = &[
        (JobFormField::Title, "Title", &tui_state.job_form_title),
        (
            JobFormField::Type,
            "Type [coding_task|research|other]",
            &tui_state.job_form_type,
        ),
        (
            JobFormField::Priority,
            "Priority [low|medium|high|critical]",
            &tui_state.job_form_priority,
        ),
        (
            JobFormField::Description,
            "Description",
            &tui_state.job_form_description,
        ),
    ];

    for (i, &(field, label, value)) in fields.iter().enumerate() {
        let is_focused = tui_state.job_form_focus == field;
        let is_editing = is_focused && tui_state.job_form_editing;

        let border_style = if is_editing {
            theme.warning()
        } else if is_focused {
            theme.accent()
        } else {
            theme.muted()
        };

        let field_block = Block::bordered()
            .title(Span::styled(
                format!(" {label} "),
                if is_focused {
                    theme.section_header()
                } else {
                    theme.label()
                },
            ))
            .border_style(border_style);
        let field_inner = field_block.inner(sections[i]);
        frame.render_widget(field_block, sections[i]);

        let display_value = if is_editing {
            format!("{value}\u{2588}") // block cursor
        } else if value.is_empty() {
            "(empty)".to_string()
        } else {
            value.to_string()
        };
        let text_style = if value.is_empty() && !is_editing {
            theme.metadata()
        } else {
            theme.value()
        };

        frame.render_widget(
            Paragraph::new(display_value)
                .style(text_style)
                .wrap(Wrap { trim: false }),
            field_inner,
        );
    }

    // Hints
    let hint_line = Line::from(vec![
        Span::styled(" Tab", theme.accent()),
        Span::styled(":next  ", theme.muted()),
        Span::styled("Enter", theme.accent()),
        Span::styled(":edit  ", theme.muted()),
        Span::styled("Ctrl-S", theme.accent()),
        Span::styled(":submit  ", theme.muted()),
        Span::styled("Esc", theme.accent()),
        Span::styled(":cancel", theme.muted()),
    ]);
    frame.render_widget(
        Paragraph::new(hint_line).alignment(Alignment::Center),
        sections[4],
    );

    // Show command results feedback, or fallback instructions for backend submission.
    if let Some(result) = tui_state.command_results.last() {
        let style = if result.ok {
            theme.success()
        } else {
            theme.danger()
        };
        frame.render_widget(
            Paragraph::new(format!("{}: {}", result.label, result.message))
                .style(style)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false }),
            sections[5],
        );
    } else {
        let sep_w = sections[5].width as usize;
        let help_lines = vec![
            Line::from(Span::styled(
                "─".repeat(sep_w),
                Style::default().fg(Theme::SEPARATOR),
            )),
            Line::from(Span::styled("Create a job from the CLI:", theme.label())),
            Line::from(Span::styled(
                "  roko serve                               # start the server",
                theme.code_block(),
            )),
            Line::from(Span::styled(
                "  curl -X POST http://localhost:6677/api/jobs \\",
                theme.code_block(),
            )),
            Line::from(Span::styled(
                "    -H \"Content-Type: application/json\" \\",
                theme.code_block(),
            )),
            Line::from(Span::styled(
                "    -d '{\"title\":\"...\", \"job_type\":\"research\"}'",
                theme.code_block(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Or use the roko-serve API directly.",
                theme.metadata(),
            )),
            Line::from(Span::styled(
                "Jobs appear here when created via the API.",
                theme.metadata(),
            )),
        ];
        frame.render_widget(
            Paragraph::new(help_lines).wrap(Wrap { trim: false }),
            sections[5],
        );
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

use crate::tui::display_utils::truncate;
