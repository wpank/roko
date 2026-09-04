//! Collapsible plan tree widget with wave grouping, inline progress bars,
//! fixed-column layout, scrollbar, and data-rain fill for empty space.
//!
//! Ported from Mori's `plan_tree.rs` (~1078 LOC) adapted to roko's
//! `TuiState`, `PlanEntry`, and `Wave` types.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::state::{PlanEntry, TuiState};
use super::rosedust::gradient_ocean;
use crate::tui::Theme;
use crate::tui::util::truncate_middle;

// ---------------------------------------------------------------------------
// Fixed column widths (chars)
// ---------------------------------------------------------------------------

const COL_PROGRESS: usize = 6;
const COL_BAR: usize = 8;
const COL_DELTA: usize = 8;
const COL_VERIFY: usize = 3;
const COL_AGE: usize = 6;
/// Total reserved width for columns + 5 separator chars.
const RESERVED: u16 = (COL_PROGRESS + COL_BAR + COL_DELTA + COL_VERIFY + COL_AGE + 5) as u16;

// ---------------------------------------------------------------------------
// Status icons
// ---------------------------------------------------------------------------

/// Status icon and style for a plan entry.
fn plan_icon(plan: &PlanEntry) -> (&'static str, Style) {
    if !plan.active && plan.status.is_done() {
        (
            "\u{2713}", // ✓
            Style::default()
                .fg(Theme::SAGE)
                .add_modifier(Modifier::BOLD),
        )
    } else if !plan.active && plan.status.is_failed() {
        (
            "\u{2717}", // ✗
            Style::default()
                .fg(Theme::EMBER)
                .add_modifier(Modifier::BOLD),
        )
    } else if plan.active {
        (
            "\u{26a1}", // ⚡
            Style::default()
                .fg(Theme::WARNING)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            "\u{25cb}", // ○
            Style::default().fg(Theme::TEXT_GHOST),
        )
    }
}

// ---------------------------------------------------------------------------
// Public render entry-point
// ---------------------------------------------------------------------------

/// Render the collapsible plan tree: Wave -> Plan hierarchy.
/// Falls back to flat list when no waves are configured.
pub fn render_plan_tree(frame: &mut Frame<'_>, area: Rect, state: &TuiState, focused: bool) {
    let total = state.plans.len();
    let completed = state
        .plans
        .iter()
        .filter(|p| !p.active && p.tasks_failed == 0)
        .count();
    let active = state.plans.iter().filter(|p| p.active).count();
    let failed = state
        .plans
        .iter()
        .filter(|p| !p.active && p.status.is_failed())
        .count();

    let health_suffix = {
        let mut parts = Vec::new();
        if active > 0 {
            parts.push(format!("{active}\u{25b8}")); // ▸
        }
        if failed > 0 {
            parts.push(format!("{failed}\u{2717}")); // ✗
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" {}", parts.join(" "))
        }
    };

    let filter_active = is_filter_active(state);
    let filtered_plan_indices = filtered_plan_indices(state);
    let filtered_total = filtered_plan_indices.len();
    let selected_plan_id = state
        .plans
        .get(state.selected_plan_idx)
        .filter(|plan| matches_filter(plan, state))
        .or_else(|| {
            filtered_plan_indices
                .first()
                .and_then(|&idx| state.plans.get(idx))
        })
        .map(|plan| plan.id.as_str());
    let title = if focused {
        if filter_active {
            format!("Plans ({filtered_total}/{total}{health_suffix}) [Enter:detail h/l:tree]")
        } else {
            format!("Plans ({completed}/{total}{health_suffix}) [Enter:detail h/l:tree]")
        }
    } else if filter_active {
        format!("Plans ({filtered_total}/{total}{health_suffix})")
    } else {
        format!("Plans ({completed}/{total}{health_suffix})")
    };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Filter indicator
    if filter_active {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" [filter: \"{}\"", state.plan_tree_filter.pattern),
                Style::default().fg(Theme::DREAM),
            ),
            Span::styled(
                format!(" {filtered_total}/{total}]"),
                Style::default().fg(Theme::TEXT_DIM),
            ),
        ]));
    }

    // Column header
    if area.width >= 32 {
        lines.push(render_column_header(area));
    }

    if state.execution_waves.is_empty() {
        render_flat_plans(&mut lines, state, focused, area, selected_plan_id);
    } else {
        render_wave_tree(&mut lines, state, focused, area, selected_plan_id);
    }

    // Border styling
    let (border_style, title_style) = if focused {
        (Theme::focused_border_style(), Theme::focused_title_style())
    } else {
        (
            Theme::unfocused_border_style(),
            Theme::unfocused_title_style(),
        )
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Theme::block_style())
        .border_style(border_style)
        .title_style(title_style);

    let visible_height = area.height.saturating_sub(2) as usize;
    let total_lines = lines.len();

    // Scroll to keep selected visible
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_offset = state.plan_scroll_offset.min(max_scroll);
    let visible: Vec<Line> = lines
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible).block(block);
    frame.render_widget(paragraph, area);

    // (data-rain visualization removed — kept empty space clean)

    // Scrollbar
    if total_lines > visible_height {
        let inner = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );
        render_scrollbar(frame, inner, total_lines, visible_height, scroll_offset);
    }
}

