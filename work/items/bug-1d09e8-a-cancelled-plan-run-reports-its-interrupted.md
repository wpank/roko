+++
id = "bug-1d09e8"
kind = "bug"
title = "A cancelled plan run reports its interrupted task as failed, ends with a 'failed' engine error, and does not say what happens to its passed tasks' work"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs", "crates/roko-cli/src/graph_execution/batch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cancelled_run_reports_interrupted_tasks_as_cancelled' crates/roko-cli/ && cargo test -p roko-cli a_cancelled_run_reports_interrupted_tasks_as_cancelled"
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

- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the closing "Graph Engine: … failed" error, and the
  "{}: cancelling plan '{}' and stopping the run" message.
- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs`: the "stopped: its plan run is stopping" reason.
- `crates/roko-cli/src/graph_execution/batch.rs`: folding passed work into the batch branch.
- `crates/roko-serve/src/routes/runs.rs`: the summary's task counts and `result`.

## Current state

The run itself stops promptly: about 5 s after the cancel, with no agents left running.

## Plan

1. Settle a task that a cancel interrupts as `cancelled` (or `stopped`), not `failed`, and count it that way.
2. When the run was cancelled, skip the "N failed" engine error or replace it with "cancelled".
3. Name what stopped the run: a cancel over the API, from the TUI, or a real SIGINT.
4. Either fold passed tasks into the batch branch on cancel, or have the summary say that their work stays on
   attempt branches until the plan is resumed, and remove the worktrees once the run will not be resumed.

## Done when

`[[verify]]` passes: a cancelled run's interrupted task is reported as cancelled, and its last row is not a
failure.
