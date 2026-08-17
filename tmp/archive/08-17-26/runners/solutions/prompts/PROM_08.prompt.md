# PROM_08: Call predict() Before Assembly

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-08`](../ISSUE-TRACKER.md#prom-08)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.8
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Before building the system prompt, call `predictor.predict()`.
If the predictor has history, use its estimate (clamped to tier budget).

## Exact Changes

1. Construct `TaskFeatures` from the current task's role, complexity, and domain
2. Call `predictor.lock().unwrap().predict(&features)`
3. If predictor `has_history(&features)`: use `min(predicted_budget, tier.default_token_budget())`
4. If no history: use `tier.default_token_budget()` (Phase 1 default)
5. Pass the effective budget to prompt assembly

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

- [ ] First run uses tier defaults (no prediction history)
- [ ] After 10+ tasks with same role/complexity/domain, `predict()` returns learned value
- [ ] Predicted budget is within the tier envelope (never exceeds tier budget)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- First run uses tier defaults (no prediction history)
- After 10+ tasks with same role/complexity/domain, `predict()` returns learned value
- Predicted budget is within the tier envelope (never exceeds tier budget)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
