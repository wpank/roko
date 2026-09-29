+++
id = "bug-a3760a"
kind = "bug"
title = "The Graph engine's merge step runs git checkout in the user's working tree"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/cybernetic-harness/assessment-2026-09-28/process-capacity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/process-capacity.md"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::GitDeliveryBackend::git_merge", "crates/roko-cli/src/graph_execution/delivery.rs::GitDeliveryBackend", "crates/roko-cli/src/graph_execution/delivery.rs::CliCompletionDeliveryService", "crates/roko-cli/src/runner/merge.rs::GitMergeBackend"]
links = { depends_on = [], blocks = [], related = ["gap-a85a1f", "gap-3b5361"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn merge_leaves_the_user_checkout_alone' crates/roko-cli/src/graph_execution/ && cargo test -p roko-cli --lib merge_leaves_the_user_checkout_alone && ! grep -qE '\"(checkout|switch)\", *&?target' crates/roko-cli/src/graph_execution/delivery.rs"
+++

## Problem

`GitDeliveryBackend::git_merge` (`crates/roko-cli/src/graph_execution/delivery.rs`, lines ~115-215) merges a task
or plan branch into its target by running these commands with `current_dir` set to the workspace, which is the
operator's own checkout:

- `git checkout <target>`;
- `git merge --ff-only <branch>`;
- then `git merge --no-ff --no-edit <branch>`;
- `git merge --abort` if that fails.

When it runs, it:
- switches the branch of the user's working tree;
- fails when the user has uncommitted changes that conflict, or carries them onto the target branch;
- races with anything else using that tree: an editor, another roko session, a second plan.

After the merge, `run_regression` runs `cargo check --workspace` in that same user tree.

Expected: delivering a branch never changes the user's checked-out branch, index or files.

## Why it matters

- Goal `core`. Merge-back is part of the plan loop (plan, dispatch, gate, merge, resume).
- The defect is latent today:
  - Neither `GitDeliveryBackend` nor `CliCompletionDeliveryService` is constructed anywhere outside
    `delivery.rs`. Only `graph_execution/mod.rs` re-exports them.
  - Per-task worktrees are never merged back (`plan_runner.rs` ~lines 840-845 refuse
    `--worktree-per-task` with parallel plans for that reason).
- It becomes live as soon as someone wires delivery. That is the serialized-merge part of `gap-d58ae8`, and the
  accept/merge part of `gap-3b5361`. It has to be fixed first, or that wiring will damage users' checkouts.
- Related:
  - `gap-d58ae8`: Mori-shaped workflow contract, merge-back of worktrees.
  - `gap-3b5361`: `accept_attempt` has no production caller.
  - `gap-a85a1f` (parked): keeping agents from running `git checkout` inside plan worktrees.

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs`:
  - `GitDeliveryBackend::git_merge` (line ~115): the checkout-and-merge in `self.workdir`.
  - `impl DeliveryBackend for GitDeliveryBackend`:
    - `merge` (line ~220) calls `self.merge_queue.enqueue(..)` and ignores the result, so it does not wait for its
      turn. Serialization is not enforced.
    - `run_regression` runs `cargo check --workspace` in `self.workdir`.
    - `publish` pushes `<merge_commit>:refs/heads/<branch>`.
  - `CliCompletionDeliveryService` (line ~343) drives `Prepared -> Queued -> Merged -> RegressionPassed ->
    Published -> Delivered` and writes the `roko.delivery@1` checkpoint extension.
  - `mod tests` (line ~648) uses only fake backends. Nothing tests `GitDeliveryBackend` against a real repo.
- `crates/roko-cli/src/runner/merge.rs::GitMergeBackend` (lines ~240-420): the older merge backend.
  - It has a reusable `git merge-tree --write-tree HEAD <branch>` conflict pre-check (the "G06" comment,
    lines ~275-325).
  - It also merges in the user's tree. Worse, it auto-commits the user's dirty state first (`git add -A` plus
    `commit -m "chore: auto-commit before merge"`, lines ~329-355).
  - Only tests use it (`tests/merge_proof.rs`, `tests/runner_integration.rs`, through `PlanMerger::new`). Do not
    use it as the fix.
- `crates/roko-cli/src/orchestrator/worktree/mod.rs`: the worktree manager (`accept_attempt`, attempt worktrees).
  Use it if the regression step needs a temporary checkout.

## Current state

- `delivery.rs` last changed in `244f564e1` (the backlog sweep). The checkout is still at lines ~116-121 as of
  `d9e79e9d8`.
- No production caller, so no user has hit this through `roko plan run`.
- The local git here is 2.39.5. `git merge-tree --write-tree` needs 2.38 or later.
- The current `[[verify]]` (`! grep -q '"checkout", target' delivery.rs`) only checks that one line is gone. A
  `["switch", target]` or a renamed variable would pass it while the bug stays.

## Plan

1. Merge without touching any working tree, using git plumbing and running `git` with `current_dir` at the repo:
   1. Resolve the refs: `old = rev-parse refs/heads/<target>` and `theirs = rev-parse <branch>`.
   2. If `merge-base --is-ancestor <old> <theirs>`, then `new = theirs` (fast-forward).
   3. Otherwise run `merge-tree --write-tree <old> <theirs>`.
      - Exit 1 means conflicts: return `merged: false`, with the conflicted paths in the summary.
      - Exit 0 prints the tree: `new = commit-tree <tree> -p <old> -p <theirs> -m "Merge branch '<branch>' into
        <target>"`.
   4. Move the ref with a compare-and-swap: `update-ref refs/heads/<target> <new> <old>`. If the target moved in
      the meantime, this fails. Report that as not merged. Never force it.

   Pull the `merge-tree` invocation and its conflict parsing out of `runner/merge.rs` into one shared helper
   instead of writing a second copy.
2. Never move a branch that is checked out.
   - If `git worktree list --porcelain` shows `refs/heads/<target>` checked out in any worktree, including the
     user's, moving the ref would leave that tree's files and index out of step with its HEAD.
   - Recommended: in that case, write the result to `refs/roko/delivered/<plan_id>` and return a clear summary
     telling the user to run `git merge --ff-only refs/roko/delivered/<plan_id>`. Do not update the branch.
   - Alternative: fail closed with the same message and no ref at all. Simpler, but it loses the prepared merge.
3. Run the regression in a checkout that is not the user's.
   - `git worktree add --detach <tmp> <new>` under `.roko/worktrees/`, then `cargo check --workspace` there, then
     `git worktree remove`.
   - Use the orchestrator worktree manager if it already provides this.
4. Make `merge` wait for its merge-queue slot, or drop the queue call. As written, `enqueue` is fire-and-forget.
   The compare-and-swap in step 1 is the actual race guard.
5. Tests: `#[tokio::test]`s in `delivery.rs` against a temp repo.
   - `merge_leaves_the_user_checkout_alone`:
     - The user is on branch `work` with an uncommitted edit; the target is `main`, not checked out.
     - After `merge`, HEAD is still `work`, the edit is still there, `git status` is unchanged, and `main` equals
       the returned `merge_commit`.
   - `merge_into_checked_out_target_does_not_move_it`: step 2's rule.
   - `merge_conflict_is_reported_without_touching_refs`.

## Done when

- `GitDeliveryBackend::merge` never runs `checkout`, `switch`, `merge` or `reset` in the workspace.
- A merge into a branch that is not checked out advances it atomically. A merge into a checked-out branch leaves
  it alone and reports where the result is.
- A conflict reports the paths and changes no refs.
- The regression step runs outside the user's tree.
- Verify:
  `grep -rqw 'fn merge_leaves_the_user_checkout_alone' crates/roko-cli/src/graph_execution/ && cargo test -p roko-cli --lib merge_leaves_the_user_checkout_alone && ! grep -qE '"(checkout|switch)", *&?target' crates/roko-cli/src/graph_execution/delivery.rs`

## Notes

- Risky area: git refs in users' repositories. Only ever update refs with the old-value argument
  (compare-and-swap). Never `--force`. Never touch the index or working tree of any existing worktree.
- Do not wire `CliCompletionDeliveryService` into `plan run` in this item. That wiring belongs to `gap-d58ae8`
  and `gap-3b5361`, and should come after this fix.
- The same hazard exists in `runner/merge.rs::GitMergeBackend`, which auto-commits the user's dirty tree. It is
  only used by tests today. Either leave it alone or file a separate item to delete it. Do not route the new code
  through it.
- The repo rule stands: never delete worktrees or plan branches that roko did not create for this merge. Remove
  only the temporary regression worktree.
- Safe to do in parallel with most work. It touches only `delivery.rs`, plus a small helper shared with
  `runner/merge.rs`.

## Original notes

`git_merge` merges a task branch by running `git checkout <target>` with `current_dir` set to the workspace (`graph_execution/delivery.rs:116-119`). That switches the branch of the user's own working tree. It fails on, or carries over, uncommitted changes, and it races with anything else using that tree: an editor, another session, a second plan.

Fix: merge in a dedicated worktree or with git plumbing (`git merge-tree` plus `update-ref`), so the user's checkout is never touched.

Re-verified 2026-09-29 at d9e79e9d8: git_merge still checks out the target branch in the workspace (delivery.rs:116-121). GitDeliveryBackend is not constructed anywhere in the workspace, so no Graph run reaches this merge step today (per-task worktrees are never merged back, plan_runner.rs:840-845). The defect is latent and must be fixed before GitDeliveryBackend is wired, for example for the serialized-merge part of gap-d58ae8.
