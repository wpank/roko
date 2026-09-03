//! Agent command approval modal.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::super::dashboard::Theme;

/// Render the approval modal for an agent command.
///
/// Shows the agent name, the command text in a code-block style, and
/// prominent approve (green) / reject (red) buttons.
/// Centered ~60x40 rectangle.
pub fn render_approval(
    frame: &mut Frame<'_>,
    area: Rect,
    role: &str,
    command: &str,
    theme: &Theme,
) {
    let popup = centered_rect(60, 40, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Approval Required ",
            theme.section_header(),
        ))
        .title_alignment(Alignment::Center)
        .border_style(theme.danger());

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Agent: ", theme.label()),
            Span::styled(role, theme.accent_bold()),
        ]),
        Line::from(""),
        Line::from(Span::styled("Command:", theme.label())),
    ];

    // Render the command in a code-block style.
    let code_style = theme.code_block();
    for line in command.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {line}"),
            code_style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Prominent approve/reject buttons.
    lines.push(Line::from(vec![
        Span::styled(
            " [y] approve ",
            Style::default()
                .fg(Theme::VOID)
                .bg(Theme::SAGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            " [n] reject ",
            Style::default()
                .fg(Theme::BONE)
                .bg(Theme::EMBER)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
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
