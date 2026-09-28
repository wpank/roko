//! F2 Plans view -- Mori-style wave browser + plan detail.
//!
//! Layout: left 31% (wave list with pipeline header + collapsible plan
//! groups), one-cell VOID gutter, right detail (tasks, gate results,
//! timing).
//!
//! Renders hierarchical wave groups with gradient progress bars, status
//! icons, phase indicators, and timing matching the Mori Plans screen (F2).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap};
use std::collections::{HashMap, HashSet};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::empty_state::render_pane_empty_compact;
use crate::tui::input::FocusZone;
use crate::tui::state::{AgentStatus, PlanEntry, TaskEntry, TaskStatus, TuiState};
use crate::tui::util::truncate_middle;

// ---------------------------------------------------------------------------
// Task dependency tree (F2 right panel, P1-TUI-G3)
// ---------------------------------------------------------------------------

/// Flattened entry produced by the dependency-tree DFS.
struct TaskTreeRow<'a> {
    task: &'a TaskEntry,
    /// Tree depth (0 = root task with no parents in this plan).
    depth: usize,
    /// `connector[i]` is true when the ancestor at depth i was the last child
    /// of its parent — controls whether we draw │ or space at that column.
    connector: Vec<bool>,
}

/// Build a topological dependency tree from the plan's task list.
///
/// Returns tasks in DFS pre-order (parent before children).  Tasks whose
/// `depends_on` entries are absent from the plan are treated as roots.
/// Falls back to flat order (depth=0) when no dependencies are declared.
fn build_task_dep_tree<'a>(tasks: &'a [TaskEntry]) -> Vec<TaskTreeRow<'a>> {
    let has_deps = tasks.iter().any(|t| !t.depends_on.is_empty());
    if !has_deps {
        return tasks
            .iter()
            .map(|t| TaskTreeRow {
                task: t,
                depth: 0,
                connector: vec![],
            })
            .collect();
    }

    let id_to_idx: HashMap<&str, usize> = tasks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.as_str(), i))
        .collect();

    // children[parent_idx] = [child_idx, ...]
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut has_parent: HashSet<usize> = HashSet::new();
    for (child_idx, task) in tasks.iter().enumerate() {
        for dep_id in &task.depends_on {
            if let Some(&parent_idx) = id_to_idx.get(dep_id.as_str()) {
                children.entry(parent_idx).or_default().push(child_idx);
                has_parent.insert(child_idx);
            }
        }
    }

    let roots: Vec<usize> = (0..tasks.len())
        .filter(|i| !has_parent.contains(i))
        .collect();

    let mut result: Vec<TaskTreeRow<'a>> = Vec::with_capacity(tasks.len());
    let mut visited: HashSet<usize> = HashSet::new();

    fn visit<'a>(
        idx: usize,
        depth: usize,
        connector: Vec<bool>,
        tasks: &'a [TaskEntry],
        children: &HashMap<usize, Vec<usize>>,
        visited: &mut HashSet<usize>,
        result: &mut Vec<TaskTreeRow<'a>>,
    ) {
        if visited.contains(&idx) {
            return;
        }
        visited.insert(idx);
        result.push(TaskTreeRow {
            task: &tasks[idx],
            depth,
            connector: connector.clone(),
        });
        let kids = children.get(&idx).map(|v| v.as_slice()).unwrap_or(&[]);
        for (i, &kid_idx) in kids.iter().enumerate() {
            let is_last = i + 1 == kids.len();
            let mut kid_connector = connector.clone();
            kid_connector.push(is_last);
            visit(
                kid_idx,
                depth + 1,
                kid_connector,
                tasks,
                children,
                visited,
                result,
            );
        }
    }

    for root_idx in roots {
        visit(
            root_idx,
            0,
            vec![],
            tasks,
            &children,
            &mut visited,
            &mut result,
        );
    }
    // Append any tasks not reached (cycles / orphan refs) flat.
    for (idx, task) in tasks.iter().enumerate() {
        if !visited.contains(&idx) {
            result.push(TaskTreeRow {
                task,
                depth: 0,
                connector: vec![],
            });
        }
    }
    result
}

/// Build the tree-connector prefix for one `TaskTreeRow` (e.g. `"    ├── "`).
fn task_tree_prefix(row: &TaskTreeRow<'_>) -> String {
    if row.depth == 0 {
        return String::new();
    }
    let mut prefix = String::new();
    for depth_i in 0..row.depth.saturating_sub(1) {
        let ancestor_is_last = row.connector.get(depth_i).copied().unwrap_or(true);
        if ancestor_is_last {
            prefix.push_str("    ");
        } else {
            prefix.push_str("\u{2502}   ");
        }
    }
    let is_last = row.connector.last().copied().unwrap_or(true);
    if is_last {
        prefix.push_str("\u{2514}\u{2500}\u{2500} ");
    } else {
        prefix.push_str("\u{251C}\u{2500}\u{2500} ");
    }
    prefix
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Fractional block characters for smooth progress bars.
const BLOCKS: &[char] = &[
    ' ', '\u{2591}', '\u{258F}', '\u{258E}', '\u{258D}', '\u{258C}', '\u{258B}', '\u{258A}',
    '\u{2589}', '\u{2588}',
];

// ---------------------------------------------------------------------------
// Public render
// ---------------------------------------------------------------------------

/// Render the full plans view.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let (sidebar, detail) =
        crate::tui::layout::responsive_panel_split(area, 31, 100, area.height / 3);
    render_left_panel(frame, sidebar, _data, tui_state, view_state, theme);
    render_right_panel(frame, detail, _data, tui_state, view_state, theme);
}

// ---------------------------------------------------------------------------
// Left panel: compact pipeline row + wave/plan tree
// ---------------------------------------------------------------------------

fn render_left_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    render_wave_tree(frame, area, _data, tui_state, view_state, theme);
}

// ---------------------------------------------------------------------------
// Wave tree: hierarchical wave -> plan list
// ---------------------------------------------------------------------------

/// Live plan entries keyed by plan id, with each entry's index in
/// `tui_state.plans` (the index `selected_plan_idx` refers to).
fn plan_entries_by_id(tui_state: &TuiState) -> HashMap<&str, (usize, &PlanEntry)> {
    tui_state
        .plans
        .iter()
        .enumerate()
        .map(|(index, plan)| (plan.id.as_str(), (index, plan)))
        .collect()
}

