# PROM_07: Load BudgetPredictor at Plan Run Startup

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-07`](../ISSUE-TRACKER.md#prom-07)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.7
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Load `BudgetPredictor` from `.roko/learn/budget-predictor.json` at
plan run startup.

## Exact Changes

1. Import `roko_compose::budget_predictor::{BudgetPredictor, TaskFeatures, load_predictor, persist_predictor}`
2. In plan runner init, load predictor: `load_predictor(&learn_dir).unwrap_or_default().unwrap_or_default()`
3. Wrap in `Arc<Mutex<BudgetPredictor>>` and attach to the runner context
4. At run end (or periodically), persist with `persist_predictor(&predictor, &learn_dir)`

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

- [ ] `roko plan run` loads predictor without error (even if file does not exist)
- [ ] After run completes, `.roko/learn/budget-predictor.json` exists
- [ ] Second run loads the file produced by the first run

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run` loads predictor without error (even if file does not exist)
- After run completes, `.roko/learn/budget-predictor.json` exists
- Second run loads the file produced by the first run
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
