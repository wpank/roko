//! Semantic parsing and cached styling for agent output.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::ansi::parse_ansi_line;
use super::dashboard::Theme;

/// Semantic segment types for agent output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    /// Natural language reasoning.
    Thinking,
    /// Markdown headers.
    Heading,
    /// Tool invocations and shell commands.
    ToolUse,
    /// Code blocks and indented code snippets.
    Code,
    /// Positive outcomes.
    Success,
    /// Failures and compiler errors.
    Error,
    /// Empty lines.
    Blank,
    /// Turn boundary markers.
    TurnMarker,
}

impl SegmentKind {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Thinking => "thinking",
            Self::Heading => "heading",
            Self::ToolUse => "tool use",
            Self::Code => "code",
            Self::Success => "success",
            Self::Error => "error",
            Self::Blank => "blank",
            Self::TurnMarker => "turn",
        }
    }

    /// Icon prefix character for the segment kind.
    const fn icon(self) -> &'static str {
        match self {
            Self::Thinking => "\u{2502} ", // |
            Self::Heading => "",
            Self::ToolUse => "\u{25b8} ", // >
            Self::Code => "\u{2502} ",    // |
            Self::Success => "\u{2713} ", // check
            Self::Error => "\u{2717} ",   // x
            Self::Blank => "",
            Self::TurnMarker => "",
        }
    }
}

/// Consecutive lines that share the same semantic kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentGroup {
    /// Shared kind for all lines in the group.
    pub kind: SegmentKind,
    /// Raw lines in display order.
    pub lines: Vec<String>,
}

/// Cached parse and render result for an agent output buffer.
#[derive(Debug, Clone, Default)]
pub struct CachedRender {
    /// Last rendered byte length of the raw output.
    pub last_len: usize,
    /// Parsed semantic groups.
    pub groups: Vec<SegmentGroup>,
    /// Styled lines ready for ratatui rendering.
    pub styled_lines: Vec<Line<'static>>,
}

pub(crate) fn output_byte_len(lines: &[String]) -> usize {
    lines.iter().map(String::len).sum::<usize>() + lines.len().saturating_sub(1)
}

pub(crate) fn render_cached_output(lines: &[String], theme: &Theme) -> CachedRender {
    let groups = parse_segment_groups(lines);
    let styled_lines = render_groups(&groups, theme);
    CachedRender {
        last_len: output_byte_len(lines),
        groups,
        styled_lines,
    }
}

fn parse_segment_groups(lines: &[String]) -> Vec<SegmentGroup> {
    let mut groups = Vec::new();
    let mut state = ParseState::default();

    for raw_line in preprocess_lines(lines) {
        let plain = strip_ansi(&raw_line);
        let trimmed = plain.trim();
        let is_fence = plain.trim_start().starts_with("```");
        let was_in_indented_code = state.in_indented_code;
        let kind = classify_line(&plain, trimmed, is_fence, &state);

        push_group(&mut groups, kind, raw_line);

        if is_fence {
            state.in_fenced_code = !state.in_fenced_code;
            state.in_indented_code = false;
            state.prev_was_blank = false;
        } else if state.in_fenced_code {
            state.prev_was_blank = false;
        } else if kind == SegmentKind::Blank {
            state.prev_was_blank = true;
            state.in_indented_code = false;
        } else {
            state.prev_was_blank = false;
            if kind == SegmentKind::Code {
                state.in_indented_code = was_in_indented_code || leading_spaces(&plain) >= 4;
            } else {
                state.in_indented_code = false;
            }
        }
    }

    groups
}

fn preprocess_lines(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();

    for line in lines {
        let expanded = insert_prompt_breaks(line);
        if expanded.is_empty() {
            out.push(String::new());
            continue;
        }

        let mut saw_chunk = false;
        for chunk in expanded.lines() {
            saw_chunk = true;
            if chunk.len() > 120 && !chunk.trim_start().starts_with("```") {
                out.extend(split_on_sentences(chunk));
            } else {
                out.push(chunk.to_string());
            }
        }
        if !saw_chunk {
            out.push(String::new());
        }
    }

    out
}

