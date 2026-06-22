# EVAL_14: `StructuralCompletenessCriterion`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-14`](../ISSUE-TRACKER.md#eval-14)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.14
- Priority: **P2**
- Effort: 5 hours
- Depends on: `EVAL_13` (source 5.13)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

AST-based replacement for `SymbolGate` at `crates/roko-gate/src/symbol_gate.rs`. Takes a list of structural expectations and matches them against the flattened AST items.

## Exact Changes

1. Define `StructuralExpectation`: `{ kind: String, name_pattern: String, path: Option<String>, visibility: Option<String>, substantive_body: bool }`.
2. Implement `StructuralCompletenessCriterion`:
   - Consumes `EvidenceKind::Ast`.
   - Matches expectations against flattened AST items using regex on name_pattern.
   - When `substantive_body = true`, checks that matched items do not contain `todo!()` or `unimplemented!()` in body_text.
   - Score = met_expectations / total_expectations. Hard severity.
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

- [ ] Test against a file with 3 expected functions, 1 missing
- [ ] Test `todo!()` detection

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test against a file with 3 expected functions, 1 missing
- Test `todo!()` detection
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
