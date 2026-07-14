# SH01-T06B1-B2B1-CORRECTION implementation evidence

Assignment:
- Plan: `tmp/status-quo/self-heal/plans/SH01-runner-lifecycle/tasks.toml`
- Base SHA: `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- Branch/worktree: `agent/CTRL-02-SH01-process-reconstruct` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-02-SH01-PROCESS`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: `crates/roko-cli/src/runner/agent_stream.rs`; this evidence file

Requirement:
- Original defect or missing behavior: `spawn_agent` registered its PID only after event publication, prompt input, and both reader-task setup paths, leaving a spawned process unowned if later setup failed or panicked. When the stdout event channel closed, the reader broke only out of the per-line event loop, then continued reading the live child and could remain attached indefinitely.
- Acceptance requirements: register the PID immediately after obtaining it so every later path is killable through orphan cleanup; return from the stdout reader task at the first failed event delivery; directly prove both ordering and termination behaviors; pass the focused `agent_stream` suite and quality gates.
- Explicit non-goals: do not reconstruct the precursor's `AgentHandle::is_finished` C4 probe; do not edit any other source, manifest, checklist, or status; do not claim the broader SH01 cancellation chain complete.
- Dependencies and their integration commits: historical SH01 prerequisites through `1649c18b2`; independent of the separate attempt-ownership correction. This reconstruction owns only the SH01-T06B1 process-registration and SH01-T06B2B1 reader-termination corrections embedded in precursor `3041d095d`.

Reproduction:
- Pre-fix commands: `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-cli --lib runner::agent_stream::tests::spawned_pid_is_registered_before_started_delivery_completes -- --exact` and the same command for `runner::agent_stream::tests::closed_event_channel_terminates_stdout_reader_before_child_exit`.
- Expected: PID registration is visible while `Started` delivery is deliberately blocked; a stdout reader whose receiver is closed finishes while its child continues sleeping.
- Actual: both tests failed before the production correction with exit 101. The first reported `spawned PID was not registered while Started delivery was blocked`; the second timed out and reported `stdout reader remained attached to the live child after its event channel closed`. Both tests completed process and registry cleanup before asserting the expected failure.

Implementation:
- Design and invariants: `spawn_agent` now registers the child immediately after obtaining its PID and before the first subsequent await or fallible reader setup. The stdout producer returns from its task on failed delivery, so closure is a terminal condition rather than an inner-loop condition. Process-backed tests deliberately block `Started` delivery and keep a child alive after stdout output to prove the ordering and task-exit boundaries directly.
- Files/symbols changed: `spawn_agent` and two focused Unix regressions plus their `scripted_agent` fixture in `crates/roko-cli/src/runner/agent_stream.rs`.
- Compatibility/migration: internal process supervision only; no public API, serialization, or migration change.
- Failure/recovery/security behavior: once `Command::spawn` yields a PID, crash/orphan cleanup owns that process even if event delivery, prompt input, or stream setup later stalls or fails. A closed event consumer cannot leave the stdout producer waiting on a still-running child. Existing confirmed kill paths unregister the PID after cleanup, and both regressions verify cleanup without detached processes.

Verification:
- Both exact regression commands above: failed before the correction for the intended reasons; passed after the correction (1 passed each).
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-cli runner::agent_stream`: passed; 17 focused library tests passed, 0 failed, and filtered integration targets completed successfully. The command emitted the existing non-fatal `missing documentation for the crate` warning from `tests/plan_validation.rs`.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo check -p roko-cli --lib`: passed.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo clippy -p roko-cli --lib -- -D warnings`: passed.
- `rustfmt --edition 2024 --check crates/roko-cli/src/runner/agent_stream.rs`: passed.
- `cargo fmt --all -- --check`: remains nonzero solely for the pre-existing out-of-scope formatting delta at `crates/roko-cli/src/tui/views/agents_view.rs:879`; this commit does not touch that file.
- `git diff --check`: passed.

Review readiness:
- Implementation commit: the candidate commit containing this evidence; its immutable SHA is reported at handoff to avoid a self-referential hash.
- Diff scope reviewed: only the assigned `agent_stream.rs` and this evidence file are included. Against precursor `3041d095d`, the two production changes are the exact early-registration move and `break`-to-`return` correction. The new tests supply direct proof absent from the precursor; `AgentHandle::is_finished` and every other precursor path are excluded.
- Known limitations: none within the assigned correction; C4 liveness probing remains deliberately excluded.
- Required reviewer focus: verify registration precedes every fallible/awaiting post-spawn operation, closed-channel termination exits the task rather than only an inner loop, and no `is_finished` hunk entered this commit.

Integration:
- Review evidence: pending independent review.
- Integration commit: pending.
- Post-merge commands/results: pending integration owner.
- Final status: `IMPLEMENTED_UNREVIEWED` only after candidate commit; not DONE before accepted merge and post-merge proof.
