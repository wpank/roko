# TUI Implementation Checklist

Complete, ordered checklist to make the TUI fully functional. Each item is self-contained with all context needed for an agent with no prior knowledge.

**Total items**: 52
**Workspace root**: `/Users/will/dev/nunchi/roko/roko`
**TUI source**: `crates/roko-cli/src/tui/`

---

## Phase 0: Critical path fixes (unblock everything else)

### 0.1 Fix plans directory path
- **Files**: `crates/roko-cli/src/plan.rs:133-135`
- **Problem**: `plans_dir()` returns `workdir.join(".roko").join("plans")` but actual plan directories (P06-process-management, P07-autofix-retry, W01-wire-system-prompts, etc.) live at `workdir.join("plans")`.
- **Fix**: Change `plans_dir()` to check both `workdir.join("plans")` and `workdir.join(".roko").join("plans")`, preferring whichever exists. Or add a config option. The simplest fix: check `workdir.join("plans")` first, fall back to `.roko/plans/`.
- **Also update**: `crates/roko-cli/src/plan.rs:138` `list_plan_files()` uses `plans_dir()`.
- **Verify**: `cargo test -p roko-cli` passes. Running `cargo run -p roko-cli -- plan list` shows real plans.
- **Acceptance**: Plan tree in TUI shows real plan entries instead of "Plans (0/0)".

### 0.2 Fix episode file path alignment
- **Files**: `crates/roko-cli/src/tui/dashboard.rs:38-39,507,620,2836-2838`
- **Problem**: TUI reads episodes from `.roko/memory/episodes.jsonl` (constants `MEMORY_DIR = ".roko/memory"`, `EPISODES_FILE = "episodes.jsonl"`). But the orchestrator at `crates/roko-cli/src/orchestrate.rs:4449` writes to `.roko/episodes.jsonl`. The paths don't match.
- **Fix**: Change `MEMORY_DIR` to `".roko"` in dashboard.rs, OR change the orchestrator to write to `.roko/memory/episodes.jsonl`, OR (best) check both locations in `load_best_effort()`. The simplest single-line fix: change line 38 to `const MEMORY_DIR: &str = ".roko";`.
- **Also check**: `TASK_METRICS_FILE` path at line 40 — ensure it also aligns with where the orchestrator writes task metrics.
- **Verify**: `cargo check -p roko-cli`. After running agents and generating episodes, the TUI logs tab (F5) shows episode entries.
- **Acceptance**: Episode data appears in TUI when `.roko/episodes.jsonl` exists.

### 0.3 Add Mouse variant to Event enum
- **Files**: `crates/roko-cli/src/tui/event.rs` (75 lines)
- **Problem**: The `Event` enum at line ~10 has only `Key(KeyEvent)`, `Resize(u16, u16)`, `Tick` variants. No `Mouse` variant. In `EventHandler::next()` at line ~57-73, crossterm `Event::Mouse(...)` hits `_ => continue` and is silently discarded. The terminal has `EnableMouseCapture` active but mouse events never reach the app.
- **Fix**: Add `Mouse(crossterm::event::MouseEvent)` variant to the `Event` enum. In `EventHandler::next()`, add a match arm: `crossterm::event::Event::Mouse(m) => return Ok(Event::Mouse(m))`. In `app.rs main_loop()` at line ~332, add `Event::Mouse(m) => { self.handle_mouse(m); }` to the event match.
- **Verify**: `cargo check -p roko-cli`. Mouse clicks change focus, scroll wheel scrolls.
- **Acceptance**: Clicking on a panel changes focus. Scroll wheel scrolls content.

### 0.4 Fix mouse hit-test hardcoded 80x24
- **Files**: `crates/roko-cli/src/tui/app.rs:1093-1094`
- **Problem**: `handle_mouse()` computes hit zones using `Rect::new(0, 0, 80, 24)` instead of actual terminal size. Every click on terminals >80x24 maps to the wrong zone.
- **Fix**: Store the terminal size in `App` (update on `Event::Resize`) and use it in `handle_mouse()`. Add a field `terminal_size: (u16, u16)` to `App`, initialize from `crossterm::terminal::size()`, update on resize events, use in hit-test.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Mouse clicks target correct panels on any terminal size.

---

## Phase 1: Data flow wiring

