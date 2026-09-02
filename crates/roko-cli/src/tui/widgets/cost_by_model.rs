//! Per-model cost statistics table widget.
//!
//! Columns: Model | Provider | Tasks | Pass% | Avg Duration | Total Cost | $/Task | Distribution
//! Pass rates are color-coded: green >80%, amber 50-80%, red <50%.
//!
//! Features:
//!   - Sortable columns (cycle with `s` key via `CostSortMode`)
//!   - Per-provider subtotal rows
//!   - Gradient braille distribution bars
//!   - Per-model cost sparkline in distribution column

use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use super::braille;
use crate::tui::dashboard::Theme;
use crate::tui::state::{CostSortMode, TuiState};

/// Aggregated per-model cost entry.
#[derive(Debug, Clone, Default)]
struct ModelCostEntry {
    provider: String,
    tasks: u64,
    passed: u64,
    total_duration_ms: u64,
    total_cost_usd: f64,
    /// Per-event cost timeline for sparkline rendering.
    cost_timeline: Vec<f64>,
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

/// Per-provider subtotal for grouping.
#[derive(Debug, Clone, Default)]
struct ProviderSubtotal {
    tasks: u64,
    passed: u64,
    total_cost_usd: f64,
    total_duration_ms: u64,
}

/// Render the "Cost by Model" table.
pub fn render_cost_by_model_table(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let sort_label = tui_state.cost_sort_mode.label();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" Cost by Model [s:sort by {sort_label}] "),
            Theme::title_style(),
        ))
        .border_style(Theme::unfocused_border_style())
        .style(Theme::block_style());
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
        entry.cost_timeline.push(event.cost_usd);
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
            // as a duration proxy (1 token ~ 1ms for display purposes).
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
        let empty = Paragraph::new(Span::styled(
            "  No cost data yet \u{2014} breakdowns appear after agent turns",
            theme.muted(),
        ));
        frame.render_widget(empty, inner);
        return;
    }

    // Sort models according to the active sort mode.
    let mut sorted: Vec<(String, ModelCostEntry)> = models.into_iter().collect();
    match tui_state.cost_sort_mode {
        CostSortMode::Name => sorted.sort_by(|a, b| a.0.cmp(&b.0)),
        CostSortMode::Cost => {
            sorted.sort_by(|a, b| b.1.total_cost_usd.partial_cmp(&a.1.total_cost_usd).unwrap_or(std::cmp::Ordering::Equal));
        }
        CostSortMode::Tasks => sorted.sort_by(|a, b| b.1.tasks.cmp(&a.1.tasks)),
        CostSortMode::PassRate => {
            sorted.sort_by(|a, b| b.1.pass_pct().partial_cmp(&a.1.pass_pct()).unwrap_or(std::cmp::Ordering::Equal));
        }
    }

    // Build provider subtotals.
    let mut provider_totals: BTreeMap<String, ProviderSubtotal> = BTreeMap::new();
    for (_, entry) in &sorted {
        let prov = if entry.provider.is_empty() {
            "(unknown)".to_string()
        } else {
            entry.provider.clone()
        };
        let pt = provider_totals.entry(prov).or_default();
        pt.tasks += entry.tasks;
        pt.passed += entry.passed;
        pt.total_cost_usd += entry.total_cost_usd;
        pt.total_duration_ms += entry.total_duration_ms;
    }

    // Compute max cost for relative bar sizing.
    let max_cost = sorted
        .iter()
        .map(|(_, e)| e.total_cost_usd)
        .fold(0.01_f64, f64::max);

    // Column sort indicator.
    let sort_indicator = |col: &str, mode: CostSortMode| -> Span<'static> {
        let active = tui_state.cost_sort_mode == mode;
        let label = if active {
            format!("{col} \u{25bc}")
        } else {
            col.to_string()
        };
        Span::styled(
            label,
            if active {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            },
        )
    };

    // Build table rows with inline cost distribution bars.
    let header = Row::new(vec![
        Cell::from(sort_indicator("Model", CostSortMode::Name)),
        Cell::from(Span::styled("Provider", Style::default().fg(theme.muted))),
        Cell::from(sort_indicator("Tasks", CostSortMode::Tasks)),
        Cell::from(sort_indicator("Pass%", CostSortMode::PassRate)),
        Cell::from(Span::styled("Avg Dur", Style::default().fg(theme.muted))),
        Cell::from(sort_indicator("Cost", CostSortMode::Cost)),
        Cell::from(Span::styled("$/Task", Style::default().fg(theme.muted))),
        Cell::from(Span::styled(
            "Distribution",
            Style::default().fg(theme.muted),
        )),
    ])
    .height(1);

    let bar_col_width = 12usize;

    // Group rows by provider, inserting subtotal rows after each provider group.
    let mut all_rows: Vec<Row<'_>> = Vec::new();
    let mut current_provider: Option<String> = None;
    let multi_provider = provider_totals.len() > 1;

    for (model, entry) in &sorted {
        let this_provider = if entry.provider.is_empty() {
            "(unknown)".to_string()
        } else {
            entry.provider.clone()
        };

        // Insert provider subtotal row when provider changes (only if >1 providers).
        if multi_provider {
            if let Some(ref prev) = current_provider {
                if *prev != this_provider {
                    if let Some(pt) = provider_totals.get(prev) {
                        all_rows.push(provider_subtotal_row(prev, pt, bar_col_width, theme));
                    }
                }
            }
            current_provider = Some(this_provider.clone());
        }

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

        // Gradient braille sparkline for cost distribution.
        let cost_frac = (entry.total_cost_usd / max_cost).clamp(0.0, 1.0);
        let bar_data: Vec<f64> = if entry.cost_timeline.len() >= 2 {
            // Show cost-over-time sparkline
            entry.cost_timeline.clone()
        } else {
            // Fall back to a proportional bar
            vec![cost_frac]
        };
        let distribution_cell = if bar_data.len() >= 2 {
            // Cost-over-time sparkline using gradient
            let spans = braille::braille_spans_gradient(
                &bar_data,
                0.0, // auto-scale
                bar_col_width,
                Theme::ROSE_DIM,
                Theme::ROSE,
            );
            Cell::from(Line::from(spans))
        } else {
            // Single-value proportional bar
            let filled = (cost_frac * bar_col_width as f64).round() as usize;
            let empty = bar_col_width.saturating_sub(filled);
            let bar_str = format!(
                "{}{}",
                "\u{2588}".repeat(filled),
                "\u{2500}".repeat(empty),
            );
            Cell::from(Span::styled(bar_str, cost_style(entry.total_cost_usd, theme)))
        };

        all_rows.push(Row::new(vec![
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
            distribution_cell,
        ]));
    }

    // Final provider subtotal (if multi-provider).
    if multi_provider {
        if let Some(ref prev) = current_provider {
            if let Some(pt) = provider_totals.get(prev) {
                all_rows.push(provider_subtotal_row(prev, pt, bar_col_width, theme));
            }
        }
    }

    // Add a totals row.
    let total_tasks: u64 = sorted.iter().map(|(_, e)| e.tasks).sum();
    let total_passed: u64 = sorted.iter().map(|(_, e)| e.passed).sum();
    let total_cost: f64 = sorted.iter().map(|(_, e)| e.total_cost_usd).sum();
    let total_dur: u64 = sorted.iter().map(|(_, e)| e.total_duration_ms).sum();
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
            Cell::from(Span::styled(
                "\u{2588}".repeat(bar_col_width),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
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
        ratatui::layout::Constraint::Length(bar_col_width as u16),
    ];

    let table = Table::new(all_rows, widths)
        .header(header)
        .row_highlight_style(theme.selection());

    frame.render_widget(table, inner);
}

