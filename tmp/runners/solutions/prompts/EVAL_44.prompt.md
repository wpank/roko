# EVAL_44: Web dashboard -- Arena pages (React)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-44`](../ISSUE-TRACKER.md#eval-44)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.44
- Priority: **P3**
- Effort: 12 hours
- Depends on: `EVAL_42` (source 5.42)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_44 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. ArenaOverview: metric cards (total evals, pass rate, mean duration, mean cost) + recent eval timeline.
2. EvalHistory: table with expandable rows showing per-criterion detail.
3. Zustand store subscribing to SSE eval events.

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

- [ ] Component renders without errors

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_44 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Component renders without errors
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_44 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
