+++
id = "bug-e139f9"
kind = "bug"
title = "HermesHttpAgent claims streaming but its run_streaming sends no events"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/hermes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "find-0d280d"
anchors = ["crates/roko-agent/src/hermes/http_adapter.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["find-0d280d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn hermes_http_streaming_forwards_each_event' crates/roko-agent/src/ && cargo test -p roko-agent --lib hermes_http_streaming"
+++

## Problem

`HermesHttpAgent` reports `supports_streaming = true`, but its `run_streaming` (`hermes/http_adapter.rs`, around line 403) never sends on `event_tx`, so a Hermes HTTP turn shows no live events at all.

## Plan

Send events as the response arrives, or report `supports_streaming = false` so callers use the non-streaming path. Add a test named `hermes_http_streaming_*`.

## Done when

- `cargo test -p roko-agent --lib hermes_http_streaming` passes.

## Notes

- Reported on 2026-10-01 by the worker on find-0d280d, during the evening close-out round.
- 2026-10-01 (wk-streams): implemented on work/gap-b35a57 (`51c8e3d9c`); cargo verification deferred to the batch
  check. `run_streaming` now passes each backend event to `event_tx` as it arrives (a `then` over the stream that
  awaits the send) and still collects the response from the same events, so `supports_streaming = true` is now
  true. Test: `hermes_http_streaming_forwards_each_event` serves the `chat_basic.sse` fixture from a wiremock server
  and checks that the caller receives the text deltas and `Done`. The verify now guards against a filter that
  matches no test.
