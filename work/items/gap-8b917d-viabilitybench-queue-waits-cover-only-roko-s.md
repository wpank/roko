+++
id = "gap-8b917d"
kind = "gap"
title = "ViabilityBench queue waits cover only Roko's provider 429s; dispatch-slot and Claude-Code waits are unmeasured, and metrics.py's docstring is stale"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK29 gap-064c40)"
discovered_from = "gap-064c40; follows up on gap-3006e9's deferred benchmark-side wiring and gap-04e8e2's partial fix"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py::_rate_limit_waits", "benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/analysis/metrics.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_plan_slice_queue_wait_includes_dispatch_slot_time' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_plan_slice_queue_wait_includes_dispatch_slot_time -q"

[[verify]]
command = "! grep -q 'no runner records yet' benchmarks/viabilitybench/analysis/metrics.py"
+++

## Problem

Three related gaps in queue-wait measurement, all in the plan-level slice path:

1. `driver/run_roko.py` (`_rate_limit_waits`, around line 645) sets `queue_wait_s` from the metering proxy's HTTP
   429 rows only (`attempt.queue_wait_s = _rate_limit_waits(window, rows)`, line 613). It does not read the
   dispatch-slot timing (`ready_at_ms`/`dispatched_at_ms`) that the Graph engine now records per task
   (`crates/roko-graph/src/engine.rs`, landed in gap-3006e9), so a task's wait for a free parallelism slot is never
   added to `queue_wait_s` — only its wait for a rate-limited provider is.
2. `driver/run_cli.py` (the Claude Code CLI arm) sets no `queue_wait_s` at all: grepping the file for
   `queue_wait_s` finds nothing. Its subprocess-streaming loop (`_pump`/`_drain`/`_feed`, using
   `iter(stream.readline, b"")` at line 779) blocks reading the CLI's stdout between turns; that blocked-read time
   is a real wait but is not captured as `queue_wait_s` or anything else.
3. `analysis/metrics.py`'s module docstring (line 37) still says "Queue waits are read from
   `execution.queue_wait_s`, which no runner records yet." That is now false: `run_roko.py` has recorded it (via
   429 waits) since gap-04e8e2 landed. The docstring is stale and misleads a reader into thinking no runner
   measures queue waits at all, when one runner measures one kind of wait.

## Why it matters

Pilot benchmark (epic spec-567e52), proof goal. S09 §4.9 / App. D.12 promise "queue waits, meaning time that ready
work spent waiting for a dispatch slot or a provider rate limit" (`tmp/cybernetic-harness/specs/S09-experiments.md:630-632`)
— both causes, not just one. Today only the Roko arm's provider-rate-limit cause is measured; the Roko arm's
dispatch-slot cause and the whole of the Claude Code arm's wait time are silently zero/absent, which understates
`roko_plan`'s true queue wait and makes the `roko_plan`/`fd_claude` makespan-ratio comparison (S09 §4.9) less
informative than the spec assumes. gap-3006e9's own Notes already flagged the dispatch-slot half as "still to do...
wait for batch 12"; no item was filed for either half once that batch passed.

## Where

- `benchmarks/viabilitybench/driver/run_roko.py::_rate_limit_waits` (around line 645) and its caller (line 613).
- `benchmarks/viabilitybench/driver/run_cli.py` (no `queue_wait_s` producer at all; the streaming loop is
  `_pump`/`_drain`/`_feed` starting around line 769).
- `benchmarks/viabilitybench/analysis/metrics.py` (module docstring, line 37).
- Graph-side data now available: `crates/roko-graph/src/engine.rs` (`NodeTiming`: `ready_at_ms`, `dispatched_at_ms`,
  `slot_wait_ms()`, landed in gap-3006e9) and the activity log under `.roko/state/graph/<plan>/activities.jsonl`.

## Current state

Confirmed at HEAD: `run_roko.py` only ever sets `queue_wait_s` from `_rate_limit_waits`; `run_cli.py` has no
`queue_wait_s` reference anywhere; `metrics.py:37`'s docstring sentence is unchanged since before gap-04e8e2 landed.
gap-3006e9 (done, Rust side) recorded the per-task ready/dispatch timing in the engine and activity log but
explicitly deferred surfacing it to "the benchmark runner" (its own Notes) — that surfacing never happened.

## Plan

1. In `run_roko.py`, read the Graph plan's activity log (or checkpoint) for each attempt's `ready_at_ms` /
   `dispatched_at_ms`, compute the slot wait, and add it to `queue_wait_s` alongside the existing rate-limit wait.
2. In `run_cli.py`, decide what "queue wait" means for the CLI arm (likely: time blocked reading the CLI's stdout
   between tool turns, if that is judged a real queue-shaped wait under D.12 — otherwise record `queue_wait_s =
   None` explicitly rather than leaving the field entirely absent, so the report's "not recorded" accounting is
   honest about why).
3. Fix `metrics.py`'s module docstring (line 37) to describe what is actually recorded post-fix.
4. Add a test exercising a fake Roko run with both a rate-limit wait and a slot wait, asserting `queue_wait_s` sums
   both.

## Done when

- `run_roko.py`'s `queue_wait_s` includes dispatch-slot wait as well as provider rate-limit wait.
- `run_cli.py` either records a real queue-wait number or explicit `None` with a reason, consistently with the
  "not recorded" reporting in `analysis/report.py`.
- `metrics.py`'s docstring no longer says "which no runner records yet".
- The `[[verify]]` commands pass.

## Notes

- Depends on gap-3006e9's engine-side data already existing; this item is specifically the benchmark-side wiring
  that gap-3006e9's own Notes said was still needed.
- Filed from a backlog-wave follow-up report (PK29, working on gap-064c40, which itself only reconciled paper text
  and does not touch `metrics.py` or the drivers).
