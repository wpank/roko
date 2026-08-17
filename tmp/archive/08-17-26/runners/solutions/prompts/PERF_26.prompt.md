# PERF_26: Speculative Pre-Warming in Workflow Engine

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-26`](../ISSUE-TRACKER.md#perf-26)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.26
- Priority: **??**
- Effort: ?
- Depends on: `PERF_22` (source 10.22)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

While implementer runs, speculatively pre-warm reviewer's model caller
for instant acquisition on implementation completion.

## Exact Changes

1. After spawning implementer in workflow loop:
   - Check if template includes review phase (standard/full)
   - If yes and pool has idle capacity: `tokio::spawn(pool.pre_warm_for(...))`
2. Add `pub async fn pre_warm_for(&self, provider: &str, model: &str)` to
   `WarmDispatchPool` -- creates a single warm slot for the given pair
3. When reviewer dispatched, `pool.acquire()` finds pre-warmed slot (warm hit)
4. If implementation fails (no review): slot sits idle, evicted after timeout

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/warm_dispatch_pool.rs`

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

- [ ] Standard workflow: reviewer acquisition <5ms (warm hit from speculation)
- [ ] Express workflow: no speculation (no reviewer phase)
- [ ] Failed implementation: pre-warmed slot eventually evicted, not leaked
- [ ] Pool metrics show speculative warm hits

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Standard workflow: reviewer acquisition <5ms (warm hit from speculation)
- Express workflow: no speculation (no reviewer phase)
- Failed implementation: pre-warmed slot eventually evicted, not leaked
- Pool metrics show speculative warm hits
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
