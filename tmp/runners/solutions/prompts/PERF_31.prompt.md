# PERF_31: Performance Benchmark Suite Definition

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-31`](../ISSUE-TRACKER.md#perf-31)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.31
- Priority: **??**
- Effort: ?
- Depends on: `PERF_15` (source 10.15), `PERF_30` (source 10.30)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Benchmark suite with 5 tasks measuring non-inference overhead across
workflow templates.

## Exact Changes

1. Create `perf.json` with 5 benchmark tasks:
   - `perf-001`: minimal prompt, express workflow, no gates (baseline)
   - `perf-002`: single tool call (file write), express, no gates
   - `perf-003`: code edit, express workflow, express gates
   - `perf-004`: code gen, standard workflow, full gates
   - `perf-005`: multi-step, full workflow, express gates
2. All use fast models (gpt-4.1-nano) to isolate framework overhead
3. Each specifies model, workflow template, gate mode

## Write Scope

- `.roko/bench/suites/perf.json`

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

- [ ] Suite definition validates against `BenchSuite` schema
- [ ] Results include per-task wall-clock, inference, and overhead time

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Suite definition validates against `BenchSuite` schema
- Results include per-task wall-clock, inference, and overhead time
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
