# RNNR_03: Wire serialized merge via PlanMerger into runner event loop

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-03`](../ISSUE-TRACKER.md#rnnr-03)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.3
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_01` (source 14.1), `RNNR_02` (source 14.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: After a task completes successfully, enqueue its changes for
serialized merge into the integration branch via `PlanMerger`. The `PlanMerger`
already wraps `MergeQueue` and has `GitMergeBackend` + `CargoCheckRegressionGate`;
it just needs to be called from the event loop.

## Exact Changes

1. Add `plan_merger: Option<PlanMerger>` to the run context
2. After a task reaches completion, collect changed files via `git diff --name-only HEAD~1`
   in the task's worktree
3. Build `MergeRequest { plan_id, branch_name, files_changed, priority }` and
   call `plan_merger.submit(request, gate_tx)`
4. Process merge completions via `drain_next()` in the event loop's select
5. On merge conflict, record the conflict details and mark the task for
   reprocessing (don't silently swallow)
6. On merge success, update the integration branch state

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] Task merges are serialized (no concurrent `git merge` operations)
- [ ] Tasks touching disjoint files merge without waiting for each other
- [ ] Tasks touching overlapping files are serialized by the queue
- [ ] Merge conflicts are recorded and reported, not silently swallowed

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Task merges are serialized (no concurrent `git merge` operations)
- Tasks touching disjoint files merge without waiting for each other
- Tasks touching overlapping files are serialized by the queue
- Merge conflicts are recorded and reported, not silently swallowed
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
