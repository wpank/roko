+++
id = "gap-d1f787"
kind = "gap"
title = "work.py next treats files changed in any other worktree as busy"
status = "done"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "a56a62e41"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W1-work-graph-audit.md (R3, R6); W13-active-sessions-coordination.md (rec 2)"
anchors = ["tools/work.py::pick_next", "tools/work.py::cmd_next", "tools/test_work.py"]
lane = "tracker"
parent = "spec-1e1b45"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_next_treats_files_changed_in_other_worktrees_as_busy' tools/test_work.py && python3 tools/test_work.py -k test_next_treats_files_changed_in_other_worktrees_as_busy"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T08:41:47Z"
commit = "a56a62e41"
by = "wk-gates"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-01T08:36:19Z"
model = "claude-opus-5-5"
forced = false
evidence = "Premise re-checked at bdeaff586: pick_next's busy set came from live claims only. a56a62e41: worktree_changes() reads each other worktree (read-only, 16 at a time): commits since the merge-base with HEAD for branches with unmerged commits (one for-each-ref --no-merged call) plus 'git status --porcelain -z' (uncommitted and untracked); the main checkout counts only uncommitted files; work/, target/ and node_modules/ ignored. pick_next skips an item anchored on such a file with the reason 'touches files changed in worktree <name> (<branch>)'; next --json keeps stdout as the list of picks and writes the skipped items with worktree and branch to stderr; --ignore-worktrees disables the scan. On the real repo: 224 worktrees, 697 busy files from 35 of them, scan 1.65 s, next 2.4 s. tools/test_work.py: committed and uncommitted changes in other worktrees hold items back with the worktree named, a clean merged worktree and work/ edits add nothing, a worker sees the main checkout's uncommitted edits; the [[verify]] command passes."
+++

## Problem

`pick_next` counts a file as busy only when a live claim's item anchors it. Work in progress without a matching claim
is invisible to it: sessions that don't use the tool, and claimed items whose fixes touch more files than their
anchors name. On 09-29 the six `roko-wt-*` worktrees were changing 63 files and the claims covered 6 of them (W1);
bug-7d7200 had 4 anchors and 44 changed files. `next` would still have handed out 106 of the 241 open items, each
anchored on a file dirty in one of those worktrees (W13).

## Why it matters

Goal `tooling`, epic spec-1e1b45. It is the cheapest guard against two agents editing the same file, and it needs no
cooperation from other sessions (W1 R6).

## Where

- `tools/work.py`: `pick_next` (the `busy` set) and `cmd_next` (skip reasons, `--json`).
- Tests: `tools/test_work.py`; the `RepoTest` fixture can add a second worktree with `git worktree add`.

## Current state

Checked at `41c7ffbd6`: `busy` comes from live claims only. The repo has 11 worktrees.

## Plan

1. For every entry of `git worktree list --porcelain` other than the one running `next`, collect the files changed
   since its merge-base with the current branch (`git -C <wt> diff --name-only <merge-base>`), plus
   `git -C <wt> status --porcelain` (uncommitted and untracked files).
2. When `next` runs in a worktree, add the main checkout's uncommitted edits too.
3. Ignore `work/` (item files and views are dirty during every sweep) and build output (`target/`, `node_modules/`).
4. Add these files to `busy`. The skip reason names the worktree and branch, in text and in `--json`.
5. `--ignore-worktrees` turns the scan off. A clean, fully merged worktree adds nothing.

## Done when

- [ ] An item anchored on a file changed in another worktree, committed or not, is not picked.
- [ ] The skip reason names that worktree.
- [ ] `next` stays under about 2 s with 11 worktrees.
- [ ] The `[[verify]]` command passes.

## Notes

- Read-only git only; never write in another worktree.
- gap-823dce makes `claim` repeat the same check under a lock.
