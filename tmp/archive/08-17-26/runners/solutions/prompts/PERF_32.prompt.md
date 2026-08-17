# PERF_32: Quality Benchmark Suite Definition

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-32`](../ISSUE-TRACKER.md#perf-32)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.32
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_32 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Quality regression suite with 5 tasks testing code generation, bug
fixing, and refactoring quality.

## Exact Changes

1. Create `quality.json` with 5 tasks:
   - `qual-001`: fix compilation error (type mismatch)
   - `qual-002`: reverse string with Unicode handling
   - `qual-003`: refactor loops to iterators
   - `qual-004`: add error handling
   - `qual-005`: implement a trait
2. Each includes `expected_gates` for automated scoring
3. Self-contained (no external deps)

## Write Scope

- `.roko/bench/suites/quality.json`

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

- [ ] Suite validates against `BenchSuite` schema
- [ ] Each task has clear pass/fail via gate verdicts
- [ ] Tasks cover different agent capabilities

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_32 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Suite validates against `BenchSuite` schema
- Each task has clear pass/fail via gate verdicts
- Tasks cover different agent capabilities
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_32 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
