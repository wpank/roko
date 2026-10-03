+++
id = "bug-045773"
kind = "bug"
title = "A reattached worktree's base_commit is hardcoded to None instead of read back from the original attach"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/orchestrator-worktree"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK03 gap-d254a3)"
discovered_from = "gap-d254a3"
anchors = ["crates/roko-cli/src/orchestrator/worktree/mod.rs::WorktreeHandle", "crates/roko-cli/src/graph_execution/workspaces.rs::lease_from_handle"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn reattached_handle_keeps_its_original_base_commit' crates/roko-cli/ && cargo test -p roko-cli reattached_handle_keeps_its_original_base_commit"
+++

## Problem

A re-attached worktree checkout's `base_commit` is hardcoded to `None` instead of being read back from the
original attach. `crates/roko-cli/src/orchestrator/worktree/mod.rs:1602`, inside the function that reconstructs
a `WorktreeHandle` for an existing worktree directory (reading its path and mtime from disk), sets
`base_commit: None` unconditionally. Compare the fresh-claim path at line 738: `base_commit:
Some(claim.marker.target_oid.clone())` — populated from the claim marker's recorded target commit. Nothing reads
that same marker (or an equivalent recorded value) back on reattach.

`crates/roko-cli/src/graph_execution/workspaces.rs::lease_from_handle`'s own comment explains why this matters:
"The commit the checkout started from, so the attempt's diff leaves out what siblings landed before it; the
configured base only when that is unknown, as for a re-attached checkout (backlog 1124)." So when `base_commit`
is `None` (every reattach, per this bug), `lease_from_handle` falls back to `self.manager.base_branch().to_string()`
— the base BRANCH's name, not the specific commit the checkout actually started from. A branch name can have
moved since the checkout was first created, so this fallback doesn't give the same point-in-time guarantee the
comment describes for a reattached checkout specifically.

## Why it matters

A reattached attempt's diff may fail to exclude work siblings landed between the original attach and the
reattach, exactly the scenario `lease_from_handle`'s own comment says `base_commit` exists to prevent — but only
for the reattach case, which is the one path that currently can't supply it.

## Where

- `crates/roko-cli/src/orchestrator/worktree/mod.rs:1602` (reattach reconstruction, `base_commit: None`) vs. `:738`
  (fresh claim, correctly populated from `claim.marker.target_oid`).
- `crates/roko-cli/src/graph_execution/workspaces.rs::lease_from_handle` (the fallback this bug triggers).

## Current state

Unfixed. The claim marker (or an equivalent small record) that carries `target_oid` on fresh attach is not read
back when reconstructing a handle for an existing worktree.

## Plan

1. Persist the original `target_oid` somewhere the reattach path can read it back (the claim marker file itself,
   if it survives in the worktree directory, or a small sidecar file written alongside the worktree).
2. Populate `base_commit` from that recorded value instead of `None` in the reattach reconstruction.
3. Regression test: create a worktree (fresh claim), simulate a reattach, and confirm the reconstructed handle's
   `base_commit` matches the original `target_oid`, not `None`.

## Done when

- A reattached worktree's `WorktreeHandle.base_commit` matches what the original claim recorded, not `None`.
- `lease_from_handle` no longer falls back to the base branch name for a reattached checkout when the original
  commit is actually known.
- The `[[verify]]` command passes.

## Notes

- Backlog task 1124 is the origin of the "configured base only when unknown, as for a re-attached checkout" rule
  this bug defeats — check that task's text for any constraint on how the recorded value should be persisted.
