//! F5 Logs view -- scrollable log tail with level-based coloring.
//!
//! Multi-source log view combining:
//! - Signals from `.roko/engrams.jsonl`
//! - Episodes from `.roko/episodes.jsonl`
//! - Efficiency events from `.roko/learn/efficiency.jsonl`
//! - Verify results from signal data
//!
//! Each source is color-coded by type and severity.
//!
//! Features:
//! - Error context lines below failure entries (dimmed, indented)
//! - Grouping modes: chronological (default), by-plan, by-task (toggle with G)
//! - Entry detail expansion on Enter (full payload, ms timestamp, source, IDs)
//! - Error Digest sub-view (sub-tab 2 / Alt+3)
//! - Severity-appropriate styling with source-type icons

use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::input::FocusZone;
use crate::tui::input::LogFilterLevel;
use crate::tui::state::{LogEntry, LogEntryLevel, SearchMode, TuiState};
use crate::tui::util::truncate_middle;

// ---------------------------------------------------------------------------
// Grouping mode
// ---------------------------------------------------------------------------

/// How log entries are organized in the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogGrouping {
    /// Flat chronological list (default).
    #[default]
    Chronological,
    /// Entries grouped under collapsible plan headers.
    ByPlan,
    /// Entries grouped under task headers.
    ByTask,
}

impl LogGrouping {
    /// Cycle to the next grouping mode (used by the G key toggle).
    #[allow(dead_code)]
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Chronological => Self::ByPlan,
            Self::ByPlan => Self::ByTask,
            Self::ByTask => Self::Chronological,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Chronological => "chrono",
            Self::ByPlan => "by-plan",
            Self::ByTask => "by-task",
        }
    }
}

/// Render the full logs view.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    // Sub-tab 2 = Error Digest aggregation panel.
    if view_state.sub_tab == 2 {
        crate::tui::widgets::error_digest::render_error_aggregation_panel(
            frame,
            area,
            tui_state,
            view_state.scroll,
            theme,
        );
        return;
    }

    let all_entries = tui_state.unified_log_entries();

    // Sub-tab 1 ("Signals") shows only signal: and episode: sources.
    // Sub-tab 0 ("Log") shows all entries.
    if view_state.sub_tab == 1 {
        let filtered: Vec<LogEntry> = all_entries
            .iter()
            .filter(|e| e.source.starts_with("signal:") || e.source.starts_with("episode:"))
            .cloned()
            .collect();
        render_with_entries(frame, area, &filtered, _data, tui_state, view_state, theme);
    } else {
        render_with_entries(
            frame,
            area,
            all_entries,
            _data,
            tui_state,
            view_state,
            theme,
        );
    }
}

/// Count visible log entries after applying the active level filter.
pub(crate) fn filtered_entry_count(_data: &DashboardData, tui_state: &TuiState) -> usize {
    tui_state
        .unified_log_entries()
        .iter()
        .filter(|entry| tui_state.log_level_visible(entry.level.filter_level()))
        .count()
}

