# EVAL_15: `ComplexityCriterion`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-15`](../ISSUE-TRACKER.md#eval-15)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.15
- Priority: **P2**
- Effort: 3 hours
- Depends on: `EVAL_13` (source 5.13)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Checks cyclomatic/cognitive complexity and body line count per function from AST evidence. Default thresholds: cyclomatic 15, cognitive 20, body lines 100.

## Exact Changes

1. Implement `ComplexityCriterion`:
   - Consumes `EvidenceKind::Ast`.
   - Score = fraction of functions within all three thresholds. Soft severity. Default threshold 0.9.
   - Emit Findings for each function exceeding any threshold.
2. Gate behind `#[cfg(feature = "ast")]`.

## Write Scope

- `crates/roko-eval-metrics/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Test with functions of varying complexity

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test with functions of varying complexity
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
