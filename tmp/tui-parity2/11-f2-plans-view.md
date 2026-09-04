# F2 Plans View Audit

Audited: 2026-09-01

## Files Reviewed

| File | LOC | Role |
|---|---|---|
| `crates/roko-cli/src/tui/views/plans_view.rs` | ~1374 | Main F2 view: left plan tree + right detail panel |
| `crates/roko-cli/src/tui/widgets/plan_tree.rs` | ~1050 | Standalone plan tree widget (used by dashboard F1) |
| `crates/roko-cli/src/tui/modals/plan_detail.rs` | ~340 | Plan detail overlay modal |
| `crates/roko-cli/src/tui/modals/task_detail.rs` | ~178 | Task detail overlay modal |
| `crates/roko-cli/src/tui/modals/wave_overview.rs` | ~146 | Wave overview popup |
| `crates/roko-cli/src/tui/modals/queue_overview.rs` | ~163 | Milestone queue popup |
| Mori reference: `apps/mori/src/tui/widgets/plan_tree.rs` | ~1078 | Original plan tree for comparison |

---

## 1. Plan Tree Rendering: DAG Visualization

**Rating: Partial -- wave grouping works, but no DAG visualization**

The plan tree renders a **two-level hierarchy**: Waves containing Plans. This is computed
from task `depends_on_plan` edges via `resolve_plan_wave()` in `state.rs:4194-4209`, which
assigns each plan to a topological wave number (wave N means all dependencies are in waves
< N). The result is correct but loses all structural information.

What exists:
- Wave headers with collapse/expand icons (`plans_view.rs:288-293`, `plan_tree.rs:277-281`)
- Plans indented under their wave with status icons
- Wave progress bars (8-char gradient fill, `plan_tree.rs:309-318`)
- Per-plan progress bars (8-char solid fill, `plan_tree.rs:487-514`)
- Column-aligned fixed layout: plan | prog | bar | delta | vfy | age (`plan_tree.rs:22-28`)

What is missing:
- **No dependency arrows between plans.** A plan in wave 2 that depends on plans A and B
  in wave 1 shows no visual link to A or B specifically. The user sees "Wave 2" but not
  "this plan depends on plan-foo and plan-bar."
- **No intra-wave task dependency edges.** Tasks within a plan have `depends_on` fields,
  but the task table in the right panel (`plans_view.rs:949-1001`) is a flat list with no
  dependency information shown.
- **No DAG structure at all.** The rendering is a grouped flat list, not a graph. Mori had
  the same limitation -- its `plan_tree.rs` also renders wave groups without dependency
  arrows.
- Mori's `wave_blockers` computation (`mori plan_tree.rs:177-228`) produces "after W{N}"
  labels for pending waves, showing which earlier wave blocks them. **Roko does not have
  this.** The blocker label was one of the few dependency hints in mori's UI.

**Files:**
- Wave computation: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` lines 4130-4210
- Wave tree render: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/plans_view.rs` lines 240-375
- Widget wave tree: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/plan_tree.rs` lines 228-367

---

## 2. Plan Status at a Glance: Colors, Icons, Status Text

**Rating: Good -- comprehensive and consistent**

The icon/color system is well-implemented across both `plans_view.rs` and `plan_tree.rs`:

| Status | Icon | Color | Where |
|---|---|---|---|
| Completed | checkmark (U+2713) | `theme.success` / `Theme::SAGE` | `plan_tree.rs:36-41`, `plans_view.rs:446-449` |
| Failed | X (U+2717) | `theme.danger` / `Theme::EMBER` | `plan_tree.rs:42-47`, `plans_view.rs:450-456` |
| Active | play (U+25B6) | `theme.warning` / `Theme::WARNING` | `plan_tree.rs:48-53`, `plans_view.rs:457-463` |
| Pending | circle (U+25CB) | `theme.muted` / `Theme::TEXT_GHOST` | `plan_tree.rs:54-60`, `plans_view.rs:464-468` |

Wave-level icons follow the same pattern with the addition of middle-dot (U+00B7) for
inactive waves.

Semantic color gradation is implemented for progress bars:
- `>=0.9` -> success/green
- `>=0.5` -> accent/blue
- `>=0.2` -> warning/yellow
- `<0.2` -> muted

This is consistent between `plans_view.rs:1302-1312` and `plan_tree.rs:462-464`
(`Theme::semantic_color`).

Phase abbreviations in the plan tree widget (`plan_tree.rs:832-850`) map 15 phase strings
to 4-char codes (prfl, strt, impl, vfy, mrge, done, fail, comp, test, revw, crit, vdct,
docs, cmit, gate). These appear as a suffix on active plan names.

**Minor issues:**
- The `plans_view.rs` version and the `plan_tree.rs` widget version diverge in their color
  logic. `plans_view.rs` uses `theme.accent` for active plans while `plan_tree.rs` uses
  `Theme::ROSE_BRIGHT`. These are likely the same color under the default theme but the
  code paths are unnecessarily different.
- Active plans that are 100% done but still running show `Theme::WARNING` in the widget
  (`plan_tree.rs:459-460`) -- good, this catches the "all tasks done, gates pending" case.

---

## 3. Task List Within a Plan: Readable and Scrollable

**Rating: Good with caveats**

The right panel in `plans_view.rs` renders a columnar task table:

```
 icon | task (name)      | status      | agent         | cost/budget