### 1.1 Bridge gate_results into TuiState
- **Files**: `crates/roko-cli/src/tui/state.rs` method `update_from_snapshot()` around line 827
- **Problem**: `DashboardData` has `gate_results: Vec<GateResultSummary>` with fields `{gate, plan_id, passed, summary}`. `TuiState` has `gate_results: Vec<GateResultEntry>` with fields `{gate, plan_id, passed, output}`. The field names differ (`summary` vs `output`). No code bridges them.
- **Fix**: In `update_from_snapshot()`, after the cost section (~line 832), add:
  ```rust
  self.gate_results = data.gate_results.iter().map(|g| GateResultEntry {
      gate: g.gate.clone(),
      plan_id: g.plan_id.clone(),
      passed: g.passed,
      output: g.summary.clone(),
  }).collect();
  ```
- **Verify**: `cargo check -p roko-cli`. Gate results appear in dashboard Errors sub-tab.
- **Acceptance**: Error digest widget shows real gate pass/fail results.

### 1.2 Populate token_history and token_rate
- **Files**: `crates/roko-cli/src/tui/state.rs` method `update_from_snapshot()`
- **Problem**: `token_history: HashMap<String, VecDeque<u64>>` and `token_rate: f64` are never populated. The `token_sparkline` widget always shows "waiting for data...".
- **Fix**: In `update_from_snapshot()`, build `token_history` from `data.efficiency_events`. Group events by role, bucket by time window (~5 seconds), push token sums into VecDeque per role. Compute `token_rate` as total tokens / elapsed minutes from the events.
- **Data source**: `data.efficiency_events` is `Vec<AgentEfficiencyEvent>` with fields `{role, input_tokens, output_tokens, timestamp_ms, ...}`.
- **Verify**: `cargo check -p roko-cli`. Token sparkline renders data when efficiency events exist.
- **Acceptance**: Token sparkline shows a chart instead of "waiting for data...".

### 1.3 Populate PlanEntry nested tasks, elapsed, wave, failures
- **Files**: `crates/roko-cli/src/tui/state.rs` method `update_from_snapshot()` around line 730-757
- **Problem**: `PlanEntry.tasks` is always `Vec::new()`, `.tasks_failed` always 0, `.elapsed_secs` always 0.0, `.wave` always None.
- **Fix**: For each plan in `data.plans`, load its task definitions from the plan's TOML file (use `crate::task_parser::TasksFile`). Populate `tasks: Vec<TaskEntry>` with task id/name/status. Count failed tasks from task-tracker data. Compute elapsed from episode durations. Set wave from plan metadata if available.
- **Data sources**: `data.plans` has `PlanSummary { id, title, completed, task_count, dir }`. Task definitions are in `{plan_dir}/tasks.toml`.
- **Verify**: `cargo check -p roko-cli`. Expanded plans show nested tasks. Progress bars show partial progress.
- **Acceptance**: Plan tree shows intermediate progress (e.g., 3/10 instead of 0/10 or 10/10).

### 1.4 Populate orchestrator_state, current_iteration, current_phase
- **Files**: `crates/roko-cli/src/tui/state.rs` method `update_from_snapshot()`
- **Problem**: These fields are permanently "idle", 0, and empty. Should reflect real orchestrator state from executor state JSON.
- **Fix**: In `update_from_snapshot()`, read from `data.executor_state` (if it has an `orchestrator_state` or `status` field). Extract iteration count and current phase label. `DashboardData` loads `.roko/state/executor.json` — check what fields that JSON contains and map them.
- **Verify**: `cargo check -p roko-cli`. Header bar shows "running" during active execution.
- **Acceptance**: Orchestrator state indicator reflects actual state.

### 1.5 Populate git structured fields from background thread
- **Files**: `crates/roko-cli/src/tui/app.rs` `drain_background_channels()` around line 1400-1430
- **Problem**: `GitBgData` carries `view_data: Option<GitViewData>` which is stored in `tui_state.git_view_data`. But the structured fields `git_branch_tree`, `git_commit_graph`, `git_worktree_list` are never extracted from it.
- **Fix**: In the `git_rx` handler, after setting `git_view_data`, also populate:
  ```rust
  if let Some(ref vd) = self.tui_state.git_view_data {
      self.tui_state.git_branch_tree = vd.branches.clone(); // map types if needed
      self.tui_state.git_commit_graph = vd.commits.clone();
      self.tui_state.git_worktree_list = vd.worktrees.clone();
  }
  ```
  May need type conversions between `GitViewData` fields and `state::GitBranchNode`/`GitCommitEntry` types.
