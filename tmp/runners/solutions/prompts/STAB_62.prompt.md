# STAB_62: Fix GatePipeline / ComposedGatePipeline duplication

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-62`](../ISSUE-TRACKER.md#stab-62)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.62
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_62 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`GatePipeline` and `ComposedGatePipeline` partially duplicate logic. Dead code
`let _ = pipeline;` exists.

## Exact Changes

1. Have `ComposedGatePipeline` Sequential mode delegate to `GatePipeline`.
2. Or deprecate `GatePipeline` in favor of `ComposedGatePipeline`.
3. Remove dead code assignments.

## Write Scope

- `crates/roko-gate/src/gate_pipeline.rs`

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

- [ ] Sequential gate execution uses a single code path
- [ ] No dead code assignments

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_62 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Sequential gate execution uses a single code path
- No dead code assignments
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_62 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
