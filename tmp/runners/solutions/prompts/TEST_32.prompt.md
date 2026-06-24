# TEST_32: State persistence under failure

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-32`](../ISSUE-TRACKER.md#test-32)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.32
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_32 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test crash-during-save: write executor state, corrupt the file (truncate to half), verify `load_state()` detects corruption and falls back to initial state (not panic)
2. Test missing state file: verify `load_state()` on missing file returns clean initial state
3. Test state with 100 completed tasks: verify save/load roundtrip preserves all task statuses
4. Test concurrent save: two instances saving to the same file simultaneously, verify last-writer-wins (no interleaved data)
5. Test malformed JSON recovery: write `{` to state file, verify load handles gracefully

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

- [ ] Corrupted state files produce fallback, not panic
- [ ] No data loss under normal roundtrip
- [ ] Concurrent writes do not produce corrupted output

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_32 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Corrupted state files produce fallback, not panic
- No data loss under normal roundtrip
- Concurrent writes do not produce corrupted output
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_32 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