fn insert_prompt_breaks(line: &str) -> String {
    let mut expanded = line.to_string();
    for marker in ["Now ", "Let ", "I'll ", "I need ", "Next ", "First "] {
        expanded = expanded.replace(&format!(" {marker}"), &format!("\n{marker}"));
        expanded = expanded.replace(&format!(":{marker}"), &format!(":\n{marker}"));
    }
    expanded
}

fn split_on_sentences(line: &str) -> Vec<String> {
    if line.len() <= 120 {
        return vec![line.to_string()];
    }

    let split_at = line
        .match_indices(". ")
        .map(|(idx, _)| idx + 1)
        .filter(|idx| *idx <= 120)
        .last()
        .or_else(|| line[..120].rmatch_indices(' ').map(|(idx, _)| idx).next())
        .unwrap_or(120);

    let (head, tail) = line.split_at(split_at);
    let mut lines = vec![head.trim_end().to_string()];
    let tail = tail.trim_start();
    if !tail.is_empty() {
        lines.extend(split_on_sentences(tail));
    }
    lines
}

fn strip_ansi(line: &str) -> String {
    parse_ansi_line(line)
        .into_iter()
        .map(|span| span.content.into_owned())
        .collect()
}

fn is_heading(trimmed: &str) -> bool {
    trimmed.starts_with("# ") || trimmed.starts_with("## ") || trimmed.starts_with("### ")
}

