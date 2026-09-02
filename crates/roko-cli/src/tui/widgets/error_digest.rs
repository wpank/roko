//! Error digest widget showing gate failures and recent errors.
//!
//! Provides two rendering modes:
//! - `render_error_digest`: compact inline panel for embedding in other views
//! - `render_error_aggregation_panel`: full scrollable panel for the F5:Errors tab
//!   with categorized errors grouped by source, severity icons, cross-source
//!   dedup, remediation hints, and scroll support

use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use roko_core::dashboard_snapshot::{DiagnosisSeverity, ErrorEntry, GateVerdictView, SnapshotStats};

use super::super::dashboard::Theme;
use crate::tui::state::TuiState;

// ---------------------------------------------------------------------------
// Error categories
// ---------------------------------------------------------------------------

/// Error source category for aggregation and grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ErrorCategory {
    Gate,
    Compile,
    Agent,
    Preflight,
    Conductor,
    Runtime,
}

impl ErrorCategory {
    fn label(self) -> &'static str {
        match self {
            Self::Gate => "gate",
            Self::Compile => "compile",
            Self::Agent => "agent",
            Self::Preflight => "preflight",
            Self::Conductor => "conductor",
            Self::Runtime => "runtime",
        }
    }

    fn heading(self) -> &'static str {
        match self {
            Self::Gate => "Gate Failures",
            Self::Compile => "Compile Errors",
            Self::Agent => "Agent Failures",
            Self::Preflight => "Preflight Checks",
            Self::Conductor => "Conductor Diagnoses",
            Self::Runtime => "Runtime Errors",
        }
    }

    fn from_message(msg: &str) -> Self {
        let lower = msg.to_ascii_lowercase();
        if lower.contains("gate") || lower.contains("verify") || lower.contains("rung") {
            Self::Gate
        } else if lower.contains("compil") || lower.contains("cargo") || lower.contains("rustc") {
            Self::Compile
        } else if lower.contains("agent") || lower.contains("stall") || lower.contains("timeout") {
            Self::Agent
        } else if lower.contains("preflight") || lower.contains("pre-flight") {
            Self::Preflight
        } else if lower.contains("conductor") || lower.contains("diagnos") {
            Self::Conductor
        } else {
            Self::Runtime
        }
    }
}

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// Severity level for rendering icons and colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    fn icon(self) -> &'static str {
        match self {
            Self::Error => "\u{2718}",   // ✘
            Self::Warning => "\u{25b2}", // ▲
            Self::Info => "\u{25cf}",    // ●
        }
    }

    fn color(self, theme: &Theme) -> Color {
        match self {
            Self::Error => theme.danger,
            Self::Warning => theme.warning,
            Self::Info => theme.info,
        }
    }

    fn from_category(cat: ErrorCategory) -> Self {
        match cat {
            ErrorCategory::Gate | ErrorCategory::Compile | ErrorCategory::Runtime => Self::Error,
            ErrorCategory::Agent | ErrorCategory::Preflight => Self::Warning,
            ErrorCategory::Conductor => Self::Info,
        }
    }
}

