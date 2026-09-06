//! Compact affect indicator strip.
//!
//! Renders a 1-2 line summary of the current Daimon affect state:
//!   Affect: flow  P:0.7 A:0.3 D:0.5  conf:82%  [curious engaged]
//!
//! Renders nothing when `affect` is `None`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::Theme;
use roko_core::AffectSnapshot;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Choose a color for a PAD value in [-1, 1].
/// Negative -> ember, near-zero -> dim, positive -> sage.
fn pad_color(v: f64) -> Color {
    if v >= 0.3 {
        Theme::SAGE
    } else if v <= -0.3 {
        Theme::EMBER
    } else {
        Theme::TEXT_DIM
    }
}

/// Choose a label color based on behavioral state keyword.
fn state_color(label: &str) -> Color {
    let l = label.to_ascii_lowercase();
    if l.contains("flow") || l.contains("engaged") || l.contains("calm") || l.contains("focuse") {
        Theme::SAGE
    } else if l.contains("stress") || l.contains("struggling") || l.contains("exhaust") {
        Theme::EMBER
    } else if l.contains("curious") || l.contains("explore") || l.contains("alert") {
        Theme::DREAM
    } else if l.contains("idle") || l.contains("rest") || l.contains("sleep") {
        Theme::TEXT_DIM
    } else {
        Theme::ROSE
    }
}

// ---------------------------------------------------------------------------
// Public render
// ---------------------------------------------------------------------------

/// Render the compact affect strip into `area`.
///
/// If `affect` is `None` or the area is too small, renders nothing.
/// The strip is 1 line tall (2 if biases exist and height permits).
pub fn render_affect_strip(frame: &mut Frame<'_>, area: Rect, affect: Option<&AffectSnapshot>) {
    let Some(snap) = affect else {
        return;
    };
    if area.width < 20 || area.height < 1 {
        return;
    }

    let mut lines: Vec<Line<'_>> = Vec::new();

    // Line 1: label  P/A/D values  confidence
    {
        let label = if snap.behavioral_state.is_empty() {
            "unknown"
        } else {
            snap.behavioral_state.as_str()
        };
        let label_color = state_color(label);

        let conf_pct = (snap.confidence * 100.0).round() as u64;
        let conf_color = if snap.confidence >= 0.7 {
            Theme::SAGE
        } else if snap.confidence >= 0.4 {
            Theme::WARNING
        } else {
            Theme::EMBER
        };

        let mut spans = vec![
            Span::styled("Affect ", Style::default().fg(Theme::TEXT_GHOST)),
            Span::styled(
                label,
                Style::default()
                    .fg(label_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
            Span::styled("P:", Style::default().fg(Theme::TEXT_GHOST)),
            Span::styled(
                format!("{:.1}", snap.pleasure),
                Style::default().fg(pad_color(snap.pleasure)),
            ),
            Span::styled(" A:", Style::default().fg(Theme::TEXT_GHOST)),
            Span::styled(
                format!("{:.1}", snap.arousal),
                Style::default().fg(pad_color(snap.arousal)),
            ),
            Span::styled(" D:", Style::default().fg(Theme::TEXT_GHOST)),
            Span::styled(
                format!("{:.1}", snap.dominance),
                Style::default().fg(pad_color(snap.dominance)),
            ),
            Span::styled("  conf:", Style::default().fg(Theme::TEXT_GHOST)),
            Span::styled(
                format!("{conf_pct}%"),
                Style::default().fg(conf_color),
            ),
        ];

        // P2-07: Cognitive energy gauge.
        if snap.cognitive_energy > 0.0 || snap.efe_tier.is_some() {
            let energy_pct = (snap.cognitive_energy * 100.0).round() as u64;
            let energy_color = if snap.cognitive_energy >= 0.6 {
                Theme::SAGE
            } else if snap.cognitive_energy >= 0.3 {
                Theme::WARNING
            } else {
                Theme::EMBER
            };
            spans.push(Span::styled("  E:", Style::default().fg(Theme::TEXT_GHOST)));
            spans.push(Span::styled(
                format!("{energy_pct}%"),
                Style::default().fg(energy_color),
            ));
            if let Some(tier) = snap.efe_tier {
                spans.push(Span::styled(
                    format!(" EFE:{tier}"),
                    Style::default().fg(Theme::DREAM),
                ));
            }
        }

        // Append active biases on the same line if there is room.
        if !snap.active_biases.is_empty() && area.width >= 60 {
            let biases = snap.active_biases.join(" ");
            spans.push(Span::styled("  [", Style::default().fg(Theme::TEXT_GHOST)));
            spans.push(Span::styled(
                biases,
                Style::default().fg(Theme::DREAM),
            ));
            spans.push(Span::styled("]", Style::default().fg(Theme::TEXT_GHOST)));
        }

        lines.push(Line::from(spans));
    }

    // Line 2 (optional): active biases when they didn't fit on line 1.
    if area.height >= 2 && !snap.active_biases.is_empty() && area.width < 60 {
        let biases = snap.active_biases.join(" ");
        lines.push(Line::from(vec![
            Span::styled("       [", Style::default().fg(Theme::TEXT_GHOST)),
            Span::styled(biases, Style::default().fg(Theme::DREAM)),
            Span::styled("]", Style::default().fg(Theme::TEXT_GHOST)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}
