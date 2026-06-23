# TEST_24: Learning artifact roundtrip tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-24`](../ISSUE-TRACKER.md#test-24)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.24
- Priority: **P0**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1), `TEST_05` (source 15.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Addresses AP-NO-ROUNDTRIP. Every learning artifact type must survive write -> drop -> reload -> verify cycle.

## Exact Changes

1. Test `cascade-router.json`: write with observations across 3 models, drop the `CascadeRouter`, create new one from same file, verify observation counts preserved and routing decisions consistent
2. Test `gate-thresholds.json`: write with per-rung EMA values, reload, verify EMA and CUSUM state preserved
3. Test `section-effects.json`: write section weights, reload, verify weights and counts match
4. Test `episodes.jsonl`: append 10 episodes in one logger instance, create new logger instance on same file, append 10 more, verify all 20 present and ordered
5. Test `efficiency.jsonl`: same append-across-restarts pattern
6. Test `playbooks/`: write 3 playbook files, create new `PlaybookStore` on same directory, verify all 3 queryable
7. Test `costs.jsonl` (if exists): append cost records, reload, verify totals
8. Test affect state (if applicable): write DaimonState, reload, verify fields

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

- [ ] All artifact types survive the roundtrip
- [ ] No data loss across "process restarts" (new instances from same files)
- [ ] JSONL files support append without corrupting previous entries
- [ ] JSON files are well-formed after write

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All artifact types survive the roundtrip
- No data loss across "process restarts" (new instances from same files)
- JSONL files support append without corrupting previous entries
- JSON files are well-formed after write
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
