# EVAL_38: Registry client (CLI operations)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-38`](../ISSUE-TRACKER.md#eval-38)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.38
- Priority: **P3**
- Effort: 6 hours
- Depends on: `EVAL_34` (source 5.34), `EVAL_36` (source 5.36)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_38 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. REST client for configurable registry URL.
2. Methods: `publish`, `search`, `download`, `fork`, `yank`.
3. CLI integration stubs for `roko eval publish/search/install/fork`.

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Mock HTTP server test for search and download

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_38 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Mock HTTP server test for search and download
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_38 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
