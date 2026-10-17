+++
id = "bug-e529e3"
kind = "bug"
title = "A run whose event index outgrows the summary's scan window (10,000 events or 8 MB) reads 'running' forever in GET /api/runs/{id}/summary and the MCP run_status, and its diff route answers 409"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/routes"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#R3-5"
discovered_from = "agent:dogfood-will (round v6 run 3, 2026-10-16)"
anchors = ["crates/roko-serve/src/routes/runs.rs::summary_and_plans", "crates/roko-serve/src/routes/runs.rs::run_summary", "crates/roko-serve/src/routes/runs.rs::MAX_DETAIL_EVENTS"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-dceafd", "gap-5fda00"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn summary_sees_the_end_of_a_run_longer_than_its_scan_window' crates/roko-serve/ && cargo test -p roko-serve summary_sees_the_end_of_a_run_longer_than_its_scan_window"
+++

## Problem

A run whose event index holds more than the summary's scan window reads "running" forever.

Run `ea0c32cb` (round v6 dogfood run 3, 2026-10-16, roko 47b3cdd20, `[serve] live_agent_output = "trusted"`) failed at
22:47:41 after 15m27s.
- Its run index (`.roko/events-by-run/<sha256(run_id)>.jsonl`) holds 21,409 events (8.17 MB). 21,103 of them are
  `agent_output`, mostly one-token glm-4.7 reasoning deltas.
- `plan_completed` and `run_completed` are its last rows: 21,403 and 21,409, at 8.17 MB.
- From 22:55 until serve stopped, `GET /api/runs/ea0c32cb…/summary` answered `state: "running"`, `active: true` and
  `finished_at: null`, with tasks 2/2 passed. Those two tasks are OSC and ENV, whose completions are rows 136 and 206.
- The Hermes page showed the run as failed, because it reads the live hub.

The MCP `run_status` tool returns the same summary (`summarize_run`), so a chat host that asks about this run hears
"running". `GET /api/runs/{id}/diff` reads it too, and refuses with 409 "still active".

## Why it matters

Goals `visibility` and `hermes`: the summary is the one answer hosts post about a run. Trusted live output, which round
v6's demos turn on for the streaming words, makes a long index normal, since every reasoning token is an event. A run
that never reads as ended blocks every route that waits for the end.

## Where

- `crates/roko-serve/src/routes/runs.rs::summary_and_plans` reads the run's index from the start, through
  `read_for_run(…, 0, MAX_DETAIL_EVENTS, MAX_DETAIL_SCAN_BYTES, …)` (10,000 events, 8 MB), and hands that page to
  `run_summary`.
- `crates/roko-serve/src/routes/runs.rs::run_summary` derives completion, verdict and task outcomes only from the
  events it is given. With no terminal event among them and no in-memory handle, the run is `running`.
  - Here the resume's handle had replaced this run's handle under the plan key (`launch_single_plan_run` inserts by
    `spec.key`). Without the resume, the old handle's recorded end would have hidden the problem until serve
    restarted. After a restart, every run this long reads this way.
- The detail route `GET /api/runs/{id}` and the diff route (`get_run_diff`, through `summary_and_plans`) use the same
  limits.

## Current state

- Nothing reads the tail of the index.
- The run's manifest (`.roko/runs/<run>/manifest.json`, `closed`) records how the run ended. But after a resume, that
  record belongs to the resumed invocation: for `ea0c32cb`, whose resume ran as `c598613c`, it says
  `closed.status = "succeeded"`. So it is not a drop-in source for the original run's verdict.
- Evidence: `tmp/hermes-roko-v6/dogfood-will/evidence/run3/events-by-run/ea0c32cb-7cde-4ec7-9ef7-1a9de68d7f1b.jsonl.gz`
  and `tmp/hermes-roko-v6/dogfood-will/logs/summary-final-ea0c32cb.json`.
- Finding R3-5 in `tmp/hermes-roko-v6/findings/dogfood-will.md` blamed the resume; the cause is this scan window.

## Plan

Options:
1. Don't count `agent_output` and `agent_heartbeat` events against the summary's event budget. Check first that
   `run_summary` uses none of them: it reads lifecycle, task, gate, spawn and cost events. The byte limit still bounds
   the read.
2. Read the lifecycle tail: after the bounded read from the start, read the last few hundred KB of the index for
   terminal events (`plan_completed`, `run_completed`, `run_failed`) and merge them.
3. Keep a small per-run lifecycle record (started, ended, verdict, cost, task outcomes), written as the run goes, and
   let the summary read that.

Option 2 is the smallest correct change; option 3 is the durable one.

Add the test `summary_sees_the_end_of_a_run_longer_than_its_scan_window`: an index with a start, two task completions,
12,000 `agent_output` rows and a failed end gives `state: "failed"`, `active: false` and the right task counts.

## Done when

- The `[[verify]]` passes.
- The summary of a trusted-output run with more than 10,000 events reads the run's real end state.

## Notes

- Keep the read bounded: the limits stop a huge index from stalling the route.
- Related: bug-dceafd (an interrupted run's end state) and gap-5fda00 (live agent output).
