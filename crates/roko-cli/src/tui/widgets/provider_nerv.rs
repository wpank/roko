//! NERV-style provider monitoring widgets.
//!
//! Four visual components inspired by Evangelion's MAGI display system,
//! rendered with the ROSEDUST palette:
//!
//! - **Unit array** (B1): grid of provider status cells with sparklines
//! - **Waveform** (B3): multi-channel scrolling oscilloscope traces
//! - **Provider detail**: expanded single-provider health panel
//! - **Credit status bar**: compact one-line credit indicator

use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::dashboard::Theme;
use crate::tui::state::TuiState;

// ---------------------------------------------------------------------------
// Block characters for oscilloscope traces
// ---------------------------------------------------------------------------

/// Eight-level block chart characters (bottom-to-top fill).
const BLOCKS: [char; 8] = [
    '\u{2581}', // ▁
    '\u{2582}', // ▂
    '\u{2583}', // ▃
    '\u{2584}', // ▄
    '\u{2585}', // ▅
    '\u{2586}', // ▆
    '\u{2587}', // ▇
    '\u{2588}', // █
];

/// Map a normalized 0.0..=1.0 value to a block character.
fn block_char(t: f64) -> char {
    let idx = (t.clamp(0.0, 1.0) * 7.0).round() as usize;
    BLOCKS[idx.min(7)]
}

// ---------------------------------------------------------------------------
// Display-oriented provider status
// ---------------------------------------------------------------------------

/// Aggregated provider status for the NERV display widgets.
///
/// Built from `TuiState` efficiency events rather than requiring direct
/// access to `roko_learn::ProviderStatus` (whose fields include
/// `std::time::Instant` and `pub(crate)` visibility).
#[derive(Debug, Clone)]
pub struct ProviderNervStatus {
    /// Provider name (e.g. "anthropic", "openai").
    pub name: String,
    /// Total requests observed.
    pub total_requests: u64,
    /// Successful requests (output_tokens > 0).
    pub successes: u64,
    /// Total cost in USD.
    pub total_cost_usd: f64,
    /// Total latency in milliseconds (sum of wall_time_ms).
    pub total_latency_ms: u64,
    /// Per-request cost timeline for sparkline rendering.
    pub cost_timeline: Vec<f64>,
    /// Per-request latency timeline for sparkline rendering.
    pub latency_timeline: Vec<f64>,
    /// Per-request error timeline (1.0 = error, 0.0 = success).
    pub error_timeline: Vec<f64>,
    /// Set of active models seen for this provider.
    pub models: Vec<String>,
    /// Circuit breaker state label.
    pub circuit_state: CircuitLabel,
    /// Whether the provider uses CLI dispatch (no billing/credit tracking).
    pub is_cli: bool,
}

/// Simplified circuit breaker label for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitLabel {
    Closed,
    Open,
    HalfOpen,
    Unknown,
}

impl CircuitLabel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "HEALTHY",
            Self::Open => "DOWN",
            Self::HalfOpen => "PROBING",
            Self::Unknown => "N/A",
        }
    }
}

impl ProviderNervStatus {
    /// Success rate as a percentage (0.0 - 100.0).
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.successes as f64 / self.total_requests as f64 * 100.0
        }
    }

    /// Average latency in milliseconds.
    #[must_use]
    pub fn avg_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_latency_ms as f64 / self.total_requests as f64
        }
    }

    /// Error rate as a percentage (0.0 - 100.0).
    #[must_use]
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.total_requests - self.successes) as f64 / self.total_requests as f64 * 100.0
        }
    }

    /// Cost per minute (estimated from the timeline).
    #[must_use]
    pub fn cost_per_min(&self) -> f64 {
        if self.total_latency_ms == 0 {
            return 0.0;
        }
        let minutes = self.total_latency_ms as f64 / 60_000.0;
        if minutes < 0.001 {
            0.0
        } else {
            self.total_cost_usd / minutes
        }
    }

    /// Health color for this provider in the ROSEDUST palette.
    fn health_color(&self) -> ratatui::style::Color {
        let rate = self.success_rate();
        if self.total_requests == 0 {
            Theme::TEXT_GHOST
        } else if rate >= 90.0 {
            Theme::SAGE
        } else if rate >= 70.0 {
            Theme::WARNING
        } else {
            Theme::EMBER
        }
    }

    /// Status dot character.
    fn dot(&self) -> &'static str {
        if self.is_cli {
            "\u{25c6}" // ◆ diamond for CLI
        } else if self.total_requests == 0 {
            "\u{25cb}" // ○ empty circle
        } else if self.success_rate() >= 90.0 {
            "\u{25cf}" // ● filled circle
        } else if self.success_rate() >= 70.0 {
            "\u{25d1}" // ◑ half-filled
        } else {
            "\u{25cb}" // ○ empty circle (error)
        }
    }
}

