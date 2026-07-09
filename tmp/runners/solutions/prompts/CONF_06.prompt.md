# CONF_06: Wire `agent.tier_models` Into Live Dispatch

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-06`](../ISSUE-TRACKER.md#conf-06)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.6
- Priority: **P2**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`agent.tier_models` maps task tiers to model slugs. The mapping is loaded into
`CascadeRouter` via `model_slugs_for_config()` at `service_factory.rs:251` and
`model_call_service.rs:814`, but the tier routing path is never called at dispatch
time in runner v2. All tasks use the default model regardless of their declared tier.

The legacy orchestrate.rs path DOES use tier models (confirmed at lines 989, 5367,
9842, 10301, 11720, 12837, 13314, 13634), calling `task.effective_model()` with
`tier_models` from config. Runner v2 does not.

## Exact Changes

1. In `RunnerConfig`, store `tier_models: HashMap<String, String>` populated from
   `roko_config.agent.tier_models`.
2. At dispatch time in the event loop, call `task.effective_model(&default_model,
   Some(&config.tier_models))` to resolve the model for the current task.
3. Pass the resolved model to the agent spawn command instead of always using the
   default.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/task_parser.rs`
- `crates/roko-cli/src/runner/types.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Setting `[agent.tier_models]` with `T3 = "claude-opus-4-6"` causes T3 tasks to

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Setting `[agent.tier_models]` with `T3 = "claude-opus-4-6"` causes T3 tasks to
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
