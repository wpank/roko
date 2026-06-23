# LERN_19: Wire Experiment Winner Auto-Application

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-19`](../ISSUE-TRACKER.md#lern-19)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.19
- Priority: **P2**
- Effort: 3 hours
- Depends on: `LERN_05` (source 7.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ExperimentStore::apply_winners()` (at `prompt_experiment.rs:522`) already exists and writes winning variants to a static overrides path. `apply_winners_to()` (at line 532) writes to a specified path. But no startup code loads and applies winners.

The `experiment-winners.json` file is written when an experiment concludes (Wilson CI convergence). Winners are `ExperimentWinner` with `experiment_id`, `winning_variant`, `value`, `confidence`.

## Exact Changes

1. At `roko run` startup, load `experiment-winners.json` from `.roko/learn/`.
2. If winners exist and `auto_apply_winners` config is true (default true), call `store.apply_winners(&winners)`.
3. The applied overrides should influence prompt assembly (applied as static section overrides).
4. Log auto-applied winners at INFO: `"Auto-applied experiment winner: {experiment_id} -> {variant}"`.
5. Add `learning.auto_apply_winners: bool` to config schema (default true).
6. Guard with config check -- skip if disabled.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-learn/src/prompt_experiment.rs`

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

- [ ] Manually conclude an experiment (set stats to trigger convergence), restart, verify winner is auto-applied
- [ ] `auto_apply_winners = false` skips application
- [ ] Log entry confirms auto-application

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Manually conclude an experiment (set stats to trigger convergence), restart, verify winner is auto-applied
- `auto_apply_winners = false` skips application
- Log entry confirms auto-application
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
