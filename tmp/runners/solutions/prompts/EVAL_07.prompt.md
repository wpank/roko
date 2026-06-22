# EVAL_07: Scaffold `roko-eval-metrics` crate

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-07`](../ISSUE-TRACKER.md#eval-07)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.7
- Priority: **P1**
- Effort: 2 hours
- Depends on: `EVAL_01` (source 5.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Dependencies: `roko-eval`, `roko-core`, `roko-gate` (for parse functions in `compile_errors.rs` and `test_gate.rs`), `async-trait`, `serde`, `serde_json`. Optional: `tree-sitter`, `tree-sitter-rust` behind `ast` feature flag (Phase 3).

## Exact Changes

1. Create `crates/roko-eval-metrics/Cargo.toml`. Use feature gating: `[features] ast = ["dep:tree-sitter", "dep:tree-sitter-rust"]`.
2. Create `crates/roko-eval-metrics/src/lib.rs` with module declarations and re-exports.
3. Register in workspace `Cargo.toml`.

## Write Scope

- `Cargo.toml`

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

- [ ] The change matches the Implementation Steps above.
- [ ] No files outside Write Scope were touched.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
