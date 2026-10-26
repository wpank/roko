+++
id = "bug-e81b32"
kind = "bug"
title = "work.py next and claim rewrite other worktrees' git index: the worktree scan runs a plain git status there, which re-runs roko-cli's build.rs and costs each worker a roko-cli rebuild"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["tools/work"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "worker on bug-2a31bc/bug-a66fef/bug-8d8f98, 2026-10-26: an unexplained roko-cli rebuild traced to an outside index rewrite"
discovered_from = "bug-2a31bc's CI-like verify re-run (2026-10-26)"
anchors = ["tools/work.py::worktree_changes", "tools/test_work.py"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = ["gap-d1f787"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_worktree_scan_leaves_other_worktrees_index_alone' tools/test_work.py && python3 tools/test_work.py -k test_worktree_scan_leaves_other_worktrees_index_alone"
+++

## Problem

`tools/work.py`'s `worktree_changes()` reads every other worktree of the repo. `work.py next` (through
`pick_next`) and `work.py claim` both call it. For each worktree it runs
`git -C <path> status --porcelain -z --no-renames`. A plain `git status` refreshes the index and writes it back
whenever it can take the lock (git-status(1), "BACKGROUND REFRESH"). So the scan rewrites other worktrees' `index`
files, although gap-d1f787, which added the scan, required "Read-only git only; never write in another worktree".

Reproduced on 2026-10-26 in one worktree: after `touch` on a tracked file whose content did not change,
`git --no-optional-locks status --porcelain` left `<git-dir>/index` untouched (mtime 1793013994.16). A plain
`git status --porcelain` then rewrote it (mtime 1793014653.45).

## Why it matters

roko-cli's `build.rs` reruns when its worktree's index changes (`git_watch_paths`). So each rewrite costs that
worker a `roko_cli` recompile on its next cargo command: about 4 minutes, and more under load. Seen on 2026-10-26:
a worker's index was rewritten at 12:09:52 while its job waited for a heavy.sh slot, with no git command of its own
in that window. Its CI-like re-run of bug-2a31bc's verify then recompiled roko-cli. In a batch round, every `next`
and every `claim` can do this to every active worker.

The write also takes `index.lock` in the other worktree. A worker's own commit, checkout or `git add` at that
moment can then fail with "Unable to create '…/index.lock': File exists", the conflict git-status(1) warns about
for status run in the background.

## Where

- `tools/work.py::worktree_changes`: the `status` call in its `scan` closure, the only git call in work.py that
  runs inside another worktree. Callers: `cmd_next` through `pick_next`, and `cmd_claim`.
- `tools/test_work.py::TestWorktreeBusy`: the scan's tests.

## Current state

The scan's other git call, `git diff --name-only <head>...<branch>`, runs in this checkout and compares commits only,
so it writes nothing. Only the per-worktree `status` writes.

## Plan

1. Run the status as `git --no-optional-locks -C <path> status …`, or set `GIT_OPTIONAL_LOCKS=0` in the scan's
   subprocess environment.
2. Add `test_worktree_scan_leaves_other_worktrees_index_alone` to `TestWorktreeBusy`. In another worktree, `touch` a
   tracked file without changing it, so its cached stat information is stale. Run `worktree_changes()`, then assert
   that the other worktree's index file (`git rev-parse --git-path index` there) has the same bytes and mtime as
   before.

## Done when

- The new test passes, and fails without the change. Verify:
  `grep -qw 'def test_worktree_scan_leaves_other_worktrees_index_alone' tools/test_work.py && python3 tools/test_work.py -k test_worktree_scan_leaves_other_worktrees_index_alone`
- The existing `TestWorktreeBusy` tests still pass.

## Notes

- Found by the worker on bug-2a31bc, bug-a66fef and bug-8d8f98 (2026-10-26), while tracing an unexpected roko-cli
  rebuild.
- The same applies to anyone reading another agent's worktree by hand: use `git --no-optional-locks`.
