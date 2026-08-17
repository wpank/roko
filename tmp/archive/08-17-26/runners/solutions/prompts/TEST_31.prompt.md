# TEST_31: Gate failure edge cases

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-31`](../ISSUE-TRACKER.md#test-31)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.31
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_03` (source 15.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test missing cargo binary: set PATH to empty, verify `CompileGate` returns clear "cargo not found" or "command not found" error (not raw OS error)
2. Test empty project: run gates on a directory with no `Cargo.toml`, verify clear error message
3. Test very large output: mock shell gate that produces 1MB of stderr, verify output is truncated and not OOM
4. Test shell gate with non-UTF-8 output: verify output is handled (lossy conversion, not panic)
5. Test concurrent gate execution: run 3 `CompileGate` instances on the same `GateTestProject` simultaneously, verify no interference
6. Test gate timeout: configure a shell gate with 1-second timeout and a command that sleeps 5 seconds, verify timeout

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

- [ ] Every edge case produces a clear error message
- [ ] No panics, no OOM, no hangs
- [ ] Concurrent execution is safe

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every edge case produces a clear error message
- No panics, no OOM, no hangs
- Concurrent execution is safe
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
