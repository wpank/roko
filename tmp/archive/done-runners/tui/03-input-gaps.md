# TUI Input & Key Binding Gaps

Exhaustive audit of every `TuiAction` variant, every key handler, every dispatch arm, mouse support, scroll behavior, and focus routing.

**Audit date**: 2026-04-14
**Files audited**: `input.rs` (775L), `app.rs` dispatch_action (495-1117), `scroll.rs` (95L), `hit_test.rs` (205L), `event.rs` (76L)

---

## CRITICAL — Entire subsystems broken

| ID | Gap | Where | Impact |
|----|-----|-------|--------|
| K1 | **Mouse events dropped by EventHandler** — `Event` enum has only `Key`/`Resize`/`Tick` variants, no `Mouse`. `EventHandler::next()` at `event.rs:67` hits `_ => continue` for mouse events, silently discarding them. `EnableMouseCapture` IS active but `handle_mouse()` is never called. | `event.rs:10-17,67`. `app.rs:332-351` uses `EventHandler`. Only the unused async `run()` at `app.rs:143-164` handles mouse. | All mouse support (click, scroll wheel) is dead code. 100% broken. |
| K2 | **ScrollAccel never instantiated or used** — Complete acceleration system (1x→2x→4x→8x within 300ms, direction-change reset) exists in `scroll.rs`, exported via `mod.rs`, but `App` has no `ScrollAccel` field and no key handler calls `push()`. | `scroll.rs` entire file. No import in `app.rs`. | All scrolling is fixed at 1 line per keypress. Scroll acceleration is entirely dead code. |

## HIGH — Wrong behavior / key does wrong thing

| ID | Gap | Where | Impact |
|----|-----|-------|--------|
| K3 | Logs tab `End`/`G` maps to `ScrollAgentEnd` — scrolls agent pane, not log pane | `input.rs:598` | Pressing End on Logs tab resets agent output scroll instead of going to log bottom |
| K4 | `plan_scroll_offset` modified by `scroll_focused()` but `plan_tree.rs` reads `plan_scroll` — never synced | `app.rs:1124` writes `plan_scroll_offset`; `plan_tree.rs:148` reads `plan_scroll` | Plan tree scroll position never changes despite key presses |
| K5 | `output_scroll` field read by widgets but never modified by any key handler | `state.rs:388` declared; `agent_output.rs:66`, `command_output.rs:80` read it | Key handlers modify `agent_scroll`; widgets read `output_scroll` — different fields |
| K6 | `show_wave_overview`, `show_help`, `show_plan_detail` have no key intercept in `handle_key` | `input.rs:292-302` only checks task_picker, task_detail, queue_overview | When these modals are open, keystrokes fall through to tab handlers. `q` quits instead of closing modal. |
| K7 | Two parallel modal systems (`TuiAction` + legacy `OverlayState`) can desync | `app.rs:476-481` checks legacy overlay first; `:569-573` sets both systems for `ShowPlanDetail` | `show_plan_detail` and `overlay: Some(Detail(...))` track same modal independently |
| K8 | `DrillIn`/`DrillOut` on Git/Inspect tabs operates on plans, not tab-specific data | `app.rs:794-811` always toggles `plans[selected].expanded` regardless of active tab | `Left`/`Right`/`h`/`l` on Git tab expands/collapses a plan entry, not a git branch |
| K9 | `PageUp`/`PageDown`/`Home` scroll by 1 line instead of page-sized amount | `app.rs:533-534` `scroll_focused()` always called with delta ±1 | PageUp is identical to Up arrow |
| K10 | No upper bound clamping on any scroll field — can scroll past content | All scroll handlers in `dispatch_action` use saturating_add but never check content length | User can scroll into empty space below content |
| K11 | Agents `PageUp`/`PageDown` always target agent pane, not focus-aware | `input.rs:555-556` hardcode `ScrollAgentUp`/`ScrollAgentDown` | PageUp on Agents tab ignores focus zone |
| K12 | Dashboard `End` always targets agent pane regardless of focus | `input.rs:472` hardcodes `ScrollAgentEnd` | End key ignores focus zone on Dashboard |
| K13 | Agents `Home` scrolls agent by 1 line instead of going to top (0) | `input.rs:558` → `ScrollAgentUp` → `app.rs:541-544` decrements by 1 | Home key works like Up arrow |
| K14 | Focus cycling visits 5 zones on ALL tabs, even single-pane tabs | `input.rs:54-75` Tab/BackTab cycles PlanTree→TaskProgress→AgentOutput→CommandOutput→RightPanel | On Logs/Config/Inspect tabs, Tab cycles through invisible zones |
| K15 | `SwitchTab` resets focus to `PlanTree` even on tabs with no PlanTree panel | `app.rs:509` always sets `focus = PlanTree` | Switching to Logs tab sets focus to PlanTree zone |
| K16 | `WaveNext`/`WavePrev` uses `plans.len()` not `execution_waves.len()` as max | `app.rs:812-823` | Wave navigation wraps at wrong boundary |
| K17 | `MouseClick` uses hardcoded 80x24 terminal size instead of actual dimensions | `app.rs:1093-1094` `Rect::new(0, 0, 80, 24)` | If mouse worked, click hit zones would be wrong on larger terminals |
| K18 | Two separate `FocusZone` enums with incompatible variants | `hit_test.rs:6-22` has `RightContent`/`HeaderTab(usize)`/`DetailTab(usize)`; `input.rs:37-49` has `RightPanel` | `HeaderTab` click maps to `PlanTree` focus (wrong); `DetailTab` click maps to `RightPanel` (wrong) |

