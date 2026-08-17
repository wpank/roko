# TEST_27: Memory usage regression tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-27`](../ISSUE-TRACKER.md#test-27)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.27
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1), `TEST_21` (source 15.21)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Addresses the 9.5-11.5GB RSS leak from dogfood sessions. Unbounded vectors in efficiency events and enrichment artifacts were identified as contributors.

## Exact Changes

1. Test efficiency events vector is bounded: append 10000 events, verify the in-memory vector does not exceed a configured cap (e.g., 1000 entries). After flush, verify vector is cleared.
2. Test episode logger does not accumulate in memory: write 10000 episodes, verify memory stays flat (each write flushes to disk).
3. Test enrichment context is dropped after use: build enrichment for a task, verify the string is not held after dispatch context goes out of scope.
4. Test executor state serialization does not grow unboundedly: 100-task plan with all tasks completed, verify serialized state JSON is < 1MB.

## Design Guidance

Memory measurement on macOS: use `mach_task_info` via the `mach2` crate or parse `ps -o rss` output. On Linux: parse `/proc/self/status` for VmRSS. For cross-platform: use `std::alloc::GlobalAlloc` wrapper that tracks high-water mark.

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] No unbounded vector growth (verified by size checks, not just compilation)
- [ ] Baselines are documented in test comments
- [ ] Tests pass on both macOS and Linux

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No unbounded vector growth (verified by size checks, not just compilation)
- Baselines are documented in test comments
- Tests pass on both macOS and Linux
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
