# TEST_18: Adaptive threshold parity tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-18`](../ISSUE-TRACKER.md#test-18)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.18
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_04` (source 15.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`AdaptiveThresholds` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs` (line 168, 957 LOC). Uses EMA, CUSUM, and SPC detectors to decide whether to skip gates that consistently pass.

## Exact Changes

1. Create `AdaptiveThresholds::new()`
2. Feed 25 consecutive passes on rung 1 (clippy), verify `should_skip_rung(1)` returns true (or the skip threshold is documented)
3. Feed 1 failure on rung 1, verify skip decision resets (consecutive streak broken)
4. Save to JSON, reload, verify state is identical (EMA, CUSUM, streak all preserved)
5. Test temperament adjustments: Conservative never skips, Aggressive skips earlier
6. Test role-based overrides: high `gate_pass_rate_floor` prevents skipping
7. Test threshold updates: observe 100 outcomes, verify EMA converges
8. Test SPC detector integration: `SpcDetector` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/spc.rs` (line 473) feeds observations, detects change points

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

- [ ] Skip decision matches expected behavior at documented streak thresholds
- [ ] JSON roundtrip preserves all state
- [ ] Temperament and role overrides work

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Skip decision matches expected behavior at documented streak thresholds
- JSON roundtrip preserves all state
- Temperament and role overrides work
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
