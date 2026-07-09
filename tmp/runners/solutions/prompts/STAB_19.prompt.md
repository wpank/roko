# STAB_19: Wire BudgetPredictor to prompt assembly

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-19`](../ISSUE-TRACKER.md#stab-19)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.19
- Priority: **P1**
- Effort: 3 hours
- Depends on: `STAB_18` (source 1.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`BudgetPredictor` in `budget_predictor.rs` is 679 LOC with EMA-based prediction, failure
inflation, partial-match fallback, and persistence. No caller invokes `predictor.predict()`.
Token budgets are static constants.

## Exact Changes

1. Load `BudgetPredictor` from `.roko/learn/budget-predictions.json` at startup (or create new).
2. Before prompt assembly, call `predictor.predict(role, task_id)` to get predicted budget.
3. Use predicted budget as input to `PromptAssemblyService::with_token_budget()` (capped
   by the ContextTier from Task 1.18).
4. After task completion, call `predictor.observe(role, task_id, actual_tokens, success)`.
5. Persist predictor state during periodic flush.

## Design Guidance

The predictor should override static budgets only when it has sufficient observations
(>= 5 for the role/task combination). Below that threshold, use the ContextTier default.
This prevents cold-start issues where the predictor has no data.

## Write Scope

- `crates/roko-compose/src/budget_predictor.rs`
- `crates/roko-compose/src/prompt_assembly_service.rs`

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

- [ ] Run the same task type 5 times
- [ ] By run 5, predicted budget converges toward actual usage (within 20%)
- [ ] `.roko/learn/budget-predictions.json` has entries after runs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run the same task type 5 times
- By run 5, predicted budget converges toward actual usage (within 20%)
- `.roko/learn/budget-predictions.json` has entries after runs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
