//! Conductor supervision panel widget.
//!
//! Displays the conductor's reactive intelligence state:
//! - Watcher health status (12 watchers, each with name + status)
//! - Recent interventions (conductor alerts and diagnoses)
//! - Circuit breaker state per plan
//! - Conductor config thresholds (silence timeout, compile fail, etc.)
//!
//! This is a read-only panel; it does not accept input.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::Theme;
use crate::tui::dashboard::AlertSummary;
use crate::tui::empty_state;

use roko_core::dashboard_snapshot::{DiagnosisSeverity, DiagnosisSummary};

// ---------------------------------------------------------------------------
// Conductor snapshot: data struct populated by TuiState refresh
// ---------------------------------------------------------------------------

/// Display-ready snapshot of conductor state for the TUI.
#[derive(Debug, Clone, Default)]
pub struct ConductorSnapshot {
    /// Watcher status rows (name, healthy, last-fire description).
    pub watchers: Vec<WatcherStatus>,
    /// Circuit breaker summary per plan.
    pub circuit_breakers: Vec<CircuitBreakerRow>,
    /// Conductor config thresholds for display.
    pub thresholds: ConductorThresholds,
}

/// Status row for a single conductor watcher.
#[derive(Debug, Clone)]
pub struct WatcherStatus {
    /// Watcher name (e.g. "ghost-turn", "compile-fail-repeat").
    pub name: String,
    /// Whether the watcher has fired recently (true = fired / unhealthy).
    pub fired: bool,
    /// Severity of the most recent firing, if any.
    pub severity: Option<String>,
    /// Description of the most recent finding.
    pub last_message: Option<String>,
}

/// Circuit breaker summary for one plan.
#[derive(Debug, Clone)]
pub struct CircuitBreakerRow {
    /// Plan identifier.
    pub plan_id: String,
    /// Number of recorded failures.
    pub failure_count: u32,
    /// Maximum failures before trip.
    pub max_failures: u32,
    /// Whether the breaker is currently tripped.
    pub tripped: bool,
    /// Most recent failure reason.
    pub last_reason: Option<String>,
}

/// Conductor config thresholds for display.
#[derive(Debug, Clone)]
pub struct ConductorThresholds {
    pub silence_timeout_secs: u64,
    pub compile_fail_threshold: u32,
    pub task_stall_secs: u64,
    pub context_pressure_pct: u8,
    pub phase_timeout_secs: u64,
    pub max_agents: usize,
    pub max_auto_fix_attempts: u32,
}