// ---------------------------------------------------------------------------
// Aggregate from TuiState
// ---------------------------------------------------------------------------

/// Build NERV provider status entries from `TuiState` efficiency events.
pub fn aggregate_nerv_providers(tui_state: &TuiState) -> Vec<ProviderNervStatus> {
    let mut map: BTreeMap<String, ProviderNervStatus> = BTreeMap::new();

    for event in &tui_state.efficiency_events {
        let provider_name = infer_provider_name(&event.model, &event.backend);
        let entry = map.entry(provider_name.clone()).or_insert_with(|| {
            ProviderNervStatus {
                name: provider_name,
                total_requests: 0,
                successes: 0,
                total_cost_usd: 0.0,
                total_latency_ms: 0,
                cost_timeline: Vec::new(),
                latency_timeline: Vec::new(),
                error_timeline: Vec::new(),
                models: Vec::new(),
                circuit_state: CircuitLabel::Unknown,
                is_cli: false,
            }
        });

        entry.total_requests += 1;
        let is_success = event.output_tokens > 0;
        if is_success {
            entry.successes += 1;
        }
        entry.total_cost_usd += event.cost_usd;
        let latency = if event.wall_time_ms > 0 {
            event.wall_time_ms
        } else {
            event.duration_ms
        };
        entry.total_latency_ms += latency;
        entry.cost_timeline.push(event.cost_usd);
        entry.latency_timeline.push(latency as f64);
        entry.error_timeline.push(if is_success { 0.0 } else { 1.0 });

        // Track unique models.
        let model = event.model.trim();
        if !model.is_empty() && !entry.models.contains(&model.to_string()) {
            entry.models.push(model.to_string());
        }

        // Detect CLI-based providers.
        let backend_lower = event.backend.to_ascii_lowercase();
        if backend_lower.contains("cli") || backend_lower.contains("codex") {
            entry.is_cli = true;
        }
    }

    // Infer circuit state from recent success rate.
    for entry in map.values_mut() {
        if entry.total_requests == 0 {
            entry.circuit_state = CircuitLabel::Unknown;
        } else if entry.success_rate() >= 70.0 {
            entry.circuit_state = CircuitLabel::Closed;
        } else if entry.success_rate() >= 30.0 {
            entry.circuit_state = CircuitLabel::HalfOpen;
        } else {
            entry.circuit_state = CircuitLabel::Open;
        }
    }

    map.into_values().collect()
}

/// Infer provider name from model slug and backend identifier.
fn infer_provider_name(model: &str, backend: &str) -> String {
    // Prefer backend if non-empty and descriptive.
    let be = backend.trim().to_ascii_lowercase();
    if !be.is_empty() && be != "unknown" {
        // Normalize common backend names to short provider names.
        if be.contains("anthropic") || be.contains("claude") {
            return "anthropic".to_string();
        }
        if be.contains("openai") || be.contains("gpt") {
            return "openai".to_string();
        }
        if be.contains("gemini") || be.contains("google") {
            return "google".to_string();
        }
        if be.contains("cerebras") {
            return "cerebras".to_string();
        }
        if be.contains("perplexity") || be.contains("pplx") {
            return "perplexity".to_string();
        }
        // Use as-is for other backends.
        return be;
    }
    // Fall back to model name heuristics.
    let lower = model.trim().to_ascii_lowercase();
    if lower.contains("claude") || lower.contains("anthropic") {
        "anthropic".to_string()
    } else if lower.contains("gpt") || lower.contains("openai") || lower.contains("o1") {
        "openai".to_string()
    } else if lower.contains("gemini") || lower.contains("google") {
        "google".to_string()
    } else if lower.contains("cerebras") {
        "cerebras".to_string()
    } else if lower.contains("perplexity") || lower.contains("pplx") {
        "perplexity".to_string()
    } else if lower.is_empty() {
        "unknown".to_string()
    } else {
        lower.split('/').next().unwrap_or(&lower).to_string()
    }
}

// ---------------------------------------------------------------------------
// Widget 1: Provider Unit Array (NERV B1 pattern)
// ---------------------------------------------------------------------------

