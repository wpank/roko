# PERF_22: Wire WarmDispatchPool Into WorkflowEngine

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-22`](../ISSUE-TRACKER.md#perf-22)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.22
- Priority: **??**
- Effort: ?
- Depends on: `PERF_21` (source 10.21)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Pool lifecycle: pre-warm on workflow start, evict on completion, log
metrics.

## Exact Changes

1. In `WorkflowEngine::run()`, if `self.driver.services.warm_pool.is_some()`:
   - Call `pool.pre_warm().await` before main workflow loop
   - Call `pool.evict_idle().await` after workflow completes
2. Log pool metrics at workflow end:
   ```rust
   info!(warm_hits = m.warm_hits, cold_misses = m.cold_misses,
         avg_acquire_us = m.avg_acquire_us, "warm pool stats");
   ```

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`

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

- [ ] Pool metrics logged after workflow run
- [ ] Pre-warm creates slots for configured targets
- [ ] Evict removes idle slots past timeout

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Pool metrics logged after workflow run
- Pre-warm creates slots for configured targets
- Evict removes idle slots past timeout
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
