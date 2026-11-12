+++
id = "bug-fb88bf"
kind = "bug"
title = "A hand-off check's tool id repeats across a run's tries: roko names it roko-hand-off-{turns}, so two tries that hand off after the same number of turns share an id (\"roko-hand-off-11\" for tasks 1 and 2 of Undertow run ecc0f747), and a reader that keys tool calls by id drops one (the Hermes plugin's live relay did)"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round-v8-run-analyst-v8#handoff-ids"
discovered_from = "agent:run-analyst-v8 (round v8, checking log_run's rows, 2026-11-12)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::send_back"]
links = { depends_on = [], blocks = [], related = ["bug-cbbaa8", "gap-6a319c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn hand_off_check_ids_are_unique_across_tries' crates/roko-agent/src && cargo test -p roko-agent --lib hand_off_check_ids_are_unique_across_tries"
+++

## Problem

When a try hands off, roko runs the task's checks as a `run_tests` call and publishes it as a tool call with the id
`roko-hand-off-{iterations}` (`send_back`). The id counts only the try's own turns. Any two tries in a run that hand
off after the same number of turns get the same id, and that's common.

Round v8's Undertow phase 1 run ecc0f747 (roko ba5866664, 2026-11-12) has one:

| Try | `tool_start` | `tool_result` | Output |
|---|---|---|---|
| Task 1, try 1 | 16:10:38.519 | 16:10:41.130 | "the task's 4 checks pass" |
| Task 2, try 1 | 16:13:32.261 | 16:13:39.388 | "the task's 2 checks pass" |

Both are `roko-hand-off-11`: each try made 12 model calls. Round v7's Undertow runs have 102 hand-off `tool_start`
records in 12 runs.

Every stream record also names its task and try, so a reader can key by (task, attempt, id). Readers that key by the
id alone lose calls:

- **The Hermes plugin's live relay** (`live_relay.py`, `_seen_tool`) drops a tool step whose id it has already sent
  in that run. It does this to merge roko's double publication of one call (the tool-steps record and the trusted
  one). Task 2's hand-off check in ecc0f747 was such a step, so a chat's live feed never showed it.
- **Round v8's run telemetry** (`extract.read_index`, run-analyst-v8) merged the two calls into one. Task 2's 7.1 s
  check vanished, and its time counted as model replies. That's fixed on the reader's side: it now keys by try as
  well.

A model's own call ids (`call_…`) are unique. roko's are the only ones that repeat.

## Why it matters

Goal `visibility`: what a person watches live should show every step. Tool-call ids are what readers use to join a
call's start to its result and to merge duplicate records. An id that repeats within a run makes any of them silently
drop or merge calls. Nothing fails, so nobody notices.

## Where

- `crates/roko-agent/src/tool_loop/mod.rs::send_back` makes the id: `format!("roko-hand-off-{iterations}")`.
- The tests that match the prefix (`id.starts_with("roko-hand-off")`) and the exact ids `roko-hand-off-1` and `-3`
  are in the same file's tests.

## Current state

Not fixed. Filing only (round v8, the lead's call): nobody fixes it this round. The run telemetry works around it.

## Plan

1. Make the id unique within a run. Two options:
   - add something only this try has, e.g. `roko-hand-off-{attempt key or loop instance}-{iterations}`;
   - add a short random or per-process counter.

   Keep the `roko-hand-off` prefix, which readers match on.
2. Update the tests that expect `roko-hand-off-1` and `-3` to match the new form, by prefix.
3. Test `hand_off_check_ids_are_unique_across_tries`: two tool loops that each hand off after the same number of turns
   publish different ids.

## Done when

`hand_off_check_ids_are_unique_across_tries` passes (the `[[verify]]` command).

## Notes

- Evidence: round v8's run-analyst-v8 (`tmp/hermes-roko-v8/findings/run-analyst-v8.md`, 17:4x), read at $0 from
  Undertow's `events-by-run` index for ecc0f747.
- The plugin's `_seen_tool` could also key by (task, try, id). That's the plugin's own work, and an id that is unique
  in the run makes it unnecessary.
- Related: bug-cbbaa8 (the hand-off check that sends a try back) and gap-6a319c (tool spans in a try's settlement).
