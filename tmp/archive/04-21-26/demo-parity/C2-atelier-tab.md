# C2: Add F9 Atelier tab to TUI

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
- After C1: Dashboard(F1), Plans(F2), Agents(F3), Git(F4), Logs(F5), Config(F6), Inspect(F7), Marketplace(F8), Atelier(F9)
- `ALL: [Tab; 9]`

**SubView enum** at `crates/roko-cli/src/tui/views/mod.rs`:
- F9: PrdWorkshop, PlanExplorer

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Goal

Replace the atelier placeholder from C1 with a full PRD-and-plan workshop view. The Atelier shows PRD lifecycle status with Unicode status badges, plan execution progress, and task breakdowns -- all loaded from local files.

**Status badge mapping:**
| Status | Badge | Style |
|--------|-------|-------|
| idea | `IDEA` | muted |
| draft | `DRFT` | warning/amber |
| published | `PUBL` | success/green |
| planned | `PLAN` | info/blue |

**Task status icons:**
| State | Icon |
|-------|------|
| pending | `[ ]` |
| running | `[>]` |
| done | `[x]` |
| failed | `[!]` |

## Dependency

C1 must be completed first. C1 creates the `Tab::Atelier` variant, updates `Tab::ALL` to `[Tab; 9]`, adds the `atelier_view` module declaration in `views/mod.rs`, and creates the placeholder file.

**CHECK:** Read `crates/roko-cli/src/tui/tabs.rs` and confirm `Tab::Atelier` exists and `ALL` has 9 entries. If not, apply all tab changes from C1 first.

## Steps

### Step 1: Verify prerequisites

Read these files first:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/tabs.rs` -- confirm `Atelier` variant exists
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/mod.rs` -- confirm `pub mod atelier_view;` exists
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/atelier_view.rs` -- read the placeholder

### Step 2: Replace the atelier view

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/atelier_view.rs`

Replace the entire file with:

