+++
id = "bug-09e21e"
kind = "bug"
title = "A prompt run refused by the workspace runner lock is recorded in the active run's event index, so the active run's summary says failed and the refused run's id answers 404"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-serve/runs"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
last_verified_rev = "137755aa1"
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs::WorkspaceLogWriter", "crates/roko-serve/src/lib.rs::server_event_to_dashboard", "crates/roko-serve/src/lib.rs::run_plan_id", "crates/roko-cli/src/serve_runtime.rs::run_prompt_plan_on_local_runtime"]
links = { depends_on = [], blocks = [], related = ["gap-9a8d66"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn foreign_plan_events_stay_out_of_the_runs_index' crates/roko-cli/ && cargo test -p roko-cli foreign_plan_events_stay_out_of_the_runs_index"

[closed]
at = 2026-10-13
at_ts = "2026-10-13T16:35:50Z"
commit = "137755aa1"
by = "roko-fixes (Claude fork, hermes-plugin dogfood)"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-13T15:31:01Z"
forced = false
evidence = "137755aa1: a run's workspace event log (crates/roko-cli/src/graph_execution/event_log.rs, WorkspaceLogWriter::owns) records only its own events: one naming another run, or a plan outside the run's plan set from its PlanSetLoaded, is neither indexed nor stamped with its id; serve queues prompt runs so a second one no longer fails at the lock inside another's run (gap-9a8d66); a prompt run refused at the runner lock writes its error and run_completed into its own index (serve_runtime.rs record_refused_run), and a queued run cancelled before it started records run_completed cancelled in its index (routes/run.rs record_unstarted_run). Verify foreign_plan_events_stay_out_of_the_runs_index passes; roko-cli lib 3619 passed (gist test skipped), roko-serve 1014 lib + all integration tests passed, clippy -D warnings clean on both."
+++

## Problem

Two prompt runs started over `/mcp` `run_prompt` a few seconds apart in one workspace (Hermes plugin dogfood,
2026-10-13, `roko serve` built from `6ab6fde47`, a small Python project):

- Run `c237c5b3-d2ad-4e63-a47e-bb02fa6ce274` held the runner lock and succeeded. Its own rows end with
  `task_completed T1 passed`, `plan_completed success=true` and `run_completed outcome=succeeded`, and its batch
  branch has the change (2 files, +17).
- Runs `da891352-1709-45bf-bc8c-ac8417494046` and `176016c8-c4df-4696-9d06-93f7ff93c8a7` got a `run_id` back,
  then failed with `run failed: a run is active in this workspace`.
- Their rows (`task_started`, `agent_spawned`, `plan_started`, `task_completed outcome=failed`, `error`,
  `plan_completed success=false`, for plans `run-da891352` and `run-176016c8`, with the prompt text as the task
  id) are in c237c5b3's event index, stamped `run_id = c237c5b3-…`.
- So `GET /api/runs/c237c5b3-…/summary` says `state: failed`, `verdict: failed`, `tasks: {passed: 1, failed: 2,
  total: 3}`, lists the refused runs' plans under `untrusted_input_boundary`, and its milestones end "run failed"
  right after "task T1 passed".
- `GET /api/runs/da891352-…/summary` and `/events` answer 404: a refused run leaves no record at all.

Hosts show the run that succeeded as failed ("failed · gates 1/1 · tasks 1/3") and the refused ones as failed with
no reason.

## Why it matters

A chat host reports the run summary as the truth (goal `truth`; the Hermes integration reads nothing else). A run
that succeeded but is reported as failed, and run ids that vanish, break the claim that roko's verdicts can be
trusted. In the dogfood session, Hermes retried the vanished run and tripped its own tool-loop guard.

## Where

- `crates/roko-cli/src/graph_execution/event_log.rs::WorkspaceLogWriter`: `record` stamps every event the shared
  hub publishes with its own run id (`fields.entry("run_id").or_insert_with(…)`) when the event carries none.
- `crates/roko-serve/src/lib.rs::server_event_to_dashboard`, `run_plan_id`: serve's bridge publishes a one-shot
  run's `RunStarted`, `RunCompleted` and `Error` on the shared state hub as `PlanStarted { plan_id: "run-<8>" }`,
  `PlanCompleted` and `Error`, with no run id. The run's task and agent rows have none either.
- `crates/roko-cli/src/serve_runtime.rs::run_prompt_plan_on_local_runtime`: where the lock refuses the second run.

## Current state

Any event published on serve's hub while a run is active is taken into that run's index, and the run's summary
counts its tasks. That covers another run's events and server-wide errors alike. Nothing records a refused
prompt run under its own id.

## Plan

1. Have the serve bridge publish the run id with a one-shot run's events. Alternatively, have `WorkspaceEventLog`
   take only events whose `plan_id` is in its run's plan set, plus the run-level events it writes itself.
2. Never stamp a foreign event with the recording run's id: drop it, or route it to its own run's index.
3. Give a refused run a record of its own: a manifest plus an index row with the refusal, so its id resolves to
   `failed: a run is active in this workspace` instead of 404. The queueing item filed alongside this one may
   remove the refusal altogether.
4. Test it: start two prompt runs on one serve with the second refused. The first run's summary must stay
   `succeeded` with 1 task, and the second run must have a summary that names the refusal.

## Done when

`[[verify]]` passes: a refused or concurrent run's events never enter another run's index, and both summaries
tell the truth.

## Notes

Found by the hermes-plugin-roko dogfood: Hermes sessions `20261013_161252_7fbc4b` and `20261013_161252_ce4679`.
The workspace was a scratch project, so the evidence is not in the repo.
