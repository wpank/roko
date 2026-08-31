# TUI Parity: Implementation Plan

> **Status note (2026-08-31):** This is the original implementation plan, not
> a live completion report. PR #74's merged baseline was live-audited as 19
> verified, 11 partial/scaffolded, and 8 not operational. Follow-up source fixes
> for P0.2 and P0.4 yield 21 source-implemented, 9 partial, and 8 not operational;
> they still require final interactive verification. The current checkbox/status
> ledger is [00-INDEX.md](00-INDEX.md), and the evidence is
> [the post-merge live audit](audits/post-merge-live-audit-2026-08-31.md).

> Phased plan with file:line targets, verification steps, and dependencies.
> All paths are relative to `/Users/will/dev/nunchi/roko/roko/`.

## Expanded integration reconciliation

The development-speed integration adds the following source-complete work without redefining the
remaining P0–P7 gaps:

- `88c724744`: connected sessions honor the configured refresh cadence, skip idle redraws, omit
  the broad filesystem watcher when StateHub is authoritative, and exclude high-churn trees from
  fallback watches.
- `52d5f4df4`: terminal run duration/outcome becomes immutable, active plan/agent state converges,
  confirmed and unconfirmed PIDs remain distinguishable, and replayed plan/task/agent lifecycle
  counters are idempotent.
- Active integration handoff: preserve the authoritative `PlanStarted.tasks_total`, maintain
  cumulative connected token history without zero-delta EMA corruption, avoid panic-hook reset
  during unwinding, and use wrapping post-processing seed arithmetic.

These changes address the pictured stale timer/task denominator, idle CPU/watcher pressure, and
debug abort. They do **not** make pause/retry/skip operational, create the missing gate-output
producer, populate critical-path ETA or plan-detail fields, remove the remaining MCP render read,
or repair incomplete focus/scroll paths. No interactive/live acceptance is claimed until the
coordinator's final batched run completes.

---

## Phase 1 (P0): Make the TUI show live data during plan run

**Goal**: Fix the `DashboardData` / `TuiState` split so that token sparklines, efficiency
panels, cost-by-model, and progress bars show real values during `plan run`.

**Root cause**: In connected mode, `DashboardData` is initialized as `default()` at
`app.rs:587-588` and never refreshed. Several render widgets read from `DashboardData`
instead of `TuiState`.

### P0.1: Make token sparkline read from TuiState

**What**: When `DashboardData.efficiency_events` is empty (connected mode), the token
sparkline should synthesize display data from `TuiState.cumulative_input_tokens`,
`cumulative_output_tokens`, and `cost_dollars`. Add a `token_event_ring: VecDeque<u64>`
(capacity 120) to `DashboardSnapshot` that records each token event value for sparkline
granularity.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-core/src/dashboard_snapshot.rs` | Add `pub token_event_ring: VecDeque<u64>` to `DashboardSnapshot`. In `apply()` for `EfficiencyEvent` with metric `"input_tokens"` or `"output_tokens"`, push to ring buffer (bounded to 120 entries). |
| `crates/roko-cli/src/tui/state.rs` | Add `pub token_event_history: VecDeque<u64>` to `TuiState`. In `update_from_dashboard_snapshot()` (~line 2968), populate from `snap.token_event_ring`. |
| `crates/roko-cli/src/tui/widgets/token_sparkline.rs:75` | In `render_token_sparkline()`, check if `data.efficiency_events` is empty. If so, build the sparkline from `tui_state.token_event_history` and `tui_state.cumulative_input_tokens` / `cost_dollars`. |
| `crates/roko-cli/src/tui/pages/efficiency.rs:39` | In `build_efficiency_snapshot()`, accept an optional `&TuiState` parameter. When `data` is empty, synthesize the snapshot from TuiState fields. |
| `crates/roko-cli/src/tui/views/dashboard_view.rs:1937` | Pass `&tui_state` to the sparkline render call alongside `&data`. |

### P0.2: Add tasks_total to PlanStarted event

**What**: The runner knows the total task count at plan load time but never publishes it.
`DashboardEvent::PlanStarted` only carries `plan_id`. Add `tasks_total: usize`.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-core/src/dashboard_snapshot.rs:78-80` | Add `tasks_total: usize` field to `DashboardEvent::PlanStarted`. |
| `crates/roko-core/src/dashboard_snapshot.rs:1153-1163` | In `apply()` for `PlanStarted`, set `plan.tasks_total = tasks_total` instead of starting at 0. |
| `crates/roko-cli/src/runner/tui_bridge.rs:24-28` | Update `plan_started()` to accept and forward `tasks_total`. |
| `crates/roko-cli/src/runner/event_loop.rs` | At the call site where `tui.plan_started()` is called, pass `plan.tasks.len()` from the loaded plan. |

