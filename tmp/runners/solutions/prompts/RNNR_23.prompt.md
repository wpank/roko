# RNNR_23: Add `--only` flag for selective task execution

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-23`](../ISSUE-TRACKER.md#rnnr-23)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.23
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Allow running only specific tasks from a plan, matching the
mega-parity runner's `--only A,B,C` flag.

## Exact Changes

1. Add `--only <task_ids>` CLI flag (comma-separated list of task IDs)
2. When `--only` is set, filter the task DAG to include only specified tasks
   and their transitive dependencies
3. Tasks not in the set are marked as `Skipped` in result files
4. Combine with `--resume-plan`: `--only T5,T6 --resume-plan` re-runs T5 and T6
   but skips everything else
5. Validate that all specified task IDs exist in the plan (error early)

## Write Scope

- `crates/roko-cli/src/main.rs`
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

- [ ] `roko plan run --only T5,T6` runs only T5 and T6 (and their deps)
- [ ] Tasks not in the list are skipped
- [ ] `--only` combined with `--resume-plan` works correctly
- [ ] Invalid task IDs produce clear error before execution starts

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run --only T5,T6` runs only T5 and T6 (and their deps)
- Tasks not in the list are skipped
- `--only` combined with `--resume-plan` works correctly
- Invalid task IDs produce clear error before execution starts
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
