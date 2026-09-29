+++
id = "bug-109b5a"
kind = "bug"
title = "No pre-spawn stale .git/index.lock cleanup (with .git indirection) before agent dispatch"
status = "open"
triage = "verified"
severity = "p1"
size = "S"
goal = "core"
subsystem = ["roko-cli/worktree"]
created = 2026-09-05
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:3392", "crates/roko-cli/src/graph_execution/workspaces.rs::acquire", "crates/roko-cli/src/orchestrator/worktree/cleanup.rs::clear_stale_locks_unlocked", "crates/roko-cli/src/orchestrator/worktree/mod.rs::create_locked", "crates/roko-cli/src/orchestrator/worktree/git_ops.rs::is_stale_lock"]
links = { depends_on = [], blocks = [], related = ["gap-7ed79a", "gap-4ec59f", "q-1faa0c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE 'clear_stale_index_lock|clean_stale_index_lock' crates/roko-cli/src/graph_task_dispatch.rs && grep -rqw 'fn stale_index_lock_is_cleared_before_dispatch_with_gitdir_indirection' crates/roko-cli/src && cargo test -p roko-cli --lib stale_index_lock_is_cleared_before_dispatch_with_gitdir_indirection"
+++

## Problem

If a git process dies while holding `index.lock` (for example an agent killed mid-`git add`, or a crashed run), the
lock file stays behind. The next agent dispatched into that checkout finds every index-writing git command failing
with `fatal: Unable to create '.../index.lock': File exists`, and the task fails or stalls on work it cannot
commit or inspect.

`roko plan run` (Graph engine) does no stale-lock cleanup before dispatching an agent:

- Default mode (shared checkout, no `--worktree-per-task`): no cleanup at all.
- `--worktree-per-task`: locks are cleared only when a brand-new attempt worktree is created
  (`WorktreeManager::create_locked`). A retry reuses the same attempt worktree (attempt number is always 0, and
  `acquire` returns the tracked worktree), so a lock left by the failed attempt is not cleared.
- The existing cleanup assumes `<repo_root>/.git` is a directory. When the workspace is itself a linked worktree or
  a submodule, `.git` is a file containing `gitdir: <path>`, and the real lock lives under that path; it is never
  found.

Expected: immediately before each agent dispatch, a stale `index.lock` in the git directory that serves the task's
working directory (following `gitdir:` indirection) is removed, and a fresh one (a live git process) is left alone.

## Why it matters

- Goal `core` (plan runs work reliably): a crash or kill leaves the checkout unusable for the next task until a
  human deletes the file; retries fail the same way and burn budget.
- The git audit (finding G03) recorded this as fixed on 2026-09-04, but the fix lived in
  `crates/roko-cli/src/runner/event_loop.rs` (`ensure_attempt_workdir`), which was deleted with Runner-v2 on
  2026-09-06 (`6b5da8616`). Nothing replaced it on the Graph path.
- Related: `q-1faa0c` (dev-audit runtime fixes unverified on the Graph engine after Runner-v2 deletion),
  `gap-4ec59f` (worktree isolation: flip default and add startup repair), `bug-53475e` (failed cleanup retains the
  kernel mutation lock), parked umbrella `gap-7ed79a` (git audit: worktree/merge safety).

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`, "Worktree isolation: acquire" block (~line 3360-3395): acquires the
  lease (if a workspace provider is configured) and computes `effective_workdir` (worktree path, else the shared
  `self.workdir`). This is the pre-dispatch point where cleanup belongs.
- `crates/roko-cli/src/graph_execution/workspaces.rs::WorktreeExecutionWorkspaceProvider::acquire`: returns the
  tracked worktree unchanged when the attempt already exists (no cleanup on reuse). `reconcile` (~line 156) only
  reports `WorktreeHealth::StaleLock` as a `Conflict`.
- `crates/roko-cli/src/orchestrator/worktree/cleanup.rs::clear_stale_locks_unlocked` (~line 274): removes
  `<repo_root>/.git/index.lock` and `<repo_root>/.git/worktrees/*/index.lock` when stale; hardcodes `.git` as a
  directory (~lines 278, 287). `clear_stale_locks` (public, takes the repository mutation lock) has no production
  caller.
- `crates/roko-cli/src/orchestrator/worktree/mod.rs::create_locked` (~line 568): calls
  `clear_stale_locks_unlocked` before creating a worktree.
- `crates/roko-cli/src/orchestrator/worktree/git_ops.rs::is_stale_lock` (~line 839): stale means mtime at least
  `STALE_LOCK_SECS` old; `STALE_LOCK_SECS = roko_core::defaults::DEFAULT_STALE_LOCK_SECS = 60`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` (~line 1154): enables the worktree provider only with
  `--worktree-per-task`.
- Entry point: `roko plan run <dir>` -> Graph task dispatch -> agent spawn.

## Current state

- Pre-dispatch cleanup: absent on the Graph path (no `index.lock` handling in `graph_task_dispatch.rs`).
- Creation-time cleanup: present for new attempt worktrees only.
- `.git` file indirection: not handled anywhere in `orchestrator/worktree/cleanup.rs`.
- The deleted G03 fix resolved `gitdir:` by reading the `.git` file and removed `index.lock` older than 60 s in the
  resolved directory; that approach can be reused.
- `cleanup.rs` last changed in `244f564e1`.

## Plan

1. Add `pub fn clear_stale_index_lock_for_workdir(workdir: &Path) -> Option<PathBuf>` in
   `crates/roko-cli/src/orchestrator/worktree/cleanup.rs` (free function or on `WorktreeManager`):
   - Resolve the git dir for `workdir`: if `workdir/.git` is a directory use it; if it is a file, parse
     `gitdir: <path>` (relative paths resolve against `workdir`). Alternative: `git -C <workdir> rev-parse
     --git-path index.lock`, which also handles submodules and `GIT_DIR`, at the cost of spawning git.
   - If `<gitdir>/index.lock` exists and `is_stale_lock` says it is stale, remove it and log at `warn` with path
     and age; return the removed path.
2. Call it in `graph_task_dispatch.rs` right after `effective_workdir` is computed and before the agent is spawned,
   for both modes. In worktree mode this also covers reused attempt worktrees.
3. Make `clear_stale_locks_unlocked` use the same resolver for `repo_root` so creation-time cleanup also works when
   the repo root is a linked worktree.
4. Tests (unit, temp dirs, no real git needed for the indirection case):
   `stale_index_lock_is_cleared_before_dispatch_with_gitdir_indirection` (a workdir whose `.git` is a `gitdir:` file
   pointing at a dir with an old `index.lock`: removed), plus cases for a plain `.git` dir and a fresh lock (kept).
   Set the lock's mtime into the past with `filetime` or `std::fs::File::set_modified`.

## Done when

- A stale `index.lock` left in the shared checkout, in a reused attempt worktree, or behind a `gitdir:` file is
  removed before the next agent dispatch, with a warning log naming it.
- A lock younger than `STALE_LOCK_SECS` is never removed.
- Verify (proposed; the current `[[verify]]` passes as soon as any file outside `orchestrator/worktree/` mentions
  `clear_stale_locks`, even in a comment, and ignores the indirection half):

  ```
  grep -qE 'clear_stale_index_lock|clean_stale_index_lock' crates/roko-cli/src/graph_task_dispatch.rs && grep -rqw 'fn stale_index_lock_is_cleared_before_dispatch_with_gitdir_indirection' crates/roko-cli/src && cargo test -p roko-cli --lib stale_index_lock_is_cleared_before_dispatch_with_gitdir_indirection
  ```

## Notes

- Deleting a lock that a live git process holds corrupts that operation. In the shared checkout the user's own git
  may hold it — for example `git commit` holds `index.lock` while the commit-message editor is open, which can
  exceed 60 s. Options: (a) delete at the 60 s TTL in both modes (the G03 behaviour); (b) delete in roko-owned
  attempt worktrees, but in the shared checkout only warn, or fail the dispatch with a clear message, unless the
  lock is much older (for example 10 minutes). Recommend (b); the shared checkout belongs to the user.
- `gap-4ec59f` (step 1) proposes calling `clear_stale_locks` once at run startup. That helps at run start. It does
  not replace the per-dispatch check (locks left mid-run by a killed attempt) or the `gitdir:` resolution.
- Do not take the repository mutation lock on the dispatch hot path unless needed. The per-workdir check touches
  one file.
- Small and self-contained. Conflicts only with concurrent edits to the acquire block in `graph_task_dispatch.rs`
  or to `orchestrator/worktree/cleanup.rs` (`bug-53475e` also touches cleanup code: coordinate).

## Original notes

Post-crash stale index.lock files can hang agent spawns; fix added 5-min TTL stale lock removal in ensure_attempt_workdir() before turn_start. Register marks DONE (2026-09-04/05), but the runtime hook was placed in crates/roko-cli/src/runner/event_loop.rs (Runner-v2), which was retired/deleted 20...

Imported without verification from:
- `tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register`
- `tmp/archive/git-audit/impl-G03-index-lock-cleanup.md`

A source claims this was fixed; confirm against current code before closing.

Warning: every file this item cites is gone (`crates/roko-cli/src/runner/event_loop.rs`) — likely obsolete or moved.

How to verify: grep for index.lock cleanup reachable from the Graph engine's attempt workdir preparation before provider dispatch.

Verified 2026-09-28: still true - stale index.lock removal exists only as WorktreeManager::clear_stale_locks (orchestrator/worktree/cleanup.rs:261-271) with no production caller; the Graph path only reports WorktreeHealth::StaleLock as a reconcile conflict (graph_execution/workspaces.rs:156), with no pre-dispatch cleanup.

Rechecked 2026-09-29 at d9e79e9d8: still open, with one correction to the 2026-09-28 note. The opt-in --worktree-per-task path does clear stale locks when it creates an attempt worktree: WorktreeManager::create_locked (orchestrator/worktree/mod.rs:568) calls clear_stale_locks_unlocked (cleanup.rs:274). Two things remain. Default shared-checkout Graph runs do no index.lock cleanup before dispatch. The existing cleanup hardcodes repo_root/.git (cleanup.rs:278, :287), so it does not resolve a .git file (gitdir: indirection) when the workspace is itself a linked worktree or submodule.

The [[verify]] command is unsound (see the check notes). Proposed replacement, not yet validated: `A named test the fix adds, e.g. cargo test -p roko-cli --lib stale_index_lock_is_cleared_before_dispatch_with_gitdir_indirection, covering a shared-checkout Graph dispatch and a workdir whose .git is a gitdir: file. The current grep ignores the .git-indirection half, and a doc-comment mention outside orchestrator/worktree/ would pass it.`.