/// Render the logs view with explicit log entries (for integration layer).
fn render_with_entries(
    frame: &mut Frame<'_>,
    area: Rect,
    entries: &[LogEntry],
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let sections = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);

    // Level filter first.
    let level_filtered: Vec<&LogEntry> = entries
        .iter()
        .filter(|entry| tui_state.log_level_visible(entry.level.filter_level()))
        .collect();

    // Apply search filter (Filter mode excludes non-matches).
    let search = &tui_state.log_search;
    let filtered_entries: Vec<&LogEntry> =
        if search.active && search.mode == SearchMode::Filter && search.compiled.is_some() {
            let re = search.compiled.as_ref().unwrap();
            level_filtered
                .into_iter()
                .filter(|e| re.is_match(&e.message) || re.is_match(&e.source))
                .collect()
        } else {
            level_filtered
        };

    // Resolve grouping mode from TUI state.
    let grouping = tui_state.log_grouping;

    // Status bar with source counts
    let signal_count = tui_state.recent_signals.len();
    let episode_count = tui_state.episodes_cache.len();
    let eff_count = tui_state.efficiency_events.len();
    let gate_count = tui_state.gate_result_summaries.len();
    let event_count = tui_state.event_log.len();

    let (tail_label, tail_style) = if view_state.auto_tail {
        (
            "TAIL",
            Style::default()
                .fg(Theme::SAGE)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("SCROLL", Style::default().fg(Theme::BONE_DIM))
    };
    let focused = matches!(tui_state.focus, FocusZone::RightPanel);
    let entry_label = if filtered_entries.len() == entries.len() {
        format!(" {} entries ", entries.len())
    } else {
        format!(" {}/{} entries ", filtered_entries.len(), entries.len())
    };
    let mut status_spans = vec![
        Span::styled(entry_label, theme.muted()),
        Span::styled(format!("[{tail_label}]"), tail_style),
        Span::styled("  ", theme.muted()),
    ];

    // Show active search match info inline in the status bar.
    if search.active && search.match_count > 0 {
        let current_1 = search.current_match + 1;
        let total = search.match_count;
        status_spans.extend([
            Span::styled(
                format!("[{current_1}"),
                Style::default()
                    .fg(Theme::BONE_BRIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("/{total}]"), theme.muted()),
            Span::styled("  ", theme.muted()),
        ]);
    }

    for (key_idx, level) in LogFilterLevel::all().iter().enumerate() {
        let style = if tui_state.log_level_visible(*level) {
            level_filter_style(*level, theme)
        } else {
            Style::default().fg(Theme::TEXT_PHANTOM)
        };
        // Show key hint next to label: "1:INF 2:WRN 3:ERR 4:DBG"
        status_spans.push(Span::styled(
            format!(" {}:{}", key_idx + 1, level.label()),
            style,
        ));
    }

    // Grouping indicator
    if grouping != LogGrouping::Chronological {
        status_spans.extend([
            Span::styled("  ", theme.muted()),
            Span::styled(
                format!("[{}]", grouping.label()),
                Style::default()
                    .fg(Theme::DREAM)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }

    if sections[0].width >= 104 {
        status_spans.extend([
            Span::styled("  \u{00b7}  ", theme.muted()),
            Span::styled(format!("\u{25c6}{signal_count}"), theme.info()),
            Span::styled("  ", theme.muted()),
            Span::styled(format!("\u{25cf}{episode_count}"), theme.accent()),
            Span::styled("  ", theme.muted()),
            Span::styled(format!("\u{25a0}{gate_count}"), theme.warning()),
            Span::styled("  ", theme.muted()),
            Span::styled(
                format!("\u{25b8}{event_count}"),
                Style::default().fg(Theme::SAGE),
            ),
        ]);
    }
    if sections[0].width >= 160 {
        status_spans.extend([
            Span::styled("  ", theme.muted()),
            Span::styled(format!("\u{2261}{eff_count}"), theme.muted()),
        ]);
    }
    let status_line1 = Line::from(status_spans);
    let status = Paragraph::new(vec![status_line1])
        .alignment(if sections[0].width < 104 {
            Alignment::Left
        } else {
            Alignment::Right
        })
        .style(Style::default().bg(Theme::BG_RAISED));
    frame.render_widget(status, sections[0]);

    // Log content
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::unfocused_border_style()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        Theme::unfocused_title_style()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Logs ", title_style))
        .border_style(border_style);
    let inner = block.inner(sections[1]);
    frame.render_widget(block, sections[1]);

    if filtered_entries.is_empty() {
        let empty_text = if entries.is_empty() {
            "No logs yet \u{00b7} events appear here as the run starts"
        } else if search.active && search.mode == SearchMode::Filter {
            "No logs match this search \u{00b7} press / to change filter"
        } else {
            "No logs match the active levels \u{00b7} press a to show all"
        };
        crate::tui::empty_state::render_pane_empty_compact(frame, inner, empty_text, theme);
        return;
    }

    // Build lines based on grouping mode.
    let lines = match grouping {
        LogGrouping::Chronological => {
            build_chronological_lines(&filtered_entries, view_state, tui_state, inner.width, theme)
        }
        LogGrouping::ByPlan => build_grouped_lines(
            &filtered_entries,
            view_state,
            tui_state,
            inner.width,
            theme,
            GroupBy::Plan,
        ),
        LogGrouping::ByTask => build_grouped_lines(
            &filtered_entries,
            view_state,
            tui_state,
            inner.width,
            theme,
            GroupBy::Task,
        ),
    };

    let row_offsets = wrapped_row_offsets(&lines, inner.width);
    let total_rendered_rows = row_offsets.last().copied().unwrap_or(0);
    let max_scroll = total_rendered_rows.saturating_sub(inner.height as usize);
    let max_scroll = max_scroll.min(u16::MAX as usize) as u16;
    let scroll = if view_state.auto_tail {
        max_scroll
    } else {
        (view_state.scroll).min(max_scroll)
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Line builders
// ---------------------------------------------------------------------------

/// Build chronological lines with context lines under error entries and
/// inline detail expansion for the selected row.
fn build_chronological_lines<'a>(
    filtered_entries: &[&LogEntry],
    view_state: &ViewState,
    tui_state: &TuiState,
    width: u16,
    theme: &Theme,
) -> Vec<Line<'a>> {
    let search = &tui_state.log_search;

    let row_focus_idx = if view_state.auto_tail {
        filtered_entries.len().saturating_sub(1)
    } else {
        (view_state.scroll as usize).min(filtered_entries.len().saturating_sub(1))
    };

    let highlight_style = Style::default().fg(Theme::TEXT_STRONG).bg(Theme::DREAM);
    let current_match_hl_style = Style::default()
        .fg(Theme::BONE_BRIGHT)
        .bg(Theme::DREAM_BRIGHT)
        .add_modifier(Modifier::BOLD);

    let current_match_filtered_idx = if search.active && !search.match_indices.is_empty() {
        if search.mode == SearchMode::Filter {
            Some(
                search
                    .current_match
                    .min(filtered_entries.len().saturating_sub(1)),
            )
        } else {
            search.match_indices.get(search.current_match).copied()
        }
    } else {
        None
    };

    let source_width = match width {
        0..=63 => 10,
        64..=99 => 14,
        100..=139 => 20,
        _ => 28,
    };
    let show_timestamp = width >= 58;
    let expanded_idx = tui_state.log_expanded_idx;

    let mut lines: Vec<Line<'a>> = Vec::new();
    for (idx, entry) in filtered_entries.iter().enumerate() {
        let selected = idx == row_focus_idx;
        let is_current_match = current_match_filtered_idx == Some(idx);
        let row_bg = if is_current_match {
            Some(Theme::DREAM_DEEP)
        } else if selected {
            Some(theme.selection_background)
        } else {
            None
        };

        lines.push(build_entry_line(
            entry,
            selected,
            is_current_match,
            row_bg,
            source_width,
            show_timestamp,
            search,
            &highlight_style,
            &current_match_hl_style,
            theme,
        ));

        // Error context line: show detail below failure entries.
        if matches!(entry.level, LogEntryLevel::Error | LogEntryLevel::Warn) {
            if let Some(ctx) = extract_error_context(entry) {
                let dim_style = Style::default()
                    .fg(Theme::TEXT_GHOST)
                    .add_modifier(Modifier::ITALIC);
                let ctx_text = if ctx.len() > 100 { &ctx[..100] } else { &ctx };
                lines.push(Line::from(vec![
                    Span::styled("    ", dim_style),
                    Span::styled(ctx_text.to_owned(), dim_style),
                ]));
            }
        }

        // Inline detail expansion for the selected row when Enter was pressed.
        if expanded_idx == Some(idx) {
            let detail_lines = build_detail_expansion(entry, width, theme);
            lines.extend(detail_lines);
        }
    }

    lines
}

/// Build a single entry line with all styling applied.
fn build_entry_line<'a>(
    entry: &LogEntry,
    selected: bool,
    is_current_match: bool,
    row_bg: Option<ratatui::style::Color>,
    source_width: usize,
    show_timestamp: bool,
    search: &crate::tui::state::LogSearchState,
    highlight_style: &Style,
    current_match_hl_style: &Style,
    theme: &Theme,
) -> Line<'a> {
    let prefix_style = if selected {
        theme.selection()
    } else {
        theme.muted()
    };
    let badge_style = style_with_bg(level_badge_style(entry.level, theme), row_bg);
    let src_style = style_with_bg(source_style(&entry.source, theme), row_bg);
    let message_style = style_with_bg(level_message_style(entry.level, theme), row_bg);
    let ts_style = style_with_bg(theme.label(), row_bg);
    let icon_style = style_with_bg(source_style(&entry.source, theme), row_bg);

    let match_hl = if is_current_match {
        *current_match_hl_style
    } else {
        *highlight_style
    };

    let message_spans: Vec<Span<'a>> = if search.active && search.compiled.is_some() {
        let re = search.compiled.as_ref().unwrap();
        highlight_spans(&entry.message, re, message_style, match_hl)
    } else {
        vec![Span::styled(entry.message.clone(), message_style)]
    };

    let icon = source_icon(&entry.source);
    let mut spans = vec![
        Span::styled(if selected { "\u{25b6}" } else { " " }, prefix_style),
        Span::styled(icon, icon_style),
    ];
    if show_timestamp {
        spans.push(Span::styled(entry.timestamp.clone(), ts_style));
        spans.push(Span::raw(" "));
    }
    spans.extend([
        Span::styled(format!("[{}]", entry.level.label()), badge_style),
        Span::raw(" "),
        Span::styled(truncate_middle(&entry.source, source_width), src_style),
        Span::raw(" \u{00b7} "),
    ]);
    spans.extend(message_spans);
    Line::from(spans)
}

// ---------------------------------------------------------------------------
// Grouped rendering (by-plan / by-task)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum GroupBy {
    Plan,
    Task,
}

