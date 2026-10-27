+++
id = "gap-0af587"
kind = "gap"
title = "roko diagnose and the run summary don't say why the gaming screen left a task unverified: diagnose says 'it has no verify steps'"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "ee5bd215d"
source = "find-2ba86d follow-up, 2026-10-27"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::describe_task", "crates/roko-serve/src/routes/runs.rs::summarize_run"]
links = { depends_on = [], blocks = [], related = ["find-2ba86d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn diagnose_names_the_gaming_screens_reason' crates/roko-cli/src && cargo test -p roko-cli --lib diagnose_names_the_gaming_screens_reason"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T10:22:54Z"
commit = "ee5bd215d"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-27T08:31:11Z"
forced = false
evidence = "ee5bd215d: roko-learn adds GAMING_SCREEN_RUNG (the pre_verify:gaming step red_flags::settle_suspect writes), AttemptVerdictRecord::gaming_reason and RunRecords::gaming_flags (each task whose last try the screen flagged, with the reason). roko diagnose reads them: TaskDiagnosis::gaming_flagged in --json and \"completed; unverified: <reason>\" in place of \"it has no verify steps\", which a task without verify steps keeps; the text lists tasks completed unverified without --verbose. GET /api/runs/{id}/summary (and MCP run_status) gains gaming_flagged [{plan_id, task_id, attempt, reason}], documented in docs/v3/26-HTTP-API.md. roko plan run keeps each task's reason from its settled tries (GraphTaskDispatcher::note_gaming_flag/gaming_flags): its graph.plan_unverified event and the run's end print \"Task T is unverified: <reason>\", and --json gains gaming_flagged. Verify passes: diagnose_names_the_gaming_screens_reason; also gaming_flags_are_the_tasks_whose_last_try_was_flagged, a_runs_summary_says_why_the_gaming_screen_left_a_task_unverified, a_later_try_clears_the_gaming_screens_flag, a_plans_end_names_the_gaming_screens_reasons. roko-cli lib 4086 passed (gist test skipped), roko-learn lib 1412 and roko-serve lib 1090 passed; clippy -D warnings clean on the three crates. Follow-up gap-2f557f: roko run and plan run's undelivered-work line still say no check ran."
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
