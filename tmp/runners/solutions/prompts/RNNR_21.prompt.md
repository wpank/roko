# RNNR_21: Wire TaskDefFingerprint for mid-run edit detection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-21`](../ISSUE-TRACKER.md#rnnr-21)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.21
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `TaskDefFingerprint` struct and `from_task()` method already exist
in `persist.rs`. The resume logic in `resume.rs` already has `prepare_resume()`
which accepts `snapshot_fingerprints`. Verify this is fully wired and handle
the case where a task definition changed between runs.

## Exact Changes

1. Verify `TaskDefFingerprint::from_task()` is called at plan load time and
   stored in the checkpoint
2. In `prepare_resume()`, compare stored fingerprints against current task defs
3. If a task's fingerprint changed, mark it as ready for re-execution and
   log: "Task {id} definition changed since last run, re-executing"
4. If dependencies of a changed task need re-running, cascade the reset
5. Report mismatches in `ResumeReport` (already has `TaskMismatch` struct)

## Write Scope

- `crates/roko-cli/src/runner/resume.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Editing a task's prompt between runs causes it to re-execute on resume
- [ ] Unchanged tasks still skipped on resume
- [ ] Dependency cascading: if task A changed and task B depends on A, both re-run
- [ ] Mismatches reported clearly in resume output

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Editing a task's prompt between runs causes it to re-execute on resume
- Unchanged tasks still skipped on resume
- Dependency cascading: if task A changed and task B depends on A, both re-run
- Mismatches reported clearly in resume output
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
