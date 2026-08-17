# PERF_41: Batch Inference Collector for Plan Execution

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-41`](../ISSUE-TRACKER.md#perf-41)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.41
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_41 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`BatchCollector` that accumulates inference requests from concurrent
plan tasks and dispatches them in parallel, sharing connection resources.

## Exact Changes

1. Create `batch.rs` with:
   - `BatchCollector { pending, batch_window: Duration, max_batch_size: usize }`
   - `submit(request) -> Result<ModelCallResponse>`: queue + wait for batch flush
   - `flush()`: dispatch all pending via `futures::future::join_all()`
2. Auto-flush when `pending.len() >= max_batch_size` or `batch_window` elapses
3. Each request gets individual response via `oneshot::channel`
4. Default: `batch_window = 50ms`, `max_batch_size = 10`
5. Export from `lib.rs`

## Write Scope

- `crates/roko-agent/src/batch.rs`
- `crates/roko-agent/src/lib.rs`

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

- [ ] 5 requests submitted within batch window: dispatched concurrently
- [ ] Each request gets individual response
- [ ] Partial batch flushes on timeout

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_41 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 5 requests submitted within batch window: dispatched concurrently
- Each request gets individual response
- Partial batch flushes on timeout
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_41 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
