//! Gate output widget — shows live gate rung output with color-coded lines.
//!
//! Displays compile/test/clippy output as it streams from gate execution,
//! with an animated spinner and elapsed time in the title when a gate is
//! running. Rung transitions are detected from the output stream and rendered
//! as separator headers. After gate completion, a verdict summary line shows
//! pass/fail status and duration for each rung.

use std::collections::VecDeque;
use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

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
            .bg(Theme::ROSE_EMBER)
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
                    .bg(Theme::ROSE_EMBER)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Theme::SAGE)
                    .add_modifier(Modifier::BOLD)
            };
            let leading_ws: String = raw.chars().take_while(|c| c.is_whitespace()).collect();
            return vec![
                Span::styled(leading_ws, Style::default()),
                Span::styled("test ".to_owned(), Style::default().fg(Theme::DREAM)),
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

    // Error lines: bold label, normal message, subtle red background
    if trimmed.starts_with("error") {
        if let Some(colon) = trimmed.find(':') {
            let label = &trimmed[..=colon];
            let msg = &trimmed[colon + 1..];
            let leading_ws: String = raw.chars().take_while(|c| c.is_whitespace()).collect();
            return vec![
                Span::styled(leading_ws, Style::default().bg(Theme::ROSE_EMBER)),
                Span::styled(
                    label.to_owned(),
                    Style::default()
                        .fg(Theme::EMBER)
                        .bg(Theme::ROSE_EMBER)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    msg.to_owned(),
                    Style::default().fg(Theme::EMBER).bg(Theme::ROSE_EMBER),
                ),
            ];
        }
    }

    let kind = classify_line(raw);
    let theme = Theme::dark();
    vec![Span::styled(display, kind_style(kind, &theme))]
}

/// Map a rung name to its semantic theme color.
fn rung_color(name: &str) -> Color {
    match name {
        n if n.contains("compile") || n.contains("build") || n.contains("check") => Theme::SAGE,
        n if n.contains("test") => Theme::DREAM,
        n if n.contains("lint") || n.contains("clippy") => Theme::WARNING,
        _ => Theme::ROSE_BRIGHT,
    }
}

/// Build a rung header line that separates gate stages in the output.
///
/// Renders as: `═══ ⚙ compile ══════════════════════`
/// with per-rung color (compile=SAGE, test=DREAM, clippy=WARNING).
fn rung_header_line(rung_name: &str, width: usize) -> Line<'static> {
    let icon = rung_icon(rung_name);
    let label = format!(" {icon} {rung_name} ");
    let label_len = label.chars().count();
    // 3 chars for leading "═══", rest for trailing fill
    let trail_len = width.saturating_sub(label_len).saturating_sub(3);
    let bar = format!(
        "\u{2550}\u{2550}\u{2550}{}{}\u{2550}",
        label,
        "\u{2550}".repeat(trail_len.saturating_sub(1)),
    );
    let color = rung_color(rung_name);
    Line::from(Span::styled(
        bar,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}

/// Return an icon for each known gate rung name.
fn rung_icon(name: &str) -> &'static str {
    match name {
        n if n.contains("compile") || n.contains("build") || n.contains("check") => "\u{2692}", // hammer and pick
        n if n.contains("lint") || n.contains("clippy") => "\u{26a0}", // warning sign
        n if n.contains("test") => "\u{2713}",                         // checkmark
        n if n.contains("symbol") => "\u{2234}",                       // therefore
        n if n.contains("integration") => "\u{2687}",                  // die face-6
        _ => "\u{25cf}",                                               // filled circle
    }
}

// ---------------------------------------------------------------------------
// Rung detection from output content
// ---------------------------------------------------------------------------

/// Detect the rung that a line belongs to based on its content.
/// Returns `Some("rung_name")` when the line signals the start of a new rung.
fn detect_rung_transition(line: &str) -> Option<&'static str> {
    let trimmed = line.trim();
    // `Compiling` / `Checking` indicate compile rung
    if trimmed.starts_with("Compiling ") || trimmed.starts_with("Checking ") {
        return Some("compile");
    }
    // `running N tests` indicates test rung
    if trimmed.starts_with("running ") && trimmed.contains("test") {
        return Some("test");
    }
    // Clippy warnings with `clippy::` or `Checking` after compile finished
    if trimmed.contains("clippy::") {
        return Some("clippy");
    }
    None
}

// ---------------------------------------------------------------------------
// Verdict summary
// ---------------------------------------------------------------------------

