//! Scrollable plan detail modal.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::tui::dashboard::Theme;
use crate::tui::state::{GateResultEntry, PlanBudgetSummary, PlanEntry, TaskStatus, TuiState};

/// Render the plan detail modal overlay.
pub fn render_plan_detail_modal(
    frame: &mut Frame<'_>,
    area: Rect,
    plan_id: &str,
    plan: Option<&PlanEntry>,
    tui_state: &TuiState,
    scroll: u16,
    theme: &Theme,
) {
    let popup = centered_rect(86, 84, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Plan Detail: {} ", plan_id))
        .title_alignment(Alignment::Center)
        .border_style(theme.warning());

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Two-chunk layout: scrollable body + fixed footer.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    match plan {
        Some(plan) => render_plan(frame, &chunks, plan, tui_state, scroll, theme),
        None => render_missing_plan(frame, &chunks, plan_id, theme),
    }
}

fn render_plan(
    frame: &mut Frame<'_>,
    chunks: &[Rect],
    plan: &PlanEntry,
    tui_state: &TuiState,
    scroll: u16,
    theme: &Theme,
) {
    let pct = if plan.tasks_total > 0 {
        plan.tasks_done as f64 / plan.tasks_total as f64
    } else {
        0.0
    };

    let budget = tui_state.plan_budget_summary(plan);

    // Collect gate results for this plan.
    let plan_gates: Vec<&GateResultEntry> = tui_state
        .gate_results
        .iter()
        .filter(|g| g.plan_id == plan.id)
        .collect();

    let mut lines: Vec<Line> = Vec::new();

    // ── Header ──────────────────────────────────────────────────────
    let title = if plan.name.is_empty() {
        plan.id.as_str()
    } else {
        plan.name.as_str()
    };
    lines.push(Line::from(Span::styled(
        title,
        theme.accent().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "\u{2550}".repeat(chunks[0].width.saturating_sub(1) as usize),
        theme.muted(),
    )));

    // ── Status section ──────────────────────────────────────────────
    let (status_badge, status_style) = if plan.status.is_done() {
        ("COMPLETE", theme.success())
    } else if plan.status.is_failed() {
        ("FAILED", theme.danger())
    } else if plan.status.is_active() || plan.active {
        ("RUNNING", theme.warning())
    } else {
        ("PENDING", theme.muted())
    };

    lines.push(Line::from(vec![
        Span::styled("Status ", theme.muted()),
        Span::styled(
            format!(" {} ", status_badge),
            status_style.add_modifier(Modifier::BOLD),
        ),
        Span::styled("  Phase ", theme.muted()),
        Span::styled(&plan.phase, theme.text().add_modifier(Modifier::BOLD)),
        Span::styled("  Wave ", theme.muted()),
        Span::styled(
            plan.wave
                .map(|w| w.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            theme.text(),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("ID     ", theme.muted()),
        Span::styled(plan.id.as_str(), theme.text()),
    ]));

    // ── Cost section ────────────────────────────────────────────────
    let cost_line = format_cost_line(&budget);
    lines.push(Line::from(vec![
        Span::styled("Cost   ", theme.muted()),
        Span::styled(cost_line, theme.text().add_modifier(Modifier::BOLD)),
    ]));

    // ── Progress bar ────────────────────────────────────────────────
    let bar_width = chunks[0].width.saturating_sub(12).min(40) as usize;
    let progress_bar = build_progress_bar(pct, bar_width);
    lines.push(Line::from(vec![
        Span::styled("       ", theme.muted()),
        Span::styled(progress_bar, theme.info()),
        Span::styled(
            format!(" {:.0}%", pct * 100.0),
            theme.text().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({}/{})", plan.tasks_done, plan.tasks_total),
            theme.muted(),
        ),
    ]));

    // ── Timing ──────────────────────────────────────────────────────
    lines.push(Line::from(vec![
        Span::styled("Time   ", theme.muted()),
        Span::styled(format_elapsed(plan.elapsed_secs), theme.text()),
    ]));

    // ── Branch / worktree / commit ──────────────────────────────────
    if plan.branch.is_some() || plan.worktree_path.is_some() || plan.last_commit.is_some() {
        let mut spans = Vec::new();
        if let Some(branch) = &plan.branch {
            spans.push(Span::styled("Branch ", theme.muted()));
            spans.push(Span::styled(branch.as_str(), theme.info()));
        }
        if let Some(commit) = &plan.last_commit {
            if !spans.is_empty() {
                spans.push(Span::styled("  ", theme.text()));
            }
            spans.push(Span::styled("Commit ", theme.muted()));
            spans.push(Span::styled(commit.as_str(), theme.text()));
        }
        if let Some(wt) = &plan.worktree_path {
            if !spans.is_empty() {
                spans.push(Span::styled("  ", theme.text()));
            }
            spans.push(Span::styled("WT ", theme.muted()));
            spans.push(Span::styled(wt.as_str(), theme.text()));
        }
        lines.push(Line::from(spans));
    }

    // ── Changes stats ───────────────────────────────────────────────
    if let Some(files) = plan.files_modified {
        let mut spans = vec![
            Span::styled("Chg    ", theme.muted()),
            Span::styled(format!("{} file(s)", files), theme.text()),
        ];
        if let Some(ins) = plan.insertions {
            spans.push(Span::styled(format!("  +{ins}"), theme.success()));
        }
        if let Some(del) = plan.deletions {
            spans.push(Span::styled(format!("  -{del}"), theme.danger()));
        }
        lines.push(Line::from(spans));
    }

    // ── Failure summary ─────────────────────────────────────────────
    if plan.tasks_failed > 0 {
        let failed_tasks: Vec<&str> = plan
            .tasks
            .iter()
            .filter(|t| t.status.is_failed())
            .map(|t| {
                if t.name.is_empty() {
                    t.id.as_str()
                } else {
                    t.name.as_str()
                }
            })
            .collect();
        let sample = failed_tasks
            .iter()
            .take(3)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if failed_tasks.len() > 3 { " ..." } else { "" };
        lines.push(Line::from(vec![
            Span::styled("Fail   ", theme.danger()),
            Span::styled(
                format!("{} failed: {}{}", plan.tasks_failed, sample, suffix),
                theme.danger(),
            ),
        ]));
    }

    lines.push(Line::from(""));

    // ── Tasks section ───────────────────────────────────────────────
    if !plan.tasks.is_empty() {
        lines.push(Line::from(Span::styled(
            "Tasks",
            theme.accent().add_modifier(Modifier::BOLD),
        )));

        // Column header.
        lines.push(Line::from(vec![
            Span::styled("  ", theme.text()),
            Span::styled(format!("{:<24}", "Task"), theme.muted()),
            Span::styled(format!("{:<10}", "Status"), theme.muted()),
            Span::styled(format!("{:<16}", "Agent"), theme.muted()),
            Span::styled("Cost", theme.muted()),
        ]));

        for task in &plan.tasks {
            let icon = match task.status {
                TaskStatus::Done => "\u{2713}",
                TaskStatus::Failed | TaskStatus::Blocked => "\u{2717}",
                TaskStatus::Active => "\u{25B6}",
                TaskStatus::Pending => "\u{25CB}",
            };
            let status_style = match task.status {
                TaskStatus::Done => theme.success(),
                TaskStatus::Failed | TaskStatus::Blocked => theme.danger(),
                TaskStatus::Active => theme.warning(),
                TaskStatus::Pending => theme.muted(),
            };

            let agent_display = task.agent_id.as_deref().unwrap_or("-");

            let task_cost_key = format!("{}:{}", plan.id, task.id);
            let task_cost = tui_state
                .cost_per_task
                .get(&task_cost_key)
                .copied()
                .unwrap_or(0.0);
            let cost_str = if task_cost > 0.0 {
                format!("${:.2}", task_cost)
            } else {
                "-".to_string()
            };

            let task_label = if task.name.is_empty() {
                task.id.as_str()
            } else {
                task.name.as_str()
            };
            // Truncate long task names to fit columns.
            let task_label_trunc: String = if task_label.len() > 22 {
                format!("{}..", &task_label[..20])
            } else {
                task_label.to_string()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{icon} "), status_style),
                Span::styled(
                    format!("{:<24}", task_label_trunc),
                    theme.text().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{:<10}", task.status.label()), status_style),
                Span::styled(format!("{:<16}", agent_display), theme.muted()),
                Span::styled(cost_str, theme.text()),
            ]));

            // Dependencies.
            if !task.depends_on.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("    deps: ", theme.muted()),
                    Span::styled(task.depends_on.join(", "), theme.text()),
                ]));
            }

            // Acceptance criteria (may contain markdown).
            if let Some(acceptance) = &task.acceptance_text {
                if acceptance.contains('\n')
                    || acceptance.contains("**")
                    || acceptance.contains('`')
                    || acceptance.contains("# ")
                {
                    lines.push(Line::from(Span::styled("    accept:", theme.muted())));
                    for md_line in markdown_to_lines(acceptance, theme) {
                        let mut indented = vec![Span::raw("      ".to_string())];
                        indented.extend(md_line.spans);
                        lines.push(Line::from(indented));
                    }
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("    accept: ", theme.muted()),
                        Span::styled(acceptance.clone(), theme.text()),
                    ]));
                }
            }

            // Verify command.
            if let Some(verify) = &task.verify_command {
                lines.push(Line::from(vec![
                    Span::styled("    verify: ", theme.muted()),
                    Span::styled(verify.as_str(), theme.info()),
                ]));
            }

            // Start time.
            if let Some(started) = &task.started_at {
                lines.push(Line::from(vec![
                    Span::styled("    started: ", theme.muted()),
                    Span::styled(started.as_str(), theme.text()),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No tasks recorded for this plan.",
            theme.muted(),
        )));
    }

    // ── Gate results ────────────────────────────────────────────────
    if !plan_gates.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Gate Results",
            theme.accent().add_modifier(Modifier::BOLD),
        )));

        lines.push(Line::from(vec![
            Span::styled("  ", theme.text()),
            Span::styled(format!("{:<16}", "Gate"), theme.muted()),
            Span::styled(format!("{:<10}", "Result"), theme.muted()),
            Span::styled("Detail", theme.muted()),
        ]));

        for gate in &plan_gates {
            let (result_str, result_style) = if gate.passed {
                ("PASS", theme.success())
            } else {
                ("FAIL", theme.danger())
            };
            let detail = if gate.output.len() > 40 {
                format!("{}..", &gate.output[..38])
            } else {
                gate.output.clone()
            };

            lines.push(Line::from(vec![
                Span::styled("  ", theme.text()),
                Span::styled(format!("{:<16}", gate.gate), theme.text()),
                Span::styled(
                    format!("{:<10}", result_str),
                    result_style.add_modifier(Modifier::BOLD),
                ),
                Span::styled(detail, theme.muted()),
            ]));
        }
    }

    lines.push(Line::from(""));

    // ── Scrollable body ─────────────────────────────────────────────
    let paragraph = Paragraph::new(lines)
        .style(theme.text())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, chunks[0]);

    // ── Footer ──────────────────────────────────────────────────────
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc]", theme.accent_bold()),
            Span::styled(" close  ", theme.muted()),
            Span::styled("[j/k]", theme.accent_bold()),
            Span::styled(" scroll  ", theme.muted()),
            Span::styled("[Tab]", theme.accent_bold()),
            Span::styled(" sub-tab", theme.muted()),
        ])),
        chunks[1],
    );
}

