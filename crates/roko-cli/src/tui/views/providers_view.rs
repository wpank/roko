//! F11 Providers view -- NERV-style institutional provider monitor.
//!
//! Layout (top to bottom):
//!   1. Header:     fullwidth katakana title + pattern status
//!   2. Unit array: per-provider cards with mini sparklines
//!   3. Waveforms:  cost/latency/error time series
//!   4. Detail:     full provider table
//!   5. Footer:     credit status strip
//!
//! Data source: `TuiState.efficiency_events` aggregated per-provider.

use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::state::TuiState;

// ---------------------------------------------------------------------------
// Block characters for waveform rendering
// ---------------------------------------------------------------------------

/// Block element characters mapped to 0..7 intensity levels.
const BLOCKS: [char; 8] = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];

/// Map a value in `0.0..=max` to a block character.
fn block_char(value: f64, max: f64) -> char {
    if max <= 0.0 || value <= 0.0 {
        return BLOCKS[0];
    }
    let idx = ((value / max) * 7.0).round().clamp(0.0, 7.0) as usize;
    BLOCKS[idx]
}

/// Render a sparkline string from a slice of values.
fn sparkline_str(values: &[f64]) -> String {
    let max = values.iter().copied().fold(0.0_f64, f64::max);
    values.iter().map(|v| block_char(*v, max)).collect()
}

// ---------------------------------------------------------------------------
// Per-provider aggregation (mirrors config_view::aggregate_providers logic)
// ---------------------------------------------------------------------------

/// Aggregated metrics for a single provider.
#[derive(Default)]
struct ProviderAgg {
    total_calls: u64,
    successes: u64,
    total_latency_ms: u64,
    total_cost: f64,
    models: std::collections::BTreeSet<String>,
    /// Recent cost samples for sparkline (bounded to 40 entries).
    cost_history: Vec<f64>,
    /// Recent latency samples for sparkline (bounded to 40 entries).
    latency_history: Vec<f64>,
    /// Recent error rate samples (0.0 or 1.0 per call).
    error_history: Vec<f64>,
}

impl ProviderAgg {
    fn success_rate(&self) -> f64 {
        if self.total_calls > 0 {
            self.successes as f64 / self.total_calls as f64 * 100.0
        } else {
            0.0
        }
    }

    fn avg_latency_ms(&self) -> f64 {
        if self.total_calls > 0 {
            self.total_latency_ms as f64 / self.total_calls as f64
        } else {
            0.0
        }
    }

    fn error_rate(&self) -> f64 {
        if self.total_calls > 0 {
            (self.total_calls - self.successes) as f64 / self.total_calls as f64 * 100.0
        } else {
            0.0
        }
    }

    fn status_icon(&self) -> (&'static str, &'static str, StatusKind) {
        if self.total_calls == 0 {
            ("\u{25cb}", "no data", StatusKind::Unconfigured) // open circle
        } else if self.success_rate() >= 90.0 {
            ("\u{25cf}", "healthy", StatusKind::Healthy) // filled circle
        } else if self.success_rate() >= 70.0 {
            ("~", "degraded", StatusKind::Degraded)
        } else {
            ("!", "unhealthy", StatusKind::Unhealthy)
        }
    }

    fn models_display(&self) -> String {
        let names: Vec<&str> = self.models.iter().map(String::as_str).collect();
        if names.is_empty() {
            "--".to_string()
        } else if names.len() <= 3 {
            names.join(", ")
        } else {
            format!("{}, +{}", names[..2].join(", "), names.len() - 2)
        }
    }
}

#[derive(Clone, Copy)]
enum StatusKind {
    Healthy,
    Degraded,
    Unhealthy,
    Unconfigured,
}

impl StatusKind {
    fn style(self, theme: &Theme) -> Style {
        match self {
            Self::Healthy => theme.success(),
            Self::Degraded => theme.warning(),
            Self::Unhealthy => theme.danger(),
            Self::Unconfigured => theme.muted(),
        }
    }
}