/// Build a verdict summary line from completed gate results.
///
/// Format: `[check] compile 0.5s | [check] test 12.3s | [x] clippy 2.1s (3 warnings)`
fn verdict_summary_line<'a>(
    summaries: &[super::super::dashboard::GateResultSummary],
    selected_plan: Option<&str>,
    width: usize,
) -> Option<Line<'a>> {
    let relevant: Vec<_> = summaries
        .iter()
        .filter(|s| match selected_plan {
            Some(pid) => s.plan_id == pid,
            None => true,
        })
        .collect();

    if relevant.is_empty() {
        return None;
    }

    let mut spans: Vec<Span<'a>> = Vec::new();
    spans.push(Span::styled(" ".to_owned(), Style::default()));

    for (i, s) in relevant.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " \u{2502} ".to_owned(), // │
                Style::default().fg(Theme::TEXT_GHOST),
            ));
        }
        let (icon, icon_style) = if s.passed {
            ("\u{2713}", Style::default().fg(Theme::SAGE))
        } else {
            (
                "\u{2717}",
                Style::default()
                    .fg(Theme::EMBER)
                    .add_modifier(Modifier::BOLD),
            )
        };
        spans.push(Span::styled(icon.to_owned(), icon_style));
        spans.push(Span::styled(
            format!(" {} ", s.gate_name),
            Style::default().fg(Theme::BONE),
        ));

        let dur_secs = s.duration_ms as f64 / 1000.0;
        let dur_str = if dur_secs >= 60.0 {
            format!("{:.0}m{:.0}s", dur_secs / 60.0, dur_secs % 60.0)
        } else {
            format!("{dur_secs:.1}s")
        };
        spans.push(Span::styled(dur_str, Style::default().fg(Theme::TEXT_DIM)));

        if !s.summary.is_empty() {
            spans.push(Span::styled(
                format!(" ({})", s.summary),
                Style::default().fg(if s.passed {
                    Theme::TEXT_DIM
                } else {
                    Theme::EMBER
                }),
            ));
        }
    }

    // Truncate if too wide
    let total_chars: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if total_chars > width {
        // Just let ratatui handle clipping
    }

    Some(Line::from(spans))
}

// ---------------------------------------------------------------------------
// Title builder
// ---------------------------------------------------------------------------

fn build_title<'a>(
    current_gate_rung: Option<&(String, Instant)>,
    atmosphere: &crate::tui::atmosphere::Atmosphere,
    theme: &Theme,
) -> Vec<Span<'a>> {
    let mut spans = vec![Span::styled(" Gate Output", theme.section_header())];

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
        spans.push(Span::styled(format!(" {MIDDLE_DOT} idle "), theme.label()));
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

