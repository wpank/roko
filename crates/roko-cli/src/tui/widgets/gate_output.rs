//! Gate output widget — shows live gate rung output with color-coded lines.
//!
//! Displays compile/test/clippy output as it streams from gate execution,
//! with an animated spinner and elapsed time in the title when a gate is
//! running.

use std::collections::VecDeque;
use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::Theme;
use crate::tui::state::TuiState;

// ---------------------------------------------------------------------------
// Line classification
// ---------------------------------------------------------------------------

/// Classify a gate output line for color-coding.
fn line_style(line: &str, theme: &Theme) -> Style {
    let trimmed = line.trim();
    // Success patterns
    if trimmed.contains("test result: ok")
        || trimmed.starts_with("Compiling ")
        || trimmed.starts_with("Finished ")
        || trimmed.starts_with("Checking ")
    {
        return Style::default().fg(Theme::SAGE);
    }
    // Error patterns
    if trimmed.starts_with("error[E")
        || trimmed.starts_with("error:")
        || trimmed.contains("FAILED")
        || trimmed.starts_with("failures:")
    {
        return Style::default()
            .fg(Theme::EMBER)
            .add_modifier(Modifier::BOLD);
    }
    // Warning patterns
    if trimmed.starts_with("warning:") || trimmed.starts_with("warning[") {
        return Style::default().fg(Theme::WARNING);
    }
    // Test-running patterns
    if trimmed.starts_with("running ") || trimmed.starts_with("test ") {
        return Style::default().fg(Theme::DREAM);
    }
    // Dim info lines
    if trimmed.starts_with("Downloading")
        || trimmed.starts_with("Downloaded")
        || trimmed.starts_with("Blocking")
        || trimmed.is_empty()
    {
        return theme.muted();
    }
    // Default: normal text
    Style::default().fg(Theme::TEXT_DIM)
}

// ---------------------------------------------------------------------------
// Title builder
// ---------------------------------------------------------------------------

fn build_title<'a>(
    current_gate_rung: Option<&(String, Instant)>,
    atmosphere: &crate::tui::atmosphere::Atmosphere,
) -> Vec<Span<'a>> {
    let mut spans = vec![Span::styled(
        " Gate Output",
        Style::default()
            .fg(Theme::ROSE_BRIGHT)
            .add_modifier(Modifier::BOLD),
    )];

    if let Some((rung_name, started_at)) = current_gate_rung {
        let elapsed = started_at.elapsed().as_secs();
        let spinner = atmosphere.spinner();
        spans.push(Span::styled(
            format!(" {MIDDLE_DOT} {rung_name} {elapsed}s {spinner} "),
            Style::default().fg(Theme::WARNING),
        ));
    } else {
        spans.push(Span::styled(
            format!(" {MIDDLE_DOT} idle "),
            Style::default().fg(Theme::TEXT_GHOST),
        ));
    }

    spans
}

const MIDDLE_DOT: char = '\u{00b7}';

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the gate output widget.
///
/// Shows color-coded gate rung output with an animated title when a gate is
/// running. Falls back to an idle placeholder when no output is available.
pub fn render_gate_output(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let title_spans = build_title(tui_state.current_gate_rung.as_ref(), &tui_state.atmosphere);
    let is_running = tui_state.current_gate_rung.is_some();
    let border_color = if is_running {
        Theme::WARNING
    } else {
        Theme::TEXT_GHOST
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(title_spans))
        .border_style(Style::default().fg(border_color))
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height < 1 {
        return;
    }

    let lines = &tui_state.gate_output_lines;
    if lines.is_empty() {
        let placeholder = if is_running {
            format!(" {} waiting for output...", tui_state.atmosphere.spinner())
        } else {
            " no gate output".to_string()
        };
        frame.render_widget(Paragraph::new(placeholder).style(theme.muted()), inner);
        return;
    }

    let styled_lines: Vec<Line<'_>> = lines
        .iter()
        .map(|raw| Line::from(Span::styled(raw.clone(), line_style(raw, theme))))
        .collect();

    let total = styled_lines.len();
    let visible = inner.height as usize;

    // Auto-scroll to bottom (follow tail), clamped by gate_output_scroll.
    let scroll = if tui_state.gate_output_scroll == 0 {
        // Auto-tail: show the latest lines.
        total.saturating_sub(visible) as u16
    } else {
        let max_scroll = total.saturating_sub(visible);
        max_scroll.saturating_sub(tui_state.gate_output_scroll) as u16
    };

    frame.render_widget(
        Paragraph::new(styled_lines)
            .style(Style::default().fg(Theme::TEXT_DIM))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        inner,
    );
}

// ---------------------------------------------------------------------------
// Convenience: should the gate output widget be shown?
// ---------------------------------------------------------------------------

/// Returns `true` when the gate output widget should replace the normal
/// agent output panel — i.e. when a gate is running or there is recent
/// gate output to display.
#[must_use]
pub fn should_show(
    current_gate_rung: &Option<(String, Instant)>,
    gate_output_lines: &VecDeque<String>,
) -> bool {
    current_gate_rung.is_some() || !gate_output_lines.is_empty()
}
