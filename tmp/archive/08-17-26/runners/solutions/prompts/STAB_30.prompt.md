# STAB_30: Fix ACP gate rung ordering (clippy before test)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-30`](../ISSUE-TRACKER.md#stab-30)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.30
- Priority: **P1**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

ACP runs gates in order: compile -> test -> clippy. Canonical order: compile (0) -> clippy
(1) -> test (2). Running test before clippy wastes 5-15 minutes when a trivial lint exists.

## Exact Changes

1. Reorder the hardcoded gate list to: compile, clippy, test.
2. Add short-circuit: if clippy fails, skip test (return early with clippy failure).
3. Alternatively: replace the hardcoded list with a call to `GateService` which orders by
   rung index.

## Design Guidance

Prefer using `GateService` for ordering rather than hardcoding. This ensures ACP gates
stay consistent with other paths.

## Write Scope

- `crates/roko-acp/src/runner.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] ACP gate execution runs in order: compile, clippy, test
- [ ] If clippy fails, test is skipped

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- ACP gate execution runs in order: compile, clippy, test
- If clippy fails, test is skipped
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