## MEDIUM — Dead code, stubs, actions that do nothing useful

| ID | Gap | Where | Impact |
|----|-----|-------|--------|
| K19 | `ApproveCommand`/`ApproveAll`/`RejectCommand` — all dead code | `app.rs:657-665` only clear `pending_approval` which is never set to `Some(...)` | Approve/reject keys (`y`/`a`/`A`/`x`) do nothing |
| K20 | `TogglePause` — cosmetic only, no orchestrator integration | `app.rs:636-643` toggles `pipeline_run_state` string between "paused"/"running" | `p` key changes a display label but doesn't actually pause anything |
| K21 | `CollapseExpand` — unreachable duplicate of `ExpandCollapse` | `input.rs:242`, `app.rs:627-635` identical implementation | No key binding maps to this variant |
| K22 | `ConfigStartEdit` — unreachable variant | `input.rs:214`, `app.rs:1024-1044` | No key binding emits this; `ConfigToggle` subsumes it |
| K23 | `filter_active`/`filter_text` never consumed by rendering or data filtering | `app.rs:728-731` sets them; no widget or view reads them for filtering | `/` filter mode accepts text but nothing is actually filtered |
| K24 | Task picker always opens with empty task list | `app.rs:606-613` creates `ModalState::TaskPicker { tasks: Vec::new() }` | `Ctrl-T` opens picker but it's always empty |
| K25 | Task picker `SelectPlanUp`/`Down` navigates plan list, not task list | `input.rs:355-356` | Up/Down in task picker scroll plans instead of tasks |
| K26 | Plans tab: `z`, `c`, `V` all map to `ReverifyPlan` (triplicate) | `input.rs:532,535,537` | Three keys do identical thing; missing `DiagnosePlan` and `Reverify` as separate actions |
| K27 | Task detail `Tab` always sends `SwitchDetailTab(0)` instead of cycling | `input.rs:366` | Tab key in task detail modal doesn't cycle sub-tabs |
| K28 | `SwitchAgentTab` accepts out-of-bounds indices (1-7) with no clamping | `input.rs:562-568`, `app.rs:644-653` | Pressing `7` with 2 agents sets `selected_agent_tab = 6` silently |
| K29 | `agent_pane_group` toggled by `g` on Agents tab but never consumed by rendering | `app.rs:791-793` increments mod 2 | Key works but has no visible effect |
| K30 | `DismissNotification` only works on Dashboard tab | `input.rs:492` only in `handle_dashboard_key` | `n` key for notifications not available on other tabs |
| K31 | All confirm-yes handlers write signals to `.roko/signals.jsonl` but orchestrator doesn't read this file for commands | `app.rs:748-779` | Confirming destructive actions (restart, reset, force-advance) writes a log entry but doesn't trigger any action |
| K32 | Wave/Queue overview modals always opened with empty data | `app.rs:583-605` constructs with `Vec::new()` | `w` and `u`/`F8` open modals but they're always empty |

