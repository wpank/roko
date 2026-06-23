# PERF_24: Wire WarmDispatchPool Into `roko serve`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-24`](../ISSUE-TRACKER.md#perf-24)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.24
- Priority: **??**
- Effort: ?
- Depends on: `PERF_23` (source 10.23)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Long-running server benefits most. Pre-warm on startup, periodic
eviction via background task.

## Exact Changes

1. Read `WarmPoolConfig` from `roko.toml` `[conductor.warm_pool]` section
   (or defaults)
2. Construct `WarmDispatchPool` with config, pre-warm on startup
3. Spawn periodic eviction task: `tokio::spawn` with 60s interval calling
   `pool.evict_idle().await`
4. Share pool with route handlers that dispatch agent work

## Write Scope

- `crates/roko-serve/src/embedded.rs`

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

- [ ] `roko serve` starts with warm slots pre-created (startup log)
- [ ] After 5+ minutes idle, warm slots evicted
- [ ] Multiple concurrent API requests reuse warm slots

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko serve` starts with warm slots pre-created (startup log)
- After 5+ minutes idle, warm slots evicted
- Multiple concurrent API requests reuse warm slots
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
