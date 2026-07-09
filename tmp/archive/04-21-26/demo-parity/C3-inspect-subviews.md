# C3: Implement F7 Inspect sub-views

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
- After C1: Dashboard(F1) through Atelier(F9), `ALL: [Tab; 9]`

**SubView enum** at `crates/roko-cli/src/tui/views/mod.rs`:
- F7: `EngramDag`, `EpisodeReplay`, `KnowledgeBrowse`

**Key ViewState fields** (in state.rs):
- `sub_tab: usize` -- which sub-view within a tab (0 = overview)
- `selected: usize` -- list selection index
- `scroll: usize` -- scroll offset for scrollable content

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Goal

The F7 Inspect tab currently renders a single overview layout. Three sub-views are defined in the `SubView` enum (`EngramDag`, `EpisodeReplay`, `KnowledgeBrowse`) but are not wired. This task adds rendering code for all three and dispatches them based on `view_state.sub_tab`.

**Sub-view index mapping:**
| sub_tab | View |
|---------|------|
| 0 | Overview (existing layout, unchanged) |
| 1 | Engram DAG |
| 2 | Episode Replay |
| 3 | Knowledge Browse |

**EngramDag:** hashes truncated to 8 chars. ASCII tree connectors. Confidence bar with filled/empty blocks.

**EpisodeReplay:** includes timing column (`wall_time_ms`). Shows `✓`/`✗` gate icon.

**KnowledgeBrowse:** case-insensitive search via `view_state.search_query` (if the field exists). All three sub-views render a centered empty-state message when no data is available.

**Audit update (2026-04-22):** the sub-views render, but Knowledge Browse is still a simple unfiltered list and does not yet satisfy the search/filter/scroll acceptance criteria below.

- [ ] Add case-insensitive Knowledge Browse filtering through a real `ViewState` search field and make selection/scroll behavior explicit for all three Inspect sub-views.

## Dependency

None. The SubView variants already exist. `context_view.rs` already exists with a working `render()` function.

## Steps

### Step 1: Read existing code

Read these files first:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/context_view.rs` -- understand the existing render function
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/mod.rs` -- confirm SubView variants exist
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs` -- fields: `recent_signals`, `efficiency_events`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` -- ViewState field names

Confirm the exact field names in `SignalSummary` and `AgentEfficiencyEvent` before writing code.

### Step 2: Modify context_view.rs

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/context_view.rs`

**2a. Add `std::path::Path` to imports** (if not already imported):

Find:
```rust
use std::collections::{BTreeMap, HashMap};
```

Replace with:
```rust
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
```

**2b. Replace the `render()` function to dispatch on sub_tab:**

Find the existing `render()` function. Wrap its body in a `sub_tab == 0` branch and add dispatch for sub-views 1-3:

```rust
/// Render the full context/inspect view with sub-view dispatch.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    match view_state.sub_tab {
        1 => render_engram_dag(frame, area, data, view_state, theme),
        2 => render_episode_replay(frame, area, data, view_state, theme),
        3 => render_knowledge_browse(frame, area, data, view_state, theme),
        _ => {
            // sub_tab 0 (or any unknown value) = Overview -- existing layout.
            let ctx_data = build_context_data(data);
            render_with_context_data(
                frame,
                area,
                data,
                &ctx_data,
                view_state,
                theme,
                matches!(tui_state.focus, FocusZone::RightPanel),
            );
        }
    }
}
```

**2c. Add the three sub-view render functions.** Insert before `build_context_data()`:

```rust
// ===========================================================================
// Sub-view 1: Engram DAG
// ===========================================================================

