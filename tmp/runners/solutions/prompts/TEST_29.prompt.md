# TEST_29: Gate pipeline throughput tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-29`](../ISSUE-TRACKER.md#test-29)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.29
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_03` (source 15.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Tests the overhead of gate infrastructure itself, separate from the cost of running cargo commands.

## Exact Changes

1. Test `GateService` with 3 mock shell gates (`true`): 100 iterations, assert total time < 1s (pipeline overhead < 10ms per run)
2. Test `AdaptiveThresholds` update speed: 10000 observations, assert total time < 1s
3. Test `SpcDetector` update speed: 10000 observations, assert < 2s (BOCPD is more expensive)
4. Test `ComposedGatePipeline` with `ParallelGate(3 mock gates)`: verify parallel execution is faster than sequential
5. Test `feedback_for_agent()` parsing speed: 1000 lines of mixed compile output, assert < 100ms

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

- [ ] Pipeline overhead is measurable and bounded
- [ ] Parallel gate execution provides speedup over sequential
- [ ] Statistical detectors scale linearly with observations

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Pipeline overhead is measurable and bounded
- Parallel gate execution provides speedup over sequential
- Statistical detectors scale linearly with observations
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
