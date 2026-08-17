# TEST_19: Gate feedback parity tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-19`](../ISSUE-TRACKER.md#test-19)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.19
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_04` (source 15.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`feedback_for_agent()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/feedback.rs` (line 202) produces `GateFeedback` (line 53) with `errors`, `warnings`, `suggestions` fields.

`classify_gate_failure()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/compile_errors.rs` (line 491) maps output to `FailureClass` variants (line 39).

## Exact Changes

1. Feed compile error output through `feedback_for_agent()`, verify `errors` contains error-classified lines
2. Verify `warnings` contains warning-classified lines
3. Verify `suggestions` contains help/note lines
4. Test noise filtering: cargo progress lines (Downloading, Compiling, Checking, Fresh, Running) are stripped
5. Test fallback: non-empty output with no classified lines produces at least one error entry
6. Test empty output: produces empty feedback (not crash)
7. Test `classify_gate_failure()` maps to correct `FailureClass`:
   - Syntax error -> `SyntaxError`
   - Missing import -> `ImportError`
   - Type mismatch -> `TypeError`
   - Borrow error -> `BorrowOrLifetime`
8. Test every `FailureClass` variant has at least one exercised input

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Every `FailureClass` variant has at least one test input
- [ ] Noise filtering strips all known cargo progress patterns
- [ ] Feedback structure is correct for each error category

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every `FailureClass` variant has at least one test input
- Feedback structure is correct for each error category
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
