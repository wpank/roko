+++
id = "gap-6754ab"
kind = "gap"
title = "Attempt worktrees and other check checkouts build C++ projects cold: a JUCE plugin task's first check takes 38-62 s where the same check on a warm build tree takes 4-8 s"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-roko-speed#baseline"
discovered_from = "agent:roko-speed (round v7 baseline, 2026-10-19)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/git_ops.rs::give_worktree_dependency_dirs", "crates/roko-cli/src/orchestrator/worktree/mod.rs::WorktreeConfig", "crates/roko-core/src/config/gates.rs::GatesConfig"]
links = { depends_on = [], blocks = [], related = ["gap-7c3e13", "gap-2da1ff", "gap-e02ec4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn seeded_worktree_builds_nothing' crates/roko-cli/src && cargo test -p roko-cli --lib orchestrator::worktree::build_trees"
+++

## Problem

Every new checkout roko makes for a check starts without the project's build tree:
- an attempt's worktree (`.roko/worktrees/attempt-*`);
- the red-on-base base checkout;
- the delivery's regression checkout.

Its first check therefore configures and builds the whole project. On a C++/JUCE project that is 14-21 s of
`cmake --preset plugins` (juceaide is built at configure time) plus 22-36 s of build at -j4, for every plugin task.
A warm tree takes 3-4 s.

Measured on round v7's JUCE fixture (`tmp/hermes-roko-v7/roko-speed`, roko 3b34aed92, stand-in model), plugin tasks'
checks in fresh worktrees:
- T2: 38.1 s.
- T4: 61.7 s.
- T4's retry in the same worktree, now warm: 7.7 s.

The agents themselves ran for under a second each, so the cold builds were the whole critical path.

## Why it matters

Round v7 builds a VST3 synth from Hermes as many small C++ tasks. Each task's first check, and each delivery's first
regression, pays a full JUCE build. Under the round's limiter that also means minutes queued for a build slot. The
person waits through it, and every rung of the model ladder pays it again on a fresh checkout.

## Where

- `crates/roko-cli/src/orchestrator/worktree/git_ops.rs::give_worktree_dependency_dirs`: already gives a new
  worktree a copy-on-write clone of the checkout's `node_modules` (gap-2da1ff), via `clone_tree`. Build trees go
  here too.
- `crates/roko-cli/src/orchestrator/worktree/mod.rs::WorktreeConfig`: carries the setting from `[gates]`.
- `crates/roko-core/src/config/gates.rs::GatesConfig`: the new setting.

## Current state

- `node_modules` is cloned.
- Cargo target dirs are gap-7c3e13 (open): cargo's fingerprints need different handling.
- Nothing gives a CMake build tree. A plain clone does not work: CMake trees hold absolute paths, so CMake refuses a
  cache made for another directory.

A prototype (`tmp/hermes-roko-v7/roko-speed/bench/warm_proto.py`) shows what does work:
1. clonefile the tree;
2. rewrite the source checkout's absolute path to the new checkout's in the clone's text files, keeping each file's
   mtime;
3. give tracked files whose content is unchanged the source checkout's mtimes, so make treats them as unchanged;
4. for FetchContent/ExternalProject, drop the populate stamps' lines from `CMakeFiles/CMakeRuleHashes.txt`. CMake
   hashes each custom command's text, absolute paths included, and deletes the output of a rule whose hash changed,
   so the populate step would download and extract again, with new mtimes on every source.

Prototype results:
- JUCE outside the build tree: 0.56 s of prep, then `cmake --preset plugins` 2.6 s and a build of 1.1 s with nothing
  recompiled.
- FetchContent: 3.6 s of prep, 4.0 s configure, ~7 s build.
- Cold: ~50 s.

## Plan

1. A module beside `git_ops.rs` (`build_trees.rs`):
   - find the source checkout's warm build trees: git-ignored directories holding a `CMakeCache.txt` at most two
     levels down, or the directories `[gates] warm_build_dirs` names;
   - give each to a new checkout: clone, relocate, drop populate rule hashes, sync mtimes.
2. Never let a seeded tree point the checkout's build at the person's files. If the tree names a path under the
   source checkout that the new checkout lacks (an untracked dependency, `.git/index`, ...), drop that tree and
   build cold as today. Every failure is logged and non-fatal.
3. `[gates] warm_build_dirs`: unset means auto-detect CMake build trees, a list names the directories, `[]` turns
   it off.
4. Call it where `node_modules` is cloned (attempt worktrees). Expose it for the red-on-base base checkout
   (gap-e02ec4) and the regression checkout.
5. Tests:
   - a synthetic tree: paths relocated, mtimes kept, populate hashes dropped, unchanged sources synced, a tree that
     names an untracked path refused;
   - a real CMake project, when cmake is installed: after seeding, the new worktree's first build compiles
     nothing.

## Done when

- A new attempt worktree of a checkout with a built CMake tree builds nothing on its first `cmake --build`.
- The fixture plan's plugin checks drop from 38-62 s to single-digit seconds, with before/after numbers recorded in
  the closing evidence.
- `cargo test -p roko-cli --lib orchestrator::worktree::build_trees` passes.

## Notes

- The source checkout is the person's: read only. Nothing is ever written there.
- A seed that is older than the new checkout's sources is fine: changed files keep their new mtimes and rebuild.
- Round v7 evidence: `tmp/hermes-roko-v7/findings/roko-speed.md`.
