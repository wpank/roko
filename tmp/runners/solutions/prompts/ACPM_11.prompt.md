# ACPM_11: Implement Parallel Agent Spawning in Runner

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-11`](../ISSUE-TRACKER.md#acpm-11)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.11
- Priority: **P1**
- Effort: 6 hours
- Depends on: `ACPM_10` (source 9.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The runner at `crates/roko-acp/src/runner.rs` performs side effects for the pipeline. It currently handles single-agent actions (`SpawnStrategist`, `SpawnImplementer`, etc.). It needs to handle `SpawnParallelAgents` by spawning multiple agents concurrently.

## Exact Changes

1. Add `handle_spawn_parallel()` method that creates a `tokio::task::JoinSet`.
2. For each `ParallelAgentSpec`, spawn an agent task. Reuse the existing agent spawning logic (model resolution, system prompt building, tool permissions).
3. As each agent completes, feed `ParallelAgentCompleted` or `ParallelAgentFailed` back to the pipeline state machine via `step()`.
4. Emit ACP session updates (`ToolCall` / `ToolCallUpdate`) for each parallel agent's progress, using the agent's role as the tool call title.
5. Track per-agent cost and add to `WorkflowRun.total_cost_usd`.
6. Handle cancellation: when `CancelToken` fires, cancel all in-flight agents in the `JoinSet`.

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

- [ ] Integration test: spawn 2 mock agents in parallel, both complete
- [ ] Verify both agents' ToolCall updates appear in the ACP event stream
- [ ] Cost is sum of both agents' costs
- [ ] Cancellation kills all parallel agents

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Integration test: spawn 2 mock agents in parallel, both complete
- Verify both agents' ToolCall updates appear in the ACP event stream
- Cost is sum of both agents' costs
- Cancellation kills all parallel agents
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
