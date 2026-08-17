# DEBT_29: Fix inconsistent max_tokens across dispatch paths

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#debt-29`](../ISSUE-TRACKER.md#debt-29)
- Source: `tmp/solutions/roko/tasks/12-CODE-DEBT.md` — Task 12.29
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DEBT_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

_(no implementation section in source — read source task)_

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/12-CODE-DEBT.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Same model produces same max_tokens regardless of entry point
- [ ] Setting `max_tokens = 32768` in model profile config is respected by all paths

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DEBT_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Same model produces same max_tokens regardless of entry point
- Setting `max_tokens = 32768` in model profile config is respected by all paths
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DEBT_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
