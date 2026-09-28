+++
id = "bug-e831bd"
kind = "bug"
title = "Memory growth during plan runs: unbounded efficiency_events Vec (9.5GB RSS after 17 min)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-08-13
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (F5) + 1. Demo (P1-3)"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (F5) + 1. Demo (P1-3)"
anchors = ["crates/roko-cli/src/runner/mod.rs::run", "crates/roko-cli/src/tui/jsonl_tailer.rs::IncrementalTailer"]
links = { depends_on = [], blocks = [], related = ["bug-f2a387"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "obsolete: the unbounded efficiency_events Vec lived in the deleted Runner-v2 event loop (runner::run is now an erroring stub, crates/roko-cli/src/runner/mod.rs:58-84) and nothing in graph_execution/ or graph_task_dispatch.rs accumulates efficiency events. Remaining in-memory copies are TUI IncrementalTailer snapshots (tui/jsonl_tailer.rs:36, uncapped Vec over the rotation-bounded efficiency.jsonl); TUI memory growth is tracked by bug-f2a387"
+++
Dogfood saw 9.5GB RSS after 17 min; root cause cited as unbounded `efficiency_events: Vec` never drained in runner v2 (F5 / P1-3).

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (F5) + 1. Demo (P1-3)`

How to verify: grep `efficiency_events` in crates/roko-cli (runner + graph_execution) and check it is bounded/drained after flush; long plan run RSS check.

Verified 2026-09-28: superseded - Runner-v2 loop removed; see bug-f2a387 for TUI-side buffers.