- **Also fix**: `git_age` is never updated by background thread (`app.rs:305` sends empty string). Compute age from `git log -1 --format=%cr` in the background thread.
- **Verify**: `cargo check -p roko-cli`. Git tab (F4) shows branch tree.
- **Acceptance**: Git tab shows real branches, commits, worktrees.

### 1.6 Unify notification systems
- **Files**: `crates/roko-cli/src/tui/state.rs:138-151`, `crates/roko-cli/src/tui/modals/notification.rs`, `crates/roko-cli/src/tui/app.rs:62`
- **Problem**: Two separate `Notification` types exist: `state::Notification` (with `level: NotificationLevel`) in `TuiState.notifications` (never populated), and `modals::Notification` (with `kind: NotificationKind`) in `App.notifications` (actually used). They never interact.
- **Fix**: Remove `state::Notification` and `state::NotificationLevel`. Remove `TuiState.notifications`. Use only `App.notifications` with `modals::Notification`. Or alias them. The key is having ONE notification system.
- **Verify**: `cargo check -p roko-cli`. Notifications still render after inject/confirm/save.
- **Acceptance**: Single notification type, no dead `TuiState.notifications` field.

### 1.7 Add cross-platform system metrics
- **Files**: `crates/roko-cli/src/tui/app.rs:1633` (`collect_sys_metrics_bg`)
- **Problem**: System metrics only work on macOS via `top -l 2`. Linux returns `SysMetrics::default()` (all zeros). No indication to user.
- **Fix**: Add `sysinfo` crate as dependency to `roko-cli/Cargo.toml`. Replace the `#[cfg(target_os = "macos")]` `top` parsing with `sysinfo::System` API calls that work cross-platform: `sys.global_cpu_usage()`, `sys.used_memory()`, `sys.total_memory()`.
- **Also fix**: Network/disk fields named `_bytes_sec` but storing totals (app.rs:1682-1683). Compute actual rates by storing previous values and dividing by elapsed time.
- **Verify**: `cargo check -p roko-cli`. Sys metrics widget shows real CPU/MEM on both macOS and Linux.
- **Acceptance**: `sys_metrics` widget shows non-zero CPU% and memory on any platform.

### 1.8 Fix plan status to show partial progress
- **Files**: `crates/roko-cli/src/tui/state.rs:734-739`
- **Problem**: `PlanEntry.phase` is binary "done"/"pending". `tasks_done` is 0 or total (all-or-nothing). Progress bars jump from 0% to 100%.
- **Fix**: Use task-tracker data to count individual completed tasks. `DashboardData` has `task_trackers` or per-plan task tracker snapshots. Count tasks with status "completed"/"passed" for `tasks_done`, "failed" for `tasks_failed`. Derive phase from the most advanced active task.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Progress bars show intermediate values (e.g., 3/10 tasks done).

---

## Phase 2: Input / interaction fixes

### 2.1 Render inject mode input line
- **Files**: `crates/roko-cli/src/tui/app.rs` `draw()` method around line 373-469
- **Problem**: When `input_mode == InputMode::Inject`, the user types but nothing is rendered. No input box, no cursor, no mode indicator. User types blind.
- **Fix**: In `draw()`, after rendering the status footer and before modals, check `self.tui_state.input_mode`. If `Inject`, render a 1-line input bar overlaying the bottom of the content area. Show mode label "inject>" in accent color, then `self.tui_state.message_input` text, then a blinking cursor block. Use `Clear` + `Paragraph` with the input text.
- **Verify**: `cargo check -p roko-cli`. Press `i` on dashboard — input bar appears. Type text — it's visible. Press Enter — notification shows. Press Esc — bar disappears.
- **Acceptance**: User can see what they're typing during inject mode.

