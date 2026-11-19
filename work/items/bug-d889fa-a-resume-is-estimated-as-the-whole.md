+++
id = "bug-d889fa"
kind = "bug"
title = "A resume is estimated as the whole plan and says nothing of its cap already spent: phase 6's resume card read \"estimated $2.02 … spending up to $5.00\" with T1 and T2 kept and $3.00 of the run's cap gone, so it had $2.00"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_estimate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "c39fc94bc"
source = "round v8 hermes-pilot-v8 and run-analyst-v8: phase 6 run 23837ede (Undertow 8945b3c, roko e722f0e1f); filed by roko-planner-v8 at the lead's word"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/plan_estimate.rs::estimate_with", "crates/roko-serve/src/routes/plans/reads.rs::plan_estimate"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-6605f7", "bug-d97e01"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_resumes_estimate_counts_kept_work_and_spend' crates/ && cargo test -p roko-cli --lib a_resumes_estimate_counts_kept_work_and_spend"
+++

## Problem

roko estimates a resume as the whole plan, and it gives a client nothing to tell the approver how much of the cap
the run has already spent.

- **What the cards said.** Phase 6 (run 23837ede) first stopped at $3.0048, with T1 and T2 passed and kept.
  - The resume's card said "Resume roko plan p6-the-window (6 tasks in 3 waves; estimated $2.02, high confidence),
    spending up to $5.00".
  - The second resume's card said "estimated $2.06 … spending up to $9.00".
- **A resume's cap counts the whole run.** The checkpoint's spend carries into the budget ledger, so the first resume
  had $2.00 to spend, not $5.00. It stopped at "$4.99 of $5.00 · $3.00 spent before it resumed".
- **The estimate ignores kept work.** `plan_estimate::estimate_with` takes the tasks still to run from their `status`
  in tasks.toml. A Graph run records a finished task in its checkpoint and leaves the status in tasks.toml as written.
  So the resume was priced with T1 and T2 again: $2.02, against the first card's $2.03.

## Why it matters

- The card is where a person approves money. "Spending up to $5.00" for a run that can spend $2.00 more misleads both
  ways: the approver thinks they gave $5, and the run stops sooner than they expect.
- The plugin's headline after the stop does say "$3.00 spent before it resumed", but only after the fact.
- Goal: core.

## Where

- `crates/roko-cli/src/plan_estimate.rs::estimate_with`: which tasks are left.
- `crates/roko-serve/src/routes/plans/reads.rs::plan_estimate`: what a client reads before approving.
- The card's words are the Hermes plugin's (`hermes_plugin_roko/tools.py`, `_plan_run`), which is plugin-wow-v8's
  part. roko's part is to give the plugin the fields below.

## Plan

1. When the plan has a resumable run, price only the tasks its checkpoint has not completed. Take this from the
   checkpoint, not from the status in tasks.toml.
2. Report the resumable run's spend so far, and the room left under a given cap, beside the estimate (for example
   `spent_before_usd`). A client can then say "$2.00 more: $3.00 of $5.00 already spent".
3. Add the test `a_resumes_estimate_counts_kept_work_and_spend`. A plan whose checkpoint has T1 and T2 completed and
   $3.00 spent is estimated over T3 to T6 only, and its estimate reports the $3.00.

## Done when

- The `[[verify]]` command passes.

## Notes

- Evidence: hermes-pilot-v8's findings ("Phase 6, the window": the rerun's and resume 2's cards); its evidence
  `p6-rerun-status.json` and `p6-resume2-status*.json`.
- Related: gap-6605f7 (the estimator), bug-d97e01 (phase 6's estimate miss).
