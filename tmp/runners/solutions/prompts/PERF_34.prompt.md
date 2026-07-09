# PERF_34: Per-PR Performance Check CI Workflow

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-34`](../ISSUE-TRACKER.md#perf-34)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.34
- Priority: **??**
- Effort: ?
- Depends on: `PERF_33` (source 10.33)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_34 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

CI workflow running perf benchmark on every PR, comparing against
main branch baseline.

## Exact Changes

1. Create workflow triggered on `pull_request`:
   - Build release binary
   - Run perf benchmark suite
   - Download main branch baseline from artifact cache
   - Run `roko bench compare`
   - Post comparison as PR comment or fail check
2. Cache main branch results as GitHub Actions artifact
3. On merge to main: update cached baseline

## Write Scope

- `.github/workflows/perf-check.yml`

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

- [ ] PRs get perf regression check reporting overhead changes
- [ ] 20%+ regression fails check (configurable)
- [ ] Main baseline updates on merge

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_34 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- PRs get perf regression check reporting overhead changes
- 20%+ regression fails check (configurable)
- Main baseline updates on merge
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_34 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
