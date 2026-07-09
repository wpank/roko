# STAB_51: Make `dangerously_skip_permissions` configurable

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-51`](../ISSUE-TRACKER.md#stab-51)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.51
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_51 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Line 394: `dangerously_skip_permissions: true`. Always. No configuration.

## Exact Changes

1. Add `skip_permissions: bool` to `[execution]` config (default: true for backward compat).
2. Generate default contract YAML during `roko init`.
3. Read config in plan.rs instead of hardcoding.
4. Log warning when running with skip_permissions = true.

## Write Scope

- `crates/roko-cli/src/commands/plan.rs`

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

- [ ] `skip_permissions = false` with contract YAML enforces restrictions
- [ ] Default (true) maintains current behavior

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_51 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `skip_permissions = false` with contract YAML enforces restrictions
- Default (true) maintains current behavior
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_51 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
