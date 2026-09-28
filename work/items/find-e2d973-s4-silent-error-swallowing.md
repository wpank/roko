+++
id = "find-e2d973"
kind = "finding"
title = "S4: Silent Error Swallowing"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S4. Silent Error Swallowing"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S4. Silent Error Swallowing"
anchors = ["orchestrate.rs", "jsonl_logger.rs:76-77", "workflow_engine.rs:438", "unified.rs:141", "run.rs:1063", "config.rs:2197", "terminal.rs:359-362", "fswatcher.rs:27"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Systemic audit finding (2026-04-28) with 8 open checklist fixes: S4.1 Audit and replace `.ok()` in orchestrate.rs; S4.2 Log JSONL write failures (at minimum warn); S4.3 Log affect persist failures; S4.4 Surface serve failure to user (not just tra; S4.5 Surface episode logger failure more promine…

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S4. Silent Error Swallowing`

How to verify: Check each open sub-item (S4.1, S4.2, S4.3, S4.4, S4.5, S4.6, S4.9, S4.10). S4.1 targets orchestrate.rs (removed) so likely obsolete; check remaining .ok() swallowing in JSONL/affect/episode writers; overlaps backlog #343.
