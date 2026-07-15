# CTRL-02 workspace-lock precursor implementation evidence

Assignment:
- Plan: `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, Wave 0 `CTRL-02`; bounded precursor adjacent to `SH02-T06` in `tmp/status-quo/self-heal/plans/SH02-isolation-recovery/tasks.toml`
- Base SHA: `1eb2eabb604f3dd45fcf16f52ed826a668824cda`
- Historical precursor: `3041d095d4daebed2c9e05c63eacb18e668e37e3` relative to parent `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- Branch/worktree: `agent/CTRL-02-workspace-lock` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-02-workspace-lock`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: `crates/roko-cli/src/workspace_lock.rs` and this evidence record
- Dependencies and their integration commits: the clean integration base contains the exact inherited `3041d095d` workspace-lock precursor; no full SH02 task is claimed

Requirement:
- Original defect: the old lock path opened `.roko/runtime/roko.lock` with `truncate(true)` before attempting the advisory lock. A losing process therefore erased the live owner's PID before `try_lock_exclusive` reported contention. Normal guard drop also left stale PID diagnostics behind.
- Expected behavior: a contender that does not own the advisory lock never changes the owner diagnostic; stale diagnostics are replaced only after acquisition; normal owner release clears its diagnostic before unlocking; and the next owner can acquire and publish its PID without the previous guard erasing it.
- Acceptance for this bounded candidate: focused same-process and real-process contention, stale replacement, owner-release/reacquisition regressions, `cargo test -p roko-cli workspace_lock`, formatting, affected-package all-target check, diff hygiene, and independent review of the exact candidate.
- Explicit non-goals: dirty-work inventory, quarantine, task/worktree ownership, kill-and-resume attribution, idempotent worktree recovery, runner resume/persist/orchestrator changes, manifests, task status, master edits, or complete `SH02-T06`/issue 53 closure. The SH02-T06 manifest currently omits this real source path and must be corrected later by its manifest owner.

Reproduction:
- Historical source proof: `git diff 1649c18b2c3d2b3602bfe17398b0e1454a19c5ef..3041d095d4daebed2c9e05c63eacb18e668e37e3 -- crates/roko-cli/src/workspace_lock.rs` shows the old `OpenOptions::truncate(true)` before `try_lock_exclusive`; therefore any losing open truncated the current owner's PID.
- Regression proof: `separate_process_contenders_never_truncate_owner_pid` starts a real helper process, verifies its distinct PID in the lock file, performs 32 losing acquisitions from the parent, and asserts the exact owner bytes after every failure. This would deterministically fail against the pre-`3041d095d` implementation on the first contender.
- Issue context: `tmp/status-quo/issues/53-STALE-LOCK-AND-DIRTY-WORKTREE-BLOCK-RECOVERY.md` records a dead PID diagnostic after timeout. This candidate proves only the singleton diagnostic portion, not dirty-worktree recovery.

Implementation:
- The inherited production implementation opens without truncation, acquires the exclusive advisory lock first, then seeks/truncates/writes/syncs the current PID. Failed contenders only read diagnostics.
- `WorkspaceLockGuard::drop` clears and syncs the diagnostic while its file descriptor still owns the advisory lock, then unlocks. A crash may leave a stale PID, which the next successful owner replaces under lock.
- Added a real subprocess test harness with bounded waits and kill-on-unwind cleanup. It proves distinct-process contention, owner-only clearing, and release-to-next-owner ordering; existing same-process and stale-diagnostic tests remain intact.
- Compatibility and failure behavior: no public API, lock path, file format, dependency, or runtime behavior changed in this reconstruction. Test helpers are ignored unless invoked by the parent tests and leave only `tempfile`-owned files.

Verification:
- Working directory for every command: `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-02-workspace-lock`.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/targets/CTRL-02-workspace-lock CARGO_INCREMENTAL=0 cargo test -p roko-cli --lib workspace_lock -- --nocapture`
  - Exit 0. The parent harness ran 4 tests passed, 0 failed, with the subprocess helper ignored. Each of the two controlled child processes ran the exact helper once and passed. This covers 32 losing acquisitions against a distinct live owner PID, same-process contention, stale replacement, normal owner clearing, and successor PID survival.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/targets/CTRL-02-workspace-lock CARGO_INCREMENTAL=0 cargo check -p roko-cli --all-targets`
  - Exit 0. The only diagnostic was the pre-existing missing-crate-documentation warning in `crates/roko-cli/tests/plan_validation.rs`, outside this reserved scope.
- `cargo fmt --all -- --check`
  - Exit 0 after formatting the new focused test code.
- `git diff --check`
  - Exit 0.
- Exact scope census and artifact cleanup:
  - `git status --short` reports only the two reserved candidate paths.
  - The isolated Cargo target was removed with `cargo clean --target-dir /Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/targets/CTRL-02-workspace-lock`; no helper process, lock file, test tempdir, local target, generated index, log, symlink, or other artifact remains.

Review readiness:
- Implementation components: inherited production fix in `3041d095d`; present-base adversarial regressions and attribution in this candidate.
- Exact candidate SHA: supplied by the independent reviewer after this evidence and the reserved Rust file are committed together.
- Cumulative candidate scope: exactly the reserved Rust file and this evidence record.
- Known limitations: full `SH02-T06` remains `ready`; issue 53 dirty-worktree preservation/reacquisition remains open; the SH02-T06 manifest source list remains stale and is owned by a later manifest reconciliation.
- Required reviewer focus: actual cross-process locking, loser non-mutation, cleanup strictly before unlock, safe successor acquisition/PID persistence, bounded helper cleanup, and strict non-claim of full recovery.

Integration:
- Independent review: `tmp/status-quo/execution-evidence/CTRL-02-WORKSPACE-LOCK-REVIEW.md`,
  ACCEPTED at review commit `acd2675e474f72ccccd4cbb1dfa9f7d4db1b93ef`
  for exact candidate `2e4296e4e4f5f87eead09430b9652bc80e3342fc`.
- Integration commits: candidate `4454af335`; review `cd185001b`.
- Post-merge verification on `cd185001b`: the focused suite again passed four
  parent tests plus both controlled child-helper executions, with zero failures;
  `cargo check -p roko-cli --all-targets`, formatting, diff hygiene, and clean
  integration status passed. The only check warning remains the pre-existing
  missing-doc warning in `tests/plan_validation.rs`.
- Final status: `DONE` only for the bounded CTRL-02 workspace-lock precursor
  attribution. Full `SH02-T06`, issue 53 dirty-work recovery, and manifest
  source-list reconciliation remain open.
