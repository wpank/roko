+++
id = "bug-724982"
kind = "bug"
title = "roko chat, the vision loop and dispatch_v2's direct model calls write no cost rows"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli"]
created = 2026-10-01
updated = 2026-10-01
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