### 2.2 Render filter mode input line + wire filter_text to filter
- **Files**: `crates/roko-cli/src/tui/app.rs` `draw()`, `dispatch_action()` around line 728-736
- **Problem**: Same as inject — no visual rendering. Additionally, typing goes to `filter_text` but `plan_tree.rs` reads `filter` (different field). The `AcceptFilter` action at line 728-731 sets `filter_active` but never copies `filter_text` into `filter`.
- **Fix**: (1) Same input bar rendering as 2.1 but with "filter>" label. (2) In `dispatch_action` for `AcceptFilter`, add `self.tui_state.filter = self.tui_state.filter_text.clone();`. (3) In `update_from_snapshot()` at state.rs:847, the line `self.filter = self.filter_text.clone()` already exists — verify it runs AFTER AcceptFilter sets filter_text.
- **Verify**: `cargo check -p roko-cli`. Press `/` on Plans tab — filter bar appears. Type text — it's visible. Press Enter — plan tree filters.
- **Acceptance**: Filter mode shows input text AND actually filters the plan tree.

### 2.3 Fix PageUp/PageDown to scroll by page
- **Files**: `crates/roko-cli/src/tui/input.rs` (PageUp/PageDown handlers), `crates/roko-cli/src/tui/app.rs` `scroll_focused()`
- **Problem**: PageUp/PageDown call `scroll_focused(+/-1)`, scrolling by 1 line (identical to Up/Down).
- **Fix**: Add a `page_size` parameter (default 20 or compute from area height). In `input.rs`, return a new action like `ScrollFocusedPageUp`/`PageDown`, or pass a larger delta. Simplest: in `dispatch_action`, when the action is `ScrollFocusedUp` from PageUp, call `scroll_focused(-20)` instead of `scroll_focused(-1)`. You can distinguish by adding a `page: bool` field to the action, or by creating `ScrollPageUp`/`ScrollPageDown` variants.
- **Also fix**: Home should go to offset 0, End should go to content length.
- **Verify**: `cargo check -p roko-cli`. PageUp/PageDown move content by ~20 lines.
- **Acceptance**: PageUp is noticeably faster than Up.

### 2.4 Wire ScrollAccel into scroll handlers
- **Files**: `crates/roko-cli/src/tui/scroll.rs` (95 lines), `crates/roko-cli/src/tui/app.rs`
- **Problem**: `ScrollAccel` in `scroll.rs` is a complete acceleration system (1x→2x→4x→8x within 300ms). It's exported from mod.rs but never instantiated. `App` has no `ScrollAccel` field.
- **Fix**: Add `scroll_accel: ScrollAccel` field to `App` struct. In `dispatch_action`, for scroll actions, call `self.scroll_accel.push(direction)` to get the accelerated delta, then use that delta instead of hardcoded 1.
- **Verify**: `cargo check -p roko-cli`. Holding Down key accelerates scroll speed.
- **Acceptance**: Rapid key-repeat scrolls faster than single presses.

### 2.5 Fix plan_scroll vs plan_scroll_offset desync
- **Files**: `crates/roko-cli/src/tui/state.rs:400,402`, `crates/roko-cli/src/tui/app.rs:1124`, `crates/roko-cli/src/tui/widgets/plan_tree.rs:148`
- **Problem**: Key handlers modify `plan_scroll_offset` (via `scroll_focused()`). Plan tree widget reads `plan_scroll`. These are two separate fields that are never synchronized.
- **Fix**: Remove `plan_scroll` field. Change `plan_tree.rs` to read `plan_scroll_offset` instead. Update all references. Or: rename `plan_scroll_offset` to `plan_scroll` and remove the other.
- **Verify**: `cargo check -p roko-cli`. Scrolling in plan tree actually moves the view.
- **Acceptance**: Up/Down in plan tree with focus on PlanTree scrolls the visible plan list.

### 2.6 Fix output_scroll vs agent_scroll desync
- **Files**: `crates/roko-cli/src/tui/state.rs:386,388`, `crates/roko-cli/src/tui/widgets/agent_output.rs:66`
- **Problem**: Key handlers modify `agent_scroll: Option<usize>`. Some widgets read `output_scroll: usize`. These are separate fields.
- **Fix**: Remove `output_scroll`. Change all widget reads to use `agent_scroll.unwrap_or(0)`. Or: sync them in dispatch_action.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Agent output scroll position reflects keypress changes.

