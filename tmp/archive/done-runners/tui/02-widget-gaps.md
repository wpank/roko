# TUI Widget Gaps -- Exhaustive Audit

Generated: 2026-04-14
Source: Every file in `crates/roko-cli/src/tui/widgets/` read line-by-line.
Mori reference: `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/widgets/`

Total Roko widget LOC: 5,453 across 28 files.
Total Mori widget LOC: 8,104 across 27 files.
Mori has 27 files; Roko has 28 (rosedust.rs is palette-only, not a widget).

---

## 1. Master Widget Inventory

Every widget file, its render function signature, what it reads, where it is called,
and what data it depends on.

### 1.1 Widgets integrated into the TUI (called from views or app.rs)

| # | Widget file | LOC | Render function | Called from | Takes TuiState? |
|---|-------------|-----|-----------------|-------------|-----------------|
| 1 | `header_bar.rs` | 400 | `render_header_bar(frame, area, &TuiState)` | `app.rs:1170` | Yes |
| 2 | `status_bar.rs` | 187 | `render_status_bar(frame, area, &TuiState)` | `app.rs:1175` | Yes |
| 3 | `plan_tree.rs` | 943 | `render_plan_tree(frame, area, &TuiState, focused)` | `dashboard_view.rs:108` | Yes |
| 4 | `phase_compact.rs` | 365 | `render_phase_compact(frame, area, &TuiState, focused)` | `dashboard_view.rs:109` | Yes |
| 5 | `task_progress.rs` | 510 | `render_task_progress(frame, area, &TuiState, focused)` | `dashboard_view.rs:110` | Yes |
| 6 | `wave_progress.rs` | 118 | `render_wave_progress(frame, area, &TuiState)` | `dashboard_view.rs:724`, `app.rs:398` | Yes |
| 7 | `token_sparkline.rs` | 183 | `render_token_sparkline(frame, area, &TuiState)` | `dashboard_view.rs:726` | Yes |
| 8 | `sys_metrics.rs` | 216 | `render_sys_metrics(frame, area, &TuiState)` | `dashboard_view.rs:728` | Yes |
| 9 | `command_output.rs` | 144 | `render_command_output(frame, area, &TuiState, focused)` | **NONE** (never called from any view) | Yes |
| 10 | `agent_output.rs` | 167 | `render_agent_output(frame, area, &TuiState, focused)` | **NONE** (agents_view has its own `render_output_body`) | Yes |
| 11 | `agent_pool.rs` | 313 | `render_agent_pool(frame, area, &TuiState, focused)` | **NONE** (agents_view has its own `render_agent_roster`; a separate `modals/agent_pool_modal.rs` exists) | Yes |
| 12 | `parallel_pool.rs` | 157 | `render_parallel_pool(frame, area, &[ParallelAgentState], selected, &Theme)` | `dashboard_view.rs:241` | No (own type) |
| 13 | `diff_panel.rs` | 80 | `render_diff_panel(frame, area, &str, Option<usize>, &Theme)` | `dashboard_view.rs:367` | No (raw args) |
| 14 | `error_digest.rs` | 204 | `render_error_digest(frame, area, &[GateVerdict], &[ErrorEntry], &SnapshotStats, &Theme)` | `dashboard_view.rs:435` | No (raw args) |
| 15 | `braille.rs` | 79 | `braille_spans_f64/f32/u64(...)` (utility, not a widget) | `sys_metrics.rs`, `token_sparkline.rs` | N/A |
| 16 | `rosedust.rs` | 231 | N/A (palette constants + gradients) | Every MoriTheme-based widget | N/A |

### 1.2 Widgets NEVER called from any view, app.rs, or modal (dead code)

| # | Widget file | LOC | Render function | Signature issue |
|---|-------------|-----|-----------------|-----------------|
| 17 | `agent_grid.rs` | 136 | `render_agent_grid(frame, area, &[AgentState], &Theme)` | Takes `roko_core::dashboard_snapshot::AgentState` (not TuiState) -- incompatible with view infrastructure |
| 18 | `context_gauge.rs` | 69 | `render_context_gauge(frame, area, used: u64, total: u64, &Theme)` | Takes raw u64 args -- no caller |
| 19 | `phase_bar.rs` | 97 | `render_phase_bar(frame, area, phase: &str, &Theme)` | Takes raw &str -- superseded by `phase_compact` |
| 20 | `phase_timeline.rs` | 108 | `render_phase_timeline(frame, area, &[PhaseEntry], current_idx, &Theme)` | Uses its own `PhaseEntry` type -- never called |
| 21 | `plan_list.rs` | 134 | `render_plan_list(frame, area, &[PlanEntry], selected, scroll, &Theme)` | Uses its own `PlanEntry` type (NOT state::PlanEntry) -- never called |
| 22 | `status_badge.rs` | 97 | `render_status_badge(frame, area, StatusBadge, &Theme)` | Takes own enum -- never called from any view |
| 23 | `tab_bar.rs` | 78 | `render_tab_bar(frame, area, &[TabDef], active, &Theme)` | Takes own `TabDef` type -- dashboard_view has inline tab bar code instead |
| 24 | `wave_bar.rs` | 92 | `render_wave_bar(frame, area, &WaveProgress, &Theme)` | Uses own `WaveProgress` type -- superseded by `wave_progress` |
| 25 | `token_bar.rs` | 110 | `render_token_bar(frame, area, &[AgentState], &Theme)` | Takes `roko_core::dashboard_snapshot::AgentState` -- incompatible |
| 26 | `scrollbar.rs` | 76 | `render_scrollbar(frame, area, total, visible, offset, &Theme)` | Uses Theme -- `plan_tree` has its own inline scrollbar; plans_view has its own `render_scrollbar`; never called from widgets |

