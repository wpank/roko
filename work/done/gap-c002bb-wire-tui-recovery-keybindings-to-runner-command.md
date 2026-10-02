+++
id = "gap-c002bb"
kind = "gap"
title = "Wire TUI Recovery Keybindings to Runner Command Channel"
status = "done"
triage = "verified"
severity = "p1"
size = "L"
goal = "visibility"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "6a08f9e2c"
source = "tmp/backlog/archive/386-tui-recovery-keybindings-runner-channel.md#386 — Wire TUI Recovery Keybindings to Runner Command Channel"
discovered_from = "audit:tmp/backlog/archive/386-tui-recovery-keybindings-runner-channel.md#386 — Wire TUI Recovery Keybindings to Runner Command Channel"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::route_execution_commands", "crates/roko-cli/src/graph_execution/plan_runner.rs::PlanControl", "crates/roko-cli/src/graph_execution/plan_runner.rs::terminate_in_flight_agents", "crates/roko-cli/src/tui/app/modals.rs::send_tui_command_for_confirm", "crates/roko-cli/src/tui/app/channels.rs::drain_execution_acks", "crates/roko-cli/src/execution_control.rs::ExecutionCommandKind", "crates/roko-graph/src/engine.rs::FlowHandle"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'TUI command queued (post-execution; plan still running)' crates/roko-cli/src/graph_execution/plan_runner.rs && grep -rqw 'fn tui_skip_command_skips_the_running_task' crates/roko-cli/src && cargo test -p roko-cli --lib tui_skip_command_skips_the_running_task"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T07:43:03Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "L"
claimed_at = "2026-10-01T16:12:55Z"
forced = false
evidence = "TUI skip, retry, repair, reverify, approve/reject and reset reach the Graph run; tui_skip_command_skips_the_running_task passes (wk-childenv 338194043); gate 6h2 passed at 285282248 (cargo check, clippy -D warnings, 11,366 lib tests in roko-agent/cli/core/fs/gate/graph/learn/serve, canaries C1-C8 plus integration tests, 446 roko-cli bin tests, run_evidence py, portal tsc and 809 vitest); merged in 6a08f9e2c"
+++

## Problem

During `roko plan run` with the TUI (auto-launched on an interactive terminal, or via `--tui`), the recovery actions
in the TUI do nothing to the run, but the TUI tells the operator they were accepted:

- Soft retry, repair (preserve or clean), re-verify gates, force-advance, cancel agent, approve/reject and reset
  send an `ExecutionCommand` on the in-process channel. The Graph runner acknowledges them with
  `CommandAckStatus::Accepted` and a debug log "TUI command queued (post-execution; plan still running)", then drops
  them. Nothing is queued. After the last plan finishes, any late command is acked "plan finished — re-run to apply".
- "Cancel agent" (`ConfirmAction::CancelAgent`) is mapped to `ExecutionCommandKind::Skip` with the comment that
  the engine will terminate the agent and mark the task skipped. The runner ignores `Skip`, so the agent keeps
  running.
- "Reset selected plan" (`ConfirmAction::ResetSelectedPlan`) sends `Cancel`, not `Reset`.

Only pause, resume and cancel (whole plan, or every running plan) take effect.

Expected: each TUI recovery key either changes the run as its label says, or is rejected with a clear reason.
No command may be acked `Accepted` and then dropped.

## Why it matters

- Goal `visibility`: the TUI is the operator's control surface for a live run. Keys that silently do nothing are
  worse than missing keys, because the operator believes they acted (for example, that a runaway agent was
  stopped).
- A runaway or stuck agent can only be stopped by cancelling the whole plan or run.
- Related: `bug-8208a6` (the CLI equivalents `roko plan pause/cancel/resume/retry` write `.roko/state/control.json`,
  which the Graph engine never reads; `ControlCommand::poll` has no caller).

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::route_execution_commands` (~line 1775): drains
  `exec_cmd_rx` each tick. `Cancel` sets `PlanControl.cancel` (or drops a not-yet-started plan via
  `scheduler.cancel_pending`); `Pause`/`Resume` flip the shared pause flag; all other kinds fall into one arm that
  acks `Accepted` and drops the command.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` (~line 1449): after the plan loop, late commands are acked
  "plan finished — re-run to apply".