impl Default for ConductorThresholds {
    fn default() -> Self {
        Self {
            silence_timeout_secs: 180,
            compile_fail_threshold: 3,
            task_stall_secs: 300,
            context_pressure_pct: 80,
            phase_timeout_secs: 1800,
            max_agents: 8,
            max_auto_fix_attempts: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// All 12 watcher names (must match conductor.rs default_watchers order)
// ---------------------------------------------------------------------------

/// Canonical watcher names in the order they are registered by the conductor.
pub const WATCHER_NAMES: &[&str] = &[
    "ghost-turn",
    "review-loop",
    "iteration-loop",
    "test-failure-budget",
    "compile-fail-repeat",
    "context-window-pressure",
    "spec-drift",
    "cost-overrun",
    "time-overrun",
    "stuck-pattern",
    "worktree-count",
    "disk-pressure",
];

// ---------------------------------------------------------------------------
// Build a ConductorSnapshot from TuiState data
// ---------------------------------------------------------------------------

/// Build a conductor snapshot from the alerts and diagnoses already in TuiState.
///
/// This populates watcher statuses from conductor alert signals (which carry
/// the watcher name in the `kind` field as `conductor:alert:<watcher>`), and
/// fills circuit breaker / threshold info from the config.
pub fn build_conductor_snapshot(
    alerts: &[AlertSummary],
    diagnoses: &[DiagnosisSummary],
    config: &roko_core::config::schema::ConductorConfig,
) -> ConductorSnapshot {
    use std::collections::HashMap;

    // Index alerts by watcher name.
    let mut watcher_alerts: HashMap<String, &AlertSummary> = HashMap::new();
    for alert in alerts {
        // Kind format: "conductor:alert:<watcher-name>"
        if let Some(watcher_name) = alert.kind.strip_prefix("conductor:alert:") {
            watcher_alerts
                .entry(watcher_name.to_string())
                .or_insert(alert);
        }
    }

    // Build watcher status rows.
    let watchers = WATCHER_NAMES
        .iter()
        .map(|&name| {
            if let Some(alert) = watcher_alerts.get(name) {
                WatcherStatus {
                    name: name.to_string(),
                    fired: true,
                    severity: Some(alert.severity.clone()),
                    last_message: if alert.message.is_empty() {
                        None
                    } else {
                        Some(alert.message.clone())
                    },
                }
            } else {
                WatcherStatus {
                    name: name.to_string(),
                    fired: false,
                    severity: None,
                    last_message: None,
                }
            }
        })
        .collect();

    // Build circuit breaker rows from diagnoses that mention circuit breaker.
    let circuit_breakers = diagnoses
        .iter()
        .filter(|d| {
            d.subject.contains("Circuit Breaker")
                || d.subject.contains("circuit breaker")
                || d.detail.contains("circuit breaker")
        })
        .map(|d| CircuitBreakerRow {
            plan_id: d.id.clone(),
            failure_count: 0, // not available from diagnosis summary
            max_failures: 2,
            tripped: d.severity == DiagnosisSeverity::Alert,
            last_reason: Some(d.detail.clone()),
        })
        .collect();

    let thresholds = ConductorThresholds {
        silence_timeout_secs: config.silence_timeout_secs,
        compile_fail_threshold: config.compile_fail_threshold,
        task_stall_secs: config.task_stall_secs,
        context_pressure_pct: config.context_pressure_pct,
        phase_timeout_secs: config.phase_timeout_secs,
        max_agents: config.max_agents,
        max_auto_fix_attempts: config.max_auto_fix_attempts,
    };

    ConductorSnapshot {
        watchers,
        circuit_breakers,
        thresholds,
    }
}

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the full conductor supervision panel.
///
/// Layout: three vertical sections:
/// 1. Watcher grid (top ~50%) -- 12 watchers with status icons
/// 2. Recent interventions (middle ~25%) -- diagnoses + alerts
/// 3. Config thresholds (bottom ~25%) -- key thresholds
pub fn render_conductor_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &ConductorSnapshot,
    diagnoses: &[DiagnosisSummary],
    alerts: &[AlertSummary],
    focused: bool,
    theme: &Theme,
) {
    let border = if focused {
        Theme::focused_border_style()
    } else {
        theme.muted()
    };
    let title_style = if focused {
        Theme::focused_title_style()
    } else {
        theme.muted()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Conductor ", title_style))
        .border_style(border)
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 20 || inner.height < 6 {
        frame.render_widget(
            Paragraph::new(Span::styled("  too small", theme.muted())),
            inner,
        );
        return;
    }

    // Adaptive layout based on available height.
    let has_interventions = !diagnoses.is_empty() || !alerts.is_empty();
    let has_breakers = !snapshot.circuit_breakers.is_empty();

    let sections = if has_interventions || has_breakers {
        Layout::vertical([
            Constraint::Percentage(45), // watchers
            Constraint::Percentage(30), // interventions
            Constraint::Percentage(25), // thresholds + breakers
        ])
        .split(inner)
    } else {
        Layout::vertical([
            Constraint::Percentage(60), // watchers
            Constraint::Length(0),       // no interventions
            Constraint::Percentage(40), // thresholds
        ])
        .split(inner)
    };

    render_watcher_grid(frame, sections[0], &snapshot.watchers, theme);
    if has_interventions || has_breakers {
        render_interventions(
            frame,
            sections[1],
            diagnoses,
            alerts,
            &snapshot.circuit_breakers,
            theme,
        );
    }
    render_thresholds(frame, sections[2], &snapshot.thresholds, theme);
}

// ---------------------------------------------------------------------------
// Section: Watcher grid
// ---------------------------------------------------------------------------

fn render_watcher_grid(
    frame: &mut Frame<'_>,
    area: Rect,
    watchers: &[WatcherStatus],
    theme: &Theme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let total = watchers.len();
    let healthy_count = watchers.iter().filter(|w| !w.fired).count();
    let fired_count = total - healthy_count;

    let title = format!(
        " Watchers ({healthy_count}/{total} healthy) "
    );
    let title_color = if fired_count > 0 {
        theme.warning
    } else {
        theme.success
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, Style::default().fg(title_color)))
        .border_style(Style::default().fg(Theme::TEXT_GHOST))
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let compact = inner.width < 50;
    let mut lines: Vec<Line<'_>> = Vec::new();

    for watcher in watchers.iter().take(inner.height as usize) {
        let (icon, icon_style) = if watcher.fired {
            let sev_color = match watcher.severity.as_deref() {
                Some("critical") => theme.danger,
                Some("warning") => theme.warning,
                _ => theme.info,
            };
            (
                "\u{25cf} ", // filled circle
                Style::default()
                    .fg(sev_color)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                "\u{25cb} ", // empty circle
                Style::default().fg(theme.success),
            )
        };

        let name_style = if watcher.fired {
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_DIM)
        };

        let name_w = if compact { 18 } else { 24 };
        let name_display = if watcher.name.len() > name_w {
            format!("{:.width$}", watcher.name, width = name_w)
        } else {
            format!("{:<width$}", watcher.name, width = name_w)
        };

        let mut spans = vec![
            Span::styled(" ", Style::default()),
            Span::styled(icon, icon_style),
            Span::styled(name_display, name_style),
        ];

        // Show severity label for fired watchers.
        if watcher.fired {
            let sev_label = watcher.severity.as_deref().unwrap_or("info");
            let sev_color = match sev_label {
                "critical" => theme.danger,
                "warning" => theme.warning,
                _ => theme.info,
            };
            spans.push(Span::styled(
                format!(" {sev_label:<8}"),
                Style::default().fg(sev_color),
            ));

            if !compact {
                if let Some(ref msg) = watcher.last_message {
                    let remaining = (inner.width as usize)
                        .saturating_sub(name_w + 14);
                    let truncated = if msg.len() > remaining {
                        format!("{:.width$}..", msg, width = remaining.saturating_sub(2))
                    } else {
                        msg.clone()
                    };
                    spans.push(Span::styled(
                        format!(" {truncated}"),
                        Style::default().fg(Theme::TEXT_DIM),
                    ));
                }
            }
        } else {
            spans.push(Span::styled(
                " ok",
                Style::default().fg(Theme::TEXT_GHOST),
            ));
        }

        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        empty_state::render_pane_empty_compact(
            frame,
            inner,
            "No watcher data",
            theme,
        );
        return;
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ---------------------------------------------------------------------------
// Section: Recent interventions
// ---------------------------------------------------------------------------

fn render_interventions(
    frame: &mut Frame<'_>,
    area: Rect,
    diagnoses: &[DiagnosisSummary],
    alerts: &[AlertSummary],
    circuit_breakers: &[CircuitBreakerRow],
    theme: &Theme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let total_items = diagnoses.len() + alerts.len() + circuit_breakers.len();
    let title = format!(" Interventions ({total_items}) ");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            if total_items > 0 {
                Style::default().fg(theme.warning)
            } else {
                theme.muted()
            },
        ))
        .border_style(Style::default().fg(Theme::TEXT_GHOST))
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line<'_>> = Vec::new();
    let max_lines = inner.height as usize;

    // Circuit breaker rows first (most critical).
    for cb in circuit_breakers.iter().take(max_lines) {
        let status_icon = if cb.tripped {
            Span::styled(
                "\u{26a0} ",
                Style::default()
                    .fg(theme.danger)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                "\u{25b6} ",
                Style::default().fg(theme.warning),
            )
        };
        let plan_label = if cb.plan_id.len() > 20 {
            format!("{:.20}", cb.plan_id)
        } else {
            cb.plan_id.clone()
        };
        let status_text = if cb.tripped {
            "TRIPPED".to_string()
        } else {
            format!("{}/{} failures", cb.failure_count, cb.max_failures)
        };
        lines.push(Line::from(vec![
            Span::styled(" ", Style::default()),
            status_icon,
            Span::styled(
                format!("CB {plan_label}"),
                Style::default()
                    .fg(if cb.tripped {
                        theme.danger
                    } else {
                        theme.warning
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {status_text}"),
                Style::default().fg(Theme::TEXT_DIM),
            ),
        ]));
    }

    // Diagnosis rows.
    let remaining = max_lines.saturating_sub(lines.len());
    for diag in diagnoses.iter().take(remaining) {
        let (sev_icon, sev_color) = match diag.severity {
            DiagnosisSeverity::Alert => ("\u{25cf} ", theme.danger),
            DiagnosisSeverity::Warn => ("\u{25cf} ", theme.warning),
            DiagnosisSeverity::Info => ("\u{25cb} ", Theme::TEXT_DIM),
        };

        let subject_w = (inner.width as usize).saturating_sub(12).min(40);
        let subject = if diag.subject.len() > subject_w {
            format!("{:.width$}..", diag.subject, width = subject_w.saturating_sub(2))
        } else {
            diag.subject.clone()
        };

        let mut spans = vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                sev_icon,
                Style::default()
                    .fg(sev_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(subject, Style::default().fg(theme.foreground)),
        ];

        // Show intervention taken if available.
        if let Some(ref action) = diag.intervention_taken {
            let action_w = (inner.width as usize).saturating_sub(subject_w + 14);
            let truncated = if action.len() > action_w {
                format!("{:.width$}..", action, width = action_w.saturating_sub(2))
            } else {
                action.clone()
            };
            spans.push(Span::styled(
                format!(" -> {truncated}"),
                Style::default().fg(Theme::SAGE),
            ));
        }

        lines.push(Line::from(spans));
    }

    // Alert rows (from conductor signals).
    let remaining = max_lines.saturating_sub(lines.len());
    for alert in alerts.iter().take(remaining) {
        let sev_color = match alert.severity.as_str() {
            "critical" => theme.danger,
            "warning" => theme.warning,
            _ => theme.info,
        };

        let msg_w = (inner.width as usize).saturating_sub(4);
        let msg = if alert.message.len() > msg_w {
            format!("{:.width$}..", alert.message, width = msg_w.saturating_sub(2))
        } else {
            alert.message.clone()
        };

        lines.push(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                "\u{25aa} ",
                Style::default().fg(sev_color),
            ),
            Span::styled(msg, Style::default().fg(Theme::TEXT_DIM)),
        ]));
    }

