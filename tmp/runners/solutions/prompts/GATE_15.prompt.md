# GATE_15: Route by failure action in Runner v2 event loop

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-15`](../ISSUE-TRACKER.md#gate-15)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.15
- Priority: **P1**
- Effort: 4 hours
- Depends on: `GATE_07` (source 4.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Runner v2's event loop at `crates/roko-cli/src/runner/event_loop.rs` receives `GateCompletion` from the gate channel and currently always retries on failure (up to max iterations). It does not differentiate between retryable failures and structural/blocked/human-needed failures.

`GateFailureAction` at `crates/roko-gate/src/compile_errors.rs:69` has four variants: Retry, NeedsReplan, Blocked, NeedsHuman. The runner's `gate_dispatch.rs:309` already maps these to `RunnerFailureKind` variants but the event loop doesn't fully utilize them.

After Task 4.7, `GateCompletion` will carry `failure_classification` from `GateReport`. The event loop should use this to decide whether to retry, replan, pause, or escalate.

## Exact Changes

1. In the gate completion handler (`gate_rx.recv()` branch around line 494), extract the failure classification:
   ```rust
   if !completion.passed {
       match completion.failure_kind {
           Some(RunnerFailureKind::Structural) => {
               // Don't retry -- mark task as needing replan
               // Log reason, skip retry, mark failed with replan flag
           }
           Some(RunnerFailureKind::Resource) => {
               // External blocker -- pause and alert
               // Don't consume retry budget
           }
           Some(RunnerFailureKind::Permanent) => {
               // Needs human -- stop immediately
               // Mark task as permanently failed
           }
           _ => {
               // Retry with feedback (existing behavior)
           }
       }
   }
   ```
2. Ensure `RunnerFailureKind` variants map correctly from `GateReport.failure_classification.recommended_action` string.
3. When `NeedsReplan` is detected, emit a `RunnerEvent` that the caller can use to trigger replanning.
4. When `NeedsHuman` is detected, log a prominent warning and mark the task as failed without consuming retries.

## Design Guidance

The existing `RunnerFailureKind` mapping in `gate_dispatch.rs:309-322` is a good starting point. The event loop should respect it rather than always defaulting to retry. The key change is behavioral: `Structural` failures should not retry, `Resource` failures should pause, and `Permanent` failures should stop.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Structural failures (NeedsReplan) do not consume retry budget
- [ ] Resource failures (Blocked) pause the task
- [ ] Permanent failures (NeedsHuman) immediately fail the task

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Structural failures (NeedsReplan) do not consume retry budget
- Resource failures (Blocked) pause the task
- Permanent failures (NeedsHuman) immediately fail the task
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
