//! Bounded, redacted in-memory notification history and its modal renderer.

use std::collections::VecDeque;
use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::notification::NotificationLevel;
use crate::tui::dashboard::Theme;
use crate::tui::display_utils::truncate;

/// Maximum number of notification records retained in memory.
pub const NOTIFICATION_HISTORY_LIMIT: usize = 200;

/// One redacted notification retained independently from transient toasts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRecord {
    /// Monotonic session-local notification identifier.
    pub id: u64,
    /// Creation timestamp as Unix epoch milliseconds.
    pub created_at: u64,
    /// Notification severity.
    pub level: NotificationLevel,
    /// Stable producer category (for example `gate` or `tui`).
    pub source: String,
    /// Redacted text exactly as eligible for display.
    pub message: String,
    /// Canonical related run/plan identifier, when available.
    pub related_run: Option<String>,
    /// Canonical related task identifier, when available.
    pub related_task: Option<String>,
    /// Manual dismissal timestamp. Expiration does not mark dismissal.
    pub dismissed_at: Option<u64>,
}

/// Severity filters for Notification History.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationFilters {
    /// Show informational records.
    pub info: bool,
    /// Show warning records.
    pub warn: bool,
    /// Show error records.
    pub error: bool,
    /// Show debug records.
    pub debug: bool,
}

impl Default for NotificationFilters {
    fn default() -> Self {
        Self {
            info: true,
            warn: true,
            error: true,
            debug: true,
        }
    }
}

impl NotificationFilters {
    /// Whether a severity is currently visible.
    #[must_use]
    pub const fn includes(self, level: NotificationLevel) -> bool {
        match level {
            NotificationLevel::Info => self.info,
            NotificationLevel::Warn => self.warn,
            NotificationLevel::Error => self.error,
            NotificationLevel::Debug => self.debug,
        }
    }

    /// Toggle one filter bit.
    pub fn toggle(&mut self, level: NotificationLevel) {
        let bit = match level {
            NotificationLevel::Info => &mut self.info,
            NotificationLevel::Warn => &mut self.warn,
            NotificationLevel::Error => &mut self.error,
            NotificationLevel::Debug => &mut self.debug,
        };
        *bit = !*bit;
    }
}

/// Scrub a message before it crosses the notification storage boundary.
#[must_use]
pub fn redact_notification_message(message: &str) -> String {
    static SCRUBBER: OnceLock<roko_core::obs::LogScrubber> = OnceLock::new();
    SCRUBBER
        .get_or_init(roko_core::obs::LogScrubber::new)
        .scrub(message)
}

/// Insert a record at the back, evicting the oldest at the exact bound.
pub fn push_history_record(
    history: &mut VecDeque<NotificationRecord>,
    evictions: &mut u64,
    record: NotificationRecord,
) {
    if history.len() >= NOTIFICATION_HISTORY_LIMIT {
        history.pop_front();
        *evictions = evictions.saturating_add(1);
    }
    history.push_back(record);
}

/// Indices of records admitted by the filters, newest first.
#[must_use]
pub fn filtered_indices_newest_first(
    history: &VecDeque<NotificationRecord>,
    filters: NotificationFilters,
) -> Vec<usize> {
    searched_indices_newest_first(history, filters, "")
}

/// Indices admitted by both severity filters and a case-insensitive query.
///
/// The query searches every operator-relevant field instead of only the
/// rendered message: message, producer source, related run, and related task.
#[must_use]
pub fn searched_indices_newest_first(
    history: &VecDeque<NotificationRecord>,
    filters: NotificationFilters,
    query: &str,
) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    history
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(index, record)| {
            let query_matches = query.is_empty()
                || record.message.to_lowercase().contains(&query)
                || record.source.to_lowercase().contains(&query)
                || record
                    .related_run
                    .as_deref()
                    .is_some_and(|run| run.to_lowercase().contains(&query))
                || record
                    .related_task
                    .as_deref()
                    .is_some_and(|task| task.to_lowercase().contains(&query));
            (filters.includes(record.level) && query_matches).then_some(index)
        })
        .collect()
}

