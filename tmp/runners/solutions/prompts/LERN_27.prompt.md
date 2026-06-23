# LERN_27: Wire Curriculum Ordering for Plan Execution

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-27`](../ISSUE-TRACKER.md#lern-27)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.27
- Priority: **P3**
- Effort: 3 hours
- Depends on: `LERN_09` (source 7.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CurriculumScheduler` (at `curriculum.rs:116`) reorders tasks by difficulty. `DifficultyModel` (at line 58) learns task difficulty from outcomes. `reorder_tasks()` (at line 492) takes tasks and a difficulty model.

This applies to plan execution (multi-task), not single `roko run`. The entry point is the runner/plan execution path.

## Exact Changes

1. In the plan execution path (likely `runner/` or `plan run` command), before executing tasks:
2. Load `DifficultyModel` from `.roko/learn/curriculum.json` (or create new).
3. Call `reorder_tasks(&tasks, &model)` to sort tasks by difficulty (easier first).
4. Execute in curriculum order to build routing signal from simpler tasks before attempting complex ones.
5. After each task, call `model.observe(&task, success)` to update difficulty estimates.
6. Save model state after plan completion.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-learn/src/curriculum.rs`

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

- [ ] Plan with mixed-difficulty tasks executes simpler tasks first
- [ ] Difficulty model accumulates observations across plan runs
- [ ] Task order changes as model learns from outcomes

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Plan with mixed-difficulty tasks executes simpler tasks first
- Difficulty model accumulates observations across plan runs
- Task order changes as model learns from outcomes
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
