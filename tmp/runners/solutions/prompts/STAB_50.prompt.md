# STAB_50: Expose `max_concurrent_tasks` from config

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-50`](../ISSUE-TRACKER.md#stab-50)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.50
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_50 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Line 115 of `event_loop.rs`: `max_concurrent_tasks: 1`. Despite a full DAG scheduler,
plans execute sequentially.

## Exact Changes

1. Add `max_concurrent_tasks` to `[execution]` config in roko.toml.
2. Read from config instead of hardcoding 1.
3. Default to 1, allow up to 8.
4. Add `--parallel <N>` CLI flag for override.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
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

- [ ] `--parallel 4` with 4 independent tasks starts all 4 simultaneously
- [ ] Default (no flag) runs sequentially

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_50 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `--parallel 4` with 4 independent tasks starts all 4 simultaneously
- Default (no flag) runs sequentially
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_50 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
