+++
id = "spec-8c8f00"
kind = "spec"
title = "WorkflowEngine plan 5.3 Editor Integration"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-acp"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.3 Editor Integration"
discovered_from = "audit:tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.3 Editor Integration"
anchors = ["textDocument/publishDiagnostics", "AgentCursorMoved", "AgentNavigated"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
5 open items (2026-04-28 plan written for the since-retired WorkflowEngine; retarget to Graph engine or close): 5.3.1 Tool call interception: when agent reads/writes a file; 5.3.2 Gate gutter marks: `GateFailed.line_marks` → `textDocu; 5.3.3 CallGraph trace: code-intelligence index provides fn →…

Imported without verification from:
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.3 Editor Integration`

Warning: every file this item cites is gone (`textDocument/publishDiagnostics`) — likely obsolete or moved.

How to verify: WorkflowEngine was retired (#276); check whether a Graph-engine/config equivalent exists for: 5.3.1 Tool call interception: when agent reads/writes a file, emit `AgentCursorMoved`; 5.3.2 Gate gutter marks: `GateFailed.line_marks` → `textDocument/publishDiagnostics`; 5.3.3 CallGraph trace: code-intelligence index provides fn → file:line tree; 5.3.4…