fn render_wave_tree(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let focused = matches!(tui_state.focus, FocusZone::PlanTree);
    // Rows render `plan_summaries`; live status comes from the `PlanEntry`
    // with the same plan id — the two lists are never aligned by position.
    let entries = plan_entries_by_id(tui_state);
    let entry_at = |summary_idx: usize| {
        tui_state
            .plan_summaries
            .get(summary_idx)
            .and_then(|summary| entries.get(summary.id.as_str()).copied())
    };
    let is_selected_at = |summary_idx: usize| {
        entry_at(summary_idx).is_some_and(|(entry_idx, _)| entry_idx == view_state.selected)
    };
    let total_plans = tui_state.plan_summaries.len();
    let completed = tui_state
        .plan_summaries
        .iter()
        .filter(|p| p.completed)
        .count();
    let failed = tui_state
        .plans
        .iter()
        .filter(|p| p.status.is_failed())
        .count();

    let mut health_suffix = String::new();
    let active = tui_state
        .plans
        .iter()
        .filter(|p| p.status.is_active())
        .count();
    if active > 0 {
        health_suffix.push_str(&format!(" {active}\u{25b8}"));
    }
    if failed > 0 {
        health_suffix.push_str(&format!(" {failed}\u{2717}"));
    }

    // Filter state for F2 left panel (uses the same plan_tree_filter as F1).
    let filter_active =
        tui_state.plan_tree_filter.active && !tui_state.plan_tree_filter.pattern.is_empty();
    let filtered_suffix = if filter_active {
        let filtered_count = tui_state
            .plan_summaries
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                entry_at(*i)
                    .map(|(_, p)| tui_state.plan_tree_filter.matches_plan_or_tasks(p))
                    .unwrap_or(true)
            })
            .count();
        format!(", {filtered_count}/{total_plans} filtered")
    } else {
        String::new()
    };

    let title = if focused {
        format!(
            " Plans ({completed}/{total_plans}{health_suffix}{filtered_suffix}) [/:filter Enter:detail] "
        )
    } else {
        format!(" Plans ({completed}/{total_plans}{health_suffix}{filtered_suffix}) ")
    };

    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::unfocused_border_style()
    };
    let title_style = if focused || active > 0 {
        if focused {
            Theme::focused_title_style()
        } else {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        }
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

    if tui_state.plan_summaries.is_empty() {
        crate::tui::empty_state::render_empty_state(
            frame,
            inner,
            crate::tui::tabs::Tab::Plans,
            &tui_state.atmosphere,
        );
        return;
    }

    let content_width = inner.width as usize;
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Filter indicator (shared with plan_tree.rs widget)
    if filter_active {
        lines.push(Line::from(vec![
            Span::styled(" /", Style::default().fg(Theme::DREAM)),
            Span::styled(
                tui_state.plan_tree_filter.pattern.clone(),
                Style::default().fg(theme.foreground),
            ),
            Span::styled("/ ", Style::default().fg(Theme::DREAM)),
        ]));
    }

    // Mori keeps pipeline state inside the plan tree rather than spending two
    // additional bordered panels on information repeated by the detail pane.
    let pct = if total_plans > 0 {
        completed as f64 / total_plans as f64
    } else {
        0.0
    };
    let pipeline_color = progress_color(pct, total_plans, completed, theme);
    let pipeline_bar_width = content_width.saturating_sub(22).clamp(4, 12);
    lines.push(Line::from(vec![
        Span::styled(" \u{25c8} Pipeline ", Style::default().fg(Theme::DREAM)),
        Span::styled(
            build_progress_bar(pct, pipeline_bar_width),
            Style::default().fg(pipeline_color),
        ),
        Span::styled(
            format!(" {completed}/{total_plans}"),
            Style::default()
                .fg(pipeline_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // Column header
    if inner.height > 4 && content_width >= 30 {
        let name_w = content_width.saturating_sub(20);
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:<name_w$}", "plan"),
                Style::default().fg(Theme::COL_HEADER),
            ),
            Span::styled(
                format!("{:>6}", "prog"),
                Style::default().fg(Theme::COL_HEADER),
            ),
            Span::styled(
                format!("{:>8}", "bar"),
                Style::default().fg(Theme::COL_HEADER),
            ),
        ]));
    }

    // Build wave groups from the ordered plan list.
    //
    // Use real wave data from PlanEntry::wave when any plan has a wave
    // assignment. Fall back to synthetic wave groups of ~4 plans each
    // when wave data is absent (wave computation not yet done).
    let has_real_waves = tui_state.plans.iter().any(|p| p.wave.is_some());
    let use_waves = tui_state.plan_summaries.len() > 3;

    if use_waves {
        // Build wave groups: real data or synthetic fallback.
        let wave_groups: Vec<(usize, Vec<usize>)> = if has_real_waves {
            // Group plan indices by their wave assignment. Plans without a
            // PlanEntry (summary-only) go to wave 0.
            let mut groups: std::collections::BTreeMap<usize, Vec<usize>> =
                std::collections::BTreeMap::new();
            for i in 0..tui_state.plan_summaries.len() {
                let wave_idx = entry_at(i).and_then(|(_, plan)| plan.wave).unwrap_or(0);
                groups.entry(wave_idx).or_default().push(i);
            }
            groups.into_iter().collect()
        } else {
            // Synthetic groups of ~4 plans each.
            let wave_size = 4usize;
            let num_waves = (tui_state.plan_summaries.len() + wave_size - 1) / wave_size;
            (0..num_waves)
                .map(|w| {
                    let start = w * wave_size;
                    let end = (start + wave_size).min(tui_state.plan_summaries.len());
                    (w, (start..end).collect())
                })
                .collect()
        };

        for (wave_idx, plan_indices) in &wave_groups {
            let wave_done = plan_indices
                .iter()
                .filter(|&&i| {
                    tui_state
                        .plan_summaries
                        .get(i)
                        .map(|p| p.completed)
                        .unwrap_or(false)
                })
                .count();
            let wave_total = plan_indices.len();
            let all_done = wave_done == wave_total;
            let any_active = plan_indices
                .iter()
                .any(|&i| entry_at(i).is_some_and(|(_, p)| p.status.is_active()));

            // Is this wave selected (contains selected plan)?
            let wave_selected = plan_indices.iter().any(|&i| is_selected_at(i));
            // Default: expand selected wave and completed waves, collapse others.
            // Respect the user's explicit collapse toggle.
            let explicitly_collapsed = tui_state.collapsed_waves.contains(wave_idx);
            let expanded = !explicitly_collapsed && (wave_selected || all_done || any_active);

            // Wave header
            let (wave_icon, wave_style) = if all_done {
                (
                    "\u{2713}", // checkmark
                    Style::default().fg(theme.success),
                )
            } else if any_active {
                (
                    "\u{25b6}", // ▶
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    "\u{25cb}", // ○
                    Style::default().fg(theme.muted),
                )
            };

            let collapse_icon = if expanded {
                "\u{25be}" // ▾
            } else {
                "\u{25b8}" // ▸
            };

            // Wave progress bar (8-char)
            let wave_fill = wave_done as f64 / wave_total.max(1) as f64;
            let wave_bar = build_mini_bar(8, wave_fill, all_done, any_active, theme);

            // Count failed in wave
            let wave_failed = plan_indices
                .iter()
                .filter(|&&i| entry_at(i).is_some_and(|(_, p)| p.status.is_failed()))
                .count();

            let mut wave_spans = vec![
                Span::styled(
                    format!(" {collapse_icon} "),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(format!("{wave_icon} "), wave_style),
                Span::styled(
                    format!("Wave {} ", wave_idx),
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("({wave_done}/{wave_total}) "),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(
                    wave_bar,
                    Style::default().fg(if all_done {
                        theme.success
                    } else if any_active {
                        theme.accent
                    } else {
                        theme.muted
                    }),
                ),
            ];

            if wave_failed > 0 {
                wave_spans.push(Span::styled(
                    format!(" \u{2717}{wave_failed}"),
                    Style::default().fg(theme.danger),
                ));
            }

            // "after W{N}" blocker label for pending/non-started waves
            if !all_done && !any_active && *wave_idx > 0 {
                // Find the highest incomplete predecessor wave
                let blocker_waves: Vec<usize> = wave_groups
                    .iter()
                    .filter(|(w, indices)| {
                        *w < *wave_idx
                            && indices.iter().any(|&i| {
                                tui_state
                                    .plan_summaries
                                    .get(i)
                                    .map(|p| !p.completed)
                                    .unwrap_or(false)
                            })
                    })
                    .map(|(w, _)| *w)
                    .collect();
                if !blocker_waves.is_empty() {
                    let blocker_label = blocker_waves
                        .iter()
                        .map(|w| format!("W{w}"))
                        .collect::<Vec<_>>()
                        .join(",");
                    wave_spans.push(Span::styled(
                        format!(" after {blocker_label}"),
                        Style::default().fg(theme.muted),
                    ));
                }
            }

            // Fill remaining width with horizontal line
            let used: usize = wave_spans.iter().map(|s| s.content.chars().count()).sum();
            let avail = content_width;
            if avail > used + 1 {
                wave_spans.push(Span::styled(
                    format!(" {}", "\u{2500}".repeat(avail - used - 1)),
                    Style::default().fg(Theme::SEPARATOR),
                ));
            }
            lines.push(Line::from(wave_spans));

            if !expanded {
                continue;
            }

            // Plans within wave (apply filter)
            for &i in plan_indices {
                let entry = entry_at(i).map(|(_, p)| p);
                if filter_active
                    && entry.is_some_and(|p| !tui_state.plan_tree_filter.matches_plan_or_tasks(p))
                {
                    continue;
                }
                if let Some(plan) = tui_state.plan_summaries.get(i) {
                    render_plan_line(
                        &mut lines,
                        plan,
                        entry,
                        is_selected_at(i),
                        theme,
                        content_width,
                        true,
                    );
                }
            }
        }
    } else {
        // Flat list (apply filter)
        for (i, plan) in tui_state.plan_summaries.iter().enumerate() {
            let entry = entry_at(i).map(|(_, p)| p);
            if filter_active
                && entry.is_some_and(|p| !tui_state.plan_tree_filter.matches_plan_or_tasks(p))
            {
                continue;
            }
            render_plan_line(
                &mut lines,
                plan,
                entry,
                is_selected_at(i),
                theme,
                content_width,
                false,
            );
        }
    }

    // Scroll
    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let scroll_offset =
        (view_state.scroll as usize).min(total_lines.saturating_sub(visible_height));
    let visible: Vec<Line<'_>> = lines
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible);
    frame.render_widget(paragraph, inner);

    // Scrollbar
    if total_lines > visible_height {
        render_scrollbar(
            frame,
            inner,
            total_lines,
            visible_height,
            scroll_offset,
            theme,
        );
    }
}

// ---------------------------------------------------------------------------
// Single plan line
// ---------------------------------------------------------------------------

/// Render one F2 plan row. `tui_plan` is the live entry for the same plan
/// id, when one exists.
fn render_plan_line(
    lines: &mut Vec<Line<'_>>,
    plan: &crate::plan::PlanSummary,
    tui_plan: Option<&PlanEntry>,
    is_selected: bool,
    theme: &Theme,
    content_width: usize,
    indented: bool,
) {
    let is_active = tui_plan.map(|p| p.status.is_active()).unwrap_or(false);
    let is_failed = tui_plan.map(|p| p.status.is_failed()).unwrap_or(false);
    let task_total = tui_plan.map(|p| p.tasks_total).unwrap_or(plan.task_count);
    let task_done =
        tui_plan
            .map(|p| p.tasks_done)
            .unwrap_or(if plan.completed { task_total } else { 0 });

    // Status icon
    let (icon, icon_style) = if plan.completed {
        (
            "\u{2713}", // checkmark
            Style::default().fg(theme.success),
        )
    } else if is_failed {
        (
            "\u{2717}", // X
            Style::default()
                .fg(theme.danger)
                .add_modifier(Modifier::BOLD),
        )
    } else if is_active {
        (
            "\u{25b6}", // ▶
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            "\u{25cb}", // ○
            Style::default().fg(theme.muted),
        )
    };

    // Text styling
    let text_style = if plan.completed {
        Style::default().fg(theme.success)
    } else if is_active {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else if is_failed {
        Style::default().fg(theme.danger)
    } else {
        Style::default().fg(theme.foreground)
    };

    let bg = if is_selected {
        theme.selection_background
    } else {
        Color::Reset
    };

    let indent = if indented { "   " } else { " " };

    // Progress fraction
    let fill_pct = if task_total > 0 {
        task_done as f64 / task_total as f64
    } else {
        if plan.completed { 1.0 } else { 0.0 }
    };

    // Progress cell
    let progress_str = if task_total > 0 {
        format!("{}/{}", task_done.min(99), task_total.min(99))
    } else {
        "\u{00b7}".to_string()
    };
    let progress_color = if plan.completed {
        theme.success
    } else if is_active {
        semantic_color(fill_pct, theme)
    } else if is_failed {
        theme.danger
    } else {
        theme.muted
    };

    // Bar cell (8 chars)
    let bar_w = 8usize;
    let filled = (fill_pct.clamp(0.0, 1.0) * bar_w as f64).round() as usize;
    let empty = bar_w.saturating_sub(filled);
    let bar_color = if plan.completed {
        theme.success
    } else if is_failed {
        theme.danger
    } else if is_active {
        semantic_color(fill_pct, theme)
    } else if task_done == 0 {
        Theme::SEPARATOR
    } else {
        semantic_color(fill_pct, theme)
    };

    // Name column budget
    let reserved = 20usize; // progress + bar + separators
    let name_budget = content_width
        .saturating_sub(indent.len() + 2 + reserved)
        .max(8);
    let plan_name = truncate_middle(&plan.title, name_budget);

    let sep_style = Style::default().fg(Theme::SEPARATOR);

    let mut spans = vec![
        Span::styled(indent.to_string(), Style::default().bg(bg)),
        Span::styled(format!("{icon} "), icon_style.bg(bg)),
        Span::styled(
            format!("{:<width$}", plan_name, width = name_budget),
            text_style.bg(bg),
        ),
        Span::styled("\u{2502}", sep_style.bg(bg)),
        Span::styled(
            format!("{:>6}", progress_str),
            Style::default().fg(progress_color).bg(bg),
        ),
        Span::styled("\u{2502}", sep_style.bg(bg)),
        Span::styled(
            format!(
                "{}{}",
                "\u{2588}".repeat(filled.min(bar_w)),
                "\u{2500}".repeat(empty)
            ),
            Style::default().fg(bar_color).bg(bg),
        ),
    ];

    // Task count
    if content_width > 45 {
        spans.push(Span::styled(
            format!(" {}t", task_total),
            Style::default().fg(theme.muted).bg(bg),
        ));
    }

    lines.push(Line::from(spans));

    // Selected plan detail row
    if is_selected {
        let mut detail_spans = vec![Span::styled(format!("{indent}  "), Style::default().bg(bg))];

        // Mini progress bar
        if task_total > 0 {
            let mini_filled = (fill_pct * 8.0).round() as usize;
            let mini_empty = 8usize.saturating_sub(mini_filled);
            detail_spans.push(Span::styled(
                format!(
                    " {}{}",
                    "\u{2588}".repeat(mini_filled.min(8)),
                    "\u{2500}".repeat(mini_empty)
                ),
                Style::default().fg(semantic_color(fill_pct, theme)).bg(bg),
            ));
            detail_spans.push(Span::styled("  ", Style::default().bg(bg)));
        }

        // Status label
        let status_label = if plan.completed {
            "done"
        } else if is_active {
            "running"
        } else if is_failed {
            "failed"
        } else {
            "pending"
        };
        detail_spans.push(Span::styled(
            status_label.to_string(),
            Style::default()
                .fg(if plan.completed {
                    theme.success
                } else if is_active {
                    theme.accent
                } else if is_failed {
                    theme.danger
                } else {
                    theme.muted
                })
                .bg(bg),
        ));

        if task_total > 0 {
            detail_spans.push(Span::styled(
                format!(" \u{00b7} {task_total} tasks"),
                Style::default().fg(theme.muted).bg(bg),
            ));
        }

        if plan.old_format {
            detail_spans.push(Span::styled(
                " \u{00b7} old format",
                Style::default().fg(theme.warning).bg(bg),
            ));
        }

        lines.push(Line::from(detail_spans));

        // Show last error as an additional detail line.
        if let Some(err) = &plan.last_error {
            let err_budget = content_width.saturating_sub(indent.len() + 6);
            lines.push(Line::from(vec![
                Span::styled(format!("{indent}  "), Style::default().bg(bg)),
                Span::styled(
                    "\u{26a0} ",
                    Style::default()
                        .fg(theme.danger)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    truncate(err, err_budget),
                    Style::default().fg(theme.danger).bg(bg),
                ),
            ]));
        }
    }
}

// ---------------------------------------------------------------------------
// Right panel: plan detail
// ---------------------------------------------------------------------------

fn render_right_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let focused = matches!(tui_state.focus, FocusZone::RightPanel);
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::unfocused_border_style()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        Theme::unfocused_title_style()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Plan Detail ", title_style))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if let Some(plan) = tui_state.plans.get(tui_state.selected_plan_idx) {
        let plan_summary = tui_state
            .plan_summaries
            .iter()
            .find(|summary| summary.id == plan.id);
        let plan_execution = tui_state
            .current_plan_execution
            .as_ref()
            .filter(|exec| exec.plan_id == plan.id);
        render_plan_summary(
            frame,
            inner,
            plan,
            plan_summary,
            plan_execution,
            tui_state,
            view_state,
            theme,
        );
    } else {
        render_pane_empty_compact(frame, inner, "Select a plan to view details", theme);
    }
}

