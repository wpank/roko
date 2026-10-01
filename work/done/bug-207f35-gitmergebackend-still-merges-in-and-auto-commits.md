+++
id = "bug-207f35"
kind = "bug"
title = "GitMergeBackend still merges in, and auto-commits, the checkout it is given"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/runner"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-merge-safety's report on bug-a3760a)"
anchors = ["crates/roko-cli/src/runner/merge.rs::GitMergeBackend", "crates/roko-cli/src/runner/merge.rs::default_merge_backend", "crates/roko-cli/tests/merge_proof.rs", "crates/roko-cli/tests/runner_integration.rs"]
lane = "rust-cold"
parent = "spec-a0e40a"
links = { depends_on = [], blocks = [], related = ["bug-a3760a", "spec-f830c4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'auto-commit before merge' crates/roko-cli/src/ && ! grep -qF '\"merge\", \"--no-ff\"' crates/roko-cli/src/runner/merge.rs"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in d5192d4f0. GitMergeBackend and CargoCheckRegressionGate are deleted; PlanMerger fails closed without injected backends. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: static checks pass on MAIN."
+++

## Problem

`GitMergeBackend::merge` in `crates/roko-cli/src/runner/merge.rs` works directly in the checkout it is given (`PlanMergerConfig::workdir`). If `git status --porcelain` shows changes, it runs `git add -A` and `git commit --allow-empty -m "chore: auto-commit before merge"`. It then runs `git merge --ff-only <branch>`, or `git merge --no-ff --no-edit <branch>`, in that same checkout. Pointed at the operator's checkout, it commits their uncommitted work and moves their branch. bug-a3760a removed this behaviour from `GitDeliveryBackend` but left this backend as it was.

## Why it matters

No production code calls it today. But it is the default backend of `PlanMerger` (`default_merge_backend`), and step 3 of spec-f830c4's plan proposed routing merges through `PlanMerger`. The first caller to wire it would bring bug-a3760a back in a worse form. Epic spec-a0e40a.

## Where

- `crates/roko-cli/src/runner/merge.rs`:
  - `GitMergeBackend::merge`: the "G02" auto-commit block, then the "G05" fast-forward or `--no-ff` merge.
  - `PlanMerger::default_merge_backend`, and `PlanMerger::prepare`, which falls back to it.
- Callers: `crates/roko-cli/tests/merge_proof.rs`, `crates/roko-cli/tests/runner_integration.rs` and the tests in `merge.rs`. There are no others.

## Current state

Checked on `work/bug-a3760a` (`809ae920d`). `git grep 'GitMergeBackend\|default_merge_backend\|PlanMerger::new'` finds no caller in `crates/*/src` outside `merge.rs`. Since that commit, `delivery.rs` uses `git_merge_tree`, `git_output` and `git_command` from the same file, so those three must stay.

## Plan

Design choice:

- **Option A (recommended):** delete `GitMergeBackend` together with its tests. Also delete `PlanMerger` and `CargoCheckRegressionGate` if nothing else needs them. `GitDeliveryBackend` already merges with plumbing and runs the regression in a temporary worktree. Two merge paths is the duplication CLAUDE.md warns about.
- **Option B:** port `GitMergeBackend` to the plumbing that `GitDeliveryBackend::git_merge` uses: `git_merge_tree`, `commit-tree` and a compare-and-swap `update-ref`. It would then need no auto-commit and no checkout.

## Done when

- [ ] No code path in `crates/roko-cli/src` stages, commits or merges in a checkout it was handed.
- [ ] The `[[verify]]` command passes.

## Notes

- Keep `git_merge_tree`, `git_output` and `git_command`, which delivery uses.
- Run the tests' git commands in temp repos only, never in the repository itself.
- 2026-09-30 (wk-integrate): Implemented on `work/bug-453481` at `ace875c56`; cargo verification deferred to the batch check.
  - In the worktree: `cargo check -p roko-cli -p roko-graph --lib --tests` and `cargo clippy -p roko-cli -p roko-graph -p roko-execution --no-deps -D warnings` clean; nightly rustfmt clean. `cargo test -p roko-cli --lib runner::merge`: 14 passed; `--test runner_integration --test merge_proof`: 6 + 6 passed.
  - Option A: deleted `GitMergeBackend` (the auto-commit and `git merge` in the checkout it was given) and `CargoCheckRegressionGate` (a `cargo check` in that same checkout), the defaults `default_merge_backend` and `default_regression_gate`, the helpers only they used (`git_success`, `git_conflicted_paths`) and `GitMergeBackend`'s three tests. `git_command`, `git_output`, `git_merge_tree` and `merge_tree_result` stay.
  - `PlanMerger` stays: `tests/runner_integration.rs` and `tests/merge_proof.rs` still drive it, with injected backends. With no built-in backend, a merger missing either fails the merge closed (`merge_without_backends_fails_closed`). No production code calls it; roko merges with `GitDeliveryBackend`.
