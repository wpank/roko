# DEBT_12: Consolidate dispatch_direct.rs into dispatch_v2.rs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#debt-12`](../ISSUE-TRACKER.md#debt-12)
- Source: `tmp/solutions/roko/tasks/12-CODE-DEBT.md` — Task 12.12
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DEBT_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

_(no implementation section in source — read source task)_

## Write Scope

- `crates/roko-cli/src/dispatch_direct.rs`
- `crates/roko-cli/src/dispatch_v2.rs`
- `crates/roko-cli/src/lib.rs`

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

- [ ] `dispatch_direct.rs` deleted
- [ ] `grep -rn 'dispatch_direct' crates/roko-cli/` returns 0

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DEBT_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `dispatch_direct.rs` deleted
- `grep -rn 'dispatch_direct' crates/roko-cli/` returns 0
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DEBT_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