fn is_tool_use(trimmed: &str) -> bool {
    [
        "\u{25b8} ",
        "> ",
        "$ ",
        "Running ",
        "Reading ",
        "Writing ",
        "Editing ",
        "Created ",
        "Searching ",
        "Executing ",
        "Deleting ",
        "Moving ",
        "Copying ",
        "Installing ",
        "Fetching ",
        "Calling ",
        "Updating ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

fn is_success(trimmed: &str) -> bool {
    trimmed.contains('✓')
        || trimmed.contains("PASS")
        || trimmed.contains("APPROVE")
        || trimmed.starts_with("ok ")
}

fn is_error(trimmed: &str) -> bool {
    trimmed.contains("ERROR")
        || trimmed.contains("FAILED")
        || trimmed.contains("REVISE")
        || trimmed.contains("error[")
        || trimmed.starts_with("error")
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == ' ').count()
}

#[derive(Debug, Default, Clone, Copy)]
struct ParseState {
    in_fenced_code: bool,
    in_indented_code: bool,
    prev_was_blank: bool,
}

fn classify_line(line: &str, trimmed: &str, is_fence: bool, state: &ParseState) -> SegmentKind {
    let leading_spaces = leading_spaces(line);

    if state.in_fenced_code {
        return SegmentKind::Code;
    }

    if state.in_indented_code && (trimmed.is_empty() || leading_spaces >= 4) {
        return SegmentKind::Code;
    }

    if trimmed.starts_with("────") {
        SegmentKind::TurnMarker
    } else if trimmed.is_empty() {
        SegmentKind::Blank
    } else if is_fence {
        SegmentKind::Code
    } else if state.prev_was_blank && leading_spaces >= 4 {
        SegmentKind::Code
    } else if is_heading(trimmed) {
        SegmentKind::Heading
    } else if is_tool_use(trimmed) {
        SegmentKind::ToolUse
    } else if is_success(trimmed) {
        SegmentKind::Success
    } else if is_error(trimmed) {
        SegmentKind::Error
    } else {
        SegmentKind::Thinking
    }
}

fn push_group(groups: &mut Vec<SegmentGroup>, kind: SegmentKind, line: String) {
    let should_merge = groups.last().is_some_and(|group| group.kind == kind);
    if should_merge {
        if let Some(group) = groups.last_mut() {
            group.lines.push(line);
        }
        return;
    }

    groups.push(SegmentGroup {
        kind,
        lines: vec![line],
    });
}

/// Maximum lines to show inline for a tool-use or code group before folding.
const FOLD_THRESHOLD: usize = 6;
/// Number of visible lines shown when a group is folded.
const FOLD_PREVIEW_LINES: usize = 3;

fn render_groups(groups: &[SegmentGroup], theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut prev_kind: Option<SegmentKind> = None;
    let mut turn_index: usize = 0;

    for group in groups {
        // Insert a visual separator when transitioning between different
        // non-blank segment kinds. This gives clear boundaries between
        // tool calls, reasoning, code, and results.
        if let Some(prev) = prev_kind {
            let needs_separator = !matches!(group.kind, SegmentKind::Blank)
                && !matches!(prev, SegmentKind::Blank)
                && prev != group.kind;
            if needs_separator {
                lines.push(Line::default());
            }
        }

        if matches!(group.kind, SegmentKind::TurnMarker) {
            turn_index += 1;
            lines.push(render_turn_boundary(turn_index, &group.lines, theme));
            prev_kind = Some(group.kind);
            continue;
        }

        lines.push(render_group_header(group.kind, group.lines.len(), theme));
        if matches!(group.kind, SegmentKind::Blank) {
            lines.extend(std::iter::repeat_with(Line::default).take(group.lines.len()));
            prev_kind = Some(group.kind);
            continue;
        }
        let icon = group.kind.icon();
        let style = content_style(group.kind, theme);

        // Fold long tool-use groups: show first N lines + "[+M lines]" summary.
        let should_fold =
            matches!(group.kind, SegmentKind::ToolUse) && group.lines.len() > FOLD_THRESHOLD;

        if should_fold {
            for line in group.lines.iter().take(FOLD_PREVIEW_LINES) {
                lines.push(render_content_line(line, style, icon));
            }
            let hidden = group.lines.len() - FOLD_PREVIEW_LINES;
            lines.push(Line::from(vec![
                Span::styled(" ", style),
                Span::styled(
                    format!("  \u{2026} [+{hidden} lines]"),
                    Style::default()
                        .fg(Theme::TEXT_GHOST)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        } else if matches!(group.kind, SegmentKind::Code) {
            // Render code blocks with a left border and detect JSON.
            render_code_block(&group.lines, theme, &mut lines);
        } else {
            for line in &group.lines {
                lines.push(render_content_line(line, style, icon));
            }
        }
        prev_kind = Some(group.kind);
    }

    lines
}

/// Render a turn boundary with a turn number and decorative separator.
fn render_turn_boundary(turn_index: usize, raw_lines: &[String], theme: &Theme) -> Line<'static> {
    // Extract any timestamp-like text from the raw turn marker line.
    let extra = raw_lines
        .first()
        .and_then(|line| {
            let plain = strip_ansi(line);
            let trimmed = plain
                .trim()
                .trim_start_matches('\u{2500}')
                .trim_start_matches('-')
                .trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .unwrap_or_default();

    let turn_label = format!(" turn {turn_index} ");
    let extra_part = if extra.is_empty() {
        String::new()
    } else {
        format!(" {extra} ")
    };
    let sep = "\u{2500}".repeat(6);

    Line::from(vec![
        Span::styled(
            format!("{sep}\u{253c}"),
            Style::default().fg(Theme::TEXT_PHANTOM),
        ),
        Span::styled(
            turn_label,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(extra_part, Style::default().fg(Theme::TEXT_GHOST)),
        Span::styled(
            format!("\u{253c}{sep}"),
            Style::default().fg(Theme::TEXT_PHANTOM),
        ),
    ])
}

/// Render a code block with a left-margin border and optional JSON key highlighting.
fn render_code_block(code_lines: &[String], theme: &Theme, out: &mut Vec<Line<'static>>) {
    let code_style = Style::default().fg(theme.success).bg(Theme::BG_SECONDARY);
    let border_style = Style::default().fg(Theme::TEXT_PHANTOM);
    let fence_style = Style::default()
        .fg(Theme::TEXT_GHOST)
        .bg(Theme::BG_SECONDARY);

    // Detect if this is a JSON block (first non-fence, non-blank line starts with { or [).
    let is_json = code_lines.iter().any(|line| {
        let trimmed = strip_ansi(line).trim().to_string();
        !trimmed.is_empty()
            && !trimmed.starts_with("```")
            && (trimmed.starts_with('{') || trimmed.starts_with('[') || trimmed.starts_with("\""))
    });

    for line in code_lines {
        let plain = strip_ansi(line);
        let trimmed = plain.trim();

        // Fence lines (``` markers) rendered dimmer.
        if trimmed.starts_with("```") {
            out.push(Line::from(vec![
                Span::styled("\u{2502} ", border_style),
                Span::styled(line.to_owned(), fence_style),
            ]));
            continue;
        }

        if is_json {
            out.push(render_json_line(line, &plain, border_style, theme));
        } else {
            let mut spans = Vec::new();
            spans.push(Span::styled("\u{2502} ", border_style));
            for span in parse_ansi_line(line) {
                spans.push(Span::styled(
                    span.content.into_owned(),
                    code_style.patch(span.style),
                ));
            }
            out.push(Line::from(spans));
        }
    }
}

/// Render a single JSON line with key highlighting.
fn render_json_line(raw: &str, plain: &str, border_style: Style, theme: &Theme) -> Line<'static> {
    let key_style = Style::default()
        .fg(Theme::BONE)
        .bg(Theme::BG_SECONDARY)
        .add_modifier(Modifier::BOLD);
    let string_style = Style::default().fg(Theme::SAGE).bg(Theme::BG_SECONDARY);
    let punct_style = Style::default().fg(Theme::TEXT_DIM).bg(Theme::BG_SECONDARY);
    let number_style = Style::default().fg(Theme::WARNING).bg(Theme::BG_SECONDARY);
    let bool_style = Style::default()
        .fg(Theme::DREAM_BRIGHT)
        .bg(Theme::BG_SECONDARY);
    let default_style = Style::default().fg(theme.success).bg(Theme::BG_SECONDARY);

    let mut spans = vec![Span::styled("\u{2502} ", border_style)];

    // Simple character-level JSON coloring on the plain text.
    let chars: Vec<char> = plain.chars().collect();
    let mut i = 0;
    let mut in_key = false;

    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '"' => {
                // Find closing quote.
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1; // skip escaped char
                    }
                    i += 1;
                }
                i += 1; // past closing quote
                let s: String = chars[start..i.min(chars.len())].iter().collect();
                // Check if this is a key (followed by ':' after optional whitespace).
                let rest: String = chars[i.min(chars.len())..].iter().collect();
                let is_key = rest.trim_start().starts_with(':');
                if is_key {
                    spans.push(Span::styled(s, key_style));
                    in_key = true;
                } else {
                    spans.push(Span::styled(s, string_style));
                    in_key = false;
                }
            }
            ':' if in_key => {
                spans.push(Span::styled(":", punct_style));
                in_key = false;
                i += 1;
            }
            '{' | '}' | '[' | ']' | ',' => {
                spans.push(Span::styled(ch.to_string(), punct_style));
                i += 1;
            }
            c if c.is_ascii_digit() || c == '-' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == '-'
                        || chars[i] == 'e'
                        || chars[i] == 'E')
                {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                spans.push(Span::styled(s, number_style));
            }
            _ => {
                // Collect a run of non-special characters.
                let start = i;
                while i < chars.len()
                    && !matches!(chars[i], '"' | '{' | '}' | '[' | ']' | ',' | ':')
                    && !(chars[i].is_ascii_digit() && i > start)
                {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let trimmed_s = s.trim();
                let style = match trimmed_s {
                    "true" | "false" => bool_style,
                    "null" => Style::default()
                        .fg(Theme::TEXT_GHOST)
                        .bg(Theme::BG_SECONDARY),
                    _ => default_style,
                };
                spans.push(Span::styled(s, style));
            }
        }
    }

    // Use the raw line if it contains ANSI (fallback).
    if raw != plain && spans.len() <= 2 {
        spans.clear();
        spans.push(Span::styled("\u{2502} ", border_style));
        for span in parse_ansi_line(raw) {
            spans.push(Span::styled(
                span.content.into_owned(),
                default_style.patch(span.style),
            ));
        }
    }

    Line::from(spans)
}

fn render_group_header(kind: SegmentKind, line_count: usize, theme: &Theme) -> Line<'static> {
    let accent = match kind {
        SegmentKind::Thinking => Theme::FG_DIM,
        SegmentKind::Heading => theme.accent,
        SegmentKind::ToolUse => theme.info,
        SegmentKind::Code => theme.success,
        SegmentKind::Success => theme.success,
        SegmentKind::Error => theme.danger,
        SegmentKind::Blank => theme.muted,
        SegmentKind::TurnMarker => theme.muted,
    };
    let header_style = Style::default()
        .fg(accent)
        .add_modifier(Modifier::DIM | Modifier::BOLD);

    let mut spans = vec![
        Span::styled(" ", Style::default()),
        Span::styled(kind.label().to_ascii_uppercase(), header_style),
    ];
    // Show line count for groups with more than a few lines.
    if line_count > 3
        && !matches!(
            kind,
            SegmentKind::Blank | SegmentKind::TurnMarker | SegmentKind::Heading
        )
    {
        spans.push(Span::styled(
            format!(" ({line_count})"),
            Style::default()
                .fg(Theme::TEXT_PHANTOM)
                .add_modifier(Modifier::DIM),
        ));
    }
    spans.push(Span::styled(" ", Style::default()));
    spans.push(Span::styled(
        "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
        Style::default().fg(Theme::TEXT_PHANTOM),
    ));

    Line::from(spans)
}

fn render_content_line(line: &str, base_style: Style, icon: &str) -> Line<'static> {
    let mut spans = Vec::new();
    spans.push(Span::styled(" ", base_style));
    if !icon.is_empty() {
        spans.push(Span::styled(
            icon.to_string(),
            base_style.add_modifier(Modifier::DIM),
        ));
    }

    for span in parse_ansi_line(line) {
        spans.push(Span::styled(
            span.content.into_owned(),
            base_style.patch(span.style),
        ));
    }

    Line::from(spans)
}

