# RNNR_30: Add `--pause` flag for inter-wave inspection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-30`](../ISSUE-TRACKER.md#rnnr-30)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.30
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_20` (source 14.20)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Pause between waves so a human can inspect the merged result, fix
issues, and resume. Essential for large plans where wave gate failures need
human judgment.

## Exact Changes

1. Add `--pause` flag to `PlanCmd::Run` in `main.rs`
2. After each wave completes and merges, print summary:
   ```
   Wave 2 complete: 5/5 tasks succeeded, 3 files modified
   Integration branch: roko/run-20260429
   Press Enter to continue, or 's' to stop...
   ```
3. Wait for user input before dispatching next wave
4. On 's': save checkpoint and exit cleanly (resumable with `--resume-plan`)
5. While paused, human can inspect integration branch, run manual tests
6. After resume, re-read integration branch state (human may have made changes)

## Write Scope

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] `--pause` stops execution between waves and waits for input
- [ ] User can inspect and modify integration branch during pause
- [ ] Enter resumes execution; 's' saves and exits
- [ ] Changes made during pause visible to subsequent waves

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `--pause` stops execution between waves and waits for input
- User can inspect and modify integration branch during pause
- Enter resumes execution; 's' saves and exits
- Changes made during pause visible to subsequent waves
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
