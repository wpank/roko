# TUI Modal & Overlay Gaps (Exhaustive Audit)

Updated 2026-04-14 by exhaustive code audit. Corrections to previous version noted inline.

## Summary

- 14 modal files exist in `crates/roko-cli/src/tui/modals/`
- 9 ModalState variants in enum (Quit, Approval, Confirm, Inject, WaveOverview, QueueOverview, AgentPool, TaskPicker, BatchReview)
- 2 modal files have NO ModalState variant (plan_detail.rs, task_detail.rs) -- **orphaned render functions**
- 1 modal file uses wrong theme system and is never called (help.rs)
- Dual visibility tracking (ModalState enum + show_* booleans) creates desync risk
- All "data" modals opened with empty Vec data -- never populated from runtime

**CORRECTION**: Previous doc said "3 present, 10 missing". Actually 9 ModalState variants exist with render dispatch + 2 boolean-controlled overlays + 1 dead file = 12 files with render code. The issue is not "missing" but "wired but broken."

## Section 1: Modals with broken keyboard handling

| ID | Modal | File:Line | Severity | Issue |
|----|-------|-----------|----------|-------|
| M1 | **Approval** | `modals/mod.rs:48`, `app.rs:432` | HIGH | ModalState::Approval renders [y]approve/[n]reject but `input.rs` has NO intercept for Approval modal. Keys fall through to global handler. User cannot approve or reject. |
| M2 | **BatchReview** | `modals/batch_review.rs:27` | HIGH | Renders [a]accept/[r]reject/[s]skip/[Esc] buttons but `input.rs` has NO intercept for BatchReview modal state. All rendered hotkeys are dead. |
| M3 | **WaveOverview** | `modals/mod.rs:61` | MEDIUM | `input.rs` has NO intercept for WaveOverview. Opened via `show_wave_overview` boolean but modal renders as ModalState with scroll_offset that cannot be changed. |
| M4 | **AgentPool** | `modals/mod.rs:74` | MEDIUM | `input.rs` has NO intercept for AgentPool modal. Scroll offset in ModalState cannot be changed by user. |

## Section 2: Modals with no render path (orphaned)

| ID | Modal | File:Line | Severity | Issue |
|----|-------|-----------|----------|-------|
| M5 | **Plan Detail** | `modals/plan_detail.rs:1-80` | HIGH | `render_plan_detail_modal()` takes `PlanState`, `TaskState[]`, scroll, theme. But there is NO `ModalState::PlanDetail` variant. `app.rs` never calls this function. `show_plan_detail` boolean is toggled (`app.rs:570`) but nothing renders. |
| M6 | **Task Detail** | `modals/task_detail.rs:1-60` | HIGH | `render_task_detail_modal()` takes `TaskState`, `GateVerdict[]`, output, scroll, theme. NO `ModalState::TaskDetail` variant. `app.rs` never calls this. `show_task_detail` boolean is toggled (`app.rs:578`) AND input.rs intercepts keys for it (`input.rs:296-297`) -- so keys are consumed but nothing is visible. |
| M7 | **Help** | `modals/help.rs:1-60` | LOW | `render_help_modal()` exists but uses `crate::tui::mori_theme::MoriTheme` -- a module path that does NOT exist in `tui/mod.rs`. Cannot compile through this import path. Not called from `app.rs` (which uses its own `render_help_overlay` with `help_lines()` at `app.rs:439`). Completely dead code. |

## Section 3: Modals opened with empty data

| ID | Modal | File:Line | Severity | Issue |
|----|-------|-----------|----------|-------|
| M8 | **WaveOverview** | `app.rs:585-587` | HIGH | `ShowWaveOverview` action creates `ModalState::WaveOverview { waves: Vec::new(), scroll_offset: 0 }`. Waves never populated from `tui_state.execution_waves`. |
| M9 | **QueueOverview** | `app.rs:596-598` | HIGH | `ShowQueueOverview` action creates `ModalState::QueueOverview { milestones: Vec::new(), ... }`. Milestones never populated. |
| M10 | **TaskPicker** | `app.rs:607-609` | HIGH | `OpenTaskPicker` action creates `ModalState::TaskPicker { tasks: Vec::new(), ... }`. Tasks never populated. |
| M11 | **AgentPool** | `app.rs` (dispatch_action) | MEDIUM | `ModalState::AgentPool { agents: Vec::new(), ... }`. Agents never populated from `tui_state.agents`. |
| M12 | **BatchReview** | `app.rs` (dispatch_action) | MEDIUM | `ModalState::BatchReview { results: Vec::new(), ... }`. Results never populated. |

## Section 4: Dual modal visibility tracking desync

| ID | Issue | File:Line | Severity |
|----|-------|-----------|----------|
| M13 | **Two tracking systems**: `active_modal: Option<ModalState>` in App struct AND `show_*` booleans in TuiState. Some modals use ModalState (Quit, Approval, Confirm, Inject, WaveOverview, QueueOverview, AgentPool, TaskPicker, BatchReview), others use booleans (plan_detail, task_detail, help, wave_overview, queue_overview, agent_pool_modal, task_picker). Several modals are tracked by BOTH systems simultaneously. | `app.rs:64-84`, `state.rs:409-422` | HIGH |
| M14 | **show_wave_overview toggles boolean AND sets ModalState** at `app.rs:584-587`. If ModalState is dismissed by Quit action (`app.rs:501: active_modal = None`) the boolean may remain true (and vice versa). | `app.rs:584-587`, `app.rs:499-502` | HIGH |
| M15 | **ModalVisibility struct** (`input.rs:338-345`) reads show_* booleans for key routing, but `render_modals()` reads `active_modal` for rendering. Key handler and renderer disagree on what's visible. | `input.rs:338-345`, `app.rs:444-450` | HIGH |
| M16 | **dismiss_all_modals()** clears show_* booleans (`state.rs`) but Quit handler also clears `active_modal` (`app.rs:501`). No single function clears both. | `app.rs:499-502`, `state.rs` | MEDIUM |

## Section 5: Modal infrastructure gaps

| ID | Gap | File:Line | Severity |
|----|-----|-----------|----------|
| M17 | **ConfirmAction enum has 14 variants** (`input.rs`) but only a few are wired to actual operations in `dispatch_action()`. Most variants (DiagnosePlan, RepairPlanPreserve, RepairPlanClean, SoftRetryPlan, GitReconcile, MergeBatchToMain, MergePlan, MergeAllDone) have no handler. | `input.rs:55-80`, `app.rs:748-779` | MEDIUM |
| M18 | **Notification rendering works** but `expire_notifications()` is never called from the event loop -- notifications accumulate forever if more than the TTL check catches. (CORRECTION: previous doc said "notifications vec never populated" -- WRONG. `SubmitInject` at `app.rs:670-700` and `ConfirmYes` push notifications.) | `app.rs:670-700`, `modals/notification.rs` | MEDIUM |
| M19 | **No modal z-ordering**: if both `active_modal` and `show_help` are true, both render on top of each other (dim overlay applied once at `app.rs:432`, help at 439, modal at 444). | `app.rs:432-450` | LOW |