/// Infer provider name from a model slug.
fn infer_provider(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("claude") || lower.contains("anthropic") {
        "anthropic".to_string()
    } else if lower.contains("gpt") || lower.contains("openai") || lower.contains("o1") || lower.contains("o3") || lower.contains("o4") {
        "openai".to_string()
    } else if lower.contains("gemini") || lower.contains("google") {
        "google".to_string()
    } else if lower.contains("sonar") || lower.contains("perplexity") {
        "perplexity".to_string()
    } else if lower.contains("cerebras") || lower.contains("llama") {
        "cerebras".to_string()
    } else {
        trimmed.split('/').next().unwrap_or(trimmed).to_string()
    }
}

/// Shorten a model name for compact display (e.g. "claude-sonnet-4-5" -> "sonnet-4-5").
fn short_model(model: &str) -> String {
    let s = model.trim();
    // Strip common prefixes.
    for prefix in &["claude-", "gpt-", "gemini-", "models/"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    s.to_string()
}

const HISTORY_CAP: usize = 40;

fn aggregate_providers(tui_state: &TuiState) -> BTreeMap<String, ProviderAgg> {
    let mut providers: BTreeMap<String, ProviderAgg> = BTreeMap::new();
    for event in &tui_state.efficiency_events {
        let name = infer_provider(&event.model);
        let entry = providers.entry(name).or_default();
        entry.total_calls += 1;
        if event.output_tokens > 0 {
            entry.successes += 1;
        }
        entry.total_latency_ms += event.wall_time_ms;
        entry.total_cost += event.cost_usd;
        entry.models.insert(short_model(&event.model));

        // Append to bounded history vectors.
        if entry.cost_history.len() >= HISTORY_CAP {
            entry.cost_history.remove(0);
        }
        entry.cost_history.push(event.cost_usd);

        if entry.latency_history.len() >= HISTORY_CAP {
            entry.latency_history.remove(0);
        }
        entry.latency_history.push(event.wall_time_ms as f64);

        if entry.error_history.len() >= HISTORY_CAP {
            entry.error_history.remove(0);
        }
        entry.error_history.push(if event.output_tokens > 0 { 0.0 } else { 1.0 });
    }
    providers
}

// ---------------------------------------------------------------------------
// Global waveform aggregation
// ---------------------------------------------------------------------------

/// Aggregate global waveform histories across all providers.
struct GlobalWaveforms {
    cost: Vec<f64>,
    latency: Vec<f64>,
    error: Vec<f64>,
}

fn aggregate_global_waveforms(tui_state: &TuiState) -> GlobalWaveforms {
    let cap = HISTORY_CAP;
    let mut cost = Vec::with_capacity(cap);
    let mut latency = Vec::with_capacity(cap);
    let mut error = Vec::with_capacity(cap);

    for event in tui_state.efficiency_events.iter().rev().take(cap).rev() {
        cost.push(event.cost_usd);
        latency.push(event.wall_time_ms as f64);
        error.push(if event.output_tokens > 0 { 0.0 } else { 1.0 });
    }

    GlobalWaveforms { cost, latency, error }
}

// ---------------------------------------------------------------------------
// Determine overall pattern status
// ---------------------------------------------------------------------------

fn pattern_status(providers: &BTreeMap<String, ProviderAgg>) -> (&'static str, Style) {
    if providers.is_empty() {
        return ("STANDBY", Style::default().fg(Theme::TEXT_GHOST));
    }
    let any_unhealthy = providers.values().any(|p| p.total_calls > 0 && p.success_rate() < 70.0);
    let any_degraded = providers.values().any(|p| p.total_calls > 0 && p.success_rate() < 90.0);
    if any_unhealthy {
        return ("ALERT", Style::default().fg(Theme::EMBER).add_modifier(Modifier::BOLD));
    }
    if any_degraded {
        return ("CAUTION", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD));
    }
    ("NOMINAL", Style::default().fg(Theme::SAGE).add_modifier(Modifier::BOLD))
}

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

