//! Elegant empty state renderer for TUI tabs with no data.
//!
//! Provides centered messages with animated icons and contextual hints
//! so the user knows what to do when a view has nothing to show yet.
//!
//! Two levels of empty states:
//! - **Tab-level** (`render_empty_state`): full animated icon + title + hints for an entire tab.
//! - **Pane-level** (`render_pane_empty` / `render_pane_empty_compact`): contextual messages
//!   for individual panes within a tab (e.g. "no plan selected" in the task list pane).

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use super::atmosphere::Atmosphere;
use super::tabs::Tab;
use super::theme::Theme;

/// Braille spinner frames for subtle loading animation.
const SPINNER: &[char] = &['\u{2801}', '\u{2809}', '\u{2819}', '\u{281B}', '\u{281E}', '\u{2836}', '\u{2834}', '\u{2824}'];

/// Pane-level empty state variants for individual view panes within a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneEmpty {
    /// No plan selected — shown in task/agent/gate panes.
    NoPlanSelected,
    /// Plan selected but no tasks.
    NoTasks,
    /// Plan selected but no agents running.
    NoAgents,
    /// Waiting for gate output.
    GateWaiting,
    /// No agent output yet.
    NoAgentOutput,
    /// No log entries match filter.
    NoLogsMatch,
    /// Empty inspect data.
    NoInspectData,
}

/// Per-pane empty state content: (icon, title, hint_lines).
fn pane_empty_content(pane: PaneEmpty) -> (&'static str, &'static str, &'static [&'static str]) {
    match pane {
        PaneEmpty::NoPlanSelected => (
            "\u{25CB}",
            "No plan selected",
            &[
                "Select a plan to see its tasks and agents",
                "Use \u{2191}\u{2193} to navigate the plan tree",
            ],
        ),
        PaneEmpty::NoTasks => (
            "\u{2610}",
            "No tasks",
            &["This plan has no tasks defined"],
        ),
        PaneEmpty::NoAgents => (
            "\u{2B21}",
            "No agents active",
            &[
                "No agents are currently working on this plan",
                "Agents spawn when tasks are dispatched",
            ],
        ),
        PaneEmpty::GateWaiting => (
            "\u{2026}",
            "Waiting for gates",
            &[
                "Gate output will stream here during verification",
                "Gates run: compile \u{2192} test \u{2192} clippy",
            ],
        ),
        PaneEmpty::NoAgentOutput => (
            "\u{2500}",
            "No output",
            &[
                "Agent output will appear here during execution",
                "Select an agent from the roster to view its output",
            ],
        ),
        PaneEmpty::NoLogsMatch => (
            "\u{2205}",
            "No matches",
            &[
                "No entries match the current filter",
                "Press 'a' to show all levels",
            ],
        ),
        PaneEmpty::NoInspectData => (
            "\u{2B22}",
            "No telemetry",
            &[
                "Run a plan to generate telemetry data",
                "roko plan run plans/ --engine runner-v2",
            ],
        ),
    }
}

/// Per-tab empty state content: (icon, title, hint_lines).
fn empty_content(tab: Tab) -> (&'static str, &'static str, &'static [&'static str]) {
    match tab {
        Tab::Dashboard => (
            "\u{25C9}",
            "No active run",
            &[
                "Start a plan to see the dashboard come alive.",
                "roko plan run plans/ --engine runner-v2",
            ],
        ),
        Tab::Plans => (
            "\u{2610}",
            "No plans loaded",
            &[
                "Create a plan to get started.",
                "roko plan create   or   roko prd plan <slug>",
            ],
        ),
        Tab::Agents => (
            "\u{2B21}",
            "No agents online",
            &[
                "Agents appear when plans execute.",
                "roko agent start --name <id>",
            ],
        ),
        Tab::Git => (
            "\u{2387}",
            "No repository data",
            &["Ensure you are inside a git repository."],
        ),
        Tab::Logs => (
            "\u{2261}",
            "No log entries",
            &[
                "Events appear here as the run progresses.",
                "Run a plan to generate log output.",
            ],
        ),
        Tab::Config => (
            "\u{2699}",
            "Configuration loading",
            &["Press Enter to toggle sections."],
        ),
        Tab::Inspect => (
            "\u{2B22}",
            "No signals recorded",
            &[
                "Run agents to populate the signal DAG.",
                "roko run \"<prompt>\"",
            ],
        ),
        Tab::Marketplace => (
            "\u{229A}",
            "No jobs posted",
            &[
                "Jobs appear when agents or operators post work items.",
                "Press 'n' to create a new job.",
            ],
        ),
        Tab::Atelier => (
            "\u{270E}",
            "No PRDs found",
            &[
                "Capture ideas and draft PRDs to populate the workshop.",
                "roko prd idea \"<description>\"",
            ],
        ),
        Tab::Learning => (
            "\u{2B24}",
            "No learning data",
            &[
                "The cascade router learns from task completions.",
                "roko plan run plans/ --engine runner-v2",
            ],
        ),
    }
}