fn render_missing_plan(frame: &mut Frame<'_>, chunks: &[Rect], plan_id: &str, theme: &Theme) {
    let body = Paragraph::new(vec![
        Line::from(Span::styled("Plan not found", theme.danger())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Plan ID: ", theme.muted()),
            Span::styled(plan_id, theme.text()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "The plan snapshot is no longer available in TuiState.",
            theme.muted(),
        )),
    ])
    .wrap(Wrap { trim: false });

    frame.render_widget(body, chunks[0]);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc]", theme.accent_bold()),
            Span::styled(" close", theme.muted()),
        ])),
        chunks[1],
    );
}

/// Convert raw markdown text into styled ratatui [`Line`]s.
///
/// Handles headers (`#`, `##`, `###`), fenced code blocks, bold (`**...**`),
/// inline code (`` `...` ``), bullet lists (`- ` / `* `), and numbered lists.
fn markdown_to_lines<'a>(text: &str, theme: &Theme) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();
    let mut in_code_block = false;

    for raw in text.lines() {
        // -- fenced code blocks --
        if raw.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("  {raw}"),
                theme.code_block(),
            )));
            continue;
        }

        // -- headers --
        if let Some(rest) = raw.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                rest.to_string(),
                theme.section_header(),
            )));
            continue;
        }
        if let Some(rest) = raw.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                rest.to_string(),
                theme.section_header(),
            )));
            continue;
        }
        if let Some(rest) = raw.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                rest.to_string(),
                theme.section_header(),
            )));
            lines.push(Line::from(Span::styled(
                "\u{2500}".repeat(rest.len().max(20)),
                theme.muted(),
            )));
            continue;
        }

        // -- bullet lists --
        let trimmed = raw.trim_start();
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let bullet_text = &trimmed[2..];
            let mut spans = vec![Span::styled("  \u{2022} ".to_string(), theme.label())];
            spans.extend(inline_markdown_spans(bullet_text, theme));
            lines.push(Line::from(spans));
            continue;
        }

        // -- numbered lists (e.g. "1. ") --
        if let Some(dot_pos) = trimmed.find(". ") {
            if dot_pos <= 3 && trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                let prefix = &trimmed[..dot_pos + 2];
                let rest = &trimmed[dot_pos + 2..];
                let mut spans = vec![Span::styled(format!("  {prefix}"), theme.label())];
                spans.extend(inline_markdown_spans(rest, theme));
                lines.push(Line::from(spans));
                continue;
            }
        }

        // -- plain text with inline formatting --
        if trimmed.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(inline_markdown_spans(raw, theme)));
        }
    }
    lines
}

