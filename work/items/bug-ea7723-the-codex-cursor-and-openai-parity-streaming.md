+++
id = "bug-ea7723"
kind = "bug"
title = "The codex, cursor and openai_parity streaming tests stay ignored until testutil's stream events carry session ids"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/testutil"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "a040867c9"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report, checked on work/bug-739dcc at 1efb2fddb)"
anchors = ["crates/roko-agent/src/testutil.rs", "crates/roko-agent/tests/"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-25d24e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn stream_events_carry_session_ids' crates/roko-agent/src/ && cargo test -p roko-agent --lib stream_events_carry_session_ids"
+++

## Problem

bug-25d24e fixes the cache-read key in `testutil::response_from_stream_events`, which closes only its usage-key part. The codex, cursor and openai_parity streaming tests stay `#[ignore]`d for a second reason: testutil's stream events carry no session ids, which those adapters require (wk-model-truth).

## Why it matters

One settled record per attempt (epic spec-b7303f): the streaming parity tests are the guard that streamed and batch dispatch record the same thing. p3.

## Where

`crates/roko-agent/src/testutil.rs` (the stream-event builders) and the ignored streaming tests.

## Plan

1. Give testutil's stream events session ids in each provider's format, and un-ignore the codex, cursor and openai_parity streaming tests.
2. Add `stream_events_carry_session_ids`.

## Done when

- [ ] No streaming parity test is ignored for missing session ids.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/mt-l3` at `a040867c9`; cargo verification deferred to the batch check. `stream_events_carry_session_ids` (targeted `cargo test` passed), and the codex, cursor and openai_parity `streaming` tests, no longer ignored, pass. Stream events now carry the session ids in production, not only in testutil. `StreamEvent.session` holds the `id`, `session_id` and `thread_id` a chunk or response named (`tool_loop::session_ids`), set by `parse_sse_line` (the OpenAI-compatible, Codex and Cursor streams) and by `response_to_synthetic_stream`. `collect_stream_to_response` writes the last of each back where `extract_session` reads them, so a streamed turn keeps the provider's session as an answered one does. `testutil::response_from_stream_events` now calls `collect_stream_to_response` instead of its own copy. The tool_call parity tests stay ignored, for a separate fixture drift.
