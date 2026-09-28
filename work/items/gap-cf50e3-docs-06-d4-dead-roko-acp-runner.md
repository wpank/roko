+++
id = "gap-cf50e3"
kind = "gap"
title = "DOCS-06 D4: Dead roko-acp runner.rs staging code (spawn_runtime_event_bridge, ~460 LOC)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-acp"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D4. roko-acp runner.rs staging code (~460 LOC)"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D4. roko-acp runner.rs staging code (~460 LOC)"
anchors = ["crates/roko-acp/src/runner.rs", "spawn_runtime_event_bridge"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
DC-08 confirmed the spawn_runtime_event_bridge cluster in roko-acp runner.rs is dead; decision: REMOVE.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D4. roko-acp runner.rs staging code (~460 LOC)`
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Confirmed Dead — CLEAN NOW`
- `tmp/dogfood/2026-09-18-session.md#Final Summary`
- `docs/v3/39-ROADMAP.md#3.2 Dead Code Cleanup`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep spawn_runtime_event_bridge callers. / grep callers of the staging functions in roko-acp.

Merged 2 mined candidates: m4-048, m5-085.