## LOW — Missing key bindings for existing actions

| ID | Gap | Mori key | ConfirmAction variant | Roko binding |
|----|-----|----------|----------------------|--------------|
| K33 | No `Ctrl-G` for git reconcile | `Ctrl-G` | `ConfirmAction::GitReconcile` | Missing |
| K34 | No binding for merge plan | `m` | `ConfirmAction::MergePlan` | Missing |
| K35 | No binding for merge batch to main | `M` | `ConfirmAction::MergeBatchToMain` | Missing |
| K36 | No binding for merge all done | — | `ConfirmAction::MergeAllDone` | Missing |
| K37 | No binding for ingest task | — | `ConfirmAction::IngestTask` | Missing |
| K38 | No binding for agent pool modal | — | `show_agent_pool_modal` exists in state | Missing |
| K39 | No `Alt+1-7` for agent role tabs | `Alt+1-7` | Would need `SwitchAgentTab` variant | Missing |
| K40 | No `Space`/`End` for resume auto-scroll in agent output | `Space`/`End` | `ScrollAgentEnd` exists but only bound on some tabs | Inconsistent |
| K41 | No `Ctrl-R` restart all plans (only Refresh is on Ctrl-R) | `Ctrl-R` | `ConfirmAction::RestartAllPlans` | `Ctrl-R` is `Refresh` instead |

## Scroll fields — key coverage matrix

| Field | Defined | Has key handler? | Handler | Notes |
|-------|---------|-----------------|---------|-------|
| `agent_scroll: Option<usize>` | `state.rs:386` | YES | `ScrollAgentUp/Down/End`, `scroll_focused` when AgentOutput | None=auto-tail, Some(n)=pinned |
| `output_scroll: usize` | `state.rs:388` | **NO** | Never modified | Read by widgets — DESYNC with agent_scroll |
| `diff_scroll: usize` | `state.rs:390` | YES | `ScrollDiffUp/Down`, `scroll_focused` when RightPanel | Shared across tabs |
| `task_scroll: usize` | `state.rs:392` | YES | `scroll_focused` when TaskProgress | Only via ScrollFocusedUp/Down |
| `command_output_scroll: usize` | `state.rs:394` | YES | `scroll_focused` when CommandOutput | |
| `plan_detail_scroll: usize` | `state.rs:396` | YES | `ScrollDetailUp/Down` | |
| `plan_summary_scroll: usize` | `state.rs:398` | **NO** | Never modified | Plan summary cannot be scrolled |
| `plan_scroll_offset: usize` | `state.rs:400` | YES | `scroll_focused` when PlanTree | Keys write THIS field |
| `plan_scroll: usize` | `state.rs:402` | **NO** | Never modified | plan_tree.rs reads THIS field — DESYNC |
| `log_scroll: usize` | `state.rs:404` | YES | `ScrollLogUp/Down` | |
| `task_detail_scroll: usize` | `state.rs:406` | **NO** | Never modified | Task detail cannot be scrolled |
| `config_scroll_offset: usize` | `state.rs:510` | **NO** | Never modified | Config view cannot be scrolled |

## Summary

| Severity | Count | Previous doc |
|----------|-------|--------------|
| CRITICAL | 2 | 0 |
| HIGH | 16 | 4 |
| MEDIUM | 14 | 4 |
| LOW | 9 | 33 |
| **Total** | **41** | **41** |

The previous doc had 41 items but miscategorized severity — most "missing keys" were listed as individual items when the root causes are systemic (e.g., mouse entirely broken accounts for K35-K37 in old doc). This audit found 2 critical system-level bugs the old doc completely missed.
