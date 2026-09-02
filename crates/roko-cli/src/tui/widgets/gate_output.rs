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
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::Theme;
use crate::tui::state::TuiState;

// ---------------------------------------------------------------------------
// Line classification
// ---------------------------------------------------------------------------

/// Semantic classification of a cargo/gate output line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    /// `Compiling`, `Checking`, `Finished`, `test result: ok`
    Success,
    /// `error[E`, `error:`, `FAILED`, `failures:`
    Error,
    /// `warning:`, `warning[`
    Warning,
    /// `note:` helper lines from rustc
    Note,
    /// `test some::path ... ok/FAILED`, `running N tests`
    TestLine,
    /// File path + line:col location references (`--> src/foo.rs:12:5`)
    Location,
    /// `Downloading`, `Downloaded`, `Blocking`, blank lines
    Dim,
    /// Everything else
    Default,
}

/// Classify a gate output line for color-coding.
fn classify_line(line: &str) -> LineKind {
    let trimmed = line.trim();
    // Success patterns
    if trimmed.contains("test result: ok")
        || trimmed.starts_with("Compiling ")
        || trimmed.starts_with("Finished ")
        || trimmed.starts_with("Checking ")
    {
        return LineKind::Success;
    }
    // Error patterns
    if trimmed.starts_with("error[E")
        || trimmed.starts_with("error:")
        || trimmed.contains("FAILED")
        || trimmed.starts_with("failures:")
    {
        return LineKind::Error;
    }
    // Warning patterns
    if trimmed.starts_with("warning:") || trimmed.starts_with("warning[") {
        return LineKind::Warning;
    }
    // Note/help patterns from rustc
    if trimmed.starts_with("note:") || trimmed.starts_with("help:") {
        return LineKind::Note;
    }
    // Location arrows from rustc
    if trimmed.starts_with("--> ") || trimmed.starts_with("= ") {
        return LineKind::Location;
    }
    // Test-running patterns
    if trimmed.starts_with("running ")
        || trimmed.starts_with("test ")
        || trimmed.starts_with("test result:")
    {
        return LineKind::TestLine;
    }
    // Dim info lines
    if trimmed.starts_with("Downloading")
        || trimmed.starts_with("Downloaded")
        || trimmed.starts_with("Blocking")
        || trimmed.is_empty()
    {
        return LineKind::Dim;
    }
    LineKind::Default
}

/// Convert a `LineKind` classification into a ratatui `Style`.
fn kind_style(kind: LineKind, _theme: &Theme) -> Style {
    match kind {
        LineKind::Success => Style::default().fg(Theme::SAGE),
        LineKind::Error => Style::default()
            .fg(Theme::EMBER)
            .add_modifier(Modifier::BOLD),
        LineKind::Warning => Style::default()
            .fg(Theme::WARNING)
            .add_modifier(Modifier::BOLD),
        LineKind::Note => Style::default().fg(Theme::DREAM_BRIGHT),
        LineKind::Location => Style::default().fg(Theme::TEXT_DIM),
        LineKind::TestLine => Style::default().fg(Theme::DREAM),
        LineKind::Dim => Style::default().fg(Theme::TEXT_GHOST),
        LineKind::Default => Style::default().fg(Theme::TEXT_DIM),
    }
}