```

Column widths: 3 | Min(16) | 12 | Min(8) | 14 (`plans_view.rs:1003-1009`).

Strengths:
- Table header row with styled column names (`plans_view.rs:1011-1018`)
- Per-task cost/budget display with danger coloring when over budget (`plans_view.rs:966-997`)
- Selected row highlighting via `view_state.secondary_selected` (`plans_view.rs:960-963`)
- Agent assignment shown per-task

Weaknesses:
- **No scrolling of the task list.** The task section gets `Constraint::Min(0)` in the
  vertical layout (`plans_view.rs:901`) meaning it takes remaining space, but the `Table`
  widget is rendered without `StatefulWidget` / `TableState` scroll support. If there are
  more tasks than visible rows, they are simply clipped. This is a significant usability
  gap for plans with many tasks (30+ is common in roko's own plans).
- **No task dependency information.** The `depends_on` field exists on `TaskDef` but is not
  stored in `TaskEntry` (`state.rs:544-549`) and not rendered anywhere.
- **Task ID is truncated to 32 chars** (`plans_view.rs:979`) which may clip meaningful
  suffixes in long task IDs.
- **No task duration/elapsed display.** The task table shows cost but not per-task elapsed
  time.

The plan detail modal (`plan_detail.rs`) also has a task list, but as a scrollable
`Paragraph` with `scroll((scroll, 0))` -- so the modal version scrolls but the inline
right-panel version does not.

**Files:**
- Task table: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/plans_view.rs` lines 921-1019
- TaskEntry struct: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` lines 544-549

---

## 4. Wave Visualization: Clarity

**Rating: Good**

The wave rendering is clear and information-dense:

- Wave header line: collapse icon + status icon + "Wave N" + "(done/total)" + gradient
  progress bar + failed count + horizontal rule fill
  (`plans_view.rs:310-354`, `plan_tree.rs:289-347`)
- The gradient bar in `plan_tree.rs` uses an ocean gradient with heartbeat animation for
  active waves (`plan_tree.rs:308-318`). The `plans_view.rs` version uses a simpler
  single-color mini bar (`plans_view.rs:294-296`).
- Collapse/expand via arrow icons is visually clear
- Failed plan count in the wave header is highlighted in danger/ember color

Weaknesses:
- **No wave ETA.** The wave overview modal (`wave_overview.rs:21-26`) has `eta_secs` and
  `total_duration_secs` fields, but these are only populated when the modal is explicitly
  opened. The inline wave headers do not show ETA or total duration.
- **No "blocked by" indicator.** Mori's plan_tree had `wave_blockers` logic that labels
  pending waves with "after W{N}" to show which earlier wave gates them. Roko computes
  waves from plan dependencies but does not expose the blocking relationship in the UI.
- **No wave-level health indicators beyond failed count.** Mori tracked `wave_flailing`
  (plans with >= 5 retries or > 3 iterations) and `wave_warning` (moderate retry counts)
  with distinct warning icons. Roko only counts failed plans.
- The `plans_view.rs` wave logic derives collapse state from a combination of conditions
  (`plans_view.rs:265-266`): not explicitly collapsed AND (wave selected OR all done OR
  any active). This is reasonable but means a user who collapses a wave sees it re-expand
  when it becomes active.
- The `plan_tree.rs` widget uses `wave.expanded` from state directly, giving the user full
  control. The two implementations diverge on collapse semantics.

---

## 5. Dependency Arrows: Shown?

**Rating: Not implemented**

There are no dependency arrows, lines, or visual connections anywhere in the F2 view.

- Plan-to-plan dependencies are computed in `state.rs:4130-4163` to produce wave numbers,
  but the edge information is discarded after wave assignment. The `Wave` struct
  (`state.rs`, `plan_tree.rs:876`) stores only `index`, `plans`, `done`, `total`,
  `expanded` -- no dependency edges.
- Task-to-task dependencies exist in the `TaskDef` struct (`depends_on`, `depends_on_plan`
  fields) and are used in `state.rs:4165-4191` for wave computation, but `TaskEntry`
  (`state.rs:544-549`) does not carry dependency information at all.
- The operations page scaffold (`tui/pages/operations.rs:36`) references a "dag" widget
  described as "Task graph and dependency states" but this appears to be a placeholder.

This is the single largest visualization gap in the F2 view.

---

## 6. Plan Detail Modal: Information Completeness

**Rating: Good -- covers essentials, missing some operational detail**

The plan detail modal (`plan_detail.rs`) displays:
- Title, ID, Wave number (`plan_detail.rs:127-148`)
- Status with color-coded label (COMPLETE/FAILED/ACTIVE/PENDING) (`plan_detail.rs:75-83`)
- Phase label
- Task counts: done/total, failed count, failure sample (`plan_detail.rs:91-124`)
- Timing: elapsed duration, progress percentage (`plan_detail.rs:154-166`)
- Git context: branch, worktree path, last commit (`plan_detail.rs:174-195`)
- Change stats: files modified, insertions, deletions (`plan_detail.rs:198-209`)
- Scrollable task list with status coloring (`plan_detail.rs:247-276`)
- Footer with keybinding hints (`plan_detail.rs:220-228`)

The right panel detail in `plans_view.rs` shows even more:
- Plan name, status with icon, ID (`plans_view.rs:791-814`)
- Task counts + gate results summary (`plans_view.rs:815-839`)
- Progress bar with percentage (`plans_view.rs:840-853`)
- Cost/budget with projection and over-budget warning (`plans_view.rs:854-878`)
- Last error (from plan summary) (`plans_view.rs:879-889`)
- Columnar task table with cost (`plans_view.rs:921-1019`)
- Gate verify results section (`plans_view.rs:1022-1091`)
- Timing section: total, avg/done, current task, gates (`plans_view.rs:1093-1185`)

Missing from both:
- **No task dependency graph or blocked-by information**
- **No retry/iteration count** -- mori tracked `task_retries`, `spawn_retries`, `iteration`
  per plan; roko's `PlanEntry` does not have these fields
- **No agent output/log snippet** -- the user must switch to F4 (Agents) to see agent
  output
- **No link to PRD or source plan file** -- the plan detail does not reference where the
  plan came from or how to inspect the tasks.toml
- **No diff summary** -- the change stats (files/insertions/deletions) are present but
  there is no way to view the actual diff from the plan detail

---

## 7. Filter/Search: Discoverability and Functionality

**Rating: Good functionality, poor discoverability**

Filter is triggered by `/` key when focus is on the plan tree (`input.rs:868`). This enters
`InputMode::PlanFilter` (`app.rs:2120-2124`) and shows the filter indicator as `/pattern/`
in the plan tree header (`plan_tree.rs:122-132`).

The filter supports:
- Case-insensitive substring matching on plan ID and name (`state.rs:813-817`)
- Status prefix filters: `status:failed`, `status:active`, `status:done`, `status:pending`
  (`state.rs:783-798`)
- Enter accepts the filter and keeps it active (`app.rs:2126-2128`)
- Escape cancels and clears the filter (`app.rs:2130-2133`)
- The title bar shows filtered count: "Plans (completed/total, filtered/total filtered)"
  (`plan_tree.rs:106-110`)
- Selection index is clamped to the filtered list (`plan_tree.rs:101-105`)

Discoverability issues:
- **The `/` keybinding is only shown in the plan tree title when focused** (`plan_tree.rs:
  112-115` shows `[Enter:detail h/l:tree]` but not the `/` filter hint). The `plans_view.rs`
  version does not show any keybinding hints at all in the title (`plans_view.rs:109`).
- **No search icon or affordance** -- there is no visual indicator that filtering is
  available until the user presses `/`.
- **No fuzzy matching** -- the filter is strict substring only. Typing "auth" would not
  match a plan named "authentication-wiring" since it does contain "auth", but a typo like
  "atuh" would fail silently.
- The `plans_view.rs` left panel does not use the `PlanTreeFilter` at all -- it has no
  filter support. Only the `plan_tree.rs` widget (used on F1 Dashboard) supports filtering.
  **The F2 Plans view's own left panel is not filterable.**

**Files:**
- Filter state: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` lines 768-825
- Filter keybinding: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs` line 868
- Filter action: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs` lines 2120-2133