### 2.7 Add key intercepts for all open modals
- **Files**: `crates/roko-cli/src/tui/input.rs:292-302`
- **Problem**: `handle_key()` only checks `show_task_picker`, `show_task_detail`, `show_queue_overview` for modal intercepts. `show_wave_overview`, `show_help`, `show_plan_detail`, `show_agent_pool_modal` have NO key intercept. When these modals are open, keys fall through to tab handlers. `q` quits instead of closing the modal.
- **Fix**: Add intercept blocks for each unchecked modal. For all of them, Esc/q should close the modal, Up/Down should scroll, and other keys should be consumed (return `TuiAction::None`).
- **Verify**: `cargo check -p roko-cli`. Press `w` to open wave overview, then Esc to close it.
- **Acceptance**: Every modal can be closed with Esc. Keys don't leak through to background tab.

### 2.8 Make focus cycling tab-aware
- **Files**: `crates/roko-cli/src/tui/input.rs:54-75`
- **Problem**: Tab/BackTab cycles through all 5 focus zones (PlanTree, TaskProgress, AgentOutput, CommandOutput, RightPanel) regardless of which tab is active. On Logs/Config/Inspect tabs, this cycles through invisible zones.
- **Fix**: Make `FocusZone::next()`/`prev()` take the active `Tab` as parameter. On single-pane tabs (Logs, Config, Inspect), cycle between fewer zones. On Dashboard, keep all 5. On Plans, use PlanTree + RightPanel. On Agents, use AgentOutput + RightPanel.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Tab cycling on Logs tab doesn't visit PlanTree/AgentOutput zones.

### 2.9 Add visible focus indicator
- **Files**: `crates/roko-cli/src/tui/views/dashboard_view.rs`, `plans_view.rs`, `agents_view.rs`
- **Problem**: Focus zone changes internally but most panels have no visible indicator. Only plan_tree shows a subtle title text change.
- **Fix**: When rendering each panel, check if `tui_state.focus` matches this panel's zone. If focused, use `Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.accent))` instead of the default muted border.
- **Verify**: `cargo check -p roko-cli`. Press Tab — border color of focused panel changes.
- **Acceptance**: User can visually identify which panel has focus.

### 2.10 Fix DrillIn/DrillOut to be tab-specific
- **Files**: `crates/roko-cli/src/tui/app.rs:794-811`
- **Problem**: `DrillIn`/`DrillOut` always toggle `plans[selected].expanded` regardless of active tab. On Git tab, Left/Right should navigate git branches, not expand plans.
- **Fix**: In `dispatch_action` for `DrillIn`/`DrillOut`, check `self.tui_state.active_tab`. On `Tab::Git`, navigate git branch cursor. On `Tab::Inspect`, navigate context nodes. On `Tab::Dashboard`/`Tab::Plans`, expand/collapse plans as before.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Left/Right on Git tab navigates branches, not plans.

### 2.11 Fix Logs End/G to scroll log pane
- **Files**: `crates/roko-cli/src/tui/input.rs:598`
- **Problem**: Logs tab `End`/`G` maps to `ScrollAgentEnd` which resets agent output scroll, not log scroll.
- **Fix**: Change line 598 to return a log-specific scroll action. Either create `ScrollLogEnd` or map to something that sets `log_scroll` to the end. Simplest: `TuiAction::ScrollLogDown` with a large delta, or add `ScrollLogEnd` variant.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Pressing End on Logs tab scrolls to bottom of log.

### 2.12 Add scroll upper-bound clamping
- **Files**: `crates/roko-cli/src/tui/app.rs` all scroll handlers in `dispatch_action()`
- **Problem**: No upper bound clamping on any scroll field. User can scroll past content into empty space.
- **Fix**: After incrementing any scroll field, clamp to `content_length.saturating_sub(visible_height)`. This requires knowing content length, which may need to be stored in TuiState or computed at scroll time.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Cannot scroll past the last line of content.

### 2.13 Fix Ctrl-C to force-quit in all states
- **Files**: `crates/roko-cli/src/tui/input.rs`
- **Problem**: Ctrl-C is handled in `handle_global_key` but modal intercepts run first. When task_picker, confirm, inject, or filter mode is active, Ctrl-C returns `TuiAction::None`.
- **Fix**: Check Ctrl-C BEFORE modal intercepts in `handle_key()`. Add at the very top of `handle_key`: `if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) { return TuiAction::Quit; }`.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Ctrl-C always quits, even in inject/filter/confirm modes.

---

## Phase 3: Modal fixes