/// Build rich spans for a single gate output line.
///
/// For test lines (`test some::path ... ok` / `... FAILED`), the test name
/// is rendered bold and the result is colored green/red. For other line
/// kinds the entire line gets the classified style.
fn style_line<'a>(raw: &str, max_w: usize) -> Vec<Span<'a>> {
    let display = if raw.chars().count() > max_w && max_w > 1 {
        let truncated: String = raw.chars().take(max_w.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    } else {
        raw.to_owned()
    };

    let trimmed = raw.trim();

    // Test result lines: `test foo::bar ... ok` or `test foo::bar ... FAILED`
    if trimmed.starts_with("test ") {
        if let Some(dots_pos) = trimmed.find(" ... ") {
            let name_part = &trimmed[5..dots_pos]; // after "test "
            let result_part = &trimmed[dots_pos + 5..]; // after " ... "
            let result_style = if result_part.contains("FAILED") || result_part.contains("FAIL") {
                Style::default()
                    .fg(Theme::EMBER)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::SAGE)
            };
            let leading_ws: String = raw.chars().take_while(|c| c.is_whitespace()).collect();
            return vec![
                Span::styled(leading_ws, Style::default()),
                Span::styled(
                    "test ".to_owned(),
                    Style::default().fg(Theme::DREAM),
                ),
                Span::styled(
                    name_part.to_owned(),
                    Style::default()
                        .fg(Theme::BONE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ... ".to_owned(), Style::default().fg(Theme::TEXT_GHOST)),
                Span::styled(result_part.to_owned(), result_style),
            ];
        }
    }

    // Warning lines: bold label, normal message
    if trimmed.starts_with("warning:") {
        let msg = &trimmed[8..];
        let leading_ws: String = raw.chars().take_while(|c| c.is_whitespace()).collect();
        return vec![
            Span::styled(leading_ws, Style::default()),
            Span::styled(
                "warning:".to_owned(),
                Style::default()
                    .fg(Theme::WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(msg.to_owned(), Style::default().fg(Theme::WARNING)),
        ];
    }

    // Error lines: bold label, normal message
    if trimmed.starts_with("error") {
        if let Some(colon) = trimmed.find(':') {
            let label = &trimmed[..=colon];
            let msg = &trimmed[colon + 1..];
            let leading_ws: String = raw.chars().take_while(|c| c.is_whitespace()).collect();
            return vec![
                Span::styled(leading_ws, Style::default()),
                Span::styled(
                    label.to_owned(),
                    Style::default()
                        .fg(Theme::EMBER)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    msg.to_owned(),
                    Style::default().fg(Theme::EMBER),
                ),
            ];
        }
    }

    let kind = classify_line(raw);
    let theme = Theme::dark();
    vec![Span::styled(display, kind_style(kind, &theme))]
}

/// Build a rung header line that separates gate stages in the output.
fn rung_header_line(rung_name: &str, width: usize) -> Line<'static> {
    let icon = rung_icon(rung_name);
    let label = format!(" {icon} {rung_name} ");
    let pad_len = width.saturating_sub(label.chars().count()).saturating_sub(2);
    let left_pad = pad_len / 2;
    let right_pad = pad_len - left_pad;
    let bar: String = format!(
        "\u{2500}{}\u{2500}{}{}",
        "\u{2500}".repeat(left_pad),
        label,
        "\u{2500}".repeat(right_pad),
    );
    Line::from(Span::styled(
        bar,
        Style::default()
            .fg(Theme::ROSE_BRIGHT)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Return an icon for each known gate rung name.
fn rung_icon(name: &str) -> &'static str {
    match name {
        n if n.contains("compile") || n.contains("build") || n.contains("check") => "\u{2692}", // hammer and pick
        n if n.contains("lint") || n.contains("clippy") => "\u{26a0}", // warning sign
        n if n.contains("test") => "\u{2713}", // checkmark
        n if n.contains("symbol") => "\u{2234}", // therefore
        n if n.contains("integration") => "\u{2687}", // die face-6
        _ => "\u{25cf}", // filled circle
    }
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
        let elapsed_str = compact_elapsed(elapsed);
        let spinner = atmosphere.spinner();
        let icon = rung_icon(rung_name);
        spans.push(Span::styled(
            format!(" {MIDDLE_DOT} {icon} {rung_name} {elapsed_str} {spinner} "),
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

/// Format elapsed seconds as compact duration: "45s", "3m", "1h05m".
fn compact_elapsed(secs: u64) -> String {
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

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the gate output widget.
///
/// Shows color-coded gate rung output with an animated title when a gate is
/// running. Falls back to an idle placeholder when no output is available.
pub fn render_gate_output(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let mut title_spans = build_title(tui_state.current_gate_rung.as_ref(), &tui_state.atmosphere);
    let is_running = tui_state.current_gate_rung.is_some();
    let border_color = if is_running {
        Theme::WARNING
    } else {
        Theme::TEXT_GHOST
    };

    // Show tail/scroll status inline in the title.
    if !tui_state.gate_output_lines.is_empty() {
        if tui_state.gate_output_scroll == 0 {
            title_spans.push(Span::styled(
                "[TAIL]",
                Style::default()
                    .fg(Theme::SAGE)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            title_spans.push(Span::styled(
                format!("[SCROLL +{}]", tui_state.gate_output_scroll),
                Style::default().fg(Theme::BONE_DIM),
            ));
        }
        title_spans.push(Span::raw(" "));
    }

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
            " Waiting for gate results...".to_string()
        };
        frame.render_widget(Paragraph::new(placeholder).style(theme.muted()), inner);
        return;
    }

    let max_line_w = inner.width as usize;

    // Build styled output with a rung header at the top.
    let mut styled_lines: Vec<Line<'_>> = Vec::with_capacity(lines.len() + 1);

    // Insert a rung header at the top when we know the rung name.
    if let Some((rung_name, _)) = &tui_state.current_gate_rung {
        styled_lines.push(rung_header_line(rung_name, max_line_w));
    }

    for raw in lines.iter() {
        let spans = style_line(raw, max_line_w);
        styled_lines.push(Line::from(spans));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_cargo_output() {
        assert_eq!(classify_line("   Compiling roko-core v0.1.0"), LineKind::Success);
        assert_eq!(classify_line("error[E0433]: failed to resolve"), LineKind::Error);
        assert_eq!(classify_line("error: aborting due to error"), LineKind::Error);
        assert_eq!(classify_line("warning: unused variable"), LineKind::Warning);
        assert_eq!(classify_line("warning[unused_imports]: unused"), LineKind::Warning);
        assert_eq!(classify_line("note: `#[warn(unused)]` on by default"), LineKind::Note);
        assert_eq!(classify_line("help: consider removing"), LineKind::Note);
        assert_eq!(classify_line("  --> src/main.rs:12:5"), LineKind::Location);
        assert_eq!(classify_line("test foo::bar ... ok"), LineKind::TestLine);
        assert_eq!(classify_line("running 42 tests"), LineKind::TestLine);
        assert_eq!(classify_line("test result: ok. 42 passed"), LineKind::Success);
        assert_eq!(classify_line("   Downloading serde"), LineKind::Dim);
        assert_eq!(classify_line(""), LineKind::Dim);
        assert_eq!(classify_line("some random output"), LineKind::Default);
    }

    #[test]
    fn test_line_styling_splits_name_and_result() {
        let spans = style_line("test my_mod::my_test ... ok", 120);
        assert!(spans.len() >= 4, "test lines should be split into multiple spans");
        // The test name span should be bold
        let name_span = &spans[2];
        assert_eq!(name_span.content.as_ref(), "my_mod::my_test");
    }

    #[test]
    fn test_line_failed_gets_ember() {
        let spans = style_line("test failing_test ... FAILED", 120);
        let result_span = spans.last().unwrap();
        assert_eq!(result_span.content.as_ref(), "FAILED");
        // EMBER color for failures
        assert_eq!(
            result_span.style.fg,
            Some(Theme::EMBER),
        );
    }

    #[test]
    fn rung_icons_are_distinct() {
        let names = ["compile", "lint", "test", "symbol", "integration", "other"];
        let icons: Vec<_> = names.iter().map(|n| rung_icon(n)).collect();
        // Most should be distinct (other falls back to generic)
        let mut deduped = icons.clone();
        deduped.sort();
        deduped.dedup();
        assert!(deduped.len() >= 5, "rung icons should be mostly distinct");
    }

    #[test]
    fn compact_elapsed_formats() {
        assert_eq!(compact_elapsed(0), "0s");
        assert_eq!(compact_elapsed(45), "45s");
        assert_eq!(compact_elapsed(90), "1m30s");
        assert_eq!(compact_elapsed(3661), "1h01m");
    }

    #[test]
    fn rung_header_renders() {
        let line = rung_header_line("compile", 40);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("compile"), "header should contain rung name");
    }
}
