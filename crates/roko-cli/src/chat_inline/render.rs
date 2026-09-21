//! Viewport rendering: input area, completion dropdown, command palette,
//! thinking spinner, streaming, error, status bar.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use super::session::thinking_label;
use super::types::{ChatSession, Phase};
use crate::inline::styled;
use crate::inline::symbols;
use crate::tui::Theme;

// ---------------------------------------------------------------------------
// Top-level viewport
// ---------------------------------------------------------------------------

pub(crate) fn render_viewport(frame: &mut Frame<'_>, session: &ChatSession, theme: &Theme) {
    let area = frame.area();

    match session.phase {
        Phase::Input => render_input(frame, area, session, theme),
        Phase::Thinking => {
            let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

            let elapsed = session
                .thinking_started
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            let label = thinking_label(elapsed);
            let spinner = styled::spinner_line(theme, session.tick, label, elapsed);
            frame.render_widget(Paragraph::new(spinner), chunks[0]);
            render_status_bar(frame, chunks[1], session, theme);
        }
        Phase::Streaming => {
            session.streaming.render(frame, area, theme);
        }
        Phase::Error { ref error, .. } => {
            let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
            let hint = Line::from(vec![
                Span::styled(
                    format!("  {} ", symbols::WARN),
                    Style::default().fg(Theme::EMBER),
                ),
                Span::styled(
                    super::session::truncate_str(error, area.width as usize - 6),
                    Style::default().fg(Theme::EMBER),
                ),
                Span::raw("  "),
                Span::styled(
                    "[r]",
                    Style::default()
                        .fg(Theme::BONE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("etry  ", theme.muted()),
                Span::styled(
                    "[q]",
                    Style::default()
                        .fg(Theme::BONE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("uit", theme.muted()),
            ]);
            frame.render_widget(Paragraph::new(hint), chunks[0]);
            render_status_bar(frame, chunks[1], session, theme);
        }
        Phase::Done => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    "bye.".to_string(),
                    theme.muted(),
                )])),
                area,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Input area
// ---------------------------------------------------------------------------

fn render_input(frame: &mut Frame<'_>, area: Rect, session: &ChatSession, theme: &Theme) {
    // --- Command palette overlay ---
    if session.input.palette.active {
        render_palette(frame, area, session, theme);
        return;
    }

    let dropdown_visible = session.input.completion.visible;
    let match_count = session.input.completion.matches.len();

    // Build the input paragraph first so we can ask ratatui for the exact
    // wrapped line count.
    let input_paragraph = build_input_paragraph(session, theme, dropdown_visible);
    let input_lines = input_paragraph.line_count(area.width).max(1);
    let max_input_height = (area.height as usize).saturating_sub(2);
    let input_height = input_lines.min(6).min(max_input_height).max(1) as u16;

    // Compute dropdown height: min(match_count, 8, viewport - input - 2)
    let dropdown_height = if dropdown_visible {
        let max_rows = (area.height as usize).saturating_sub(input_height as usize + 2);
        match_count.min(8).min(max_rows) as u16
    } else {
        0
    };

    let chunks = if dropdown_height > 0 {
        Layout::vertical([
            Constraint::Min(1),                  // upper spacer
            Constraint::Length(dropdown_height), // dropdown
            Constraint::Length(input_height),    // input area
            Constraint::Length(1),               // status bar
        ])
        .split(area)
    } else {
        // No dropdown -- 3-zone layout (pad to 4 for uniform indexing)
        let base = Layout::vertical([
            Constraint::Min(1),               // spacer
            Constraint::Length(input_height), // input area
            Constraint::Length(1),            // status bar
        ])
        .split(area);
        vec![base[0], Rect::default(), base[1], base[2]].into()
    };

    // Dropdown rendering
    if dropdown_height > 0 {
        let dropdown_area = chunks[1];
        let selected = session.input.completion.selected;
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(dropdown_height as usize);

        for (i, m) in session
            .input
            .completion
            .matches
            .iter()
            .take(dropdown_height as usize)
            .enumerate()
        {
            let is_selected = i == selected;
            let base_style = if is_selected {
                theme.selection()
            } else {
                theme.text()
            };
            let dim_style = if is_selected {
                theme.selection()
            } else {
                theme.muted()
            };

            // Build command span with highlighted matched chars
            let prefix = if is_selected { "> " } else { "  " };
            let mut spans: Vec<Span<'static>> = vec![Span::styled(prefix.to_string(), base_style)];

            // Render command with fuzzy-match highlights
            let cmd_chars: Vec<char> = m.command.chars().collect();
            let highlight_style = Style::default()
                .fg(Theme::BONE)
                .add_modifier(Modifier::BOLD);
            let mut ci = 0;
            let mut run_start = 0;
            let matched_set: std::collections::HashSet<usize> =
                m.matched_indices.iter().copied().collect();

            while ci < cmd_chars.len() {
                if matched_set.contains(&ci) {
                    // Flush non-highlighted run
                    if run_start < ci {
                        let s: String = cmd_chars[run_start..ci].iter().collect();
                        spans.push(Span::styled(s, base_style));
                    }
                    // Highlighted char
                    spans.push(Span::styled(
                        cmd_chars[ci].to_string(),
                        if is_selected {
                            highlight_style.bg(theme.selection_background)
                        } else {
                            highlight_style
                        },
                    ));
                    run_start = ci + 1;
                }
                ci += 1;
            }
            if run_start < cmd_chars.len() {
                let s: String = cmd_chars[run_start..].iter().collect();
                spans.push(Span::styled(s, base_style));
            }

            // Pad and add description
            let cmd_width = m.command.len() + 2; // prefix + command
            let pad = if cmd_width < 14 { 14 - cmd_width } else { 2 };
            spans.push(Span::styled(" ".repeat(pad), dim_style));
            spans.push(Span::styled(m.description.to_string(), dim_style));

            lines.push(Line::from(spans));
        }

        frame.render_widget(Paragraph::new(lines), dropdown_area);
    }

    // Render the pre-built input paragraph into the input area.
    frame.render_widget(input_paragraph, chunks[2]);

    render_status_bar(frame, chunks[3], session, theme);
}

// ---------------------------------------------------------------------------
// Input paragraph builder
// ---------------------------------------------------------------------------

/// Build the input paragraph for height measurement and rendering.
///
/// This produces a single `Paragraph` with `.wrap(Wrap { trim: false })` that
/// covers all three input modes (reverse search, single-line, multi-line).
/// The caller uses `paragraph.line_count(width)` to allocate the correct
/// vertical space before rendering.
fn build_input_paragraph<'a>(
    session: &ChatSession,
    theme: &'a Theme,
    dropdown_visible: bool,
) -> Paragraph<'a> {
    let prompt_style = Style::default()
        .fg(Theme::ROSE)
        .add_modifier(Modifier::BOLD);
    let cont_style = Style::default().fg(Theme::TEXT_DIM);
    let cursor_style = Style::default()
        .fg(Theme::BONE)
        .add_modifier(Modifier::REVERSED);

    let paragraph = if session.input.search.active {
        // Reverse search prompt: (reverse-i-search) 'query': matched_text
        let query = &session.input.search.query;
        let match_info = if session.input.search.matches.is_empty() && !query.is_empty() {
            "no match"
        } else {
            ""
        };
        let matched_text = session
            .input
            .search
            .current_match(&session.input.history)
            .unwrap_or("");
        let mut spans = vec![
            Span::styled(
                "(reverse-i-search) ".to_string(),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::styled("'".to_string(), Style::default().fg(Theme::TEXT_GHOST)),
            Span::styled(
                query.to_string(),
                Style::default()
                    .fg(Theme::ROSE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("'".to_string(), Style::default().fg(Theme::TEXT_GHOST)),
            Span::styled(": ".to_string(), Style::default().fg(Theme::TEXT_DIM)),
        ];
        if match_info.is_empty() {
            spans.push(Span::styled(matched_text.to_string(), theme.text()));
        } else {
            spans.push(Span::styled(
                match_info.to_string(),
                Style::default().fg(Theme::EMBER),
            ));
        }
        Paragraph::new(Line::from(spans))
    } else if !session.input.buffer.contains('\n') {
        // Single-line input with cursor rendering
        let before_cursor = &session.input.buffer[..session.input.cursor];
        let after_cursor = &session.input.buffer[session.input.cursor..];

        let mut input_spans = vec![
            Span::styled(format!("{} ", symbols::PROMPT), prompt_style),
            Span::styled(before_cursor.to_string(), theme.text()),
            Span::styled(
                if after_cursor.is_empty() {
                    symbols::CURSOR.to_string()
                } else {
                    after_cursor.chars().next().unwrap_or(' ').to_string()
                },
                cursor_style,
            ),
            Span::styled(
                if after_cursor.len() > 1 {
                    after_cursor[after_cursor
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1)..]
                        .to_string()
                } else {
                    String::new()
                },
                theme.text(),
            ),
        ];

        // Ghost text -- only when dropdown is NOT visible and cursor is at end
        if !dropdown_visible {
            if let Some(ghost) = session.input.ghost_suggestion() {
                input_spans.push(Span::styled(
                    ghost.to_string(),
                    Style::default().fg(Theme::TEXT_GHOST),
                ));
            }
        }

        Paragraph::new(Line::from(input_spans))
    } else {
        // Multi-line rendering
        let (cursor_line, cursor_col) = session.input.cursor_line_col();
        let buf_lines: Vec<&str> = session.input.buffer.split('\n').collect();
        let line_count_label = format!("[{} lines]", buf_lines.len());

        let mut rendered_lines: Vec<Line<'static>> = Vec::new();
        for (i, line_text) in buf_lines.iter().enumerate() {
            let prefix = if i == 0 {
                Span::styled(format!("{} ", symbols::PROMPT), prompt_style)
            } else {
                Span::styled(format!("{} ", symbols::BAR), cont_style)
            };

            if i == cursor_line {
                let before = &line_text[..cursor_col.min(line_text.len())];
                let at_end = cursor_col >= line_text.len();
                let cursor_char = if at_end {
                    symbols::CURSOR.to_string()
                } else {
                    line_text[cursor_col..]
                        .chars()
                        .next()
                        .unwrap_or(' ')
                        .to_string()
                };
                let after = if at_end || cursor_col + 1 >= line_text.len() {
                    String::new()
                } else {
                    line_text[cursor_col
                        + line_text[cursor_col..]
                            .chars()
                            .next()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1)..]
                        .to_string()
                };

                let mut spans = vec![
                    prefix,
                    Span::styled(before.to_string(), theme.text()),
                    Span::styled(cursor_char, cursor_style),
                    Span::styled(after, theme.text()),
                ];
                if i == 0 {
                    spans.push(Span::styled(
                        format!("  {line_count_label}"),
                        Style::default().fg(Theme::TEXT_GHOST),
                    ));
                }
                rendered_lines.push(Line::from(spans));
            } else {
                let mut spans = vec![prefix, Span::styled(line_text.to_string(), theme.text())];
                if i == 0 {
                    spans.push(Span::styled(
                        format!("  {line_count_label}"),
                        Style::default().fg(Theme::TEXT_GHOST),
                    ));
                }
                rendered_lines.push(Line::from(spans));
            }
        }

        Paragraph::new(rendered_lines)
    };

    paragraph.wrap(Wrap { trim: false })
}

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

fn render_palette(frame: &mut Frame<'_>, area: Rect, session: &ChatSession, theme: &Theme) {
    let palette = &session.input.palette;
    let max_visible = 10.min(area.height.saturating_sub(3) as usize);
    let visible_matches = palette.matches.len().min(max_visible);
    let palette_height = (visible_matches as u16 + 2).min(area.height.saturating_sub(1));

    let chunks = Layout::vertical([
        Constraint::Min(1),                 // spacer
        Constraint::Length(palette_height), // palette body
        Constraint::Length(1),              // status bar
    ])
    .split(area);

    let palette_area = chunks[1];

    // Search bar
    let search_line = Line::from(vec![
        Span::styled(
            format!("{} ", symbols::PROMPT),
            Style::default()
                .fg(Theme::ROSE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if palette.query.is_empty() {
                "type to filter...".to_string()
            } else {
                palette.query.clone()
            },
            if palette.query.is_empty() {
                Style::default().fg(Theme::TEXT_GHOST)
            } else {
                Style::default().fg(Theme::BONE)
            },
        ),
        Span::styled(
            symbols::CURSOR.to_string(),
            Style::default()
                .fg(Theme::BONE)
                .add_modifier(Modifier::REVERSED),
        ),
    ]);

    let mut lines: Vec<Line<'static>> = vec![search_line];

    // Scrolling: center selected item in view
    let scroll_offset = if palette.selected >= max_visible {
        palette.selected - max_visible + 1
    } else {
        0
    };

    for (i, m) in palette
        .matches
        .iter()
        .skip(scroll_offset)
        .take(max_visible)
        .enumerate()
    {
        let actual_idx = scroll_offset + i;
        let is_selected = actual_idx == palette.selected;
        let base_style = if is_selected {
            theme.selection()
        } else {
            theme.text()
        };
        let dim_style = if is_selected {
            theme.selection()
        } else {
            theme.muted()
        };

        let prefix = if is_selected { "> " } else { "  " };
        let cmd_width = m.command.len() + 2;
        let pad = if cmd_width < 20 { 20 - cmd_width } else { 2 };

        lines.push(Line::from(vec![
            Span::styled(prefix.to_string(), base_style),
            Span::styled(m.command.to_string(), base_style),
            Span::styled(" ".repeat(pad), dim_style),
            Span::styled(m.description.to_string(), dim_style),
        ]));
    }

    // Hint line if there are more matches
    if palette.matches.len() > max_visible {
        let remaining = palette.matches.len() - max_visible;
        lines.push(Line::from(vec![Span::styled(
            format!("  ... {remaining} more"),
            Style::default().fg(Theme::TEXT_GHOST),
        )]));
    }

    frame.render_widget(Paragraph::new(lines), palette_area);
    render_status_bar(frame, chunks[2], session, theme);
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn render_status_bar(frame: &mut Frame<'_>, area: Rect, session: &ChatSession, theme: &Theme) {
    let model = session.cost.primary_model().unwrap_or("—").to_string();

    // Build the base status bar
    let mut spans = vec![
        Span::styled(
            format!("${:.4}", session.cost.total_cost.max(0.0)),
            Style::default().fg(Theme::SAGE),
        ),
        Span::styled(format!("  {}  ", symbols::SEP), theme.muted()),
        Span::styled(
            format!(
                "{} in / {} out",
                session.cost.input_tokens, session.cost.output_tokens
            ),
            Style::default().fg(Theme::TEXT_DIM),
        ),
        Span::styled(format!("  {}  ", symbols::SEP), theme.muted()),
        Span::styled(model, theme.info()),
    ];

    // Turn counter
    if session.turn_count > 0 {
        spans.push(Span::styled(format!("  {}  ", symbols::SEP), theme.muted()));
        spans.push(Span::styled(
            format!("turn {}", session.turn_count),
            Style::default().fg(Theme::TEXT_DIM),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