/// Parse inline markdown (`**bold**` and `` `code` ``) into styled spans.
fn inline_markdown_spans<'a>(text: &str, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the earliest inline marker.
        let bold_pos = remaining.find("**");
        let code_pos = remaining.find('`');

        let next = match (bold_pos, code_pos) {
            (Some(b), Some(c)) => {
                if b <= c {
                    Some(("**", b))
                } else {
                    Some(("`", c))
                }
            }
            (Some(b), None) => Some(("**", b)),
            (None, Some(c)) => Some(("`", c)),
            (None, None) => None,
        };

        match next {
            None => {
                spans.push(Span::styled(remaining.to_string(), theme.text()));
                break;
            }
            Some((marker, pos)) => {
                // Push text before the marker.
                if pos > 0 {
                    spans.push(Span::styled(remaining[..pos].to_string(), theme.text()));
                }
                let after_open = &remaining[pos + marker.len()..];
                if let Some(close) = after_open.find(marker) {
                    let inner = &after_open[..close];
                    let style = if marker == "**" {
                        theme.text().add_modifier(Modifier::BOLD)
                    } else {
                        theme.code_block()
                    };
                    spans.push(Span::styled(inner.to_string(), style));
                    remaining = &after_open[close + marker.len()..];
                } else {
                    // No closing marker — emit the rest as plain text.
                    spans.push(Span::styled(remaining[pos..].to_string(), theme.text()));
                    break;
                }
            }
        }
    }
    spans
}

/// Build a visual progress bar using block characters.
fn build_progress_bar(pct: f64, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let filled = (pct * width as f64).round() as usize;
    let mut bar = String::with_capacity(width);
    for i in 0..width {
        if i < filled {
            bar.push('\u{2588}'); // full block
        } else if i == filled {
            // Partial block based on fractional part.
            let frac = (pct * width as f64) - filled as f64;
            if frac > 0.5 {
                bar.push('\u{2593}'); // dark shade
            } else if frac > 0.25 {
                bar.push('\u{2592}'); // medium shade
            } else {
                bar.push('\u{2591}'); // light shade
            }
        } else {
            bar.push('\u{2591}'); // light shade
        }
    }
    bar
}

/// Format cost/budget as a display string.
fn format_cost_line(budget: &PlanBudgetSummary) -> String {
    let spent = format!("${:.2}", budget.spent_usd);
    let budget_str = if budget.budget_usd > 0.0 {
        format!("${:.2}", budget.budget_usd)
    } else {
        "unlimited".to_string()
    };
    let projected = if budget.projected_total_usd > 0.0 {
        format!("${:.2}", budget.projected_total_usd)
    } else {
        "n/a".to_string()
    };
    format!("{spent} / {budget_str}  projected: {projected}")
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
