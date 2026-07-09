# XCUT_18: Implement Force-Kill Escalation for Agent Processes

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-18`](../ISSUE-TRACKER.md#xcut-18)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.18
- Priority: **P0**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ProcessSupervisor` at line 839 of `crates/roko-runtime/src/process.rs` has `shutdown_all()` (line 951) and `kill_all()` (line 1033) but no SIGTERM-to-SIGKILL escalation. The dogfood session (documented in `tmp/dogfood/CONTEXT.md`) revealed agents surviving `force_shutdown` because SIGTERM was ignored by the subprocess tree. The `Drop` impl at line 1248 force-kills on drop, but this is a last resort.

`crates/roko-agent/src/process/kill.rs` has kill logic for individual processes. `crates/roko-agent/src/process/mod.rs` and `registry.rs` also reference force_shutdown/SIGKILL. These need to be coordinated with the supervisor's escalation.

## Exact Changes

1. Add `ProcessSupervisor::shutdown_with_escalation(timeout: Duration)`:
   - Phase 1 (0 to timeout/2): SIGTERM to process group.
   - Phase 2 (timeout/2 to timeout): SIGKILL to process group.
   - Phase 3 (after timeout): kill each PID individually if group kill failed.
2. Use `nix::sys::signal::killpg()` for process group signaling (already used in the codebase).
3. Track process group IDs at spawn time via `process_group(0)` in the `Command` builder.
4. Log each escalation step: `tracing::warn!("agent {} did not respond to SIGTERM, escalating to SIGKILL", pid)`.
5. After kill, verify process is dead with `waitpid(WNOHANG)`.

## Write Scope

- `crates/roko-runtime/src/process.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Agent processes that ignore SIGTERM are killed within `timeout` seconds
- [ ] Process group kill catches child processes spawned by the agent
- [ ] Log output shows the escalation progression
- [ ] No zombie processes remain after shutdown

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Agent processes that ignore SIGTERM are killed within `timeout` seconds
- Process group kill catches child processes spawned by the agent
- Log output shows the escalation progression
- No zombie processes remain after shutdown
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
