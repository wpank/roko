# INNO_21: Implement steering channel

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-21`](../ISSUE-TRACKER.md#inno-21)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.21
- Priority: **P2**
- Effort: 8 hours
- Depends on: `INNO_20` (source 11.20)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`WorkflowEngine` at `crates/roko-runtime/src/workflow_engine.rs` runs the main
execution loop. It needs a channel to receive steering actions without blocking.

## Exact Changes

1. Create `crates/roko-runtime/src/steering.rs`.
2. Define `SteeringChannel` wrapping `(mpsc::Sender<SteeringAction>,
   mpsc::Receiver<SteeringAction>)`.
3. Implement `SteeringChannel::new(buffer: usize) -> (SteeringSender,
   SteeringReceiver)`.
4. In the workflow engine's main loop, poll the steering receiver at each
   iteration alongside the agent task using `tokio::select!`.
5. On receiving a `SteeringAction`:
   - `Redirect` -> inject guidance into the agent's next prompt
   - `Skip` -> mark task as deferred, move to next
   - `BudgetAdjust` -> update PlanBudgetManager
   - `InjectContext` -> append to current prompt context
6. Record every steering action to `.roko/steer/audit.jsonl`.
7. Add `pub mod steering;` to `crates/roko-runtime/src/lib.rs`.

## Write Scope

- `crates/roko-runtime/src/lib.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

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

- [ ] Sending a `Redirect` action via the channel injects guidance into the next agent prompt iteration
- [ ] Sending a `Skip` action stops the current task and moves to the next
- [ ] Audit trail records all steering actions with timestamps
- [ ] Channel is non-blocking: the execution loop continues if no steering action is pending

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Sending a `Redirect` action via the channel injects guidance into the next agent prompt iteration
- Sending a `Skip` action stops the current task and moves to the next
- Audit trail records all steering actions with timestamps
- Channel is non-blocking: the execution loop continues if no steering action is pending
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
