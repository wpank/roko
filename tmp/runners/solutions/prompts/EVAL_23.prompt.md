# EVAL_23: Aggregation and disagreement detection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-23`](../ISSUE-TRACKER.md#eval-23)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.23
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_18` (source 5.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Trimmed mean aggregation, learned judge weights, and disagreement detection. See PRD-04 Section 7.

## Exact Changes

1. Implement `trimmed_mean(scores: &mut [f64], trim_fraction: f64) -> Option<f64>`.
2. Define `LearnedJudgeWeights { weights: BTreeMap<String, f64>, fit_at_ms: i64, n_canary: u32, r_squared: f64, active: bool }`. Active when >= 500 canary examples.
3. Define `PanelDisagreement { agreement_rate: f64, score_spread: f64, krippendorff_alpha: f64, needs_human_review: bool, reason: Option<String> }`.
4. Implement `detect_disagreement(verdicts: &[PositionSwapResult]) -> PanelDisagreement`:
   - agreement_rate = consistent_judges / total_judges
   - score_spread = max_score - min_score
   - krippendorff_alpha: implement nominal alpha for a small panel
   - Flag for review when agreement < 0.5, spread > 0.3, or alpha < 0.4.

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

- [ ] Test trimmed mean with 5 scores
- [ ] Test disagreement detection with high/low agreement panels

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test trimmed mean with 5 scores
- Test disagreement detection with high/low agreement panels
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
