# 05 - Navigation and Information Architecture Audit

**Date:** 2026-09-01
**Scope:** `crates/roko-cli/src/tui/views/*.rs`, `crates/roko-cli/src/tui/widgets/*.rs`,
tab definitions, input handling, header/status bars, overall layout composition.

---

## 1. Tab Organization (F1-F10): Logical Grouping Assessment

### Current Layout

| F-key | Tab | Sub-views | Primary Concern |
|-------|-----|-----------|-----------------|
| F1 | Dashboard | Health, Mesh, Cost, Affect + 8 letter-key sub-tabs | Operational overview |
| F2 | Plans | DAG, Task, Waves | Plan execution detail |
| F3 | Agents | Output, Gates, Tokens (+ 7 role tabs) | Agent lifecycle |
| F4 | Git | Branches, Commits, Worktrees | Source control |
| F5 | Logs | Log, Signals, Errors | Observability |
| F6 | Config | Config, Providers, Models | System configuration |
| F7 | Inspect | Overview, Signals, Episodes, Knowledge, Cost/Model, 3-Panel | Deep inspection |
| F8 | Marketplace | Jobs, Detail, New Job | Job board |
| F9 | Atelier | PRDs, Plans | PRD workshop |
| F10 | Learning | Route, History, Efficiency | ML/routing data |

### Mori Reference (7 tabs)

Mori had: Dashboard, Plans, Agents, Git, Logs, Config, Inspect.
Roko added: Marketplace (F8), Atelier (F9), Learning (F10).

### Redundancy Issues

**FINDING R-01: Cost data appears in at least four places.**
- F1 Dashboard header bar: cost/budget utilization (line 334-352 of `header_bar.rs`)
- F1 Dashboard sub-tab "Agents": per-agent token burn + cost tables (dashboard_view.rs)
- F7 Inspect sub-view 0 "Overview": cost breakdown per model (context_view.rs line 78)
- F7 Inspect sub-view 4 "Cost/Model": dedicated `cost_by_model` widget (context_view.rs line 72)
- F6 Config sub-view 2 "Models": model comparison with cost column (config_view.rs line 582)
- F10 Learning sub-view 3 "Efficiency": avg cost per model + bar chart (learning_view.rs line 416)
- Status bar: spend/budget (status_bar.rs line 126)

The user encounters cost information in header, footer, F1, F6, F7, and F10 with slightly different aggregations each time. There is no single canonical cost dashboard.

**FINDING R-02: Model/provider health data is in three tabs.**
- F6 Config sub-view 1 "Providers": provider health table (config_view.rs line 514)
- F6 Config sub-view 2 "Models": model comparison with gate pass rate (config_view.rs line 582)
- F10 Learning sub-view 1 "Route": per-model stats table + sparklines (learning_view.rs line 124)
- F10 Learning sub-view 3 "Efficiency": per-model efficiency stats (learning_view.rs line 416)

Config/Providers and Learning/Efficiency show nearly identical data (model name, success rate, call counts) computed from the same `efficiency_events` source. The F6 version infers provider from model name; the F10 version groups by model slug directly. Both show pass rates.

**FINDING R-03: Verify/gate results span three locations.**
- F1 Dashboard sub-tab 3 "Verify" (`render_sub_gate`): gate results table + failure details
- F6 Config runtime section "Runtime: Verify Thresholds" and "Runtime: Verify Results"
- F7 Inspect has no dedicated gate sub-view, but the Overview shows C-Factor

**FINDING R-04: Plan data is split across F1, F2, and F9.**
- F1 Dashboard left panel: plan tree, phase compact, task progress
- F2 Plans: wave browser + plan detail with tasks
- F9 Atelier: PRD list with task progress per PRD

The F1 left panel duplicates F2's plan tree when plans are active. F9's task list is a third representation of the same task data, grouped by PRD slug.

### Missing Information

**FINDING M-01: No dedicated alerts/notifications tab.**
Warnings appear as a conditional 1-line bar below the header (header_bar.rs line 584). Toast notifications overlay the active tab via modals. There is no persistent alert history view. The Error Digest (F5 sub-view 2) aggregates errors but not warnings or system alerts.

**FINDING M-02: No feeds/triggers/recipes tab.**
The E27 (feeds/recipes) and E31 (triggers) subsystems are "complete" per CLAUDE.md but have no TUI surface. There is no way to view active feeds, trigger configurations, or recipe DAG status from the dashboard.

