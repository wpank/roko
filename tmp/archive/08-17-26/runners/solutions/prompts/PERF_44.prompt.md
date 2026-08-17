# PERF_44: Fill BenchmarkRegressionGate Implementation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-44`](../ISSUE-TRACKER.md#perf-44)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.44
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_44 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Replace stub with baseline capture, storage, and comparison logic.
Currently `verify()` always returns `Verdict::pass` (line 74: "Stub: no baseline
infrastructure yet").

## Exact Changes

1. Implement baseline capture:
   - After successful benchmark, store timing in
     `.roko/state/bench-baselines/<gate-name>.json`
   - Format: `{ task_id, wall_ms, overhead_ms, tokens, timestamp }`
2. Comparison logic in `verify()`:
   - Load baseline for current task
   - If current > baseline * (1 + threshold_pct/100): `Verdict::fail`
   - If no baseline: pass and capture baseline (first run)
3. Never skip re-check after previous failure

## Write Scope

- `crates/roko-gate/src/benchmark_gate.rs`

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

- [ ] First run: passes, creates baseline file
- [ ] Second run (same perf): passes
- [ ] Third run (injected 30% slowdown): fails with regression message

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_44 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- First run: passes, creates baseline file
- Second run (same perf): passes
- Third run (injected 30% slowdown): fails with regression message
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_44 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
