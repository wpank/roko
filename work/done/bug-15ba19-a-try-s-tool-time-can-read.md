+++
id = "bug-15ba19"
kind = "bug"
title = "A try's tool time can read 0 ms with the time counted as model time: T2 of a fixture run gave run_tests 2 calls 0 ms (model_ms 11.6 s) though each check ran about 6 s"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-19
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "99dc563b8"
source = "round-v7-roko-speed#c-770cb5"
discovered_from = "agent:roko-speed (round v7, fixture run c-770cb5, 2026-10-20)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/live_tool_calls.rs::LiveToolCalls"]
links = { depends_on = [], blocks = [], related = ["gap-6a319c", "gap-770cb5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_late_done_leaves_tool_time_with_the_tools' crates/roko-cli/src && cargo test -p roko-cli --lib a_late_done_leaves_tool_time_with_the_tools"

[closed]
at = 2026-10-20
at_ts = "2026-10-19T23:17:26Z"
commit = "99dc563b8"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-19T22:50:58Z"
forced = false
evidence = "A call's turn end is now the latest Done the tap saw before the call's result, by event order, not by ms. Cause, from a run with the tap's order logged (d-15ba19): call-10-0's result and the next turn's Done both came at 1792450699658 ms, so 'latest Done <= result' took the later turn and the span was 0. Verify a_late_done_leaves_tool_time_with_the_tools passes (a run_tests call 1001..6000 with the next Done in the result's ms gives tools_ms 4999, not 0); the 5 live_tool_calls tests and 33 attempt tests pass; clippy is clean. Same $0 fixture (FAKE_CHECKS=1, glm-4.7 slug), before (d-15ba19, main 2cdb0fa66): T3 run_tests 2 calls 0 ms with model_ms 13,072, T4 model_ms 10,580 (one check lost). After (f-15ba19, this branch): model_ms 24-32 ms on every try (the stand-in answers at once), run_tests 5.6 s / 45.3 s / 5.1 s / 50.9 s, no 0 ms tool."
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
