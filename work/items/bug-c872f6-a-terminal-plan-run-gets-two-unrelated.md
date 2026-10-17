+++
id = "bug-c872f6"
kind = "bug"
title = "A terminal plan run gets two unrelated run ids, graph-<plan>-<uuid> for its records and graph-<uuid> for its events, prints neither, and serve's summary answers only the events id"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#R4-2"
discovered_from = "agent:dogfood-will (round v6 run 4, 2026-10-17)"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs::graph_run_id", "crates/roko-cli/src/graph_checkpoint.rs::create_fresh_checkpoint"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cli_plan_run_uses_one_run_id_for_records_and_events' crates/roko-cli/ && cargo test -p roko-cli cli_plan_run_uses_one_run_id_for_records_and_events"
+++

## Problem

A terminal `roko plan run` gives one run two unrelated ids, and prints neither.

Round v6 dogfood run 4 (2026-10-17, roko b2970cb12) ran two terminal runs: `plan run plans/svf-g4 --model … --fresh
--frozen-learning --budget-override … --no-tui --no-serve`, one pinned to gpt-5.4-mini and one to claude-sonnet-4-6.
Each run's records sit under one id and its events under another:

| | strong run | top run |
|---|---|---|
| run record (`.roko/runs/<id>/`: manifest, attempts, exposures, decisions) | `graph-svf-g4-20cd2f9d-…` | `graph-svf-g4-4624823f-…` |
| event log and serve's index (`.roko/events.jsonl`, `events-by-run/`) | `graph-9573f566-…` | `graph-3b4d3d85-…` |
| batch branch the CLI prints | `roko/batch/20261017T141034Z-3888bbc8` | `roko/batch/20261017T141247Z-f60b9f63` |

- `GET /api/runs/graph-svf-g4-20cd2f9d-…/summary` answers 404, and so does the batch id. Only the events id answers,
  and nothing the run prints leads to it.
- The run's terminal output ends with the batch branch and the cost ($0.14). Linking a serve summary to a run record
  means matching titles and times by hand.

## Why it matters

Goal `visibility`: a CLI run is the case serve exists to show next to chat runs. Hosts, `roko diagnose`, guides and
evidence all need one id per run. Here the attempt verdicts (records id) and the events (events id) of the same run
cannot be joined on an id.

## Where

- `crates/roko-cli/src/graph_execution/event_log.rs::graph_run_id` returns the caller's run id, else a new
  `graph-<uuid>`.
- `crates/roko-cli/src/graph_checkpoint.rs::create_fresh_checkpoint` returns the caller's run id, else a new
  `graph-<plan>-<uuid>`.
- When `plan run` gets no run id from a caller (serve passes one; the terminal does not), each mints its own.

## Current state

Runs started through serve have one id everywhere, because serve passes its `run_id` to both. Only CLI-started runs
split.

## Plan

1. Mint the run id once, where a CLI plan run starts. Pass it to the checkpoint, the event log, the attempt book and
   the batch.
2. Print it at the start and in the closing summary ("run graph-svf-g4-…: …"), so `roko diagnose <id>`, `GET
   /api/runs/<id>/summary` and the record directory all take the same id.
3. Add the test `cli_plan_run_uses_one_run_id_for_records_and_events`: a fake-provider plan run without a caller id
   writes its manifest and its event index under the same id.

## Done when

- The `[[verify]]` passes.
- A terminal run's printed id answers `GET /api/runs/<id>/summary`.

## Notes

Evidence: `tmp/hermes-roko-v6/dogfood-will/logs/run4-summary-*.json` (the 404s and the 200s),
`tmp/hermes-roko-v6/dogfood-will/evidence/run4/`.
