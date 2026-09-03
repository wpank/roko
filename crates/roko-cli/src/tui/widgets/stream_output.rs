//! Semantic agent output stream renderer.
//!
//! Parses `\u{001e}roko.stream.v1` prefixed records from agent output and renders
//! them as structured, visually distinct blocks (tool calls, reasoning, text).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::Theme;

// ---------------------------------------------------------------------------
// Stream record protocol
// ---------------------------------------------------------------------------

/// The record separator character that prefixes semantic stream records.
const RECORD_SEP: char = '\u{001e}';

/// The version prefix that follows the record separator.
const STREAM_PREFIX: &str = "roko.stream.v1 ";

/// Maximum number of output lines shown inline for a tool result.
const TOOL_RESULT_MAX_LINES: usize = 3;

/// A parsed semantic record from the agent output stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamRecord {
    /// Free-form text output from the agent.
    Text { content: String },
    /// Internal reasoning / chain-of-thought.
    Reasoning { content: String },
    /// The start of a tool invocation.
    ToolStart { tool_name: String, tool_id: String },
    /// The result returned by a tool.
    ToolResult { tool_id: String, output: String },
    /// A line that does not carry the stream protocol prefix.
    Plain { content: String },
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a single line of agent output into a [`StreamRecord`].
///
/// Lines that begin with `\x1e` (the ASCII record separator) followed by
/// `roko.stream.v1 ` are treated as semantic records whose JSON payload
/// determines the variant. All other lines become [`StreamRecord::Plain`].
#[must_use]
pub fn parse_stream_line(line: &str) -> StreamRecord {
    // Fast path: reject lines that don't start with the record separator.
    let rest = match line.strip_prefix(RECORD_SEP) {
        Some(r) => r,
        None => return StreamRecord::Plain { content: line.to_owned() },
    };

    let json_str = match rest.strip_prefix(STREAM_PREFIX) {
        Some(j) => j,
        None => return StreamRecord::Plain { content: line.to_owned() },
    };

    // Parse the JSON payload.
    let value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return StreamRecord::Plain { content: line.to_owned() },
    };

    let kind = value.get("kind").and_then(|v| v.as_str()).unwrap_or("");

    match kind {
        "text" => StreamRecord::Text {
            content: value
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
        },
        "reasoning" => StreamRecord::Reasoning {
            content: value
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
        },
        "tool_start" => StreamRecord::ToolStart {
            tool_name: value
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            tool_id: value
                .get("tool_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
        },
        "tool_result" => StreamRecord::ToolResult {
            tool_id: value
                .get("tool_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            output: value
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
        },
        // Unknown kind — treat as plain text so nothing is silently dropped.
        _ => StreamRecord::Plain { content: line.to_owned() },
    }
}

// ---------------------------------------------------------------------------
// Line rendering
// ---------------------------------------------------------------------------

/// Convert raw output lines into styled ratatui [`Line`]s.
///
/// Each line is parsed via [`parse_stream_line`] and rendered with
/// ROSEDUST-themed styles that visually distinguish tool calls, reasoning,
/// regular text, and plain pass-through output.
#[must_use]
pub fn render_output_lines<'a>(lines: &[String], theme: &Theme) -> Vec<Line<'a>> {
    let mut styled: Vec<Line<'a>> = Vec::with_capacity(lines.len());

    for line in lines {
        let record = parse_stream_line(line);
        match record {
            StreamRecord::ToolStart { tool_name, .. } => {
                styled.push(Line::from(Span::styled(
                    format!("\u{25b8} {tool_name}"),
                    Style::default()
                        .fg(Theme::DREAM)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            StreamRecord::ToolResult { output, .. } => {
                let result_lines: Vec<&str> = output.lines().collect();
                let display_count = result_lines.len().min(TOOL_RESULT_MAX_LINES);
                for result_line in &result_lines[..display_count] {
                    styled.push(Line::from(Span::styled(
                        format!("\u{2502} {result_line}"),
                        Style::default().fg(Theme::TEXT_DIM),
                    )));
                }
                if result_lines.len() > TOOL_RESULT_MAX_LINES {
                    let remaining = result_lines.len() - TOOL_RESULT_MAX_LINES;
                    styled.push(Line::from(Span::styled(
                        format!("\u{2502} ... {remaining} more line(s)"),
                        Style::default().fg(Theme::TEXT_GHOST),
                    )));
                }
            }
            StreamRecord::Reasoning { content, .. } => {
                styled.push(Line::from(Span::styled(
                    format!("\u{25d0} {content}"),
                    Style::default()
                        .fg(Theme::TEXT_DIM)
                        .add_modifier(Modifier::ITALIC),
                )));
            }
            StreamRecord::Text { content } => {
                styled.push(Line::from(Span::styled(
                    content,
                    theme.text(),
                )));
            }
            StreamRecord::Plain { content } => {
                styled.push(Line::from(Span::raw(content)));
            }
        }
    }

    styled
}

// ---------------------------------------------------------------------------
// Widget render entry-point
// ---------------------------------------------------------------------------

/// Render the semantic agent output stream widget.
///
/// Parses each line in `lines` as a potential `roko.stream.v1` record and
/// renders it with structured styling. Supports scroll offset and auto-tail
/// modes.
///
/// # Arguments
///
/// * `f` — the ratatui frame to draw into.
/// * `area` — the bounding rectangle for this widget.
/// * `lines` — raw output lines from the agent (may contain stream records).
/// * `theme` — the active ROSEDUST theme.
/// * `scroll_offset` — number of lines scrolled up from the bottom (0 = tail).
/// * `auto_tail` — when `true` and `scroll_offset == 0`, follow new output.
pub fn render_stream_output(
    f: &mut Frame<'_>,
    area: Rect,
    lines: &[String],
    theme: &Theme,
    scroll_offset: usize,
    auto_tail: bool,
) {
    // -- Title --
    let mut title_spans = vec![Span::styled(
        " Agent Output",
        Style::default()
            .fg(Theme::ROSE_BRIGHT)
            .add_modifier(Modifier::BOLD),
    )];

    // -- Scroll / tail indicator in title --
    if !lines.is_empty() {
        let indicator = if auto_tail && scroll_offset == 0 {
            Span::styled(
                " [TAIL]",
                Style::default()
                    .fg(Theme::SAGE)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!(" [SCROLL +{scroll_offset}]"),
                Style::default().fg(Theme::BONE_DIM),
            )
        };
        title_spans.push(indicator);
    }
    title_spans.push(Span::raw(" "));

    // -- Block chrome --
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(title_spans))
        .border_style(Style::default().fg(Theme::TEXT_GHOST))
        .style(Theme::block_style());
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 4 || inner.height < 1 {
        return;
    }

    // -- Empty state --
    if lines.is_empty() {
        f.render_widget(
            Paragraph::new(" Waiting for agent output...")
                .style(theme.muted()),
            inner,
        );
        return;
    }

    // -- Build styled lines --
    let styled_lines = render_output_lines(lines, theme);
    let total = styled_lines.len();
    let visible = inner.height as usize;

    // -- Scroll position indicator (top-right corner) --
    let effective_offset = if auto_tail && scroll_offset == 0 {
        0
    } else {
        scroll_offset
    };

    let scroll_y = if effective_offset == 0 {
        // Auto-tail: show the latest lines.
        total.saturating_sub(visible) as u16
    } else {
        let max_scroll = total.saturating_sub(visible);
        max_scroll.saturating_sub(effective_offset) as u16
    };

    // -- Render paragraph --
    f.render_widget(
        Paragraph::new(styled_lines)
            .style(Style::default().fg(Theme::TEXT_DIM))
            .scroll((scroll_y, 0)),
        inner,
    );

    // -- Scroll position tag in top-right corner of inner area --
    if total > visible {
        let current_top = scroll_y as usize;
        let pos_text = format!("[{}/{}]", current_top + 1, total);
        let pos_x = inner
            .right()
            .saturating_sub(pos_text.len() as u16)
            .max(inner.x);
        let pos_span = Span::styled(pos_text, Style::default().fg(Theme::TEXT_GHOST));
        f.render_widget(
            Paragraph::new(Line::from(pos_span)),
            Rect::new(pos_x, inner.y, inner.right().saturating_sub(pos_x), 1),
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_line() {
        let rec = parse_stream_line("hello world");
        assert_eq!(rec, StreamRecord::Plain { content: "hello world".to_owned() });
    }

    #[test]
    fn parse_text_record() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hello\"}";
        let rec = parse_stream_line(line);
        assert_eq!(rec, StreamRecord::Text { content: "hello".to_owned() });
    }

    #[test]
    fn parse_reasoning_record() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"reasoning\",\"content\":\"thinking...\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::Reasoning { content: "thinking...".to_owned() }
        );
    }

    #[test]
    fn parse_tool_start_record() {
        let line =
            "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"read_file\",\"tool_id\":\"t1\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::ToolStart {
                tool_name: "read_file".to_owned(),
                tool_id: "t1".to_owned(),
            }
        );
    }

    #[test]
    fn parse_tool_result_record() {
        let line =
            "\x1eroko.stream.v1 {\"kind\":\"tool_result\",\"tool_id\":\"t1\",\"output\":\"line1\\nline2\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::ToolResult {
                tool_id: "t1".to_owned(),
                output: "line1\nline2".to_owned(),
            }
        );
    }

    #[test]
    fn parse_invalid_json_returns_plain() {
        let line = "\x1eroko.stream.v1 not-json";
        let rec = parse_stream_line(line);
        assert!(matches!(rec, StreamRecord::Plain { .. }));
    }

    #[test]
    fn parse_unknown_kind_returns_plain() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"unknown_kind\",\"data\":1}";
        let rec = parse_stream_line(line);
        assert!(matches!(rec, StreamRecord::Plain { .. }));
    }

    #[test]
    fn parse_wrong_prefix_returns_plain() {
        let line = "\x1ewrong.prefix {\"kind\":\"text\",\"content\":\"x\"}";
        let rec = parse_stream_line(line);
        assert!(matches!(rec, StreamRecord::Plain { .. }));
    }

    #[test]
    fn render_output_lines_handles_mixed_content() {
        let theme = Theme::dark();
        let lines = vec![
            "plain line".to_owned(),
            "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hello\"}".to_owned(),
            "\x1eroko.stream.v1 {\"kind\":\"reasoning\",\"content\":\"hmm\"}".to_owned(),
            "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"bash\",\"tool_id\":\"t2\"}".to_owned(),
            "\x1eroko.stream.v1 {\"kind\":\"tool_result\",\"tool_id\":\"t2\",\"output\":\"ok\"}".to_owned(),
        ];
        let rendered = render_output_lines(&lines, &theme);
        assert_eq!(rendered.len(), 5);
    }

    #[test]
    fn tool_result_truncation() {
        let theme = Theme::dark();
        let long_output = (0..10).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let lines = vec![
            format!(
                "\x1eroko.stream.v1 {{\"kind\":\"tool_result\",\"tool_id\":\"t3\",\"output\":{}}}",
                serde_json::to_string(&long_output).unwrap()
            ),
        ];
        let rendered = render_output_lines(&lines, &theme);
        // 3 visible lines + 1 "... N more" line
        assert_eq!(rendered.len(), TOOL_RESULT_MAX_LINES + 1);
    }

    #[test]
    fn tool_start_uses_dream_color() {
        let theme = Theme::dark();
        let lines = vec![
            "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"grep\",\"tool_id\":\"t4\"}".to_owned(),
        ];
        let rendered = render_output_lines(&lines, &theme);
        assert_eq!(rendered.len(), 1);
        let span = &rendered[0].spans[0];
        assert_eq!(span.style.fg, Some(Theme::DREAM));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn reasoning_uses_italic() {
        let theme = Theme::dark();
        let lines = vec![
            "\x1eroko.stream.v1 {\"kind\":\"reasoning\",\"content\":\"step 1\"}".to_owned(),
        ];
        let rendered = render_output_lines(&lines, &theme);
        let span = &rendered[0].spans[0];
        assert!(span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn empty_content_fields_handled() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"text\"}";
        let rec = parse_stream_line(line);
        assert_eq!(rec, StreamRecord::Text { content: String::new() });
    }
}
