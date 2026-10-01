+++
id = "bug-724982"
kind = "bug"
title = "roko chat, the vision loop and dispatch_v2's direct model calls write no cost rows"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "a788dfd8d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-c1f6b8"
anchors = ["crates/roko-cli/src/chat_session.rs", "crates/roko-cli/src/dispatch_v2.rs", "crates/roko-cli/src/vision_loop.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-c1f6b8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib chat_calls_write_cost_rows"
+++

## Problem

After bug-c1f6b8, `roko chat` (chat_session.rs), the vision loop and dispatch_v2's direct ModelCallService path still write no `costs.jsonl` rows. Each can opt in with one line: `FeedbackService::with_cost_records()` or `ModelCallFeedbackRecorder::with_cost_records()`.

## Plan

Opt each one in, and add a test named `chat_calls_write_cost_rows`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-c1f6b8, during the evening close-out round.

2026-10-02 (wk-planrun): implemented on work/gap-dd4826, based on a788dfd8d, which has bug-c1f6b8's `with_cost_records()`; cargo verification deferred to the batch check.
Four callers whose model calls nothing else costs now write `.roko/learn/costs.jsonl` rows: `ChatFeedbackRuntime::new` (chat_session.rs, `roko chat`'s FeedbackService); `run_direct_provider_chat` (chat.rs, `roko agent chat` against a direct provider, included at the coordinator's request); `record_model_call_feedback` (vision_loop/evaluator.rs); and `dispatch_via_model_call_service` (dispatch_v2.rs, which has no callers today). `record_agent_dispatch_feedback` in dispatch_v2.rs stays as it is: it serves Graph attempts and helper calls, which Graph feedback already costs. Opting it in would count them twice.
Test: `chat_calls_write_cost_rows` (chat_session.rs). A chat turn whose fake Claude CLI reports usage writes exactly one cost row, with role `chat`, provider `mock` and its tokens.
