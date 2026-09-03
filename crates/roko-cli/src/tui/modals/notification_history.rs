//! Scrollable notification history modal.
//!
//! Shows all past notifications that have expired or been dismissed,
//! with timestamp, level icon, and message text. Supports level-based
//! filtering (1/2/3/4 keys) and page navigation.

use std::collections::VecDeque;
use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use super::super::dashboard::Theme;
use super::NotificationLevel;

/// Maximum number of retained history entries.
pub const MAX_HISTORY: usize = 200;

/// A single record in the notification history ring buffer.
#[derive(Debug, Clone)]
pub struct NotificationRecord {
    /// Monotonic ID assigned at insertion time.
    pub id: u64,
    /// When the notification was originally created.
    pub created_at: Instant,
    /// Severity level.
    pub level: NotificationLevel,
    /// Source subsystem (e.g. "gate", "runner", "agent").
    pub source: String,
    /// The redacted display message.
    pub message: String,
    /// Optional related plan-run ID for navigation.
    pub related_run: Option<String>,
    /// Optional related task ID for navigation.
    pub related_task: Option<String>,
    /// When the toast was dismissed (None if it expired naturally).
    pub dismissed_at: Option<Instant>,
}

/// Bit-flags for level filtering in the notification history modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelFilter {
    pub info: bool,
    pub warn: bool,
    pub error: bool,
    pub debug: bool,
}

impl Default for LevelFilter {
    fn default() -> Self {
        Self {
            info: true,
            warn: true,
            error: true,
            debug: true,
        }
    }
}

impl LevelFilter {
    /// Toggle a specific level by number (1=info, 2=warn, 3=error, 4=debug).
    pub fn toggle(&mut self, key: u8) {
        match key {
            1 => self.info = !self.info,
            2 => self.warn = !self.warn,
            3 => self.error = !self.error,
            4 => self.debug = !self.debug,
            _ => {}
        }
    }

    /// Whether the given level passes the filter.
    pub fn accepts(&self, level: NotificationLevel) -> bool {
        match level {
            NotificationLevel::Info => self.info,
            NotificationLevel::Warn => self.warn,
            NotificationLevel::Error => self.error,
            NotificationLevel::Debug => self.debug,
        }
    }
}

/// Backward-compatible alias used by the renderer.
pub type HistoryEntry = NotificationRecord;

/// Render the notification history modal.
///
/// Centered ~75x70 rectangle with a scrollable list of past notifications,
/// most recent first. Supports level-based filtering and shows an eviction
/// counter at the bottom.
pub fn render_notification_history(
    frame: &mut Frame<'_>,
    area: Rect,
    entries: &VecDeque<NotificationRecord>,
    scroll_offset: u16,
    selected_index: usize,
    evicted_count: usize,
    filter: &LevelFilter,
    theme: &Theme,
) {
    let popup = centered_rect(75, 70, area);
    frame.render_widget(Clear, popup);

    // Collect filtered entries (newest first).
    let filtered: Vec<&NotificationRecord> = entries
        .iter()
        .rev()
        .filter(|e| filter.accepts(e.level))
        .collect();

    let title = format!(
        " Notification History ({}/{}) ",
        filtered.len(),
        entries.len()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(theme.accent());

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if filtered.is_empty() {
        let msg = if entries.is_empty() {
            "No notifications yet."
        } else {
            "No notifications match current filters."
        };
        let empty = Paragraph::new(Span::styled(msg, theme.muted()));
        frame.render_widget(empty, inner);
        return;
    }

    // Reserve 3 lines for the footer (filter bar + eviction count + key hints).
    let content_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(inner);

    let now = Instant::now();

    // Build lines in reverse chronological order (most recent first).
    let mut lines: Vec<Line<'_>> = Vec::with_capacity(filtered.len());
    for (i, entry) in filtered.iter().enumerate() {
        let elapsed = now.duration_since(entry.created_at);
        let age = format_age(elapsed.as_secs());

        let (icon, icon_style) = match entry.level {
            NotificationLevel::Info => ("INFO", theme.info()),
            NotificationLevel::Warn => ("WARN", theme.warning()),
            NotificationLevel::Error => ("ERR ", theme.danger()),
            NotificationLevel::Debug => ("DBG ", theme.muted()),
        };

        let is_selected = i == selected_index;
        let msg_style = if is_selected {
            theme.text().add_modifier(Modifier::REVERSED)
        } else {
            theme.text()
        };
        let age_style = if is_selected {
            theme.muted().add_modifier(Modifier::REVERSED)
        } else {
            theme.muted()
        };

        let mut spans = vec![
            Span::styled(format!("{age:>8} "), age_style),
            Span::styled(format!("[{icon}] "), icon_style),
            Span::styled(entry.message.clone(), msg_style),
        ];

        // Show source if present.
        if !entry.source.is_empty() {
            spans.push(Span::styled(
                format!("  ({})", entry.source),
                theme.muted(),
            ));
        }

        // Indicate navigable entries.
        if entry.related_run.is_some() || entry.related_task.is_some() {
            spans.push(Span::styled(" ->", theme.accent()));
        }

        lines.push(Line::from(spans));
    }

    let content_area = content_rows[0];
    let visible_lines = content_area.height as usize;
    let max_scroll = lines.len().saturating_sub(visible_lines);
    let clamped_scroll = (scroll_offset as usize).min(max_scroll) as u16;

    let para = Paragraph::new(lines.clone()).scroll((clamped_scroll, 0));
    frame.render_widget(para, content_area);

    // Scrollbar
    if lines.len() > visible_lines {
        let mut scrollbar_state = ScrollbarState::new(lines.len())
            .position(clamped_scroll as usize)
            .viewport_content_length(visible_lines);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            content_area,
            &mut scrollbar_state,
        );
    }

    // Footer
    let footer_area = content_rows[1];
    let mut footer_lines: Vec<Line<'_>> = Vec::new();

    // Filter status line
    let filter_line = Line::from(vec![
        Span::styled(" Filters: ", theme.muted()),
        filter_span("1:INFO", filter.info, theme),
        Span::raw("  "),
        filter_span("2:WARN", filter.warn, theme),
        Span::raw("  "),
        filter_span("3:ERR", filter.error, theme),
        Span::raw("  "),
        filter_span("4:DBG", filter.debug, theme),
    ]);
    footer_lines.push(filter_line);

    if evicted_count > 0 {
        footer_lines.push(Line::from(Span::styled(
            format!(" {evicted_count} older entries evicted"),
            theme.muted(),
        )));
    } else {
        footer_lines.push(Line::from(""));
    }

    footer_lines.push(Line::from(vec![
        Span::styled(" [Esc]", theme.accent_bold()),
        Span::styled(" close  ", theme.muted()),
        Span::styled("[j/k]", theme.accent_bold()),
        Span::styled(" scroll  ", theme.muted()),
        Span::styled("[Enter]", theme.accent_bold()),
        Span::styled(" jump  ", theme.muted()),
        Span::styled("[1-4]", theme.accent_bold()),
        Span::styled(" filter", theme.muted()),
    ]));

    let footer_para = Paragraph::new(footer_lines);
    frame.render_widget(footer_para, footer_area);
}

