# EVAL_10: `TestCriterion`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-10`](../ISSUE-TRACKER.md#eval-10)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.10
- Priority: **P1**
- Effort: 4 hours
- Depends on: `EVAL_07` (source 5.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Migrates `TestGate` from `crates/roko-gate/src/test_gate.rs`. Reuses `roko_gate::parse_test_counts` (exported from `crates/roko-gate/src/test_gate.rs`). Score = pass_rate = passed / (passed + failed). Extracts failing test names and their stdout blocks as Findings.

## Exact Changes

1. Implement `TestCriterion`:
   - `name()` = "test"
   - `criterion_kind()` = `CriterionKind::Deterministic`
   - `is_hard()` = configurable (default true)
   - `required_evidence()` = `[EvidenceKind::ProcessOutput, EvidenceKind::ProcessStatus]`
   - `evaluate()`: parse test output with `parse_test_counts()`, compute pass_rate, extract failing test names and their output blocks as Findings.
2. Support `TestSelector` (All, Changed, Specific) for evidence filtering.

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

- [ ] Unit test with mock test output containing 2 failures out of 14
- [ ] Test that pass_rate = 12/14

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test with mock test output containing 2 failures out of 14
- Test that pass_rate = 12/14
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
