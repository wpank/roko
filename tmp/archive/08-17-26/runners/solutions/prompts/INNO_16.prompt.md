# INNO_16: Track gate effectiveness metrics

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-16`](../ISSUE-TRACKER.md#inno-16)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.16
- Priority: **P1**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

No tracking of precision/recall per gate rung. Without this data, gates cannot
self-improve.

## Exact Changes

1. Create `crates/roko-learn/src/gate_effectiveness.rs`.
2. Define `GateEffectiveness` struct: `rung_id: String`, `true_positives: u64`,
   `false_positives: u64`, `true_negatives: u64`, `false_negatives: u64`.
3. Compute precision = TP / (TP + FP), recall = TP / (TP + FN), F1 score.
4. A "true positive" = gate fails AND the issue was real (confirmed by
   autofix succeeding after addressing the flagged issue).
5. A "false positive" = gate fails AND the fix attempt succeeds without
   addressing the flagged issue (the gate was wrong).
6. Persist to `.roko/learn/gate-effectiveness.json`.
7. Add `roko learn gates` CLI showing effectiveness report.
8. Add `pub mod gate_effectiveness;` to `crates/roko-learn/src/lib.rs`.

## Write Scope

- `crates/roko-learn/src/lib.rs`
- `crates/roko-cli/src/commands/learn.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After 20+ runs, `roko learn gates` shows precision/recall per rung
- [ ] At least one gate with precision < 0.5 is flagged for review
- [ ] Effectiveness data persists across runs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 20+ runs, `roko learn gates` shows precision/recall per rung
- At least one gate with precision < 0.5 is flagged for review
- Effectiveness data persists across runs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
