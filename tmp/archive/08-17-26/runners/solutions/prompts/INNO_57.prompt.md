# INNO_57: Wire experiment feedback into CascadeRouter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-57`](../ISSUE-TRACKER.md#inno-57)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.57
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_57 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

ExperimentStore at `crates/roko-learn/src/prompt_experiment.rs` runs A/B
experiments but outcomes are not fed back to CascadeRouter routing weights.

## Exact Changes

1. After an experiment arm concludes, extract the model and prompt variant.
2. Feed the outcome into CascadeRouter as an observation with experiment context.
3. Winning experiment arms boost the associated model's routing weight.
4. Losing arms reduce the weight.

## Write Scope

- `crates/roko-learn/src/cascade_router.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] An experiment with 3 arms on the same task type: after 10 trials, the winning model has the highest routing weight
- [ ] Experiment observations are visible in `cascade-router.json`
- [ ] The experiment store and router are no longer operating independently

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_57 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- An experiment with 3 arms on the same task type: after 10 trials, the winning model has the highest routing weight
- Experiment observations are visible in `cascade-router.json`
- The experiment store and router are no longer operating independently
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_57 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
