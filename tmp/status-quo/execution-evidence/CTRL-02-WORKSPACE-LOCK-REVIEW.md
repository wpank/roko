# CTRL-02 workspace-lock precursor independent review

- Verdict: **ACCEPTED**
- Exact candidate: `2e4296e4e4f5f87eead09430b9652bc80e3342fc`
- Candidate parent/base: `1eb2eabb604f3dd45fcf16f52ed826a668824cda`
- Historical precursor: `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- Historical parent: `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- Review branch/worktree: `review/CTRL-02-workspace-lock-2e4296e4` /
  `reviews/CTRL-02-workspace-lock-2e4296e4`
- Confidence: high for the bounded singleton-lock precursor

## Independent requirement reconstruction

Issue 53 records two distinct recovery problems: a dead PID left in
`.roko/runtime/roko.lock`, and an attributable dirty/registered worktree that
resume could not reacquire. This candidate owns only the singleton advisory-lock
precursor. It must ensure that a process which loses lock acquisition cannot
truncate or replace the live owner's PID, that an owner replaces a stale
diagnostic only after it owns the lock, and that normal release clears its PID
before unlocking without later erasing a successor's PID.

This is not complete `SH02-T06`. Dirty-work preservation/quarantine, worktree
ownership, crash attribution, resume/reacquisition, and deterministic repair
remain open. The current SH02-T06 manifest also does not list
`crates/roko-cli/src/workspace_lock.rs`; a later manifest owner must reconcile
that source boundary without treating this bounded acceptance as full issue 53
closure.

## Candidate identity, scope, and history

I read the full master, full SH02 manifest, issue 53, worker evidence, complete
candidate diff, current source and call sites, and the `1649c18b2..3041d095d`
historical production diff. The cumulative candidate changes exactly two paths:

- `crates/roko-cli/src/workspace_lock.rs` (test module only);
- `tmp/status-quo/execution-evidence/CTRL-02-WORKSPACE-LOCK.md`.

There is no production, public API, manifest, master, status, Cargo metadata,
lockfile, runner, resume, persist, or orchestrator change in the candidate.
The production prefix of `workspace_lock.rs` through `#[cfg(test)]` is
byte-identical to `3041d095d`; the candidate adds only 155 lines of focused
test support/regressions plus its evidence record. The historical precursor is
an ancestor of the candidate.

The pre-precursor `1649c18b2` implementation read the PID and then opened the
same file with `.truncate(true)` before `try_lock_exclusive()`. Therefore a
losing contender deterministically erased the owner bytes before reporting
contention. The candidate's distinct-process regression would fail at its first
post-contender byte assertion against that source.

## Production-path review

The inherited implementation satisfies the bounded ownership contract:

1. It creates the runtime directory and opens `roko.lock` read/write without
   truncation.
2. It calls `try_lock_exclusive()` before any file mutation.
3. Only the successful owner runs `set_len(0)`, seeks to byte zero, writes its
   PID, and `sync_data()`s it. A stale PID is diagnostic data, not ownership;
   the advisory lock is the authority, so a new successful owner may replace it.
4. A failed contender performs only `read_to_string` after lock failure. It may
   report `unknown` if an owner releases during that diagnostic read, but it
   never mutates the lock file.
5. `WorkspaceLockGuard::drop` executes clear, sync, then unlock in that exact
   order while it still owns the advisory lock. It performs no file mutation
   after unlock, so it cannot erase the PID written by a successor.
6. If a process crashes, the OS releases its advisory lock when the file
   descriptor closes; the stale PID remains until the next owner replaces it
   under lock. No PID-liveness guess can override a live advisory owner.

The production call sites in plan Runner v2, PRD draft/plan generation, and
serve bind the guard to `_lock` for the mutating command scope. The `#[must_use]`
guard prevents accidental unnoticed immediate release at other call sites.
No call site uses the PID text to decide ownership.

Drop cannot return cleanup errors, so the implementation intentionally ignores
clear/sync/unlock results and relies on file close for ultimate advisory-lock
release. The tested normal filesystem path clears correctly. Filesystem-failure
recovery and audit logging are not claimed by this precursor.

## Adversarial test assessment

The new harness invokes the actual Rust test binary as a distinct child process,
passes a unique temporary workspace and role through environment variables, and
waits for explicit ready/acquired/release marker files. It is not a simulated
same-process lock.

- `separate_process_contenders_never_truncate_owner_pid` observes the child's
  real PID, performs 32 losing parent acquisitions, and checks both the error
  and exact owner bytes after every loss.
- `normal_release_cannot_clear_the_next_owners_pid` starts a real waiting
  successor, drops the first guard, waits for the successor's acquired marker,
  and proves the lock file contains the successor PID before releasing it.
- Existing tests independently cover same-process contention and replacement
  of a stale diagnostic after acquisition.
- Parent waits have ten-second deadlines. The contender acquisition and helper
  release loops also have ten-second deadlines. `LockProcess::Drop` signals
  release, waits at most two seconds, then kills and reaps the child. Successful
  tests explicitly wait/reap both children. The temporary directory owns every
  marker and lock file.

I found no test-only bypass, weakened assertion, unbounded retry, orphan-child
path, production behavior change, or hidden full-recovery claim.

## Independent verification

Focused cold test, using the shared integration target but compiling the exact
candidate worktree sources:

```text
CARGO_TARGET_DIR=.../integration/target CARGO_INCREMENTAL=0 \
  cargo test -p roko-cli --lib workspace_lock -- --nocapture

exit 0; finished in 9m14s
parent: 4 passed; 0 failed; 1 ignored
owner child helper: 1 passed; 0 failed
successor child helper: 1 passed; 0 failed
```

The ignored row is only the helper's ordinary parent-harness listing; each
parent test explicitly launches it with `--ignored --exact`, and both child
executions passed.

Affected-package gate:

```text
CARGO_TARGET_DIR=.../integration/target CARGO_INCREMENTAL=0 \
  cargo check -p roko-cli --all-targets

exit 0; finished in 4m01s
```

The sole warning is the pre-existing missing crate documentation warning in
`crates/roko-cli/tests/plan_validation.rs`, outside this candidate.

Additional independent checks:

```text
cargo fmt --all -- --check                         exit 0
git diff --check 1eb2eabb6..2e4296e4e             exit 0
exact cumulative path census                       2 expected paths
3041d095d ancestor check                            pass
production-prefix identity to 3041d095d             exact
historical/current mutation-order assertions        pass
review worktree before evidence                     clean
```

The order assertions prove old truncate-before-lock, current
lock-before-truncate/write/sync, and Drop clear/sync-before-unlock directly from
the immutable Git blobs. Rebuilding the entire historical workspace solely to
reproduce the deterministic first byte mismatch would add no evidence beyond
that source-order proof and the passing real-process candidate test.

## Verdict and next action

**ACCEPTED.** No correction is required for this bounded candidate. The
integration owner may merge exact candidate `2e4296e4e4f5f87eead09430b9652bc80e3342fc`
with this review record, rerun the focused child-process suite and package gate
post-merge, and record only the workspace-lock precursor as integrated.
`SH02-T06`, issue 53 dirty-work recovery, and the manifest source-list
reconciliation must remain open.