### P0.3: Fix cost_usd event ordering race

**What**: The cost event fires AFTER `agent_completed` in the same `TurnCompleted` handler,
so `find_agent_key_for_task()` returns `None` because the agent is already inactive.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/runner/agent_events.rs:160-187` | Move the `tui.efficiency_event("cost_usd", ...)` call (line 187) to BEFORE `tui.agent_completed()` (line 162). Or, include `agent_key` in the efficiency event payload so `find_agent_key_for_task` is not needed. |
| `crates/roko-core/src/dashboard_snapshot.rs:1635-1650` | Alternative: make `find_agent_key_for_task()` also search recently-inactive agents (where `active == false` but `last_event_ts` is within 5 seconds). |

### P0.4: Populate token_rate and token_history from snapshots

**What**: `TuiState.token_rate` and `token_history` are only populated in the disk-based
`update_from_snapshot()` path, not in the connected-mode `update_from_dashboard_snapshot()`.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/state.rs:2968-2974` | In `update_from_dashboard_snapshot()`, after computing `cumulative_input_tokens` and `cumulative_output_tokens`, compute `token_rate` as `(total_tokens as f64) / elapsed_secs`. Push the current `token_total` to `token_history` (bounded VecDeque). |

### P0.5: Bridge learning/efficiency data to connected-mode TuiState

**What**: The learning sub-tab, efficiency sparkline, and cost-by-model widget read from
`DashboardData` fields that are empty in connected mode. Several of these already have
partial `TuiState` equivalents (`cascade_router_json`, `gate_thresholds_json`).

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/state.rs` | Add `pub efficiency_events_live: Vec<AgentEfficiencyEvent>` to `TuiState`. In `update_from_dashboard_snapshot()`, reconstruct efficiency event entries from per-agent token/cost deltas in the snapshot. |
| `crates/roko-cli/src/tui/views/dashboard_view.rs` | For the learning sub-tab render, check `tui_state.cascade_router_json` before `data.cascade_router`. Same pattern for experiments and adaptive thresholds. |
| `crates/roko-cli/src/tui/widgets/cost_by_model.rs` | Accept an optional `&TuiState` parameter. When `data.efficiency_events` is empty, build the cost-by-model table from `tui_state.agents` (which have per-agent model, cost, token data). |

**Verification**:

1. Run `cargo test --workspace` -- all existing tests pass.
2. Run `cargo run -p roko-cli -- plan run plans/ --engine runner-v2 --approval` with a real plan.
3. Verify: token sparkline shows non-zero values within 30s of first agent dispatch.
4. Verify: progress bar shows correct denominator from plan start (not incrementing from 0).
5. Verify: cost_usd updates per-agent (not just total).
6. Verify: learning sub-tab shows cascade router data during run.

**Dependencies**: None (this is the first phase).

---

## Phase 2 (P1): Make interaction actually work

**Goal**: Recovery keybindings, search, filter, and other interactive features produce real
effects instead of being facade-only.

**Depends on**: Phase 1 is independent, but Phase 2 benefits from Phase 1's data fixes.

### P1.1: Add TUI-to-runner command channel

**What**: Create an `mpsc::Sender<TuiCommand>` that the TUI App holds and the runner event
loop receives. When recovery keys are confirmed, send a command instead of (or in addition
to) writing to `engrams.jsonl`.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/runner/types.rs` | Define `pub enum TuiCommand { Pause, Resume, SoftRetry { plan_id: String }, Repair { plan_id: String, preserve_completed: bool }, ReverifyGates { plan_id: String }, Skip { plan_id: String, task_id: String }, Cancel { plan_id: String } }`. |
| `crates/roko-cli/src/runner/event_loop.rs` | Accept `Option<mpsc::Receiver<TuiCommand>>` in the event loop. Add a `select!` branch that polls the receiver. Handle each variant: `Pause` sets a `paused` flag checked before dispatching next task; `Resume` clears it; `SoftRetry` requeues failed tasks; etc. |
| `crates/roko-cli/src/tui/app.rs:1610-1666` | In the `ConfirmYes` handler, after writing to `engrams.jsonl`, also send the corresponding `TuiCommand` via the stored `tui_command_tx: Option<mpsc::Sender<TuiCommand>>`. |
| `crates/roko-cli/src/commands/plan.rs` | When creating the TUI App for plan run, create the `mpsc::channel(32)` and pass the sender to App, the receiver to the runner. |

