# C4: Implement F6 Config sub-views

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**TUI source:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/`
**Framework:** ratatui + crossterm + tokio
**Theme:** ROSEDUST (rose=#AA7088, bone=#C8B890, bg=#060608)
**Architecture:**
- `App` (app.rs) owns `TuiState` + `DashboardData`, runs 60fps loop
- `TuiState` (state.rs) holds all display state, populated by two paths:
  - StateHub: `drain_snapshot_channel()` -> `update_from_dashboard_snapshot()`
  - File poll: `tick_snapshot()` -> `DashboardData::tick()` -> `update_from_snapshot()`
- Views (views/*.rs) receive `(&DashboardData, &TuiState, &ViewState, &Theme)` -- zero I/O
- Widgets (widgets/*.rs) are reusable ratatui components
- Navigation: F-keys switch tabs, 1-9 switch sub-views, j/k navigate, Enter expands

**Tab enum** at `crates/roko-cli/src/tui/tabs.rs`:
- Current: Dashboard(F1), Plans(F2), Agents(F3), Git(F4), Logs(F5), Config(F6), Inspect(F7)
- `ALL: [Tab; 7]`, with `fkey()`, `from_key()`, `label()`, `label_with_key()`, `index()` methods

**SubView enum** at `crates/roko-cli/src/tui/views/mod.rs` -- already has these variants defined (but NOT all rendered):
- F6: ConfigEditor, ProviderHealth, ModelComparison
- F7: EngramDag, EpisodeReplay, KnowledgeBrowse

**Key ViewState fields** (in state.rs):
- `sub_tab: usize` -- which sub-view within a tab
- `selected_primary: usize`, `selected_secondary: usize` -- list selection indices
- `scroll_offset: usize` -- for scrollable content

### Pre-commit (MANDATORY)
```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Goal

The F6 Config tab currently renders only the config editor (sub_tab 0). Two additional sub-views are defined in the `SubView` enum (`ProviderHealth`, `ModelComparison`) but are not rendered. This task adds rendering code for both and dispatches them based on `view_state.sub_tab`.

## Dependency

None. The SubView variants already exist. The config_view.rs file already exists with a working `render()` function.

**Audit update (2026-04-22):** the two sub-views exist, but the implementation still uses `Block::default().borders(...)`, `--` in empty states, and lacks the explicit provider/tier helper split required below.

- [ ] Polish Config sub-views to the acceptance criteria: `Block::bordered()`, em-dash empty values, standalone `infer_provider()` / `infer_tier()`, and threshold colors for green/amber/red/ghost.

## Steps

### Step 1: Read existing code

Read these files:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/config_view.rs` -- understand the existing render function
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/mod.rs` -- confirm SubView::ProviderHealth, ModelComparison exist
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs` -- search for `cascade_router`, `CascadeRouterState`, `efficiency_events`

Key data sources already in `DashboardData`:
- `data.cascade_router: CascadeRouterState` -- has `model_slugs: Vec<String>` and
  `confidence_stats: HashMap<String, ConfidenceStats>` where `ConfidenceStats` has
  `trials: u64` and `successes: u64`
- `data.efficiency_events: Vec<AgentEfficiencyEvent>` -- each has `model`,
  `input_tokens`, `output_tokens`, `cost_usd`, `wall_time_ms`, `gate_passed`
- `data.efficiency: EfficiencySummary` -- aggregate stats

Also search for `ProviderHealth` types in the crate:
```bash
grep -rn 'pub struct ProviderHealth' crates/ --include='*.rs' | grep -v target/
grep -rn 'pub struct CircuitState' crates/ --include='*.rs' | grep -v target/
```

The dashboard already imports `roko_learn::provider_health::{CircuitState, ProviderHealth}`.

### Step 2: Modify config_view.rs

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/config_view.rs`

**2a. Add new imports:**

Find:
```rust
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
```

Replace with:
```rust
use ratatui::layout::{Constraint, Layout};
use ratatui::text::Span;
use ratatui::style::Modifier;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap};
```

Note: only add what is not already imported. Check the existing import list first.

**2b. Replace the `render()` function to dispatch on sub_tab:**

Find the existing `render()` function signature:
```rust
/// Render the full config editor view.
#[allow(clippy::too_many_lines)]
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    tui_state: &TuiState,
    _view_state: &ViewState,
    theme: &Theme,
) {
```

