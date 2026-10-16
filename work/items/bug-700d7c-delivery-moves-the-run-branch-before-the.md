+++
id = "bug-700d7c"
kind = "bug"
title = "Delivery moves the run branch before the whole-plan check and resets it on failure; the papers say it moves only once the check passes, and parallel plans can start from the unchecked merge"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-0212ef worker, 2026-10-16"
discovered_from = "gap-0212ef"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::git_merge", "crates/roko-cli/src/graph_execution/batch.rs::deliver", "crates/roko-cli/src/orchestrator/worktree/mod.rs::attempt_base"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-0212ef"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn delivery_moves_the_run_branch_only_after_its_check_passes' crates/roko-cli/ && cargo test -p roko-cli delivery_moves_the_run_branch_only_after_its_check_passes"
+++

## Problem

Delivery moves the run branch before the whole-plan check runs, and moves it back when the check fails.
`GitDeliveryBackend::git_merge` (`crates/roko-cli/src/graph_execution/delivery.rs:368`) moves
`roko/batch/<run-id>` to the merge commit with a compare-and-swap `update-ref` in the delivery's `Queued` step;
the `Merged` step then runs the plan's whole-plan check on that commit in the regression checkout
(`run_regression`); and `BatchIntegration::deliver` (`crates/roko-cli/src/graph_execution/batch.rs:288-316`) resets
the branch to its old tip when the check fails or is cancelled ("roko: undo a delivery whose regression check did
not pass").

Both papers describe the other order: "the run branch moves only once it passes" (`docs/whitepaper/04-orchestration.md:70-71`)
and "nothing runs after the branch moves" (research paper §7.3, `tmp/cybernetic-harness/paper/sections/07-plan-loop.md:60`).

While the check runs, the run branch points at an unchecked merge. A plan's attempts take their base from the run
branch by name until the plan has an accepted attempt (`WorktreeManager::attempt_base`,
`crates/roko-cli/src/orchestrator/worktree/mod.rs:922`, falls back to `WorktreeConfig::base_branch`, which the plan
runner sets to the batch branch). With `[conductor] max_parallel_plans` above 1 (the default is 1; the `thorough`
preset in `crates/roko-core/src/config/presets.rs` sets 4), a plan running beside a delivering one can start an
attempt from that unchecked merge. If the check then fails and the run branch is reset, the failed plan's commits
stay in the other plan's history and return to the run branch when that plan is delivered, checked only by the
second plan's own whole-plan check.

## Why it matters

Goal `golden-path`: the whole-plan check is what lets the operator trust the run branch, so a plan whose check
failed must never reach it, directly or through another plan. Decision 2 of 2026-10-15 keeps the papers as written,
so the code has to match them. Found while fixing gap-0212ef, whose README wording ("the run branch keeps a plan only
if that check passes") holds under either order.

## Where

- `delivery.rs::git_merge`: computes the merge with `merge-tree` and `commit-tree`, then moves the target before any
  check runs.
- `delivery.rs::advance_delivery`: `Queued` calls the backend's `merge`, `Merged` calls `run_regression`.
- `batch.rs::deliver`: undoes the merge after a failed or cancelled check.
- `orchestrator/worktree/mod.rs::attempt_base`: an attempt in a plan with no accepted commit starts from the batch
  branch's current tip.
- Entry point: `roko run plans/<dir>` or `roko plan run`, through `graph_execution/plan_runner.rs::deliver_plan_to_batch`.

## Current state

Found by reading the code at `5e32264ce`; the parallel-plan leak is inferred from that reading and not reproduced.
`meta_verify_failure_fails_a_plan_whose_tasks_passed` (`plan_runner.rs`) asserts that the batch is back at `HEAD`
after a failed check, which holds under both orders, so no test pins the order.

## Plan

1. Separate computing the merge from moving the target: the backend's `merge` returns the merge commit (recorded on
   the receipt as now) without moving the target, `run_regression` checks that commit, and the target moves by the
   same compare-and-swap (against the tip the merge was computed on) only after the check passes. A failed or
   cancelled check then leaves the run branch where it was.
2. Keep resume working: a receipt in `Merged` reruns its check; one whose check passed but whose target did not move
   yet moves it. Keep the parked-merge path for a checked-out target as it is.
3. Drop the undo in `BatchIntegration::deliver`, or keep it only for receipts written by older runs.
4. Add `delivery_moves_the_run_branch_only_after_its_check_passes`: a whole-plan check step that records
   `git rev-parse roko/batch/<run>` while it runs; assert it equals the tip before the delivery, and that the branch
   is at the merge after a passing check.

## Done when

The `[[verify]]` command passes: the new test exists and passes, and the existing delivery and batch tests still do.

## Notes

- Don't edit the papers (decision 2 of 2026-10-15).
- `BatchIntegration::promote` (moving a named branch after every delivery) is out of scope.
