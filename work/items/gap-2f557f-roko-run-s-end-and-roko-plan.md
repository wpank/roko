+++
id = "gap-2f557f"
kind = "gap"
title = "roko run's end and roko plan run's undelivered-work line still say no check ran on a task the gaming screen flagged"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/run"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-0af587 follow-up, 2026-10-27"
discovered_from = "gap-0af587"
anchors = ["crates/roko-cli/src/run.rs::print_workflow_run_report", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body"]
links = { depends_on = [], blocks = [], related = ["gap-0af587", "find-2ba86d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_flagged_one_task_run_says_why_it_is_unverified' crates/roko-cli/src && cargo test -p roko-cli --lib a_flagged_one_task_run_says_why_it_is_unverified"
+++

## Problem

find-2ba86d's gaming screen settles `unverified` an attempt whose checks ran and passed on a change that may pass
them without doing the task. gap-0af587 made `roko diagnose`, serve's run summary (`gaming_flagged`) and the end of
`roko plan run` say so, with the reason the attempt's verdict gives (its `pre_verify:gaming` step). Other reports
still treat `unverified` as "no check ran", which is false for such a task (and for bug-b7d9ad's tasks whose
checks changed the person's checkout, whose checks ran too):

- `roko run "<prompt>"` ends a flagged one-task run with "workflow unverified: no check ran on its work, so it was
  not delivered" (`crates/roko-cli/src/run.rs::print_workflow_run_report`, the
  `closed_run_status(..) == Some("unverified")` branch).
- `roko plan run` with a batch prints "Plan <id>: no check ran on the work of <tasks>, so it was not delivered"
  (`crates/roko-cli/src/graph_execution/plan_runner.rs`, the `plan_unverified_work` loop). gap-0af587's line after
  it names the screen's reason, so the two lines contradict each other.
- The log line "plan completed: UNVERIFIED (some tasks ran no verify step)" in `run_one_plan`.
- Doc comments that define unverified as "ran no verify step": `TaskVerdictCounts::unverified` (plan_runner.rs),
  `PlanOutcome::Unverified` (`graph_execution/plan_set.rs`), `AttemptOutcome::Unverified`
  (`crates/roko-learn/src/telemetry/records.rs`: "The task has no verify steps, so nothing checked the result").

## Why it matters

Goal `truth`: the screen's item asked roko to say why an attempt is unverified. `roko run` is the entry point for a
small change, so its one-line verdict is often all a person reads, and today it says something false for a flagged
task.

## Where

- `crates/roko-cli/src/run.rs::print_workflow_run_report`: reads the run's closed status from its manifest; the
  run's verdicts are in `.roko/runs/<run>/attempts.jsonl`, and
  `roko_learn::telemetry::report::RunRecords::gaming_flags` (gap-0af587) gives each flagged task's reason.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the undelivered-work `println!` in the run's end, and the
  `tracing::warn!` "plan completed: UNVERIFIED" in `run_one_plan`. `GraphTaskDispatcher::gaming_flags(plan_id)`
  (gap-0af587) has the reasons in process.
- The doc comments listed above.

## Current state

gap-0af587 (2026-10-27) added `AttemptVerdictRecord::gaming_reason`, `RunRecords::gaming_flags`,
`GraphTaskDispatcher::gaming_flags`, the notes `gaming_flagged_note` and `unverified_why` in plan_runner.rs, and
the wording in `roko diagnose`. Nothing else changed.

## Plan

1. `roko run`: when the run closed `unverified`, read its verdicts (`RunRecords::load(run_dir).gaming_flags()`)
   and say "workflow unverified: <reason>, so it was not delivered" for a flagged task; keep "no check ran on its
   work" for a task without verify steps.
2. `roko plan run`: word the undelivered-work line so it does not claim no check ran when the screen flagged one
   of its tasks (for example "the work of T1, T2 is unverified, so it was not delivered", with the reasons on the
   lines that follow), and the same for the log line.
3. Correct the doc comments.

## Done when

`a_flagged_one_task_run_says_why_it_is_unverified` passes: a `roko run` report for a run whose task's verdict
carries the gaming step names the step's reason and does not say that no check ran; a run whose task has no verify
steps keeps "no check ran on its work".

## Notes

Found while doing gap-0af587, kept out of it to keep that change to the places its item names.