---

## 8. What Would Make Plan Management Feel Intuitive

### High-impact improvements (ordered by value):

1. **Scrollable task table in right panel.** The task list clips silently when there are
   more tasks than visible rows. Use `StatefulWidget` with `TableState` to add row-level
   scrolling. This is the most immediately irritating gap when inspecting large plans.

2. **Add filter support to the F2 plans_view.rs left panel.** Currently the `/` filter
   only works in the `plan_tree.rs` widget on the F1 Dashboard. The dedicated F2 Plans
   view has no filtering at all, which is backwards.

3. **Show "blocked by" on pending waves.** Port mori's `wave_blockers` logic to display
   "after W{N}" labels on pending waves. This takes <30 lines and immediately explains why
   a wave has not started.

4. **Per-task elapsed time in the task table.** The right panel task table shows cost but
   not duration. Adding a duration column helps identify stalled tasks.

5. **Task dependency display.** Carry `depends_on` from `TaskDef` into `TaskEntry` and
   render it as a "deps: T1, T3" column or as a hover/detail expansion. This closes the
   dependency visibility gap without requiring full graph rendering.

6. **Show retry/iteration counts.** Add `retries` and `iteration` fields to `PlanEntry`
   and display them in the right panel header. Plans that have been retried multiple times
   are operationally important.

