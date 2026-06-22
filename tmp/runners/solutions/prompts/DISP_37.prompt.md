# DISP_37: Add Cache Hit Rate Metrics to CacheCell

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-37`](../ISSUE-TRACKER.md#disp-37)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.37
- Priority: **P3**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_37 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CacheCell` (L1 response cache, 128 entries, exact match) in `ModelCallService` has no metrics. Cannot measure cache hit rate, size, or savings.

## Exact Changes

1. Add counters to `CacheCell`: `hits: AtomicU64`, `misses: AtomicU64`, `evictions: AtomicU64`
2. Add `pub fn metrics(&self) -> CacheMetrics` returning a snapshot
3. Emit cache metrics via `RuntimeEvent` periodically or on session end
4. Log cache hit rate at `tracing::debug!` level on each lookup

## Design Guidance

Use atomics for thread-safe counting without locks. The metrics snapshot should be cheap to produce. Include hit rate percentage and estimated cost savings (hits * average call cost) in the metrics.

## Write Scope

- `crates/roko-agent/src/model_call_service.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After N calls with some duplicates, `CacheMetrics.hits` > 0
- [ ] Hit rate is calculated correctly: `hits / (hits + misses)`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_37 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After N calls with some duplicates, `CacheMetrics.hits` > 0
- Hit rate is calculated correctly: `hits / (hits + misses)`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_37 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
