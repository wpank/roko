# PERF_20: Export WarmDispatchPool Module

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-20`](../ISSUE-TRACKER.md#perf-20)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.20
- Priority: **??**
- Effort: ?
- Depends on: `PERF_19` (source 10.19)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Add `warm_dispatch_pool` module to crate, export key types.

## Exact Changes

1. Add `pub mod warm_dispatch_pool;` to `lib.rs`
2. Add re-exports:
   `pub use warm_dispatch_pool::{WarmDispatchPool, WarmPoolConfig, WarmPoolMetrics, WarmSlotGuard};`

## Write Scope

- `crates/roko-runtime/src/lib.rs`

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

- [ ] `cargo doc -p roko-runtime` generates docs for new module

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
