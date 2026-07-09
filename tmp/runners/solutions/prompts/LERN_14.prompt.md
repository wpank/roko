# LERN_14: Wire Provider Health Circuit Breaker to CascadeRouter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-14`](../ISSUE-TRACKER.md#lern-14)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.14
- Priority: **P1**
- Effort: 3 hours
- Depends on: `LERN_09` (source 7.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ProviderHealthTracker` (at `provider_health.rs`) implements circuit breaker logic per provider. `LearningRuntime` already has `healthy_model_slugs()` (at `runtime_feedback.rs:1466`) that filters models by provider health.

`CascadeRouter` accepts `ProviderHealthRegistry` in helper functions (at `cascade_router.rs:658, 689`) but these are not called from the live selection path in `select_for_frequency_among()`.

## Exact Changes

1. In `roko run`, load `ProviderHealthTracker` from the `LearningRuntime` (it is created during `LearningRuntime::open()`).
2. Before calling `CascadeRouter::select_for_frequency_among()`, filter candidate slugs through `runtime.healthy_model_slugs(&all_slugs, provider_of_fn)`.
3. Pass only healthy models as candidates to the router.
4. After a model call failure, update provider health via `LearningRuntime` (already done in `record_completed_run()` step 3).
5. Log when a provider circuit opens: `warn!(provider = %p, "Circuit breaker open, excluding models")`.

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

- [ ] When a provider has high failure rate, its models are excluded from routing candidates
- [ ] Router falls back to healthy providers
- [ ] Circuit breaker recovery restores models to candidate pool

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- When a provider has high failure rate, its models are excluded from routing candidates
- Router falls back to healthy providers
- Circuit breaker recovery restores models to candidate pool
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