Replace the entire function (from the doc comment through its closing `}`) with two
functions: a thin dispatcher and the renamed original:

```rust
/// Render the config view -- dispatches to sub-views based on `view_state.sub_tab`.
#[allow(clippy::too_many_lines)]
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    match view_state.sub_tab {
        1 => render_provider_health(frame, area, data, theme),
        2 => render_model_comparison(frame, area, data, theme),
        _ => render_config_editor(frame, area, data, tui_state, theme),
    }
}

/// Render the config editor (sub_tab 0) -- original implementation.
///
/// Extracted from the old `render()` so the dispatcher above can call it.
#[allow(clippy::too_many_lines)]
fn render_config_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    tui_state: &TuiState,
    theme: &Theme,
) {
    // ---- PASTE THE EXACT BODY OF THE OLD render() FUNCTION HERE ----
    // Nothing changes inside the body -- the caller changed, not the logic.
    let focused = matches!(tui_state.focus, FocusZone::RightPanel);
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
        .title(Span::styled(" Config ", title_style))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    // Build the flat item list (editable fields + runtime sections)
    let mut items = build_flat_items(data.root(), &tui_state.config_pending);

    // Append runtime data sections
    append_runtime_sections(&mut items, data);

    // Clamp cursor
    let cursor = tui_state.config_cursor.min(items.len().saturating_sub(1));
    let viewport_h = inner.height as usize;

    // Compute lines each item takes (field with description on selected = 2 lines)
    let mut line_offsets: Vec<usize> = Vec::with_capacity(items.len());
    let mut total_lines = 0usize;
    for (i, item) in items.iter().enumerate() {
        line_offsets.push(total_lines);
        total_lines += item_height(item, i == cursor);
    }

    // Scroll to keep cursor visible
    let cursor_top = line_offsets.get(cursor).copied().unwrap_or(0);
    let cursor_bottom =
        cursor_top + item_height(items.get(cursor).unwrap_or(&ConfigItem::SaveButton), true);
    let mut scroll = 0;
    if cursor_top < scroll {
        scroll = cursor_top;
    }
    if cursor_bottom > scroll + viewport_h {
        scroll = cursor_bottom.saturating_sub(viewport_h);
    }

    // Render visible items
    let mut lines: Vec<Line<'_>> = Vec::new();
    let has_pending = !tui_state.config_pending.is_empty();

    for (i, item) in items.iter().enumerate() {
        let is_selected = i == cursor;
        match item {
            ConfigItem::Header(name) => {
                lines.push(render_header(name, inner.width, theme));
            }
            ConfigItem::Field { meta, value, source } => {
                let is_modified = tui_state.config_pending.contains_key(meta.key);
                let is_editing = is_selected
                    && tui_state.config_editing
                    && tui_state.config_edit_key.as_deref() == Some(meta.key);

                let display_value = if is_editing {
                    &tui_state.config_edit_buffer
                } else {
                    value
                };

                lines.push(render_field_line(
                    meta.label,
                    display_value,
                    &meta.kind,
                    *source,
                    is_selected,
                    is_modified,
                    is_editing,
                    inner.width,
                    theme,
                ));

                // Show description on selected field
                if is_selected {
                    lines.push(render_description(meta.description, inner.width, theme));
                }
            }
            ConfigItem::SaveButton => {
                lines.push(render_save_button(
                    is_selected,
                    has_pending,
                    inner.width,
                    theme,
                ));
            }
        }
    }

    // Apply scroll offset: skip `scroll` lines from the top
    let visible_lines: Vec<Line<'_>> = lines
        .into_iter()
        .skip(scroll)
        .take(viewport_h.saturating_sub(1)) // leave room for hint bar
        .collect();

    let content = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    frame.render_widget(content, inner);

    // Hint bar at the bottom
    let hint_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    let hint = if tui_state.config_editing {
        Line::from(vec![
            Span::styled("Enter", theme.accent()),
            Span::styled(":confirm  ", theme.muted()),
            Span::styled("Esc", theme.accent()),
            Span::styled(":cancel", theme.muted()),
        ])
    } else {
        Line::from(vec![
            Span::styled("j/k", theme.accent()),
            Span::styled(":nav  ", theme.muted()),
            Span::styled("h/l", theme.accent()),
            Span::styled(":cycle  ", theme.muted()),
            Span::styled("Enter", theme.accent()),
            Span::styled(":edit  ", theme.muted()),
            Span::styled("Ctrl-S", theme.accent()),
            Span::styled(":save", theme.muted()),
        ])
    };
    frame.render_widget(Paragraph::new(hint), hint_area);
}
```