### 3.1 Create PlanDetail modal + wire rendering
- **Files**: `crates/roko-cli/src/tui/modals/mod.rs`, `crates/roko-cli/src/tui/modals/plan_detail.rs`, `crates/roko-cli/src/tui/app.rs`
- **Problem**: `show_plan_detail` boolean toggles but no `ModalState::PlanDetail` variant exists. No render function is called. Enter on a plan does nothing visible.
- **Fix**: Add `PlanDetail { plan_id: String, scroll_offset: usize }` variant to `ModalState` enum. In `dispatch_action` for `ShowPlanDetail`, create this variant with the selected plan's ID. In `render_modal()`, render the plan detail with task list, status, timing. Populate `plan_detail_content` from the plan's task definitions.
- **Verify**: `cargo check -p roko-cli`. Press Enter on a plan — detail modal appears with plan info.
- **Acceptance**: Plan detail modal shows plan name, tasks, status, progress.

### 3.2 Populate data modals with real data
- **Files**: `crates/roko-cli/src/tui/app.rs:583-612`
- **Problem**: WaveOverview, QueueOverview, TaskPicker all open with `Vec::new()`.
- **Fix**: Pass real data: `waves: self.tui_state.execution_waves.clone()` for WaveOverview. `tasks: self.tui_state.current_task_checklist.iter().map(...)` for TaskPicker.
- **Verify**: `cargo check -p roko-cli`. Press `w` — wave overview shows real wave data.
- **Acceptance**: Modals show real data instead of empty lists.

### 3.3 Unify modal systems into one
- **Files**: `crates/roko-cli/src/tui/app.rs:60,84`, `crates/roko-cli/src/tui/state.rs:410-422`, `crates/roko-cli/src/tui/modals/mod.rs`
- **Problem**: Three parallel systems: `active_modal: Option<ModalState>` in App, `show_*` booleans in TuiState, legacy `overlay: Option<OverlayState>` in App. They can desync.
- **Fix**: Pick `ModalState` enum as the single source of truth. Remove all `show_*` booleans from TuiState. Remove legacy `overlay`. Update `has_modal()` to check the single `active_modal` field. Update all key intercepts in `input.rs` to check `active_modal` variants instead of `show_*` booleans. Update `draw()` to render based on `active_modal`.
- **Verify**: `cargo check -p roko-cli`. All modals open/close correctly.
- **Acceptance**: One modal tracking system. No `show_*` booleans. No legacy overlay.

### 3.4 Wire confirm actions to signal orchestrator
- **Files**: `crates/roko-cli/src/tui/app.rs:748-778`
- **Problem**: ConfirmYes writes a JSON signal to `signals.jsonl` but the orchestrator doesn't read this file for commands. All confirm actions (restart, reset, force-advance, reverify) are logging-only.
- **Fix**: Write a control signal that the orchestrator's signal watcher can pick up. Either: (a) write to a dedicated `.roko/state/control.json` file that the orchestrator polls, or (b) use the existing DashboardEvent channel if TUI runs alongside the orchestrator. At minimum, document that confirm actions require a running orchestrator with signal polling.
- **Verify**: After confirming a restart, the signal appears in signals.jsonl with `kind: "roko.tui.confirm"`.
- **Acceptance**: Confirm actions produce actionable signals.

---

## Phase 4: Dead code cleanup

### 4.1 Remove dead widget files
- **Files**: 13 widget files with zero callers
- **List**: `widgets/agent_grid.rs`, `widgets/agent_output.rs`, `widgets/agent_pool.rs`, `widgets/command_output.rs`, `widgets/context_gauge.rs`, `widgets/phase_bar.rs`, `widgets/phase_timeline.rs`, `widgets/plan_list.rs`, `widgets/scrollbar.rs`, `widgets/status_badge.rs`, `widgets/tab_bar.rs`, `widgets/token_bar.rs`, `widgets/wave_bar.rs`
- **Fix**: Delete these files. Remove their `mod` declarations from `widgets/mod.rs`. Remove any `pub use` exports from `tui/mod.rs`. Fix any resulting compilation errors (there should be none since they have zero callers).
- **Verify**: `cargo check -p roko-cli`. `cargo test -p roko-cli`.
- **Acceptance**: 13 fewer files, ~1600 fewer LOC. No functionality lost.

