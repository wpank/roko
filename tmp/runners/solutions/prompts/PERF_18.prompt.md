# PERF_18: Parallel Gate Rungs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-18`](../ISSUE-TRACKER.md#perf-18)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.18
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Run independent gate rungs concurrently. Compile (0) + diff (3) +
fmt (4) are independent and can run in parallel. Clippy (1) and test (2) depend
on compile passing.

## Exact Changes

1. In `run_gates()`, group rungs by dependency:
   - Parallel set 1: {0 compile, 3 diff, 4 fmt}
   - Sequential set 2 (if compile passed): {1 clippy}
   - Sequential set 3 (if compile passed): {2 test}
   - Sequential set 4: {5 custom/shell, 6 judge}
2. Execute parallel sets with `futures::future::join_all()` or `tokio::join!`
3. If any gate in set 1 fails, still report all set 1 results but skip sets 2-4
4. Preserve existing short-circuit and adaptive threshold skip logic

## Write Scope

- `crates/roko-gate/src/gate_service.rs`

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

- [ ] Wall-clock gate phase time reduced when compile and fmt run in parallel
- [ ] Gate verdicts identical to sequential execution
- [ ] If compile fails, clippy and test are still skipped

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Wall-clock gate phase time reduced when compile and fmt run in parallel
- Gate verdicts identical to sequential execution
- If compile fails, clippy and test are still skipped
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