/// Render an elegant centered empty state for the given tab.
///
/// Shows an animated icon, a title, and contextual hint lines. The icon
/// pulses between bright and dim using the atmosphere's breathing cycle.
pub fn render_empty_state(
    frame: &mut Frame<'_>,
    area: Rect,
    tab: Tab,
    atmosphere: &Atmosphere,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }

    let (icon, title, hints) = empty_content(tab);

    // Animated spinner next to the icon.
    let spinner_idx = (atmosphere.frame() as usize / 6) % SPINNER.len();
    let spinner_ch = SPINNER[spinner_idx];

    // Breathing brightness for the icon — subtle pulse.
    let brightness = atmosphere.breathing_brightness();
    let icon_color = super::theme::brighten(Theme::ROSE_DIM, brightness);

    let mut lines: Vec<Line<'_>> = Vec::new();

    // Vertical centering: add blank lines to push content to the middle.
    let content_height = 3 + hints.len(); // icon + blank + title + hints
    let padding = area.height.saturating_sub(content_height as u16) / 2;
    for _ in 0..padding {
        lines.push(Line::from(""));
    }

    // Icon line with spinner.
    lines.push(Line::from(vec![
        Span::styled(
            format!("{icon} "),
            Style::default().fg(icon_color),
        ),
        Span::styled(
            spinner_ch.to_string(),
            Style::default().fg(Theme::TEXT_GHOST),
        ),
    ]));

    // Blank separator.
    lines.push(Line::from(""));

    // Title.
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Theme::TEXT_DIM)
            .add_modifier(Modifier::BOLD),
    )));

    // Blank separator before hints.
    lines.push(Line::from(""));

    // Hint lines in muted color.
    for hint in hints {
        lines.push(Line::from(Span::styled(
            hint.to_string(),
            Style::default().fg(Theme::TEXT_GHOST),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// Render a pane-level empty state with animated icon and contextual hints.
///
/// Similar to `render_empty_state` but uses [`PaneEmpty`] variants for
/// individual panes within a tab. The `frame_count` parameter drives the
/// spinner animation (typically `atmosphere.frame()` or a tick counter).
pub fn render_pane_empty(
    f: &mut Frame<'_>,
    area: Rect,
    pane: PaneEmpty,
    _theme: &Theme,
    frame_count: usize,
) {
    if area.width < 8 || area.height < 3 {
        return;
    }

    let (icon, title, hints) = pane_empty_content(pane);

    let spinner_idx = (frame_count / 6) % SPINNER.len();
    let spinner_ch = SPINNER[spinner_idx];

    let mut lines: Vec<Line<'_>> = Vec::new();

    // Vertical centering.
    let content_height = 3 + hints.len();
    let padding = area.height.saturating_sub(content_height as u16) / 2;
    for _ in 0..padding {
        lines.push(Line::from(""));
    }

    // Icon line with spinner.
    lines.push(Line::from(vec![
        Span::styled(
            format!("{icon} "),
            Style::default().fg(Theme::ROSE_DIM),
        ),
        Span::styled(
            spinner_ch.to_string(),
            Style::default().fg(Theme::TEXT_GHOST),
        ),
    ]));

    // Blank separator.
    lines.push(Line::from(""));

    // Title.
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Theme::TEXT_DIM)
            .add_modifier(Modifier::BOLD),
    )));

    // Blank separator before hints.
    lines.push(Line::from(""));

    // Hint lines.
    for hint in hints {
        lines.push(Line::from(Span::styled(
            hint.to_string(),
            Style::default().fg(Theme::TEXT_GHOST),
        )));
    }

    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// Render a compact pane-level empty state — just centered dim text.
///
/// Use this in smaller panes where the full icon/spinner/title treatment
/// would consume too much space.
pub fn render_pane_empty_compact(
    f: &mut Frame<'_>,
    area: Rect,
    message: &str,
    _theme: &Theme,
) {
    if area.width < 4 || area.height < 1 {
        return;
    }

    let mut lines: Vec<Line<'_>> = Vec::new();
    let padding = area.height.saturating_sub(1) / 2;
    for _ in 0..padding {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        message.to_string(),
        Style::default().fg(Theme::TEXT_GHOST),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn every_tab_has_empty_content() {
        for tab in Tab::ALL {
            let (icon, title, hints) = empty_content(tab);
            assert!(!icon.is_empty(), "tab {tab:?} has no icon");
            assert!(!title.is_empty(), "tab {tab:?} has no title");
            assert!(!hints.is_empty(), "tab {tab:?} has no hints");
        }
    }

    #[test]
    fn render_without_panic() {
        let atmo = Atmosphere::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        for tab in Tab::ALL {
            terminal
                .draw(|frame| {
                    render_empty_state(frame, frame.area(), tab, &atmo);
                })
                .unwrap();
        }
    }

    #[test]
    fn tiny_area_does_not_panic() {
        let atmo = Atmosphere::default();
        let backend = TestBackend::new(4, 2);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_empty_state(frame, frame.area(), Tab::Dashboard, &atmo);
            })
            .unwrap();
    }

    #[test]
    fn every_pane_has_content() {
        let all = [
            PaneEmpty::NoPlanSelected,
            PaneEmpty::NoTasks,
            PaneEmpty::NoAgents,
            PaneEmpty::GateWaiting,
            PaneEmpty::NoAgentOutput,
            PaneEmpty::NoLogsMatch,
            PaneEmpty::NoInspectData,
        ];
        for pane in all {
            let (icon, title, hints) = pane_empty_content(pane);
            assert!(!icon.is_empty(), "pane {pane:?} has no icon");
            assert!(!title.is_empty(), "pane {pane:?} has no title");
            assert!(!hints.is_empty(), "pane {pane:?} has no hints");
        }
    }

    #[test]
    fn render_pane_empty_without_panic() {
        let theme = Theme::dark();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let panes = [
            PaneEmpty::NoPlanSelected,
            PaneEmpty::NoTasks,
            PaneEmpty::NoAgents,
            PaneEmpty::GateWaiting,
            PaneEmpty::NoAgentOutput,
            PaneEmpty::NoLogsMatch,
            PaneEmpty::NoInspectData,
        ];
        for pane in panes {
            terminal
                .draw(|frame| {
                    render_pane_empty(frame, frame.area(), pane, &theme, 0);
                })
                .unwrap();
        }
    }

    #[test]
    fn render_pane_empty_compact_without_panic() {
        let theme = Theme::dark();
        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_pane_empty_compact(frame, frame.area(), "No data", &theme);
            })
            .unwrap();
    }

    #[test]
    fn pane_empty_tiny_area_does_not_panic() {
        let theme = Theme::dark();
        let backend = TestBackend::new(4, 2);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_pane_empty(frame, frame.area(), PaneEmpty::NoPlanSelected, &theme, 0);
            })
            .unwrap();
    }

    #[test]
    fn compact_tiny_area_does_not_panic() {
        let theme = Theme::dark();
        let backend = TestBackend::new(2, 0);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_pane_empty_compact(frame, frame.area(), "x", &theme);
            })
            .unwrap();
    }
}
