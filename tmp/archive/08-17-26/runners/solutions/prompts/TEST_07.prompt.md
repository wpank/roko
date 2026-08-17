# TEST_07: Runtime and workflow engine integration tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-07`](../ISSUE-TRACKER.md#test-07)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.7
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Key types:
- `PipelineStateV2` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/pipeline_state.rs` (line 530) -- 10-state state machine
- `WorkflowEngine` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs` (line 105)
- `EventBus` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/event_bus.rs` (line 233)
- `JsonlLogger` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/jsonl_logger.rs` (line 15)
- `ProcessSupervisor` at `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/process.rs` (line 839)

Only 1 existing integration test file (`process_supervisor.rs`). The state machine, event bus, and workflow engine have zero integration coverage.

## Exact Changes

1. Test `PipelineStateV2` state machine transitions: Start -> StrategyPhase -> ImplementPhase -> GatePhase -> ReviewPhase -> CommitPhase -> Done
2. Test `PipelineStateV2` error paths: agent failure at ImplementPhase triggers retry up to `max_autofix_attempts`
3. Test `PipelineStateV2` gate failure: GatesFailed input triggers autofix iteration or terminal failure
4. Test `EventBus` publish-subscribe: register consumer, publish event, verify consumer receives it
5. Test `EventBus` fan-out: register 3 consumers, publish 1 event, verify all 3 receive it
6. Test `JsonlLogger` writes events to disk in JSONL format, verify each line is valid JSON
7. Test `ProcessSupervisor` tracks spawned processes and reports them via status queries
8. Test `WorkflowEngine` lifecycle: construct with mock services, verify state transitions

## Write Scope

- `crates/roko-runtime/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] 8+ new tests, all passing
- [ ] Every `PipelineStateV2` major transition path is covered
- [ ] EventBus fan-out works with 3+ consumers

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 8+ new tests, all passing
- Every `PipelineStateV2` major transition path is covered
- EventBus fan-out works with 3+ consumers
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