// ---------------------------------------------------------------------------
// Selected plan detail
// ---------------------------------------------------------------------------

fn render_plan_summary(
    frame: &mut Frame<'_>,
    area: Rect,
    plan: &PlanEntry,
    plan_summary: Option<&crate::plan::PlanSummary>,
    plan_execution: Option<&crate::tui::dashboard::PlanExecutionSnapshot>,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let plan_name = if plan.name.is_empty() {
        plan_summary
            .map(|summary| summary.title.as_str())
            .unwrap_or(plan.id.as_str())
    } else {
        plan.name.as_str()
    };
    let summary_completed = plan_summary
        .map(|summary| summary.completed)
        .unwrap_or(false);
    let tasks_total = plan
        .tasks_total
        .max(plan_summary.map_or(0, |summary| summary.task_count));
    let tasks_done = plan.tasks_done.min(tasks_total);
    let pct = if tasks_total > 0 {
        tasks_done as f64 / tasks_total as f64
    } else if summary_completed {
        1.0
    } else {
        0.0
    };
    let raw_status = if plan.status.is_active() && !plan.phase.is_empty() {
        plan.phase.as_str()
    } else if summary_completed {
        "completed"
    } else {
        plan.status.label()
    };
    let (status_icon, status_color, status_label) = if summary_completed || plan.status.is_done() {
        ("\u{2713}", theme.success, "completed")
    } else if plan.status.is_failed() {
        ("\u{2717}", theme.danger, "failed")
    } else if plan.status.is_active() {
        ("\u{25b6}", theme.warning, raw_status)
    } else {
        ("\u{25cb}", theme.muted, raw_status)
    };
    let plan_gates: Vec<_> = tui_state
        .gate_result_summaries
        .iter()
        .filter(|gate| gate.plan_id == plan.id)
        .collect();
    let gate_passed = plan_gates.iter().filter(|gate| gate.passed).count();
    let last_error = plan_summary.and_then(|summary| summary.last_error.as_deref());
    let bar_w = area.width.saturating_sub(28).clamp(10, 32) as usize;
    let bar = build_progress_bar(pct, bar_w);
    let bar_color = if tasks_done == tasks_total && tasks_total > 0 {
        theme.success
    } else if plan.tasks_failed > 0 {
        theme.danger
    } else {
        semantic_color(pct, theme)
    };

    let mut header_lines = vec![
        Line::from(vec![
            Span::styled(" plan: ", theme.label()),
            Span::styled(plan_name, theme.value()),
        ]),
        Line::from(vec![
            Span::styled(" status: ", theme.label()),
            Span::styled(
                format!("[{status_icon} {}]", status_label.to_uppercase()),
                Style::default()
                    .fg(Color::Black)
                    .bg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("id ", theme.label()),
            Span::styled(truncate(&plan.id, 24), theme.metadata()),
        ]),
        {
            // Split tasks into implementation vs verification.
            let impl_tasks: Vec<_> = plan
                .tasks
                .iter()
                .filter(|t| t.verify_command.is_none())
                .collect();
            let verify_tasks: Vec<_> = plan
                .tasks
                .iter()
                .filter(|t| t.verify_command.is_some())
                .collect();
            let impl_done = impl_tasks.iter().filter(|t| t.status.is_done()).count();
            let verify_done = verify_tasks.iter().filter(|t| t.status.is_done()).count();

            Line::from(vec![
                Span::styled(" tasks: ", theme.label()),
                Span::styled(format!("{tasks_done}/{tasks_total} done"), theme.value()),
                Span::styled(
                    format!("  impl {impl_done}/{}", impl_tasks.len()),
                    Style::default().fg(
                        if !impl_tasks.is_empty() && impl_done == impl_tasks.len() {
                            theme.success
                        } else {
                            theme.foreground
                        },
                    ),
                ),
                Span::styled(
                    format!("  verify {verify_done}/{}", verify_tasks.len()),
                    Style::default().fg(
                        if !verify_tasks.is_empty() && verify_done == verify_tasks.len() {
                            theme.success
                        } else {
                            theme.foreground
                        },
                    ),
                ),
                Span::styled(
                    format!("  {} failed", plan.tasks_failed),
                    Style::default().fg(if plan.tasks_failed > 0 {
                        theme.danger
                    } else {
                        theme.muted
                    }),
                ),
                Span::styled(
                    format!("  {gate_passed}/{} gates", plan_gates.len()),
                    Style::default().fg(if plan_gates.is_empty() {
                        theme.muted
                    } else if gate_passed == plan_gates.len() {
                        theme.success
                    } else {
                        theme.warning
                    }),
                ),
            ])
        },
        Line::from(vec![
            Span::styled(" progress: ", theme.label()),
            Span::styled(
                format!("{:.0}%", pct * 100.0),
                Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(bar, Style::default().fg(bar_color)),
            Span::styled(
                format!(" {tasks_done}/{tasks_total}"),
                Style::default().fg(theme.foreground),
            ),
        ]),
    ];
    let cost = tui_state.plan_budget_summary(plan);
    let budget_text = if cost.budget_usd > 0.0 {
        format!(
            "${:.3} / ${:.2} ({:.0}%)",
            cost.spent_usd,
            cost.budget_usd,
            cost.spent_usd / cost.budget_usd * 100.0
        )
    } else {
        format!("${:.3} / unlimited", cost.spent_usd)
    };
    header_lines.push(Line::from(vec![
        Span::styled(" cost: ", theme.label()),
        Span::styled(budget_text, Style::default().fg(theme.warning)),
        Span::styled(
            format!("  projected ${:.3}", cost.projected_total_usd),
            Style::default().fg(
                if cost.budget_usd > 0.0 && cost.projected_total_usd > cost.budget_usd {
                    theme.danger
                } else {
                    theme.muted
                },
            ),
        ),
    ]));
    if let Some(err) = last_error {
        header_lines.push(Line::from(vec![
            Span::styled(
                " error: ",
                Style::default()
                    .fg(theme.danger)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                truncate(err, area.width.saturating_sub(10) as usize),
                Style::default()
                    .fg(theme.danger)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    header_lines.push(Line::from(Span::styled(
        format!(
            " {}",
            "\u{2550}".repeat(area.width.saturating_sub(3) as usize)
        ),
        theme.section_header(),
    )));

    let header_height = header_lines.len() as u16;
    // Count agents on this plan for sizing.
    let plan_agents: Vec<_> = tui_state
        .agents
        .iter()
        .filter(|a| a.current_plan == plan.id || a.current_plan == plan.name)
        .collect();
    let agents_height = if plan_agents.is_empty() {
        2
    } else {
        (plan_agents.len() as u16).min(4) + 2
    };
    let has_git_info =
        plan.branch.is_some() || plan.worktree_path.is_some() || plan.last_commit.is_some();
    let git_height = if has_git_info { 4 } else { 0 };

    let sections = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Min(0),
        Constraint::Length(agents_height),
        Constraint::Length(6),
        Constraint::Length(git_height),
        Constraint::Length(4),
    ])
    .split(area);

    frame.render_widget(Paragraph::new(header_lines), sections[0]);
    render_plan_tasks(
        frame,
        sections[1],
        &plan.id,
        &plan.tasks,
        tui_state,
        view_state,
        theme,
    );
    render_plan_agents(frame, sections[2], &plan_agents, theme);
    render_plan_gates(frame, sections[3], &plan_gates, theme);
    if has_git_info {
        render_plan_git_info(frame, sections[4], plan, theme);
    }
    render_plan_timing(frame, sections[5], plan, plan_execution, &plan_gates, theme);
}

fn render_plan_tasks(
    frame: &mut Frame<'_>,
    area: Rect,
    plan_id: &str,
    tasks: &[TaskEntry],
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let task_count = tasks.len();
    let impl_count = tasks.iter().filter(|t| t.verify_command.is_none()).count();
    let verify_count = tasks.iter().filter(|t| t.verify_command.is_some()).count();
    let title_text = if task_count > 0 {
        if verify_count > 0 {
            format!(" Tasks ({task_count}: {impl_count} impl, {verify_count} verify) ")
        } else {
            format!(" Tasks ({task_count}) ")
        }
    } else {
        " Tasks ".to_string()
    };
    let block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(title_text, theme.section_header()))
        .border_style(Style::default().fg(Theme::SEPARATOR));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tasks.is_empty() {
        render_pane_empty_compact(frame, inner, "No tasks recorded for this plan", theme);
        return;
    }

    // Build the dependency tree; falls back to flat order when no deps exist.
    let tree_rows = build_task_dep_tree(tasks);
    let tree_len = tree_rows.len();

    // Use Paragraph-based rendering for scrollable task list.
    let visible_height = inner.height as usize;
    let header_lines = 1usize;
    let available_rows = visible_height.saturating_sub(header_lines);

    // Compute scroll offset to keep selected task visible.
    let selected = view_state
        .secondary_selected
        .min(tree_len.saturating_sub(1));
    let scroll_offset = if tree_len > available_rows {
        selected
            .saturating_sub(available_rows / 2)
            .min(tree_len.saturating_sub(available_rows))
    } else {
        0
    };

    let content_w = inner.width as usize;
    let has_deps = tasks.iter().any(|t| !t.depends_on.is_empty());
    let mut lines: Vec<Line<'_>> = Vec::with_capacity(visible_height);

    // Column header — when showing deps as tree, omit the old "deps" column
    // (the hierarchy itself conveys the dependency structure).
    let header_agent_col = if has_deps { "dep-tree" } else { "agent" };
    lines.push(Line::from(vec![Span::styled(
        format!(
            " {:<3}  {:<title_w$} {:<8} {:<10} {:>8}",
            " ",
            "task",
            "status",
            header_agent_col,
            "cost",
            title_w = content_w.saturating_sub(38).max(8)
        ),
        theme.section_header(),
    )]));

    for (i, row) in tree_rows
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(available_rows)
    {
        let task = row.task;
        let (icon, icon_color) = task_status_icon(task, theme);
        let task_title = if task.name.is_empty() {
            task.id.as_str()
        } else {
            task.name.as_str()
        };
        let global_idx = scroll_offset + i;
        let is_selected = global_idx == selected;
        let bg = if is_selected {
            theme.selection_background
        } else {
            Color::Reset
        };

        let key = format!("{plan_id}:{}", task.id);
        let spent = tui_state.cost_per_task.get(&key).copied().unwrap_or(0.0);
        let budget = tui_state.task_budget(plan_id, &task.id);
        let cost = if budget > 0.0 {
            format!("${spent:.2}/{budget:.0}")
        } else if spent > 0.0 {
            format!("${spent:.3}")
        } else {
            "\u{00b7}".to_string()
        };

        // Agent column (rightmost detail column).
        let agent_col = if let Some(agent) = &task.agent_id {
            (truncate(agent, 10), theme.foreground)
        } else {
            ("-".to_string(), theme.muted)
        };

        // Tree-connector prefix: "    ├── " or "    └── " etc.
        let prefix = task_tree_prefix(row);
        let prefix_len = prefix.chars().count();

        // Budget title column — prefix consumes some of the name width.
        let title_budget = content_w.saturating_sub(38 + prefix_len).max(4);

        let connector_style = Style::default().fg(theme.muted);
        let mut spans: Vec<Span<'_>> = vec![Span::styled(
            format!(" {icon} "),
            Style::default().fg(icon_color).bg(bg),
        )];
        if !prefix.is_empty() {
            spans.push(Span::styled(prefix, connector_style.bg(bg)));
        }
        spans.extend([
            Span::styled(
                format!(
                    "{:<width$}",
                    truncate(task_title, title_budget),
                    width = title_budget
                ),
                Style::default()
                    .fg(if is_selected {
                        theme.accent
                    } else {
                        theme.foreground
                    })
                    .bg(bg),
            ),
            Span::styled(
                format!(" {:<8}", truncate(task.status.label(), 8)),
                match task.status {
                    TaskStatus::Done => theme.badge_complete(),
                    TaskStatus::Failed | TaskStatus::Blocked => theme.badge_failed(),
                    TaskStatus::Active => theme.badge_running(),
                    TaskStatus::Pending => theme.badge_pending(),
                }
                .bg(bg),
            ),
            Span::styled(
                format!(" {:<10}", agent_col.0),
                Style::default().fg(agent_col.1).bg(bg),
            ),
            Span::styled(
                format!(" {:>8}", cost),
                Style::default()
                    .fg(if budget > 0.0 && spent >= budget {
                        theme.danger
                    } else {
                        theme.muted
                    })
                    .bg(bg),
            ),
        ]);
        lines.push(Line::from(spans));
    }

    // Scroll indicator when tasks are clipped
    if tree_len > available_rows {
        let remaining = tree_len.saturating_sub(scroll_offset + available_rows);
        if remaining > 0 && lines.len() < visible_height {
            lines.push(Line::from(Span::styled(
                format!("  \u{25be} {remaining} more"),
                Style::default().fg(theme.muted),
            )));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);

    // Scrollbar for task list
    if tree_len > available_rows {
        render_scrollbar(frame, inner, tree_len, available_rows, scroll_offset, theme);
    }
}

fn render_plan_agents(
    frame: &mut Frame<'_>,
    area: Rect,
    plan_agents: &[&crate::tui::state::AgentRow],
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(" Agents on Plan ", theme.section_header()))
        .border_style(Style::default().fg(Theme::SEPARATOR));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if plan_agents.is_empty() {
        render_pane_empty_compact(frame, inner, "No agents assigned", theme);
        return;
    }

    let rows: Vec<Row<'_>> = plan_agents
        .iter()
        .map(|agent| {
            let badge_style = match agent.status {
                AgentStatus::Active => theme.badge_running(),
                AgentStatus::Done => theme.badge_complete(),
                AgentStatus::Failed => theme.badge_failed(),
                AgentStatus::Idle => theme.badge_pending(),
            };
            Row::new(vec![
                Cell::from(Span::styled(truncate(&agent.id, 16), theme.value())),
                Cell::from(Span::styled(
                    truncate(&agent.role, 12),
                    Style::default().fg(theme.accent),
                )),
                Cell::from(Span::styled(truncate(&agent.model, 20), theme.metadata())),
                Cell::from(Span::styled(
                    format!(" {} ", agent.status.label()),
                    badge_style,
                )),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(16),
        Constraint::Length(12),
        Constraint::Min(10),
        Constraint::Length(8),
    ];
    let table = Table::new(rows, widths)
        .header(Row::new([" agent", "role", "model", "status"]).style(theme.section_header()))
        .column_spacing(1);
    frame.render_widget(table, inner);
}

fn render_plan_git_info(frame: &mut Frame<'_>, area: Rect, plan: &PlanEntry, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(" Source Control ", theme.section_header()))
        .border_style(Style::default().fg(Theme::SEPARATOR));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let branch_val = plan.branch.as_deref().unwrap_or("unknown");
    let worktree_val = plan.worktree_path.as_deref().unwrap_or("unknown");
    let commit_val = plan
        .last_commit
        .as_deref()
        .map(|c| if c.len() > 8 { &c[..8] } else { c })
        .unwrap_or("unknown");

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Branch:   ", theme.label()),
            Span::styled(
                truncate(branch_val, inner.width.saturating_sub(13) as usize),
                theme.value(),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Worktree: ", theme.label()),
            Span::styled(
                truncate(worktree_val, inner.width.saturating_sub(13) as usize),
                theme.value(),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Commit:   ", theme.label()),
            Span::styled(commit_val, theme.metadata()),
        ]),
    ];

    // Show diff stats if available.
    if let Some(files) = plan.files_modified {
        let ins = plan.insertions.unwrap_or(0);
        let del = plan.deletions.unwrap_or(0);
        lines.push(Line::from(vec![
            Span::styled(" Changes:  ", theme.label()),
            Span::styled(format!("{files} files"), theme.value()),
            Span::styled(format!("  +{ins}"), Style::default().fg(theme.success)),
            Span::styled(format!("  -{del}"), Style::default().fg(theme.danger)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn render_plan_gates(
    frame: &mut Frame<'_>,
    area: Rect,
    plan_gates: &[&crate::tui::dashboard::GateResultSummary],
    theme: &Theme,
) {
    let passed_count = plan_gates.iter().filter(|g| g.passed).count();
    let failed_count = plan_gates.len() - passed_count;
    let title_text = if plan_gates.is_empty() {
        " Verify Results ".to_string()
    } else {
        format!(" Verify Results ({passed_count}\u{2713} {failed_count}\u{2717}) ")
    };
    let block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(title_text, theme.section_header()))
        .border_style(Style::default().fg(Theme::SEPARATOR));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if plan_gates.is_empty() {
        render_pane_empty_compact(frame, inner, "No gate results yet", theme);
        return;
    }

    let rows: Vec<Row<'_>> = plan_gates
        .iter()
        .rev()
        .map(|gate| {
            let (icon, color) = if gate.passed {
                ("\u{2713}", theme.success)
            } else {
                ("\u{2717}", theme.danger)
            };

            Row::new(vec![
                Cell::from(Span::styled(format!(" {icon}"), Style::default().fg(color))),
                Cell::from(Span::styled(
                    truncate(&gate.gate_name, 14),
                    if gate.passed {
                        theme.value()
                    } else {
                        Style::default().fg(theme.danger)
                    },
                )),
                Cell::from(Span::styled(truncate(&gate.summary, 36), theme.value())),
                Cell::from(Span::styled(
                    format_duration_ms(gate.duration_ms),
                    theme.metadata(),
                )),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(3),
        Constraint::Length(14),
        Constraint::Min(12),
        Constraint::Length(8),
    ];
    let table = Table::new(rows, widths)
        .header(Row::new([" ", "gate", "summary", "time"]).style(theme.section_header()))
        .column_spacing(1);
    frame.render_widget(table, inner);
}

fn render_plan_timing(
    frame: &mut Frame<'_>,
    area: Rect,
    plan: &PlanEntry,
    plan_execution: Option<&crate::tui::dashboard::PlanExecutionSnapshot>,
    plan_gates: &[&crate::tui::dashboard::GateResultSummary],
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(" Timing ", theme.section_header()))
        .border_style(Style::default().fg(Theme::SEPARATOR));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let timing_lines = build_timing_lines(plan, plan_execution, plan_gates, theme);
    if timing_lines.is_empty() {
        render_pane_empty_compact(frame, inner, "Timing not available", theme);
        return;
    }

    frame.render_widget(
        Paragraph::new(timing_lines).wrap(Wrap { trim: false }),
        inner,
    );
}

fn build_timing_lines(
    plan: &PlanEntry,
    plan_execution: Option<&crate::tui::dashboard::PlanExecutionSnapshot>,
    plan_gates: &[&crate::tui::dashboard::GateResultSummary],
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let tasks_done = plan.tasks_done;
    let tasks_total = plan.tasks_total;
    let tasks_remaining = tasks_total.saturating_sub(tasks_done);

    if plan.elapsed_secs > 0.0 {
        lines.push(Line::from(vec![
            Span::styled(" total ", theme.label()),
            Span::styled(format_duration_secs(plan.elapsed_secs), theme.value()),
        ]));
    }

    if plan.elapsed_secs > 0.0 && tasks_done > 0 {
        lines.push(Line::from(vec![
            Span::styled(" avg/done ", theme.label()),
            Span::styled(
                format_duration_secs(plan.elapsed_secs / tasks_done as f64),
                theme.value(),
            ),
        ]));
    }

    // ETA: proportional estimate from elapsed time and completed task ratio.
    // Only shown when the plan is active (tasks remaining > 0) and we have
    // enough data (at least one task done) to project forward.
    if plan.elapsed_secs > 0.0 && tasks_done > 0 && tasks_remaining > 0 {
        let per_task_secs = plan.elapsed_secs / tasks_done as f64;
        let eta_secs = per_task_secs * tasks_remaining as f64;
        lines.push(Line::from(vec![
            Span::styled(" eta ", theme.label()),
            Span::styled(
                format_duration_secs(eta_secs),
                Style::default().fg(theme.warning),
            ),
        ]));
    }

    if let Some(exec) = plan_execution {
        if let Some(current_task) = exec.tasks.iter().find(|task| task.is_current) {
            if let Some(current_secs) = parse_duration_secs(&current_task.duration) {
                lines.push(Line::from(vec![
                    Span::styled(" current ", theme.label()),
                    Span::styled(
                        format_duration_secs(current_secs),
                        Style::default().fg(theme.warning),
                    ),
                ]));
            }
        }
    }

    if !plan_gates.is_empty() {
        let gate_secs = plan_gates
            .iter()
            .map(|gate| gate.duration_ms as f64 / 1000.0)
            .sum::<f64>();
        lines.push(Line::from(vec![
            Span::styled(" gates ", theme.label()),
            Span::styled(format_duration_secs(gate_secs), theme.value()),
        ]));
    }

    lines
}

fn task_status_icon(task: &TaskEntry, theme: &Theme) -> (&'static str, Color) {
    match task.status {
        TaskStatus::Done => ("\u{2713}", theme.success),
        TaskStatus::Active => ("\u{25b6}", theme.warning),
        TaskStatus::Failed | TaskStatus::Blocked => ("\u{2717}", theme.danger),
        TaskStatus::Pending => ("\u{25cb}", theme.muted),
    }
}

// ---------------------------------------------------------------------------
// Scrollbar (buffer-direct rendering)
// ---------------------------------------------------------------------------

fn render_scrollbar(
    frame: &mut Frame<'_>,
    area: Rect,
    total: usize,
    visible: usize,
    offset: usize,
    theme: &Theme,
) {
    if total <= visible || area.height == 0 {
        return;
    }

    let track_height = area.height as usize;
    let thumb_height = ((visible as f64 / total as f64) * track_height as f64)
        .ceil()
        .max(1.0) as usize;
    let thumb_top = if total > visible {
        ((offset as f64 / (total - visible) as f64) * (track_height - thumb_height) as f64).round()
            as usize
    } else {
        0
    };

    let x = area.x + area.width.saturating_sub(1);
    let buf = frame.buffer_mut();

    for i in 0..track_height {
        let y = area.y + i as u16;
        let in_thumb = i >= thumb_top && i < thumb_top + thumb_height;
        let (ch, color) = if in_thumb {
            ('\u{2588}', theme.accent) // filled block
        } else {
            ('\u{2502}', Theme::SEPARATOR) // thin line
        };
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char(ch);
            cell.set_fg(color);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a smooth fractional progress bar string.
fn build_progress_bar(progress: f64, width: usize) -> String {
    let progress = progress.clamp(0.0, 1.0);
    let filled_exact = progress * width as f64;
    let full_blocks = filled_exact.floor() as usize;
    let fractional = filled_exact - full_blocks as f64;
    let fractional_idx = (fractional * (BLOCKS.len() - 1) as f64).round() as usize;

    let mut bar = String::with_capacity(width);
    for _ in 0..full_blocks.min(width) {
        bar.push('\u{2588}');
    }
    if full_blocks < width && fractional_idx > 0 {
        bar.push(BLOCKS[fractional_idx.min(BLOCKS.len() - 1)]);
    }
    let remaining = width.saturating_sub(bar.chars().count());
    for _ in 0..remaining {
        bar.push('\u{2500}');
    }
    bar
}

/// Build a compact mini-bar for wave headers.
fn build_mini_bar(
    width: usize,
    fill_pct: f64,
    done: bool,
    _active: bool,
    _theme: &Theme,
) -> String {
    let filled = (fill_pct.clamp(0.0, 1.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}]",
        "\u{2588}".repeat(filled.min(width)),
        if done { "\u{2588}" } else { "\u{2500}" }
            .repeat(empty)
            .chars()
            .take(empty)
            .collect::<String>()
    )
}

/// Progress bar gradient color.
fn progress_color(pct: f64, total: usize, completed: usize, theme: &Theme) -> Color {
    if completed == total && total > 0 {
        theme.success
    } else if pct >= 0.5 {
        theme.accent
    } else if pct >= 0.2 {
        theme.warning
    } else {
        theme.muted
    }
}

/// Semantic color based on completion fraction.
fn semantic_color(pct: f64, theme: &Theme) -> Color {
    if pct >= 0.9 {
        theme.success
    } else if pct >= 0.5 {
        theme.accent
    } else if pct >= 0.2 {
        theme.warning
    } else {
        theme.muted
    }
}

fn format_duration_ms(duration_ms: u64) -> String {
    if duration_ms >= 1000 {
        format_duration_secs(duration_ms as f64 / 1000.0)
    } else {
        format!("{duration_ms}ms")
    }
}

fn format_duration_secs(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).round() as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

fn parse_duration_secs(duration: &str) -> Option<f64> {
    if duration.is_empty() || duration == "--" {
        return None;
    }

    let mut total = 0.0;
    let mut matched = false;
    for part in duration.split_whitespace() {
        if let Some(ms) = part.strip_suffix("ms") {
            total += ms.parse::<f64>().ok()? / 1000.0;
            matched = true;
        } else if let Some(hours) = part.strip_suffix('h') {
            total += hours.parse::<f64>().ok()? * 3600.0;
            matched = true;
        } else if let Some(minutes) = part.strip_suffix('m') {
            total += minutes.parse::<f64>().ok()? * 60.0;
            matched = true;
        } else if let Some(seconds) = part.strip_suffix('s') {
            total += seconds.parse::<f64>().ok()?;
            matched = true;
        }
    }

    matched.then_some(total)
}

use crate::tui::display_utils::truncate;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanSummary;
    use crate::tui::state::PlanPhase;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn summary(id: &str) -> PlanSummary {
        PlanSummary {
            id: id.to_string(),
            title: id.to_string(),
            task_count: 2,
            tasks_done: 0,
            tasks_failed: 0,
            completed: false,
            status: "ready".to_string(),
            superseded_by: None,
            old_format: false,
            last_error: None,
            group: None,
        }
    }

    fn rendered_rows(state: &TuiState, selected: usize) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
        let view_state = ViewState {
            selected,
            ..ViewState::default()
        };
        terminal
            .draw(|frame| {
                render_wave_tree(
                    frame,
                    frame.area(),
                    &DashboardData::default(),
                    state,
                    &view_state,
                    &Theme::dark(),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect()
    }

    #[test]
    fn live_status_joins_disk_plans_by_id_not_index() {
        // Disk plans sort alphabetically; only the second one is running and
        // it is the only live entry, so index 0 of each list differs.
        let mut state = TuiState::default();
        state.plan_summaries = vec![summary("add-plan-queue"), summary("01-backend")];
        state.plans = vec![PlanEntry {
            id: "01-backend".to_string(),
            name: "01-backend".to_string(),
            status: PlanPhase::Active,
            tasks_total: 2,
            tasks_done: 1,
            ..PlanEntry::default()
        }];

        let rows = rendered_rows(&state, 0);
        let row_for = |id: &str| {
            rows.iter()
                .find(|row| row.contains(id))
                .unwrap_or_else(|| panic!("no row for {id}: {rows:#?}"))
                .clone()
        };
        let active = row_for("01-backend");
        let idle = row_for("add-plan-queue");
        assert!(active.contains('\u{25b6}'), "running plan row: {active}");
        assert!(active.contains("1/2"), "running plan progress: {active}");
        assert!(!idle.contains('\u{25b6}'), "idle plan row: {idle}");
        assert!(idle.contains("0/2"), "idle plan progress: {idle}");
    }

    #[test]
    fn plan_set_load_refreshes_rows_and_later_starts_join_by_id() {
        use roko_core::DashboardEvent;
        use roko_core::dashboard_snapshot::{DashboardSnapshot, PlanSetEntry};

        let entry = |plan_id: &str| PlanSetEntry {
            plan_id: plan_id.to_string(),
            title: plan_id.to_string(),
            tasks_total: 2,
            ..PlanSetEntry::default()
        };
        let mut snap = DashboardSnapshot::default();
        snap.apply_with_ts(
            &DashboardEvent::PlanSetLoaded {
                plans: vec![entry("02-portal"), entry("01-backend")],
            },
            1_000,
        );
        let mut state = TuiState::default();
        state.update_from_dashboard_snapshot(&snap);
        let mut ids: Vec<&str> = state.plan_summaries.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["01-backend", "02-portal"]);

        snap.apply_with_ts(
            &DashboardEvent::PlanStarted {
                plan_id: "01-backend".to_string(),
                tasks_total: 2,
            },
            2_000,
        );
        state.update_from_dashboard_snapshot(&snap);
        let rows = rendered_rows(&state, state.selected_plan_idx);
        let row_for = |id: &str| rows.iter().find(|row| row.contains(id)).cloned();
        let started = row_for("01-backend").expect("started row");
        let queued = row_for("02-portal").expect("queued row");
        assert!(started.contains('\u{25b6}'), "started plan row: {started}");
        assert!(!queued.contains('\u{25b6}'), "queued plan row: {queued}");
    }
}
