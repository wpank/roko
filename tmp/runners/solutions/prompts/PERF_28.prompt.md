# PERF_28: PGO CI Integration

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-28`](../ISSUE-TRACKER.md#perf-28)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.28
- Priority: **??**
- Effort: ?
- Depends on: `PERF_27` (source 10.27)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Add PGO build step to release workflow. Published binaries are
profile-optimized.

## Exact Changes

1. Create `.github/workflows/pgo-build.yml` triggered on release tags:
   - Install `llvm-tools-preview`
   - Build instrumented binary
   - Run representative workloads (config show, plan validate, status)
   - Merge profiles
   - Rebuild with PGO data
   - Upload PGO binary as release artifact
2. Keep standard non-PGO build as fallback
3. Compare PGO vs non-PGO binary sizes

## Write Scope

- `.github/workflows/pgo-build.yml`

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

- [ ] CI produces PGO binary on release tags
- [ ] PGO failure does not block release (fallback to standard build)
- [ ] Release notes indicate whether binary is PGO-optimized

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- CI produces PGO binary on release tags
- PGO failure does not block release (fallback to standard build)
- Release notes indicate whether binary is PGO-optimized
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