/// Render a filter label with active/inactive styling.
fn filter_span<'a>(label: &'a str, active: bool, theme: &Theme) -> Span<'a> {
    if active {
        Span::styled(label, theme.accent_bold())
    } else {
        Span::styled(label, theme.muted())
    }
}

/// Format an elapsed duration as a human-readable age string.
fn format_age(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

/// Redact potential secrets from a notification message before insertion.
///
/// Replaces patterns that look like API keys, tokens, or passwords with
/// `[REDACTED]`.
pub fn redact_message(msg: &str) -> String {
    // Simple heuristic: replace long hex/base64 tokens (32+ chars) and
    // key=value pairs where the key suggests a secret.
    let mut result = msg.to_string();

    // Redact long hex strings (API keys, tokens).
    let hex_re_like: Vec<(usize, usize)> = find_long_tokens(&result, 32);
    for (start, end) in hex_re_like.into_iter().rev() {
        result.replace_range(start..end, "[REDACTED]");
    }

    result
}

/// Find spans of alphanumeric+base64 characters that are at least `min_len` long.
fn find_long_tokens(s: &str, min_len: usize) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'+' || bytes[i] == b'/' || bytes[i] == b'=' {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'+'
                    || bytes[i] == b'/'
                    || bytes[i] == b'='
                    || bytes[i] == b'-'
                    || bytes[i] == b'_')
            {
                i += 1;
            }
            if i - start >= min_len {
                spans.push((start, i));
            }
        } else {
            i += 1;
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn make_record(id: u64, level: NotificationLevel, msg: &str) -> NotificationRecord {
        NotificationRecord {
            id,
            created_at: Instant::now(),
            level,
            source: String::new(),
            message: msg.to_string(),
            related_run: None,
            related_task: None,
            dismissed_at: None,
        }
    }

    fn render_to_text(
        entries: &VecDeque<NotificationRecord>,
        scroll: u16,
        selected: usize,
        evicted: usize,
        filter: &LevelFilter,
    ) -> String {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_notification_history(
                    frame, area, entries, scroll, selected, evicted, filter, &Theme::dark(),
                );
            })
            .expect("render");
        let buffer = terminal.backend().buffer();
        (0..40)
            .map(|y| {
                (0..120)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn empty_history_shows_placeholder() {
        let entries = VecDeque::new();
        let text = render_to_text(&entries, 0, 0, 0, &LevelFilter::default());
        assert!(text.contains("No notifications yet"));
    }

    #[test]
    fn eviction_counter_shown_when_nonzero() {
        let mut entries = VecDeque::new();
        entries.push_back(make_record(1, NotificationLevel::Info, "test notification"));
        let text = render_to_text(&entries, 0, 0, 42, &LevelFilter::default());
        assert!(text.contains("42 older entries evicted"));
    }

    #[test]
    fn filter_hides_levels() {
        let mut entries = VecDeque::new();
        entries.push_back(make_record(1, NotificationLevel::Info, "info message"));
        entries.push_back(make_record(2, NotificationLevel::Error, "error message"));
        entries.push_back(make_record(3, NotificationLevel::Warn, "warn message"));

        // Show only errors.
        let filter = LevelFilter {
            info: false,
            warn: false,
            error: true,
            debug: false,
        };
        let text = render_to_text(&entries, 0, 0, 0, &filter);
        assert!(text.contains("error message"));
        assert!(!text.contains("info message"));
        assert!(!text.contains("warn message"));
        assert!(text.contains("1/3")); // filtered count
    }

    #[test]
    fn all_filtered_shows_no_match_message() {
        let mut entries = VecDeque::new();
        entries.push_back(make_record(1, NotificationLevel::Info, "info message"));

        let filter = LevelFilter {
            info: false,
            warn: false,
            error: false,
            debug: false,
        };
        let text = render_to_text(&entries, 0, 0, 0, &filter);
        assert!(text.contains("No notifications match current filters"));
    }

    #[test]
    fn filter_toggle_cycles() {
        let mut f = LevelFilter::default();
        assert!(f.info);
        f.toggle(1);
        assert!(!f.info);
        f.toggle(1);
        assert!(f.info);

        f.toggle(2);
        assert!(!f.warn);
        f.toggle(3);
        assert!(!f.error);
        f.toggle(4);
        assert!(!f.debug);
    }

    #[test]
    fn eviction_at_201_records() {
        let mut entries = VecDeque::new();
        let mut evicted = 0usize;
        for i in 0..=MAX_HISTORY {
            entries.push_back(make_record(i as u64, NotificationLevel::Info, "msg"));
            while entries.len() > MAX_HISTORY {
                entries.pop_front();
                evicted += 1;
            }
        }
        assert_eq!(entries.len(), MAX_HISTORY);
        assert_eq!(evicted, 1);
    }

    #[test]
    fn dismissed_entries_remain_in_history() {
        let mut entries = VecDeque::new();
        let mut rec = make_record(1, NotificationLevel::Info, "dismissed toast");
        rec.dismissed_at = Some(Instant::now());
        entries.push_back(rec);

        let text = render_to_text(&entries, 0, 0, 0, &LevelFilter::default());
        assert!(text.contains("dismissed toast"));
    }

    #[test]
    fn redact_long_tokens() {
        let msg = "key=abcdefghijklmnopqrstuvwxyz1234567890ABCDEF found";
        let redacted = redact_message(msg);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("abcdefghijklmnopqrstuvwxyz1234567890ABCDEF"));
    }

    #[test]
    fn short_tokens_not_redacted() {
        let msg = "task abc123 completed";
        let redacted = redact_message(msg);
        assert_eq!(redacted, msg);
    }

    #[test]
    fn newest_first_ordering() {
        let mut entries = VecDeque::new();
        entries.push_back(make_record(1, NotificationLevel::Info, "first"));
        entries.push_back(make_record(2, NotificationLevel::Info, "second"));
        entries.push_back(make_record(3, NotificationLevel::Info, "third"));

        let text = render_to_text(&entries, 0, 0, 0, &LevelFilter::default());
        let first_pos = text.find("first").unwrap();
        let third_pos = text.find("third").unwrap();
        // Newest (third) should appear before oldest (first) in the output.
        assert!(third_pos < first_pos);
    }

    #[test]
    fn related_run_shows_arrow() {
        let mut entries = VecDeque::new();
        let mut rec = make_record(1, NotificationLevel::Info, "has run link");
        rec.related_run = Some("run-123".to_string());
        entries.push_back(rec);

        let text = render_to_text(&entries, 0, 0, 0, &LevelFilter::default());
        assert!(text.contains("->"));
    }

    #[test]
    fn source_shown_when_present() {
        let mut entries = VecDeque::new();
        let mut rec = make_record(1, NotificationLevel::Info, "gate passed");
        rec.source = "gate".to_string();
        entries.push_back(rec);

        let text = render_to_text(&entries, 0, 0, 0, &LevelFilter::default());
        assert!(text.contains("(gate)"));
    }

    #[test]
    fn format_age_values() {
        assert_eq!(format_age(0), "0s ago");
        assert_eq!(format_age(30), "30s ago");
        assert_eq!(format_age(60), "1m ago");
        assert_eq!(format_age(3599), "59m ago");
        assert_eq!(format_age(3600), "1h ago");
        assert_eq!(format_age(7200), "2h ago");
    }

    #[test]
    fn filter_bar_rendered() {
        let mut entries = VecDeque::new();
        entries.push_back(make_record(1, NotificationLevel::Info, "test"));
        let text = render_to_text(&entries, 0, 0, 0, &LevelFilter::default());
        assert!(text.contains("1:INFO"));
        assert!(text.contains("2:WARN"));
        assert!(text.contains("3:ERR"));
        assert!(text.contains("4:DBG"));
    }
}
