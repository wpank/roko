# TEST_15: CLI explain, status, and utility smoke tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-15`](../ISSUE-TRACKER.md#test-15)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.15
- Priority: **P1**
- Effort: 2 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test `roko explain agent` outputs concept explanation (non-empty stdout)
2. Test `roko explain gate` outputs gate concept explanation
3. Test `roko explain agent --depth deep` produces longer output than default
4. Test `roko status` on initialized workspace shows signal/episode counts
5. Test `roko status --surfaces` shows surface inventory
6. Test `roko history list` on workspace with no sessions returns empty (not crash)
7. Test `roko completions bash` outputs bash completion script (contains `_roko` or `roko`)
8. Test `roko completions zsh` outputs zsh completion script

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
- [ ] Explain depth levels (brief, standard, deep) tested
- [ ] Completion scripts are non-empty
- [ ] Tests complete in < 10 seconds total

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 8 tests, all passing
- Explain depth levels (brief, standard, deep) tested
- Completion scripts are non-empty
- Tests complete in < 10 seconds total
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
