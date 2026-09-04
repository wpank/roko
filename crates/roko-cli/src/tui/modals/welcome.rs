//! First-run welcome modal shown when `roko.toml` is absent.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::super::dashboard::Theme;

/// Render the first-run welcome modal.
///
/// Centered fixed-size popup with workspace initialization prompt and
/// provider setup hints.
pub fn render_welcome(frame: &mut Frame<'_>, area: Rect, initialized: bool, theme: &Theme) {
    let popup = centered_rect_fixed(60, 16, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Welcome to Roko ")
        .title_alignment(Alignment::Center)
        .border_style(theme.accent());

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines: Vec<Line<'_>> = Vec::new();

    if initialized {
        // Post-init confirmation
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Workspace initialized.",
            theme.success().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Next steps:", theme.text())));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("    roko config set-secret", theme.accent()),
            Span::styled(" ANTHROPIC_API_KEY ", theme.text()),
            Span::styled("<key>", theme.muted()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    roko config set-secret", theme.accent()),
            Span::styled(" OPENAI_API_KEY ", theme.text()),
            Span::styled("<key>", theme.muted()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Press ", theme.muted()),
            Span::styled("any key", theme.accent()),
            Span::styled(" to continue", theme.muted()),
        ]));
    } else {
        // Initial welcome prompt
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No workspace found in this directory.",
            theme.text(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Roko needs a .roko/ directory and roko.toml",
            theme.muted(),
        )));
        lines.push(Line::from(Span::styled(
            "  to store state, plans, and configuration.",
            theme.muted(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  [Enter]", theme.success().add_modifier(Modifier::BOLD)),
            Span::styled(" Initialize workspace   ", theme.text()),
            Span::styled("[Esc]", theme.warning().add_modifier(Modifier::BOLD)),
            Span::styled(" Skip", theme.text()),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}
