//! Daimon affect state view for the F1 Dashboard tab (sub-view 4).
//!
//! Displays PAD gauges (Pleasure/Arousal/Dominance), behavioral state badge,
//! confidence bar, active somatic marker list, and dispatch bias indicators.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::state::TuiState;

/// Width of the bar portion in the PAD gauge (character cells).
const BAR_WIDTH: usize = 20;

/// Build a text-art PAD bar: `[-1.0 ████████░░░░░░░░░░░░ 1.0]  +0.65`
///
/// Filled portion uses `fill_color`, empty portion uses `theme.muted`.
fn pad_bar<'a>(label: &str, value: f64, fill_color: ratatui::style::Color, theme: &Theme) -> Line<'a> {
    let ratio = ((value + 1.0) / 2.0).clamp(0.0, 1.0);
    let filled = (ratio * BAR_WIDTH as f64).round() as usize;
    let empty = BAR_WIDTH.saturating_sub(filled);

    let bar_filled: String = "\u{2588}".repeat(filled);
    let bar_empty: String = "\u{2591}".repeat(empty);

    Line::from(vec![
        Span::styled(
            format!("{label:<10} "),
            theme.label(),
        ),
        Span::styled("[-1.0 ", Style::default().fg(Theme::TEXT_GHOST)),
        Span::styled(bar_filled, Style::default().fg(fill_color)),
        Span::styled(bar_empty, Style::default().fg(Theme::TEXT_PHANTOM)),
        Span::styled(" 1.0]", Style::default().fg(Theme::TEXT_GHOST)),
        Span::styled(
            format!("  {:+.2}", value),
            theme.value(),
        ),
    ])
}

/// Pick gauge fill color: green for positive, red for negative, amber near zero.
fn gauge_color(v: f64, theme: &Theme) -> ratatui::style::Color {
    if v > 0.2 {
        theme.success
    } else if v < -0.2 {
        theme.danger
    } else {
        theme.warning
    }
}

/// Build a confidence percentage bar: `Confidence  [████████████░░░░░░░░]  72%`
fn confidence_bar<'a>(confidence: f64, theme: &Theme) -> Line<'a> {
    let pct = (confidence * 100.0).clamp(0.0, 100.0);
    let ratio = confidence.clamp(0.0, 1.0);
    let filled = (ratio * BAR_WIDTH as f64).round() as usize;
    let empty = BAR_WIDTH.saturating_sub(filled);

    let fill_color = Theme::progress_gradient(ratio);

    let bar_filled: String = "\u{2588}".repeat(filled);
    let bar_empty: String = "\u{2591}".repeat(empty);

    Line::from(vec![
        Span::styled("Confidence ", theme.label()),
        Span::styled("[", Style::default().fg(Theme::TEXT_GHOST)),
        Span::styled(bar_filled, Style::default().fg(fill_color)),
        Span::styled(bar_empty, Style::default().fg(Theme::TEXT_PHANTOM)),
        Span::styled("]", Style::default().fg(Theme::TEXT_GHOST)),
        Span::styled(
            format!("  {pct:.0}%"),
            theme.value(),
        ),
    ])
}

