# TEST_28: Startup latency regression tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-28`](../ISSUE-TRACKER.md#test-28)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.28
- Priority: **P1**
- Effort: 2 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test `roko --version` completes in < 500ms (cold start)
2. Test `roko --help` completes in < 500ms
3. Test `roko status --workdir <tmpdir>` completes in < 2s (includes config loading)
4. Test `roko config show --workdir <tmpdir>` completes in < 1s
5. Run each measurement 3 times, take median to reduce variance

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

- [ ] All measurements within baseline thresholds
- [ ] Median of 3 runs used
- [ ] Thresholds are generous for CI runners (2x local dev machine)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All measurements within baseline thresholds
- Median of 3 runs used
- Thresholds are generous for CI runners (2x local dev machine)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
