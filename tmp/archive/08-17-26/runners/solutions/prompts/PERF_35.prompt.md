# PERF_35: Nightly HAL Benchmark CI Workflow

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-35`](../ISSUE-TRACKER.md#perf-35)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.35
- Priority: **??**
- Effort: ?
- Depends on: `PERF_29` (source 10.29)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_35 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Nightly workflow running roko through HAL's SWE-bench mini evaluation
and tracking quality over time.

## Exact Changes

1. Create workflow on `schedule` (daily 2AM UTC) + `workflow_dispatch`:
   - Build release binary
   - Install `hal-harness` via pip
   - Run `hal-eval` on SWE-bench mini (50 tasks)
   - Upload results as artifacts
   - Compare with previous nightly
2. `max_concurrent: 5` for cost control
3. Default model: `gpt-4.1-mini`

## Write Scope

- `.github/workflows/hal-bench.yml`

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

- [ ] Nightly runs and produces HAL results
- [ ] Results include per-task pass/fail, cost, duration
- [ ] Budget does not exceed ~$20/night

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_35 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Nightly runs and produces HAL results
- Results include per-task pass/fail, cost, duration
- Budget does not exceed ~$20/night
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_35 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
