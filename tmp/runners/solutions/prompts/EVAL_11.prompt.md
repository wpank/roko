# EVAL_11: `FormatCriterion`, `SecurityCriterion`, `DiffCriterion`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-11`](../ISSUE-TRACKER.md#eval-11)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.11
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_07` (source 5.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Three smaller criteria following the same pattern as the above.

## Exact Changes

1. `FormatCriterion`: consumes ProcessOutput from `cargo fmt --check`. Extracts unformatted file paths from diff output. Score: 0.0 if any unformatted files, 1.0 otherwise. Hard severity. Findings list unformatted file paths.
2. `SecurityCriterion`: consumes ProcessOutput from `cargo audit`. Emits Info-level findings when audit tool is missing (do not fail). Parses advisory list when available.
3. `DiffCriterion`: consumes Diff evidence. Analyzes git diff stats (files changed, insertions, deletions). Optionally consumes SemanticDiff evidence for richer analysis. Score = 1.0 (always passes; the criterion is informational). Soft severity.

## Write Scope

- `crates/roko-eval-metrics/src/lib.rs`

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

- [ ] Unit tests for each criterion with mock evidence

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit tests for each criterion with mock evidence
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
