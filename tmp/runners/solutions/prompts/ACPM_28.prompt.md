# ACPM_28: Inject Shared Context into Parallel Agents

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-28`](../ISSUE-TRACKER.md#acpm-28)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.28
- Priority: **P1**
- Effort: 4 hours
- Depends on: `ACPM_11` (source 9.11), `ACPM_27` (source 9.27)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The runner spawns parallel agents (Task 9.11). When agents complete, their outputs should be published to the `SharedContextStore` so the verdict merge has full context.

## Exact Changes

1. Create a `SharedContextStore` per parallel execution phase in the runner.
2. When `ParallelAgentCompleted` fires, publish the agent's output summary to the store: `store.publish(role, "findings", &output_summary)`.
3. When all agents complete, include the full `store.snapshot()` in the `MergeVerdicts` action context.
4. Drop the store after the parallel phase ends (automatic via `Arc` refcount).

## Write Scope

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

- [ ] When Architect finishes before Auditor, the VerdictMerge input includes Architect's findings
- [ ] Shared context appears in the `MergeVerdicts` action's outputs
- [ ] Store is dropped after the parallel phase ends

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- When Architect finishes before Auditor, the VerdictMerge input includes Architect's findings
- Shared context appears in the `MergeVerdicts` action's outputs
- Store is dropped after the parallel phase ends
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
