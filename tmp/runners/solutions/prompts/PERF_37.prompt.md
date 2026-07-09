# PERF_37: Wire Cost Tracking Into Bench Results

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-37`](../ISSUE-TRACKER.md#perf-37)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.37
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_37 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `cost_usd` field in `BenchResult` is currently always 0.0.
Connect to the learning subsystem's cost tracking.

## Exact Changes

1. After each bench task, read cost from `ModelCallResponse` or efficiency event
2. Sum costs across all model calls for total `cost_usd`
3. For local models (Ollama): estimate from token counts, or leave 0.0
4. Set `result.cost_usd = total_cost`

## Write Scope

- `crates/roko-cli/src/bench.rs`

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

- [ ] API-backed models: non-zero `cost_usd` in results
- [ ] Local models: `cost_usd = 0.0`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_37 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- API-backed models: non-zero `cost_usd` in results
- Local models: `cost_usd = 0.0`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_37 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
