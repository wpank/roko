+++
id = "gap-0af587"
kind = "gap"
title = "roko diagnose and the run summary don't say why the gaming screen left a task unverified: diagnose says 'it has no verify steps'"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "find-2ba86d follow-up, 2026-10-27"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::describe_task", "crates/roko-serve/src/routes/runs.rs::summarize_run"]
links = { depends_on = [], blocks = [], related = ["find-2ba86d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn diagnose_names_the_gaming_screens_reason' crates/roko-cli/src && cargo test -p roko-cli --lib diagnose_names_the_gaming_screens_reason"
+++

## Problem

find-2ba86d's gaming screen settles an attempt whose checks passed on a suspect change `unverified`, and its reason
is in the attempt's verdict (the step `pre_verify:gaming`, `skip_reason`), on the hub (the warning
`gaming:<plan>/<task>`) and in the log. The places a person or a chat host reads after the run do not show it:

- `roko diagnose <plan>` describes any `unverified` task without `checkout_written` findings as "completed; it has no
  verify steps" (`describe_task`), which is wrong for a task whose verify steps passed and that the screen flagged.
- The run summary a host posts (`GET /api/runs/{run_id}/summary`, `summarize_run`) and the end of `roko plan run`
  list `checkout_written` paths, but not the gaming screen's findings, so a host sees `unverified` without the why.

## Why it matters

Goal `truth`: the item that added the screen asked roko to "say why" an attempt is unverified. The verdict says it;
the reports people read do not, and one says something false.

## Where

- `crates/roko-cli/src/commands/diagnose.rs::describe_task` and `RunRecords::load`, which already read the run's
  verdicts (`roko_learn::telemetry::report::RunRecords`) for `checkout_written`.
- `crates/roko-learn/src/telemetry/report.rs::RunRecords::checkout_writes`: the model for a reader of the gaming
  step.
- `crates/roko-serve/src/routes/runs.rs::summarize_run` and the end of `roko plan run`
  (`crates/roko-cli/src/graph_execution/plan_runner.rs`).

## Current state

The verdict row of a flagged attempt has `outcome = "unverified"`, `gate_verdict = "unverified"` and a step
`{"rung": "pre_verify:gaming", "passed": null, "skipped": true, "skip_reason": "the checks passed, but the change may
pass them without doing the task: hidden_state `money.py`: …"}` (`red_flags::settle_suspect`). Nothing reads it.

## Plan

1. A `RunRecords` reader for the gaming step's reason per task, as `checkout_writes` is for its findings.
2. `roko diagnose` says "unverified: <reason>" for such a task, in text and `--json`.
3. The run summary and the end of `roko plan run` list the flagged tasks with their reasons.

## Done when

`diagnose_names_the_gaming_screens_reason` passes: a plan whose task's verdict carries the gaming step is described
with its reason, not "it has no verify steps".

## Notes

Follow-up of find-2ba86d (2026-10-27), kept out of it to keep that change's footprint to the screen and its verdict.
