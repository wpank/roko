+++
id = "bug-fbefe0"
kind = "bug"
title = "roko-serve and roko-cli rebuild on every cargo command in a worktree: their build scripts watch files that don't exist there"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-serve/build", "roko-cli/build"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "coordinator, batch-15a gate: `cargo check -v` reports roko-serve dirty because apps/portal/out/index.html is missing (2026-09-30)"
anchors = ["crates/roko-serve/build.rs", "crates/roko-cli/build.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'rerun-if-changed=../../apps/portal/out/index.html\");$' crates/roko-serve/build.rs && ! grep -q 'rerun-if-changed=../../.git/HEAD' crates/roko-cli/build.rs"
+++

## Problem

Cargo treats a `rerun-if-changed` path that does not exist as always stale, so the build script reruns on every build and the crate and everything that depends on it rebuilds.

- `crates/roko-serve/build.rs` emits `cargo:rerun-if-changed=../../apps/portal/out/index.html`, so that the build notices when the portal export appears. The export is a gitignored build product, so it is missing in every fresh checkout and git worktree. `cargo check -v` in a worktree reports `Dirty roko-serve: the file crates/roko-serve/../../apps/portal/out/index.html is missing`. roko-serve then rebuilds on every cargo command, and so do roko-cli, roko-acp and every other crate that depends on it.
- `crates/roko-cli/build.rs` emits `rerun-if-changed=../../.git/HEAD`, `../../.git/refs/` and (since gap-8cb382) `../../.git/index`. In a git worktree `.git` is a file, not a directory, so all three paths are missing.

## Why it matters

Every batch gate, every worker build in its own worktree, and every roko plan run that builds this repo in a worktree (`--worktree-per-task`, self-hosting) recompiles roko-serve and roko-cli each time. A standalone rerun of one roko-cli test took 3–5 minutes instead of seconds, and parallel worker builds drove the machine's load past 200. p1 because it multiplies the cost of self-hosting.

## Where

`crates/roko-serve/build.rs` (the portal watch) and `crates/roko-cli/build.rs` (the git watches).

## Current state

Both scripts emit paths that are missing in worktrees.

## Plan

1. **roko-serve:** emit the portal `rerun-if-changed` only when `apps/portal/out/index.html` exists. When it is missing, keep the fallback, and say in the build.rs comment and the portal README that exporting the portal needs a rebuild of roko-serve (for example, `touch crates/roko-serve/build.rs`). Don't watch `apps/portal` as a directory: it holds node_modules. Check the other demo-app paths the same way.
2. **roko-cli:** resolve the real git dirs with `git rev-parse --git-dir` and `--git-common-dir`. Emit `rerun-if-changed` on `<git-dir>/HEAD`, `<git-dir>/index` and `<common-dir>/refs/` only when they exist, and none when git is unavailable.
3. **Prove it:** in a git worktree, run `cargo check -p roko-cli --lib` twice; the second run must report `Fresh roko-serve` and `Fresh roko-cli` under `-v`. Record the timings in the Notes.

## Done when

- [ ] A second `cargo check -p roko-cli` in a worktree rebuilds nothing.
- [ ] The `[[verify]]` command passes.
