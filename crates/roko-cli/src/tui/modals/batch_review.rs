//! Batch review modal — placeholder for TUI batch-review UI.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::dashboard::Theme;

/// Result row shown inside the batch-review modal.
#[derive(Debug, Clone)]
pub struct BatchTaskResult {
    /// Task identifier.
    pub task_id: String,
    /// Human-readable outcome label (e.g. "pass", "fail").
    pub outcome: String,
    /// Whether the task passed.
    pub passed: bool,
}

/// Render the batch-review modal (stub).
pub fn render_batch_review(
    frame: &mut Frame<'_>,
    area: Rect,
    batch_name: &str,
    results: &[BatchTaskResult],
    _scroll_offset: u16,
    theme: &Theme,
) {
    let text = if results.is_empty() {
        format!("Batch \"{batch_name}\": no results yet")
    } else {
        let pass = results.iter().filter(|r| r.passed).count();
        let fail = results.len() - pass;
        format!("Batch \"{batch_name}\": {pass} passed, {fail} failed")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Batch Review ");
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph, area);
}
