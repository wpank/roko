+++
id = "bug-453481"
kind = "bug"
title = "Delivery merges the branch head instead of the verified commit_oid, so later commits land unverified"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-merge-safety's report on bug-a3760a)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::git_merge", "crates/roko-graph/src/delivery.rs::CompletionDeliveryRequest"]
lane = "rust-cold"
parent = "spec-a0e40a"
links = { depends_on = ["bug-a3760a"], blocks = [], related = ["spec-f830c4", "gap-415c54"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn merge_takes_the_verified_commit_not_the_branch_head' crates/roko-cli/src/ && cargo test -p roko-cli --lib merge_takes_the_verified_commit_not_the_branch_head"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in d5192d4f0. Delivery merges the verified commit_oid, never the branch head; a bad or rewritten id merges nothing. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

`GitDeliveryBackend::git_merge` resolves `request.branch` to its current head and merges that commit. It never compares the head with `request.commit_oid`, the commit the plan's gates accepted (documented as "Accepted merge commit OID"). Any commit added to the branch after verification gets merged into the target unverified, and the receipt still names the verified commit. Such a commit might come from a late agent write, a retry or a manual commit.

## Why it matters

Delivery must merge only what the gates verified. That guarantee is the point of the integration step (epic spec-a0e40a), and verdicts are only honest if it holds. The bug is latent today, because nothing outside `delivery.rs` builds a `CompletionDeliveryRequest`. Fix it before step 4 of spec-f830c4 wires delivery into `plan run`.

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs::git_merge`: `let theirs = resolve_commit(workdir, branch)`. The already-merged check, the fast-forward and `commit-tree -p` all use `theirs`.
- `crates/roko-graph/src/delivery.rs::CompletionDeliveryRequest`: the `commit_oid` field, which is part of the request fingerprint.

## Current state

On `work/bug-a3760a` (`809ae920d`), `commit_oid` is used in only two places: in the fingerprint, and as the fallback commit for the regression (`receipt.merge_commit.unwrap_or(&receipt.request.commit_oid)`). At BASE the merge ran `git merge <branch>`, which has the same flaw.

## Plan

1. In `git_merge`, resolve `request.commit_oid` to a commit. If it doesn't resolve, fail closed.
2. Require the commit to be on the branch (`is_ancestor(commit_oid, <branch head>)`). If the branch was rewritten, fail closed.
3. Merge `commit_oid`, not the branch head. If the branch has moved past it, say so in the summary: the later commits wait for their own verified delivery.
4. Add `merge_takes_the_verified_commit_not_the_branch_head`, using a temp repo as `merge_leaves_the_user_checkout_alone` does. Record `commit_oid`, then commit again on the plan branch and deliver. Check that the target's second parent is `commit_oid` and that the later commit is not reachable from the target.

## Done when

- [ ] A delivery never merges a commit that was added to the branch after `commit_oid`.
- [ ] If `commit_oid` is missing or not on the branch, the merge fails and no ref moves.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on bug-a3760a, which rewrote `git_merge`. It is not on BASE yet.
- 2026-09-30 (wk-integrate): Implemented on `work/bug-453481` at `55a97258a`; cargo verification deferred to the batch check.
  - In the worktree: `cargo check -p roko-cli -p roko-graph --lib --tests` and `cargo clippy -p roko-cli -p roko-graph -p roko-execution --no-deps -D warnings` clean; nightly rustfmt clean. `cargo test -p roko-cli --lib graph_execution::delivery`: 26 passed (27 with bug-aaa924).
  - `GitDeliveryBackend::git_merge` merges `request.commit_oid`, never the branch head. The id must be a commit id (hex; a ref name is refused), must resolve, and must be on the branch (`merge-base --is-ancestor <oid> <head>`); otherwise nothing is merged and no ref moves. When the branch moved past it, the summary names the head and says its later commits wait for a verified delivery of their own. The merge commit's message names the commit.
  - The existing real-git delivery tests now deliver the plan head's id (`git_request`); they passed a fake `abc123`.
  - Tests: `merge_takes_the_verified_commit_not_the_branch_head`, `merge_fails_closed_without_the_verified_commit_on_the_branch`.