/// Render the engram/signal DAG as an indented ASCII tree.
///
/// Empty state: centered message if `data.recent_signals` is empty.
/// Scrollable via `view_state.scroll`.
/// Hash displayed as first 8 characters.
fn render_engram_dag(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    view_state: &ViewState,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            " Engram DAG ",
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if data.recent_signals.is_empty() {
        frame.render_widget(
            Paragraph::new("no signals -- run agents to generate engrams")
                .style(theme.muted())
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let visible_height = inner.height as usize;
    let total = data.recent_signals.len();
    let effective_scroll = view_state.scroll.min(total.saturating_sub(visible_height));

    let lines: Vec<Line<'_>> = data
        .recent_signals
        .iter()
        .enumerate()
        .skip(effective_scroll)
        .take(visible_height)
        .map(|(i, signal)| {
            // Nested indent based on position (visual approximation of DAG depth).
            let depth = i % 4;
            let indent = "  ".repeat(depth);
            let connector = if depth == 0 { "\u{2500} " } else { "\u{2514}\u{2500} " };

            let hash_display = if signal.hash_short.len() >= 8 {
                &signal.hash_short[..8]
            } else {
                &signal.hash_short
            };

            let conf_bar = confidence_bar(signal.confidence, 6);
            let conf_style = confidence_style(signal.confidence, theme);

            let kind_style = match signal.kind.as_str() {
                k if k.starts_with("gate:") => theme.info(),
                k if k.starts_with("conductor:") => theme.warning(),
                k if k.starts_with("agent:") => theme.success(),
                _ => theme.text(),
            };

            Line::from(vec![
                Span::styled(format!("{indent}{connector}"), theme.muted()),
                Span::styled(format!("{hash_display} "), theme.muted()),
                Span::styled(truncate(&signal.kind, 24), kind_style),
                Span::raw(" "),
                Span::styled(conf_bar, conf_style),
                Span::styled(format!(" {:.2}", signal.confidence), conf_style),
                Span::styled(
                    format!("  {}", truncate(&signal.source, 16)),
                    theme.muted(),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

fn confidence_bar(confidence: f64, width: usize) -> String {
    let filled = (confidence.clamp(0.0, 1.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "{}{}",
        "\u{2588}".repeat(filled), // █
        "\u{2500}".repeat(empty),  // ─
    )
}

fn confidence_style(confidence: f64, theme: &Theme) -> ratatui::style::Style {
    if confidence >= 0.8 {
        theme.success()
    } else if confidence >= 0.5 {
        theme.warning()
    } else {
        theme.danger()
    }
}

// ===========================================================================
// Sub-view 2: Episode Replay
// ===========================================================================

/// Episode entry loaded from `.roko/episodes.jsonl`.
#[derive(Debug, Clone, Default)]
struct EpisodeEntry {
    agent_id: String,
    role: String,
    task_id: String,
    model: String,
    turns: usize,
    gate_passed: bool,
    cost_usd: f64,
    /// Wall-clock time for the episode in milliseconds.
    wall_time_ms: u64,
    timestamp: String,
}

fn load_episodes(root: &Path) -> Vec<EpisodeEntry> {
    // Check both canonical paths.
    let candidates = [
        root.join(".roko").join("episodes.jsonl"),
        root.join(".roko").join("memory").join("episodes.jsonl"),
    ];

    let path = candidates.iter().find(|p| p.exists())?;
    let content = std::fs::read_to_string(path).ok()?;

    Some(
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| {
                let v: serde_json::Value = serde_json::from_str(line).ok()?;
                Some(EpisodeEntry {
                    agent_id: v["agent_id"].as_str().unwrap_or("").to_string(),
                    role: v["role"].as_str().unwrap_or("").to_string(),
                    task_id: v["task_id"].as_str().unwrap_or("").to_string(),
                    model: v["model"].as_str().unwrap_or("").to_string(),
                    turns: v["turns"].as_u64().unwrap_or(0) as usize,
                    gate_passed: v["gate_passed"].as_bool().unwrap_or(false),
                    cost_usd: v["cost_usd"].as_f64().unwrap_or(0.0),
                    wall_time_ms: v["wall_time_ms"].as_u64().unwrap_or(0),
                    timestamp: v["timestamp"].as_str().unwrap_or("").to_string(),
                })
            })
            .collect(),
    )
    .unwrap_or_default()
}

fn load_episodes(root: &Path) -> Vec<EpisodeEntry> {
    // Wrapper to handle the Option pattern cleanly.
    load_episodes_inner(root).unwrap_or_default()
}

fn load_episodes_inner(root: &Path) -> Option<Vec<EpisodeEntry>> {
    let candidates = [
        root.join(".roko").join("episodes.jsonl"),
        root.join(".roko").join("memory").join("episodes.jsonl"),
    ];
    let path = candidates.iter().find(|p| p.exists())?;
    let content = std::fs::read_to_string(path).ok()?;
    Some(
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| {
                let v: serde_json::Value = serde_json::from_str(line).ok()?;
                Some(EpisodeEntry {
                    agent_id: v["agent_id"].as_str().unwrap_or("").to_string(),
                    role: v["role"].as_str().unwrap_or("").to_string(),
                    task_id: v["task_id"].as_str().unwrap_or("").to_string(),
                    model: v["model"].as_str().unwrap_or("").to_string(),
                    turns: v["turns"].as_u64().unwrap_or(0) as usize,
                    gate_passed: v["gate_passed"].as_bool().unwrap_or(false),
                    cost_usd: v["cost_usd"].as_f64().unwrap_or(0.0),
                    wall_time_ms: v["wall_time_ms"].as_u64().unwrap_or(0),
                    timestamp: v["timestamp"].as_str().unwrap_or("").to_string(),
                })
            })
            .collect(),
    )
}

/// Render the episode replay viewer.
///
/// Columns: gate icon, task id, role, model, turns, cost, wall_time.
/// Empty state: centered message.
/// Scrollable via `view_state.selected`.
fn render_episode_replay(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    view_state: &ViewState,
    theme: &Theme,
) {
    let episodes = load_episodes(data.root());

    let block = Block::bordered()
        .title(Span::styled(
            format!(" Episode Replay ({}) ", episodes.len()),
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if episodes.is_empty() {
        frame.render_widget(
            Paragraph::new("no episodes recorded -- run agents to generate")
                .style(theme.muted())
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    if inner.height < 4 || inner.width < 30 {
        return;
    }

    let sections = Layout::vertical([
        Constraint::Length(2), // Summary bar
        Constraint::Min(0),    // Episode table
    ])
    .split(inner);

    // Summary bar (single pass over episodes).
    let total = episodes.len();
    let passed = episodes.iter().filter(|e| e.gate_passed).count();
    let total_cost: f64 = episodes.iter().map(|e| e.cost_usd).sum();
    let total_turns: usize = episodes.iter().map(|e| e.turns).sum();
    let avg_turns = if total > 0 { total_turns / total } else { 0 };
    let total_ms: u64 = episodes.iter().map(|e| e.wall_time_ms).sum();
    let avg_ms = if total > 0 { total_ms / total as u64 } else { 0 };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("total: ", theme.muted()),
            Span::styled(total.to_string(), theme.text()),
            Span::styled("  pass: ", theme.muted()),
            Span::styled(passed.to_string(), theme.success()),
            Span::styled("  fail: ", theme.muted()),
            Span::styled(
                (total - passed).to_string(),
                if total > passed { theme.danger() } else { theme.muted() },
            ),
            Span::styled("  cost: ", theme.muted()),
            Span::styled(format!("${total_cost:.3}"), theme.warning()),
            Span::styled("  avg turns: ", theme.muted()),
            Span::styled(avg_turns.to_string(), theme.text()),
            Span::styled("  avg time: ", theme.muted()),
            Span::styled(format!("{avg_ms}ms"), theme.muted()),
        ]))
        .wrap(Wrap { trim: false }),
        sections[0],
    );

    // Episode table.
    let visible = sections[1].height as usize;
    let selected = view_state.selected.min(episodes.len().saturating_sub(1));
    let scroll = if selected >= visible { selected - visible + 1 } else { 0 };

    let rows: Vec<Row<'_>> = episodes
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible.saturating_sub(1))
        .map(|(i, ep)| {
            let is_sel = i == selected;
            let gate_icon = if ep.gate_passed { "\u{2713}" } else { "\u{2717}" }; // ✓ ✗
            let gate_style = if ep.gate_passed { theme.success() } else { theme.danger() };
            let row_style = if is_sel { theme.selection() } else { theme.text() };

            Row::new(vec![
                Cell::from(Span::styled(gate_icon, gate_style)),
                Cell::from(Span::styled(truncate(&ep.task_id, 10), row_style)),
                Cell::from(Span::styled(truncate(&ep.role, 12), theme.muted())),
                Cell::from(Span::styled(truncate(&ep.model, 16), theme.muted())),
                Cell::from(Span::styled(ep.turns.to_string(), theme.text())),
                Cell::from(Span::styled(format!("${:.3}", ep.cost_usd), theme.warning())),
                Cell::from(Span::styled(format!("{}ms", ep.wall_time_ms), theme.muted())),
            ])
        })
        .collect();

    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(16),
                Constraint::Length(5),
                Constraint::Length(8),
                Constraint::Length(8),
            ],
        )
        .header(
            Row::new(["", "task", "role", "model", "turns", "cost", "time"])
                .style(theme.accent().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1),
        sections[1],
    );
}

// ===========================================================================
// Sub-view 3: Knowledge Browse
// ===========================================================================

#[derive(Debug, Clone, Default)]
struct KnowledgeEntry {
    topic: String,
    content_preview: String,
    confidence: f64,
    source: String,
}

fn load_knowledge(root: &Path) -> Vec<KnowledgeEntry> {
    let mut entries = Vec::new();

    // Primary: .roko/neuro/knowledge.jsonl
    let neuro_path = root.join(".roko").join("neuro").join("knowledge.jsonl");
    if let Ok(content) = std::fs::read_to_string(&neuro_path) {
        for line in content.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                entries.push(KnowledgeEntry {
                    topic: v["topic"]
                        .as_str()
                        .or_else(|| v["key"].as_str())
                        .unwrap_or("")
                        .to_string(),
                    content_preview: v["content"]
                        .as_str()
                        .or_else(|| v["value"].as_str())
                        .or_else(|| v["summary"].as_str())
                        .unwrap_or("")
                        .chars()
                        .take(120)
                        .collect(),
                    confidence: v["confidence"].as_f64().unwrap_or(0.5),
                    source: v["source"].as_str().unwrap_or("neuro").to_string(),
                });
            }
        }
    }

    // Secondary: scan .roko/memory/ for JSONL/JSON files (except episodes.jsonl).
    let memory_dir = root.join(".roko").join("memory");
    if memory_dir.is_dir() {
        if let Ok(dir_entries) = std::fs::read_dir(&memory_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext != "jsonl" && ext != "json" {
                    continue;
                }
                let stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                if stem == "episodes" {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let line_count = content.lines().filter(|l| !l.trim().is_empty()).count();
                    entries.push(KnowledgeEntry {
                        topic: stem,
                        content_preview: format!("{line_count} entries"),
                        confidence: 1.0,
                        source: "memory".to_string(),
                    });
                }
            }
        }
    }

    // Sort by confidence descending.
    entries.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    entries
}

/// Render the knowledge browser.
///
/// Filters entries by `view_state.search_query` (case-insensitive topic match),
/// if that field exists on ViewState. If not available, renders all entries.
/// Empty state: centered multi-line message.
fn render_knowledge_browse(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    view_state: &ViewState,
    theme: &Theme,
) {
    let all_knowledge = load_knowledge(data.root());

    // Case-insensitive filter via search_query (if the field exists on ViewState).
    // MOCK: Replace with actual ViewState::search_query field access when available.
    let knowledge: Vec<&KnowledgeEntry> = all_knowledge.iter().collect();

    let block = Block::bordered()
        .title(Span::styled(
            format!(" Knowledge ({}) ", knowledge.len()),
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if knowledge.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("No knowledge entries found.", theme.muted())),
            Line::from(""),
            Line::from(Span::styled(
                "Knowledge accumulates as agents execute tasks.",
                theme.muted(),
            )),
            Line::from(Span::styled(
                "Sources: .roko/neuro/knowledge.jsonl  .roko/memory/",
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

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    let visible = inner.height as usize;
    let selected = view_state.selected.min(knowledge.len().saturating_sub(1));
    let scroll = if selected >= visible { selected - visible + 1 } else { 0 };

    let content_max = (inner.width as usize).saturating_sub(40);
    let rows: Vec<Row<'_>> = knowledge
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible.saturating_sub(1))
        .map(|(i, entry)| {
            let is_sel = i == selected;
            let conf_style = confidence_style(entry.confidence, theme);
            let row_style = if is_sel { theme.selection() } else { theme.text() };

            Row::new(vec![
                Cell::from(Span::styled(truncate(&entry.topic, 20), row_style)),
                Cell::from(Span::styled(confidence_bar(entry.confidence, 5), conf_style)),
                Cell::from(Span::styled(truncate(&entry.source, 10), theme.muted())),
                Cell::from(Span::styled(
                    truncate(&entry.content_preview, content_max),
                    theme.muted(),
                )),
            ])
        })
        .collect();

    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(20),
                Constraint::Length(5),
                Constraint::Length(10),
                Constraint::Min(10),
            ],
        )
        .header(
            Row::new(["topic", "conf", "source", "preview"])
                .style(theme.accent().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1),
        inner,
    );
}

fn truncate(s: &str, max: usize) -> String {
    if max < 4 || s.len() <= max {
        return s.to_string();
    }
    let mut end = max - 3;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}
```

**NOTE about duplicate `load_episodes` function:** The code block above accidentally defines `load_episodes` twice. Replace both with the `load_episodes_inner` helper pattern:

```rust
fn load_episodes(root: &Path) -> Vec<EpisodeEntry> {
    fn inner(root: &Path) -> Option<Vec<EpisodeEntry>> {
        let candidates = [
            root.join(".roko").join("episodes.jsonl"),
            root.join(".roko").join("memory").join("episodes.jsonl"),
        ];
        let path = candidates.iter().find(|p| p.exists())?;
        let content = std::fs::read_to_string(path).ok()?;
        Some(
            content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|line| {
                    let v: serde_json::Value = serde_json::from_str(line).ok()?;
                    Some(EpisodeEntry {
                        agent_id: v["agent_id"].as_str().unwrap_or("").to_string(),
                        role: v["role"].as_str().unwrap_or("").to_string(),
                        task_id: v["task_id"].as_str().unwrap_or("").to_string(),
                        model: v["model"].as_str().unwrap_or("").to_string(),
                        turns: v["turns"].as_u64().unwrap_or(0) as usize,
                        gate_passed: v["gate_passed"].as_bool().unwrap_or(false),
                        cost_usd: v["cost_usd"].as_f64().unwrap_or(0.0),
                        wall_time_ms: v["wall_time_ms"].as_u64().unwrap_or(0),
                        timestamp: v["timestamp"].as_str().unwrap_or("").to_string(),
                    })
                })
                .collect(),
        )
    }
    inner(root).unwrap_or_default()
}
```

### Step 3: Update input.rs for sub-view navigation

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs`

