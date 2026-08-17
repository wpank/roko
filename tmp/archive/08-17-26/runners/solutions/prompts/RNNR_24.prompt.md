# RNNR_24: Enhance `--dry-run` with wave plan preview

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-24`](../ISSUE-TRACKER.md#rnnr-24)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.24
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Enhance the existing `--dry-run` to show wave structure and critical
path analysis. Currently `cmd_plan_dry_run()` exists but may not show wave
breakdown or critical path.

## Exact Changes

1. Build the DAG via `UnifiedTaskDag` and compute waves
2. Display wave structure:
   ```
   Wave 0 (3 tasks, parallel):
     T1: "Wire episode logging" [mechanical, 2 files]
     T2: "Wire cascade router"  [mechanical, 1 file]
   Wave 1 (2 tasks, parallel):
     T4: "Wire replan logic" [integrative, 4 files]  deps: T1, T2
   Total: 5 tasks, 2 waves, max parallelism: 3
   ```
3. Highlight critical path from `UnifiedTaskDag::critical_path()`
4. Include file overlap warnings (tasks in same wave touching same files)
5. No git operations or agent dispatches during dry run

## Write Scope

- `crates/roko-cli/src/commands/plan.rs`

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

- [ ] Wave structure displayed with task details and dependencies
- [ ] Critical path highlighted
- [ ] File overlap warnings shown for potential merge conflicts
- [ ] No execution occurs during dry run

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Wave structure displayed with task details and dependencies
- Critical path highlighted
- File overlap warnings shown for potential merge conflicts
- No execution occurs during dry run
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
