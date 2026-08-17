# RNNR_28: Wire chain fusion from DAG into TaskScheduler

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-28`](../ISSUE-TRACKER.md#rnnr-28)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.28
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_25` (source 14.25)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Wire `UnifiedTaskDag::fuse_linear_chains()` into the scheduler so
linear sequences of mechanical tasks are collapsed into single dispatch units.

## Exact Changes

1. Add `fusion_enabled: bool` to scheduler construction or a separate config
2. When enabled, call `dag.fuse_linear_chains(&FusionConfig::default())` before
   converting to `SchedulableTask`s
3. Fused tasks get combined prompts: "Step 1: [task A]. Step 2: [task B]."
4. Fused tasks inherit the union of all constituent task file scopes
5. If any step in a fused task fails, entire fused unit fails (but individual
   step results tracked)
6. Default: `max_chain_length=3`, `same_tier_only=true`

## Write Scope

- `crates/roko-runtime/src/task_scheduler.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Linear chains of mechanical tasks fused into single dispatch units
- [ ] Fused tasks produce correct combined prompts
- [ ] Fusion reduces total wave count (verified via `--dry-run`)
- [ ] Fused task failure attributed to the failing step

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Linear chains of mechanical tasks fused into single dispatch units
- Fused tasks produce correct combined prompts
- Fusion reduces total wave count (verified via `--dry-run`)
- Fused task failure attributed to the failing step
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
