+++
id = "bug-eff5fa"
kind = "bug"
title = "A plan-check hand-back can reach its try without the check's failing lines: after a resume the try is numbered attempt 0 and the prompt drops retry feedback on attempt 0, and the hand-back clears the failing tests it parsed, so its retry_feedback event lists no errors"
status = "done"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "6f5003fb2"
source = "round v8 lead: phase 6's T4 hand-back (Undertow p6-the-window, run 23837ede, invocation ede1f9a7)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::RetryFeedbackBook", "crates/roko-cli/src/graph_execution/plan_handback.rs::handback_feedback"]
links = { depends_on = [], blocks = [], related = ["bug-59ed90"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn feedback_left_before_a_tasks_first_dispatch_makes_it_a_retry' crates/roko-cli/src && grep -rqw 'fn a_hand_back_keeps_the_failing_tests_it_quotes' crates/roko-cli/src && cargo test -p roko-cli --lib -- feedback_left_before_a_tasks_first_dispatch_makes_it_a_retry a_hand_back_keeps_the_failing_tests_it_quotes"

[closed]
at = 2026-11-13
at_ts = "2026-11-13T01:22:11Z"
commit = "6f5003fb2"
by = "roko-recovery"
executor = "claude-agent"
size = "S"
claimed_at = "2026-11-13T00:32:38Z"
forced = false
evidence = "6f5003fb2 on work/bug-eff5fa: a plan-check hand-back's try gets the check's failing lines also after a resume. Cause 1 (phase 6's T4, invocation ede1f9a7): the prompt adds retry feedback only when ctx.attempt > 0, and RetryFeedbackBook::next_attempt numbered the hand-back's try 0 (the task's earlier attempts were cleared when it passed, and the resumed process had not dispatched it), so its prompt dropped the feedback. A dispatch that starts with pending feedback is now a retry: at least attempt 1 and at least the attempt the feedback was left for; in one process nothing changes. Cause 2: handback_feedback emptied the failing tests it parsed (only compile errors hide the raw excerpt, bug-59ed90), so the retry_feedback event's errors were empty; the failing tests now stay. Red first on 4816a2cce: feedback_left_before_a_tasks_first_dispatch_makes_it_a_retry (attempt 0, expected 1) and a_hand_back_keeps_the_failing_tests_it_quotes (test_failures [] with P4.06's ERROR line in the excerpt); both pass now. Branch checks: nightly fmt; roko-cli lib 4158 passed, 0 failed (the plan_runner hand-back tests included); bin tests (plan, status, diagnose) 65 passed; workspace clippy -D warnings clean."
+++

## Problem

A plan-check hand-back can reach its try with none of the check's failing lines. In round v8's phase 6 (Undertow
`p6-the-window`, run 23837ede, third invocation ede1f9a7), the plan check `tools/check.sh --plugin` failed with exit
8: T4's step cells, steps_test.cpp:111 and :232/:234. roko handed it back to T4.

- **The `retry_feedback` event:** it carried the hand-back's diagnosis ("its step `tools/check.sh --plugin` failed,
  and its output follows, from where it fails") with `errors: []`, `attempt: 0` and `next_attempt: 1`.
- **T4's hand-back prompt** (`.roko/prompt-logs/p6-the-window-T4-1794520853370.txt`) has no "Previous attempt
  feedback" section at all. The agent had to run the checks again itself to find what failed.

Two causes:

1. **The feedback never reaches the prompt when the try is the task's first dispatch in its process.**
   - The prompt adds the gate feedback only when `ctx.attempt > 0`.
   - `RetryFeedbackBook::next_attempt` numbers a dispatch as the task's earlier attempts plus this process's
     dispatches. The earlier attempts are cleared when a task passes, and this process's count is 0 until it
     dispatches the task.
   - `GraphTaskDispatcher::hand_back` leaves the feedback for attempt `started`: this process's dispatches of the
     task, 0 after a resume.
   - So a hand-back's try in a resumed run is numbered attempt 0, a first attempt, and its prompt drops the feedback
     it was handed.
2. **The hand-back clears the failing tests it parsed.** `handback_feedback` empties `compile_errors`,
   `test_failures` and `clippy_warnings`, so that the raw excerpt shows. Only compile errors hide the raw output,
   though (bug-59ed90's rule). With the failing tests gone, the event's `errors` list is empty, and so is the
   prompt's failing-tests list.

## Why it matters

roko's core promise is that a failed check goes back with its errors. The guide says so, and the phase 6 hand-back
didn't do it.

## Where

- `crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs` (`RetryFeedbackBook::next_attempt`,
  `GraphTaskDispatcher::hand_back`).
- `crates/roko-cli/src/graph_execution/plan_handback.rs::handback_feedback`.
- `crates/roko-cli/src/dispatch/prompt_builder.rs` (the `ctx.attempt > 0` guard).

## Plan

1. Red tests first:
   - Feedback left for a task the process has not dispatched yet, as a hand-back after a resume leaves it, makes
     its next dispatch a retry (attempt 1 or more), so the prompt renders it.
   - A hand-back of a doctest failure keeps the failing case in its feedback's failing tests and in the event's
     `errors`, with the excerpt.
2. A dispatch that starts with pending feedback is a retry: at least attempt 1, and at least the attempt the feedback
   was left for.
3. `handback_feedback` keeps the failing tests it parsed. It still drops compile errors and warnings, so the raw
   excerpt, which leads with the failure, stays.

## Done when

The tests pass and the `[[verify]]` passes.
