# EVAL_19: Bradley-Terry MLE with Davidson ties

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-19`](../ISSUE-TRACKER.md#eval-19)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.19
- Priority: **P1**
- Effort: 8 hours
- Depends on: `EVAL_18` (source 5.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The core statistical model. BT MLE via logistic regression with high regularization (C=10^6). Davidson tie parameter (nu). Elo scale mapping: `Elo_i = theta_i * 400 / ln(10)`.

## Exact Changes

1. Define `ComparisonTriple { candidate_a: String, candidate_b: String, outcome: ComparisonOutcome }` where `ComparisonOutcome` is `APreferred | BPreferred | Tie`.
2. Define `BtResult { elo_scores: BTreeMap<String, f64>, tie_parameter: f64, comparison_count: u32, log_likelihood: f64 }`.
3. Implement BT MLE fitting:
   - Construct the logistic regression problem: for each triple, create a row in X with +1 for candidate_a, -1 for candidate_b.
   - Solve via iterative reweighted least squares (IRLS) with regularization.
   - Map fitted theta values to Elo scale.
   - Include Davidson tie parameter when ties are present.
4. BCa bootstrap confidence intervals (B=1000 resamples) are an optional extension (can be a `compute_confidence_intervals()` method that is expensive to call).

## Design Guidance

For the IRLS solver, use a simple iterative approach rather than pulling in `nalgebra` as a hard dependency. The problem is small (typically 2-5 candidates). 20 iterations of Newton-Raphson suffice. Regularize by adding C * theta to the gradient and C to the Hessian diagonal.

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

- [ ] Known-answer test: feed 10 comparisons where A always beats B, verify Elo_A > Elo_B by >200 points
- [ ] Test with ties: verify tie_parameter > 0 when ties are present
- [ ] Test with equal outcomes: verify Elo scores are approximately equal

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Known-answer test: feed 10 comparisons where A always beats B, verify Elo_A > Elo_B by >200 points
- Test with ties: verify tie_parameter > 0 when ties are present
- Test with equal outcomes: verify Elo scores are approximately equal
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
