# TEST_14: CLI knowledge and learn smoke tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-14`](../ISSUE-TRACKER.md#test-14)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.14
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test `roko knowledge stats` on empty store returns zero counts
2. Test `roko knowledge query "test"` on empty store returns no results (not crash)
3. Test `roko learn all` on empty learn directory returns empty state
4. Test `roko learn router` on empty cascade-router.json returns defaults
5. Test `roko learn experiments` on empty experiments.json returns empty
6. Test `roko learn efficiency` on empty efficiency.jsonl returns empty
7. Test `roko learn episodes` on empty episodes.jsonl returns empty
8. Pre-seed `.roko/learn/efficiency.jsonl` with 3 events, verify `roko learn efficiency` parses them
9. Pre-seed `.roko/episodes.jsonl` with 3 episodes, verify `roko learn episodes` shows counts

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

- [ ] 9 tests, all passing
- [ ] All learn subcommands tested with both empty and seeded data
- [ ] No panics on missing/empty files
- [ ] Tests complete in < 15 seconds total

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 9 tests, all passing
- All learn subcommands tested with both empty and seeded data
- No panics on missing/empty files
- Tests complete in < 15 seconds total
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
