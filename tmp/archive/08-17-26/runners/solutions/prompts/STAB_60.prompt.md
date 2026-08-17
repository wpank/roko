# STAB_60: Make workspace map cap proportional to context tier

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-60`](../ISSUE-TRACKER.md#stab-60)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.60
- Priority: **P2**
- Effort: 1 hour
- Depends on: `STAB_18` (source 1.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_60 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`WORKSPACE_MAP_LINE_LIMIT = 200` is fixed. Should scale with context tier.

## Exact Changes

1. Make cap proportional: Surgical 50, Focused 150, Full 300, Extended 500.
2. Or filter to files relevant to the current task.

## Write Scope

- `crates/roko-compose/src/prompt_assembly_service.rs`

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

- [ ] Surgical tier: workspace map 50 lines max
- [ ] Full tier: 300 lines

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_60 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Surgical tier: workspace map 50 lines max
- Full tier: 300 lines
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_60 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
