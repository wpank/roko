+++
id = "spec-a52c77"
kind = "spec"
title = "WorkflowEngine plan 5.2 Episode Replay"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.2 Episode Replay"
discovered_from = "audit:tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.2 Episode Replay"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
5 open items (2026-04-28 plan written for the since-retired WorkflowEngine; retarget to Graph engine or close): 5.2.1 Record: JsonlLogger already persists all events (Phase; 5.2.2 Replay: load events.jsonl → re-emit to EventBus at scr; 5.2.3 Branch-from-here: reconstruct context up to scrub posi…

Imported without verification from:
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.2 Episode Replay`

How to verify: WorkflowEngine was retired (#276); check whether a Graph-engine/config equivalent exists for: 5.2.1 Record: JsonlLogger already persists all events (Phase 3.4); 5.2.2 Replay: load events.jsonl → re-emit to EventBus at scrub position; 5.2.3 Branch-from-here: reconstruct context up to scrub position, create new session; 5.2.4 Auto-extract learnings…
