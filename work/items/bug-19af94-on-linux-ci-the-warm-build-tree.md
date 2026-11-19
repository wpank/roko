+++
id = "bug-19af94"
kind = "bug"
title = "On Linux CI the warm build tree tests run where no tree can be given: clones_on probes an empty directory, which cp --reflink=always copies on any filesystem, so the nine tests ran on ext4, where give_build_trees clones nothing and every checkout builds cold"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "S"
subsystem = ["roko-cli/worktree"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 lead: release PR #88's CI run 43183197388 (Test + Clippy, ubuntu-latest), for #89"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/build_trees.rs::give_build_trees", "crates/roko-cli/src/orchestrator/worktree/git_ops.rs::clone_tree"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-40bf59", "gap-6754ab", "gap-e02ec4", "gap-dc24dd"], supersedes = [], duplicate_of = "" }
+++

## Problem

Release PR #88's CI (run 43183197388, job "Test + Clippy" on `ubuntu-latest`) failed nine roko-cli lib tests that pass
on macOS. All nine are about warm build trees (gap-6754ab, gap-e02ec4, gap-dc24dd, f177be7e9):

- `orchestrator::worktree::build_trees::tests`:
  - `warm_build_tree_is_relocated_with_its_times`;
  - `warm_build_tree_drops_only_populate_rule_hashes`;
  - `warm_build_tree_gives_unchanged_files_their_times`;
  - `warm_build_tree_is_refused_when_it_cannot_move`;
  - `seeded_worktree_builds_nothing`.
- `graph_task_dispatch::attempt_workspace::tests::a_dependent_attempt_starts_from_its_dependency_s_build_trees`.
- `graph_execution::workspaces::tests::acquire_gives_the_checkout_warm_build_trees`.
- `graph_execution::delivery::tests::the_regression_checkout_starts_with_the_workdirs_build_trees`.
- `spec_red_on_base::tests::warm_seeds_the_base_from_the_workspace_build`.

Each found no tree given ("left: 0, right: 1", "the checkout got the build tree: No such file or directory",
`grep: build/CMakeCache.txt: No such file or directory`).

`give_build_trees` gives a checkout a tree only by a copy-on-write clone (`cp -a --reflink=always` on Linux), and
the ext4 of CI's runners can't make one. bug-40bf59 meant those tests to skip there: each first asks `clones_on`.
That probe clones an empty directory, though, and `cp --reflink=always` copies an empty directory on any
filesystem. It only fails at the first file ("failed to clone 'full-copy/f' from 'full/f': Operation not supported",
seen in a Linux container on overlayfs, 2026-11-19). So on ext4 the probe said yes, the tests ran, and no tree was
given.

On a Linux filesystem without reflinks (ext4, the default, and overlayfs in containers), no checkout ever gets a
warm tree: every attempt's worktree, base checkout and regression checkout builds the project cold.

## Why it matters

Goal core. A red CI blocks the release PR (#89). On Linux, the warm build trees that cut a JUCE check from about 50 s
to 4 s do nothing on the most common filesystem.

## Where

- `crates/roko-cli/src/orchestrator/worktree/build_trees.rs`: `give_tree` (clone or nothing) and the test-only
  `clones_on`.
- `crates/roko-cli/src/orchestrator/worktree/git_ops.rs::clone_tree`.
- The `clones_on` guards in `graph_execution/workspaces.rs`, `graph_execution/delivery.rs`,
  `graph_task_dispatch/attempt_workspace.rs` and `spec_red_on_base.rs`.

## Plan

1. Where the filesystem can't clone, `give_tree` copies a tree small enough that copying it costs less than a cold
   build, keeping its files' times (`cp -pPR`). A larger tree still builds cold, as before.
2. The tests run on every filesystem, by a clone or by a copy, and no longer skip. `clones_on` goes.
3. New tests for both sides of the copy limit, run without clones on any filesystem.
4. Proof: the nine tests in a Linux container on overlayfs, red before and green after.

## Done when

The nine tests pass on Linux without reflinks and on APFS, a small tree is copied where no clone can be made, a large
one builds cold, and the `[[verify]]` passes.
