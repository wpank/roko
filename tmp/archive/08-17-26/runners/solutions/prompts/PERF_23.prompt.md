# PERF_23: Wire WarmDispatchPool Into `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-23`](../ISSUE-TRACKER.md#perf-23)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.23
- Priority: **??**
- Effort: ?
- Depends on: `PERF_22` (source 10.22)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Construct pool in CLI `run_once()` path, pass to `EffectServices`.
For CLI one-shot: no pre-warm (first request warms HTTP client; second reuses).

## Exact Changes

1. Build `WarmPoolConfig::default()` (no pre-warm for CLI)
2. Create `model_caller_factory` closure from existing `create_agent_for_model`
   or `ModelCallService` constructor
3. Construct `WarmDispatchPool::new(config, Arc::new(factory))`
4. Set `effect_services.warm_pool = Some(Arc::new(pool))`
5. Pool lives for run duration, dropped on completion

## Write Scope

- `crates/roko-cli/src/run.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Standard workflow (2 agent calls): warm slot reused for second call (verify
- [ ] Express workflow (1 call): works correctly

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Standard workflow (2 agent calls): warm slot reused for second call (verify
- Express workflow (1 call): works correctly
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
