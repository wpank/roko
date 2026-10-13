+++
id = "gap-7c6663"
kind = "gap"
title = "A confirm rung's wait publishes no event, so a run's event stream cannot show that it waits on the person"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::confirm_rung", "crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs"]
links = { depends_on = [], blocks = [], related = ["gap-25d9cc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_confirm_wait_is_published' crates/roko-cli/ && cargo test -p roko-cli a_confirm_wait_is_published"
+++

## Problem

While a `confirm` rung waits for the person, the run's event stream says nothing about it. In run
`1a778f62-e889-4344-93ee-b4d21d4b92ae` (Hermes dogfood, 2026-10-13, assistant pack, confirm rung over `REPORT.md`):

- `agent_completed` arrives at 14:28:52.8Z, followed only by `agent_heartbeat` rows until `gate_result
  rung[sign-off] passed` at 14:30:24Z. That row's `output_text` is `$ confirm REPORT.md`.
- No `gate_rung_started` row appears for the sign-off rung, and no row says that a person must answer, what the
  question is, or by when.
- The passing `gate_result` does not say that a person approved it. The attempt record keeps
  `confirmed_by_user`, but the event does not.

## Why it matters

Hosts that follow the event stream cannot show "waiting for your sign-off since 14:28" or "approved by you at
14:30" without polling `confirm_pending` on the side (goal `visibility`). This includes Hermes' `roko watch`, its
run cards and timeline, and SSE clients. A run that waits on a person looks like a run that hangs.

## Where

- `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::confirm_rung`: writes the hold, polls the review
  log, and removes the hold.
- `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs`: the pack-rung loop that calls it.
- `DashboardEvent` already has `GateRungStarted` and `InboxItemReceived`.

## Current state

Command rungs publish `gate_rung_started`. Pack rungs, the confirm rung among them, do not.

## Plan

1. Publish `GateRungStarted` for every pack rung, the confirm rung included.
2. When the hold is written, publish an event carrying the run id, plan and task, attempt key, question and
   deadline. This could be `InboxItemReceived` with a review category, or a new `ConfirmRequested`.
3. Publish an event when the hold is answered or times out.
4. Put the decision (approved, rejected or no answer) and the note in the `gate_result` detail.

## Done when

`[[verify]]` passes: a confirm rung's wait and its answer appear in the run's event index.
