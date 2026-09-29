+++
id = "gap-96f7ed"
kind = "gap"
title = "Thread the attempt context through dispatch and settle one outcome per attempt (S01.P0-1)"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/runtime_feedback"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e4"
discovered_from = "tmp/cybernetic-harness/specs/S01-instrumentation.md (P0-1, P0-3, P0-7); workstreams/assessment/W5-contention-parallelism.md (rec 4)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::GraphTaskDispatcher::emit_feedback", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::GraphTaskDispatcher::open_attempt", "crates/roko-cli/src/runtime_feedback/mod.rs::FeedbackEvent"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["gap-528762", "gap-c8e1f1"], blocks = [], related = ["bug-c34782", "bug-35379d", "gap-ad0d39"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_feedback_records_share_attempt_key' crates/roko-cli/src/ && cargo test -p roko-cli --lib graph_feedback_records_share_attempt_key"

[[verify]]
command = "grep -rqw 'fn graph_attempt_ordinal_survives_resume' crates/roko-cli/src/ && cargo test -p roko-cli --lib graph_attempt_ordinal_survives_resume"
+++

## Problem

Dispatch never hands learning one settled record per attempt:

- `emit_feedback` takes `succeeded: bool`, and dispatch passes `verification.is_ok()` (`graph_task_dispatch.rs:4038`).
  An `Unverified` attempt therefore counts as a success.
- `FeedbackEvent::TaskCompleted` carries no attempt key, typed verdict, executed model or cost source; it has only
  `initial_model`.
- Each new learning loop is wired by editing `dispatch()` itself.

## Why it matters

W5 rec 4 makes this the hook for every loop. Loop and router fixes then become edits to their own modules, and
S01.P0-1 alone blocks about 50 items. It unblocks bug-c34782, bug-35379d, gap-ad0d39, gap-1f2661 and gap-c7c946.

## Where

- **`graph_task_dispatch.rs`:**
  - `GraphTaskDispatcher::dispatch`, `dispatch_streaming`, `settle_task_verification`, `emit_feedback` and
    `next_attempt_id`;
  - after the split (gap-c8e1f1) these live in the spine and in `verification.rs`, `feedback.rs` and `streaming.rs`.
- **`crates/roko-cli/src/runtime_feedback/mod.rs`:** `FeedbackEvent` and `FeedbackFacade`.
- **`plan_runner.rs`:** the `RunMetricsRecord` run id.

## Current state

Checked at `41c7ffbd6`:
- Attempt ids have been filled since `3d0637232`, but they are unique within a single process only.
- The learn-a worktree (`feat/learning-completion-loops`, uncommitted) edits `graph_task_dispatch.rs`,
  `runtime_feedback/mod.rs` and `episodes.rs`.

## Plan

1. **Mint the key.** Mint the `AttemptKey` (gap-528762) at the top of `dispatch` and `dispatch_streaming`, once the
   retry key is resolved. Write the `attempt_open` line before prompt assembly.
2. **Carry the context.** Pass an `AttemptContext` into `emit_feedback`, and replace `succeeded: bool` with a
   `SettledAttempt`: verdict, failure class, `learning_label`, executed model, cost and `cost_source`.
3. **Publish once.** Publish it once per attempt as `FeedbackEvent::AttemptSettled` through `FeedbackFacade`, and
   write its `VerdictRecord`. A second settlement of the same key is a counted no-op.
4. **Stamp the other records.**
   - Put the key on the efficiency, episode and cost rows.
   - Set `RunMetricsRecord.run_id` to the checkpoint's run id.
5. **Leave existing sinks alone.** They keep their current input. bug-c34782 (router), bug-35379d (executed model)
   and gap-ad0d39 (cost) each switch their own sink.

## Done when

- [ ] One attempt's efficiency, episode, cost and verdict rows share one attempt key.
- [ ] Killing a run after an `attempt_open` line and resuming it gives the next attempt a new, higher ordinal; the old
      key counts as abandoned.
- [ ] Tests `graph_feedback_records_share_attempt_key` and `graph_attempt_ordinal_survives_resume` pass: both
      `[[verify]]` commands.

## Notes

These are hot files. Start only after the env and learn-a branches merge and the dispatch split (gap-c8e1f1) lands.

- **From gap-528762 (2026-09-29):** `TaskAttemptReceiptV1.attempt` is documented as 0-based (`receipt.rs:80`), but
  `AttemptKey` is 1-based with the same string layout. Pick one base here and convert the other.
- Prompt-experiment keys use a per-process `graph-<uuid>` run id and 0-based ordinals, so they won't join to
  `AttemptKey` until this item switches them to the attempt context.
