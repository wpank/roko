# STAB_36: Normalize model aliases at load time

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-36`](../ISSUE-TRACKER.md#stab-36)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.36
- Priority: **P2**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_36 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`glm-5-1` on provider "zai" vs `glm51` on provider "zhipu" both resolve to `glm-5.1`.
Multiple Claude aliases exist. Duplicate entries confuse routing.

## Exact Changes

1. Build an alias table at config load time.
2. Normalize all model slugs to canonical form.
3. Warn on duplicates.
4. CascadeRouter uses canonical slugs.

## Write Scope

- `crates/roko-orchestrator/src/service_factory.rs`
- `crates/roko-core/src/config/schema.rs`

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

- [ ] Config with both `glm51` and `glm-5-1` produces a warning
- [ ] CascadeRouter tracks a single canonical entry

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_36 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Config with both `glm51` and `glm-5-1` produces a warning
- CascadeRouter tracks a single canonical entry
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_36 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
