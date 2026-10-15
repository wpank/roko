+++
id = "bug-7256b9"
kind = "bug"
title = "A plan run roko serve cancels can end with a 'completed with task-level failures' error, when it settles within the cancel handler's 500 ms"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "roko-checks, bug-1d09e8 serve verification (tmp/hermes-roko-v5/findings/roko-checks.md), 2026-10-15"
discovered_from = "bug-1d09e8"
anchors = ["crates/roko-serve/src/routes/plans/run_control.rs"]
links = { depends_on = [], blocks = [], related = ["bug-1d09e8", "bug-db89d4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cancelled_run_publishes_no_task_failure_error' crates/roko-serve/src && cargo test -p roko-serve --lib a_cancelled_run_publishes_no_task_failure_error"
+++

## Problem

A plan run that `roko serve` cancels can end with an `error` event "plan <id> completed with task-level failures",
although the person cancelled it and nothing failed. Hosts show it as a failure under a cancelled run.

Whether the error appears depends on timing. The cancel handler (`cancel_plan_run`) waits 500 ms after the cancel,
then aborts the run's handler task:
- a run that settles after that is aborted before it can publish the error;
- a run that settles within the 500 ms publishes it.

Seen 2026-10-15 on work/bug-1d09e8 (04e7d475b), `tmp/hermes-roko-v5/roko-checks/cancel-demo/run-serve.sh`, run
`after-serve`. The run was cancelled with `/mcp` `run_cancel` while T2's stand-in agent ran. The agent died on the
SIGTERM, so the run settled at once. The events end with `run_completed outcome=cancelled`, then
`error: "plan cancel-demo completed with task-level failures"`. On main 7b6a5af44 the same run (`before-serve-1`)
settled later and was aborted, so it had no such row.

## Why it matters

A cancel is the person's choice, not a failure. bug-1d09e8 stopped reporting the cancelled task as failed. This
row then becomes the one misleading failure left on a cancelled run, once bug-db89d4 turns the closing tally into a
notice.

## Where

`crates/roko-serve/src/routes/plans/run_control.rs`, in the task the plan-run handler spawns (around
`let failure = (!success).then(|| format!("plan {plan_id} completed with task-level failures"))`). It publishes
`ServerEvent::Error` for any run that did not succeed, cancelled ones included.

## Current state

`plan_run_end` already ends a cancelled run as `RunState::Cancelled`, with no error. Only the bus error above is
published regardless of the cancel.

## Plan

1. Publish the error only when the run was not cancelled:
   `let failure = (!success && !task_cancel.is_cancelled()).then(...)`.
2. Test with a runtime whose `run_plan` returns `success: false` as soon as its cancel token fires. After a cancel,
   no `Error` event names the plan and the status says cancelled.

## Done when

`[[verify]]` passes: a cancelled run that settles at once publishes no "completed with task-level failures" error.
