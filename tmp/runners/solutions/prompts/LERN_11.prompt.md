# LERN_11: Wire Conductor Bandit to `roko run` Retry Loop

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-11`](../ISSUE-TRACKER.md#lern-11)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.11
- Priority: **P1**
- Effort: 5 hours
- Depends on: `LERN_09` (source 7.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ConductorBandit` (at `conductor.rs:110`) manages retry decisions with 7 actions: Continue, InjectHint(ErrorDigest), InjectHint(SkillSuggestion), InjectHint(SimplifyApproach), SwitchModel, Restart, Abort. It uses Thompson+linear blended scoring over a 19-dimension context.

`ConductorState` (at `conductor.rs:40`) requires: `iteration`, `consecutive_failures`, `error_pattern` (one-hot for 10 ErrorPattern variants), `elapsed_ms`, `cost_so_far_usd`, `model_tier`, `task_complexity`.

`ConductorBandit::load_or_new(path)` (at `conductor.rs:146`) loads from `.roko/learn/conductor.json`.

Currently `roko run` has no retry loop -- it runs once and returns. But the `WorkflowEngine` (used from `run.rs`) supports retry via `WorkflowRunConfig`. The conductor should be consulted when the engine decides whether to retry.

## Exact Changes

1. Load `ConductorBandit::load_or_new(&roko_dir.join("learn/conductor.json"))` at run initialization.
2. After a task fails (gate failure or agent error), build `ConductorState`:
   - `iteration`: current retry count
   - `consecutive_failures`: count of consecutive failures
   - `error_pattern`: classify from gate error output (use `ErrorPattern` enum variants)
   - `elapsed_ms`: wall clock since task start
   - `cost_so_far_usd`: accumulated cost from `AgentResult` usage
   - `model_tier`: hash of current model slug
   - `task_complexity`: from `RoutingContext.complexity`
3. Call `bandit.select_action(&state)` to get `ConductorAction`.
4. Execute the action:
   - `Continue`: proceed with retry as normal
   - `InjectHint(ErrorDigest)`: append error summary to next prompt
   - `InjectHint(SkillSuggestion)`: query `SkillLibrary` for matching skills, inject into prompt
   - `InjectHint(SimplifyApproach)`: add simplification directive to prompt
   - `SwitchModel`: request a different model from `CascadeRouter` (exclude current model from candidates)
   - `Restart`: reset task state, retry from clean start
   - `Abort`: mark task as failed, stop retrying
5. After retry outcome, call `bandit.record_outcome(&state, action, reward)` where reward is computed from success/failure.
6. Save bandit state via `bandit.save(&conductor_path)`.

## Write Scope

- `crates/roko-cli/src/run.rs`

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

- [ ] Run a task that fails, verify `conductor.json` observation count > 0
- [ ] After 20+ failing tasks, conductor starts selecting non-Continue actions
- [ ] Conductor decisions are logged at INFO level

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run a task that fails, verify `conductor.json` observation count > 0
- After 20+ failing tasks, conductor starts selecting non-Continue actions
- Conductor decisions are logged at INFO level
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
