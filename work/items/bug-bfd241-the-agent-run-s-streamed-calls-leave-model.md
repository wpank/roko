+++
id = "bug-bfd241"
kind = "bug"
title = "The agent run's streamed calls leave model_reported null although the provider's chunks name the model; only helper rows name it"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch/streaming", "roko-agent/dispatcher"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix1's report, checked on work/gap-dad97b at f39777257)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/streaming.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "gap-dad97b", "bug-a5f181"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streamed_attempts_record_the_provider_reported_model' crates/roko-cli/src/ && cargo test -p roko-cli --lib streamed_attempts_record_the_provider_reported_model"
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