```rust
//! F9 Atelier view -- PRD and plan workshop.
//!
//! Layout: top 3-line stats bar + left 40% (PRD list) + right 60% (plan detail).
//!
//! Data sources:
//!   - PRDs: `.roko/prd/` directory (YAML front-matter + markdown body)
//!   - Plan tasks: `plans/<slug>/tasks.toml` or `.roko/plans/<slug>/tasks.toml`
//!   - Agent/episode counts: `DashboardData` (already populated by file-poll)

use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::state::TuiState;

// ---------------------------------------------------------------------------
// PRD data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct PrdEntry {
    slug: String,
    title: String,
    status: PrdStatus,
    plan_count: usize,
    task_total: usize,
    task_done: usize,
    task_failed: usize,
    path: PathBuf,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum PrdStatus {
    #[default]
    Idea,
    Draft,
    Published,
    Planned,
}

impl PrdStatus {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "published" | "publish" => Self::Published,
            "draft" => Self::Draft,
            "planned" | "plan" => Self::Planned,
            _ => Self::Idea,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Idea => "idea",
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Planned => "planned",
        }
    }

    /// Four-character Unicode badge shown in the PRD list.
    fn badge(self) -> &'static str {
        match self {
            Self::Idea => "IDEA",
            Self::Draft => "DRFT",
            Self::Published => "PUBL",
            Self::Planned => "PLAN",
        }
    }

    /// Sort weight: lower = more urgent / active.
    fn weight(self) -> u8 {
        match self {
            Self::Planned => 0,
            Self::Published => 1,
            Self::Draft => 2,
            Self::Idea => 3,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct TaskEntry {
    id: String,
    title: String,
    status: TaskState,
    agent: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum TaskState {
    #[default]
    Pending,
    Running,
    Done,
    Failed,
}

impl TaskState {
    fn icon(self) -> &'static str {
        match self {
            Self::Pending => "[ ]",
            Self::Running => "[>]",
            Self::Done => "[x]",
            Self::Failed => "[!]",
        }
    }

    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "done" | "completed" | "passed" | "skipped" => Self::Done,
            "running" | "active" | "executing" | "in_progress" | "implementing"
            | "gating" | "verifying" | "reviewing" => Self::Running,
            "failed" | "error" | "gate_rejected" => Self::Failed,
            _ => Self::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// Data loading
// ---------------------------------------------------------------------------

fn load_prds(root: &Path) -> Vec<PrdEntry> {
    let prd_dir = root.join(".roko").join("prd");
    let Ok(entries) = std::fs::read_dir(&prd_dir) else {
        return Vec::new();
    };

    let mut prds: Vec<PrdEntry> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let slug = e.file_name().to_string_lossy().into_owned();
            let dir_path = e.path();
            let content = read_prd_content(&dir_path, &slug);
            let title = extract_title(&content, &slug);
            let status = extract_status(&content);
            Some(PrdEntry {
                slug,
                title,
                status,
                path: dir_path,
                ..Default::default()
            })
        })
        .collect();

    // Enrich with plan/task counts.
    let plans_root = if root.join("plans").exists() {
        root.join("plans")
    } else {
        root.join(".roko").join("plans")
    };

    for prd in &mut prds {
        let plan_path = plans_root.join(&prd.slug);
        if plan_path.exists() {
            let tasks = load_tasks_for_plan(&plan_path);
            prd.plan_count = 1;
            prd.task_total = tasks.len();
            prd.task_done = tasks.iter().filter(|t| t.status == TaskState::Done).count();
            prd.task_failed = tasks.iter().filter(|t| t.status == TaskState::Failed).count();
            // Upgrade status to Planned if a plan exists.
            if prd.status == PrdStatus::Published {
                prd.status = PrdStatus::Planned;
            }
        }
    }

    // Sort: active/planned first, then alphabetical by slug.
    prds.sort_by(|a, b| {
        a.status
            .weight()
            .cmp(&b.status.weight())
            .then(a.slug.cmp(&b.slug))
    });
    prds
}

fn read_prd_content(dir: &Path, slug: &str) -> String {
    // Try prd.md, then <slug>.md
    std::fs::read_to_string(dir.join("prd.md"))
        .or_else(|_| std::fs::read_to_string(dir.join(format!("{slug}.md"))))
        .unwrap_or_default()
}

fn extract_title(content: &str, fallback: &str) -> String {
    if content.starts_with("---") {
        for line in content.lines().skip(1) {
            if line.starts_with("---") {
                break;
            }
            if let Some(rest) = line.strip_prefix("title:") {
                let t = rest.trim().trim_matches('"').trim_matches('\'');
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("# ") {
            return heading.trim().to_string();
        }
    }
    fallback.replace('-', " ")
}

fn extract_status(content: &str) -> PrdStatus {
    if content.starts_with("---") {
        for line in content.lines().skip(1) {
            if line.starts_with("---") {
                break;
            }
            if let Some(rest) = line.strip_prefix("status:") {
                return PrdStatus::from_str(rest.trim().trim_matches('"'));
            }
        }
    }
    PrdStatus::Idea
}

fn load_tasks_for_plan(plan_dir: &Path) -> Vec<TaskEntry> {
    let tasks_path = plan_dir.join("tasks.toml");
    let Ok(content) = std::fs::read_to_string(&tasks_path) else {
        return Vec::new();
    };

    let mut tasks = Vec::new();
    let mut cur_id = String::new();
    let mut cur_title = String::new();
    let mut cur_status = TaskState::Pending;
    let mut cur_agent = String::new();
    let mut in_task = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[[task]]") || trimmed.starts_with("[[tasks]]") {
            flush_task(
                &mut tasks,
                in_task,
                &cur_id,
                &cur_title,
                cur_status,
                &cur_agent,
            );
            in_task = true;
            cur_id.clear();
            cur_title.clear();
            cur_status = TaskState::Pending;
            cur_agent.clear();
        } else if in_task {
            if let Some(v) = extract_toml_str(trimmed, "id") {
                cur_id = v;
            } else if let Some(v) = extract_toml_str(trimmed, "title") {
                cur_title = v;
            } else if let Some(v) = extract_toml_str(trimmed, "name") {
                if cur_title.is_empty() {
                    cur_title = v;
                }
            } else if let Some(v) = extract_toml_str(trimmed, "status") {
                cur_status = TaskState::from_str(&v);
            } else if let Some(v) = extract_toml_str(trimmed, "agent") {
                cur_agent = v;
            }
        }
    }
    flush_task(&mut tasks, in_task, &cur_id, &cur_title, cur_status, &cur_agent);
    tasks
}

fn flush_task(
    tasks: &mut Vec<TaskEntry>,
    in_task: bool,
    id: &str,
    title: &str,
    status: TaskState,
    agent: &str,
) {
    if in_task && !id.is_empty() {
        tasks.push(TaskEntry {
            id: id.to_string(),
            title: title.to_string(),
            status,
            agent: agent.to_string(),
        });
    }
}

fn extract_toml_str(line: &str, key: &str) -> Option<String> {
    // Handles `key = "value"`, `key= "value"`, `key="value"`
    let line = line.trim();
    let rest = if line.starts_with(&format!("{key} = ")) {
        &line[key.len() + 3..]
    } else if line.starts_with(&format!("{key}=")) {
        &line[key.len() + 1..]
    } else {
        return None;
    };
    Some(rest.trim().trim_matches('"').trim_matches('\'').to_string())
}

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

/// Render the full atelier view.
///
/// Handles empty state and terminal resize via percentage constraints.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    _tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let prds = load_prds(data.root());

    let rows = Layout::vertical([
        Constraint::Length(3), // Stats bar
        Constraint::Min(0),    // Main content
    ])
    .split(area);

    render_stats_bar(frame, rows[0], &prds, data, theme);

    if prds.is_empty() {
        render_empty(frame, rows[1], theme);
        return;
    }

    let panels = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ])
    .split(rows[1]);

    let selected = view_state.selected.min(prds.len().saturating_sub(1));
    render_prd_list(frame, panels[0], &prds, selected, theme);
    render_plan_detail(frame, panels[1], &prds, selected, data, theme);
}

// ---------------------------------------------------------------------------
// Stats bar
// ---------------------------------------------------------------------------

fn render_stats_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    prds: &[PrdEntry],
    data: &DashboardData,
    theme: &Theme,
) {
    let block = Block::bordered().border_style(theme.muted());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let plan_count = prds.iter().filter(|p| p.plan_count > 0).count();
    let done_tasks: usize = prds.iter().map(|p| p.task_done).sum();
    let total_tasks: usize = prds.iter().map(|p| p.task_total).sum();

    let all_done = total_tasks > 0 && done_tasks == total_tasks;
    let tasks_style = if all_done { theme.success() } else { theme.text() };

    let cols = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ])
    .split(inner);

    let stat = |label: &str, value: String, style| {
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{label}: "), theme.muted()),
            Span::styled(value, style),
        ]))
        .alignment(Alignment::Center)
    };

    frame.render_widget(stat("PRDs", prds.len().to_string(), theme.text()), cols[0]);
    frame.render_widget(stat("Plans", plan_count.to_string(), theme.info()), cols[1]);
    frame.render_widget(
        stat("Tasks", format!("{done_tasks}/{total_tasks}"), tasks_style),
        cols[2],
    );
    frame.render_widget(
        stat("Agents", data.agents.len().to_string(), theme.text()),
        cols[3],
    );
    frame.render_widget(
        stat("Episodes", data.efficiency.event_count.to_string(), theme.muted()),
        cols[4],
    );
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------

fn render_empty(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let block = Block::bordered()
        .title(Span::styled(
            " Atelier ",
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("No PRDs found.", theme.muted())),
        Line::from(""),
        Line::from(Span::styled(
            "Create one with: roko prd idea \"your idea\"",
            theme.muted(),
        )),
        Line::from(Span::styled(
            "Then draft: roko prd draft new \"your-slug\"",
            theme.muted(),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        inner,
    );
}

// ---------------------------------------------------------------------------
// Left panel: PRD list
// ---------------------------------------------------------------------------

fn render_prd_list(
    frame: &mut Frame<'_>,
    area: Rect,
    prds: &[PrdEntry],
    selected: usize,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            format!(" PRDs ({}) ", prds.len()),
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 10 {
        return;
    }

    let visible_height = inner.height as usize;
    let scroll = if selected >= visible_height {
        selected - visible_height + 1
    } else {
        0
    };

    let items: Vec<ListItem<'_>> = prds
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, prd)| {
            let is_sel = i == selected;
            let badge_style = match prd.status {
                PrdStatus::Idea => theme.muted(),
                PrdStatus::Draft => theme.warning(),
                PrdStatus::Published => theme.success(),
                PrdStatus::Planned => theme.info(),
            };

            let progress = if prd.task_total > 0 {
                format!(" {}/{}", prd.task_done, prd.task_total)
            } else {
                String::new()
            };

            let title_max = (inner.width as usize).saturating_sub(12 + progress.len());
            let row_style = if is_sel { theme.selection() } else { theme.text() };

            let mut spans = vec![
                Span::styled(format!(" {} ", prd.status.badge()), badge_style),
                Span::styled(truncate(&prd.title, title_max), row_style),
            ];

            if !progress.is_empty() {
                let progress_style = if prd.task_failed > 0 {
                    theme.danger()
                } else if prd.task_done == prd.task_total {
                    theme.success()
                } else {
                    theme.muted()
                };
                spans.push(Span::styled(progress, progress_style));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

// ---------------------------------------------------------------------------
// Right panel: plan detail with task list
// ---------------------------------------------------------------------------

fn render_plan_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    prds: &[PrdEntry],
    selected: usize,
    data: &DashboardData,
    theme: &Theme,
) {
    let Some(prd) = prds.get(selected) else {
        return;
    };

    let block = Block::bordered()
        .title(Span::styled(
            format!(" {} ", truncate(&prd.title, 40)),
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 20 {
        return;
    }

    let sections = Layout::vertical([
        Constraint::Length(5), // PRD metadata
        Constraint::Min(0),    // Task list
    ])
    .split(inner);

    // PRD metadata
    let status_style = match prd.status {
        PrdStatus::Idea => theme.muted(),
        PrdStatus::Draft => theme.warning(),
        PrdStatus::Published => theme.success(),
        PrdStatus::Planned => theme.info(),
    };
    let completion = if prd.task_total > 0 {
        format!("{:.0}%", prd.task_done as f64 / prd.task_total as f64 * 100.0)
    } else {
        "\u{2014}".to_string() // em dash
    };

    let meta_lines = vec![
        Line::from(vec![
            Span::styled("slug:       ", theme.muted()),
            Span::styled(&prd.slug, theme.text()),
        ]),
        Line::from(vec![
            Span::styled("status:     ", theme.muted()),
            Span::styled(prd.status.label(), status_style),
        ]),
        Line::from(vec![
            Span::styled("tasks:      ", theme.muted()),
            Span::styled(
                format!("{}/{}", prd.task_done, prd.task_total),
                theme.text(),
            ),
            Span::styled(format!("  ({completion})"), theme.muted()),
        ]),
        Line::from(vec![
            Span::styled("failed:     ", theme.muted()),
            Span::styled(
                prd.task_failed.to_string(),
                if prd.task_failed > 0 {
                    theme.danger()
                } else {
                    theme.muted()
                },
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(meta_lines).wrap(Wrap { trim: false }),
        sections[0],
    );

    // Task list: look in plans/<slug> then .roko/plans/<slug>.
    let root = data.root();
    let task_dir = if root.join("plans").join(&prd.slug).exists() {
        root.join("plans").join(&prd.slug)
    } else {
        root.join(".roko").join("plans").join(&prd.slug)
    };
    let tasks = load_tasks_for_plan(&task_dir);

    let task_block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(
            format!(" Tasks ({}) ", tasks.len()),
            theme.muted(),
        ))
        .border_style(theme.muted());
    let task_inner = task_block.inner(sections[1]);
    frame.render_widget(task_block, sections[1]);

    if tasks.is_empty() {
        frame.render_widget(
            Paragraph::new("no tasks -- run 'roko prd plan <slug>' to generate")
                .style(theme.muted())
                .wrap(Wrap { trim: false }),
            task_inner,
        );
        return;
    }

    let title_max = (task_inner.width as usize).saturating_sub(24);
    let rows: Vec<Row<'_>> = tasks
        .iter()
        .map(|task| {
            let icon_style = match task.status {
                TaskState::Pending => theme.muted(),
                TaskState::Running => theme.warning(),
                TaskState::Done => theme.success(),
                TaskState::Failed => theme.danger(),
            };
            Row::new(vec![
                Cell::from(Span::styled(task.status.icon(), icon_style)),
                Cell::from(Span::styled(truncate(&task.id, 8), theme.muted())),
                Cell::from(Span::styled(
                    truncate(&task.title, title_max),
                    theme.text(),
                )),
                Cell::from(Span::styled(truncate(&task.agent, 12), theme.muted())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Min(10),
        Constraint::Length(12),
    ];
    frame.render_widget(
        Table::new(rows, widths)
            .header(
                Row::new(["", "id", "title", "agent"])
                    .style(theme.accent().add_modifier(Modifier::BOLD)),
            )
            .column_spacing(1),
        task_inner,
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

### Step 3: Verify

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Check that:
1. The atelier view compiles without warnings
2. Empty state renders when `.roko/prd/` does not exist
3. PRD list renders with correct status badges
4. Task list renders with `[x]`/`[>]`/`[ ]`/`[!]` status icons, color-coded consistently
5. Stats bar shows PRD/Plan/Task/Agent/Episode counts
6. `PUBL` badge is green (`theme.success()`), `DRFT` is amber (`theme.warning()`), `PLAN` is blue (`theme.info()`), `IDEA` is muted

## Acceptance criteria

- [ ] `atelier_view.rs` replaces the placeholder with full implementation
- [ ] Top stats bar renders 5 counters: PRDs / Plans / Tasks (done/total) / Agents / Episodes
- [ ] Left panel lists PRDs with 4-char status badges, progress counter, consistent colors
- [ ] PRD sorting: planned > published > draft > idea, then alphabetical
- [ ] Right panel shows selected PRD metadata (slug, status, task count, completion %, failed count)
- [ ] Task list renders with status icons `[ ]`/`[>]`/`[x]`/`[!]`, color-coded to theme
- [ ] Empty state renders when no `.roko/prd/` directory exists
- [ ] TOML task parser handles both `[[task]]` and `[[tasks]]` section headers
- [ ] `Block::bordered()` used throughout
- [ ] `cargo clippy` clean, `cargo +nightly fmt` clean
- [ ] All existing tests pass
