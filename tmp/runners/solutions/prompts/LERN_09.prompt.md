# LERN_09: Wire CascadeRouter Model Selection in `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-09`](../ISSUE-TRACKER.md#lern-09)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.9
- Priority: **P1**
- Effort: 4 hours
- Depends on: `LERN_07` (source 7.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CascadeRouter::select_for_frequency_among()` (at `cascade_router.rs:329`) takes a `RoutingContext` and candidate model slugs, returns a model slug. It transitions through 3 stages: Static (< 50 obs), Confidence (50-200), UCB (200+).

Currently `roko run` uses the model from config (`resolve_effective_model()` at `run.rs:20`). There is no router consultation.

`dispatch_direct.rs` handles the actual agent dispatch. It could accept a model override parameter.

## Exact Changes

1. After loading `CascadeRouter` (from Task 7.5) and building `RoutingContext` (from Task 7.7), call `router.select_for_frequency_among(&ctx, &candidate_slugs)`.
2. `candidate_slugs` = all configured model slugs from `Config.agent.models` or provider config.
3. Use the router-selected model instead of the config default, unless the user specified `--model` (force override).
4. Log the routing decision: `info!(model = %selected, stage = ?stage, "CascadeRouter selected model")`.
5. Fall back to config default if router returns no candidate or if `candidate_slugs` is empty.
6. Thread the selected model through to `dispatch_direct.rs` dispatch call.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/dispatch_direct.rs`

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

- [ ] Run 60+ tasks, observe CascadeRouter `stage` transitions in logs
- [ ] `cascade-router.json` shows increasing observation counts
- [ ] User `--model` flag still overrides router selection

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run 60+ tasks, observe CascadeRouter `stage` transitions in logs
- `cascade-router.json` shows increasing observation counts
- User `--model` flag still overrides router selection
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
