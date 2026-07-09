# DISP_24: Identify Live Exports from orchestrate.rs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-24`](../ISSUE-TRACKER.md#disp-24)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.24
- Priority: **P1**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`orchestrate.rs` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` is 22,522 lines. Most of it is dead code (the `PlanRunner` is never instantiated). But some exports are used by tests, other modules, or `lib.rs` re-exports.

## Exact Changes

1. Run a comprehensive grep for all imports from orchestrate.rs:
   ```bash
   grep -rn 'orchestrate::' crates/ --include='*.rs' | grep -v 'orchestrate.rs' | grep -v target/
   ```
2. Run a grep for items re-exported via lib.rs:
   ```bash
   grep -n 'orchestrate' crates/roko-cli/src/lib.rs
   ```
3. Categorize each export as:
   - **Live (used in production code)**: must be preserved or migrated
   - **Test-only**: can be moved to test helpers
   - **Dead (no callers)**: safe to delete
4. Document the categorization in a comment block at the top of orchestrate.rs
5. For each live export, identify the target module where it should live after decomposition

## Design Guidance

This is an analysis task. Do not modify orchestrate.rs. Produce a categorized list that subsequent tasks use to plan extraction. The goal is to understand what is actually needed before deleting anything.

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A categorized list of all orchestrate.rs exports exists (in a comment or separate tracking doc)
- [ ] Each export is marked as live/test-only/dead with the importing file
- [ ] No code changes to orchestrate.rs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A categorized list of all orchestrate.rs exports exists (in a comment or separate tracking doc)
- Each export is marked as live/test-only/dead with the importing file
- No code changes to orchestrate.rs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