// ---------------------------------------------------------------------------
// Wave tree rendering
// ---------------------------------------------------------------------------

/// Compute a 3-char verify cell string for a plan based on gate results.
///
/// Returns a short summary like "3/4" (passed/total) when gate results exist
/// for the plan, or the placeholder dot when none are available.
fn plan_verify_cell(plan_id: &str, state: &TuiState) -> (String, Color) {
    let results: Vec<_> = state
        .gate_result_summaries
        .iter()
        .filter(|g| g.plan_id == plan_id)
        .collect();
    if results.is_empty() {
        return (
            format!("{:>width$}", "\u{00b7}", width = COL_VERIFY),
            Theme::TEXT_PHANTOM,
        );
    }
    let passed = results.iter().filter(|g| g.passed).count();
    let total = results.len();
    let color = if passed == total {
        Theme::SAGE
    } else if passed == 0 {
        Theme::EMBER
    } else {
        Theme::WARNING
    };
    // Fit into COL_VERIFY (3 chars): e.g. "3/4", "ok", "0/2"
    let text = if passed == total {
        "\u{2713}".to_string() // ✓
    } else {
        format!("{passed}/{total}")
    };
    (format!("{:>width$}", text, width = COL_VERIFY), color)
}

fn render_wave_tree(
    lines: &mut Vec<Line<'static>>,
    state: &TuiState,
    focused: bool,
    area: Rect,
    selected_plan_id: Option<&str>,
) {
    let selected_wave = state
        .execution_waves
        .get(state.current_wave())
        .map(|wave| wave.index);

    for wave in &state.execution_waves {
        let wave_plans: Vec<&PlanEntry> = wave
            .plans
            .iter()
            .filter_map(|plan_id| state.plans.iter().find(|p| p.id == *plan_id))
            .filter(|plan| matches_filter(plan, state))
            .collect();
        if wave_plans.is_empty() {
            continue;
        }

        let all_done = wave.done == wave.total && wave.total > 0;
        let any_active = wave
            .plans
            .iter()
            .any(|plan_id| state.plans.iter().any(|p| p.id == *plan_id && p.active));

        // Wave header icon and style
        let (wave_icon, wave_style) = if all_done {
            (
                "\u{2713}", // ✓
                Style::default().fg(Theme::SAGE),
            )
        } else if any_active {
            (
                "\u{25b6}", // ►
                Style::default()
                    .fg(Theme::ROSE)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                "\u{00b7}", // ·
                Style::default().fg(Theme::TEXT_GHOST),
            )
        };

        let collapse_icon = if wave.expanded {
            "\u{25be}"
        } else {
            "\u{25b8}"
        }; // ▾ / ▸
        let wave_selected = selected_wave == Some(wave.index);
        let header_bg = if wave_selected {
            Theme::BG_HIGHLIGHT
        } else {
            Theme::BG_RAISED
        };

        let wave_label_fg = if all_done {
            Theme::SAGE
        } else if any_active {
            Theme::BONE_BRIGHT
        } else {
            Theme::BONE_DIM
        };

        let mut wave_spans = vec![
            Span::styled(
                format!(" {collapse_icon} "),
                Style::default().fg(Theme::FG_DIM).bg(header_bg),
            ),
            Span::styled(format!("{wave_icon} "), wave_style.bg(header_bg)),
            Span::styled(
                format!("Wave {} ", wave.index),
                Style::default()
                    .fg(wave_label_fg)
                    .bg(header_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({}/{}) ", wave.done, wave.total),
                Style::default().fg(Theme::FG_DIM).bg(header_bg),
            ),
        ];

        // Wave progress bar (8-char gradient bar)
        let wave_fill = wave.done as f64 / wave.total.max(1) as f64;
        wave_spans.extend(render_gradient_bar(
            8,
            wave_fill,
            if any_active {
                Some(state.atmosphere.heartbeat())
            } else {
                None
            },
        ));

        // Count failed plans in this wave
        let wave_failed = wave
            .plans
            .iter()
            .filter(|plan_id| {
                state
                    .plans
                    .iter()
                    .any(|p| p.id == **plan_id && !p.active && p.status.is_failed())
            })
            .count();
        if wave_failed > 0 {
            wave_spans.push(Span::styled(
                format!(" \u{2717}{wave_failed}"),
                Style::default().fg(Theme::EMBER),
            ));
        }

        // "after W{N}" blocker label for pending waves
        if !all_done && !any_active && !wave.blocked_by_waves.is_empty() {
            let blocker_label = wave
                .blocked_by_waves
                .iter()
                .filter(|&&w| {
                    // Only show incomplete blocker waves
                    state
                        .execution_waves
                        .iter()
                        .any(|ew| ew.index == w && ew.done < ew.total)
                })
                .map(|w| format!("W{w}"))
                .collect::<Vec<_>>()
                .join(",");
            if !blocker_label.is_empty() {
                wave_spans.push(Span::styled(
                    format!(" after {blocker_label}"),
                    Style::default().fg(Theme::TEXT_GHOST).bg(header_bg),
                ));
            }
        }

        // Fill remaining width with horizontal line
        let used: usize = wave_spans.iter().map(|s| s.content.chars().count()).sum();
        let avail = (area.width.saturating_sub(2)) as usize;
        if avail > used + 1 {
            wave_spans.push(Span::styled(
                format!(" {}", "\u{2500}".repeat(avail - used - 1)),
                Style::default().fg(Theme::TEXT_GHOST).bg(header_bg),
            ));
        }
        lines.push(Line::from(wave_spans));

        if !wave.expanded {
            continue;
        }

        // Plans within wave
        for plan in wave_plans {
            render_plan_line(
                lines,
                plan,
                state,
                focused,
                area,
                true,
                selected_plan_id,
                wave_selected,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Flat plan list (no waves)
// ---------------------------------------------------------------------------

fn render_flat_plans(
    lines: &mut Vec<Line<'static>>,
    state: &TuiState,
    focused: bool,
    area: Rect,
    selected_plan_id: Option<&str>,
) {
    for plan in &state.plans {
        if matches_filter(plan, state) {
            render_plan_line(
                lines,
                plan,
                state,
                focused,
                area,
                false,
                selected_plan_id,
                false,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Single plan line
// ---------------------------------------------------------------------------

fn render_plan_line(
    lines: &mut Vec<Line<'static>>,
    plan: &PlanEntry,
    state: &TuiState,
    focused: bool,
    area: Rect,
    indented: bool,
    selected_plan_id: Option<&str>,
    wave_selected: bool,
) {
    let is_selected = focused && selected_plan_id == Some(plan.id.as_str());

    let (icon, icon_style) = plan_icon(plan);

    // Text styling by plan status
    let text_style = if !plan.active && plan.status.is_done() {
        Style::default().fg(Theme::SAGE)
    } else if plan.active {
        Style::default()
            .fg(Theme::ROSE_BRIGHT)
            .add_modifier(Modifier::BOLD)
    } else if plan.status.is_failed() {
        Style::default().fg(Theme::EMBER)
    } else {
        Style::default().fg(Theme::TEXT_DIM)
    };

    let bg = if is_selected {
        Theme::BG_HIGHLIGHT
    } else if wave_selected {
        Theme::BG_SECONDARY
    } else {
        Theme::BG
    };
    let icon_s = if is_selected {
        icon_style.bg(bg)
    } else {
        icon_style
    };
    let text_s = if is_selected {
        text_style.bg(bg)
    } else {
        text_style
    };

    let indent = if indented { "   " } else { " " };

    // -- Task progress fraction --
    let fill_pct = if plan.tasks_total > 0 {
        plan.tasks_done as f64 / plan.tasks_total as f64
    } else {
        0.0
    };

    // Progress cell (6 chars): e.g. " 3/10"
    let progress_cell = if plan.tasks_total > 0 {
        let color = if !plan.active && plan.tasks_failed == 0 {
            Theme::SAGE
        } else if plan.active {
            if fill_pct >= 0.999 {
                Theme::WARNING
            } else {
                Theme::semantic_color(fill_pct)
            }
        } else if plan.status.is_failed() {
            Theme::EMBER
        } else if plan.tasks_done == 0 {
            Theme::TEXT_GHOST
        } else {
            Theme::TEXT_DIM
        };
        (
            format!(
                "{:>width$}",
                format!("{}/{}", plan.tasks_done.min(99), plan.tasks_total.min(99)),
                width = COL_PROGRESS
            ),
            color,
        )
    } else {
        (
            format!("{:>width$}", "\u{00b7}", width = COL_PROGRESS),
            Theme::TEXT_PHANTOM,
        )
    };

    // Bar cell (8 chars): gradient-filled blocks with partial-fill chars
    let bar_spans = render_progress_bar_spans(COL_BAR, fill_pct, plan);

    // Delta cell (8 chars): git dirty or health indicator
    let delta_cell = if plan.tasks_failed > 0 {
        (
            format!(
                "{:>width$}",
                truncate_middle(&format!("\u{2717}{}", plan.tasks_failed), COL_DELTA),
                width = COL_DELTA
            ),
            Theme::EMBER,
        )
    } else {
        (
            format!("{:>width$}", "\u{00b7}", width = COL_DELTA),
            Theme::TEXT_PHANTOM,
        )
    };

    // Verify cell (3 chars): gate verdict summary for this plan
    let verify_cell = plan_verify_cell(&plan.id, state);

    // Age cell (6 chars): elapsed time — use live `started_at` for active plans (P5.5)
    let live_elapsed = plan
        .started_at
        .map(|t| t.elapsed().as_secs_f64())
        .filter(|&s| s > 0.0);
    let display_elapsed = live_elapsed.unwrap_or(plan.elapsed_secs);
    let age_cell = if display_elapsed > 0.0 {
        (
            format!(
                "{:>width$}",
                truncate_middle(&format_duration(display_elapsed), COL_AGE),
                width = COL_AGE
            ),
            if plan.active && live_elapsed.is_some() {
                Theme::WARNING
            } else {
                Theme::TEXT_DIM
            },
        )
    } else {
        (
            format!("{:>width$}", "\u{00b7}", width = COL_AGE),
            Theme::TEXT_PHANTOM,
        )
    };

    // Phase tag next to the icon for active plans: "⚡impl "
    let phase_tag = if plan.active && !plan.phase.is_empty() {
        phase_abbrev(&plan.phase)
    } else {
        ""
    };

    // Build prefix: indent + icon + optional phase tag
    let prefix_plain = if phase_tag.is_empty() {
        format!("{indent}{icon} ")
    } else {
        format!("{indent}{icon}{phase_tag} ")
    };
    let available_title = area
        .width
        .saturating_sub(prefix_plain.len() as u16)
        .saturating_sub(RESERVED)
        .max(10) as usize;
    let plan_name = truncate_plan_name(&plan.name, available_title);

    let title_span = Span::styled(
        format!("{plan_name:<width$}", width = available_title),
        text_s.bg(bg),
    );

    let sep = |bg_c: Color| {
        Span::styled(
            "\u{2502}",
            Style::default().fg(Theme::TEXT_PHANTOM).bg(bg_c),
        )
    };

    let mut spans = if phase_tag.is_empty() {
        vec![Span::styled(prefix_plain, icon_s.bg(bg)), title_span]
    } else {
        let phase_color = Theme::phase_accent(&plan.phase);
        vec![
            Span::styled(format!("{indent}{icon}"), icon_s.bg(bg)),
            Span::styled(
                format!("{phase_tag} "),
                Style::default()
                    .fg(phase_color)
                    .bg(bg)
                    .add_modifier(Modifier::DIM),
            ),
            title_span,
        ]
    };
    spans.push(sep(bg));
    spans.push(Span::styled(
        progress_cell.0,
        Style::default().fg(progress_cell.1).bg(bg),
    ));
    spans.push(sep(bg));
    for mut s in bar_spans {
        // Apply bg to each gradient span
        s.style = s.style.bg(bg);
        spans.push(s);
    }
    spans.extend([
        sep(bg),
        Span::styled(delta_cell.0, Style::default().fg(delta_cell.1).bg(bg)),
        sep(bg),
        Span::styled(verify_cell.0, Style::default().fg(verify_cell.1).bg(bg)),
        sep(bg),
        Span::styled(age_cell.0, Style::default().fg(age_cell.1).bg(bg)),
    ]);

    lines.push(Line::from(spans));

    // Selected plan detail row (expanded info)
    if is_selected {
        // Show full name when it was truncated
        let name_was_truncated = plan.name.chars().count() > available_title;
        if name_was_truncated {
            lines.push(Line::from(vec![
                Span::styled(format!("{indent}  "), Style::default().bg(bg)),
                Span::styled(
                    truncate_middle(&plan.name, area.width.saturating_sub(6) as usize),
                    Style::default().fg(Theme::BONE_DIM).bg(bg),
                ),
            ]));
        }

        let mut detail_spans = vec![Span::styled(format!("{indent}  "), Style::default().bg(bg))];

        // Mini progress bar in the detail row
        if plan.tasks_total > 0 {
            detail_spans.push(Span::styled(
                compact_progress_glyphs(8, fill_pct),
                Style::default().fg(Theme::semantic_color(fill_pct)).bg(bg),
            ));
            detail_spans.push(Span::styled(
                "  ",
                Style::default().fg(Theme::TEXT_PHANTOM).bg(bg),
            ));
        }

        let mut detail_parts: Vec<(String, Color)> = Vec::new();
        if !plan.phase.is_empty() {
            detail_parts.push((format!("phase {}", plan.phase), Theme::ROSE_DIM));
        }
        if plan.tasks_failed > 0 {
            detail_parts.push((format!("{} failed", plan.tasks_failed), Theme::EMBER));
        }
        if display_elapsed > 0.0 {
            detail_parts.push((
                format!("elapsed {}", format_duration(display_elapsed)),
                Theme::TEXT_GHOST,
            ));
        }

        for (idx, (text, color)) in detail_parts.iter().enumerate() {
            if idx > 0 {
                detail_spans.push(Span::styled(
                    " \u{00b7} ",
                    Style::default().fg(Theme::TEXT_PHANTOM).bg(bg),
                ));
            }
            detail_spans.push(Span::styled(
                truncate_middle(text, area.width.saturating_sub(8) as usize),
                Style::default().fg(*color).bg(bg),
            ));
        }
        if detail_spans.len() > 1 {
            lines.push(Line::from(detail_spans));
        }
    }
}

// ---------------------------------------------------------------------------
// Column header
// ---------------------------------------------------------------------------

fn render_column_header(area: Rect) -> Line<'static> {
    let reserved: u16 = (COL_PROGRESS + COL_BAR + COL_DELTA + COL_VERIFY + COL_AGE + 5) as u16;
    let title_width = area.width.saturating_sub(reserved + 3).max(10) as usize;
    let theme = Theme::dark();
    let meta = theme.metadata();
    let sep = Span::styled("\u{2502}", Style::default().fg(Theme::TEXT_PHANTOM));
    Line::from(vec![
        Span::styled(
            format!(" {:<width$}", "plan", width = title_width),
            theme.section_header(),
        ),
        sep.clone(),
        Span::styled(format!("{:>6}", "prog"), meta),
        sep.clone(),
        Span::styled(format!("{:>8}", "bar"), meta),
        sep.clone(),
        Span::styled(format!("{:>8}", "delta"), meta),
        sep.clone(),
        Span::styled(format!("{:>3}", "vfy"), meta),
        sep,
        Span::styled(format!("{:>6}", "age"), meta),
    ])
}

// ---------------------------------------------------------------------------
// Gradient bar spans (ocean gradient)
// ---------------------------------------------------------------------------

fn render_gradient_bar(width: usize, fill_pct: f64, heartbeat: Option<f64>) -> Vec<Span<'static>> {
    let grad = gradient_ocean();
    let filled = ((fill_pct.clamp(0.0, 1.0)) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    let mut spans = Vec::with_capacity(width);
    for i in 0..filled.min(width) {
        let t = if filled <= 1 {
            0.5
        } else {
            i as f64 / (filled - 1).max(1) as f64
        };
        let mut color = grad.sample(t);
        if let Some(hb) = heartbeat {
            color = super::rosedust::brighten(color, hb);
        }
        spans.push(Span::styled(
            "\u{2588}".to_string(),
            Style::default().fg(color),
        ));
    }
    if empty > 0 {
        spans.push(Span::styled(
            "\u{2500}".repeat(empty),
            Style::default().fg(Theme::TEXT_PHANTOM),
        ));
    }
    spans
}

// ---------------------------------------------------------------------------
// Scrollbar (lightweight, buffer-direct)
// ---------------------------------------------------------------------------

fn render_scrollbar(
    frame: &mut Frame<'_>,
    area: Rect,
    total: usize,
    visible: usize,
    offset: usize,
) {
    if total <= visible || area.height == 0 {
        return;
    }

    let track_height = area.height as usize;
    let thumb_height = ((visible as f64 / total as f64) * track_height as f64)
        .ceil()
        .max(1.0) as usize;
    let thumb_top = if total > visible {
        ((offset as f64 / (total - visible) as f64) * (track_height - thumb_height) as f64).round()
            as usize
    } else {
        0
    };

    let x = area.x + area.width.saturating_sub(1);
    let buf = frame.buffer_mut();

    for i in 0..track_height {
        let y = area.y + i as u16;
        let in_thumb = i >= thumb_top && i < thumb_top + thumb_height;
        let (ch, color) = if in_thumb {
            ('\u{2588}', Theme::ROSE_DIM) // █
        } else {
            ('\u{2502}', Theme::TEXT_PHANTOM) // │
        };
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char(ch);
            cell.set_fg(color);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_filter_active(state: &TuiState) -> bool {
    state.plan_tree_filter.active && !state.plan_tree_filter.pattern.is_empty()
}

fn filtered_plan_indices(state: &TuiState) -> Vec<usize> {
    state
        .plans
        .iter()
        .enumerate()
        .filter_map(|(idx, plan)| matches_filter(plan, state).then_some(idx))
        .collect()
}

fn matches_filter(plan: &PlanEntry, state: &TuiState) -> bool {
    if !is_filter_active(state) {
        return true;
    }
    state.plan_tree_filter.matches_plan_or_tasks(plan)
}

/// Truncate a plan name, keeping the first dash-delimited segment intact.
///
/// For plan names like `async-runtime-patterns`, this produces
/// `async-runtime-...ns` rather than the generic `async-r...atterns`, keeping
/// the leading segment (the most distinctive part) fully readable.
fn truncate_plan_name(name: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let char_count = name.chars().count();
    if char_count <= max {
        return name.to_string();
    }
    if max <= 3 {
        return "\u{2026}".repeat(max); // …
    }
    // Find the first segment boundary (first '-')
    let first_dash = name.find('-').unwrap_or(char_count);
    // Keep at least the first segment + dash, up to half the budget
    let left_budget = (first_dash + 1).min(max.saturating_sub(4)); // need room for …+tail
    let right_budget = max.saturating_sub(left_budget + 1); // 1 for …
    if right_budget == 0 {
        // No room for tail, fall back to simple left + …
        let left: String = name.chars().take(max - 1).collect();
        return format!("{left}\u{2026}");
    }
    let left: String = name.chars().take(left_budget).collect();
    let right: String = name
        .chars()
        .rev()
        .take(right_budget)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{left}\u{2026}{right}")
}

/// Build gradient progress bar spans with partial-fill characters.
///
/// Uses `█▓▒░` for the fractional cell and colors based on progress:
/// red (0-25%), yellow (25-75%), green (75-100%).
fn render_progress_bar_spans(width: usize, fill_pct: f64, plan: &PlanEntry) -> Vec<Span<'static>> {
    if plan.tasks_total == 0 {
        return vec![Span::styled(
            format!("{:>width$}", "\u{00b7}", width = width),
            Style::default().fg(Theme::TEXT_PHANTOM),
        )];
    }
    let pct = fill_pct.clamp(0.0, 1.0);
    let filled_f = pct * width as f64;
    let filled = filled_f as usize;
    let frac = filled_f - filled as f64;
    let empty = width
        .saturating_sub(filled)
        .saturating_sub(if frac > 0.01 { 1 } else { 0 });

    // Color based on status and progress
    let bar_color = if !plan.active && plan.tasks_failed == 0 && plan.status.is_done() {
        Theme::SAGE
    } else if plan.status.is_failed() {
        Theme::EMBER
    } else {
        Theme::progress_gradient(pct)
    };

    let mut spans = Vec::with_capacity(3);

    // Filled portion
    if filled > 0 {
        spans.push(Span::styled(
            "\u{2588}".repeat(filled.min(width)),
            Style::default().fg(bar_color),
        ));
    }

    // Fractional cell using gradient characters
    if frac > 0.01 && filled < width {
        let partial = if frac >= 0.75 {
            "\u{2593}" // ▓
        } else if frac >= 0.5 {
            "\u{2592}" // ▒
        } else {
            "\u{2591}" // ░
        };
        spans.push(Span::styled(
            partial.to_string(),
            Style::default().fg(bar_color),
        ));
    }

    // Empty portion
    if empty > 0 {
        spans.push(Span::styled(
            "\u{2500}".repeat(empty),
            Style::default().fg(Theme::TEXT_PHANTOM),
        ));
    }

    spans
}

fn compact_progress_glyphs(width: usize, fill_pct: f64) -> String {
    if width == 0 {
        return String::new();
    }
    let filled = ((fill_pct.clamp(0.0, 1.0)) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        " {}{}",
        "\u{2588}".repeat(filled.min(width)),
        "\u{2500}".repeat(empty)
    )
}

fn phase_abbrev(phase: &str) -> &'static str {
    match phase {
        p if p.contains("preflight") => "prfl",
        p if p.contains("strateg") => "strt",
        p if p.contains("implement") => "impl",
        p if p.contains("verify") => "vfy ",
        p if p.contains("merge") => "mrge",
        p if p.contains("complete") || p.contains("done") => "done",
        p if p.contains("fail") => "fail",
        p if p.contains("compile") => "comp",
        p if p.contains("test") => "test",
        p if p.contains("review") => "revw",
        p if p.contains("critic") => "crit",
        p if p.contains("verdict") => "vdct",
        p if p.contains("doc") => "docs",
        p if p.contains("commit") => "cmit",
        p if p.contains("gate") || p.contains("gating") => "gate",
        _ => "run ",
    }
}

/// Format seconds as compact duration: "30m", "2h", "1d", "45s".
fn format_duration(secs: f64) -> String {
    let s = secs as u64;
    if s >= 86400 {
        format!("{}d", s / 86400)
    } else if s >= 3600 {
        format!("{}h", s / 3600)
    } else if s >= 60 {
        format!("{}m", s / 60)
    } else {
        format!("{}s", s)
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

    use crate::tui::state::Wave;

    fn sample_state() -> TuiState {
        use crate::tui::dashboard::DashboardData;
        use crate::tui::state::TuiState;

        let data = DashboardData::default();
        let mut state = TuiState::from_dashboard_data(&data);
        state.plans = vec![
            PlanEntry {
                id: "plan-alpha".into(),
                name: "plan-alpha".into(),
                wave: Some(0),
                tasks_total: 5,
                tasks_done: 3,
                tasks_failed: 0,
                active: true,
                phase: "implementing".into(),
                elapsed_secs: 120.0,
                ..Default::default()
            },
            PlanEntry {
                id: "plan-beta".into(),
                name: "plan-beta".into(),
                wave: Some(0),
                tasks_total: 4,
                tasks_done: 4,
                tasks_failed: 0,
                active: false,
                phase: "done".into(),
                elapsed_secs: 300.0,
                ..Default::default()
            },
            PlanEntry {
                id: "plan-gamma".into(),
                name: "plan-gamma".into(),
                wave: Some(1),
                tasks_total: 3,
                tasks_done: 1,
                tasks_failed: 2,
                active: false,
                phase: "failed".into(),
                elapsed_secs: 60.0,
                ..Default::default()
            },
        ];
        state.execution_waves = vec![
            Wave {
                index: 0,
                plans: vec!["plan-alpha".into(), "plan-beta".into()],
                done: 1,
                total: 2,
                expanded: true,
                blocked_by_waves: Vec::new(),
            },
            Wave {
                index: 1,
                plans: vec!["plan-gamma".into()],
                done: 0,
                total: 1,
                expanded: true,
                blocked_by_waves: vec![0],
            },
        ];
        state
    }

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

    #[test]
    fn plan_tree_renders_without_panic() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = sample_state();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_plan_tree(frame, area, &state, true);
            })
            .unwrap();
    }

    #[test]
    fn plan_tree_flat_fallback() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.execution_waves.clear();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_plan_tree(frame, area, &state, false);
            })
            .unwrap();
    }

    #[test]
    fn plan_tree_empty() {
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.plans.clear();
        state.execution_waves.clear();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_plan_tree(frame, area, &state, false);
            })
            .unwrap();
    }

    #[test]
    fn plan_tree_filters_visible_plans_and_clamps_selection() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.plan_tree_filter.active = true;
        state.plan_tree_filter.pattern = "BETA".into();
        state.plan_tree_filter.reparse();
        state.selected_plan_idx = 2;

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_plan_tree(frame, area, &state, true);
            })
            .unwrap();

        let rendered = rendered_text(&terminal);
        assert!(rendered.contains("1/3"), "{rendered}");
        assert!(rendered.contains("plan-beta"), "{rendered}");
        assert!(!rendered.contains("plan-alpha"), "{rendered}");
        assert!(!rendered.contains("plan-gamma"), "{rendered}");
        assert!(rendered.contains("phase done"), "{rendered}");
    }

    #[test]
    fn middle_truncation_edge_cases() {
        assert_eq!(truncate_middle("hello", 10), "hello");
        assert_eq!(truncate_middle("hello world", 5), "he\u{2026}ld");
        assert_eq!(truncate_middle("abc", 0), "");
        assert_eq!(truncate_middle("abcdef", 3), "\u{2026}\u{2026}\u{2026}");
    }

    #[test]
    fn plan_name_truncation_keeps_first_segment() {
        // Fits entirely
        assert_eq!(truncate_plan_name("plan-alpha", 20), "plan-alpha");
        // Truncation keeps first segment intact
        assert_eq!(
            truncate_plan_name("async-runtime-patterns", 16),
            "async-\u{2026}-patterns"
        );
        // Very short budget
        assert_eq!(
            truncate_plan_name("something-long", 3),
            "\u{2026}\u{2026}\u{2026}"
        );
        // Empty
        assert_eq!(truncate_plan_name("anything", 0), "");
        // No dash — still keeps both ends visible
        assert_eq!(truncate_plan_name("abcdefghijklmnop", 8), "abcd\u{2026}nop");
    }

    #[test]
    fn phase_abbrev_known() {
        assert_eq!(phase_abbrev("implementing"), "impl");
        assert_eq!(phase_abbrev("compile-gate"), "comp");
        assert_eq!(phase_abbrev("verifying"), "vfy ");
    }

    #[test]
    fn format_duration_ranges() {
        assert_eq!(format_duration(30.0), "30s");
        assert_eq!(format_duration(120.0), "2m");
        assert_eq!(format_duration(7200.0), "2h");
        assert_eq!(format_duration(90000.0), "1d");
    }

    #[test]
    fn progress_glyphs_bounds() {
        let empty = compact_progress_glyphs(0, 0.5);
        assert!(empty.is_empty());
        let full = compact_progress_glyphs(4, 1.0);
        assert!(full.contains('\u{2588}'));
    }
}
