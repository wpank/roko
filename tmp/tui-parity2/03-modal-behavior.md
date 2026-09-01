# 03 -- Modal Behavior and Dismiss Logic Audit

## Summary

The modal key-dispatch architecture has a **critical priority inversion bug**: the
`ModalState`-based intercept in `input::handle_key` runs before the `InputMode`-based
dispatch, making the `Quit`, `Confirm`, `Inject`, and `BatchReview` modals completely
unresponsive to their intended keybindings (y/n/Esc/Enter/text input). The only way
to exit a stuck Quit or Confirm modal is Ctrl+C (hard quit). This is a regression from
mori's architecture, which dispatches on `InputMode` first and checks modal visibility
only inside `InputMode::Normal`.

---

## Architecture Overview

### Key dispatch flow (roko)

File: `crates/roko-cli/src/tui/input.rs`, `handle_key()` (line 481)

```
1. Ctrl+C                          -> QuitConfirmed (always, line 488)
2. Modal intercept (active_modal)  -> per-modal handler or `_ => None` (line 493)
3. InputMode::Confirm              -> handle_confirm_key (NEVER REACHED, line 508)
4. InputMode::ConfigEdit           -> handle_config_edit_key (line 513)
5. InputMode::Inject               -> handle_inject_key (NEVER REACHED when modal set, line 516)
6. InputMode::Filter               -> handle_filter_key (line 519)
7. InputMode::LogSearch            -> handle_log_search_key (line 522)
8. InputMode::PlanFilter           -> handle_plan_filter_key (line 525)
9. Global keys                     -> handle_global_key (line 530)
10. Per-tab dispatch               -> handle_{tab}_key (line 535)
```

Steps 3-8 are unreachable when `active_modal` is `Some(_)` because step 2 returns
early.

### Key dispatch flow (mori -- reference)

File: `apps/mori/src/tui/input.rs`, `handle_key()` (line 138)

```
1. match input_mode {
     Normal  -> modal visibility checks (task_picker, task_detail, queue_overview)
                then handle_normal_key (global + per-tab)
     Inject  -> handle_inject_key
     Filter  -> handle_filter_key
     Confirm -> handle_confirm_key      <-- ALWAYS reachable
   }
```

In mori, `InputMode` is the top-level discriminant. Modal visibility is checked
**inside** `InputMode::Normal` only. The `Confirm` and `Inject` modes are never
blocked by modal visibility.

---

## Bug: Priority Inversion Between ModalState and InputMode

### Root cause

In `input.rs` lines 492-505:

```rust
// Modal intercepts (highest priority first)
if let Some(modal) = modals.active_modal {
    return match modal {
        ModalState::Help            => handle_help_key(key),
        ModalState::Approval { .. } => handle_approval_key(key),
        ModalState::WaveOverview .. => handle_wave_overview_key(key),
        ModalState::PlanDetail ..   => handle_plan_detail_key(key),
        ModalState::TaskPicker ..   => handle_task_picker_key(key),
        ModalState::TaskDetail ..   => handle_task_detail_key(key),
        ModalState::QueueOverview.. => handle_queue_overview_key(key),
        ModalState::AgentPool ..    => handle_agent_pool_key(key),
        _ => TuiAction::None,       // <-- catches Quit, Confirm, Inject, BatchReview
    };
}
```

Four `ModalState` variants fall into the `_ => TuiAction::None` catch-all:

| ModalState variant | Has key handler? | Intended InputMode | Result |
|---|---|---|---|
| `Quit`             | No (uses `handle_confirm_key`) | `InputMode::Confirm` | **Dead** -- all keys produce `None` |
| `Confirm { .. }`   | No (uses `handle_confirm_key`) | `InputMode::Confirm` | **Dead** -- all keys produce `None` |
| `Inject { .. }`    | No (uses `handle_inject_key`)  | `InputMode::Inject`  | **Never set** as `active_modal` in practice; separate bug |
| `BatchReview { .. }`| No handler anywhere           | None                 | **Dead** -- no keyboard interaction possible |

### Impact per modal