**FINDING M-03: No groups/connectivity tab.**
E28 (agent groups) and E29 (connectivity/relay) have no TUI surface for viewing group membership, pheromone state, or relay connection status.

**FINDING M-04: No safety/security overview.**
E34 (safety contracts) is "complete 8/8" but the TUI has no surface for viewing active safety policies, quarantine state, incident links, immune graph results, or corrigibility status. The `affect_view.rs` shows Daimon affect state but not safety state.

**FINDING M-05: No named-surface rendering.**
E37 (named surfaces) defines Workbench/Inbox/Canvas/Minimap/Autonomy projections with V2Surface mapping in `tabs.rs` (line 10-18, 62-71), but the actual rendering path ignores V2Surface entirely. `render_tab_content` dispatches on `Tab`, not `V2Surface`. The mapping exists as dead code.

---

## 2. Sub-tab Organization Within Each Tab

### F1 Dashboard: Two competing navigation systems

**FINDING S-01: F1 has two independent sub-navigation layers.**

The Dashboard view has a **region sub-view system** (number keys 1-4: Health, Mesh, Cost, Affect) defined in `views/mod.rs` lines 145-150, AND a separate **letter-key sub-tab system** (a/o/d/e/g/m/L/P: Agents, Output, Diff, Verify, Git, MCP, Learning, Procs) defined in `dashboard_view.rs` lines 37-46.

The region sub-views (1-4) replace the entire Dashboard content area. The letter sub-tabs only control the right panel detail view. These two systems are orthogonal but use the same real estate. Sub-view 4 (Affect) hijacks the full area (line 62-65), while sub-views 1-3 are documented but their rendering is not clearly separated from the letter sub-tab system.

This means the Dashboard effectively has **12 sub-views** (4 region + 8 letter), making it the most complex tab by far. A user pressing `4` sees a completely different layout than one pressing `a`.

**File:** `crates/roko-cli/src/tui/views/dashboard_view.rs` lines 37-46, 60-83
**File:** `crates/roko-cli/src/tui/views/mod.rs` lines 145-150

### F7 Inspect: Overloaded catch-all

**FINDING S-02: Inspect has 6 sub-views with no cohesive theme.**

Sub-views: Overview (token burn + cost + routing + alerts), Signals (signal DAG), Episodes (replay), Knowledge (neuro store), Cost/Model (dedicated cost table), 3-Panel Inspect (MCP + Learning + Prompt Stats).

This tab is the "everything else" drawer. The Overview sub-view alone has four quadrants (health + C-Factor, token burn by role, cost by model, cascade router + alerts). The 3-Panel Inspect sub-view (sub_tab 5) adds three more panes. The Cost/Model sub-view (sub_tab 4) duplicates data from the Overview's mid-right quadrant.

**File:** `crates/roko-cli/src/tui/views/context_view.rs` lines 58-90
**File:** `crates/roko-cli/src/tui/views/mod.rs` lines 176-183

### F8 Marketplace and F9 Atelier: Sparse sub-views

F8 has 3 sub-views (Jobs, Detail, New Job) but Detail and Jobs are really one master-detail view. F9 has 2 sub-views (PRDs, Plans) where PlanExplorer is just a fullscreen version of the right panel already visible in the default split. These could be simplified to single views with master-detail layout and no sub-tabs.

---

## 3. Panel Layout Assessment

### Layout patterns by tab

| Tab | Pattern | Split Ratios | Notes |
|-----|---------|--------------|-------|
| F1 Dashboard | Master-detail | 38% / 1px / 62% | Left panel only shown when plans active |
| F2 Plans | Master-detail | 31% / 69% | Wave list left, task detail right |
| F3 Agents | Master-detail | 32% / 68% | Agent roster left, output right |
| F4 Git | 2-column, 2-row | 35% / 65% | Branch tree + worktree + status left; commits + info right |
| F5 Logs | Single panel | 100% | Status bar + scrollable log |
| F6 Config | Single panel | 100% | Scrollable config editor (sub-tabs are fullscreen) |
| F7 Inspect | 4-quadrant | 20% / 40%+40% / 40% | Overview has health / token / cost / router |
| F8 Marketplace | Master-detail | 35% / 65% | Job list left, detail right |
| F9 Atelier | Stats + master-detail | 3-line stats + 40% / 60% | Stats bar + PRD list + plan detail |
| F10 Learning | Single/stacked | Variable | Sub-views use different layouts |

