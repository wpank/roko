# STAB_63: Wire ProcessRewardModel to orchestrator

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-63`](../ISSUE-TRACKER.md#stab-63)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.63
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_63 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

ProcessRewardModel tracks per-turn gate snapshots, derives Promise (probability of eventual
success) and Progress signals. Not instantiated. Tasks clearly failing continue consuming budget.

## Exact Changes

1. Instantiate PRM per-task in event loop.
2. After each gate snapshot, update PRM.
3. If Promise < 0.1, abort early.
4. Log PRM signals in episodes.

## Write Scope

- `crates/roko-gate/src/process_reward.rs`
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

- [ ] Task failing compile 3 times with worsening output aborted by PRM

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_63 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Task failing compile 3 times with worsening output aborted by PRM
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_63 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
