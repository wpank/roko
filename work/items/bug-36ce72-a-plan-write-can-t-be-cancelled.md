+++
id = "bug-36ce72"
kind = "bug"
title = "A plan write can't be cancelled: plan_generate answers with its operation id as run_id, but run_cancel looks only for prompt and plan runs and fails not_found for that id, so phase 5.5's cancelled write f43afc45 ran on to completion and was billed (the 'interrupted' the host showed is the plugin's wording)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-serve/mcp"]
created = 2026-10-28
updated = 2026-10-28
last_verified = 2026-10-28
source = "round-v7 team-lead: phase 5.5 plan write, op f43afc45, roko-main-38bedecea"
discovered_from = "agent:hermes-pilot (round v7, phase 5.5), the lead's ask"
anchors = ["crates/roko-serve/src/routes/mcp.rs::cancel_run", "crates/roko-serve/src/state.rs::OperationHandle", "crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation"]
links = { depends_on = [], blocks = [], related = ["bug-a0f01e", "bug-1e8263"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_cancel_stops_a_plan_write' crates/roko-serve/src && cargo test -p roko-serve --lib run_cancel_stops_a_plan_write"
+++

## Problem

roko has no way to stop a plan write once it has started. The MCP `plan_generate` tool answers with the operation's
id as `run_id`, the id that `run_status` follows. But `run_cancel { run_id }` looks only for prompt runs, queued
prompt runs and plan runs. For that id it finds nothing and fails with `not_found`, "no active execution for this
plan" (404 on `POST /api/plans/{id}/cancel`). The planner call runs on and is billed.

Phase 5.5 on roko-main-38bedecea (Undertow d273d60): the chat cancelled plan write f43afc45, then asked again with a
cap. In `~/dev/undertow/.roko/state/operations.jsonl`, f43afc45 ran its full 32 seconds and ended `completed` at
11:03:29Z, for $0.0374. It also raced the second write, c281646c, for the same plan directory (bug-1e8263).

The host told the person the write was `interrupted`: "roko serve stopped while this run was going (a restart or a
crash)". **That wording is the plugin's, not roko's.** It is how hermes-plugin-roko marks a cancel that finds
nothing running, from its v0.5 handling of older roko versions: its test
`test_cancel_finding_nothing_running_marks_an_older_rokos_run_interrupted` (`tests/test_v05_runs.py`), and its desktop
card marks that reason `source: 'plugin'`. roko's own reason for a run whose process died is "roko stopped while this run was going
(machine restart or crash)" (`crates/roko-serve/src/interrupted_runs.rs::INTERRUPTED_REASON`), and it is given only
for a run whose manifest is open. A plan write has no run manifest, under `.roko/runs/` or anywhere else. plugin-wow
handles the plugin's side. This item is roko's: the cancel has nothing to stop the write with.

## Why it matters

- **Money.** A person who cancels a write still pays for it, at whatever cap it started with: here roko's $2.00
  default, which the person had meant to lower.
- **Wrong state.** Whatever a host shows after the cancel is wrong: the write keeps going, and it can still
  overwrite the plan the person asked for next.
- **Contract.** The MCP contract presents the operation id as a `run_id`, and `run_cancel` takes a `run_id`. A host
  can't tell that this one id is uncancellable.
- Goal: golden-path. Related: bug-a0f01e (operation records; it notes that nothing produces a `Cancelled` status
  yet).

## Where

- `crates/roko-serve/src/routes/mcp.rs::cancel_run`, the `run_cancel` tool, tries in order:
  - `routes/run.rs::cancel_background_run` (`AppState.active_runs`: prompt runs);
  - `cancel_queued_prompt_run`;
  - `routes/plans/run_control.rs::cancel_plan_run`. That covers active plans, the plan queue, and then
    `interrupted_runs::settle`, which needs a run manifest. It fails with `not_found` for an operation id.
- `crates/roko-serve/src/state.rs::OperationHandle` holds the write's `JoinHandle` and no cancel token. No route
  aborts an operation: `GET /api/operations/{id}` only reads.
- `crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation` spawns the write
  (`runtime.generate_plan_reporting`). The planner call under it is a CLI subprocess or an HTTP stream.
- `crates/roko-serve/src/routes/mcp.rs::generate_plan` answers `run_id: <operation id>`.

## Current state

- A plan write is an operation, not a run. It has no run manifest and no entry in `active_runs` or
  `active_plans`.
- `operations.jsonl` keeps each finished operation's status. Nothing ever records `cancelled`.

## Plan

1. Give `OperationHandle` a cancel token, and pass it to the plan write, so a cancel stops the planner call. That
   means a CLI planner's process group, or dropping the HTTP stream.
2. Record the operation as `cancelled` in its handle and in `operations.jsonl`, with what it spent so far. Publish
   `OperationCompleted { success: false }`.
3. Make `run_cancel` look in `AppState.operations` for a running plan write under that id, before the run lookups,
   and answer `{ run_id, state: "cancelled" }`. Add `POST /api/operations/{id}/cancel` for the REST side.
4. A write cancelled before it writes leaves no plan directory. One cancelled while writing leaves the directory as
   it was before.
5. Test: start a plan write held at its planner call, then `run_cancel` its `run_id`. It answers `cancelled`, the
   planner is stopped, and `run_status` says `cancelled`.

## Done when

- `run_cancel` on the `run_id` that `plan_generate` returned stops the write and answers `cancelled`, and
  `run_status` and `GET /api/operations/{id}` report `cancelled`.
- The `[[verify]]` passes.

## Notes

- I couldn't tell from roko's records which call the plugin sent: `auth-audit.jsonl` records `POST /mcp` without a
  tool name. Whichever it was, roko has no path that stops a plan write. plugin-wow is checking the plugin's call
  and owns the "interrupted" wording.
