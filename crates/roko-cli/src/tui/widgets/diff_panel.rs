//! Reusable diff viewer widget with syntax coloring, line numbers,
//! file headers, and word-level change highlighting.
//!
//! Used by the F1 Dashboard diff sub-tab and the F4 Git view inline diff.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::dashboard::Theme;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Options for diff rendering.
#[derive(Debug, Clone, Copy)]
pub struct DiffRenderOpts {
    /// Show line numbers in a left gutter.
    pub line_numbers: bool,
    /// Width of the gutter (including separator). Computed from total lines if 0.
    pub gutter_width: u16,
    /// Attempt word-level highlighting on changed lines.
    pub word_highlight: bool,
}

impl Default for DiffRenderOpts {
    fn default() -> Self {
        Self {
            line_numbers: true,
            gutter_width: 0,
            word_highlight: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API: full-widget render (for F1 Dashboard)
// ---------------------------------------------------------------------------

/// Render a diff panel with its own block and border.
///
/// This is the main entry point for callers that want a self-contained
/// bordered widget (e.g. the F1 Dashboard diff sub-tab).
pub fn render_diff_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    diff_text: &str,
    scroll: Option<usize>,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Diff")
        .border_style(Theme::unfocused_border_style())
        .title_style(Theme::title_style())
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if diff_text.is_empty() {
        let empty =
            Paragraph::new("No diff available \u{2014} diffs appear when agents modify files")
                .style(theme.muted())
                .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    }

    render_diff_content(
        frame,
        inner,
        diff_text,
        scroll,
        theme,
        &DiffRenderOpts::default(),
    );
}

// ---------------------------------------------------------------------------
// Public API: content-only render (for embedding in other views)
// ---------------------------------------------------------------------------

/// Render diff content into an already-prepared area (no block/border).
///
/// Callers that manage their own `Block` (e.g. git_view.rs) can use this
/// to get the same rendering quality as `render_diff_panel` without the
/// border overhead.
pub fn render_diff_content(
    frame: &mut Frame<'_>,
    area: Rect,
    diff_text: &str,
    scroll: Option<usize>,
    theme: &Theme,
    opts: &DiffRenderOpts,
) {
    if area.width == 0 || area.height == 0 || diff_text.is_empty() {
        return;
    }

    let max_w = area.width as usize;
    let lines = format_diff_lines(diff_text, max_w, theme, opts);

    let total_lines = lines.len();
    let visible = area.height as usize;

    let scroll_offset = match scroll {
        Some(pos) => pos.min(total_lines.saturating_sub(visible)),
        None => total_lines.saturating_sub(visible),
    };

    let paragraph = Paragraph::new(lines).scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Public API: line formatting (for callers that build their own Paragraph)
// ---------------------------------------------------------------------------

/// Format diff text into styled `Line`s with optional line numbers and
/// word-level highlighting.
///
/// This is the lowest-level public entry point. Use it when you need the
/// formatted lines but want to assemble the `Paragraph` yourself.
pub fn format_diff_lines<'a>(
    diff_text: &str,
    max_width: usize,
    theme: &'a Theme,
    opts: &DiffRenderOpts,
) -> Vec<Line<'a>> {
    let raw_lines: Vec<&str> = diff_text.lines().collect();

    // Compute +/- stats for summary header.
    let (additions, deletions) = count_diff_stats(&raw_lines);

    // Determine gutter width from total line count.
    let gutter_w = if opts.line_numbers {
        if opts.gutter_width > 0 {
            opts.gutter_width as usize
        } else {
            // digits needed + 1 separator char
            digit_count(raw_lines.len()) + 1
        }
    } else {
        0
    };

    let content_w = max_width.saturating_sub(gutter_w);

    // Pre-collect adjacent +/- line pairs for word highlighting.
    let word_pairs = if opts.word_highlight {
        find_word_highlight_pairs(&raw_lines)
    } else {
        Vec::new()
    };

    // Track old/new line numbers for the gutter.
    let mut old_ln: Option<usize> = None;
    let mut new_ln: Option<usize> = None;

    // Build stats summary as the first line.
    let mut result: Vec<Line<'a>> = Vec::with_capacity(raw_lines.len() + 1);
    if additions > 0 || deletions > 0 {
        let mut stats_spans: Vec<Span<'a>> = Vec::new();
        if opts.line_numbers {
            stats_spans.push(Span::styled(" ".repeat(gutter_w), Style::default()));
        }
        stats_spans.push(Span::styled(
            format!("+{additions}"),
            Style::default().fg(theme.success),
        ));
        stats_spans.push(Span::styled(" / ", Style::default().fg(Theme::TEXT_DIM)));
        stats_spans.push(Span::styled(
            format!("-{deletions}"),
            Style::default().fg(theme.danger),
        ));
        let file_count = raw_lines
            .iter()
            .filter(|l| l.starts_with("diff --git"))
            .count();
        if file_count > 0 {
            stats_spans.push(Span::styled(
                format!("  ({file_count} file{})", if file_count == 1 { "" } else { "s" }),
                Style::default().fg(Theme::TEXT_DIM),
            ));
        }
        result.push(Line::from(stats_spans));
    }

    result.extend(raw_lines.iter().enumerate().map(|(i, line)| {
            // Update line counters from hunk headers.
            if line.starts_with("@@") {
                let (o, n) = parse_hunk_header(line);
                old_ln = Some(o);
                new_ln = Some(n);
            }

            let mut spans: Vec<Span<'a>> = Vec::new();

            // Gutter: line numbers.
            if opts.line_numbers {
                let gutter_text = format_gutter(line, &mut old_ln, &mut new_ln, gutter_w);
                spans.push(Span::styled(gutter_text, gutter_style(line, theme)));
            }

            // Content spans.
            if line.starts_with("diff --git") {
                // File header: extract filename with section_header style + separator.
                let filename = extract_filename(line);
                let sep_len = content_w.saturating_sub(filename.len() + 2);
                let separator = "\u{2500}".repeat(sep_len.min(60));
                spans.push(Span::styled(
                    format!(" {filename} "),
                    theme.section_header(),
                ));
                spans.push(Span::styled(
                    separator,
                    Style::default().fg(Theme::TEXT_PHANTOM),
                ));
            } else if line.starts_with("index ") {
                spans.push(Span::styled(
                    truncate_line(line, content_w),
                    Style::default().fg(Theme::TEXT_PHANTOM),
                ));
            } else if opts.word_highlight && is_changed_line(line) {
                // Try word-level highlighting.
                if let Some(partner_idx) = word_pairs.iter().find_map(|&(a, b)| {
                    if a == i {
                        Some(b)
                    } else if b == i {
                        Some(a)
                    } else {
                        None
                    }
                }) {
                    let partner = raw_lines[partner_idx];
                    let word_spans = word_highlight_spans(line, partner, content_w, theme);
                    spans.extend(word_spans);
                } else {
                    spans.push(Span::styled(
                        truncate_line(line, content_w),
                        diff_line_style(line, theme),
                    ));
                }
            } else {
                spans.push(Span::styled(
                    truncate_line(line, content_w),
                    diff_line_style(line, theme),
                ));
            }

            Line::from(spans)
        }));

    result
}

// ---------------------------------------------------------------------------
// Diff line styling (public for reuse)
// ---------------------------------------------------------------------------

/// Return the base style for a diff line based on its prefix.
pub fn diff_line_style(line: &str, theme: &Theme) -> Style {
    if line.starts_with("diff --git") {
        theme.section_header()
    } else if line.starts_with("index ") {
        Style::default().fg(Theme::TEXT_PHANTOM)
    } else if line.starts_with("@@") {
        Style::default()
            .fg(Theme::HUNK)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("+++ ") {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("--- ") {
        Style::default()
            .fg(theme.danger)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with('+') {
        Style::default().fg(theme.success)
    } else if line.starts_with('-') {
        Style::default().fg(theme.danger)
    } else {
        // Context lines rendered dim for visual hierarchy.
        Style::default()
            .fg(Theme::TEXT_DIM)
            .add_modifier(Modifier::DIM)
    }
}

// ---------------------------------------------------------------------------
// Line number gutter
// ---------------------------------------------------------------------------

fn format_gutter(
    line: &str,
    old_ln: &mut Option<usize>,
    new_ln: &mut Option<usize>,
    width: usize,
) -> String {
    if line.starts_with("diff --git")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
        || line.starts_with("@@")
    {
        // Structural lines: no line number, just padding.
        " ".repeat(width)
    } else if line.starts_with('-') {
        // Deletion: show old line number.
        let num = old_ln.map(|n| {
            *old_ln = Some(n + 1);
            n
        });
        format_gutter_number(num, width)
    } else if line.starts_with('+') {
        // Addition: show new line number.
        let num = new_ln.map(|n| {
            *new_ln = Some(n + 1);
            n
        });
        format_gutter_number(num, width)
    } else {
        // Context line: advance both counters.
        let num = new_ln.map(|n| {
            *new_ln = Some(n + 1);
            n
        });
        if let Some(o) = old_ln {
            *old_ln = Some(*o + 1);
        }
        format_gutter_number(num, width)
    }
}

fn format_gutter_number(num: Option<usize>, width: usize) -> String {
    match num {
        Some(n) => {
            let s = n.to_string();
            let pad = width.saturating_sub(s.len() + 1);
            format!("{}{s} ", " ".repeat(pad))
        }
        None => " ".repeat(width),
    }
}

fn gutter_style(line: &str, theme: &Theme) -> Style {
    if line.starts_with('+') {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::DIM)
    } else if line.starts_with('-') {
        Style::default()
            .fg(theme.danger)
            .add_modifier(Modifier::DIM)
    } else {
        Style::default().fg(Theme::TEXT_PHANTOM)
    }
}

// ---------------------------------------------------------------------------
// Hunk header parsing
// ---------------------------------------------------------------------------

/// Parse `@@ -old_start,old_count +new_start,new_count @@` and return
/// `(old_start, new_start)`.
fn parse_hunk_header(line: &str) -> (usize, usize) {
    let mut old_start = 1usize;
    let mut new_start = 1usize;

    if let Some(rest) = line.strip_prefix("@@ -") {
        // Parse old_start
        let old_part = rest.split_whitespace().next().unwrap_or("");
        old_start = old_part
            .split(',')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        // Parse new_start: find the +N part
        if let Some(plus_idx) = rest.find('+') {
            let after_plus = &rest[plus_idx + 1..];
            let new_part = after_plus.split_whitespace().next().unwrap_or("");
            new_start = new_part
                .split(',')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
        }
    }

    (old_start, new_start)
}

// ---------------------------------------------------------------------------
// File header extraction
// ---------------------------------------------------------------------------

/// Extract filename from a `diff --git a/path b/path` line.
fn extract_filename(line: &str) -> String {
    // diff --git a/src/main.rs b/src/main.rs
    if let Some(rest) = line.strip_prefix("diff --git ") {
        // Take the b/ path (destination).
        if let Some(b_part) = rest.split(" b/").nth(1) {
            return b_part.to_string();
        }
        // Fallback: take the a/ path.
        if let Some(a_part) = rest.strip_prefix("a/") {
            if let Some(name) = a_part.split_whitespace().next() {
                return name.to_string();
            }
        }
    }
    line.to_string()
}

// ---------------------------------------------------------------------------
// Word-level highlighting
// ---------------------------------------------------------------------------

fn is_changed_line(line: &str) -> bool {
    (line.starts_with('+') && !line.starts_with("+++ "))
        || (line.starts_with('-') && !line.starts_with("--- "))
}

/// Find pairs of adjacent -/+ lines for word-level diffing.
///
/// Returns `(del_index, add_index)` pairs. Only pairs single -/+ runs
/// (not multi-line deletions followed by multi-line additions).
fn find_word_highlight_pairs(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].starts_with('-') && !lines[i].starts_with("--- ") {
            // Count consecutive deletions.
            let del_start = i;
            while i < lines.len() && lines[i].starts_with('-') && !lines[i].starts_with("--- ") {
                i += 1;
            }
            let del_end = i;
            // Count consecutive additions immediately after.
            let add_start = i;
            while i < lines.len() && lines[i].starts_with('+') && !lines[i].starts_with("+++ ") {
                i += 1;
            }
            let add_end = i;

            let del_count = del_end - del_start;
            let add_count = add_end - add_start;

            // Pair up lines 1:1 where possible.
            let pair_count = del_count.min(add_count);
            for j in 0..pair_count {
                pairs.push((del_start + j, add_start + j));
            }
        } else {
            i += 1;
        }
    }
    pairs
}

/// Produce spans for a changed line with word-level highlighting.
///
/// Compares `line` against `partner` to find which characters differ,
/// then renders the differing characters with a background highlight.
fn word_highlight_spans<'a>(
    line: &str,
    partner: &str,
    max_width: usize,
    theme: &'a Theme,
) -> Vec<Span<'a>> {
    let is_add = line.starts_with('+');
    let base_style = if is_add {
        Style::default().fg(theme.success)
    } else {
        Style::default().fg(theme.danger)
    };
    let highlight_style = if is_add {
        Style::default()
            .fg(theme.success)
            .bg(Theme::BG_HIGHLIGHT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.danger)
            .bg(Theme::BG_HIGHLIGHT)
            .add_modifier(Modifier::BOLD)
    };

    // Strip the leading +/- for comparison.
    let line_content = if line.len() > 1 { &line[1..] } else { "" };
    let partner_content = if partner.len() > 1 {
        &partner[1..]
    } else {
        ""
    };

    // Find common prefix and suffix lengths.
    let prefix_len = line_content
        .chars()
        .zip(partner_content.chars())
        .take_while(|(a, b)| a == b)
        .count();

    let suffix_len = line_content
        .chars()
        .rev()
        .zip(partner_content.chars().rev())
        .take_while(|(a, b)| a == b)
        .count();

    let line_char_count = line_content.chars().count();

    // Prevent overlap between prefix and suffix.
    let suffix_len = suffix_len.min(line_char_count.saturating_sub(prefix_len));

    let change_start = prefix_len;
    let change_end = line_char_count.saturating_sub(suffix_len);

    // If nothing differs or everything differs, just use base style.
    if change_start >= change_end || (change_start == 0 && change_end == line_char_count) {
        return vec![Span::styled(truncate_line(line, max_width), base_style)];
    }

    // Build the leading +/- character.
    let prefix_char: String = line.chars().take(1).collect();

    // Split line_content into three regions.
    let before: String = line_content.chars().take(change_start).collect();
    let changed: String = line_content
        .chars()
        .skip(change_start)
        .take(change_end - change_start)
        .collect();
    let after: String = line_content.chars().skip(change_end).collect();

    // Truncate the total to max_width.
    let total_len = 1 + before.len() + changed.len() + after.len();
    if total_len > max_width && max_width > 1 {
        // Simple truncation: just fall back to whole-line style.
        return vec![Span::styled(truncate_line(line, max_width), base_style)];
    }

    vec![
        Span::styled(prefix_char, base_style),
        Span::styled(before, base_style),
        Span::styled(changed, highlight_style),
        Span::styled(after, base_style),
    ]
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count total additions and deletions in a diff.
fn count_diff_stats(lines: &[&str]) -> (usize, usize) {
    let mut adds = 0usize;
    let mut dels = 0usize;
    for line in lines {
        if line.starts_with('+') && !line.starts_with("+++ ") {
            adds += 1;
        } else if line.starts_with('-') && !line.starts_with("--- ") {
            dels += 1;
        }
    }
    (adds, dels)
}