- `crates/roko-cli/src/graph_execution/plan_runner.rs::PlanControl` (~line 1700): only a `cancel: Arc<AtomicBool>`;
  `run_one_plan` polls it and calls `FlowHandle::cancel()` (~line 2129).
- `crates/roko-graph/src/engine.rs::FlowHandle::cancel`: whole-graph cancel only ("already-started nodes are not
  interrupted"); there is no per-node cancel or skip API.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::terminate_in_flight_agents` / `kill_in_flight_agents`
  (~line 355): signal every registered agent process tree (`roko_agent::process::registered_pids`); no per-task
  variant.
- `crates/roko-cli/src/execution_control.rs`: `ExecutionCommand` (has `plan_id`, `task_id`, `attempt`) and
  `ExecutionCommandKind` (`Pause`, `Resume`, `SoftRetry`, `Repair { preserve_completed }`, `ReverifyGates`,
  `Skip`, `Cancel`, `Approve { approval_id }`, `RejectApproval { .. }`, `Reset`).
- `crates/roko-cli/src/tui/app/modals.rs::send_tui_command_for_confirm` (~line 158): maps confirm actions to
  commands (see Problem).
- `crates/roko-cli/src/tui/app/actions.rs` (~line 540): pause/resume key.
- `crates/roko-cli/src/tui/app/channels.rs::drain_execution_acks` (~line 200): shows `Accepted` as an info toast
  ("command accepted" or the ack message), `Rejected` as a warning.

## Current state

- The command channel (P2-TUI-3) exists and pause/resume/cancel work (re-verified 2026-09-29 at `d9e79e9d8`).
- No code consumes `SoftRetry`, `Repair`, `ReverifyGates`, `Skip`, `Approve`, `RejectApproval` or `Reset` on the
  Graph path. The Runner-v2 loop that once handled some of them was deleted on 2026-09-06 (`6b5da8616`).
- No approval requests are raised on the Graph path (`graph_task_dispatch.rs` has no approval resolver), so
  `Approve`/`RejectApproval` have nothing to resolve.
- Recent commits touching `plan_runner.rs`: `c41e78c7a`, `725f21e05`; none changed command routing semantics.

## Plan

1. Stop acknowledging no-ops. In `route_execution_commands` and the post-loop drain, ack every command that will
   not take effect as `CommandAckStatus::Rejected` with a reason, for example "skip is not supported during a Graph
   run; cancel the plan and re-run with --resume-plan". Delete the "queued (post-execution)" arm. This alone makes
   the TUI honest and is a safe first commit.
2. Skip of a running task (`CancelAgent`, task-scoped `Skip`):
   - Give the Graph task dispatcher (`crates/roko-cli/src/graph_task_dispatch.rs`) a registry of in-flight
     attempts keyed by `(plan_id, task_id)` holding a cancellation token and the attempt's agent PIDs.
   - Pass a handle to that registry into `route_execution_commands`; on `Skip` with a running task, trigger its
     token and SIGTERM only that task's process tree (a per-task form of `terminate_in_flight_agents`).
   - The dispatcher returns a distinct outcome for an operator skip. Design choice: (a) mark the task failed with
     reason "skipped by operator" (dependents are skipped by the engine; simplest; recommended first), or
     (b) mark it skipped-as-done so dependents run (needs a new engine status and checkpoint representation).
   - Ack `Completed` once the task has settled; `Rejected` if the task is not running.
3. Plan-scoped recovery for a plan that already finished failed in this run (`SoftRetry`, `Repair`, `Reset`,
   `ForceAdvance`): re-admit the plan to `PlanSetScheduler` with resume semantics (the same checkpoint reuse as
   `roko plan run --resume-plan`; `preserve_completed=false` resets eligible state without erasing committed
   receipts). For a plan that is still running, reject with "plan is running; cancel it first".
4. `ReverifyGates`: either run the gate pipeline for the plan's completed tasks once the plan is idle, or reject
   with a reason. Rejecting is acceptable for this item.
5. `Approve`/`RejectApproval`: reject with "no pending approvals on the Graph path" until approvals exist.
6. Fix the TUI mapping: `ResetSelectedPlan` should send `Reset` (or be relabelled as cancel).
7. Tests in `plan_runner.rs` (unit, fake dispatcher): `tui_skip_command_skips_the_running_task` (skip reaches the
   named running task, its attempt ends, ack is `Completed`), and a test that every unsupported kind is acked
   `Rejected`, never `Accepted`.

## Done when

- Pressing cancel-agent in the TUI during a run stops that task's agent within a few seconds; the task is recorded
  as failed (or skipped) by operator, and other tasks keep running.
- Soft retry / repair on a plan that failed earlier in the same run re-runs its failed tasks without restarting
  `roko plan run`.
- Every command that has no effect is shown as a warning toast with a reason; no `Accepted`-then-dropped path is
  left.
- Verify (the current `[[verify]]` contains prose inside the command and cannot run; proposed replacement):

  ```
  ! grep -q 'TUI command queued (post-execution; plan still running)' crates/roko-cli/src/graph_execution/plan_runner.rs && grep -rqw 'fn tui_skip_command_skips_the_running_task' crates/roko-cli/src && cargo test -p roko-cli --lib tui_skip_command_skips_the_running_task
  ```

## Notes

- Process signalling is risky: signal only the process tree registered for the target attempt, never
  `registered_pids()` wholesale; the existing all-agents path is for interrupts only.
- Checkpoint/persistence semantics for "skipped by operator" must survive resume; do not invent a status the
  checkpoint loader rejects (`crates/roko-cli/src/graph_checkpoint.rs`).
- Step 1 can land alone and quickly; steps 2-3 are the bulk of the work.
- `bug-8208a6` should reuse the same routing (CLI `control.json` commands -> `ExecutionCommand`), so do this item
  first or together.
- Conflicts with other work in `graph_execution/plan_runner.rs` (plan loop, scheduler) and
  `graph_task_dispatch.rs`; do not run in parallel with items touching those.
- 2026-10-01 (wk-childenv): partial on work/gap-1555ac (Plan steps 1, 4, 5 and 6); cargo verification deferred
  to the batch check. `route_execution_commands` (plan_runner.rs) rejects, with a reason (`reject_command`),
  every command a Graph run does not carry out (soft retry, repair, reset, re-verify, skip, approve/reject) and
  a cancel naming a plan that is neither running nor waiting; the post-run drain rejects late commands. No
  `Accepted`-then-dropped path is left. `R` and `Ctrl-d` still send `Cancel`, the TUI's only plan cancel, and
  now say "Cancel plan" (confirm text, help, docs/v2). Tests: `unsupported_tui_commands_are_rejected_with_a_reason`,
  `tui_cancel_reaches_a_running_plan_and_drops_a_waiting_one`, `reset_plan_key_confirms_and_sends_a_cancel`.
- Left (steps 2, 3 and the verify test): a per-task stop for `Skip` (`GraphConductor::supervise` in
  `graph_task_dispatch/supervision.rs` already keeps a cancel token per running attempt, but only while the
  conductor is on, and `run_watched` is where an operator stop would end the provider call), then
  `tui_skip_command_skips_the_running_task`; re-admitting a plan that failed earlier in the run for retry,
  repair and reset. Once reset works, `R` can send `Reset` again and cancel needs a key of its own.
- 2026-10-02 (wk-childenv): Plan steps 2, 3 and 6 on work/gap-1555ac; cargo verification deferred to the batch
  check.
  - Skip (step 2, option a): `graph_task_dispatch/operator_stop.rs` adds `OperatorStops`, the attempts whose
    provider calls run, by plan and task. `run_watched` registers each attempt and ends its call when the
    operator stops it (`AttemptInterrupted::StoppedByOperator`), as the stall watchdog does. The attempt fails
    as `Cancelled` ("stopped by the operator"), so it is not retried; the engine skips the task's dependants,
    and the plan's other tasks run on. `route_execution_commands` sends a skip naming a running task there
    and acks it `Completed`; a skip of a task with no running agent is rejected with that reason. Only the
    named attempt's call is dropped, no process group is signalled wholesale.
  - Re-run (step 3): soft retry and repair that keeps completed work resume the plan's checkpoint, while reset
    and a clean repair archive it as `--fresh` does (`PlanRerun`). Both apply to a plan that failed or was
    cancelled earlier in the same run. `PlanSetScheduler::retry` puts it back to pending, with every plan
    blocked behind it that nothing else blocks now. The command is acked `Accepted`, and the plan starts once
    a slot is free. A plan that is running, waiting, blocked or did not fail is refused with the reason
    (`rerun_refusal`). A run with a re-run plan settles by each plan's last run. `roko plan retry`
    (control.json) takes the same path.
  - Keys (step 6): cancel has a key of its own, `C` on the Plans tab (`ConfirmAction::CancelPlan`; `Ctrl-d`
    too, via the renamed `TuiAction::CancelSelectedPlan`). `R` sends `Reset` again. Help, confirm texts and
    docs/v2 CLI-REFERENCE say so.
  - Tests: `tui_skip_command_skips_the_running_task`, `tui_retry_and_reset_run_a_failed_plan_again` and the
    reworked `unsupported_tui_commands_are_rejected_with_a_reason` (plan_runner.rs);
    `an_operator_stop_ends_the_running_attempt_without_a_retry` (watchdog.rs, a real dispatch against a
    silent fake provider); `a_stop_reaches_only_the_task_it_names_while_it_runs` (operator_stop.rs);
    `retry_runs_a_failed_plan_again_and_frees_what_it_blocked` (plan_set.rs);
    `reset_plan_key_confirms_and_sends_a_reset` and `cancel_plan_key_confirms_and_sends_a_cancel` (TUI).
  - Every Done-when line is now covered. Two limits remain. A plan can be re-run only while the run is still
    going: once the last plan ends, the run ends, and late commands are rejected with "the plan run has
    finished". Re-verifying gates (step 4), approvals (step 5) and force-advance are still rejected with
    their reasons.

## Original notes

pause/retry/skip are visual-only with no runner effect. TUI parity audit found that recovery keybindings (pause, retry, skip) write to `.roko/engrams.jsonl` and show a toast, but the runner has no command channel from TUI. These are visual-only with no actual effect on execution.

Imported without verification from:
- `tmp/backlog/archive/386-tui-recovery-keybindings-runner-channel.md#386 — Wire TUI Recovery Keybindings to Runner Command Channel`

How to verify: Check whether the gap described in tmp/backlog/archive/386-tui-recovery-keybindings-runner-channel.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Verified 2026-09-28: The P2-TUI-3 command channel exists and Cancel/Pause/Resume take effect (plan_runner.rs exec_cmd_rx drain). SoftRetry/Repair/ReverifyGates/Skip/Approve/RejectApproval/Reset are only acked `Accepted` ('TUI command queued (post-execution; plan still running)') with no consumer, and once the flow ends they are acked 'plan finished — re-run to apply'. Retry and skip therefore still have no runner effect.

Re-verified 2026-09-29: pause, resume and cancel reach the Graph run (plan_runner.rs::route_execution_commands). What remains: retry (SoftRetry), skip, repair, reverify-gates, approve/reject and reset are acknowledged as Accepted and dropped, so those TUI keys still have no runner effect. The TUI's confirm action CancelAgent (tui/app/modals.rs) sends Skip on the assumption that the engine terminates the agent and marks the task skipped, but the runner ignores Skip, so cancelling an agent from the TUI also does nothing during a run.
