//! Task progress widget with semantic progress bar and dependency-tree task list.
//!
//! Ported from Mori's task_progress.rs — uses MoriTheme, Atmosphere, TuiState.
//! Renders tasks as an authored dependency graph (tree-drawing characters) instead
//! of a synthesized flat wave list, so the actual `depends_on` structure from
//! tasks.toml is visible during execution (P1-TUI-G3).
//!
//! Layout:
//! ```text
//! ┌ Tasks · plan-001 (5/12) ────────────────────────┐
//! │ ████████░░░░░░░░░░░░░  5/12  ETA:~8m            │
//! │  RUN  2 active · 5 queued · phase implementing   │
//! │ ✓ t-001  Wire SystemPromptBuilder                │
//! │ ├── ► t-002  ⏱2m  Add episode logging           │
//! │ │   └── · t-003  Refactor gate pipeline          │
//! │ ✗ t-004  Fix clippy warnings                     │
//! └─────────────────────────────────────────────────┘
//! ```

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use std::collections::{HashMap, HashSet};

use super::super::state::{PlanEntry, TaskRow, TaskRowStatus, TuiState};
use crate::tui::Theme;
use crate::tui::util::truncate_middle;

// ---------------------------------------------------------------------------
// Dependency tree helpers
// ---------------------------------------------------------------------------

/// One entry in the flattened dependency-tree render list.
struct TreeRow<'a> {
    task: &'a TaskRow,
    /// Indentation depth (0 = root).
    depth: usize,
    /// Per-depth "is last child" flags for drawing connector lines.
    /// `connector[i]` is true when this subtree's ancestor at depth `i` is
    /// the last child of its parent (so we draw a space instead of │).
    connector: Vec<bool>,
}

