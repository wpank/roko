# EVAL_43: TUI eval trace widget and evidence browser

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-43`](../ISSUE-TRACKER.md#eval-43)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.43
- Priority: **P2**
- Effort: 6 hours
- Depends on: `EVAL_05` (source 5.5), `EVAL_40` (source 5.40)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_43 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. `EvalTraceWidget`: compact table rendering criterion name, pass/fail, duration, score bar, finding count.
2. `CriterionBarWidget`: horizontal bar for 0.0-1.0 scores with color coding (red < 0.5, yellow 0.5-0.8, green > 0.8).

## Write Scope

- `crates/roko-cli/src/tui/widgets/mod.rs`

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

- [ ] Widget render test for correct line count

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_43 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Widget render test for correct line count
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_43 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
