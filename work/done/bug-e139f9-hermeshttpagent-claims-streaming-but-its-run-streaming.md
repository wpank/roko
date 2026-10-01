+++
id = "bug-e139f9"
kind = "bug"
title = "HermesHttpAgent claims streaming but its run_streaming sends no events"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/hermes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "find-0d280d"
anchors = ["crates/roko-agent/src/hermes/http_adapter.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["find-0d280d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn hermes_http_streaming_forwards_each_event' crates/roko-agent/src/ && cargo test -p roko-agent --lib hermes_http_streaming"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:25Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:47:13Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
