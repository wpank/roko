+++
id = "bug-5013e8"
kind = "bug"
title = "roko plan status shows a failed task as 'ready': its task rows read tasks.toml's authored status and take the run's view only when the checkpoint succeeded"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "337914fb3"
source = "round v8 roko-recovery terminal captures (tmp/hermes-roko-v8/design/terminal/before, a0089c19c), the lead's approved proposal (tmp/hermes-roko-v8/design/terminal/README.md)"
discovered_from = "agent:roko-recovery (round v8, a0089c19c)"
anchors = ["crates/roko-cli/src/commands/plan.rs::cmd_plan_dir_status"]
links = { depends_on = [], blocks = [], related = ["gap-082a14"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_rows_say_what_the_run_did' crates/roko-cli/src && cargo test -p roko-cli --lib plan_status_view::tests::plan_status_rows_say_what_the_run_did"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T17:13:38Z"
commit = "337914fb3"
by = "roko-recovery"
executor = "claude-agent"
size = "S"
claimed_at = "2026-11-12T14:38:51Z"
forced = false
evidence = "337914fb3 on work/gap-5043c6: roko plan status reads each task's row from the run (roko_cli::plan_status_view, through diagnose's report and the run view's table), not tasks.toml's authored status. plan_status_rows_say_what_the_run_did passes: a failed run reads task 1 passed, task 2 failed after 2 tries (check runs failed), task 3 did not run, Next roko diagnose demo; an interrupted run reads task 1 passed, task 2 stopped, task 3 waiting, Next roko run plans/demo --resume-plan; a plan that never ran waits, Next roko run plans/demo. The verify names the lib test, since the rows are built in the lib (under --bin roko the filter matched nothing). On round v8's story workspace roko plan status plans/broken reads task 1 failed, 2 tries, check imports failed (a0089c19c: T1 ready)."
+++

## Problem

`roko plan status plans/<p>` takes each task's row from `tasks.toml`'s authored `status`. It promotes that to
`done` only when the plan's Graph checkpoint succeeded. After a failed or interrupted run, every task reads as
authored, usually `ready`, whatever the run did. A failed task reads `ready`, and so does a task that passed in a
run that stopped later. In round v8's capture (`tmp/hermes-roko-v8/design/terminal/before/plan-status-broken.*`):

```
status:          failed

  T1  ready       Parse the settings file
```

Expected: `T1  failed`, after 2 tries.

## Why it matters

Goal tooling. The row contradicts the status line above it. gap-082a14 is the other side: the Graph engine never
writes status back to tasks.toml. The CLI views should read the run, as `plan list` does
(`plan::overlay_graph_checkpoint_status`).

## Where

`crates/roko-cli/src/commands/plan.rs::cmd_plan_dir_status`, the task rows near its end (`is_pre_execution_status`).
The run's view of each task is in `graph_checkpoint::inspect_canonical_checkpoint` (its committed records) and in the
run's `attempts.jsonl` verdicts.

## Plan

Derive each row from the checkpoint inspection: passed, failed, blocked, or not run. Fall back to tasks.toml only
when the plan has no checkpoint. Fixed with round v8's plan status redesign.

The rows come from `roko_cli::plan_status_view`, in the lib, so the test is a lib test: it builds its run with the
lib's checkpoint fixtures (`start_plan_checkpoint` is test-only there). The verify first named `--bin roko`, where
the filter would match nothing and pass.

## Done when

`plan_status_rows_say_what_the_run_did` passes: a failed task reads failed, and a task passed in an interrupted run
reads passed. The `[[verify]]` passes.
