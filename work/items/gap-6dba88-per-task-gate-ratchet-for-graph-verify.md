+++
id = "gap-6dba88"
kind = "gap"
title = "Per-task gate ratchet for Graph verify: flag a step that passed on an earlier attempt and now fails"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "M"
subsystem = ["roko-gate", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-verify-loops 99adacd6d"
anchors = ["crates/roko-gate/src/ratchet.rs::GateRatchet", "crates/roko-cli/src/graph_task_dispatch/verification.rs::GraphTaskDispatcher::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::RetryFeedbackBook"]
links = { depends_on = [], blocks = [], related = ["find-4b4344", "bug-e0f472"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_step_that_passed_on_an_earlier_attempt_and_fails_now_is_a_regression' crates/roko-cli/src && cargo test -p roko-cli --lib a_step_that_passed_on_an_earlier_attempt_and_fails_now_is_a_regression && grep -rqw 'fn step_order_within_an_attempt_is_not_a_regression' crates/roko-cli/src && cargo test -p roko-cli --lib step_order_within_an_attempt_is_not_a_regression"
+++

## Problem

Nothing on the Graph path notices when a task's retry breaks something an earlier attempt had fixed. For example,
attempt 1 fixes the compile error but fails the tests, and attempt 2 fixes the tests but breaks compile again. This
is the thrash that `roko_gate::GateRatchet` was written to catch. It cannot be wired as it stands, because its model
does not fit authored verify steps:
- it keys on the plan id, so one task's pass hides another task's first failure;
- it keeps only the highest canonical rung passed, and treats any later failure on a lower rung as a regression
  (`can_regress`, `ratchet.rs:104-110`).
Graph tasks author their own `[[task.verify]]` steps in any order. A pass of `test` followed by a failure of
`clippy` in the same attempt would read as a regression. `99adacd6d` left `GateRatchet` unwired for this reason.

Expected: per task, and across that task's attempts, a verify step that passed on an earlier attempt and fails now
is reported as a regression. The next attempt's retry feedback says so.

## Why it matters

Goal `learning`. It is a cheap signal that a retry loop is thrashing, and useful feedback for the agent ("step X
passed on attempt 1 and fails now: your last change broke it"). It is the Graph form of closure P1-11 in
`find-4b4344`.

## Where

- `crates/roko-gate/src/ratchet.rs::GateRatchet`: the per-plan, rung-ordered ratchet. It is only re-exported
  (`roko-gate/src/lib.rs:186`) and has no production caller.
- `crates/roko-cli/src/graph_task_dispatch.rs:2059-2400`: the Graph verify loop.
  - `step_outcomes: Vec<(String, bool)>` collects `(phase, passed)` for each step (:2063, :2183).
  - The post-auto-fix re-run replaces them (:2325-2395).
  - Failure feedback is built from the failing steps.
- `crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::RetryFeedbackBook`: per-plan, per-task state kept in
  `retry-feedback.json` beside the Graph checkpoint and scoped to the checkpoint run. `record` is at :117 and
  `clear` at :137. It is the natural place for per-task step history that must survive `--resume-plan`.

## Current state

Checked at `33e107da1`: `GateRatchet` has no caller outside `ratchet.rs` tests. The Graph verify loop keeps no
history of which steps passed on earlier attempts.

## Plan

1. Identify a step by its index plus a hash of `phase` and `command` from `tasks.toml`, not by rung.
2. For each `(plan, task)`, keep the set of steps that passed on any earlier attempt of the current checkpoint
   run. Keep it next to the pending feedback in `RetryFeedbackBook`, so it survives a resume and `--fresh` resets
   it.
3. After a failed verify, list the failing steps that are in that set, and:
   - log a WARN;
   - add a TUI notice;
   - add a line to the next attempt's `GateFeedback`, for example in `diagnosis` or a new `regressions` field,
     rendered ahead of the errors.
   Clear the history when the task passes.
4. Decide what happens to `roko_gate::GateRatchet`: delete it, or re-key it as `(plan, task)` with step
   identities. Also update `find-4b4344`: its plan step 3 (`record_pass(plan_id, rung)`) is replaced by this item,
   and its verify greps for the name `GateRatchet`.
5. Tests (roko-cli lib):
   - `a_step_that_passed_on_an_earlier_attempt_and_fails_now_is_a_regression`;
   - `step_order_within_an_attempt_is_not_a_regression`: `test` passes, then `clippy` fails in one attempt, and no
     regression is reported.

## Done when

- A retry that breaks a previously passing step is flagged in the log, the TUI and the next attempt's prompt.
- Step order within one attempt is never flagged.
- The `[[verify]]` command passes.

## Notes

- Advisory only. Do not change which steps run or whether the task passes.
- `graph_task_dispatch.rs` is large and heavily edited. Keep the logic in a small module, for example
  `graph_task_dispatch/step_ratchet.rs`.