/// NERV B1 pattern -- grid of provider unit cells.
///
/// Each cell is ~20 chars wide x 5 rows:
/// ```text
/// ┌─ openai ──────────┐
/// │ ● HEALTHY   $2.41 │
/// │ ▃▅▇▆▄▃▅▆▇▅ 42req │
/// │ gpt-4o, gpt-4o-m… │
/// │ 98% pass  124ms   │
/// └────────────────────┘
/// ```
pub fn render_provider_unit_array(
    frame: &mut Frame<'_>,
    area: Rect,
    providers: &[ProviderNervStatus],
) {
    let theme = Theme::dark();

    if providers.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "  No provider data \u{2014} run agents to populate",
            theme.muted(),
        ));
        frame.render_widget(empty, area);
        return;
    }

    // Each cell is 22 chars wide (20 inner + 2 border), 5 rows + 2 border = 7.
    let cell_w: u16 = 22;
    let cell_h: u16 = 7;
    let cols = ((area.width + 1) / (cell_w + 1)).max(1) as usize;

    for (i, provider) in providers.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = area.x + (col as u16) * (cell_w + 1);
        let y = area.y + (row as u16) * cell_h;

        // Check bounds.
        if x + cell_w > area.x + area.width || y + cell_h > area.y + area.height {
            break;
        }

        let cell_area = Rect::new(x, y, cell_w, cell_h);
        render_unit_cell(frame, cell_area, provider, &theme);
    }
}

