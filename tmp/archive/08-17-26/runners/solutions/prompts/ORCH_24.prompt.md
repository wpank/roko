# ORCH_24: Wire ProcessSupervisor into WorkflowEngine

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-24`](../ISSUE-TRACKER.md#orch-24)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.24
- Priority: **P2**
- Effort: 4 hours
- Depends on: `ORCH_03` (source 2.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ProcessSupervisor` at `crates/roko-runtime/src/process.rs` (1,354 LOC) provides Erlang-style supervision strategies (OneForOne, OneForAll, RestForOne) with configurable restart limits. It tracks processes via `ProcessHandle` with unique `ProcessId`, cooperative shutdown, and session metadata.

Currently, WorkflowEngine spawns agents via `EffectDriver::spawn_agent()` which calls `model_caller.call()` but does not track the agent process via ProcessSupervisor. This means:
- No timeout enforcement (stuck agents run forever)
- No restart on transient failure
- No process inventory for the dashboard

## Exact Changes

1. Add `process_supervisor: Option<Arc<ProcessSupervisor>>` to `EffectServices`.
2. When spawning agents, if ProcessSupervisor is available:
   - Create a `SpawnConfig` with the agent's label, timeout, and session metadata
   - Register the process with ProcessSupervisor
   - Set a timeout via `ProcessSessionConfig::timeout_ms`
3. On agent completion or failure, deregister from ProcessSupervisor.
4. On timeout, ProcessSupervisor sends SIGTERM (grace period) then SIGKILL.
5. Set default `SupervisionStrategy::OneForOne { max_restarts: 1 }` so transient failures get one retry.

## Design Guidance

The ProcessSupervisor should be optional (like all EffectServices). When not available, timeout enforcement is the caller's responsibility (which may mean no enforcement). The supervision strategy should be configurable per-task via the tasks.toml `timeout_secs` field.

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/effect_driver.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Spawned agents are registered with ProcessSupervisor
- [ ] Timeout enforcement kills stuck agents after configured seconds
- [ ] OneForOne strategy retries once on transient failure
- [ ] Process inventory is available for dashboard queries
- [ ] No ProcessSupervisor: behavior is unchanged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Spawned agents are registered with ProcessSupervisor
- Timeout enforcement kills stuck agents after configured seconds
- OneForOne strategy retries once on transient failure
- Process inventory is available for dashboard queries
- No ProcessSupervisor: behavior is unchanged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