**FINDING L-01: Inconsistent master-detail split ratios.**
The left panel width varies: 38% (F1), 31% (F2), 32% (F3), 35% (F4/F8), 40% (F9). While minor visual differences, this creates a slightly different spatial expectation when switching tabs. Standardizing on 33% or 35% would give a more uniform feel.

**FINDING L-02: F1 Dashboard right panel is extremely dense.**
The Agents sub-tab has 4 vertical sections (parallel pool, route table, output panel, token/system strip) in the 62% right panel. On a standard 80-column terminal, this leaves ~50 columns for content, and the 4-section vertical layout compresses each section to 5-7 rows. Below ~30 rows terminal height, this becomes unusable.

**FINDING L-03: F7 Inspect Overview is a 4-quadrant layout that doesn't scale down.**
The layout uses fixed percentage splits: 20% top, 40% mid, 40% bottom, with a horizontal 50/50 in the mid section. On an 80x24 terminal, each quadrant gets roughly 40x5 cells -- barely enough for table headers. No responsive collapse path exists.

---

## 4. Information Duplication Across Tabs

### Concrete duplications

| Data | Where it appears | Identical? |
|------|-----------------|------------|
| Task progress (done/total) | F1 header bar, F1 left panel task_progress, F2 wave detail, F9 atelier stats bar, status bar | Same source, different aggregation |
| Cost/spend | F1 header, F1 sub-tab agents, F7 overview, F7 cost/model, F6 models, F10 efficiency, status bar | Same source, 7 views |
| Agent count/status | F1 header (in-flight count), F1 sub-tab agents (pool), F3 full view, F9 stats bar, status bar | Same source |
| Model pass rate | F6 models, F10 route, F10 efficiency | Nearly identical tables |
| Gate results | F1 sub-tab verify, F6 runtime sections | Same data |
| Git branch/status | F1 sub-tab git, F4 full view, status bar | F1 sub-tab is a compressed version of F4 |
| Elapsed time | F1 header bar, status bar | Identical |

**FINDING D-01: The F1 Dashboard sub-tab system creates mini-clones of other tabs.**
The 8 letter-key sub-tabs (Agents, Output, Diff, Verify, Git, MCP, Learning, Procs) are effectively compressed versions of F3, F5, F7, and F4. This design made sense in Mori where there were only 7 tabs and the Dashboard was the single control surface. With 10 tabs, it creates redundancy: the user can see Git info in F1 sub-tab "g" or F4, agent info in F1 sub-tab "a" or F3, learning info in F1 sub-tab "L" or F10.

---

## 5. Information That Is Hard to Find

**FINDING H-01: MCP connection state is buried.**
MCP connections appear in: (a) F1 header bar as a small `MCP:N` counter (header_bar.rs line 429), (b) F1 sub-tab "m" (MCP) in the dashboard right panel, and (c) F7 sub-view 5 "3-Panel Inspect". There is no first-class MCP tab or prominent sub-view. A user troubleshooting MCP issues must know to press F1 then `m`, or F7 then `6`.

**FINDING H-02: Experiments data is deeply nested.**
Prompt experiments appear in: (a) F6 Config runtime section "Runtime: Experiments" at the bottom of a scrollable list, and (b) F1 Dashboard sub-tab "Learning" (sub_tab 6). Finding experiment status requires scrolling to the bottom of F6 or knowing the `L` sub-tab exists.

**FINDING H-03: Dream consolidation has no TUI surface.**
The `widgets/dream_view.rs` file exists but is not referenced by any view's `render` function. There is no way to see dream consolidation status, journal entries, or archive data from the TUI. The file defines rendering functions but they appear to be wired only in some non-standard path (not found in the views/ dispatch).

**File:** `crates/roko-cli/src/tui/widgets/dream_view.rs` -- exists but unreferenced in views/

**FINDING H-04: Knowledge store data requires navigating to F7 sub-view 4.**
The knowledge browser is sub-view 4 of the Inspect tab (press F7 then `4`). The Inspect tab's default sub-view (Overview) shows token burn and cost data, not knowledge. A user looking for knowledge data has no visual cue that it lives in Inspect.

**FINDING H-05: System metrics are header-only.**
CPU, memory, disk, network, and FPS metrics appear only in the header bar (header_bar.rs lines 372-463). There is a `sys_metrics.rs` widget file but it is not referenced by any view. The `SysMetrics` widget exists but has no dedicated sub-view.

