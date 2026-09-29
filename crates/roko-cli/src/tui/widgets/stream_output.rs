//! Semantic agent output stream renderer.
//!
//! Parses `\u{001e}roko.stream.v1` prefixed records from agent output and renders
//! them as structured, visually distinct blocks (tool calls, reasoning, text).
//!
//! # Visual semantics (P2.6)
//!
//! - **Tool start**: `▶ tool_name` in DREAM (bold)
//! - **Tool result (success)**: `✓ done` in SAGE, output lines prefixed with `│`
//! - **Tool result (error)**: `✗ error` in EMBER, output lines prefixed with `│`
//! - **Reasoning**: `◐ text` in DIM italic
//! - **Separators**: thin `─` lines between tool blocks
//! - **Fold/unfold**: long results show first N lines + "... N more lines"
//!   with an expanded set for unfolded tool IDs
//! - **Search highlighting**: matching text is split into highlighted spans

use std::collections::HashSet;

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

/// Maximum number of output lines shown inline for a folded tool result.
const TOOL_RESULT_FOLDED_LINES: usize = 3;

/// Maximum number of output lines shown inline for an unfolded tool result.
const TOOL_RESULT_UNFOLDED_LINES: usize = 50;

/// A parsed semantic record from the agent output stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamRecord {
    /// Free-form text output from the agent.
    Text {
        content: String,
        /// Whether this record was emitted before the agent turn completed
        /// (i.e. it is a live preview rather than a settled transcript).
        live: bool,
        /// Whether the safety screener has validated this record.
        /// Defaults to `true` when the field is absent from the payload.
        screened: bool,
    },
    /// Internal reasoning / chain-of-thought.
    Reasoning {
        content: String,
        /// Whether this record is a live preview.
        live: bool,
        /// Whether the safety screener has validated this record.
        screened: bool,
    },
    /// The start of a tool invocation.
    ToolStart {
        tool_name: String,
        tool_id: String,
        /// Whether this record is a live preview.
        live: bool,
        /// Whether the safety screener has validated this record.
        screened: bool,
        /// Optional target of the tool invocation (e.g. a file path for Write).
        target: Option<String>,
    },
    /// The result returned by a tool.
    ToolResult {
        tool_id: String,
        output: String,
        /// Whether the tool reported an error. Parsed from optional `"is_error"`
        /// boolean in the JSON payload; defaults to heuristic detection.
        is_error: bool,
    },
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
        None => {
            return StreamRecord::Plain {
                content: line.to_owned(),
            };
        }
    };

    let json_str = match rest.strip_prefix(STREAM_PREFIX) {
        Some(j) => j,
        None => {
            return StreamRecord::Plain {
                content: line.to_owned(),
            };
        }
    };

    // Parse the JSON payload.
    let value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => {
            return StreamRecord::Plain {
                content: line.to_owned(),
            };
        }
    };

    let kind = value.get("kind").and_then(|v| v.as_str()).unwrap_or("");

    match kind {
        "text" => StreamRecord::Text {
            content: value
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            live: value
                .get("live")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            // `screened` is absent on pre-protocol records; default to true.
            screened: value
                .get("screened")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        },
        "reasoning" => StreamRecord::Reasoning {
            content: value
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            live: value
                .get("live")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            screened: value
                .get("screened")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
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
            live: value
                .get("live")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            screened: value
                .get("screened")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            target: value
                .get("target")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned()),
        },
        "tool_result" => {
            let output = value
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            // Explicit `is_error` field takes priority; fall back to heuristic
            // detection from the output text.
            let is_error = value
                .get("is_error")
                .and_then(|v| v.as_bool())
                .unwrap_or_else(|| detect_error_output(&output));
            StreamRecord::ToolResult {
                tool_id: value
                    .get("tool_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                output,
                is_error,
            }
        }
        // Unknown kind -- treat as plain text so nothing is silently dropped.
        _ => StreamRecord::Plain {
            content: line.to_owned(),
        },
    }
}

/// Heuristic: does the output text look like an error?
fn detect_error_output(output: &str) -> bool {
    let first = output.lines().next().unwrap_or("");
    let t = first.trim();
    t.starts_with("ERROR")
        || t.starts_with("Error")
        || t.starts_with("error")
        || t.starts_with("FAILED")
        || t.starts_with("panic")
        || t.starts_with("traceback")
        || t.starts_with("Traceback")
}

// ---------------------------------------------------------------------------
// Render options
// ---------------------------------------------------------------------------

/// Configuration for [`render_output_lines_styled`].
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Tool IDs whose results should be shown unfolded (up to
    /// `TOOL_RESULT_UNFOLDED_LINES` lines instead of `TOOL_RESULT_FOLDED_LINES`).
    pub unfolded_tool_ids: HashSet<String>,
    /// Optional compiled search regex for highlighting matching text.
    pub search_pattern: Option<regex::Regex>,
}

// ---------------------------------------------------------------------------
// Line rendering
// ---------------------------------------------------------------------------

