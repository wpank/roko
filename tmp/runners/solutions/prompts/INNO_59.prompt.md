# INNO_59: Implement hindsight relabeling in dream cycle

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-59`](../ISSUE-TRACKER.md#inno-59)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.59
- Priority: **P3**
- Effort: 8 hours
- Depends on: `INNO_45` (source 11.45)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_59 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: AgentHER -- +7-12 percentage points and 2x data efficiency on
WebArena/ToolBench. With Verify gates as the relabeling oracle, failed runs
become positive episodes for sub-goals.

## Exact Changes

1. Load failed episodes from the episode log.
2. For each failed episode, analyze the trajectory to identify sub-goals
   actually achieved.
3. Create new positive episodes for those sub-goals with reduced scope.
4. Store relabeled episodes with `provenance: Inferred` and
   `source: hindsight_relabeling`.
5. Feed relabeled episodes into the learning loop.

## Write Scope

- `crates/roko-dreams/src/cycle.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A failed episode that correctly identified right files produces a positive sub-goal episode
- [ ] Relabeled episodes are tagged as Inferred
- [ ] The learning loop's positive trajectory count increases after dream consolidation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_59 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A failed episode that correctly identified right files produces a positive sub-goal episode
- Relabeled episodes are tagged as Inferred
- The learning loop's positive trajectory count increases after dream consolidation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_59 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