### P1.2: Wire log search render in logs_view.rs

**What**: `logs_view.rs` never references `tui_state.log_search`. The search bar appears but
matching lines are not highlighted and filter mode does not exclude non-matches.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/views/logs_view.rs:74-209` | In `render_with_entries()`, after level filtering, check `tui_state.log_search.active` and `log_search.compiled`. In `Filter` mode, exclude entries that don't match. In `Highlight` mode, wrap matching spans in `Style::new().bg(theme.DREAM).fg(theme.TEXT_STRONG)`. Use `log_search.match_indices` to set `log_scroll` to the current match position when navigating with `n`/`N`. |

### P1.3: Wire plan tree filter render

**What**: `plan_tree.rs` reads old `state.filter`/`filter_active` fields instead of the new
`PlanTreeFilter` struct with `status:` prefix parsing.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/widgets/plan_tree.rs:794-798` | Replace reads of `state.filter_active` and `state.filter` with reads of `tui_state.plan_tree_filter`. When `plan_tree_filter.active`, apply `text_filter` against plan slug/name and `status_filter` against `PlanPhase`. |
| `crates/roko-cli/src/tui/widgets/plan_tree.rs:1004` | Update the test `plan_tree_filters_visible_plans_and_clamps_selection` to use the new `PlanTreeFilter` fields. |

### P1.4: Make F3 role tabs switch agent output

**What**: Clicking role tabs on F3 highlights the tab but does not switch the displayed
agent output.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/views/agents_view.rs` | In the role tab click handler, set `tui_state.selected_agent_idx` to the first agent matching the clicked role. Filter the agent output panel to show only agents of the selected role. |

### P1.5: Display critical_path_eta_minutes

**What**: The field exists in `TuiState` and the header bar has a display path, but the
field is never written.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/state.rs` | In `update_from_dashboard_snapshot()`, after updating plan entries, call `remaining_eta_minutes()` from `task_dag.rs:782` using the current plan tasks and completed set. Write the result to `self.critical_path_eta_minutes`. |
| `crates/roko-cli/src/tui/views/dashboard_view.rs` | In the phase_compact detail line, add `ETA ~{minutes}m` when `tui_state.critical_path_eta_minutes.is_some()`. |

### P1.6: Register F7 sub-tab 5

