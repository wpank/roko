+++
id = "gap-184cba"
kind = "gap"
title = "A warm build tree keeps objects compiled with a definition that names the person's checkout: relocate rewrites flags.make but keeps its mtime, so those objects never rebuild in the worktree (Undertow's UNDERTOW_SOURCE_DIR in phase 4.1's rehearsal p41): rebuild exactly them after the clone"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 phase 4.1 rehearsal p41, the lead's follow-up to bug-b7d9ad"
discovered_from = "agent:roko-ladder (round v7 rehearsal p41)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/build_trees.rs::relocate", "crates/roko-cli/src/orchestrator/worktree/build_trees.rs::write_keeping_times"]
links = { depends_on = [], blocks = [], related = ["bug-b7d9ad", "gap-6754ab"], supersedes = [], duplicate_of = "" }
+++

## Problem

A warm build tree roko clones from the person's checkout into an attempt's worktree (gap-6754ab) keeps the
compile-time paths of what it already built. `relocate` rewrites every absolute path in the tree's text files to the
worktree's, including a target's `flags.make`, but gives each file back its mtime (`write_keeping_times`), so make
rebuilds nothing. That is the point for include paths: an unchanged header is the same file in both checkouts. It is
wrong for a definition whose value is the checkout's path. The objects compiled with it keep the person's checkout
baked in.

Round v7's phase 4.1 rehearsal (p41, `tmp/hermes-roko-v7/roko-ladder/rehearsals/p41/`, run `20261021-120220`):
Undertow's spec target has `UNDERTOW_SOURCE_DIR="${PROJECT_SOURCE_DIR}"` (`tests/spec/CMakeLists.txt:67`). T2's
check `tools/spec.sh 4 budget`, run in its worktree with 2 warm trees, wrote the review PNGs through that path into the
base checkout (`ws/`: `M artifacts/ui/editor-{idle,playing,clipping}.png`). T2's worktree held nothing and settled
`already_satisfied`, and the delivery had no PNGs.

bug-b7d9ad detects this after the fact: it compares the person's checkout before and after an attempt's checks, warns
on the hub, and settles such an attempt `unverified`. Undertow now chooses its artifact directory at run time
(Undertow ceac70e). This item is prevention: the objects whose definitions name the checkout should not survive the
clone.

## Where

- `crates/roko-cli/src/orchestrator/worktree/build_trees.rs::relocate` and `write_keeping_times`.

## Plan

- When relocating changes a line that sets a target's definitions (`C_DEFINES`, `CXX_DEFINES`, ... in a Makefile
  generator's `flags.make`), write that file with a fresh mtime. make then rebuilds that target's objects, and only
  those, with the worktree's path.
- Ninja keeps every command line in `build.ninja` and rebuilds an edge whose command changed, so check what a
  relocated Ninja tree rebuilds before relying on it there.
- A test with real CMake, as `seeded_worktree_builds_nothing` does: a target with
  `target_compile_definitions(x PRIVATE SRC_DIR="${PROJECT_SOURCE_DIR}")`, seeded into a worktree, rebuilds x's
  objects and prints the worktree's path, while a target without such a definition rebuilds nothing.

## Why not in bug-b7d9ad

It is not trivial. Generators differ: a Makefile tree's `flags.make` versus Ninja's one `build.ninja`. A fresh
`flags.make` rebuilds every object of its target. And it needs real-CMake tests per generator. The lead asked for it
as a follow-up.

## Done when

An object built with a definition that names the source checkout is rebuilt in a seeded worktree, so a check run there
writes under the worktree, and a tree without such a definition still rebuilds nothing. Checked by the tests in
`[[verify]]`.
