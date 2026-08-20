# 119 — TUI Recovery Keybindings (s / z / S / R / c)

**Priority**: P1 — Without recovery keybindings, a stuck or failed agent requires Ctrl-C and a full restart; there is no way to intervene in a running plan from the TUI without losing all in-progress state.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/tui/app.rs`, `crates/roko-cli/src/tui/tabs.rs`, `crates/roko-cli/src/runner/event_loop.rs`
**Depends on**: #114 (diagnose command — the `z` diagnose modal reads from it)
**Sources**: `tmp/backlog/_checklist-gaps.md` §1.4, `tmp/backlog/_mori-old-gaps.md` MO-34

---

## Background

Mori's TUI gave operators five keys for plan recovery during a live run:
- `s` — soft retry: preserve completed tasks, re-run only the failed tasks
- `z` — diagnose: open a modal showing the plan's diagnostic report (from `roko diagnose`)
- `S` — repair with error context: preserve work, clean worktree, re-dispatch with error context injected
- `R` — reset: clean slate, discard worktree, restart from scratch
- `c` — reverify gates only: re-run gate pipeline on existing output without re-running the agent

These operations exist at the runner level (retry logic, worktree cleanup, gate dispatch) but none of them are accessible from the TUI during a live run. The TUI has an inject modal and an approval modal, but no recovery shortcuts. The underlying runner channels exist (the runner listens for `TuiCommand` messages), so this is primarily a TUI input handling and channel wiring task.

Additional polish required by the checklist: context-sensitive keybind hints in the status bar that only show recovery keys when a failed task is selected on the F2:plans tab.

## Current State

- `crates/roko-cli/src/tui/app.rs` (4,576 LOC) — handles key input and dispatches `TuiAction` variants; does not have recovery action variants.
- `crates/roko-cli/src/tui/tabs.rs` — tab rendering; status bar keybind hints are static.
- `crates/roko-cli/src/runner/event_loop.rs` — handles `TuiCommand` messages from the TUI channel.
- The operations exist in code: retry in event loop, worktree clean via `WorktreeManager`, gate dispatch via `gate_dispatch.rs`.
- No `TuiAction::SoftRetry`, `TuiAction::DiagnosePlan`, `TuiAction::RepairPlan`, `TuiAction::ResetPlan`, `TuiAction::ReverifyGates` variants exist.

## Implementation Plan

1. **Add `TuiAction` variants**:
   ```rust
   SoftRetry { plan_id: String },
   DiagnosePlan { plan_id: String },
   RepairPlan { plan_id: String, with_error_context: bool },
   ResetPlan { plan_id: String },
   ReverifyGates { plan_id: String },
   ```

2. **Map keys to actions in `app.rs`**: In the F2:plans tab key handler, when a plan row is selected and in a failed/stalled state:
   - `s` → `TuiAction::SoftRetry`
   - `z` → `TuiAction::DiagnosePlan`
   - `S` → `TuiAction::RepairPlan { with_error_context: true }`
   - `R` → `TuiAction::ResetPlan` (with confirmation dialog)
   - `c` → `TuiAction::ReverifyGates`

3. **Confirmation dialog for destructive actions**: `R` (ResetPlan) must show a modal: "Reset plan `<id>`? This discards the worktree. [y/N]". `S` (RepairPlan) should also confirm.

4. **Send `TuiCommand` to runner**: Each `TuiAction` variant translates to a `TuiCommand` sent over the existing channel to `event_loop.rs`.

5. **Handle `TuiCommand` in runner**:
   - `SoftRetry`: mark failed tasks as `Pending` and re-dispatch; do not touch completed tasks.
   - `DiagnosePlan`: run `cmd_diagnose()` in a background task and display the JSON output in a modal.
   - `RepairPlan`: clean the worktree (`WorktreeManager::clean()`), re-dispatch with injected error context.
   - `ResetPlan`: destroy and recreate worktree, reset all tasks to `Pending`, re-dispatch.
   - `ReverifyGates`: skip agent dispatch, only run gate pipeline on current worktree output.

6. **Context-sensitive status bar hints**: In `tabs.rs`, the status bar at the bottom should show recovery hints only when a failed plan is selected on F2: `[s]oft-retry [z]diagnose [S]repair [R]reset [c]reverify`. When no failed plan is selected, show normal navigation hints.

7. **Global keys**: `Ctrl-A` (approve all pending approvals) and `Ctrl-G` (git reconcile) should be global (not tab-specific).

## Acceptance Criteria

1. Pressing `s` with a failed plan selected on F2 triggers soft retry (re-runs only failed tasks).
2. Pressing `z` opens a modal showing the diagnostic JSON for the selected plan.
3. Pressing `R` shows a confirmation dialog before resetting the plan.
4. Pressing `c` re-runs only the gate pipeline without agent dispatch.
5. Status bar shows recovery hints only when a failed plan is selected.
6. Global `Ctrl-A` approves all pending approvals visible in the F5:approvals queue.
7. All recovery actions are logged in the structured log (`--log-file` path, see #115).

## Verification Checklist

- [ ] Run a plan that has a failed task; press `s` and verify the task restarts.
- [ ] Press `z` and verify a diagnose modal appears with plan details.
- [ ] Press `R` and verify a confirmation dialog appears; confirm with `y`; verify worktree is reset.
- [ ] Press `c` and verify only the gate output changes (no new agent turn in episodes.jsonl).
- [ ] Verify status bar shows `[s]oft-retry` only when a failed plan row is selected.
- [ ] Verify status bar shows normal navigation hints when a completed plan is selected.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/app.rs` | Add `TuiAction` recovery variants; key handler for F2 tab |
| `crates/roko-cli/src/tui/tabs.rs` | Context-sensitive status bar hints |
| `crates/roko-cli/src/runner/event_loop.rs` | Handle recovery `TuiCommand` variants |
| `crates/roko-cli/src/runner/types.rs` | Add recovery variants to `TuiCommand` |
| `crates/roko-cli/src/tui/mod.rs` | Wire confirmation dialog module |
