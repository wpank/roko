+++
id = "bug-1d09e8"
kind = "bug"
title = "A cancelled plan run reports its interrupted task as failed, ends with a 'failed' engine error, and does not say what happens to its passed tasks' work"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_execution", "roko-core/dashboard_snapshot", "roko-serve/runs"]
created = 2026-10-13
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "04e7d475b"
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::PlanRunInterrupt", "crates/roko-cli/src/graph_execution/plan_runner.rs::cancel_reason", "crates/roko-cli/src/serve_runtime.rs", "crates/roko-core/src/dashboard_snapshot.rs::classify_task_outcome", "crates/roko-serve/src/routes/runs.rs::run_summary"]
links = { depends_on = [], blocks = [], related = ["bug-db89d4", "bug-7b62ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cancelled_run_reports_interrupted_tasks_as_cancelled' crates/roko-cli/ && cargo test -p roko-cli --lib a_cancelled_run_reports_interrupted_tasks_as_cancelled"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T19:52:44Z"
commit = "04e7d475b"
by = "roko-checks"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-15T15:37:42Z"
forced = true
evidence = "04e7d475b: a task cut off by a stop (its node failed with RokoError::Cancelled) is task_completed outcome=cancelled with a graph.task_cancelled notice, raises no 'task failed' error, sits in the checkpoint's cancelled set and counts apart from failures (TaskVerdictCounts, run metrics, TaskOutcomeClass::Cancelled in the snapshot, TUI and run summary, which gains tasks.cancelled); a host's cancel token requests PlanRunInterrupt::Cancel (label cancel) and closing the TUI TuiClosed, and the notices name the cause and say a resume keeps the passed tasks. Tests: a_cancelled_run_reports_interrupted_tasks_as_cancelled, a_task_a_stop_cut_off_is_cancelled_not_failed, a_stopping_run_holds_back_failures_from_the_poll, a_cancellation_is_told_apart_by_its_text plus updated ones; roko-cli lib 3639, roko-core lib 2019, roko-serve lib 1018, graph_timeout_matrix 7, honest_verdicts_canary 1, plan_execute 6 passed; clippy -D warnings and rustfmt clean. Live, zero cost (tmp/hermes-roko-v5/roko-checks/cancel-demo): CLI SIGINT and serve /mcp run_cancel both show T2 cancelled with its reason and the cause named ('a cancel request is stopping the run'); the serve summary counts cancelled 1, failed 0; a resume after the cancel ran only T2 and T3. With bug-db89d4 merged (main merged in at 122d1bd7b), the test asserts that a cancelled run publishes no error event at all (roko-cli lib 3652, roko-core 2024, roko-serve 1025 passed on that state). Left: serve's 'completed with task-level failures' error on a fast-settling cancel is bug-7256b9. Claimed with --force over batch.rs, which this item does not touch (roko-delivery's bug-db89d4)."
+++

## Problem

Run `0f2ac806-cdb3-46c1-9d94-8f74487ae737` (plan `project-tally-a-small-python-cli-expense-ledger-2`, Hermes
dogfood, 2026-10-13) was cancelled over `/mcp` `run_cancel` while T3 ran, after T1, T2 and T4 had passed:

- T3 ends with `task_completed outcome=failed` and the error "task 'T3' failed: cancelled: agent for …/T3
  stopped: its plan run is stopping". The summary counts it as failed (`failed: 1, other: 1, passed: 3`).
- The event log says "SIGINT: cancelling plan '…' and stopping the run" and "SIGINT: run stopped, 0 plan(s) not
  started", although no signal was sent; the cancel came over the API.
- The run's last error is "Graph Engine: 1 plan(s), 0 succeeded, 1 failed".
- The summary's `result` names `roko/batch/0f2ac806-…` at the base commit with 0 files changed. T1, T2 and T4's
  commits exist only on `roko/attempt/*` branches in four worktrees left under `.roko/worktrees/`. Nothing tells
  the person whether `--resume-plan` will reuse them, or how to keep that work.

Hosts render this as "✗ T3 … failed" and "⚠ Graph Engine: 1 plan(s), 0 succeeded, 1 failed" under a cancelled run.

## Why it matters

A cancel is the person's choice, not a failure. Reporting it as a failure makes the task look broken, feeds
learners a false failure, and leaves the person unsure what they lost (goals `truth` and `core`).

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs`:
  - `PlanRunInterrupt`: what stopped the run, its label and the `cause` phrase the run's notices use;
  - `cancel_reason` and `cancelled_tasks`: a task whose node failed with a `RokoError::Cancelled`;
  - the watch loop, which holds a failure back while the run stops, and the final diff, which reports the task;
  - `TaskVerdictCounts`, `task_outcomes` and `plan_stopped_message`.
- The cancel bridges request `PlanRunInterrupt::Cancel`: `serve_runtime.rs`, `run.rs` and `graph_entry_cells.rs`.
- `crates/roko-core/src/dashboard_snapshot.rs`: `TASK_OUTCOME_CANCELLED` and `TaskOutcomeClass::Cancelled`.
- `crates/roko-serve/src/routes/runs.rs::run_summary`: the summary's task counts.
- The TUI's `TaskStatus::Cancelled`.

## Current state

The run itself stops promptly: about 5 s after the cancel, with no agents left running.

Fixed on `work/bug-1d09e8`:

- **The cut-off task.** Every task-path producer of `RokoError::Cancelled` means the run or the operator
  stopped the task. A task whose node failed with one:
  - is published as `task_completed outcome=cancelled`, with a notice "plan 'p': task 'T3' cancelled: <reason>";
  - raises no "task 'T3' failed" error;
  - sits in the checkpoint's `roko.task.outcome@1` `cancelled` set, not in `failed`;
  - is counted apart from failures in the plan's verdict counts and the run metrics.
- **Holding back.** While a run stops, the live poll holds a node's failure back until the graph's output
  carries the error. A graph that never settles reports the tasks it cut off as cancelled.
- **Counting.** `classify_task_outcome` classes a cancelled outcome as `Cancelled`, which counts as neither done
  nor failed. That covers the dashboard snapshot, the TUI and the run summary, which gains a `cancelled` count.
- **Naming the stop.** A host's cancel token requests `PlanRunInterrupt::Cancel` (label `cancel`, exit 130).
  Closing the TUI requests `TuiClosed` (`tui-closed`). The notices name the cause:
  - "a cancel request is stopping the run: cancelling plan 'p'";
  - "plan 'p' stopped by a cancel request: 3 task(s) passed, 1 cancelled; a resume keeps the passed tasks and
    runs the rest";
  - "a cancel request stopped the run".
  SIGINT, SIGTERM and SIGHUP keep their names.

## Plan

1. Settle a task that a cancel interrupts as `cancelled`, not `failed`, and count it that way (done).
2. When the run was cancelled, do not end it with the "N failed" engine error. roko-delivery's bug-db89d4
   rewrites that closing row to count plans by outcome, so an interrupted plan gets a notice. This item's test
   checks that no other error is published; tighten it to "no error at all" once bug-db89d4 is merged.
3. Name what stopped the run: a cancel over the API, from the TUI, or a real SIGINT (done).
4. Say what happens to the passed tasks' work. The stop notice says a resume keeps them (done). Delivering the
   verified tasks of a plan that did not succeed to the batch branch, and cleaning up worktrees, is
   roko-delivery's (bug-db89d4 and its follow-ups).

## Done when

`[[verify]]` passes: a cancelled run's interrupted task is reported as cancelled, and the run publishes no error
apart from the closing tally, which bug-db89d4 turns into a notice; then its last row is not a failure.

## Notes

- Split agreed with roko-delivery on 2026-10-15. roko-delivery owns:
  - the closing "Graph Engine: N plan(s) …" row and `plan_outcomes`;
  - `deliver_plan_to_batch`, `batch.rs` and `run_result.rs`;
  - worktree retention.
- Not covered:
  - A single task the operator stops while the run goes on is still reported `failed` by the live poll at
    first, then `cancelled` by the final diff. The engine's status snapshot carries no errors.
  - The TUI graph tab's `GraphNodeStatus` (`view_state.rs`) still shows a cancelled node as failed.
- bug-7b62ff (a cancel during delivery's post-merge regression check) is a different code path: the regression
  check never watches the run's interrupt. It stays filed.