**File:** `crates/roko-cli/src/tui/widgets/sys_metrics.rs` -- exists but not rendered in any view

---

## 6. Information Hierarchy Assessment

### Header bar (most prominent, always visible)

The header bar packs 9 sections into a single line:
1. Health dot + name
2. Queue/plan name
3. Wave indicator
4. Progress bar with gradient
5. Plan count
6. ETA/elapsed/cost/tokens
7. System metrics (CPU/MEM/agents/gates/MCP/NET/DSK/FPS)
8. Active agent spinner
9. F-key strip

**FINDING IH-01: The header bar is overloaded.**
At 140 columns, all 9 sections fit. At 80 columns (`compact` mode), sections 6c (MCP/NET/DSK/FPS) and the percentage display are hidden (header_bar.rs lines 284, 427). But the remaining content still competes for ~35 characters after the F-key strip claims ~50 characters.

The most important runtime information (progress, cost, health) is crammed into a dense single line alongside system metrics and F-key labels. Cost appears as `$0.123` next to CPU percentage, making visual scanning difficult.

### Status bar (always visible at bottom)

The status bar duplicates: git info, progress count, health summary, cost/budget, and keybind hints. The cost/budget appears in both header and footer with identical data (header_bar.rs line 334 vs status_bar.rs line 126).

**FINDING IH-02: Header and status bar duplicate progress and cost.**
Both bars show done/total counts and cost/budget. The status bar adds git branch info and keybind hints; the header bar adds system metrics and agent spinner. These could be specialized: header for system state, footer for context/navigation.

### Tab content hierarchy

The F1 Dashboard is designed as the information hierarchy root, with its left panel showing the "what's happening" summary (plan tree + phase + tasks) and the right panel offering detail drilldowns. This is correct in principle. However:

**FINDING IH-03: The default Dashboard view shows Agents, not Health.**
The default sub-tab when entering F1 is sub_tab 0 = "Agents" (the letter-key system). The region sub-view 1 ("Health") from the number-key system exists conceptually but the actual default render path goes to `render_sub_agents` (dashboard_view.rs line 183). A new user sees agent pool data, not a health overview.

---

## 7. Breadcrumb/Context: Does the User Know Where They Are?

**FINDING BC-01: Active tab is indicated but not the active sub-view.**
The F-key strip in the header bar highlights the active tab with reversed colors (header_bar.rs line 534). But:
- The **region sub-view** (number-key 1-4) is not shown anywhere persistent. The user must remember which sub-view they activated.
- The **letter sub-tab** within F1 is shown as a 1-line bar at the top of the right panel (dashboard_view.rs line 196-262), which disappears when the user switches tabs.
- Sub-view labels are rendered by `SubView::bar_label()` (mod.rs line 243-257) but this bar is only shown in tabs that explicitly call it. Not all views render this bar.

**FINDING BC-02: No breadcrumb trail for deep navigation.**
When a user is in F7 > sub-view 4 > Knowledge Browse > selected item, there is no indicator of the navigation path. The tab bar shows "F7 Inspect" but not "Knowledge Browse" or the selected item name.

**FINDING BC-03: F10 Learning is missing from the F-key strip.**
The header bar F-key strip lists F1 through F9 (header_bar.rs lines 480-489) but omits F10 Learning entirely. A user scanning the header bar would not know F10 exists. This is a significant discoverability gap.

**File:** `crates/roko-cli/src/tui/widgets/header_bar.rs` lines 480-489

---

## 8. State: Can the User Tell What's Active/Selected/Focused?

### Focus zone indication

**FINDING ST-01: Focus zone is visually indicated through border/title color.**
Focused panels use `Theme::focused_border_style()` and `Theme::focused_title_style()` (bright accent border, highlighted title). Unfocused panels use `Theme::unfocused_*` or `theme.muted()`. This is consistent across most views:
- Git view: `if focused { Theme::focused_border_style() }` (git_view.rs line 107)
- Logs view: `if focused { Theme::focused_border_style() }` (logs_view.rs line 159-168)
- Dashboard: left panel panels check `matches!(tui_state.focus, FocusZone::PlanTree)` (dashboard_view.rs line 122)