    if lines.is_empty() {
        empty_state::render_pane_empty_compact(
            frame,
            inner,
            "No interventions",
            theme,
        );
        return;
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ---------------------------------------------------------------------------
// Section: Config thresholds
// ---------------------------------------------------------------------------

fn render_thresholds(
    frame: &mut Frame<'_>,
    area: Rect,
    thresholds: &ConductorThresholds,
    theme: &Theme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Thresholds ",
            Style::default().fg(Theme::TEXT_DIM),
        ))
        .border_style(Style::default().fg(Theme::TEXT_GHOST))
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let compact = inner.width < 50;
    let label_style = theme.label();
    let value_style = theme.value();

    let items: Vec<(&str, String)> = if compact {
        vec![
            ("silence", format!("{}s", thresholds.silence_timeout_secs)),
            ("compile", format!("{}x", thresholds.compile_fail_threshold)),
            ("stall", format!("{}s", thresholds.task_stall_secs)),
            ("ctx%", format!("{}%", thresholds.context_pressure_pct)),
            ("phase", format!("{}s", thresholds.phase_timeout_secs)),
            ("agents", format!("{}", thresholds.max_agents)),
        ]
    } else {
        vec![
            (
                "silence timeout",
                format_duration_secs(thresholds.silence_timeout_secs),
            ),
            (
                "compile fail max",
                format!("{}", thresholds.compile_fail_threshold),
            ),
            (
                "task stall timeout",
                format_duration_secs(thresholds.task_stall_secs),
            ),
            (
                "context pressure",
                format!("{}%", thresholds.context_pressure_pct),
            ),
            (
                "phase timeout",
                format_duration_secs(thresholds.phase_timeout_secs),
            ),
            ("max agents", format!("{}", thresholds.max_agents)),
            (
                "max auto-fix",
                format!("{}", thresholds.max_auto_fix_attempts),
            ),
        ]
    };

