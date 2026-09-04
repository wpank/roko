//! Token burn sparkline widget.
//!
//! Shows an efficiency summary, a token-usage sparkline, and a compact model
//! tier distribution using the live dashboard snapshot.

use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::braille;
use super::rosedust::brighten;
use crate::tui::Theme;
use crate::tui::dashboard::DashboardData;
use crate::tui::pages::efficiency::{
    EfficiencySnapshot, build_efficiency_snapshot, model_tier_label, tier_bar_labels,
};
use crate::tui::state::TuiState;

fn fmt_tokens(n: u64) -> String {
    if n == 0 {
        "0".to_string()
    } else if n < 1_000 {
        format!("{n}")
    } else if n < 10_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else if n < 1_000_000 {
        format!("{}k", n / 1_000)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

fn fmt_rate(rate: f64) -> String {
    if rate <= 0.5 {
        "idle".to_string()
    } else if rate >= 1_000_000.0 {
        format!("{:.1}M/min", rate / 1_000_000.0)
    } else if rate >= 1_000.0 {
        format!("{:.1}k/min", rate / 1_000.0)
    } else {
        format!("{rate:.0}/min")
    }
}

fn tier_color(tier: &str) -> ratatui::style::Color {
    match tier {
        "T0" => Theme::SAGE,
        "T1" => Theme::ROSE,
        "T2" => Theme::WARNING,
        _ => Theme::TEXT_DIM,
    }
}

/// Map a 0..1 normalized burn value to a gradient color:
/// low (green/sage) -> medium (yellow/warning) -> high (red/ember).
fn burn_gradient(t: f64) -> ratatui::style::Color {
    Theme::progress_gradient(1.0 - t.clamp(0.0, 1.0))
}

/// Generic fallback label for a tier with no events. Non-empty tiers are
/// labeled by their dominant model family/slug via
/// [`EfficiencySnapshot::tier_labels`], never hardcoded claude tier names.
fn tier_fallback_label(tier: &str) -> &'static str {
    match tier {
        "T0" => "fast",
        "T1" => "std",
        "T2" => "pro",
        _ => "other",
    }
}

fn sparkline_window(width: usize, total_samples: usize) -> usize {
    let preferred = if width >= 120 {
        100
    } else if width >= 80 {
        50
    } else {
        10
    };
    preferred.min(total_samples.max(2))
}

/// Render the token burn sparkline widget.
pub fn render_token_sparkline(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    state: &TuiState,
) {
    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2) as usize;
    if inner_width < 10 || inner_height < 2 {
        return;
    }

    // Prefer the file-backed efficiency snapshot; when efficiency events are
    // empty (live/connected mode where DashboardData isn't loaded from disk),
    // fall back to the cumulative token/cost fields on TuiState which are
    // populated from the DashboardSnapshot push path.
    let snapshot = {
        let file_snap = build_efficiency_snapshot(data);
        if file_snap.event_count > 0 || file_snap.total_tokens > 0 {
            file_snap
        } else {
            build_snapshot_from_tui_state(state)
        }
    };
    let window = sparkline_window(inner_width, snapshot.token_series.len());
    let display: Vec<u64> = snapshot
        .token_series
        .iter()
        .rev()
        .take(window)
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let pulsed_color = brighten(Theme::ROSE, state.atmosphere.breathing_brightness());
    let border_color = if snapshot.total_cost_usd > 0.0 {
        Theme::ROSE_DIM
    } else {
        Theme::TEXT_GHOST
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Efficiency")
        .style(Theme::block_style())
        .border_style(Style::default().fg(border_color))
        .title_style(Theme::title_style());
    let inner = block.inner(area);

    if inner.width < 8 || inner.height < 1 {
        return;
    }

    let theme = Theme::dark();
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Cost color: gradient from sage (cheap) through warning to ember (expensive).
    let cost_color = burn_gradient(
        (snapshot.total_cost_usd / 5.0).clamp(0.0, 1.0), // $5 = full red
    );
    let summary1 = Line::from(vec![
        Span::styled(" tokens ", theme.label()),
        Span::styled(fmt_tokens(snapshot.total_tokens), theme.value()),
        Span::styled(" cost ", theme.label()),
        Span::styled(
            format!("${:.2}", snapshot.total_cost_usd),
            Style::default().fg(cost_color),
        ),
        Span::styled(" avg/task ", theme.label()),
        Span::styled(
            fmt_tokens(snapshot.average_tokens_per_task.round() as u64),
            theme.value(),
        ),
    ]);
    lines.push(summary1);

    if inner.height > 3 {
        let summary2 = Line::from(vec![
            Span::styled(" succ ", theme.label()),
            Span::styled(
                format!("{:.0}%", snapshot.success_rate * 100.0),
                Style::default().fg(if snapshot.success_rate >= 0.9 {
                    Theme::SAGE
                } else if snapshot.success_rate >= 0.6 {
                    Theme::WARNING
                } else {
                    Theme::EMBER
                }),
            ),
            Span::styled(" events ", theme.label()),
            Span::styled(format!("{}", snapshot.event_count), theme.value()),
            Span::styled(" window ", theme.label()),
            Span::styled(format!("{window}"), Style::default().fg(Theme::TEXT_DIM)),
        ]);
        lines.push(summary2);
    }

    if display.len() >= 2 {
        let rate = if snapshot.total_tokens > 0 {
            display.iter().copied().sum::<u64>() as f64 / display.len() as f64
        } else {
            0.0
        };
        let min_val = *display.iter().min().unwrap_or(&0);
        let max_val = *display.iter().max().unwrap_or(&1);
        let range_label = format!("{}-{}", fmt_tokens(min_val), fmt_tokens(max_val));
        let rate_label = fmt_rate(rate);
        // Color the rate label by burn intensity (high rate = warm, low = cool).
        let rate_intensity = (rate / 10_000.0).clamp(0.0, 1.0);
        let rate_color = burn_gradient(rate_intensity);
        let spark_w = inner_width
            .saturating_sub(range_label.len() + rate_label.len() + 5)
            .max(8);
        let mut spans = vec![Span::styled(
            format!(" {} ", range_label),
            Style::default().fg(Theme::TEXT_GHOST),
        )];
        // Use gradient coloring: sage for low values, warning mid, ember peaks.
        let normalized: Vec<f64> = {
            let min_f = min_val as f64;
            let range_f = (max_val - min_val).max(1) as f64;
            display
                .iter()
                .map(|&v| (v as f64 - min_f) / range_f)
                .collect()
        };
        spans.extend(braille::braille_spans_gradient(
            &normalized,
            1.0,
            spark_w,
            Theme::SAGE,
            pulsed_color,
        ));
        spans.push(Span::styled(
            format!(" {} ", rate_label),
            Style::default().fg(rate_color),
        ));
        lines.push(Line::from(spans));
    } else {
        lines.push(Line::from(Span::styled(
            format!(" {} waiting for data", state.atmosphere.spinner()),
            Style::default().fg(Theme::TEXT_DIM),
        )));
    }

    let remaining_rows = inner_height.saturating_sub(lines.len());
    let event_count = snapshot.event_count.max(1) as f64;
    for tier in ["T0", "T1", "T2"].into_iter().take(remaining_rows) {
        let count = snapshot.tier_counts.get(tier).copied().unwrap_or_default();
        let pct = count as f64 / event_count;
        let bar_label = snapshot
            .tier_labels
            .get(tier)
            .map_or(tier_fallback_label(tier), String::as_str);
        let label = format!(" {:>2} {:<8} ", tier, bar_label);
        let suffix = format!(" {} ({:.0}%)", count, pct * 100.0);
        let bar_w = inner_width
            .saturating_sub(label.len() + suffix.len())
            .max(6);
        let filled = (pct.clamp(0.0, 1.0) * bar_w as f64).round() as usize;
        let empty = bar_w.saturating_sub(filled);
        lines.push(Line::from(vec![
            Span::styled(label, Style::default().fg(tier_color(tier))),
            Span::styled(
                "\u{2588}".repeat(filled.min(bar_w)),
                Style::default().fg(tier_color(tier)),
            ),
            Span::styled(
                "\u{2500}".repeat(empty),
                Style::default().fg(Theme::TEXT_PHANTOM),
            ),
            Span::styled(suffix, theme.label()),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Build an [`EfficiencySnapshot`] from live [`TuiState`] fields when the
/// file-backed efficiency events are unavailable (connected/live mode).
fn build_snapshot_from_tui_state(state: &TuiState) -> EfficiencySnapshot {
    let total_input_tokens = state.cumulative_input_tokens;
    let total_output_tokens = state.cumulative_output_tokens;
    let total_tokens = total_input_tokens + total_output_tokens;
    let total_cost_usd = state.cost_dollars;
    let task_count = state.agents.len().max(1);
    let average_tokens_per_task = if task_count == 0 {
        0.0
    } else {
        total_tokens as f64 / task_count as f64
    };
    let average_cost_per_task = if task_count == 0 {
        0.0
    } else {
        total_cost_usd / task_count as f64
    };

    // Derive success rate from gate_pass_rate if available.
    let success_rate = state.gate_pass_rate.unwrap_or(0.0);

    // Build tier counts from agent models.
    let mut tier_counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    for tier in ["T0", "T1", "T2"] {
        tier_counts.insert(tier, 0);
    }
    for agent in &state.agents {
        *tier_counts
            .entry(model_tier_label(&agent.model))
            .or_default() += 1;
    }
    let tier_labels = tier_bar_labels(state.agents.iter().map(|a| a.model.as_str()));

    // Build token series from per-agent token history; each sample is the
    // sum of all agents' cumulative totals at that point.
    let token_series = if state.token_history.is_empty() {
        // No history — emit a single sample so the sparkline has something.
        if total_tokens > 0 {
            vec![total_tokens]
        } else {
            Vec::new()
        }
    } else {
        let max_len = state
            .token_history
            .values()
            .map(|v| v.len())
            .max()
            .unwrap_or(0);
        (0..max_len)
            .map(|i| {
                state
                    .token_history
                    .values()
                    .map(|v| v.get(i).copied().unwrap_or(0))
                    .sum()
            })
            .collect()
    };

    EfficiencySnapshot {
        event_count: state.agents.len(),
        task_count,
        total_tokens,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
        average_tokens_per_task,
        average_cost_per_task,
        success_rate,
        tier_counts,
        tier_labels,
        token_series,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::AgentRow;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use roko_learn::efficiency::AgentEfficiencyEvent;

    fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn event(model: &str, task_id: &str) -> AgentEfficiencyEvent {
        let mut event = AgentEfficiencyEvent::default_event();
        event.model = model.to_string();
        event.plan_id = "plan-1".to_string();
        event.task_id = task_id.to_string();
        event.input_tokens = 1_000;
        event.output_tokens = 200;
        event
    }

    fn render(data: &DashboardData, state: &TuiState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_token_sparkline(frame, area, data, state);
            })
            .unwrap();
        rendered_text(&terminal)
    }

    #[test]
    fn sparkline_labels_tiers_by_family_not_sonnet() {
        let mut data = DashboardData::default();
        data.efficiency_events = vec![
            event("gpt-5.6-sol", "t1"),
            event("glm-5.1", "t2"),
            event("codex-mini", "t3"),
        ];
        data.efficiency.total_input_tokens = 4_000;
        data.efficiency.total_output_tokens = 800;
        data.efficiency.total_cost_usd = 0.42;
        let state = TuiState::new();

        let text = render(&data, &state, 100, 12);
        assert!(text.contains("Efficiency"), "title missing:\n{text}");
        // Codex/gpt/glm tokens render under family labels...
        assert!(text.contains("gpt"), "gpt label missing:\n{text}");
        assert!(text.contains("glm"), "glm label missing:\n{text}");
        assert!(text.contains("codex"), "codex label missing:\n{text}");
        // ...never under hardcoded claude tier names.
        assert!(!text.contains("sonnet"), "stale label present:\n{text}");
        assert!(!text.contains("haiku"), "stale label present:\n{text}");
    }

    #[test]
    fn sparkline_empty_uses_generic_fallback_labels() {
        let data = DashboardData::default();
        let state = TuiState::new();
        let text = render(&data, &state, 100, 12);
        assert!(
            text.contains("waiting for data"),
            "placeholder missing:\n{text}"
        );
        assert!(text.contains("fast"), "fallback label missing:\n{text}");
        assert!(text.contains("std"), "fallback label missing:\n{text}");
        assert!(text.contains("pro"), "fallback label missing:\n{text}");
        assert!(!text.contains("sonnet"), "stale label present:\n{text}");
    }

    #[test]
    fn sparkline_tiny_area_does_not_panic() {
        let data = DashboardData::default();
        let state = TuiState::new();
        let _ = render(&data, &state, 8, 3);
    }

    #[test]
    fn snapshot_from_tui_state_labels_live_agents() {
        let mut state = TuiState::new();
        for (id, model) in [
            ("a1", "codex-mini"),
            ("a2", "glm-5.1"),
            ("a3", "totally-custom-model"),
        ] {
            let agent = AgentRow {
                id: id.to_string(),
                model: model.to_string(),
                ..AgentRow::default()
            };
            state.agents.push(agent);
        }

        let snap = build_snapshot_from_tui_state(&state);
        assert_eq!(snap.tier_counts.get("T0").copied(), Some(1)); // codex-mini -> Fast
        assert_eq!(snap.tier_counts.get("T1").copied(), Some(2)); // glm + unknown
        assert_eq!(
            snap.tier_labels.get("T0").map(String::as_str),
            Some("codex")
        );
        assert_eq!(snap.tier_labels.get("T1").map(String::as_str), Some("glm"));
        // No label is a claude tier name.
        assert!(snap.tier_labels.values().all(|l| l != "sonnet"));
    }

    #[test]
    fn snapshot_from_tui_state_empty_model_degrades_gracefully() {
        let mut state = TuiState::new();
        state.agents.push(AgentRow::default()); // model == ""
        let snap = build_snapshot_from_tui_state(&state);
        assert_eq!(snap.tier_counts.get("T1").copied(), Some(1));
        assert_eq!(
            snap.tier_labels.get("T1").map(String::as_str),
            Some("unknown")
        );
    }
}
