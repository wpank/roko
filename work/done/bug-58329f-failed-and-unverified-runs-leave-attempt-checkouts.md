+++
id = "bug-58329f"
kind = "bug"
title = "Failed and unverified runs leave attempt checkouts behind, and a run that did nothing leaves an empty worktree, attempt branch and batch branch"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/orchestrator", "roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "320c9998f"
source = "tmp/hermes-roko-v5/findings/fresh-guide.md items 12, 25, 29 (fresh-guide, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: a newcomer's first session"
anchors = ["crates/roko-cli/src/orchestrator/worktree/cleanup.rs", "crates/roko-cli/src/orchestrator/worktree/mod.rs::WorktreeManager", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan"]
links = { depends_on = [], blocks = [], related = ["bug-06bd18", "bug-74164c", "gap-f67a72", "gap-415c54"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_run_that_did_nothing_leaves_no_checkout_or_branch' crates/roko-cli/ && cargo test -p roko-cli --lib a_run_that_did_nothing_leaves_no_checkout_or_branch"

[[verify]]
command = "grep -rqw 'fn an_unverified_plan_releases_its_accepted_checkout' crates/roko-cli/ && cargo test -p roko-cli --lib an_unverified_plan_releases_its_accepted_checkout"

[closed]
at = 2026-10-16
at_ts = "2026-10-15T23:13:43Z"
commit = "320c9998f"
by = "commit trailer"
executor = "unknown"
forced = false
evidence = "320c9998f: at a plan's end (any outcome) release_plan_checkouts removes its accepted attempts' checkouts (release_accepted) and its no-work ones (WorktreeManager::remove_idle_attempts: clean and still at their base commit, branch deleted by CAS while at that commit); a run that delivered and accepted nothing removes the batch branch it created (BatchIntegration::discard_if_unused); failed checkouts with work stay (bug-06bd18 untouched); events graph.attempt_checkouts_removed and graph.batch_discarded; --json batch.discarded. Verify: a_run_that_did_nothing_leaves_no_checkout_or_branch and an_unverified_plan_releases_its_accepted_checkout pass; full roko-cli lib 3650 passed (6 ignored, gist skipped) and roko-serve lib 1025 passed with a scratch HOME; clippy -D warnings clean; integration graph_timeout_matrix (7), plan_branch_integration (4), operator_checkout_clean, honest_verdicts_canary, attempt_diff_canary pass (scratch HOME); worktree_task_diff's fails-verify test updated for the removed batch branch and still fails on bug-ee2752. Live: an offline run (HTTPS_PROXY blocked, $0) left no checkout, attempt branch or batch branch (worktrees 8 and roko/* branches 31 before and after); an unverified run ($0.004) had its accepted checkout removed and kept its work on roko/plan/<run>."
+++

## Problem

After a newcomer's first session (fresh-guide, 2026-10-15, roko-main 7b6a5af44) the project held, besides its own
branches, eight `roko/*` branches and four attempt checkouts under `.roko/worktrees/`, two still checked out:

- `attempt-8acaf668…` of run `1d158449`, which failed at dispatch before any agent ran: a clean checkout at the base
  commit, on a branch at the base commit, plus `roko/batch/1d158449-…` at the base. None of the three holds anything.
- `attempt-1f14ec16…` (run `c2dd59a7`, unverified) and `attempt-5b479755…` (run `4363bc38`, T1 passed): clean
  checkouts of accepted attempts, whose commits are on their plan branches.
- `attempt-28efa40e…` (run `4363bc38`, T2 failed): uncommitted changes, kept for post-mortem.

Only a delivered plan's accepted checkouts are removed today (gap-415c54); every other checkout stays until
`roko doctor disk --fix` removes it, after 7 days and only once its run ended for good (gap-f67a72), or a later run of
the same plan sets it aside (bug-06bd18). So every failed or unverified run leaves checkouts behind, and a run that
never got going leaves a branch and a worktree that hold nothing.

Expected, by the retention rules roko already states (a checkout is kept while it holds something to inspect or
resume; work on a branch needs no checkout):

1. When a plan ends, however it ends, the checkouts of the attempts it accepted are removed, as a delivered plan's
   are: their commits are on the plan branch. Their attempt branches stay, unless `[runner] delete_attempt_branches`.
2. A failed attempt's checkout that holds no work (no changes, no commits since it was made) is removed with its
   branch when its plan ends; there is nothing to inspect or resume.
3. A run that delivered nothing and accepted nothing removes its batch branch when it ends.
4. A failed attempt's checkout that holds work is kept, as now: for post-mortem, for a resume to continue it
   (bug-056b40), set aside by a new run (bug-06bd18), and removed by `roko doctor disk --fix` after 7 days.

## Why it matters

Leftovers clutter the person's repository and `git worktree list`/`git branch`, cost disk, and make roko look
careless (goal `hermes`).

## Where

- `crates/roko-cli/src/orchestrator/worktree/mod.rs`: `WorktreeManager::release_accepted` (delivered plans only),
  the attempt checkouts it tracks.
- `crates/roko-cli/src/orchestrator/worktree/cleanup.rs`: the retention rules (`remove_leftover_checkouts`).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan` and `deliver_plan_to_batch`: where a plan ends
  and a delivered plan's checkouts are released.
- `crates/roko-cli/src/graph_execution/batch.rs`: the batch branch, created at the run's start.

## Plan

1. `WorktreeManager` remembers which attempt checkouts it made for each plan.
2. At a plan's end, release its accepted checkouts whatever its outcome, and remove its other checkouts that hold no
   work (clean, at the commit they were made from), with their branches when those are still at that commit.
3. At the run's end, delete the batch branch (compare-and-swap at its base) when no plan delivered anything and no
   plan accepted anything.
4. Say what was removed in the run's events and its `--json` report.

## Done when

The `[[verify]]` test passes: a run whose task fails at once without changing anything leaves no attempt checkout,
no attempt branch and no batch branch; a run whose task fails with changes keeps that checkout; an unverified plan's
accepted checkout is removed and its work stays on its plan branch.

## Notes

- Do not undo bug-06bd18 (kept checkouts of another run are set aside, never deleted).
- A checkout with uncommitted work is never removed by this.