/// Convert raw output lines into styled ratatui [`Line`]s.
///
/// This is the simple entry point that preserves backward compatibility.
/// Each line is parsed via [`parse_stream_line`] and rendered with
/// ROSEDUST-themed styles that visually distinguish tool calls, reasoning,
/// regular text, and plain pass-through output.
#[must_use]
pub fn render_output_lines<'a>(lines: &[String], theme: &Theme) -> Vec<Line<'a>> {
    render_output_lines_styled(lines, theme, &RenderOptions::default())
}

/// Convert raw output lines into styled ratatui [`Line`]s with full options.
///
/// Supports fold/unfold per tool ID, search highlighting, and visual
/// separators between semantic blocks.
#[must_use]
pub fn render_output_lines_styled<'a>(
    lines: &[String],
    theme: &Theme,
    opts: &RenderOptions,
) -> Vec<Line<'a>> {
    let mut styled: Vec<Line<'a>> = Vec::with_capacity(lines.len() * 2);
    let mut prev_kind: Option<BlockKind> = None;

    for line in lines {
        let record = parse_stream_line(line);
        let cur_kind = BlockKind::from_record(&record);

        // Insert a separator line between distinct semantic blocks.
        if should_insert_separator(prev_kind, cur_kind) {
            styled.push(render_separator(theme));
        }
        prev_kind = Some(cur_kind);

        match record {
            StreamRecord::ToolStart {
                tool_name,
                target,
                live,
                ..
            } => {
                if live {
                    // ▸ tool_name [target]  (DREAM, bold) — single live-step line
                    let label = match target.as_deref() {
                        Some(tgt) => format!("\u{25b8} {tool_name} {tgt}"),
                        None => format!("\u{25b8} {tool_name}"),
                    };
                    styled.push(Line::from(Span::styled(
                        label,
                        Style::default()
                            .fg(Theme::DREAM)
                            .add_modifier(Modifier::BOLD),
                    )));
                } else {
                    // ▶ tool_name  (DREAM, bold)
                    styled.push(Line::from(Span::styled(
                        format!("\u{25b6} {tool_name}"),
                        Style::default()
                            .fg(Theme::DREAM)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
            }
            StreamRecord::ToolResult {
                tool_id,
                output,
                is_error,
            } => {
                render_tool_result(&mut styled, &tool_id, &output, is_error, opts, theme);
            }
            StreamRecord::Reasoning {
                content, screened, ..
            } => {
                let text = if screened {
                    format!("\u{25d0} {content}")
                } else {
                    format!("\u{25d0} [unscreened] {content}")
                };
                let base_style = if screened {
                    Style::default()
                        .fg(Theme::TEXT_DIM)
                        .add_modifier(Modifier::ITALIC)
                } else {
                    Style::default()
                        .fg(Theme::TEXT_GHOST)
                        .add_modifier(Modifier::DIM)
                        .add_modifier(Modifier::ITALIC)
                };
                styled.push(highlight_line(&text, base_style, opts));
            }
            StreamRecord::Text { content, screened, .. } => {
                if screened {
                    styled.push(highlight_line(&content, theme.text(), opts));
                } else {
                    // Unscreened live text: dimmed and labelled.
                    let text = format!("[unscreened] {content}");
                    let base_style = Style::default()
                        .fg(Theme::TEXT_GHOST)
                        .add_modifier(Modifier::DIM);
                    styled.push(highlight_line(&text, base_style, opts));
                }
            }
            StreamRecord::Plain { content } => {
                styled.push(highlight_line(&content, Style::default(), opts));
            }
        }
    }

    styled
}

/// Semantic block kind for separator logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Tool,
    Reasoning,
    Text,
}

impl BlockKind {
    fn from_record(rec: &StreamRecord) -> Self {
        match rec {
            StreamRecord::ToolStart { .. } | StreamRecord::ToolResult { .. } => Self::Tool,
            StreamRecord::Reasoning { .. } => Self::Reasoning,
            StreamRecord::Text { .. } | StreamRecord::Plain { .. } => Self::Text,
        }
    }
}

/// Decide whether to insert a visual separator between two adjacent blocks.
fn should_insert_separator(prev: Option<BlockKind>, cur: BlockKind) -> bool {
    match prev {
        None => false,
        Some(p) => {
            // Separator when transitioning between tool blocks and non-tool blocks,
            // or between a tool result and a new tool start.
            match (p, cur) {
                (BlockKind::Tool, BlockKind::Text)
                | (BlockKind::Tool, BlockKind::Reasoning)
                | (BlockKind::Text, BlockKind::Tool)
                | (BlockKind::Reasoning, BlockKind::Tool) => true,
                _ => false,
            }
        }
    }
}

/// Render a thin separator line.
fn render_separator<'a>(theme: &Theme) -> Line<'a> {
    let _ = theme;
    Line::from(Span::styled(
        "\u{2500}\u{2500}\u{2500}",
        Style::default().fg(Theme::SEPARATOR),
    ))
}

/// Render a tool result block with status icon, color, and fold/unfold.
fn render_tool_result(
    styled: &mut Vec<Line<'_>>,
    tool_id: &str,
    output: &str,
    is_error: bool,
    opts: &RenderOptions,
    theme: &Theme,
) {
    // Status header line: ✓ done (green) or ✗ error (red)
    let (icon, status_text, status_color) = if is_error {
        ("\u{2717}", "error", Theme::EMBER)
    } else {
        ("\u{2713}", "done", Theme::SAGE)
    };
    styled.push(Line::from(Span::styled(
        format!("{icon} {status_text}"),
        Style::default()
            .fg(status_color)
            .add_modifier(Modifier::BOLD),
    )));

    // Result body lines with fold/unfold.
    let result_lines: Vec<&str> = output.lines().collect();
    if result_lines.is_empty() {
        return;
    }

    let is_unfolded = opts.unfolded_tool_ids.contains(tool_id);
    let max_lines = if is_unfolded {
        TOOL_RESULT_UNFOLDED_LINES
    } else {
        TOOL_RESULT_FOLDED_LINES
    };
    let display_count = result_lines.len().min(max_lines);

    // Body color: dim for success, ember-tinted for errors.
    let body_color = if is_error {
        Theme::ROSE_DIM
    } else {
        Theme::TEXT_DIM
    };

    for result_line in &result_lines[..display_count] {
        let text = format!("\u{2502} {result_line}");
        let base_style = Style::default().fg(body_color);
        styled.push(highlight_line(&text, base_style, opts));
    }

    if result_lines.len() > max_lines {
        let remaining = result_lines.len() - max_lines;
        let fold_hint = if is_unfolded {
            format!("\u{2502} ... {remaining} more line(s)")
        } else {
            format!("\u{2502} ... {remaining} more line(s) \u{25b8}")
        };
        styled.push(Line::from(Span::styled(
            fold_hint,
            Style::default().fg(Theme::TEXT_GHOST),
        )));
    }

    // Bottom cap for the tool block.
    let _ = theme;
    styled.push(Line::from(Span::styled(
        "\u{2514}\u{2500}",
        Style::default().fg(Theme::SEPARATOR),
    )));
}

// ---------------------------------------------------------------------------
// Search highlighting
// ---------------------------------------------------------------------------

/// Render a line of text, splitting it into highlighted spans if a search
/// pattern is active. When no pattern is set, returns a single-span line.
fn highlight_line<'a>(text: &str, base_style: Style, opts: &RenderOptions) -> Line<'a> {
    let re = match opts.search_pattern.as_ref() {
        Some(re) if !text.is_empty() => re,
        _ => return Line::from(Span::styled(text.to_owned(), base_style)),
    };

    let highlight_style = base_style
        .bg(Theme::BG_HIGHLIGHT)
        .fg(Theme::BONE_BRIGHT)
        .add_modifier(Modifier::BOLD);

    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut last_end = 0;

    for mat in re.find_iter(text) {
        let start = mat.start();
        let end = mat.end();
        if start > last_end {
            spans.push(Span::styled(text[last_end..start].to_owned(), base_style));
        }
        spans.push(Span::styled(text[start..end].to_owned(), highlight_style));
        last_end = end;
    }

    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_owned(), base_style));
    }

    if spans.is_empty() {
        Line::from(Span::styled(text.to_owned(), base_style))
    } else {
        Line::from(spans)
    }
}

