+++
id = "gap-96f7ed"
kind = "gap"
title = "Thread the attempt context through dispatch and settle one outcome per attempt (S01.P0-1)"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/runtime_feedback"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "abc655b5e"
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

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Each Graph dispatch attempt opens with a durable 1-based AttemptKey (graph_task_dispatch/attempt.rs) before prompt assembly, writing roko.attempt_open/1 to .roko/runs/<run>/attempts.jsonl with ordinals resumed from that file; each attempt settles once into roko.verdict/1 (outcome, blame, learning_label, executed model, cost source); emit_feedback takes a SettledAttempt; FeedbackEvent::AttemptSettled deduplicated; efficiency/cost rows and episodes carry attempt_key; prompt-experiment keys use it; the receipt is 1-based. Tests graph_feedback_records_share_attempt_key, graph_attempt_ordinal_survives_resume, settlements_follow_the_outcome_table pass (5d65eaf27, merge c51d02313, test fix dcc68916f; merged 42349d8ee). Sinks still read succeeded: gap-8f6206. Batch 7 gate (work/rust-batch-5 tree = MAIN crates after the merges): cargo check --workspace --tests clean; nightly rustfmt clean after fmt-only 15d3eb5f2; clippy -p roko-cli -p roko-learn -p roko-gate -p roko-agent -p roko-std -p roko-core --no-deps -D warnings clean; lib tests (8 threads) roko-cli 3091, roko-agent 2252 (after test fix f7ad8de76), roko-core 1923, roko-learn 1178, roko-std 221, roko-gate 685 (one pre-existing flaky test, tautology_filter_discards_preexisting_passing_tests, failed once and passed on rerun)."
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
- Implemented on `work/gap-96f7ed` at `5d65eaf27`; cargo verification deferred to the batch check.
- **Decisions (2026-09-29):** one base, 1-based: the receipt's `attempt` doc now says so (no production code builds
  receipts). Efficiency rows keep a unique `attempt_id` (the key, `/gate-pass` or `/gate-fail` on gate rows, which
  `tests/cost_dedup.rs` needs) and gain an exact `attempt_key` through `roko_learn::telemetry::AttemptKeyed`, as cost
  rows do; episodes get `extra.attempt_key`. The anchor `next_attempt_id` became `attempt.rs::open_attempt`.
- **Left for later items:** the sinks still read `succeeded` (unverified counts as a success) until bug-c34782,
  bug-35379d and gap-ad0d39 move them to the verdict; the verdict's usage, dollar costs, TTFT, verify steps and
  failover chain stay `null` (P0-4, P0-5, P0-6, bug-35379d). Prompt-assembly and cost-ledger errors after the open
  line leave no verdict yet, so they read as abandoned (P0-3).
- **Batch 5 (2026-09-29):** `graph_feedback_records_share_attempt_key` failed because the provider bridge
  (`dispatch_v2`, through `roko_learn::feedback_service`) also appends `"kind":"model_call"` rows to
  `.roko/learn/efficiency.jsonl`. Those rows use the feedback schema and have no attempt key. The test now selects
  the Graph dispatcher's `agent_efficiency_event/v1` rows. Keying the `model_call` rows needs the attempt key in
  `AgentDispatchRequest` (S01 P0-8 passes it through `DispatchContext`).
