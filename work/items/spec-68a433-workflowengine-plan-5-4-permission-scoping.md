+++
id = "spec-68a433"
kind = "spec"
title = "WorkflowEngine plan 5.4 Permission Scoping"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/safety"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.4 Permission Scoping"
discovered_from = "audit:tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.4 Permission Scoping"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
5 open items (2026-04-28 plan written for the since-retired WorkflowEngine; retarget to Graph engine or close): 5.4.1 11 permission scopes (file_reads, searches, edits_src,; 5.4.2 Per-scope state: `auto | ask | deny` + call_count; 5.4.3 Mode → safety mapping (architect = read-only, research; 5.4.4…

Imported without verification from:
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.4 Permission Scoping`

How to verify: WorkflowEngine was retired (#276); check whether a Graph-engine/config equivalent exists for: 5.4.1 11 permission scopes (file_reads, searches, edits_src, shell_safe, git_commit, etc.); 5.4.2 Per-scope state: `auto | ask | deny` + call_count; 5.4.3 Mode → safety mapping (architect = read-only, research = no writes); 5.4.4 Per-worktree scope for…
