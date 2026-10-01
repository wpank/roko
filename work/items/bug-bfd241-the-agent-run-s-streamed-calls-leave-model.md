+++
id = "bug-bfd241"
kind = "bug"
title = "The agent run's streamed calls leave model_reported null although the provider's chunks name the model; only helper rows name it"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch/streaming", "roko-agent/dispatcher"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "bf40f3269"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix1's report, checked on work/gap-dad97b at f39777257)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/streaming.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "gap-dad97b", "bug-a5f181"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streamed_attempts_record_the_provider_reported_model' crates/roko-cli/src/ && cargo test -p roko-cli --lib streamed_attempts_record_the_provider_reported_model"

[closed]
at = 2026-10-01
by = "coordinator (session 7622b882)"
evidence = "Batch 20b gate on cad1a56e1 (MAIN bf40f3269 has the same crates): check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests roko-agent 2278, roko-cli 3261, roko-core 1956, roko-learn 1207, roko-gate 690, roko-std 227 and roko-cli bin 429 all pass, including streamed_attempts_record_the_provider_reported_model. Merged 3dfdef519."
+++

## Problem

wk-bench-fix1 ran the benchmark's Roko arm against b0ede92d7 (the batch merge of bug-31438d). The rows for the agent run's streamed calls had `model_reported: null`, although the provider's stream chunks name the model; only the helper-call rows named it. At b0ede92d7, neither `graph_task_dispatch/streaming.rs` nor the dispatcher sets `model_reported`.

## Why it matters

One settled record per attempt (epic spec-b7303f): the main calls of an attempt are exactly the ones the pin check needs. The benchmark's Roko arm (gap-dad97b) has to fall back to the proxy.

## Where

The streaming dispatch path, where chunks are turned into the attempt's usage.

## Plan

1. Take the model from the stream's chunks (the last chunk that names one), and carry it into the attempt's usage and records, as the batch path does.
2. Add `streamed_attempts_record_the_provider_reported_model`.

## Done when

- [ ] Streamed attempts record the model their provider reported.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-31438d, once merged.
- Implemented on `work/bug-739dcc` at `7faf89805`; cargo verification deferred to the batch check. `streamed_attempts_record_the_provider_reported_model` (targeted `cargo test` passed; the test asserts the call streamed). Root cause: with live output on (the stall watchdog's tap, on by default) the tool loop streams each model call and rebuilds the response with `collect_stream_to_response`, which dropped the chunks' model; helper calls are not streamed, so only they named it. `StreamEvent` gains `model`: `parse_sse_line`, Anthropic's `message_start` and `response_to_synthetic_stream` set it, and the collector writes the last one back as the response's `model`.