/// A categorized, timestamped error for the aggregation panel.
#[derive(Debug, Clone)]
struct CategorizedError {
    category: ErrorCategory,
    severity: Severity,
    message: String,
    ts_millis: u64,
    /// Optional source context (e.g. plan/task id).
    source: String,
    /// Suggested remediation action, if inferrable.
    remediation: Option<&'static str>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format a millisecond timestamp as a short time string.
fn fmt_ts(ts_millis: u64) -> String {
    let secs = ts_millis / 1000;
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Normalize a message for cross-source dedup comparison.
fn normalize_for_dedup(msg: &str) -> String {
    let lower = msg.to_ascii_lowercase();
    lower
        .trim_start_matches("gate failed for ")
        .trim_start_matches("agent ")
        .trim_start_matches("compile gate failed: ")
        .trim_start_matches("test gate failed: ")
        .trim()
        .to_string()
}

/// Infer a remediation hint from the error message.
fn infer_remediation(msg: &str, category: ErrorCategory) -> Option<&'static str> {
    let lower = msg.to_ascii_lowercase();
    match category {
        ErrorCategory::Compile => {
            if lower.contains("semicolon") || lower.contains("syntax") {
                Some("fix syntax error in source file")
            } else if lower.contains("missing") || lower.contains("unresolved") {
                Some("add missing import or dependency")
            } else if lower.contains("type") || lower.contains("mismatch") {
                Some("fix type mismatch in source")
            } else {
                Some("fix compile error and retry")
            }
        }
        ErrorCategory::Gate => {
            if lower.contains("test") {
                Some("fix failing tests")
            } else if lower.contains("clippy") || lower.contains("lint") {
                Some("resolve clippy/lint warnings")
            } else if lower.contains("format") || lower.contains("fmt") {
                Some("run cargo fmt")
            } else if lower.contains("compile") || lower.contains("build") {
                Some("fix compile error and retry")
            } else {
                Some("investigate gate failure output")
            }
        }
        ErrorCategory::Agent => {
            if lower.contains("timeout") || lower.contains("stall") {
                Some("check agent responsiveness or increase timeout")
            } else {
                Some("check agent logs for details")
            }
        }
        ErrorCategory::Preflight => Some("fix preflight conditions before retrying"),
        ErrorCategory::Conductor => None,
        ErrorCategory::Runtime => {
            if lower.contains("disk") || lower.contains("space") {
                Some("free disk space or increase quota")
            } else if lower.contains("memory") || lower.contains("oom") {
                Some("reduce memory usage or increase limits")
            } else {
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public render entry-point (compact, existing API preserved)
// ---------------------------------------------------------------------------

/// Render the error digest panel (compact inline version).
///
/// Top half: gate pass/fail summary.
/// Bottom half: recent errors list.
pub fn render_error_digest(
    frame: &mut Frame<'_>,
    area: Rect,
    gates: &[GateVerdictView],
    errors: &[ErrorEntry],
    stats: &SnapshotStats,
    theme: &Theme,
) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::unfocused_border_style())
        .title(Span::styled("Errors & Gates", theme.accent()))
        .style(Theme::block_style());
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if inner.height < 3 {
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    // --- Verify summary ---
    render_gate_summary(frame, sections[0], gates, stats, theme);

    // --- Error list ---
    render_error_list(frame, sections[1], errors, theme);
}

// ---------------------------------------------------------------------------
// Full aggregation panel (for F5:Errors tab)
// ---------------------------------------------------------------------------

/// Render the full error aggregation panel.
///
/// Collects errors from all sources (gate failures, efficiency gate errors,
/// snapshot errors, conductor diagnoses, conductor alerts, gate recent failures)
/// into categorized sections with cross-source dedup, severity icons, and
/// remediation hints.
/// Panel border turns red when errors are active.
pub fn render_error_aggregation_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    tui_state: &TuiState,
    scroll: u16,
    theme: &Theme,
) {
    let all_errors = collect_all_errors(tui_state);
    let has_active_errors = !all_errors.is_empty();

    let error_count = all_errors.len();
    let title = if error_count == 0 {
        " Error Digest ".to_string()
    } else {
        format!(" Error Digest ({error_count} errors) ")
    };

    let border_style = if has_active_errors {
        theme.danger()
    } else {
        Theme::unfocused_border_style()
    };
    let title_style = if has_active_errors {
        Style::default()
            .fg(theme.danger)
            .add_modifier(Modifier::BOLD)
    } else {
        Theme::title_style()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, title_style))
        .border_style(border_style)
        .style(Theme::block_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height < 3 {
        return;
    }

    if all_errors.is_empty() {
        let v_pad = inner.height / 2;
        let mut empty_lines: Vec<Line<'_>> = Vec::new();
        for _ in 0..v_pad.saturating_sub(1) {
            empty_lines.push(Line::from(""));
        }
        empty_lines.push(Line::from(Span::styled(
            "no errors recorded",
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::ITALIC),
        )));
        empty_lines.push(Line::from(""));
        empty_lines.push(Line::from(Span::styled(
            "errors from gates, agents, and runtime",
            Style::default().fg(theme.muted),
        )));
        empty_lines.push(Line::from(Span::styled(
            "will appear here when they occur",
            Style::default().fg(theme.muted),
        )));
        let empty = Paragraph::new(empty_lines)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    // Layout: top 3 lines = category summary, rest = scrollable grouped list.
    let sections = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(inner);

    // Category summary counts with severity icons.
    render_category_summary(frame, sections[0], &all_errors, theme);

    // Scrollable error list grouped by source category.
    render_grouped_error_list(frame, sections[1], &all_errors, scroll, theme);
}

/// Render category summary bar showing counts per error type with severity icons.
fn render_category_summary(
    frame: &mut Frame<'_>,
    area: Rect,
    errors: &[CategorizedError],
    theme: &Theme,
) {
    let mut counts: BTreeMap<ErrorCategory, usize> = BTreeMap::new();
    for err in errors {
        *counts.entry(err.category).or_default() += 1;
    }

    let mut spans: Vec<Span<'_>> = vec![Span::styled("  ", Style::default())];
    for (cat, count) in &counts {
        let sev = Severity::from_category(*cat);
        let sev_color = sev.color(theme);
        spans.push(Span::styled(
            format!("{} ", sev.icon()),
            Style::default().fg(sev_color),
        ));
        spans.push(Span::styled(
            format!("{}: {count}  ", cat.label()),
            Style::default().fg(sev_color),
        ));
    }

    let line1 = Line::from(spans);

    // Total + newest timestamp.
    let newest_ts = errors.iter().map(|e| e.ts_millis).max().unwrap_or(0);
    let line2 = Line::from(vec![
        Span::styled(
            format!("  total: {}  ", errors.len()),
            Style::default()
                .fg(theme.danger)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("latest: {}", fmt_ts(newest_ts)), theme.muted()),
    ]);

    let sep = "\u{2500}".repeat(area.width.saturating_sub(2) as usize);
    let line3 = Line::from(Span::styled(
        format!(" {sep}"),
        Style::default().fg(theme.muted),
    ));

    frame.render_widget(Paragraph::new(vec![line1, line2, line3]), area);
}

/// Render the scrollable error list grouped by source category.
fn render_grouped_error_list(
    frame: &mut Frame<'_>,
    area: Rect,
    errors: &[CategorizedError],
    scroll: u16,
    theme: &Theme,
) {
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Render in fixed category order so groups are stable.
    let categories = [
        ErrorCategory::Gate,
        ErrorCategory::Compile,
        ErrorCategory::Agent,
        ErrorCategory::Preflight,
        ErrorCategory::Conductor,
        ErrorCategory::Runtime,
    ];

    for cat in &categories {
        let mut cat_errors: Vec<&CategorizedError> =
            errors.iter().filter(|e| e.category == *cat).collect();
        if cat_errors.is_empty() {
            continue;
        }
        // Most recent first within each group.
        cat_errors.sort_by(|a, b| b.ts_millis.cmp(&a.ts_millis));

        let sev = Severity::from_category(*cat);
        let heading_color = sev.color(theme);

        // Section heading.
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", sev.icon()),
                Style::default().fg(heading_color),
            ),
            Span::styled(
                format!("{} ({})", cat.heading(), cat_errors.len()),
                Style::default()
                    .fg(heading_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for err in &cat_errors {
            let ts = fmt_ts(err.ts_millis);
            let sev_color = err.severity.color(theme);
            let max_msg_len = (area.width as usize).saturating_sub(28);
            let msg = if err.message.chars().count() > max_msg_len && max_msg_len > 1 {
                let truncated: String =
                    err.message.chars().take(max_msg_len.saturating_sub(1)).collect();
                format!("{truncated}\u{2026}")
            } else {
                err.message.clone()
            };

            let mut spans = vec![
                Span::styled(format!("   [{ts}] "), theme.muted()),
                Span::styled(
                    format!("{} ", err.severity.icon()),
                    Style::default().fg(sev_color),
                ),
            ];

            if !err.source.is_empty() {
                spans.push(Span::styled(
                    format!("{}: ", err.source),
                    Style::default().fg(theme.info),
                ));
            }

            spans.push(Span::styled(msg, Style::default().fg(sev_color)));
            lines.push(Line::from(spans));

            // Remediation hint line.
            if let Some(hint) = err.remediation {
                lines.push(Line::from(vec![
                    Span::styled("     \u{2192} ", Style::default().fg(theme.muted)),
                    Span::styled(
                        hint,
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }
        }

        // Blank separator between groups.
        lines.push(Line::from(""));
    }

    let para = Paragraph::new(lines).scroll((scroll, 0));
    frame.render_widget(para, area);
}

/// Collect errors from all TUI sources into a unified list with cross-source dedup.
fn collect_all_errors(tui_state: &TuiState) -> Vec<CategorizedError> {
    let mut errors = Vec::new();

    // 1. Gate recent failures (from runner state).
    for failure in &tui_state.gate_recent_failures {
        let category = ErrorCategory::from_message(&failure.summary);
        let severity = Severity::from_category(category);
        let remediation = infer_remediation(&failure.summary, category);
        errors.push(CategorizedError {
            category,
            severity,
            message: failure.summary.clone(),
            ts_millis: failure.ts.timestamp_millis() as u64,
            source: format!("{}/{}", failure.plan_id, failure.task_id),
            remediation,
        });
    }

    // 2. Efficiency events with gate failures.
    for event in &tui_state.efficiency_events {
        if event.gate_passed == Some(false) {
            let ts = chrono::DateTime::parse_from_rfc3339(&event.timestamp)
                .map(|dt| dt.timestamp_millis() as u64)
                .unwrap_or(0);
            for gate_error in &event.gate_errors {
                let category = ErrorCategory::from_message(gate_error);
                let severity = Severity::from_category(category);
                let remediation = infer_remediation(gate_error, category);
                errors.push(CategorizedError {
                    category,
                    severity,
                    message: gate_error.clone(),
                    ts_millis: ts,
                    source: format!("{}/{}", event.plan_id, event.task_id),
                    remediation,
                });
            }
            // If no specific gate errors, create a generic one.
            if event.gate_errors.is_empty() {
                errors.push(CategorizedError {
                    category: ErrorCategory::Gate,
                    severity: Severity::Error,
                    message: format!("gate failed for {} (model: {})", event.task_id, event.model),
                    ts_millis: ts,
                    source: format!("{}/{}", event.plan_id, event.task_id),
                    remediation: Some("investigate gate failure output"),
                });
            }
        }
    }

    // 3. Gate results page failure rows.
    for failure_row in &tui_state.gate_results_page.failure_rows {
        let category = ErrorCategory::from_message(&failure_row.error_excerpt);
        let severity = Severity::from_category(category);
        let remediation = infer_remediation(&failure_row.error_excerpt, category);
        errors.push(CategorizedError {
            category,
            severity,
            message: failure_row.error_excerpt.clone(),
            ts_millis: failure_row.created_at_ms.max(0) as u64,
            source: failure_row.task_id.clone(),
            remediation,
        });
    }

    // 4. Failed agents.
    for agent in &tui_state.agent_summaries {
        let status = crate::tui::state::AgentStatus::from(agent.status.as_str());
        if status.is_failed() {
            let ts = tui_state
                .agents
                .iter()
                .find(|r| r.id == agent.id)
                .map(|r| r.last_event_at_ms)
                .unwrap_or(0);
            errors.push(CategorizedError {
                category: ErrorCategory::Agent,
                severity: Severity::Warning,
                message: format!("agent {} failed", agent.id),
                ts_millis: ts,
                source: agent.plan_id.clone().unwrap_or_default(),
                remediation: Some("check agent logs for details"),
            });
        }
    }

    // 5. Conductor diagnoses (severity-mapped).
    for diag in &tui_state.diagnoses {
        let severity = match diag.severity {
            DiagnosisSeverity::Alert => Severity::Error,
            DiagnosisSeverity::Warn => Severity::Warning,
            DiagnosisSeverity::Info => Severity::Info,
        };
        let remediation = diag.suggested_action.as_deref().and_then(|action| {
            if action.trim().is_empty() {
                None
            } else if action.contains("restart") {
                Some("restart the affected component")
            } else if action.contains("timeout") || action.contains("increase") {
                Some("increase timeout or resource limit")
            } else if action.contains("disk") || action.contains("space") {
                Some("free disk space or increase quota")
            } else {
                Some("review conductor diagnosis for suggested action")
            }
        });
        errors.push(CategorizedError {
            category: ErrorCategory::Conductor,
            severity,
            message: if diag.detail.is_empty() {
                diag.subject.clone()
            } else {
                format!("{}: {}", diag.subject, diag.detail)
            },
            ts_millis: diag.ts.timestamp_millis() as u64,
            source: String::new(),
            remediation,
        });
    }

    // 6. Conductor alerts.
    for alert in &tui_state.conductor_alerts {
        let severity = match alert.severity.as_str() {
            "error" | "critical" | "alert" => Severity::Error,
            "warning" | "warn" => Severity::Warning,
            _ => Severity::Info,
        };
        errors.push(CategorizedError {
            category: ErrorCategory::Conductor,
            severity,
            message: alert.message.clone(),
            ts_millis: alert.created_at_ms.max(0) as u64,
            source: String::new(),
            remediation: None,
        });
    }

    // Cross-source dedup: normalize messages and keep the entry with the
    // latest timestamp for each (source, normalized_message) pair.
    errors.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then_with(|| normalize_for_dedup(&a.message).cmp(&normalize_for_dedup(&b.message)))
            .then_with(|| b.ts_millis.cmp(&a.ts_millis))
    });
    errors.dedup_by(|a, b| {
        a.source == b.source
            && normalize_for_dedup(&a.message) == normalize_for_dedup(&b.message)
    });

    // Sort by timestamp descending (most recent first).
    errors.sort_by(|a, b| b.ts_millis.cmp(&a.ts_millis));

    errors
}

fn category_color(cat: ErrorCategory, theme: &Theme) -> Color {
    Severity::from_category(cat).color(theme)
}

/// Render the gate pass/fail ratio header.
fn render_gate_summary(
    frame: &mut Frame<'_>,
    area: Rect,
    gates: &[GateVerdictView],
    stats: &SnapshotStats,
    theme: &Theme,
) {
    let total = stats.gates_passed + stats.gates_failed;
    let ratio_text = if total == 0 {
        "No gates evaluated".to_string()
    } else {
        let pct = (stats.gates_passed as f64 / total as f64 * 100.0).round();
        format!("Gates: {}/{} passed ({pct}%)", stats.gates_passed, total)
    };

    let ratio_style = if stats.gates_failed > 0 {
        theme.danger()
    } else if total > 0 {
        theme.success()
    } else {
        theme.muted()
    };

    // Show the last few failed gates inline.
    let recent_failures: Vec<&GateVerdictView> =
        gates.iter().rev().filter(|g| !g.passed).take(3).collect();

    let mut lines = vec![Line::from(Span::styled(ratio_text, ratio_style))];

    for gv in &recent_failures {
        lines.push(Line::from(vec![
            Span::styled("\u{2718} ", theme.danger()),
            Span::styled(format!("{}/{} ", gv.plan_id, gv.task_id), theme.text()),
            Span::styled(&gv.gate, theme.muted()),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// Render the scrollable error list.
fn render_error_list(frame: &mut Frame<'_>, area: Rect, errors: &[ErrorEntry], theme: &Theme) {
    if errors.is_empty() {
        let empty = Paragraph::new("No errors").style(theme.muted());
        frame.render_widget(empty, area);
        return;
    }

    // Show most recent errors first.
    let items: Vec<ListItem<'_>> = errors
        .iter()
        .rev()
        .take(area.height as usize)
        .map(|entry| {
            let ts = fmt_ts(entry.ts_millis);
            let line = Line::from(vec![
                Span::styled(format!("[{ts}] "), theme.muted()),
                Span::styled("\u{2718} ", theme.danger()),
                Span::styled(&entry.message, theme.danger()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
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
    fn error_digest_renders_without_panic() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();

        let gates = vec![
            GateVerdictView {
                plan_id: "p1".into(),
                task_id: "t1".into(),
                gate: "compile".into(),
                passed: true,
                ts_millis: 1_000_000,
            },
            GateVerdictView {
                plan_id: "p1".into(),
                task_id: "t2".into(),
                gate: "test".into(),
                passed: false,
                ts_millis: 1_001_000,
            },
        ];

        let errors = vec![
            ErrorEntry {
                message: "compilation failed".into(),
                ts_millis: 1_001_000,
            },
            ErrorEntry {
                message: "test timeout".into(),
                ts_millis: 1_002_000,
            },
        ];

        let stats = SnapshotStats {
            gates_passed: 1,
            gates_failed: 1,
            errors_total: 2,
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_error_digest(frame, area, &gates, &errors, &stats, &theme);
            })
            .unwrap();
    }

    #[test]
    fn error_digest_empty() {
        let backend = TestBackend::new(60, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_error_digest(frame, area, &[], &[], &SnapshotStats::default(), &theme);
            })
            .unwrap();
    }

    #[test]
    fn error_aggregation_panel_renders_without_panic() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let state = TuiState::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_error_aggregation_panel(frame, area, &state, 0, &theme);
            })
            .unwrap();
    }

    #[test]
    fn error_aggregation_panel_with_failures() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        let mut state = TuiState::new();

        // Add some gate failures.
        state.gate_recent_failures.push(roko_core::FailureEntry {
            plan_id: "test-plan".into(),
            task_id: "t1".into(),
            summary: "compile gate failed: missing semicolon".into(),
            ..Default::default()
        });
        state.gate_recent_failures.push(roko_core::FailureEntry {
            plan_id: "test-plan".into(),
            task_id: "t2".into(),
            summary: "test gate failed: 3 tests failed".into(),
            ..Default::default()
        });

        // Add a failed agent.
        state
            .agent_summaries
            .push(crate::tui::dashboard::AgentSummary {
                id: "agent-1".into(),
                label: "implementer".into(),
                plan_id: Some("test-plan".into()),
                status: "failed".into(),
            });

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_error_aggregation_panel(frame, area, &state, 0, &theme);
            })
            .unwrap();
    }

    #[test]
    fn category_detection() {
        assert_eq!(
            ErrorCategory::from_message("gate verification failed"),
            ErrorCategory::Gate
        );
        assert_eq!(
            ErrorCategory::from_message("cargo compilation error"),
            ErrorCategory::Compile
        );
        assert_eq!(
            ErrorCategory::from_message("agent timeout after 5min"),
            ErrorCategory::Agent
        );
        assert_eq!(
            ErrorCategory::from_message("preflight check denied"),
            ErrorCategory::Preflight
        );
        assert_eq!(
            ErrorCategory::from_message("conductor diagnosis: stall detected"),
            ErrorCategory::Conductor
        );
        assert_eq!(
            ErrorCategory::from_message("unexpected error"),
            ErrorCategory::Runtime
        );
    }

    #[test]
    fn severity_icons_and_colors() {
        assert_eq!(Severity::Error.icon(), "\u{2718}");
        assert_eq!(Severity::Warning.icon(), "\u{25b2}");
        assert_eq!(Severity::Info.icon(), "\u{25cf}");

        let theme = Theme::dark();
        assert_eq!(Severity::Error.color(&theme), theme.danger);
        assert_eq!(Severity::Warning.color(&theme), theme.warning);
        assert_eq!(Severity::Info.color(&theme), theme.info);
    }

    #[test]
    fn remediation_inference() {
        assert_eq!(
            infer_remediation("compile gate failed: missing semicolon", ErrorCategory::Compile),
            Some("fix syntax error in source file"),
        );
        assert_eq!(
            infer_remediation("test gate failed", ErrorCategory::Gate),
            Some("fix failing tests"),
        );
        assert_eq!(
            infer_remediation("agent timeout", ErrorCategory::Agent),
            Some("check agent responsiveness or increase timeout"),
        );
        assert!(infer_remediation("unknown runtime issue", ErrorCategory::Runtime).is_none());
    }

    #[test]
    fn cross_source_dedup() {
        let mut state = TuiState::new();

        // Same error from two sources: gate_recent_failures and gate results page.
        state.gate_recent_failures.push(roko_core::FailureEntry {
            plan_id: "plan-a".into(),
            task_id: "t1".into(),
            summary: "compile gate failed: missing import".into(),
            ..Default::default()
        });

        state
            .gate_results_page
            .failure_rows
            .push(crate::tui::dashboard::GateFailureRow {
                task_id: "plan-a/t1".into(),
                error_excerpt: "compile gate failed: missing import".into(),
                created_at_ms: 5000,
                gate_name: String::new(),
            });

        let errors = collect_all_errors(&state);
        let matching: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("missing import"))
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "cross-source dedup should merge duplicates from different sources"
        );
    }

    #[test]
    fn normalize_strips_common_prefixes() {
        assert_eq!(normalize_for_dedup("compile gate failed: foo"), "foo");
        assert_eq!(normalize_for_dedup("test gate failed: bar"), "bar");
        assert_eq!(normalize_for_dedup("gate failed for task-1 (model: x)"), "task-1 (model: x)");
        assert_eq!(normalize_for_dedup("  some error  "), "some error");
    }
}
