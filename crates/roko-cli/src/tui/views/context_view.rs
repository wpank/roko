//! F7 Inspect / Context view -- token burn, cost breakdown, routing, health.
//!
//! Four-section layout:
//! - Top 20%: system health summary + C-Factor
//! - Mid-left 40%: token burn per role from efficiency events
//! - Mid-right 40%: cost breakdown per model
//! - Bottom 40%: cascade router decisions + conductor alerts

use std::collections::{BTreeMap, HashMap};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::input::FocusZone;
use crate::tui::state::TuiState;

/// Token burn data for sparkline rendering.
#[derive(Debug, Clone)]
struct TokenBurnData {
    /// Agent identifier for the burn series.
    pub agent_id: String,
    /// Cumulative token count over time.
    pub cumulative: Vec<u64>,
}

/// Context view data container, populated externally.
#[derive(Debug, Clone, Default)]
struct ContextViewData {
    /// Per-agent token burn series used by the sparkline panel.
    pub token_burns: Vec<TokenBurnData>,
}

/// Per-role token/cost aggregate.
#[derive(Debug, Clone, Default)]
struct RoleAggregate {
    input_tokens: u64,
    output_tokens: u64,
    cost_usd: f64,
    turns: u64,
    cache_read_tokens: u64,
}

/// Per-model cost aggregate.
#[derive(Debug, Clone, Default)]
struct ModelCostAggregate {
    input_tokens: u64,
    output_tokens: u64,
    cost_usd: f64,
    turns: u64,
    wall_time_ms: u64,
}

/// Render the full context/inspect view.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    match view_state.sub_tab {
        1 => render_signal_dag(frame, area, tui_state, view_state, theme),
        2 => render_episode_replay(frame, area, tui_state, view_state, theme),
        3 => render_knowledge_browse(frame, area, tui_state, view_state, theme),
        4 => {
            crate::tui::widgets::cost_by_model::render_cost_by_model_table(
                frame, area, tui_state, theme,
            );
        }
        5 => render_three_panel_inspect(frame, area, tui_state, theme),
        6 => render_cfactor_detail(frame, area, tui_state, theme),
        _ => {
            let ctx_data = build_context_data(tui_state);
            render_with_context_data(
                frame,
                area,
                tui_state,
                &ctx_data,
                view_state,
                theme,
                matches!(tui_state.focus, FocusZone::RightPanel),
            );
        }
    }
}

/// Render the context view with explicit context data (for integration layer).
fn render_with_context_data(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    ctx_data: &ContextViewData,
    view_state: &ViewState,
    theme: &Theme,
    focused: bool,
) {
    let sections = Layout::vertical([
        Constraint::Percentage(20), // Health summary
        Constraint::Percentage(40), // Token burn + cost breakdown side by side
        Constraint::Percentage(40), // Cascade router + alerts
    ])
    .split(area);

    render_health_summary(frame, sections[0], tui_state, focused, theme);

    let mid_panels = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[1]);
    render_token_burn_by_role(frame, mid_panels[0], tui_state, view_state, focused, theme);
    render_cost_by_model(frame, mid_panels[1], tui_state, focused, theme);

    let bottom_panels =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(sections[2]);
    render_cascade_router(
        frame,
        bottom_panels[0],
        tui_state,
        ctx_data,
        view_state,
        focused,
        theme,
    );
    render_alerts_and_health(frame, bottom_panels[1], tui_state, focused, theme);
}