/// Render the Notification History modal without an active search query.
///
/// This compatibility entry point preserves the original renderer API. New
/// interactive callers should use [`render_notification_history_with_search`].
#[allow(clippy::too_many_arguments)]
pub fn render_notification_history(
    frame: &mut Frame<'_>,
    area: Rect,
    history: &VecDeque<NotificationRecord>,
    filters: NotificationFilters,
    selected_index: usize,
    scroll_offset: usize,
    evictions: u64,
    theme: &Theme,
) {
    render_notification_history_with_search(
        frame,
        area,
        history,
        filters,
        "",
        false,
        selected_index,
        scroll_offset,
        evictions,
        theme,
    );
}

/// Render the searchable Notification History modal.
#[allow(clippy::too_many_arguments)]
pub fn render_notification_history_with_search(
    frame: &mut Frame<'_>,
    area: Rect,
    history: &VecDeque<NotificationRecord>,
    filters: NotificationFilters,
    query: &str,
    query_editing: bool,
    selected_index: usize,
    scroll_offset: usize,
    evictions: u64,
    theme: &Theme,
) {
    let width = area.width.saturating_sub(4).min(100).max(20);
    let height = area.height.saturating_sub(4).min(32).max(8);
    let popup = centered_rect(width, height, area);
    frame.render_widget(Clear, popup);

    let visible = searched_indices_newest_first(history, filters, query);
    let title = format!(
        " Notification History ({}/{}, evicted {evictions}) ",
        visible.len(),
        history.len()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, theme.accent_bold()))
        .border_style(theme.accent());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if inner.width < 8 || inner.height < 3 {
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(inner);
    let filters_line = Line::from(vec![
        filter_span("1 INFO", filters.info, theme),
        Span::raw(" "),
        filter_span("2 WARN", filters.warn, theme),
        Span::raw(" "),
        filter_span("3 ERROR", filters.error, theme),
        Span::raw(" "),
        filter_span("4 DEBUG", filters.debug, theme),
    ]);
    frame.render_widget(Paragraph::new(filters_line), rows[0]);
    let query_marker = if query_editing { "▸" } else { "/" };
    let query_text = if query.is_empty() {
        format!(" {query_marker} search message/source/run/task")
    } else {
        format!(" {query_marker}{query}")
    };
    frame.render_widget(
        Paragraph::new(query_text).style(if query_editing {
            theme.selection()
        } else {
            theme.muted()
        }),
        rows[1],
    );

    if visible.is_empty() {
        frame.render_widget(
            Paragraph::new("No notifications match the active filters and query.")
                .alignment(Alignment::Center)
                .style(theme.muted()),
            rows[2],
        );
    } else {
        let selected = selected_index.min(visible.len().saturating_sub(1));
        let viewport = rows[2].height as usize;
        let max_scroll = visible.len().saturating_sub(viewport);
        let scroll = scroll_offset.min(max_scroll).min(selected);
        let lines = visible
            .iter()
            .skip(scroll)
            .take(viewport)
            .enumerate()
            .map(|(row, history_index)| {
                let record = &history[*history_index];
                let is_selected = scroll + row == selected;
                notification_line(record, is_selected, rows[2].width, theme)
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), rows[2]);
    }

    frame.render_widget(
        Paragraph::new(" / search · Ctrl-U clear · 1-4 severity · Enter jump · Esc close ")
            .style(theme.muted()),
        rows[3],
    );
}

fn filter_span<'a>(label: &'a str, enabled: bool, theme: &Theme) -> Span<'a> {
    let style = if enabled {
        theme.selection().add_modifier(Modifier::BOLD)
    } else {
        theme.muted()
    };
    Span::styled(label, style)
}

