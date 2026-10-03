+++
id = "bug-0b7695"
kind = "bug"
title = "After a failover to another model, the attempt record still names the planned model, so bench Roko arms end infra_error (shakedown D3, D7)"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "PK36 shakedown via gap-821c93 (2026-10-03)"
discovered_from = "gap-821c93"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs", "benchmarks/viabilitybench/driver/test_shakedown.py"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-31c0e8", "gap-821c93"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn failover_attempt_records_the_model_that_answered' crates/roko-cli/src/ && cargo test -p roko-cli failover_attempt_records_the_model_that_answered"
+++

## Problem

When a provider fails (a 500, or a 401 auth failure) and the attempt moves to another model, the attempt record
still names the first model. PK36's shakedown (task 3314, `benchmarks/viabilitybench/driver/test_shakedown.py`,
scenarios D3 and D7), run on 2026-10-03 against a binary built from main at 823f2cfca with
`VB_REQUIRE_REAL_ROKO=1`, shows it: the metering proxy's `model_requested`/`model_reported` fields climb from the
cheap rung (`gpt-oss-120b`) to `glm-4.7` and then `gpt-5.4-mini`, but every recorded attempt's `model_dispatched`
stays `gpt-oss-120b`. ViabilityBench's driver compares the two, finds `model_mismatch`, and ends the run
`infra_error` instead of recording a clean escalation.

## Why it matters

The attempt ledger is the source of truth for what ran, what it cost, and which rung served a task (S04, M3).
Recording the planned model in place of the one that answered misattributes cost and outcome, and it voids every
benchmark Roko-arm run in which a provider fails and the run moves on.

## Where

- `crates/roko-cli/src/graph_task_dispatch/attempt.rs`: `model_dispatched` is written once, from
  `dispatch.target.model_slug` (about line 918), and never updated.
- `crates/roko-cli/src/graph_task_dispatch/failover.rs`: failover picks the next candidate and republishes the
  dashboard row with the fallback's slug (backlog 1128), but doesn't update the attempt.
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs`: rung substitution.

## Current state

The dashboard row follows a failover (`failover_publishes_fallback_slug_to_hub`); the attempt record doesn't.
Shakedown D2, D4, D5, D6 and D10 pass on the same binary.

## Plan

1. When failover dispatches a different model than the planned one, record the model that served the call on the
   attempt (and keep the planned one, e.g. as `model_planned`), so the settled verdict, the cost row and the
   self-model's labels name the model that answered.
2. A Rust test: a provider error on the first candidate fails over to the second, and the settled attempt record
   names the second model.
3. Run the shakedown's D3 and D7 against a binary built with the fix.

## Done when

- [ ] The `[[verify]]` command passes.
- [ ] Shakedown D3 and D7 pass against a binary built from the fix.

## Notes

- Found by gap-821c93's run of the shakedown (w3-pk21, 2026-10-03); PK36 (gap-31c0e8) stays open until all eight
  scenarios pass.
