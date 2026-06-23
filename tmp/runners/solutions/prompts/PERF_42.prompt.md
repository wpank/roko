# PERF_42: End-to-End Performance Validation Script

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-42`](../ISSUE-TRACKER.md#perf-42)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.42
- Priority: **??**
- Effort: ?
- Depends on: `PERF_15` (source 10.15), `PERF_30` (source 10.30)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_42 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Script running before/after measurements across model/template/gate
combinations.

## Exact Changes

1. Create `scripts/perf-validate.sh`:
   - Iterate models (gpt-4.1-nano, gpt-4.1-mini)
   - Iterate templates (express, standard)
   - Iterate gates (none, express, full)
   - Run `roko run --model M --workflow-template T --gates G --output json "Reply with only hello"`
   - Capture `/usr/bin/time -l` output for wall clock + peak RSS
   - Store results in `.roko/bench/perf-YYYYMMDD/`
2. `chmod +x scripts/perf-validate.sh`
3. Parse timing output for wall clock, peak RSS, syscall count

## Write Scope

- `scripts/perf-validate.sh`

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

- [ ] Script runs all 12 combinations (2 x 2 x 3)
- [ ] Each produces timing data and JSON output
- [ ] Results directory contains 12+ result files

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_42 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Script runs all 12 combinations (2 x 2 x 3)
- Each produces timing data and JSON output
- Results directory contains 12+ result files
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_42 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
