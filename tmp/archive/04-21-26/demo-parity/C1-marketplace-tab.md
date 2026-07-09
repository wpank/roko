# C1: Add F8 Marketplace tab to TUI

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

Add an F8 Marketplace tab that displays `.roko/jobs/*.json` files as a browsable job board. This tab lets operators see pending, active, and completed jobs posted by agents or external systems.

The left panel (35%) shows the job list. The right panel (65%) shows the selected job's detail with word-wrapped description.

## Dependency

None. This task is self-contained.

## Steps

### Step 1: Update the Tab enum

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/tabs.rs`

Read the file first, then make these changes.

**1a. Add two new variants:**

Find:
```rust
    /// F7 - Engram DAG inspector, episode replay.
    Inspect,
}
```

Replace with:
```rust
    /// F7 - Engram DAG inspector, episode replay.
    Inspect,
    /// F8 - Marketplace: job board for agent tasks.
    Marketplace,
    /// F9 - Atelier: PRD and plan workshop.
    Atelier,
}
```

**1b. Update `ALL` array from `[Tab; 7]` to `[Tab; 9]`:**

Find:
```rust
    pub const ALL: [Tab; 7] = [
        Tab::Dashboard,
        Tab::Plans,
        Tab::Agents,
        Tab::Git,
        Tab::Logs,
        Tab::Config,
        Tab::Inspect,
    ];
```

Replace with:
```rust
    pub const ALL: [Tab; 9] = [
        Tab::Dashboard,
        Tab::Plans,
        Tab::Agents,
        Tab::Git,
        Tab::Logs,
        Tab::Config,
        Tab::Inspect,
        Tab::Marketplace,
        Tab::Atelier,
    ];
```

**1c. Add match arms to `fkey()`:**

Find:
```rust
            Self::Inspect => KeyCode::F(7),
```

Replace with:
```rust
            Self::Inspect => KeyCode::F(7),
            Self::Marketplace => KeyCode::F(8),
            Self::Atelier => KeyCode::F(9),
```

**1d. Add match arms to `from_key()`:**

Find:
```rust
            KeyCode::F(7) => Some(Self::Inspect),
            _ => None,
```

Replace with:
```rust
            KeyCode::F(7) => Some(Self::Inspect),
            KeyCode::F(8) => Some(Self::Marketplace),
            KeyCode::F(9) => Some(Self::Atelier),
            _ => None,
```

**1e. Add match arms to `label()`:**

Find:
```rust
            Self::Inspect => "Inspect",
```

Replace with:
```rust
            Self::Inspect => "Inspect",
            Self::Marketplace => "Market",
            Self::Atelier => "Atelier",
```

**1f. Add match arms to `label_with_key()`:**

Find:
```rust
            Self::Inspect => "F7 Inspect",
```

Replace with:
```rust
            Self::Inspect => "F7 Inspect",
            Self::Marketplace => "F8 Market",
            Self::Atelier => "F9 Atelier",
```

**1g. Add match arms to `index()`:**

Find:
```rust
            Self::Inspect => 6,
```

Replace with:
```rust
            Self::Inspect => 6,
            Self::Marketplace => 7,
            Self::Atelier => 8,
```

**1h. Update `next()`:**

Find:
```rust
            Self::Config => Self::Inspect,
            Self::Inspect => Self::Dashboard,
```

Replace with:
```rust
            Self::Config => Self::Inspect,
            Self::Inspect => Self::Marketplace,
            Self::Marketplace => Self::Atelier,
            Self::Atelier => Self::Dashboard,
```

**1i. Update `prev()`:**

Find:
```rust
            Self::Dashboard => Self::Inspect,
            Self::Plans => Self::Dashboard,
```

Replace with:
```rust
            Self::Dashboard => Self::Atelier,
            Self::Plans => Self::Dashboard,
```

Find:
```rust
            Self::Inspect => Self::Config,
```

Replace with:
```rust
            Self::Inspect => Self::Config,
            Self::Marketplace => Self::Inspect,
            Self::Atelier => Self::Marketplace,
```

**1j. Update the cycle test** (search for `for _ in 0..7`):

Find:
```rust
        for _ in 0..7 {
            t = t.next();
        }
        assert_eq!(t, Tab::Dashboard);

        for _ in 0..7 {
            t = t.prev();
        }