**IMPORTANT:** The `render_config_editor` body above shows the expected structure. Copy
the exact body from the old `render()` function -- do not use this as a template if the
actual code differs. The function extraction is mechanical: same body, new private name.

**2c. Add the two new sub-view render functions.**

Insert both functions BEFORE the `append_runtime_sections` function. Separate them with
a comment banner.

Column widths use fixed `Constraint::Length` values so columns align regardless of data.
The "best" value per column is highlighted with `Modifier::BOLD`.

```rust
// ===========================================================================
// Sub-view 1: Provider Health (sub_tab == 1)
// ===========================================================================

/// Render the provider health monitoring view.
///
/// Status legend:
///   healthy  (green)  -- trials > 0 && success rate >= 80%
///   degraded (amber)  -- trials > 0 && success rate >= 50%
///   failing  (red)    -- trials > 0 && success rate <  50%
///   idle     (ghost)  -- trials == 0 (no data yet)
fn render_provider_health(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            " Provider Health ",
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if data.cascade_router.model_slugs.is_empty() {
        let msg = Paragraph::new(
            "no provider data -- run agents to populate the cascade router",
        )
        .style(theme.muted())
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(msg, center_rect(inner));
        return;
    }

    if inner.height < 4 || inner.width < 30 {
        return;
    }

    let sections = Layout::vertical([
        Constraint::Length(3), // summary bar
        Constraint::Min(0),    // provider table
    ])
    .split(inner);

    // Summary bar
    let total_models = data.cascade_router.model_slugs.len();
    let total_trials: u64 = data
        .cascade_router
        .confidence_stats
        .values()
        .map(|s| s.trials)
        .sum();
    let total_successes: u64 = data
        .cascade_router
        .confidence_stats
        .values()
        .map(|s| s.successes)
        .sum();
    let overall_rate_pct = if total_trials > 0 {
        total_successes as f64 / total_trials as f64 * 100.0
    } else {
        0.0
    };
    let overall_rate_str = if total_trials > 0 {
        format!("{overall_rate_pct:.1}%")
    } else {
        "\u{2014}".to_string() // em-dash
    };
    let overall_style = if total_trials == 0 {
        theme.muted()
    } else if overall_rate_pct >= 80.0 {
        theme.success()
    } else if overall_rate_pct >= 50.0 {
        theme.warning()
    } else {
        theme.danger()
    };

    let summary = Paragraph::new(Line::from(vec![
        Span::styled("providers:", theme.muted()),
        Span::raw(" "),
        Span::styled(total_models.to_string(), theme.text()),
        Span::raw("   "),
        Span::styled("total trials:", theme.muted()),
        Span::raw(" "),
        Span::styled(total_trials.to_string(), theme.text()),
        Span::raw("   "),
        Span::styled("overall success:", theme.muted()),
        Span::raw(" "),
        Span::styled(overall_rate_str, overall_style.add_modifier(Modifier::BOLD)),
    ]));
    frame.render_widget(summary, sections[0]);

    // Provider table
    // Columns: Provider (10) | Status (8) | Model (24 min) | Trials (6) | Rate (6)
    // Fixed widths keep columns stable as data changes.
    let widths = [
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Min(16),
        Constraint::Length(6),
        Constraint::Length(6),
    ];

    let header = Row::new(["Provider", "Status", "Model", "Trials", "Rate"])
        .style(theme.accent().add_modifier(Modifier::BOLD));

    let mut rows: Vec<Row<'_>> = Vec::with_capacity(data.cascade_router.model_slugs.len());

    for slug in &data.cascade_router.model_slugs {
        let stats = data.cascade_router.confidence_stats.get(slug);
        let trials = stats.map_or(0, |s| s.trials);
        let successes = stats.map_or(0, |s| s.successes);
        let rate_pct = if trials > 0 {
            successes as f64 / trials as f64 * 100.0
        } else {
            0.0
        };

        // Green / amber / red / ghost based on success rate
        let (status_text, rate_style) = if trials == 0 {
            ("idle", theme.muted())
        } else if rate_pct >= 80.0 {
            ("healthy", theme.success())
        } else if rate_pct >= 50.0 {
            ("degraded", theme.warning())
        } else {
            ("failing", theme.danger())
        };

        let rate_str = if trials > 0 {
            format!("{rate_pct:.0}%")
        } else {
            "\u{2014}".to_string()
        };

        let provider = infer_provider(slug);

        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(provider, theme.text())),
                Cell::from(Span::styled(status_text, rate_style)),
                Cell::from(Span::styled(truncate(slug, 24), theme.text())),
                Cell::from(Span::styled(
                    if trials > 0 { trials.to_string() } else { "\u{2014}".to_string() },
                    theme.text(),
                )),
                Cell::from(Span::styled(rate_str, rate_style.add_modifier(Modifier::BOLD))),
            ])
        );
    }

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1);
    frame.render_widget(table, sections[1]);
}

// ===========================================================================
// Sub-view 2: Model Comparison (sub_tab == 2)
// ===========================================================================

/// Per-model aggregate built from efficiency events + cascade router stats.
struct ModelComparisonAgg {
    slug: String,
    total_cost: f64,
    total_input: u64,
    total_output: u64,
    event_count: u64,
    passed_count: u64,
    total_wall_ms: u64,
    trials: u64,
    successes: u64,
}

impl ModelComparisonAgg {
    fn gate_rate(&self) -> Option<f64> {
        if self.event_count == 0 {
            None
        } else {
            Some(self.passed_count as f64 / self.event_count as f64)
        }
    }

    fn avg_wall_ms(&self) -> Option<f64> {
        if self.event_count == 0 {
            None
        } else {
            Some(self.total_wall_ms as f64 / self.event_count as f64)
        }
    }
}

/// Render the model comparison view.
///
/// Columns: Model | Cost | Tier | Gate% | Uses | Avg | In | Out
///
/// Highlighting: the row with the lowest cost and the row with the highest
/// gate rate each get BOLD on the relevant cell so operators can spot the
/// best option per metric at a glance.
fn render_model_comparison(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            " Model Comparison ",
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if data.efficiency_events.is_empty() && data.cascade_router.model_slugs.is_empty() {
        let msg = Paragraph::new("no model data -- run agents to populate")
            .style(theme.muted())
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(msg, center_rect(inner));
        return;
    }

    if inner.height < 4 || inner.width < 40 {
        return;
    }

    // Aggregate per model. Use BTreeMap for deterministic order before sort.
    let mut model_map: std::collections::BTreeMap<String, ModelComparisonAgg> =
        std::collections::BTreeMap::new();

    for event in &data.efficiency_events {
        let slug = if event.model.is_empty() {
            "unknown".to_string()
        } else {
            event.model.clone()
        };
        let entry = model_map.entry(slug.clone()).or_insert_with(|| ModelComparisonAgg {
            slug,
            total_cost: 0.0,
            total_input: 0,
            total_output: 0,
            event_count: 0,
            passed_count: 0,
            total_wall_ms: 0,
            trials: 0,
            successes: 0,
        });
        entry.total_cost += event.cost_usd;
        entry.total_input += event.input_tokens;
        entry.total_output += event.output_tokens;
        entry.event_count += 1;
        if event.gate_passed {
            entry.passed_count += 1;
        }
        entry.total_wall_ms += event.wall_time_ms;
    }

    // Merge cascade router stats (fills in trial data for models not in efficiency events)
    for slug in &data.cascade_router.model_slugs {
        if let Some(stats) = data.cascade_router.confidence_stats.get(slug) {
            let entry = model_map.entry(slug.clone()).or_insert_with(|| ModelComparisonAgg {
                slug: slug.clone(),
                total_cost: 0.0,
                total_input: 0,
                total_output: 0,
                event_count: 0,
                passed_count: 0,
                total_wall_ms: 0,
                trials: 0,
                successes: 0,
            });
            entry.trials = stats.trials;
            entry.successes = stats.successes;
        }
    }

    // Sort: highest total_cost first (most expensive at top)
    let mut models: Vec<&ModelComparisonAgg> = model_map.values().collect();
    models.sort_by(|a, b| {
        b.total_cost
            .partial_cmp(&a.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Identify "best" rows for per-column highlighting.
    // best_cost_idx: model with lowest cost that has at least one event
    // best_gate_idx: model with highest gate rate that has at least one event
    let best_cost_idx = models
        .iter()
        .enumerate()
        .filter(|(_, m)| m.event_count > 0)
        .min_by(|(_, a), (_, b)| {
            a.total_cost
                .partial_cmp(&b.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i);

    let best_gate_idx = models
        .iter()
        .enumerate()
        .filter(|(_, m)| m.event_count > 0)
        .max_by(|(_, a), (_, b)| {
            a.gate_rate()
                .unwrap_or(0.0)
                .partial_cmp(&b.gate_rate().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i);

    let rows: Vec<Row<'_>> = models
        .iter()
        .enumerate()
        .map(|(idx, m)| {
            let is_best_cost = best_cost_idx == Some(idx);
            let is_best_gate = best_gate_idx == Some(idx);

            // Cost cell
            let cost_str = format!("${:.4}", m.total_cost);
            let cost_base = if m.total_cost > 1.0 {
                theme.danger()
            } else if m.total_cost > 0.1 {
                theme.warning()
            } else {
                theme.text()
            };
            let cost_style = if is_best_cost {
                // Bold green to indicate lowest cost
                theme.success().add_modifier(Modifier::BOLD)
            } else {
                cost_base
            };

            // Gate% cell
            let (gate_str, gate_base) = match m.gate_rate() {
                None => ("\u{2014}".to_string(), theme.muted()),
                Some(r) if r >= 0.8 => (format!("{:.0}%", r * 100.0), theme.success()),
                Some(r) if r >= 0.5 => (format!("{:.0}%", r * 100.0), theme.warning()),
                Some(r) => (format!("{:.0}%", r * 100.0), theme.danger()),
            };
            let gate_style = if is_best_gate {
                gate_base.add_modifier(Modifier::BOLD)
            } else {
                gate_base
            };

            let avg_str = m.avg_wall_ms().map_or(
                "\u{2014}".to_string(),
                |ms| format!("{ms:.0}ms"),
            );

            let tier = infer_tier(&m.slug);

            Row::new(vec![
                Cell::from(Span::styled(truncate(&m.slug, 22), theme.text())),
                Cell::from(Span::styled(cost_str, cost_style)),
                Cell::from(Span::styled(tier, theme.muted())),
                Cell::from(Span::styled(gate_str, gate_style)),
                Cell::from(Span::styled(m.event_count.to_string(), theme.text())),
                Cell::from(Span::styled(avg_str, theme.muted())),
                Cell::from(Span::styled(format_count(m.total_input), theme.muted())),
                Cell::from(Span::styled(format_count(m.total_output), theme.muted())),
            ])
        })
        .collect();

    // Fixed column widths for stable alignment
    // Model (22 min) | Cost (9) | Tier (8) | Gate% (6) | Uses (5) | Avg (8) | In (7) | Out (7)
    let widths = [
        Constraint::Min(14),
        Constraint::Length(9),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(5),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(7),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(["Model", "Cost", "Tier", "Gate%", "Uses", "Avg", "In", "Out"])
                .style(theme.accent().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1);
    frame.render_widget(table, inner);
}

// ---------------------------------------------------------------------------
// Shared helpers for config sub-views
// ---------------------------------------------------------------------------

/// Infer the human-readable provider name from a model slug.
fn infer_provider(slug: &str) -> &'static str {
    if slug.contains("claude") {
        "Anthropic"
    } else if slug.contains("gpt") || slug.contains("o1") || slug.contains("o3") {
        "OpenAI"
    } else if slug.contains("gemini") {
        "Google"
    } else if slug.contains("ollama") || slug.contains("llama") {
        "Ollama"
    } else {
        "Other"
    }
}

/// Infer a tier label from a model slug.
fn infer_tier(slug: &str) -> &'static str {
    if slug.contains("opus") {
        "deep"
    } else if slug.contains("sonnet") || slug.contains("gpt-4") {
        "balanced"
    } else if slug.contains("haiku") || slug.contains("gpt-3") {
        "fast"
    } else {
        "auto"
    }
}

/// Return a small centered rect for empty-state messages.
fn center_rect(area: Rect) -> Rect {
    let vert = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    vert[1]
}

/// Format a token count using K/M suffixes.
fn format_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Truncate a string to at most `max` bytes, respecting UTF-8 char boundaries.
fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max.saturating_sub(1);
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
```

