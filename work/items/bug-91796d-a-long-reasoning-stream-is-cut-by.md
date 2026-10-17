+++
id = "bug-91796d"
kind = "bug"
title = "A long reasoning stream is cut by roko's own per-call limits: a 4,096-event cap denies glm-4.7's calls and a 120 s total request timeout breaks streams still delivering"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-agent/streaming"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "tmp/hermes-roko-v6/findings/dogfood-will.md#R3-1 (dogfood-will, 2026-10-16)"
discovered_from = "hermes-roko-v6 round: dogfood-will Run 3, pocket-synth through Hermes"
anchors = ["crates/roko-agent/src/immune_boundary.rs::drive_streaming_inner", "crates/roko-agent/src/openai_compat_backend.rs::stream_turn", "crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::stream_turn", "crates/roko-agent/src/tool_loop/backends/gemini_native.rs::stream_turn"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_long_reasoning_stream_passes_the_boundary' crates/roko-agent/ && grep -rqw 'fn a_stream_that_keeps_delivering_outlives_the_request_timeout' crates/roko-agent/ && cargo test -p roko-agent a_long_reasoning_stream_passes_the_boundary && cargo test -p roko-agent a_stream_that_keeps_delivering_outlives_the_request_timeout"
+++

## Problem

roko's own per-call limits cut a reasoning model's long stream. That happens on exactly the hard tasks the ladder
climbs to a stronger rung for. In dogfood-will's Run 3 (2026-10-16, `$V6/findings/dogfood-will.md` R3-1), task SVF
climbed to glm-4.7 (z.ai) and lost all four attempts on that rung to the limits below.

1. **The 4,096-event cap.** `MAX_PROVIDER_STREAM_CHUNKS = 4_096` counts every non-tool-result stream event of one
   model call, each reasoning delta included (`immune_boundary.rs` `drive_streaming_inner`). A call over it is
   denied as `provider_stream_limit_exceeded`. glm-4.7 streamed 7,915, 4,577, 4,012 and 4,264 reasoning events in
   SVF attempts 3–6, about one token (~3 bytes) each, 12–24 KB of text per attempt. Attempts 3, 4 and 6 were denied.
   The 4 MiB byte limit was far away.
2. **The 120 s total request timeout.** Every streamed API call is sent with reqwest's total
   `.timeout(request_timeout)`, which is `DEFAULT_REQUEST_TIMEOUT_MS = 120_000` in the tool loop. The timeout also
   covers reading the body, so a stream still delivering tokens is cut at 120 s with "read chunk failed: error
   decoding response body". SVF attempts 5 and 6 ended 120.02 s after dispatch started. The same total timeout is set
   on the Anthropic and Gemini streaming requests.

## Why it matters

The pitch is cheap models first and a climb when a check fails. The first climb died here: about 11 of the run's
15m27s went on these 4 attempts, and the plan failed. Reasoning models need long streams on hard tasks.

## Where

- `crates/roko-agent/src/immune_boundary.rs::drive_streaming_inner`: the per-call counts.
- `crates/roko-agent/src/openai_compat_backend.rs::stream_turn`,
  `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::stream_turn`,
  `crates/roko-agent/src/tool_loop/backends/gemini_native.rs::stream_turn`: the total timeout on streamed requests.

## Plan

1. Count a call's content by its bytes (the 4 MiB limit stays). Replace the 4,096-event cap with a ceiling that no
   real call reaches, so it stops only a runaway stream: one event per output token, and no model call outputs
   anywhere near 256K tokens.
2. A streamed request gets no total timeout. Instead:
   - `request_timeout` bounds the wait for the response, then each silence between chunks (a stalled stream);
   - a long ceiling (30 min) bounds a stream that keeps delivering.
   Non-streamed requests keep their total timeout.

## Done when

`[[verify]]` passes:
- a scripted provider that streams 8,000 one-token reasoning deltas in one call passes the boundary, and a runaway
  stream is still denied;
- an OpenAI-compatible stream that keeps delivering past `request_timeout` completes, and one that stalls longer than
  it fails with a timeout.
