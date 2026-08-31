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

const HEARTBEAT_FRAMES: [&str; 4] = ["\u{00b7}", "\u{00b0}", ".", "\u{25cf}"];

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the bottom status bar.
pub fn render_status_bar(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let bg = Style::default().bg(Theme::BG_SECONDARY);

    let (done, total) = state.task_counts();
    let all_done = total > 0 && state.plans.iter().all(|p| !p.active);
    let has_failures = state.plans.iter().any(|p| p.tasks_failed > 0);

    let mut spans: Vec<Span<'static>> = vec![Span::styled(" ", bg)];

    // ── 1. Git info: branch, commit hash, last commit time ────────────
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
        if !state.git_age.is_empty() {
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

    // ── 2. Heartbeat + pause indicator ────────────────────────────────
    let hb_idx = (state.atmosphere.frame() / 8) as usize % HEARTBEAT_FRAMES.len();
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

    // ── 3. Plan progress + health summary ─────────────────────────────
    let progress_text = if all_done && !has_failures {
        "COMPLETE".to_string()
    } else if has_failures {
        let err_count = state.plans.iter().filter(|p| p.tasks_failed > 0).count();
        format!("ERR:{err_count}")
    } else {
        format!(" {done}/{total}")
    };
    let progress_style = if has_failures {
        Theme::error_style()
    } else if all_done {
        Theme::success_style()
    } else {
        Style::default().fg(Theme::ROSE)
    };
    spans.push(Span::styled(
        format!(" {progress_text} "),
        progress_style.bg(Theme::BG_SECONDARY),
    ));

    // Health summary: active plans, live agents, flailing, retries, failures
    let active_count = state.plans.iter().filter(|p| p.active).count();
    let live_agents = state.active_agent_count();
    let flailing_count = state.plans.iter().filter(|p| p.tasks_failed >= 3).count();
    let total_failures: usize = state.plans.iter().map(|p| p.tasks_failed).sum();

    if active_count > 0 || live_agents > 0 {
        spans.push(Span::styled(
            format!(" {active_count}\u{25b8} {live_agents}ag"),
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

    // Keep aggregate spend visible on the literal bottom line. The F2 plan
    // detail supplies the per-plan projection and per-task budget breakdown.
    let aggregate_budget = state.aggregate_plan_budget();
    if state.cost_dollars > 0.001 || aggregate_budget > 0.0 {
        let cost = if aggregate_budget > 0.0 {
            format!(
                " ${:.2} / ${aggregate_budget:.2} ({:.0}%)",
                state.cost_dollars,
                state.cost_dollars / aggregate_budget * 100.0
            )
        } else {
            format!(" ${:.2} / unlimited", state.cost_dollars)
        };
        spans.push(Span::styled(
            cost,
            Style::default().fg(Theme::BONE).bg(Theme::BG_SECONDARY),
        ));
    }

    spans.push(Span::styled(
        " \u{2502} ",
        Style::default().fg(Theme::ROSE_DIM).bg(Theme::BG_SECONDARY),
    ));

    // ── 4. Context-sensitive keybind hints ────────────────────────────
    let keys = context_key_hints(state, has_failures);

    spans.push(Span::styled(
        format!(" {keys}"),
        Style::default().fg(Theme::FG_DIM).bg(Theme::BG_SECONDARY),
    ));

    let line = Line::from(spans);
    let p = Paragraph::new(line).style(bg);
    frame.render_widget(p, area);
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
                hints.push("R:retry");
                hints.push("D:diag");
            }
            hints.push("Tab:panel");
        }
        Tab::Plans => {
            hints.push("\u{2191}\u{2193}:nav");
            // Check if we have a selected plan with tasks to show item-specific hints.
            let selected_task_status = state.plans.get(state.selected_plan_idx).and_then(|plan| {
                plan.tasks
                    .iter()
                    .find(|t| t.status == TaskStatus::Failed || t.status == TaskStatus::Active)
                    .map(|t| t.status)
            });
            match selected_task_status {
                Some(TaskStatus::Failed) => {
                    hints.push("Enter:expand");
                    hints.push("r:retry");
                    hints.push("s:skip");
                    hints.push("d:details");
                }
                Some(TaskStatus::Active) => {
                    hints.push("Enter:expand");
                    hints.push("d:details");
                }
                _ => {
                    hints.push("Enter:expand");
                    hints.push("h/l:drill");
                    hints.push("/:filter");
                }
            }
        }
        Tab::Agents => {
            hints.push("\u{2191}\u{2193}:nav");
            let agent_status = state.agents.get(state.selected_agent).map(|a| a.status);
            match agent_status {
                Some(AgentStatus::Active) => {
                    hints.push("x:stop");
                    hints.push("c:chat");
                    hints.push("d:details");
                }
                Some(AgentStatus::Failed) => {
                    hints.push("S:start");
                    hints.push("d:details");
                }
                Some(AgentStatus::Idle) => {
                    hints.push("S:start");
                    hints.push("d:details");
                }
                _ => {
                    hints.push("`:cycle");
                    hints.push("Ctrl+T:topology");
                    hints.push("i:inject");
                }
            }
        }
        Tab::Git => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("h/l:drill");
            hints.push("Enter:expand");
        }
        Tab::Logs => {
            hints.push("\u{2191}\u{2193}/PgUp/PgDn:scroll");
            hints.push("1-4:levels");
            hints.push("a:all");
            hints.push("/:search");
        }
        Tab::Config => {
            hints.push("j/k:nav");
            hints.push("Enter:toggle");
            hints.push("r:reload");
        }
        Tab::Inspect => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("Tab:panel");
            hints.push("Enter:details");
        }
        Tab::Marketplace => {
            hints.push("j/k:nav");
            hints.push("Enter:detail");
            hints.push("n:new");
            hints.push("r:refresh");
        }
        Tab::Atelier => {
            hints.push("j/k:nav");
            hints.push("Enter:detail");
            hints.push("p:publish");
            hints.push("g:gen plan");
        }
        Tab::Learning => {
            hints.push("\u{2191}\u{2193}:nav");
            hints.push("Enter:details");
        }
    }

    // Always append help hint if there's room.
    if hints.len() < 5 {
        hints.push("?:help");
    }

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
        assert!(
            failed.contains("R:retry"),
            "Expected R:retry, got: {failed}"
        );
        assert!(failed.contains("D:diag"), "Expected D:diag, got: {failed}");

        let healthy = key_hints_for_tab(Tab::Dashboard, false);
        assert!(
            !healthy.contains("R:retry"),
            "Unexpected R:retry in: {healthy}"
        );
        assert!(
            !healthy.contains("D:diag"),
            "Unexpected D:diag in: {healthy}"
        );
    }

    #[test]
    fn plans_tab_shows_drill_hints_by_default() {
        let hints = key_hints_for_tab(Tab::Plans, false);
        assert!(
            hints.contains("Enter:expand"),
            "Expected Enter:expand in: {hints}"
        );
        assert!(
            hints.contains("h/l:drill"),
            "Expected h/l:drill in: {hints}"
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
    fn config_tab_shows_reload_hint() {
        let hints = key_hints_for_tab(Tab::Config, false);
        assert!(hints.contains("r:reload"), "Expected r:reload in: {hints}");
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

        assert!(rendered_text(&terminal).contains("$2.50 / $10.00 (25%)"));
    }
}
