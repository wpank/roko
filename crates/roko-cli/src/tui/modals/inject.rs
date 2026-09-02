//! Signal injection modal — placeholder for TUI inject UI.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::dashboard::Theme;

/// Render the signal-injection modal (stub).
pub fn render_inject(
    frame: &mut Frame<'_>,
    area: Rect,
    target_agent: &str,
    input_text: &str,
    _cursor_pos: usize,
    theme: &Theme,
) {
    let display = if input_text.is_empty() {
        format!("Inject to {target_agent}: (empty)")
    } else {
        format!("Inject to {target_agent}: {input_text}")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Inject Signal ");
    let paragraph = Paragraph::new(display)
        .block(block)
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph, area);
}
