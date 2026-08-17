# EVAL_21: Position swap-and-discard

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-21`](../ISSUE-TRACKER.md#eval-21)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.21
- Priority: **P1**
- Effort: 4 hours
- Depends on: `EVAL_18` (source 5.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Mandatory position bias mitigation. For every pairwise comparison by every judge, present both orderings and discard inconsistent results. See PRD-04 Section 5 for the full rationale.

## Exact Changes

1. Define `PairwiseVerdict { PreferA, PreferB, Tie }`.
2. Define `PositionSwapResult { judge: JudgeSpec, verdict_ab: PairwiseVerdict, verdict_ba: PairwiseVerdict, consistent: bool, effective_verdict: Option<PairwiseVerdict> }`.
3. Implement `check_consistency(verdict_ab, verdict_ba) -> bool`:
   - `(PreferA, PreferB)` -> true (same artifact preferred regardless of position)
   - `(PreferB, PreferA)` -> true
   - `(Tie, Tie)` -> true
   - Everything else -> false (position-dependent, discard)
4. Implement IPI metric: `ipi(results: &[PositionSwapResult]) -> f64` = fraction of inconsistent results.

## Write Scope

- `crates/roko-eval-judge/src/lib.rs`

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

- [ ] Test consistent case: (PreferA, PreferB) -> consistent=true
- [ ] Test inconsistent case: (PreferA, PreferA) -> consistent=false, discarded
- [ ] Test IPI calculation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test consistent case: (PreferA, PreferB) -> consistent=true
- Test inconsistent case: (PreferA, PreferA) -> consistent=false, discarded
- Test IPI calculation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
