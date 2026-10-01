+++
id = "bug-ea7723"
kind = "bug"
title = "The codex, cursor and openai_parity streaming tests stay ignored until testutil's stream events carry session ids"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/testutil"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "479bec688"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report, checked on work/bug-739dcc at 1efb2fddb)"
anchors = ["crates/roko-agent/src/testutil.rs", "crates/roko-agent/tests/"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-25d24e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn stream_events_carry_session_ids' crates/roko-agent/src/ && cargo test -p roko-agent --lib stream_events_carry_session_ids"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T12:49:50Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20d gate on fae7133cd, re-checked with the clippy fix on 9f3c184c5 (MAIN 479bec688 has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/compose/core/execution/graph/neuro/runtime/serve; lib tests roko-cli 3282, roko-agent 2282, roko-core 1962, roko-serve 990, roko-compose 561, roko-graph 476, roko-runtime 288, roko-execution 245, roko-neuro 239 all pass; extras: codex/cursor/openai parity 4+4+4 (streaming tests no longer ignored), default_engine 1, C1 1, C7 2, bin 429, graph_task_dispatch loop 10/10, including stream_events_carry_session_ids. Merged 335132dd2 (work/mt-l3 9cc5cc469)."
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