```

Replace with:
```rust
        for _ in 0..9 {
            t = t.next();
        }
        assert_eq!(t, Tab::Dashboard);

        for _ in 0..9 {
            t = t.prev();
        }
```

### Step 2: Update FocusZone for new tabs

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs`

In `FocusZone::next()`, find:
```rust
            Tab::Git | Tab::Logs | Tab::Config | Tab::Inspect => self,
```

Replace with:
```rust
            Tab::Git | Tab::Logs | Tab::Config | Tab::Inspect
            | Tab::Marketplace | Tab::Atelier => self,
```

Same change in `FocusZone::prev()`.

### Step 3: Add SubView variants and update views/mod.rs

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/mod.rs`

**3a. Add module declarations** after `pub mod plans_view;`:

Find:
```rust
pub mod plans_view;
```

Replace with:
```rust
pub mod plans_view;
pub mod marketplace_view;
pub mod atelier_view;
```

**3b. Add SubView variants** after the KnowledgeBrowse variant:

Find:
```rust
    /// Knowledge browser (Neuro store).
    KnowledgeBrowse,
}
```

Replace with:
```rust
    /// Knowledge browser (Neuro store).
    KnowledgeBrowse,

    // -- Region 8: Marketplace (F8) --
    /// Job listing browser.
    JobList,
    /// Individual job detail.
    JobDetail,
    /// Create new job form.
    CreateJob,

    // -- Region 9: Atelier (F9) --
    /// PRD workshop list.
    PrdWorkshop,
    /// Plan detail explorer.
    PlanExplorer,
}
```

**3c. Add `for_tab` arms:**

Find:
```rust
            Tab::Inspect => &[
                SubView::EngramDag,
                SubView::EpisodeReplay,
                SubView::KnowledgeBrowse,
            ],
```

Replace with:
```rust
            Tab::Inspect => &[
                SubView::EngramDag,
                SubView::EpisodeReplay,
                SubView::KnowledgeBrowse,
            ],
            Tab::Marketplace => &[
                SubView::JobList,
                SubView::JobDetail,
                SubView::CreateJob,
            ],
            Tab::Atelier => &[
                SubView::PrdWorkshop,
                SubView::PlanExplorer,
            ],
```

**3d. Add `label()` arms:**

Find:
```rust
            Self::KnowledgeBrowse => "Knowledge",
```

Replace with:
```rust
            Self::KnowledgeBrowse => "Knowledge",
            Self::JobList => "Jobs",
            Self::JobDetail => "Detail",
            Self::CreateJob => "New Job",
            Self::PrdWorkshop => "PRDs",
            Self::PlanExplorer => "Plans",
```

**3e. Add render dispatch arms in `render_tab_content()`:**

Find:
```rust
        Tab::Inspect => context_view::render(frame, area, data, tui_state, view_state, theme),
    }
```

Replace with:
```rust
        Tab::Inspect => context_view::render(frame, area, data, tui_state, view_state, theme),
        Tab::Marketplace => marketplace_view::render(frame, area, data, tui_state, view_state, theme),
        Tab::Atelier => atelier_view::render(frame, area, data, tui_state, view_state, theme),
    }
```

### Step 4: Update header_bar.rs F-key strip

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/header_bar.rs`

Find the `fkey_items` vector. The exact color constants depend on what `Theme::` exports. Use the same pattern as existing entries -- match the color to the conceptual role (Marketplace = sage/green for marketplace activity, Atelier = dream/dim for creative work).

Find:
```rust
        (" F7", Theme::BONE_DIM, "inspect", Tab::Inspect),
    ];
```

Replace with:
```rust
        (" F7", Theme::BONE_DIM, "inspect", Tab::Inspect),
        (" F8", Theme::SAGE, "mkt", Tab::Marketplace),
        (" F9", Theme::DREAM, "atl", Tab::Atelier),
    ];
```

If `Theme::SAGE` or `Theme::DREAM` do not exist, use `Theme::BONE_DIM` as a safe fallback and add a `// MOCK: update color when theme is extended` comment.