/// Render the full NERV-style providers monitor view.
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    _view_state: &ViewState,
    theme: &Theme,
) {
    let providers = aggregate_providers(tui_state);
    let waveforms = aggregate_global_waveforms(tui_state);

    // Outer NERV double-border frame.
    let outer_block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Theme::ROSE_DIM))
        .style(Style::default().bg(Theme::BG));
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    if inner.height < 10 || inner.width < 30 {
        // Terminal too small -- render a minimal placeholder.
        let msg = Paragraph::new("Terminal too small for provider monitor")
            .style(theme.muted())
            .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
        return;
    }

    // Vertical layout: header | unit array | waveforms | detail table | footer.
    let _provider_count = providers.len().max(1);
    let unit_array_height = 6_u16; // 4 content rows + 2 border
    let waveform_height = 5_u16;   // 3 rows + 2 border
    let footer_height = 1_u16;

    let sections = Layout::vertical([
        Constraint::Length(2),                // header
        Constraint::Length(unit_array_height), // unit array
        Constraint::Length(waveform_height),   // waveforms
        Constraint::Min(4),                   // detail table
        Constraint::Length(footer_height),     // footer
    ])
    .split(inner);

    render_header(frame, sections[0], &providers, theme);
    render_unit_array(frame, sections[1], &providers, theme);
    render_waveforms(frame, sections[2], &waveforms, theme);
    render_detail_table(frame, sections[3], &providers, theme);
    render_footer(frame, sections[4], &providers, theme);
}

// ---------------------------------------------------------------------------
// Section 1: Header
// ---------------------------------------------------------------------------

