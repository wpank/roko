//! Top header bar — ported from Mori.
//!
//! 9 sections: health-aware pulsing dot + name, queue/plan name, wave
//! indicator, gradient progress bar, plan count with semantic coloring,
//! ETA/elapsed/cost/tokens, system metrics (CPU/MEM/agents/gates), active
//! agent spinner with role label, F-key strip.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::super::state::TuiState;
use super::rosedust::gradient_fire;
use crate::tui::Theme;

const HEARTBEAT_FRAMES: [&str; 2] = ["\u{25cf}", "\u{25cb}"];

// ---------------------------------------------------------------------------
// Health status
// ---------------------------------------------------------------------------

/// Derive the overall health status from TUI state for the pulsing dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HealthStatus {
    Healthy,
    Gating,
    Error,
    Idle,
}

impl HealthStatus {
    fn from_state(state: &TuiState) -> Self {
        let has_failures = state.plans.iter().any(|p| p.tasks_failed > 0);
        if has_failures {
            return Self::Error;
        }
        let is_gating = state.current_phase.to_ascii_lowercase().contains("gat")
            || state.current_phase.to_ascii_lowercase().contains("verif");
        if is_gating {
            return Self::Gating;
        }
        let has_active = state.plans.iter().any(|p| p.active);
        if has_active {
            Self::Healthy
        } else {
            Self::Idle
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Healthy => Theme::SAGE,
            Self::Gating => Theme::WARNING,
            Self::Error => Theme::EMBER,
            Self::Idle => Theme::TEXT_GHOST,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sep() -> Span<'static> {
    Span::styled(
        "\u{2502}",
        Style::default()
            .fg(Theme::TEXT_PHANTOM)
            .bg(Theme::BG_SECONDARY),
    )
}

fn hdr_pct_color(pct: f64) -> Color {
    if pct >= 0.8 {
        Theme::EMBER
    } else if pct >= 0.5 {
        Theme::WARNING
    } else {
        Theme::SAGE
    }
}

fn hdr_success_color(pct: f64) -> Color {
    if pct >= 0.8 {
        Theme::SAGE
    } else if pct >= 0.5 {
        Theme::WARNING
    } else {
        Theme::EMBER
    }
}

fn hdr_fmt_bytes(b: u64) -> String {
    fmt_bytes_short(b)
}

/// Format a byte count as a short human-readable string (e.g. "12G", "384M").
///
/// Public so that other modules (e.g. warning bar in `state.rs`) can reuse it.
pub fn fmt_bytes_short(b: u64) -> String {
    const GIB: u64 = 1 << 30;
    const MIB: u64 = 1 << 20;
    if b >= GIB {
        format!("{:.0}G", b as f64 / GIB as f64)
    } else if b >= MIB {
        format!("{:.0}M", b as f64 / MIB as f64)
    } else {
        format!("{}K", b / 1024)
    }
}

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

/// Derive a short queue/plan label from the TUI state.
fn queue_label(state: &TuiState) -> Option<String> {
    // Prefer active plan names; fall back to first plan.
    if let Some(active) = state.plans.iter().find(|p| p.active) {
        return Some(truncate_label(&active.id, 24));
    }
    if let Some(first) = state.plans.first() {
        return Some(truncate_label(&first.id, 24));
    }
    None
}

fn truncate_label(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}..", &s[..max.saturating_sub(2)])
    }
}

use crate::tui::display_utils::shorten_model;

