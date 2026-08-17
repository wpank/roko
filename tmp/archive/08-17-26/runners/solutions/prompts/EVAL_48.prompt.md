# EVAL_48: Criterion authoring format (user TOML criteria)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-48`](../ISSUE-TRACKER.md#eval-48)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.48
- Priority: **P2**
- Effort: 5 hours
- Depends on: `EVAL_03` (source 5.3), `EVAL_04` (source 5.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_48 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Users author criteria at `.roko/criteria/*.toml`.
2. Two modes:
   - Shell (deterministic): `[criterion.check] type = "shell" command = "..."`. Exit 0 = pass.
   - Judge (stochastic): `[criterion.check] type = "judge"`. Delegates to judge panel with custom rubric.
3. Parser: read TOML into `CustomCriterionDef`, construct `ShellCriterion` or `JudgePanelCriterion`.

## Write Scope

- `crates/roko-eval/src/lib.rs`

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

- [ ] Parse and execute a custom shell criterion
- [ ] Parse a judge criterion definition (execution requires judge panel from Phase 4)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_48 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Parse and execute a custom shell criterion
- Parse a judge criterion definition (execution requires judge panel from Phase 4)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_48 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
