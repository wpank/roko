+++
id = "bug-53475e"
kind = "bug"
title = "Failed worktree cleanup retains kernel mutation lock requiring operator intervention"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-cli/worktree"]
created = 2026-09-05
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "32938ad4b"
source = "tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/orchestrator/worktree/creation_journal.rs::retain_lock_if_cleanup_unproved", "crates/roko-cli/src/orchestrator/worktree/creation_journal.rs::acquire_repository_mutation_lock", "crates/roko-cli/src/orchestrator/worktree/git_ops.rs::start_owned_operation", "crates/roko-cli/src/orchestrator/worktree/git_ops.rs::mark_cleanup_unproved", "crates/roko-cli/src/orchestrator/worktree/cleanup.rs::clear_stuck_mutation_lock", "crates/roko-cli/src/orchestrator/worktree/mod.rs::REPOSITORY_MUTATION_LOCK", "crates/roko-cli/src/graph_execution/plan_runner.rs:1157"]
links = { depends_on = [], blocks = [], related = ["gap-7ed79a", "gap-4ec59f", "q-1faa0c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn unproved_cleanup_fails_fast_instead_of_blocking_later_operations' crates/roko-cli/src && cargo test -p roko-cli --lib unproved_cleanup_fails_fast_instead_of_blocking_later_operations"
+++

## Problem

With `roko plan run --worktree-per-task`, every worktree mutation (create an attempt worktree, remove, prune) runs
under two locks: the in-process operation mutex (`WorktreeManager.operations`) and a cross-process `flock` on
`<git common dir>/roko-worktree-mutation.lock` (normally `.git/roko-worktree-mutation.lock`).

If a git child process started by one of those mutations cannot be proved gone (it ignored termination during
cancellation, `try_wait` failed and the kill also failed, or the worker panicked), the operation is marked "cleanup
unproved". The manager then deliberately leaks both locks with `std::mem::forget` so that no later git mutation
can overlap a possibly still-running git process. The locks stay held until the roko process exits. Consequences:

- Every later worktree operation in that process waits forever: the next task's `acquire`, every `release`,
  `prune`. The run hangs with no error and no log that says why.
- Another roko process on the same repository blocks forever in `acquire_repository_mutation_lock`, which uses a
  blocking `flock` with no timeout.
- The only recovery is for an operator to find and kill the process.

The original finding (git audit G07) proposed removing a stuck lock file at startup. That does not fix the hang: a
lock file left by a dead process never blocks anything, because acquisition opens the file with `O_CREAT` and
takes a `flock`, which the kernel released when the holder died. The helper's own doc comment
(`cleanup.rs`, above `clear_stuck_mutation_lock`) says the same.

Expected: after an unproved cleanup, later operations either resume automatically once the stray git process is
gone, or fail fast with an error that names the process to check. No operation blocks forever.

## Why it matters

- Goal `core` (plan runs work reliably): one unlucky cancellation or git failure hangs the rest of a worktree-mode
  run silently.
- Worktree isolation is opt-in today, but `gap-4ec59f` proposes making it the default; this hang would then affect
  every run.
- Related: `bug-109b5a` (stale `index.lock` before dispatch, same module), `q-1faa0c` (dev-audit fixes unverified
  on the Graph engine), parked umbrella `gap-7ed79a` (git audit worktree/merge safety).

## Where

- `crates/roko-cli/src/orchestrator/worktree/git_ops.rs`:
  - git child supervision (~lines 340-372): calls `lifecycle.mark_cleanup_unproved()` when
    `terminate_direct_child` fails after a cancel or a `try_wait` error; the child PID is available there
    (`child.id()`).
  - `start_owned_operation` (~line 688): runs each mutation on a dedicated "roko-worktree-mutation" thread; on a
    panic it marks cleanup unproved; if unproved it calls `std::mem::forget(operation)` (~line 725), leaking the
    in-process operation mutex guard.
  - `await_owned_operation` (~line 642) and `await_owned_operation_controlled` (~line 655): callers' wait points.
- `crates/roko-cli/src/orchestrator/worktree/creation_journal.rs`:
  - `OperationLifecycle` (~line 87): `cleanup_unproved: AtomicBool`, `mark_cleanup_unproved`,
    `cleanup_was_unproved`.
  - `retain_lock_if_cleanup_unproved` (~line 238): `std::mem::forget(repository_lock)` when unproved, keeping the
    `flock` for the process lifetime.
  - `acquire_repository_mutation_lock` (~line 1146): opens or creates the lock file and calls
    `rustix::fs::flock(.., LockExclusive)`, a blocking call with no timeout.
- `crates/roko-cli/src/orchestrator/worktree/cleanup.rs::clear_stuck_mutation_lock` (~line 181): removes a lock
  file older than `STUCK_MUTATION_LOCK_AGE_SECS` (300 s, `mod.rs` ~line 66) if a non-blocking `flock` succeeds.
  No production caller.
- `crates/roko-cli/src/orchestrator/worktree/mod.rs`: `REPOSITORY_MUTATION_LOCK` (~line 81);
  `create_for_attempt` (~line 765) shows the usual pattern: take the operation mutex, acquire the repository lock,
  do the work, then `retain_lock_if_cleanup_unproved`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` (~line 1157): the only production `WorktreeManager::new`,
  under `--worktree-per-task`.

## Current state

- The retention is intentional fail-safe behaviour (comments at `creation_journal.rs` ~line 244 and `git_ops.rs`
  ~line 722). What is missing is any way out other than process exit, and any diagnostic.
- The G07 fix (a startup call to the stuck-lock remover) was in `crates/roko-cli/src/runner/event_loop.rs`, deleted
  with Runner-v2 on 2026-09-06 (`6b5da8616`). `clear_stuck_mutation_lock` survives but is uncalled, and as noted
  above it cannot unblock a live process.
- The lock lives in the Git common directory, not `.roko/` as the original note says.

## Plan

1. Replace the leaks with retained ownership that can be re-proved. Add to `WorktreeManager` a
   `retained: Arc<parking_lot::Mutex<Option<RetainedOwnership>>>` holding the `RepositoryMutationLock`, the
   operation guard and the unproved child PIDs (with a start-time fingerprint if available). Record the PID in
   `OperationLifecycle` when calling `mark_cleanup_unproved`.
2. At the start of every mutation (before waiting on `operations`): if `retained` is set, check whether each
   recorded PID is gone (`kill(pid, 0)` returns `ESRCH`, or the fingerprint no longer matches). If all are gone,
   drop the retained guards (releasing both locks) and continue. If any is alive, return a new
   `WorktreeError::OwnershipRetained { pids, since }` immediately, with a message such as "a git process from an
   earlier worktree operation (pid N) could not be stopped; kill it or restart roko".
3. Cross-process: acquire the `flock` non-blocking in a loop with backoff up to a deadline (for example 30 s), then
   fail with an error. Write the holder's PID and start time into the lock file after locking, so the error can name
   the holder.
4. Surface the error: the Graph dispatcher already fails the task on an `acquire` error
   (`graph_task_dispatch.rs`, "failed to acquire worktree for ..."), so a fast error fails the attempt instead of
   hanging the run. Make sure the message reaches the TUI/log.
5. Optional hygiene: call `clear_stuck_mutation_lock` once at `--worktree-per-task` startup to remove old unowned
   lock files. It is harmless, but it is not the fix. (`gap-4ec59f` step 1 already plans this startup call.)
6. Tests (unit, in `orchestrator/worktree/`): `unproved_cleanup_fails_fast_instead_of_blocking_later_operations`
   (simulate an unproved cleanup with a live dummy PID; the next operation returns `OwnershipRetained` within a
   short timeout), and a test that once the PID has exited the next operation succeeds and the locks are released.

## Done when

- After a simulated unproved cleanup, the next worktree operation in the same process returns an error naming the
  stray PID within seconds, or proceeds once that PID has exited. It never blocks indefinitely.
- A second process contending for the repository lock gets a timed error naming the holder PID.
- Verify (proposed; the current `[[verify]]` only checks that `clear_stuck_mutation_lock` gains a caller outside
  `orchestrator/worktree/`, which would not fix the hang):

  ```
  grep -rqw 'fn unproved_cleanup_fails_fast_instead_of_blocking_later_operations' crates/roko-cli/src && cargo test -p roko-cli --lib unproved_cleanup_fails_fast_instead_of_blocking_later_operations
  ```

## Notes

- Safety-critical concurrency code. The invariant to keep: no git mutation may start while a git process from an
  earlier mutation might still be running. Releasing the locks is allowed only after the recorded PIDs are proved
  gone. When in doubt, fail closed with an error, never overlap.
- Reuse the PID fingerprint approach already used for orphan agents (`roko_agent::process::registry`, "start
  fingerprint") rather than inventing a new one.
- Touches the same files as `bug-109b5a` (`orchestrator/worktree/cleanup.rs`); do not run the two in parallel
  without coordinating.
- Only reachable with `--worktree-per-task` today; becomes default-path if `gap-4ec59f` lands.
- 2026-10-01 (wk-tiers): Implemented on `work/gap-4ec59f` at `8e23f0a79`; cargo verification deferred to the batch check.
  - Nothing is leaked any more. After an unproved cleanup, the worker stores the repository lock and the git PIDs
    it could not stop (`OperationLifecycle::mark_cleanup_unproved_for`) in the manager's reservation
    (`OperationState.retained`, `git_ops.rs`). Every later mutation checks them first
    (`release_retained_ownership`). It releases them once each PID is gone (`kill(pid, 0)` gives ESRCH).
    Otherwise it returns `WorktreeError::OwnershipRetained` at once, naming the PIDs. With no PID known (the
    create-rollback path), mutations stay refused until roko restarts, and the error says so.
  - Cross-process: the `flock` is tried without blocking in a loop for up to 60 s (`REPOSITORY_LOCK_WAIT`), then
    the wait fails naming the holder. The holder writes `<pid> <unix secs>` into the lock file after locking.
  - `clear_stuck_mutation_lock` now unlinks the file before it releases the flock. A process waiting on the old
    file then fails its binding check, instead of mutating beside the holder of a new file.
  - Tests:
    - `unproved_cleanup_fails_fast_instead_of_blocking_later_operations`: with no PID, the next prune is refused
      at once; with a live `sleep` PID, it is refused naming that PID; once the process is reaped, prune runs and
      releases the hold.
    - `a_contended_repository_lock_times_out_naming_its_holder`: a subprocess with a 300 ms wait, set through a
      test-only environment variable.
    - `unproved_create_cleanup_permanently_withholds_mutation_owner` now asserts the retained ownership instead
      of a leaked guard.
  - Not done:
    - PID start-time fingerprints. A reused PID keeps the hold, so this fails closed.
    - Whether the TUI shows the error. The Graph dispatcher reports it through its existing "failed to acquire
      worktree" path; that path was not checked.

## Original notes

Fix added non-blocking flock probe + 5-min age gate on .roko/roko-worktree-mutation.lock at runner startup. Register marks DONE (2026-09-04/05), but the runtime hook was placed in crates/roko-cli/src/runner/event_loop.rs (Runner-v2), which was retired/deleted 2026-09-06; the Graph plan-execution...

Imported without verification from:
- `tmp/archive/git-audit/06-FINDINGS-REGISTER.md#register`
- `tmp/archive/git-audit/impl-G07-stuck-lock-recovery.md`

A source claims this was fixed; confirm against current code before closing.

Warning: every file this item cites is gone (`.roko/roko-worktree-mutation.lock`, `crates/roko-cli/src/orchestrator/worktree.rs`, `crates/roko-cli/src/runner/event_loop.rs`) — likely obsolete or moved.

How to verify: Check stuck-lock recovery for roko-worktree-mutation.lock is invoked at Graph plan-run startup.

Verified 2026-09-28: still true - WorktreeManager::clear_stuck_mutation_lock (orchestrator/worktree/cleanup.rs:181) has no production caller, so nothing on Graph plan-run startup recovers a stuck roko-worktree-mutation.lock (orchestrator/worktree/mod.rs:81). The worktree code moved from orchestrator/worktree.rs to the orchestrator/worktree/ module.
