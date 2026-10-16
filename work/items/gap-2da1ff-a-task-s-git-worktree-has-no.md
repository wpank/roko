+++
id = "gap-2da1ff"
kind = "gap"
title = "A task's git worktree has no node_modules (or in-project venv), so a Node project's tests fail there before the agent changes anything"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "roko-init round v5, 2026-10-15 (tmp/hermes-roko-v5/findings/roko-init.md)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/git_ops.rs::isolate_worktree_config"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b", "gap-1625c0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn worktree_gets_the_checkouts_node_modules' crates/roko-cli/ && cargo test -p roko-cli --lib worktree_gets_the_checkouts_node_modules"
+++

## Problem

A task of a plan run, and a `roko run` prompt run, works in its own git worktree of the project
(`[runner] worktree_per_task`, on by default). The worktree holds only tracked files, so the dependencies a
project installs into its checkout are not there: `node_modules`, and a virtualenv kept inside the project but not
activated. The agent's own test runs and the gate rungs then fail on the unchanged project.

Repro (2026-10-15, roko-init): a Node package whose test `require`s a devDependency
(`$V/roko-init/after/node-dep`). In the checkout `npm test` passes (`# pass 1`); in `git worktree add <dir> HEAD`
it fails with `Error: Cannot find module 'localdep'` (`# fail 1`). That is the tree a task's agent and its
`[[gates.rungs]]` run in.

## Why it matters

A Node project's checks fail on every task before the agent changes anything, so runs fail or burn retries on a
failure nobody caused. `roko init` now works around it for its own Node rungs (bug-4e336b / gap-1625c0 branch:
`(test -d node_modules || npm ci …) && npm test`), but the agent still starts without the dependencies, and a
hand-written rung has no such guard.

## Where

- `crates/roko-cli/src/orchestrator/worktree/git_ops.rs::isolate_worktree_config`: what a new worktree gets
  besides the checkout (today only config directories such as `.cursor/`).
- `crates/roko-cli/src/orchestrator/worktree/mod.rs`: worktree creation.

## Current state

No code links or copies dependency directories into a task's worktree. Rust (`target/` rebuilt), Go (global
module cache) and an activated virtualenv (absolute path on `PATH`) work; `node_modules` and an unactivated
in-project `.venv` do not.

## Plan

1. When the checkout has a `node_modules/` that git ignores, at any depth, give a new worktree a copy-on-write
   clone of it: `clonefile` on APFS (`cp -c`), a reflink on Linux (`cp --reflink=always`).
   - Never a symlink or a hardlink: both let an agent's in-place edit reach the operator's checkout. A symlink
     is also not ignored by `node_modules/`, so roko would commit it.
   - Only on the checkout's device, and never a full copy: where no clone can be made, the worktree gets none,
     as today.
   - An absolute symlink in the clone that leads into the checkout is pointed at the worktree's copy.
   - The worktree's git must ignore the clone too, or it is removed.
2. Do not clone a virtualenv. Its scripts name its absolute path, so a clone would still install into the
   operator's. Document that roko needs it activated.
3. Test: a worktree of a package with ignored dependencies gets an independent clone of them (nested ones too),
   and its status stays clean.

## Done when

`worktree_gets_the_checkouts_node_modules` passes (the `[[verify]]` command).

## Notes

- 2026-10-16 (roko-init): fixed as planned above, in `orchestrator/worktree/git_ops.rs`
  (`give_worktree_dependency_dirs`, called from `isolate_worktree_config` for every new worktree).

Mind disk use: a clone is cheap on APFS, a copy is not. Do not write into the operator's checkout.