fn notification_line<'a>(
    record: &'a NotificationRecord,
    selected: bool,
    width: u16,
    theme: &Theme,
) -> Line<'a> {
    let marker = if selected { "▶" } else { " " };
    let level = match record.level {
        NotificationLevel::Info => "INFO",
        NotificationLevel::Warn => "WARN",
        NotificationLevel::Error => "ERR ",
        NotificationLevel::Debug => "DBG ",
    };
    let dismissed = if record.dismissed_at.is_some() {
        " ×"
    } else {
        ""
    };
    let target = record
        .related_run
        .as_deref()
        .map(|run| match record.related_task.as_deref() {
            Some(task) => format!(" [{run}/{task}]"),
            None => format!(" [{run}]"),
        })
        .unwrap_or_default();
    let prefix = format!("{marker} [{level}] {}{dismissed}{target}: ", record.source);
    let message_width = (width as usize).saturating_sub(prefix.chars().count());
    let style = if selected {
        theme.selection()
    } else {
        Style::default().fg(Theme::TEXT_DIM)
    };
    Line::from(Span::styled(
        format!("{prefix}{}", truncate(&record.message, message_width)),
        style,
    ))
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn record(id: u64, level: NotificationLevel, message: &str) -> NotificationRecord {
        NotificationRecord {
            id,
            created_at: 1_000 + id,
            level,
            source: "test".into(),
            message: redact_notification_message(message),
            related_run: None,
            related_task: None,
            dismissed_at: None,
        }
    }

    #[test]
    fn notification_history_bound_filter_order_and_redaction() {
        let mut history = VecDeque::new();
        let mut evictions = 0;
        for id in 0..=NOTIFICATION_HISTORY_LIMIT as u64 {
            push_history_record(
                &mut history,
                &mut evictions,
                record(
                    id,
                    if id % 2 == 0 {
                        NotificationLevel::Info
                    } else {
                        NotificationLevel::Error
                    },
                    "token sk-ant-api03-aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789ABCD",
                ),
            );
        }
        assert_eq!(history.len(), NOTIFICATION_HISTORY_LIMIT);
        assert_eq!(history.front().map(|entry| entry.id), Some(1));
        assert_eq!(evictions, 1);
        assert!(
            history
                .iter()
                .all(|entry| !entry.message.contains("sk-ant"))
        );

        let filters = NotificationFilters {
            info: false,
            warn: false,
            error: true,
            debug: false,
        };
        let visible = filtered_indices_newest_first(&history, filters);
        assert!(visible.windows(2).all(|pair| pair[0] > pair[1]));
        assert!(
            visible
                .iter()
                .all(|index| history[*index].level == NotificationLevel::Error)
        );
    }

    #[test]
    fn notification_search_composes_with_severity_and_all_identity_fields() {
        let mut history = VecDeque::from([
            NotificationRecord {
                source: "gate".into(),
                related_run: Some("plan-alpha".into()),
                related_task: Some("compile-task".into()),
                ..record(1, NotificationLevel::Error, "Compilation failed")
            },
            NotificationRecord {
                source: "scheduler".into(),
                related_run: Some("plan-beta".into()),
                related_task: Some("dispatch-task".into()),
                ..record(2, NotificationLevel::Info, "Worker ready")
            },
        ]);
        let errors_only = NotificationFilters {
            info: false,
            warn: false,
            error: true,
            debug: false,
        };

        assert_eq!(
            searched_indices_newest_first(&history, errors_only, "FAILED"),
            [0]
        );
        assert_eq!(
            searched_indices_newest_first(&history, errors_only, "gate"),
            [0]
        );
        assert_eq!(
            searched_indices_newest_first(&history, errors_only, "alpha"),
            [0]
        );
        assert_eq!(
            searched_indices_newest_first(&history, errors_only, "compile"),
            [0]
        );
        assert!(searched_indices_newest_first(&history, errors_only, "worker").is_empty());

        // Ensure the test does not accidentally rely on insertion mutability.
        history.clear();
        assert!(searched_indices_newest_first(&history, errors_only, "").is_empty());
    }

    #[test]
    fn searchable_renderer_shows_query_and_only_matching_records() {
        let history = VecDeque::from([
            NotificationRecord {
                source: "gate".into(),
                related_run: Some("plan-alpha".into()),
                related_task: Some("compile-task".into()),
                ..record(1, NotificationLevel::Error, "Compilation failed")
            },
            NotificationRecord {
                source: "scheduler".into(),
                related_run: Some("plan-beta".into()),
                related_task: Some("dispatch-task".into()),
                ..record(2, NotificationLevel::Info, "Worker ready")
            },
        ]);
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_notification_history_with_search(
                    frame,
                    frame.area(),
                    &history,
                    NotificationFilters::default(),
                    "compile-task",
                    true,
                    0,
                    0,
                    0,
                    &Theme::dark(),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        let rendered = buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("compile-task"));
        assert!(rendered.contains("Compilation failed"));
        assert!(!rendered.contains("Worker ready"));
    }
}