// ---------------------------------------------------------------------------
// Structured record renderer (P2-TUI-9 / TUI-G6)
// ---------------------------------------------------------------------------

/// Convert a slice of structured [`AgentOutputRecord`]s into styled ratatui
/// [`Line`]s with semantic block rendering.
///
/// This is the preferred render path when output is sourced from
/// [`AgentOutputHistory`] — it uses the pre-classified `kind` and tool
/// metadata directly instead of re-parsing raw text.
///
/// Applies the same visual conventions as [`render_output_lines_styled`]:
/// - **Tool call**: `▶ tool_name` in DREAM (bold), wrench icon
/// - **Tool result (success)**: `✓ done` in SAGE, lines prefixed with `│`
/// - **Tool result (error)**: `✗ error` in EMBER, lines prefixed with `│`
/// - **Reasoning**: `◐ text` in DIM italic, brain icon
/// - **Error**: `✗ text` in EMBER bold
/// - **System**: dim separator line
/// - **Separators**: thin `─` lines between semantic block transitions
/// - **Fold/unfold**: controlled via [`RenderOptions::unfolded_tool_ids`]
/// - **Search highlighting**: splits text into highlighted spans
#[must_use]
pub fn render_output_records_styled<'a>(
    records: &[crate::tui::state::AgentOutputRecord],
    theme: &Theme,
    opts: &RenderOptions,
) -> Vec<Line<'a>> {
    use crate::tui::state::OutputRecordKind;

    let mut styled: Vec<Line<'a>> = Vec::with_capacity(records.len() * 2);
    let mut prev_kind: Option<RecordBlockKind> = None;

    for record in records {
        let cur_kind = RecordBlockKind::from_output_kind(record.kind);

        // Insert a thin separator when crossing between distinct block kinds.
        if should_insert_separator_rk(prev_kind, cur_kind) {
            styled.push(render_separator(theme));
        }
        prev_kind = Some(cur_kind);

        match record.kind {
            OutputRecordKind::ToolCall => {
                // Re-parse the raw record text to check for live step + target.
                // `record.text` holds the original encoded line when ingested from
                // a stream; for records created from structured events it may just
                // be human-readable text, in which case parsing returns Plain and
                // the live/target path is skipped safely.
                let live_step = if record.text.starts_with('\u{001e}') {
                    match parse_stream_line(&record.text) {
                        StreamRecord::ToolStart {
                            live: true, target, ..
                        } => Some(target),
                        _ => None,
                    }
                } else {
                    None
                };

                let name = record
                    .tool_name
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .unwrap_or("tool");

                let label = match live_step {
                    // ▸ tool_name [target]  (DREAM, bold) — live step
                    Some(Some(ref tgt)) => format!("\u{25b8} {name} {tgt}"),
                    Some(None) => format!("\u{25b8} {name}"),
                    // ▶ tool_name  (DREAM, bold) — settled call
                    None => format!("\u{25b6} {name}"),
                };
                styled.push(Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(Theme::DREAM)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            OutputRecordKind::ToolResult => {
                // Use the tool_id from the record if available for fold state.
                let tool_id = record.tool_id.as_deref().unwrap_or("").to_owned();
                // Treat non-empty error text starting with error keywords as errors.
                let is_error = detect_error_output(&record.text);
                render_tool_result(&mut styled, &tool_id, &record.text, is_error, opts, theme);
            }
            OutputRecordKind::Reasoning => {
                // Check for unscreened live reasoning by re-parsing raw text.
                let unscreened = if record.text.starts_with('\u{001e}') {
                    matches!(
                        parse_stream_line(&record.text),
                        StreamRecord::Reasoning { live: true, screened: false, .. }
                    )
                } else {
                    false
                };

                // ◐ reasoning text  (TEXT_DIM, italic)
                let text = if record.text.is_empty() {
                    "\u{25d0}".to_owned()
                } else if unscreened {
                    format!("\u{25d0} [unscreened] {}", record.text)
                } else {
                    format!("\u{25d0} {}", record.text)
                };
                let base_style = if unscreened {
                    Style::default()
                        .fg(Theme::TEXT_GHOST)
                        .add_modifier(Modifier::DIM)
                        .add_modifier(Modifier::ITALIC)
                } else {
                    Style::default()
                        .fg(Theme::TEXT_DIM)
                        .add_modifier(Modifier::ITALIC)
                };
                styled.push(highlight_line(&text, base_style, opts));
            }
            OutputRecordKind::Error => {
                // ✗ error text  (EMBER, bold)
                let text = if record.text.is_empty() {
                    "\u{2717}".to_owned()
                } else {
                    format!("\u{2717} {}", record.text)
                };
                let base_style = Style::default()
                    .fg(Theme::EMBER)
                    .add_modifier(Modifier::BOLD);
                styled.push(highlight_line(&text, base_style, opts));
            }
            OutputRecordKind::System => {
                // System/boundary lines rendered as dim separators.
                if record.text.is_empty()
                    || record.text.trim_start_matches('\u{2500}').trim().is_empty()
                {
                    styled.push(Line::from(Span::styled(
                        "\u{2500}\u{2500}\u{2500}",
                        Style::default().fg(Theme::SEPARATOR),
                    )));
                } else {
                    styled.push(highlight_line(
                        &record.text,
                        Style::default().fg(Theme::TEXT_GHOST),
                        opts,
                    ));
                }
            }
            OutputRecordKind::Text => {
                // Check for unscreened live text by re-parsing raw text.
                let unscreened = if record.text.starts_with('\u{001e}') {
                    matches!(
                        parse_stream_line(&record.text),
                        StreamRecord::Text { live: true, screened: false, .. }
                    )
                } else {
                    false
                };

                if unscreened {
                    let text = format!("[unscreened] {}", record.text);
                    let base_style = Style::default()
                        .fg(Theme::TEXT_GHOST)
                        .add_modifier(Modifier::DIM);
                    styled.push(highlight_line(&text, base_style, opts));
                } else {
                    styled.push(highlight_line(&record.text, theme.text(), opts));
                }
            }
        }
    }

    styled
}

/// Block kind enum for separator logic in the records render path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordBlockKind {
    Tool,
    Reasoning,
    Text,
    System,
}

