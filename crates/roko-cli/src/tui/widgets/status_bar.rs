//! Bottom status bar — ported from Mori.
//!
//! 5 sections: git info (branch + commit + age), heartbeat + pause indicator,
//! plan progress + health summary, cost/budget utilization, and context-sensitive
//! keybind hints.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::super::state::TuiState;
use super::super::tabs::Tab;
use crate::tui::Theme;

const HEARTBEAT_FRAMES: [&str; 2] = ["\u{25cf}", "\u{25cb}"];

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the bottom status bar.
pub fn render_status_bar(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let bg = Style::default().bg(Theme::BG_SECONDARY);
    let compact = area.width < 100;

    let (done, total) = state.task_counts();
    let all_done = total > 0 && state.plans.iter().all(|p| !p.active);
    let has_failures = state.plans.iter().any(|p| p.tasks_failed > 0);

    let mut spans: Vec<Span<'static>> = vec![Span::styled(" ", bg)];

    // ── 1. Git info: branch, commit hash ──────────────────────────────
    if !state.git_branch.is_empty() {
        spans.push(Span::styled(
            state.git_branch.clone(),
            Style::default().fg(Theme::BONE).bg(Theme::BG_SECONDARY),
        ));
        if !state.git_commit_short.is_empty() {
            spans.push(Span::styled(
                format!(" {}", &state.git_commit_short),
                Style::default()
                    .fg(Theme::TEXT_GHOST)
                    .bg(Theme::BG_SECONDARY),
            ));
        }
        // Only show commit age on wider terminals to save space.
        if !compact && !state.git_age.is_empty() {
            spans.push(Span::styled(
                format!(" {}", &state.git_age),
                Style::default()
                    .fg(Theme::TEXT_GHOST)
                    .bg(Theme::BG_SECONDARY),
            ));
        }
        spans.push(Span::styled(
            " \u{2502} ",
            Style::default().fg(Theme::ROSE_DIM).bg(Theme::BG_SECONDARY),
        ));
    }

    // ── 2. Heartbeat + pause indicator + elapsed time ─────────────────
    let hb_idx = (state.atmosphere.frame() / 15) as usize % HEARTBEAT_FRAMES.len();
    spans.push(Span::styled(
        HEARTBEAT_FRAMES[hb_idx],
        Style::default().fg(Theme::ROSE_DIM).bg(Theme::BG_SECONDARY),
    ));
    if state.is_paused {
        spans.push(Span::styled(
            " PAUSED ",
            Style::default()
                .fg(Theme::VOID)
                .bg(Theme::WARNING)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Input mode badge (FILTER/SEARCH/INJECT/EDIT)
    if let Some(badge) = state.input_mode.badge_label() {
        spans.push(Span::styled(
            format!(" [{badge}] "),
            Style::default()
                .fg(Theme::VOID)
                .bg(Theme::DREAM)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Elapsed time - always show when a run is active.
    let elapsed_secs = state.elapsed_secs() as u64;
    if elapsed_secs > 0 {
        spans.push(Span::styled(
            format!(" {}", format_elapsed(elapsed_secs)),
            Style::default().fg(Theme::BONE_DIM).bg(Theme::BG_SECONDARY),
        ));
    }

    // ── 3. Plan progress: compact mini-bar + counts + health ──────────
    if total > 0 {
        spans.push(Span::styled(
            " \u{2502} ",
            Style::default().fg(Theme::ROSE_DIM).bg(Theme::BG_SECONDARY),
        ));

        // Compact 8-char inline progress bar.
        let bar_w = 8usize;
        let fill_pct = done as f64 / total.max(1) as f64;
        let filled = ((bar_w as f64) * fill_pct).round() as usize;
        let empty = bar_w.saturating_sub(filled);
        let bar_color = Theme::semantic_color(fill_pct);
        if filled > 0 {
            spans.push(Span::styled(
                "\u{2588}".repeat(filled),
                Style::default().fg(bar_color).bg(Theme::BG_SECONDARY),
            ));
        }
        if empty > 0 {
            spans.push(Span::styled(
                "\u{2500}".repeat(empty),
                Style::default()
                    .fg(Theme::TEXT_PHANTOM)
                    .bg(Theme::BG_SECONDARY),
            ));
        }
    }

    let progress_text = if all_done && !has_failures {
        "COMPLETE".to_string()
    } else if has_failures {
        let err_count = state.plans.iter().filter(|p| p.tasks_failed > 0).count();
        format!("ERR:{err_count}")
    } else if total > 0 {
        let pct = (done as f64 / total as f64 * 100.0) as u32;
        format!(" {done}/{total} {pct}%")
    } else {
        String::new()
    };
    let progress_style = if has_failures {
        Theme::error_style()
    } else if all_done {
        Theme::success_style()
    } else {
        Style::default().fg(Theme::ROSE)
    };
    if !progress_text.is_empty() {
        spans.push(Span::styled(
            format!(" {progress_text}"),
            progress_style.bg(Theme::BG_SECONDARY),
        ));
    }

    // Health summary: active plans, live agents, flailing, failures.
    let active_count = state.plans.iter().filter(|p| p.active).count();
    let live_agents = state.active_agent_count();
    let flailing_count = state.plans.iter().filter(|p| p.tasks_failed >= 3).count();
    let total_failures: usize = state.plans.iter().map(|p| p.tasks_failed).sum();

    if active_count > 0 || live_agents > 0 {
        spans.push(Span::styled(
            format!(" {active_count}\u{25b8}{live_agents}ag"),
            Style::default().fg(Theme::ROSE_DIM).bg(Theme::BG_SECONDARY),
        ));
    }
    if flailing_count > 0 {
        spans.push(Span::styled(
            format!(" \u{26a0}{flailing_count}"),
            Style::default().fg(Theme::EMBER).bg(Theme::BG_SECONDARY),
        ));
    }
    if total_failures > 0 {
        spans.push(Span::styled(
            format!(" \u{2717}{total_failures}"),
            Style::default().fg(Theme::EMBER).bg(Theme::BG_SECONDARY),
        ));
    }

    // Keep aggregate spend visible on the literal bottom line.
    if !compact {
        let aggregate_budget = state.aggregate_plan_budget();
        if state.cost_dollars > 0.001 || aggregate_budget > 0.0 {
            let cost = if aggregate_budget > 0.0 {
                format!(
                    " ${:.2}/${aggregate_budget:.2}",
                    state.cost_dollars,
                )
            } else {
                format!(" ${:.2}", state.cost_dollars)
            };
            spans.push(Span::styled(
                cost,
                Style::default().fg(Theme::BONE).bg(Theme::BG_SECONDARY),
            ));
        }
    }

    spans.push(Span::styled(
        " \u{2502} ",
        Style::default().fg(Theme::ROSE_DIM).bg(Theme::BG_SECONDARY),
    ));

    // ── 4. Context-sensitive keybind hints ────────────────────────────
    let prefix_width = Line::from(spans.clone()).width();
    let hint_width = (area.width as usize)
        .saturating_sub(prefix_width)
        .saturating_sub(1);
    let keys = fit_context_hints(&context_key_hints(state, has_failures), hint_width);

    spans.push(Span::styled(
        format!(" {keys}"),
        Style::default().fg(Theme::FG_DIM).bg(Theme::BG_SECONDARY),
    ));

    let line = Line::from(spans);
    let p = Paragraph::new(line).style(bg);
    frame.render_widget(p, area);
}

/// Format elapsed seconds into compact human-readable form.
fn format_elapsed(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h{m:02}m")
    } else if m > 0 {
        format!("{m}m{s:02}s")
    } else {
        format!("{s}s")
    }
}

/// Keep complete hint tokens within the remaining footer width. Help is
/// retained while lower-priority trailing actions are progressively removed;
/// allowing `Paragraph` to clip the final glyphs produces misleading keys.
fn fit_context_hints(hints: &str, max_width: usize) -> String {
    let mut tokens = hints.split("  ").collect::<Vec<_>>();
    let help = tokens
        .last()
        .is_some_and(|token| *token == "?:help")
        .then(|| tokens.pop())
        .flatten();

    loop {
        let candidate = tokens
            .iter()
            .copied()
            .chain(help)
            .collect::<Vec<_>>()
            .join("  ");
        if Line::from(candidate.as_str()).width() <= max_width {
            return candidate;
        }
        if tokens.pop().is_none() {
            return help
                .filter(|token| Line::from(*token).width() <= max_width)
                .unwrap_or_default()
                .to_string();
        }
    }
}

/// Build context-sensitive keybind hints based on the current tab, selection
/// state, and item status. Returns at most 5 hint tokens to avoid visual
/// clutter, each formatted as `key:action` and separated by two spaces.
fn context_key_hints(state: &TuiState, has_failures: bool) -> String {
    use super::super::state::{AgentStatus, TaskStatus};

    let mut hints: Vec<&str> = Vec::with_capacity(6);

    match state.active_tab {
        Tab::Dashboard => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("a/o/d/e/g:sub-tab");
            if has_failures {
                hints.push("F2:inspect failure");
            }
            hints.push("Tab:panel");
        }
        Tab::Plans => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("Tab:panel");
            // Check if we have a selected plan with tasks to show item-specific hints.
            let selected_task_status = state.plans.get(state.selected_plan_idx).and_then(|plan| {
                plan.tasks
                    .iter()
                    .find(|t| t.status == TaskStatus::Failed || t.status == TaskStatus::Active)
                    .map(|t| t.status)
            });
            match selected_task_status {
                Some(TaskStatus::Failed) => {
                    hints.push("Enter:detail");
                    hints.push("z:diagnose");
                }
                Some(TaskStatus::Active) => {
                    hints.push("Enter:detail");
                    hints.push("e:expand");
                }
                _ => {
                    hints.push("Enter:detail");
                    hints.push("/:filter");
                }
            }
        }
        Tab::Agents => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("Tab:panel");
            let agent_status = state.agents.get(state.selected_agent).map(|a| a.status);
            match agent_status {
                Some(AgentStatus::Active) => {
                    hints.push("x:stop");
                    hints.push("c:chat");
                }
                Some(AgentStatus::Failed) | Some(AgentStatus::Idle) => {
                    hints.push("S:start");
                    hints.push("d:details");
                }
                _ => {
                    hints.push("`:cycle");
                    hints.push("i:inject");
                }
            }
        }
        Tab::Git => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("Tab:panel");
            hints.push("Enter:expand");
        }
        Tab::Logs => {
            hints.push("\u{2191}\u{2193}:scroll");
            hints.push("Tab:panel");
            hints.push("/:search");
        }
        Tab::Config => {
            hints.push("j/k:nav");
            hints.push("Tab:panel");
            hints.push("Enter:toggle");
        }
        Tab::Inspect => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("Tab:panel");
            hints.push("Enter:details");
        }
        Tab::Marketplace => {
            hints.push("j/k:nav");
            hints.push("Tab:panel");
            hints.push("Enter:detail");
        }
        Tab::Atelier => {
            hints.push("j/k:nav");
            hints.push("Tab:panel");
            hints.push("Enter:detail");
        }
        Tab::Learning => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("Tab:panel");
            hints.push("Enter:details");
        }
    }

    // Reserve the final slot for help. In failure states the contextual list
    // can already contain five actions; silently omitting discovery is worse
    // than dropping the lowest-priority trailing action.
    if hints.len() >= 5 {
        hints.truncate(4);
    }
    hints.push("?:help");

    // Cap at 5 hints.
    hints.truncate(5);
    hints.join("  ")
}

