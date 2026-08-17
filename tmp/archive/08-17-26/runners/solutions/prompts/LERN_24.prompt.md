# LERN_24: Wire Force-Backend Override Learning

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-24`](../ISSUE-TRACKER.md#lern-24)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.24
- Priority: **P3**
- Effort: 3 hours
- Depends on: `LERN_09` (source 7.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CascadeRouter` implements `ForceBackendOverrideRecorder` trait (at `cascade_router.rs:134`) with `record_override_outcome(model_slug, success) -> bool`. This is already implemented: it updates the static table with `OVERRIDE_LEARNING_RATE` and returns whether the override was a "surprise" (router would have chosen differently).

But `roko run` does not detect when the user specifies `--model` and does not call `record_override_outcome()`.

## Exact Changes

1. In `roko run`, detect when the user provides `--model` flag (force backend override).
2. After the task completes, if a model override was active, call `router.record_override_outcome(&model_slug, success)`.
3. Log the result: if the override was a "surprise", note it: `"Override learning: router would have chosen {other_model}, user forced {model} (success={success})"`.
4. After N successful overrides for a pattern, the router's static table is updated and the router starts choosing that model automatically.
5. Save router state after override recording.

## Write Scope

- `crates/roko-cli/src/run.rs`
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

- [ ] Use `--model opus` flag 10 times, verify override count in `cascade-router.json`
- [ ] Router static table entry for the override pattern shows updated weights
- [ ] Without `--model`, router incorporates override learnings into selection

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Use `--model opus` flag 10 times, verify override count in `cascade-router.json`
- Router static table entry for the override pattern shows updated weights
- Without `--model`, router incorporates override learnings into selection
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