/// Top section: system health summary with C-Factor and key metrics.
fn render_health_summary(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    focused: bool,
    theme: &Theme,
) {
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        theme.accent()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        theme.accent()
    };
    let block = Block::bordered()
        .title(Span::styled(" System Health ", title_style))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .split(inner);

    // Left column: token/cost summary
    let eff = &tui_state.efficiency_summary;
    let token_lines = vec![
        Line::from(vec![
            Span::styled("input tokens:  ", theme.muted()),
            Span::styled(format_count(eff.total_input_tokens), theme.info()),
        ]),
        Line::from(vec![
            Span::styled("output tokens: ", theme.muted()),
            Span::styled(format_count(eff.total_output_tokens), theme.info()),
        ]),
        Line::from(vec![
            Span::styled("total cost:    ", theme.muted()),
            Span::styled(format!("${:.4}", eff.total_cost_usd), theme.warning()),
        ]),
        Line::from(vec![
            Span::styled("avg wall time: ", theme.muted()),
            Span::styled(format!("{:.0}ms", eff.average_wall_time_ms), theme.info()),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(token_lines).wrap(Wrap { trim: false }),
        cols[0],
    );

    // Middle column: pass rate and event counts
    let pass_rate = if eff.event_count > 0 {
        eff.passed_count as f64 / eff.event_count as f64 * 100.0
    } else {
        0.0
    };
    let pass_style = if pass_rate >= 80.0 {
        theme.success()
    } else if pass_rate >= 50.0 {
        theme.warning()
    } else {
        theme.danger()
    };

    let rate_lines = vec![
        Line::from(vec![
            Span::styled("pass rate:     ", theme.muted()),
            Span::styled(format!("{pass_rate:.1}%"), pass_style),
        ]),
        Line::from(vec![
            Span::styled("events:        ", theme.muted()),
            Span::raw(eff.event_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("agents:        ", theme.muted()),
            Span::raw(tui_state.agent_summaries.len().to_string()),
        ]),
        Line::from(vec![
            Span::styled("plans:         ", theme.muted()),
            Span::raw(tui_state.plan_summaries.len().to_string()),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(rate_lines).wrap(Wrap { trim: false }),
        cols[1],
    );

    // Right column: C-Factor or cascade router summary
    let mut right_lines = if let Some(ref cf) = tui_state.cfactor {
        let cf_style = if cf.overall >= 0.7 {
            theme.success()
        } else if cf.overall >= 0.4 {
            theme.warning()
        } else {
            theme.danger()
        };
        vec![
            Line::from(vec![
                Span::styled("C-Factor:      ", theme.muted()),
                Span::styled(format!("{:.3}", cf.overall), cf_style),
            ]),
            Line::from(vec![
                Span::styled("  gate pass:   ", theme.muted()),
                Span::raw(format!("{:.2}", cf.components.gate_pass_rate)),
            ]),
            Line::from(vec![
                Span::styled("  cost eff:    ", theme.muted()),
                Span::raw(format!("{:.2}", cf.components.cost_efficiency)),
            ]),
            Line::from(vec![
                Span::styled("  first try:   ", theme.muted()),
                Span::raw(format!("{:.2}", cf.components.first_try_rate)),
            ]),
        ]
    } else {
        let router_models = tui_state.cascade_router.model_slugs.len();
        let total_trials: u64 = tui_state
            .cascade_router
            .confidence_stats
            .values()
            .map(|s| s.trials)
            .sum();
        vec![
            Line::from(vec![
                Span::styled("C-Factor:      ", theme.muted()),
                Span::styled("(not computed)", theme.muted()),
            ]),
            Line::from(vec![
                Span::styled("router models: ", theme.muted()),
                Span::raw(router_models.to_string()),
            ]),
            Line::from(vec![
                Span::styled("router trials: ", theme.muted()),
                Span::raw(total_trials.to_string()),
            ]),
            Line::from(vec![
                Span::styled("gate types:    ", theme.muted()),
                Span::raw(tui_state.gate_results_page.gate_rows.len().to_string()),
            ]),
        ]
    };
    if let Some(eta) = tui_state.critical_path_eta_minutes {
        right_lines.push(Line::from(vec![
            Span::styled("crit-path ETA: ", theme.muted()),
            Span::styled(format!("{eta}m"), theme.info()),
        ]));
    }
    frame.render_widget(
        Paragraph::new(right_lines).wrap(Wrap { trim: false }),
        cols[2],
    );
}

/// Token burn per role from efficiency events.
fn render_token_burn_by_role(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    _view_state: &ViewState,
    focused: bool,
    theme: &Theme,
) {
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        theme.accent()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        theme.accent()
    };
    let block = Block::bordered()
        .title(Span::styled(" Token Burn by Role ", title_style))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tui_state.efficiency_events.is_empty() {
        let empty =
            Paragraph::new("No efficiency data yet \u{2014} token burn appears after agent turns")
                .style(theme.muted())
                .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    // Aggregate by role
    let mut role_agg: BTreeMap<String, RoleAggregate> = BTreeMap::new();
    for event in &tui_state.efficiency_events {
        let role = if event.role.is_empty() {
            "unknown"
        } else {
            event.role.as_str()
        };
        let entry = role_agg.entry(role.to_string()).or_default();
        entry.input_tokens += event.input_tokens;
        entry.output_tokens += event.output_tokens;
        entry.cost_usd += event.cost_usd;
        entry.turns += 1;
        entry.cache_read_tokens += event.cache_read_tokens;
    }

    let rows: Vec<Row<'_>> = role_agg
        .iter()
        .map(|(role, agg)| {
            let total_tokens = agg.input_tokens + agg.output_tokens;
            let cache_pct = if agg.input_tokens > 0 {
                format!(
                    "{:.0}%",
                    agg.cache_read_tokens as f64 / agg.input_tokens as f64 * 100.0
                )
            } else {
                "-".to_string()
            };
            Row::new(vec![
                Cell::from(truncate(role, 14)),
                Cell::from(format_count(total_tokens)),
                Cell::from(format!("${:.3}", agg.cost_usd)),
                Cell::from(agg.turns.to_string()),
                Cell::from(cache_pct),
            ])
        })
        .collect();

    // Total row
    let total_tokens: u64 = role_agg
        .values()
        .map(|a| a.input_tokens + a.output_tokens)
        .sum();
    let total_cost: f64 = role_agg.values().map(|a| a.cost_usd).sum();
    let total_turns: u64 = role_agg.values().map(|a| a.turns).sum();
    let total_cache: u64 = role_agg.values().map(|a| a.cache_read_tokens).sum();
    let total_input: u64 = role_agg.values().map(|a| a.input_tokens).sum();
    let total_cache_pct = if total_input > 0 {
        format!("{:.0}%", total_cache as f64 / total_input as f64 * 100.0)
    } else {
        "-".to_string()
    };

    let mut all_rows = rows;
    all_rows.push(
        Row::new(vec![
            Cell::from(Span::styled("TOTAL", theme.accent_bold())),
            Cell::from(Span::styled(format_count(total_tokens), theme.accent())),
            Cell::from(Span::styled(format!("${:.3}", total_cost), theme.warning())),
            Cell::from(Span::styled(total_turns.to_string(), theme.accent())),
            Cell::from(total_cache_pct),
        ])
        .style(theme.accent()),
    );

    let widths = [
        Constraint::Min(10),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(6),
    ];
    let table = Table::new(all_rows, widths)
        .header(
            Row::new(["role", "tokens", "cost", "turns", "cache"])
                .style(theme.accent().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1);
    frame.render_widget(table, inner);
}

/// Cost breakdown per model from efficiency events.
fn render_cost_by_model(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    focused: bool,
    theme: &Theme,
) {
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        theme.accent()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        theme.accent()
    };
    let block = Block::bordered()
        .title(Span::styled(" Cost by Model ", title_style))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tui_state.efficiency_events.is_empty() {
        let empty = Paragraph::new("No cost data yet \u{2014} breakdowns appear after agent turns")
            .style(theme.muted())
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    // Aggregate by model
    let mut model_agg: BTreeMap<String, ModelCostAggregate> = BTreeMap::new();
    for event in &tui_state.efficiency_events {
        let model = if event.model.is_empty() {
            "unknown"
        } else {
            event.model.as_str()
        };
        let entry = model_agg.entry(model.to_string()).or_default();
        entry.input_tokens += event.input_tokens;
        entry.output_tokens += event.output_tokens;
        entry.cost_usd += event.cost_usd;
        entry.turns += 1;
        entry.wall_time_ms += event.wall_time_ms;
    }

    // Sort by cost descending, filter inactive models
    let mut sorted: Vec<(&String, &ModelCostAggregate)> = model_agg.iter().collect();
    sorted.sort_by(|a, b| {
        b.1.cost_usd
            .partial_cmp(&a.1.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let total_before_filter = sorted.len();
    sorted.retain(|(_m, agg)| agg.turns > 0 || agg.cost_usd > 0.0);
    let hidden_models = total_before_filter - sorted.len();

    let rows: Vec<Row<'_>> = sorted
        .iter()
        .map(|(model, agg)| {
            let cost_per_turn = if agg.turns > 0 {
                format!("${:.4}", agg.cost_usd / agg.turns as f64)
            } else {
                "-".to_string()
            };
            let cost_style = if agg.cost_usd > 1.0 {
                theme.danger()
            } else if agg.cost_usd > 0.1 {
                theme.warning()
            } else {
                theme.text()
            };
            Row::new(vec![
                Cell::from(truncate(&display_model(Some(model.as_str())), 20)),
                Cell::from(Span::styled(format!("${:.4}", agg.cost_usd), cost_style)),
                Cell::from(agg.turns.to_string()),
                Cell::from(cost_per_turn),
                Cell::from(format_count(agg.input_tokens + agg.output_tokens)),
            ])
        })
        .collect();

    // Total row
    let total_cost: f64 = model_agg.values().map(|a| a.cost_usd).sum();
    let total_turns: u64 = model_agg.values().map(|a| a.turns).sum();
    let total_tokens: u64 = model_agg
        .values()
        .map(|a| a.input_tokens + a.output_tokens)
        .sum();
    let total_cpt = if total_turns > 0 {
        format!("${:.4}", total_cost / total_turns as f64)
    } else {
        "-".to_string()
    };
    let mut all_rows = rows;
    all_rows.push(
        Row::new(vec![
            Cell::from(Span::styled("TOTAL", theme.accent_bold())),
            Cell::from(Span::styled(format!("${total_cost:.4}"), theme.warning())),
            Cell::from(Span::styled(total_turns.to_string(), theme.accent())),
            Cell::from(total_cpt),
            Cell::from(Span::styled(format_count(total_tokens), theme.accent())),
        ])
        .style(theme.accent()),
    );

    if hidden_models > 0 {
        all_rows.push(Row::new(vec![Cell::from(Span::styled(
            format!("({hidden_models} inactive models hidden)"),
            theme.muted(),
        ))]));
    }

    let widths = [
        Constraint::Min(12),
        Constraint::Length(9),
        Constraint::Length(5),
        Constraint::Length(7),
        Constraint::Length(7),
    ];
    let table = Table::new(all_rows, widths)
        .header(
            Row::new(["model", "cost", "turns", "$/turn", "tokens"])
                .style(theme.accent().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1);
    frame.render_widget(table, inner);
}

/// Cascade router decisions and model routing info.
fn render_cascade_router(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    ctx_data: &ContextViewData,
    _view_state: &ViewState,
    focused: bool,
    theme: &Theme,
) {
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        theme.accent()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        theme.accent()
    };
    let block = Block::bordered()
        .title(Span::styled(" Cascade Route ", title_style))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).split(inner);

    // Route model stats
    if tui_state.cascade_router.model_slugs.is_empty() && ctx_data.token_burns.is_empty() {
        let empty = Paragraph::new("No routing decisions yet \u{2014} run agents to populate")
            .style(theme.muted())
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, sections[0]);
    } else {
        let total_slugs = tui_state.cascade_router.model_slugs.len();
        let active_slugs: Vec<&String> = tui_state
            .cascade_router
            .model_slugs
            .iter()
            .filter(|slug| {
                tui_state
                    .cascade_router
                    .confidence_stats
                    .get(*slug)
                    .map_or(false, |s| s.trials > 0)
            })
            .collect();
        let hidden_cascade = total_slugs - active_slugs.len();

        let mut rows: Vec<Row<'_>> = active_slugs
            .iter()
            .map(|slug| {
                let stats = tui_state.cascade_router.confidence_stats.get(*slug);
                let trials = stats.map_or(0, |s| s.trials);
                let successes = stats.map_or(0, |s| s.successes);
                let rate = if trials > 0 {
                    successes as f64 / trials as f64 * 100.0
                } else {
                    0.0
                };
                let rate_style = if rate >= 80.0 {
                    theme.success()
                } else if rate >= 50.0 {
                    theme.warning()
                } else if trials > 0 {
                    theme.danger()
                } else {
                    theme.muted()
                };
                Row::new(vec![
                    Cell::from(truncate(&display_model(Some(slug.as_str())), 20)),
                    Cell::from(trials.to_string()),
                    Cell::from(successes.to_string()),
                    Cell::from(Span::styled(format!("{rate:.0}%"), rate_style)),
                ])
            })
            .collect();

        if hidden_cascade > 0 {
            rows.push(Row::new(vec![Cell::from(Span::styled(
                format!("({hidden_cascade} inactive hidden)"),
                theme.muted(),
            ))]));
        }

        if rows.is_empty() {
            // Show sparklines from token burns instead
            let burn_lines: Vec<Line<'_>> = ctx_data
                .token_burns
                .iter()
                .take(sections[0].height as usize)
                .map(|burn| {
                    let total: u64 = burn.cumulative.last().copied().unwrap_or(0);
                    Line::from(vec![
                        Span::styled(
                            format!("{:>16}", truncate(&burn.agent_id, 16)),
                            theme.muted(),
                        ),
                        Span::raw(" "),
                        Span::styled(format_count(total), theme.info()),
                        Span::styled(" tokens", theme.muted()),
                    ])
                })
                .collect();
            frame.render_widget(
                Paragraph::new(burn_lines).wrap(Wrap { trim: false }),
                sections[0],
            );
        } else {
            let widths = [
                Constraint::Min(14),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(6),
            ];
            let table = Table::new(rows, widths)
                .header(
                    Row::new(["model", "tries", "wins", "rate"])
                        .style(theme.accent().add_modifier(Modifier::BOLD)),
                )
                .column_spacing(1);
            frame.render_widget(table, sections[0]);
        }
    }

    // Summary line at the bottom
    let total_trials: u64 = tui_state
        .cascade_router
        .confidence_stats
        .values()
        .map(|s| s.trials)
        .sum();
    let total_success: u64 = tui_state
        .cascade_router
        .confidence_stats
        .values()
        .map(|s| s.successes)
        .sum();
    let overall_rate = if total_trials > 0 {
        format!("{:.1}%", total_success as f64 / total_trials as f64 * 100.0)
    } else {
        "-".to_string()
    };

    let summary = Paragraph::new(vec![Line::from(vec![
        Span::styled("models: ", theme.muted()),
        Span::raw(tui_state.cascade_router.model_slugs.len().to_string()),
        Span::styled("  trials: ", theme.muted()),
        Span::raw(total_trials.to_string()),
        Span::styled("  success: ", theme.muted()),
        Span::raw(overall_rate),
    ])])
    .wrap(Wrap { trim: false });
    frame.render_widget(summary, sections[1]);
}

/// Conductor alerts and gate threshold health.
fn render_alerts_and_health(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    focused: bool,
    theme: &Theme,
) {
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        theme.muted()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        theme.muted()
    };
    let block = Block::bordered()
        .title(Span::styled(" Alerts & Gates ", title_style))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);

    // Conductor alerts
    if tui_state.conductor_alerts.is_empty() {
        let empty = Paragraph::new("No conductor alerts \u{2014} all systems nominal")
            .style(theme.success())
            .alignment(Alignment::Center);
        frame.render_widget(empty, sections[0]);
    } else {
        let items: Vec<ListItem<'_>> = tui_state
            .conductor_alerts
            .iter()
            .take(sections[0].height as usize)
            .map(|alert| {
                let severity_style = match alert.severity.as_str() {
                    "error" | "critical" => theme.danger(),
                    "warning" | "warn" => theme.warning(),
                    _ => theme.info(),
                };
                ListItem::new(Line::from(vec![
                    Span::styled(&alert.severity, severity_style),
                    Span::raw(": "),
                    Span::styled(truncate(&alert.message, 30), theme.text()),
                ]))
            })
            .collect();
        frame.render_widget(List::new(items), sections[0]);
    }

    // Verify threshold summary
    if tui_state.gate_results_page.threshold_rows.is_empty()
        && tui_state.gate_results_page.gate_rows.is_empty()
    {
        let empty = Paragraph::new("No gate data yet \u{2014} thresholds appear after gate runs")
            .style(theme.muted())
            .alignment(Alignment::Center);
        frame.render_widget(empty, sections[1]);
    } else {
        let rows: Vec<Row<'_>> = tui_state
            .gate_results_page
            .gate_rows
            .iter()
            .map(|row| {
                let rate_style = if row.pass_rate >= 0.8 {
                    theme.success()
                } else if row.pass_rate >= 0.5 {
                    theme.warning()
                } else {
                    theme.danger()
                };
                Row::new(vec![
                    Cell::from(truncate(&row.gate_name, 14)),
                    Cell::from(row.total_runs.to_string()),
                    Cell::from(Span::styled(
                        format!("{:.0}%", row.pass_rate * 100.0),
                        rate_style,
                    )),
                    Cell::from(format!("{:.0}ms", row.avg_duration_ms)),
                ])
            })
            .collect();

        let widths = [
            Constraint::Min(10),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(8),
        ];
        let table = Table::new(rows, widths)
            .header(
                Row::new(["gate", "runs", "pass", "avg"])
                    .style(theme.accent().add_modifier(Modifier::BOLD)),
            )
            .column_spacing(1);
        frame.render_widget(table, sections[1]);
    }
}

// ---------------------------------------------------------------------------
// Sub-view: Signal DAG (sub_tab == 1)
// ---------------------------------------------------------------------------

/// A single rendered row in the signal DAG tree.
struct DagRow {
    /// The full styled line to display.
    spans: Vec<(String, ratatui::style::Style)>,
    /// Index into `tui_state.recent_signals` for selection tracking.
    signal_index: usize,
}

/// Build a DFS-ordered list of [`DagRow`]s from a flat signal slice.
///
/// Signals that have a `parent_hash` pointing to another signal in the slice
/// are shown as children; all others are roots. Roots and siblings are sorted
/// by `created_at_ms` so the tree is chronologically ordered.
fn build_dag_rows(
    signals: &[crate::tui::dashboard_types::SignalSummary],
    theme: Theme,
) -> Vec<DagRow> {
    use std::collections::{HashMap, HashSet};

    // Map id → slice index for O(1) look-up.
    let id_to_idx: HashMap<&str, usize> = signals
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

    // Build adjacency: parent_id → sorted list of child indices.
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut has_parent: HashSet<usize> = HashSet::new();

    for (idx, sig) in signals.iter().enumerate() {
        if let Some(parent_id) = &sig.parent_hash {
            if let Some(&parent_idx) = id_to_idx.get(parent_id.as_str()) {
                children.entry(parent_idx).or_default().push(idx);
                has_parent.insert(idx);
            }
        }
    }

    // Sort each child list chronologically.
    for child_list in children.values_mut() {
        child_list.sort_by_key(|&i| signals[i].created_at_ms);
    }

    // Collect roots (no resolved parent), sorted chronologically.
    let mut roots: Vec<usize> = (0..signals.len())
        .filter(|i| !has_parent.contains(i))
        .collect();
    roots.sort_by_key(|&i| signals[i].created_at_ms);

    // DFS walk with prefix tracking.
    // `prefix_parts` is a stack of booleans: `true` means "this ancestor still
    // has siblings below me" (render │), `false` means it was the last child
    // (render   ).
    let mut rows: Vec<DagRow> = Vec::with_capacity(signals.len());

    fn walk(
        idx: usize,
        prefix_parts: &mut Vec<bool>,
        is_last: bool,
        signals: &[crate::tui::dashboard_types::SignalSummary],
        children: &std::collections::HashMap<usize, Vec<usize>>,
        rows: &mut Vec<DagRow>,
        theme: Theme,
    ) {
        let sig = &signals[idx];

        // Build the tree connector string from the ancestor prefix parts.
        let mut connector = String::new();
        for &has_more in prefix_parts.iter() {
            if has_more {
                connector.push_str("\u{2502}  "); // │  (vertical bar + 2 spaces)
            } else {
                connector.push_str("   "); // 3 spaces for a closed branch
            }
        }
        if prefix_parts.is_empty() {
            // Root node: just a horizontal line.
            connector.push_str("\u{2500}\u{2500} "); // ── (two dashes + space)
        } else if is_last {
            connector.push_str("\u{2514}\u{2500} "); // └─  (last child)
        } else {
            connector.push_str("\u{251C}\u{2500} "); // ├─  (non-last child)
        }

        // Timestamp: HH:MM:SS from epoch-ms.
        let ts = format_dag_timestamp(sig.created_at_ms);

        // Kind style: color by signal category.
        let kind_style = dag_kind_style(sig, theme);

        // Short hash (8 chars).
        let hash = truncate(&sig.id, 8);

        // Content preview: truncate to remaining width (generous budget).
        let preview = if sig.payload_preview.is_empty() {
            String::new()
        } else {
            truncate(&sig.payload_preview, 48)
        };

        // Confidence indicator (compact: a single char or digit).
        let conf_str = match sig.confidence {
            Some(c) => format!("{:.0}%", (c * 100.0).clamp(0.0, 100.0)),
            None => "   ".to_string(),
        };
        let conf_style = confidence_style(sig.confidence, &theme);

        // Assemble spans: connector | hash | kind | conf | preview
        let spans: Vec<(String, ratatui::style::Style)> = vec![
            (connector, theme.muted()),
            (hash, theme.muted()),
            (" ".to_string(), ratatui::style::Style::default()),
            (truncate(&sig.kind, 22), kind_style),
            (" ".to_string(), ratatui::style::Style::default()),
            (conf_str, conf_style),
            (" ".to_string(), ratatui::style::Style::default()),
            (ts, theme.muted()),
            (
                if preview.is_empty() {
                    String::new()
                } else {
                    format!("  {preview}")
                },
                theme.muted(),
            ),
        ];

        rows.push(DagRow {
            spans,
            signal_index: idx,
        });

        // Recurse into children.
        if let Some(kids) = children.get(&idx) {
            let last_kid = kids.len().saturating_sub(1);
            for (ki, &child_idx) in kids.iter().enumerate() {
                let child_is_last = ki == last_kid;
                // Push whether this level still has more siblings after this child.
                prefix_parts.push(!child_is_last);
                walk(
                    child_idx,
                    prefix_parts,
                    child_is_last,
                    signals,
                    children,
                    rows,
                    theme,
                );
                prefix_parts.pop();
            }
        }
    }

    let last_root = roots.len().saturating_sub(1);
    for (ri, &root_idx) in roots.iter().enumerate() {
        let mut prefix = Vec::new();
        walk(
            root_idx,
            &mut prefix,
            ri == last_root,
            signals,
            &children,
            &mut rows,
            theme,
        );
    }

    rows
}

/// Pick a style for the signal kind column based on the kind string.
fn dag_kind_style(
    sig: &crate::tui::dashboard_types::SignalSummary,
    theme: Theme,
) -> ratatui::style::Style {
    let kind = sig.kind.as_str();
    if kind.contains("gate:") {
        // Gate verdicts: green for pass, red for fail.
        let passed = !sig.payload_preview.contains("fail")
            && !sig.payload_preview.contains("error")
            && !sig.payload_preview.contains("reject");
        if passed {
            theme.success()
        } else {
            theme.danger()
        }
    } else if kind.contains("agent:")
        || kind.contains("episode")
        || kind.contains("dispatch")
        || kind.contains("model_select")
        || kind.contains("model_route")
    {
        // Agent outputs / routing: blue-ish (info).
        theme.info()
    } else if kind.contains("error") || kind.contains("fail") || kind.contains("reject") {
        theme.danger()
    } else if kind.contains("warn") {
        theme.warning()
    } else {
        theme.text()
    }
}

/// Format an epoch-millisecond timestamp as `HH:MM:SS`.
fn format_dag_timestamp(ms: i64) -> String {
    if ms <= 0 {
        return "--:--:--".to_string();
    }
    let secs = ms / 1000;
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

fn render_signal_dag(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let block = Block::bordered().title(Span::styled(
        " Signal DAG  [↑↓ scroll] ",
        theme.accent(),
    ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tui_state.recent_signals.is_empty() {
        let empty = Paragraph::new("No signals yet \u{2014} run agents to populate the signal DAG")
            .style(theme.muted())
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    // Build the DAG row list once per render.
    let dag_rows = build_dag_rows(&tui_state.recent_signals, *theme);
    let total_rows = dag_rows.len();
    let visible_height = inner.height as usize;

    let scroll = bounded_scroll(view_state.scroll as usize, total_rows, visible_height);
    let selected_abs = view_state.selected;

    // Convert each DagRow to a ratatui ListItem with styled Spans.
    let items: Vec<ListItem<'_>> = dag_rows
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(row_idx, dag_row)| {
            let is_selected = dag_row.signal_index == selected_abs
                || row_idx == selected_abs;
            let spans: Vec<Span<'_>> = if is_selected {
                // Highlight the whole line on selection.
                dag_row
                    .spans
                    .iter()
                    .map(|(text, _)| Span::styled(text.clone(), theme.selection()))
                    .collect()
            } else {
                dag_row
                    .spans
                    .iter()
                    .map(|(text, style)| Span::styled(text.clone(), *style))
                    .collect()
            };
            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut list_state = ListState::default();
    // Keep the ListState selection in sync with the relative cursor position.
    if selected_abs >= scroll && selected_abs.saturating_sub(scroll) < visible_height {
        list_state.select(Some(selected_abs.saturating_sub(scroll)));
    }

    let list = List::new(items);
    frame.render_stateful_widget(list, inner, &mut list_state);
}

// ---------------------------------------------------------------------------
// Sub-view: Episode Replay (sub_tab == 2)
// ---------------------------------------------------------------------------

fn render_episode_replay(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    // Determine focus zones.
    let list_focused = matches!(tui_state.focus, FocusZone::InspectTree);
    let detail_focused = matches!(tui_state.focus, FocusZone::InspectDetail);

    // Use a horizontal split when the terminal is wide enough, otherwise stack.
    let use_horizontal = area.width >= 100;
    let (list_area, detail_area) = if use_horizontal {
        let chunks = Layout::horizontal([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(area);
        (chunks[0], chunks[1])
    } else {
        let list_h = (area.height / 3).clamp(6, 14).min(area.height.saturating_sub(4).max(1));
        let chunks = Layout::vertical([
            Constraint::Length(list_h),
            Constraint::Min(0),
        ])
        .split(area);
        (chunks[0], chunks[1])
    };

    // ── Left panel: episode list ──────────────────────────────────────────────

    let list_border_style = if list_focused {
        Theme::focused_border_style()
    } else {
        Theme::unfocused_border_style()
    };
    let list_title_style = if list_focused {
        Theme::focused_title_style()
    } else {
        Theme::unfocused_title_style()
    };
    let list_hint = if list_focused {
        " [↓/↑] move  [→/l] detail "
    } else {
        " [Tab] focus "
    };
    let list_block = Block::bordered()
        .border_style(list_border_style)
        .title(Span::styled(" Episode List ", list_title_style))
        .title_bottom(Span::styled(list_hint, theme.muted()));
    let list_inner = list_block.inner(list_area);
    frame.render_widget(list_block, list_area);

    if tui_state.episodes_cache.is_empty() {
        let empty =
            Paragraph::new("No episodes yet \u{2014} agent turns populate the episode log")
                .style(theme.muted())
                .wrap(Wrap { trim: false });
        frame.render_widget(empty, list_inner);
        // Still render an empty detail panel.
        render_episode_detail_empty(frame, detail_area, detail_focused, theme);
        return;
    }

    // The episodes cache is stored oldest-first; display newest-first.
    let episodes_rev: Vec<&roko_learn::episode_logger::Episode> =
        tui_state.episodes_cache.iter().rev().collect();
    let total = episodes_rev.len();

    // `plan_scroll_offset` tracks the selected episode index (absolute, 0-based).
    let selected_idx = tui_state.plan_scroll_offset.min(total.saturating_sub(1));

    let header_height = 1usize;
    let visible_height = (list_inner.height as usize).saturating_sub(header_height);

    // Keep the selected row inside the visible window.
    let scroll = if selected_idx < visible_height {
        0
    } else {
        selected_idx.saturating_sub(visible_height.saturating_sub(1))
    };
    let scroll = scroll.min(total.saturating_sub(visible_height.max(1)));

    let rows: Vec<Row<'_>> = episodes_rev
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(abs_idx, ep)| {
            let outcome_style = if ep.success {
                theme.success()
            } else {
                theme.danger()
            };
            let outcome = if ep.success { "pass" } else { "fail" };
            let is_selected = abs_idx == selected_idx;
            let row_style = if is_selected {
                theme.selection()
            } else {
                Style::default()
            };
            let wall_time_ms = (ep.duration_secs.max(0.0) * 1000.0).round() as u64;
            Row::new(vec![
                Cell::from(ep.timestamp.format("%H:%M:%S").to_string()),
                Cell::from(truncate(&ep.agent_id, 14)),
                Cell::from(Span::styled(outcome.to_string(), outcome_style)),
                Cell::from(format!("{wall_time_ms}ms")),
                Cell::from(format_count(ep.usage.input_tokens + ep.usage.output_tokens)),
            ])
            .style(row_style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Min(10),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(7),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(["time", "agent", "result", "wall", "tokens"])
                .style(theme.accent().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1);
    frame.render_widget(table, list_inner);

    // ── Right panel: episode detail ───────────────────────────────────────────

    let selected_episode = episodes_rev.get(selected_idx).copied();
    match selected_episode {
        None => render_episode_detail_empty(frame, detail_area, detail_focused, theme),
        Some(ep) => render_episode_detail(
            frame,
            detail_area,
            ep,
            tui_state.inspect_detail_scroll,
            detail_focused,
            theme,
        ),
    }
}

/// Render the detail panel when no episode is selected.
fn render_episode_detail_empty(
    frame: &mut Frame<'_>,
    area: Rect,
    focused: bool,
    theme: &Theme,
) {
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::unfocused_border_style()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        Theme::unfocused_title_style()
    };
    let block = Block::bordered()
        .border_style(border_style)
        .title(Span::styled(" Episode Detail ", title_style));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let msg = Paragraph::new("Select an episode from the list")
        .style(theme.muted())
        .alignment(Alignment::Center);
    frame.render_widget(msg, inner);
}

/// Render the full detail panel for a single episode.
fn render_episode_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    ep: &roko_learn::episode_logger::Episode,
    scroll_offset: usize,
    focused: bool,
    theme: &Theme,
) {
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::unfocused_border_style()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        Theme::unfocused_title_style()
    };
    let hint = if focused {
        " [↓/↑] scroll  [←/h] list "
    } else {
        " [Tab] focus "
    };
    let block = Block::bordered()
        .border_style(border_style)
        .title(Span::styled(" Episode Detail ", title_style))
        .title_bottom(Span::styled(hint, theme.muted()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build lines for the detail view.
    let mut lines: Vec<Line<'_>> = Vec::new();

    // ── Identity ──────────────────────────────────────────────────────────────
    lines.push(Line::from(vec![
        Span::styled("  Metadata", theme.accent_bold()),
    ]));
    lines.push(Line::default());

    let label = theme.label();
    let text = theme.text();
    let muted = theme.muted();
    let success = theme.success();
    let danger = theme.danger();

    macro_rules! kv {
        ($k:expr, $v:expr) => {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:16}", $k), label),
                Span::styled($v.to_string(), text),
            ]));
        };
    }

    kv!("Agent:", truncate(&ep.agent_id, 40));
    kv!("Task:", truncate(&ep.task_id, 40));
    if !ep.agent_template.is_empty() {
        kv!("Role:", truncate(&ep.agent_template, 40));
    }
    if !ep.kind.is_empty() {
        kv!("Kind:", truncate(&ep.kind, 40));
    }
    if !ep.model.is_empty() {
        kv!("Model:", truncate(&ep.model, 40));
    }
    if !ep.backend.is_empty() {
        kv!("Backend:", truncate(&ep.backend, 40));
    }
    kv!("Started:", ep.started_at.format("%Y-%m-%d %H:%M:%S UTC"));
    kv!("Completed:", ep.completed_at.format("%Y-%m-%d %H:%M:%S UTC"));

    // ── Duration ──────────────────────────────────────────────────────────────
    let wall_ms = (ep.duration_secs.max(0.0) * 1000.0).round() as u64;
    let duration_str = if wall_ms >= 60_000 {
        format!("{:.1}m", wall_ms as f64 / 60_000.0)
    } else if wall_ms >= 1_000 {
        format!("{:.2}s", wall_ms as f64 / 1_000.0)
    } else {
        format!("{wall_ms}ms")
    };
    kv!("Duration:", duration_str);

    // ── Outcome ───────────────────────────────────────────────────────────────
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled("  Outcome", theme.accent_bold()),
    ]));
    lines.push(Line::default());

    let (outcome_label, outcome_style) = if ep.success {
        ("pass", success)
    } else {
        ("fail", danger)
    };
    lines.push(Line::from(vec![
        Span::styled(format!("  {:16}", "Result:"), label),
        Span::styled(outcome_label, outcome_style),
    ]));

    if let Some(ref reason) = ep.failure_reason {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:16}", "Failure:"), label),
            Span::styled(truncate(reason, 60), danger),
        ]));
    }
    if let Some(ref summary) = ep.reasoning_summary {
        if !summary.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:16}", "Summary:"), label),
                Span::styled(truncate(summary, 60), text),
            ]));
        }
    }
    if let Some(ref reflection) = ep.reflection {
        if !reflection.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:16}", "Reflection:"), label),
                Span::styled(truncate(reflection, 60), muted),
            ]));
        }
    }

    // ── Gate verdicts ─────────────────────────────────────────────────────────
    if !ep.gate_verdicts.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("  Gates", theme.accent_bold()),
        ]));
        lines.push(Line::default());
        for verdict in &ep.gate_verdicts {
            let (v_label, v_style) = if verdict.passed {
                ("pass", success)
            } else {
                ("fail", danger)
            };
            let sig_part = verdict
                .signature
                .as_deref()
                .map(|s| format!("  sig:{}", &s[..s.len().min(12)]))
                .unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled(format!("  {:16}", truncate(&verdict.gate, 14)), label),
                Span::styled(v_label, v_style),
                Span::styled(sig_part, muted),
            ]));
        }
    }

    // ── Token / cost accounting ───────────────────────────────────────────────
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled("  Tokens & Cost", theme.accent_bold()),
    ]));
    lines.push(Line::default());

    kv!(
        "Input tokens:",
        format_count(ep.usage.input_tokens)
    );
    kv!(
        "Output tokens:",
        format_count(ep.usage.output_tokens)
    );
    if ep.usage.cache_read_tokens > 0 {
        kv!(
            "Cache read:",
            format_count(ep.usage.cache_read_tokens)
        );
    }
    if ep.usage.cache_write_tokens > 0 {
        kv!(
            "Cache write:",
            format_count(ep.usage.cache_write_tokens)
        );
    }
    kv!(
        "Total tokens:",
        format_count(ep.usage.input_tokens + ep.usage.output_tokens)
    );
    if ep.usage.cost_usd > 0.0 {
        kv!("Cost:", format!("${:.6}", ep.usage.cost_usd));
    }
    if ep.usage.cost_usd_without_cache > 0.0 && ep.usage.cost_usd_without_cache != ep.usage.cost_usd {
        kv!(
            "Cost (no cache):",
            format!("${:.6}", ep.usage.cost_usd_without_cache)
        );
    }

    // ── HDC fingerprint ───────────────────────────────────────────────────────
    if let Some(ref fp) = ep.hdc_fingerprint {
        if !fp.is_empty() {
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("  HDC Fingerprint", theme.accent_bold()),
            ]));
            lines.push(Line::default());
            // Show the fingerprint in 32-char chunks for readability.
            let fp_display = truncate(fp, 64);
            lines.push(Line::from(vec![
                Span::styled("  ", label),
                Span::styled(fp_display, muted),
            ]));
        }
    }

    // ── Emotional tag ─────────────────────────────────────────────────────────
    if let Some(ref tag) = ep.emotional_tag {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("  Affect (PAD)", theme.accent_bold()),
        ]));
        lines.push(Line::default());
        kv!("Pleasure:", format!("{:.3}", tag.pad.pleasure));
        kv!("Arousal:", format!("{:.3}", tag.pad.arousal));
        kv!("Dominance:", format!("{:.3}", tag.pad.dominance));
        kv!("Intensity:", format!("{:.3}", tag.intensity));
        if !tag.trigger.is_empty() {
            kv!("Trigger:", truncate(&tag.trigger, 40));
        }
    }

    // ── Turns / extra ─────────────────────────────────────────────────────────
    if ep.turns > 0 {
        lines.push(Line::default());
        kv!("Turns:", ep.turns);
    }
    if !ep.trigger_kind.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:16}", "Trigger:"), label),
            Span::styled(truncate(&ep.trigger_kind, 40), text),
        ]));
    }

    // Render with scroll offset applied.
    let total_lines = lines.len();
    let visible = inner.height as usize;
    let max_scroll = total_lines.saturating_sub(visible);
    let clamped_scroll = scroll_offset.min(max_scroll);

    let visible_lines: Vec<Line<'_>> = lines
        .into_iter()
        .skip(clamped_scroll)
        .take(visible)
        .collect();

    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Sub-view: Knowledge Browse (sub_tab == 3)