/// Render a single provider unit cell.
fn render_unit_cell(
    frame: &mut Frame<'_>,
    area: Rect,
    provider: &ProviderNervStatus,
    _theme: &Theme,
) {
    let health_color = provider.health_color();
    let title = format!(" {} ", provider.name);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, Style::default().fg(health_color)))
        .border_style(Style::default().fg(Theme::TEXT_PHANTOM))
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 10 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Line 1: dot + status + cost
    let dot = provider.dot();
    let state_label = provider.circuit_state.as_str();
    let cost_str = format_cost_compact(provider.total_cost_usd);
    let padding_len = (inner.width as usize)
        .saturating_sub(dot.len() + 1 + state_label.len() + cost_str.len());
    let padding = " ".repeat(padding_len);
    lines.push(Line::from(vec![
        Span::styled(
            format!("{dot} {state_label}"),
            Style::default().fg(health_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(padding, Style::default()),
        Span::styled(cost_str, Style::default().fg(Theme::BONE_DIM)),
    ]));

    // Line 2: cost sparkline + request count
    let req_label = format!("{}req", provider.total_requests);
    let spark_width = (inner.width as usize).saturating_sub(req_label.len() + 1);
    let spark = build_block_sparkline(&provider.cost_timeline, spark_width, Theme::ROSE_DIM);
    lines.push(Line::from(vec![
        Span::styled(spark, Style::default().fg(Theme::ROSE_DIM)),
        Span::styled(" ", Style::default()),
        Span::styled(req_label, Style::default().fg(Theme::TEXT_GHOST)),
    ]));

    // Line 3: active models (truncated)
    let models_str = if provider.models.is_empty() {
        "\u{2014}".to_string()
    } else {
        let joined = provider.models.join(", ");
        truncate_str(&joined, inner.width as usize)
    };
    lines.push(Line::from(Span::styled(
        models_str,
        Style::default().fg(Theme::TEXT_DIM),
    )));

    // Line 4: pass rate + avg latency
    let pass_pct = provider.success_rate();
    let pass_str = format!("{pass_pct:.0}% pass");
    let latency_str = format_latency_compact(provider.avg_latency_ms());
    let gap = (inner.width as usize).saturating_sub(pass_str.len() + latency_str.len());
    let pass_color = if pass_pct >= 90.0 {
        Theme::SAGE
    } else if pass_pct >= 70.0 {
        Theme::WARNING
    } else {
        Theme::EMBER
    };
    lines.push(Line::from(vec![
        Span::styled(pass_str, Style::default().fg(pass_color)),
        Span::styled(" ".repeat(gap), Style::default()),
        Span::styled(latency_str, Style::default().fg(Theme::TEXT_GHOST)),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Widget 2: Provider Waveform (NERV B3 oscilloscope)
// ---------------------------------------------------------------------------

/// NERV B3 oscilloscope -- multi-channel scrolling traces.
///
/// ```text
/// COST $/min  ▁▂▃▄▃▂▁▂▃▅▇▆▅▃▂▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▂▃▄
/// LATENCY ms  ▃▃▃▃▄▅▅▄▃▃▃▂▂▂▃▃▄▄▃▃▃▃▃▃▃▃▃▃▃▃▃▃▃▃
/// ERROR rate  ▁▁▁▁▁▁▁▁▁▁▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
/// ```
pub fn render_provider_waveform(
    frame: &mut Frame<'_>,
    area: Rect,
    providers: &[ProviderNervStatus],
) {
    let theme = Theme::dark();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Waveform ", theme.section_header()))
        .border_style(Theme::unfocused_border_style())
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 20 || inner.height < 3 {
        return;
    }

    // Merge all provider timelines into composite channels.
    let mut cost_data: Vec<f64> = Vec::new();
    let mut latency_data: Vec<f64> = Vec::new();
    let mut error_data: Vec<f64> = Vec::new();
    for p in providers {
        cost_data.extend_from_slice(&p.cost_timeline);
        latency_data.extend_from_slice(&p.latency_timeline);
        error_data.extend_from_slice(&p.error_timeline);
    }

    if cost_data.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "  Waiting for data\u{2026}",
            theme.muted(),
        ));
        frame.render_widget(empty, inner);
        return;
    }

    let label_width = 12; // "COST $/min  "
    let trace_width = (inner.width as usize).saturating_sub(label_width);

    let channels: Vec<(&str, &[f64], ratatui::style::Color)> = vec![
        ("COST $/min", &cost_data, Theme::ROSE_BRIGHT),
        ("LATENCY ms", &latency_data, Theme::DREAM),
        ("ERROR rate", &error_data, Theme::EMBER),
    ];

    let mut lines: Vec<Line> = Vec::new();

    for (label, data, color) in &channels {
        if lines.len() >= inner.height as usize {
            break;
        }
        let padded_label = format!("{:<12}", label);
        let trace = build_block_sparkline(data, trace_width, *color);
        lines.push(Line::from(vec![
            Span::styled(
                padded_label,
                Style::default()
                    .fg(Theme::BONE_DIM)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(trace, Style::default().fg(*color)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ---------------------------------------------------------------------------
// Widget 3: Provider Detail (expanded view)
// ---------------------------------------------------------------------------

/// Expanded detail panel for a selected provider.
///
/// Shows:
/// - Large health gauge bar
/// - Credit status
/// - Cost breakdown
/// - Error history
/// - Circuit breaker state
/// - Model list with per-model stats
pub fn render_provider_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    provider: &ProviderNervStatus,
) {
    let theme = Theme::dark();
    let health_color = provider.health_color();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" {} \u{2014} Detail ", provider.name),
            Style::default().fg(health_color).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Theme::TEXT_PHANTOM))
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 20 || inner.height < 6 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // -- Health gauge bar --
    let gauge_width = (inner.width as usize).saturating_sub(14); // "Health: " + " XX%"
    let rate = provider.success_rate() / 100.0;
    let filled = (rate * gauge_width as f64).round() as usize;
    let empty = gauge_width.saturating_sub(filled);
    let gauge_str = format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    );
    lines.push(Line::from(vec![
        Span::styled("Health: ", theme.label()),
        Span::styled(gauge_str, Style::default().fg(health_color)),
        Span::styled(
            format!(" {:.0}%", provider.success_rate()),
            Style::default()
                .fg(health_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // -- Credit status --
    let credit_str = if provider.is_cli {
        "CLI dispatch (no billing)".to_string()
    } else if provider.total_cost_usd > 0.0 {
        format!("${:.4} spent", provider.total_cost_usd)
    } else {
        "No cost data".to_string()
    };
    let credit_color = if provider.is_cli {
        Theme::DREAM
    } else if provider.total_cost_usd > 1.0 {
        Theme::ROSE_BRIGHT
    } else {
        Theme::BONE_DIM
    };
    lines.push(Line::from(vec![
        Span::styled("Credit: ", theme.label()),
        Span::styled(credit_str, Style::default().fg(credit_color)),
    ]));

    // -- Circuit breaker state --
    let circuit_color = match provider.circuit_state {
        CircuitLabel::Closed => Theme::SAGE,
        CircuitLabel::HalfOpen => Theme::WARNING,
        CircuitLabel::Open => Theme::EMBER,
        CircuitLabel::Unknown => Theme::TEXT_GHOST,
    };
    lines.push(Line::from(vec![
        Span::styled("Circuit: ", theme.label()),
        Span::styled(
            provider.circuit_state.as_str(),
            Style::default()
                .fg(circuit_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({} consecutive failures)", provider.total_requests - provider.successes),
            Style::default().fg(Theme::TEXT_GHOST),
        ),
    ]));

    // -- Cost breakdown --
    let avg_cost = if provider.total_requests > 0 {
        provider.total_cost_usd / provider.total_requests as f64
    } else {
        0.0
    };
    lines.push(Line::from(vec![
        Span::styled("Avg $/req: ", theme.label()),
        Span::styled(
            format_cost_compact(avg_cost),
            Style::default().fg(Theme::BONE_DIM),
        ),
        Span::styled("  Total: ", theme.label()),
        Span::styled(
            format_cost_compact(provider.total_cost_usd),
            Style::default().fg(Theme::BONE),
        ),
    ]));

    // -- Latency --
    lines.push(Line::from(vec![
        Span::styled("Latency: ", theme.label()),
        Span::styled(
            format_latency_compact(provider.avg_latency_ms()),
            Style::default().fg(Theme::TEXT_STRONG),
        ),
        Span::styled(" avg", Style::default().fg(Theme::TEXT_GHOST)),
    ]));

    // -- Error history sparkline --
    if !provider.error_timeline.is_empty() {
        let err_spark_width = (inner.width as usize).saturating_sub(10);
        let err_spark = build_block_sparkline(
            &provider.error_timeline,
            err_spark_width,
            Theme::EMBER,
        );
        lines.push(Line::from(vec![
            Span::styled("Errors:  ", theme.label()),
            Span::styled(err_spark, Style::default().fg(Theme::EMBER)),
        ]));
    }

    // -- Separator --
    if lines.len() + 2 < inner.height as usize {
        lines.push(Line::from(Span::styled(
            "\u{2500}".repeat(inner.width as usize),
            Style::default().fg(Theme::TEXT_PHANTOM),
        )));
    }

    // -- Model list --
    if !provider.models.is_empty() && lines.len() + 1 < inner.height as usize {
        lines.push(Line::from(vec![
            Span::styled("Models: ", theme.label()),
            Span::styled(
                truncate_str(&provider.models.join(", "), (inner.width as usize).saturating_sub(8)),
                Style::default().fg(Theme::TEXT_SOFT),
            ),
        ]));
    }

    // Model table if we have enough room.
    if provider.models.len() > 1 && lines.len() + provider.models.len() + 1 < inner.height as usize
    {
        for model in &provider.models {
            let short = shorten_model_name(model);
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    truncate_str(&short, 20),
                    Style::default().fg(Theme::TEXT),
                ),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Widget 4: Credit Status Bar (compact one-liner)
// ---------------------------------------------------------------------------

/// Compact bar showing credit status across all providers.
///
/// ```text
/// Credits: openai ● | anthropic ● | google ○ | perplexity ● | claude-cli ◆
/// ```
///
/// - Green filled dot `\u{25cf}` = available (success rate >= 70%)
/// - Red empty dot `\u{25cb}` = no credits or high error rate
/// - Diamond `\u{25c6}` = CLI dispatch (no billing)
pub fn render_credit_status_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    providers: &[ProviderNervStatus],
) {
    let theme = Theme::dark();

    if providers.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "Credits: \u{2014}",
            theme.muted(),
        ));
        frame.render_widget(empty, area);
        return;
    }

    let mut spans: Vec<Span> = vec![Span::styled(
        "Credits: ",
        Style::default()
            .fg(Theme::BONE_DIM)
            .add_modifier(Modifier::BOLD),
    )];

    for (i, p) in providers.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " \u{2502} ",
                Style::default().fg(Theme::TEXT_PHANTOM),
            )); // │ separator
        }

        let dot = p.dot();
        let color = p.health_color();
        spans.push(Span::styled(
            format!("{} ", p.name),
            Style::default().fg(Theme::TEXT_DIM),
        ));
        spans.push(Span::styled(dot, Style::default().fg(color)));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

// ---------------------------------------------------------------------------
// Convenience: render all NERV widgets in a vertical stack
// ---------------------------------------------------------------------------

/// Render the full NERV provider monitoring panel using a vertical layout.
///
/// Distributes the available area among unit array, waveform, and credit bar.
/// If a `selected_idx` is provided and in bounds, also shows the detail panel.
pub fn render_nerv_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    selected_idx: Option<usize>,
) {
    let providers = aggregate_nerv_providers(tui_state);

    if providers.is_empty() {
        let theme = Theme::dark();
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " Provider Monitor ",
                theme.section_header(),
            ))
            .border_style(Theme::unfocused_border_style())
            .style(Theme::block_style());
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let empty = Paragraph::new(Span::styled(
            "  No provider data \u{2014} run agents to populate",
            theme.muted(),
        ));
        frame.render_widget(empty, inner);
        return;
    }

    let show_detail = selected_idx
        .map(|idx| idx < providers.len())
        .unwrap_or(false);

    // Layout: credit bar (1 row) + unit array + waveform (5 rows) [+ detail]
    let credit_h = 1u16;
    let waveform_h = 5u16;
    let detail_h = if show_detail { 12u16 } else { 0 };
    let unit_h = area
        .height
        .saturating_sub(credit_h + waveform_h + detail_h);

    let mut y = area.y;

    // Credit status bar.
    if credit_h > 0 {
        let bar_area = Rect::new(area.x, y, area.width, credit_h);
        render_credit_status_bar(frame, bar_area, &providers);
        y += credit_h;
    }

    // Unit array.
    if unit_h > 0 {
        let arr_area = Rect::new(area.x, y, area.width, unit_h);
        render_provider_unit_array(frame, arr_area, &providers);
        y += unit_h;
    }

    // Waveform.
    if y + waveform_h <= area.y + area.height {
        let wave_area = Rect::new(area.x, y, area.width, waveform_h);
        render_provider_waveform(frame, wave_area, &providers);
        y += waveform_h;
    }

    // Detail panel (if selected).
    if show_detail {
        if let Some(idx) = selected_idx {
            if let Some(p) = providers.get(idx) {
                let remaining = (area.y + area.height).saturating_sub(y);
                if remaining >= 6 {
                    let detail_area = Rect::new(area.x, y, area.width, remaining);
                    render_provider_detail(frame, detail_area, p);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a block-character sparkline string from data.
///
/// Uses `\u{2581}` through `\u{2588}` for eight discrete levels.
/// Takes the last `width` samples (scrolling window).
fn build_block_sparkline(data: &[f64], width: usize, _color: ratatui::style::Color) -> String {
    if data.is_empty() || width == 0 {
        return " ".repeat(width);
    }

    let offset = data.len().saturating_sub(width);
    let visible = &data[offset..];

    let max = visible
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.001); // avoid division by zero

    let mut result = String::with_capacity(width * 3);
    for i in 0..width {
        let v = visible.get(i).copied().unwrap_or(0.0);
        let t = (v / max).clamp(0.0, 1.0);
        result.push(block_char(t));
    }
    result
}

/// Format a cost value compactly.
fn format_cost_compact(usd: f64) -> String {
    if usd < 0.001 {
        "-".to_string()
    } else if usd < 1.0 {
        format!("${usd:.3}")
    } else if usd < 100.0 {
        format!("${usd:.2}")
    } else {
        format!("${usd:.0}")
    }
}

/// Format latency compactly.
fn format_latency_compact(ms: f64) -> String {
    if ms < 1.0 {
        "-".to_string()
    } else if ms < 1000.0 {
        format!("{ms:.0}ms")
    } else if ms < 60_000.0 {
        format!("{:.1}s", ms / 1000.0)
    } else {
        format!("{:.1}m", ms / 60_000.0)
    }
}

/// Truncate a string to at most `max` characters with an ellipsis.
fn truncate_str(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "\u{2026}".to_string();
    }
    let keep = max - 1;
    let truncated: String = s.chars().take(keep).collect();
    format!("{truncated}\u{2026}")
}

/// Shorten a model name for compact display.
fn shorten_model_name(model: &str) -> String {
    model
        .replace("claude-", "")
        .replace("gpt-", "")
        .replace("-codex", "c")
        .replace("-mini", "m")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_providers() -> Vec<ProviderNervStatus> {
        vec![
            ProviderNervStatus {
                name: "openai".to_string(),
                total_requests: 42,
                successes: 41,
                total_cost_usd: 2.41,
                total_latency_ms: 5_200,
                cost_timeline: vec![0.05, 0.06, 0.04, 0.07, 0.05, 0.06, 0.08, 0.04],
                latency_timeline: vec![120.0, 135.0, 110.0, 145.0, 125.0, 130.0, 140.0, 115.0],
                error_timeline: vec![0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                models: vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                ],
                circuit_state: CircuitLabel::Closed,
                is_cli: false,
            },
            ProviderNervStatus {
                name: "anthropic".to_string(),
                total_requests: 28,
                successes: 26,
                total_cost_usd: 1.85,
                total_latency_ms: 8_400,
                cost_timeline: vec![0.08, 0.07, 0.06, 0.09, 0.07, 0.06],
                latency_timeline: vec![300.0, 280.0, 310.0, 290.0, 320.0, 300.0],
                error_timeline: vec![0.0, 0.0, 1.0, 0.0, 1.0, 0.0],
                models: vec!["claude-sonnet-4-20250514".to_string()],
                circuit_state: CircuitLabel::Closed,
                is_cli: false,
            },
            ProviderNervStatus {
                name: "claude-cli".to_string(),
                total_requests: 5,
                successes: 5,
                total_cost_usd: 0.0,
                total_latency_ms: 15_000,
                cost_timeline: vec![0.0; 5],
                latency_timeline: vec![3000.0; 5],
                error_timeline: vec![0.0; 5],
                models: vec!["claude-sonnet-4-20250514".to_string()],
                circuit_state: CircuitLabel::Closed,
                is_cli: true,
            },
        ]
    }

    #[test]
    fn unit_array_renders_without_panic() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let providers = sample_providers();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_provider_unit_array(frame, area, &providers);
            })
            .unwrap();
    }

    #[test]
    fn unit_array_empty_renders_without_panic() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_provider_unit_array(frame, area, &[]);
            })
            .unwrap();
    }

    #[test]
    fn waveform_renders_without_panic() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let providers = sample_providers();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_provider_waveform(frame, area, &providers);
            })
            .unwrap();
    }

    #[test]
    fn waveform_empty_renders_without_panic() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_provider_waveform(frame, area, &[]);
            })
            .unwrap();
    }

    #[test]
    fn detail_renders_without_panic() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let providers = sample_providers();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_provider_detail(frame, area, &providers[0]);
            })
            .unwrap();
    }

    #[test]
    fn credit_bar_renders_without_panic() {
        let backend = TestBackend::new(100, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let providers = sample_providers();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_credit_status_bar(frame, area, &providers);
            })
            .unwrap();
    }

    #[test]
    fn credit_bar_empty_renders_without_panic() {
        let backend = TestBackend::new(100, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_credit_status_bar(frame, area, &[]);
            })
            .unwrap();
    }

    #[test]
    fn nerv_panel_renders_without_panic() {
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = TuiState::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_nerv_panel(frame, area, &state, None);
            })
            .unwrap();
    }

    #[test]
    fn nerv_panel_with_selected_renders_without_panic() {
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TuiState::new();

        // Inject some efficiency events.
        let mut event = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        event.model = "claude-sonnet-4-20250514".into();
        event.backend = "anthropic-api".into();
        event.cost_usd = 0.05;
        event.wall_time_ms = 12_000;
        event.output_tokens = 500;
        state.efficiency_events.push(event);

        let mut event2 = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        event2.model = "gpt-4o".into();
        event2.backend = "openai".into();
        event2.cost_usd = 0.03;
        event2.wall_time_ms = 8_000;
        event2.output_tokens = 300;
        state.efficiency_events.push(event2);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_nerv_panel(frame, area, &state, Some(0));
            })
            .unwrap();
    }

    #[test]
    fn block_char_maps_full_range() {
        assert_eq!(block_char(0.0), '\u{2581}');
        assert_eq!(block_char(1.0), '\u{2588}');
        // Mid-range should be a middle block character.
        let mid = block_char(0.5);
        assert!(BLOCKS.contains(&mid));
    }

    #[test]
    fn build_block_sparkline_basic() {
        let data = vec![0.0, 0.5, 1.0, 0.75, 0.25];
        let result = build_block_sparkline(&data, 5, Theme::ROSE);
        assert_eq!(result.chars().count(), 5);
    }

    #[test]
    fn build_block_sparkline_scrolling() {
        let data: Vec<f64> = (0..20).map(|i| i as f64 / 20.0).collect();
        // Width smaller than data: should only show the last 5 samples.
        let result = build_block_sparkline(&data, 5, Theme::ROSE);
        assert_eq!(result.chars().count(), 5);
    }

    #[test]
    fn build_block_sparkline_empty() {
        let result = build_block_sparkline(&[], 10, Theme::ROSE);
        assert_eq!(result.len(), 10); // spaces
    }

    #[test]
    fn format_cost_compact_ranges() {
        assert_eq!(format_cost_compact(0.0), "-");
        assert!(format_cost_compact(0.05).starts_with('$'));
        assert!(format_cost_compact(5.0).starts_with('$'));
        assert!(format_cost_compact(150.0).starts_with('$'));
    }

    #[test]
    fn format_latency_compact_ranges() {
        assert_eq!(format_latency_compact(0.0), "-");
        assert!(format_latency_compact(500.0).contains("ms"));
        assert!(format_latency_compact(5_000.0).contains('s'));
        assert!(format_latency_compact(120_000.0).contains('m'));
    }

    #[test]
    fn truncate_str_works() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello w\u{2026}");
    }

    #[test]
    fn infer_provider_name_from_model() {
        assert_eq!(infer_provider_name("claude-sonnet-4", ""), "anthropic");
        assert_eq!(infer_provider_name("gpt-4o", ""), "openai");
        assert_eq!(infer_provider_name("gemini-pro", ""), "google");
        assert_eq!(infer_provider_name("unknown-model", "cerebras"), "cerebras");
    }

    #[test]
    fn infer_provider_name_prefers_backend() {
        assert_eq!(
            infer_provider_name("some-model", "anthropic-api"),
            "anthropic"
        );
        assert_eq!(infer_provider_name("some-model", "openai-compat"), "openai");
    }

    #[test]
    fn aggregate_from_empty_state() {
        let state = TuiState::new();
        let providers = aggregate_nerv_providers(&state);
        assert!(providers.is_empty());
    }

    #[test]
    fn aggregate_groups_by_provider() {
        let mut state = TuiState::new();

        let mut e1 = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        e1.model = "claude-sonnet-4-20250514".into();
        e1.backend = "anthropic-api".into();
        e1.cost_usd = 0.05;
        e1.wall_time_ms = 1000;
        e1.output_tokens = 100;
        state.efficiency_events.push(e1);

        let mut e2 = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        e2.model = "gpt-4o".into();
        e2.backend = "openai".into();
        e2.cost_usd = 0.03;
        e2.wall_time_ms = 800;
        e2.output_tokens = 200;
        state.efficiency_events.push(e2);

        let mut e3 = roko_learn::efficiency::AgentEfficiencyEvent::default_event();
        e3.model = "claude-haiku-4-5".into();
        e3.backend = "anthropic-api".into();
        e3.cost_usd = 0.01;
        e3.wall_time_ms = 500;
        e3.output_tokens = 50;
        state.efficiency_events.push(e3);

        let providers = aggregate_nerv_providers(&state);
        assert_eq!(providers.len(), 2); // anthropic + openai
        let anthropic = providers.iter().find(|p| p.name == "anthropic").unwrap();
        assert_eq!(anthropic.total_requests, 2);
        assert_eq!(anthropic.models.len(), 2);
    }

    #[test]
    fn provider_nerv_status_methods() {
        let p = ProviderNervStatus {
            name: "test".to_string(),
            total_requests: 100,
            successes: 90,
            total_cost_usd: 5.0,
            total_latency_ms: 50_000,
            cost_timeline: Vec::new(),
            latency_timeline: Vec::new(),
            error_timeline: Vec::new(),
            models: Vec::new(),
            circuit_state: CircuitLabel::Closed,
            is_cli: false,
        };
        assert!((p.success_rate() - 90.0).abs() < 0.01);
        assert!((p.avg_latency_ms() - 500.0).abs() < 0.01);
        assert!((p.error_rate() - 10.0).abs() < 0.01);
    }

    #[test]
    fn circuit_label_display() {
        assert_eq!(CircuitLabel::Closed.as_str(), "HEALTHY");
        assert_eq!(CircuitLabel::Open.as_str(), "DOWN");
        assert_eq!(CircuitLabel::HalfOpen.as_str(), "PROBING");
        assert_eq!(CircuitLabel::Unknown.as_str(), "N/A");
    }

    #[test]
    fn health_color_ranges() {
        let healthy = ProviderNervStatus {
            name: "t".into(),
            total_requests: 10,
            successes: 10,
            total_cost_usd: 0.0,
            total_latency_ms: 0,
            cost_timeline: Vec::new(),
            latency_timeline: Vec::new(),
            error_timeline: Vec::new(),
            models: Vec::new(),
            circuit_state: CircuitLabel::Closed,
            is_cli: false,
        };
        assert_eq!(healthy.health_color(), Theme::SAGE);

        let degraded = ProviderNervStatus {
            successes: 8,
            ..healthy.clone()
        };
        assert_eq!(degraded.health_color(), Theme::WARNING);

        let bad = ProviderNervStatus {
            successes: 3,
            ..healthy.clone()
        };
        assert_eq!(bad.health_color(), Theme::EMBER);

        let no_data = ProviderNervStatus {
            total_requests: 0,
            successes: 0,
            ..healthy
        };
        assert_eq!(no_data.health_color(), Theme::TEXT_GHOST);
    }

    #[test]
    fn dot_symbols() {
        let cli = ProviderNervStatus {
            name: "t".into(),
            total_requests: 5,
            successes: 5,
            total_cost_usd: 0.0,
            total_latency_ms: 0,
            cost_timeline: Vec::new(),
            latency_timeline: Vec::new(),
            error_timeline: Vec::new(),
            models: Vec::new(),
            circuit_state: CircuitLabel::Closed,
            is_cli: true,
        };
        assert_eq!(cli.dot(), "\u{25c6}"); // diamond

        let healthy = ProviderNervStatus {
            is_cli: false,
            ..cli.clone()
        };
        assert_eq!(healthy.dot(), "\u{25cf}"); // filled circle

        let no_data = ProviderNervStatus {
            total_requests: 0,
            successes: 0,
            is_cli: false,
            ..cli
        };
        assert_eq!(no_data.dot(), "\u{25cb}"); // empty circle
    }
}
