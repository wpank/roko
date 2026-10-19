+++
id = "bug-40bf59"
kind = "bug"
title = "gap-6754ab's warm build tree tests assume a copy-on-write filesystem: on CI's ubuntu ext4, cp --reflink=always fails, no tree is given, and six tests fail"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "S"
subsystem = ["roko-cli/worktree"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-roko-speed#gap-e02ec4"
discovered_from = "agent:roko-speed (round v7, while writing gap-e02ec4, 2026-10-19)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/git_ops.rs::clone_tree", "crates/roko-cli/src/orchestrator/worktree/build_trees.rs", "crates/roko-cli/src/graph_execution/workspaces.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-6754ab", "gap-e02ec4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn clones_on' crates/roko-cli/src && cargo test -p roko-cli --lib -- warm_build_tree seeded_worktree_builds_nothing acquire_gives_the_checkout_warm_build_trees"
+++

## Problem

The warm build tree tests from gap-6754ab (merged in 22667b513) pass only on a filesystem that makes
copy-on-write clones.

`give_build_trees` clones a build tree with the platform's `cp`: `cp -cRp` (clonefile) on macOS, and
`cp -a --reflink=always` on Linux. On a filesystem that cannot clone, `clone_tree` returns false. No tree is given,
and the checkout builds cold, which is the intended fallback. The tests, though, assert that a tree was given.

CI runs `cargo test --workspace` on `ubuntu-latest`, whose disk is ext4, and ext4 has no reflink. These tests would
fail there:
- `orchestrator::worktree::build_trees::tests`:
  - `warm_build_tree_is_relocated_with_its_times`;
  - `warm_build_tree_drops_only_populate_rule_hashes`;
  - `warm_build_tree_gives_unchanged_files_their_times`;
  - `warm_build_tree_is_refused_when_it_cannot_move`, in its `named.len() == 1` assertion;
  - `seeded_worktree_builds_nothing`, where cmake is installed.
- `graph_execution::workspaces::tests::acquire_gives_the_checkout_warm_build_trees`.

Found while writing gap-e02ec4's seeding test, which has the same dependency. It was not seen in a run: CI has not run
on these commits yet.

## Why it matters

A red CI blocks whatever branch carries these commits, release PRs included. The production fallback is right; only
the tests assume a clone.

## Where

- `crates/roko-cli/src/orchestrator/worktree/build_trees.rs`: the tests module.
- `crates/roko-cli/src/graph_execution/workspaces.rs`: `acquire_gives_the_checkout_warm_build_trees`.

## Plan

1. Add `clones_on(dir)`, a test-only probe that tries `clone_tree` on an empty directory in the test's temp dir.
2. Each clone-dependent test returns early, saying so, when the filesystem cannot clone. The refusal test keeps its
   other assertions and skips only the one that needs a clone.

## Done when

The tests pass on APFS as before and skip on a filesystem without clones. Checked by the command in `[[verify]]`.

## Notes

- A Linux runner on Btrfs or XFS with reflink would run them in full.