// ---------------------------------------------------------------------------

fn render_knowledge_browse(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let block = Block::bordered().title(Span::styled(" Knowledge Browse ", theme.accent()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tui_state.knowledge_entries.is_empty() {
        let empty =
            Paragraph::new("No knowledge entries yet \u{2014} the neuro store fills across runs")
                .style(theme.muted())
                .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    let query = view_state.search_query.trim().to_ascii_lowercase();
    let filtered = tui_state
        .knowledge_entries
        .iter()
        .filter(|entry| {
            query.is_empty()
                || entry.kind.to_ascii_lowercase().contains(&query)
                || entry.content_preview.to_ascii_lowercase().contains(&query)
                || entry
                    .tags
                    .iter()
                    .any(|t| t.to_ascii_lowercase().contains(&query))
        })
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        let empty = Paragraph::new(format!(
            "no knowledge entries match '{}'",
            view_state.search_query.trim()
        ))
        .alignment(Alignment::Center)
        .style(theme.muted())
        .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    let visible_height = inner.height as usize;
    let scroll = bounded_scroll(view_state.scroll as usize, filtered.len(), visible_height);
    let selected = selected_in_window(view_state.selected, scroll, visible_height);

    let items: Vec<ListItem<'_>> = filtered
        .iter()
        .skip(scroll)
        .take(visible_height)
        .map(|entry| {
            ListItem::new(Line::from(vec![
                Span::styled(truncate(&entry.kind, 12), theme.accent()),
                Span::raw("  "),
                Span::styled(
                    confidence_bar(Some(entry.confidence), 5),
                    confidence_style(Some(entry.confidence), theme),
                ),
                Span::raw("  "),
                Span::styled(truncate(&entry.content_preview, 50), theme.text()),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(selected);
    frame.render_stateful_widget(
        List::new(items).highlight_symbol("> "),
        inner,
        &mut list_state,
    );
}

fn bounded_scroll(requested: usize, total: usize, visible_height: usize) -> usize {
    if visible_height == 0 || total <= visible_height {
        0
    } else {
        requested.min(total - visible_height)
    }
}

fn selected_in_window(selected: usize, scroll: usize, visible_height: usize) -> Option<usize> {
    if visible_height == 0 || selected < scroll {
        None
    } else {
        let relative = selected - scroll;
        (relative < visible_height).then_some(relative)
    }
}

fn signal_depth(signal: &crate::tui::dashboard::SignalSummary) -> usize {
    if signal.lineage.is_empty() {
        usize::from(signal.parent_hash.is_some())
    } else {
        signal.lineage.len().min(4)
    }
}

fn confidence_bar(confidence: Option<f64>, width: usize) -> String {
    let Some(confidence) = confidence else {
        return format!("[{}]", "\u{2500}".repeat(width));
    };
    let filled = (confidence.clamp(0.0, 1.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}]",
        "\u{2588}".repeat(filled.min(width)),
        "\u{2500}".repeat(empty)
    )
}

fn confidence_style(confidence: Option<f64>, theme: &Theme) -> Style {
    match confidence {
        Some(value) if value >= 0.75 => theme.success(),
        Some(value) if value >= 0.45 => theme.warning(),
        Some(_) => theme.danger(),
        None => theme.muted(),
    }
}

/// Build context data from available dashboard data.
fn build_context_data(tui_state: &TuiState) -> ContextViewData {
    // Build token burn sparklines from efficiency events
    let mut burn_map: HashMap<String, Vec<u64>> = HashMap::new();
    for event in &tui_state.efficiency_events {
        let id = event.agent_id.clone();
        let cumulative = burn_map.entry(id).or_default();
        let prev = cumulative.last().copied().unwrap_or(0);
        cumulative.push(prev + event.input_tokens + event.output_tokens);
    }

    let mut token_burns: Vec<TokenBurnData> = burn_map
        .into_iter()
        .map(|(agent_id, cumulative)| TokenBurnData {
            agent_id,
            cumulative,
        })
        .collect();
    token_burns.sort_by(|a, b| {
        let a_total = a.cumulative.last().copied().unwrap_or(0);
        let b_total = b.cumulative.last().copied().unwrap_or(0);
        b_total.cmp(&a_total)
    });

    ContextViewData { token_burns }
}

fn format_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

use crate::tui::display_utils::{display_model, truncate};

// ---------------------------------------------------------------------------
// Three-panel inspect view (item 127)
// ---------------------------------------------------------------------------

/// Render the three-panel MCP / Learning / Prompt Stats layout.
fn render_three_panel_inspect(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    theme: &Theme,
) {
    // Give MCP panel more width since it shows longer paths/commands.
    let columns = Layout::horizontal([
        Constraint::Percentage(38),
        Constraint::Percentage(32),
        Constraint::Percentage(30),
    ])
    .split(area);

    render_mcp_panel(frame, columns[0], tui_state, theme);
    render_learning_panel(frame, columns[1], tui_state, theme);
    render_prompt_stats_panel(frame, columns[2], tui_state, theme);
}

/// Column 1: MCP runtime status panel.
///
/// Uses the richer `McpConfigView` (from TuiState) for per-server commands
/// and error messages, falling back to `InspectData.mcp` for tool/index counts.
fn render_mcp_panel(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let block = Block::bordered()
        .title(Span::styled(" MCP / Providers ", theme.section_header()))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mcp_config = &tui_state.mcp_config_view;
    let mcp_stats = &tui_state.inspect_data.mcp;
    let max_label = inner.width.saturating_sub(12) as usize;

    let mut lines: Vec<Line<'_>> = Vec::new();

    // Config file status
    let config_label = mcp_config
        .resolved_path
        .as_ref()
        .map(|p| p.display().to_string())
        .or_else(|| {
            mcp_config
                .configured_path
                .as_ref()
                .map(|p| p.display().to_string())
        });
    lines.push(Line::from(vec![
        Span::styled("config: ", theme.label()),
        if let Some(ref path) = config_label {
            Span::styled(truncate(path, max_label), theme.success())
        } else if mcp_stats.config_exists {
            Span::styled("roko.toml", theme.success())
        } else {
            Span::styled("not configured", theme.metadata())
        },
    ]));

    // Error from MCP config loading
    if let Some(ref error) = mcp_config.error {
        lines.push(Line::from(vec![
            Span::styled("error:  ", theme.danger()),
            Span::styled(truncate(error, max_label), theme.danger()),
        ]));
    }

    // Server list with connection status indicators
    if let Some(ref config) = mcp_config.config {
        let server_count = config.servers.len();
        lines.push(Line::from(vec![
            Span::styled("servers:", theme.label()),
            Span::raw(" "),
            Span::styled(server_count.to_string(), theme.value()),
        ]));
        let max_servers = inner.height.saturating_sub(6) as usize;
        for server in config.servers.iter().take(max_servers) {
            let (icon, icon_style) = if !server.command.is_empty() {
                ("+", theme.success())
            } else {
                ("?", theme.warning())
            };
            let cmd = if !server.command.is_empty() {
                truncate(&server.command, max_label.saturating_sub(6))
            } else {
                "(no command)".to_string()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {icon} "), icon_style),
                Span::styled(format!("{}: ", truncate(&server.name, 10)), theme.accent()),
                Span::styled(cmd, theme.text()),
            ]));
        }
        if server_count > max_servers {
            lines.push(Line::from(Span::styled(
                format!("  (+{} more)", server_count - max_servers),
                theme.metadata(),
            )));
        }
    } else if !mcp_stats.servers.is_empty() {
        // Fall back to InspectData server names
        lines.push(Line::from(vec![
            Span::styled("servers:", theme.label()),
            Span::raw(" "),
            Span::styled(mcp_stats.servers.len().to_string(), theme.value()),
        ]));
        for name in mcp_stats.servers.iter().take(5) {
            lines.push(Line::from(vec![
                Span::styled("  ? ", theme.metadata()),
                Span::styled(truncate(name, max_label), theme.metadata()),
            ]));
        }
        if mcp_stats.servers.len() > 5 {
            lines.push(Line::from(Span::styled(
                format!("  (+{} more)", mcp_stats.servers.len() - 5),
                theme.metadata(),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No MCP servers configured",
            theme.metadata(),
        )));
    }

    // Tool and index stats
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("tools:  ", theme.label()),
        if mcp_stats.tool_count > 0 {
            Span::styled(mcp_stats.tool_count.to_string(), theme.value())
        } else {
            Span::styled("0", theme.metadata())
        },
        Span::styled("   index: ", theme.label()),
        if mcp_stats.index_file_count > 0 {
            Span::styled(
                format!(
                    "{} files / {} sym",
                    mcp_stats.index_file_count, mcp_stats.index_symbol_count
                ),
                theme.value(),
            )
        } else {
            Span::styled("not built", theme.metadata())
        },
    ]));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Column 2: Learning state metrics panel.
///
/// Shows episode stats, experiment winners (previously unused), playbook rules,
/// routing coverage, and gate thresholds with value-based coloring.
fn render_learning_panel(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let block = Block::bordered()
        .title(Span::styled(" Learning ", theme.section_header()))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let learn = &tui_state.inspect_data.learning;
    let has_any_data = learn.episode_count > 0
        || learn.playbook_rule_count > 0
        || !learn.gate_thresholds.is_empty()
        || !tui_state.experiment_winners.is_empty();

    if !has_any_data {
        let empty = Paragraph::new("Data available during active runs")
            .style(theme.muted())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    let ep_total = learn.episode_count;
    let ep_passed = learn.episodes_passed;
    let ep_failed = learn.episodes_failed;
    let accuracy = if ep_total > 0 {
        ep_passed as f64 / ep_total as f64 * 100.0
    } else {
        0.0
    };

    let accuracy_style = if ep_total == 0 {
        theme.muted()
    } else if accuracy >= 80.0 {
        theme.success()
    } else if accuracy >= 50.0 {
        theme.warning()
    } else {
        theme.danger()
    };

    let mut lines: Vec<Line<'_>> = vec![
        Line::from(vec![
            Span::styled("episodes: ", theme.label()),
            Span::styled(ep_total.to_string(), theme.value()),
            Span::styled(format!(" ({ep_passed}P/{ep_failed}F)"), theme.metadata()),
        ]),
        Line::from(vec![
            Span::styled("accuracy: ", theme.label()),
            if ep_total > 0 {
                Span::styled(format!("{accuracy:.1}%"), accuracy_style)
            } else {
                Span::styled("-", theme.metadata())
            },
        ]),
        Line::from(vec![
            Span::styled("playbook: ", theme.label()),
            Span::styled(
                format!("{} rules", learn.playbook_rule_count),
                theme.value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("routing:  ", theme.label()),
            Span::styled(
                format!(
                    "{:.0}% ({} models)",
                    learn.routing_coverage_pct,
                    tui_state.cascade_router.model_slugs.len()
                ),
                theme.value(),
            ),
        ]),
    ];

    // Knowledge tier distribution
    if !tui_state.knowledge_entries.is_empty() {
        let total_k = tui_state.knowledge_entries.len();
        let high_conf = tui_state
            .knowledge_entries
            .iter()
            .filter(|e| e.confidence >= 0.75)
            .count();
        let mid_conf = tui_state
            .knowledge_entries
            .iter()
            .filter(|e| e.confidence >= 0.45 && e.confidence < 0.75)
            .count();
        let low_conf = total_k - high_conf - mid_conf;
        lines.push(Line::from(vec![
            Span::styled("knowledge:", theme.label()),
            Span::styled(format!(" {total_k}"), theme.value()),
            Span::styled(" (", theme.metadata()),
            Span::styled(format!("{high_conf}"), theme.success()),
            Span::styled("/", theme.metadata()),
            Span::styled(format!("{mid_conf}"), theme.warning()),
            Span::styled("/", theme.metadata()),
            Span::styled(format!("{low_conf}"), theme.danger()),
            Span::styled(")", theme.metadata()),
        ]));
    }

    // Experiments: active and winners
    let has_experiments =
        !tui_state.experiment_winners.is_empty() || !tui_state.experiments.is_empty();
    if has_experiments {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Experiments:",
            theme.section_header(),
        )));
        let max_experiments = (inner.height as usize).saturating_sub(lines.len() + 2);
        let mut shown = 0usize;
        // Active experiments first
        for exp in &tui_state.experiments {
            if shown >= max_experiments {
                break;
            }
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", truncate(&exp.experiment_id, 8)),
                    theme.label(),
                ),
                Span::styled(
                    format!("{} ({}t)", exp.status, exp.total_trials),
                    theme.info(),
                ),
            ]));
            shown += 1;
        }
        // Then concluded winners
        for w in &tui_state.experiment_winners {
            if shown >= max_experiments {
                break;
            }
            let label = if w.winner.is_empty() {
                truncate(&w.winner_variant_id, 10)
            } else {
                truncate(&w.winner, 10)
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", truncate(&w.experiment_id, 8)),
                    theme.label(),
                ),
                Span::styled(format!("+ {label}"), theme.success()),
                if w.sample_size > 0 {
                    Span::styled(format!(" n={}", w.sample_size), theme.metadata())
                } else {
                    Span::raw("")
                },
            ]));
            shown += 1;
        }
    }

    // Gate thresholds with value coloring
    let remaining_height = (inner.height as usize).saturating_sub(lines.len() + 1);
    if !learn.gate_thresholds.is_empty() && remaining_height > 2 {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Gate Thresholds:",
            theme.section_header(),
        )));
        for (rung, t) in learn
            .gate_thresholds
            .iter()
            .take(remaining_height.saturating_sub(1))
        {
            let threshold_style = if *t >= 0.8 {
                theme.success()
            } else if *t >= 0.5 {
                theme.warning()
            } else {
                theme.danger()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", truncate(rung, 10)), theme.label()),
                Span::styled(format!("{t:.3}"), threshold_style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Column 3: Prompt / token statistics panel.
///
/// Shows per-role token breakdown with input/output split and cache hit rate,
/// plus overall context utilization summary.
fn render_prompt_stats_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(" Token / Cost ", theme.section_header()))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tui_state.efficiency_events.is_empty() {
        let empty = Paragraph::new("Data available during active runs")
            .style(theme.muted())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    // Compute per-role aggregates with input/output/cache breakdown
    let mut role_data: BTreeMap<String, (u64, u64, u64, u64, f64)> = BTreeMap::new(); // (in, out, cache_read, turns, cost)
    for ev in &tui_state.efficiency_events {
        let role = if ev.role.is_empty() {
            "unknown"
        } else {
            ev.role.as_str()
        };
        let entry = role_data.entry(role.to_string()).or_default();
        entry.0 += ev.input_tokens;
        entry.1 += ev.output_tokens;
        entry.2 += ev.cache_read_tokens;
        entry.3 += 1;
        entry.4 += ev.cost_usd;
    }

    let mut lines: Vec<Line<'_>> = vec![Line::from(Span::styled(
        "Per-Role Breakdown:",
        theme.section_header(),
    ))];

    let remaining = (inner.height as usize).saturating_sub(6); // reserve for summary
    for (role, (inp, out, cache, turns, cost)) in role_data.iter().take(remaining) {
        let cache_pct = if *inp > 0 {
            format!("{:.0}%c", *cache as f64 / *inp as f64 * 100.0)
        } else {
            "-".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {}: ", truncate(role, 8)), theme.label()),
            Span::styled(format_count(*inp), theme.value()),
            Span::styled("/", theme.metadata()),
            Span::styled(format_count(*out), theme.value()),
            Span::styled(format!(" {cache_pct}"), theme.metadata()),
        ]));
        let _ = (turns, cost); // used below in totals
    }

    // Overall totals
    let total_in: u64 = role_data.values().map(|r| r.0).sum();
    let total_out: u64 = role_data.values().map(|r| r.1).sum();
    let total_cache: u64 = role_data.values().map(|r| r.2).sum();
    let total_cost: f64 = role_data.values().map(|r| r.4).sum();
    let cache_ratio = if total_in > 0 {
        total_cache as f64 / total_in as f64 * 100.0
    } else {
        0.0
    };

    // Compute avg input tokens per turn for context utilization
    let total_turns: u64 = role_data.values().map(|r| r.3).sum();
    let avg_input_per_turn = if total_turns > 0 {
        total_in / total_turns
    } else {
        0
    };

    // Context window headroom (use configured or default model limit)
    let ctx_limit = crate::tui::state::model_context_limit(
        &tui_state
            .efficiency_events
            .last()
            .map_or(String::new(), |e| e.model.clone()),
    );
    let utilization_pct = if ctx_limit > 0 {
        avg_input_per_turn as f64 / ctx_limit as f64 * 100.0
    } else {
        0.0
    };
    let util_style = if utilization_pct > 80.0 {
        theme.danger()
    } else if utilization_pct > 50.0 {
        theme.warning()
    } else {
        theme.success()
    };

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("Summary:", theme.section_header())));
    lines.push(Line::from(vec![
        Span::styled(" in/out: ", theme.label()),
        Span::styled(format_count(total_in), theme.value()),
        Span::styled(" / ", theme.metadata()),
        Span::styled(format_count(total_out), theme.value()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" cache:  ", theme.label()),
        Span::styled(
            format!("{cache_ratio:.1}%"),
            if cache_ratio > 50.0 {
                theme.success()
            } else {
                theme.info()
            },
        ),
        Span::styled("   cost: ", theme.label()),
        Span::styled(format!("${total_cost:.4}"), theme.warning()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" ctx:    ", theme.label()),
        Span::styled(format!("{utilization_pct:.0}%"), util_style),
        Span::styled(
            format!(
                " ({}/{})",
                format_count(avg_input_per_turn),
                format_count(ctx_limit)
            ),
            theme.metadata(),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ---------------------------------------------------------------------------
// C-Factor detail view (sub-tab 6, W3 visualization)
// ---------------------------------------------------------------------------

/// Render a full-page C-Factor detail panel with overall gauge, component
/// breakdown bars, trend direction, and agent contribution table.
fn render_cfactor_detail(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let block = Block::bordered()
        .title(Span::styled(" C-Factor Detail ", theme.section_header()))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cf = match tui_state.cfactor.as_ref() {
        Some(cf) => cf,
        None => {
            let empty =
                Paragraph::new("C-Factor not yet computed. Data appears during active plan runs.")
                    .style(theme.muted())
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: false });
            frame.render_widget(empty, inner);
            return;
        }
    };

    // Layout: top gauge row, middle component bars, bottom agent contributions.
    let sections = Layout::vertical([
        Constraint::Length(5), // Overall score gauge + trend
        Constraint::Min(12),   // Component breakdown bars
        Constraint::Length(8), // Agent contributions table
    ])
    .split(inner);

    // ── Section 1: Overall score gauge + trend ─────────────────────────
    render_cfactor_gauge(frame, sections[0], cf, tui_state, theme);

    // ── Section 2: Component breakdown bars ────────────────────────────
    render_cfactor_components(frame, sections[1], cf, theme);

    // ── Section 3: Agent contributions ─────────────────────────────────
    render_cfactor_agents(frame, sections[2], cf, theme);
}

/// Render the overall C-Factor score as a colored bar gauge with trend indicator.
fn render_cfactor_gauge(
    frame: &mut Frame<'_>,
    area: Rect,
    cf: &roko_learn::cfactor::CFactor,
    tui_state: &TuiState,
    theme: &Theme,
) {
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Score value with tier coloring
    let score_style = cfactor_tier_style(cf.overall, theme);
    let pct = (cf.overall * 100.0).round() as u16;

    // Compute trend from buckets
    let trend = compute_cfactor_trend(&tui_state.cfactor_trend_buckets);
    let trend_indicator = match trend {
        Trend::Improving => Span::styled(" ^ improving", theme.success()),
        Trend::Declining => Span::styled(" v declining", theme.danger()),
        Trend::Stable => Span::styled(" ~ stable", theme.metadata()),
        Trend::Unknown => Span::styled(" ? insufficient data", theme.metadata()),
    };

    lines.push(Line::from(vec![
        Span::styled("Overall: ", theme.label()),
        Span::styled(format!("{:.3}", cf.overall), score_style),
        Span::styled(format!(" ({pct}%)"), theme.metadata()),
        Span::raw("  "),
        trend_indicator,
    ]));

    // ASCII gauge bar
    let bar_width = (area.width as usize).saturating_sub(4).min(60);
    let filled = ((cf.overall * bar_width as f64).round() as usize).min(bar_width);
    let empty_part = bar_width.saturating_sub(filled);
    let bar_str = format!(
        "[{}{}]",
        "\u{2588}".repeat(filled),
        "\u{2500}".repeat(empty_part),
    );
    lines.push(Line::from(Span::styled(bar_str, score_style)));

    // Episode count and timestamp
    lines.push(Line::from(vec![
        Span::styled("Episodes: ", theme.label()),
        Span::styled(cf.episode_count.to_string(), theme.value()),
        Span::styled(
            format!("   computed: {}", cf.computed_at.format("%H:%M:%S")),
            theme.metadata(),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

/// Render individual C-Factor component scores as labeled horizontal bars.
fn render_cfactor_components(
    frame: &mut Frame<'_>,
    area: Rect,
    cf: &roko_learn::cfactor::CFactor,
    theme: &Theme,
) {
    let c = &cf.components;

    // Component list: (label, value, short_name)
    let components: Vec<(&str, f64)> = vec![
        ("Gate pass rate     ", c.gate_pass_rate),
        ("First-try rate     ", c.first_try_rate),
        ("Cost efficiency    ", c.cost_efficiency),
        ("Speed              ", c.speed),
        ("Knowledge growth   ", c.knowledge_growth),
        ("Turn-taking equal. ", c.turn_taking_equality),
        ("HDC diversity      ", c.hdc_diversity),
        ("Convergence vel.   ", c.convergence_velocity),
        ("Info flow rate     ", c.information_flow_rate),
        ("Knowledge integr.  ", c.knowledge_integration_rate),
        ("Social perceptive. ", c.social_perceptiveness),
    ];

    let bar_budget = (area.width as usize).saturating_sub(30).min(40);
    let visible = (area.height as usize).min(components.len());
    let mut lines: Vec<Line<'_>> = vec![Line::from(Span::styled(
        "Component Breakdown:",
        theme.section_header(),
    ))];

    for (label, value) in components.iter().take(visible) {
        let v = value.clamp(0.0, 1.0);
        let filled = (v * bar_budget as f64).round() as usize;
        let empty_part = bar_budget.saturating_sub(filled);
        let bar = format!(
            "{}{}",
            "\u{2588}".repeat(filled),
            "\u{2500}".repeat(empty_part),
        );
        let val_style = cfactor_tier_style(*value, theme);
        lines.push(Line::from(vec![
            Span::styled(*label, theme.label()),
            Span::styled(format!("{:.2} ", value), val_style),
            Span::styled(bar, val_style),
        ]));
    }

    // Pathology alerts
    if !cf.pathologies.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Pathologies:",
            Style::default()
                .fg(Theme::EMBER)
                .add_modifier(Modifier::BOLD),
        )));
        for p in cf.pathologies.iter().take(3) {
            let desc = format!("  ! {}", cfactor_pathology_label(p));
            lines.push(Line::from(Span::styled(desc, theme.danger())));
        }
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

/// Render the per-agent contribution table.
fn render_cfactor_agents(
    frame: &mut Frame<'_>,
    area: Rect,
    cf: &roko_learn::cfactor::CFactor,
    theme: &Theme,
) {
    if cf.agent_contributions.is_empty() {
        let empty = Paragraph::new("No per-agent contributions available")
            .style(theme.muted())
            .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let header = Row::new(["Agent", "Episodes", "Contribution", "Impact"])
        .style(theme.section_header())
        .bottom_margin(0);

    let visible = (area.height as usize).saturating_sub(2);
    let rows: Vec<Row<'_>> = cf
        .agent_contributions
        .iter()
        .take(visible)
        .map(|ac| {
            let impact_style = if ac.contribution_score > 0.01 {
                theme.success()
            } else if ac.contribution_score < -0.01 {
                theme.danger()
            } else {
                theme.metadata()
            };
            let impact_label = if ac.contribution_score > 0.01 {
                "positive"
            } else if ac.contribution_score < -0.01 {
                "negative"
            } else {
                "neutral"
            };
            Row::new([
                Cell::from(Span::styled(truncate(&ac.agent_id, 16), theme.value())),
                Cell::from(Span::styled(ac.episode_count.to_string(), theme.text())),
                Cell::from(Span::styled(
                    format!("{:+.4}", ac.contribution_score),
                    impact_style,
                )),
                Cell::from(Span::styled(impact_label, impact_style)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(
        Block::bordered()
            .title(Span::styled(
                " Agent Contributions ",
                theme.section_header(),
            ))
            .border_style(theme.accent()),
    );

    frame.render_widget(table, area);
}

/// Trend direction derived from C-Factor buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trend {
    Improving,
    Declining,
    Stable,
    Unknown,
}

/// Compute trend from recent C-Factor trend buckets.
///
/// Compares the average of the last two buckets against the two before them.
/// Requires at least 3 buckets for a directional signal.
fn compute_cfactor_trend(buckets: &[roko_learn::aggregate::CFactorBucket]) -> Trend {
    if buckets.len() < 3 {
        return Trend::Unknown;
    }
    let n = buckets.len();
    // Recent half vs older half
    let mid = n / 2;
    let older_avg: f64 = buckets[..mid].iter().map(|b| b.avg).sum::<f64>() / mid as f64;
    let recent_avg: f64 = buckets[mid..].iter().map(|b| b.avg).sum::<f64>() / (n - mid) as f64;
    let delta = recent_avg - older_avg;

    if delta > 0.02 {
        Trend::Improving
    } else if delta < -0.02 {
        Trend::Declining
    } else {
        Trend::Stable
    }
}

/// Map a C-Factor score (0..1) to a tier style.
fn cfactor_tier_style(score: f64, theme: &Theme) -> Style {
    if score >= 0.7 {
        theme.success()
    } else if score >= 0.4 {
        theme.warning()
    } else {
        theme.danger()
    }
}

/// Return a short human label for a collective pathology variant.
fn cfactor_pathology_label(p: &roko_learn::cfactor::CollectivePathology) -> &'static str {
    match p {
        roko_learn::cfactor::CollectivePathology::Cascade { .. } => {
            "Cascade: failure triggering downstream failures"
        }
        roko_learn::cfactor::CollectivePathology::Groupthink { .. } => {
            "Groupthink: fleet converged on narrow model set"
        }
        roko_learn::cfactor::CollectivePathology::EchoChamber { .. } => {
            "EchoChamber: repeated knowledge across agents"
        }
        roko_learn::cfactor::CollectivePathology::Deadlock { .. } => {
            "Deadlock: agents blocked on the same task"
        }
        roko_learn::cfactor::CollectivePathology::Hallucination { .. } => {
            "Hallucination: ungrounded claims without gate support"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::dashboard::SignalSummary;

    fn signal(parent_hash: Option<&str>, lineage: &[&str]) -> SignalSummary {
        SignalSummary {
            id: "abcdef0123456789".into(),
            kind: "gate:compile".into(),
            created_at_ms: 0,
            confidence: Some(0.5),
            plan_id: None,
            task_id: None,
            parent_hash: parent_hash.map(str::to_string),
            lineage: lineage.iter().map(|value| (*value).to_string()).collect(),
            payload_preview: String::new(),
        }
    }

    #[test]
    fn confidence_bar_renders_known_and_unknown_values() {
        assert_eq!(confidence_bar(Some(0.5), 6), "[███───]");
        assert_eq!(confidence_bar(Some(1.2), 4), "[████]");
        assert_eq!(confidence_bar(None, 4), "[────]");
    }

    #[test]
    fn bounded_scroll_clamps_to_last_full_window() {
        assert_eq!(bounded_scroll(10, 5, 3), 2);
        assert_eq!(bounded_scroll(10, 3, 5), 0);
        assert_eq!(bounded_scroll(10, 5, 0), 0);
    }

    #[test]
    fn selected_in_window_is_relative_when_visible() {
        assert_eq!(selected_in_window(5, 3, 4), Some(2));
        assert_eq!(selected_in_window(2, 3, 4), None);
        assert_eq!(selected_in_window(8, 3, 4), None);
    }

    #[test]
    fn signal_depth_prefers_lineage_then_parent() {
        assert_eq!(signal_depth(&signal(None, &[])), 0);
        assert_eq!(signal_depth(&signal(Some("parent"), &[])), 1);
        assert_eq!(signal_depth(&signal(Some("parent"), &["a", "b", "c"])), 3);
        assert_eq!(
            signal_depth(&signal(Some("parent"), &["a", "b", "c", "d", "e"])),
            4
        );
    }

    /// Build a [`SignalSummary`] with explicit id, parent_hash, kind, and timestamp.
    fn make_signal(id: &str, parent: Option<&str>, kind: &str, ts: i64) -> SignalSummary {
        SignalSummary {
            id: id.to_string(),
            kind: kind.to_string(),
            created_at_ms: ts,
            confidence: None,
            plan_id: None,
            task_id: None,
            parent_hash: parent.map(str::to_string),
            lineage: Vec::new(),
            payload_preview: String::new(),
        }
    }

    #[test]
    fn build_dag_rows_flat_list_all_roots() {
        // Three unrelated signals — all should appear as roots in timestamp order.
        let signals = vec![
            make_signal("aaa", None, "gate:compile", 300),
            make_signal("bbb", None, "agent:run", 100),
            make_signal("ccc", None, "gate:test", 200),
        ];
        let theme = crate::tui::theme::Theme::no_color();
        let rows = build_dag_rows(&signals, theme);
        assert_eq!(rows.len(), 3);
        // Chronological order: bbb(100), ccc(200), aaa(300).
        assert_eq!(rows[0].signal_index, 1); // bbb
        assert_eq!(rows[1].signal_index, 2); // ccc
        assert_eq!(rows[2].signal_index, 0); // aaa
    }

    #[test]
    fn build_dag_rows_parent_child_ordering() {
        // root → child1 → grandchild
        //      → child2
        let signals = vec![
            make_signal("root", None, "agent:dispatch", 0),
            make_signal("child1", Some("root"), "gate:compile", 10),
            make_signal("grandchild", Some("child1"), "gate:test", 20),
            make_signal("child2", Some("root"), "gate:clippy", 15),
        ];
        let theme = crate::tui::theme::Theme::no_color();
        let rows = build_dag_rows(&signals, theme);
        // DFS order: root, child1, grandchild, child2.
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].signal_index, 0); // root
        assert_eq!(rows[1].signal_index, 1); // child1
        assert_eq!(rows[2].signal_index, 2); // grandchild
        assert_eq!(rows[3].signal_index, 3); // child2
    }

    #[test]
    fn build_dag_rows_dangling_parent_becomes_root() {
        // Signal references a parent that isn't in the slice — treated as root.
        let signals = vec![
            make_signal("orphan", Some("missing-parent"), "gate:compile", 5),
        ];
        let theme = crate::tui::theme::Theme::no_color();
        let rows = build_dag_rows(&signals, theme);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].signal_index, 0);
    }

    #[test]
    fn format_dag_timestamp_formats_correctly() {
        // 3661 seconds = 01:01:01
        assert_eq!(format_dag_timestamp(3_661_000), "01:01:01");
        // Zero / negative → placeholder.
        assert_eq!(format_dag_timestamp(0), "--:--:--");
        assert_eq!(format_dag_timestamp(-1), "--:--:--");
    }
}
