# RNNR_12: Wire cumulative context into agent dispatch

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-12`](../ISSUE-TRACKER.md#rnnr-12)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.12
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_01` (source 14.1), `RNNR_11` (source 14.11)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Before dispatching each task's agent, generate the cumulative context
section from all previously completed tasks in the plan and inject it into the
prompt.

## Exact Changes

1. Track `completed_summaries: Vec<CompletedTaskSummary>` on the run context
2. After each task completes, collect changed files via `git diff --stat`
   in the task's worktree and append to `completed_summaries`
3. Before dispatching a new task, call `cumulative_context(&completed_summaries, 4000)`
4. Inject the result into the agent's system prompt (layer 5, contextual knowledge)
5. Also inject the list of files the current task will modify (from task config)
   so the agent can check those files against prior changes

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

- [ ] Each dispatched agent receives cumulative section with prior task changes
- [ ] Section grows as more tasks complete
- [ ] Token budget prevents section from consuming too much context
- [ ] First task in a plan receives an empty cumulative section

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Each dispatched agent receives cumulative section with prior task changes
- Section grows as more tasks complete
- Token budget prevents section from consuming too much context
- First task in a plan receives an empty cumulative section
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
