//! Per-model cost statistics table widget.
//!
//! Columns: Model | Provider | Tasks | Pass% | Avg Duration | Total Cost | $/Task
//! Pass rates are color-coded: green >80%, amber 50-80%, red <50%.

use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::tui::dashboard::Theme;
use crate::tui::state::TuiState;

/// Aggregated per-model cost entry.
#[derive(Debug, Clone, Default)]
struct ModelCostEntry {
    provider: String,
    tasks: u64,
    passed: u64,
    total_duration_ms: u64,
    total_cost_usd: f64,
}

impl ModelCostEntry {
    fn pass_pct(&self) -> f64 {
        if self.tasks == 0 {
            0.0
        } else {
            self.passed as f64 / self.tasks as f64 * 100.0
        }
    }

    fn avg_duration_ms(&self) -> f64 {
        if self.tasks == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.tasks as f64
        }
    }

    fn cost_per_task(&self) -> f64 {
        if self.tasks == 0 {
            0.0
        } else {
            self.total_cost_usd / self.tasks as f64
        }
    }
}

/// Render the "Cost by Model" table.
pub fn render_cost_by_model_table(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let focused = false; // standalone widget, not focus-tracked
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        theme.accent()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Cost by Model ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 20 || inner.height < 3 {
        return;
    }

    // Aggregate efficiency events by model.
    let mut models: BTreeMap<String, ModelCostEntry> = BTreeMap::new();
    for event in &tui_state.efficiency_events {
        // Only count final-turn events for task-level stats.
        if !event.is_final_turn {
            continue;
        }
        let model_key = if event.model.is_empty() {
            "(unknown)".to_string()
        } else {
            event.model.clone()
        };
        let entry = models.entry(model_key).or_default();
        if entry.provider.is_empty() && !event.backend.is_empty() {
            entry.provider.clone_from(&event.backend);
        }
        entry.tasks += 1;
        if event.gate_passed == Some(true) {
            entry.passed += 1;
        }
        let dur = if event.wall_time_ms > 0 {
            event.wall_time_ms
        } else {
            event.duration_ms
        };
        entry.total_duration_ms += dur;
        entry.total_cost_usd += event.cost_usd;
    }

    // When efficiency events are empty (live/connected mode), fall back to
    // the per-agent data in TuiState which is populated from the snapshot
    // push path.
    if models.is_empty() {
        for agent in &tui_state.agents {
            let total_tokens = agent.input_tokens + agent.output_tokens;
            if total_tokens == 0 {
                continue;
            }
            let model_key = if agent.model.is_empty() {
                "(unknown)".to_string()
            } else {
                agent.model.clone()
            };
            let entry = models.entry(model_key).or_default();
            entry.tasks += 1;
            // No per-agent cost breakdown available; distribute the global
            // cost proportionally by token share later. For now count tokens
            // as a duration proxy (1 token ≈ 1ms for display purposes).
            entry.total_duration_ms += total_tokens;
        }
        // Distribute global cost proportionally across models by token count.
        let total_cost = tui_state.cost_dollars;
        let total_tokens: u64 = tui_state
            .agents
            .iter()
            .map(|a| a.input_tokens + a.output_tokens)
            .sum();
        if total_cost > 0.0 && total_tokens > 0 {
            for agent in &tui_state.agents {
                let agent_tokens = agent.input_tokens + agent.output_tokens;
                if agent_tokens == 0 {
                    continue;
                }
                let model_key = if agent.model.is_empty() {
                    "(unknown)".to_string()
                } else {
                    agent.model.clone()
                };
                if let Some(entry) = models.get_mut(&model_key) {
                    entry.total_cost_usd +=
                        total_cost * (agent_tokens as f64 / total_tokens as f64);
                }
            }
        }
    }

    if models.is_empty() {
        let empty = Paragraph::new(Span::styled("  no efficiency data", theme.muted()));
        frame.render_widget(empty, inner);
        return;
    }

    // Build table rows.
    let header = Row::new(vec![
        Cell::from(Span::styled("Model", Style::default().fg(theme.muted))),
        Cell::from(Span::styled("Provider", Style::default().fg(theme.muted))),
        Cell::from(Span::styled("Tasks", Style::default().fg(theme.muted))),
        Cell::from(Span::styled("Pass%", Style::default().fg(theme.muted))),
        Cell::from(Span::styled("Avg Dur", Style::default().fg(theme.muted))),
        Cell::from(Span::styled("Cost", Style::default().fg(theme.muted))),
        Cell::from(Span::styled("$/Task", Style::default().fg(theme.muted))),
    ])
    .height(1);

    let rows: Vec<Row<'_>> = models
        .iter()
        .map(|(model, entry)| {
            let pass_pct = entry.pass_pct();
            let pass_style = if pass_pct > 80.0 {
                theme.success()
            } else if pass_pct >= 50.0 {
                theme.warning()
            } else if entry.tasks > 0 {
                theme.danger()
            } else {
                theme.muted()
            };

            let model_display = truncate_model(model, 24);
            let provider_display = truncate_model(&entry.provider, 12);

            Row::new(vec![
                Cell::from(Span::styled(
                    model_display,
                    Style::default().fg(theme.foreground),
                )),
                Cell::from(Span::styled(
                    provider_display,
                    Style::default().fg(theme.muted),
                )),
                Cell::from(Span::styled(
                    format!("{:>5}", entry.tasks),
                    Style::default().fg(theme.foreground),
                )),
                Cell::from(Span::styled(format!("{pass_pct:>5.1}%"), pass_style)),
                Cell::from(Span::styled(
                    format_duration(entry.avg_duration_ms()),
                    Style::default().fg(theme.foreground),
                )),
                Cell::from(Span::styled(
                    format_cost(entry.total_cost_usd),
                    cost_style(entry.total_cost_usd, theme),
                )),
                Cell::from(Span::styled(
                    format_cost(entry.cost_per_task()),
                    Style::default().fg(theme.muted),
                )),
            ])
        })
        .collect();

    // Add a totals row.
    let total_tasks: u64 = models.values().map(|e| e.tasks).sum();
    let total_passed: u64 = models.values().map(|e| e.passed).sum();
    let total_cost: f64 = models.values().map(|e| e.total_cost_usd).sum();
    let total_dur: u64 = models.values().map(|e| e.total_duration_ms).sum();
    let overall_pass_pct = if total_tasks > 0 {
        total_passed as f64 / total_tasks as f64 * 100.0
    } else {
        0.0
    };
    let overall_avg_dur = if total_tasks > 0 {
        total_dur as f64 / total_tasks as f64
    } else {
        0.0
    };
    let overall_cpt = if total_tasks > 0 {
        total_cost / total_tasks as f64
    } else {
        0.0
    };
    let total_pass_style = if overall_pass_pct > 80.0 {
        theme.success()
    } else if overall_pass_pct >= 50.0 {
        theme.warning()
    } else if total_tasks > 0 {
        theme.danger()
    } else {
        theme.muted()
    };

    let mut all_rows = rows;
    all_rows.push(
        Row::new(vec![
            Cell::from(Span::styled(
                "TOTAL",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled("", Style::default())),
            Cell::from(Span::styled(
                format!("{total_tasks:>5}"),
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                format!("{overall_pass_pct:>5.1}%"),
                total_pass_style,
            )),
            Cell::from(Span::styled(
                format_duration(overall_avg_dur),
                Style::default().fg(theme.foreground),
            )),
            Cell::from(Span::styled(
                format_cost(total_cost),
                cost_style(total_cost, theme).add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                format_cost(overall_cpt),
                Style::default().fg(theme.muted),
            )),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD)),
    );

    let widths = [
        ratatui::layout::Constraint::Min(16),
        ratatui::layout::Constraint::Min(10),
        ratatui::layout::Constraint::Length(7),
        ratatui::layout::Constraint::Length(7),
        ratatui::layout::Constraint::Length(8),
        ratatui::layout::Constraint::Length(9),
        ratatui::layout::Constraint::Length(9),
    ];

    let table = Table::new(all_rows, widths)
        .header(header)
        .row_highlight_style(theme.selection());

    frame.render_widget(table, inner);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate_model(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len < 5 {
        s[..max_len].to_string()
    } else {
        let half = (max_len - 2) / 2;
        let end_start = s.len() - (max_len - half - 2);
        format!("{}..{}", &s[..half], &s[end_start..])
    }
}

fn format_duration(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{ms:>5.0}ms")
    } else if ms < 60_000.0 {
        format!("{:>5.1}s", ms / 1000.0)
    } else {
        format!("{:>5.1}m", ms / 60_000.0)
    }
}

fn format_cost(usd: f64) -> String {
    if usd < 0.001 {
        "     -".to_string()
    } else if usd < 1.0 {
        format!("${usd:>7.4}")
    } else if usd < 10.0 {
        format!("${usd:>7.3}")
    } else {
        format!("${usd:>7.2}")
    }
}

fn cost_style(usd: f64, theme: &Theme) -> Style {
    if usd >= 5.0 {
        theme.danger()
    } else if usd >= 1.0 {
        theme.warning()
    } else {
        Style::default().fg(theme.foreground)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn cost_by_model_renders_without_panic() {
        let backend = TestBackend::new(100, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let state = TuiState::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_cost_by_model_table(frame, area, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn cost_by_model_with_events() {
        let backend = TestBackend::new(100, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let mut state = TuiState::new();

        let mut event = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        event.model = "claude-sonnet-4-20250514".into();
        event.backend = "anthropic-api".into();
        event.cost_usd = 0.05;
        event.wall_time_ms = 12_000;
        event.gate_passed = Some(true);
        event.is_final_turn = true;
        state.efficiency_events.push(event);

        let mut event2 = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        event2.model = "gpt-4o".into();
        event2.backend = "openai".into();
        event2.cost_usd = 0.03;
        event2.wall_time_ms = 8_000;
        event2.gate_passed = Some(false);
        event2.is_final_turn = true;
        state.efficiency_events.push(event2);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_cost_by_model_table(frame, area, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn truncate_model_works() {
        assert_eq!(truncate_model("short", 10), "short");
        assert_eq!(truncate_model("a-very-long-model-name-here", 14).len(), 14);
    }

    #[test]
    fn format_helpers() {
        assert!(format_duration(500.0).contains("ms"));
        assert!(format_duration(5_000.0).contains("s"));
        assert!(format_duration(120_000.0).contains("m"));
        assert!(format_cost(0.0).contains('-'));
        assert!(format_cost(0.05).contains('$'));
    }
}
