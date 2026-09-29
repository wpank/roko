+++
id = "gap-8f6206"
kind = "gap"
title = "Learning consumers read the settled verdict's learning label instead of succeeded (S01.P0-3)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["learn", "dispatch"]
created = 2026-09-29
updated = 2026-09-29
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