/// Number of decimal digits needed to display `n`.
fn digit_count(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    ((n as f64).log10().floor() as usize) + 1
}

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the gate output widget.
///
/// Shows color-coded gate rung output with an animated title when a gate is
/// running. Falls back to an idle placeholder when no output is available.
///
/// When `selected_plan_id` is `Some`, only gate results for that plan are
/// included in the verdict summary. The streaming output lines are always
/// shown (they are already scoped to the active gate run by the snapshot).
pub fn render_gate_output(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    theme: &Theme,
    selected_plan_id: Option<&str>,
) {
    let mut title_spans = build_title(
        tui_state.current_gate_rung.as_ref(),
        &tui_state.atmosphere,
        theme,
    );
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
        // Improved waiting state: show spinner + elapsed time when running.
        let placeholder = if is_running {
            if let Some((_rung, started_at)) = &tui_state.current_gate_rung {
                let elapsed = started_at.elapsed().as_secs();
                let elapsed_str = compact_elapsed(elapsed);
                format!(
                    " {} waiting for output... ({elapsed_str})",
                    tui_state.atmosphere.spinner()
                )
            } else {
                format!(" {} waiting for output...", tui_state.atmosphere.spinner())
            }
        } else if !tui_state.gate_result_summaries.is_empty() {
            // Gate is not running but we have completed results — show verdict.
            let max_w = inner.width as usize;
            if let Some(verdict) =
                verdict_summary_line(&tui_state.gate_result_summaries, selected_plan_id, max_w)
            {
                frame.render_widget(Paragraph::new(vec![verdict]), inner);
                return;
            }
            " Waiting for gate results...".to_string()
        } else {
            " Waiting for gate results...".to_string()
        };
        frame.render_widget(Paragraph::new(placeholder).style(theme.muted()), inner);
        return;
    }

    // Reserve gutter width for line numbers (dim, left margin).
    let line_count = lines.len();
    let gutter_w = if line_count > 0 {
        // Width of the largest line number + 1 space separator.
        digit_count(line_count) + 1
    } else {
        0
    };
    let content_w = (inner.width as usize).saturating_sub(gutter_w);

    // Build styled output with rung headers inserted at transition points.
    let mut styled_lines: Vec<Line<'_>> = Vec::with_capacity(lines.len() + 4);

    // Track the current detected rung to insert headers on transition.
    let mut current_detected_rung: Option<&'static str> = None;

    // If we know the active rung from the state and there are lines but no
    // rung transition was detected yet, insert a header at the top.
    let mut inserted_initial_header = false;
    if let Some((rung_name, _)) = &tui_state.current_gate_rung {
        styled_lines.push(rung_header_line(rung_name, inner.width as usize));
        inserted_initial_header = true;
        // Map the state rung name to our detection categories to avoid
        // a duplicate header when the first line also triggers detection.
        current_detected_rung = detect_rung_transition(rung_name);
    }

    let num_width = digit_count(line_count);
    for (idx, raw) in lines.iter().enumerate() {
        // Check for rung transition and insert a separator header.
        if let Some(new_rung) = detect_rung_transition(raw) {
            if current_detected_rung != Some(new_rung) {
                // Don't insert a duplicate if we already put the initial header
                // and this is the same rung.
                let skip = inserted_initial_header && styled_lines.len() == 1;
                if !skip {
                    styled_lines.push(rung_header_line(new_rung, inner.width as usize));
                }
                current_detected_rung = Some(new_rung);
            }
        }

        // Prepend dim line number.
        let mut spans = vec![Span::styled(
            format!("{:>width$} ", idx + 1, width = num_width),
            Style::default().fg(Theme::TEXT_PHANTOM),
        )];
        spans.extend(style_line(raw, content_w));
        styled_lines.push(Line::from(spans));
    }

    // Append verdict summary after the output when the gate has completed.
    if !is_running && !tui_state.gate_result_summaries.is_empty() {
        if let Some(verdict) = verdict_summary_line(
            &tui_state.gate_result_summaries,
            selected_plan_id,
            inner.width as usize,
        ) {
            styled_lines.push(Line::default()); // blank separator
            styled_lines.push(verdict);
        }
    }

    let total = styled_lines.len();
    let visible = inner.height as usize;

    // Auto-scroll to bottom (follow tail), clamped by gate_output_scroll.
    let scroll_offset = if tui_state.gate_output_scroll == 0 {
        // Auto-tail: show the latest lines.
        total.saturating_sub(visible) as u16
    } else {
        let max_scroll = total.saturating_sub(visible);
        max_scroll.saturating_sub(tui_state.gate_output_scroll) as u16
    };

    frame.render_widget(
        Paragraph::new(styled_lines)
            .style(Style::default().fg(Theme::TEXT_DIM))
            .scroll((scroll_offset, 0)),
        inner,
    );

    // Scrollbar when content exceeds viewport.
    if total > visible && visible > 0 {
        let mut sb_state = ScrollbarState::new(total).position(scroll_offset as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(Theme::ROSE))
            .track_style(Style::default().fg(Theme::TEXT_PHANTOM))
            .begin_symbol(Some("\u{25b2}"))
            .end_symbol(Some("\u{25bc}"));
        frame.render_stateful_widget(scrollbar, inner, &mut sb_state);
    }
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
        assert_eq!(
            classify_line("   Compiling roko-core v0.1.0"),
            LineKind::Success
        );
        assert_eq!(
            classify_line("error[E0433]: failed to resolve"),
            LineKind::Error
        );
        assert_eq!(
            classify_line("error: aborting due to error"),
            LineKind::Error
        );
        assert_eq!(classify_line("warning: unused variable"), LineKind::Warning);
        assert_eq!(
            classify_line("warning[unused_imports]: unused"),
            LineKind::Warning
        );
        assert_eq!(
            classify_line("note: `#[warn(unused)]` on by default"),
            LineKind::Note
        );
        assert_eq!(classify_line("help: consider removing"), LineKind::Note);
        assert_eq!(classify_line("  --> src/main.rs:12:5"), LineKind::Location);
        assert_eq!(classify_line("test foo::bar ... ok"), LineKind::TestLine);
        assert_eq!(classify_line("running 42 tests"), LineKind::TestLine);
        assert_eq!(
            classify_line("test result: ok. 42 passed"),
            LineKind::Success
        );
        assert_eq!(classify_line("   Downloading serde"), LineKind::Dim);
        assert_eq!(classify_line(""), LineKind::Dim);
        assert_eq!(classify_line("some random output"), LineKind::Default);
    }

    #[test]
    fn test_line_styling_splits_name_and_result() {
        let spans = style_line("test my_mod::my_test ... ok", 120);
        assert!(
            spans.len() >= 4,
            "test lines should be split into multiple spans"
        );
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
        assert_eq!(result_span.style.fg, Some(Theme::EMBER),);
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
        // Should use double-line box chars
        assert!(
            text.contains('\u{2550}'),
            "header should use double-line border"
        );
    }

    #[test]
    fn rung_color_maps_correctly() {
        assert_eq!(rung_color("compile"), Theme::SAGE);
        assert_eq!(rung_color("build"), Theme::SAGE);
        assert_eq!(rung_color("test"), Theme::DREAM);
        assert_eq!(rung_color("clippy"), Theme::WARNING);
        assert_eq!(rung_color("lint"), Theme::WARNING);
        assert_eq!(rung_color("other"), Theme::ROSE_BRIGHT);
    }

    #[test]
    fn digit_count_values() {
        assert_eq!(digit_count(0), 1);
        assert_eq!(digit_count(1), 1);
        assert_eq!(digit_count(9), 1);
        assert_eq!(digit_count(10), 2);
        assert_eq!(digit_count(99), 2);
        assert_eq!(digit_count(100), 3);
        assert_eq!(digit_count(1000), 4);
    }

    #[test]
    fn detect_rung_transition_compile() {
        assert_eq!(
            detect_rung_transition("   Compiling roko-core v0.1.0"),
            Some("compile")
        );
        assert_eq!(
            detect_rung_transition("   Checking roko-core"),
            Some("compile")
        );
    }

    #[test]
    fn detect_rung_transition_test() {
        assert_eq!(detect_rung_transition("running 42 tests"), Some("test"));
        assert_eq!(detect_rung_transition("running 1 test"), Some("test"));
    }

    #[test]
    fn detect_rung_transition_clippy() {
        assert_eq!(
            detect_rung_transition("warning: clippy::needless_return"),
            Some("clippy")
        );
    }

    #[test]
    fn detect_rung_transition_none() {
        assert_eq!(detect_rung_transition("some random output"), None);
        assert_eq!(detect_rung_transition("error: something"), None);
    }

    #[test]
    fn verdict_summary_with_results() {
        use crate::tui::dashboard::GateResultSummary;
        let summaries = vec![
            GateResultSummary {
                plan_id: "plan-a".into(),
                gate_name: "compile".into(),
                passed: true,
                rung: 1,
                duration_ms: 500,
                summary: String::new(),
            },
            GateResultSummary {
                plan_id: "plan-a".into(),
                gate_name: "test".into(),
                passed: false,
                rung: 2,
                duration_ms: 12300,
                summary: "3 failures".into(),
            },
        ];
        let line = verdict_summary_line(&summaries, None, 120);
        assert!(line.is_some());
        let line = line.unwrap();
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("compile"), "should contain compile rung");
        assert!(text.contains("test"), "should contain test rung");
        assert!(
            text.contains("3 failures"),
            "should contain failure summary"
        );
    }

    #[test]
    fn verdict_summary_filters_by_plan() {
        use crate::tui::dashboard::GateResultSummary;
        let summaries = vec![
            GateResultSummary {
                plan_id: "plan-a".into(),
                gate_name: "compile".into(),
                passed: true,
                rung: 1,
                duration_ms: 500,
                summary: String::new(),
            },
            GateResultSummary {
                plan_id: "plan-b".into(),
                gate_name: "test".into(),
                passed: true,
                rung: 2,
                duration_ms: 1000,
                summary: String::new(),
            },
        ];
        let line = verdict_summary_line(&summaries, Some("plan-a"), 120);
        assert!(line.is_some());
        let text: String = line
            .unwrap()
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("compile"), "should contain plan-a's gate");
        assert!(!text.contains("test"), "should not contain plan-b's gate");
    }

    #[test]
    fn verdict_summary_empty_returns_none() {
        let line = verdict_summary_line(&[], None, 120);
        assert!(line.is_none());
    }
}
