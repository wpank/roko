# 120 — Plan Run Preflight Checks

**Priority**: P1 — Without preflight checks, `roko plan run` proceeds into execution even when critical preconditions fail (missing API key, insufficient disk, stale locks), wasting tokens and producing confusing mid-run errors.
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/commands/do_cmd.rs`
**Depends on**: None (reuses existing doctor check logic)
**Sources**: `tmp/backlog/_checklist-gaps.md` §1.6, `tmp/backlog/_mori-old-gaps.md` MO-39

---

## Background

`roko doctor` performs workspace health checks: valid config, LLM credentials present, disk space, git state, Rust toolchain. These checks exist and work, but they are only run when the operator explicitly invokes `roko doctor`. Running `roko plan run plans/` starts execution immediately without any pre-run validation.

In practice, common failures discovered mid-run include: missing API key (wastes startup time before the first agent dispatch), disk space below the worktree threshold (worktree creation fails mid-run), stale `.roko/state/runner.lock` from a previous crash (causes immediate failure), and no valid plans directory (fails with a confusing path error instead of a clear message).

Mori ran a subset of doctor checks as a startup gate inside the execution loop. Roko should do the same: fast, blocking preflight checks that fail with actionable messages before any LLM is invoked.

## Current State

- `crates/roko-cli/src/commands/` — `doctor` command exists with multiple check types.
- `crates/roko-cli/src/runner/event_loop.rs` — no preflight gate at startup.
- `crates/roko-core/src/config/loader.rs` — config loading exists; credential validation is present.
- Disk space check: `roko doctor disk` exists; the check is not called from `plan run`.
- Stale lock detection: `runner.lock` is written/read but not checked before startup.

## Implementation Plan

1. **Create `preflight.rs` in `crates/roko-cli/src/runner/`**: Implement `PlanRunPreflight::run(config, plans_dir) -> PlanRunPreflightResult` returning a list of `PreflightCheck { name, status: Pass|Warn|Fail, message }`.

2. **Checks to implement** (in order, fail-fast on FAIL, continue on WARN):
   - **Config valid**: can `RokoConfig::load()` deserialize `roko.toml` without error?
   - **LLM credentials**: is at least one provider's API key set in the environment or config?
   - **Disk space**: is free space on the workspace partition ≥ 2 GB?
   - **Git state**: are we inside a git repo? Is there at least one commit? (Warn if dirty.)
   - **Plans directory**: does the given path exist and contain at least one `tasks.toml`?
   - **Rust toolchain**: does `rustc --version` exit 0?
   - **No stale lock**: does `.roko/state/runner.lock` exist AND contain a PID that is not running? If so, remove it with a warning.
   - **No stale worktrees exceeding limit**: warn if more than 10 stale worktrees exist (disk pressure).

3. **Integrate into event loop startup**: In `event_loop.rs`, before entering the main `tokio::select!` loop, call `PlanRunPreflight::run()`. If any check returns `Fail`, print the check name and message, then return early with a non-zero exit code. If any check returns `Warn`, print the warning and continue.

4. **`--skip-preflight` flag**: For testing and CI scenarios where preflight checks are known to be redundant, add `--skip-preflight` to `plan run`.

5. **Output format**: When checks fail, print in the same format as `roko doctor`:
   ```
   [FAIL] LLM credentials: No API key found. Set ANTHROPIC_API_KEY or configure a provider in roko.toml.
   [WARN] Git state: Working tree has uncommitted changes.
   ```

6. **Exit code**: Exit code 1 if any `Fail` check; exit code 0 (with warnings printed) if only `Warn` checks.

## Acceptance Criteria

1. `roko plan run` with a missing API key prints a `[FAIL]` message and exits before any LLM call.
2. `roko plan run` with disk space < 2 GB prints a `[FAIL]` message and exits.
3. `roko plan run` with an invalid plans directory prints a `[FAIL]` message.
4. A stale `runner.lock` is removed with a `[WARN]` message and execution continues.
5. Uncommitted git changes produce a `[WARN]` but do not block execution.
6. `--skip-preflight` bypasses all checks.
7. A workspace passing all checks shows no preflight output and proceeds normally.

## Verification Checklist

- [ ] Unset `ANTHROPIC_API_KEY`; run `roko plan run`; verify `[FAIL]` and no LLM invocation.
- [ ] Create a stale `runner.lock` with a dead PID; verify `[WARN]` and the lock is removed.
- [ ] Run `roko plan run /nonexistent`; verify `[FAIL] Plans directory` message.
- [ ] Run with `--skip-preflight`; verify no preflight output and execution proceeds.
- [ ] Pass all checks; verify no preflight output appears.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/preflight.rs` | New file: `PlanRunPreflight`, check implementations |
| `crates/roko-cli/src/runner/mod.rs` | Export `preflight` module |
| `crates/roko-cli/src/runner/event_loop.rs` | Call `PlanRunPreflight::run()` at startup |
| `crates/roko-cli/src/commands/do_cmd.rs` | Add `--skip-preflight` flag |
