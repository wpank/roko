# ACPM_13: Add Parallel Progress to ACP Session Updates

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-13`](../ISSUE-TRACKER.md#acpm-13)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.13
- Priority: **P2**
- Effort: 3 hours
- Depends on: `ACPM_11` (source 9.11)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`SessionUpdate` at `crates/roko-acp/src/types.rs` has 11 variants covering text chunks, tool calls, plans, and usage. It has no variant for parallel agent progress.

## Exact Changes

1. Add `ParallelProgress` variant to `SessionUpdate`:
   ```rust
   ParallelProgress {
       total_agents: u32,
       completed_agents: u32,
       agent_statuses: Vec<ParallelAgentStatus>,
   }
   ```
   where `ParallelAgentStatus { role: String, status: ToolCallStatus }`.
2. Emit `ParallelProgress` updates from the runner whenever a parallel agent completes.
3. Add corresponding `PlanEntry` updates showing each parallel agent as a sub-step.

## Write Scope

- `crates/roko-acp/src/types.rs`
- `crates/roko-acp/src/runner.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] ACP client receives `ParallelProgress` updates during parallel execution
- [ ] Progress shows correct completed/total counts
- [ ] Plan entries show individual agent status

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- ACP client receives `ParallelProgress` updates during parallel execution
- Progress shows correct completed/total counts
- Plan entries show individual agent status
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
