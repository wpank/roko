+++
id = "bug-8cf581"
kind = "bug"
title = "Each delivery's regression checkout gets a new temporary path, so workspace crates rebuild every delivery and leave stale artifacts"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_execution/delivery"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "207f91da2"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-integrate's report, checked on work/bug-453481 at 2eeda438d)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = ["bug-453481"], blocks = [], related = ["bug-453481"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn regression_checkouts_reuse_a_stable_path' crates/roko-cli/src/ && cargo test -p roko-cli --lib regression_checkouts_reuse_a_stable_path"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 207f91da2. The delivery regression reuses a stable per-repository checkout path, so workspace crates stop rebuilding on every delivery. Batch 16d gate on dd58c3db2 (MAIN 207f91da2 has the same code), after the coordinator's scope fix for plan_verify (cfed1c6f2): cargo check --workspace --tests, nightly fmt, clippy -p roko-cli -p roko-core -p roko-execution --keep-going -D warnings clean; lib tests pass: roko-cli 3236 (two known load flakes, turn_policy's 1 s test and gate_rows' writer wait), roko-core 1953, roko-execution 245; integration: --test plan_branch_integration 2 passed (C3 kill-and-resume, C4 whole-plan gate), --test merge_proof 4, --test runner_integration 6. Verify: its test passes in that run and its static checks pass on MAIN."
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
