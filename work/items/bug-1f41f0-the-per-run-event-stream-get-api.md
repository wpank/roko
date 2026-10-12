+++
id = "bug-1f41f0"
kind = "bug"
title = "The per-run event stream (GET /api/runs/{id}/events/stream) sends only keepalives for Graph runs"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/runs"]
created = 2026-10-12
updated = 2026-10-12
last_verified = 2026-10-12
source = "hermes-plugin-roko 0.2.0 UI work, roko serve probe 2026-10-11 (scratchpad roko-ui-data.md)"
anchors = ["crates/roko-serve/src/routes/runs.rs::run_events_sse"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn per_run_stream_carries_a_graph_runs_events' crates/roko-serve/ && cargo test -p roko-serve per_run_stream_carries_a_graph_runs_events"
+++

## Problem

`GET /api/runs/{id}/events/stream` (and `GET /api/runs/{id}/events` with `Accept: text/event-stream`) sends
only `: keepalive` comments for every Graph run, live or finished. Observed on 2026-10-11 (roko serve
`856ed7570`): 10 s on a finished plan run returned nothing else, and `?source=runtime` on the same run answered
404 `has no event index`.

## Why it matters

It is the documented per-run live stream, and the only one keyed by run id: the hub stream (`/api/events`) is
keyed by plan id, and a prompt run appears there twice (its real plan and a synthetic `run-<8 hex>` one). A
client following one run must poll `GET /api/runs/{id}/events?cursor=` instead. Goal: live visibility.

## Where

- `crates/roko-serve/src/routes/runs.rs::run_events_sse` forces `source = "runtime"`: it replays only the
  runtime index and streams only the runtime bus (`SseAdapter`). Graph runs write the runner index
  (`events-by-run`), and their events reach the StateHub, not the runtime bus.

## Current state

`read_for_run` already reads either index and pages it by byte cursor; the JSON events route serves Graph runs
from the runner index correctly.

## Plan

1. In `run_events_sse`, when the run has no runtime index but has a runner index, replay the runner index from
   the cursor (`Last-Event-ID` or `?cursor=`), then follow it by polling for rows past the last cursor about
   twice a second, emitting each as an SSE event with `id: <cursor>`, and end the stream after `run_completed`.
2. Keep the runtime path unchanged for runs that have a runtime index.
3. A test: a Graph run's stream replays its rows, delivers a row appended later, and ends after
   `run_completed`.

## Done when

- The per-run stream of a Graph run carries its events, live and replayed, with resumable ids.
- The `[[verify]]` test passes.
