# XCUT_21: Add Process Orphan Detection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-21`](../ISSUE-TRACKER.md#xcut-21)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.21
- Priority: **P4**
- Effort: 4 hours
- Depends on: `XCUT_18` (source 19.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

When `roko plan run` is killed (SIGKILL, OOM, power failure), spawned agent processes may survive as orphans. There is no mechanism to detect and clean up orphans from a previous run on restart. `ProcessSupervisor` at line 839 of `process.rs` tracks handles in memory but does not persist PIDs to disk.

## Exact Changes

1. On agent spawn, write PID to `.roko/state/pids/<run_id>/<task_id>.pid`.
2. On clean agent exit, remove the PID file.
3. On `plan run` startup, scan `.roko/state/pids/` for PID files from previous runs.
4. For each PID file, check if the process is still running (`kill(pid, 0)` or equivalent).
5. If running, log a warning: `tracing::warn!("orphan process {} from previous run {} still running", pid, run_id)`.
6. Add `roko util cleanup-orphans` subcommand that kills orphaned processes.
7. In non-interactive mode (daemon, CI), auto-kill orphans from the same run directory.

## Write Scope

- `crates/roko-runtime/src/process.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/main.rs`

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

- [ ] PID files are created on agent spawn and removed on clean exit
- [ ] `roko plan run` warns about orphans from previous runs
- [ ] `roko util cleanup-orphans` kills orphaned agent processes

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- PID files are created on agent spawn and removed on clean exit
- `roko plan run` warns about orphans from previous runs
- `roko util cleanup-orphans` kills orphaned agent processes
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