### Step 5: Update status_bar.rs keybind hints

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/status_bar.rs`

Find:
```rust
        Tab::Inspect => "\u{2191}\u{2193}:nav  ?:help",
    };
```

Replace with:
```rust
        Tab::Inspect => "\u{2191}\u{2193}:nav  1-3:sub-view  ?:help",
        Tab::Marketplace => "j/k:nav  Enter:detail  n:new  r:refresh  ?:help",
        Tab::Atelier => "j/k:nav  Enter:expand  1/2:sub-view  ?:help",
    };
```

### Step 6: Create the marketplace view module

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/marketplace_view.rs`

Create this file with the following complete contents:

```rust
//! F8 Marketplace view -- job board browser.
//!
//! Layout: left 35% (job list) | right 65% (job detail).
//!
//! Data source: `.roko/jobs/*.json` files (no roko-serve required).
//! Job type tags: research = rose, coding_task = bone, other = muted.
//! Status icons: pending = open circle, active = play, done = check, failed = cross.
//!
//! Keyboard:
//!   j/k     -- navigate list (wraps at boundaries)
//!   Enter   -- focus detail panel
//!   r       -- signal refresh (next file poll picks up changes)

use std::path::Path;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap};
use serde::Deserialize;

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::state::TuiState;

// ---------------------------------------------------------------------------
// Job data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
struct Job {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    job_type: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    state: String, // alias: some payloads use "state" instead of "status"
    #[serde(default)]
    posted_by: String,
    #[serde(default)]
    assigned_to: String,
    #[serde(default)]
    priority: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    reward: String,
    #[serde(default)]
    plan_id: String,
}

impl Job {
    /// Canonical status string, preferring `status` over `state`.
    fn effective_status(&self) -> &str {
        if !self.status.is_empty() {
            &self.status
        } else if !self.state.is_empty() {
            &self.state
        } else {
            "unknown"
        }
    }
}

// ---------------------------------------------------------------------------
// Data loading
// ---------------------------------------------------------------------------

fn load_jobs(root: &Path) -> Vec<Job> {
    let jobs_dir = root.join(".roko").join("jobs");
    let Ok(entries) = std::fs::read_dir(&jobs_dir) else {
        return Vec::new();
    };

    let mut jobs: Vec<Job> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "json")
        })
        .filter_map(|e| {
            let data = std::fs::read_to_string(e.path()).ok()?;
            let mut job: Job = serde_json::from_str(&data).ok()?;
            if job.id.is_empty() {
                job.id = e
                    .path()
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
            }
            Some(job)
        })
        .collect();

    // Sort newest first (by created_at descending, then by id).
    jobs.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then(b.id.cmp(&a.id))
    });
    jobs
}

// ---------------------------------------------------------------------------
// Public render entry point
// ---------------------------------------------------------------------------

/// Render the full marketplace view.
///
/// Handles terminal resize: the layout uses percentage constraints so it
/// adapts automatically to any terminal width.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    _tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let jobs = load_jobs(data.root());

    if jobs.is_empty() {
        render_empty(frame, area, theme);
        return;
    }

    let panels = Layout::horizontal([
        Constraint::Percentage(35),
        Constraint::Percentage(65),
    ])
    .split(area);

    let selected = view_state.selected.min(jobs.len().saturating_sub(1));
    render_job_list(frame, panels[0], &jobs, selected, theme);
    if let Some(job) = jobs.get(selected) {
        render_job_detail(frame, panels[1], job, theme);
    }
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------

fn render_empty(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let block = Block::bordered()
        .title(Span::styled(
            " Marketplace ",
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("No jobs posted.", theme.muted())),
        Line::from(""),
        Line::from(Span::styled(
            "Jobs appear when agents or operators post work items to .roko/jobs/.",
            theme.muted(),
        )),
        Line::from(Span::styled(
            "Press 'n' to create a new job manually.",
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
// Left panel: job list
// ---------------------------------------------------------------------------

fn render_job_list(
    frame: &mut Frame<'_>,
    area: Rect,
    jobs: &[Job],
    selected: usize,
    theme: &Theme,
) {
    // Count by canonical status for the header badge.
    let pending = jobs
        .iter()
        .filter(|j| matches!(j.effective_status(), "open" | "pending" | "assigned"))
        .count();
    let active = jobs
        .iter()
        .filter(|j| matches!(j.effective_status(), "active" | "running" | "in_progress"))
        .count();
    let done = jobs
        .iter()
        .filter(|j| matches!(j.effective_status(), "done" | "completed" | "evaluated"))
        .count();

    let block = Block::bordered()
        .title(Span::styled(
            format!(" Jobs ({}) {pending}P {active}A {done}D ", jobs.len()),
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 10 {
        return;
    }

    let visible_height = inner.height as usize;
    // Scroll to keep `selected` visible.
    let scroll = if selected >= visible_height {
        selected - visible_height + 1
    } else {
        0
    };

    let items: Vec<ListItem<'_>> = jobs
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, job)| {
            let is_sel = i == selected;
            let status = job.effective_status();

            let (icon, status_style) = match status {
                "open" | "pending" => ("\u{25cb}", theme.muted()),      // ○
                "assigned" => ("\u{25d4}", theme.info()),               // ◔
                "in_progress" | "active" | "running" => ("\u{25b6}", theme.warning()), // ▶
                "submitted" => ("\u{25d1}", theme.info()),              // ◑
                "done" | "completed" | "evaluated" => ("\u{2713}", theme.success()), // ✓
                "failed" | "cancelled" => ("\u{2717}", theme.danger()), // ✗
                _ => ("\u{00b7}", theme.muted()),                       // ·
            };

            // Job type color tag (research=rose, coding_task=bone/dim, other=muted).
            let type_style = match job.job_type.as_str() {
                "research" => Style::default().fg(Theme::ROSE),
                "coding_task" | "coding" => Style::default().fg(Theme::BONE_DIM),
                _ => theme.muted(),
            };

            let avail_width = (inner.width as usize).saturating_sub(8);
            let title = truncate(&job.title, avail_width);
            let row_style = if is_sel { theme.selection() } else { theme.text() };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {icon} "), status_style),
                // Small type indicator
                Span::styled(
                    format!("[{}] ", &job.job_type.chars().take(3).collect::<String>()),
                    type_style,
                ),
                Span::styled(title, row_style),
            ]))
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

// ---------------------------------------------------------------------------
// Right panel: job detail
// ---------------------------------------------------------------------------

fn render_job_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    job: &Job,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            format!(" {} ", truncate(&job.title, 40)),
            theme.accent().add_modifier(Modifier::BOLD),
        ))
        .border_style(theme.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 20 {
        return;
    }

    let sections = Layout::vertical([
        Constraint::Length(8), // Metadata table
        Constraint::Min(0),    // Description (word-wrapped)
    ])
    .split(inner);

    let status = job.effective_status();
    let status_style = match status {
        "open" | "pending" => theme.muted(),
        "assigned" | "submitted" => theme.info(),
        "in_progress" | "active" | "running" => theme.warning(),
        "done" | "completed" | "evaluated" => theme.success(),
        "failed" | "cancelled" => theme.danger(),
        _ => theme.text(),
    };
    let priority_style = match job.priority.as_str() {
        "critical" | "p0" => theme.danger(),
        "high" | "p1" => theme.warning(),
        "medium" | "p2" | "" => theme.muted(),
        _ => theme.muted(),
    };

    let col_widths = [Constraint::Length(11), Constraint::Min(0)];

    let meta_rows = vec![
        Row::new([
            Cell::from(Span::styled("id:", theme.muted())),
            Cell::from(Span::styled(&job.id, theme.text())),
        ]),
        Row::new([
            Cell::from(Span::styled("status:", theme.muted())),
            Cell::from(Span::styled(status, status_style)),
        ]),
        Row::new([
            Cell::from(Span::styled("type:", theme.muted())),
            Cell::from(Span::styled(&job.job_type, theme.text())),
        ]),
        Row::new([
            Cell::from(Span::styled("priority:", theme.muted())),
            Cell::from(Span::styled(
                if job.priority.is_empty() { "\u{2014}" } else { &job.priority },
                priority_style,
            )),
        ]),
        Row::new([
            Cell::from(Span::styled("posted by:", theme.muted())),
            Cell::from(if job.posted_by.is_empty() {
                Span::styled("\u{2014}", theme.muted())
            } else {
                Span::styled(&job.posted_by, theme.text())
            }),
        ]),
        Row::new([
            Cell::from(Span::styled("assigned:", theme.muted())),
            Cell::from(if job.assigned_to.is_empty() {
                Span::styled("(unassigned)", theme.muted())
            } else {
                Span::styled(&job.assigned_to, theme.info())
            }),
        ]),
        Row::new([
            Cell::from(Span::styled("created:", theme.muted())),
            Cell::from(Span::styled(
                if job.created_at.is_empty() { "\u{2014}" } else { &job.created_at },
                theme.muted(),
            )),
        ]),
        Row::new([
            Cell::from(Span::styled("tags:", theme.muted())),
            Cell::from(Span::styled(
                if job.tags.is_empty() {
                    "(none)".to_string()
                } else {
                    job.tags.join(", ")
                },
                theme.muted(),
            )),
        ]),
    ];

    frame.render_widget(
        Table::new(meta_rows, col_widths).column_spacing(1),
        sections[0],
    );

    // Description with proper word-wrap using ratatui's Wrap widget.
    let desc_block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(" Description ", theme.muted()))
        .border_style(theme.muted());
    let desc_inner = desc_block.inner(sections[1]);
    frame.render_widget(desc_block, sections[1]);

    let desc_text = if job.description.is_empty() {
        "No description provided.".to_string()
    } else {
        job.description.clone()
    };
    frame.render_widget(
        Paragraph::new(desc_text)
            .style(theme.text())
            .wrap(Wrap { trim: false }),
        desc_inner,
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> String {
    if max < 4 || s.len() <= max {
        return s.to_string();
    }
    // Find a char boundary near max-3 for the "..." suffix.
    let mut end = max - 3;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}
```

### Step 7: Create atelier view placeholder

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/atelier_view.rs`

```rust
//! F9 Atelier view -- PRD and plan workshop.
//!
//! Placeholder module. Full implementation is in C2-atelier-tab.md.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::state::TuiState;

/// Render the atelier view (placeholder; see C2-atelier-tab.md).
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    _tui_state: &TuiState,
    _view_state: &ViewState,
    theme: &Theme,
) {
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
        Line::from(Span::styled(
            "PRD & Plan Workshop",
            theme.accent().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This tab shows PRDs, plans, and task status.",
            theme.muted(),
        )),
        Line::from(Span::styled(
            "See C2-atelier-tab.md for the full implementation.",
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
```

### Step 8: Verify

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Check that:
1. `Tab::ALL` has 9 entries and `index_is_sequential` test passes with 9 indices
2. `next_prev_cycle` test passes with 9 iterations
3. F8 opens Marketplace, F9 opens Atelier
4. The marketplace view renders the empty state when `.roko/jobs/` does not exist
5. The atelier placeholder renders without panicking
6. All existing tests pass

## Acceptance criteria

- [ ] `Tab::Marketplace` and `Tab::Atelier` variants exist
- [ ] `Tab::ALL` is `[Tab; 9]`
- [ ] F8 and F9 key bindings work in `fkey()` and `from_key()`
- [ ] `next()`/`prev()` cycle through all 9 tabs without gaps
- [ ] `SubView::JobList`, `JobDetail`, `CreateJob` exist
- [ ] `SubView::PrdWorkshop`, `PlanExplorer` exist
- [ ] `marketplace_view.rs` compiles and renders job list from `.roko/jobs/`
- [ ] Job type tags are colored: research = rose, coding_task = bone, other = muted
- [ ] Job detail description is word-wrapped via `Wrap { trim: false }`
- [ ] Empty state renders with centered message when no jobs exist
- [ ] `atelier_view.rs` compiles and renders placeholder
- [ ] `render_tab_content()` dispatches to both new views
- [ ] Header bar F-key strip shows F8 and F9
- [ ] Status bar shows context-sensitive keybind hints for both tabs
- [ ] `Block::bordered()` used throughout (not `Block::default().borders(Borders::ALL)`)
- [ ] All existing tests pass
- [ ] `cargo clippy` clean, `cargo +nightly fmt` clean
