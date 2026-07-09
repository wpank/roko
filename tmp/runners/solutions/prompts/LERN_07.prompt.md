# LERN_07: Build Full RoutingContext in `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-07`](../ISSUE-TRACKER.md#lern-07)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.7
- Priority: **P1**
- Effort: 4 hours
- Depends on: `LERN_05` (source 7.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`RoutingContext` (at `model_router.rs:130`) has 18 features:
- `task_category: TaskCategory`
- `complexity: ComplexityBand`
- `iteration: u32`
- `role: AgentRole`
- `crate_familiarity: f64`
- `has_prior_failure: bool`
- `conductor_load: f64`
- `active_agents: u32`
- `ready_queue_depth: u32`
- `max_queue_wait_hours: f64`
- `daimon_policy: DaimonPolicy`
- `thinking_level: Option<ThinkingLevel>`
- `temperament: Option<f64>`
- `plan_context_tokens: Option<u32>`
- `tier_thresholds: Option<TierThresholds>`

Currently `CompletedRunInput::from_episode()` (at `runtime_feedback.rs:863`) derives a simplified context where 9 of these features are zeroed.

## Exact Changes

1. Add `routing_context: Option<RoutingContext>` to `CompletedRunInput` struct (at `runtime_feedback.rs:283`).
2. In `roko run`, after resolving the prompt and model, construct a `RoutingContext`:
   - `task_category`: derive from prompt analysis or agent role, default `TaskCategory::Implementation`
   - `complexity`: derive from prompt length/structure, default `ComplexityBand::Standard`
   - `iteration`: retry count (0 for first attempt)
   - `role`: from agent config or `AgentRole::Implementer`
   - `crate_familiarity`: query episode history for success rate in same context (default 0.5)
   - `has_prior_failure`: from retry state
   - `conductor_load`: 0.0 for single-run (accurate for non-orchestrated mode)
   - `active_agents`: 0 for single-run
   - `daimon_policy`: load from `.roko/daimon/affect.json` if exists, else default
3. In `LearningRuntime::record_completed_run()`, if `input.routing_context` is `Some`, use it instead of deriving from the episode.
4. Pass the context through to the CascadeRouter observation.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-learn/src/runtime_feedback.rs`

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

- [ ] Run `roko run` with two different prompts, check `cascade-router.json` observations have distinct context vectors
- [ ] Fields that were previously 0 now have meaningful values

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run `roko run` with two different prompts, check `cascade-router.json` observations have distinct context vectors
- Fields that were previously 0 now have meaningful values
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
