# STAB_75: Add agent execution time monitoring

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-75`](../ISSUE-TRACKER.md#stab-75)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.75
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_75 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

~5% of agents ignore instructions, taking 5-15x longer. No monitoring detects this.

## Exact Changes

1. Track expected time per tier (fast: 2min, standard: 10min, complex: 30min).
2. Monitor actual duration.
3. If > 3x expected, log warning.
4. Optionally: SIGTERM and retry.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] 10x-longer-than-expected agent triggers duration warning

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_75 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 10x-longer-than-expected agent triggers duration warning
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_75 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
