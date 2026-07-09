# STAB_69: Add cross-session cost aggregation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-69`](../ISSUE-TRACKER.md#stab-69)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.69
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_69 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Cost tracking per session exists but no cross-session aggregation for daily budget enforcement.

## Exact Changes

1. `CostsDb.aggregate_since(today_start)` -> daily total.
2. Initialize `BudgetGuardrail.day_spent` from aggregate.
3. Expose aggregates via `roko learn efficiency`.

## Write Scope

- `crates/roko-learn/src/costs_db.rs`
- `crates/roko-learn/src/budget.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] 3 sessions on same day shows cumulative daily cost in `roko learn efficiency`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_69 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 3 sessions on same day shows cumulative daily cost in `roko learn efficiency`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_69 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
