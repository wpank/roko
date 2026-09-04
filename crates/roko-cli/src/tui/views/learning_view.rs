//! F10 Learning view -- cascade router & model routing insights.
//!
//! Layout:
//!   Sub-view 1 (Route): cascade stage + per-model stats table
//!   Sub-view 2 (History): stage transition timeline
//!   Sub-view 3 (Efficiency): per-model cost/pass sparklines
//!
//! Data sources:
//!   - `TuiState.cascade_router` (CascadeRouterState from cascade-router.json)
//!   - `TuiState.efficiency_events` (AgentEfficiencyEvent from efficiency.jsonl)

use std::collections::HashMap;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Cell, Paragraph, Row, Table, Wrap};

use super::{SubView, ViewState};
use crate::tui::dashboard::Theme;
use crate::tui::state::TuiState;
use crate::tui::tabs::Tab;

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

/// Render the full learning view.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &crate::tui::dashboard::DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
    render_sub_tab_bar(frame, rows[0], view_state, theme);

    match view_state.active_sub_view(Tab::Learning) {
        SubView::LearningRouter => render_router(frame, rows[1], tui_state, theme),
        SubView::LearningHistory => render_history(frame, rows[1], tui_state, theme),
        SubView::LearningEfficiency => render_efficiency(frame, rows[1], tui_state, theme),
        _ => render_router(frame, rows[1], tui_state, theme),
    }
}

fn render_sub_tab_bar(frame: &mut Frame<'_>, area: Rect, view_state: &ViewState, theme: &Theme) {
    let label = SubView::bar_label(Tab::Learning, view_state.sub_tab);
    let bar = Paragraph::new(Line::from(Span::styled(label, theme.muted())))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Theme::BG_RAISED));
    frame.render_widget(bar, area);
}

// ---------------------------------------------------------------------------
// Sub-view 1: Route overview
// ---------------------------------------------------------------------------

fn render_router(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let router = &tui_state.cascade_router;

    if router.model_slugs.is_empty() {
        let block = Block::bordered()
            .title(Span::styled(" Cascade Route ", theme.section_header()))
            .border_style(theme.muted());
        let inner = block.inner(area);
        frame.render_widget(block, area);
        crate::tui::empty_state::render_empty_state(
            frame,
            inner,
            crate::tui::tabs::Tab::Learning,
            &tui_state.atmosphere,
        );
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(5), // stage indicator
        Constraint::Length(1), // separator
        Constraint::Min(6),    // model stats table
        Constraint::Length(1), // separator
        Constraint::Length(6), // bar chart
    ])
    .split(area);

    render_stage_indicator(frame, chunks[0], tui_state, theme);
    render_separator(frame, chunks[1], theme);
    render_model_table(frame, chunks[2], tui_state, theme);
    render_separator(frame, chunks[3], theme);
    render_selection_bars(frame, chunks[4], tui_state, theme);
}

fn render_separator(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let width = area.width as usize;
    let line = "\u{2500}".repeat(width);
    let p = Paragraph::new(Line::from(Span::styled(
        line,
        Style::default().fg(Theme::SEPARATOR),
    )));
    frame.render_widget(p, area);
    let _ = theme;
}

