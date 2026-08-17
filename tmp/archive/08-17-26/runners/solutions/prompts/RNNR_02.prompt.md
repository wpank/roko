# RNNR_02: Implement three-tier branch model in WorktreeManager

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-02`](../ISSUE-TRACKER.md#rnnr-02)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.2
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_01` (source 14.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Implement the source/integration/task branch hierarchy from the
mega-parity runner. Currently `format_branch_name()` creates task branches
but there is no integration branch concept.

## Exact Changes

1. Add `pub async fn create_integration_branch(&self, run_id: &str) -> Result<String, WorktreeError>`
   that creates branch `roko/run-{run_id}` from current HEAD
2. Modify `create()` to accept optional `base_branch: Option<&str>` parameter;
   when provided, fork from that branch instead of deriving from `self.config.base_branch`
3. Add `integration_branch: Option<String>` field to `WorktreeConfig` so all
   task worktrees in a run fork from the same integration branch
4. Add `pub fn backup_branch_name(run_id: &str, task_id: &str) -> String` that
   returns `roko/{run_id}-{task_id}-backup-{timestamp}` for retry preservation
5. Update `format_branch_name()` to use pattern `roko/{run_id}-{task_id}`

## Write Scope

_None — this is a documentation/verification-only batch._

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

- [ ] Integration branch created once per plan run
- [ ] All task worktrees fork from integration branch, not source branch
- [ ] `git branch --list 'roko/*'` shows expected hierarchy after a run
- [ ] Existing callers of `create_for_plan()` still work (backward compat)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Integration branch created once per plan run
- All task worktrees fork from integration branch, not source branch
- `git branch --list 'roko/*'` shows expected hierarchy after a run
- Existing callers of `create_for_plan()` still work (backward compat)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