**What**: `render_three_panel_inspect` exists at `context_view.rs:1071` but sub-tab 5 is
never defined in the sub-tab list, making it unreachable.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/views/mod.rs` | Add a 6th `SubView` entry for the Inspect tab (e.g., `ThreePanel`) so sub-tab index 5 routes to `render_three_panel_inspect`. |

**Verification**:

1. Run `cargo test --workspace`.
2. Launch TUI during a plan run. Press `p` -- verify the runner actually pauses (no new tasks dispatched). Press `p` again -- verify it resumes.
3. Press `s` on a failed plan -- verify the task is retried (not just a toast).
4. On F5 Logs, press `/`, type a pattern -- verify matching lines are highlighted yellow.
5. On F2 Plans, press `/`, type `status:failed` -- verify only failed plans appear.
6. Verify CP-ETA appears in the header bar during a multi-task plan run.
7. On F7, verify sub-tab 5 is reachable via Alt+5.

**Dependencies**: P1.1 depends on runner event loop structure (no Phase 1 dependency).

---

## Phase 3 (P2): Show gate execution

**Goal**: During the 30-120s that gates run, show live cargo/test output instead of nothing.

**Depends on**: Independent of Phases 1-2, but most impactful after Phase 1 fixes data flow.

### P2.1: Forward gate output through DashboardEvent

**What**: `DashboardEvent::GateResult` carries only plan_id/task_id/gate/passed. The raw
cargo/test output in `GateCompletion.output` is never sent to the TUI.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-core/src/dashboard_snapshot.rs:78-80` | Add `DashboardEvent::GateOutputLine { plan_id: String, task_id: String, gate: String, line: String }` variant. Optionally add `output_text: Option<String>` to the existing `GateResult` variant. |
| `crates/roko-core/src/dashboard_snapshot.rs` | In `apply()`, handle `GateOutputLine` by appending to a new `gate_output_lines: VecDeque<String>` field (bounded to 500 lines). |
| `crates/roko-cli/src/runner/tui_bridge.rs` | Add `gate_output_line(&self, plan_id, task_id, gate, line)` method that publishes `GateOutputLine`. |
| `crates/roko-cli/src/runner/gate_dispatch.rs` | During gate execution, as stdout/stderr lines arrive from the cargo subprocess, call `tui.gate_output_line()` for each line. This requires changing from `output()` collection to streaming line-by-line. |
| `crates/roko-cli/src/runner/event_loop.rs` | After gate completion, also forward the full `GateCompletion.output` via a `GateResult` with `output_text`. |

### P2.2: Build GateOutputWidget

**What**: Create a dedicated gate output widget (similar to mori's `command_output.rs`,
218 LOC) that renders streaming cargo/test output with pass/fail line coloring.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/widgets/gate_output.rs` | **New file** (~250 LOC). `render_gate_output(frame, area, tui_state, theme)`. Reads `tui_state.gate_output_lines`. Color-codes lines: green for "test result: ok" / "Compiling", red for "error[E" / "FAILED", dim for info. Shows animated spinner in title when gate is running. Scrollbar for long output. Uses existing `FocusZone::CommandOutput` for focus navigation. |
| `crates/roko-cli/src/tui/widgets/mod.rs` | Add `pub mod gate_output;` |
| `crates/roko-cli/src/tui/state.rs` | Add `pub gate_output_lines: VecDeque<String>` (bounded to 500), `pub gate_running: Option<String>` (current gate name). Populate from `DashboardSnapshot.gate_output_lines` in `update_from_dashboard_snapshot()`. |
| `crates/roko-cli/src/tui/views/dashboard_view.rs` | Wire `gate_output::render_gate_output()` into the Output sub-tab (sub_tab 1) or as a dedicated sub-tab. When `gate_running.is_some()`, show the gate output widget; when idle, show agent output. |

### P2.3: Show live gate rung indicator

**What**: `GateRungStarted` events are published but only land in the event log. No
"rung X running for Ns" display exists.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/state.rs` | Add `pub current_gate_rung: Option<(String, String, Instant)>` (gate_name, rung_name, start_time) to `TuiState`. Set on `GateRungStarted`, clear on `GateResult`. |
| `crates/roko-cli/src/tui/views/dashboard_view.rs` | In the phase_compact widget or task_progress, when `current_gate_rung.is_some()`, render: `"[gate: {rung_name} {elapsed}s]"` with a spinner character. |
| `crates/roko-cli/src/tui/widgets/task_progress.rs` | Add a gate-in-progress line showing the current rung name and elapsed time. |

**Verification**:

