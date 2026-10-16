+++
id = "gap-2da1ff"
kind = "gap"
title = "A task's git worktree has no node_modules (or in-project venv), so a Node project's tests fail there before the agent changes anything"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "ddd0d074f"
source = "roko-init round v5, 2026-10-15 (tmp/hermes-roko-v5/findings/roko-init.md)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/git_ops.rs::isolate_worktree_config"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b", "gap-1625c0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn worktree_gets_the_checkouts_node_modules' crates/roko-cli/ && cargo test -p roko-cli --lib worktree_gets_the_checkouts_node_modules"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T09:22:43Z"
commit = "ddd0d074f"
by = "roko-init"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-16T08:07:41Z"
forced = false
evidence = "ddd0d074f (on main 19c468604). A new worktree gets a copy-on-write clone of each git-ignored node_modules of the checkout, nested ones too: clonefile via cp -c on APFS, cp --reflink=always on Linux, only on the checkout's device. Never a link and never a full copy. Absolute symlinks into the checkout are repointed into the worktree, a clone the worktree's git does not ignore is removed, and virtualenvs are not cloned (documented in docs/v3/04-EXECUTION.md). Checks at ddd0d074f: worktree_gets_the_checkouts_node_modules (the [[verify]] command) passes; it asserts the clone, that editing it leaves the checkout unchanged, the relative and repointed absolute links, and a clean worktree status. roko-cli lib 3705 / bin 466 tests pass; all 73 integration targets ran, failing only on known items (bug-2a31bc x2, bug-c63cca item_03, bug-ee2752); clippy -D warnings and fmt clean. Live, a scratch Node project whose test needs node_modules (local file: dependency), plain 'npm test' rung, scratch HOME: before, npm test in a git worktree of HEAD fails 'Cannot find module localdep'. After, run-20261016-084014-926: roko logged 'gave the worktree a copy-on-write clone ... dir=node_modules elapsed_ms=42', and verify:0/test ran in the attempt worktree and passed (exit 0, 439 ms), gpt-oss-120b, $0.0172. The operator checkout was unchanged and the attempt committed no node_modules path. Evidence: tmp/hermes-roko-v5/findings/roko-init.md"
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
