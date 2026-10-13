+++
id = "gap-9a8d66"
kind = "gap"
title = "/mcp run_prompt hands out a run_id and then fails the run at once when another run holds the workspace, while plan_run queues"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/mcp"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
last_verified_rev = "137755aa1"
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-serve/src/routes/mcp.rs::start_prompt_run", "crates/roko-cli/src/serve_runtime.rs::run_prompt_plan_on_local_runtime"]
links = { depends_on = [], blocks = [], related = ["bug-09e21e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_second_prompt_run_waits_or_is_refused_up_front' crates/roko-serve/ && cargo test -p roko-serve a_second_prompt_run_waits_or_is_refused_up_front"

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
evidence = "137755aa1: prompt runs and plan runs share serve's run queue (AppState::prompt_queue beside plan_queue, oldest first across both; run_control.rs workspace_busy/start_queued_run, run.rs start_gated_run). A second run_prompt on a busy workspace answers state queued with its position and starts under the id it was handed when the runs before it end; run_status and GET /api/run/{id}/status answer for it, run_cancel takes it out of the queue; a full queue or a runner lock held outside serve (CliRuntime::runner_lock_holder) refuses it before any id. Documented in the run_prompt tool description and docs/v3/26-HTTP-API.md. Verify a_second_prompt_run_waits_or_is_refused_up_front passes; roko-serve 1014 lib + integration tests passed."
+++

## Problem

`/mcp` `run_prompt` answers `{run_id, state: "running"}` at once. When another run holds the workspace's runner
lock, the run then fails within about a second with `run failed: a run is active in this workspace`.
`plan_run` behaves differently: a plan run started while another plan runs waits in serve's plan queue (state
`queued`, with a position).

In the Hermes dogfood (2026-10-13), a person asked for two small features at nearly the same time. The second
`run_prompt` came back with a run id, the host polled it and found the run gone (see the companion bug about
misattributed rows and 404s). Hermes' model then retried the same call twice, and both retries were refused the
same way. That tripped Hermes' `same_tool_failure` warning, and the person saw three failed runs with no reason.

## Why it matters

Chat hosts commonly send two small requests one after the other, and a run id that fails asynchronously for a
reason the host cannot see is the worst outcome (goal `hermes`).

## Where

- `crates/roko-serve/src/routes/mcp.rs::start_prompt_run`: answers with the run id before the run takes the lock.
- `crates/roko-cli/src/serve_runtime.rs::run_prompt_plan_on_local_runtime`: takes the runner lock and fails at once
  ("One plan executor at a time: a prompt run fails at once while a plan run, or another prompt run, holds the
  lock").
- Serve's plan queue (`AppState::plan_queue`) is what `plan_run` uses.

## Current state

The refusal is deliberate (decision 9113 scopes one plan executor per workspace), but the host only learns about
it by polling, and the refused run leaves no record (bug filed alongside this item).

## Plan

Pick one and document it in the `/mcp` tool description:

1. Queue a prompt run behind the active run, as `plan_run` does: `state: queued`, `position`, started when the lock
   frees. This is the friendlier option for chat hosts.
2. Refuse synchronously: return a JSON-RPC error naming the active run (and how to wait for it or cancel it), and
   never hand out a run id for a run that cannot start.

## Done when

`[[verify]]` passes: a second `run_prompt` on a busy workspace either waits and then runs, or is refused before
any run id is returned. It never fails after the run id has been handed out.

## Notes

The plugin side (hermes-plugin-roko) can show "queued" once roko reports it.
