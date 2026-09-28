+++
id = "gap-6a673d"
kind = "gap"
title = "Serve template dispatch injects prompt-experiment context without durable assignment receipts"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/experiments", "roko-learn/experiments"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#prompt-experiment-coverage-across-runtimes----partial"
anchors = ["crates/roko-serve/src/dispatch.rs:48", "crates/roko-serve/src/feedback.rs"]
links = { depends_on = [], blocks = [], related = ["gap-fdd27f"], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

According to GAPS.md, serve adds ephemeral experiment context instead of replacing a canonical named section under a durable assignment/dispatch receipt. Its process-local dedup can therefore double-count after a crash. Related residuals:
- LearningRuntime's variant-only WAL can replay an already committed outcome when a later snapshot or truncation step fails.
- A permissive startup cache hides malformed state.
- Concurrent commits can publish cache snapshots out of order.

A three-phase fix was designed (per-assignment receipts, replay-safe dedup IDs, monotonic cache publication). Backlog #359 was marked implemented on 2026-09-04, which conflicts with GAPS.md, so check the code first. The Graph plan path assigns no experiments at all (gap-fdd27f).

Fix: give serve the receipt protocol that ACP uses (canonical section replacement, dispatched hash, idempotent settle) and make cache publication monotonic. Define tombstone compaction before the store approaches its 64 MiB ceiling.