fn render_stage_indicator(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let router = &tui_state.cascade_router;
    let total_trials: u64 = router.confidence_stats.values().map(|s| s.trials).sum();

    let (stage_label, stage_color, next_threshold) = if total_trials < 10 {
        ("Static", Theme::STAGE_STATIC, 10u64)
    } else if total_trials < 30 {
        ("Confidence", Theme::STAGE_CONFIDENCE, 30)
    } else {
        ("UCB (LinUCB)", Theme::STAGE_UCB, u64::MAX)
    };

    let block = Block::bordered()
        .title(Span::styled(" Cascade Stage ", theme.section_header()))
        .border_style(theme.muted());

    let text = vec![
        Line::from(vec![
            Span::styled("  Stage: ", theme.label()),
            Span::styled(
                stage_label,
                Style::default()
                    .fg(stage_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Observations: ", theme.label()),
            Span::styled(
                if next_threshold == u64::MAX {
                    format!("{total_trials} (fully adaptive)")
                } else {
                    format!("{total_trials} / {next_threshold} (next stage at {next_threshold})")
                },
                theme.value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Models: ", theme.label()),
            Span::styled(router.model_slugs.len().to_string(), theme.value()),
        ]),
    ];

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

fn render_model_table(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let router = &tui_state.cascade_router;

    let header = Row::new(vec![
        Cell::from(Span::styled("Model", theme.label())),
        Cell::from(Line::from(Span::styled("Trials", theme.label())).alignment(Alignment::Right)),
        Cell::from(
            Line::from(Span::styled("Successes", theme.label())).alignment(Alignment::Right),
        ),
        Cell::from(
            Line::from(Span::styled("Pass Rate", theme.label())).alignment(Alignment::Right),
        ),
        Cell::from(Span::styled("Sparkline", theme.label())),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let mut rows = Vec::new();
    for slug in &router.model_slugs {
        let stats = router.confidence_stats.get(slug);
        let (trials, successes) = stats.map(|s| (s.trials, s.successes)).unwrap_or((0, 0));
        let pass_rate = if trials > 0 {
            format!("{:.1}%", successes as f64 / trials as f64 * 100.0)
        } else {
            "\u{2014}".to_string()
        };

        let spark = model_sparkline(slug, &tui_state.efficiency_events);

        let rate_color = if trials == 0 {
            theme.muted
        } else if successes * 100 >= trials * 80 {
            Theme::RATE_GOOD
        } else if successes * 100 >= trials * 50 {
            Theme::RATE_MID
        } else {
            Theme::RATE_BAD
        };

        rows.push(Row::new(vec![
            Cell::from(Span::styled(slug.as_str(), theme.value())),
            Cell::from(
                Line::from(Span::styled(trials.to_string(), theme.value()))
                    .alignment(Alignment::Right),
            ),
            Cell::from(
                Line::from(Span::styled(successes.to_string(), theme.value()))
                    .alignment(Alignment::Right),
            ),
            Cell::from(
                Line::from(Span::styled(pass_rate, Style::default().fg(rate_color)))
                    .alignment(Alignment::Right),
            ),
            Cell::from(spark),
        ]));
    }

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(12),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(
        Block::bordered()
            .title(Span::styled(" Per-Model Stats ", theme.section_header()))
            .border_style(theme.muted()),
    );

    frame.render_widget(table, area);
}

/// Build a mini sparkline string from efficiency events for a given model.
fn model_sparkline(
    model_slug: &str,
    events: &[roko_learn::efficiency::AgentEfficiencyEvent],
) -> String {
    let blocks = [
        '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
        '\u{2588}',
    ];
    let window: usize = 5;

    let model_events: Vec<bool> = events
        .iter()
        .filter(|e| event_model_slug(e) == model_slug)
        .map(|e| e.gate_passed.unwrap_or(false))
        .collect();

    if model_events.is_empty() {
        return "\u{2014}".to_string();
    }

    let mut pass_windows: Vec<f64> = Vec::with_capacity(model_events.len());
    for i in 0..model_events.len() {
        let start = i.saturating_sub(window.saturating_sub(1));
        let slice = &model_events[start..=i];
        let rate = slice.iter().filter(|&&p| p).count() as f64 / slice.len() as f64;
        pass_windows.push(rate);
    }

    // Take last 20 data points
    let tail: Vec<f64> = pass_windows
        .into_iter()
        .rev()
        .take(20)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    tail.iter()
        .map(|&v| {
            let idx = (v * 7.0).round() as usize;
            blocks[idx.min(7)]
        })
        .collect()
}

use crate::tui::display_utils::{display_model, event_model_slug};

fn render_selection_bars(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let router = &tui_state.cascade_router;
    let colors = Theme::SERIES_COLORS;

    let bars: Vec<Bar> = router
        .model_slugs
        .iter()
        .enumerate()
        .map(|(i, slug)| {
            let trials = router
                .confidence_stats
                .get(slug)
                .map(|s| s.trials)
                .unwrap_or(0);
            let label = display_model(Some(slug.as_str()));
            let label = if label.len() > 12 {
                label[..12].to_string()
            } else {
                label
            };
            Bar::default()
                .value(trials)
                .label(Line::from(label))
                .style(Style::default().fg(colors[i % colors.len()]))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::bordered()
                .title(Span::styled(
                    " Selection Frequency ",
                    theme.section_header(),
                ))
                .border_style(theme.muted()),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(
            area.width
                .saturating_sub(4)
                .checked_div(bars.len().max(1) as u16)
                .unwrap_or(5)
                .min(12)
                .max(3),
        )
        .bar_gap(1);

    frame.render_widget(bar_chart, area);
}

// ---------------------------------------------------------------------------
// Sub-view 2: Stage transition history
// ---------------------------------------------------------------------------

fn render_history(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let router = &tui_state.cascade_router;
    let total_trials: u64 = router.confidence_stats.values().map(|s| s.trials).sum();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    if total_trials == 0 {
        lines.push(Line::from(Span::styled(
            "  No observations yet.",
            theme.muted(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  The router progresses through three stages as it gathers data:",
            theme.muted(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("    1. ", theme.text()),
            Span::styled(
                "Static",
                Style::default()
                    .fg(Theme::STAGE_STATIC)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("       (0-9 obs)   Fixed model order", theme.muted()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    2. ", theme.text()),
            Span::styled(
                "Confidence",
                Style::default()
                    .fg(Theme::STAGE_CONFIDENCE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   (10-29 obs)  Weighted by pass rate", theme.muted()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    3. ", theme.text()),
            Span::styled(
                "UCB",
                Style::default()
                    .fg(Theme::STAGE_UCB)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "          (30+ obs)   Fully adaptive routing",
                theme.muted(),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Run tasks to start learning: roko plan run plans/ --engine runner-v2",
            theme.muted(),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  Current observations: ", theme.label()),
            Span::styled(total_trials.to_string(), theme.value()),
        ]));
        lines.push(Line::from(""));

        let stages = [
            ("Static", 0u64, 10u64, Theme::STAGE_STATIC),
            ("Confidence", 10, 30, Theme::STAGE_CONFIDENCE),
            ("UCB (LinUCB)", 30, u64::MAX, Theme::STAGE_UCB),
        ];

        for (label, from, to, color) in &stages {
            let active = if *to == u64::MAX {
                total_trials >= *from
            } else {
                total_trials >= *from && total_trials < *to
            };
            let marker = if active { "\u{25b6} " } else { "  " };
            let range_str = if *to == u64::MAX {
                format!("{from}+")
            } else {
                format!("{from}-{}", to - 1)
            };

            let style = if active {
                Style::default().fg(*color).add_modifier(Modifier::BOLD)
            } else if total_trials >= *from {
                Style::default().fg(*color)
            } else {
                theme.muted()
            };

            lines.push(Line::from(vec![
                Span::raw(format!("  {marker}")),
                Span::styled(format!("{label:<16}"), style),
                Span::styled(format!("  ({range_str} obs)"), theme.muted()),
            ]));
        }

        let sep_width = area.width.saturating_sub(6) as usize;
        lines.push(Line::from(Span::styled(
            format!("  {}", "\u{2500}".repeat(sep_width)),
            Style::default().fg(Theme::SEPARATOR),
        )));
        lines.push(Line::from(Span::styled(
            "  Stage Progression:",
            theme.section_header(),
        )));
        lines.push(Line::from(""));

        let bar_width = area.width.saturating_sub(8) as usize;
        let scale = if total_trials > 0 {
            bar_width as f64 / total_trials.max(30) as f64
        } else {
            1.0
        };

        let static_w = ((10.min(total_trials) as f64) * scale).round() as usize;
        let confidence_w = if total_trials > 10 {
            ((total_trials.min(30) - 10) as f64 * scale).round() as usize
        } else {
            0
        };
        let ucb_w = if total_trials > 30 {
            ((total_trials - 30) as f64 * scale).round() as usize
        } else {
            0
        };

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "\u{2588}".repeat(static_w),
                Style::default().fg(Theme::STAGE_STATIC),
            ),
            Span::styled(
                "\u{2588}".repeat(confidence_w),
                Style::default().fg(Theme::STAGE_CONFIDENCE),
            ),
            Span::styled(
                "\u{2588}".repeat(ucb_w),
                Style::default().fg(Theme::STAGE_UCB),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("\u{25a0}", Style::default().fg(Theme::STAGE_STATIC)),
            Span::raw(" Static  "),
            Span::styled("\u{25a0}", Style::default().fg(Theme::STAGE_CONFIDENCE)),
            Span::raw(" Confidence  "),
            Span::styled("\u{25a0}", Style::default().fg(Theme::STAGE_UCB)),
            Span::raw(" UCB"),
        ]));
    }

    let block = Block::bordered()
        .title(Span::styled(
            " Stage Transition History ",
            theme.section_header(),
        ))
        .border_style(theme.muted());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Sub-view 3: Efficiency by model
// ---------------------------------------------------------------------------

fn render_efficiency(frame: &mut Frame<'_>, area: Rect, tui_state: &TuiState, theme: &Theme) {
    let events = &tui_state.efficiency_events;

    if events.is_empty() {
        let block = Block::bordered()
            .title(Span::styled(" Model Efficiency ", theme.section_header()))
            .border_style(theme.muted());
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No efficiency events recorded yet.",
                theme.muted(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Efficiency events track per-turn cost, latency, and gate pass rate.",
                theme.muted(),
            )),
            Line::from(Span::styled(
                "Data appears after agent task turns complete.",
                theme.muted(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Source: .roko/learn/efficiency.jsonl",
                theme.muted(),
            )),
        ];
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let mut model_stats: HashMap<String, ModelEffStats> = HashMap::new();
    for event in events {
        let model = event_model_slug(event).to_string();
        let entry = model_stats.entry(model).or_default();
        entry.count += 1;
        if event.gate_passed == Some(true) {
            entry.passed += 1;
        }
        entry.total_cost += event.cost_usd;
        entry.total_latency_ms += event.wall_time_ms;
    }

    let chunks = Layout::vertical([
        Constraint::Min(6),    // stats table
        Constraint::Length(1), // separator
        Constraint::Length(8), // cost bar chart
    ])
    .split(area);

    // -- Stats table --
    let header = Row::new(vec![
        Cell::from(Span::styled("Model", theme.label())),
        Cell::from(Line::from(Span::styled("Events", theme.label())).alignment(Alignment::Right)),
        Cell::from(Line::from(Span::styled("Passed", theme.label())).alignment(Alignment::Right)),
        Cell::from(Line::from(Span::styled("Pass %", theme.label())).alignment(Alignment::Right)),
        Cell::from(Line::from(Span::styled("Avg Cost", theme.label())).alignment(Alignment::Right)),
        Cell::from(
            Line::from(Span::styled("Avg Latency", theme.label())).alignment(Alignment::Right),
        ),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let mut sorted_models: Vec<_> = model_stats.iter().collect();
    sorted_models.sort_by(|a, b| b.1.count.cmp(&a.1.count));

    let rows: Vec<Row> = sorted_models
        .iter()
        .map(|(model, stats)| {
            let pass_pct = if stats.count > 0 {
                format!("{:.1}%", stats.passed as f64 / stats.count as f64 * 100.0)
            } else {
                "\u{2014}".to_string()
            };
            let avg_cost = if stats.count > 0 {
                format!("${:.4}", stats.total_cost / stats.count as f64)
            } else {
                "\u{2014}".to_string()
            };
            let avg_latency = if stats.count > 0 {
                format!("{}ms", stats.total_latency_ms / stats.count as u64)
            } else {
                "\u{2014}".to_string()
            };

            let rate_color = if stats.count == 0 {
                theme.muted
            } else if stats.passed * 100 >= stats.count * 80 {
                Theme::RATE_GOOD
            } else if stats.passed * 100 >= stats.count * 50 {
                Theme::RATE_MID
            } else {
                Theme::RATE_BAD
            };

            Row::new(vec![
                Cell::from(Span::styled(
                    display_model(Some(model.as_str())),
                    theme.value(),
                )),
                Cell::from(
                    Line::from(Span::styled(stats.count.to_string(), theme.value()))
                        .alignment(Alignment::Right),
                ),
                Cell::from(
                    Line::from(Span::styled(stats.passed.to_string(), theme.value()))
                        .alignment(Alignment::Right),
                ),
                Cell::from(
                    Line::from(Span::styled(pass_pct, Style::default().fg(rate_color)))
                        .alignment(Alignment::Right),
                ),
                Cell::from(
                    Line::from(Span::styled(avg_cost, theme.metadata()))
                        .alignment(Alignment::Right),
                ),
                Cell::from(
                    Line::from(Span::styled(avg_latency, theme.metadata()))
                        .alignment(Alignment::Right),
                ),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Percentage(13),
            Constraint::Percentage(18),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(
        Block::bordered()
            .title(Span::styled(
                " Model Efficiency Stats ",
                theme.section_header(),
            ))
            .border_style(theme.muted()),
    );

    frame.render_widget(table, chunks[0]);
    render_separator(frame, chunks[1], theme);

    // -- Cost bar chart --
    let colors = Theme::SERIES_COLORS;
    let bars: Vec<Bar> = sorted_models
        .iter()
        .enumerate()
        .filter(|(_, (_, stats))| stats.count > 0)
        .map(|(i, (model, stats))| {
            let avg = stats.total_cost / stats.count as f64;
            let value = (avg * 10000.0).round() as u64;
            let label = display_model(Some(model.as_str()));
            let label = if label.len() > 12 {
                label[..12].to_string()
            } else {
                label
            };
            Bar::default()
                .value(value)
                .label(Line::from(label))
                .style(Style::default().fg(colors[i % colors.len()]))
        })
        .collect();

    if !bars.is_empty() {
        let bar_chart = BarChart::default()
            .block(
                Block::bordered()
                    .title(Span::styled(
                        " Avg Cost (\u{00d7}10\u{207b}\u{2074} $) ",
                        theme.section_header(),
                    ))
                    .border_style(theme.muted()),
            )
            .data(BarGroup::default().bars(&bars))
            .bar_width(
                chunks[2]
                    .width
                    .saturating_sub(4)
                    .checked_div(bars.len().max(1) as u16)
                    .unwrap_or(5)
                    .min(12)
                    .max(3),
            )
            .bar_gap(1);

        frame.render_widget(bar_chart, chunks[2]);
    }
}

#[derive(Debug, Default)]
struct ModelEffStats {
    count: usize,
    passed: usize,
    total_cost: f64,
    total_latency_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::dashboard::{CascadeRouterModelStats, DashboardData};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

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

    fn render_view(tui_state: &TuiState, sub_tab: usize, width: u16, height: u16) -> String {
        let data = DashboardData::default();
        let view_state = ViewState {
            sub_tab,
            ..ViewState::default()
        };
        let theme = Theme::dark();
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render(frame, area, &data, tui_state, &view_state, &theme);
            })
            .unwrap();
        rendered_text(&terminal)
    }

    fn efficiency_event(
        model: &str,
        task_id: &str,
    ) -> roko_learn::efficiency::AgentEfficiencyEvent {
        let mut event = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        event.model = model.to_string();
        event.plan_id = "plan-1".to_string();
        event.task_id = task_id.to_string();
        event.gate_passed = Some(true);
        event.cost_usd = 0.05;
        event.wall_time_ms = 4_000;
        event
    }

    #[test]
    fn router_empty_state_renders_placeholder() {
        let state = TuiState::new();
        let text = render_view(&state, 0, 100, 20);
        assert!(text.contains("No learning data"), "missing:\n{text}");
    }

    #[test]
    fn router_renders_non_claude_model_rows() {
        let mut state = TuiState::new();
        state.cascade_router.model_slugs = vec!["gpt-5.6-sol".to_string(), "glm-5.1".to_string()];
        state.cascade_router.confidence_stats.insert(
            "gpt-5.6-sol".to_string(),
            CascadeRouterModelStats {
                trials: 12,
                successes: 9,
            },
        );
        state.cascade_router.confidence_stats.insert(
            "glm-5.1".to_string(),
            CascadeRouterModelStats {
                trials: 4,
                successes: 1,
            },
        );

        let text = render_view(&state, 0, 120, 30);
        assert!(
            text.contains("Cascade Stage"),
            "stage block missing:\n{text}"
        );
        assert!(text.contains("gpt-5.6-sol"), "codex row missing:\n{text}");
        assert!(text.contains("glm-5.1"), "glm row missing:\n{text}");
        assert!(text.contains("75.0%"), "pass rate missing:\n{text}");
    }

    #[test]
    fn history_sub_view_renders_without_panic() {
        let mut state = TuiState::new();
        state.cascade_router.confidence_stats.insert(
            "glm-5.1".to_string(),
            CascadeRouterModelStats {
                trials: 15,
                successes: 10,
            },
        );
        let text = render_view(&state, 1, 100, 24);
        assert!(
            text.contains("Stage Transition History"),
            "history missing:\n{text}"
        );
    }

    #[test]
    fn efficiency_sub_view_renders_model_rows() {
        let mut state = TuiState::new();
        state
            .efficiency_events
            .push(efficiency_event("gpt-5.6-sol", "t1"));
        state
            .efficiency_events
            .push(efficiency_event("glm-5.1", "t2"));

        let text = render_view(&state, 2, 120, 30);
        assert!(
            text.contains("Model Efficiency Stats"),
            "stats block missing:\n{text}"
        );
        // Models are grouped by exact slug (shortened for display).
        assert!(text.contains("5.6-sol"), "codex model missing:\n{text}");
        assert!(text.contains("glm-5.1"), "glm model missing:\n{text}");
    }

    #[test]
    fn efficiency_sub_view_empty_renders_placeholder() {
        let state = TuiState::new();
        let text = render_view(&state, 2, 100, 20);
        assert!(
            text.contains("No efficiency events recorded yet"),
            "placeholder missing:\n{text}"
        );
    }
}
