+++
id = "gap-08d1c4"
kind = "gap"
title = "Architecture proof runs (express/ACP/HTTP parity, DAG, auto-fix, crash-resume, router learning, knowledge reuse, tournament, replay)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#Proof Runs"
discovered_from = "audit:tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#Proof Runs"
anchors = ["roko run \"add health check\""]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
10 open items (2026-04-28 plan written for the since-retired WorkflowEngine; retarget to Graph engine or close): P.1 `roko run "add health check"` → express pipeline → episo; P.2 Same prompt via ACP → identical behavior, AcpAdapter emi; P.3 Same prompt via HTTP → SseAdapter streams same events…

Imported without verification from:
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#Proof Runs`

How to verify: WorkflowEngine was retired (#276); check whether a Graph-engine/config equivalent exists for: P.1 `roko run "add health check"` → express pipeline → episode recorded → cost tracked; P.2 Same prompt via ACP → identical behavior, AcpAdapter emits all message types; P.3 Same prompt via HTTP → SseAdapter streams same events; P.4 Multi-task DAG plan →…
