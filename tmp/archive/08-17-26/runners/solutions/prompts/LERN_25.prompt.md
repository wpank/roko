# LERN_25: Wire Pareto Frontier Active Use in CascadeRouter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-25`](../ISSUE-TRACKER.md#lern-25)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.25
- Priority: **P3**
- Effort: 2 hours
- Depends on: `LERN_09` (source 7.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The CascadeRouter already computes and caches a Pareto frontier (recomputed every `PARETO_RECOMPUTE_INTERVAL` observations). Helper functions for `pareto_adjusted_alpha()` exist but are not called from the live selection path in `select_for_frequency_among()`.

## Exact Changes

1. In `select_for_frequency_among()` (at `cascade_router.rs:329`), after UCB scoring, apply `pareto_adjusted_alpha()` to down-weight models dominated on the Pareto frontier (higher cost AND lower success than another model).
2. This should only activate once the router has enough observations for a meaningful frontier (> 100 observations total).
3. Log when a model is deprioritized due to Pareto dominance.

## Write Scope

- `crates/roko-learn/src/cascade_router.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After 100+ observations, dominated models get lower selection probability
- [ ] Pareto frontier computation visible in logs
- [ ] Non-dominated models are not penalized

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 100+ observations, dominated models get lower selection probability
- Pareto frontier computation visible in logs
- Non-dominated models are not penalized
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