**Total dead widget code: 997 lines (18% of all widget LOC).**

---

## 2. Per-Widget Detailed Gap Analysis

### W01: agent_grid.rs (136 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/agent_grid.rs`
- **Signature**: `render_agent_grid(frame, area, agents: &[roko_core::dashboard_snapshot::AgentState], theme: &Theme)`
- **TuiState fields read**: None (uses external AgentState type)
- **Called from**: Nowhere (only tests)
- **Data populated**: N/A -- takes `roko_core::dashboard_snapshot::AgentState` which has `agent_id`, `role`, `active`, `output_bytes`
- **Gap**: Uses `roko_core::dashboard_snapshot::AgentState` (4 fields), NOT `state::AgentRow` (11 fields). Cannot be called from any view without an adapter. Superseded by `agent_pool.rs` which uses TuiState directly.
- **Missing features vs Mori**: Mori's agent_grid (121 LOC) is similar in scope. But Roko's version is uncallable.
- **Severity**: Dead code. Remove or adapt to TuiState.

### W02: agent_output.rs (167 LOC) -- DEAD CODE, superseded

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/agent_output.rs`
- **Signature**: `render_agent_output(frame, area, state: &TuiState, focused: bool)`
- **TuiState fields read**: `state.agents[state.selected_agent].role`, `state.agents[state.selected_agent].last_output_line`, `state.agents[state.selected_agent].active`, `state.output_scroll`, `state.atmosphere`
- **Called from**: Nowhere -- `agents_view.rs` has its own `render_output_body` (lines 628-797) and `render_role_tabs` (lines 581-621)
- **Data populated**: `last_output_line` is set from episodes/task-outputs in `update_from_snapshot`
- **Gaps vs Mori** (Mori agent_output is 1,679 LOC):
  1. No ANSI escape sequence parsing -- Mori has `AnsiSegment` parser + `CachedRender`
  2. No per-line syntax coloring (Mori colors tool calls, errors, file paths differently)
  3. No `nerv_viz` background effect (Mori renders neural activity visualization behind text)
  4. No integrated scrollbar (Mori draws its own scrollbar with thumb position)
  5. No parallel agent tab switching (Mori shows tabs for each active agent with output)
  6. Only reads `last_output_line` (single line) -- Mori reads full output buffer
  7. No output truncation/line-wrap handling
  8. No auto-scroll indicator (showing "auto" vs "pinned" scroll state)
- **Severity**: Dead code. The agents_view reimplements most of this inline, but also has all the same gaps vs Mori.

### W03: agent_pool.rs (313 LOC) -- DEAD CODE, partially superseded

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/agent_pool.rs`
- **Signature**: `render_agent_pool(frame, area, state: &TuiState, focused: bool)`
- **TuiState fields read**: `state.agents[]` (all AgentRow fields), `state.selected_agent`, `state.atmosphere`, `state.active_agent_count()`
- **Called from**: Nowhere -- `agents_view` has `render_agent_roster` (inline). The modal `modals/agent_pool_modal.rs` has its own separate `render_agent_pool`.
- **Data populated**: AgentRow populated from DashboardData agents + episodes in `update_from_snapshot`
- **Gaps**:
  1. `agents[].input_tokens` / `output_tokens` -- only set from episodes (post-completion), zero during live execution
  2. `agents[].model` -- only set from episodes, empty string if no episode matches
  3. `agents[].context_limit` -- hardcoded to 200,000 in `update_from_snapshot`
  4. `agents[].current_task` -- only set from episodes, not live
  5. Contains `gradient_bar` rendering logic that duplicates `rosedust::gradient_context`
  6. No sorting (active agents should sort to top)
  7. No filter support
- **Severity**: Dead code with significant duplication.

