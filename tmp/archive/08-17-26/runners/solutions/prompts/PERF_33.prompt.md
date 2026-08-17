# PERF_33: Benchmark Comparison Command

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-33`](../ISSUE-TRACKER.md#perf-33)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.33
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_33 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko bench compare <baseline> <current>` subcommand that compares
two benchmark result files and reports regressions.

## Exact Changes

1. Add `bench compare` subcommand to CLI
2. Load two JSON result files
3. Per-task comparison:
   - Wall-clock: flag if current > baseline * 1.2 (20% regression)
   - Overhead: flag if non-inference time increased
   - Gate pass rate: flag if any passing gate now fails
4. Output comparison table to stdout
5. Exit code 1 if any regression exceeds threshold
6. `--threshold <percent>` flag for custom tolerance

## Write Scope

- `crates/roko-cli/src/bench.rs`
- `crates/roko-cli/src/main.rs`

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

- [ ] `roko bench compare a.json b.json` outputs comparison table
- [ ] Exit 0 if no regressions, 1 if threshold exceeded
- [ ] `--threshold 50` allows up to 50% before failing

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_33 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko bench compare a.json b.json` outputs comparison table
- Exit 0 if no regressions, 1 if threshold exceeded
- `--threshold 50` allows up to 50% before failing
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_33 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
