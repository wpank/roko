# STAB_68: Wire StagingBuffer lightweight promotion

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-68`](../ISSUE-TRACKER.md#stab-68)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.68
- Priority: **P2**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_68 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Candidates in StagingBuffer progress Raw -> Replayed -> Validated but promotion requires
a full dream cycle. Buffer grows unbounded without one.

## Exact Changes

1. Add lightweight promotion check in LearningRuntime.
2. After each run, check for Validated candidates.
3. Promote to KnowledgeStore without full dream cycle.

## Write Scope

- `crates/roko-dreams/src/staging.rs`

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

- [ ] Validated candidates appear in KnowledgeStore without manual dream run

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_68 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Validated candidates appear in KnowledgeStore without manual dream run
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_68 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