fn content_style(kind: SegmentKind, theme: &Theme) -> Style {
    match kind {
        SegmentKind::Thinking => Style::default().fg(Theme::FG_DIM),
        SegmentKind::Heading => Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
        SegmentKind::ToolUse => Style::default().fg(theme.info),
        SegmentKind::Code => Style::default().fg(theme.success).bg(Theme::BG_SECONDARY),
        SegmentKind::Success => Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD),
        SegmentKind::Error => Style::default()
            .fg(theme.danger)
            .add_modifier(Modifier::BOLD),
        SegmentKind::Blank => Style::default(),
        SegmentKind::TurnMarker => Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
    }
}

#[cfg(test)]
mod tests {
    use super::{SegmentKind, output_byte_len, parse_segment_groups, render_cached_output};
    use crate::tui::dashboard::Theme;

    #[test]
    fn parses_and_groups_semantic_segments() {
        let groups = parse_segment_groups(&[
            "# Heading".to_string(),
            "Running cargo check".to_string(),
            "$ cargo test".to_string(),
            "Thinking out loud".to_string(),
            "Next thought".to_string(),
            "✓ PASS".to_string(),
            "ERROR compile failed".to_string(),
            String::new(),
            "──── turn 2".to_string(),
        ]);

        assert_eq!(
            groups.iter().map(|group| group.kind).collect::<Vec<_>>(),
            vec![
                SegmentKind::Heading,
                SegmentKind::ToolUse,
                SegmentKind::Thinking,
                SegmentKind::Success,
                SegmentKind::Error,
                SegmentKind::Blank,
                SegmentKind::TurnMarker,
            ]
        );
        assert_eq!(groups[1].lines.len(), 2);
        assert_eq!(groups[2].lines.len(), 2);
    }

