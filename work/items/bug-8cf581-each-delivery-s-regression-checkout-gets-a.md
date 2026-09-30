+++
id = "bug-8cf581"
kind = "bug"
title = "Each delivery's regression checkout gets a new temporary path, so workspace crates rebuild every delivery and leave stale artifacts"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_execution/delivery"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-integrate's report, checked on work/bug-453481 at 2eeda438d)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = ["bug-453481"], blocks = [], related = ["bug-453481"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn regression_checkouts_reuse_a_stable_path' crates/roko-cli/src/ && cargo test -p roko-cli --lib regression_checkouts_reuse_a_stable_path"
+++

## Problem

On bug-453481's branch, delivery runs its regression command in a temporary detached checkout (`graph_execution/delivery.rs:255-264`, `.tempdir()`). Every delivery gets a new path. Cargo keys workspace crates by path, so each delivery rebuilds every workspace crate from scratch, and leaves another set of stale artifacts in the shared target directory (`with_regression_target_dir`, :156).

## Why it matters

Integration and a whole-plan check (epic spec-a0e40a): deliveries are slow, and the target directory grows with every delivery.

## Where

`regression_output` and the checkout creation in `delivery.rs`.

## Plan

1. Use a stable per-repository checkout path (for example under `.roko/state/regression-checkout/`), reset to the commit under test for each delivery. Serialize its use.
2. Add `regression_checkouts_reuse_a_stable_path`.

## Done when

- [ ] Consecutive deliveries reuse one checkout path, and the target directory doesn't grow per delivery.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-integrate): Implemented on `work/spec-f830c4` at `3584ddee4`; cargo verification deferred to the batch check.
  - Every delivery's regression runs in `.roko/state/regression-checkout`, reset to the merge commit (`checkout --detach --force`, then `clean -ffd`). An exclusive lock on `regression-checkout.lock` serializes it across processes. Anything else at that path is left alone, and the delivery fails saying so.
  - Tests: `regression_checkouts_reuse_a_stable_path`; `regression_runs_in_a_separate_checkout_of_the_merge` (was `…_temporary_…`) and `batch_branch_merges_plan_in_temp_worktree` now expect that path.
  - The target dir's growth was not measured; the stable path is what lets Cargo reuse the previous build.
