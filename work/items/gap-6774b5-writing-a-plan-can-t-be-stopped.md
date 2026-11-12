+++
id = "gap-6774b5"
kind = "gap"
title = "A plan write's spend reaches the event stream only as bare efficiency events, and the plan list has no row for a plan being written: a host has to poll the operation to show SPENT, and closing the box that asked loses sight of the write"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/plans"]
created = 2026-11-12
updated = 2026-11-12
source = "round v8 portal-v8, split from gap-6c4d24 (2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, 33c33e738)"
anchors = ["crates/roko-cli/src/plan_authoring.rs::AuthoringSpend::record", "crates/roko-serve/src/routes/plans/reads.rs::list_plans", "crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation"]
links = { depends_on = ["bug-36ce72"], blocks = ["gap-f8c57a"], related = ["gap-6c4d24", "gap-5c3347"], supersedes = [], duplicate_of = "" }
last_verified = 2026-11-12

[[verify]]
command = "grep -rqw 'fn plan_writing_calls_publish_their_cost_as_they_settle' crates/roko-cli/src && grep -rqw 'fn the_plan_list_shows_a_plan_being_written' crates/roko-serve/src && cargo test -p roko-cli --lib plan_writing_calls_publish_their_cost_as_they_settle && cargo test -p roko-serve --lib the_plan_list_shows_a_plan_being_written"
+++

## Problem

While roko writes a plan, a host can't show what the write has spent from the event stream, and the plan list
can't show the write at all:

- **Spend.** Each settled planner call goes out as `token_usage` and an `efficiency_event` (`cost_usd`) for plan
  `<slug>`, task `generate`. Neither carries the call's model, its number in the write, whether a rate priced it,
  or what the write has spent so far. A run's calls publish `call_cost` for that (gap-5c3347); a write's don't. The
  running total and the cap are only in the operation's progress, which a host has to poll
  (`GET /api/operations/{id}`).
- **No row.** The plan doesn't exist until it is written, so `GET /api/plans` has no row for it. The write lives
  only in the box or chat that asked for it, and closing that loses sight of it.

## Why it matters

Goal visibility: asking for a plan is the first thing a new user does, and a write takes up to a minute of paid
model calls. The lead's ask (round v8, 2026-11-12): a plan write publishes its spend as it goes on the event stream,
with plan-writing calls priced as they settle, and the plan list shows a row for a plan being written.

## Where

- `crates/roko-cli/src/plan_authoring.rs::AuthoringSpend::record`: what a settled plan-writing call publishes
  (serve passes it the hub it streams, `serve_runtime.rs::spend_bridge`).
- `crates/roko-serve/src/routes/plans/reads.rs::list_plans`: `GET /api/plans`.
- `crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation`: the write's operation.

## Current state

- `call_cost` (`DashboardEvent::CallCost`) is published only by a Graph task's tool loop. The snapshot doesn't
  fold it into totals, so a host that sums it doesn't double-count `efficiency_event`.
- `GET /api/plans` lists plan directories (`runtime.list_plans`).

## Plan

1. `AuthoringSpend::record` also publishes `call_cost` for each plan-writing call as it settles:
   - the call's model, its tokens, and its price (`cost_usd`, null when no rate prices its model);
   - its number in the write (`call`, under attempt 1);
   - what the write has spent so far (`attempt_cost_usd`).
   A host then shows a write's spend as it shows a run's. The `efficiency_event` stays, for older hosts.
2. `GET /api/plans` adds a row for each plan being written that isn't on disk yet. The row has the plan summary's
   fields (`status: "writing"`, no tasks) and its title (the given title, or the request's first line). It also has
   `writing: { run_id, stage, elapsed_secs, calls, cost_usd, max_usd }`. The other rows don't change.
3. Tests:
   - `plan_writing_calls_publish_their_cost_as_they_settle` (roko-cli);
   - `the_plan_list_shows_a_plan_being_written` (roko-serve).

## Done when

- Each plan-writing call publishes `call_cost` as it settles, with the write's running total.
- `GET /api/plans` lists a plan being written, until its plan is on disk or the write ends.
- The `[[verify]]` passes.

## Notes

- Split from gap-6c4d24 (round v8 portal-v8). Its portal half shipped without these server parts.
- The cancel is bug-36ce72. The portal's stop key, SPENT and writing row are gap-f8c57a.
