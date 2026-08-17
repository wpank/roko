# EVAL_09: `LintCriterion`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-09`](../ISSUE-TRACKER.md#eval-09)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.9
- Priority: **P1**
- Effort: 4 hours
- Depends on: `EVAL_07` (source 5.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Migrates `ClippyGate` from `crates/roko-gate/src/clippy_gate.rs`. Two modes: strict (binary pass/fail matching exit code) and graduated (weighted score: errors * 0.2 + warnings * 0.05, subtracted from 1.0).

## Exact Changes

1. Implement `LintCriterion` with configurable `LintMode` (Strict, Graduated).
2. Parse clippy diagnostic output into `LintDiagnostic` structs (severity, rule_id, message, file, line, col, suggestion).
3. In Strict mode: score = 0.0 if exit code != 0, else 1.0.
4. In Graduated mode: score = (1.0 - errors * 0.2 - warnings * 0.05).clamp(0.0, 1.0).
5. Emit Findings with rule_id (clippy lint name) and suggestion text.

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

- [ ] Unit tests for both strict and graduated modes
- [ ] Test that findings carry clippy rule_id

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit tests for both strict and graduated modes
- Test that findings carry clippy rule_id
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
