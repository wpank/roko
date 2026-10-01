+++
id = "bug-82cbef"
kind = "bug"
title = "A verify step stopped by an interrupt settles as gate_failed, not cancelled"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-b367bf"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-b367bf"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib interrupted_verify_settles_as_cancelled"
+++

## Problem

With gap-b367bf a running gate command is stopped on cancellation, but the step's outcome is recorded as `gate_failed`. That is an honest-outcome error: a cancelled verify taught nothing about the code. bug-ceb581 already prevents the retry.

## Plan

Settle a verify interrupted by cancellation as cancelled (no gate-failure record, no learning). Add a test named `interrupted_verify_settles_as_cancelled`.

## Done when

- `cargo test -p roko-cli --lib interrupted_verify_settles_as_cancelled` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-b367bf, during the evening close-out round.
- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  Once the plan run began to stop (`begin_stop`, which the interrupt calls before it signals the run's commands), a
  verify step that fails, or that would start, or the auto-fix, ends the verify with `RokoError::Cancelled`. Nothing
  re-runs it, no sibling settle waits for it, and no gate-failure record, feedback or learner sees it.
  `Settlement::verified` settles that as `AttemptOutcome::Cancelled`, an unchanged-tree probe passes it through, and
  the streaming path reports `TaskDispatchOutcomeKind::Cancelled`. Test: `interrupted_verify_settles_as_cancelled`.
