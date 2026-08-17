# TEST_36: CLI stderr/stdout separation tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-36`](../ISSUE-TRACKER.md#test-36)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.36
- Priority: **P1**
- Effort: 2 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_36 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test `roko --help`: output on stdout, nothing (or only log lines) on stderr
2. Test `roko status --json`: JSON on stdout, any logs on stderr only
3. Test `roko plan validate <invalid>`: error message on stderr, nothing meaningful on stdout
4. Test `roko config show`: config on stdout, nothing on stderr
5. Test `roko run` (no args): error on stderr, usage hint on stderr
6. Test `--quiet` flag (if exists): suppresses informational output but not errors

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

- [ ] Normal output goes to stdout
- [ ] Errors go to stderr
- [ ] JSON output on stdout is not mixed with log lines

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_36 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Normal output goes to stdout
- Errors go to stderr
- JSON output on stdout is not mixed with log lines
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_36 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
