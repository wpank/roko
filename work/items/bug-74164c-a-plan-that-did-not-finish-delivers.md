+++
id = "bug-74164c"
kind = "bug"
title = "A plan that did not finish delivers none of its verified tasks' work: the batch branch stays at its base and the passed tasks' commits sit on branches nothing names"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/graph_execution", "roko-serve/runs"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "tmp/hermes-roko-v5/findings/fresh-guide.md item 29 (fresh-guide, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: a newcomer's first session"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan", "crates/roko-cli/src/graph_execution/batch.rs::BatchIntegration", "crates/roko-serve/src/routes/run_result.rs::run_result"]
links = { depends_on = [], blocks = [], related = ["bug-1d09e8", "bug-db89d4", "gap-8409d0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_plan_that_did_not_finish_delivers_its_verified_tasks' crates/roko-cli/ && cargo test -p roko-cli --lib a_plan_that_did_not_finish_delivers_its_verified_tasks"

[[verify]]
command = "grep -rqw 'fn run_summary_says_which_work_is_partial' crates/roko-serve/ && cargo test -p roko-serve --lib run_summary_says_which_work_is_partial"
+++

## Problem

A newcomer's plan-first run (`4363bc38`, plan `add-two-flags-to-the-wordy-cli-chars-which-count`, fresh-guide,
2026-10-15, roko-main 7b6a5af44) had two tasks: T1 passed its check (8.5 s), T2 failed four times. The run failed,
as it should. But the verified half is lost to the person:

- `roko/batch/4363bc38-…` still points at the base `43016ac`; the summary's `result` lists no files, no diff and no
  merge command, and the plugin's Changes tab says "No changes to review. This run left no verified work on its
  branch."
- T1's commit `97319be` (`Roko-Verdict: passed`) exists only on `roko/plan/add-two-flags-…` and an attempt branch,
  which nothing names.

The same happens to a cancelled or interrupted plan whose earlier tasks passed (bug-1d09e8, step 4).

Expected: a plan that did not succeed still delivers the work of its verified tasks into the run's batch branch,
marked partial in the run's summary, `result` and CLI output, so the person can review it and take it or leave it.
Only verified work is delivered (never a task that ran no check, nor one built on such work), and nothing is merged
into the person's checkout.

## Why it matters

The person paid for T1 and it passed its check; throwing it away, or hiding it behind roko's branch names, wastes the
work and the trust (goals `hermes`, `truth`). A host can only offer what roko's summary names.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan`: delivers only `Some(batch) if
  outcome.succeeded()` (`deliver_plan_to_batch`).
- `crates/roko-cli/src/graph_execution/batch.rs`: `PlanWork` (bug-db89d4) reads what a run accepted onto a plan
  branch, with each commit's verdict; `BatchIntegration::request`/`deliver`.
- `crates/roko-cli/src/graph_execution/delivery.rs`: `GitDeliveryBackend` (a delivery with no regression steps only
  merges).
- `crates/roko-serve/src/routes/run_result.rs::run_result`: the summary's `result`.

## Current state

bug-db89d4 (merged, 40019913d) records, for a plan that was not delivered, the work no check ran on (`roko.batch@1` `work.unverified`).
Acceptance (gap-3b5361) puts each passed or unverified attempt on `roko/plan/<plan>`, with `Roko-Verdict` trailers.

## Plan

1. When a plan under a batch ended without succeeding, before any whole-plan check ran (its tasks did not all pass),
   deliver the verified prefix of what this run accepted onto its plan branch (`PlanWork::verified_tip`) into the
   batch: a plumbing merge only, with no whole-plan check (the plan did not finish, so its whole-plan check does not
   apply), recorded as partial. A plan whose tasks all passed but whose whole-plan check failed delivers nothing, as
   now.
2. Record in the plan's `roko.batch@1` record which tasks were delivered and which were not, with why.
3. The summary's `result` says `partial: true` and, per plan, what was and was not delivered; the CLI's closing
   lines say "partially delivered".
4. Remove the delivered attempts' checkouts as a full delivery does.

## Done when

The `[[verify]]` tests pass: a two-task plan whose T1 passes and T2 fails ends failed, with T1's work (only) on the
batch branch, a partial record in its checkpoint, and `result.partial` plus the per-plan breakdown in its summary.

## Notes

- Measured trust: a task with no checks is never delivered; partial work is reviewed, never merged silently.
- bug-1d09e8 (roko-checks) owns the cancel wording and task counts; this item delivers a cancelled plan's verified
  tasks too.
