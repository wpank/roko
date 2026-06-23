# PROM_09: Call record() After Gate Results

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-09`](../ISSUE-TRACKER.md#prom-09)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.9
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: After gate results are known, call `predictor.record()` with
actual token usage and success/failure outcome.

## Exact Changes

1. After gate verdict for a task, extract actual `input_tokens` from the agent response
2. Determine `success: bool` from gate verdict
3. Call `predictor.lock().unwrap().record(&features, actual_tokens, success)`
4. The predictor applies 1.3x failure inflation automatically on failure

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

- [ ] Run a plan with 5+ tasks
- [ ] Verify `budget-predictor.json` has entries for each feature combination
- [ ] Verify observation counts increase with each run
- [ ] Verify failed tasks have inflated EMA values

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run a plan with 5+ tasks
- Verify `budget-predictor.json` has entries for each feature combination
- Verify observation counts increase with each run
- Verify failed tasks have inflated EMA values
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
