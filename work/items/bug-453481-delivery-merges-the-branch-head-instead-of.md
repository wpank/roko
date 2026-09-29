+++
id = "bug-453481"
kind = "bug"
title = "Delivery merges the branch head instead of the verified commit_oid, so later commits land unverified"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-merge-safety's report on bug-a3760a)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::git_merge", "crates/roko-graph/src/delivery.rs::CompletionDeliveryRequest"]
lane = "rust-cold"
parent = "spec-a0e40a"
links = { depends_on = ["bug-a3760a"], blocks = [], related = ["spec-f830c4", "gap-415c54"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn merge_takes_the_verified_commit_not_the_branch_head' crates/roko-cli/src/ && cargo test -p roko-cli --lib merge_takes_the_verified_commit_not_the_branch_head"
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
