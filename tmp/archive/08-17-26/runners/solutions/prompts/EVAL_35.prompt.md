# EVAL_35: Dependency resolution and lock file

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-35`](../ISSUE-TRACKER.md#eval-35)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.35
- Priority: **P3**
- Effort: 6 hours
- Depends on: `EVAL_34` (source 5.34)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_35 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Implement dependency resolution: expand transitive deps, unify version ranges, resolve to highest matching stable.
2. Produce `eval.lock` file.
3. Local installation layout: `.roko/eval/installed/<ns>/<name>/<version>/`.

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

- [ ] Test resolution with diamond dependency

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_35 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test resolution with diamond dependency
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_35 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