Read the file. Search for `SwitchSubView` handling to understand if it already generically maps number keys to sub-view indices for all tabs. If it does, no changes are needed.

If number key handling is tab-specific, add:
```rust
// In Normal mode key handling, for Tab::Inspect:
KeyCode::Char('1') if current_tab == Tab::Inspect => Some(TuiAction::SwitchSubView(0)),
KeyCode::Char('2') if current_tab == Tab::Inspect => Some(TuiAction::SwitchSubView(1)),
KeyCode::Char('3') if current_tab == Tab::Inspect => Some(TuiAction::SwitchSubView(2)),
KeyCode::Char('4') if current_tab == Tab::Inspect => Some(TuiAction::SwitchSubView(3)),
```

Find where `TuiAction::SwitchSubView` is handled in the action processor (likely `app.rs`). Ensure it sets `view_state.sub_tab` to the given index.

### Step 4: Verify

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Check that:
1. F7 tab still shows the overview layout at sub_tab 0 (pressing 1)
2. Pressing 2 on F7 shows Engram DAG with 8-char truncated hashes and confidence bars
3. Pressing 3 on F7 shows Episode Replay with timing column
4. Pressing 4 on F7 shows Knowledge Browse
5. All three sub-views show centered empty-state messages when no data exists

## Acceptance criteria

- [ ] `render()` in context_view.rs dispatches on `view_state.sub_tab` (0=overview, 1=engram, 2=episode, 3=knowledge)
- [ ] Overview (sub_tab 0) renders unchanged
- [ ] `render_engram_dag()` renders with 8-char truncated hashes, ASCII tree connectors, confidence bars
- [ ] `render_episode_replay()` renders with gate icon, task, role, model, turns, cost, `wall_time_ms` column
- [ ] `render_knowledge_browse()` renders with topic, confidence bar, source, content preview
- [ ] All three sub-views show informative centered empty-state message when no data
- [ ] Case-insensitive filtering in KnowledgeBrowse (via `view_state.search_query` or MOCK comment)
- [ ] `confidence_bar()` and `confidence_style()` are shared by both engram and knowledge sub-views
- [ ] Scrolling via `view_state.scroll` / `view_state.selected` works in all sub-views
- [ ] Number keys 1-4 switch sub-views on F7 tab
- [ ] `Block::bordered()` used throughout
- [ ] `cargo clippy` clean, `cargo +nightly fmt` clean
- [ ] All existing tests pass
