# 146 — Plan CLI Control Commands (`retry` / `pause` / `resume` / `cancel`)

**Priority**: P1 — Claude and other automated agents cannot control plan execution without a terminal; the TUI has `p` for pause but no CLI equivalents exist for scripted control of a running plan.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/runner/`
**Depends on**: None (signal-based IPC is the mechanism, not TUI keybindings)
**Sources**: `tmp/backlog/_checklist-gaps.md` §4.1

---

## Background

The self-hosting loop requires Claude to be able to control plan execution programmatically. Currently, all execution control actions (pause, resume, cancel, retry) are only available through:
1. TUI keybindings (interactive terminal session required).
2. Direct file editing (fragile, no clean signal mechanism).

Neither approach works when Claude is running headlessly or when the operator has stepped away from the terminal. Mori had CLI commands for these operations that sent signals to the running orchestrator process.

The three critical commands:
- `roko plan retry <plan-id> [task-id]` — retry a failed plan or specific task without restarting the full run.
- `roko plan pause` / `roko plan resume` — pause/resume the entire execution (all in-progress plans complete their current turn before pausing).
- `roko plan cancel <plan-id>` — cancel a specific running plan and mark it as terminated.

These complement the TUI recovery keybindings (#119) by providing the same operations through a non-interactive interface.

## Current State

- `crates/roko-cli/src/commands/plan.rs` — has `plan list`, `plan show`, `plan validate` but not `plan retry`, `plan pause`, `plan resume`, or `plan cancel`.
- Runner-v2 event loop — listens for `TuiCommand` over a channel; the same mechanism can be extended to accept commands from a control socket or signal file.
- `.roko/state/runner.lock` — contains the PID of the running runner process; can be used to signal the process.
- `roko inject` — already sends signals to sessions; similar IPC can be applied.

## Implementation Plan

1. **Control socket or signal file approach**: Rather than Unix signals (which are coarse), use a control file: `.roko/state/control.json`. The CLI command writes a JSON command to this file; the runner event loop polls it every 250ms and acts on commands. Commands are removed after processing.

   Alternative: use a named pipe or Unix domain socket for lower-latency communication.

2. **Runner control loop**: Add to the `tokio::select!` in `event_loop.rs`:
   - Poll `.roko/state/control.json` every 250ms.
   - Parse `{"command": "pause"|"resume"|"cancel"|"retry", "plan_id": "...", "task_id": "..."}`.
   - Dispatch to the appropriate handler.
   - Delete the file after processing.

3. **`roko plan retry <plan-id> [task-id]`**:
   - Writes `{"command": "retry", "plan_id": "<id>", "task_id": "<id>"}` to control file.
   - Runner marks the task as `Pending` and re-dispatches it.
   - If no `task_id` is given, retry all failed tasks in the plan.

4. **`roko plan pause` / `roko plan resume`**:
   - `pause`: writes `{"command": "pause"}`. Runner finishes current agent turn, then stops dispatching new tasks.
   - `resume`: writes `{"command": "resume"}`. Runner continues dispatching.
   - CLI commands are instantaneous (write to control file and exit); actual pause takes until the current turn completes.

5. **`roko plan cancel <plan-id>`**:
   - Writes `{"command": "cancel", "plan_id": "<id>"}`.
   - Runner sends cancellation to the plan's agent handle (see #139) and marks the plan as `Cancelled`.
   - Emits `RunnerEvent::PlanCancelled`.

6. **`roko plan status` (bonus)**: While implementing, add `roko plan status <plan-id>` that reads the current runner state from `.roko/state/run-state.json` and prints the plan's task completion status.

7. **Error when no runner is active**: If `.roko/state/runner.lock` does not exist or the PID is not running, print a clear error: "No active plan run found. Start one with `roko plan run plans/`."

## Acceptance Criteria

1. `roko plan pause` causes the runner to stop dispatching new tasks after the current turn completes.
2. `roko plan resume` causes the runner to resume dispatching.
3. `roko plan cancel <plan-id>` cancels the specified plan and emits `PlanCancelled` event.
4. `roko plan retry <plan-id>` marks all failed tasks in the plan as `Pending` and re-dispatches.
5. `roko plan retry <plan-id> <task-id>` retries only the specified task.
6. All commands print a clear error when no runner is active.
7. Commands work from a second terminal while `roko plan run` is active in the first.

## Verification Checklist

- [ ] Start `roko plan run` in terminal 1; run `roko plan pause` in terminal 2; verify dispatching stops.
- [ ] Run `roko plan resume`; verify dispatching continues.
- [ ] Run `roko plan cancel <id>`; verify the plan is marked `Cancelled` in state.
- [ ] Deliberately fail a task; run `roko plan retry <plan-id>`; verify the task restarts.
- [ ] Run any control command without an active runner; verify a clear error message.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/plan.rs` | Add `retry`, `pause`, `resume`, `cancel` subcommands |
| `crates/roko-cli/src/runner/event_loop.rs` | Add control file polling and command handlers |
| `crates/roko-cli/src/runner/types.rs` | Add `PlanCancelled` event variant; control command enum |
