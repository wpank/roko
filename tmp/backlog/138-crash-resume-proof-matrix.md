# 138 — Crash/Resume Proof Matrix (Seven Scenarios, No Duplicate Completion)

**Priority**: P1 — Resume code exists in `runner/resume.rs` but has never been proven correct under real crash conditions; without a reproducible proof, the resume guarantee is untested and cannot be trusted for unattended production runs.
**Size**: S (1-2 days)
**Crates**: `crates/roko-cli/src/runner/resume.rs`, `tests/`
**Depends on**: #137 (crash snapshot must include router/threshold state for full proof coverage)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §E-3 (suggested 123)

---

## Background

`runner/resume.rs` implements snapshot-based resume: when a plan run is interrupted, it can restart from the last snapshot without re-executing completed tasks. This is essential for long self-hosting runs where a crash 80% through a 6-hour plan should not require starting over.

The implementation is present but not systematically proven. The mori-diffs audit identified seven crash scenarios that must be verified to have no duplicate task completion:

1. Crash during agent output (agent is mid-turn, pre-gate).
2. Crash post-agent turn, pre-gate start.
3. Crash during gate execution.
4. Crash post-gate, pre-snapshot write.
5. Stale PID file (process no longer running).
6. Stale plan IDs in snapshot (tasks that were added to the plan after the snapshot was written).
7. JSONL tail corruption (last line of `episodes.jsonl` or `events.jsonl` is incomplete).

For each scenario, the proof requires: a script that reliably induces the crash condition, a resume attempt, and an assertion that no task appears in `episodes.jsonl` with two `TaskCompleted` entries for the same task ID.

## Current State

- `crates/roko-cli/src/runner/resume.rs` — resume logic exists; reads `RunStateSnapshot` and skips completed tasks.
- `.roko/state/run-state.json` — written on each runner tick.
- `.roko/state/runner.lock` — PID file; stale lock detection is partial (see #120 preflight checks).
- `episodes.jsonl` tail corruption: no detection or recovery logic.
- No systematic crash/resume test suite exists.

## Implementation Plan

1. **Test harness infrastructure**: Create `tests/crash_resume/` directory with a helper that:
   - Runs `roko plan run` as a subprocess with `--log-file /tmp/test.jsonl`.
   - Sends SIGKILL or SIGTERM to the process at a specified point (controlled by a signal from a helper thread or a deliberate panic).
   - Reads the log file to determine which scenario was reached at crash time.
   - Resumes with `roko plan run ... --resume-plan`.
   - Asserts no duplicate `TaskCompleted` entries in `.roko/episodes.jsonl`.

2. **Scenario 1 — crash during agent turn**: Kill the process while the agent subprocess is still running (between agent spawn and agent completion). Verify: on resume, the task restarts (agent turn is not counted as completed).

3. **Scenario 2 — crash post-turn, pre-gate**: Kill after agent turn completes but before `GateStarted` event. Verify: task does not re-run the agent turn; gate re-runs.

4. **Scenario 3 — crash during gate**: Kill during a `cargo build` gate subprocess. Verify: gate re-runs on resume; no partial gate result is accepted.

5. **Scenario 4 — crash post-gate, pre-snapshot**: Kill after gate passes but before `RunStateSnapshot` is written. This is the hardest case: the task is complete but the snapshot says it is not. Verify: the task may re-run (idempotent is acceptable) OR the JSONL-based recovery detects completion. Do not allow two `TaskCompleted` events.

6. **Scenario 5 — stale PID**: Write a `runner.lock` with a dead PID, then start fresh. Verify: preflight check (#120) removes the stale lock and run proceeds.

7. **Scenario 6 — stale plan IDs**: Add a new task to `tasks.toml` after a partial run snapshot. Verify: resume recognizes the new task as `Pending` and runs it.

8. **Scenario 7 — JSONL tail corruption**: Truncate the last line of `episodes.jsonl` to simulate a write-interrupt. Verify: JSONL reader skips the truncated line rather than panicking, and the valid preceding entries are readable.

## Acceptance Criteria

1. All seven scenarios complete without panicking.
2. No duplicate `TaskCompleted` entries appear in `episodes.jsonl` for any scenario.
3. Scenario 7 (JSONL corruption) does not panic; the reader logs a warning and skips the corrupted line.
4. Test harness is deterministic and can be run in CI without live LLM access (use a mock agent).
5. `cargo test -p roko-cli crash_resume` passes.

## Verification Checklist

- [ ] Scenario 1: kill during agent turn; resume; verify no duplicate task completion.
- [ ] Scenario 4: kill post-gate pre-snapshot; resume; verify task either re-runs once or is skipped (not twice).
- [ ] Scenario 7: truncate JSONL last line; resume; verify warning is logged and no panic.
- [ ] Run all 7 scenarios in sequence; verify all pass.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/resume.rs` | Fix any bugs discovered during scenario testing |
| `crates/roko-cli/src/runner/event_loop.rs` | Fix scenario 4 post-gate pre-snapshot race if found |
| `tests/crash_resume/` | New directory with test harness and 7 scenario tests |
| `crates/roko-fs/src/` | Add JSONL truncation recovery (scenario 7) |
