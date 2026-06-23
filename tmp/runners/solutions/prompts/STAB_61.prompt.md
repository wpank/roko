# STAB_61: Wire knowledge store to CascadeRouter model selection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-61`](../ISSUE-TRACKER.md#stab-61)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.61
- Priority: **P2**
- Effort: 3 hours
- Depends on: `STAB_11` (source 1.11)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_61 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Knowledge store contains task-specific insights that could inform model routing. CascadeRouter
does not query it. `DreamRoutingAdvice` is generated but not loaded.

## Exact Changes

1. Load `DreamRoutingAdvice` at CascadeRouter initialization.
2. Apply `dream_advice_to_routing_bias()`.
3. Query knowledge for task-specific model hints.

## Write Scope

- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-learn/src/cascade_router.rs`

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

- [ ] After dream cycle with routing advice, model selections reflect the advice

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_61 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After dream cycle with routing advice, model selections reflect the advice
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_61 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