1. Run `cargo test --workspace`.
2. Run a plan with gate execution. Verify:
   - During cargo check/test, live output lines appear in the TUI.
   - Lines containing "error" are colored red; lines containing "ok" are colored green.
   - A spinner shows in the title bar during gate execution.
   - The task progress area shows "gate: compile 12s" or similar.
3. After gate completes, verify the output is replaced by the next phase's content.

**Dependencies**: None (standalone).

---

## Phase 4 (P3): Fix performance

**Goal**: Eliminate per-frame disk I/O.

**Depends on**: Independent. Can be done in any order.

### P3.1: Cache MCP config

**What**: The MCP sub-tab reads `roko.toml` + MCP config files on every render frame
(`dashboard_view.rs:737`).

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/state.rs` | Add `pub cached_mcp_config: Option<McpConfigCache>` with a `last_refresh: Instant` field. |
| `crates/roko-cli/src/tui/views/dashboard_view.rs:737` | Instead of reading config files, read from `tui_state.cached_mcp_config`. |
| `crates/roko-cli/src/tui/app.rs` | In `drain_background_channels()` or on fs-watcher signal, refresh `cached_mcp_config` (at most once per 5 seconds). |

### P3.2: Cache config TOML parse

**What**: F6 Config re-reads and re-parses `roko.toml` on every frame
(`config_view.rs:65`, `config_meta.rs:637`).

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/state.rs` | Add `pub cached_config: Option<CachedConfig>` with parsed TOML and `last_refresh: Instant`. |
| `crates/roko-cli/src/tui/views/config_view.rs:65` | Read from `tui_state.cached_config` instead of calling `RokoConfig::load()`. |
| `crates/roko-cli/src/tui/app.rs` | Refresh `cached_config` on Ctrl-S save, on fs-watcher signal, or every 5 seconds. |

### P3.3: Move F7 inspect file reads to background

**What**: F7 Inspect reads `mcp-stats.json`, `playbook.json`, etc. on every frame.

**Changes**:

| File | Change |
|------|--------|
| `crates/roko-cli/src/tui/views/context_view.rs` | Replace inline file reads with reads from `TuiModel.inspect_data` (which already exists and refreshes on 5-second cadence via `refresh_inspect_data()`). Ensure all file reads go through `InspectData`, not directly. |

**Verification**:

1. Run `cargo test --workspace`.
2. Run `roko dashboard` and monitor with `fs_usage -w -f pathname roko` or `strace`.
3. Verify: no `roko.toml` reads after initial load (except on Ctrl-S or watcher trigger).
4. Verify: no MCP config reads except on background refresh.
5. Verify: F6 Config still shows correct data and responds to Ctrl-S saves.

**Dependencies**: None.

---

## Phase 5 (P4-P7): Polish and enrichment

**Goal**: Visual density improvements, plan detail enrichment, keyboard model fixes,
and sub-tab corrections.

**Depends on**: Phases 1-4 should be done first. These are polish items.

### P4: Visual density and polish

| Item | File | Change |
|------|------|--------|
| **P4.1** Shrink bottom ribbon | `crates/roko-cli/src/tui/views/dashboard_view.rs:1919` | Change the bottom ribbon layout from `Constraint::Length(6)` to `Constraint::Length(4)`. Combine wave progress and token sparkline into a tighter 2-row layout. |
| **P4.2** Contextual empty states | `crates/roko-cli/src/tui/views/dashboard_view.rs`, `plans_view.rs`, `agents_view.rs`, `logs_view.rs` | Replace 8 generic "waiting for data..." / "no data yet" strings with context-specific messages: e.g., "No agents dispatched yet -- waiting for plan task to start", "Run `roko plan run` to see live gate data". |
| **P4.3** NET/DSK metrics | `crates/roko-cli/src/tui/widgets/sys_metrics.rs` | In `collect_sys_metrics_bg()`, add network bytes/sec and disk free space sampling using `sysinfo` crate (already a dependency). Render in the system metrics panel. |
| **P4.4** Effects default | `crates/roko-cli/src/tui/effects_config.rs:86-88` | Change `EffectsConfig::default()` to `Self::from_preset(EffectsPreset::Minimal)`. |
| **P4.5** PAUSED badge | `crates/roko-cli/src/tui/widgets/status_bar.rs:67` | Style the "PAUSED" text with `Style::new().fg(theme.VOID).bg(theme.WARNING)` for inverted badge visibility. |
| **P4.6** Warning bar | `crates/roko-cli/src/tui/widgets/header_bar.rs` | Add a 1-line warning bar below the header that appears when `tui_state.warnings.len() > 0`. Warnings: disk low, provider unhealthy, stale snapshot. Dismiss with `n`. |
| **P4.7** Header extras | `crates/roko-cli/src/tui/widgets/header_bar.rs` | Add MCP connection count, NET rate, DSK free, and FPS to the header status area. Data from `tui_state.sys_metrics` and `cached_mcp_config`. |

