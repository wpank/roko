# EVAL_32: Anti-Goodhart safeguards

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-32`](../ISSUE-TRACKER.md#eval-32)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.32
- Priority: **P2**
- Effort: 6 hours
- Depends on: `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_32 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Canary set management and Spearman rho tracking. Integrates with `crates/roko-learn/src/drift.rs` for correlation as drift signal.

## Exact Changes

1. Define `CanarySet { items: Vec<CanaryItem>, metadata: CanaryMetadata }` persisted to `.roko/eval/canary.json`.
2. Define `CanaryItem { id, prompt, human_rating: f64, last_evaluated: Option<DateTime<Utc>> }`.
3. Implement `spearman_rho(x: &[f64], y: &[f64]) -> f64`: Spearman rank correlation.
4. Implement `check_canary_drift(canary: &CanarySet, recent_scores: &[(String, f64)], threshold: f64) -> Option<DriftDetection>`: if rho < threshold (default 0.6), return drift alert.
5. Define `RubricRotationSchedule { current_emphasis: HashMap<String, f64>, last_rotated: DateTime<Utc>, rotation_interval_days: u32 }` for quarterly rubric emphasis changes.

## Write Scope

- `crates/roko-eval/src/lib.rs`

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

- [ ] Test Spearman rho calculation with known ranked lists
- [ ] Test drift detection with correlated and uncorrelated lists

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_32 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test Spearman rho calculation with known ranked lists
- Test drift detection with correlated and uncorrelated lists
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_32 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
