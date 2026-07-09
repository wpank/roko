# TEST_17: Gate verdict equivalence tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-17`](../ISSUE-TRACKER.md#test-17)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.17
- Priority: **P0**
- Effort: 4 hours
- Depends on: `TEST_03` (source 15.3), `TEST_04` (source 15.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Create a `GateTestProject` that passes all gates
2. Run through `GateService::run_gates()` -- capture verdicts
3. Run through legacy `run_rung()` / `run_canonical_rung()` path if accessible -- capture verdicts
4. Assert: same pass/fail result per gate for compile, clippy, test
5. Assert: test counts match for TestGate
6. Create a `GateTestProject` that fails compile -- verify both paths report failure
7. Create a `GateTestProject` that fails clippy -- verify both paths report clippy failure
8. Assert: gate ordering is identical (rung 0 before rung 1 before rung 2)
9. Assert: duration is within reasonable bounds (not pathologically different)

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

- [ ] Verdicts match for rungs 0-2 across both passing and failing scenarios
- [ ] Gate ordering is identical
- [ ] Duration measurements are within 2x of each other

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Verdicts match for rungs 0-2 across both passing and failing scenarios
- Gate ordering is identical
- Duration measurements are within 2x of each other
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
