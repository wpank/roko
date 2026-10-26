+++
id = "gap-a94400"
kind = "gap"
title = "OpenAI's keepalive events on a streamed Responses call count as no progress: roko drops them, so a call its provider says is alive every 30 s looks silent to the stall watchdog"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round v7 roko-effort, a recorded gpt-5.4-mini call in gap-1ba0ac's A/B (c4fa953f T4: 114 s, 3 keepalives), the lead's ask"
discovered_from = "agent:roko-effort (round v7, gap-1ba0ac A/B)"
anchors = ["crates/roko-agent/src/openai_responses.rs::ResponsesTurn", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs::AttemptProgress"]
links = { depends_on = [], blocks = [], related = ["gap-1ba0ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_keepalive_says_the_call_is_alive' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_keepalive_says_the_call_is_alive"
+++

## Problem

OpenAI sends `{"type":"keepalive"}` events on a streamed Responses call while its model works and emits nothing
else. A recorded gpt-5.4-mini call (round v7's A/B, c4fa953f T4) shows the pattern: it reasoned 10,672 tokens in
114 s and sent 3 keepalives after its reasoning item started.

roko's `ResponsesTurn` drops events it doesn't map, keepalives included. The bytes reset the backend's wait for the
next chunk, but nothing reaches the live output, so the stall watchdog sees no progress.

gap-1ba0ac's own heartbeat covers a silence inside an open reasoning item. A silence with nothing open (after
`response.in_progress`, between output items, or under a model that streams no reasoning item) has only the
provider's keepalives to show the call is alive.

## Why it matters

The watchdog warns at 180 s of silence and cancels at 300 s (Undertow's `[conductor]`). A call that is plainly alive,
since its provider says so every 30 s, should count as progress. The next phases run on this path.

## Where

- `crates/roko-agent/src/openai_responses.rs::ResponsesTurn`: `line` maps Responses events to the tool loop's stream
  events. `response.in_progress` and a reasoning item's start already become empty `ReasoningDelta`s, which the
  watchdog counts and everything else ignores (gap-1ba0ac).
- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs::AttemptProgress`: any live event is progress.

## Plan

Map `keepalive` to an empty `ReasoningDelta`, as `response.in_progress` already is. Test it:
- at the parser;
- through the backend over a real stream that sends only keepalives for a while;
- at the watchdog: a 120 s silent stream with keepalives every 30 s isn't warned, though the same silence without
  them is.

## Done when

The `[[verify]]` test passes, and a stream of only keepalives keeps an attempt's progress current.
