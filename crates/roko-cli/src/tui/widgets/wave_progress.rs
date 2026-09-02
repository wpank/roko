//! Wave progress ribbon — proportional segments per execution wave with
//! animated ocean gradient fill, per-wave percentage annotations, and
//! overall completion summary.
//!
//! Ported from Mori's wave_progress.rs.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::super::state::TuiState;
use super::rosedust::gradient_ocean;
use crate::tui::Theme;

// ---------------------------------------------------------------------------
// Public render
// ---------------------------------------------------------------------------

/// Render the wave progress ribbon.
pub fn render_wave_progress(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    if state.execution_waves.is_empty() {
        let placeholder = Line::from(Span::styled(
            "No execution waves",
            Style::default().fg(Theme::TEXT_GHOST),
        ));
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

    // Overall completion summary (right-aligned)
    let total_done: usize = state.execution_waves.iter().map(|w| w.done).sum();
    let overall_pct = if total_plans > 0 {
        (total_done as f64 / total_plans as f64 * 100.0).round() as u32
    } else {
        0
    };
    let summary = format!(" {total_done}/{total_plans} ({overall_pct}%)");
    let summary_width = summary.len();
    let bar_area_width = width.saturating_sub(summary_width);

    let gradient = gradient_ocean();
    let current_wave = state.current_wave();
    let elapsed = state.atmosphere.elapsed();
    let mut spans: Vec<Span> = Vec::new();

    for (idx, wave) in state.execution_waves.iter().enumerate() {
        let wave_width =
            (wave.total as f64 / total_plans as f64 * bar_area_width as f64).ceil() as usize;
        let wave_width = wave_width.max(3); // minimum 3 chars per wave

        let fraction = if wave.total > 0 {
            wave.done as f64 / wave.total as f64
        } else {
            0.0
        };

        let is_current = idx == current_wave;
        let bar_color = if fraction >= 1.0 {
            Theme::SAGE
        } else if is_current {
            gradient.sample(0.7)
        } else {
            Theme::TEXT_GHOST
        };

        let filled = (fraction * wave_width as f64) as usize;

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
                Style::default().fg(if is_current {
                    Theme::BONE
                } else {
                    Theme::FG_DIM
                }),
            ));
            let bar_w = wave_width.saturating_sub(label_len + 1);
            let filled_bar = filled.min(bar_w);
            let empty_bar = bar_w.saturating_sub(filled_bar);

            if is_current && filled_bar > 0 {
                // Per-cell ocean gradient with animated offset
                for j in 0..filled_bar {
                    let t = (j as f64 / bar_w.max(1) as f64 + elapsed * 0.1) % 1.0;
                    let c = gradient.sample(t);
                    spans.push(Span::styled("\u{2588}", Style::default().fg(c)));
                }
            } else {
                spans.push(Span::styled(
                    "\u{2588}".repeat(filled_bar),
                    Style::default().fg(bar_color),
                ));
            }
            spans.push(Span::styled(
                "\u{2500}".repeat(empty_bar),
                Style::default().fg(Theme::TEXT_GHOST),
            ));
        } else {
            let empty = wave_width.saturating_sub(filled);
            if is_current && filled > 0 {
                for j in 0..filled {
                    let t = (j as f64 / wave_width.max(1) as f64 + elapsed * 0.1) % 1.0;
                    let c = gradient.sample(t);
                    spans.push(Span::styled("\u{2588}", Style::default().fg(c)));
                }
            } else {
                spans.push(Span::styled(
                    "\u{2588}".repeat(filled),
                    Style::default().fg(bar_color),
                ));
            }
            spans.push(Span::styled(
                "\u{2500}".repeat(empty),
                Style::default().fg(Theme::TEXT_GHOST),
            ));
        }
    }

    // Append overall completion summary
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