### P5: Enrich plan detail

| Item | File | Change |
|------|------|--------|
| **P5.1** TaskEntry depends_on | `crates/roko-cli/src/tui/state.rs` (TaskEntry struct) | Add `pub depends_on: Vec<String>` to `TaskEntry`. Populate from the loaded plan's `task.depends_on` field during snapshot conversion. Display in plan detail modal as indented dependency list. |
| **P5.2** TaskEntry acceptance | `crates/roko-cli/src/tui/state.rs` (TaskEntry struct) | Add `pub acceptance: Option<String>` and `pub verify_commands: Vec<String>` to `TaskEntry`. Populate from task TOML. Display in plan detail modal under the task description. |
| **P5.3** Files-modified stats | `crates/roko-cli/src/tui/modals/plan_detail.rs` | Add a "Changes" section to the plan detail modal showing files modified count, insertions, deletions (from `git diff --stat` for the plan's worktree branch). Data from `tui_state.git_view_data`. |
| **P5.4** Branch/worktree info | `crates/roko-cli/src/tui/modals/plan_detail.rs` | Add branch name, worktree path, and last commit hash to the plan detail modal header. Data from plan execution state. |
| **P5.5** Per-plan elapsed timer | `crates/roko-cli/src/tui/state.rs` | Add `pub started_at: Option<Instant>` to `PlanEntry`. Set from `PlanStarted` event. In the plan tree render, display elapsed time next to each active plan. |

### P6: Keyboard model fixes

| Item | File | Change |
|------|------|--------|
| **P6.1** Number key shadowing | `crates/roko-cli/src/tui/input.rs:640-656` | Guard the global `1`-`9` tab-switch handler: only fire when `active_tab != Tab::Agents`. Or remove the number-key global binding entirely and rely on F-keys only. |
| **P6.2** Reassign `v` key | `crates/roko-cli/src/tui/input.rs:692` | Change `v` from `CycleEffectsPreset` to `ReverifyPlan` (matching mori). Move effects cycling to `Ctrl-E` only (which already exists for `ToggleScreenPostFx`). |
| **P6.3** Tab focus on 7 tabs | `crates/roko-cli/src/tui/input.rs` (FocusZone::next/prev) | For each of Git, Logs, Config, Inspect, Marketplace, Atelier, Learning: define 2-3 focus zones (e.g., Git: BranchList / CommitGraph / BranchInfo) and wire `next()`/`prev()` to cycle through them. |
| **P6.4** Help overlay update | `crates/roko-cli/src/tui/modals/help.rs` | Update the help text: add F8-F10 tabs, remove incorrect "1-7 agent tab" claim, add `/` search, `n/N` nav, `f` filter mode, `z/d/m/M` plan actions, `Ctrl-G` git reconcile, log level `1-4`, `L` detail tab. Add scroll support for small terminals. |
| **P6.5** Diff/Procs scroll fix | `crates/roko-cli/src/tui/views/dashboard_view.rs` | Verify that the Diff sub-tab and Procs sub-tab each use their own scroll offset field (not a shared one). If shared, split into `diff_scroll` and `procs_scroll` in TuiState. |

### P7: Sub-tab specific fixes

| Item | File | Change |
|------|------|--------|
| **P7.1** Git diff refresh | `crates/roko-cli/src/tui/app.rs` (apply_git_bg_data) | In `apply_git_bg_data()`, also run `git diff` and update `tui_state.git_diff`. This makes the F1 Diff sub-tab show current diff, not initial-load-only. |
| **P7.2** Log vs Signals sub-tabs | `crates/roko-cli/src/tui/views/logs_view.rs` | For sub-tab 1 ("Signals"), filter `unified_log_entries()` to only show entries with source `signal:` or `episode:`. Sub-tab 0 ("Log") keeps showing all entries. |
| **P7.3** Procs scroll state | `crates/roko-cli/src/tui/views/dashboard_view.rs` | Verify the Procs sub-tab uses its own scroll offset. If it shares with another sub-tab, give it a dedicated `procs_scroll: u16` in `TuiState`. |
| **P7.4** Agent attempt count | `crates/roko-cli/src/tui/views/agents_view.rs` | In the agent output panel title, append `(attempt N, turn M)` from `agent.attempt_count` and `agent.turn_count`. Both fields exist in the agent state. |

**Verification**:

1. Run `cargo test --workspace`.
2. Visual checks per sub-item:
   - P4.4: Launch TUI, verify effects are on by default (subtle bloom/rain).
   - P6.1: On F3 Agents, press `3` -- verify it selects agent tab 3, not switches to F3.
   - P6.2: Press `v` -- verify it triggers verify, not effects cycling.
   - P6.4: Press `?` -- verify help shows F1-F10, correct bindings, and scrolls.
   - P7.1: Make a code change, verify F1 Diff sub-tab updates within ~1 second.
   - P7.2: Switch to F5 Logs sub-tab 1 ("Signals") -- verify only signal entries appear.
3. Run `cargo clippy --workspace --no-deps -- -D warnings`.

**Dependencies**: Phase 5 benefits from Phase 1's data fixes (plan detail fields need
populated TuiState). Phase 5 items P5.3 and P5.4 depend on git data being in TuiState
(which already works via the git watcher).

