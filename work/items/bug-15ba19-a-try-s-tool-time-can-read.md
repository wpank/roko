+++
id = "bug-15ba19"
kind = "bug"
title = "A try's tool time can read 0 ms with the time counted as model time: T2 of a fixture run gave run_tests 2 calls 0 ms (model_ms 11.6 s) though each check ran about 6 s"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-19
updated = 2026-10-19
source = "round-v7-roko-speed#c-770cb5"
discovered_from = "agent:roko-speed (round v7, fixture run c-770cb5, 2026-10-20)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/live_tool_calls.rs::LiveToolCalls"]
links = { depends_on = [], blocks = [], related = ["gap-6a319c", "gap-770cb5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_late_done_leaves_tool_time_with_the_tools' crates/roko-cli/src && cargo test -p roko-cli --lib a_late_done_leaves_tool_time_with_the_tools"
+++

## Problem

A try's tool time can read 0 ms, with the time counted as model time instead.

In the $0 fixture run `c-770cb5` (round v7, `$V7/roko-speed/runs/c-770cb5`), the stand-in agent called
`run_tests` with `build: "checks"` twice in each task. Each check really ran: T2's took 5.9 s and 5.6 s, as the tool
results say. Yet T2's verdict has these phases:

| Field | Value |
|---|---|
| `tools_by_name` | `run_tests` 2 calls 0 ms, `write_file` 1 call 0 ms, `read_file` 1 call 28 ms |
| `tools_ms` | 28 |
| `model_ms` | 11,587 (the stand-in answers at once) |

T1, T3 and T4 in the same run were attributed about right: `run_tests` took 2.1 s, 13.1 s and 19.1 s. T3's
`write_file` also reads 0 ms for 5 calls, where T1's 5 writes read 720 ms.

## Why it matters

plugin-wow's timing strip and the round's speed work read `phases`. A check-heavy try that reports its build time as
model time sends speed work to the wrong place.

## Where

- `crates/roko-cli/src/graph_task_dispatch/live_tool_calls.rs`: `observe_at` and `timing`. A call's span runs from
  `max(announce, latest Done ≤ result)` to its result. A span is empty when a model call's `Done` is observed right
  before the tool's result.
- The live tap's events: `immune_boundary.rs` forwards them with `try_send` into a 64-slot channel, which can drop or
  bunch them.

## Plan

1. Reproduce: record the tap's event order and receive times for a try like T2.
2. Base a call's start on the model call that announced it, not the latest `Done` before the result. If the order is
   unreliable, fall back to the announce.
3. Add a test with a `Done` that arrives after the tools ran.

## Done when

A try whose tools run for seconds reports them in `tools_ms` and `tools_by_name`, whatever order its `Done` and tool
results reach the tap. Checked by the test in `[[verify]]`.

## Notes

- Found by roko-speed (round v7) in gap-770cb5's fixture run, from roko-speed's own gap-6a319c timing code.