#### Quit modal (ModalState::Quit + InputMode::Confirm)
- **Opens via:** pressing `q` when no modal is active
- **Expected keys:** `y`/Enter = quit, `n`/Esc = cancel
- **Actual behavior:** All keys except Ctrl+C are swallowed. User cannot confirm or
  cancel. The modal renders "[y] yes [n] no" hints but neither key works.
- **Only escape:** Ctrl+C hard-quits (bypasses modal intercept at line 488)

#### Confirm modal (ModalState::Confirm + InputMode::Confirm)
- **Opens via:** `open_confirm_modal()` -- force-advance, reset, merge, repair, etc.
- **Expected keys:** `y`/Enter = confirm action, `n`/Esc = cancel
- **Actual behavior:** Identical to Quit -- completely unresponsive
- **Severity:** All destructive-action confirmation dialogs are broken. The user sees
  the dialog but cannot interact with it at all.

#### Inject modal (ModalState::Inject + InputMode::Inject)
- **Opens via:** `StartInject` action sets `InputMode::Inject` but does NOT set
  `active_modal` to `ModalState::Inject`. The `ModalState::Inject` variant exists in
  the enum and has a renderer but is never constructed in `app.rs`.
- **Net effect:** The inject text-input works because it uses `InputMode::Inject`
  without setting `active_modal`, so the modal intercept never fires. However, the
  inject overlay is never visually rendered as a modal (no popup box, no dimming).
  This is a separate rendering bug, not a dismiss bug.

#### BatchReview modal (ModalState::BatchReview)
- **Opens via:** Can be set by external code (the variant exists) but no open path
  was found in app.rs dispatch.
- **Has key handler:** No -- neither in the modal intercept nor as an InputMode.
- **Rendered keys:** The renderer shows `[a] approve [r] reject [s] skip [Esc] close`
  but none of these are wired.
- **Net effect:** If this modal were ever opened, it would be completely inert.

---

## Modal-by-Modal Key Handling Audit

### Modals WITH dedicated key handlers (work correctly)

| Modal | Handler | Dismiss keys | Scroll keys | Other keys | Notes |
|---|---|---|---|---|---|
| **Help** | `handle_help_key` | Esc, `?`, `q` (toggle) | Up/Down/j/k, PgUp/PgDn, Home/End | -- | Correct |
| **Approval** | `handle_approval_key` | Esc, `n`/`N` (reject) | -- | `y`/`Y`/Enter (approve), `A`/Ctrl-A (approve all) | Correct |
| **WaveOverview** | `handle_wave_overview_key` | Esc, `w` (toggle) | Up/Down/j/k | -- | Correct |
| **PlanDetail** | `handle_plan_detail_key` | Esc | Up/Down/j/k | -- | Missing: PgUp/PgDn, Home/End, `q` to close |
| **TaskPicker** | `handle_task_picker_key` | Esc | Up/Down/j/k | Enter (select) | Missing: PgUp/PgDn, `q` to close |
| **TaskDetail** | `handle_task_detail_key` | Esc, `q` | Up/Down/j/k | Tab (sub-tab) | Missing: PgUp/PgDn, Home/End |
| **QueueOverview** | `handle_queue_overview_key` | Esc, `q` (toggle) | Up/Down/j/k | -- | Missing: Left/Right/h/l for milestone nav (mori has this) |
| **AgentPool** | `handle_agent_pool_key` | Esc, `q` (CloseModal) | Up/Down/j/k | -- | Missing: PgUp/PgDn |

### Modals WITHOUT key handlers (broken)

| Modal | Handler | Dismiss keys | Why broken |
|---|---|---|---|
| **Quit** | None (catch-all `_ => None`) | None work | `InputMode::Confirm` handler unreachable |
| **Confirm** | None (catch-all `_ => None`) | None work | `InputMode::Confirm` handler unreachable |
| **Inject** | None (never set as active_modal) | N/A | Uses `InputMode::Inject` without `active_modal`; not rendered as modal |
| **BatchReview** | None | None | No handler exists anywhere |

---

## Scroll Key Audit Inside Modals

