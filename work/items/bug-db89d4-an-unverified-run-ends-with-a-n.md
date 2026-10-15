+++
id = "bug-db89d4"
kind = "bug"
title = "An unverified run ends with a 'N failed' engine error, and nothing says where its unchecked work is"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/graph_execution", "roko-serve/runs"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "tmp/hermes-roko-v5/findings/fresh-guide.md items 15, 20 (fresh-guide, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: a newcomer's first session"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-cli/src/graph_execution/batch.rs", "crates/roko-serve/src/routes/run_result.rs::run_result"]
links = { depends_on = [], blocks = [], related = ["bug-1d09e8", "gap-8409d0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_unverified_run_says_where_its_unchecked_work_is' crates/roko-cli/ && cargo test -p roko-cli --lib an_unverified_run_says_where_its_unchecked_work_is"

[[verify]]
command = "grep -rqw 'fn run_summary_names_unverified_work' crates/roko-serve/ && cargo test -p roko-serve --lib run_summary_names_unverified_work"
+++

## Problem

A newcomer's chat run (`c2dd59a7`, fresh-guide, 2026-10-15, roko-main 7b6a5af44) in a workspace with no gate rungs
did the right work: one commit `d0dc746` (+1 line in `wordy/cli.py`, +8 in `tests/test_cli.py`; the tests pass).
Its one task ran no verify step, so the plan ended `unverified`, as it should. But:

- The run's last event is the error `Graph Engine: 1 plan(s), 0 succeeded, 1 failed`. The closing tally counts every
  plan that did not succeed as failed. Hosts show it as a failure row next to the `unverified` verdict, and Hermes'
  model concluded "treat this run as not passing".
- Nothing says where the work is. It is not on the batch branch: `roko/batch/c2dd59a7-…` is still at its base,
  because only a plan that succeeded is delivered. The commit is only on `roko/plan/c2dd59a7-…` and an attempt
  branch. The summary's `result` names the batch branch with no files, no diff and no merge command, so the plugin's
  Changes tab says "No changes to review" (fresh-guide items 15 and 20). The CLI prints only "Plans that did not
  succeed: <plan>".

Expected: the run reads as `unverified` once, with no failure row, and the summary, `result` and the CLI say where the
unchecked work is (its branch and commit, the files) and how to see it, marked as not checked by any gate. Unverified
work is still never delivered into the batch branch or merged.

## Why it matters

The first good result a newcomer gets reads as a failure, and the work is invisible unless they know roko's branch
names (goals `hermes`, `truth`). Measured trust: a run with no checks never counts as a success, but it must not be
reported as a failure either.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body`: the closing `DashboardEvent::Error`
  ("Graph Engine: {plan_count} plan(s), {} succeeded, {} failed"), fed by `plan_outcomes` (a bool per plan), and the
  CLI's closing lines ("Plans that did not succeed").
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan`: where a plan's outcome is settled and a
  succeeded plan is delivered (`deliver_plan_to_batch`).
- `crates/roko-cli/src/graph_execution/batch.rs`: the batch record a plan's checkpoint keeps under `roko.batch@1`.
- `crates/roko-serve/src/routes/run_result.rs::run_result`: the summary's `result` (gap-8409d0), which reads that
  record and git.

## Current state

Attempts are accepted onto `roko/plan/<plan>` (gap-3b5361) whatever their verdict, as long as it is replayable
(`passed`, `unverified`, …); each accepted commit carries `Roko-Plan`, `Roko-Task`, `Roko-Run` and `Roko-Verdict`
trailers. Only `deliver_plan_to_batch` (succeeded plans) writes the `roko.batch@1` record.

## Plan

1. Count the closing tally by outcome (succeeded, unverified, failed, blocked, cancelled, interrupted) and publish
   it as an error only when a plan failed; otherwise as a log line.
2. When a plan under a batch does not succeed, read what this run accepted onto its plan branch (the commits whose
   trailers name the plan and run, by first parent) and record it in the plan's `roko.batch@1` record: the tasks
   whose work reached the batch, and the work that ran no check (its commit, the commit it starts from, its tasks).
3. The summary's `result` gains `unverified`: for each such plan, its branch, commit, base, files, diffstat and the
   command that shows it. `GET /api/runs/{id}/diff?work=unverified` returns that diff.
4. The CLI's closing lines name each plan's outcome and, for unverified work, the branch and the diff command.

## Done when

The `[[verify]]` tests pass: an unverified run's closing tally is not an error, its plan record names the unchecked
commit, and its summary `result.unverified` names the plan branch and files with a review command.

## Notes

- Unverified work is not delivered into the batch branch and gets no merge command in `result.merge_command`.
- bug-1d09e8 (roko-checks) covers a cancelled run's task outcomes and wording; the closing tally belongs here.
