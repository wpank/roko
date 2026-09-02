# Audit: F8 Marketplace, F9 Atelier, F10 Learning Views

**Date:** 2026-09-01
**Files examined:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/marketplace_view.rs` (613 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/atelier_view.rs` (544 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/learning_view.rs` (584 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/affect_view.rs` (162 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/dream_view.rs` (433 lines)

---

## F8 Marketplace View

### 1. What data is shown?

The marketplace view renders jobs from `TuiState.marketplace_jobs` (type `Vec<roko_core::MarketplaceJob>`).
Data is loaded from `.roko/jobs/*.json` files by `scan_marketplace_jobs()` in `dashboard.rs` (line 975).

Displayed fields per job:
- **List panel:** Status icon (Unicode circle/play/check/cross), job type tag (3-char abbreviation, color-coded), title (truncated).
- **Detail panel:** id, status (with valid-transition hint via `JobStatus::parse`), type, priority (color-coded critical/high/medium), posted_by, assigned_to, created_at, tags, full description (word-wrapped), and an optional progress bar for in-progress jobs (percent, agent_id, message from `tui_state.job_progress`).

### 2. Is the view populated or mostly empty/stub?

**Fully populated.** This is a production-quality view with:
- Split-panel layout (35% list / 65% detail).
- Three sub-views: `JobList` (default dual-panel), `JobDetail` (full-screen detail), `CreateJob` (job creation form).
- The `CreateJob` sub-view is a complete 4-field form (Title, Type, Priority, Description) with Tab/Enter/Ctrl-S/Esc navigation, block cursor rendering, focus/edit border styling, and command result feedback.
- Empty state with helpful guidance text.
- Job assignment inline prompt (`tui_state.job_assign_editing` / `job_assign_buffer`).
- Scroll management for the list panel with selected-item tracking.
- Keybinding hints in the detail footer (s:status, a:assign, n:new, r:refresh).

### 3. Navigation within the view

- **j/k or Up/Down:** Navigate job list (with scroll clamping).
- **Enter:** Focus/expand detail panel (switches to `SubView::JobDetail`).
- **n:** Switch to `SubView::CreateJob` form.
- **r:** Trigger refresh.
- **Ctrl-S:** Submit job creation form.
- **Tab focus zones:** `MarketList` and `MarketDetail` with Tab/Shift-Tab cycling.
- **Number keys (1-3):** Switch between JobList, JobDetail, CreateJob sub-views.
- Form navigation: Tab cycles fields, Enter enters edit mode, Esc cancels.

### 4. Visual quality of rendering

**Good.** Consistent use of the Theme API (`theme.accent()`, `theme.muted()`, `theme.success()`, etc.).
Uses proper Unicode icons for status. Color-codes job types (research = rose, coding_task = bone). Priority
has red/yellow/muted coloring. Progress bar uses full-block/light-shade characters. The layout is responsive
(percentage-based constraints). Minimum size guards prevent rendering into too-small areas. The CreateJob form
has proper focused/editing/unfocused border style states.

### 5. Is this view essential or could it be merged?

**Marginal.** The marketplace is a self-contained feature domain (job posting, assignment, tracking) that
justifies its own tab. However, in practice, jobs are typically created via the CLI or API, and the TUI is
primarily a monitoring surface. The job _creation form_ in the TUI adds real value as it avoids shelling out.
If tab count ever needs reduction, this could merge into a "Work" super-tab alongside Atelier (F9), since
both deal with work-item management.

### 6. What's missing?

- **No status transition action in TUI:** The keybinding hint shows `s:status` but no corresponding
  `handle_marketplace_key` branch transitions the status. The hint `a:assign` likewise has no input.rs
  handler beyond the `job_assign_editing` state toggle (which must be wired elsewhere).
- **No delete/cancel job action.**
- **No filtering or search** across jobs (no search_query usage).
- **No sort toggle** (currently sorted by created_at descending, hardcoded).
- **Job progress bar** depends on `tui_state.job_progress` which has no visible writer path -- unclear
  if any runtime actually populates this.
- **No test coverage** for the rendering functions (unlike dream_view which has 3 tests).

### 7. PRD comparison

The marketplace was specified in E38 (Artifact marketplace). The TUI view covers the job-browser side
(listing, detail, creation) but not artifact/package/publish/economics dimensions. The E38 manifest
notes "durable storage/search, executable publish/install pipelines" remain product work. The TUI
correctly reflects the job-board subset that is implemented.

---

## F9 Atelier View

### 1. What data is shown?

The atelier view renders PRDs from `TuiState.atelier_prds` (type `Vec<roko_core::PrdSummary>`) and
per-slug tasks from `TuiState.atelier_tasks_by_slug`.

Data is loaded by `scan_atelier_prds()` in `dashboard.rs` (line 1008), which:
- Scans `.roko/prd/*.md` for PRD markdown files, extracting title (first `#` heading), slug (filename stem), and status (from frontmatter/content).
- Scans `plans/` and `.roko/plans/` for `tasks.toml` files, correlating them to PRD slugs by substring match.
- Tallies task_done/task_total/task_failed per PRD.

Displayed data:
- **Stats bar:** PRD count, plan count, tasks done/total, agent count, episode count.
- **PRD list:** Status badge (IDEA/DRFT/PUBL/PLAN), title (truncated), task progress fraction (color-coded).
- **Plan detail:** slug, status (with action hint), task done/total + percentage, failed count,
  CLI actions block (contextual commands based on PRD lifecycle stage), task table with
  status icon/id/title/agent columns.

### 2. Is the view populated or mostly empty/stub?

**Fully populated.** This is a comprehensive PRD lifecycle viewer:
- Stats bar across the top with 5 metrics.
- Split layout (40% PRD list / 60% plan detail).
- `PlanExplorer` sub-view for full-screen plan detail.
- Empty state with CLI guidance text.
- Task table with status-colored icons (pending `[ ]`, running `[>]`, done `[x]`, failed `[!]`).
- Contextual CLI action blocks that change based on PRD status (idea/draft shows "publish + plan",
  published shows "plan + run", planned shows "run").

### 3. Navigation within the view

- **j/k or Up/Down:** Navigate PRD list.
- **Enter:** Expand/focus detail (switches to `SubView::PlanExplorer`).
- **r:** Trigger refresh.
- **Tab focus zones:** `AtelierList` and `AtelierDetail`.
- **Number keys (1-2):** Switch between PrdWorkshop and PlanExplorer sub-views.
- Keybinding hints: p:publish, g:gen plan, r:refresh.

### 4. Visual quality of rendering

**Good.** Uses Theme consistently. Status badges are 4-char abbreviations with semantic coloring (idea=muted,
draft=warning, published=success, planned=info). Task progress in the PRD list is color-coded (red for
failures, green for complete, muted for partial). The stats bar uses a 5-column equal-width layout. The
task table has a styled header row. CLI actions in the detail panel are genuinely helpful -- they show the
exact command to run next based on the PRD's current lifecycle stage.

### 5. Is this view essential or could it be merged?

**Essential.** The Atelier is the primary window into the PRD-to-plan lifecycle, which is the core
self-hosting workflow (`prd idea -> prd draft -> prd plan -> plan run`). This view lets you see the full
pipeline status without shelling out. It could conceivably merge with F2 Plans, but the PRD dimension
(idea/draft/published/planned lifecycle) is distinct from the plan execution detail that F2 provides.

### 6. What's missing?

- **Keybinding actions `p` (publish) and `g` (gen plan) are hinted but not wired** in `handle_atelier_key`.
  The key handler only has j/k/Enter/r/Home/End. These hints are purely aspirational.
- **No PRD content preview.** The detail panel shows metadata and tasks but never shows the actual PRD
  markdown content. A "description" or "content" section would be valuable.
- **Slug-to-plan matching is fragile:** uses bidirectional substring matching (`plan_lower.contains(&slug_lower)
  || slug_lower.contains(&plan_lower)`), which can produce false positives for short slugs.
- **No task scrolling** in the plan detail panel -- if there are more tasks than fit, they silently overflow.
- **Status detection is keyword-based** (scanning markdown content for "status: published" etc.), which is
  fragile and won't work with YAML frontmatter parsing or other formatting.
- **No test coverage** for rendering functions.
- **No inline PRD editing** -- the "Atelier" (workshop) name implies a creative workspace, but it is
  read-only. All mutations require CLI commands.

### 7. PRD comparison

The Atelier maps to the PRD lifecycle commands (`roko prd idea/draft/plan`). The view accurately represents
the data that exists. The gap is that it is observation-only -- the "workshop" metaphor suggests editing/
creating PRDs within the TUI, which is not implemented.

---

## F10 Learning View

### 1. What data is shown?

The learning view renders cascade router state and efficiency events from:
- `TuiState.cascade_router` (type `CascadeRouterState` -- model slugs, per-model confidence stats with trials/successes).
- `TuiState.efficiency_events` (type `Vec<AgentEfficiencyEvent>` -- per-turn records with model, cost_usd, wall_time_ms, gate_passed).

Three sub-views:

**Sub-view 1 (Router):**
- Cascade stage indicator (Static/Confidence/UCB based on observation count thresholds: <10/10-30/30+).
- Per-model stats table: Model, Trials, Successes, Pass Rate (color-coded), Sparkline (rolling window pass rate).
- Selection frequency bar chart (BarChart widget with per-model trial counts).

**Sub-view 2 (History):**
- Current observation count.
- Stage progression timeline: Static -> Confidence -> UCB with active indicator, observation range labels.
- Visual progression bar using colored full-block characters (yellow/cyan/green).
- Legend.

**Sub-view 3 (Efficiency):**
- Per-model efficiency table: Model, Events, Passed, Pass %, Avg Cost, Avg Latency.
- Average cost bar chart (BarChart widget, scaled to 10^-4 dollars).

### 2. Is the view populated or mostly empty/stub?

**Fully populated.** All three sub-views are complete implementations:
- The sparkline generation (`model_sparkline`) uses a 5-event rolling window mapped to 8-level Unicode block characters, taking the last 20 data points.
- BarChart widgets are properly sized with dynamic bar widths based on terminal width and model count.
- Empty states show appropriate messages ("No cascade router data" / "No efficiency events recorded yet").
- The efficiency sub-view aggregates per-model stats (count, passed, total_cost, total_latency_ms) from raw events.

### 3. Navigation within the view

- **j/k or Up/Down:** Scroll within sub-views.
- **r:** Trigger refresh.
- **Number keys (1-3):** Switch between Route, History, Efficiency sub-views.
- **Tab focus zones:** `LearningMetrics` and `LearningDetail`.

Navigation is minimal compared to F8/F9 -- there is no row selection or drill-down within any sub-view.

### 4. Visual quality of rendering

**Good, with minor style inconsistency.** The learning view uses `Style::default().fg(theme.muted)` (accessing
the raw `Color` field) rather than `theme.muted()` (the method that returns a `Style`). This is technically
correct but stylistically inconsistent with F8/F9 which use the method form. Both produce the same result,
but the field access pattern bypasses any future Theme method changes.

The sparkline rendering is elegant. The stage progression bar in the History sub-view is well-designed --
the three-color filled bar with legend provides an intuitive visualization. The bar charts use dynamic
width calculation that adapts to terminal size.

### 5. Is this view essential or could it be merged?

**Essential for operators.** The learning view is the only window into how the cascade router is performing
and which models are being selected. During self-hosting runs, this is critical for understanding whether
the adaptive routing is converging and whether specific models are dragging down pass rates. It cannot
merge with any other tab because learning/routing is a cross-cutting concern, not tied to plans or agents.

### 6. What's missing?

- **No row selection or drill-down.** You cannot select a model to see its detailed history, recent events,
  or failure modes. The tables are display-only.
- **No experiment display.** The learning system includes A/B experiments (`roko-learn` experiments module),
  but the learning view shows only cascade router and efficiency. Experiment arms, assignment counts, and
  winner declarations have no TUI representation.
- **No playbook/when-then display.** E25 advanced learning includes when/then playbooks. These are not
  surfaced anywhere in the TUI.
- **No gate threshold display.** Adaptive gate thresholds (EMA per rung in `.roko/learn/gate-thresholds.json`)
  are not shown.
- **No cost budget tracking.** The ACP system has USD budget enforcement, but the learning view does not
  show cumulative cost vs. budget.
- **The History sub-view is somewhat redundant** with the stage indicator in the Route sub-view. Both
  communicate the same three-stage progression; History just adds a colored bar.
- **`shorten_model` imported but not directly used** in this file -- `display_model` is used instead.
  The import is not dead (used by other TUI code via `display_utils`), but it creates a confusing import.
- **No test coverage.**

---

## Affect View (Bonus: F1 Dashboard sub-view 4)

### 1. What data is shown?

Renders `TuiState.affect` (type `Option<roko_core::AffectSnapshot>`):
- **PAD gauges:** Pleasure, Arousal, Dominance -- three `Gauge` widgets mapping [-1,1] to [0,1] ratio,
  with green/red/yellow color coding based on value thresholds.
- **State label:** Behavioral state (Coasting/Focused/Struggling/Exploring/Resting) with confidence percentage.
- **Recent markers:** Somatic marker list (up to 8 entries) with valence-colored labels.
- **Active biases:** Dispatch bias list.

### 2. Is the view populated or mostly empty/stub?

**Fully populated but disconnected from rendering.** The view has a complete implementation with
PAD gauges, state label, marker/bias lists, and empty-state handling. However, it is registered
as `SubView::AffectView` (sub-view 4 of Dashboard/F1) in `views/mod.rs` but is **never actually
dispatched** from `dashboard_view.rs`. The dashboard_view file does not reference `affect_view`,
`AffectView`, or any sub-view dispatch at all -- it uses its own internal 8-sub-tab system
(Agents/Output/Diff/Verify/Git/MCP/Learning/Procs) that is entirely separate from the
`SubView` enum. This means pressing `4` on the Dashboard tab will select `SubView::AffectView`
in `ViewState` but the dashboard renderer ignores `view_state` sub-tab selection entirely.

### 3. Navigation

None within the view itself. It is a pure display panel.

### 4. Visual quality

Good. The `Gauge` widget usage for PAD dimensions is appropriate. Color thresholds are sensible
(positive > 0.2 = green, negative < -0.2 = red, neutral = yellow). The marker/bias split
layout (60/40) is clean.

### 5. Essential or mergeable?

**Should be integrated into Dashboard.** It belongs on the Dashboard tab as promised by the
SubView registration, but needs actual wiring into `dashboard_view.rs`'s rendering dispatch.

### 6. What's missing?

- **Not wired into rendering pipeline.** The SubView variant exists but dashboard_view ignores it.
- **No trend/history.** Only shows the latest PAD snapshot, not how affect has changed over time.
- **No energy fields.** E23 (cognitive autonomy) mentions CorticalState energy fields and energy
  accounting, but the affect view only shows PAD/behavioral state/markers/biases.

---

## Dream View (Bonus: widget, not a tab view)

### 1. What data is shown?

Renders `DreamSnapshot` (a local struct defined in the widget, NOT in roko-core):
- **Phase header:** Current dream phase (Idle/NREM Replay/REM Imagination/Integration/Threat Rehearsal)
  with progress counters, cycle count, waking improvement percentage, MAP-Elites archive coverage.
- **Replay candidates:** Episode IDs with utility scores and graphical bars.
- **Hypotheses:** Counterfactual hypotheses with confidence percentages (color-coded).

### 2. Is the view populated or mostly empty/stub?

**Complete widget implementation, but orphaned.** The widget is fully implemented with:
- Phase-specific coloring using custom Theme constants (DREAM_BRIGHT, DREAM_REM, SAGE, EMBER).
- Progress bars for replay candidate utility.
- Confidence-colored hypothesis entries.
- Three tests (normal render, idle render, small area render).

However, `render_dream_view` is **never called** from any view or the rendering pipeline. No view
file imports or uses it. No TuiState field contains a `DreamSnapshot`. The `DreamSnapshot` struct
is defined locally in the widget file rather than in roko-core or TuiState.

### 3. Navigation

None -- it is a pure display widget.

### 4. Visual quality

Good. Uses phase-specific colors from Theme constants. The utility bar (full-block/light-shade) is
clean. Small-area guard returns early if height < 3 or width < 20.

### 5. Essential or mergeable?

**Should live under F7 Inspect (Knowledge sub-view) or F1 Dashboard.** Dreams are part of the
knowledge subsystem (`roko-dreams`). The widget is ready to embed but needs a data source and a
call site.

### 6. What's missing?

- **Not called from any rendering path.** Dead code in production.
- **No data source wired.** TuiState has no `DreamSnapshot` field; the dashboard data pipeline
  does not produce one. The `roko-dreams` crate's runtime state would need to be projected into
  a snapshot format and loaded into TuiState.
- **DreamSnapshot is a local type** rather than a roko-core shared type, so integration requires
  either moving the struct or bridging.

---

## Summary Table

| View | Lines | Populated | Wired | Sub-views | Tests | Key gaps |
|------|-------|-----------|-------|-----------|-------|----------|
| F8 Marketplace | 613 | Yes | Yes | 3 (List, Detail, CreateJob) | 0 | Status/assign keybinds hinted but unwired; no search/filter |
| F9 Atelier | 544 | Yes | Yes | 2 (PRDs, Plans) | 0 | Publish/gen-plan keybinds hinted but unwired; no PRD content preview; fragile slug matching |
| F10 Learning | 584 | Yes | Yes | 3 (Route, History, Efficiency) | 0 | No drill-down; no experiments/playbooks/gate-thresholds display |
| Affect (F1 sub-4) | 162 | Yes | **No** | 0 | 0 | Registered as SubView but dashboard_view ignores SubView dispatch |
| Dream (widget) | 433 | Yes | **No** | 0 | 3 | Fully orphaned -- no call site, no data source in TuiState |

## Priority Recommendations

1. **Wire the affect view into dashboard_view.rs** -- It is registered as Dashboard sub-view 4 but
   dashboard_view uses its own separate sub-tab system. Either integrate it as the "4" sub-tab or
   remove the SubView::AffectView registration to avoid confusion.

2. **Wire the dream view into F7 Inspect or F1 Dashboard** -- The widget is complete and tested but
   has no call site. Needs a `DreamSnapshot` field in TuiState and a data loading path from
   `roko-dreams` state.

3. **Wire the hinted keybinds in F8 and F9** -- Both views render keybinding hints (`s:status`,
   `a:assign`, `p:publish`, `g:gen plan`) that have no corresponding `handle_*_key` implementation.
   This is misleading to users.

4. **Add row selection to F10 Learning** -- The tables are display-only with no way to select a model
   for detail. At minimum, j/k should highlight a model row and show expanded stats.

5. **Add experiment/playbook display to F10** -- The learning system has A/B experiments and when/then
   playbooks that are entirely invisible in the TUI.

6. **Fix style inconsistency in learning_view.rs** -- Uses `theme.muted` (field access) vs the
   `theme.muted()` (method) pattern used everywhere else. Not a bug but a maintenance hazard.

---

## Implementation Status (2026-09-02 swarm)

F8/F9/F10 view improvements (task #17): marketplace, atelier, learning tab visual quality.
