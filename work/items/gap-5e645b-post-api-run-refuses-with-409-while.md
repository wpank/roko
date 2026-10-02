+++
id = "gap-5e645b"
kind = "gap"
title = "POST /api/run refuses with 409 while a plan run is live instead of queueing, unlike plan runs since 9111"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/routes"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "wave-3 follow-up reports 2026-10-02 (w3-pk73 gap-5e9292)"
discovered_from = "backlog task 9113 (run.rs), extending the queueing 9104/9111 built for plan runs (find-8872ad)"
anchors = ["crates/roko-serve/src/routes/run.rs::start_run", "crates/roko-serve/src/state.rs::AppState"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn prompt_run_is_queued_not_refused_while_a_plan_runs' crates/roko-serve/ && cargo test -p roko-serve prompt_run_is_queued_not_refused_while_a_plan_runs"
+++

## Problem

`POST /api/run` (`crates/roko-serve/src/routes/run.rs::start_run`, lines ~46-71) runs a prompt as a gated one-task
plan through the Graph engine. While any plan run is live in the workspace, it refuses the request outright:

```rust
if state.live_plan_runs().await > 0 {
    return Err(ApiError::conflict(
        "a plan run is active in this workspace; start the prompt run once it ends",
    ));
}
```

The route's own doc comment (lines ~46-51) and a test name (`api_run_is_refused_while_a_plan_run_is_live`,
same file, ~line 892) both say this is a known, named gap, not an oversight: "prompt runs are not queued behind
plan runs **yet**: while a plan run is live the request is refused with 409 instead of waiting for the workspace"
(backlog task 9113).

Since then, backlog tasks 9104 and 9111 (commits `3d7441310`, `54c31ff46`) built exactly this queueing machinery
for the sibling plan-run routes: `AppState.plan_queue` (a bounded `VecDeque<QueuedPlanRun>`,
`crates/roko-serve/src/state.rs:774`), with `start_plan_run` (`routes/plans/run_control.rs:327`) enqueuing a second
run and returning 202 `{run_id, queued: true, position}` instead of 409 (closing finding find-8872ad). `POST
/api/run` was not updated to use it, so a user who submits a prompt run while any plan (including one they
themselves queued) is active still gets refused outright, with no way to queue and no retry guidance beyond "once
it ends."

## Why it matters

Goal: hermes (9111, the queueing fix this extends, is itself goal=hermes) / a consistent run-admission contract.
An assistant or host (MCP `run_prompt`/`plan_run`, gap-ce1d11's package) that fires off a prompt run and a plan run
in quick succession gets an error instead of a handle for one of them, exactly the UX gap 9111 fixed for two plan
runs. `tests/endpoint_smoke.py` (entry 6 of this same report) also submits a bare prompt run and would be affected
by a concurrent plan run with no recourse.

## Where

- `crates/roko-serve/src/routes/run.rs::start_run` (the 409 check, lines ~57-61).
- The queue to extend or reuse: `crates/roko-serve/src/state.rs::AppState::plan_queue`,
  `crates/roko-serve/src/routes/plans/run_control.rs` (the queueing logic 9111 added for plan runs).

## Current state

Unfixed; the refusal and its "yet" are explicit and tested as the current, intentional interim state (backlog task
9113). No work item tracks turning it into a queue.

## Plan

1. Decide whether a prompt run queues on the *same* `plan_queue` (simplest, but mixes prompt-run and plan-run
   semantics in one structure) or a parallel, smaller queue specific to prompt runs (cleaner separation, more code).
2. On a live plan run, `start_run` enqueues the prompt run's `RunRequest` instead of refusing, returns 202
   `{id, queued: true, position}`, and starts it once the workspace frees up, mirroring `start_plan_run`'s pattern.
3. `GET /api/run/{id}/status` reports `queued` and position for a run that has not started yet.
4. Update `api_run_is_refused_while_a_plan_run_is_live` (or replace it with a
   `api_run_is_queued_while_a_plan_run_is_live` test) to match the new contract.

## Done when

- `POST /api/run` while a plan run is live returns 202 `{queued: true, position}` instead of 409, and the queued
  run starts once the workspace is free.
- The `[[verify]]` command passes.

## Notes

- Checked `gap-ce1d11` (PK74, /mcp run tools) and `gap-99c9ae` (PK76, plan.run cell / outbound effects): neither's
  task list (9115-9121, 9128-9135) mentions this queueing gap, so it is not already covered by either package;
  filing separately rather than burying it in an unrelated package's Notes.
- If a parallel queue is chosen (option 2 above), make sure a prompt run and a plan run queued against the same
  workspace are still mutually exclusive — only one of either kind should run at a time, matching "one plan executor
  runs at a time."
