+++
id = "gap-0eb14e"
kind = "gap"
title = "A model call's cost record has no time to first token: call_cost says how long a call took but not when its first token came, so a call slow to start and one slow to finish read the same; only a try's first call has one"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round v7 roko-effort, the phase 5.2 usage read (run 13260e60), the lead's ask"
discovered_from = "agent:roko-effort (round v7, phase 5.2 usage read)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::StreamTally", "crates/roko-agent/src/tool_loop/mod.rs::CallFacts", "crates/roko-cli/src/graph_task_dispatch/call_cost.rs::MeteredCalls", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
links = { depends_on = [], blocks = [], related = ["gap-1ba0ac", "gap-ece700"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_call_record_says_when_its_first_token_came' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_call_record_says_when_its_first_token_came"
+++

## Problem

A model call's cost record (`call_cost`, `DashboardEvent::CallCost`) says how long the call took (`wall_ms`) but not
when its first token came. Only a try's first call has a time to first token, in the verdict's `timing` and the
efficiency row's `time_to_first_token_ms`. The live stream records carry no timestamps.

So the records can't tell a call that was slow to start (a long prefill, a queue at the provider, silent reasoning)
from one that was slow to finish (a long answer). Round v7's phase 5.2 read (run 13260e60) found this:
- every one of T1#1's 15 calls had its total time;
- only the first had a time to first token (3.76 s);
- call 7's 32 s couldn't be split into waiting and writing.

## Why it matters

Reasoning models made time to first token a main cost of a turn. It is the round's "add telemetry" goal: the guide,
run analysis and model choice need per-call start and finish times from the records alone.

## Where

- `crates/roko-agent/src/tool_loop/mod.rs`:
  - `StreamEvent` (its `timestamp` is when the backend parsed the event);
  - `StreamTally`, which watches what a streamed call sent;
  - `CallFacts` and the loop's `TurnBudget::record_call` sites.
- The streaming backends know when they sent the request (`request_start` in `stream_turn`):
  - `crates/roko-agent/src/openai_compat_backend.rs`, chat completions and Responses;
  - `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs`;
  - `crates/roko-agent/src/tool_loop/backends/gemini_native.rs`.
- `crates/roko-cli/src/graph_task_dispatch/call_cost.rs::MeteredCalls` publishes the record.
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent::CallCost` is the record itself.

## Plan

1. Each event from a streaming backend that knows when it sent its request carries that instant (`sent_at`).
2. The loop's `StreamTally` keeps the time from that instant to the call's first token: its first text delta,
   non-empty reasoning delta or tool-call event, as `StreamEventKind::is_model_output` says (the predicate the try's
   own time to first token uses, gap-7a8474). Empty reasoning deltas only say the call is alive (gap-1ba0ac) and
   don't count.
3. `CallFacts` carries it as `first_token_ms`, and `call_cost` records it beside `wall_ms`, so "slow to start" and
   "slow to finish" read apart.
4. A call that wasn't streamed, or whose backend doesn't say when it sent the request, records `null`. Never an
   estimate.

## Done when

- `call_cost` rows carry `first_token_ms` on the OpenAI paths (chat completions and Responses), and on the Anthropic
  and Gemini native streams.
- A fake streamer that waits a known time before its first delta shows that wait.
- The `[[verify]]` test passes.
