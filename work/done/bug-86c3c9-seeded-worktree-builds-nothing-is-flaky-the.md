+++
id = "bug-86c3c9"
kind = "bug"
title = "seeded_worktree_builds_nothing is flaky: the whole test can run inside one second, and macOS's make 3.81 compares whole seconds, so the changed source can look no newer than its object"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/worktree"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "e5ba3bb4a"
source = "round-v7-roko-speed#gap-e02ec4-post-verify"
discovered_from = "agent:roko-speed (round v7, post-merge verify of gap-e02ec4, 2026-10-19)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/build_trees.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-6754ab", "bug-40bf59"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'compares whole seconds' crates/roko-cli/src/orchestrator/worktree/build_trees.rs && cargo test -p roko-cli --lib seeded_worktree_builds_nothing"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T19:56:07Z"
commit = "1d02d3d65"
forced = false
evidence = "The test dates its edited source 2 s after the object file, so make's whole-second times see the change. Before the fix, the test failed 2 of 5 runs on the merged tree at 97d9b1819. After it, 5 of 5 passed in the gap-cb9044 chain."
+++

## Problem

`orchestrator::worktree::build_trees::tests::seeded_worktree_builds_nothing` (gap-6754ab) fails now and then with "a
changed file did not rebuild: [100%] Built target warm".

The test builds a tiny CMake project in a repository, gives a new checkout a warm copy of the build tree, builds there,
writes a changed `src/b.c`, and expects the next build to recompile `b.c.o`. All of it can happen within one second.
make 3.81, the one macOS ships, compares modification times in whole seconds. When the edit falls in the same second
as the build that made `b.c.o`, make sees nothing newer and rebuilds nothing.

Seen on 2026-10-19 in the post-merge verify of gap-e02ec4 at 97d9b1819. The same test passed in that branch's own run
an hour earlier.

## Why it matters

A flaky test reads as a regression and trains people to rerun until green. The code under test is right: in real use
an edit comes seconds or minutes after the build.

## Where

- `crates/roko-cli/src/orchestrator/worktree/build_trees.rs`: the test `seeded_worktree_builds_nothing`.

## Plan

After writing the change, date `src/b.c` two seconds after the object file `b.c.o`, which keeps the test deterministic
under whole-second timestamps.

## Done when

The test dates its change after the build, and passes. Checked by the command in `[[verify]]`.

## Notes

- Found by roko-speed in a post-merge verify; fixed on `work/gap-cb9044`.
