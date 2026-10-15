+++
id = "bug-e998be"
kind = "bug"
title = "An attempt refused because its branch is still checked out gets no safe way forward, so a chat model removed the checkout with git worktree remove --force"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/orchestrator"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "f6f61d459"
source = "tmp/hermes-roko-v5/findings/repro-will.md#F2,F3 (repro-will, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: Will's Mac crashed at 17:02 with roko runs in flight"
anchors = ["crates/roko-cli/src/orchestrator/worktree/mod.rs::create_locked", "crates/roko-cli/src/orchestrator/worktree/acceptance.rs::set_aside_kept_checkout", "crates/roko-cli/src/orchestrator/worktree/git_ops.rs::branch_checked_out_reason"]
links = { depends_on = [], blocks = [], related = ["bug-06bd18", "bug-dceafd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_refused_attempt_says_where_its_branch_is_and_how_to_move_it_aside' crates/roko-cli/ && cargo test -p roko-cli a_refused_attempt_says_where_its_branch_is_and_how_to_move_it_aside"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T20:52:46Z"
commit = "f6f61d459"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "S"
model = "claude-opus-5-5"
forced = false
evidence = "create_locked's refusal for an attempt branch that is still checked out (git_ops::worktree_with_branch, branch_checked_out_reason) now names the checkout that has the branch, says roko left it as it is and deleted nothing, and gives the move-aside command set-aside itself runs (git worktree move <path> <path>-kept && git branch -m <branch> <branch>-kept); it never suggests a forced removal. set_aside_kept_checkout logs why it left a checkout alone. cargo test -p roko-cli a_refused_attempt_says_where_its_branch_is_and_how_to_move_it_aside passes: it runs the given command and the attempt then starts, the old checkout's uncommitted file kept; a_new_run_sets_aside_the_checkouts_another_run_kept still passes; roko-cli --lib 3636 passed, 0 failed (gist test skipped); clippy -p roko-cli -D warnings clean. The crash case itself (kill -9 of serve mid-attempt, then a re-run) already worked on main through bug-06bd18, checked live (tmp/hermes-roko-v5/findings/roko-recovery.md)."
+++

## Problem

When a new attempt cannot get its checkout because the attempt's branch is still checked out by a leftover
checkout, roko fails the task with:

```
agent error (worktree-isolation): failed to acquire worktree for <plan>/<task>/attempt-0: workspace I/O error:
cannot safely reattach worktree `attempt-…`: branch `roko/attempt/attempt-…` is already checked out in this
repository
```

The message says neither where that checkout is, nor whose it is, nor what roko did with it, nor what the person
can run. After the 2026-10-15 17:02 crash every later run of `demo-hello-world` failed this way (repro-will F2, Will's
binary 216f88c84), and with approvals off the chat model "fixed" it itself with
`git worktree remove --force .roko/worktrees/attempt-…`, then started paid runs unasked (repro-will F3). Removing a
checkout by force throws away whatever uncommitted work it held.

## Why it matters

Goal `core`: "fix it and run it again" is the loop after a crash, and roko must recover its own state so nobody else
has to. bug-06bd18 (1e609bb67) already sets aside the checkout a dead or earlier run kept (branch renamed to
`<branch>-<run8>`, checkout moved with `git worktree move`, nothing deleted), and roko-recovery checked on main
7b6a5af44 that a re-run after `kill -9` of serve mid-attempt now succeeds, with the crashed attempt's commits and
uncommitted work kept (`tmp/hermes-roko-v5/roko-recovery/zc/`). What is left is the case set-aside cannot handle:
the refusal must say what roko did and give a safe command, never a forced removal.

## Where

- `crates/roko-cli/src/orchestrator/worktree/mod.rs::create_locked`: refuses when `git worktree list --porcelain`
  shows the attempt's branch checked out anywhere.
- `crates/roko-cli/src/orchestrator/worktree/acceptance.rs::set_aside_kept_checkout`: skips a checkout that fails a
  re-attach's identity checks, silently.

## Current state

Set-aside handles a kept or crashed attempt checkout at its canonical path whose HEAD is on its own branch. It does
not handle the branch checked out somewhere else (a checkout someone moved by hand without renaming its branch, or
one checked out elsewhere), or a canonical checkout that fails the identity checks (e.g. an unrecoverable creation
claim). Those still end in the refusal above.

## Plan

1. When the branch is checked out, find where (`worktree <path>` before its `branch` line in the porcelain list).
2. Refuse with a message that names that path, says roko left it as it is and deleted nothing, says why it did not
   set it aside (the identity check's reason, when there was one), and gives the safe command that does what
   set-aside does: `git -C <repo> worktree move <path> <path>-kept && git -C <repo> branch -m <branch> <branch>-kept`.
3. Never suggest `git worktree remove --force` (a test asserts the text).

## Done when

`[[verify]]` passes: a test makes the attempt's branch checked out at another path, and the refusal names the path,
says nothing was deleted, gives the move-and-rename command, and does not contain `remove --force`.
