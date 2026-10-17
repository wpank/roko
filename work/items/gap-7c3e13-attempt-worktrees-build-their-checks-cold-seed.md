+++
id = "gap-7c3e13"
kind = "gap"
title = "Attempt worktrees build their checks cold: seed each one's target dir with a copy-on-write clone of a warm one, without making a changed crate fresh"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-be2a53 worker 2026-10-17: the item's plan offered a warm APFS clone per checkout; the fix dropped the shared dir instead"
discovered_from = "work:bug-be2a53"
anchors = ["crates/roko-cli/src/orchestrator/worktree/git_ops.rs::give_worktree_dependency_dirs"]
links = { depends_on = [], blocks = [], related = ["bug-be2a53"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn attempt_worktrees_start_with_a_warm_target' crates/roko-cli/src/orchestrator/ && cargo test -p roko-cli --lib attempt_worktrees_start_with_a_warm_target"
+++

## Problem

Every attempt worktree (`.roko/worktrees/<attempt>`, the default since gap-4ec59f) builds its Rust checks from
scratch in its own `target/`, third-party crates included. Nothing shares a target dir with it now (bug-be2a53),
and by default nothing did before. For a large workspace that costs minutes per attempt before the first check
result, and again on every retry in a fresh worktree.

## Why it matters

Check latency and the cost of retries. bug-be2a53's plan named the alternative to dropping a shared target dir: a
target dir per checkout, seeded from a warm one by an APFS clone so it starts fast. That fix took the simpler
route and left this one open.

## Where

`crates/roko-cli/src/orchestrator/worktree/git_ops.rs::give_worktree_dependency_dirs`. It already gives a new
worktree a copy-on-write clone of `node_modules` (`clone_tree`).

## Current state

Each attempt worktree starts with no `target/`. The delivery's regression check has a stable target dir of its own
(`.roko/state/regression-target`, bug-be2a53), so it is warm after the first delivery.

## Plan

1. When the file system can clone and there is room, clone a warm target (the workdir's own `target/`, or a
   configured one) into the new worktree's `target/` with `clone_tree`.
2. Keep it safe. A clone is safe only when every workspace source in the worktree is newer than every fingerprint
   in the clone. Either clone before the worktree's files are written, or drop the clone's fingerprints of
   workspace members (`.fingerprint/<member>-*`). Cargo then rebuilds the workspace's own crates from the worktree's
   sources and reuses only third-party builds.
3. Pin with a test that a clone never makes a changed crate fresh.

## Done when

- [ ] A new attempt worktree's first check reuses third-party builds and never a workspace crate built from other
  sources.
