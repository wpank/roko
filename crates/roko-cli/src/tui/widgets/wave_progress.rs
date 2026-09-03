//! Wave progress ribbon — proportional segments per execution wave with
//! gradient progress bars, status badges, timing info, and task counts.
//!
//! Ported from Mori's wave_progress.rs.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::super::state::TuiState;
use crate::tui::theme::gradient_progress;
use crate::tui::Theme;

// ---------------------------------------------------------------------------
// Gradient bar characters (descending density)
// ---------------------------------------------------------------------------

const GRAD_FULL: char = '\u{2588}'; // █
const GRAD_3Q: char = '\u{2593}'; // ▓
const GRAD_HALF: char = '\u{2592}'; // ▒
const GRAD_1Q: char = '\u{2591}'; // ░

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Determine wave status from its counters and position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaveStatus {
    Done,
    Active,
    Pending,
}

fn wave_status(wave_idx: usize, current_wave: usize, fraction: f64) -> WaveStatus {
    if fraction >= 1.0 {
        WaveStatus::Done
    } else if wave_idx == current_wave {
        WaveStatus::Active
    } else {
        WaveStatus::Pending
    }
}

/// Format seconds into a compact `Xm Ys` or `Xs` string.
fn fmt_elapsed(secs: f64) -> String {
    let s = secs as u64;
    if s >= 60 {
        format!("{}m{}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

/// Render a gradient progress bar segment into `spans`.
///
/// Uses `█▓▒░` gradient characters. The fill transitions red->yellow->green
/// through `gradient_progress()`, while the empty portion uses dim `░`.
fn push_gradient_bar(spans: &mut Vec<Span<'static>>, width: usize, fraction: f64) {
    if width == 0 {
        return;
    }
    let grad = gradient_progress();
    let filled_exact = fraction * width as f64;
    let filled_full = filled_exact as usize;
    let partial = filled_exact - filled_full as f64;

    // Filled cells — per-cell color gradient
    for j in 0..filled_full.min(width) {
        let t = if width > 1 {
            j as f64 / (width - 1) as f64
        } else {
            fraction
        };
        let c = grad.sample(t);
        spans.push(Span::styled(
            String::from(GRAD_FULL),
            Style::default().fg(c),
        ));
    }

    // Partial cell at the fill boundary
    if filled_full < width {
        let edge_t = if width > 1 {
            filled_full as f64 / (width - 1) as f64
        } else {
            fraction
        };
        let edge_c = grad.sample(edge_t);
        let ch = if partial >= 0.75 {
            GRAD_3Q
        } else if partial >= 0.5 {
            GRAD_HALF
        } else {
            GRAD_1Q
        };
        // Only show partial if there is actual progress at this cell
        if partial > 0.01 {
            spans.push(Span::styled(
                String::from(ch),
                Style::default().fg(edge_c),
            ));
        } else {
            spans.push(Span::styled(
                String::from(GRAD_1Q),
                Style::default().fg(Theme::TEXT_PHANTOM),
            ));
        }

        // Empty cells
        let remaining = width.saturating_sub(filled_full + 1);
        if remaining > 0 {
            spans.push(Span::styled(
                GRAD_1Q.to_string().repeat(remaining),
                Style::default().fg(Theme::TEXT_PHANTOM),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Public render
// ---------------------------------------------------------------------------

/// Render the wave progress ribbon.
///
/// Layout (multi-line when height >= 2):
///   Line 0: title + overall summary
///   Line 1+: one line per wave with badge, gradient bar, and counts
///
/// Falls back to single-line ribbon when height == 1.
pub fn render_wave_progress(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let theme = Theme::from_env();

    if state.execution_waves.is_empty() {
        let placeholder = Line::from(Span::styled("No execution waves", theme.label()));
        frame.render_widget(Paragraph::new(placeholder), area);
        return;
    }

    let total_plans: usize = state.execution_waves.iter().map(|w| w.total).sum();
    if total_plans == 0 {
        return;
    }

    let width = area.width as usize;
    if width < 10 {
        return;
    }

    let total_done: usize = state.execution_waves.iter().map(|w| w.done).sum();
    let overall_pct = if total_plans > 0 {
        (total_done as f64 / total_plans as f64 * 100.0).round() as u32
    } else {
        0
    };
    let current_wave = state.current_wave();
    let elapsed = state.atmosphere.elapsed();

    let height = area.height as usize;

    if height >= 2 {
        render_multiline(
            frame,
            area,
            state,
            &theme,
            total_done,
            total_plans,
            overall_pct,
            current_wave,
            elapsed,
            width,
        );
    } else {
        render_single_line(
            frame,
            area,
            state,
            &theme,
            total_done,
            total_plans,
            overall_pct,
            current_wave,
            width,
        );
    }
}

/// Multi-line layout: title row + one row per wave.
#[allow(clippy::too_many_arguments)]
fn render_multiline(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    theme: &Theme,
    total_done: usize,
    total_plans: usize,
    overall_pct: u32,
    current_wave: usize,
    elapsed_secs: f64,
    width: usize,
) {
    let mut lines: Vec<Line> = Vec::new();

    // --- Title line ---
    let mut title_spans: Vec<Span> = Vec::new();
    title_spans.push(Span::styled("Waves", theme.section_header()));
    title_spans.push(Span::styled("  ", Style::default()));
    // Overall counts
    let counts_color = Theme::progress_gradient(overall_pct as f64 / 100.0);
    title_spans.push(Span::styled(
        format!("{total_done}/{total_plans}"),
        Style::default()
            .fg(counts_color)
            .add_modifier(Modifier::BOLD),
    ));
    title_spans.push(Span::styled(
        format!(" ({overall_pct}%)"),
        theme.label(),
    ));
    // Estimated remaining time
    if overall_pct > 0 && overall_pct < 100 && elapsed_secs > 1.0 {
        let rate = total_done as f64 / elapsed_secs;
        if rate > 0.0 {
            let remaining_secs = (total_plans - total_done) as f64 / rate;
            title_spans.push(Span::styled(
                format!("  ~{} remaining", fmt_elapsed(remaining_secs)),
                Style::default().fg(Theme::TEXT_GHOST),
            ));
        }
    }
    lines.push(Line::from(title_spans));

    // --- Per-wave lines ---
    let max_waves = (area.height as usize).saturating_sub(1);
    for (idx, wave) in state.execution_waves.iter().take(max_waves).enumerate() {
        let fraction = if wave.total > 0 {
            wave.done as f64 / wave.total as f64
        } else {
            0.0
        };
        let status = wave_status(idx, current_wave, fraction);

        let mut spans: Vec<Span> = Vec::new();

        // Status badge
        let (badge_text, badge_style) = match status {
            WaveStatus::Done => (" DONE ", theme.badge_complete()),
            WaveStatus::Active => (" ACTIVE ", theme.badge_running()),
            WaveStatus::Pending => (" PENDING ", Style::default().fg(Theme::TEXT_GHOST)),
        };
        spans.push(Span::styled(badge_text, badge_style));
        spans.push(Span::styled(" ", Style::default()));

        // Wave label
        spans.push(Span::styled(
            format!("W{}", wave.index),
            theme.label(),
        ));
        spans.push(Span::styled(" ", Style::default()));

        // Task counts: done/total
        let count_color = if wave.done == wave.total && wave.total > 0 {
            Theme::SAGE
        } else if wave.done > 0 {
            Theme::WARNING
        } else {
            Theme::TEXT_DIM
        };
        spans.push(Span::styled(
            format!("{}/{}", wave.done, wave.total),
            Style::default().fg(count_color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(" ", Style::default()));

        // Gradient progress bar — use remaining width
        let prefix_len: usize = spans.iter().map(|s| s.width()).sum();
        // Reserve space for timing suffix
        let timing_text = if status == WaveStatus::Active && elapsed_secs > 0.5 {
            format!(" {}", fmt_elapsed(elapsed_secs))
        } else {
            String::new()
        };
        let bar_width = width
            .saturating_sub(prefix_len)
            .saturating_sub(timing_text.len());

        push_gradient_bar(&mut spans, bar_width, fraction);

        // Timing info for active wave
        if !timing_text.is_empty() {
            spans.push(Span::styled(
                timing_text,
                Style::default().fg(Theme::TEXT_GHOST),
            ));
        }

        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Single-line ribbon fallback (original layout, polished).
#[allow(clippy::too_many_arguments)]
fn render_single_line(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    theme: &Theme,
    total_done: usize,
    total_plans: usize,
    overall_pct: u32,
    current_wave: usize,
    width: usize,
) {
    let summary = format!(" {total_done}/{total_plans} ({overall_pct}%)");
    let summary_width = summary.len();
    let bar_area_width = width.saturating_sub(summary_width);

    let mut spans: Vec<Span> = Vec::new();

    for (idx, wave) in state.execution_waves.iter().enumerate() {
        let wave_width =
            (wave.total as f64 / total_plans as f64 * bar_area_width as f64).ceil() as usize;
        let wave_width = wave_width.max(3);

        let fraction = if wave.total > 0 {
            wave.done as f64 / wave.total as f64
        } else {
            0.0
        };

        let is_current = idx == current_wave;

        // Wave label with completion percentage when space allows
        let pct_val = (fraction * 100.0).round() as u32;
        let label = if wave_width > 10 {
            format!("W{} {pct_val}%", wave.index)
        } else {
            format!("W{}", wave.index)
        };
        let label_len = label.len();

        if wave_width > label_len + 1 {
            spans.push(Span::styled(
                format!("{label} "),
                if is_current {
                    theme.section_header()
                } else {
                    theme.label()
                },
            ));
            let bar_w = wave_width.saturating_sub(label_len + 1);
            push_gradient_bar(&mut spans, bar_w, fraction);
        } else {
            push_gradient_bar(&mut spans, wave_width, fraction);
        }
    }

    // Overall completion summary
    let summary_style = if overall_pct >= 100 {
        Style::default()
            .fg(Theme::SAGE)
            .add_modifier(Modifier::BOLD)
    } else if overall_pct >= 50 {
        Style::default().fg(Theme::BONE_DIM)
    } else {
        Style::default().fg(Theme::TEXT_DIM)
    };
    spans.push(Span::styled(summary, summary_style));

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