| Key | Help | PlanDetail | TaskDetail | WaveOverview | QueueOverview | AgentPool | TaskPicker |
|---|---|---|---|---|---|---|---|
| Up/k | Scroll up | Scroll up | Scroll up | Scroll up | Select up | Scroll up | Select up |
| Down/j | Scroll down | Scroll down | Scroll down | Scroll down | Select down | Scroll down | Select down |
| PgUp | Page up | **None** | **None** | **None** | **None** | **None** | **None** |
| PgDn | Page down | **None** | **None** | **None** | **None** | **None** | **None** |
| Home | Jump top | **None** | **None** | **None** | **None** | **None** | **None** |
| End | Jump bottom | **None** | **None** | **None** | **None** | **None** | **None** |

Only the Help modal supports PgUp/PgDn/Home/End. All other scrollable modals are
limited to one-line-at-a-time scrolling.

---

## Tab/Number Key Interception

When any modal with a dedicated handler is active, Tab and number keys are consumed
by the modal intercept and mapped to `TuiAction::None` (the `_` catch-all in each
per-modal handler). This is correct -- they should not switch tabs while a modal is
open.

**Exception:** `handle_task_detail_key` maps Tab to `TuiAction::SwitchDetailTab(0)`,
which is intentional (cycles sub-tabs within the task detail modal).

For the broken modals (Quit, Confirm), ALL keys including Tab and numbers produce
`TuiAction::None`, which is correct in isolation but the problem is that the
_intended_ keys (y/n/Esc) also produce `None`.

---

## Modal Stack / Z-Order / Focus

- **No modal stack.** `active_modal` is `Option<ModalState>` -- only one modal at a
  time.
- **`dismiss_all_modals()`** clears `active_modal`, `pending_confirm`, and resets
  `input_mode` from Confirm to Normal. If the modal is an Approval, it auto-rejects.
- **Focus stealing:** Opening a modal does not save/restore the previous focus zone.
  After dismissing a modal, focus remains wherever it was. This is acceptable since
  modals don't change `focus`.
- **Dim overlay:** `app.rs` line 1091 correctly renders a dim overlay when
  `active_modal.is_some()`, then renders the modal on top.

---

## Test Coverage Gap

The existing tests bypass the broken path:

- `quit_opens_confirmation_modal_instead_of_exiting` (line 4884): dispatches
  `TuiAction::Quit` directly, verifying the modal opens. Does not test key input.
- `confirming_quit_exits` (line 4898): dispatches `TuiAction::ConfirmYes` directly,
  bypassing `handle_key`. Does not test that pressing 'y' produces `ConfirmYes`.
- `modal_visibility_reads_active_modal` (line 1325 in input.rs): tests the
  `ModalVisibility` struct construction but not key routing through it.
- No test exists that opens a Quit/Confirm modal and then sends a key through
  `handle_key` to verify the action.

---

## Fix

### Option A: Add dedicated handlers for Quit/Confirm/BatchReview (minimal, recommended)

Add three new arms to the modal intercept match in `input.rs` line 494:

```rust
if let Some(modal) = modals.active_modal {
    return match modal {
        ModalState::Help            => handle_help_key(key),
        ModalState::Approval { .. } => handle_approval_key(key),
        ModalState::WaveOverview .. => handle_wave_overview_key(key),
        ModalState::PlanDetail ..   => handle_plan_detail_key(key),
        ModalState::TaskPicker ..   => handle_task_picker_key(key),
        ModalState::TaskDetail ..   => handle_task_detail_key(key),
        ModalState::QueueOverview.. => handle_queue_overview_key(key),
        ModalState::AgentPool ..    => handle_agent_pool_key(key),
        // --- NEW ---
        ModalState::Quit            => handle_confirm_key(key),       // reuse confirm handler
        ModalState::Confirm { .. }  => handle_confirm_key(key),
        ModalState::BatchReview { .. } => handle_batch_review_key(key),
        ModalState::Inject { .. }   => handle_inject_key(key),        // if ever used as active_modal
    };
}
```

Where `handle_batch_review_key` is:
```rust
fn handle_batch_review_key(key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => TuiAction::CloseModal,
        KeyCode::Up | KeyCode::Char('k')  => TuiAction::ModalScrollUp,
        KeyCode::Down | KeyCode::Char('j') => TuiAction::ModalScrollDown,
        KeyCode::Char('a') => TuiAction::ConfirmYes,     // approve batch
        KeyCode::Char('r') => TuiAction::ConfirmNo,      // reject batch
        // KeyCode::Char('s') => TuiAction::SkipBatch,   // needs new action
        _ => TuiAction::None,
    }
}
```

