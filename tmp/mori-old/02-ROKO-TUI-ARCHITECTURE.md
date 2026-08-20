# Roko TUI Architecture Deep-Dive

**Date**: 2026-08-19
**Scope**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/`
**Total size**: ~44,095 LOC across 78 Rust files (34 top-level, 14 modals, 11 views, 22 widgets, 3 pages)

---

## 1. File Structure

### Top-Level Files (34 files)

| File | LOC | Purpose |
|------|-----|---------|
| `mod.rs` | 59 | Module root. Re-exports `App`, `Tab`, `TuiState`, `Theme`, `Event`, `Page`, etc. |
| `app.rs` | 4,576 | **The main shell.** `App` struct owns all state, the event loop, draw dispatch, key routing, background I/O channels, terminal lifecycle, signal handling, and adaptive frame rate. This is the god object. |
| `state.rs` | 5,290 | **`TuiState` -- the full mutable state container.** Every navigation cursor, scroll position, modal, agent row, plan entry, git data, log entry, config editor state, efficiency event, cascade router snapshot, process metric, knowledge entry, marketplace job, and PRD summary. Matches Mori's `RunState`. |
| `dashboard.rs` | 7,445 | **`DashboardData` -- the data model.** Loads all `.roko/` JSONL files (signals, episodes, efficiency, experiments, gate thresholds, cascade router, c-factor, knowledge, marketplace jobs, PRDs), caches incremental cursors, and exposes derived views. Also contains `DashboardScaffold` for text-mode output and `DashboardSummary`. |
| `tabs.rs` | 252 | `Tab` enum (F1-F10), `V2Surface` enum for named-surface mapping, fkey/label/index/next/prev methods. Well-tested. |
| `input.rs` | 1,605 | `InputMode` state machine (Normal/Inject/Filter/Confirm/ConfigEdit), `FocusZone` panel focus, `TuiAction` enum (72 action variants), `handle_key` dispatch with modal intercept priority. `LogFilterLevel` for log view filtering. |
| `event.rs` | 80 | Crossterm event polling wrapper: `Event` (Key/Mouse/Resize/Tick), `EventHandler` with configurable tick rate. |
| `theme.rs` | 490 | **ROSEDUST palette.** Named RGB constants (VOID, ROSE, BONE, DREAM, SAGE, EMBER, WARNING, TEXT, BG variants), dark/no-color/high-contrast modes, semantic helpers (`role_accent`, `phase_accent`, `semantic_color`), `Gradient` type with fire/ocean presets, `brighten` utility. |
| `layout.rs` | 116 | `centered_rect`, `responsive_outer_margin` (1-cell margin for terminals >= 120x50), `split_horizontal`, `split_vertical`. |
| `fs_watch.rs` | 256 | `.roko/` directory watcher using `notify` + `notify_debouncer_full`. 200ms debounce window. Falls back to 1s poll-based fingerprinting if `notify` fails. Emits `FsRefresh::Coalesced` signals. |
| `git_watch.rs` | ~280 | Git admin directory watcher (`.git/HEAD`, `.git/refs/`, worktree indirection). 500ms debounce. Same notify+poll fallback pattern as `fs_watch`. Emits `GitRefresh::Coalesced`. |
| `ws_client.rs` | ~460 | WebSocket client for live agent streaming. `AgentStreamClient` connects to `roko-serve` event bus, filters for one agent, parses `StreamChunk` variants (Connected/Text/Reasoning/ToolCall/Usage/Error/Done/Disconnected). Exponential backoff reconnection (1s-30s). |
| `postfx.rs` | 920 | **Post-processing visual effects on raw ratatui `Buffer`.** `VizContext` snapshot for NervViz overlays. Functions: `dim_overlay`, `modal_glow`, `nerv_viz_overlay` (gradient vignette driven by task/plan progress, context pressure, token rate), `floating_particles`, various blend modes (additive, screen). |
| `postfx_pipeline.rs` | ~160 | Pipeline orchestrator: calls `nerv_viz_overlay` + `floating_particles` + `dim_overlay` per frame based on `EffectsConfig` flags. |
| `atmosphere.rs` | 165 | `Atmosphere` struct: tracks elapsed time and frame count. Provides `breathing_brightness` (sine wave oscillation), `heartbeat` (double-pulse pattern), `spinner`/`spinner_ethereal` animation characters. Full-frame bloom post-processing pass. |
| `effects_config.rs` | ~240 | `EffectsPreset` (Off/Minimal/Full), `EffectsConfig` with flags for `screen_postfx`, `nerv_viz`, `particles`. Loads from `roko.toml` `[tui]` section. |
| `segment.rs` | ~400 | Semantic parsing of agent output into `SegmentKind` (Thinking/Heading/ToolUse/Code/Success/Error/Blank/TurnMarker). `CachedRender` for incremental re-parsing. Styles each segment with ROSEDUST colors. |
| `ansi.rs` | ~180 | ANSI escape code parser. Converts ANSI-colored terminal output into ratatui `Span`s with RGB styles. |
| `scroll.rs` | ~80 | `ScrollAccel`: keyboard scroll acceleration for held-key scrolling. Ramps speed after repeated presses. |
| `cursors.rs` | ~130 | Incremental file cursors: `SignalCursor` (engrams.jsonl), `EpisodeCursor` (episodes.jsonl), `EventLogCursor` (events.json). Track file offsets for delta reads. |
| `jsonl_cursor.rs` | ~200 | Generic incremental JSONL reader. Tracks byte offset, only reads appended content on `tick()`. |
| `jsonl_tailer.rs` | ~260 | Generic typed incremental JSONL tailer wrapping `JsonlCursor`. Deserializes new lines into `Vec<T>`. Used for efficiency events and c-factor history. |
| `hit_test.rs` | ~200 | `HitZones` struct: computes screen regions from layout math for mouse click routing. Maps (x,y) coordinates to `FocusZone` variants. |
| `display_utils.rs` | ~90 | Formatting helpers: `shorten_model` (truncates model slugs), `event_model_slug` (extracts model from efficiency events). |
| `util.rs` | ~30 | `truncate_middle` function: truncates strings with "..." in the middle. |
| `config_meta.rs` | ~830 | Configuration editor metadata. `ConfigItem` enum (SectionHeader/Field/RuntimeSection), `ConfigFieldKind` enum. `build_flat_items` reads `roko.toml` and produces editable field list. Supports bool toggles, enum cycles, and text editing. |
| `approval_ipc.rs` | ~60 | `ApprovalChannel` and `ApprovalRequest` for orchestrator-to-TUI command approval flow. |
| `task_outputs.rs` | ~230 | `TaskOutputCursors`: keyed incremental file readers for per-task output in `.roko/task-outputs/`. |
| `verdicts.rs` | ~470 | `VerdictsAggregator`: incremental reader for gate verdict signals from `.roko/substrate/`. Parses verdict kind, outcome, and confidence into summary rows. |
| `dashboard_gen.rs` | ~140 | `DashboardGenerationState` and `DurableDashboardGenerationCounter`: monotonic generation tracking for cache invalidation. |

### Views Directory (11 files)

| File | LOC | Tab | Purpose |
|------|-----|-----|---------|
| `mod.rs` | 297 | -- | `SubView` enum (30 variants), `ViewState` struct, `render_tab_content` dispatch function |
| `dashboard_view.rs` | 2,378 | F1 | Master-detail layout: left 38% (plan tree + phase compact + task progress), right 62% (8 sub-tabs: Agents/Output/Diff/Verify/Git/MCP/Learning/Procs), bottom ribbon |
| `plans_view.rs` | 1,429 | F2 | Wave browser: left 35% (pipeline header + wave/plan tree), right 65% (selected plan detail with tasks, gate results, timing) |
| `agents_view.rs` | 1,296 | F3 | Agent roster: left 32% (roster + summary + token sparkline), right 68% (role tabs + scrollable output) |
| `git_view.rs` | 789 | F4 | Git browser: left 35% (branch tree + worktree list), right 65% (commit graph + branch info). Collects git data via `Command` subprocess calls |
| `logs_view.rs` | ~400 | F5 | Multi-source log tail: signals + episodes + efficiency events + gate results. Level-based coloring, level filter toggles, auto-tail mode |
| `config_view.rs` | ~500 | F6 | Config editor: scrollable field list, inline editing, provider health sub-view, model comparison sub-view |
| `context_view.rs` | 1,110 | F7 | Token burn per role, cost breakdown per model, cascade router decisions, C-Factor display. Sub-views: Signal DAG, Episode Replay, Knowledge Browse |
| `marketplace_view.rs` | ~550 | F8 | Job board: left 35% (job list), right 65% (job detail). Job creation form sub-view |
| `atelier_view.rs` | ~500 | F9 | PRD workshop: left 40% (PRD list), right 60% (plan detail/task list). Status badges (IDEA/DRFT/PUBL/PLAN) |
| `learning_view.rs` | ~500 | F10 | Cascade router: stage indicator + model stats table + selection bar chart. History timeline. Per-model efficiency breakdown |

### Widgets Directory (22 files)

| File | LOC | Purpose |
|------|-----|---------|
| `mod.rs` | 19 | Module declarations |
| `rosedust.rs` | 10 | Compatibility shim re-exporting `Theme` colors as `MoriTheme` |
| `header_bar.rs` | ~400 | Top header bar: heartbeat + name, wave indicator, fire-gradient progress bar, plan count, ETA/elapsed/cost/tokens, sys metrics (CPU/MEM), active agent spinner, F-key strip |
| `status_bar.rs` | ~350 | Bottom status bar: git info (branch/commit/age), heartbeat + pause indicator, plan progress + health summary, cost/budget utilization, context-sensitive keybind hints |
| `plan_tree.rs` | ~600 | Collapsible plan tree: wave grouping, inline progress bars, fixed-column layout (progress/bar/delta/verify/age), scrollbar, data-rain fill for empty space |
| `plan_list.rs` | ~250 | Simpler plan list widget (legacy scaffold) |
| `task_progress.rs` | ~400 | Task checklist widget with status icons, phase labels, progress bars |
| `wave_progress.rs` | ~200 | Wave progress ribbon: proportional segments per wave with animated ocean gradient fill |
| `wave_bar.rs` | ~150 | Compact wave indicator bar |
| `phase_timeline.rs` | ~300 | Phase transition timeline with duration labels |
| `phase_compact.rs` | ~250 | Compact inline phase indicator |
| `token_sparkline.rs` | ~250 | Token burn sparkline: efficiency summary, braille sparkline, model tier distribution |
| `context_gauge.rs` | ~200 | Context window usage gauge with gradient fill |
| `diff_panel.rs` | ~300 | Git diff panel with syntax-colored +/- lines |
| `sys_metrics.rs` | ~200 | System metrics widget: CPU/MEM bars with semantic coloring |
| `error_digest.rs` | ~250 | Error ring buffer display with timestamps |
| `dream_view.rs` | ~350 | Dream consolidation state visualization |
| `parallel_pool.rs` | ~200 | Parallel agent pool status overview |
| `branch_tree.rs` | ~250 | Git branch tree with current-branch highlighting |
| `braille.rs` | ~150 | Braille sparkline renderer: 2x horizontal density using Unicode braille characters |
| `tab_bar.rs` | ~150 | Tab bar rendering with active/inactive styling |
| `status_badge.rs` | ~100 | Compact status badge (done/active/failed/pending) |

### Modals Directory (14 files)

| File | LOC | Purpose |
|------|-----|---------|
| `mod.rs` | 305 | `ModalState` enum (12 variants), `render_modal` dispatch, `render_modals` top-level entry, `modal_area` sizing |
| `help.rs` | ~300 | Global help overlay: keybinding reference, tab descriptions |
| `quit.rs` | ~50 | Quit confirmation dialog |
| `approval.rs` | ~150 | Agent command approval dialog (approve/reject/approve-all) |
| `confirm.rs` | ~100 | Generic destructive action confirmation |
| `inject.rs` | ~150 | Free-text injection modal with cursor and text editing |
| `plan_detail.rs` | ~250 | Plan detail browser with scrollable task list |
| `task_detail.rs` | ~300 | Task detail browser: assigned agents, gate results, output |
| `task_picker.rs` | ~200 | Task selection picker with search |
| `wave_overview.rs` | ~150 | Wave progress modal with per-plan timing |
| `queue_overview.rs` | ~200 | Milestone queue browser |
| `agent_pool_modal.rs` | ~200 | Full agent roster modal |
| `batch_review.rs` | ~200 | Batch-pause review modal with task results |
| `notification.rs` | ~200 | Toast notification overlay in bottom-right corner |

### Pages Directory (3 files)

| File | LOC | Purpose |
|------|-----|---------|
| `mod.rs` | 424 | `PageId` enum (16 variants), `PageScaffold`, `WidgetScaffold`, `PageRegistry`, `Page` trait. Text-mode rendering infrastructure |
| `efficiency.rs` | 258 | Efficiency page data builders for Health/Trends/Correlations/Parameters scaffold pages |
| `operations.rs` | 162 | Operations page builders for AgentStatus/PlanView/LogView scaffold pages |

---

## 2. The F1-F10 Tab System

10 tabs, each bound to a function key:

| Key | Tab | Label | Sub-views (1-N keys) |
|-----|-----|-------|---------------------|
| F1 | Dashboard | "Dashboard" | Health, Mesh Status, Cost |
| F2 | Plans | "Plans" | DAG View, Task Detail, Wave Progress |
| F3 | Agents | "Agents" | Output Stream, Gate Results, Token Burn |
| F4 | Git | "Git" | Branch Tree, Commit Graph, Worktrees |
| F5 | Logs | "Logs" | Filtered Log, Signal Stream |
| F6 | Config | "Config" | Config Editor, Provider Health, Model Comparison |
| F7 | Inspect | "Inspect" | Overview, Signal DAG, Episode Replay, Knowledge Browse |
| F8 | Marketplace | "Marketplace" | Job List, Job Detail, Create Job |
| F9 | Atelier | "Atelier" | PRD Workshop, Plan Explorer |
| F10 | Learning | "Learning" | Route, History, Efficiency |

Tab/Right-arrow cycles forward (wraps). Shift+Tab/Left-arrow cycles backward.
Number keys 1-9 select sub-views within the current tab.

Additionally, the F1 Dashboard view has its own 8 detail sub-tabs accessible via letter keys:
`a` (Agents), `o` (Output), `d` (Diff), `e` (Verify), `g` (Git), `m` (MCP), `L` (Learning), `P` (Procs).

---

## 3. How Each Tab Is Implemented

### F1 Dashboard (`dashboard_view.rs`, 2,378 LOC)

**Layout**: Master-detail. Left panel 38% width, right panel 62%. Bottom ribbon.

**Left panel** (3 vertical sections):
- Plan tree widget (`widgets/plan_tree.rs`): collapsible wave groups, inline progress bars, status icons, scrollbar
- Phase compact widget (`widgets/phase_compact.rs`): current phase indicator for selected plan
- Task progress widget (`widgets/task_progress.rs`): task checklist for selected plan

**Right panel** (8 sub-tabs, letter-key activated):
- **Agents**: agent roster table (role, model, status, tokens, cost)
- **Output**: scrollable agent output with ANSI + semantic segment parsing
- **Diff**: git diff panel with +/- coloring
- **Verify**: gate results table with pass/fail/threshold columns, verdict sparklines
- **Git**: git summary lines (branch, commit, status)
- **MCP**: active tool definitions, MCP server status
- **Learning**: cascade router state, experiment summaries, efficiency snapshots
- **Procs**: live process metrics (PID, CPU%, MEM, state, uptime) with braille sparklines

**Bottom ribbon**: wave progress bar + token sparkline + sys metrics (CPU/MEM).

**Data sources**: `TuiState` for agent/plan/task data, `DashboardData` for efficiency/gate/signal data, background `SysSnapshot` for CPU/MEM.

**What actually works**: Plan tree renders real plan data from `.roko/state/state-snapshot.json`. Agent roster populates from `DashboardSnapshot` events. Gate results render from signal parsing. Git summary from background git commands. Token sparkline from efficiency events. System metrics from background `sysinfo` thread. The Output sub-tab renders real agent output via `segment.rs` semantic parsing and `ansi.rs`. Diff panel reads real `git diff` output.

### F2 Plans (`plans_view.rs`, 1,429 LOC)

**Layout**: Left 35% (pipeline header + plan summary + wave tree), right 65% (task list + gate results).

**Left panel**: Pipeline header shows total progress (X/Y tasks). Selected plan summary. Wave tree shows plans grouped by execution wave with gradient progress bars, status icons (check/cross/play/circle), and elapsed time.

**Right panel**: Task list for selected plan with status, dependencies, assignees. Gate results for selected task.

**Data sources**: `TuiState.plans`, `TuiState.current_task_checklist`, `TuiState.plan_summaries`.

**What works**: Real plan data renders. Task status parsing handles many status strings (implementing, gating, verifying, reviewing, etc.). Progress bars use fractional block characters for smooth rendering.

### F3 Agents (`agents_view.rs`, 1,296 LOC)

**Layout**: Left 32% (agent roster + summary + sparkline), right 68% (role tabs + output).

**Left panel**: Agent roster with role-colored labels, status chips, context gauges. Summary line with active/idle/done/failed counts. Token sparkline.

**Right panel**: 7 fixed role tabs (implementer, strategist, architect, auditor, critic, conductor, researcher). Shows output for selected agent with ANSI color support. WebSocket streaming support via `ws_client.rs`.

**Data sources**: `TuiState.agents` (populated from `DashboardSnapshot`), `AgentStreamClient` for live output, efficiency events for token data.

**What works**: Agent roster populates from live snapshot events. WebSocket streaming connects to `roko-serve` event bus for real-time output. Role-based coloring uses the ROSEDUST palette.

### F4 Git (`git_view.rs`, 789 LOC)

**Layout**: Left 35% (branch tree + worktree list), right 65% (commit graph + branch info).

**Data sources**: Real git commands (`git branch`, `git log`, `git worktree list`, `git status`) executed by background thread triggered by `GitWatchHandle`.

**What works**: Branch tree populates from `git branch`. Worktree list from `git worktree list`. Commit graph from `git log --graph --oneline`. Current branch highlighting. Data refreshes on debounced git metadata changes. Zero I/O on the render path (background thread populates `GitViewData`).

### F5 Logs (`logs_view.rs`, ~400 LOC)

**Layout**: Status bar (2 lines) + scrollable log body.

**Data sources**: Unified log entries from `TuiState.unified_log_entries()` combining:
- Recent signals from `.roko/engrams.jsonl`
- Episodes from `.roko/episodes.jsonl`
- Efficiency events from `.roko/learn/efficiency.jsonl`
- Gate result summaries
- Event log from `.roko/state/events.json`

**What works**: Level-based coloring (info=sage, warn=warning, error=ember, debug=dim). Level filter toggles (I/W/E/D keys). Auto-tail mode. Source type prefixes. Entry counts in status bar.

### F6 Config (`config_view.rs`, ~500 LOC)

**Layout**: Single-panel scrollable config editor with inline editing.

**Sub-views**: Config Editor (sub-tab 0), Provider Health (sub-tab 1), Model Comparison (sub-tab 2).

**Data sources**: `roko.toml` parsed by `config_meta.rs` into editable fields. Provider health from `.roko/learn/provider-health.json`. Model metrics from efficiency events.

**What works**: Config fields render with section grouping. Provider health shows circuit state (Closed/HalfOpen/Open), success rates, latency. Model comparison shows per-model token/cost/speed stats. Config editing is partially wired (cursor movement, toggle, cycle work; persistence needs verification).

### F7 Inspect (`context_view.rs`, 1,110 LOC)

**Layout**: Four sections: system health (top 20%), token burn per role (mid-left 40%), cost per model (mid-right 40%), cascade router + alerts (bottom 40%).

**Sub-views**: Overview (default), Signal DAG, Episode Replay, Knowledge Browse.

**Data sources**: Efficiency events for token/cost aggregation, cascade router for routing decisions, c-factor for system health.

**What works**: Token burn table renders per-role aggregates. Cost table renders per-model aggregates. C-Factor gauge displays. Knowledge browse reads from neuro store. Signal DAG and Episode Replay are partially populated -- they render data when present but the sub-views are sparse.

### F8 Marketplace (`marketplace_view.rs`, ~550 LOC)

**Layout**: Left 35% (job list) | right 65% (job detail). Job creation form sub-view.

**Data sources**: `.roko/jobs/*.json` files loaded by `DashboardData`.

**What works**: Job list renders with status icons (pending=circle, active=play, done=check, failed=cross). Job detail shows full metadata. Job creation form has editable fields (title, description, type, budget). j/k navigation. The data loading is real -- it scans the jobs directory.

### F9 Atelier (`atelier_view.rs`, ~500 LOC)

**Layout**: Top 3-line stats bar + left 40% (PRD list) + right 60% (plan detail).

**Data sources**: `roko_core::PrdSummary` from `.roko/prd/`, task lists from plan directories.

**What works**: PRD list with status badges (IDEA/DRFT/PUBL/PLAN). Stats bar shows PRD/plan/task counts. Task list for selected plan with status icons. Plan explorer sub-view shows plan directory contents.

### F10 Learning (`learning_view.rs`, ~500 LOC)

**Layout**: Three sub-views: Route (default), History, Efficiency.

**Route sub-view**: Stage indicator (3-tier visual), model stats table (model, successes, failures, trials, confidence), selection frequency bar chart.

**History sub-view**: Stage transition timeline.

**Efficiency sub-view**: Per-model cost/pass breakdown.

**Data sources**: `TuiState.cascade_router` from `.roko/learn/cascade-router.json`, efficiency events.

**What works**: Route overview renders cascade router state with real data. Bar chart uses `ratatui::BarChart`. Stage indicator shows current tier. Empty state has placeholder message. Model stats table populates from router confidence data.

---

## 4. What Actually Works vs What's Stubbed/Broken

### Solidly Working

1. **Tab navigation** (F1-F10, Tab/Shift-Tab, arrow keys) -- fully wired, well-tested
2. **Plan tree** -- renders real plan data from durable Runner projection
3. **Agent roster** -- populates from DashboardSnapshot events
4. **Git view** -- real git commands via background thread, debounced refresh
5. **Log view** -- multi-source log tail with filtering and auto-tail
6. **File watchers** -- `.roko/` and `.git/` both watched with debounce + poll fallback
7. **System metrics** -- background `sysinfo` thread for CPU/MEM
8. **Token tracking** -- efficiency events parsed and displayed in sparklines/gauges
9. **Modal system** -- 12 modal types all render, dispatch correctly
10. **ROSEDUST theme** -- consistent palette, dark/no-color/high-contrast modes
11. **Keyboard acceleration** -- scroll acceleration for held keys
12. **PostFX pipeline** -- dim overlay, modal glow, NervViz, floating particles all render
13. **Adaptive frame rate** -- ~60fps active, ~20fps idle
14. **Terminal cleanup** -- panic hook, signal handler (SIGINT/SIGTERM/SIGHUP), cleanup guard
15. **Marketplace job list** -- reads from `.roko/jobs/` directory
16. **PRD workshop** -- reads from `.roko/prd/` directory
17. **Cascade router display** -- reads from `cascade-router.json`
18. **WebSocket agent streaming** -- connects to `roko-serve`, reconnects with backoff

### Partially Working / Rough Edges

1. **Config editor** -- cursor movement and field display work, but actual save/persistence path is unclear. `ConfigSave` action exists but the round-trip back to `roko.toml` is not obviously verified
2. **Agent output streaming** -- WebSocket client works but requires a running `roko-serve` instance. Falls back to polling `.roko/task-outputs/` files
3. **Signal DAG sub-view** (F7 sub-tab 1) -- renders if signal data exists but is a sparse list, not a visual DAG
4. **Episode Replay sub-view** (F7 sub-tab 2) -- renders episode list but no actual "replay" functionality
5. **Knowledge Browse sub-view** (F7 sub-tab 3) -- reads from neuro store but browsing is basic list display
6. **Hit testing** -- `HitZones` computed but mouse support is gated behind `capture_mouse` flag (default false)
7. **Inject modal** -- text input works but actually sending the injection to an agent requires the orchestrator approval channel to be connected
8. **Connected mode vs standalone** -- two paths (standalone `roko dashboard` replays from disk; connected mode receives live `DashboardSnapshot` events). The connected path is more complete but requires an active plan run

### Stubbed / Not Really Useful Yet

1. **Named V2 surfaces** (`V2Surface` enum) -- the tab-to-surface mapping exists but the actual named-surface TUI rendering is explicitly called out as a product residual
2. **Mesh Status sub-view** (F1 sub-tab 2) -- renders available data but "agent mesh" concept is largely aspirational
3. **Job creation form** (F8 sub-tab 3) -- form renders with fields but `SubmitJob` action's backend persistence is not verified
4. **Dream view widget** -- renders dream consolidation state but the dream subsystem itself has limited runtime integration
5. **Page scaffold system** (`PageId`, `PageScaffold`, `PageRegistry`) -- this is the legacy text-mode rendering infrastructure. It's separate from the ratatui tab system and only used by `roko dashboard --text` mode. Many of the 16 `PageId` variants (Correlations, Parameters, Experiments, Optimizer, Dreams) render placeholder widget lists, not actual visualizations

---

## 5. Data Flow

### Data Loading Pipeline

```
.roko/ directory (JSONL/JSON files)
    |
    v
DashboardData::load_best_effort()  -- or --  StateHub::bootstrap_from_workdir()
    |                                              |
    | (standalone mode)                   (connected mode)
    v                                              v
Incremental cursors:                    DashboardSnapshot events
  SignalCursor (engrams.jsonl)               |
  EpisodeCursor (episodes.jsonl)             v
  EventLogCursor (events.json)          watch::Receiver<DashboardSnapshot>
  IncrementalTailer (efficiency.jsonl)       |
  IncrementalTailer (c-factor.jsonl)         v
  TaskOutputCursors (task-outputs/)     TuiState.update_from_snapshot()
    |
    v
DashboardData fields populated
    |
    v
TuiState.update_from_snapshot(&data)
    |
    v
Views read from TuiState + DashboardData (zero I/O)
```

### Background I/O Channels

The render path does zero I/O. All expensive work runs on background threads:

1. **System metrics thread** (`tui-sys-metrics`): polls `sysinfo::System` every ~2s, sends `SysSnapshot` via `tokio::sync::watch`
2. **Git collection thread** (`tui-git-collect`): runs git commands, sends `GitBgData` via `std::sync::mpsc::sync_channel`
3. **Filesystem watcher** (`FsWatchHandle`): `notify` debouncer or poll fallback, sends `FsRefresh::Coalesced`
4. **Git watcher** (`GitWatchHandle`): same pattern for `.git/` directory
5. **Agent topology fetch** (one-shot): fetches from `roko-serve` `/api/agents/topology`
6. **WebSocket agent streams** (`AgentStreamClient`): per-agent tokio tasks consuming the event bus

### Refresh Cycle

On each tick (every 16ms at ~60fps):
1. `drain_shutdown_signal()` -- check for external shutdown
2. `drain_snapshot_channel()` -- check for live DashboardSnapshot updates
3. `drain_approval_requests()` -- check for orchestrator approval requests
4. `drain_background_channels()` -- drain sys metrics, git data, fs/git watcher signals
5. `expire_notifications()` -- age out old toast notifications
6. Adaptive frame rate: skip draws when idle (user inactive 3s + no active agents)

On filesystem change events:
1. `DashboardData::tick()` re-reads changed files via incremental cursors
2. `TuiState::update_from_snapshot()` propagates changes

---

## 6. Key Binding System

### Architecture

Key dispatch follows a strict priority chain in `input::handle_key`:

1. **Ctrl+C** -- always quits (bypasses all modals)
2. **Modal intercepts** -- if a modal is active, keys route to modal-specific handler
3. **Input mode intercepts** -- Inject, Filter, ConfigEdit, Confirm modes capture text
4. **Normal mode** -- per-tab key bindings

### Global Keys (Normal Mode)

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit (opens confirm dialog) |
| `F1`-`F10` | Switch tab |
| `Tab` / `Right` | Next tab |
| `Shift+Tab` / `Left` | Previous tab |
| `1`-`9` | Switch sub-view within tab |
| `?` | Show help modal |
| `j` / `Down` | Select next item |
| `k` / `Up` | Select previous item |
| `h` / `Shift+Tab` | Focus previous panel |
| `l` / `Tab` | Focus next panel |
| `Enter` | Expand/collapse or drill in |
| `Backspace` | Drill out |
| `v` | Cycle visual effects preset |
| `x` | Toggle screen PostFX |
| `p` | Toggle pause |
| `r` | Refresh |
| `Space` | Open task picker |
| `w` | Show wave overview |
| `W` | Show queue overview |

### Dashboard-Specific Keys (F1)

| Key | Action |
|-----|--------|
| `a`/`o`/`d`/`e`/`g`/`m`/`L`/`P` | Switch detail sub-tab |
| `J`/`K` | Scroll focused panel |
| `PgUp`/`PgDn` | Page scroll |
| `Home`/`End` | Jump to start/end |
| `G` | Scroll to end (tail) |

### Agents-Specific Keys (F3)

| Key | Action |
|-----|--------|
| `t` | Toggle agent topology |
| `1`-`7` | Switch role tab |

### Config-Specific Keys (F6)

| Key | Action |
|-----|--------|
| `Space` / `Enter` | Toggle bool / start edit |
| `[` / `]` | Cycle enum left/right |
| `s` | Save config |
| `Esc` | Cancel edit |

### Logs-Specific Keys (F5)

| Key | Action |
|-----|--------|
| `I`/`W`/`E`/`D` | Toggle level filter |
| `A` | Show all levels |
| `G` | Jump to end (tail) |
| `/` | Start filter mode |

### Modal Keys

| Context | Key | Action |
|---------|-----|--------|
| Help | `Esc`/`?`/`q` | Close |
| Quit confirm | `y` | Confirm quit |
| Quit confirm | `n`/`Esc` | Cancel |
| Approval | `a` | Approve |
| Approval | `A` | Approve all |
| Approval | `r`/`Esc` | Reject |
| Inject | `Enter` | Submit |
| Inject | `Esc` | Cancel |
| Any modal | `Esc` | Close |

---

## 7. Real-Time Update Mechanism

### Three Update Paths

**Path 1: Standalone `roko dashboard`**
- `App::new()` creates an in-process `SharedStateHub`
- Bootstraps from workdir (reads `.roko/state/state-snapshot.json`)
- Replays `events.jsonl` into the snapshot
- Filesystem watcher triggers `DashboardData::tick()` which re-reads JSONL files via incremental cursors
- Updates flow: files -> DashboardData -> TuiState -> render

**Path 2: Connected `roko plan run --engine runner-v2` (with TUI approval)**
- Runner creates `SharedStateHub` and passes it to `App::new_connected()`
- Runner pushes `DashboardEvent` variants into the hub
- TUI receives live `DashboardSnapshot` via `tokio::sync::watch`
- The hub is the source of truth; disk files are not re-read

**Path 3: WebSocket Agent Streaming**
- `AgentStreamClient` connects to `ws://localhost:6677/api/events`
- Filters for specific agent ID
- Parses `StreamChunk` variants
- TUI polls via `try_recv()` on each frame tick (non-blocking)

### Refresh Cadence

- **Frame rate**: 16ms tick (~60fps), adaptive to ~20fps when idle
- **Filesystem watcher**: 200ms debounce window
- **Git watcher**: 500ms debounce window
- **System metrics**: ~2s sampling interval on background thread
- **WebSocket reconnect**: 1s initial backoff, doubles to max 30s

---

## 8. Color Scheme and Styling

### ROSEDUST Palette

The TUI uses a custom dark palette called "ROSEDUST" (warm rose/indigo aesthetic inherited from Mori):

```
Background:
  BG           = #000000 (pure black)
  BG_SECONDARY = #0E0C10 (near-black with purple tint)
  BG_HIGHLIGHT = #221C24 (dark purple highlight)

Text:
  TEXT         = #A58E9E (muted lavender)
  TEXT_DIM     = #91788A (dimmer lavender)
  TEXT_GHOST   = #6E5569 (faint purple)
  TEXT_PHANTOM = #372A37 (barely visible)

Accents:
  ROSE         = #B97894 (primary accent -- warm pink)
  ROSE_BRIGHT  = #DC9BB4 (bright pink for highlights)
  ROSE_DIM     = #8C6070 (subdued pink)
  BONE         = #D7C69E (warm tan for selected content)
  BONE_DIM     = #A08E6C (subdued tan)
  DREAM        = #7873A5 (indigo/purple for info/research)
  SAGE         = #7D9E8C (green for success)
  EMBER        = #C36E55 (orange-red for errors)
  WARNING      = #C39B5F (amber for warnings)
```

### Three Theme Modes

1. **Dark** (default): ROSEDUST palette, RGB colors
2. **No Color** (`NO_COLOR` env var): all colors reset
3. **High Contrast** (`ROKO_HIGH_CONTRAST` env var): WCAG 2.1 AA compliant, pure white on black, bright primary colors

### Semantic Color Functions

- `role_accent(role)`: implementer=ROSE, strategist=DREAM, architect=BONE, auditor=SAGE, critic=EMBER, conductor=WARNING, researcher=DREAM
- `phase_accent(phase)`: preflight=ghost, implement=rose, compile/test=warning, review=bone_dim, gate/verify=sage, fail=ember, done=sage
- `semantic_color(t)`: 0-0.4=EMBER, 0.4-0.8=WARNING, 0.8-1.0=SAGE

### Visual Effects

- **Breathing**: sine-wave brightness oscillation (0.8-1.0 range) on bright cells
- **Heartbeat**: double-pulse pattern for status indicators
- **Modal glow**: bloom effect around modal edges using the accent color
- **Dim overlay**: 45% dimming on content behind modals
- **NervViz**: gradient vignette driven by task/plan progress, context pressure, token rate
- **Floating particles**: ambient particles with drift animation
- **Gradients**: fire (red->amber->gold) and ocean (blue->teal->cyan) three-stop linear gradients

---

## 9. Layout Approach

### Global Layout

```
+-----------------------------------------------------------------------+
| Header Bar (1 row): heartbeat | wave | progress | plans | cost | sys  |
+-----------------------------------------------------------------------+
| Wave Progress Ribbon (1 row, hidden when no waves)                    |
+-----------------------------------------------------------------------+
|                                                                       |
|                      Content Area (flexible)                          |
|         (dispatched to active tab view via render_tab_content)        |
|                                                                       |
+-----------------------------------------------------------------------+
| Status Bar (1 row): git info | heartbeat | progress | cost | keys    |
+-----------------------------------------------------------------------+
```

Responsive outer margin: 1-cell margin on all sides when terminal >= 120x50.

### Per-Tab Layouts

Most tabs use a split-pane layout:
- **Dashboard**: 38% left / 62% right (master-detail)
- **Plans**: 35% left / 65% right
- **Agents**: 32% left / 68% right
- **Git**: 35% left / 65% right
- **Marketplace**: 35% left / 65% right
- **Atelier**: 40% left / 60% right
- **Logs**: single panel (full width)
- **Config**: single panel (full width)
- **Learning**: single panel with vertical sections

Layout uses `ratatui::layout::Layout` with `Constraint::Percentage`, `Constraint::Length`, and `Constraint::Min` for flexible sizing.

### Modal Sizing

Modals use percentage-based centered rectangles:
- Help: 86% x 84%
- Plan detail: 86% x 84%
- Quit: fixed 42x8
- Approval: 60% x 40%
- Wave overview: 80% x 70%
- Queue overview: 85% x 75%
- Agent pool: 90% x 70%

---

## 10. Testing

### Unit Tests (37 files with `#[cfg(test)]`)

Most infrastructure files have unit tests:
- `tabs.rs`: fkey roundtrip, next/prev cycle, index sequential, v2 surface mapping
- `theme.rs`: color mode verification, semantic color ranges, gradient sampling, brighten clamping
- `layout.rs`: centered rect containment, responsive margin, split sums
- `fs_watch.rs`: poll fallback emits refresh within 500ms
- `atmosphere.rs`: breathing/heartbeat range checks, luminance values
- `input.rs`: key dispatch tests (not read fully but test module exists)
- `dashboard.rs`: extensive data loading and summary tests
- `state.rs`: status parsing, model context limits
- `views/dashboard_view.rs`, `git_view.rs`, `context_view.rs`: rendering tests
- Various widget files: rendering and data processing tests

### Integration Tests

`crates/roko-cli/tests/tui_tabs.rs` (624 LOC):
- Tab system: all 10 tabs, fkey roundtrip, next/prev cycle, sub-view resolution
- DashboardSnapshot responsiveness (SH06-T03):
  - Parallel agents create distinct entries
  - Agent output isolation
  - Diagnosis ring buffer (populate, evict at 50, dedup by ID)
  - Token accumulation and routing
  - Phase transitions
  - Agent timing lifecycle (spawn/output/complete timestamps)
  - Respawn preserves tokens
  - Error ring buffer (evicts at 64)
  - Agent completion deactivation
  - Full lifecycle replay (plan start -> task start -> agent spawn -> output -> gate -> complete)

### Test Gaps

- No rendering tests that verify actual ratatui frame output
- No mouse interaction tests
- No WebSocket client tests
- No config editor round-trip tests
- No modal interaction sequence tests

---

## 11. What's Missing Compared to a Production TUI

### Functional Gaps

1. **Search/filter across all views** -- only Logs has a filter; no global search
2. **Keyboard shortcuts reference in-app** -- Help modal exists but keybinds per context are not shown inline
3. **Undo/redo for config editing** -- no history
4. **Clipboard integration** -- no copy/paste of values, hashes, etc.
5. **Terminal size responsiveness** -- layout adapts but many widgets don't gracefully degrade below ~80 columns
6. **Accessibility** -- high-contrast mode exists but no screen reader support, no focus indicators beyond border color
7. **Persistent layout preferences** -- no saved column widths, split positions, or tab order
8. **Error recovery** -- if a data file is corrupt, the view shows empty state rather than an error message

### Architectural Issues

1. **`app.rs` is a 4,576-line god object** -- owns all state, all dispatch, all I/O coordination. This is the single biggest maintainability risk. The `dispatch_action` method alone handles 72 action variants in one giant match.
2. **`state.rs` is a 5,290-line monolith** -- `TuiState` has dozens of public fields with no encapsulation. Any view can read any state.
3. **`dashboard.rs` at 7,445 lines** -- `DashboardData` is both data loader and data container. It does all the JSONL parsing, file stamping, and incremental cursor management in one struct.
4. **Two parallel state models** -- `DashboardData` (legacy scaffold) and `TuiState` (Mori-style) coexist with `update_from_snapshot` bridging them. The `DashboardData` fields are sometimes read directly by views, sometimes only consumed by `TuiState`.
5. **Two parallel page systems** -- `PageId`/`PageScaffold` (text-mode pages) and `Tab`/`SubView` (ratatui tabs) are independent hierarchies that overlap conceptually.
6. **No widget trait abstraction** -- each widget is a free function with different signatures. There's no `Widget` trait for composable rendering.

### UX Polish Missing

1. **Loading states** -- when data hasn't arrived yet, views show empty rather than a loading indicator (except for a few placeholders like "No cascade router data")
2. **Toast notifications** -- the system exists but is underused; most errors are silent
3. **Animations** -- breathing/heartbeat/particles exist but are subtle to the point of being invisible at 60fps; no transition animations between tabs
4. **Scrollbar indicators** -- plan tree has a scrollbar; most other scrollable views do not
5. **Multi-select** -- no way to select multiple plans/tasks for batch operations
6. **Status bar hints** -- the status bar shows context-sensitive key hints but they're static per tab, not per focus zone
7. **Resizable panes** -- all split ratios are fixed percentages; no drag-to-resize

### Data Freshness

1. **Standalone mode** relies on polling `.roko/` files via filesystem watcher (200ms debounce). Between debounce windows, data can be stale.
2. **Connected mode** is near-real-time but requires an active plan run.
3. **Git data** refreshes on `.git/` metadata changes (500ms debounce) but falls back to poll if notify fails.
4. **No SSE/HTTP polling** -- the TUI doesn't connect to `roko-serve` HTTP endpoints for data (only WebSocket for agent streaming and one-shot topology fetch).

### Performance Concerns

1. **Full frame redraw** on every tick at 60fps. No dirty-rectangle tracking.
2. **PostFX pipeline** iterates every cell in the buffer on every frame when effects are enabled.
3. **`DashboardData::tick()` does file I/O** when filesystem watcher fires -- this runs on the main thread.
4. **Agent output parsing** (`segment.rs`) re-parses the entire output buffer when the cache is invalidated, though the cache prevents redundant work.

---

## Summary Assessment

The Roko TUI is a **substantial and functional** monitoring dashboard. At ~44K LOC, it provides genuine real-time visibility into plan execution, agent activity, git state, learning metrics, and system health. The ROSEDUST theme gives it a distinctive visual identity, and the PostFX pipeline (modal glow, NervViz, particles) adds production polish.

The core strength is **real data integration**: plans come from the durable Runner projection, agents from DashboardSnapshot events, git from actual git commands, metrics from sysinfo, and the file watchers keep everything reasonably fresh.

The core weakness is **architectural complexity**: the three largest files (dashboard.rs, state.rs, app.rs) total 17,311 lines and contain deeply intertwined state management with two parallel data models. The 72-variant `TuiAction` dispatch in one method, the 30-variant `SubView` enum, and the dual PageId/Tab navigation systems all point to organic growth that has outrun its original architecture.

For a self-developing agent toolkit, this TUI delivers real operational value. For a production product, it needs the kind of modular widget architecture and state management refactoring that would let individual views be developed and tested independently.
