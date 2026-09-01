//! Toast notification stack rendered at the bottom-right corner.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::super::dashboard::Theme;
use super::super::display_utils::truncate;

/// Severity level for a notification toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Warn,
    Error,
    Debug,
}

/// A single toast notification.
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub created: Instant,
    pub ttl_secs: u64,
    pub level: NotificationLevel,
}

impl Notification {
    /// Create a new notification with the given level and TTL.
    pub fn new(message: impl Into<String>, level: NotificationLevel, ttl_secs: u64) -> Self {
        Self {
            message: message.into(),
            created: Instant::now(),
            ttl_secs,
            level,
        }
    }

    /// Create an info notification with default TTL (5 seconds).
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Info, 5)
    }

    /// Create a warning notification with default TTL (8 seconds).
    pub fn warn(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Warn, 8)
    }

    /// Create an error notification with default TTL (10 seconds).
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Error, 10)
    }

    /// Whether this notification has expired.
    pub fn is_expired(&self) -> bool {
        self.created.elapsed().as_secs() >= self.ttl_secs
    }
}

/// Render the notification stack in the bottom-right corner.
///
/// Notifications are stacked from the bottom up. Expired notifications are
/// skipped (caller should prune them separately).
pub fn render_notifications(
    frame: &mut Frame<'_>,
    area: Rect,
    notifications: &[Notification],
    theme: &Theme,
) {
    let active: Vec<&Notification> = notifications.iter().filter(|n| !n.is_expired()).collect();
    if active.is_empty() {
        return;
    }

    // Keep transient messages subordinate to the operational view. Narrow
    // terminals get one toast; larger canvases can retain a short history.
    let density_cap = if area.width < 100 || area.height < 32 {
        1
    } else if area.height < 48 {
        2
    } else {
        3
    };
    let usable_height = area.height.saturating_sub(4); // header/warning + footer
    let max_visible = (usable_height / 3).min(density_cap) as usize;
    if max_visible == 0 {
        return;
    }

    // Size the toast to its contents, but never let it consume more than two
    // thirds of the view. Character counts avoid byte-length inflation for
    // Unicode status text.
    let max_msg_len = active
        .iter()
        .map(|n| n.message.chars().count() + 8) // "[TAG] " prefix + padding
        .max()
        .unwrap_or(40) as u16;
    let max_toast_width = (area.width * 2 / 3).max(30).min(area.width);
    let toast_width = max_msg_len.clamp(30.min(area.width), max_toast_width);
    let toast_height: u16 = 3; // border top + message + border bottom

    for (i, notif) in active.iter().rev().take(max_visible).enumerate() {
        // Reserve the final row for the global status/footer bar.
        let y_offset = area
            .height
            .saturating_sub(1)
            .saturating_sub((i as u16 + 1) * toast_height);
        let x_offset = area.width.saturating_sub(toast_width);

        let toast_area = Rect::new(
            area.x + x_offset,
            area.y + y_offset,
            toast_width.min(area.width),
            toast_height.min(area.height.saturating_sub(y_offset)),
        );

        if toast_area.height < 3 || toast_area.width < 6 {
            continue;
        }

        frame.render_widget(Clear, toast_area);

        let border_style = match notif.level {
            NotificationLevel::Info => theme.info(),
            NotificationLevel::Warn => theme.warning(),
            NotificationLevel::Error => theme.danger(),
            NotificationLevel::Debug => theme.muted(),
        };

        let level_tag = match notif.level {
            NotificationLevel::Info => "INFO",
            NotificationLevel::Warn => "WARN",
            NotificationLevel::Error => "ERR ",
            NotificationLevel::Debug => "DBG ",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(toast_area);
        frame.render_widget(block, toast_area);

        // Truncate message to fit in one line.
        let max_msg_len = inner.width.saturating_sub(6) as usize; // "[TAG] " prefix
        let msg = truncate(&notif.message, max_msg_len);

        let line = Line::from(vec![
            Span::styled(
                format!("[{level_tag}] "),
                border_style.add_modifier(Modifier::BOLD),
            ),
            Span::styled(msg, theme.text()),
        ]);

        frame.render_widget(Paragraph::new(line), inner);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn rendered(width: u16, height: u16, count: usize) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let notifications = (0..count)
            .map(|index| {
                Notification::error(format!(
                    "failure {index} — café résumé 東京 with additional details"
                ))
            })
            .collect::<Vec<_>>();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_notifications(frame, area, &notifications, &Theme::dark());
            })
            .expect("render notifications");
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn narrow_terminal_shows_only_newest_toast_and_preserves_footer() {
        let output = rendered(80, 24, 5);
        assert_eq!(output.matches("[ERR ]").count(), 1);
        assert!(output.contains("failure 4"));
        assert!(!output.lines().last().unwrap_or_default().contains("ERR"));
    }

    #[test]
    fn unicode_message_truncation_does_not_panic() {
        let output = rendered(40, 12, 1);
        assert!(output.contains("ERR"));
    }
}
