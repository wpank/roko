+++
id = "bug-c638a4"
kind = "bug"
title = "ACP bridge under load: no send timeouts or backpressure, uncapped assistant_text, and session/prompt blocks the handler loop"
status = "open"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "M"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-f0f108"
anchors = ["crates/roko-acp/src/acp_adapter.rs", "crates/roko-acp/src/bridge_events/mod.rs", "crates/roko-acp/src/handler.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-f0f108"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib bridge_under_load"
+++

## Problem

bug-f0f108's fix landed its core. Its plan said to file three parts separately: P1-A, send timeouts and backpressure on the bridge's channels; P2-A, a cap on `assistant_text`; and 4a, `session/prompt` blocking the handler loop so other requests wait.

## Plan

Do the three parts, each with a test, under the `bridge_under_load_*` prefix.

## Done when

- `cargo test -p roko-acp --lib bridge_under_load` passes and covers all three.

## Notes

- Reported on 2026-10-01 by wk-specq, working on bug-f0f108, during the evening close-out round.
- 2026-10-01 (wk-specq): PARTIAL on work/bug-8dbffd; cargo verification deferred to the batch check.
  - P1-A: `send_cognitive_event` gives a progress event 30 s (`PROGRESS_EVENT_SEND_TIMEOUT`) in a full channel,
    then drops it. Turn-ending events (`CognitiveEvent::ends_turn`) and permission requests wait for room.
  - P2-A: `assistant_text` stops growing at 1 MiB (`MAX_ASSISTANT_TEXT_BYTES`), cut on a char boundary; the editor
    still gets every chunk.
  - 4a: worse than the item said. A request that arrived mid-prompt, or during a permission wait, was dropped with
    no response, so the client hung. It is now kept on `AcpSession::deferred_requests`, and `run_acp_server` answers
    those requests in order after the prompt. They still wait for the prompt.
  - Left open: run `session/prompt` as a task with one stdin reader routing responses, cancels and requests, so
    other requests don't wait. Also left: a write timeout for an editor that stops reading stdout. A stuck write
    still hangs the turn.
