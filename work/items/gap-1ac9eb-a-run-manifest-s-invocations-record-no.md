+++
id = "gap-1ac9eb"
kind = "gap"
title = "A run manifest's invocations record no end, outcome or event index: phase 5.1's first invocation failed at 17:20Z, yet the manifest says only 'succeeded' at 20:51Z, and the resume's events sit under serve run 4728d22e, which the manifest doesn't name"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, §11 and the telemetry gaps T13-T16), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-learn/src/telemetry/records.rs::RunInvocation", "crates/roko-cli/src/graph_execution/run_manifest.rs::RunManifests"]
links = { depends_on = [], blocks = [], related = ["gap-ece700", "bug-b7f6a8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn each_invocation_records_its_end_and_outcome' crates/roko-cli/src && cargo test -p roko-cli --lib each_invocation_records_its_end_and_outcome"
+++

## Problem

A run manifest's `invocations` records when each process started, its harness and its config. It doesn't record
when the invocation ended, how, or where its events went.

**Undertow's phase 5.1** (run 814a7340):
- Invocation 1 (74c3594bc) failed at 17:20:47Z, when T2 was exhausted on a stale range.
- Invocation 2 resumed it on 2ca53cb61 and succeeded at 20:51:29Z.
- The manifest's only end is `closed: succeeded, 20:51:29Z`. Invocation 1's failure survives only as the
  `run_completed` event in its own event index.
- Invocation 2 ran as serve run 4728d22e, whose events are indexed under that id (`events-by-run/<sha256(4728d22e…)>`),
  and the manifest doesn't name it. The run-analyst found it by searching the indexes; phase 4.1's resume
  (6059db44 under 92b8071f) was the same.

## Why it matters

- Per-invocation time, outcome and spend are what a resume's analysis needs: what failed, what the resume carried,
  and how long each took.
- Today they come from the event indexes and the operator's notes.
- Goal: truth.

## Where

- `crates/roko-learn/src/telemetry/records.rs::RunInvocation`, the record.
- `crates/roko-cli/src/graph_execution/run_manifest.rs::RunManifests`, which writes it.

## Plan

1. Add `ended_at` and `outcome` to `RunInvocation`, set when the invocation's process finishes. Use the same
   outcome words as `closed.status`.
2. Add `index_run_id` when the invocation's events are indexed under another run id, such as a serve run.
3. Optionally add `spent_usd` for the invocation alone, beside the carried amount bug-b7f6a8 already records.
4. Test: `each_invocation_records_its_end_and_outcome`.

## Done when

The `[[verify]]` passes, and a resumed run's manifest says when and how each invocation ended and where its events
are.
