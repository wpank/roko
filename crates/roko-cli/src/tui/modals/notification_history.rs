//! Scrollable notification history modal.
//!
//! Shows all past notifications that have expired or been dismissed,
//! with timestamp, level icon, and message text.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use super::super::dashboard::Theme;
use super::NotificationLevel;

/// A single entry in the notification history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// When the notification was originally created.
    pub created: Instant,
    /// The notification message.
    pub message: String,
    /// Severity level.
    pub level: NotificationLevel,
}

/// Render the notification history modal.
///
/// Centered ~75x70 rectangle with a scrollable list of past notifications,
/// most recent first. An eviction counter at the bottom shows how many
/// entries have been dropped due to the capacity cap.
pub fn render_notification_history(
    frame: &mut Frame<'_>,
    area: Rect,
    entries: &[HistoryEntry],
    scroll_offset: u16,
    evicted_count: usize,
    theme: &Theme,
) {
    let popup = centered_rect(75, 70, area);
    frame.render_widget(Clear, popup);

    let title = format!(" Notification History ({}) ", entries.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(theme.accent());

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if entries.is_empty() {
        let empty = Paragraph::new(Span::styled("No notifications yet.", theme.muted()));
        frame.render_widget(empty, inner);
        return;
    }

    // Reserve 2 lines for the footer (eviction count + key hints).
    let content_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);

    let now = Instant::now();

    // Build lines in reverse chronological order (most recent first).
    let mut lines: Vec<Line<'_>> = Vec::with_capacity(entries.len());
    for entry in entries.iter().rev() {
        let elapsed = now.duration_since(entry.created);
        let age = format_age(elapsed.as_secs());

        let (icon, icon_style) = match entry.level {
            NotificationLevel::Info => ("INFO", theme.info()),
            NotificationLevel::Warn => ("WARN", theme.warning()),
            NotificationLevel::Error => ("ERR ", theme.danger()),
            NotificationLevel::Debug => ("DBG ", theme.muted()),
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{age:>8} "), theme.muted()),
            Span::styled(format!("[{icon}] "), icon_style),
            Span::styled(entry.message.clone(), theme.text()),
        ]));
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

    // Footer: eviction count + key hints
    let footer_area = content_rows[1];
    let mut footer_lines: Vec<Line<'_>> = Vec::new();

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
        Span::styled("[Up/Down]", theme.accent_bold()),
        Span::styled(" scroll", theme.muted()),
    ]));

    let footer_para = Paragraph::new(footer_lines);
    frame.render_widget(footer_para, footer_area);
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

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    #[test]
    fn empty_history_shows_placeholder() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_notification_history(frame, area, &[], 0, 0, &Theme::dark());
            })
            .expect("render");
        let buffer = terminal.backend().buffer();
        let text: String = (0..40)
            .map(|y| {
                (0..120)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("No notifications yet"));
    }

    #[test]
    fn eviction_counter_shown_when_nonzero() {
        let entries = vec![HistoryEntry {
            created: Instant::now(),
            message: "test notification".into(),
            level: NotificationLevel::Info,
        }];
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_notification_history(frame, area, &entries, 0, 42, &Theme::dark());
            })
            .expect("render");
        let buffer = terminal.backend().buffer();
        let text: String = (0..40)
            .map(|y| {
                (0..120)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("42 older entries evicted"));
    }
}