---

## Cross-phase notes

### Test strategy

Each phase should be validated with:
1. `cargo test --workspace` -- no regressions.
2. `cargo clippy --workspace --no-deps -- -D warnings` -- clean.
3. `cargo +nightly fmt --all` -- formatted.
4. Manual TUI verification during a real `plan run`.

### Risk areas

- **P0.1/P0.5**: The `DashboardData` / `TuiState` unification touches many render functions.
  Test each affected widget individually.
- **P1.1**: The TUI command channel introduces a new concurrency primitive. Ensure the
  `mpsc::Receiver` is properly drained and doesn't block the event loop.
- **P2.1**: Streaming gate output line-by-line changes the gate dispatch architecture from
  collect-then-send to stream-as-produced. Verify that gate retry logic still has access
  to the full output.
- **P6.1**: Changing global keybindings affects all tabs. Verify no other tab relies on
  the `1`-`9` global tab switch for correct behavior.

### Files most frequently modified

These files appear across multiple phases and should be read carefully before editing:

| File | Phases | Lines | Role |
|------|--------|-------|------|
| `crates/roko-cli/src/tui/state.rs` | 1,2,3,4,5 | ~6000 | Central TUI state, all data fields |
| `crates/roko-cli/src/tui/app.rs` | 1,2,3,4 | ~4500 | Main app loop, action dispatch, drawing |
| `crates/roko-cli/src/tui/views/dashboard_view.rs` | 1,3,5 | ~2400 | F1 dashboard rendering, sub-tabs |
| `crates/roko-cli/src/tui/input.rs` | 2,5 | ~1000 | Key dispatch, per-tab handlers |
| `crates/roko-core/src/dashboard_snapshot.rs` | 1,3 | ~1700 | Event application, snapshot state |
| `crates/roko-cli/src/runner/event_loop.rs` | 1,2,3 | ~17000 | Runner main loop, event handling |
| `crates/roko-cli/src/runner/tui_bridge.rs` | 1,3 | ~400 | Runner-to-TUI event bridge |
