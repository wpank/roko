# TEST_13: CLI plan lifecycle smoke tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-13`](../ISSUE-TRACKER.md#test-13)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.13
- Priority: **P0**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test `roko plan create <name>` creates plan directory with plan.md
2. Test `roko plan list` shows created plans
3. Test `roko plan show <name>` displays plan details
4. Test `roko plan validate <dir>` on valid tasks.toml returns success
5. Test `roko plan validate <dir>` on invalid TOML returns parse error
6. Test `roko plan validate <dir>` on TOML with missing required fields returns specific error
7. Test `roko plan validate <dir>` on TOML with circular dependencies returns cycle error
8. Test `roko plan validate <dir>` on TOML wrapped in markdown fences strips fences and validates

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

- [ ] 8 tests, all passing
- [ ] Plan validation catches all known error classes
- [ ] Markdown fence stripping works
- [ ] Tests complete in < 20 seconds total

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 8 tests, all passing
- Plan validation catches all known error classes
- Markdown fence stripping works
- Tests complete in < 20 seconds total
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