/// Badge color for tab content counts.
///
/// Returns a semantic color per tab: SAGE for agents, EMBER for errors/failures,
/// WARNING for pending items, DREAM for informational counts.
fn tab_badge_color(tab: super::super::tabs::Tab) -> Color {
    use super::super::tabs::Tab;
    match tab {
        Tab::Agents => Theme::SAGE,
        Tab::Plans | Tab::Logs => Theme::EMBER,
        Tab::Git => Theme::WARNING,
        Tab::Learning => Theme::DREAM,
        _ => Theme::FG_DIM,
    }
}

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the header bar with all 8 sections.
pub fn render_header_bar(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let bg = Style::default().bg(Theme::BG_SECONDARY);
    let compact = area.width < 120;

    let (done, total) = state.task_counts();
    let elapsed_secs = state.elapsed_secs() as u64;
    let elapsed_str = format_elapsed(elapsed_secs);

    let mut spans: Vec<Span<'static>> = vec![Span::styled(" ", bg)];

    // ── 1. Health-aware pulsing status dot ──────────────────────────
    let health = HealthStatus::from_state(state);
    let hb_idx = (state.atmosphere.frame() / 15) as usize % HEARTBEAT_FRAMES.len();
    let base_color = health.color();
    let brightness = state.atmosphere.heartbeat();
    let dot_color = match base_color {
        Color::Rgb(r, g, b) => Color::Rgb(
            ((r as f64) * brightness).clamp(0.0, 255.0) as u8,
            ((g as f64) * brightness).clamp(0.0, 255.0) as u8,
            ((b as f64) * brightness).clamp(0.0, 255.0) as u8,
        ),
        other => other,
    };
    spans.push(Span::styled(
        HEARTBEAT_FRAMES[hb_idx],
        Style::default().fg(dot_color).bg(Theme::BG_SECONDARY),
    ));

    // App name
    spans.push(Span::styled(
        " roko",
        Style::default()
            .fg(Theme::ROSE)
            .bg(Theme::BG_SECONDARY)
            .add_modifier(Modifier::BOLD),
    ));

    // ── 1b. Queue/plan name ──────────────────────────────────────────
    if let Some(label) = queue_label(state) {
        spans.push(Span::styled(
            format!("  {label}"),
            Style::default().fg(Theme::BONE_DIM).bg(Theme::BG_SECONDARY),
        ));
    }

    // ── 2. Wave indicator ─────────────────────────────────────────────
    if !state.execution_waves.is_empty() {
        let total_waves = state.wave_count();
        let wave_idx = state.current_wave() + 1;
        spans.push(Span::styled(
            format!("  Wave {wave_idx}/{total_waves}"),
            Style::default().fg(Theme::BONE).bg(Theme::BG_SECONDARY),
        ));
    }

    spans.push(sep());

    // ── 3. Progress bar with fire gradient ────────────────────────────
    let bar_width = 15usize;
    if total > 0 {
        let fraction = done as f64 / total.max(1) as f64;
        let filled = (fraction * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar_color = gradient_fire().sample(fraction);

        spans.push(Span::styled("  ", bg));
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

    // ── 4. Plan count with semantic coloring ──────────────────────────
    let fill_pct = if total > 0 {
        done as f64 / total as f64
    } else {
        0.0
    };
    let all_done = state.plans.iter().all(|p| !p.active);
    let has_failures = state.plans.iter().any(|p| p.tasks_failed > 0);

    let progress_text = if all_done && total > 0 && !has_failures {
        " COMPLETE".to_string()
    } else if has_failures {
        format!(" ERR:{done}/{total}")
    } else {
        format!("  {done}/{total}")
    };
    let progress_style = if has_failures {
        Style::default()
            .fg(Theme::EMBER)
            .add_modifier(Modifier::BOLD)
    } else if all_done && total > 0 {
        Style::default()
            .fg(Theme::SAGE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::semantic_color(fill_pct))
    };
    spans.push(Span::styled(
        progress_text,
        progress_style.bg(Theme::BG_SECONDARY),
    ));

    // Percentage (hidden when compact)
    if !compact && total > 0 && !(all_done && !has_failures) {
        let pct = (fill_pct * 100.0) as u32;
        spans.push(Span::styled(
            format!("  {pct}%"),
            Style::default()
                .fg(Theme::semantic_color(fill_pct))
                .bg(Theme::BG_SECONDARY),
        ));
    }

    // In-flight agent count
    let in_flight = state.active_agent_count();
    if in_flight > 0 && !(all_done && !has_failures) {
        spans.push(Span::styled(
            format!("  {in_flight}\u{25b8}"),
            Style::default().fg(Theme::ROSE_DIM).bg(Theme::BG_SECONDARY),
        ));
    }

    spans.push(sep());

    // ── 5. ETA / elapsed / cost / tokens ──────────────────────────────
    // ETA estimate: prefer critical-path ETA, fall back to proportional.
    if total > 0 && done < total {
        let eta_display = if let Some(cp_minutes) = state.critical_path_eta_minutes {
            let cp_secs = cp_minutes as u64 * 60;
            Some((format_elapsed(cp_secs.max(1)), "CP-ETA"))
        } else if done > 0 {
            let rate = elapsed_secs as f64 / done as f64;
            let remaining = ((total - done) as f64 * rate) as u64;
            Some((format_elapsed(remaining.max(1)), "ETA"))
        } else {
            None
        };
        if let Some((eta_str, label)) = eta_display {
            spans.push(Span::styled(
                format!("  {label}:{eta_str}"),
                Style::default().fg(Theme::DREAM).bg(Theme::BG_SECONDARY),
            ));
        }
    }

    // Elapsed
    spans.push(Span::styled(
        format!("  {elapsed_str}"),
        Style::default().fg(Theme::FG_DIM).bg(Theme::BG_SECONDARY),
    ));

    // Cost
    let aggregate_budget = state.aggregate_plan_budget();
    if state.cost_dollars > 0.001 || aggregate_budget > 0.0 {
        let cost_str = if state.cost_dollars >= 1.0 {
            format!("${:.2}", state.cost_dollars)
        } else {
            format!("${:.3}", state.cost_dollars)
        };
        let cost_str = if aggregate_budget > 0.0 {
            format!(
                "{cost_str}/${aggregate_budget:.2} ({:.0}%)",
                state.cost_dollars / aggregate_budget * 100.0
            )
        } else {
            cost_str
        };
        spans.push(Span::styled(
            format!("  {cost_str}"),
            Style::default().fg(Theme::BONE_DIM).bg(Theme::BG_SECONDARY),
        ));
    }

    // Tokens
    if state.token_total > 0 {
        let tok_display = if state.token_total >= 1_000_000 {
            format!("  {}M tok", state.token_total / 1_000_000)
        } else if state.token_total >= 1_000 {
            format!("  {}K tok", state.token_total / 1_000)
        } else {
            format!("  {} tok", state.token_total)
        };
        spans.push(Span::styled(
            tok_display,
            Style::default().fg(Theme::FG_DIM).bg(Theme::BG_SECONDARY),
        ));
    }

    spans.push(sep());

    // ── 6. System metrics (CPU/MEM) ───────────────────────────────────
    {
        let colon = if compact { "" } else { ":" };

        let cpu_frac = state.sys.cpu_pct as f64 / 100.0;
        spans.push(Span::styled(
            format!(" C{}{:.0}%", colon, state.sys.cpu_pct),
            Style::default()
                .fg(hdr_pct_color(cpu_frac))
                .bg(Theme::BG_SECONDARY),
        ));

        let mem_frac = if state.sys.mem_total_bytes > 0 {
            state.sys.mem_used_bytes as f64 / state.sys.mem_total_bytes as f64
        } else {
            0.0
        };
        spans.push(Span::styled(
            format!(" M{}{}", colon, hdr_fmt_bytes(state.sys.mem_used_bytes)),
            Style::default()
                .fg(hdr_pct_color(mem_frac))
                .bg(Theme::BG_SECONDARY),
        ));
    }

    // ── 6b. Network stats (agents online + gate pass rate) ────────────
    let agent_color = if state.agents_online > 0 {
        Theme::SAGE
    } else {
        Theme::TEXT_GHOST
    };
    spans.push(Span::styled(
        format!(" {}ag", state.agents_online),
        Style::default().fg(agent_color).bg(Theme::BG_SECONDARY),
    ));
    match state.gate_pass_rate {
        Some(pass_rate) => {
            let pct = (pass_rate.clamp(0.0, 1.0) * 100.0).round();
            spans.push(Span::styled(
                format!(" GATES:{pct:.0}%"),
                Style::default()
                    .fg(hdr_success_color(pass_rate))
                    .bg(Theme::BG_SECONDARY),
            ));
        }
        None => {
            spans.push(Span::styled(
                " GATES:—",
                Style::default()
                    .fg(Theme::TEXT_GHOST)
                    .bg(Theme::BG_SECONDARY),
            ));
        }
    }

    // ── 6c. MCP connections, NET rate, DSK free, FPS ──────────────────
    if !compact {
        if state.mcp_connection_count > 0 {
            spans.push(Span::styled(
                format!(" MCP:{}", state.mcp_connection_count),
                Style::default().fg(Theme::DREAM).bg(Theme::BG_SECONDARY),
            ));
        }

        let net_rate = state.sys.net_down_bytes_sec + state.sys.net_up_bytes_sec;
        if net_rate > 0 {
            spans.push(Span::styled(
                format!(" NET:{}/s", hdr_fmt_bytes(net_rate)),
                Style::default().fg(Theme::BONE_DIM).bg(Theme::BG_SECONDARY),
            ));
        }

        if state.sys.disk_free_bytes > 0 {
            let dsk_color = if state.sys.disk_free_bytes < (1 << 30) {
                Theme::EMBER
            } else {
                Theme::BONE_DIM
            };
            spans.push(Span::styled(
                format!(" DSK:{}", hdr_fmt_bytes(state.sys.disk_free_bytes)),
                Style::default().fg(dsk_color).bg(Theme::BG_SECONDARY),
            ));
        }

        if state.tui_fps > 0.0 {
            spans.push(Span::styled(
                format!(" {:.0}fps", state.tui_fps),
                Style::default()
                    .fg(Theme::TEXT_GHOST)
                    .bg(Theme::BG_SECONDARY),
            ));
        }
    }

    spans.push(sep());

    // ── 7. Active agent spinner with role label ───────────────────────
    if let Some(agent) = state.agents.iter().find(|a| a.active) {
        let short = shorten_model(&agent.model);
        let role_color = Theme::role_accent(&agent.role);
        spans.push(Span::styled(
            format!("  {} {}({})", state.atmosphere.spinner(), agent.role, short),
            Style::default().fg(role_color).bg(Theme::BG_SECONDARY),
        ));
    }

    // ── 8. F-key strip (right-aligned) ────────────────────────────────
    use super::super::tabs::Tab;

    let all_fkey_items: Vec<(&str, Color, &str, Tab)> = vec![
        (" F1", Theme::ROSE, "dash", Tab::Dashboard),
        (" F2", Theme::BONE_DIM, "plans", Tab::Plans),
        (" F3", Theme::SAGE, "agents", Tab::Agents),
        (" F4", Theme::DREAM, "git", Tab::Git),
        (" F5", Theme::DREAM, "logs", Tab::Logs),
        (" F6", Theme::BONE_DIM, "cfg", Tab::Config),
        (" F7", Theme::BONE_DIM, "inspect", Tab::Inspect),
        (" F8", Theme::SAGE, "market", Tab::Marketplace),
        (" F9", Theme::DREAM, "atelier", Tab::Atelier),
        (" F10", Theme::BONE_DIM, "learn", Tab::Learning),
    ];

    // Keep the operational metrics legible instead of reserving a tab strip
    // wider than the terminal. Compact headers show the active destination;
    // medium headers add the four highest-frequency views.
    let current_tab = state.active_tab;
    let fkey_items = if area.width >= 150 {
        all_fkey_items
    } else if area.width >= 95 {
        all_fkey_items
            .into_iter()
            .filter(|(_, _, _, tab)| {
                *tab == current_tab
                    || matches!(tab, Tab::Dashboard | Tab::Plans | Tab::Agents | Tab::Logs)
            })
            .collect()
    } else {
        all_fkey_items
            .into_iter()
            .filter(|(_, _, _, tab)| *tab == current_tab)
            .collect()
    };

    // Compute badge counts for each tab (only shown on inactive tabs).
    let badges: Vec<usize> = fkey_items
        .iter()
        .map(|(_, _, _, tab)| {
            if *tab == current_tab {
                0
            } else {
                state.tab_badge(*tab)
            }
        })
        .collect();

    let fkey_width: u16 = fkey_items
        .iter()
        .zip(badges.iter())
        .map(|((k, _, l, _), &badge)| {
            let base = k.len() + 1 + l.len();
            if badge > 0 {
                // e.g. "(3)" adds the formatted count length
                base + format!("({badge})").len()
            } else {
                base
            }
        })
        .sum::<usize>() as u16
        + 1; // trailing space

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(fkey_width)])
        .split(area);

    // Render left content
    let left_line = Line::from(spans);
    frame.render_widget(Paragraph::new(left_line).style(bg), chunks[0]);

    // Render F-key indicators with active tab highlighting and badges
    let mut fkey_spans: Vec<Span<'static>> = Vec::new();
    for ((key, color, label, tab), &badge) in fkey_items.iter().zip(badges.iter()) {
        let is_active = *tab == current_tab;
        if is_active {
            fkey_spans.push(Span::styled(
                format!("{key}:{label}"),
                Style::default()
                    .fg(Theme::VOID)
                    .bg(*color)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            fkey_spans.push(Span::styled(
                key.to_string(),
                Style::default()
                    .fg(*color)
                    .bg(Theme::BG_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ));
            fkey_spans.push(Span::styled(
                format!(":{label}"),
                Style::default().fg(Theme::FG_DIM).bg(Theme::BG_SECONDARY),
            ));
            // Append badge count when > 0
            if badge > 0 {
                let badge_color = tab_badge_color(*tab);
                fkey_spans.push(Span::styled(
                    format!("({badge})"),
                    Style::default().fg(badge_color).bg(Theme::BG_SECONDARY),
                ));
            }
        }
    }
    fkey_spans.push(Span::styled(" ", bg));

    let fkey_line = Line::from(fkey_spans);
    frame.render_widget(Paragraph::new(fkey_line).style(bg), chunks[1]);
}

// ---------------------------------------------------------------------------
// Warning bar (P4.6) — 1-line bar below header for active warnings
// ---------------------------------------------------------------------------

/// Returns the height needed for the warning bar (0 or 1).
#[must_use]
pub fn warning_bar_height(state: &TuiState) -> u16 {
    if state.active_warnings().is_empty() {
        0
    } else {
        1
    }
}

/// Render a 1-line warning bar. Only call when `warning_bar_height > 0`.
pub fn render_warning_bar(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let warnings = state.active_warnings();
    if warnings.is_empty() || area.height == 0 {
        return;
    }

    let bg = Style::default().bg(Theme::ROSE_EMBER);
    let text = warnings.join("  |  ");
    let spans = vec![
        Span::styled(
            " \u{26a0} ",
            Style::default()
                .fg(Theme::WARNING)
                .bg(Theme::ROSE_EMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(text, Style::default().fg(Theme::BONE).bg(Theme::ROSE_EMBER)),
        Span::styled(
            "  [n] dismiss",
            Style::default().fg(Theme::TEXT_GHOST).bg(Theme::ROSE_EMBER),
        ),
    ];

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line).style(bg), area);
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
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn header_bar_renders_without_panic() {
        let data = DashboardData::default();
        let state = TuiState::from_dashboard_data(&data);
        let backend = TestBackend::new(140, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header_bar(frame, area, &state);
            })
            .unwrap();
    }

    #[test]
    fn header_bar_compact() {
        let data = DashboardData::default();
        let state = TuiState::from_dashboard_data(&data);
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header_bar(frame, area, &state);
            })
            .unwrap();
    }

    #[test]
    fn header_bar_renders_network_stats_and_missing_gate_rate() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.agents_online = 0;
        state.gate_pass_rate = None;
        let backend = TestBackend::new(140, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header_bar(frame, area, &state);
            })
            .unwrap();

        let text = rendered_text(&terminal);
        assert!(text.contains("0ag"));
        assert!(text.contains("GATES:—"));
    }

    #[test]
    fn header_bar_renders_gate_pass_rate_as_percentage() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.agents_online = 3;
        state.gate_pass_rate = Some(0.75);
        let backend = TestBackend::new(140, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header_bar(frame, area, &state);
            })
            .unwrap();

        let text = rendered_text(&terminal);
        assert!(text.contains("3ag"));
        assert!(text.contains("GATES:75%"));
    }

    #[test]
    fn header_bar_renders_spend_budget_and_utilization() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.cost_dollars = 2.5;
        state.max_plan_budget_usd = 10.0;
        state.plans.push(Default::default());
        let backend = TestBackend::new(160, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_header_bar(frame, frame.area(), &state))
            .unwrap();

        let text = rendered_text(&terminal);
        assert!(text.contains("$2.50/$10.00 (25%)"));
    }

    #[test]
    fn health_status_idle_when_no_plans() {
        let state = TuiState::from_dashboard_data(&DashboardData::default());
        assert_eq!(HealthStatus::from_state(&state), HealthStatus::Idle);
    }

    #[test]
    fn health_status_healthy_with_active_plan() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.plans.push(super::super::super::state::PlanEntry {
            active: true,
            ..Default::default()
        });
        assert_eq!(HealthStatus::from_state(&state), HealthStatus::Healthy);
    }

    #[test]
    fn health_status_error_with_failures() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.plans.push(super::super::super::state::PlanEntry {
            active: true,
            tasks_failed: 1,
            ..Default::default()
        });
        assert_eq!(HealthStatus::from_state(&state), HealthStatus::Error);
    }

    #[test]
    fn health_status_gating_when_in_gate_phase() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.plans.push(super::super::super::state::PlanEntry {
            active: true,
            ..Default::default()
        });
        state.current_phase = "gating".to_string();
        assert_eq!(HealthStatus::from_state(&state), HealthStatus::Gating);
    }

    #[test]
    fn queue_label_shows_active_plan() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.plans.push(super::super::super::state::PlanEntry {
            id: "my-cool-plan".to_string(),
            active: true,
            ..Default::default()
        });
        assert_eq!(queue_label(&state), Some("my-cool-plan".to_string()));
    }

    #[test]
    fn queue_label_truncates_long_names() {
        let mut state = TuiState::from_dashboard_data(&DashboardData::default());
        state.plans.push(super::super::super::state::PlanEntry {
            id: "very-long-plan-name-that-exceeds-24-chars".to_string(),
            active: true,
            ..Default::default()
        });
        let label = queue_label(&state).unwrap();
        assert!(label.len() <= 24);
        assert!(label.ends_with(".."));
    }
}
