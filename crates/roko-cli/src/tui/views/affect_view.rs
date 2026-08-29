//! Daimon affect state view for the F1 Dashboard tab (sub-view 4).
//!
//! Displays PAD gauges (Pleasure/Arousal/Dominance), behavioral state label,
//! active somatic marker list, and dispatch bias indicators.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::state::TuiState;

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
        let placeholder = Paragraph::new("No affect data yet.\nWaiting for first task turn...")
            .style(Style::default().fg(theme.muted));
        frame.render_widget(placeholder, inner);
        return;
    };

    // Layout: 3 gauges + state label + markers + biases
    let chunks = Layout::vertical([
        Constraint::Length(2), // pleasure gauge
        Constraint::Length(2), // arousal gauge
        Constraint::Length(2), // dominance gauge
        Constraint::Length(2), // state + confidence row
        Constraint::Min(0),    // markers + biases
    ])
    .split(inner);

    // PAD gauges. Ratatui Gauge accepts 0..=1 ratio, so map [-1, 1] -> [0, 1].
    let pad_ratio = |v: f64| ((v + 1.0) / 2.0).clamp(0.0, 1.0);

    let gauge_color = |v: f64| {
        if v > 0.2 {
            theme.success
        } else if v < -0.2 {
            theme.danger
        } else {
            theme.warning
        }
    };

    let pleasure_gauge = Gauge::default()
        .block(Block::default().title(format!(" Pleasure  {:+.2}", affect.pleasure)))
        .gauge_style(Style::default().fg(gauge_color(affect.pleasure)))
        .ratio(pad_ratio(affect.pleasure));
    frame.render_widget(pleasure_gauge, chunks[0]);

    let arousal_gauge = Gauge::default()
        .block(Block::default().title(format!(" Arousal   {:+.2}", affect.arousal)))
        .gauge_style(Style::default().fg(gauge_color(affect.arousal)))
        .ratio(pad_ratio(affect.arousal));
    frame.render_widget(arousal_gauge, chunks[1]);

    let dominance_gauge = Gauge::default()
        .block(Block::default().title(format!(" Dominance {:+.2}", affect.dominance)))
        .gauge_style(Style::default().fg(gauge_color(affect.dominance)))
        .ratio(pad_ratio(affect.dominance));
    frame.render_widget(dominance_gauge, chunks[2]);

    // State + confidence row
    let state_color = match affect.behavioral_state.as_str() {
        "Coasting" | "Focused" => theme.success,
        "Struggling" => theme.danger,
        "Exploring" => theme.info,
        "Resting" => theme.muted,
        _ => theme.foreground,
    };
    let state_line = Paragraph::new(Line::from(vec![
        Span::styled("State: ", Style::default().fg(theme.foreground)),
        Span::styled(
            &affect.behavioral_state,
            Style::default()
                .fg(state_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("   Confidence: {:.0}%", affect.confidence * 100.0),
            Style::default().fg(theme.muted),
        ),
    ]));
    frame.render_widget(state_line, chunks[3]);

    // Markers and biases side by side
    let marker_chunks =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[4]);

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
                let color = if *valence > 0.0 {
                    theme.success
                } else {
                    theme.danger
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{valence:+.2} "), Style::default().fg(color)),
                    Span::raw(label),
                ]))
            })
            .collect()
    };

    let markers_block = Block::default()
        .borders(Borders::ALL)
        .title(" Recent Markers ")
        .border_style(Style::default().fg(theme.muted));
    let markers_list = List::new(marker_items).block(markers_block);
    frame.render_widget(markers_list, marker_chunks[0]);

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
            .map(|b| ListItem::new(Span::styled(b, Style::default().fg(theme.info))))
            .collect()
    };

    let biases_block = Block::default()
        .borders(Borders::ALL)
        .title(" Active Biases ")
        .border_style(Style::default().fg(theme.muted));
    let biases_list = List::new(bias_items).block(biases_block);
    frame.render_widget(biases_list, marker_chunks[1]);
}
