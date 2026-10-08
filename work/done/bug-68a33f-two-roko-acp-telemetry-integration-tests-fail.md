+++
id = "bug-68a33f"
kind = "bug"
title = "Two roko-acp telemetry integration tests fail on main: the model-stream request to the mock provider fails"
status = "done"
triage = "verified"
severity = "p2"
size = "S"
subsystem = ["roko-acp/tests"]
created = 2026-10-02
updated = 2026-10-08
last_verified = 2026-10-08
last_verified_rev = "ad7a3a337"
source = "roko-7d: cargo nextest run --workspace for the workflow-audit merge (2026-10-02)"
discovered_from = "merge:bfd36512f"
anchors = ["crates/roko-acp/tests/telemetry_integration.rs", "crates/roko-acp/tests/helpers.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn single_dispatch_produces_episode' crates/roko-acp/tests/ && cargo test -p roko-acp --test telemetry_integration single_dispatch_"

[closed]
at = 2026-10-08
at_ts = "2026-10-08T13:21:54Z"
commit = "4289f4a12"
forced = false
evidence = "gate 24 (2026-10-08): its verify passes; check, nightly fmt, clippy -D warnings, CI feature checks, nextest --workspace --lib (15223 passed), touched crates' full tests, roko-acp integration, roko-cli bin (457) and CI canaries all pass"
+++

## Problem

Two ACP integration tests fail on main (checked on a build of main at f49242f63 and on bfd36512f):

- `single_dispatch_produces_episode`: the episode records `success = false` with
  `failure_reason = "ACP pipeline error: model stream failed: agent error (gpt-5.4): network error: transport
  error: request failed: error sending request for url (http://127.0.0.1:<port>/chat/completions)"`.
- `single_dispatch_feeds_cascade_router`: the router records (1 attempt, 0 successes) instead of (1, 1).

The other three tests in the file pass, and `mock_dispatch` itself returns Ok: the mock server's task ends
without error, so it accepted and answered one request.

## Why it matters

These are the end-to-end checks that an ACP dispatch feeds the episode log and the cascade router. While they
fail, a real regression in ACP learning would go unnoticed. The batch gates run lib tests only, so nothing
flags them.

## Where

- `crates/roko-acp/tests/helpers.rs::spawn_mock_provider_server`: accepts exactly one connection and answers with
  a non-streaming JSON body (`Content-Type: application/json`, `Connection: close`).
- `crates/roko-acp/tests/telemetry_integration.rs`
- The ACP dispatch path that makes the model-stream request (`crates/roko-acp/src/bridge_events/`).

## Current state

Unknown cause. Two candidates, neither checked: the dispatch now makes a second request (a retry, or another
call before the model stream) that finds the single-connection mock gone; or the streaming client cannot read
the non-streaming body. A recent change to the request is 89b3237e0 (`stream_options.include_usage`).

## Plan

1. Log each request the mock receives (method, path, `stream` flag) and count the connections.
2. Fix the cause: make the mock serve the request the client sends (SSE when `stream` is true, or several
   connections), or fix the dispatch if it now sends a request it should not.

## Done when

- Both tests pass. Verify:
  `grep -rqw 'fn single_dispatch_produces_episode' crates/roko-acp/tests/ && cargo test -p roko-acp --test telemetry_integration single_dispatch_`

## Notes

- Found by roko-7d's full `cargo nextest run --workspace` for the workflow-audit merge (2026-10-02).
- 2026-10-02 (roko-90): check `stream_options: {include_usage: true}` first (wave 3a's PK07, task 2101,
  89b3237e0); `[providers.<name>] stream_usage = false` turns it off per provider, which tells you quickly
  whether the mock chokes on it.
