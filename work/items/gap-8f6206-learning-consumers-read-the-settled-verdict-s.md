+++
id = "gap-8f6206"
kind = "gap"
title = "Learning consumers read the settled verdict's learning label instead of succeeded (S01.P0-3)"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["learn", "dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:00, wk-attempt-ctx's report on gap-96f7ed)"
anchors = ["crates/roko-cli/src/runtime_feedback/routing.rs::RoutingObservationSink", "crates/roko-cli/src/runtime_feedback/mod.rs::FeedbackEvent", "crates/roko-cli/src/graph_task_dispatch/feedback.rs::GraphTaskDispatcher::emit_feedback"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["gap-96f7ed"], blocks = [], related = ["gap-96f7ed", "bug-c34782"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learning_sinks_skip_attempts_without_a_learning_label' crates/roko-cli/src/ && cargo test -p roko-cli --lib learning_sinks_skip_attempts_without_a_learning_label"
+++

## Problem

gap-96f7ed settles each attempt once into a typed `roko.verdict/1` record: outcome, blame, `learning_label`, executed model, cost source. Every learning consumer still reads the old `succeeded: bool`, and in that bool an unverified attempt counts as success. The consumers are the router sinks, episodes, W07 playbooks, the daimon and prompt experiments (wk-attempt-ctx, 2026-09-29).

## Why it matters

The cybernetic thesis needs loops that learn from verified outcomes, not from provider success. Epic spec-b7303f; S01.P0-3.

## Where

Each sink behind `FeedbackEvent` (`runtime_feedback/`), and `emit_feedback` in `graph_task_dispatch/feedback.rs`. The event now carries `settled`.

## Current state

The settled record exists but no consumer reads it.

## Plan

1. Make every sink read `settled.learning_label`. A null label means no update.
2. Record a verdict for attempts that fail after the open line: prompt assembly or cost-ledger errors, which now read as abandoned.
3. Fold in bug-c34782: `RoutingObservationSink` stops learning from the pre-gate `succeeded`.
4. Add a test showing unverified and abandoned attempts update no learner.

## Done when

- [ ] No learner updates from an attempt without a learning label.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/gap-8f6206` at `04c1da262`; cargo verification deferred to the batch check. Under this
  item's cargo exception, `cargo check -p roko-cli --lib --tests`, the targeted lib tests (both `[[verify]]` tests,
  the per-sink tests, `runtime_feedback::`, `attempt::`, `feedback::`, `prompt_experiment::`), nightly fmt and
  `clippy -p roko-cli -p roko-learn --no-deps -D warnings` passed. They ran in a copy-on-write clone of
  `roko-check-target`, because the shared target dir serves other worktrees' crates as fresh.
  `verified_outcome_drives_output_verdict_and_feedback` timed out once under load, then passed alone. The
  `dispatch_feedback_projection_e2e` integration test was only compiled.
- **Decisions (2026-09-29):**
  - Learners read only `learning_label`, through `FeedbackEvent::learning_success` and
    `SettledAttempt::learning_success`. A `TaskCompleted` without a settled record teaches nothing.
  - Episodes stay one per attempt and keep `success` as the pre-S01 flag, because `roko diagnose` joins it to
    `costs.jsonl` `success` and the turn-policy test pins it. They add `extra.outcome`, `extra.blame` and
    `extra.learning_label`. `costs.jsonl` rows add `outcome` and `learning_label`.
  - A prompt-assembly or cost-ledger error after the open line settles as the new outcome `harness_error` (blame
    harness, label null) through `fail_attempt`, on both dispatch paths.
- **Left open:**
  - Readers of `episodes.jsonl` (dreams, the hindsight relabeler, the skill library, the curriculum) still read
    `success`, where an unverified attempt counts. They should read `extra.learning_label`.
  - The provider bridge still teaches the persisted router from the provider's pre-gate `success` on every Graph
    dispatch (`dispatch_v2::record_agent_dispatch_feedback`, feedback Path B, `observe_model_call`).
