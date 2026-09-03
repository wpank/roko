//! Compact phase widget with segmented bar, colored badges, and flow diagram.
//!
//! Line 1: Segmented phase bar — each phase gets a fixed-width cell,
//!         colored by status (Done=SAGE, Active=gradient+spinner, Pending=dashes).
//! Line 2: Active phase detail with badge, elapsed time, and phase flow transitions.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::atmosphere::Atmosphere;
use super::super::state::{PhaseStatus, TuiState};
use crate::tui::Theme;

// ---------------------------------------------------------------------------
// Phase labels
// ---------------------------------------------------------------------------

// Phase labels reserved for future abbreviated display.
// The segmented bar currently uses phase_pipeline names directly.

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render a compact phase widget that adapts to available height.
///
/// When idle (no phases or all pending), collapses to a single line.
/// When active, shows a segmented bar + detail line + phase flow.
///
/// ```text
/// ┌ Phase . implementer ────────────────────┐
/// │ ████████████████████░░░░░░────────────── │
/// │ * implementer  42%  2m31s                │
/// └─────────────────────────────────────────┘
/// ```
///
/// Idle (single line):
/// ```text
/// Phase: idle
/// ```
pub fn render_phase_compact(frame: &mut Frame<'_>, area: Rect, state: &TuiState, focused: bool) {
    let atm = &state.atmosphere;

    // Find active phase index
    let active_idx = state
        .phase_pipeline
        .iter()
        .position(|s| s.status == PhaseStatus::Active);

    let is_idle = state.phase_pipeline.is_empty()
        || state
            .phase_pipeline
            .iter()
            .all(|s| s.status == PhaseStatus::Pending);

    // Collapse to a single borderless line when idle and space is tight.
    if is_idle && area.height <= 2 {
        let line = Line::from(vec![
            Span::styled("Phase: ", Style::default().fg(Theme::TEXT_DIM)),
            Span::styled("idle", Style::default().fg(Theme::TEXT_GHOST)),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let theme = Theme::dark();
    let active_phase_name = active_idx.map(|idx| state.phase_pipeline[idx].name.as_str());

    let title_spans = if let Some(name) = active_phase_name {
        let badge = phase_badge(name);
        vec![
            Span::styled("Phase ", theme.section_header()),
            badge,
        ]
    } else {
        vec![Span::styled("Phase", theme.section_header())]
    };

    let (border_style, _ttl_style) = if focused {
        (Theme::focused_border_style(), Theme::focused_title_style())
    } else {
        (
            Theme::unfocused_border_style(),
            Theme::unfocused_title_style(),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(title_spans))
        .style(Theme::block_style())
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 1 || inner.width < 10 {
        return;
    }

    let inner_w = inner.width as usize;

    // ── Line 1: Segmented phase bar ──────────────────────────────────────
    let total_phases = state.phase_pipeline.len().max(1);
    let seg_width = inner_w / total_phases;
    let leftover = inner_w.saturating_sub(seg_width * total_phases);

    let spinner_ch = atm.spinner_ethereal();
    let mut bar_spans: Vec<Span> = Vec::new();

    for (i, step) in state.phase_pipeline.iter().enumerate() {
        let w = if i == total_phases - 1 {
            seg_width + leftover
        } else {
            seg_width
        };

        // Progress fraction for gradient coloring within active segment.
        let seg_frac = (i as f64) / (total_phases as f64).max(1.0);

        match step.status {
            PhaseStatus::Done => {
                // Gradient fill: earlier phases are warmer, later phases greener.
                for col in 0..w {
                    let t = seg_frac + (col as f64 / (inner_w as f64).max(1.0));
                    let color = Theme::progress_gradient(t.min(1.0));
                    bar_spans.push(Span::styled(
                        "\u{2588}",
                        Style::default().fg(color),
                    ));
                }
            }
            PhaseStatus::Active => {
                if w > 0 {
                    // Gradient fill for completed portion of active segment.
                    let fill_count = w.saturating_sub(1);
                    for col in 0..fill_count {
                        let t = seg_frac + (col as f64 / (inner_w as f64).max(1.0));
                        let color = Theme::progress_gradient(t.min(1.0));
                        bar_spans.push(Span::styled(
                            "\u{2588}",
                            Style::default().fg(color),
                        ));
                    }
                    bar_spans.push(Span::styled(
                        spinner_ch.to_string(),
                        Style::default()
                            .fg(Theme::WARNING)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }
            PhaseStatus::Failed => {
                let fill: String = "\u{2588}".repeat(w);
                bar_spans.push(Span::styled(fill, Style::default().fg(Theme::EMBER)));
            }
            PhaseStatus::Pending => {
                let fill: String = "\u{2500}".repeat(w);
                bar_spans.push(Span::styled(fill, Style::default().fg(Theme::TEXT_GHOST)));
            }
        }
    }

    let bar_line = Line::from(bar_spans);
    frame.render_widget(
        Paragraph::new(bar_line),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    if inner.height < 2 {
        return;
    }

    // ── Line 2: Active phase detail or phase flow ────────────────────────
    let detail_line = if let Some(idx) = state
        .phase_pipeline
        .iter()
        .position(|s| s.status == PhaseStatus::Failed)
    {
        let step = &state.phase_pipeline[idx];
        let mut spans = vec![
            Span::styled(
                " HALTED ",
                theme.badge_failed(),
            ),
            Span::styled(format!(" at {}", step.name), Style::default().fg(Theme::EMBER)),
        ];
        if step.elapsed_secs > 0.0 {
            spans.push(Span::styled(
                format!("  {}", format_elapsed(step.elapsed_secs)),
                Style::default().fg(Theme::TEXT_DIM),
            ));
        }
        Line::from(spans)
    } else if let Some(idx) = active_idx {
        build_active_detail(&state.phase_pipeline[idx], atm, inner_w)
    } else {
        // All done or all pending
        let all_done = state
            .phase_pipeline
            .iter()
            .all(|s| s.status == PhaseStatus::Done);
        if all_done && !state.phase_pipeline.is_empty() {
            Line::from(vec![
                Span::styled(
                    " COMPLETE ",
                    theme.badge_complete(),
                ),
                Span::styled(" all phases done", Style::default().fg(Theme::SAGE)),
            ])
        } else if state.phase_pipeline.is_empty() {
            Line::from(Span::styled(
                "no phases configured",
                Style::default().fg(Theme::TEXT_DIM),
            ))
        } else {
            // Show phase flow: "dispatch > gate > merge"
            build_phase_flow(&state.phase_pipeline)
        }
    };

    frame.render_widget(
        Paragraph::new(detail_line),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the active-phase detail line: badge + name + pct + elapsed.
fn build_active_detail(
    step: &super::super::state::PhaseStep,
    atm: &Atmosphere,
    width: usize,
) -> Line<'static> {
    let pulse_color = pulse_active(atm.heartbeat());
    let icon = atm.spinner_ethereal().to_string();
    let badge = phase_badge(&step.name);

    let mut spans = vec![
        badge,
        Span::styled(
            icon,
            Style::default()
                .fg(pulse_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", step.name),
            Style::default()
                .fg(Theme::ROSE)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    // Percentage with gradient color
    if step.pct > 0.0 {
        let pct_color = Theme::progress_gradient(step.pct as f64 / 100.0);
        spans.push(Span::styled(
            format!(" {:.0}%", step.pct.min(99.0)),
            Style::default().fg(pct_color),
        ));
    }

    // Elapsed time
    if step.elapsed_secs > 0.0 {
        let time_str = format!(" {}", format_elapsed(step.elapsed_secs));
        let time_style = {
            let pulse = atm.heartbeat();
            let base_r = 170.0_f64;
            let r = (base_r * pulse).clamp(0.0, 255.0) as u8;
            Style::default().fg(Color::Rgb(r, 112, 136))
        };
        spans.push(Span::styled(time_str, time_style));
    }

    // If there's room, append a compact phase position indicator.
    let current_len: usize = spans.iter().map(|s| s.content.len()).sum();
    if width > current_len + 8 {
        spans.push(Span::styled(
            format!("  [{}/{}]", step.pct.min(100.0) as u32 / 10, 10),
            Style::default().fg(Theme::TEXT_DIM),
        ));
    }

    Line::from(spans)
}

/// Build a phase flow line: `impl -> gate -> done` with colored arrows.
fn build_phase_flow(pipeline: &[super::super::state::PhaseStep]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, step) in pipeline.iter().enumerate() {
        if i > 0 {
            // Arrow color matches the outcome of the *previous* step.
            let prev = &pipeline[i - 1];
            let arrow_color = match prev.status {
                PhaseStatus::Done => Theme::SAGE,
                PhaseStatus::Failed => Theme::EMBER,
                _ => Theme::TEXT_GHOST,
            };
            spans.push(Span::styled(
                " \u{2192} ",
                Style::default().fg(arrow_color),
            ));
        }
        // Phase name or completion checkmark.
        let (label, style) = match step.status {
            PhaseStatus::Done => (
                "\u{2713}".to_string(),
                Style::default().fg(Theme::SAGE),
            ),
            PhaseStatus::Active => (
                step.name.clone(),
                Style::default()
                    .fg(Theme::WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
            PhaseStatus::Failed => (
                step.name.clone(),
                Style::default()
                    .fg(Theme::EMBER)
                    .add_modifier(Modifier::BOLD),
            ),
            PhaseStatus::Pending => (
                step.name.clone(),
                Style::default().fg(Theme::TEXT_DIM),
            ),
        };
        spans.push(Span::styled(label, style));
    }
    Line::from(spans)
}

/// Return a colored badge span for the given phase name.
///
/// Maps phase names to abbreviated badges:
/// `[IMPL]` green, `[GATE]` yellow, `[REPLAN]` red, `[COMPLETE]` bright green,
/// `[HALTED]` red. Falls back to a dim generic badge.
fn phase_badge(phase_name: &str) -> Span<'static> {
    let theme = Theme::dark();
    let lower = phase_name.to_ascii_lowercase();
    if lower.contains("implement") || lower.contains("dispatch") || lower.contains("exec") {
        Span::styled(" IMPL ", theme.badge_running())
    } else if lower.contains("gate") || lower.contains("compil") || lower.contains("test")
        || lower.contains("verify")
    {
        Span::styled(" GATE ", theme.badge_pending())
    } else if lower.contains("replan") || lower.contains("fail") {
        Span::styled(" REPLAN ", theme.badge_failed())
    } else if lower.contains("complete") || lower.contains("done") {
        Span::styled(" DONE ", theme.badge_complete())
    } else if lower.contains("halt") {
        Span::styled(" HALTED ", theme.badge_failed())
    } else if lower.contains("preflight") || lower.contains("prep") {
        Span::styled(
            " PREP ",
            Style::default()
                .fg(Theme::VOID)
                .bg(Theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        )
    } else if lower.contains("review") || lower.contains("critic") {
        Span::styled(
            " REVIEW ",
            Style::default()
                .fg(Theme::VOID)
                .bg(Theme::BONE_DIM)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" {} ", &phase_name.to_ascii_uppercase().chars().take(4).collect::<String>()),
            Style::default()
                .fg(Theme::VOID)
                .bg(Theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        )
    }
}

/// Format elapsed seconds into a human-readable compact string.
fn format_elapsed(secs: f64) -> String {
    let s = secs as u64;
    if s >= 3600 {
        format!("{}h{}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{}s", s)
    }
}

/// Modulate a base rose color with heartbeat pulse.
fn pulse_active(heartbeat: f64) -> Color {
    let base_r = 170.0;
    let base_g = 112.0;
    let base_b = 136.0;
    let scale = heartbeat.clamp(0.9, 1.1);
    Color::Rgb(
        (base_r * scale).min(255.0) as u8,
        (base_g * scale).min(255.0) as u8,
        (base_b * scale).min(255.0) as u8,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::super::state::{PhaseStatus, PhaseStep, TuiState};
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn make_state(steps: Vec<PhaseStep>) -> TuiState {
        use super::super::super::dashboard::DashboardData;
        let data = DashboardData::default();
        let mut state = TuiState::from_dashboard_data(&data);
        state.phase_pipeline = steps;
        state
    }

    #[test]
    fn phase_compact_renders_without_panic() {
        let steps = vec![
            PhaseStep {
                name: "preflight".into(),
                status: PhaseStatus::Done,
                elapsed_secs: 3.0,
                pct: 100.0,
            },
            PhaseStep {
                name: "implementer".into(),
                status: PhaseStatus::Active,
                elapsed_secs: 45.0,
                pct: 42.0,
            },
            PhaseStep {
                name: "reviewing".into(),
                status: PhaseStatus::Pending,
                elapsed_secs: 0.0,
                pct: 0.0,
            },
        ];
        let state = make_state(steps);
        let backend = TestBackend::new(60, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_phase_compact(frame, area, &state, false);
            })
            .unwrap();
    }

    #[test]
    fn phase_compact_all_done() {
        let steps = vec![
            PhaseStep {
                name: "preflight".into(),
                status: PhaseStatus::Done,
                elapsed_secs: 3.0,
                pct: 100.0,
            },
            PhaseStep {
                name: "implementer".into(),
                status: PhaseStatus::Done,
                elapsed_secs: 60.0,
                pct: 100.0,
            },
        ];
        let state = make_state(steps);
        let backend = TestBackend::new(60, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_phase_compact(frame, area, &state, true);
            })
            .unwrap();
    }

    #[test]
    fn phase_compact_empty_pipeline() {
        let state = make_state(Vec::new());
        let backend = TestBackend::new(60, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_phase_compact(frame, area, &state, false);
            })
            .unwrap();
    }

    #[test]
    fn phase_compact_failed_state() {
        let steps = vec![
            PhaseStep {
                name: "preflight".into(),
                status: PhaseStatus::Done,
                elapsed_secs: 3.0,
                pct: 100.0,
            },
            PhaseStep {
                name: "compile-gate".into(),
                status: PhaseStatus::Failed,
                elapsed_secs: 12.0,
                pct: 80.0,
            },
            PhaseStep {
                name: "reviewing".into(),
                status: PhaseStatus::Pending,
                elapsed_secs: 0.0,
                pct: 0.0,
            },
        ];
        let state = make_state(steps);
        let backend = TestBackend::new(60, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_phase_compact(frame, area, &state, false);
            })
            .unwrap();
    }
}