    // Render as two-column key-value grid if wide enough, single column otherwise.
    let mut lines: Vec<Line<'_>> = Vec::new();

    if !compact && inner.width >= 60 {
        // Two-column layout.
        let mut i = 0;
        while i < items.len() && lines.len() < inner.height as usize {
            let left = &items[i];
            let left_spans = vec![
                Span::styled(format!(" {:<18}", left.0), label_style),
                Span::styled(format!("{:<10}", left.1), value_style),
            ];

            if i + 1 < items.len() {
                let right = &items[i + 1];
                let mut spans = left_spans;
                spans.push(Span::styled(
                    " \u{2502} ",
                    Style::default().fg(Theme::TEXT_GHOST),
                ));
                spans.push(Span::styled(format!("{:<18}", right.0), label_style));
                spans.push(Span::styled(format!("{}", right.1), value_style));
                lines.push(Line::from(spans));
                i += 2;
            } else {
                lines.push(Line::from(left_spans));
                i += 1;
            }
        }
    } else {
        // Single-column layout.
        for (label, value) in items.iter().take(inner.height as usize) {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:<18}", label), label_style),
                Span::styled(value.to_string(), value_style),
            ]));
        }
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        inner,
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format seconds into a human-readable duration (e.g. "3m00s", "30m", "1h30m").
fn format_duration_secs(secs: u64) -> String {
    if secs == 0 {
        return "0s".to_string();
    }
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if hours > 0 {
        if mins > 0 {
            format!("{hours}h{mins:02}m")
        } else {
            format!("{hours}h")
        }
    } else if mins > 0 {
        if s > 0 {
            format!("{mins}m{s:02}s")
        } else {
            format!("{mins}m")
        }
    } else {
        format!("{s}s")
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

    fn default_snapshot() -> ConductorSnapshot {
        ConductorSnapshot {
            watchers: WATCHER_NAMES
                .iter()
                .map(|&name| WatcherStatus {
                    name: name.to_string(),
                    fired: false,
                    severity: None,
                    last_message: None,
                })
                .collect(),
            circuit_breakers: Vec::new(),
            thresholds: ConductorThresholds::default(),
        }
    }

    fn snapshot_with_fired() -> ConductorSnapshot {
        let mut snap = default_snapshot();
        snap.watchers[0] = WatcherStatus {
            name: "ghost-turn".to_string(),
            fired: true,
            severity: Some("warning".to_string()),
            last_message: Some("3 consecutive ghost turns detected".to_string()),
        };
        snap.watchers[4] = WatcherStatus {
            name: "compile-fail-repeat".to_string(),
            fired: true,
            severity: Some("critical".to_string()),
            last_message: Some("5 consecutive compile failures".to_string()),
        };
        snap
    }

    #[test]
    fn conductor_panel_renders_without_panic() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let snapshot = default_snapshot();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_conductor_panel(
                    frame, area, &snapshot, &[], &[], false, &theme,
                );
            })
            .unwrap();
    }

    #[test]
    fn conductor_panel_with_fired_watchers() {
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let snapshot = snapshot_with_fired();

        let diagnoses = vec![DiagnosisSummary {
            id: "diag-1".into(),
            severity: DiagnosisSeverity::Warn,
            subject: "Circuit Breaker: Loop Detected".into(),
            detail: "Plan plan-1 has 2 consecutive failures".into(),
            intervention_taken: Some("Restarted agent".into()),
            ..Default::default()
        }];

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_conductor_panel(
                    frame,
                    area,
                    &snapshot,
                    &diagnoses,
                    &[],
                    true,
                    &theme,
                );
            })
            .unwrap();
    }

    #[test]
    fn conductor_panel_compact_renders() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let snapshot = default_snapshot();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_conductor_panel(
                    frame, area, &snapshot, &[], &[], false, &theme,
                );
            })
            .unwrap();
    }

    #[test]
    fn conductor_panel_tiny_area() {
        let backend = TestBackend::new(15, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let snapshot = default_snapshot();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_conductor_panel(
                    frame, area, &snapshot, &[], &[], false, &theme,
                );
            })
            .unwrap();
    }

    #[test]
    fn format_duration_values() {
        assert_eq!(format_duration_secs(0), "0s");
        assert_eq!(format_duration_secs(45), "45s");
        assert_eq!(format_duration_secs(180), "3m");
        assert_eq!(format_duration_secs(300), "5m");
        assert_eq!(format_duration_secs(1800), "30m");
        assert_eq!(format_duration_secs(3600), "1h");
        assert_eq!(format_duration_secs(3661), "1h01m");
        assert_eq!(format_duration_secs(5400), "1h30m");
    }

    #[test]
    fn build_snapshot_from_alerts() {
        let alerts = vec![AlertSummary {
            id: "a1".into(),
            kind: "conductor:alert:ghost-turn".into(),
            created_at_ms: 1000,
            severity: "warning".into(),
            message: "ghost turn detected".into(),
        }];
        let config = roko_core::config::schema::ConductorConfig::default();
        let snap = build_conductor_snapshot(&alerts, &[], &config);

        assert_eq!(snap.watchers.len(), WATCHER_NAMES.len());
        assert!(snap.watchers[0].fired);
        assert_eq!(snap.watchers[0].name, "ghost-turn");
        assert!(!snap.watchers[1].fired);
    }

    #[test]
    fn watcher_count_matches_conductor() {
        assert_eq!(WATCHER_NAMES.len(), 12);
    }
}
