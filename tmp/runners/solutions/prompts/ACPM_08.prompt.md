# ACPM_08: Add ParallelExecution Phase to Pipeline State Machine

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-08`](../ISSUE-TRACKER.md#acpm-08)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.8
- Priority: **P0**
- Effort: 5 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`PipelinePhase` at `crates/roko-acp/src/pipeline.rs:12-33` has 10 variants, all serial. `PipelineEvent` at line 44-71 has 12 variants, all single-agent. `PipelineAction` at line 75-92 has 8 variants, all single-agent. The `step()` method at line 195 pattern-matches all transitions exhaustively.

## Exact Changes

1. Add `ParallelExecution` variant to `PipelinePhase`:
   ```rust
   ParallelExecution {
       agent_ids: Vec<String>,
       completed: Vec<String>,
       results: Vec<(String, String)>, // (agent_id, output)
       barrier: BarrierCondition,
   }
   ```
2. Add `BarrierCondition` enum:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub enum BarrierCondition {
       AllComplete,
       MajorityComplete,
       AnyComplete,
   }
   ```
3. Add `ParallelAgentSpec`:
   ```rust
   #[derive(Debug, Clone)]
   pub struct ParallelAgentSpec {
       pub id: String,
       pub role: String,
       pub prompt: String,
       pub context: String,
   }
   ```
4. Add new events to `PipelineEvent`:
   - `ParallelAgentCompleted { agent_id: String, output: String }`
   - `ParallelAgentFailed { agent_id: String, error: String }`
5. Add new action to `PipelineAction`:
   - `SpawnParallelAgents { specs: Vec<ParallelAgentSpec> }`
6. Add transitions to `step()`:
   - `(ParallelExecution, ParallelAgentCompleted)` -> update completed list, check barrier
   - When barrier met -> transition to next phase (caller determines what)
   - `(ParallelExecution, ParallelAgentFailed)` -> for `AllComplete`, halt; for `MajorityComplete` / `AnyComplete`, check if remaining agents suffice
7. Implement `BarrierCondition::is_met(completed: usize, total: usize) -> bool`.

## Design Guidance

The state machine must remain pure. `ParallelExecution` only tracks which agents completed and their outputs. The runner handles the actual async spawning. The barrier check is a simple count comparison.

## Write Scope

- `crates/roko-acp/src/pipeline.rs`

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

- [ ] Unit test: `ParallelExecution` with 3 agents, `AllComplete` barrier -- completing all 3 transitions to next phase
- [ ] Unit test: `MajorityComplete` transitions after 2 of 3 complete
- [ ] Unit test: one agent failure with `AllComplete` halts the pipeline
- [ ] Unit test: `AnyComplete` transitions after first completion
- [ ] All existing pipeline tests pass unchanged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: `ParallelExecution` with 3 agents, `AllComplete` barrier -- completing all 3 transitions to next phase
- Unit test: `MajorityComplete` transitions after 2 of 3 complete
- Unit test: one agent failure with `AllComplete` halts the pipeline
- Unit test: `AnyComplete` transitions after first completion
- All existing pipeline tests pass unchanged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