### Step 3: Update input.rs for sub-view navigation

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs`

Search the file for existing sub-view key handling. If number keys generically map to
`SwitchSubView` for all tabs, no changes are needed.

If sub-view key handling is tab-specific, add to the F6 Config arm:

```rust
KeyCode::Char('1') if current_tab == Tab::Config => Some(TuiAction::SwitchSubView(0)),
KeyCode::Char('2') if current_tab == Tab::Config => Some(TuiAction::SwitchSubView(1)),
KeyCode::Char('3') if current_tab == Tab::Config => Some(TuiAction::SwitchSubView(2)),
```

Check that `SwitchSubView(n)` sets `view_state.sub_tab = n` in the action handler.

### Step 4: Verify

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Check that:
1. F6 tab still shows config editor by default (sub_tab 0)
2. Pressing 2 on F6 shows provider health, 3 shows model comparison
3. Provider health columns: Provider | Status | Model | Trials | Rate
   - Status color: green=healthy, amber=degraded, red=failing, ghost=idle
   - Rate column bold on rendered value
4. Model comparison columns: Model | Cost | Tier | Gate% | Uses | Avg | In | Out
   - Lowest-cost model has its Cost cell highlighted in bold green
   - Highest gate-rate model has its Gate% cell in bold
5. Both sub-views render a centered empty-state line when data is absent
6. No column overflow at 80-column terminal width

## Column layout reference

### Provider Health (sub_tab 1)

| Column   | Width | Content                              |
|----------|-------|--------------------------------------|
| Provider | 10    | Anthropic / OpenAI / Google / Ollama |
| Status   | 8     | healthy / degraded / failing / idle  |
| Model    | 16+   | model slug (truncated at 24)         |
| Trials   | 6     | trial count or em-dash               |
| Rate     | 6     | success% or em-dash (bold)           |

Color mapping:
- `>= 80%` success -- `theme.success()` (SAGE green)
- `>= 50%` success -- `theme.warning()` (amber)
- `<  50%` success -- `theme.danger()` (EMBER red)
- `0` trials      -- `theme.muted()` (ghost)

### Model Comparison (sub_tab 2)

| Column | Width | Content                           |
|--------|-------|-----------------------------------|
| Model  | 14+   | model slug (truncated at 22)      |
| Cost   | 9     | $X.XXXX (bold green if cheapest)  |
| Tier   | 8     | deep / balanced / fast / auto     |
| Gate%  | 6     | pass% (bold if best gate rate)    |
| Uses   | 5     | event count                       |
| Avg    | 8     | avg wall_time_ms or em-dash       |
| In     | 7     | total input tokens (K/M suffix)   |
| Out    | 7     | total output tokens (K/M suffix)  |

Best-per-column highlighting (BOLD):
- **Cost**: lowest-cost model gets `theme.success().add_modifier(Modifier::BOLD)` on Cost cell
- **Gate%**: highest gate-rate model gets its gate_style with `add_modifier(Modifier::BOLD)` on Gate% cell

## Acceptance criteria

- [ ] `render()` in config_view.rs dispatches on `view_state.sub_tab`:
      0 = config editor, 1 = provider health, 2 = model comparison
- [ ] Config editor (sub_tab 0) works identically to before -- body unchanged
- [ ] `render_provider_health()` reads from `data.cascade_router`, renders
      Provider / Status / Model / Trials / Rate with fixed column widths
- [ ] Provider status green/amber/red/ghost per success rate threshold
- [ ] Em-dash used for "no data" cells (not "--")
- [ ] `render_model_comparison()` aggregates from `data.efficiency_events` +
      `data.cascade_router`, renders Model / Cost / Tier / Gate% / Uses / Avg / In / Out
- [ ] Lowest-cost model has bold green Cost cell
- [ ] Highest gate-rate model has bold Gate% cell
- [ ] Both sub-views render a centered empty-state message when data is absent
- [ ] `infer_provider()` and `infer_tier()` are standalone private helpers (not inline)
- [ ] `truncate()`, `format_count()`, `center_rect()` are private helpers in the file
- [ ] Number keys 1/2/3 switch sub-views on F6 tab
- [ ] `cargo clippy` clean, `cargo +nightly fmt` clean
- [ ] All existing tests pass