    #[test]
    fn preprocesses_prompt_breaks_and_long_sentences() {
        let groups = parse_segment_groups(&[format!(
            "Reasoning starts here and keeps going. {} Now break this thought. Let this one split too.",
            "Another sentence. ".repeat(8)
        )]);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].kind, SegmentKind::Thinking);
        assert!(groups[0].lines.len() >= 3);
    }

    #[test]
    fn hard_splits_long_single_sentence_without_boundaries() {
        let groups = parse_segment_groups(&["x".repeat(260)]);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].kind, SegmentKind::Thinking);
        assert!(groups[0].lines.len() >= 3);
        assert!(groups[0].lines.iter().all(|line| line.len() <= 120));
    }

    #[test]
    fn recognizes_fenced_and_indented_code() {
        let groups = parse_segment_groups(&[
            String::new(),
            "    let answer = 42;".to_string(),
            "    let more = answer + 1;".to_string(),
            "    ".to_string(),
            "    println!(\"done\");".to_string(),
            "```rust".to_string(),
            "fn main() {}".to_string(),
            "```".to_string(),
        ]);

        assert_eq!(groups[0].kind, SegmentKind::Blank);
        assert_eq!(groups[1].kind, SegmentKind::Code);
        assert_eq!(groups[1].lines.len(), 7);
    }

    #[test]
    fn cached_render_tracks_byte_len_and_headers() {
        let render = render_cached_output(
            &["Running tests".to_string(), "PASS".to_string()],
            &Theme::dark(),
        );

        assert_eq!(
            render.last_len,
            output_byte_len(&["Running tests".to_string(), "PASS".to_string()])
        );
        assert_eq!(render.groups.len(), 2);
        assert!(render.styled_lines.len() >= 4);
        assert!(
            render.styled_lines[0]
                .spans
                .iter()
                .any(|span| span.content.contains("TOOL USE"))
        );
    }

    #[test]
    fn recognizes_mori_success_and_error_markers() {
        let groups =
            parse_segment_groups(&["ok wrote patch".to_string(), "REVISE plan".to_string()]);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].kind, SegmentKind::Success);
        assert_eq!(groups[1].kind, SegmentKind::Error);
    }

    #[test]
    fn recognizes_extended_tool_use_prefixes() {
        let groups = parse_segment_groups(&[
            "Searching for files".to_string(),
            "Executing command".to_string(),
            "Fetching data".to_string(),
            "Calling API".to_string(),
            "Updating config".to_string(),
        ]);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].kind, SegmentKind::ToolUse);
        assert_eq!(groups[0].lines.len(), 5);
    }

    #[test]
    fn separator_lines_between_different_segment_kinds() {
        let render = render_cached_output(
            &[
                "$ cargo test".to_string(),
                "Thinking about it".to_string(),
                "error[E0001]: bad".to_string(),
            ],
            &Theme::dark(),
        );

        // 3 headers + 3 content lines + 2 separator blank lines = 8 minimum
        assert!(
            render.styled_lines.len() >= 8,
            "expected separator lines, got {} lines",
            render.styled_lines.len()
        );
    }

    #[test]
    fn blank_groups_render_a_header_line() {
        let render = render_cached_output(&[String::new(), String::new()], &Theme::dark());

        assert_eq!(render.groups.len(), 1);
        assert_eq!(render.groups[0].kind, SegmentKind::Blank);
        assert!(
            render.styled_lines[0]
                .spans
                .iter()
                .any(|span| span.content.contains("BLANK"))
        );
    }

    #[test]
    fn long_tool_use_groups_are_folded() {
        let mut tool_lines: Vec<String> = (0..10).map(|i| format!("Running step {i}")).collect();
        tool_lines.push("done".to_string());

        let render = render_cached_output(&tool_lines, &Theme::dark());

        // Should have a fold indicator somewhere in the output.
        let has_fold = render.styled_lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains("+") && span.content.contains("lines"))
        });
        assert!(has_fold, "long tool use group should show fold indicator");
    }

    #[test]
    fn code_blocks_get_border() {
        let render = render_cached_output(
            &[
                "```rust".to_string(),
                "fn main() {}".to_string(),
                "```".to_string(),
            ],
            &Theme::dark(),
        );

        // Code block lines should have a left border character.
        let has_border = render.styled_lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains('\u{2502}'))
        });
        assert!(has_border, "code blocks should have a left border");
    }

    #[test]
    fn turn_markers_show_turn_number() {
        let render = render_cached_output(
            &[
                "thinking".to_string(),
                "\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
                "more thinking".to_string(),
            ],
            &Theme::dark(),
        );

        let has_turn = render.styled_lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains("turn 1"))
        });
        assert!(has_turn, "turn markers should show numbered turn label");
    }

    #[test]
    fn json_code_block_highlights_keys() {
        let render = render_cached_output(
            &[
                "```json".to_string(),
                r#"{"name": "roko", "count": 42}"#.to_string(),
                "```".to_string(),
            ],
            &Theme::dark(),
        );

        // Should have at least one span with the key style (BONE color).
        let has_key_color = render.styled_lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains("\"name\"") && span.style.fg == Some(Theme::BONE))
        });
        assert!(
            has_key_color,
            "JSON keys should be highlighted with BONE color"
        );
    }
}
