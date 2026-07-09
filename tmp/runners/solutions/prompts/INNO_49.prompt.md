# INNO_49: Implement event log fork-from-checkpoint

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-49`](../ISSUE-TRACKER.md#inno-49)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.49
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_49 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: LangGraph 1.0 GA made durable state and fork-from-checkpoint
first-class. AGDebugger (CHI 2025): counterfactual log editing is the UX
developers actually want.

## Exact Changes

1. Ensure each task completion writes a checkpoint to the event log.
2. Add `--fork-from <task_id>` flag to `roko plan run`.
3. When specified, load event log up to the named task's last checkpoint,
   replay state, and continue from there.
4. Fork creates a new `run_id` but shares the event log prefix.
5. Forked runs preserve all learning data from the original run.

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-cli/src/main.rs`

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

- [ ] `roko plan run --fork-from task-05` starts execution from after task-05
- [ ] The fork has a new run_id visible in logs
- [ ] Learning data from the original run is available in the fork

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_49 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run --fork-from task-05` starts execution from after task-05
- The fork has a new run_id visible in logs
- Learning data from the original run is available in the fork
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_49 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