This eliminates the `_ => TuiAction::None` catch-all entirely, making the match
exhaustive. Any future `ModalState` variant added without a handler will produce a
compile error.

### Option B: Restructure to match mori's architecture (larger, cleaner)

Move `InputMode` dispatch before the modal intercept, matching mori's pattern:

```rust
pub fn handle_key(...) -> TuiAction {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return TuiAction::QuitConfirmed;
    }

    match mode {
        InputMode::Confirm   => return handle_confirm_key(key),
        InputMode::Inject    => return handle_inject_key(key),
        InputMode::Filter    => return handle_filter_key(key),
        InputMode::ConfigEdit => return handle_config_edit_key(key),
        InputMode::LogSearch => return handle_log_search_key(key),
        InputMode::PlanFilter => return handle_plan_filter_key(key),
        InputMode::Normal    => {}  // fall through to modal + global + per-tab
    }

    // Modal intercepts only apply in Normal mode
    if let Some(modal) = modals.active_modal {
        return match modal { ... };
    }

    // Global keys, per-tab dispatch
    ...
}
```

This matches mori exactly and ensures `InputMode`-based modals always work regardless
of `active_modal` state. However, it changes the assumption that modal intercepts have
highest priority, which could introduce new bugs if any modal relies on being checked
before `InputMode`.

### Recommendation

**Option A** is safer and more surgical. It makes the modal intercept match exhaustive
(no catch-all), so the compiler will catch any future missing handler. The existing
`handle_confirm_key` already handles y/n/Enter/Esc correctly and can be reused for
both Quit and Confirm modals.

Additionally:
1. Add PgUp/PgDn/Home/End to PlanDetail, TaskDetail, WaveOverview, QueueOverview, AgentPool, and TaskPicker handlers.
2. Add `q` as a dismiss key to PlanDetail and TaskPicker (consistency with other modals).
3. Add Left/Right/h/l navigation to QueueOverview (mori parity).
4. Wire `ModalState::Inject` as `active_modal` in `StartInject`, or remove the dead variant.
5. Add integration tests that send keys through `handle_key` with active modals to catch regressions.

---

## Files Involved

| File | Lines | What |
|---|---|---|
| `crates/roko-cli/src/tui/input.rs` | 481-547 | `handle_key` -- priority inversion site |
| `crates/roko-cli/src/tui/input.rs` | 492-504 | Modal intercept with `_` catch-all |
| `crates/roko-cli/src/tui/input.rs` | 507-510 | `InputMode::Confirm` -- unreachable |
| `crates/roko-cli/src/tui/input.rs` | 566-645 | Per-modal key handlers (working ones) |
| `crates/roko-cli/src/tui/input.rs` | 647-653 | `handle_confirm_key` -- correct but unreachable |
| `crates/roko-cli/src/tui/app.rs` | 1130-1138 | `TuiAction::Quit` dispatch (sets Confirm + Quit modal) |
| `crates/roko-cli/src/tui/app.rs` | 1721-1784 | `ConfirmYes`/`ConfirmNo` dispatch |
| `crates/roko-cli/src/tui/app.rs` | 2189-2198 | `open_confirm_modal` (sets both `InputMode` and `active_modal`) |
| `crates/roko-cli/src/tui/app.rs` | 2908-2920 | `dismiss_all_modals` |
| `crates/roko-cli/src/tui/modals/mod.rs` | 49-110 | `ModalState` enum (12 variants) |
| `crates/roko-cli/src/tui/modals/quit.rs` | 1-47 | Quit modal renderer (renders y/n hints that don't work) |
| `crates/roko-cli/src/tui/modals/confirm.rs` | 1-168 | Confirm modal renderer (renders y/Enter/n/Esc hints that don't work) |
| `crates/roko-cli/src/tui/modals/batch_review.rs` | 1-165 | BatchReview renderer (renders a/r/s/Esc hints with no handler) |
| `apps/mori/src/tui/input.rs` (reference) | 138-202 | Mori's `handle_key` -- correct architecture |
