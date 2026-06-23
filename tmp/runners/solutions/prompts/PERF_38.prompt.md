# PERF_38: Pareto Frontier Analysis in Bench Results

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-38`](../ISSUE-TRACKER.md#perf-38)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.38
- Priority: **??**
- Effort: ?
- Depends on: `PERF_37` (source 10.37)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_38 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Compute cost-quality Pareto frontier across models.

## Exact Changes

1. Add `pub fn pareto_frontier(results: &[BenchRunResult]) -> Vec<ParetoPoint>`
2. `ParetoPoint { model, pass_rate, avg_cost, avg_latency_ms }`
3. Keep only non-dominated points (no other point has both higher pass rate AND
   lower cost)
4. Sort by cost ascending
5. Include in benchmark suite output

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

- [ ] 5 models: frontier contains 2-4 points (not all 5)
- [ ] Dominated models excluded
- [ ] Sorted by cost ascending

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_38 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 5 models: frontier contains 2-4 points (not all 5)
- Dominated models excluded
- Sorted by cost ascending
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_38 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