/// Extract a grouping key from a log entry.
fn group_key(entry: &LogEntry, by: GroupBy) -> String {
    match by {
        GroupBy::Plan => {
            // Try to find plan= or plan_id in the message.
            if let Some(pos) = entry.message.find("plan=") {
                let rest = &entry.message[pos + 5..];
                rest.split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string()
            } else if entry.source.contains(':') {
                // Fall back to the source prefix as group.
                entry
                    .source
                    .split(':')
                    .next()
                    .unwrap_or("other")
                    .to_string()
            } else {
                "ungrouped".to_string()
            }
        }
        GroupBy::Task => {
            if let Some(pos) = entry.message.find("task=") {
                let rest = &entry.message[pos + 5..];
                rest.split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                "ungrouped".to_string()
            }
        }
    }
}

/// Build lines with entries grouped under collapsible headers.
fn build_grouped_lines<'a>(
    filtered_entries: &[&LogEntry],
    _view_state: &ViewState,
    tui_state: &TuiState,
    width: u16,
    theme: &Theme,
    by: GroupBy,
) -> Vec<Line<'a>> {
    let search = &tui_state.log_search;
    let source_width = match width {
        0..=63 => 10,
        64..=99 => 14,
        100..=139 => 20,
        _ => 28,
    };
    let show_timestamp = width >= 58;
    let highlight_style = Style::default().fg(Theme::TEXT_STRONG).bg(Theme::DREAM);
    let current_match_hl_style = Style::default()
        .fg(Theme::BONE_BRIGHT)
        .bg(Theme::DREAM_BRIGHT)
        .add_modifier(Modifier::BOLD);
    let expanded_idx = tui_state.log_expanded_idx;

    // Collect entries into ordered groups preserving first-seen order.
    let mut groups: BTreeMap<String, Vec<(usize, &LogEntry)>> = BTreeMap::new();
    for (idx, entry) in filtered_entries.iter().enumerate() {
        let key = group_key(entry, by);
        groups.entry(key).or_default().push((idx, entry));
    }

    let mut lines: Vec<Line<'a>> = Vec::new();
    let collapsed = &tui_state.log_collapsed_groups;

    for (group_name, group_entries) in &groups {
        let is_collapsed = collapsed.contains(group_name);
        let arrow = if is_collapsed { "\u{25b6}" } else { "\u{25bc}" };
        let err_count = group_entries
            .iter()
            .filter(|(_, e)| matches!(e.level, LogEntryLevel::Error))
            .count();
        let mut header_spans = vec![
            Span::styled(format!("{arrow} {group_name}"), theme.section_header()),
            Span::styled(format!(" ({} entries", group_entries.len()), theme.muted()),
        ];
        if err_count > 0 {
            header_spans.push(Span::styled(
                format!(", {err_count} errors"),
                theme.danger(),
            ));
        }
        header_spans.push(Span::styled(")", theme.muted()));
        lines.push(Line::from(header_spans));

        if is_collapsed {
            continue;
        }

        for (idx, entry) in group_entries {
            lines.push(build_entry_line(
                entry,
                false,
                false,
                None,
                source_width,
                show_timestamp,
                search,
                &highlight_style,
                &current_match_hl_style,
                theme,
            ));

            // Error context for grouped entries too.
            if matches!(entry.level, LogEntryLevel::Error | LogEntryLevel::Warn) {
                if let Some(ctx) = extract_error_context(entry) {
                    let dim_style = Style::default()
                        .fg(Theme::TEXT_GHOST)
                        .add_modifier(Modifier::ITALIC);
                    let ctx_text = if ctx.len() > 100 { &ctx[..100] } else { &ctx };
                    lines.push(Line::from(vec![
                        Span::styled("    ", dim_style),
                        Span::styled(ctx_text.to_owned(), dim_style),
                    ]));
                }
            }

            if expanded_idx == Some(*idx) {
                let detail_lines = build_detail_expansion(entry, width, theme);
                lines.extend(detail_lines);
            }
        }

        // Spacer between groups.
        lines.push(Line::from(""));
    }

    lines
}

