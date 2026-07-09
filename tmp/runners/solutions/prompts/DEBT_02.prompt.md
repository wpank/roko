# DEBT_02: Remove blanket clippy suppression from roko-cli

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#debt-02`](../ISSUE-TRACKER.md#debt-02)
- Source: `tmp/solutions/roko/tasks/12-CODE-DEBT.md` — Task 12.2
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DEBT_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

_(no implementation section in source — read source task)_

## Write Scope

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

- [ ] No crate-level `clippy::all`, `clippy::pedantic`, `clippy::nursery`, `clippy::restriction` suppression
- [ ] Zero warnings from `clippy::correctness` and `clippy::suspicious`
- [ ] Any remaining `#[allow(clippy::...)]` annotations are item-level

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DEBT_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No crate-level `clippy::all`, `clippy::pedantic`, `clippy::nursery`, `clippy::restriction` suppression
- Zero warnings from `clippy::correctness` and `clippy::suspicious`
- Any remaining `#[allow(clippy::...)]` annotations are item-level
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DEBT_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
