# TEST_25: Learning under concurrent access

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-25`](../ISSUE-TRACKER.md#test-25)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.25
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1), `TEST_05` (source 15.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Addresses AP-NO-CONCURRENT. The plan runner dispatches agents in parallel, each producing episodes and efficiency events. Concurrent writes to JSONL files and JSON state files must not corrupt data.

## Exact Changes

1. Spawn 10 tokio tasks, each appending 100 episodes to the same `EpisodeLogger`. Verify total count is 1000 with no corruption (every line is valid JSON)
2. Spawn 5 tokio tasks, each observing 20 routing outcomes to the same `CascadeRouter`. Verify total observations = 100
3. Test file contention: two `AdaptiveThresholds` instances observing the same file concurrently. Verify no partial writes or corrupted JSON
4. Test JSONL append atomicity: write partial data to simulate an interrupted write, verify reader skips malformed lines (not fatal error)

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

- [ ] No data corruption under concurrent access
- [ ] Total counts match expected values (no lost writes)
- [ ] Malformed JSONL lines are skipped, not fatal
- [ ] All tests use real tokio tasks (not serial simulation)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No data corruption under concurrent access
- Total counts match expected values (no lost writes)
- Malformed JSONL lines are skipped, not fatal
- All tests use real tokio tasks (not serial simulation)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