### W04: braille.rs (79 LOC) -- FUNCTIONAL utility

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/braille.rs`
- **Signature**: `braille_spans_f64/f32/u64(data, max, width, color) -> Vec<Span>`
- **Called from**: `sys_metrics.rs` (CPU/MEM sparklines), `token_sparkline.rs`
- **Data**: Works on raw data arrays, no TuiState dependency
- **Gap**: Functional, but the data it displays via `token_sparkline` is always empty (see W07 below). The `sys_metrics` usage works because `cpu_history`/`mem_history` are populated.
- **Severity**: Low -- utility code is correct; data flow is the problem.

### W05: branch_tree.rs (128 LOC) -- CALLED, own types

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/branch_tree.rs`
- **Signature**: `render_branch_tree(frame, area, nodes: &[GitTreeNode], cursor: usize, theme: &Theme)`
- **Called from**: `git_view.rs:115` (but git_view has its own `render_branch_tree` wrapper at line 121 that does NOT call this widget)
- **Data**: Uses own `GitTreeNode` type. TuiState has `git_branch_tree: Vec<GitBranchNode>` (different type!).
- **Gaps**:
  1. Type mismatch: widget takes `GitTreeNode` (with `BranchType` enum), state has `GitBranchNode` (with `ahead`/`behind` counters, no `BranchType`). These are incompatible.
  2. `git_branch_tree` in TuiState is never populated by `update_from_snapshot` -- always empty Vec.
  3. No ahead/behind display (the widget type doesn't have these fields, but the state type does).
  4. No worktree display.
  5. git_view.rs actually implements its own branch tree rendering inline rather than calling this widget.
- **Severity**: Dead code due to type mismatch and git_view having inline implementation.

### W06: command_output.rs (144 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/command_output.rs`
- **Signature**: `render_command_output(frame, area, state: &TuiState, focused: bool)`
- **TuiState fields read**: `state.gate_results[]` (GateResultEntry: gate, plan_id, passed, output), `state.output_scroll`, `state.atmosphere`
- **Called from**: Nowhere (no view calls it)
- **Data populated**: `gate_results` in TuiState is initialized empty and never populated by `update_from_snapshot`. The DashboardData has `gate_results: Vec<GateResultSummary>` but this is never bridged into `state.gate_results: Vec<GateResultEntry>`.
- **Gaps vs Mori** (218 LOC):
  1. Never called from any view -- completely orphaned
  2. `gate_results` is never populated (empty Vec forever)
  3. Uses `state.output_scroll` which is shared with agent_output scroll -- conflict
  4. No ANSI coloring of compiler output
  5. No line-level error highlighting with file:line hyperlinks
- **Severity**: Critical -- gate output is never shown in the TUI despite the widget existing.

### W07: context_gauge.rs (69 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/context_gauge.rs`
- **Signature**: `render_context_gauge(frame, area, used: u64, total: u64, theme: &Theme)`
- **Called from**: Nowhere
- **Gap**: Takes raw u64 args, not TuiState. No view calls it. The `agent_pool.rs` widget has inline context gauge rendering. This standalone widget adds nothing.
- **Severity**: Dead code. Low priority since agent_pool does inline context display.

### W08: diff_panel.rs (80 LOC) -- CALLED but stub

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/diff_panel.rs`
- **Signature**: `render_diff_panel(frame, area, diff_text: &str, scroll: Option<usize>, theme: &Theme)`
- **Called from**: `dashboard_view.rs:367` (sub-tab "diff")
- **Data source**: `dashboard_view` passes `current_plan_execution.agent_output_tail` as diff_text. This is the raw agent output, NOT an actual unified diff. The variable name is misleading.
- **Gaps vs Mori** (157 LOC):
  1. No actual git diff content -- uses agent output text as a proxy
  2. `diff_text` is always empty unless there is an active plan execution with agent output
  3. No diff stats header (files changed, insertions, deletions)
  4. No file-level navigation
  5. No hunk-level folding
  6. No line number column
  7. No binary file detection
  8. Scroll position uses `state.diff_scroll` from TuiState but the widget takes `Option<usize>` -- conversion handled by caller
- **Severity**: High -- renders empty most of the time; never shows real diffs.

### W09: error_digest.rs (204 LOC) -- CALLED with adapter

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/error_digest.rs`
- **Signature**: `render_error_digest(frame, area, &[GateVerdict], &[ErrorEntry], &SnapshotStats, &Theme)`
- **Called from**: `dashboard_view.rs:435` (sub-tab "errors")
- **Data source**: `dashboard_view` constructs `GateVerdict`, `ErrorEntry`, and `SnapshotStats` from `DashboardData.gate_results` and `DashboardData.recent_signals`
- **Gaps vs Mori** (407 LOC):
  1. No error categorization (compile vs test vs runtime)
  2. No error deduplication
  3. No error count trending (rate display)
  4. No clickable error locations
  5. No scrollbar on error list
  6. No filtering by gate type or plan
  7. Gate summary uses `SnapshotStats.gates_passed/gates_failed` which are computed from DashboardData -- works correctly when signals exist
- **Severity**: Medium -- functional but thin. Works only when gate signals are present.

### W10: header_bar.rs (400 LOC) -- CALLED, fully integrated

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/header_bar.rs`
- **Signature**: `render_header_bar(frame, area, state: &TuiState)`
- **Called from**: `app.rs:1170`
- **TuiState fields read**: `atmosphere`, `execution_waves`, `plans`, `agents`, `cost_dollars`, `token_total`, `sys.cpu_pct`, `sys.mem_used_bytes`, `sys.mem_total_bytes`, `active_tab`, `run_started`
- **Gaps vs Mori** (432 LOC):
  1. ETA calculation is very naive (linear extrapolation from done/total ratio)
  2. No network/disk rate in header (Mori shows net/disk in compact form)
  3. No notification badge count
  4. No "paused" indicator
  5. `sys.cpu_pct` and `sys.mem_*` are all zero unless system metrics collection is wired (it is NOT wired -- `SysMetrics` is never populated by any background thread)
  6. F-key strip shows `F6:cfg` for Config tab but there is no `Tab::Context` -- `context_view.rs` exists but has no Tab entry
- **Severity**: Low -- well-implemented but shows zeros for system metrics.

### W11: parallel_pool.rs (157 LOC) -- CALLED from dashboard_view

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/parallel_pool.rs`
- **Signature**: `render_parallel_pool(frame, area, agents: &[ParallelAgentState], selected: usize, theme: &Theme)`
- **Called from**: `dashboard_view.rs:241` (sub-tab "agents")
- **Data source**: `dashboard_view` converts `DashboardData.agents` into local `ParallelAgentState` instances
- **CRITICAL TYPE DUPLICATION**: This widget defines its own `ParallelAgentState` struct AND `AgentRunState` enum. TuiState also has its own `ParallelAgentState` struct with different fields. These are completely independent types. The dashboard_view builds the widget's type directly from DashboardData, bypassing TuiState entirely.
- **Gaps vs Mori** (307 LOC):
  1. No cost column (Mori shows per-agent cost)
  2. No context percentage gauge per agent (widget type has `context_pct` but dashboard_view sets it to 0.0)
  3. No progress bar per agent (widget type has `tokens_used`/`tokens_total` but these are set to 0/0)
  4. No sorting by cost or status
  5. No agent-click to expand output
  6. Token counts are always 0 because `DashboardData.agents` has no token data
  7. `model` field is always empty because `DashboardData.agents` has no model data
- **Severity**: High -- shows agents but all detail columns show zeros.

### W12: phase_bar.rs (97 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/phase_bar.rs`
- **Signature**: `render_phase_bar(frame, area, phase: &str, theme: &Theme)`
- **Called from**: Nowhere (only tests)
- **Gap**: Single-line phase indicator superseded by `phase_compact.rs`. Uses different phase names ("compose", "dispatch", "execute", "gate", "persist", "completed") vs the canonical 9 phases in state.rs ("preflight", "strategist", "implementer", etc.). Inconsistent.
- **Severity**: Dead code. Superseded.

### W13: phase_compact.rs (365 LOC) -- CALLED, well-integrated

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/phase_compact.rs`
- **Signature**: `render_phase_compact(frame, area, state: &TuiState, focused: bool)`
- **Called from**: `dashboard_view.rs:109`
- **TuiState fields read**: `state.phase_pipeline[]` (PhaseStep: name, status, elapsed_secs, pct), `state.atmosphere`
- **Data populated**: `phase_pipeline` built by `build_phase_pipeline()` in state.rs from `active_tasks`. `populate_phase_elapsed()` fills `elapsed_secs` from episodes.
- **Gaps**:
  1. `pct` is always 50.0 for Active, 100.0 for Done, 0.0 for Pending -- never reflects real progress. The existing gap doc says "always 0.0" but it is actually 50.0/100.0/0.0 from build_phase_pipeline heuristic.
  2. Phase-to-task mapping is a crude heuristic (midpoint-based), not actually tracking which tasks belong to which phase.
  3. No ETA per phase.
  4. No phase name abbreviation in the bar segments (Mori shows abbreviated labels in each segment).
  5. `elapsed_secs` is populated from episodes via `populate_phase_elapsed` but this only works when episodes exist with matching phase metadata.
- **Severity**: Medium -- renders, but progress data is synthetic.

### W14: phase_timeline.rs (108 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/phase_timeline.rs`
- **Signature**: `render_phase_timeline(frame, area, phases: &[PhaseEntry], current_idx: usize, theme: &Theme)`
- **Called from**: Nowhere
- **Gap**: Uses own `PhaseEntry` type (name, elapsed_secs) -- not connected to TuiState. Superseded by `phase_compact.rs` which is actually wired.
- **Severity**: Dead code.

### W15: plan_list.rs (134 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/plan_list.rs`
- **Signature**: `render_plan_list(frame, area, plans: &[PlanEntry], selected, scroll, theme: &Theme)`
- **Called from**: Nowhere
- **Gap**: Uses own `PlanEntry` type (name, progress, tasks_done, tasks_total, failed) which is different from `state::PlanEntry`. Superseded by `plan_tree.rs` which is wired and more feature-complete.
- **Severity**: Dead code.

### W16: plan_tree.rs (943 LOC) -- CALLED, most complete widget

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/plan_tree.rs`
- **Signature**: `render_plan_tree(frame, area, state: &TuiState, focused: bool)`
- **Called from**: `dashboard_view.rs:108`
- **TuiState fields read**: `state.plans[]` (all PlanEntry fields), `state.execution_waves[]`, `state.selected_plan`, `state.plan_scroll`, `state.filter`, `state.atmosphere`
- **Data populated**: Plans from `update_from_snapshot` (DashboardData.plans -> PlanEntry). Waves from `build_execution_waves`.
- **Gaps vs Mori** (1,077 LOC):
  1. No task-level drill-down (Mori expands plans to show individual tasks with status icons)
  2. `PlanEntry.tasks` Vec is always empty -- `update_from_snapshot` never populates nested tasks
  3. `PlanEntry.elapsed_secs` is always 0.0 -- never populated
  4. `PlanEntry.wave` is always None -- `update_from_snapshot` does not set wave assignments
  5. Wave tree always collapses to flat view because waves are derived from plans with no wave field
  6. `render_data_rain` function exists (lines 640-665) but is commented out ("removed -- kept empty space clean")
  7. Filter works for plan names but does NOT filter within wave groups (if a plan in a wave doesn't match, the wave header still shows)
  8. Column header "vfy" (verify) always shows dot placeholder -- no verification status data
  9. Selected plan detail row has no drill-down action (no keyboard handler for Enter on tasks)
  10. Inline scrollbar is functional (lines 672-709) but `plan_scroll` is only clamped in `update_from_snapshot`, not actively managed during navigation
- **Severity**: Medium -- core rendering is solid, but task drill-down and timing data are absent.

### W17: scrollbar.rs (76 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/scrollbar.rs`
- **Signature**: `render_scrollbar(frame, area, total_items, visible_items, scroll_offset, theme: &Theme)`
- **Called from**: Nowhere externally. `plan_tree.rs` has its own inline scrollbar. `plans_view.rs` has its own `render_scrollbar`. `task_progress.rs` uses ratatui's built-in `Scrollbar` widget directly.
- **Gap**: Three different scrollbar implementations exist: this standalone widget, plan_tree's inline version, and task_progress using ratatui's built-in. None calls this one.
- **Severity**: Dead code with duplicated functionality.

### W18: status_badge.rs (97 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/status_badge.rs`
- **Signature**: `render_status_badge(frame, area, status: StatusBadge, theme: &Theme)` + `status_badge_span(status, theme) -> Span`
- **Called from**: Nowhere
- **Gap**: Provides 8 badge variants (Active, Done, Error, Warning, Revision, Paused, Idle, Pending) but no view uses them. The `task_progress.rs` widget has inline status icons. The `plan_tree.rs` has inline plan_icon(). Both bypass this widget.
- **Severity**: Dead code. Could be useful if views adopted it.

### W19: status_bar.rs (187 LOC) -- CALLED, well-integrated

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/status_bar.rs`
- **Signature**: `render_status_bar(frame, area, state: &TuiState)`
- **Called from**: `app.rs:1175`
- **TuiState fields read**: `plans`, `agents`, `atmosphere`, `active_tab`, `git_branch`, `git_commit_short`, `git_age`
- **Gaps**:
  1. `git_branch`, `git_commit_short`, `git_age` are never populated by `update_from_snapshot` -- always empty strings, so the git section never renders
  2. No "paused" state display
  3. No notification count badge
  4. Tab::Inspect keybinds listed but Inspect tab has no keyboard handler
  5. Context-sensitive key hints are hardcoded strings that may not match actual keybinds
- **Severity**: Low -- functional, but git info always blank.

### W20: sys_metrics.rs (216 LOC) -- CALLED, data-starved

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/sys_metrics.rs`
- **Signature**: `render_sys_metrics(frame, area, state: &TuiState)`
- **Called from**: `dashboard_view.rs:728`
- **TuiState fields read**: `state.sys.cpu_pct`, `state.sys.cpu_history`, `state.sys.mem_used_bytes`, `state.sys.mem_total_bytes`, `state.sys.mem_history`, `state.sys.net_down_bytes_sec`, `state.sys.disk_read_bytes_sec`, `state.atmosphere`
- **Data populated**: `SysMetrics` struct exists in TuiState but is NEVER populated. `update_from_snapshot` does not touch `sys`. There is no background thread or polling mechanism to collect system metrics.
- **Gaps**:
  1. ALL metrics are zero -- no data collection exists anywhere in the codebase
  2. `cpu_history` and `mem_history` arrays are empty, so sparklines render blank
  3. Network/disk rates show "0B" always
  4. FPS display works (uses `atmosphere.fps()` which is computed from frame timing)
  5. Mini gauge rendering with breathing shimmer is implemented but always shows empty bar
- **Severity**: Critical -- entire widget renders zeros. Needs sysinfo crate integration.

### W21: tab_bar.rs (78 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/tab_bar.rs`
- **Signature**: `render_tab_bar(frame, area, tabs: &[TabDef], active, theme: &Theme)`
- **Called from**: Nowhere
- **Gap**: Uses own `TabDef` type. `dashboard_view.rs` has inline sub-tab bar rendering at `render_sub_tab_bar` (lines 162-181). `header_bar.rs` has F-key tab strip inline.
- **Severity**: Dead code. Two inline implementations exist instead.

### W22: task_progress.rs (510 LOC) -- CALLED, well-integrated

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/task_progress.rs`
- **Signature**: `render_task_progress(frame, area, state: &TuiState, focused: bool)`
- **Called from**: `dashboard_view.rs:110`
- **TuiState fields read**: `state.current_task_checklist[]` (TaskRow: id, title, status, elapsed_secs), `state.task_scroll`, `state.atmosphere`
- **Data populated**: `current_task_checklist` built by `build_task_checklist_from_execution()` from DashboardData active_tasks + task-trackers
- **Gaps vs Mori** (601 LOC):
  1. `elapsed_secs` is always 0.0 -- task start times are not tracked, so time tags never show
  2. No ETA estimation per task
  3. No agent assignment display (which agent is working on which task)
  4. No dependency visualization (blocked-by arrows)
  5. No task reordering by status (active first, then pending, then done)
  6. Scroll uses `task_scroll` which is clamped but keyboard navigation for task list scroll is not wired in input.rs for the dashboard view
  7. Summary line shows correct counts but phase label is hardcoded to status text
  8. No click-to-expand task detail
- **Severity**: Medium -- core checklist renders correctly but timing and agent data are absent.

### W23: token_bar.rs (110 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/token_bar.rs`
- **Signature**: `render_token_bar(frame, area, agents: &[roko_core::dashboard_snapshot::AgentState], theme: &Theme)`
- **Called from**: Nowhere
- **Gap**: Uses `roko_core::dashboard_snapshot::AgentState` which has `output_bytes` not tokens. Measures byte output, not token usage. Superseded by `token_sparkline.rs`.
- **Severity**: Dead code.

### W24: token_sparkline.rs (183 LOC) -- CALLED, data-starved

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/token_sparkline.rs`
- **Signature**: `render_token_sparkline(frame, area, state: &TuiState)`
- **Called from**: `dashboard_view.rs:726`
- **TuiState fields read**: `state.token_history` (HashMap<String, VecDeque<u64>>), `state.token_total`, `state.token_rate`, `state.atmosphere`
- **Data populated**: `token_history` is NEVER populated by `update_from_snapshot`. `token_total` is populated from efficiency data. `token_rate` is always 0.0.
- **Gaps**:
  1. `token_history` HashMap is always empty -> sparkline never renders, falls through to "waiting for data..." message
  2. `token_rate` is always 0.0 -> rate display always shows "idle"
  3. `token_total` IS populated correctly from efficiency data, so the total count is accurate
  4. Per-role breakdown section never renders because `token_history` is empty
  5. No time-windowed rate calculation
  6. No cost overlay on sparkline
- **Severity**: Critical -- always shows "waiting for data" despite having total token count available.

### W25: wave_bar.rs (92 LOC) -- DEAD CODE

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/wave_bar.rs`
- **Signature**: `render_wave_bar(frame, area, wave: &WaveProgress, theme: &Theme)`
- **Called from**: Nowhere
- **Gap**: Uses own `WaveProgress` type. Superseded by `wave_progress.rs` and the wave rendering in `plan_tree.rs`.
- **Severity**: Dead code.

### W26: wave_progress.rs (118 LOC) -- CALLED, functional

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/wave_progress.rs`
- **Signature**: `render_wave_progress(frame, area, state: &TuiState)`
- **Called from**: `dashboard_view.rs:724`, `app.rs:398`
- **TuiState fields read**: `state.execution_waves[]`, `state.current_wave()`, `state.atmosphere`
- **Data populated**: `execution_waves` built by `build_execution_waves()` from plans
- **Gaps**:
  1. Renders nothing when `execution_waves` is empty (which it isn't when plans exist -- at least wave 0 is created)
  2. No ETA per wave
  3. No total elapsed time display
  4. No wave transition animation
  5. Wave width allocation can overflow when many waves have minimum 3-char segments
- **Severity**: Low -- functional when waves exist.

### W27: rosedust.rs (231 LOC) -- PALETTE, not a widget

- **File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/rosedust.rs`
- **Signature**: N/A (constants and helpers only)
- **Called from**: Every MoriTheme-using widget
- **Gap**: Not a widget. Provides `MoriTheme` constants, `Gradient` struct, `brighten()`, gradients. Complete for current needs.
- **Severity**: N/A

---

## 3. Type Duplication / Incompatibility Issues

| Issue | Details |
|-------|---------|
| **ParallelAgentState x2** | `parallel_pool.rs` defines its own `ParallelAgentState` (role, model, task, tokens_used, tokens_total, state: AgentRunState, context_pct). `state.rs` defines a different `ParallelAgentState` (agent_id, plan_id, task_id, status: String, progress_pct: f64). The dashboard_view builds the widget's type directly, bypassing the state type entirely. |
| **PlanEntry x2** | `plan_list.rs` defines `PlanEntry` (name, progress, tasks_done: u32, tasks_total: u32, failed: bool). `state.rs` defines `PlanEntry` (22+ fields). Completely different types with same name. |
| **PhaseEntry** | `phase_timeline.rs` defines `PhaseEntry` (name, elapsed_secs). `state.rs` defines `PhaseStep` (name, status, elapsed_secs, pct). Similar but incompatible. |
| **AgentState vs AgentRow** | `agent_grid.rs` and `token_bar.rs` use `roko_core::dashboard_snapshot::AgentState` (agent_id, role, active, output_bytes). All TuiState-based widgets use `state::AgentRow` (11 fields). These can never be mixed. |
| **WaveProgress** | `wave_bar.rs` defines `WaveProgress` (wave_number, plan_count, task_count, tasks_done, eta_secs). `state.rs` defines `Wave` (index, plans, done, total, expanded). Different types. |
| **GitTreeNode vs GitBranchNode** | `branch_tree.rs` defines `GitTreeNode` (name, branch_type: BranchType, is_current, children). `state.rs` defines `GitBranchNode` (name, is_current, ahead, behind, children). Incompatible. |

---

## 4. Data Fields That Are Never Populated

| TuiState field | Read by widget(s) | Population status |
|----------------|-------------------|-------------------|
| `token_history` | token_sparkline | **NEVER populated** -- HashMap is always empty |
| `token_rate` | token_sparkline | **NEVER populated** -- always 0.0 |
| `gate_results` | command_output | **NEVER populated** -- Vec is always empty |
| `sys.cpu_pct` | header_bar, sys_metrics | **NEVER populated** -- no system metrics collection |
| `sys.cpu_history` | sys_metrics | **NEVER populated** |
| `sys.mem_used_bytes` | header_bar, sys_metrics | **NEVER populated** |
| `sys.mem_total_bytes` | header_bar, sys_metrics | **NEVER populated** |
| `sys.mem_history` | sys_metrics | **NEVER populated** |
| `sys.net_down_bytes_sec` | sys_metrics | **NEVER populated** |
| `sys.disk_read_bytes_sec` | sys_metrics | **NEVER populated** |
| `git_branch` | status_bar | **NEVER populated** by update_from_snapshot |
| `git_commit_short` | status_bar | **NEVER populated** by update_from_snapshot |
| `git_age` | status_bar | **NEVER populated** by update_from_snapshot |
| `git_branch_tree` | branch_tree (dead) | **NEVER populated** |
| `git_commit_graph` | (no widget) | **NEVER populated** |
| `git_worktree_list` | (no widget) | **NEVER populated** |
| `parallel_agents` | (no widget uses this) | **NEVER populated** -- state::ParallelAgentState Vec |
| `plans[].tasks` | plan_tree (expand) | **NEVER populated** -- always empty Vec |
| `plans[].elapsed_secs` | plan_tree (age column) | **NEVER populated** -- always 0.0 |
| `plans[].wave` | plan_tree, build_execution_waves | **NEVER populated** -- always None |
| `task_checklist[].elapsed_secs` | task_progress | **NEVER populated** -- always 0.0 |
| `agents[].context_limit` | agent_pool | **Hardcoded** to 200,000 |
| `cost_per_plan` | (no widget) | **NEVER populated** |
| `cost_per_task` | (no widget) | **NEVER populated** |
| `token_burn_history` | (no widget) | **NEVER populated** |
| `notifications` | (no widget) | **NEVER populated** |
| `log_messages` | (logs_view reads directly) | **NEVER populated** in TuiState |

---

## 5. What 02-widget-gaps.md Missed

The original gap doc had 19 items. This audit found the following additional issues:

| New ID | What was missed |
|--------|-----------------|
| W20 | `agent_grid.rs` uses incompatible `roko_core::dashboard_snapshot::AgentState` type, making it structurally uncallable from views |
| W21 | `agent_output.rs` is dead code -- `agents_view.rs` reimplements it inline with its own `render_output_body` |
| W22 | `agent_pool.rs` is dead code -- `agents_view.rs` reimplements it inline with `render_agent_roster`; modal has separate `agent_pool_modal.rs` |
| W23 | `command_output.rs` is dead code AND its data source (`gate_results`) is never populated |
| W24 | `branch_tree.rs` has type mismatch (GitTreeNode vs GitBranchNode) and is never actually called |
| W25 | `scrollbar.rs` is dead code -- three separate scrollbar implementations exist, none calls this one |
| W26 | `phase_bar.rs` uses inconsistent phase names vs canonical phases |
| W27 | `tab_bar.rs` is dead code -- two inline implementations exist |
| W28 | `plan_list.rs` has own PlanEntry type shadowing state::PlanEntry |
| W29 | 6 type duplication issues (ParallelAgentState, PlanEntry, PhaseEntry, AgentState/AgentRow, WaveProgress, GitTreeNode/GitBranchNode) |
| W30 | `sys.cpu_pct`, `sys.mem_*`, `sys.net_*`, `sys.disk_*` are all NEVER populated (no sysinfo integration) |
| W31 | `git_branch`, `git_commit_short`, `git_age` never populated -- status_bar git section is always blank |
| W32 | `plans[].tasks`, `plans[].elapsed_secs`, `plans[].wave` never populated -- plan_tree detail info is always empty |
| W33 | `parallel_agents` in state.rs is never populated and no widget reads it (dashboard_view builds parallel_pool widget's own type directly) |
| W34 | `cost_per_plan`, `cost_per_task`, `token_burn_history`, `notifications`, `log_messages` TuiState fields are never populated by any code path |
| W35 | `phase_compact.pct` is 50.0 for Active, not 0.0 as the old doc stated |
| W36 | `diff_panel` renders agent output text, NOT actual git diffs |
| W37 | `agents[].context_limit` is hardcoded 200,000, not dynamically set from model context window |
| W38 | Total dead widget code: 997 lines (18% of all widget LOC) across 10 widget files |

---

## 6. Summary Table: All Gaps

| ID | Widget | File | Gap Description | Data Dependencies | Called From | Severity |
|----|--------|------|-----------------|-------------------|-------------|----------|
| W01 | agent_grid | `agent_grid.rs` (136 LOC) | Dead code; uses incompatible `roko_core::AgentState` type | `roko_core::dashboard_snapshot::AgentState` | **None** | Low (remove) |
| W02 | agent_output | `agent_output.rs` (167 LOC) | Dead code; superseded by agents_view inline `render_output_body` | `TuiState.agents[].last_output_line` | **None** | Low (remove) |
| W03 | agent_pool | `agent_pool.rs` (313 LOC) | Dead code; superseded by agents_view `render_agent_roster` + modal | `TuiState.agents[]` (all fields) | **None** | Low (remove) |
| W04 | braille | `braille.rs` (79 LOC) | Utility works, but token_sparkline data always empty | Raw data arrays | sys_metrics, token_sparkline | Low |
| W05 | branch_tree | `branch_tree.rs` (128 LOC) | Dead; type mismatch GitTreeNode vs GitBranchNode; git_view has inline impl | `GitTreeNode` (own type) | **None** (git_view wraps, doesn't call) | Low (remove) |
| W06 | command_output | `command_output.rs` (144 LOC) | Dead code; `gate_results` never populated | `TuiState.gate_results` (always empty) | **None** | **Critical** (gate output never shown) |
| W07 | context_gauge | `context_gauge.rs` (69 LOC) | Dead code; agent_pool has inline context display | Raw u64 args | **None** | Low (remove) |
| W08 | diff_panel | `diff_panel.rs` (80 LOC) | Shows agent output, not actual diffs; often empty | `diff_text: &str` from caller | dashboard_view sub-tab | **High** (misleading content) |
| W09 | error_digest | `error_digest.rs` (204 LOC) | Functional but thin; no categorization/dedup/scrollbar | `GateVerdict[]`, `ErrorEntry[]`, `SnapshotStats` | dashboard_view sub-tab | Medium |
| W10 | header_bar | `header_bar.rs` (400 LOC) | System metrics always zero; no pause indicator | `TuiState` (many fields); `sys.*` never populated | app.rs | Medium |
| W11 | parallel_pool | `parallel_pool.rs` (157 LOC) | Type duplication; token/model/context always zero | Own `ParallelAgentState` (not state.rs type) | dashboard_view sub-tab | **High** (shows empty columns) |
| W12 | phase_bar | `phase_bar.rs` (97 LOC) | Dead code; inconsistent phase names; superseded by phase_compact | `phase: &str` | **None** | Low (remove) |
| W13 | phase_compact | `phase_compact.rs` (365 LOC) | pct is heuristic (50/100/0), not real; phase mapping is crude | `TuiState.phase_pipeline[]` | dashboard_view | Medium |
| W14 | phase_timeline | `phase_timeline.rs` (108 LOC) | Dead code; own PhaseEntry type; superseded by phase_compact | `PhaseEntry[]` (own type) | **None** | Low (remove) |
| W15 | plan_list | `plan_list.rs` (134 LOC) | Dead code; own PlanEntry type shadows state::PlanEntry | `PlanEntry[]` (own type) | **None** | Low (remove) |
| W16 | plan_tree | `plan_tree.rs` (943 LOC) | No task drill-down; elapsed/wave/tasks always empty; data-rain removed | `TuiState.plans[]`, `.execution_waves[]`, `.selected_plan`, `.plan_scroll`, `.filter` | dashboard_view | Medium |
| W17 | scrollbar | `scrollbar.rs` (76 LOC) | Dead code; 3 scrollbar impls exist, none calls this one | Raw count args | **None** | Low (remove) |
| W18 | status_badge | `status_badge.rs` (97 LOC) | Dead code; task_progress and plan_tree have inline status icons | `StatusBadge` enum | **None** | Low (remove) |
| W19 | status_bar | `status_bar.rs` (187 LOC) | Git info always blank; no pause/notification badges | `TuiState.plans`, `.agents`, `.git_*` (never populated) | app.rs | Medium |
| W20 | sys_metrics | `sys_metrics.rs` (216 LOC) | ALL metrics always zero; no sysinfo integration exists | `TuiState.sys.*` (never populated) | dashboard_view ribbon | **Critical** (all zeros) |
| W21 | tab_bar | `tab_bar.rs` (78 LOC) | Dead code; own TabDef type; dashboard and header have inline tabs | `TabDef[]` (own type) | **None** | Low (remove) |
| W22 | task_progress | `task_progress.rs` (510 LOC) | elapsed_secs always 0; no agent assignment display; scroll not wired | `TuiState.current_task_checklist[]`, `.task_scroll` | dashboard_view | Medium |
| W23 | token_bar | `token_bar.rs` (110 LOC) | Dead code; uses roko_core::AgentState type; measures bytes not tokens | `roko_core::dashboard_snapshot::AgentState` | **None** | Low (remove) |
| W24 | token_sparkline | `token_sparkline.rs` (183 LOC) | token_history never populated; always shows "waiting for data" | `TuiState.token_history` (always empty), `.token_total`, `.token_rate` | dashboard_view ribbon | **Critical** (always empty) |
| W25 | wave_bar | `wave_bar.rs` (92 LOC) | Dead code; own WaveProgress type; superseded by wave_progress | `WaveProgress` (own type) | **None** | Low (remove) |
| W26 | wave_progress | `wave_progress.rs` (118 LOC) | No ETA; no elapsed; wave transition not animated | `TuiState.execution_waves[]`, `.current_wave()`, `.atmosphere` | dashboard_view ribbon, app.rs | Low |
| W27 | rosedust | `rosedust.rs` (231 LOC) | Palette/utility, not a widget | N/A | All MoriTheme widgets | N/A |

---

## 7. Priority Actions

### Critical (blocks basic TUI usefulness)

1. **Populate `gate_results`** -- Bridge `DashboardData.gate_results` into `TuiState.gate_results` in `update_from_snapshot`, then wire `command_output.rs` into a view (or inline the rendering).
2. **Populate `token_history`** -- Build time-series from efficiency events in `update_from_snapshot` so token_sparkline actually renders.
3. **Wire system metrics** -- Add `sysinfo` crate, spawn background polling thread, populate `SysMetrics` fields.
4. **Populate `token_rate`** -- Compute from efficiency event timestamps.

### High (significant visual gaps)

5. **Fix diff_panel data source** -- Collect actual git diff output (e.g., from `git diff HEAD~1` per plan) instead of using agent output text.
6. **Populate `parallel_pool` token/model/context data** -- Either wire from episodes or add live agent metadata to DashboardData.
7. **Populate `plans[].tasks`** -- Read task list from plan TOML files in update_from_snapshot.
8. **Populate git fields** -- Run git commands to fill `git_branch`, `git_commit_short`, `git_age` in a background thread.

### Medium (functional but incomplete)

9. Fix phase_compact pct to reflect real task progress per phase.
10. Populate `task_checklist[].elapsed_secs` from task-tracker files.
11. Populate `plans[].elapsed_secs` from executor state or episode timestamps.

### Low (dead code cleanup)

12. Remove or consolidate 10 dead widget files: `agent_grid.rs`, `agent_output.rs` (widget version), `agent_pool.rs` (widget version), `context_gauge.rs`, `phase_bar.rs`, `phase_timeline.rs`, `plan_list.rs`, `scrollbar.rs`, `tab_bar.rs`, `token_bar.rs`, `wave_bar.rs`, `status_badge.rs`.
13. Resolve 6 type duplication issues.
14. Consolidate scrollbar implementations (use ratatui built-in everywhere).
