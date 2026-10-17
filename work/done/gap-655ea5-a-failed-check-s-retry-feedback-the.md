+++
id = "gap-655ea5"
kind = "gap"
title = "A failed check's retry feedback, the diagnosis the next attempt's prompt leads with, is never published, only kept in retry-feedback.json"
status = "done"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-core/dashboard"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "7d8eeb8e7"
source = "tmp/hermes-roko-v6/design/spec-v3.md §10 R2 (roko-delivery, 2026-10-16)"
discovered_from = "hermes-roko-v6 round: make learning visible on the hub"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps", "crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::RetryFeedbackBook::record", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_failed_check_publishes_its_retry_feedback' crates/roko-cli && cargo test -p roko-cli --lib a_failed_check_publishes_its_retry_feedback"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T21:14:27Z"
commit = "96d5f9050"
by = "commit trailer"
executor = "unknown"
forced = false
evidence = "96d5f9050: every site that keeps retry feedback (a failed verify, the pre-verify screen, a plan-branch conflict, a scratch copy-back conflict, a review) goes through GraphTaskDispatcher::leave_retry_feedback, which publishes retry_feedback: plan_id, task_id, attempt (from 1), next_attempt, check, failing_steps, diagnosis, errors (at most 5); /health feature retry_feedback_events; docs/v3/26-HTTP-API.md. Verify: a_failed_check_publishes_its_retry_feedback passes. Checked as a stack on f870523c0 (main 8357603f0 + gap-655ea5 + gap-de0e96 + bug-ea8c66): roko-cli lib 3738 passed (6 ignored, gist skipped), roko-serve lib 1047, roko-core lib 2035, the roko bin's learn test, clippy -D warnings clean on roko-core, roko-cli and roko-serve, fmt, plan_branch_integration (4) and connected_efficiency_render (1), with a scratch HOME. Scripted showcase, $0 (tmp/hermes-roko-v6/roko-delivery/evidence/stack): p1-slugify's two failed cheap attempts each publish retry_feedback (attempt 1 -> 2 and 2 -> 3, check verify, the failing step `python3 -m unittest tests.test_slug -q`: exit code: 1, and the diagnosis the next prompt leads with)."
+++

## Problem

When a task's check fails, roko asks a cheap model for a short diagnosis and leaves it, with the failing steps' output
parsed into compile errors, failed tests and warnings, for the task's next attempt (`GateFeedback`, kept in
`retry-feedback.json`). Nothing on the hub says so: a host sees `gate_result` failed, then the next `route_decided`,
and never what the next attempt was told. The retry loop, the first thing that makes a cheap model pass on its second
try, is invisible (spec-v3 R2; the route sheet's "feedback" per try, P5).

## Why it matters

Goal `hermes`: the plugin's route and escalation sheet (spec-v3 D6, P5) shows per try what failed and the diagnosis
the next try got; the stream shows it as it happens.

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps`: a failed verify enriches and
  keeps the feedback (`gate_retry_context.record`).
- `crates/roko-cli/src/graph_task_dispatch/red_flags.rs::reject_attempt` (the pre-verify screen) and
  `attempt_workspace.rs` (a plan-branch conflict, a scratch copy-back conflict, a reviewer's rejection) keep feedback
  the same way.
- `crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::RetryFeedbackBook::record`.

## Plan

1. `DashboardEvent::RetryFeedback` (`retry_feedback`): `plan_id`, `task_id`, `attempt` (the one that failed, from 1),
   `next_attempt`, `check` (`verify`, `pre_verify:<check>`, `plan-branch`, `scratch-copy-back`, `review`),
   `failing_steps` (a line each), `diagnosis`, `errors` (the key lines the next prompt lists, at most 5).
2. One `GraphTaskDispatcher::leave_retry_feedback` that publishes it and keeps the feedback; every site that keeps
   feedback calls it.
3. serve: `/api/events` names it; `/health` feature `retry_feedback_events`; docs/v3/26-HTTP-API.md.

## Done when

The `[[verify]]` test passes: a failed verify publishes `retry_feedback` with the attempt numbers, the failing step,
the diagnosis and the failed test's line.