/// Build a colored behavioral state badge: `[FOCUSED]` in green, `[STRESSED]` in red, etc.
fn state_badge<'a>(state: &str, theme: &Theme) -> Line<'a> {
    let (color, bg) = match state {
        "Coasting" | "Focused" => (Theme::VOID, theme.success),
        "Struggling" | "Stressed" => (Theme::BONE, theme.danger),
        "Exploring" | "Curious" => (Theme::VOID, theme.info),
        "Resting" | "Idle" => (Theme::TEXT_STRONG, Theme::BG_HIGHLIGHT),
        "Engaged" => (Theme::VOID, Theme::ROSE),
        _ => (Theme::TEXT_STRONG, Theme::BG_HIGHLIGHT),
    };

    let badge_text = format!(" {} ", state.to_uppercase());

    Line::from(vec![
        Span::styled("State ", theme.label()),
        Span::styled(
            badge_text,
            Style::default()
                .fg(color)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Render the affect state panel.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    _view_state: &ViewState,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Daimon Affect ")
        .border_style(Style::default().fg(theme.accent));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(ref affect) = tui_state.affect else {
        let spinner = tui_state.atmosphere.spinner();
        let placeholder = Paragraph::new(format!(
            "No affect data yet.\n{spinner} Waiting for first task turn..."
        ))
        .style(Style::default().fg(theme.muted));
        frame.render_widget(placeholder, inner);
        return;
    };

    // Layout: header + 3 PAD bars + blank + state badge + confidence + blank + markers/biases
    let chunks = Layout::vertical([
        Constraint::Length(1), // section header
        Constraint::Length(1), // pleasure bar
        Constraint::Length(1), // arousal bar
        Constraint::Length(1), // dominance bar
        Constraint::Length(1), // spacer
        Constraint::Length(1), // behavioral state badge
        Constraint::Length(1), // confidence bar
        Constraint::Length(1), // spacer
        Constraint::Min(0),    // markers + biases
    ])
    .split(inner);

    // -- Section header --
    let header = Paragraph::new(Line::from(Span::styled(
        "Affect State",
        theme.section_header(),
    )));
    frame.render_widget(header, chunks[0]);

    // -- PAD bars --
    let p_color = gauge_color(affect.pleasure, theme);
    let a_color = gauge_color(affect.arousal, theme);
    let d_color = gauge_color(affect.dominance, theme);

    frame.render_widget(
        Paragraph::new(pad_bar("Pleasure", affect.pleasure, p_color, theme)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(pad_bar("Arousal", affect.arousal, a_color, theme)),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(pad_bar("Dominance", affect.dominance, d_color, theme)),
        chunks[3],
    );

    // -- Behavioral state badge --
    frame.render_widget(
        Paragraph::new(state_badge(&affect.behavioral_state, theme)),
        chunks[5],
    );

    // -- Confidence bar --
    frame.render_widget(
        Paragraph::new(confidence_bar(affect.confidence, theme)),
        chunks[6],
    );

    // -- Markers and biases side by side --
    let bottom_chunks =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[8]);

    // Somatic markers
    let marker_items: Vec<ListItem> = if affect.recent_markers.is_empty() {
        vec![ListItem::new(Span::styled(
            "(no markers)",
            Style::default().fg(theme.muted),
        ))]
    } else {
        affect
            .recent_markers
            .iter()
            .take(8)
            .map(|(label, valence)| {
                let icon = if *valence > 0.3 {
                    "+"
                } else if *valence < -0.3 {
                    "-"
                } else {
                    "~"
                };
                let color = if *valence > 0.0 {
                    theme.success
                } else {
                    theme.danger
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {icon} {valence:+.2} "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(label, Style::default().fg(theme.foreground)),
                ]))
            })
            .collect()
    };

    let markers_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Recent Markers ", theme.section_header()))
        .border_style(Style::default().fg(Theme::TEXT_PHANTOM));
    let markers_list = List::new(marker_items).block(markers_block);
    frame.render_widget(markers_list, bottom_chunks[0]);

    // Active biases
    let bias_items: Vec<ListItem> = if affect.active_biases.is_empty() {
        vec![ListItem::new(Span::styled(
            "(none)",
            Style::default().fg(theme.muted),
        ))]
    } else {
        affect
            .active_biases
            .iter()
            .map(|b| {
                let (severity_icon, severity_color) = match b.as_str() {
                    s if s.contains("Struggl") || s.contains("Stress") => ("!", theme.danger),
                    s if s.contains("Rest") || s.contains("Idle") => (".", theme.muted),
                    s if s.contains("Explor") || s.contains("Curious") => ("?", theme.info),
                    _ => ("*", theme.warning),
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {severity_icon} "),
                        Style::default()
                            .fg(severity_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(b, Style::default().fg(theme.foreground)),
                ]))
            })
            .collect()
    };

    let biases_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Active Biases ", theme.section_header()))
        .border_style(Style::default().fg(Theme::TEXT_PHANTOM));
    let biases_list = List::new(bias_items).block(biases_block);
    frame.render_widget(biases_list, bottom_chunks[1]);
}