fn truncate_line(line: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let char_count = line.chars().count();
    if char_count <= max {
        line.to_string()
    } else {
        let truncated: String = line.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    }
}

fn digit_count(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    ((n as f64).log10().floor() as usize) + 1
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_line_style_covers_all_prefixes() {
        let theme = Theme::default();

        let git_header = diff_line_style("diff --git a/f b/f", &theme);
        assert_eq!(git_header.fg, Some(Theme::BONE));

        let index = diff_line_style("index abc..def 100644", &theme);
        assert_eq!(index.fg, Some(Theme::TEXT_PHANTOM));

        let hunk = diff_line_style("@@ -1,3 +1,4 @@", &theme);
        assert_eq!(hunk.fg, Some(Theme::HUNK));

        let add_header = diff_line_style("+++ b/src/main.rs", &theme);
        assert_eq!(add_header.fg, Some(theme.success));

        let del_header = diff_line_style("--- a/src/main.rs", &theme);
        assert_eq!(del_header.fg, Some(theme.danger));

        let add = diff_line_style("+new line", &theme);
        assert_eq!(add.fg, Some(theme.success));

        let del = diff_line_style("-old line", &theme);
        assert_eq!(del.fg, Some(theme.danger));

        let context = diff_line_style(" context", &theme);
        assert_eq!(context.fg, Some(Theme::TEXT_DIM));
    }

    #[test]
    fn parse_hunk_header_standard() {
        assert_eq!(parse_hunk_header("@@ -10,5 +20,7 @@ fn foo()"), (10, 20));
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), (1, 1));
        assert_eq!(parse_hunk_header("@@ -100,0 +101,3 @@"), (100, 101));
    }

    #[test]
    fn extract_filename_from_diff_header() {
        assert_eq!(
            extract_filename("diff --git a/src/main.rs b/src/main.rs"),
            "src/main.rs"
        );
        assert_eq!(
            extract_filename("diff --git a/old/path.rs b/new/path.rs"),
            "new/path.rs"
        );
    }

    #[test]
    fn word_highlight_pairs_finds_adjacent() {
        let lines = vec![
            " context",
            "-old line",
            "+new line",
            " more context",
            "-another old",
            "-second old",
            "+another new",
            "+second new",
        ];
        let pairs = find_word_highlight_pairs(&lines);
        assert_eq!(pairs, vec![(1, 2), (4, 6), (5, 7)]);
    }

    #[test]
    fn word_highlight_pairs_skips_structural_headers() {
        let lines = vec!["--- a/file.rs", "+++ b/file.rs", "-old", "+new"];
        let pairs = find_word_highlight_pairs(&lines);
        assert_eq!(pairs, vec![(2, 3)]);
    }

    #[test]
    fn word_highlight_spans_highlights_changed_chars() {
        let theme = Theme::default();
        let spans = word_highlight_spans("+hello world", "-hello earth", 80, &theme);
        // Should have 4 spans: prefix, common prefix, changed, common suffix.
        assert_eq!(spans.len(), 4);
        // First span is the '+' character.
        assert_eq!(spans[0].content.as_ref(), "+");
        // Second span is the common prefix "hello ".
        assert_eq!(spans[1].content.as_ref(), "hello ");
        // Third span is the changed part "world".
        assert_eq!(spans[2].content.as_ref(), "world");
    }

    #[test]
    fn format_diff_lines_includes_gutter() {
        let diff = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
     // old
-    // removed
";
        let theme = Theme::default();
        let opts = DiffRenderOpts {
            line_numbers: true,
            gutter_width: 4,
            word_highlight: false,
        };
        let lines = format_diff_lines(diff, 80, &theme, &opts);
        // Should have 10 lines (1 stats summary + 9 diff lines).
        assert_eq!(lines.len(), 10);
        // The context line "fn main() {" should have a gutter span with "1".
        // Offset by 1 for the stats summary line.
        let context_line = &lines[6];
        assert!(context_line.spans[0].content.contains('1'));
    }

    #[test]
    fn format_diff_lines_no_gutter() {
        let diff = "+added line\n-removed line\n context\n";
        let theme = Theme::default();
        let opts = DiffRenderOpts {
            line_numbers: false,
            gutter_width: 0,
            word_highlight: false,
        };
        let lines = format_diff_lines(diff, 80, &theme, &opts);
        // 1 stats summary + 3 diff lines.
        assert_eq!(lines.len(), 4);
        // First line is stats summary; second line is the +added content.
        assert!(lines[1].spans[0].content.starts_with('+'));
    }

    #[test]
    fn truncate_line_caps_length() {
        assert_eq!(truncate_line("short", 10), "short");
        assert_eq!(truncate_line("long string here", 6), "long \u{2026}");
    }

    #[test]
    fn digit_count_is_correct() {
        assert_eq!(digit_count(0), 1);
        assert_eq!(digit_count(1), 1);
        assert_eq!(digit_count(9), 1);
        assert_eq!(digit_count(10), 2);
        assert_eq!(digit_count(99), 2);
        assert_eq!(digit_count(100), 3);
        assert_eq!(digit_count(999), 3);
        assert_eq!(digit_count(1000), 4);
    }

    #[test]
    fn extract_filename_handles_renames() {
        assert_eq!(
            extract_filename("diff --git a/old_name.rs b/new_name.rs"),
            "new_name.rs"
        );
    }
}
