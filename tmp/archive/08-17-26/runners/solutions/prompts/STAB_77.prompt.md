# STAB_77: Unify two PipelineState state machines

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-77`](../ISSUE-TRACKER.md#stab-77)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.77
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_77 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`PipelineStateV2` (10 states) and `PlanPhase` (14 states) model the same concept but are
not interoperable.

## Exact Changes

1. Define superset state machine with optional phases.
2. Map optional phases (Enriching, DocRevision, RegeneratingVerify) to skip-when-unconfigured.
3. Both engines use unified state machine.
4. Add adapter for backward compatibility.

## Write Scope

- `crates/roko-runtime/src/pipeline_state.rs`
- `crates/roko-core/src/phase.rs`

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

- [ ] Single `WorkflowPhase` enum in both crates
- [ ] Only adapter/compat code references old types

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_77 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Single `WorkflowPhase` enum in both crates
- Only adapter/compat code references old types
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_77 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