// ---------------------------------------------------------------------------
// Provider subtotal row
// ---------------------------------------------------------------------------

fn provider_subtotal_row<'a>(
    provider: &str,
    pt: &ProviderSubtotal,
    bar_col_width: usize,
    theme: &Theme,
) -> Row<'a> {
    let pass_pct = if pt.tasks > 0 {
        pt.passed as f64 / pt.tasks as f64 * 100.0
    } else {
        0.0
    };
    let avg_dur = if pt.tasks > 0 {
        pt.total_duration_ms as f64 / pt.tasks as f64
    } else {
        0.0
    };
    let cpt = if pt.tasks > 0 {
        pt.total_cost_usd / pt.tasks as f64
    } else {
        0.0
    };
    let pass_style = if pass_pct > 80.0 {
        theme.success()
    } else if pass_pct >= 50.0 {
        theme.warning()
    } else if pt.tasks > 0 {
        theme.danger()
    } else {
        theme.muted()
    };

    let label = format!("\u{2514} {}", truncate_model(provider, 20));

    Row::new(vec![
        Cell::from(Span::styled(
            label,
            Style::default()
                .fg(Theme::BONE_DIM)
                .add_modifier(Modifier::DIM),
        )),
        Cell::from(Span::styled("", Style::default())),
        Cell::from(Span::styled(
            format!("{:>5}", pt.tasks),
            Style::default().fg(Theme::BONE_DIM),
        )),
        Cell::from(Span::styled(
            format!("{pass_pct:>5.1}%"),
            pass_style.add_modifier(Modifier::DIM),
        )),
        Cell::from(Span::styled(
            format_duration(avg_dur),
            Style::default().fg(Theme::BONE_DIM),
        )),
        Cell::from(Span::styled(
            format_cost(pt.total_cost_usd),
            Style::default().fg(Theme::BONE_DIM),
        )),
        Cell::from(Span::styled(
            format_cost(cpt),
            Style::default().fg(Theme::TEXT_GHOST),
        )),
        Cell::from(Span::styled(
            "\u{2500}".repeat(bar_col_width),
            Style::default().fg(Theme::TEXT_GHOST),
        )),
    ])
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
    fn cost_by_model_sorted_by_cost() {
        let backend = TestBackend::new(100, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let mut state = TuiState::new();
        state.cost_sort_mode = CostSortMode::Cost;

        let mut event = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        event.model = "cheap-model".into();
        event.backend = "provider-a".into();
        event.cost_usd = 0.01;
        event.is_final_turn = true;
        state.efficiency_events.push(event);

        let mut event2 = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        event2.model = "expensive-model".into();
        event2.backend = "provider-b".into();
        event2.cost_usd = 1.50;
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

    #[test]
    fn sort_mode_cycles() {
        let mode = CostSortMode::default();
        assert_eq!(mode, CostSortMode::Name);
        assert_eq!(mode.next(), CostSortMode::Cost);
        assert_eq!(mode.next().next(), CostSortMode::Tasks);
        assert_eq!(mode.next().next().next(), CostSortMode::PassRate);
        assert_eq!(mode.next().next().next().next(), CostSortMode::Name);
    }
}
