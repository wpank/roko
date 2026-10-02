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
updated = 2026-10-02
last_verified = 2026-10-02
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
- 2026-10-02 (wk-specq): the remainder is implemented on work/bug-8dbffd; cargo verification deferred to the batch
  check.
  - 4a: `run_acp_server_with_transport` is now the only stdin reader. Each `session/prompt` runs as a tokio task
    (`handler.rs::start_prompt`) that owns its session, which leaves the manager until `finish_prompt` returns it.
    The loop routes the client's responses (pending map) and `session/cancel` (the prompt's token) to it, and
    answers other requests at once. Requests for a session whose prompt is running wait in `deferred`.
    `AcpSession::inbound_routed` keeps the prompt's stream loop and permission wait from reading stdin. While a
    session is out, `session/list` shows it as it was taken, and a config reload in that time reaches it (with its
    `config_option_update`) when it comes back (`SessionManager::insert_session`).
  - Reads: `read_message` keeps a cancelled call's partial line (`read_until` into a persistent buffer).
    `read_line` dropped those bytes whenever a `select!` cancelled it.
  - Writes: give up after 60 s (`DEFAULT_WRITE_TIMEOUT`, `TransportError::WriteTimeout`), and later writes refuse.
    `run_acp_server` then skips its blocking final stdout write.
  - Tests: `bridge_under_load_answers_requests_while_a_prompt_runs`, `..._read_resumes_after_a_cancelled_read`,
    `..._write_gives_up_on_a_client_that_stopped_reading`, and
    `..._taken_session_stays_listed_and_catches_up_with_a_reload`.