**FINDING ST-02: Selection highlighting is inconsistent across views.**
- F2 Plans: selected row uses `theme.selection()` style (plans_view.rs)
- F3 Agents: selected agent row uses `theme.selection()` style
- F4 Git: selected branch uses `theme.selection()` (git_view.rs line 183)
- F5 Logs: selected row gets `theme.selection_background` bg + arrow marker (logs_view.rs line 210-215)
- F8 Marketplace: selected job uses `theme.selection()` (marketplace_view.rs line 193)
- F9 Atelier: selected PRD uses `theme.selection()` (atelier_view.rs line 286)

The selection style is consistent (using `theme.selection()`), but the selection **marker** varies: F5 Logs uses an arrow marker, other views rely solely on background color. This is minor but affects accessibility.

**FINDING ST-03: No visual indicator for input mode.**
When in `InputMode::Inject`, `InputMode::Filter`, `InputMode::LogSearch`, or `InputMode::ConfigEdit` (input.rs lines 18-33), the mode change is shown by either: (a) an input bar appearing at the bottom (`render_input_bar` in app.rs line 2812), or (b) the active field getting an underline cursor (config_view.rs line 267). However, there is no persistent mode indicator in the header or status bar. A user in Filter mode has no global visual cue that keystrokes are being captured as filter text rather than navigation.

**FINDING ST-04: Pause state is clearly indicated.**
The status bar shows "PAUSED" with a bold warning-colored badge (status_bar.rs line 68-75). This is one of the better state indicators in the TUI.

---

## 9. Comparison with Mori's 7-Tab Layout

### Structural comparison

| Aspect | Mori (7 tabs) | Roko (10 tabs) |
|--------|--------------|----------------|
| Tab count | 7 (F1-F7) | 10 (F1-F10) |
| Dashboard sub-tabs | 8 letter-key | 4 number + 8 letter (12 total) |
| Total sub-views | ~25 | ~36 |
| F-key strip | Complete (F1-F7 all visible) | Incomplete (F10 missing) |
| Primary concern | Plan execution monitoring | Plan execution + learning + marketplace + PRD workshop |

### What Mori got right that Roko should preserve

1. **Dashboard as control center**: The master-detail layout with plan tree left and multi-tab detail right is the correct pattern for monitoring active execution. Roko preserves this.

2. **7-tab cognitive budget**: Research suggests 7 +/- 2 items is the limit of working memory for categories. Mori's 7 tabs were at the sweet spot. Roko's 10 tabs push past this limit.

3. **Consistent master-detail**: Mori used master-detail consistently. Roko mostly follows this but breaks the pattern in F6 (single panel), F10 (variable layout), and F7 (4-quadrant).

### Where Roko diverges problematically

**FINDING MC-01: Tab proliferation dilutes cognitive budget.**
Three tabs (F8 Marketplace, F9 Atelier, F10 Learning) could be consolidated:
- **Marketplace** is a specialized job browser used infrequently during execution. It could be a sub-view of Dashboard or Atelier.
- **Learning** data (cascade router, efficiency) already appears as a Dashboard sub-tab ("L") and in F6 Config runtime sections. A dedicated tab duplicates this.
- **Atelier** (PRD workshop) is a development-time concern, not a runtime monitoring concern. It could be accessible via a modal or a Dashboard sub-view.

---

## 10. Reorganization Suggestions

### Suggestion 1: Consolidate to 8 tabs

| F-key | Tab | What it covers |
|-------|-----|----------------|
| F1 | Dashboard | Current layout (master-detail + 8 letter sub-tabs) |
| F2 | Plans | Current + absorb Atelier's PRD list as a sub-view |
| F3 | Agents | Current |
| F4 | Git | Current |
| F5 | Logs | Current |
| F6 | System | Config + Provider Health + Model Comparison + Learning + System Metrics |
| F7 | Inspect | Current but remove Cost/Model (it's in System) |
| F8 | Workshop | Atelier (PRDs) + Marketplace (Jobs) as two sub-views |

This reduces to 8 tabs. Learning's 3 sub-views merge into System (since they're about system tuning, not runtime monitoring). Marketplace and Atelier merge into a "Workshop" tab for development-time activities.

### Suggestion 2: Deduplicate cost surfaces

Designate **one** canonical cost view and make all others reference it:
- F7 Inspect sub-view "Cost/Model" becomes the canonical detailed cost view
- Header bar keeps the summary `$X.XX / $Y.YY` line
- Remove cost/model tables from F6 and F10 (replace with links/hints to F7)
- Remove `CostOverview` from F1 region sub-views (it adds nothing over the header bar)

### Suggestion 3: Add F10 to the F-key strip