/// Backwards-compatible wrapper for tests that use the old API.
#[cfg(test)]
fn key_hints_for_tab(tab: Tab, has_failures: bool) -> String {
    let data = super::super::dashboard::DashboardData::default();
    let mut state = TuiState::from_dashboard_data(&data);
    state.active_tab = tab;
    context_key_hints(&state, has_failures)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::super::super::dashboard::DashboardData;
    use super::super::super::state::TuiState;

    fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn status_bar_renders_without_panic() {
        let data = DashboardData::default();
        let state = TuiState::from_dashboard_data(&data);
        let backend = TestBackend::new(120, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_status_bar(frame, area, &state);
            })
            .unwrap();
    }

    #[test]
    fn dashboard_key_hints_surface_failure_actions_only_when_needed() {
        let failed = key_hints_for_tab(Tab::Dashboard, true);
        assert!(failed.contains("F2:inspect failure"), "got: {failed}");
        assert!(
            failed.contains("sub-tab"),
            "Expected sub-tab hint, got: {failed}"
        );

        let healthy = key_hints_for_tab(Tab::Dashboard, false);
        assert!(!healthy.contains("inspect failure"), "got: {healthy}");
        assert!(
            healthy.contains("sub-tab"),
            "Expected sub-tab hint, got: {healthy}"
        );
    }

    #[test]
    fn plans_tab_shows_detail_and_panel_hints() {
        let hints = key_hints_for_tab(Tab::Plans, false);
        assert!(
            hints.contains("Enter:detail"),
            "Expected Enter:detail in: {hints}"
        );
        assert!(
            hints.contains("Tab:panel"),
            "Expected Tab:panel in: {hints}"
        );
    }

    #[test]
    fn agents_tab_shows_general_hints_with_no_agents() {
        let hints = key_hints_for_tab(Tab::Agents, false);
        assert!(
            hints.contains("`:cycle") || hints.contains(":nav"),
            "Expected navigation hints in: {hints}"
        );
    }

    #[test]
    fn config_tab_shows_panel_hint() {
        let hints = key_hints_for_tab(Tab::Config, false);
        assert!(
            hints.contains("Tab:panel"),
            "Expected Tab:panel in: {hints}"
        );
    }

    #[test]
    fn hints_capped_at_five() {
        let hints = key_hints_for_tab(Tab::Dashboard, true);
        let count = hints.split("  ").count();
        assert!(
            count <= 5,
            "Expected at most 5 hints, got {count} in: {hints}"
        );
    }

    #[test]
    fn narrow_hint_line_keeps_complete_tokens_and_help() {
        let fitted = fit_context_hints("↑↓:nav  `:cycle  Ctrl+T:topology  i:inject  ?:help", 36);
        assert!(fitted.ends_with("?:help"), "got: {fitted}");
        assert!(!fitted.contains("i:inject"), "got: {fitted}");
        assert!(Line::from(fitted.as_str()).width() <= 36);
    }

    #[test]
    fn status_bar_renders_spend_budget_and_utilization() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.cost_dollars = 2.5;
        state.max_plan_budget_usd = 10.0;
        state.plans.push(Default::default());
        let backend = TestBackend::new(180, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_status_bar(frame, frame.area(), &state))
            .unwrap();

        assert!(rendered_text(&terminal).contains("$2.50/$10.00"));
    }

    #[test]
    fn status_bar_shows_elapsed_time() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.run_started = Some(std::time::Instant::now() - std::time::Duration::from_secs(125));
        let backend = TestBackend::new(120, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_status_bar(frame, frame.area(), &state))
            .unwrap();
        let text = rendered_text(&terminal);
        assert!(text.contains("2m"), "Expected elapsed time, got: {text}");
    }

    #[test]
    fn status_bar_shows_inline_progress_bar() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        let mut plan = super::super::super::state::PlanEntry::default();
        plan.tasks_total = 10;
        plan.tasks_done = 5;
        plan.active = true;
        state.plans.push(plan);
        let backend = TestBackend::new(120, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_status_bar(frame, frame.area(), &state))
            .unwrap();
        let text = rendered_text(&terminal);
        assert!(text.contains("5/10"), "Expected progress count, got: {text}");
        assert!(text.contains("50%"), "Expected percentage, got: {text}");
    }
}
