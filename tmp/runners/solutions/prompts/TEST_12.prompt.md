# TEST_12: CLI init and config smoke tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-12`](../ISSUE-TRACKER.md#test-12)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.12
- Priority: **P0**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test `roko init <tmpdir>`: creates `.roko/`, `roko.toml`, `.roko/engrams.jsonl`, `.roko/learn/`
2. Test `roko init --demo <tmpdir>`: seeds demo data (additional files/directories)
3. Test `roko config show` outputs valid TOML (contains `[agent]` section)
4. Test `roko config path` outputs the config file path (non-empty, ends with `roko.toml`)
5. Test `roko config providers list` shows configured providers
6. Test `roko config models list` shows configured models
7. Test `roko config validate` on a valid config returns success (exit 0)
8. Test `roko config validate` on an invalid config returns error with validation message
9. Test `roko doctor` runs without panic and reports status
10. Test `roko --version` outputs version string matching `roko X.Y.Z`

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

- [ ] 10 tests, all passing
- [ ] Every test uses isolated tempdir
- [ ] Tests pass without API keys
- [ ] Tests complete in < 30 seconds total

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 10 tests, all passing
- Every test uses isolated tempdir
- Tests pass without API keys
- Tests complete in < 30 seconds total
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