fn render_header(
    frame: &mut Frame<'_>,
    area: Rect,
    providers: &BTreeMap<String, ProviderAgg>,
    _theme: &Theme,
) {
    let (status_label, status_style) = pattern_status(providers);

    let title_spans = vec![
        Span::styled(
            "\u{2308} \u{FF30}\u{FF32}\u{FF2F}\u{FF36}\u{FF29}\u{FF24}\u{FF25}\u{FF32} \u{FF2D}\u{FF2F}\u{FF2E}\u{FF29}\u{FF34}\u{FF2F}\u{FF32} \u{2309}",
            Style::default()
                .fg(Theme::BONE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   PATTERN: ", Style::default().fg(Theme::TEXT_GHOST)),
        Span::styled(status_label, status_style),
    ];

    let total_cost: f64 = providers.values().map(|p| p.total_cost).sum();
    let total_calls: u64 = providers.values().map(|p| p.total_calls).sum();

    let stats_spans = vec![
        Span::styled(
            format!("  ${total_cost:.2} total"),
            Style::default().fg(Theme::BONE_DIM),
        ),
        Span::styled("  \u{00b7}  ", Style::default().fg(Theme::TEXT_PHANTOM)),
        Span::styled(
            format!("{total_calls} calls"),
            Style::default().fg(Theme::TEXT_DIM),
        ),
        Span::styled("  \u{00b7}  ", Style::default().fg(Theme::TEXT_PHANTOM)),
        Span::styled(
            format!("{} providers", providers.len()),
            Style::default().fg(Theme::TEXT_DIM),
        ),
    ];

    let header = Paragraph::new(vec![
        Line::from(title_spans),
        Line::from(stats_spans),
    ])
    .style(Style::default().bg(Theme::BG_RAISED))
    .alignment(Alignment::Center);

    frame.render_widget(header, area);
}

// ---------------------------------------------------------------------------
// Section 2: Unit array (per-provider cards)
// ---------------------------------------------------------------------------

fn render_unit_array(
    frame: &mut Frame<'_>,
    area: Rect,
    providers: &BTreeMap<String, ProviderAgg>,
    theme: &Theme,
) {
    if providers.is_empty() {
        crate::tui::empty_state::render_pane_empty_compact(
            frame,
            area,
            "No provider data \u{00b7} run agents to populate",
            theme,
        );
        return;
    }

    // Distribute horizontal space equally among providers (cap at 8 visible).
    let visible: Vec<(&String, &ProviderAgg)> = providers.iter().take(8).collect();
    let count = visible.len();
    let constraints: Vec<Constraint> = (0..count)
        .map(|_| Constraint::Ratio(1, count as u32))
        .collect();

    let cols = Layout::horizontal(constraints).split(area);

    for (i, (name, prov)) in visible.iter().enumerate() {
        render_provider_card(frame, cols[i], name, prov, theme);
    }
}

fn render_provider_card(
    frame: &mut Frame<'_>,
    area: Rect,
    name: &str,
    prov: &ProviderAgg,
    theme: &Theme,
) {
    let (icon, _status_label, kind) = prov.status_icon();
    let border_style = kind.style(theme).remove_modifier(Modifier::BOLD);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {name} "),
            Style::default()
                .fg(Theme::BONE_DIM)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 6 {
        return;
    }

    // Row 1: icon + cost
    let cost_str = format!("{icon} ${:.2}", prov.total_cost);
    let cost_style = kind.style(theme);

    // Row 2: mini sparkline from cost history
    let spark_width = inner.width as usize;
    let spark_data: Vec<f64> = if prov.cost_history.len() > spark_width {
        prov.cost_history[prov.cost_history.len() - spark_width..].to_vec()
    } else {
        let mut padded = vec![0.0; spark_width.saturating_sub(prov.cost_history.len())];
        padded.extend_from_slice(&prov.cost_history);
        padded
    };
    let spark_str = sparkline_str(&spark_data);

    // Row 3: success rate + avg latency
    let rate = prov.success_rate();
    let latency = prov.avg_latency_ms();
    let latency_display = if latency >= 1000.0 {
        format!("{:.1}s", latency / 1000.0)
    } else {
        format!("{latency:.0}ms")
    };
    let stats_str = if prov.total_calls == 0 {
        "-- no calls".to_string()
    } else {
        format!("{rate:.0}% {latency_display}")
    };

    let lines = vec![
        Line::from(Span::styled(cost_str, cost_style)),
        Line::from(Span::styled(spark_str, Style::default().fg(Theme::DREAM))),
        Line::from(Span::styled(
            stats_str,
            Style::default().fg(Theme::TEXT_DIM),
        )),
    ];

    let card_text = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(card_text, inner);
}

// ---------------------------------------------------------------------------
// Section 3: Waveforms
// ---------------------------------------------------------------------------

fn render_waveforms(
    frame: &mut Frame<'_>,
    area: Rect,
    waveforms: &GlobalWaveforms,
    _theme: &Theme,
) {
    let block = Block::new()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(Theme::SEPARATOR));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let wave_width = (inner.width as usize).saturating_sub(14); // label column ~14 chars
    render_waveform_line(frame, rows[0], "COST $/call", &waveforms.cost, Theme::ROSE, wave_width);
    render_waveform_line(frame, rows[1], "LATENCY ms ", &waveforms.latency, Theme::DREAM, wave_width);
    render_waveform_line(frame, rows[2], "ERROR rate ", &waveforms.error, Theme::EMBER, wave_width);
}

fn render_waveform_line(
    frame: &mut Frame<'_>,
    area: Rect,
    label: &str,
    data: &[f64],
    color: ratatui::style::Color,
    width: usize,
) {
    let sample: Vec<f64> = if data.len() > width {
        data[data.len() - width..].to_vec()
    } else {
        let mut padded = vec![0.0; width.saturating_sub(data.len())];
        padded.extend_from_slice(data);
        padded
    };
    let wave_str = sparkline_str(&sample);

    let line = Line::from(vec![
        Span::styled(
            format!("  {label}  "),
            Style::default()
                .fg(Theme::TEXT_GHOST)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(wave_str, Style::default().fg(color)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

// ---------------------------------------------------------------------------
// Section 4: Detail table
// ---------------------------------------------------------------------------

#[allow(clippy::cast_precision_loss)]
fn render_detail_table(
    frame: &mut Frame<'_>,
    area: Rect,
    providers: &BTreeMap<String, ProviderAgg>,
    theme: &Theme,
) {
    let block = Block::new()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Theme::SEPARATOR));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if providers.is_empty() {
        crate::tui::empty_state::render_pane_empty_compact(
            frame,
            inner,
            "No provider data",
            theme,
        );
        return;
    }

    let header_style = Style::default()
        .fg(Theme::COL_HEADER)
        .add_modifier(Modifier::BOLD);

    let header = Row::new(vec![
        Cell::from(Span::styled("Provider", header_style)),
        Cell::from(Span::styled("Models", header_style)),
        Cell::from(Span::styled("Requests", header_style)),
        Cell::from(Span::styled("Cost", header_style)),
        Cell::from(Span::styled("Latency", header_style)),
        Cell::from(Span::styled("Rate", header_style)),
        Cell::from(Span::styled("Errors", header_style)),
        Cell::from(Span::styled("Status", header_style)),
    ]);

    let rows: Vec<Row<'_>> = providers
        .iter()
        .map(|(name, prov)| {
            let (icon, status_label, kind) = prov.status_icon();
            let status_style = kind.style(theme);

            let rate = prov.success_rate();
            let avg_lat = prov.avg_latency_ms();
            let err_rate = prov.error_rate();

            let latency_display = if avg_lat >= 1000.0 {
                format!("{:.1}s", avg_lat / 1000.0)
            } else {
                format!("{avg_lat:.0}ms")
            };

            let rate_style = if rate >= 90.0 {
                theme.success()
            } else if rate >= 70.0 {
                theme.warning()
            } else {
                theme.danger()
            };

            let err_style = if err_rate > 30.0 {
                theme.danger()
            } else if err_rate > 10.0 {
                theme.warning()
            } else {
                theme.muted()
            };

            let latency_style = if avg_lat > 10_000.0 {
                theme.danger()
            } else if avg_lat > 5_000.0 {
                theme.warning()
            } else {
                theme.value()
            };

            Row::new(vec![
                Cell::from(Span::styled(
                    name.clone(),
                    Style::default().fg(Theme::BONE_DIM),
                )),
                Cell::from(Span::styled(
                    truncate_str(&prov.models_display(), 20),
                    theme.muted(),
                )),
                Cell::from(Span::styled(prov.total_calls.to_string(), theme.value())),
                Cell::from(Span::styled(
                    format!("${:.3}", prov.total_cost),
                    theme.value(),
                )),
                Cell::from(Span::styled(latency_display, latency_style)),
                Cell::from(Span::styled(format!("{rate:.0}%"), rate_style)),
                Cell::from(Span::styled(format!("{err_rate:.0}%"), err_style)),
                Cell::from(Span::styled(
                    format!("{icon} {status_label}"),
                    status_style,
                )),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(12),     // provider
        Constraint::Min(16),     // models
        Constraint::Length(10),  // requests
        Constraint::Length(10),  // cost
        Constraint::Length(10),  // latency
        Constraint::Length(8),   // rate
        Constraint::Length(8),   // errors
        Constraint::Length(14),  // status
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(theme.selection());

    frame.render_widget(table, inner);
}

/// Truncate a string to `max_len` characters, appending ellipsis if needed.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max_len.saturating_sub(1)])
    }
}

// ---------------------------------------------------------------------------
// Section 5: Footer (credit strip)
// ---------------------------------------------------------------------------

fn render_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    providers: &BTreeMap<String, ProviderAgg>,
    theme: &Theme,
) {
    let mut spans: Vec<Span<'_>> = vec![Span::styled(
        " Credits: ",
        Style::default().fg(Theme::TEXT_GHOST),
    )];

    for (i, (name, prov)) in providers.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " \u{2502} ",
                Style::default().fg(Theme::SEPARATOR),
            ));
        }
        let (icon, _status, kind) = prov.status_icon();
        let style = kind.style(theme);
        spans.push(Span::styled(format!("{name} {icon}"), style));
    }

    if providers.is_empty() {
        spans.push(Span::styled("no providers", theme.muted()));
    }

    let footer = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Theme::BG_RAISED))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}