/// Build a topological dependency tree from a flat task list.
///
/// Returns tasks in topological order (parents before children), with tree
/// position metadata.  Tasks whose `depends_on` list is empty or whose
/// dependencies are not found in the set are treated as roots.
///
/// If no task has any dependencies the list is returned as-is (flat, depth 0)
/// so we don't pay any cost on simple plans.
fn build_dep_tree<'a>(tasks: &'a [TaskRow]) -> Vec<TreeRow<'a>> {
    // Fast path: if no task declares dependencies, emit flat list.
    let has_deps = tasks.iter().any(|t| !t.depends_on.is_empty());
    if !has_deps {
        return tasks
            .iter()
            .map(|t| TreeRow {
                task: t,
                depth: 0,
                connector: vec![],
            })
            .collect();
    }

    // Index tasks by id for O(1) lookup.
    let id_to_idx: HashMap<&str, usize> = tasks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.as_str(), i))
        .collect();

    // Build a child map: parent_id → [child_idx, ...] in original order.
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

    // Roots are tasks that are not children of any other task (in this plan).
    let roots: Vec<usize> = (0..tasks.len())
        .filter(|i| !has_parent.contains(i))
        .collect();

    // DFS to emit TreeRows in pre-order (parent before children).
    let mut result: Vec<TreeRow<'a>> = Vec::with_capacity(tasks.len());
    let mut visited: HashSet<usize> = HashSet::new();

    fn visit<'a>(
        idx: usize,
        depth: usize,
        connector: Vec<bool>,
        tasks: &'a [TaskRow],
        children: &HashMap<usize, Vec<usize>>,
        visited: &mut HashSet<usize>,
        result: &mut Vec<TreeRow<'a>>,
    ) {
        if visited.contains(&idx) {
            return; // Guard against cycles.
        }
        visited.insert(idx);
        result.push(TreeRow {
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

    // Any tasks not reached (e.g. dependency cycles) are appended flat.
    for (idx, task) in tasks.iter().enumerate() {
        if !visited.contains(&idx) {
            result.push(TreeRow {
                task,
                depth: 0,
                connector: vec![],
            });
        }
    }

    result
}

/// Build the tree-connector prefix string for a given `TreeRow`.
///
/// Produces strings like `"    ├── "` or `"    └── "`.
/// `connector[i]` true → ancestor at depth i was last child → print space, not │.
fn tree_prefix(row: &TreeRow<'_>) -> String {
    if row.depth == 0 {
        return String::new();
    }
    let mut prefix = String::new();
    // For each ancestor level except the immediate parent, draw │ or space.
    for depth_i in 0..row.depth.saturating_sub(1) {
        let ancestor_is_last = row.connector.get(depth_i).copied().unwrap_or(true);
        if ancestor_is_last {
            prefix.push_str("    ");
        } else {
            prefix.push_str("\u{2502}   "); // │
        }
    }
    // Immediate parent connector: └── or ├──
    let is_last = row.connector.last().copied().unwrap_or(true);
    if is_last {
        prefix.push_str("\u{2514}\u{2500}\u{2500} "); // └──
    } else {
        prefix.push_str("\u{251C}\u{2500}\u{2500} "); // ├──
    }
    prefix
}

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the task progress widget.
pub fn render_task_progress(frame: &mut Frame<'_>, area: Rect, state: &TuiState, focused: bool) {
    let atm = &state.atmosphere;

    // Use the selected plan's tasks when a plan is selected, otherwise global checklist.
    let selected_plan = state.plans.get(state.selected_plan_idx);
    let plan_task_rows: Vec<TaskRow> = selected_plan.map(plan_task_rows).unwrap_or_default();
    let tasks: &[TaskRow] = if selected_plan.is_some() && !plan_task_rows.is_empty() {
        &plan_task_rows
    } else {
        &state.current_task_checklist
    };

    // Count by status. Accepted-with-failures tasks are finished (they fill
    // the progress bar) but are counted apart from passed ones.
    let accepted_with_failures = tasks
        .iter()
        .filter(|t| t.status == TaskRowStatus::AcceptedWithFailures)
        .count();
    let done = tasks
        .iter()
        .filter(|t| t.status == TaskRowStatus::Done)
        .count()
        + accepted_with_failures;
    let active = tasks
        .iter()
        .filter(|t| t.status == TaskRowStatus::Active)
        .count();
    let blocked = tasks
        .iter()
        .filter(|t| t.status == TaskRowStatus::Blocked)
        .count();
    let failed = tasks
        .iter()
        .filter(|t| t.status == TaskRowStatus::Failed)
        .count();
    let total = tasks.len();
    let pending = total.saturating_sub(done + active + blocked + failed);

    // Title — show plan name when filtering
    let plan_label = selected_plan.map(|p| p.name.as_str()).unwrap_or("");
    let mut title = if !plan_label.is_empty() {
        format!("Tasks · {} ({}/{})", plan_label, done, total)
    } else {
        format!("Tasks ({}/{})", done, total)
    };

    let theme = Theme::dark();
    let (border_style, ttl_style) = if focused {
        (Theme::focused_border_style(), theme.section_header())
    } else {
        (
            Theme::unfocused_border_style(),
            Theme::unfocused_title_style(),
        )
    };

    // Pre-compute how many header rows we'll have (progress bar + summary)
    let inner_width = area.width.saturating_sub(4) as usize;
    let has_bar = inner_width > 8 && total > 0;
    let header_rows: u16 = if has_bar { 2 } else { 1 };

    // Build the dependency tree (flat list when no deps exist).
    let tree_rows = build_dep_tree(tasks);
    let tree_len = tree_rows.len();

    // Visible task slots
    let visible = area.height.saturating_sub(2 + header_rows) as usize;
    let max_scroll = tree_len.saturating_sub(visible);
    let scroll = state.task_scroll.min(max_scroll);
    // `task_scroll` is also the task cursor (see [`cursor_task`]); it stays
    // inside the window because the window never scrolls past it.
    let cursor = state.task_scroll.min(tree_len.saturating_sub(1));
    let start = scroll;
    let end = (scroll + visible).min(tree_len);

    // Append scroll position to title
    if tree_len > visible && visible > 0 {
        title.push_str(&format!(" [{}-{} of {}]", start + 1, end, tree_len));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Theme::block_style())
        .border_style(border_style)
        .title_style(ttl_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 1 || inner.width < 8 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // ── Progress bar with percentage and ETA ────────────────────────────
    if has_bar {
        let fill_pct = done as f64 / total.max(1) as f64;
        let pct_display = (fill_pct * 100.0) as u32;
        let count_suffix = format!("  {done}/{total}  {pct_display}%");

        // ETA: prefer critical-path, fall back to proportional.
        let elapsed = state.elapsed_secs() as u64;
        let eta_suffix = if done > 0 && done < total {
            if let Some(cp_min) = state.critical_path_eta_minutes {
                Some(format!("  CP-ETA:{}", compact_duration(cp_min as u64 * 60)))
            } else {
                let rate = elapsed as f64 / done as f64;
                let remaining = ((total - done) as f64 * rate) as u64;
                if remaining > 0 {
                    Some(format!("  ETA:~{}", compact_duration(remaining)))
                } else {
                    None
                }
            }
        } else {
            None
        };

        let total_suffix_len =
            count_suffix.chars().count() + eta_suffix.as_ref().map_or(0, |s| s.chars().count());
        let bar_width = inner_width.saturating_sub(total_suffix_len + 1).max(4);
        let bar_spans = semantic_bar(bar_width, fill_pct, Some(atm.heartbeat()));

        let mut bar_line = vec![Span::styled(" ", Style::default())];
        bar_line.extend(bar_spans);
        bar_line.push(Span::styled(
            count_suffix,
            Style::default().fg(Theme::FG_DIM),
        ));
        if let Some(eta) = eta_suffix {
            bar_line.push(Span::styled(eta, Style::default().fg(Theme::DREAM)));
        }
        lines.push(Line::from(bar_line));
    }

    // ── Summary line ─────────────────────────────────────────────────────
    let summary = build_summary_line(
        TaskCounts {
            done,
            total,
            active,
            pending,
            blocked,
            failed,
            accepted_with_failures,
        },
        inner_width,
    );
    lines.push(summary);

    // ── Scroll-up indicator ──────────────────────────────────────────────
    if start > 0 {
        lines.push(Line::from(Span::styled(
            " \u{25b2} more",
            Style::default().fg(Theme::TEXT_DIM),
        )));
    }

    // ── Task rows (dependency-tree order) ────────────────────────────────
    for (i, row) in tree_rows[start..end].iter().enumerate() {
        let task = row.task;
        let global_idx = start + i;
        let is_selected = global_idx == cursor && focused;
        let is_active = task.status == TaskRowStatus::Active;

        // Status icons
        let active_spinner;
        let (icon, icon_style) = match task.status {
            TaskRowStatus::Done => (
                "\u{2713}",
                Style::default()
                    .fg(Theme::SAGE)
                    .add_modifier(Modifier::BOLD),
            ),
            TaskRowStatus::Active => {
                let pulse_color = pulse_rose(atm.heartbeat());
                active_spinner = atm.spinner().to_string();
                (
                    active_spinner.as_str(),
                    Style::default()
                        .fg(pulse_color)
                        .add_modifier(Modifier::BOLD),
                )
            }
            TaskRowStatus::Blocked => (
                "\u{2717}",
                Style::default()
                    .fg(Theme::STATUS_ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
            TaskRowStatus::Failed => (
                "\u{2717}",
                Style::default()
                    .fg(Theme::EMBER)
                    .add_modifier(Modifier::BOLD),
            ),
            TaskRowStatus::AcceptedWithFailures => (
                "\u{26a0}",
                Style::default()
                    .fg(Theme::WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
            TaskRowStatus::Pending => ("\u{25cb}", Style::default().fg(Theme::TEXT_DIM)),
        };

        // Active tasks get a subtle background highlight; selected items keep the stronger one.
        let (text_style, bg) = if is_selected {
            (
                Style::default()
                    .fg(Theme::BONE)
                    .add_modifier(Modifier::BOLD)
                    .bg(Theme::BG_HIGHLIGHT),
                Some(Theme::BG_HIGHLIGHT),
            )
        } else if is_active {
            (theme.value().bg(Theme::BG_RAISED), Some(Theme::BG_RAISED))
        } else {
            (Style::default().fg(Theme::TEXT), None)
        };

        let effective_icon_style = if let Some(bg_color) = bg {
            icon_style.bg(bg_color)
        } else {
            icon_style
        };

        // Tree-connector prefix (e.g. "    ├── " or "    └── ").
        let prefix = tree_prefix(row);
        let prefix_len = prefix.chars().count();

        // Time tag for active tasks
        let time_tag = match task.status {
            TaskRowStatus::Done => String::new(),
            TaskRowStatus::Active if task.elapsed_secs > 0.0 => {
                format!(" \u{23F1}{} ", compact_duration(task.elapsed_secs as u64))
            }
            _ => String::new(),
        };

        // Column layout: [space][icon][space][prefix][id][time_tag][title]
        let time_tag_len = time_tag.chars().count();
        let id_col_w = 8;
        let fixed = 4 + prefix_len + id_col_w + time_tag_len;
        let max_title = (inner.width as usize).saturating_sub(fixed + 2);
        let title_display = truncate_middle(&task.title, max_title);

        let connector_style = Style::default().fg(Theme::TEXT_PHANTOM);
        let id_style = if let Some(bg_color) = bg {
            theme.label().bg(bg_color)
        } else {
            theme.label()
        };

        let mut task_spans: Vec<Span<'_>> =
            vec![Span::styled(format!(" {icon} "), effective_icon_style)];
        if !prefix.is_empty() {
            task_spans.push(Span::styled(prefix, connector_style));
        }
        task_spans.push(Span::styled(
            format!("{:<width$}", &task.id, width = id_col_w),
            id_style,
        ));
        if !time_tag.is_empty() {
            let time_style = if let Some(bg_color) = bg {
                Style::default().fg(Theme::TEXT_DIM).bg(bg_color)
            } else {
                Style::default().fg(Theme::TEXT_DIM)
            };
            task_spans.push(Span::styled(time_tag, time_style));
        }
        task_spans.push(Span::styled(title_display, text_style));

        lines.push(Line::from(task_spans));
    }

    // ── Scroll-down indicator ────────────────────────────────────────────
    if end < tree_len {
        lines.push(Line::from(Span::styled(
            " \u{25bc} more",
            Style::default().fg(Theme::TEXT_DIM),
        )));
    }

    // ── Empty state ──────────────────────────────────────────────────────
    if tasks.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" {} waiting for tasks...", atm.spinner()),
            Style::default().fg(Theme::TEXT_DIM),
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);

    // ── Scrollbar ────────────────────────────────────────────────────────
    if tree_len > visible && visible > 0 {
        let sb_area = Rect::new(
            inner.x,
            inner.y + header_rows,
            inner.width,
            inner.height.saturating_sub(header_rows),
        );
        let mut sb_state = ScrollbarState::new(tree_len).position(scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(Theme::ROSE))
            .track_style(Style::default().fg(Theme::TEXT_PHANTOM))
            .begin_symbol(Some("\u{25b2}"))
            .end_symbol(Some("\u{25bc}"));
        frame.render_stateful_widget(scrollbar, sb_area, &mut sb_state);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a gradient progress bar with `█▓▒░` partial-fill characters.
///
/// Uses `Theme::progress_gradient` for smooth color transitions and a
/// heartbeat-pulsed leading edge for visual momentum.
fn semantic_bar(width: usize, pct: f64, heartbeat: Option<f64>) -> Vec<Span<'static>> {
    let pct = pct.clamp(0.0, 1.0);
    let filled_f = pct * width as f64;
    let filled = filled_f as usize;
    let frac = filled_f - filled as f64;
    let empty = width
        .saturating_sub(filled)
        .saturating_sub(if frac > 0.01 { 1 } else { 0 });

    let bar_color = Theme::progress_gradient(pct);
    let mut spans = Vec::with_capacity(4);

    if filled > 0 {
        if filled > 1 && pct < 1.0 {
            // Body
            spans.push(Span::styled(
                "\u{2588}".repeat(filled - 1),
                Style::default().fg(bar_color),
            ));
            // Leading edge: brighter, pulsed by heartbeat.
            let boost = heartbeat.map_or(1.0, |hb| 1.0 + hb * 0.3);
            let leading_style = match bar_color {
                Color::Rgb(r, g, b) => Style::default().fg(Color::Rgb(
                    ((r as f64) * boost).min(255.0) as u8,
                    ((g as f64) * boost).min(255.0) as u8,
                    ((b as f64) * boost).min(255.0) as u8,
                )),
                _ => Style::default().fg(bar_color),
            };
            spans.push(Span::styled("\u{2588}", leading_style));
        } else {
            spans.push(Span::styled(
                "\u{2588}".repeat(filled.min(width)),
                Style::default().fg(bar_color),
            ));
        }
    }

    // Fractional cell using gradient characters
    if frac > 0.01 && filled < width {
        let partial = if frac >= 0.75 {
            "\u{2593}" // ▓
        } else if frac >= 0.5 {
            "\u{2592}" // ▒
        } else {
            "\u{2591}" // ░
        };
        spans.push(Span::styled(
            partial.to_string(),
            Style::default().fg(bar_color),
        ));
    }

    // Empty portion
    if empty > 0 {
        spans.push(Span::styled(
            "\u{2500}".repeat(empty),
            Style::default().fg(Theme::TEXT_PHANTOM),
        ));
    }

    spans
}

/// Task rows for one plan's entries.
fn plan_task_rows(plan: &PlanEntry) -> Vec<TaskRow> {
    plan.tasks
        .iter()
        .map(|te| TaskRow {
            id: te.id.clone(),
            title: te.name.clone(),
            status: te.status,
            elapsed_secs: 0.0,
            depends_on: te.depends_on.clone(),
            acceptance_text: te.acceptance_text.clone(),
            verify_command: te.verify_command.clone(),
            files: te.files.clone(),
        })
        .collect()
}

/// The task under the task-list cursor as `(plan id, task id)`, in the order
/// the list renders: the selected plan's tasks, else the global checklist
/// (whose rows carry no plan id).
pub(crate) fn cursor_task(state: &TuiState) -> Option<(Option<String>, String)> {
    let plan = state
        .plans
        .get(state.selected_plan_idx)
        .filter(|plan| !plan.tasks.is_empty());
    let rows = plan.map_or_else(|| state.current_task_checklist.clone(), plan_task_rows);
    let tree = build_dep_tree(&rows);
    let row = tree.get(state.task_scroll.min(tree.len().checked_sub(1)?))?;
    Some((plan.map(|plan| plan.id.clone()), row.task.id.clone()))
}

/// Task counts for the summary line. `done` includes
/// `accepted_with_failures`.
#[derive(Debug, Clone, Copy, Default)]
struct TaskCounts {
    done: usize,
    total: usize,
    active: usize,
    pending: usize,
    blocked: usize,
    failed: usize,
    accepted_with_failures: usize,
}

/// Build the summary badge line: status tag + counts.
fn build_summary_line(counts: TaskCounts, width: usize) -> Line<'static> {
    let TaskCounts {
        done,
        total,
        active,
        pending,
        blocked,
        failed,
        accepted_with_failures,
    } = counts;
    let all_done = done == total && total > 0;
    // A run with accepted-with-failures tasks is finished, never clean.
    let (status_text, status_color) = if all_done && accepted_with_failures > 0 {
        ("DONE", Theme::WARNING)
    } else if all_done {
        ("DONE", Theme::SAGE)
    } else if failed > 0 {
        ("FAIL", Theme::EMBER)
    } else if active > 0 {
        ("RUN", Theme::WARNING)
    } else {
        ("WAIT", Theme::ROSE_DIM)
    };

    let mut details = Vec::new();
    if all_done && accepted_with_failures == 0 {
        details.push("all tasks clear".to_string());
    } else {
        if active > 0 {
            details.push(format!("{active} active"));
        }
        if pending > 0 {
            details.push(format!("{pending} queued"));
        }
        if blocked > 0 {
            details.push(format!("{blocked} blocked"));
        }
        if accepted_with_failures > 0 {
            details.push(format!(
                "{accepted_with_failures} \u{26a0} accepted with failures"
            ));
        }
        if failed > 0 {
            details.push(format!("{failed} failed"));
        }
    }

    let summary_str = details.join(" \u{00b7} ");
    let max_len = width.saturating_sub(status_text.len() + 4);
    let summary = if summary_str.chars().count() > max_len && max_len > 1 {
        let truncated: String = summary_str
            .chars()
            .take(max_len.saturating_sub(1))
            .collect();
        format!("{truncated}\u{2026}")
    } else {
        summary_str
    };

    Line::from(vec![
        Span::styled(
            format!(" {} ", status_text),
            Style::default()
                .fg(Theme::VOID)
                .bg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {summary}"),
            Style::default().fg(Theme::TEXT_GHOST),
        ),
    ])
}

/// Compact duration format: "5m", "1h05m", "45s".
fn compact_duration(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}h{minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{seconds}s")
    }
}

/// Modulate the ROSE_PULSE theme color with heartbeat oscillator.
fn pulse_rose(heartbeat: f64) -> Color {
    let scale = heartbeat.clamp(0.9, 1.1);
    super::super::theme::brighten(Theme::ROSE_PULSE, scale)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::super::state::{TaskRow, TaskRowStatus, TuiState};
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn make_state(tasks: Vec<TaskRow>) -> TuiState {
        use super::super::super::dashboard::DashboardData;
        let data = DashboardData::default();
        let mut state = TuiState::from_dashboard_data(&data);
        state.current_task_checklist = tasks;
        state
    }

    fn sample_tasks() -> Vec<TaskRow> {
        vec![
            TaskRow {
                id: "t-001".into(),
                title: "Wire SystemPromptBuilder".into(),
                status: TaskRowStatus::Done,
                elapsed_secs: 120.0,
                depends_on: Vec::new(),
                acceptance_text: None,
                verify_command: None,
                files: Vec::new(),
            },
            TaskRow {
                id: "t-002".into(),
                title: "Add episode logging".into(),
                status: TaskRowStatus::Active,
                elapsed_secs: 45.0,
                depends_on: vec!["t-001".into()],
                acceptance_text: Some("Episode log entries are persisted".into()),
                verify_command: Some("cargo test -p roko-cli -- episode".into()),
                files: vec!["crates/roko-cli/src/runner/episode.rs".into()],
            },
            TaskRow {
                id: "t-003".into(),
                title: "Refactor gate pipeline".into(),
                status: TaskRowStatus::Pending,
                elapsed_secs: 0.0,
                depends_on: vec!["t-001".into(), "t-002".into()],
                acceptance_text: None,
                verify_command: None,
                files: Vec::new(),
            },
            TaskRow {
                id: "t-004".into(),
                title: "Fix clippy warnings".into(),
                status: TaskRowStatus::Failed,
                elapsed_secs: 30.0,
                depends_on: Vec::new(),
                acceptance_text: None,
                verify_command: None,
                files: Vec::new(),
            },
            TaskRow {
                id: "t-005".into(),
                title: "Blocked on dependency".into(),
                status: TaskRowStatus::Blocked,
                elapsed_secs: 0.0,
                depends_on: Vec::new(),
                acceptance_text: None,
                verify_command: None,
                files: Vec::new(),
            },
        ]
    }

    #[test]
    fn task_progress_renders_without_panic() {
        let state = make_state(sample_tasks());
        let backend = TestBackend::new(60, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_task_progress(frame, area, &state, false);
            })
            .unwrap();
    }

    #[test]
    fn task_progress_empty() {
        let state = make_state(Vec::new());
        let backend = TestBackend::new(60, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_task_progress(frame, area, &state, false);
            })
            .unwrap();
    }

    #[test]
    fn task_progress_focused() {
        let state = make_state(sample_tasks());
        let backend = TestBackend::new(60, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_task_progress(frame, area, &state, true);
            })
            .unwrap();
    }

    #[test]
    fn task_progress_all_done() {
        let tasks = vec![
            TaskRow {
                id: "t-001".into(),
                title: "Done task".into(),
                status: TaskRowStatus::Done,
                elapsed_secs: 60.0,
                depends_on: Vec::new(),
                acceptance_text: None,
                verify_command: None,
                files: Vec::new(),
            },
            TaskRow {
                id: "t-002".into(),
                title: "Also done".into(),
                status: TaskRowStatus::Done,
                elapsed_secs: 30.0,
                depends_on: Vec::new(),
                acceptance_text: None,
                verify_command: None,
                files: Vec::new(),
            },
        ];
        let state = make_state(tasks);
        let backend = TestBackend::new(60, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_task_progress(frame, area, &state, false);
            })
            .unwrap();
    }

    #[test]
    fn accepted_with_failures_keeps_the_summary_amber() {
        let line = build_summary_line(
            TaskCounts {
                done: 3,
                total: 3,
                accepted_with_failures: 1,
                ..TaskCounts::default()
            },
            80,
        );
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(line.spans[0].style.bg, Some(Theme::WARNING));
        assert!(text.contains("1 \u{26a0} accepted with failures"), "{text}");
        assert!(!text.contains("all tasks clear"), "{text}");

        let clean = build_summary_line(
            TaskCounts {
                done: 3,
                total: 3,
                ..TaskCounts::default()
            },
            80,
        );
        assert_eq!(clean.spans[0].style.bg, Some(Theme::SAGE));
    }

    #[test]
    fn compact_duration_formats() {
        assert_eq!(compact_duration(0), "0s");
        assert_eq!(compact_duration(45), "45s");
        assert_eq!(compact_duration(60), "1m");
        assert_eq!(compact_duration(300), "5m");
        assert_eq!(compact_duration(3661), "1h01m");
    }
}