Immediate fix: add `(" F10", Theme::BONE_DIM, "learn", Tab::Learning)` to the `fkey_items` vector in header_bar.rs line 480-489. This is a simple omission.

### Suggestion 4: Clarify the Dashboard sub-navigation

The dual number-key / letter-key system on F1 is confusing. Options:
- **Remove region sub-views 1-3** (Health, Mesh, Cost) from F1. They are mostly empty/placeholder. Keep only Affect (sub-view 4) as a full-screen override.
- **Or**: Make number keys control the left panel content (plan tree vs health vs mesh) while letter keys control the right panel. Document this clearly in the sub-tab bar.

### Suggestion 5: Surface unseen subsystems

Add sub-views for:
- **Feeds/Triggers** (E27/E31): as a sub-view of Config or Dashboard
- **Safety** (E34): as a sub-view of Inspect or a dedicated panel in Dashboard
- **Dreams** (wire `dream_view.rs`): as a sub-view of Inspect/Knowledge
- **System metrics** (wire `sys_metrics.rs`): as a sub-view of Config

### Suggestion 6: Persistent mode indicator

Add input mode to the status bar: when in Filter/Inject/Search mode, show a small badge like `[FILTER]` or `[SEARCH]` in the status bar between the heartbeat and progress sections.

---

## Summary of Findings

| ID | Severity | Category | Finding |
|----|----------|----------|---------|
| R-01 | Medium | Redundancy | Cost data in 7 separate surfaces |
| R-02 | Medium | Redundancy | Model/provider health in 3 tabs |
| R-03 | Low | Redundancy | Gate results in 3 locations |
| R-04 | Low | Redundancy | Plan data in F1, F2, F9 |
| M-01 | Medium | Missing | No alerts/notifications history |
| M-02 | Medium | Missing | No feeds/triggers/recipes surface |
| M-03 | Low | Missing | No groups/connectivity surface |
| M-04 | Medium | Missing | No safety/security overview |
| M-05 | Low | Missing | V2Surface mapping is dead code |
| S-01 | High | Sub-tabs | F1 has two competing navigation systems (12 sub-views) |
| S-02 | Medium | Sub-tabs | F7 Inspect is an overloaded catch-all (6 sub-views, no theme) |
| L-01 | Low | Layout | Inconsistent master-detail split ratios |
| L-02 | Medium | Layout | F1 right panel too dense for small terminals |
| L-03 | Medium | Layout | F7 Inspect Overview 4-quadrant doesn't scale down |
| D-01 | High | Duplication | F1 letter sub-tabs clone F3/F4/F5/F7/F10 content |
| H-01 | Low | Hidden | MCP state buried in sub-tabs |
| H-02 | Low | Hidden | Experiments data deeply nested |
| H-03 | Medium | Hidden | Dream view widget exists but is not wired |
| H-04 | Low | Hidden | Knowledge requires F7 then sub-view 4 |
| H-05 | Medium | Hidden | System metrics widget exists but is not wired |
| IH-01 | Medium | Hierarchy | Header bar overloaded (9 sections in 1 line) |
| IH-02 | Low | Hierarchy | Header and status bar duplicate progress/cost |
| IH-03 | Medium | Hierarchy | Default Dashboard shows Agents, not Health |
| BC-01 | Medium | Breadcrumb | Active sub-view not persistently indicated |
| BC-02 | Low | Breadcrumb | No breadcrumb trail for deep navigation |
| BC-03 | High | Breadcrumb | F10 Learning missing from F-key strip |
| ST-01 | Pass | State | Focus zone properly indicated via border color |
| ST-02 | Low | State | Selection marker inconsistent (arrow vs bg-only) |
| ST-03 | Medium | State | No persistent input mode indicator |
| ST-04 | Pass | State | Pause state clearly shown |
| MC-01 | Medium | Mori parity | Tab proliferation past cognitive budget (10 > 7+2) |

### Critical path items (High severity)

1. **S-01**: The dual number-key / letter-key navigation on F1 Dashboard is the single biggest usability confusion. A new user has no way to discover 12 sub-views accessed via two different key families.

2. **D-01**: The F1 letter sub-tabs (a/o/d/e/g/m/L/P) create compressed clones of other tabs. With 10 top-level tabs, this duplication is no longer justified by "everything in one place" -- it just fragments the user's mental model.

3. **BC-03**: F10 Learning is invisible in the F-key strip. Users who don't know it exists cannot discover it. Simple fix in `header_bar.rs` line 480.
