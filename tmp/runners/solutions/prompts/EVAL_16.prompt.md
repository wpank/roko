# EVAL_16: `SemanticDiffCollector` and `SubstanceCriterion`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-16`](../ISSUE-TRACKER.md#eval-16)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.16
- Priority: **P2**
- Effort: 6 hours
- Depends on: `EVAL_13` (source 5.13)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Compares before/after ASTs to classify changes at the structural level. Catches "did nothing" failure mode with higher precision than the existing `DiffGate`'s forbidden-token matching at `crates/roko-gate/src/diff_gate.rs`.

## Exact Changes

1. `SemanticDiffCollector`: compares before/after ASTs, classifies each change as `SemanticChange { kind, significance }`. Kinds: FunctionAdded, FunctionModified, TypeChanged, FormattingOnly, DocumentationChanged, ImportChanged.
2. `SubstanceCriterion`: consumes SemanticDiff evidence, scores average significance. Threshold 0.2 (below this = "did nothing").
3. Gate behind `#[cfg(feature = "ast")]`.

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

- [ ] Test: diff adding only comments scores near 0.0 significance
- [ ] Test: diff adding a new function scores high significance

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test: diff adding only comments scores near 0.0 significance
- Test: diff adding a new function scores high significance
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
