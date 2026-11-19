+++
id = "gap-6e789b"
kind = "gap"
title = "No record says whether a retry's diagnosis came from the helper model or roko's fallback: retry_feedback carries only the text, and the log's has_enriched_diagnosis is true for both (phase 4.3's T2#2 got the fallback after z.ai refused the helper, logged as enriched)"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, §11 and the telemetry gaps T13-T16), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-learn/src/error_enrichment.rs::enrich_error_digest", "crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::retry_feedback_event"]
links = { depends_on = [], blocks = [], related = ["gap-ece700", "bug-e33b7e", "bug-cdd885"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_retry_feedback_event_says_who_wrote_its_diagnosis' crates/roko-cli/src && cargo test -p roko-cli --lib a_retry_feedback_event_says_who_wrote_its_diagnosis"
+++

## Problem

A retry's diagnosis comes from one of two places:
- a helper model's answer, from `enrich_error_digest`;
- roko's deterministic fallback, when the helper fails or times out.

No record says which. The `retry_feedback` event carries the diagnosis text alone. The log line "storing gate
feedback for retry injection" says `has_enriched_diagnosis=true` for both, because it tests only
`feedback.diagnosis.is_some()`.

**Undertow, phase 4.3** (run 3752b8ff):
- T2#1's helper call went to glm-4.7 on z.ai and was refused ("Exhausted", 615 ms, $0, at 14:37:02Z).
- T2#2 got the fallback ("The primary error is [doctest] test cases: 4 | 2 passed | 2 failed …").
- The log said `has_enriched_diagnosis=true`.
- Telling that apart took the helper's cost row ($0, 0 tokens), the circuit-open log line and the fallback's wording.

## Why it matters

- Whether a retry got a model's diagnosis or the fallback is the variable behind bug-e33b7e and bug-cdd885. Today
  it can't be read from the records.
- Goal: truth.

## Where

- `crates/roko-learn/src/error_enrichment.rs::enrich_error_digest` returns a `String` and hides which path made it.
- `crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::retry_feedback_event` builds the event.
- `crates/roko-cli/src/graph_task_dispatch/verification.rs` logs the flag.

## Plan

1. Have `enrich_error_digest` return the source alongside the text: the model and provider, or the fallback with
   why (refused, timed out, empty answer).
2. Put `diagnosis_source` on the `retry_feedback` event, and make the log's flag true only for a model's diagnosis.
3. Test: `a_retry_feedback_event_says_who_wrote_its_diagnosis`.

## Done when

The `[[verify]]` passes, and a retry whose helper fails records `diagnosis_source: fallback` with the reason.