### 4.2 Remove dead modal file (help.rs)
- **Files**: `crates/roko-cli/src/tui/modals/help.rs`
- **Problem**: Uses `crate::tui::mori_theme::MoriTheme` — a module path that doesn't exist. Never called from app.rs (which uses its own `render_help_overlay`).
- **Fix**: Delete the file. Remove from `modals/mod.rs`.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: No broken import. Help overlay still works via app.rs built-in.

### 4.3 Remove duplicate TuiState fields
- **Files**: `crates/roko-cli/src/tui/state.rs`
- **Problem**: Multiple duplicate fields: `selected_plan` + `selected_plan_idx` (use one), `cumulative_cost_usd` + `cost_dollars` (same value), `output_scroll` (dead, agent_scroll is used), `plan_scroll` (dead after 2.5 fix), `log_messages` (dead, logs_view builds its own), `notifications` (dead after 1.6 fix).
- **Fix**: Remove the dead fields. Update all references.
- **Verify**: `cargo check -p roko-cli`. `cargo test -p roko-cli`.
- **Acceptance**: No duplicate fields in TuiState.

### 4.4 Resolve type duplications
- **Files**: Various widget files, state.rs
- **Problem**: 6 types defined differently in widgets vs state: `ParallelAgentState`, `PlanEntry`, `PhaseEntry`, `AgentState`/`AgentRow`, `WaveProgress`, `GitTreeNode`/`GitBranchNode`.
- **Fix**: After removing dead widgets (4.1), most duplicates are gone. For remaining ones: use the `state.rs` types as canonical. Remove widget-local type definitions.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Each domain concept has exactly one type definition.

### 4.5 Remove legacy rendering path
- **Files**: `crates/roko-cli/src/tui/widgets/mod.rs` (`render_dashboard` ~2656 lines), `crates/roko-cli/src/tui/pages/` directory
- **Problem**: Legacy `render_dashboard` function is 2656 lines but never called. Pages directory is all placeholder scaffolds.
- **Fix**: Remove `render_dashboard` from `widgets/mod.rs`. Consider keeping `pages/` if the text-mode fallback uses it, or remove if text-mode is also dead.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Legacy code removed. No functionality lost.

---

## Phase 5: Widget completion

### 5.1 Fix diff_panel to show real git diffs
- **Files**: `crates/roko-cli/src/tui/views/dashboard_view.rs` `gather_diff_text()`, `crates/roko-cli/src/tui/widgets/diff_panel.rs`
- **Problem**: Diff panel shows agent output text filtered for `+`/`-`/`@@`/`diff ` prefixes. Not actual git diffs.
- **Fix**: Run `git diff HEAD~1` (or `git diff` for unstaged) for the selected plan's worktree. Store the diff output in a data field. Pass real diff content to diff_panel.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Diff sub-tab shows actual git diff output.

### 5.2 Add ANSI parsing to agent output
- **Files**: `crates/roko-cli/src/tui/views/agents_view.rs` `render_output_body()`, `crates/roko-cli/src/tui/views/dashboard_view.rs`
- **Problem**: Agent output is rendered as plain text. No ANSI color codes parsed.
- **Fix**: Add a simple ANSI escape sequence parser that converts `\x1b[...m` sequences to ratatui `Style` attributes. Apply these styles when building `Span`s for the output text.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Agent output with ANSI colors renders with correct colors in the TUI.

### 5.3 Fix phase_compact to use real phase data
- **Files**: `crates/roko-cli/src/tui/state.rs` `build_phase_pipeline()` around line 925-971
- **Problem**: Phase pipeline uses position-based heuristic (midpoint/3). Phase progress is synthetic.
- **Fix**: Map actual task statuses to canonical phases. If task status is "compile-gate", mark the compile-gate phase as Active. If all tasks before a phase are done, mark that phase as Done.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Phase bar reflects actual execution phase, not a synthetic midpoint.

### 5.4 Add auto-tail / scroll pinning to agent output
- **Files**: `crates/roko-cli/src/tui/views/agents_view.rs`, `dashboard_view.rs`
- **Problem**: Agent output always uses absolute scroll offset. No auto-tail that follows latest output.
- **Fix**: Use `agent_scroll: Option<usize>` semantics: `None` = auto-tail (scroll to end), `Some(n)` = pinned at line n. When new output arrives and scroll is `None`, automatically show the latest lines. When user scrolls up, set to `Some(offset)`. `End` key resets to `None`.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Agent output auto-scrolls to latest line. Scrolling up pins. End resumes auto-tail.