7. **Keybinding cheat sheet row.** Add a bottom status bar to the F2 view showing
   available keybindings: `/ filter | Enter detail | h/l tree | r retry | Esc close`.

8. **Wave ETA inline.** The wave overview modal already has `eta_secs` -- surface it in
   the wave header line. Even a rough "~5m remaining" estimate per-wave helps planning.

### Medium-impact improvements:

9. **Link to plan source.** Show the `tasks.toml` path in the plan detail, or allow
   pressing a key to open it in `$EDITOR`.

10. **Agent output peek.** Allow pressing a key on a task row to see the last N lines of
    the agent's output without switching to F4.

11. **Reconcile the two plan tree implementations.** `plans_view.rs` (F2 left panel) and
    `plan_tree.rs` (F1 widget) have diverged in feature parity: the widget has filtering,
    gradient bars, heartbeat animation, column headers, data rain fill, scrollbar;
    the F2 view has cost display, gate results, timing section. Ideally the F2 view would
    use the widget for its left panel and add the right panel detail on top.

---

## 9. Mori Comparison

The mori reference plan tree (`apps/mori/src/tui/widgets/plan_tree.rs`, ~1078 LOC) is the
direct ancestor. Key differences:

| Feature | Mori | Roko plan_tree.rs | Roko plans_view.rs |
|---|---|---|---|
| Wave header | checkmark/play/dot icons | Same | Same |
| Wave progress bar | Ocean gradient + heartbeat | Same | Single-color mini bar |
| Wave blockers ("after W{N}") | Yes (lines 177-228, 353-358) | **No** | **No** |
| Wave health (flailing/warning) | Yes (lines 302-346) | Failed count only | Failed count only |
| Plan columns | plan, prog, bar, delta, vfy, age | Same 6 columns | plan, prog, bar (3 columns) |
| Data rain fill | Yes (lines 118-146) | Removed (line 176 comment) | No |
| Filter indicator | Yes | Yes | **No filter support** |
| Collapse state | User-toggled `wave_expanded` set | `wave.expanded` from state | Derived from selection/status |
| Plan detail | Separate modal | Same modal exists | Inline right panel (richer) |
| Cost/budget display | No | No | Yes (plans_view.rs right panel) |
| Gate results inline | Verify column | Same verify column | Full gate results section |
| Timing section | No | No | Yes (total, avg, current, gates) |
| Scrollbar | Custom buffer-direct | Same | Same |

Roko's `plans_view.rs` right panel is a significant improvement over mori, which had no
comparable inline detail view. Mori relied entirely on the plan detail modal. The cost/
budget display and timing section are new to roko.

However, roko lost mori's wave blocker labels and health indicators, and the F2 left panel
is a simpler implementation than the widget used on F1.

---

## 10. Proposals: ASCII DAG, Gantt Timeline, Dependency Arrows

### Proposal A: ASCII DAG Rendering