// ---------------------------------------------------------------------------
// Error context extraction
// ---------------------------------------------------------------------------

/// Extract a short context string from error/warn entries for display below
/// the main log line.
fn extract_error_context(entry: &LogEntry) -> Option<String> {
    let msg = &entry.message;

    // Gate failures: show the excerpt after "FAILED".
    if entry.source.starts_with("gate:") && msg.contains("FAILED") {
        if let Some(pos) = msg.find("FAILED") {
            let rest = msg[pos + 6..].trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }

    // Episode failures: extract the failure reason.
    if entry.source.starts_with("episode:") && msg.contains("[FAIL]") {
        if let Some(pos) = msg.find("task=") {
            let rest = msg[pos..].trim();
            return Some(rest.to_string());
        }
    }

    // Event errors: the message itself is the context.
    if entry.source.starts_with("event:")
        && matches!(entry.level, LogEntryLevel::Error)
        && msg.len() > 20
    {
        // Already shown in main line; extract bracketed task if present.
        if msg.starts_with('[') {
            if let Some(end) = msg.find(']') {
                let detail = msg[end + 1..].trim();
                if !detail.is_empty() && detail.len() > 10 {
                    return Some(detail.to_string());
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Detail expansion (Enter key)
// ---------------------------------------------------------------------------

/// Build detail expansion lines for a log entry when Enter is pressed.
fn build_detail_expansion<'a>(entry: &LogEntry, _width: u16, theme: &Theme) -> Vec<Line<'a>> {
    let dim = Style::default()
        .fg(Theme::TEXT_DIM)
        .add_modifier(Modifier::ITALIC);
    let label_style = theme.label();
    let value_style = theme.value();

    let mut lines = Vec::new();

    // Separator.
    lines.push(Line::from(Span::styled(
        "    \u{2500}\u{2500}\u{2500} expanded detail \u{2500}\u{2500}\u{2500}",
        dim,
    )));

    // Timestamp with full precision.
    lines.push(Line::from(vec![
        Span::styled("    timestamp: ", label_style),
        Span::styled(entry.timestamp.clone(), value_style),
    ]));

    // Source component.
    lines.push(Line::from(vec![
        Span::styled("    source:    ", label_style),
        Span::styled(entry.source.clone(), value_style),
    ]));

    // Severity.
    lines.push(Line::from(vec![
        Span::styled("    severity:  ", label_style),
        Span::styled(
            entry.level.label().to_string(),
            level_badge_style(entry.level, theme),
        ),
    ]));

    // Extract IDs from message.
    let msg = &entry.message;
    if let Some(pos) = msg.find("task=") {
        let rest = &msg[pos + 5..];
        let task_id = rest.split_whitespace().next().unwrap_or("");
        lines.push(Line::from(vec![
            Span::styled("    task:      ", label_style),
            Span::styled(task_id.to_string(), value_style),
        ]));
    }
    if let Some(pos) = msg.find("plan=") {
        let rest = &msg[pos + 5..];
        let plan_id = rest.split_whitespace().next().unwrap_or("");
        lines.push(Line::from(vec![
            Span::styled("    plan:      ", label_style),
            Span::styled(plan_id.to_string(), value_style),
        ]));
    }
    if let Some(pos) = msg.find("model=") {
        let rest = &msg[pos + 6..];
        let model = rest.split_whitespace().next().unwrap_or("");
        lines.push(Line::from(vec![
            Span::styled("    model:     ", label_style),
            Span::styled(model.to_string(), value_style),
        ]));
    }

    // Full message payload.
    lines.push(Line::from(vec![
        Span::styled("    message:   ", label_style),
        Span::styled(msg.clone(), Style::default().fg(Theme::BONE)),
    ]));

    // End separator.
    lines.push(Line::from(Span::styled(
        "    \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
        dim,
    )));

    lines
}

/// Return the rendered-row offset of every logical line plus one terminal
/// sentinel containing the total rendered height. Ratatui scroll offsets are
/// measured after wrapping, while keyboard/search selection is kept in logical
/// log rows.
fn wrapped_row_offsets(lines: &[Line<'_>], width: u16) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(lines.len() + 1);
    let mut rendered = 0usize;
    for line in lines {
        offsets.push(rendered);
        rendered = rendered.saturating_add(
            Paragraph::new(line.clone())
                .wrap(Wrap { trim: false })
                .line_count(width)
                .max(1),
        );
    }
    offsets.push(rendered);
    offsets
}

// ---------------------------------------------------------------------------
// Severity styling
// ---------------------------------------------------------------------------

/// Message body style for log levels -- uses severity-appropriate colors.
///
/// - INFO: default text (no emphasis)
/// - WARN: WARNING color (amber)
/// - ERR: EMBER color (red) + bold
/// - DEBUG: TEXT_DIM (ghost)
fn level_message_style(level: LogEntryLevel, theme: &Theme) -> Style {
    match level {
        LogEntryLevel::Debug => Style::default().fg(Theme::TEXT_DIM),
        LogEntryLevel::Info => theme.muted(),
        LogEntryLevel::Warn => Style::default().fg(Theme::WARNING),
        LogEntryLevel::Error => Style::default()
            .fg(Theme::EMBER)
            .add_modifier(Modifier::BOLD),
    }
}

/// Distinct badge style for the `[LVL]` bracket -- slightly stronger than the
/// message body style so the level tag stands out from the message text.
fn level_badge_style(level: LogEntryLevel, theme: &Theme) -> Style {
    match level {
        LogEntryLevel::Debug => Style::default()
            .fg(Theme::TEXT_GHOST)
            .add_modifier(Modifier::ITALIC),
        LogEntryLevel::Info => theme.text(),
        LogEntryLevel::Warn => Style::default()
            .fg(Theme::WARNING)
            .add_modifier(Modifier::BOLD),
        LogEntryLevel::Error => Style::default()
            .fg(Theme::EMBER)
            .add_modifier(Modifier::BOLD),
    }
}

fn level_filter_style(level: LogFilterLevel, theme: &Theme) -> Style {
    match level {
        LogFilterLevel::Debug => theme.muted(),
        LogFilterLevel::Info => theme.text(),
        LogFilterLevel::Warn => theme.warning(),
        LogFilterLevel::Error => theme.danger(),
    }
}

/// Color style for log sources.
fn source_style(source: &str, theme: &Theme) -> Style {
    if source.starts_with("signal:") {
        theme.info()
    } else if source.starts_with("episode:") {
        theme.accent()
    } else if source.starts_with("gate:") {
        theme.warning()
    } else if source.starts_with("efficiency:") {
        theme.muted()
    } else if source.starts_with("event:") {
        // Distinct from signal: (DREAM) -- use SAGE so events are visually separable.
        Style::default().fg(Theme::SAGE)
    } else {
        theme.text()
    }
}

/// Return a type-indicator icon for the log source prefix.
///
/// Icons: `\u{25c6}`(signal) `\u{25cf}`(episode) `\u{25a0}`(gate) `\u{25b8}`(event)
fn source_icon(source: &str) -> &'static str {
    if source.starts_with("signal:") {
        "\u{25c6}" // diamond
    } else if source.starts_with("episode:") {
        "\u{25cf}" // filled circle
    } else if source.starts_with("gate:") {
        "\u{25a0}" // filled square
    } else if source.starts_with("efficiency:") {
        "\u{2261}" // triple bar
    } else if source.starts_with("event:") {
        "\u{25b8}" // right-pointing triangle
    } else {
        "\u{00b7}" // middle dot
    }
}

fn style_with_bg(style: Style, bg: Option<ratatui::style::Color>) -> Style {
    if let Some(bg) = bg {
        style.bg(bg)
    } else {
        style
    }
}

/// Split `text` into spans, highlighting regex matches with `hl_style`.
fn highlight_spans<'a>(
    text: &str,
    re: &regex::Regex,
    normal_style: Style,
    hl_style: Style,
) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let mut last_end = 0;
    for m in re.find_iter(text) {
        if m.start() > last_end {
            spans.push(Span::styled(
                text[last_end..m.start()].to_owned(),
                normal_style,
            ));
        }
        spans.push(Span::styled(text[m.start()..m.end()].to_owned(), hl_style));
        last_end = m.end();
    }
    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_owned(), normal_style));
    }
    if spans.is_empty() {
        spans.push(Span::styled(text.to_owned(), normal_style));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn entries() -> Vec<LogEntry> {
        (0..24)
            .map(|idx| {
                let marker = if idx == 3 {
                    " CURRENT_MARKER"
                } else if idx == 23 {
                    " LATEST_MARKER"
                } else {
                    ""
                };
                LogEntry::new(
                    format!("12:34:{idx:02}"),
                    if idx % 7 == 0 {
                        LogEntryLevel::Warn
                    } else {
                        LogEntryLevel::Info
                    },
                    "event:agent.dispatch.with-a-long-source".to_string(),
                    format!(
                        "entry {idx:02} {}{marker}",
                        "a deliberately wrapped diagnostic message ".repeat(5)
                    ),
                )
            })
            .collect()
    }

    fn render_entries_at(width: u16, height: u16, view_state: &ViewState) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let data = DashboardData::default();
        let state = TuiState::default();
        let theme = Theme::dark();
        let entries = entries();
        terminal
            .draw(|frame| {
                render_with_entries(
                    frame,
                    frame.area(),
                    &entries,
                    &data,
                    &state,
                    view_state,
                    &theme,
                );
            })
            .unwrap();
        rendered_text(&terminal)
    }

    #[test]
    fn logs_auto_tail_counts_wrapped_rows_at_common_sizes() {
        let view_state = ViewState {
            auto_tail: true,
            ..ViewState::default()
        };
        for (width, height) in [(80, 24), (120, 40), (200, 60)] {
            let rendered = render_entries_at(width, height, &view_state);
            assert!(
                rendered.contains("LATEST_MARKER"),
                "wrapped log tail missing at {width}x{height}:\n{rendered}"
            );
            assert!(rendered.contains("[TAIL]"));
        }
    }

    #[test]
    fn manual_log_row_selection_maps_to_wrapped_offset() {
        let view_state = ViewState {
            scroll: 3,
            auto_tail: false,
            ..ViewState::default()
        };
        let rendered = render_entries_at(80, 24, &view_state);
        assert!(
            rendered.contains("CURRENT_MARKER"),
            "selected logical row should remain visible after wrapping:\n{rendered}"
        );
        assert!(rendered.contains("[SCROLL]"));
    }

    #[test]
    fn wrapped_offsets_include_terminal_total() {
        let lines = vec![
            Line::from("short"),
            Line::from("this line wraps across several rendered rows"),
        ];
        let offsets = wrapped_row_offsets(&lines, 12);
        assert_eq!(offsets[0], 0);
        assert_eq!(offsets[1], 1);
        assert!(offsets[2] > 2);
    }

    #[test]
    fn source_icons_are_distinct() {
        let icons: Vec<&str> = [
            "signal:gate",
            "episode:run",
            "gate:compile",
            "efficiency:x",
            "event:bus",
            "other:unknown",
        ]
        .iter()
        .map(|s| source_icon(s))
        .collect();
        // All source-type icons should be unique.
        let mut deduped = icons.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(deduped.len(), icons.len(), "source icons must be distinct");
    }

    #[test]
    fn level_badge_info_stronger_than_body() {
        let theme = Theme::dark();
        let badge = level_badge_style(LogEntryLevel::Info, &theme);
        let body = level_message_style(LogEntryLevel::Info, &theme);
        // Badge uses theme.text() (TEXT), body uses theme.muted() (TEXT_DIM).
        assert_ne!(badge, body, "INFO badge should be brighter than body text");
    }

    #[test]
    fn error_context_extraction() {
        let gate_entry = LogEntry::new(
            "12:00:00".into(),
            LogEntryLevel::Error,
            "gate:compile".into(),
            "FAILED task=t1 cannot find crate `foo`".into(),
        );
        let ctx = extract_error_context(&gate_entry);
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("task=t1"));

        let info_entry = LogEntry::new(
            "12:00:01".into(),
            LogEntryLevel::Info,
            "event:dispatch".into(),
            "all good".into(),
        );
        assert!(extract_error_context(&info_entry).is_none());
    }

    #[test]
    fn grouping_mode_cycles() {
        assert_eq!(LogGrouping::Chronological.next(), LogGrouping::ByPlan);
        assert_eq!(LogGrouping::ByPlan.next(), LogGrouping::ByTask);
        assert_eq!(LogGrouping::ByTask.next(), LogGrouping::Chronological);
    }

    #[test]
    fn severity_styles_are_distinct() {
        let theme = Theme::dark();
        let info = level_message_style(LogEntryLevel::Info, &theme);
        let warn = level_message_style(LogEntryLevel::Warn, &theme);
        let err = level_message_style(LogEntryLevel::Error, &theme);
        assert_ne!(info, warn, "INFO vs WARN should differ");
        assert_ne!(warn, err, "WARN vs ERR should differ");
    }

    #[test]
    fn detail_expansion_builds_lines() {
        let theme = Theme::dark();
        let entry = LogEntry::new(
            "12:34:56".into(),
            LogEntryLevel::Error,
            "gate:compile".into(),
            "FAILED task=do-thing model=claude-3 plan=my-plan some error output".into(),
        );
        let lines = build_detail_expansion(&entry, 120, &theme);
        assert!(
            lines.len() >= 5,
            "detail expansion should have multiple lines"
        );
        // Check that IDs were extracted.
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text.contains("do-thing"), "should extract task ID");
        assert!(text.contains("claude-3"), "should extract model");
        assert!(text.contains("my-plan"), "should extract plan ID");
    }
}
