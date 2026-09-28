+++
id = "spec-29d932"
kind = "spec"
title = "WorkflowEngine plan 4.1 Role Config System"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#4.1 Role Config System"
discovered_from = "audit:tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#4.1 Role Config System"
anchors = ["roles/*.toml", "PermissionProvider", "ContextSource", "CustomShellGate", "CustomHttpGate", "CustomMcpGate"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
6 open items (2026-04-28 plan written for the since-retired WorkflowEngine; retarget to Graph engine or close): 4.1.1 Role TOML schema: identity, rules, tools (allowed/deni; 4.1.2 Role loader + registry (replaces 28-variant enum); 4.1.3 Hot-reload on file change; 4.1.4 Ship defaults as…

Imported without verification from:
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#4.1 Role Config System`
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#4.2 Workflow Config System`
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#4.3 Gate Config System`

Warning: every file this item cites is gone (`roles/*.toml`) — likely obsolete or moved.

How to verify: WorkflowEngine was retired (#276); check whether a Graph-engine/config equivalent exists for: 4.1.1 Role TOML schema: identity, rules, tools (allowed/denied), budget, model_hint, prompt sections; 4.1.2 Role loader + registry (replaces 28-variant enum); 4.1.3 Hot-reload on file change; 4.1.4 Ship defaults as `roles/*.toml`; 4.1.5 Role →… / WorkflowEngine was retired (#276); check whether a Graph-engine/config equivalent exists for: 4.2.1 Workflow TOML schema: steps with role, gates, on_success, on_failure; 4.2.2 Workflow → PipelineState generation (TOML drives the state machine); 4.2.3 Built-in templates as TOML (express, standard, full, plan-execution); 4.2.4 Custom failure recovery… / WorkflowEngine was retired (#276); check whether a Graph-engine/config equivalent exists for: 4.3.1 Gate TOML schema: type, threshold, timeout, custom_command; 4.3.2 `CustomShellGate` impl: runs user-defined shell command, parses exit code; 4.3.3 Per-workflow gate configuration; 4.3.4 `CustomHttpGate` impl: calls external URL, checks response; 4.3.5…

Merged 3 mined candidates: m1-235, m1-236, m1-237.