Display the plan dependency graph as an ASCII DAG in the right panel (or as a modal).
This would replace the flat task list with a topological layout:

```
  plan-core ----+----> plan-cli
                |
                +----> plan-serve ---> plan-deploy
                |
  plan-gate ----+
```

Implementation approach:
- Store plan-to-plan dependency edges alongside wave numbers in `TuiState` (currently
  discarded after wave computation in `state.rs:4130-4163`)
- Add a new `DagLayout` struct that computes node positions: x = wave number, y = index
  within wave
- Render using box-drawing characters: horizontal lines (U+2500), vertical lines (U+2502),
  corners (U+2514/U+250C), tees (U+251C/U+2524)
- Color edges by status: completed edges green, active edges yellow, pending edges dim
- This could be a toggle mode on the right panel (`d` for DAG view vs `t` for task list)

Estimated complexity: ~200-300 LOC for layout + rendering.

### Proposal B: Gantt-Style Timeline

Render a horizontal timeline showing plan execution spans:

```
  plan-core   [==========]
  plan-gate   [======]
  plan-cli              [=====----]
  plan-serve                 [===----]
  plan-deploy                      [---]
                |    |    |    |    |
               0m   5m  10m  15m  20m
```

Implementation approach:
- Use `started_at` and `elapsed_secs` from `PlanEntry` for completed/active plans
- Compute timeline scale from min(started_at) to now (for active) or max(completed_at)
- Render using block characters for completed segments, dashes for in-progress
- Overlay dependency arrows as vertical lines between bars
- This would work well as a modal (similar to wave overview) or as a third right-panel mode

Estimated complexity: ~250-350 LOC. Requires storing plan start/end timestamps, which
`PlanEntry` already partially has (`started_at`, `elapsed_secs`).

### Proposal C: Dependency Arrows in Plan Tree

Add visual dependency arrows within the existing plan tree, connecting plans to their
dependencies:

```
  Wave 0
   checkmark plan-core .............. 5/5 [========] done
   checkmark plan-gate .............. 3/3 [========] done
  Wave 1
   play plan-cli .................... 3/5 [=====---] impl
   |  depends on: plan-core, plan-gate
   circle plan-serve ................ 0/4 [--------] pending
      depends on: plan-core
```

This is the simplest approach: just add a dependency line below each plan that has
upstream dependencies. No graph layout needed.

Implementation approach:
- Carry `depends_on: Vec<String>` from the wave computation into `PlanEntry` or into
  the `Wave` struct
- In `render_plan_line()`, after the main plan line, add a dim-colored indented line
  showing "depends on: plan-a, plan-b" when dependencies exist
- Only show this on the selected plan or on all plans (configurable)

Estimated complexity: ~40-60 LOC. Low risk, high clarity improvement.

### Recommendation

Start with **Proposal C** (dependency annotation lines) as it is the lowest-effort,
highest-clarity improvement. Then add **Proposal A** (ASCII DAG) as a toggle mode on
the right panel for users who want the structural overview. **Proposal B** (Gantt) is
the most visually impressive but has the most data requirements and should come last.

---

## Summary of Gaps

| # | Gap | Severity | Fix effort |
|---|---|---|---|
| G1 | Task list in right panel does not scroll | High | Small (add TableState) |
| G2 | F2 left panel has no filter support | High | Small (use plan_tree.rs or add filter) |
| G3 | No dependency arrows or graph anywhere | Medium | Small-Medium (Proposal C) |
| G4 | No "blocked by" wave labels (mori regression) | Medium | Small (~30 LOC) |
| G5 | No per-task elapsed time in task table | Medium | Small (add column) |
| G6 | No retry/iteration counts on plans | Medium | Small (add fields) |
| G7 | Two divergent plan tree implementations | Low | Medium (reconcile or reuse widget) |
| G8 | Keybinding discoverability | Low | Small (add hint bar) |
| G9 | No wave ETA in inline headers | Low | Small (surface existing field) |
| G10 | Data rain fill removed from widget | Low | None (intentional cleanup) |
| G11 | No task dependency info in TaskEntry | Medium | Small (carry from TaskDef) |
| G12 | Wave health indicators (flailing/warning) missing | Low | Small (port from mori) |

---

## Implementation Status (2026-09-02 swarm)

F2 Plans view improvements (task #18): plan list, task tree, wave hierarchy, detail modal.
Plan detail enrichment (task #8): dependencies, accept/verify text, diff stats,
branch/worktree/commit, and per-plan elapsed time wired into plan detail modal.