---

## Phase 6: Rendering / polish

### 6.1 Enable PostFX pipeline via config
- **Files**: `crates/roko-cli/src/tui/effects_config.rs`, `crates/roko-cli/src/tui/app.rs:458`, `crates/roko-cli/src/tui/config_meta.rs`
- **Problem**: `EffectsConfig::default()` sets all effects to false. No config key or keyboard toggle exists. The entire PostFX pipeline is unreachable.
- **Fix**: Add `[tui.effects]` section to roko.toml schema. Add `screen_postfx`, `drop_shadows`, `modal_glow` boolean keys. Read these in `App::new()` and set `fx_config`. Add a keyboard toggle (e.g., `Ctrl-E`).
- **Verify**: `cargo check -p roko-cli`. Enable effects in config, see visual effects.
- **Acceptance**: PostFX effects can be toggled on.

### 6.2 Unify theme systems
- **Files**: `crates/roko-cli/src/tui/dashboard.rs:80` (Theme), `crates/roko-cli/src/tui/widgets/rosedust.rs` (MoriTheme)
- **Problem**: `Theme` struct used by modals and app.rs. `MoriTheme` struct used by widgets. Same colors, different APIs.
- **Fix**: Pick one. Since `MoriTheme` has more features (gradients, brighten), use it everywhere. Or create a shared `TuiTheme` that both can derive from.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: One theme type used throughout.

### 6.3 Fix dual Atmosphere instances
- **Files**: `crates/roko-cli/src/tui/app.rs` (App.atmosphere + tui_state.atmosphere)
- **Problem**: Both `app.atmosphere` and `tui_state.atmosphere` are ticked independently. Widgets read from `tui_state.atmosphere`. PostFX reads from `app.atmosphere`. Potential phase drift.
- **Fix**: Remove `app.atmosphere`. Use only `tui_state.atmosphere`. Update PostFX to read from `tui_state.atmosphere`.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Single atmosphere instance. No phase drift.

### 6.4 Add message throttle
- **Files**: `crates/roko-cli/src/tui/app.rs` event loop
- **Problem**: No `MAX_MESSAGES_PER_TICK` limit. Rapid orchestrator output can starve rendering.
- **Fix**: In `drain_background_channels()`, limit processing to 20 messages per drain call. Process remaining on next tick.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: TUI remains responsive during rapid output.

### 6.5 Add F7 to header F-key strip
- **Files**: `crates/roko-cli/src/tui/widgets/header_bar.rs`
- **Problem**: F-key strip shows F1-F6 only. F7 (Inspect) is hidden.
- **Fix**: Add `("F7", "inspect")` to the F-key strip rendering.
- **Verify**: `cargo check -p roko-cli`.
- **Acceptance**: Header shows F1-F7.

### 6.6 Add tracing/logging to TUI
- **Files**: Throughout `crates/roko-cli/src/tui/`
- **Problem**: Zero logging anywhere. All errors invisible. 20+ `.ok()` sites silently swallow errors.
- **Fix**: Add `tracing` subscriber that writes to a `.roko/tui.log` file (not stdout, since terminal is in raw mode). Replace key `.ok()` calls with `.inspect_err(|e| tracing::warn!("...: {e}"))`. At minimum, log: thread spawn failures, file read errors, JSON parse errors, signal write failures.
- **Verify**: `cargo check -p roko-cli`. After running TUI, `.roko/tui.log` contains log entries.
- **Acceptance**: Errors are no longer invisible. Debug log available.

### 6.7 Replace stringly-typed status with enums
- **Files**: `crates/roko-cli/src/tui/state.rs`, various views
- **Problem**: 90+ sites compare status via string literals ("running", "active", "done", "completed", "passed", etc.) with no compile-time checking.
- **Fix**: Define `AgentStatus`, `TaskStatus`, `PlanPhase` enums. Replace all string comparisons with enum matches. Implement `From<&str>` for backward compatibility with JSON deserialization.
- **Verify**: `cargo check -p roko-cli`. `cargo clippy -p roko-cli`.
- **Acceptance**: No raw status string comparisons in TUI code.
