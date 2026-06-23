# RNNR_13: Implement failure context accumulation for retries

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-13`](../ISSUE-TRACKER.md#rnnr-13)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.13
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When a task fails and is retried, accumulate structured failure context
(gate output, diff, error pattern) so the retry agent has full information.
The `FailureContext` struct already exists in `roko-orchestrator/src/repair.rs`
but is not populated with gate output or diff data in the runner.

## Exact Changes

1. After gate failure, capture: gate name, truncated gate output (2000 chars),
   the agent's diff (`git diff` in worktree), and any detected error pattern
2. Format as a "Previous Attempts" prompt section:
   "Attempt 1 failed because: [gate output]. Your changes: [diff summary]."
3. On retry dispatch, inject this section into the system prompt
4. For attempt 3+, include context from all prior failures
5. Ensure failure history survives checkpoint/resume (serialize to RunStateSnapshot)

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

- [ ] Retry attempts receive full context from all prior failures
- [ ] Failure context truncated to prevent exceeding token budgets
- [ ] Failure history survives checkpoint/resume (serializable)
- [ ] Third attempt includes context from both attempt 1 and attempt 2

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Retry attempts receive full context from all prior failures
- Failure context truncated to prevent exceeding token budgets
- Failure history survives checkpoint/resume (serializable)
- Third attempt includes context from both attempt 1 and attempt 2
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
