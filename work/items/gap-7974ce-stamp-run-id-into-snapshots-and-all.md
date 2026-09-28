+++
id = "gap-7974ce"
kind = "gap"
title = "Stamp run_id into Snapshots and All Persisted Events"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/orchestrator"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/212-run-id-snapshot-events.md#212 — Stamp run_id into Snapshots and All Persisted Events"
discovered_from = "audit:tmp/backlog/212-run-id-snapshot-events.md#212 — Stamp run_id into Snapshots and All Persisted Events"
anchors = ["crates/roko-cli/src/orchestrator/executor/snapshot.rs", "crates/roko-cli/src/runner/structured_log.rs", ".roko/events.jsonl", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/runner/persist.rs", "crates/roko-fs/src/run_index.rs", "crates/roko-cli/src/commands/run_index.rs", "ExecutorSnapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[partial] SOURCE-DONE; CLI/REPAIR CHECKPOINT VERIFIED, LIVE-RUN PROOF OPEN (2026-08-31; PR #73, expanded by… — events from multiple runs are indistinguishable and snapshots cannot be correlated with events without a run_id. `run_id` is generated at runner startup and is already present in…

Imported without verification from:
- `tmp/backlog/212-run-id-snapshot-events.md#212 — Stamp run_id into Snapshots and All Persisted Events`
- `tmp/backlog/archive/212-run-id-snapshot-events.md#(archived copy; status: SOURCE-DONE; CLI/REPAIR CHECKPOINT VERIFIED, LIVE-RUN…)`
- `tmp/backlog/_archive/_mori-diffs-gaps.md#Groups E-2 and G-1`
- `tmp/archive/backlog-closure-2026-09-01.md#Items receiving verification updates (212)`

How to verify: Check: A final live run proves every emitted line and per-run index record has the same `run_id`; [evidence: own status: Verified (2026-09-03) — run_id on events, snapshots, per-run index; 00-INDEX historical claim: PR #73 + `5f689d66e`. Active file retained…]
