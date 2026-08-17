# PROM_10: Blend Static and Predicted Budgets During Warmup

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-10`](../ISSUE-TRACKER.md#prom-10)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.10
- Priority: **??**
- Effort: 1-2 days | **Impact**: High (closes the learning loop)
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: `SectionInfluence` tracks per-section lift but weights are
not fed back into budget allocation. The system collects data about what
helps and ignores it.

## Exact Changes

1. Add `pub fn predict_with_fallback(&self, features: &TaskFeatures, static_budget: u64) -> u64`
2. Determine observation count for the feature key
3. If count < 10: return `static_budget`
4. If count 10-50: return `(static_budget + self.predict(features)) / 2`
5. If count > 50: return `self.predict(features)` (full prediction)
6. Minimum floor of 1000 tokens always applies
7. Add unit tests covering all three bands

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] 0 observations: returns static budget unchanged
- [ ] 15 observations: returns average of static and predicted
- [ ] 60 observations: returns predicted (ignores static)
- [ ] Unit tests cover all three bands

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 0 observations: returns static budget unchanged
- 15 observations: returns average of static and predicted
- 60 observations: returns predicted (ignores static)
- Unit tests cover all three bands
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