impl RecordBlockKind {
    fn from_output_kind(kind: crate::tui::state::OutputRecordKind) -> Self {
        use crate::tui::state::OutputRecordKind;
        match kind {
            OutputRecordKind::ToolCall | OutputRecordKind::ToolResult => Self::Tool,
            OutputRecordKind::Reasoning => Self::Reasoning,
            OutputRecordKind::System => Self::System,
            OutputRecordKind::Text | OutputRecordKind::Error => Self::Text,
        }
    }
}

/// Separator logic for the structured record render path.
fn should_insert_separator_rk(prev: Option<RecordBlockKind>, cur: RecordBlockKind) -> bool {
    match prev {
        None => false,
        Some(p) => matches!(
            (p, cur),
            (RecordBlockKind::Tool, RecordBlockKind::Text)
                | (RecordBlockKind::Tool, RecordBlockKind::Reasoning)
                | (RecordBlockKind::Text, RecordBlockKind::Tool)
                | (RecordBlockKind::Reasoning, RecordBlockKind::Tool)
                | (RecordBlockKind::System, RecordBlockKind::Tool)
                | (RecordBlockKind::Tool, RecordBlockKind::System)
        ),
    }
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
/// * `f` -- the ratatui frame to draw into.
/// * `area` -- the bounding rectangle for this widget.
/// * `lines` -- raw output lines from the agent (may contain stream records).
/// * `theme` -- the active ROSEDUST theme.
/// * `scroll_offset` -- number of lines scrolled up from the bottom (0 = tail).
/// * `auto_tail` -- when `true` and `scroll_offset == 0`, follow new output.
pub fn render_stream_output(
    f: &mut Frame<'_>,
    area: Rect,
    lines: &[String],
    theme: &Theme,
    scroll_offset: usize,
    auto_tail: bool,
) {
    render_stream_output_with_opts(f, area, lines, theme, scroll_offset, auto_tail, None)
}

/// Extended entry point that accepts render options for fold/unfold and search.
pub fn render_stream_output_with_opts(
    f: &mut Frame<'_>,
    area: Rect,
    lines: &[String],
    theme: &Theme,
    scroll_offset: usize,
    auto_tail: bool,
    opts: Option<&RenderOptions>,
) {
    let default_opts = RenderOptions::default();
    let opts = opts.unwrap_or(&default_opts);

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

    // -- Search indicator in title --
    if let Some(ref re) = opts.search_pattern {
        title_spans.push(Span::styled(
            format!(" [/{}/]", re.as_str()),
            Style::default()
                .fg(Theme::SAGE)
                .add_modifier(Modifier::BOLD),
        ));
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
            Paragraph::new(" Waiting for agent output...").style(theme.muted()),
            inner,
        );
        return;
    }

    // -- Build styled lines --
    let styled_lines = render_output_lines_styled(lines, theme, opts);
    let total = styled_lines.len();
    let visible = inner.height as usize;

    // -- Scroll position --
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
        assert_eq!(
            rec,
            StreamRecord::Plain {
                content: "hello world".to_owned()
            }
        );
    }

    #[test]
    fn parse_text_record() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hello\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::Text {
                content: "hello".to_owned(),
                live: false,
                screened: true,
            }
        );
    }

    #[test]
    fn parse_reasoning_record() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"reasoning\",\"content\":\"thinking...\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::Reasoning {
                content: "thinking...".to_owned(),
                live: false,
                screened: true,
            }
        );
    }

    #[test]
    fn parse_tool_start_record() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"read_file\",\"tool_id\":\"t1\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::ToolStart {
                tool_name: "read_file".to_owned(),
                tool_id: "t1".to_owned(),
                live: false,
                screened: true,
                target: None,
            }
        );
    }

    #[test]
    fn parse_tool_result_record() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"tool_result\",\"tool_id\":\"t1\",\"output\":\"line1\\nline2\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::ToolResult {
                tool_id: "t1".to_owned(),
                output: "line1\nline2".to_owned(),
                is_error: false,
            }
        );
    }

    #[test]
    fn parse_tool_result_with_is_error() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"tool_result\",\"tool_id\":\"t1\",\"output\":\"some output\",\"is_error\":true}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::ToolResult {
                tool_id: "t1".to_owned(),
                output: "some output".to_owned(),
                is_error: true,
            }
        );
    }

    #[test]
    fn parse_tool_result_heuristic_error_detection() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"tool_result\",\"tool_id\":\"t1\",\"output\":\"ERROR: file not found\"}";
        let rec = parse_stream_line(line);
        match rec {
            StreamRecord::ToolResult { is_error, .. } => assert!(is_error),
            _ => panic!("expected ToolResult"),
        }
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
        // plain + text + reasoning + separator(reasoning->tool) + tool_start
        // + separator(text->tool is not here, reasoning->tool is)
        // + tool result (status line + 1 body line + bottom cap)
        // Plus the separator between reasoning and tool_start
        // Let's count: plain, text, reasoning, sep, tool_start, status, body, cap
        assert!(
            rendered.len() >= 5,
            "should have at least 5 lines, got {}",
            rendered.len()
        );
    }

    #[test]
    fn tool_result_truncation_folded() {
        let theme = Theme::dark();
        let long_output = (0..10)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = vec![format!(
            "\x1eroko.stream.v1 {{\"kind\":\"tool_result\",\"tool_id\":\"t3\",\"output\":{}}}",
            serde_json::to_string(&long_output).unwrap()
        )];
        let rendered = render_output_lines(&lines, &theme);
        // status line + 3 folded body lines + "... N more" + bottom cap
        assert_eq!(rendered.len(), 1 + TOOL_RESULT_FOLDED_LINES + 1 + 1);
    }

    #[test]
    fn tool_result_unfolded_shows_more() {
        let theme = Theme::dark();
        let long_output = (0..10)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = vec![format!(
            "\x1eroko.stream.v1 {{\"kind\":\"tool_result\",\"tool_id\":\"t3\",\"output\":{}}}",
            serde_json::to_string(&long_output).unwrap()
        )];
        let opts = RenderOptions {
            unfolded_tool_ids: HashSet::from(["t3".to_string()]),
            ..Default::default()
        };
        let rendered = render_output_lines_styled(&lines, &theme, &opts);
        // status line + all 10 body lines + bottom cap (no truncation since 10 < 50)
        assert_eq!(rendered.len(), 1 + 10 + 1);
    }

    #[test]
    fn tool_start_uses_dream_color() {
        let theme = Theme::dark();
        let lines = vec![
            "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"grep\",\"tool_id\":\"t4\"}".to_owned(),
        ];
        let rendered = render_output_lines(&lines, &theme);
        assert!(!rendered.is_empty());
        let span = &rendered[0].spans[0];
        assert_eq!(span.style.fg, Some(Theme::DREAM));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn tool_result_success_uses_sage() {
        let theme = Theme::dark();
        let lines = vec![
            "\x1eroko.stream.v1 {\"kind\":\"tool_result\",\"tool_id\":\"t5\",\"output\":\"ok\"}"
                .to_owned(),
        ];
        let rendered = render_output_lines(&lines, &theme);
        // First line is the status header.
        let status_span = &rendered[0].spans[0];
        assert_eq!(status_span.style.fg, Some(Theme::SAGE));
        assert!(status_span.content.contains('\u{2713}')); // checkmark
    }

    #[test]
    fn tool_result_error_uses_ember() {
        let theme = Theme::dark();
        let lines = vec![
            "\x1eroko.stream.v1 {\"kind\":\"tool_result\",\"tool_id\":\"t6\",\"output\":\"ok\",\"is_error\":true}"
                .to_owned(),
        ];
        let rendered = render_output_lines(&lines, &theme);
        let status_span = &rendered[0].spans[0];
        assert_eq!(status_span.style.fg, Some(Theme::EMBER));
        assert!(status_span.content.contains('\u{2717}')); // x-mark
    }

    #[test]
    fn reasoning_uses_italic() {
        let theme = Theme::dark();
        let lines =
            vec!["\x1eroko.stream.v1 {\"kind\":\"reasoning\",\"content\":\"step 1\"}".to_owned()];
        let rendered = render_output_lines(&lines, &theme);
        let span = &rendered[0].spans[0];
        assert!(span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn empty_content_fields_handled() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"text\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::Text {
                content: String::new(),
                live: false,
                screened: true,
            }
        );
    }

    #[test]
    fn search_highlighting_splits_spans() {
        let theme = Theme::dark();
        let lines = vec!["hello world hello".to_owned()];
        let opts = RenderOptions {
            search_pattern: Some(regex::Regex::new("(?i)hello").unwrap()),
            ..Default::default()
        };
        let rendered = render_output_lines_styled(&lines, &theme, &opts);
        assert_eq!(rendered.len(), 1);
        // Should have: "hello" (highlighted) + " world " + "hello" (highlighted)
        assert!(
            rendered[0].spans.len() >= 3,
            "expected at least 3 spans for search match"
        );
    }

    #[test]
    fn search_highlighting_no_match_single_span() {
        let theme = Theme::dark();
        let lines = vec!["hello world".to_owned()];
        let opts = RenderOptions {
            search_pattern: Some(regex::Regex::new("xyz").unwrap()),
            ..Default::default()
        };
        let rendered = render_output_lines_styled(&lines, &theme, &opts);
        assert_eq!(rendered.len(), 1);
        // No match: should have exactly 1 span.
        assert_eq!(rendered[0].spans.len(), 1);
    }

    #[test]
    fn separator_inserted_between_text_and_tool() {
        let theme = Theme::dark();
        let lines = vec![
            "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hello\"}".to_owned(),
            "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"bash\",\"tool_id\":\"t7\"}".to_owned(),
        ];
        let rendered = render_output_lines(&lines, &theme);
        // text + separator + tool_start = 3 lines
        assert_eq!(rendered.len(), 3);
        // The separator is the middle line with the "---" chars.
        let sep_content: String = rendered[1]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            sep_content.contains('\u{2500}'),
            "separator should contain horizontal line char"
        );
    }

    #[test]
    fn no_separator_between_consecutive_text_lines() {
        let theme = Theme::dark();
        let lines = vec![
            "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hello\"}".to_owned(),
            "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"world\"}".to_owned(),
        ];
        let rendered = render_output_lines(&lines, &theme);
        // Just 2 text lines, no separator between same-kind blocks.
        assert_eq!(rendered.len(), 2);
    }

    #[test]
    fn detect_error_heuristics() {
        assert!(detect_error_output("ERROR: something broke"));
        assert!(detect_error_output("Error: bad input"));
        assert!(detect_error_output("error[E0308]: mismatched types"));
        assert!(detect_error_output("FAILED to compile"));
        assert!(detect_error_output("panic at line 42"));
        assert!(detect_error_output("Traceback (most recent call last):"));
        assert!(!detect_error_output("All tests passed"));
        assert!(!detect_error_output("ok"));
    }

    // ---------------------------------------------------------------------------
    // render_output_records_styled tests (P2-TUI-9 / TUI-G6)
    // ---------------------------------------------------------------------------

    use crate::tui::state::{AgentOutputRecord, OutputRecordKind};

    fn make_record_for_stream(kind: OutputRecordKind, text: &str) -> AgentOutputRecord {
        AgentOutputRecord {
            seq: 1,
            timestamp_ms: 0,
            role: "assistant".to_string(),
            kind,
            text: text.to_string(),
            redacted: false,
            tool_id: None,
            tool_name: None,
        }
    }

    fn make_tool_call_record(tool_name: &str, tool_id: &str) -> AgentOutputRecord {
        AgentOutputRecord {
            seq: 1,
            timestamp_ms: 0,
            role: "assistant".to_string(),
            kind: OutputRecordKind::ToolCall,
            text: String::new(),
            redacted: false,
            tool_id: Some(tool_id.to_string()),
            tool_name: Some(tool_name.to_string()),
        }
    }

    fn make_tool_result_record(tool_id: &str, output: &str) -> AgentOutputRecord {
        AgentOutputRecord {
            seq: 2,
            timestamp_ms: 0,
            role: "tool".to_string(),
            kind: OutputRecordKind::ToolResult,
            text: output.to_string(),
            redacted: false,
            tool_id: Some(tool_id.to_string()),
            tool_name: None,
        }
    }

    #[test]
    fn records_tool_call_uses_dream_color() {
        let theme = Theme::dark();
        let records = vec![make_tool_call_record("bash", "t1")];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        assert!(!rendered.is_empty());
        let span = &rendered[0].spans[0];
        assert_eq!(span.style.fg, Some(Theme::DREAM));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
        assert!(span.content.contains("bash"));
        assert!(span.content.starts_with('\u{25b6}'));
    }

    #[test]
    fn records_tool_result_success_uses_sage() {
        let theme = Theme::dark();
        let records = vec![make_tool_result_record("t1", "all good")];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        assert!(!rendered.is_empty());
        // First rendered line is the status header (✓ done).
        let status_span = &rendered[0].spans[0];
        assert_eq!(status_span.style.fg, Some(Theme::SAGE));
        assert!(status_span.content.contains('\u{2713}'));
    }

    #[test]
    fn records_tool_result_error_uses_ember() {
        let theme = Theme::dark();
        let records = vec![make_tool_result_record("t2", "ERROR: build failed")];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        assert!(!rendered.is_empty());
        let status_span = &rendered[0].spans[0];
        assert_eq!(status_span.style.fg, Some(Theme::EMBER));
        assert!(status_span.content.contains('\u{2717}'));
    }

    #[test]
    fn records_reasoning_uses_italic() {
        let theme = Theme::dark();
        let records = vec![make_record_for_stream(
            OutputRecordKind::Reasoning,
            "thinking...",
        )];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        assert!(!rendered.is_empty());
        assert!(
            rendered[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::ITALIC))
        );
        // Reasoning lines are prefixed with ◐.
        let text: String = rendered[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains('\u{25d0}'));
    }

    #[test]
    fn records_error_kind_uses_ember_bold() {
        let theme = Theme::dark();
        let records = vec![make_record_for_stream(
            OutputRecordKind::Error,
            "compile failed",
        )];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        assert!(!rendered.is_empty());
        let span = &rendered[0].spans[0];
        assert_eq!(span.style.fg, Some(Theme::EMBER));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
        // Error records are prefixed with ✗.
        assert!(span.content.starts_with('\u{2717}'));
    }

    #[test]
    fn records_separator_inserted_between_text_and_tool() {
        let theme = Theme::dark();
        let records = vec![
            make_record_for_stream(OutputRecordKind::Text, "hello"),
            make_tool_call_record("grep", "t3"),
        ];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        // text + separator + tool_call = 3 lines
        assert_eq!(rendered.len(), 3);
        let sep: String = rendered[1]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(sep.contains('\u{2500}'));
    }

    #[test]
    fn records_no_separator_between_consecutive_text() {
        let theme = Theme::dark();
        let records = vec![
            make_record_for_stream(OutputRecordKind::Text, "hello"),
            make_record_for_stream(OutputRecordKind::Text, "world"),
        ];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        assert_eq!(rendered.len(), 2);
    }

    #[test]
    fn records_tool_result_fold_default() {
        let theme = Theme::dark();
        let long_output = (0..10)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let records = vec![make_tool_result_record("t4", &long_output)];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        // status + 3 folded body lines + "... N more" + bottom cap
        assert_eq!(rendered.len(), 1 + TOOL_RESULT_FOLDED_LINES + 1 + 1);
    }

    #[test]
    fn records_tool_result_unfold_shows_more() {
        let theme = Theme::dark();
        let long_output = (0..10)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let records = vec![make_tool_result_record("t5", &long_output)];
        let opts = RenderOptions {
            unfolded_tool_ids: HashSet::from(["t5".to_string()]),
            ..Default::default()
        };
        let rendered = render_output_records_styled(&records, &theme, &opts);
        // status + all 10 body lines + bottom cap (no truncation since 10 < 50)
        assert_eq!(rendered.len(), 1 + 10 + 1);
    }

    #[test]
    fn records_search_highlighting_applied() {
        let theme = Theme::dark();
        let records = vec![make_record_for_stream(
            OutputRecordKind::Text,
            "hello world hello",
        )];
        let opts = RenderOptions {
            search_pattern: Some(regex::Regex::new("(?i)hello").unwrap()),
            ..Default::default()
        };
        let rendered = render_output_records_styled(&records, &theme, &opts);
        assert_eq!(rendered.len(), 1);
        // Should have at least 3 spans (hello + " world " + hello).
        assert!(
            rendered[0].spans.len() >= 3,
            "expected at least 3 spans for search match, got {}",
            rendered[0].spans.len()
        );
    }

    #[test]
    fn records_mixed_kinds_render_without_panic() {
        let theme = Theme::dark();
        let records = vec![
            make_record_for_stream(OutputRecordKind::Text, "starting"),
            make_tool_call_record("read_file", "t6"),
            make_tool_result_record("t6", "file contents here"),
            make_record_for_stream(OutputRecordKind::Reasoning, "analyzing..."),
            make_record_for_stream(OutputRecordKind::Error, "ERROR: file missing"),
            make_record_for_stream(OutputRecordKind::System, ""),
        ];
        let rendered = render_output_records_styled(&records, &theme, &RenderOptions::default());
        assert!(
            rendered.len() >= 6,
            "expected at least 6 output lines, got {}",
            rendered.len()
        );
    }

    // -----------------------------------------------------------------------
    // Live tool steps and unscreened record tests (T19)
    // -----------------------------------------------------------------------

    /// Verify that a `tool_start` record with `"live":true` and a `"target"`
    /// renders as a single `▸ tool_name target` line.
    #[test]
    fn renders_live_tool_steps() {
        let theme = Theme::dark();
        // Build a raw stream line with live=true and a target path.
        let line = "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"Write\",\"tool_id\":\"t99\",\"live\":true,\"screened\":true,\"target\":\"apps/x.ts\"}";
        let rendered = render_output_lines(&[line.to_owned()], &theme);
        assert_eq!(rendered.len(), 1, "live step must be a single line");
        let text: String = rendered[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            text.contains('\u{25b8}'),
            "live step must start with ▸, got: {text}"
        );
        assert!(text.contains("Write"), "tool name must appear in live step");
        assert!(
            text.contains("apps/x.ts"),
            "target must appear in live step: {text}"
        );
    }

    #[test]
    fn parse_live_tool_start_with_target() {
        let line = "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"Write\",\"tool_id\":\"t1\",\"live\":true,\"screened\":false,\"target\":\"apps/x.ts\"}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::ToolStart {
                tool_name: "Write".to_owned(),
                tool_id: "t1".to_owned(),
                live: true,
                screened: false,
                target: Some("apps/x.ts".to_owned()),
            }
        );
    }

    #[test]
    fn live_tool_start_no_target_renders_without_target() {
        let theme = Theme::dark();
        let line = "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"bash\",\"tool_id\":\"t5\",\"live\":true}";
        let rendered = render_output_lines(&[line.to_owned()], &theme);
        assert_eq!(rendered.len(), 1);
        let text: String = rendered[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.starts_with('\u{25b8}'), "should use ▸ for live step");
        assert!(text.contains("bash"));
        // No trailing space + empty target
        assert!(!text.ends_with(' '), "should not have trailing space: {text}");
    }

    #[test]
    fn non_live_tool_start_uses_filled_triangle() {
        let theme = Theme::dark();
        let line = "\x1eroko.stream.v1 {\"kind\":\"tool_start\",\"tool_name\":\"grep\",\"tool_id\":\"t6\",\"live\":false}";
        let rendered = render_output_lines(&[line.to_owned()], &theme);
        assert_eq!(rendered.len(), 1);
        let text: String = rendered[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            text.starts_with('\u{25b6}'),
            "non-live step must use ▶: {text}"
        );
    }

    #[test]
    fn unscreened_text_is_dimmed_and_labelled() {
        let theme = Theme::dark();
        let line = "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"secret draft\",\"live\":true,\"screened\":false}";
        let rendered = render_output_lines(&[line.to_owned()], &theme);
        assert_eq!(rendered.len(), 1);
        let text: String = rendered[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            text.contains("[unscreened]"),
            "unscreened text must be labelled: {text}"
        );
        // Must be rendered dim (TEXT_GHOST color)
        assert!(
            rendered[0]
                .spans
                .iter()
                .any(|s| s.style.fg == Some(Theme::TEXT_GHOST)),
            "unscreened text must use TEXT_GHOST color"
        );
    }

    #[test]
    fn screened_text_renders_normally() {
        let theme = Theme::dark();
        // screened defaults to true when absent
        let line = "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"normal output\",\"live\":true,\"screened\":true}";
        let rendered = render_output_lines(&[line.to_owned()], &theme);
        assert_eq!(rendered.len(), 1);
        let text: String = rendered[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            !text.contains("[unscreened]"),
            "screened text must not have unscreened label: {text}"
        );
    }

    #[test]
    fn parse_live_screened_flags() {
        // live=true, screened=false
        let line = "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hi\",\"live\":true,\"screened\":false}";
        let rec = parse_stream_line(line);
        assert_eq!(
            rec,
            StreamRecord::Text {
                content: "hi".to_owned(),
                live: true,
                screened: false,
            }
        );

        // live=false (absent means false)
        let line2 = "\x1eroko.stream.v1 {\"kind\":\"text\",\"content\":\"hi\"}";
        let rec2 = parse_stream_line(line2);
        assert_eq!(
            rec2,
            StreamRecord::Text {
                content: "hi".to_owned(),
                live: false,
                screened: true,
            }
        );
    }
}
