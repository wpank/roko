+++
id = "bug-0b7c81"
kind = "bug"
title = "Live agent_output records published during a Graph attempt say attempt 0"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
last_verified_rev = "9f60e5c16"
source = "roko-loops worker for the hermes-plugin-roko live UI, 2026-10-14 (roko main 216f88c84)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/tui_forward.rs::forward_live_event_to_tui", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs::live_output_tap"]
links = { depends_on = [], blocks = [], related = ["bug-369c21"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn live_records_carry_their_attempt' crates/roko-cli/ && cargo test -p roko-cli live_records_carry_their_attempt"

[closed]
at = 2026-10-14
at_ts = "2026-10-14T09:55:18Z"
commit = "9f60e5c16"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-14T08:26:42Z"
model = "claude-opus-5-5"
forced = false
evidence = "9f60e5c16: WatchedAttempt carries the attempt ordinal and forward_live_event_to_tui stamps it on every live record, on the event and inside the record (watchdog.rs, tui_forward.rs, streaming.rs, graph_task_dispatch.rs). Verify: cargo test -p roko-cli live_records_carry_their_attempt passes. Checked at the top of the stacked branches (all six roko-loops items): cargo test -p roko-cli --lib passed except the date-keyed active_self_model_picks_start_rung_and_logs_propensity (fails on 2026-10-14 without these changes too), learning_wiring_census passed, clippy -D warnings clean on roko-core, roko-cli and roko-serve; a zero-cost roko serve smoke (fake Claude CLI) showed the event in the run's index."
+++

## Problem

The live `agent_output` records a Graph attempt publishes while its provider call runs (`\u001eroko.stream.v1`
records with `live: true`: tool steps, and under `live_agent_output = "trusted"` text, reasoning, tool calls and
results) carry `"attempt": 0`, both on the event and inside the record. bug-369c21 numbered the records published
after the call, not these.

## Why it matters

A host shows live tool steps per attempt; with every live record on attempt 0 a retried task's steps run into one
another (goal `visibility`).

## Where

- `crates/roko-cli/src/graph_task_dispatch/tui_forward.rs::forward_live_event_to_tui`: passes `0`.
- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs::live_output_tap` and `WatchedAttempt`: the tap that calls it.

## Current state

`WatchedAttempt` knows the attempt key, not its ordinal.

## Plan

Give `WatchedAttempt` the attempt ordinal (the attempt key's), pass it through the tap to
`forward_live_event_to_tui`, and use it for every live record.

## Done when

`[[verify]]` passes: live records carry the attempt they belong to.
